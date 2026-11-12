//! Application-owned audio. The caller publishes source revisions and delivers
//! each command batch once. No server, window, GPU, renderer clock or system
//! timer is owned here. Device recovery restores loops, never old one-shots.
use super::preparation::{AudioPreparer, AudioSource, PreparationJob, platform_preparer};
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
    /// Separate transient/source caps; one preparation job is admitted at a time.
    pub preparation_pcm_bytes: usize,
    pub source_bytes: usize,
    pub pending_one_shots: usize,
    pub one_shot_ttl: Duration,
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
            preparation_pcm_bytes: 64 * 1024 * 1024,
            source_bytes: 64 * 1024 * 1024,
            pending_one_shots: 32,
            one_shot_ttl: Duration::from_millis(250),
            generated_pcm_bytes: 16 * 1024 * 1024,
            cached_clips: 128,
            voices: 128,
            looping_voices: 64,
            buses: 64,
            retry_interval: Duration::from_secs(1),
        }
    }
}
/// Implementations install immutable prepared PCM without decoding. Construction is lazy, and
/// `ready` can remain false until a browser user gesture unlocks its context.
pub trait AudioDevice {
    fn healthy(&self) -> bool;
    fn ready(&self) -> bool;
    fn unlock(&mut self) -> Result<(), String>;
    fn set_suspended(&mut self, suspended: bool) -> Result<(), String>;
    fn install(&mut self, name: &str, clip: &PreparedClip, budget: usize)
    -> Result<(), AudioError>;
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
    fn install(
        &mut self,
        name: &str,
        clip: &PreparedClip,
        budget: usize,
    ) -> Result<(), AudioError> {
        self.install_prepared_clip(name, clip, budget)
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
    pub preparation_jobs: u64,
    pub stale_preparations: u64,
    pub plays: u64,
    pub dropped_one_shots: u64,
    pub rejected_commands: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClipState {
    Loaded,
    Waiting,
    Preparing(u64),
    Invalid,
    Budget,
}
struct CachedClip {
    stamp: Option<u64>,
    state: ClipState,
    used: u64,
    retry: Duration,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClipReady {
    Ready,
    Pending,
    Failed,
}
struct OneShot {
    command: AudioCommand,
    deadline: Duration,
}
struct Loop {
    command: AudioCommand,
    started: bool,
}
pub struct AudioOutput<D: AudioDevice = AudioEngine> {
    factory: Box<dyn FnMut() -> Result<D, String>>,
    device: Option<D>,
    preparer: Box<dyn AudioPreparer>,
    next_job: u64,
    one_shots: VecDeque<OneShot>,
    limits: AudioLimits,
    now: Duration,
    next_attempt: Duration,
    requested: bool,
    suspended: bool,
    revision: Option<u64>,
    effects: FxHashMap<String, preparation::AudioEffect>,
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
        Self::with_preparer(limits, factory, platform_preparer())
    }
    pub fn with_preparer(
        limits: AudioLimits,
        factory: impl FnMut() -> Result<D, String> + 'static,
        preparer: Box<dyn AudioPreparer>,
    ) -> Self {
        Self {
            factory: Box::new(factory),
            device: None,
            preparer,
            next_job: 1,
            one_shots: VecDeque::new(),
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
    /// Service work is scheduled independently of visual FPS. A host can sleep
    /// normally once the single admitted job has published or been cancelled.
    pub fn wake_after(&self) -> Option<Duration> {
        if self.suspended {
            return None;
        }
        if !self.preparer.is_idle() {
            return Some(Duration::from_millis(5));
        }
        if self.requested && self.device.is_none() {
            return Some(self.next_attempt.saturating_sub(self.now));
        }
        self.cache
            .iter()
            .filter(|(name, entry)| entry.state == ClipState::Budget && self.pinned(name))
            .map(|(_, entry)| entry.retry.saturating_sub(self.now))
            .min()
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
            effect.fingerprint().hash(&mut hash);
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
        self.effects = audio_fx_definitions(&assets.audio_fx_src);
        let changed = self
            .cache
            .iter()
            .filter(|(name, entry)| self.stamp(assets, name) != entry.stamp)
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        for name in changed {
            self.remove_cache(&name);
            let previous = self.one_shots.len();
            self.one_shots.retain(|shot| !matches!(&shot.command, AudioCommand::Play { name: clip, .. } if clip == &name));
            self.stats.dropped_one_shots = self
                .stats
                .dropped_one_shots
                .saturating_add((previous - self.one_shots.len()) as u64);
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
    fn remove_cache(&mut self, name: &str) {
        if let Some(entry) = self.cache.remove(name) {
            if let ClipState::Preparing(job) = entry.state {
                self.preparer.cancel(job);
            }
        }
    }
    fn clear_pending_shots(&mut self) {
        self.stats.dropped_one_shots = self
            .stats
            .dropped_one_shots
            .saturating_add(self.one_shots.len() as u64);
        self.one_shots.clear();
    }
    fn clear_cache(&mut self) {
        for entry in self.cache.values() {
            if let ClipState::Preparing(job) = entry.state {
                self.preparer.cancel(job);
            }
        }
        self.cache.clear();
    }
    fn cancel_unwanted(&mut self) {
        let pending = self
            .cache
            .iter()
            .filter(|(name, e)| {
                matches!(e.state, ClipState::Waiting | ClipState::Preparing(_))
                    && !self.pinned(name)
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        for name in pending {
            self.remove_cache(&name);
        }
    }
    fn discard_device(&mut self) {
        self.device = None;
        self.clear_pending_shots();
        self.clear_cache();
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
        self.loops
            .iter()
            .map(|pending| &pending.command)
            .chain(self.one_shots.iter().map(|pending| &pending.command))
            .any(|command| matches!(command, AudioCommand::Play {name:clip,..} if clip == name))
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
        self.remove_cache(&name);
        if let Some(device) = &mut self.device {
            device.unload(&name);
        }
        true
    }
    fn source(&self, assets: &Assets, name: &str) -> Result<AudioSource, AudioError> {
        if let Some(effect) = self.effects.get(name) {
            Ok(AudioSource::Generated(effect.clone()))
        } else {
            let bytes = assets
                .audio
                .get(name)
                .ok_or_else(|| AudioError::Decode(format!("Audio asset '{name}' not found")))?;
            if bytes.len() > self.limits.source_bytes {
                return Err(AudioError::PcmBudget);
            }
            Ok(AudioSource::Encoded(bytes.clone()))
        }
    }
    fn failed_state(&mut self, name: &str, error: AudioError, retry_pcm: bool) -> ClipState {
        let retry = matches!(error, AudioError::PreparationUnavailable(_))
            || (retry_pcm && matches!(error, AudioError::PcmBudget));
        self.report(format!("Audio '{name}': {error}"));
        if retry {
            ClipState::Budget
        } else {
            ClipState::Invalid
        }
    }
    fn accept_prepared(&mut self) {
        let Some(result) = self.preparer.poll() else {
            return;
        };
        let Some(name) = self
            .cache
            .iter()
            .find(|(_, entry)| entry.state == ClipState::Preparing(result.id))
            .map(|(name, _)| name.clone())
        else {
            self.stats.stale_preparations = self.stats.stale_preparations.saturating_add(1);
            return;
        };
        let preparation_failed = result.clip.is_err();
        let installed = result.clip.and_then(|clip| {
            let Some(device) = self.device.as_mut() else {
                return Err(AudioError::Cancelled);
            };
            let mut install = device.install(&name, &clip, self.limits.pcm_bytes);
            while matches!(install, Err(AudioError::PcmBudget)) && self.evict(&name) {
                install =
                    self.device
                        .as_mut()
                        .unwrap()
                        .install(&name, &clip, self.limits.pcm_bytes);
            }
            install
        });
        let state = match installed {
            Ok(()) => {
                self.stats.clip_loads = self.stats.clip_loads.saturating_add(1);
                ClipState::Loaded
            }
            Err(error) => self.failed_state(&name, error, !preparation_failed),
        };
        if let Some(entry) = self.cache.get_mut(&name) {
            entry.state = state;
            entry.retry = self.now.saturating_add(self.limits.retry_interval);
        }
    }
    fn ensure_clip(&mut self, assets: &Assets, name: &str) -> ClipReady {
        self.serial = self.serial.saturating_add(1);
        if let Some(entry) = self.cache.get_mut(name) {
            entry.used = self.serial;
            match entry.state {
                ClipState::Loaded => return ClipReady::Ready,
                ClipState::Preparing(_) => return ClipReady::Pending,
                ClipState::Invalid => return ClipReady::Failed,
                ClipState::Budget if self.now < entry.retry => return ClipReady::Pending,
                _ => {}
            }
        }
        if self.limits.cached_clips == 0 {
            return ClipReady::Failed;
        }
        while !self.cache.contains_key(name) && self.cache.len() >= self.limits.cached_clips {
            if !self.evict(name) {
                return ClipReady::Failed;
            }
        }
        if !self.cache.contains_key(name) {
            let stamp = self.stamp(assets, name);
            self.cache.insert(
                name.into(),
                CachedClip {
                    stamp,
                    state: ClipState::Waiting,
                    used: self.serial,
                    retry: Duration::ZERO,
                },
            );
        }
        if !self.preparer.is_idle() {
            return ClipReady::Pending;
        }
        let start = self.source(assets, name).and_then(|source| {
            let id = self.next_job;
            self.next_job = id
                .checked_add(1)
                .ok_or_else(|| AudioError::Decode("audio job identity exhausted".into()))?;
            self.preparer.start(PreparationJob {
                id,
                source,
                pcm_bytes: self.limits.preparation_pcm_bytes.min(self.limits.pcm_bytes),
                generated_pcm_bytes: self.limits.generated_pcm_bytes,
                cancelled: Arc::new(AtomicBool::new(false)),
            })?;
            self.stats.preparation_jobs = self.stats.preparation_jobs.saturating_add(1);
            Ok(ClipState::Preparing(id))
        });
        let state = match start {
            Ok(state) => state,
            Err(error) => self.failed_state(name, error, false),
        };
        let entry = self.cache.get_mut(name).unwrap();
        entry.state = state;
        entry.retry = self.now.saturating_add(self.limits.retry_interval);
        self.accept_prepared();
        match self.cache.get(name).map(|e| e.state) {
            Some(ClipState::Loaded) => ClipReady::Ready,
            Some(ClipState::Invalid) => ClipReady::Failed,
            _ => ClipReady::Pending,
        }
    }
    fn pump(&mut self, assets: &Assets) {
        self.accept_prepared();
        if !self.ready() {
            return;
        }
        let mut index = 0;
        while index < self.one_shots.len() {
            let command = self.one_shots[index].command.clone();
            let AudioCommand::Play { name, .. } = &command else {
                unreachable!()
            };
            let result = if self.now >= self.one_shots[index].deadline {
                ClipReady::Failed
            } else {
                self.ensure_clip(assets, name)
            };
            if result == ClipReady::Pending {
                index += 1;
                continue;
            }
            self.one_shots.remove(index);
            if result == ClipReady::Ready
                && self
                    .device
                    .as_mut()
                    .unwrap()
                    .command(&command, self.limits.voices)
            {
                self.stats.plays = self.stats.plays.saturating_add(1);
            } else {
                self.stats.dropped_one_shots = self.stats.dropped_one_shots.saturating_add(1);
            }
        }
        for index in 0..self.loops.len() {
            if self.loops[index].started {
                continue;
            }
            let command = self.loops[index].command.clone();
            let AudioCommand::Play { name, .. } = &command else {
                unreachable!()
            };
            if self.ensure_clip(assets, name) == ClipReady::Ready
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
        self.cancel_unwanted();
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
        if suspended {
            self.clear_pending_shots();
            self.cancel_unwanted();
        }
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
                    bus, gain, looping, ..
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
                    } else if self.ready() && self.one_shots.len() < self.limits.pending_one_shots {
                        self.one_shots.push_back(OneShot {
                            command: command.clone(),
                            deadline: self.now.saturating_add(self.limits.one_shot_ttl),
                        });
                        self.pump(assets);
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
                    let previous = self.one_shots.len();
                    self.one_shots.retain(|shot| !matches!(&shot.command, AudioCommand::Play {bus: voice,..} if voice == bus));
                    self.stats.dropped_one_shots = self
                        .stats
                        .dropped_one_shots
                        .saturating_add((previous - self.one_shots.len()) as u64);
                    self.loops.retain(|pending|!matches!(&pending.command,AudioCommand::Play {bus:voice,..} if voice==bus));
                    self.cancel_unwanted();
                    if let Some(device) = &mut self.device {
                        device.command(command, self.limits.voices);
                    }
                }
                AudioCommand::ClearAll => self.stop(),
            }
        }
    }
    pub fn stop(&mut self) {
        self.clear_pending_shots();
        self.loops.clear();
        let pending = self
            .cache
            .iter()
            .filter(|(_, e)| matches!(e.state, ClipState::Waiting | ClipState::Preparing(_)))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        for name in pending {
            self.remove_cache(&name);
        }
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
        self.clear_cache();
        self.effects.clear();
        self.revision = None;
        self.volumes.clear();
        self.last_error = None;
        self.diagnostics.clear();
    }
}

impl<D: AudioDevice> Drop for AudioOutput<D> {
    fn drop(&mut self) {
        self.clear_cache();
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
