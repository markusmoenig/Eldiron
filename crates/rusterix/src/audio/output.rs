//! Application-owned audio. The caller publishes source revisions and delivers
//! each command batch once. No server, window, GPU, renderer clock or system
//! timer is owned here. Device recovery restores loops, never old one-shots.
use super::*;
use crate::{Assets, server::message::AudioCommand};
use rustc_hash::{FxHashMap, FxHasher};
use std::{
    collections::{BTreeMap, VecDeque},
    hash::{Hash, Hasher},
    time::Duration,
};

#[derive(Clone, Copy)]
pub struct AudioLimits {
    pub pcm_bytes: usize,
    pub generated_pcm_bytes: usize,
    pub cached_clips: usize,
    pub voices: usize,
    pub looping_voices: usize,
    pub buses: usize,
    pub retry_interval: Duration,
}
impl Default for AudioLimits {
    fn default() -> Self {
        Self {
            pcm_bytes: 256 * 1024 * 1024,
            generated_pcm_bytes: 16 * 1024 * 1024,
            cached_clips: 128,
            voices: 128,
            looping_voices: 64,
            buses: 64,
            retry_interval: Duration::from_secs(1),
        }
    }
}
/// Implementations own playback/decoded PCM. Factory construction is lazy, and
/// `ready` can remain false until a browser user gesture unlocks its context.
pub trait AudioDevice {
    fn healthy(&self) -> bool;
    fn ready(&self) -> bool;
    fn unlock(&mut self) -> Result<(), String>;
    fn set_suspended(&mut self, suspended: bool) -> Result<(), String>;
    fn load(&mut self, name: &str, bytes: &[u8], budget: usize) -> Result<(), AudioError>;
    fn unload(&mut self, name: &str);
    fn stop_loops(&mut self, name: &str);
    fn reset(&mut self);
    fn command(&mut self, command: &AudioCommand, voices: usize) -> bool;
}
impl AudioDevice for AudioEngine {
    fn healthy(&self) -> bool {
        self.is_healthy()
    }
    fn ready(&self) -> bool {
        self.is_ready()
    }
    fn unlock(&mut self) -> Result<(), String> {
        AudioEngine::unlock(self)
    }
    fn set_suspended(&mut self, suspended: bool) -> Result<(), String> {
        AudioEngine::set_suspended(self, suspended)
    }
    fn load(&mut self, name: &str, bytes: &[u8], budget: usize) -> Result<(), AudioError> {
        self.load_clip_bounded(name, bytes, budget)
    }
    fn unload(&mut self, name: &str) {
        self.remove_cached_clip(name);
    }
    fn stop_loops(&mut self, name: &str) {
        self.stop_looping_clip(name);
    }
    fn reset(&mut self) {
        self.reset_project();
    }
    fn command(&mut self, command: &AudioCommand, voices: usize) -> bool {
        match command {
            AudioCommand::Play {
                name,
                bus,
                gain,
                looping,
            } => self.play_bounded(name, bus, *gain, *looping, voices),
            AudioCommand::ClearBus { bus } => {
                self.clear_bus(bus);
                true
            }
            AudioCommand::ClearAll => {
                self.clear_all_buses();
                true
            }
            AudioCommand::SetBusVolume { bus, volume } => {
                self.set_bus_volume(bus, *volume);
                true
            }
        }
    }
}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioOutputStats {
    pub delivered_commands: u64,
    pub device_attempts: u64,
    pub devices_created: u64,
    pub clip_loads: u64,
    pub plays: u64,
    pub dropped_one_shots: u64,
    pub rejected_commands: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClipState {
    Loaded,
    Invalid,
    Budget,
}
struct CachedClip {
    stamp: Option<u64>,
    state: ClipState,
    used: u64,
    retry: Duration,
}
struct Loop {
    command: AudioCommand,
    started: bool,
}
pub struct AudioOutput<D: AudioDevice = AudioEngine> {
    factory: Box<dyn FnMut() -> Result<D, String>>,
    device: Option<D>,
    limits: AudioLimits,
    now: Duration,
    next_attempt: Duration,
    requested: bool,
    suspended: bool,
    revision: Option<u64>,
    effects: FxHashMap<String, u64>,
    cache: FxHashMap<String, CachedClip>,
    serial: u64,
    loops: Vec<Loop>,
    volumes: BTreeMap<String, f32>,
    diagnostics: VecDeque<String>,
    last_error: Option<String>,
    stats: AudioOutputStats,
}
impl Default for AudioOutput<AudioEngine> {
    fn default() -> Self {
        Self::new(AudioLimits::default(), || {
            AudioEngine::new().map_err(|e| e.to_string())
        })
    }
}
impl<D: AudioDevice> AudioOutput<D> {
    pub fn new(limits: AudioLimits, factory: impl FnMut() -> Result<D, String> + 'static) -> Self {
        Self {
            factory: Box::new(factory),
            device: None,
            limits,
            now: Duration::ZERO,
            next_attempt: Duration::ZERO,
            requested: false,
            suspended: false,
            revision: None,
            effects: FxHashMap::default(),
            cache: FxHashMap::default(),
            serial: 0,
            loops: Vec::new(),
            volumes: BTreeMap::new(),
            diagnostics: VecDeque::new(),
            last_error: None,
            stats: AudioOutputStats::default(),
        }
    }
    pub fn stats(&self) -> AudioOutputStats {
        self.stats
    }
    pub fn cached_clips(&self) -> usize {
        self.cache.len()
    }
    pub fn desired_loops(&self) -> usize {
        self.loops.len()
    }
    pub fn take_diagnostics(&mut self) -> Vec<String> {
        self.diagnostics.drain(..).collect()
    }
    fn report(&mut self, error: String) {
        if self.last_error.as_ref() == Some(&error) {
            return;
        }
        self.last_error = Some(error.clone());
        if self.diagnostics.len() == 16 {
            self.diagnostics.pop_front();
        }
        self.diagnostics.push_back(error);
    }
    fn stamp(&self, assets: &Assets, name: &str) -> Option<u64> {
        let mut hash = FxHasher::default();
        if let Some(effect) = self.effects.get(name) {
            1u8.hash(&mut hash);
            effect.hash(&mut hash);
        } else {
            0u8.hash(&mut hash);
            assets.audio.get(name)?.hash(&mut hash);
        }
        Some(hash.finish())
    }
    /// A revision belongs to CPU source publication, not a frame or GPU owner.
    fn sources(&mut self, assets: &Assets, revision: u64) {
        if self.revision == Some(revision) {
            return;
        }
        self.revision = Some(revision);
        self.effects = audio_fx_fingerprints(&assets.audio_fx_src);
        let changed = self
            .cache
            .iter()
            .filter(|(name, entry)| self.stamp(assets, name) != entry.stamp)
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        for name in changed {
            self.cache.remove(&name);
            if let Some(device) = &mut self.device {
                device.stop_loops(&name);
                device.unload(&name);
            }
            for pending in &mut self.loops {
                if matches!(&pending.command,AudioCommand::Play {name:clip,..} if clip==&name) {
                    pending.started = false;
                }
            }
        }
    }
    fn discard_device(&mut self) {
        self.device = None;
        self.cache.clear();
        for pending in &mut self.loops {
            pending.started = false;
        }
        self.next_attempt = self.now.saturating_add(self.limits.retry_interval);
    }
    fn ensure_device(&mut self) {
        if self.device.as_ref().is_some_and(|device| !device.healthy()) {
            self.report("Audio output device became unavailable".into());
            self.discard_device();
        }
        if self.device.is_some()
            || !self.requested
            || self.suspended
            || self.now < self.next_attempt
        {
            return;
        }
        self.stats.device_attempts = self.stats.device_attempts.saturating_add(1);
        self.next_attempt = self.now.saturating_add(self.limits.retry_interval);
        match (self.factory)() {
            Ok(mut device) => {
                for (bus, volume) in &self.volumes {
                    device.command(
                        &AudioCommand::SetBusVolume {
                            bus: bus.clone(),
                            volume: *volume,
                        },
                        self.limits.voices,
                    );
                }
                self.stats.devices_created = self.stats.devices_created.saturating_add(1);
                self.device = Some(device);
                self.last_error = None;
            }
            Err(error) => self.report(error),
        }
    }
    fn ready(&self) -> bool {
        !self.suspended
            && self
                .device
                .as_ref()
                .is_some_and(|d| d.healthy() && d.ready())
    }
    fn pinned(&self, name: &str) -> bool {
        self.loops.iter().any(
            |pending| matches!(&pending.command,AudioCommand::Play {name:clip,..} if clip==name),
        )
    }
    fn evict(&mut self, except: &str) -> bool {
        let victim = self
            .cache
            .iter()
            .filter(|(name, _)| name.as_str() != except && !self.pinned(name))
            .min_by_key(|(_, entry)| entry.used)
            .map(|(name, _)| name.clone());
        let Some(name) = victim else {
            return false;
        };
        self.cache.remove(&name);
        if let Some(device) = &mut self.device {
            device.unload(&name);
        }
        true
    }
    fn load(&mut self, assets: &Assets, name: &str) -> Result<(), AudioError> {
        let generated = if self.effects.contains_key(name) {
            match synthesize_audio_fx_wav_bounded(
                &assets.audio_fx_src,
                name,
                self.limits.generated_pcm_bytes,
            ) {
                Ok(bytes) => Some(bytes),
                Err(AudioError::PcmBudget) => return Err(AudioError::PcmBudget),
                Err(error) if !assets.audio.contains_key(name) => return Err(error),
                Err(_) => None,
            }
        } else {
            None
        };
        let bytes = generated
            .as_deref()
            .or_else(|| assets.audio.get(name).map(Vec::as_slice))
            .ok_or_else(|| AudioError::Decode(format!("Audio asset '{name}' not found")))?;
        self.device
            .as_mut()
            .unwrap()
            .load(name, bytes, self.limits.pcm_bytes)
    }
    fn ensure_clip(&mut self, assets: &Assets, name: &str) -> bool {
        self.serial = self.serial.saturating_add(1);
        if let Some(entry) = self.cache.get_mut(name) {
            entry.used = self.serial;
            if entry.state == ClipState::Loaded {
                return true;
            }
            if entry.state == ClipState::Invalid || self.now < entry.retry {
                return false;
            }
        }
        if self.limits.cached_clips == 0 {
            return false;
        }
        while !self.cache.contains_key(name) && self.cache.len() >= self.limits.cached_clips {
            if !self.evict(name) {
                return false;
            }
        }
        let mut result = self.load(assets, name);
        while matches!(result, Err(AudioError::PcmBudget)) && self.evict(name) {
            result = self.load(assets, name);
        }
        let state = match result {
            Ok(()) => {
                self.stats.clip_loads = self.stats.clip_loads.saturating_add(1);
                ClipState::Loaded
            }
            Err(error) => {
                let budget = matches!(error, AudioError::PcmBudget);
                self.report(format!("Audio '{name}': {error}"));
                if budget {
                    ClipState::Budget
                } else {
                    ClipState::Invalid
                }
            }
        };
        self.cache.insert(
            name.into(),
            CachedClip {
                stamp: self.stamp(assets, name),
                state,
                used: self.serial,
                retry: self.now.saturating_add(self.limits.retry_interval),
            },
        );
        state == ClipState::Loaded
    }
    fn pump(&mut self, assets: &Assets) {
        if !self.ready() {
            return;
        }
        for index in 0..self.loops.len() {
            if self.loops[index].started {
                continue;
            }
            let command = self.loops[index].command.clone();
            let AudioCommand::Play { name, .. } = &command else {
                unreachable!()
            };
            if self.ensure_clip(assets, name)
                && self
                    .device
                    .as_mut()
                    .unwrap()
                    .command(&command, self.limits.voices)
            {
                self.loops[index].started = true;
                self.stats.plays = self.stats.plays.saturating_add(1);
            }
        }
    }
    pub fn update(&mut self, assets: &Assets, revision: u64, elapsed: Duration) {
        self.now = self.now.saturating_add(elapsed);
        self.sources(assets, revision);
        self.ensure_device();
        self.pump(assets);
    }
    pub fn user_gesture(&mut self, assets: &Assets, revision: u64) {
        self.sources(assets, revision);
        self.requested = true;
        self.ensure_device();
        if !self.suspended
            && let Some(device) = &mut self.device
            && let Err(error) = device.unlock()
        {
            self.report(error);
        }
        self.pump(assets);
    }
    pub fn set_suspended(&mut self, suspended: bool) {
        if self.suspended == suspended {
            return;
        }
        self.suspended = suspended;
        if let Some(device) = &mut self.device
            && let Err(error) = device.set_suspended(suspended)
        {
            self.report(error);
            self.discard_device();
        }
    }
    /// Delivered one-shots are never queued across missing devices or browser
    /// policy. Loops are desired playback state until clear/stop/reset.
    pub fn dispatch(&mut self, assets: &Assets, revision: u64, commands: &[AudioCommand]) {
        self.sources(assets, revision);
        for command in commands {
            self.stats.delivered_commands = self.stats.delivered_commands.saturating_add(1);
            match command {
                AudioCommand::Play {
                    name,
                    bus,
                    gain,
                    looping,
                } => {
                    if !gain.is_finite()
                        || (!self.volumes.contains_key(bus)
                            && self.volumes.len() >= self.limits.buses)
                        || (*looping && self.loops.len() >= self.limits.looping_voices)
                    {
                        self.stats.rejected_commands =
                            self.stats.rejected_commands.saturating_add(1);
                        self.report("Audio voice/bus limit or invalid gain".into());
                        continue;
                    }
                    self.volumes.entry(bus.clone()).or_insert(1.);
                    self.requested = true;
                    self.ensure_device();
                    if *looping {
                        self.loops.push(Loop {
                            command: command.clone(),
                            started: false,
                        });
                        self.pump(assets);
                    } else if self.ready()
                        && self.ensure_clip(assets, name)
                        && self
                            .device
                            .as_mut()
                            .unwrap()
                            .command(command, self.limits.voices)
                    {
                        self.stats.plays = self.stats.plays.saturating_add(1);
                    } else {
                        self.stats.dropped_one_shots =
                            self.stats.dropped_one_shots.saturating_add(1);
                    }
                }
                AudioCommand::SetBusVolume { bus, volume } => {
                    if !volume.is_finite()
                        || (!self.volumes.contains_key(bus)
                            && self.volumes.len() >= self.limits.buses)
                    {
                        self.stats.rejected_commands =
                            self.stats.rejected_commands.saturating_add(1);
                        continue;
                    }
                    self.volumes.insert(bus.clone(), volume.clamp(0., 4.));
                    if let Some(device) = &mut self.device {
                        device.command(command, self.limits.voices);
                    }
                }
                AudioCommand::ClearBus { bus } => {
                    self.loops.retain(|pending|!matches!(&pending.command,AudioCommand::Play {bus:voice,..} if voice==bus));
                    if let Some(device) = &mut self.device {
                        device.command(command, self.limits.voices);
                    }
                }
                AudioCommand::ClearAll => self.stop(),
            }
        }
    }
    pub fn stop(&mut self) {
        self.loops.clear();
        self.requested = false;
        if let Some(device) = &mut self.device {
            device.command(&AudioCommand::ClearAll, self.limits.voices);
        }
    }
    /// Full document replacement revokes playback and source caches. A healthy
    /// device can be retained; it owns no project references or GPU resources.
    pub fn reset(&mut self) {
        self.stop();
        if let Some(device) = &mut self.device {
            device.reset();
        }
        self.cache.clear();
        self.effects.clear();
        self.revision = None;
        self.volumes.clear();
        self.last_error = None;
        self.diagnostics.clear();
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
