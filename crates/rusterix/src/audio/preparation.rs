//! Immutable CPU clips and bounded, cancellable preparation. Devices and windows
//! stay on their host thread; only source data and completed PCM cross this API.
use super::*;

#[derive(Debug, Clone)]
pub struct PreparedClip {
    pub(super) sample_rate: u32,
    pub(super) samples: Arc<[f32]>,
}
impl PreparedClip {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
    pub fn pcm_bytes(&self) -> usize {
        self.samples.len().saturating_mul(4)
    }
    pub fn decode(bytes: &[u8], pcm_bytes: usize) -> Result<Self, AudioError> {
        decode_clip_limited(bytes, pcm_bytes / 4)
    }
}

/// Normalized CPU playback definition, independent of its project/source text.
#[derive(Clone)]
pub struct AudioEffect(pub(super) AudioFxDefinition);
impl AudioEffect {
    pub fn parse(source: &str, name: &str) -> Result<Self, AudioError> {
        let root = parse_audio_fx_table(source).map_err(AudioError::Decode)?;
        let table = find_audio_fx_table(&root, name)
            .ok_or_else(|| AudioError::Decode(format!("audio fx '{name}' not found")))?;
        Ok(Self(parse_audio_fx_definition(table)))
    }
    pub(super) fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let fx = &self.0;
        let mut hash = rustc_hash::FxHasher::default();
        fx.wave.hash(&mut hash);
        fx.repeat.hash(&mut hash);
        [
            fx.duration,
            fx.attack,
            fx.decay,
            fx.sustain_level,
            fx.release,
            fx.gain,
            fx.freq,
            fx.freq_end,
            fx.noise,
            fx.lowpass,
            fx.repeat_gap,
            fx.tremolo_depth,
            fx.tremolo_freq,
        ]
        .map(f32::to_bits)
        .hash(&mut hash);
        hash.finish()
    }
}
pub enum AudioSource {
    Encoded(Arc<[u8]>),
    Generated(AudioEffect),
}
pub struct PreparationJob {
    pub id: u64,
    pub source: AudioSource,
    pub pcm_bytes: usize,
    pub generated_pcm_bytes: usize,
    pub cancelled: Arc<AtomicBool>,
}
pub struct PreparationResult {
    pub id: u64,
    pub clip: Result<PreparedClip, AudioError>,
}
impl PreparationJob {
    /// May run on a CPU worker. The token is checked during decoding/synthesis,
    /// not just after allocating and processing an entire long sound.
    pub fn run(self) -> PreparationResult {
        let clip = (|| {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(AudioError::Cancelled);
            }
            match self.source {
                AudioSource::Encoded(bytes) => {
                    decode_clip_cancellable(&bytes, self.pcm_bytes / 4, Some(&self.cancelled))
                }
                AudioSource::Generated(effect) => {
                    let fx = effect.0;
                    let limit = self.pcm_bytes.min(self.generated_pcm_bytes);
                    if audio_fx_sample_count(&fx)?.max(1) > limit / 4 {
                        return Err(AudioError::PcmBudget);
                    }
                    let mut samples =
                        synthesize_audio_fx_samples(&fx, 44100, Some(&self.cancelled))?;
                    // Preserve the reference FX's 16-bit WAV quantization without
                    // allocating or decoding an intermediate WAV on the UI path.
                    for chunk in samples.chunks_mut(1024) {
                        if self.cancelled.load(Ordering::Relaxed) {
                            return Err(AudioError::Cancelled);
                        }
                        for sample in chunk {
                            *sample = ((*sample * i16::MAX as f32) as i16) as f32 / 32768.;
                        }
                    }
                    Ok(PreparedClip {
                        sample_rate: 44100,
                        samples: samples.into(),
                    })
                }
            }
        })();
        PreparationResult { id: self.id, clip }
    }
}
/// One in-flight job per owner bounds temporary decoded PCM and source storage.
/// Completion is polled from application update, never from painting.
pub trait AudioPreparer {
    fn is_idle(&self) -> bool;
    fn start(&mut self, job: PreparationJob) -> Result<(), AudioError>;
    fn poll(&mut self) -> Option<PreparationResult>;
    fn cancel(&mut self, id: u64);
}

/// Deterministic CPU/reference adapter. Browser worker hosting remains a separate
/// platform integration; this explicit fallback does not claim nonblocking WASM.
#[derive(Default)]
pub struct InlinePreparation {
    completed: Option<PreparationResult>,
}
impl AudioPreparer for InlinePreparation {
    fn is_idle(&self) -> bool {
        self.completed.is_none()
    }
    fn start(&mut self, job: PreparationJob) -> Result<(), AudioError> {
        if !self.is_idle() {
            return Err(AudioError::PreparationUnavailable(
                "audio preparer is busy".into(),
            ));
        }
        self.completed = Some(job.run());
        Ok(())
    }
    fn poll(&mut self) -> Option<PreparationResult> {
        self.completed.take()
    }
    fn cancel(&mut self, id: u64) {
        if self.completed.as_ref().is_some_and(|r| r.id == id) {
            self.completed = None;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub struct ThreadPreparation {
    channel: Option<(
        std::sync::mpsc::SyncSender<PreparationJob>,
        std::sync::mpsc::Receiver<PreparationResult>,
    )>,
    pending: Option<(u64, Arc<AtomicBool>)>,
}
#[cfg(not(target_arch = "wasm32"))]
impl AudioPreparer for ThreadPreparation {
    fn is_idle(&self) -> bool {
        self.pending.is_none()
    }
    fn start(&mut self, job: PreparationJob) -> Result<(), AudioError> {
        if !self.is_idle() {
            return Err(AudioError::PreparationUnavailable(
                "audio preparer is busy".into(),
            ));
        }
        if self.channel.is_none() {
            let (send, receive) = std::sync::mpsc::sync_channel::<PreparationJob>(1);
            let (done, results) = std::sync::mpsc::sync_channel(1);
            std::thread::Builder::new()
                .name("eldiron-audio-prepare".into())
                .spawn(move || {
                    while let Ok(job) = receive.recv() {
                        if done.send(job.run()).is_err() {
                            break;
                        }
                    }
                })
                .map_err(|e| AudioError::PreparationUnavailable(format!("audio worker: {e}")))?;
            self.channel = Some((send, results));
        }
        let pending = (job.id, job.cancelled.clone());
        if let Err(error) = self.channel.as_ref().unwrap().0.try_send(job) {
            self.channel = None;
            return Err(AudioError::Decode(format!(
                "audio worker submission: {error}"
            )));
        }
        self.pending = Some(pending);
        Ok(())
    }
    fn poll(&mut self) -> Option<PreparationResult> {
        let id = self.pending.as_ref()?.0;
        match self.channel.as_ref()?.1.try_recv() {
            Ok(result) => {
                self.pending = None;
                Some(result)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.channel = None;
                Some(PreparationResult {
                    id,
                    clip: Err(AudioError::PreparationUnavailable(
                        "audio worker disconnected".into(),
                    )),
                })
            }
        }
    }
    fn cancel(&mut self, id: u64) {
        if let Some((pending, cancelled)) = &self.pending {
            if *pending == id {
                cancelled.store(true, Ordering::Relaxed);
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Drop for ThreadPreparation {
    fn drop(&mut self) {
        if let Some((_, cancelled)) = &self.pending {
            cancelled.store(true, Ordering::Relaxed);
        }
        // Dropping both channel endpoints wakes the detached worker; never join
        // an in-progress codec from a window/document close callback.
    }
}
pub(super) fn platform_preparer() -> Box<dyn AudioPreparer> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(ThreadPreparation::default())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(InlinePreparation::default())
    }
}

#[cfg(test)]
#[path = "preparation_tests.rs"]
mod tests;
