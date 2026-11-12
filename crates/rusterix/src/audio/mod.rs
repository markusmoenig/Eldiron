use std::f32::consts::TAU;
use std::io::Cursor;
use std::sync::Arc as StdArc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
pub mod output;
pub mod preparation;
pub use preparation::PreparedClip;

#[derive(Debug, Clone, Copy)]
pub struct AudioConfig {
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub master_volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: None,
            channels: None,
            master_volume: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug)]
pub enum AudioError {
    NoOutputDevice,
    DefaultOutputConfig(cpal::DefaultStreamConfigError),
    BuildStream(cpal::BuildStreamError),
    PlayStream(cpal::PlayStreamError),
    PcmBudget,
    Cancelled,
    PreparationUnavailable(String),
    Decode(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOutputDevice => write!(f, "no default output audio device found"),
            Self::DefaultOutputConfig(e) => {
                write!(f, "failed to query default output stream config: {e}")
            }
            Self::BuildStream(e) => write!(f, "failed to build audio output stream: {e}"),
            Self::PlayStream(e) => write!(f, "failed to start audio output stream: {e}"),
            Self::PcmBudget => write!(f, "audio PCM budget exceeded"),
            Self::Cancelled => write!(f, "audio preparation cancelled"),
            Self::PreparationUnavailable(e) => write!(f, "audio preparation unavailable: {e}"),
            Self::Decode(e) => write!(f, "failed to decode audio clip: {e}"),
        }
    }
}

impl std::error::Error for AudioError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SineVoiceId(u64);

#[derive(Debug)]
struct Voice {
    id: SineVoiceId,
    phase: f32,
    phase_inc: f32,
    gain: f32,
    samples_left: usize,
}

type DecodedClip = PreparedClip;

#[derive(Debug)]
struct ClipVoice {
    name: String,
    samples: StdArc<[f32]>,
    pos: f64,
    step: f64,
    gain: f32,
    bus: String,
    looping: bool,
}

#[derive(Debug)]
struct MixerState {
    next_id: u64,
    master_volume: f32,
    voices: Vec<Voice>,
    clips: std::collections::HashMap<String, DecodedClip>,
    clip_voices: Vec<ClipVoice>,
    bus_volumes: std::collections::HashMap<String, f32>,
}

impl MixerState {
    fn new(master_volume: f32) -> Self {
        let mut bus_volumes = std::collections::HashMap::default();
        bus_volumes.insert("master".to_string(), 1.0);
        bus_volumes.insert("music".to_string(), 1.0);
        bus_volumes.insert("sfx".to_string(), 1.0);
        bus_volumes.insert("ui".to_string(), 1.0);
        bus_volumes.insert("ambience".to_string(), 1.0);
        bus_volumes.insert("voice".to_string(), 1.0);
        Self {
            next_id: 1,
            master_volume,
            voices: Vec::new(),
            clips: std::collections::HashMap::default(),
            clip_voices: Vec::new(),
            bus_volumes,
        }
    }

    fn pcm_bytes_without_cache(&self, excluded: &str) -> usize {
        let mut pointers = std::collections::HashSet::new();
        self.clips
            .iter()
            .filter(|(name, _)| name.as_str() != excluded)
            .map(|(_, clip)| &clip.samples)
            .chain(self.clip_voices.iter().map(|voice| &voice.samples))
            .filter_map(|samples| {
                pointers
                    .insert(Arc::as_ptr(samples) as *const f32 as usize)
                    .then_some(samples.len().saturating_mul(4))
            })
            .fold(0usize, usize::saturating_add)
    }
    fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 4.0);
    }

    fn add_sine_voice(
        &mut self,
        sample_rate: u32,
        hz: f32,
        duration_seconds: f32,
        gain: f32,
    ) -> SineVoiceId {
        let id = SineVoiceId(self.next_id);
        self.next_id += 1;

        let hz = hz.max(1.0);
        let seconds = duration_seconds.max(0.0);
        let gain = gain.clamp(0.0, 1.0);
        let samples_left = (seconds * sample_rate as f32) as usize;
        let phase_inc = (hz / sample_rate as f32) * TAU;

        self.voices.push(Voice {
            id,
            phase: 0.0,
            phase_inc,
            gain,
            samples_left,
        });
        id
    }

    fn stop_voice(&mut self, id: SineVoiceId) {
        self.voices.retain(|v| v.id != id);
    }

    fn clear_clips(&mut self) {
        self.clips.clear();
        self.clip_voices.clear();
    }

    fn insert_clip(&mut self, name: String, clip: DecodedClip) {
        self.clips.insert(name, clip);
    }

    fn set_bus_volume(&mut self, bus: &str, volume: f32) {
        self.bus_volumes
            .insert(bus.to_string(), volume.clamp(0.0, 4.0));
    }

    fn get_bus_volume(&self, bus: &str) -> f32 {
        self.bus_volumes.get(bus).copied().unwrap_or(1.0)
    }

    fn clear_bus(&mut self, bus: &str) {
        self.clip_voices.retain(|v| v.bus != bus);
    }

    fn clear_all_buses(&mut self) {
        self.clip_voices.clear();
    }

    fn play_clip(
        &mut self,
        output_sample_rate: u32,
        name: &str,
        bus: &str,
        gain: f32,
        looping: bool,
    ) -> bool {
        let Some(clip) = self.clips.get(name).cloned() else {
            return false;
        };
        if clip.samples.is_empty() || clip.sample_rate == 0 || output_sample_rate == 0 {
            return false;
        }
        let step = clip.sample_rate as f64 / output_sample_rate as f64;
        self.clip_voices.push(ClipVoice {
            name: name.to_string(),
            samples: clip.samples,
            pos: 0.0,
            step,
            gain: gain.clamp(0.0, 4.0),
            bus: bus.to_string(),
            looping,
        });
        if !self.bus_volumes.contains_key(bus) {
            self.bus_volumes.insert(bus.to_string(), 1.0);
        }
        true
    }
    fn play_clip_bounded(
        &mut self,
        rate: u32,
        name: &str,
        bus: &str,
        gain: f32,
        looping: bool,
        voices: usize,
    ) -> bool {
        self.clip_voices.len() < voices && self.play_clip(rate, name, bus, gain, looping)
    }

    fn mix_next_sample(&mut self) -> f32 {
        let mut out = 0.0f32;
        for voice in &mut self.voices {
            if voice.samples_left > 0 {
                out += voice.phase.sin() * voice.gain;
                voice.phase += voice.phase_inc;
                if voice.phase > TAU {
                    voice.phase -= TAU;
                }
                voice.samples_left -= 1;
            }
        }
        let bus_volumes = &self.bus_volumes;
        for voice in &mut self.clip_voices {
            let bus_volume = bus_volumes.get(&voice.bus).copied().unwrap_or(1.0);
            if voice.looping && !voice.samples.is_empty() {
                voice.pos %= voice.samples.len() as f64;
            }
            let i0 = voice.pos.floor() as usize;
            let i1 = i0.saturating_add(1);
            if i0 < voice.samples.len() {
                let s0 = voice.samples[i0];
                let s1 = if i1 < voice.samples.len() {
                    voice.samples[i1]
                } else if voice.looping {
                    voice.samples[0]
                } else {
                    s0
                };
                let frac = (voice.pos - i0 as f64) as f32;
                out += (s0 + (s1 - s0) * frac) * voice.gain * bus_volume;
                voice.pos += voice.step;
            }
        }
        self.voices.retain(|v| v.samples_left > 0);
        self.clip_voices
            .retain(|v| v.looping || (v.pos.floor() as usize) < v.samples.len());
        (out * self.master_volume).clamp(-1.0, 1.0)
    }
}

pub struct AudioEngine {
    stream: cpal::Stream,
    mixer: Arc<Mutex<MixerState>>,
    output: OutputInfo,
    healthy: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        Self::with_config(AudioConfig::default())
    }

    pub fn with_config(config: AudioConfig) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?;
        let default_cfg = device
            .default_output_config()
            .map_err(AudioError::DefaultOutputConfig)?;

        let mut stream_config = default_cfg.config();
        if let Some(sample_rate) = config.sample_rate {
            stream_config.sample_rate = sample_rate.max(1);
        }
        if let Some(channels) = config.channels {
            stream_config.channels = channels.max(1);
        }

        let output = OutputInfo {
            device_name: device
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_else(|_| "unknown output".to_string()),
            sample_rate: stream_config.sample_rate,
            channels: stream_config.channels,
        };

        let mixer = Arc::new(Mutex::new(MixerState::new(config.master_volume)));
        let healthy = Arc::new(AtomicBool::new(true));
        let stream = build_stream(
            &device,
            &stream_config,
            default_cfg.sample_format(),
            mixer.clone(),
            healthy.clone(),
        )
        .map_err(AudioError::BuildStream)?;
        stream.play().map_err(AudioError::PlayStream)?;

        Ok(Self {
            stream,
            mixer,
            output,
            healthy,
        })
    }

    pub fn output_info(&self) -> &OutputInfo {
        &self.output
    }
    pub fn is_healthy(&self) -> bool {
        let healthy = self.healthy.load(Ordering::Relaxed) && !self.mixer.is_poisoned();
        #[cfg(target_arch = "wasm32")]
        return healthy
            && self
                .web_context()
                .is_some_and(|ctx| ctx.state() != web_sys::AudioContextState::Closed);
        #[cfg(not(target_arch = "wasm32"))]
        healthy
    }
    pub fn is_ready(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        return self.is_healthy()
            && self
                .web_context()
                .is_some_and(|ctx| ctx.state() == web_sys::AudioContextState::Running);
        #[cfg(not(target_arch = "wasm32"))]
        self.is_healthy()
    }
    #[cfg(target_arch = "wasm32")]
    fn web_context(&self) -> Option<&web_sys::AudioContext> {
        // CPAL adds enum variants when optional host features are enabled.
        #[allow(unreachable_patterns)]
        match self.stream.as_inner() {
            cpal::platform::StreamInner::WebAudio(stream) => Some(stream.audio_context()),
            _ => None,
        }
    }
    /// Resume the existing browser context in a user gesture. CPAL's play()
    /// bootstraps buffer callbacks and must not be called again for each gesture.
    pub fn unlock(&self) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        if let Some(ctx) = self.web_context() {
            let promise = ctx.resume().map_err(|e| format!("{e:?}"))?;
            wasm_bindgen_futures::spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            });
        }
        Ok(())
    }
    pub fn set_suspended(&self, suspended: bool) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        if let Some(ctx) = self.web_context() {
            if suspended {
                let promise = ctx.suspend().map_err(|e| format!("{e:?}"))?;
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                });
            } else {
                let promise = ctx.resume().map_err(|e| format!("{e:?}"))?;
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                });
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if suspended {
            self.stream.pause().map_err(|e| e.to_string())?;
        } else {
            self.stream.play().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    /// Load only if cached and actively playing unique PCM buffers fit the
    /// budget. Active voices pin their old data across cache removal/replacement.
    pub fn load_clip_bounded(
        &self,
        name: &str,
        bytes: &[u8],
        budget: usize,
    ) -> Result<(), AudioError> {
        let occupied = self
            .mixer
            .lock()
            .map_err(|_| AudioError::Decode("audio mixer is unavailable".into()))?
            .pcm_bytes_without_cache(name);
        let clip = decode_clip_limited(
            bytes,
            budget.saturating_sub(occupied) / std::mem::size_of::<f32>(),
        )?;
        self.install_prepared_clip(name, &clip, budget)
    }
    /// Publish immutable CPU data. No codec, synthesis or file IO runs here.
    pub fn install_prepared_clip(
        &self,
        name: &str,
        clip: &PreparedClip,
        budget: usize,
    ) -> Result<(), AudioError> {
        let mut mixer = self
            .mixer
            .lock()
            .map_err(|_| AudioError::Decode("audio mixer is unavailable".into()))?;
        if mixer
            .pcm_bytes_without_cache(name)
            .saturating_add(clip.samples.len().saturating_mul(4))
            > budget
        {
            return Err(AudioError::PcmBudget);
        }
        mixer.insert_clip(name.to_owned(), clip.clone());
        Ok(())
    }
    pub fn remove_cached_clip(&self, name: &str) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.clips.remove(name);
        }
    }
    pub fn reset_project(&self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            let next = mixer.next_id;
            let mut fresh = MixerState::new(mixer.master_volume);
            fresh.next_id = next;
            *mixer = fresh;
        }
    }
    pub fn stop_looping_clip(&self, name: &str) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer
                .clip_voices
                .retain(|voice| !voice.looping || voice.name != name);
        }
    }
    pub fn play_bounded(
        &self,
        name: &str,
        bus: &str,
        gain: f32,
        looping: bool,
        voices: usize,
    ) -> bool {
        self.mixer.lock().is_ok_and(|mut mixer| {
            mixer.play_clip_bounded(self.output.sample_rate, name, bus, gain, looping, voices)
        })
    }

    pub fn set_master_volume(&self, volume: f32) {
        if let Ok(mut m) = self.mixer.lock() {
            m.set_master_volume(volume);
        }
    }

    pub fn play_sine(&self, hz: f32, duration_seconds: f32, gain: f32) -> Option<SineVoiceId> {
        if let Ok(mut m) = self.mixer.lock() {
            return Some(m.add_sine_voice(self.output.sample_rate, hz, duration_seconds, gain));
        }
        None
    }

    pub fn stop_voice(&self, id: SineVoiceId) {
        if let Ok(mut m) = self.mixer.lock() {
            m.stop_voice(id);
        }
    }

    pub fn clear_clips(&self) {
        if let Ok(mut m) = self.mixer.lock() {
            m.clear_clips();
        }
    }

    pub fn load_clip_from_bytes(&self, name: &str, bytes: &[u8]) -> Result<(), AudioError> {
        let clip = decode_clip(bytes)?;
        if let Ok(mut m) = self.mixer.lock() {
            m.insert_clip(name.to_string(), clip);
        }
        Ok(())
    }

    pub fn play_one_shot(&self, name: &str, gain: f32) -> bool {
        if let Ok(mut m) = self.mixer.lock() {
            return m.play_clip(self.output.sample_rate, name, "sfx", gain, false);
        }
        false
    }

    pub fn play_on_bus(&self, name: &str, bus: &str, gain: f32, looping: bool) -> bool {
        if let Ok(mut m) = self.mixer.lock() {
            return m.play_clip(self.output.sample_rate, name, bus, gain, looping);
        }
        false
    }

    pub fn set_bus_volume(&self, bus: &str, volume: f32) {
        if let Ok(mut m) = self.mixer.lock() {
            m.set_bus_volume(bus, volume);
        }
    }

    pub fn bus_volume(&self, bus: &str) -> f32 {
        if let Ok(m) = self.mixer.lock() {
            return m.get_bus_volume(bus);
        }
        1.0
    }

    pub fn clear_bus(&self, bus: &str) {
        if let Ok(mut m) = self.mixer.lock() {
            m.clear_bus(bus);
        }
    }

    pub fn clear_all_buses(&self) {
        if let Ok(mut m) = self.mixer.lock() {
            m.clear_all_buses();
        }
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    mixer: Arc<Mutex<MixerState>>,
    healthy: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let err_fn = move |err| {
        if healthy.swap(false, Ordering::Relaxed) {
            eprintln!("[audio] output stream error: {err}");
        }
    };

    match sample_format {
        cpal::SampleFormat::F32 => build_stream_t::<f32>(device, config, mixer, err_fn),
        cpal::SampleFormat::I16 => build_stream_t::<i16>(device, config, mixer, err_fn),
        cpal::SampleFormat::U16 => build_stream_t::<u16>(device, config, mixer, err_fn),
        _ => build_stream_t::<f32>(device, config, mixer, err_fn),
    }
}

fn build_stream_t<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mixer: Arc<Mutex<MixerState>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            if let Ok(mut m) = mixer.lock() {
                for frame in data.chunks_mut(channels) {
                    let v: T = T::from_sample(m.mix_next_sample());
                    for sample in frame {
                        *sample = v;
                    }
                }
            } else {
                let zero: T = T::from_sample(0.0);
                for sample in data {
                    *sample = zero;
                }
            }
        },
        err_fn,
        None,
    )
}

fn decode_clip(bytes: &[u8]) -> Result<DecodedClip, AudioError> {
    decode_clip_limited(bytes, usize::MAX)
}
fn decode_clip_limited(bytes: &[u8], samples: usize) -> Result<DecodedClip, AudioError> {
    decode_clip_cancellable(bytes, samples, None)
}
fn decode_clip_cancellable(
    bytes: &[u8],
    samples: usize,
    cancelled: Option<&AtomicBool>,
) -> Result<DecodedClip, AudioError> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return decode_wav(bytes, samples, cancelled);
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
        return decode_ogg(bytes, samples, cancelled);
    }
    decode_wav(bytes, samples, cancelled).or_else(|_| decode_ogg(bytes, samples, cancelled))
}
struct MonoPcm<'a> {
    cancelled: Option<&'a AtomicBool>,
    samples: Vec<f32>,
    channels: usize,
    sum: f32,
    count: usize,
    limit: usize,
}
impl<'a> MonoPcm<'a> {
    fn new(channels: usize, limit: usize, cancelled: Option<&'a AtomicBool>) -> Self {
        Self {
            cancelled,
            samples: Vec::new(),
            channels: channels.max(1),
            sum: 0.,
            count: 0,
            limit,
        }
    }
    fn push(&mut self, sample: f32) -> Result<(), AudioError> {
        if self.samples.len() % 1024 == 0
            && self.cancelled.is_some_and(|c| c.load(Ordering::Relaxed))
        {
            return Err(AudioError::Cancelled);
        }
        if !sample.is_finite() {
            return Err(AudioError::Decode("non-finite audio sample".into()));
        }
        self.sum += sample;
        self.count += 1;
        if self.count == self.channels {
            self.flush()?;
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), AudioError> {
        if self.samples.len() >= self.limit {
            return Err(AudioError::PcmBudget);
        }
        if self.samples.len() == self.samples.capacity() {
            let extra = self
                .samples
                .capacity()
                .max(4096)
                .min(self.limit - self.samples.len());
            self.samples
                .try_reserve_exact(extra)
                .map_err(|_| AudioError::Decode("unable to allocate audio PCM".into()))?;
        }
        self.samples.push(self.sum / self.channels as f32);
        self.sum = 0.;
        self.count = 0;
        Ok(())
    }
    fn finish(mut self, rate: u32) -> Result<DecodedClip, AudioError> {
        if self.count > 0 {
            self.flush()?;
        }
        Ok(DecodedClip {
            sample_rate: rate.max(1),
            samples: self.samples.into(),
        })
    }
}
fn decode_wav(
    bytes: &[u8],
    limit: usize,
    cancelled: Option<&AtomicBool>,
) -> Result<DecodedClip, AudioError> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes))
        .map_err(|e| AudioError::Decode(format!("wav reader error: {e}")))?;
    let spec = reader.spec();
    let mut mono = MonoPcm::new(spec.channels as usize, limit, cancelled);
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                mono.push(
                    sample
                        .map_err(|e| AudioError::Decode(format!("wav float sample error: {e}")))?,
                )?;
            }
        }
        hound::SampleFormat::Int => {
            let denominator = (1_i64 << (spec.bits_per_sample.clamp(1, 32) - 1)) as f32;
            for sample in reader.samples::<i32>() {
                mono.push(
                    (sample.map_err(|e| AudioError::Decode(format!("wav int sample error: {e}")))?
                        as f32
                        / denominator)
                        .clamp(-1., 1.),
                )?;
            }
        }
    }
    mono.finish(spec.sample_rate)
}
fn decode_ogg(
    bytes: &[u8],
    limit: usize,
    cancelled: Option<&AtomicBool>,
) -> Result<DecodedClip, AudioError> {
    let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes))
        .map_err(|e| AudioError::Decode(format!("ogg reader error: {e}")))?;
    let rate = reader.ident_hdr.audio_sample_rate;
    let mut mono = MonoPcm::new(reader.ident_hdr.audio_channels as usize, limit, cancelled);
    while let Some(packet) = reader
        .read_dec_packet_itl()
        .map_err(|e| AudioError::Decode(format!("ogg packet decode error: {e}")))?
    {
        for sample in packet {
            mono.push(sample as f32 / 32768.)?;
        }
    }
    mono.finish(rate)
}

#[derive(Clone, Copy, Hash)]
enum FxWave {
    Sine,
    Square,
    Saw,
    Triangle,
    Noise,
}

impl FxWave {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "square" => Self::Square,
            "saw" | "sawtooth" => Self::Saw,
            "triangle" => Self::Triangle,
            "noise" => Self::Noise,
            _ => Self::Sine,
        }
    }
}

#[derive(Clone)]
struct AudioFxDefinition {
    wave: FxWave,
    duration: f32,
    attack: f32,
    decay: f32,
    sustain_level: f32,
    release: f32,
    gain: f32,
    freq: f32,
    freq_end: f32,
    noise: f32,
    lowpass: f32,
    repeat: usize,
    repeat_gap: f32,
    tremolo_depth: f32,
    tremolo_freq: f32,
}

impl Default for AudioFxDefinition {
    fn default() -> Self {
        Self {
            wave: FxWave::Sine,
            duration: 0.18,
            attack: 0.001,
            decay: 0.12,
            sustain_level: 0.0,
            release: 0.02,
            gain: 0.35,
            freq: 880.0,
            freq_end: 220.0,
            noise: 0.0,
            lowpass: 0.0,
            repeat: 1,
            repeat_gap: 0.0,
            tremolo_depth: 0.0,
            tremolo_freq: 0.0,
        }
    }
}

fn table_float(table: &toml::value::Table, key: &str, default: f32) -> f32 {
    table
        .get(key)
        .and_then(|v| v.as_float().map(|f| f as f32))
        .or_else(|| {
            table
                .get(key)
                .and_then(|v| v.as_integer().map(|i| i as f32))
        })
        .unwrap_or(default)
}

fn table_usize(table: &toml::value::Table, key: &str, default: usize) -> usize {
    table
        .get(key)
        .and_then(|v| v.as_integer())
        .map(|v| v.max(1) as usize)
        .unwrap_or(default)
}

fn parse_audio_fx_definition(table: &toml::value::Table) -> AudioFxDefinition {
    let mut fx = AudioFxDefinition::default();
    if let Some(wave) = table.get("wave").and_then(toml::Value::as_str) {
        fx.wave = FxWave::parse(wave);
    }
    fx.duration = table_float(table, "duration", fx.duration).max(0.01);
    fx.attack = table_float(table, "attack", fx.attack).max(0.0);
    fx.decay = table_float(table, "decay", fx.decay).max(0.0);
    fx.sustain_level = table_float(table, "sustain_level", fx.sustain_level).clamp(0.0, 1.0);
    fx.release = table_float(table, "release", fx.release).max(0.0);
    fx.gain = table_float(table, "gain", fx.gain).clamp(0.0, 1.5);
    fx.freq = table_float(table, "freq", fx.freq).max(1.0);
    fx.freq_end = table_float(table, "freq_end", fx.freq_end).max(1.0);
    fx.noise = table_float(table, "noise", fx.noise).clamp(0.0, 1.0);
    fx.lowpass = table_float(table, "lowpass", fx.lowpass).clamp(0.0, 1.0);
    fx.repeat = table_usize(table, "repeat", fx.repeat).max(1);
    fx.repeat_gap = table_float(table, "repeat_gap", fx.repeat_gap).max(0.0);
    fx.tremolo_depth = table_float(table, "tremolo_depth", fx.tremolo_depth).clamp(0.0, 1.0);
    fx.tremolo_freq = table_float(table, "tremolo_freq", fx.tremolo_freq).max(0.0);
    fx
}

fn parse_audio_fx_table(src: &str) -> Result<toml::value::Table, String> {
    src.parse::<toml::Table>()
        .map_err(|err| format!("audio fx TOML parse error: {err}"))
}

fn find_audio_fx_table<'a>(
    root: &'a toml::value::Table,
    name: &str,
) -> Option<&'a toml::value::Table> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(sfx) = root.get("sfx").and_then(toml::Value::as_table) {
        if let Some(table) = sfx.get(trimmed).and_then(toml::Value::as_table) {
            return Some(table);
        }
        if let Some(stripped) = trimmed.strip_prefix("sfx.") {
            return sfx.get(stripped).and_then(toml::Value::as_table);
        }
    }

    root.get(trimmed).and_then(toml::Value::as_table)
}

pub fn list_audio_fx_names(src: &str) -> Vec<String> {
    let Ok(root) = parse_audio_fx_table(src) else {
        return vec![];
    };

    if let Some(sfx) = root.get("sfx").and_then(toml::Value::as_table) {
        let mut out: Vec<String> = sfx
            .iter()
            .filter_map(|(key, value)| value.as_table().map(|_| key.clone()))
            .collect();
        out.sort();
        return out;
    }

    let mut out: Vec<String> = root
        .iter()
        .filter_map(|(key, value)| value.as_table().map(|_| key.clone()))
        .collect();
    out.sort();
    out
}

/// Normalize once at CPU source publication. Jobs need neither a source-document
/// copy nor TOML parsing; fingerprints ignore unrelated effects and metadata.
fn audio_fx_definitions(src: &str) -> rustc_hash::FxHashMap<String, preparation::AudioEffect> {
    let Ok(root) = parse_audio_fx_table(src) else {
        return Default::default();
    };
    let tables = root
        .get("sfx")
        .and_then(toml::Value::as_table)
        .unwrap_or(&root);
    tables
        .iter()
        .filter_map(|(name, value)| {
            Some((
                name.clone(),
                preparation::AudioEffect(parse_audio_fx_definition(value.as_table()?)),
            ))
        })
        .collect()
}

fn sample_wave(wave: FxWave, phase: f32) -> f32 {
    match wave {
        FxWave::Sine => phase.sin(),
        FxWave::Square => {
            if phase.sin() >= 0.0 {
                1.0
            } else {
                -1.0
            }
        }
        FxWave::Saw => 1.0 - 2.0 * (phase / TAU),
        FxWave::Triangle => 2.0 * (2.0 * (phase / TAU) - 1.0).abs() - 1.0,
        FxWave::Noise => 0.0,
    }
}

fn envelope_gain(fx: &AudioFxDefinition, t: f32) -> f32 {
    let duration = fx.duration.max(0.001);
    let attack = fx.attack.min(duration);
    let decay = fx.decay.min((duration - attack).max(0.0));
    let release = fx.release.min(duration);
    let sustain_start = attack + decay;
    let sustain_end = (duration - release).max(sustain_start);

    if attack > 0.0 && t < attack {
        return (t / attack).clamp(0.0, 1.0);
    }
    if decay > 0.0 && t < sustain_start {
        let p = ((t - attack) / decay).clamp(0.0, 1.0);
        return 1.0 + (fx.sustain_level - 1.0) * p;
    }
    if t < sustain_end {
        return fx.sustain_level;
    }
    if release > 0.0 && t < duration {
        let p = ((t - sustain_end) / release).clamp(0.0, 1.0);
        return fx.sustain_level * (1.0 - p);
    }
    0.0
}

fn synthesize_audio_fx_samples(
    fx: &AudioFxDefinition,
    sample_rate: u32,
    cancelled: Option<&AtomicBool>,
) -> Result<Vec<f32>, AudioError> {
    let sample_rate_f = sample_rate.max(1) as f32;
    let single_samples = (fx.duration.max(0.01) * sample_rate_f).ceil() as usize;
    let gap_samples = (fx.repeat_gap.max(0.0) * sample_rate_f).ceil() as usize;
    single_samples
        .checked_add(gap_samples)
        .ok_or(AudioError::PcmBudget)?;
    let total_samples = single_samples
        .checked_mul(fx.repeat.max(1))
        .and_then(|s| {
            gap_samples
                .checked_mul(fx.repeat.saturating_sub(1))
                .and_then(|g| s.checked_add(g))
        })
        .ok_or(AudioError::PcmBudget)?;
    if cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(AudioError::Cancelled);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(total_samples.max(1))
        .map_err(|_| AudioError::PcmBudget)?;
    out.resize(total_samples.max(1), 0.0f32);

    let mut rng = 0x1234_5678u32;
    let mut global_low = 0.0f32;

    for rep in 0..fx.repeat.max(1) {
        let base = rep * (single_samples + gap_samples);
        let mut phase = 0.0f32;

        for i in 0..single_samples {
            if i % 1024 == 0 && cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return Err(AudioError::Cancelled);
            }
            let t = i as f32 / sample_rate_f;
            let p = if single_samples > 1 {
                i as f32 / (single_samples - 1) as f32
            } else {
                0.0
            };
            let freq = fx.freq + (fx.freq_end - fx.freq) * p;
            phase = (phase + TAU * freq.max(1.0) / sample_rate_f) % TAU;

            let tonal = if matches!(fx.wave, FxWave::Noise) {
                0.0
            } else {
                sample_wave(fx.wave, phase)
            };

            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            let noise = ((rng as f32 / u32::MAX as f32) * 2.0) - 1.0;

            let tone_mix = if matches!(fx.wave, FxWave::Noise) {
                0.0
            } else {
                1.0 - fx.noise
            };
            let mut sample = tonal * tone_mix + noise * fx.noise;

            let env = envelope_gain(fx, t);
            if fx.tremolo_depth > 0.0 && fx.tremolo_freq > 0.0 {
                let trem = ((t * fx.tremolo_freq * TAU).sin() * 0.5 + 0.5) * fx.tremolo_depth
                    + (1.0 - fx.tremolo_depth);
                sample *= trem;
            }
            sample *= env * fx.gain;

            if fx.lowpass > 0.0 {
                let alpha = (1.0 - fx.lowpass * 0.95).clamp(0.01, 1.0);
                global_low += (sample - global_low) * alpha;
                sample = global_low;
            }

            out[base + i] = sample.clamp(-1.0, 1.0);
        }
    }

    Ok(out)
}

fn encode_wav_from_mono_f32(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|err| format!("wav writer error: {err}"))?;
        for sample in samples {
            let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(s)
                .map_err(|err| format!("wav write error: {err}"))?;
        }
        writer
            .finalize()
            .map_err(|err| format!("wav finalize error: {err}"))?;
    }
    Ok(cursor.into_inner())
}

pub fn synthesize_audio_fx_wav(src: &str, name: &str) -> Result<Vec<u8>, String> {
    let root = parse_audio_fx_table(src)?;
    let table =
        find_audio_fx_table(&root, name).ok_or_else(|| format!("audio fx '{name}' not found"))?;
    let fx = parse_audio_fx_definition(table);
    let samples = synthesize_audio_fx_samples(&fx, 44_100, None).map_err(|e| e.to_string())?;
    encode_wav_from_mono_f32(&samples, 44_100)
}
/// Guard generated PCM before the synthesizer allocates its output buffer.
pub fn synthesize_audio_fx_wav_bounded(
    src: &str,
    name: &str,
    pcm_bytes: usize,
) -> Result<Vec<u8>, AudioError> {
    let root = parse_audio_fx_table(src).map_err(AudioError::Decode)?;
    let table = find_audio_fx_table(&root, name)
        .ok_or_else(|| AudioError::Decode(format!("audio fx '{name}' not found")))?;
    let fx = parse_audio_fx_definition(table);
    if audio_fx_sample_count(&fx)?.max(1) > pcm_bytes / 4 {
        return Err(AudioError::PcmBudget);
    }
    encode_wav_from_mono_f32(&synthesize_audio_fx_samples(&fx, 44100, None)?, 44100)
        .map_err(AudioError::Decode)
}
fn audio_fx_sample_count(fx: &AudioFxDefinition) -> Result<usize, AudioError> {
    let single = (fx.duration.max(0.01) * 44100.).ceil() as usize;
    let gap = (fx.repeat_gap.max(0.) * 44100.).ceil() as usize;
    single.checked_add(gap).ok_or(AudioError::PcmBudget)?;
    single
        .checked_mul(fx.repeat.max(1))
        .and_then(|s| {
            gap.checked_mul(fx.repeat.saturating_sub(1))
                .and_then(|g| s.checked_add(g))
        })
        .ok_or(AudioError::PcmBudget)
}

#[cfg(test)]
mod tests;
