use super::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Trace {
    attempted: usize,
    fail: bool,
    ready: bool,
    healthy: bool,
    commands: Vec<AudioCommand>,
    loads: Vec<(String, Vec<f32>)>,
    unloaded: Vec<String>,
    stopped: Vec<String>,
    suspensions: Vec<bool>,
    unlocks: usize,
    resets: usize,
    install_budget: bool,
}
struct Device(Rc<RefCell<Trace>>);
impl AudioDevice for Device {
    fn healthy(&self) -> bool {
        self.0.borrow().healthy
    }
    fn ready(&self) -> bool {
        self.0.borrow().ready
    }
    fn unlock(&mut self) -> Result<(), String> {
        let mut t = self.0.borrow_mut();
        t.ready = true;
        t.unlocks += 1;
        Ok(())
    }
    fn set_suspended(&mut self, v: bool) -> Result<(), String> {
        self.0.borrow_mut().suspensions.push(v);
        Ok(())
    }
    fn install(
        &mut self,
        name: &str,
        clip: &PreparedClip,
        budget: usize,
    ) -> Result<(), AudioError> {
        if clip.pcm_bytes() > budget || self.0.borrow().install_budget {
            return Err(AudioError::PcmBudget);
        }
        self.0
            .borrow_mut()
            .loads
            .push((name.into(), clip.samples().to_vec()));
        Ok(())
    }
    fn unload(&mut self, name: &str) {
        self.0.borrow_mut().unloaded.push(name.into());
    }
    fn stop_loops(&mut self, name: &str) {
        self.0.borrow_mut().stopped.push(name.into());
    }
    fn reset(&mut self) {
        self.0.borrow_mut().resets += 1;
    }
    fn command(&mut self, command: &AudioCommand, _: usize) -> bool {
        self.0.borrow_mut().commands.push(command.clone());
        true
    }
}
fn audio_wav(samples: &[i16]) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(
        &mut bytes,
        hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
    bytes.into_inner()
}
fn fixture(limits: AudioLimits) -> (AudioOutput<Device>, Rc<RefCell<Trace>>, Assets) {
    let trace = Rc::new(RefCell::new(Trace {
        ready: true,
        healthy: true,
        ..Default::default()
    }));
    let factory = trace.clone();
    let output = AudioOutput::with_preparer(
        limits,
        move || {
            let mut t = factory.borrow_mut();
            t.attempted += 1;
            if t.fail {
                Err("device unavailable".into())
            } else {
                t.healthy = true;
                Ok(Device(factory.clone()))
            }
        },
        Box::new(preparation::InlinePreparation::default()),
    );
    let mut assets = Assets::default();
    assets
        .audio
        .insert("song".into(), audio_wav(&[100, 200, 300]).into());
    assets
        .audio
        .insert("hit".into(), audio_wav(&[400, 500, 600]).into());
    (output, trace, assets)
}
fn play(name: &str, looping: bool) -> AudioCommand {
    AudioCommand::Play {
        name: name.into(),
        bus: if looping { "music" } else { "sfx" }.into(),
        gain: 0.5,
        looping,
    }
}
fn plays(trace: &Trace) -> Vec<AudioCommand> {
    trace
        .commands
        .iter()
        .filter(|c| matches!(c, AudioCommand::Play { .. }))
        .cloned()
        .collect()
}
#[test]
fn audio_output_is_lazy_delivers_once_and_does_not_redecode_or_replay_on_unchanged_updates() {
    let (mut output, trace, assets) = fixture(AudioLimits::default());
    for _ in 0..3 {
        output.update(&assets, 1, Duration::from_millis(16));
    }
    assert_eq!(trace.borrow().attempted, 0);
    output.dispatch(&assets, 1, &[play("song", true), play("hit", false)]);
    assert_eq!(
        plays(&trace.borrow()),
        vec![play("song", true), play("hit", false)]
    );
    for _ in 0..30 {
        output.update(&assets, 1, Duration::from_millis(16));
    }
    assert_eq!(trace.borrow().loads.len(), 2);
    assert_eq!(trace.borrow().attempted, 1);
    assert_eq!(plays(&trace.borrow()).len(), 2);
    assert_eq!(output.stats().delivered_commands, 2);
    output.update(&assets, 2, Duration::ZERO);
    assert_eq!(
        trace.borrow().loads.len(),
        2,
        "unrelated content revisions cannot decode/restart unchanged audio"
    );
}
#[test]
fn audio_output_retries_with_backoff_restores_only_loops_and_preserves_bus_volume() {
    let (mut output, trace, assets) = fixture(AudioLimits::default());
    trace.borrow_mut().fail = true;
    let volume = AudioCommand::SetBusVolume {
        bus: "music".into(),
        volume: 0.25,
    };
    output.dispatch(
        &assets,
        1,
        &[volume.clone(), play("song", true), play("hit", false)],
    );
    for _ in 0..10 {
        output.update(&assets, 1, Duration::from_millis(10));
    }
    assert_eq!(trace.borrow().attempted, 1);
    assert_eq!(output.stats().dropped_one_shots, 1);
    trace.borrow_mut().fail = false;
    output.update(&assets, 1, Duration::from_secs(1));
    assert_eq!(plays(&trace.borrow()), vec![play("song", true)]);
    assert!(trace.borrow().commands.contains(&volume));
    trace.borrow_mut().healthy = false;
    output.update(&assets, 1, Duration::ZERO);
    assert_eq!(trace.borrow().attempted, 2);
    output.update(&assets, 1, Duration::from_secs(1));
    assert_eq!(
        plays(&trace.borrow()),
        vec![play("song", true), play("song", true)]
    );
    assert_eq!(output.stats().delivered_commands, 3);
}
#[test]
fn audio_output_browser_unlock_and_suspend_resume_do_not_replay_playing_voices() {
    let (mut output, trace, assets) = fixture(AudioLimits::default());
    trace.borrow_mut().ready = false;
    output.dispatch(&assets, 1, &[play("song", true), play("hit", false)]);
    assert!(plays(&trace.borrow()).is_empty());
    output.user_gesture(&assets, 1);
    assert_eq!(plays(&trace.borrow()), vec![play("song", true)]);
    for _ in 0..3 {
        output.user_gesture(&assets, 1);
    }
    assert_eq!(plays(&trace.borrow()).len(), 1);
    output.set_suspended(true);
    output.update(&assets, 1, Duration::from_secs(30));
    output.set_suspended(false);
    output.update(&assets, 1, Duration::ZERO);
    assert_eq!(trace.borrow().suspensions, vec![true, false]);
    assert_eq!(plays(&trace.borrow()).len(), 1);
    output.stop();
    trace.borrow_mut().healthy = false;
    output.update(&assets, 1, Duration::from_secs(2));
    output.update(&assets, 1, Duration::from_secs(2));
    assert_eq!(
        trace.borrow().attempted,
        1,
        "stopped music cannot cause device recovery/replay"
    );
}
#[test]
fn audio_output_source_replacement_restarts_only_affected_loops_and_reset_revokes_project_state() {
    let (mut output, trace, mut assets) = fixture(AudioLimits::default());
    output.dispatch(&assets, 1, &[play("song", true), play("hit", false)]);
    assets
        .audio
        .insert("song".into(), audio_wav(&[700, 800, 900]).into());
    output.update(&assets, 2, Duration::ZERO);
    assert_eq!(trace.borrow().stopped, vec!["song"]);
    assert_eq!(
        plays(&trace.borrow()),
        vec![play("song", true), play("hit", false), play("song", true)]
    );
    assert_eq!(
        trace.borrow().loads.last().unwrap().1,
        [700., 800., 900.].map(|s| s / 32768.)
    );
    assets.audio.remove("song");
    output.update(&assets, 3, Duration::ZERO);
    assert_eq!(plays(&trace.borrow()).len(), 3);
    assets.audio.insert("song".into(), audio_wav(&[900]).into());
    output.update(&assets, 4, Duration::ZERO);
    assert_eq!(plays(&trace.borrow()).len(), 4);
    output.reset();
    assert_eq!(trace.borrow().resets, 1);
    assert_eq!(output.cached_clips(), 0);
    assert_eq!(output.desired_loops(), 0);
    output.update(&assets, 5, Duration::from_secs(5));
    assert_eq!(plays(&trace.borrow()).len(), 4);
}
#[test]
fn audio_output_bounds_cache_loops_buses_and_memoizes_corrupt_or_missing_sources() {
    let (mut output, trace, mut assets) = fixture(AudioLimits {
        cached_clips: 2,
        looping_voices: 1,
        buses: 2,
        ..Default::default()
    });
    output.dispatch(
        &assets,
        1,
        &[play("song", true), play("song", true), play("hit", false)],
    );
    assert_eq!(output.desired_loops(), 1);
    assert_eq!(output.stats().rejected_commands, 1);
    assets
        .audio
        .insert("bad".into(), b"invalid".to_vec().into());
    output.update(&assets, 2, Duration::ZERO);
    for _ in 0..3 {
        output.dispatch(&assets, 2, &[play("bad", false)]);
    }
    assert_eq!(output.cached_clips(), 2);
    assert!(
        !trace.borrow().unloaded.contains(&"song".into()),
        "desired loop source stays pinned"
    );
    assert_eq!(
        output
            .take_diagnostics()
            .iter()
            .filter(|e| e.contains("Audio 'bad'"))
            .count(),
        1
    );
    let other = AudioCommand::SetBusVolume {
        bus: "too many".into(),
        volume: 0.2,
    };
    output.dispatch(&assets, 2, &[other]);
    assert_eq!(output.stats().rejected_commands, 2);
}
#[test]
fn audio_output_procedural_effects_override_imports_and_keep_generation_bounded() {
    let (mut output, trace, mut assets) = fixture(AudioLimits::default());
    assets.audio_fx_src = "[sfx.hit]\nwave='sine'\nduration=0.01\n".into();
    output.dispatch(&assets, 1, &[play("hit", false)]);
    assert_eq!(trace.borrow().loads[0].1.len(), 441);
    assets.audio_fx_src = "[sfx.hit]\nduration=1e30\n".into();
    output.update(&assets, 2, Duration::ZERO);
    output.dispatch(&assets, 2, &[play("hit", false)]);
    assert_eq!(
        trace.borrow().loads.len(),
        1,
        "over-budget generated sound cannot silently play a different imported asset"
    );
}

#[test]
fn audio_output_fx_invalidation_tracks_only_the_effect_that_is_actually_playing() {
    let (mut output, trace, mut assets) = fixture(AudioLimits::default());
    assets.audio_fx_src = "[sfx.song]\nwave='sine'\nduration=0.01\n".into();
    output.dispatch(&assets, 1, &[play("song", true)]);
    assets.audio_fx_src.push_str("[sfx.other]\nduration=0.02\n");
    output.update(&assets, 2, Duration::ZERO);
    assert_eq!(
        trace.borrow().loads.len(),
        1,
        "unrelated FX must not restart music"
    );
    assets.audio.insert("song".into(), vec![9, 8, 7].into());
    output.update(&assets, 3, Duration::ZERO);
    assert_eq!(
        trace.borrow().loads.len(),
        1,
        "shadowed imports must not restart FX"
    );
    // Formatting, defaults, aliases and non-playback metadata have no audible effect.
    assets.audio_fx_src =
        "[sfx.song]\nwave='SINE'\nduration=0.01\nrepeat=1\nlabel='new label'\n".into();
    output.update(&assets, 4, Duration::ZERO);
    assert_eq!(trace.borrow().loads.len(), 1);
    assets.audio_fx_src.push_str("freq=440\n");
    output.update(&assets, 5, Duration::ZERO);
    assert_eq!(trace.borrow().loads.len(), 2);
    assert_eq!(trace.borrow().stopped, vec!["song"]);
    assert_eq!(
        plays(&trace.borrow()),
        vec![play("song", true), play("song", true)]
    );
}

#[derive(Default)]
struct Jobs {
    pending: Option<PreparationJob>,
    result: Option<preparation::PreparationResult>,
    cancelled: Vec<u64>,
}
impl Jobs {
    fn finish(&mut self, ignore_cancellation: bool) {
        let job = self
            .pending
            .take()
            .expect("a real CPU preparation must have been submitted");
        if ignore_cancellation {
            job.cancelled.store(false, Ordering::Relaxed);
        }
        self.result = Some(job.run());
    }
}
struct Deferred(Rc<RefCell<Jobs>>);
impl AudioPreparer for Deferred {
    fn is_idle(&self) -> bool {
        let jobs = self.0.borrow();
        jobs.pending.is_none() && jobs.result.is_none()
    }
    fn start(&mut self, job: PreparationJob) -> Result<(), AudioError> {
        assert!(
            self.is_idle(),
            "the host must never queue unbounded preparations"
        );
        self.0.borrow_mut().pending = Some(job);
        Ok(())
    }
    fn poll(&mut self) -> Option<preparation::PreparationResult> {
        self.0.borrow_mut().result.take()
    }
    fn cancel(&mut self, id: u64) {
        let mut jobs = self.0.borrow_mut();
        jobs.cancelled.push(id);
        if let Some(job) = &jobs.pending {
            if job.id == id {
                job.cancelled.store(true, Ordering::Relaxed);
            }
        }
    }
}
fn deferred(
    limits: AudioLimits,
) -> (
    AudioOutput<Device>,
    Rc<RefCell<Trace>>,
    Assets,
    Rc<RefCell<Jobs>>,
) {
    let (mut output, trace, assets) = fixture(limits);
    let jobs = Rc::new(RefCell::new(Jobs::default()));
    output.preparer = Box::new(Deferred(jobs.clone()));
    (output, trace, assets, jobs)
}
#[test]
fn cold_audio_preparation_publishes_only_from_update_and_fresh_one_shots_play_once() {
    let (mut output, trace, assets, jobs) = deferred(AudioLimits::default());
    output.dispatch(&assets, 1, &[play("song", true), play("hit", false)]);
    assert!(plays(&trace.borrow()).is_empty());
    assert!(trace.borrow().loads.is_empty());
    assert_eq!(output.stats().preparation_jobs, 1);
    assert_eq!(output.wake_after(), Some(Duration::from_millis(5)));
    {
        let jobs = jobs.borrow();
        let AudioSource::Encoded(source) = &jobs.pending.as_ref().unwrap().source else {
            panic!("imported source expected")
        };
        assert!(
            Arc::ptr_eq(source, &assets.audio["song"]),
            "request submission must not copy the recording on the host thread"
        );
    }
    jobs.borrow_mut().finish(false);
    assert!(
        trace.borrow().loads.is_empty(),
        "CPU completion cannot touch the host device"
    );
    output.update(&assets, 1, Duration::from_millis(1));
    assert_eq!(plays(&trace.borrow()), vec![play("song", true)]);
    assert_eq!(output.stats().preparation_jobs, 2);
    jobs.borrow_mut().finish(false);
    output.update(&assets, 1, Duration::from_millis(1));
    for _ in 0..5 {
        output.update(&assets, 1, Duration::from_millis(10));
    }
    assert_eq!(
        plays(&trace.borrow()),
        vec![play("song", true), play("hit", false)]
    );
    assert_eq!(output.stats().preparation_jobs, 2);
    assert_eq!(output.stats().delivered_commands, 2);
    assert_eq!(output.stats().dropped_one_shots, 0);
    assert_eq!(output.wake_after(), None);
}
#[test]
fn source_replacement_and_project_reset_reject_late_preparations_even_if_codec_ignores_cancel() {
    let (mut output, trace, mut assets, jobs) = deferred(AudioLimits::default());
    output.dispatch(&assets, 1, &[play("song", true)]);
    let old = jobs.borrow().pending.as_ref().unwrap().id;
    assets
        .audio
        .insert("song".into(), audio_wav(&[700, 800, 900]).into());
    output.update(&assets, 2, Duration::ZERO);
    assert!(jobs.borrow().cancelled.contains(&old));
    jobs.borrow_mut().finish(true);
    output.update(&assets, 2, Duration::ZERO);
    assert!(trace.borrow().loads.is_empty());
    assert_eq!(output.stats().stale_preparations, 1);
    let replacement = jobs.borrow().pending.as_ref().unwrap().id;
    assert_ne!(old, replacement);
    output.reset();
    output.dispatch(&assets, 1, &[play("song", true)]);
    assert!(jobs.borrow().cancelled.contains(&replacement));
    jobs.borrow_mut().finish(true);
    output.update(&assets, 1, Duration::ZERO);
    assert!(trace.borrow().loads.is_empty());
    assert_eq!(output.stats().stale_preparations, 2);
    assert!(jobs.borrow().pending.as_ref().unwrap().id > replacement);
    jobs.borrow_mut().finish(false);
    output.update(&assets, 1, Duration::ZERO);
    assert_eq!(plays(&trace.borrow()), vec![play("song", true)]);
    assert_eq!(
        trace.borrow().loads[0].1,
        [700., 800., 900.].map(|s| s / 32768.)
    );
}
#[test]
fn cold_one_shots_have_bounded_count_and_age_and_cannot_replay_after_loss_or_suspend() {
    let (mut output, trace, assets, jobs) = deferred(AudioLimits {
        pending_one_shots: 2,
        one_shot_ttl: Duration::from_millis(20),
        ..Default::default()
    });
    output.dispatch(
        &assets,
        1,
        &[play("song", false), play("hit", false), play("hit", false)],
    );
    assert_eq!(output.one_shots.len(), 2);
    assert_eq!(output.stats().dropped_one_shots, 1);
    output.update(&assets, 1, Duration::from_millis(21));
    assert!(output.one_shots.is_empty());
    assert_eq!(output.stats().dropped_one_shots, 3);
    assert_eq!(jobs.borrow().cancelled.len(), 1);
    jobs.borrow_mut().finish(true);
    output.update(&assets, 1, Duration::ZERO);
    assert!(trace.borrow().loads.is_empty());
    output.dispatch(&assets, 1, &[play("hit", false)]);
    output.set_suspended(true);
    assert!(output.one_shots.is_empty());
    jobs.borrow_mut().finish(true);
    output.set_suspended(false);
    output.update(&assets, 1, Duration::ZERO);
    assert!(plays(&trace.borrow()).is_empty());
    output.dispatch(&assets, 1, &[play("song", false), play("hit", true)]);
    trace.borrow_mut().healthy = false;
    output.update(&assets, 1, Duration::ZERO);
    jobs.borrow_mut().finish(true);
    output.update(&assets, 1, Duration::from_secs(1));
    assert!(plays(&trace.borrow()).is_empty());
    jobs.borrow_mut().finish(false);
    output.update(&assets, 1, Duration::ZERO);
    assert_eq!(plays(&trace.borrow()), vec![play("hit", true)]);
}
#[test]
fn clear_bus_stop_and_drop_cancel_only_unneeded_preparations() {
    {
        let (mut output, trace, assets, jobs) = deferred(AudioLimits::default());
        let mut other_bus = play("song", true);
        if let AudioCommand::Play { bus, .. } = &mut other_bus {
            *bus = "ambience".into();
        }
        output.dispatch(&assets, 1, &[play("song", true), other_bus.clone()]);
        output.dispatch(
            &assets,
            1,
            &[AudioCommand::ClearBus {
                bus: "music".into(),
            }],
        );
        assert!(
            jobs.borrow().cancelled.is_empty(),
            "another bus still requires the same clip"
        );
        jobs.borrow_mut().finish(false);
        output.update(&assets, 1, Duration::ZERO);
        assert_eq!(plays(&trace.borrow()), vec![other_bus]);
    }
    let (mut output, trace, assets, jobs) = deferred(AudioLimits::default());
    output.dispatch(&assets, 1, &[play("song", true), play("hit", false)]);
    output.dispatch(
        &assets,
        1,
        &[AudioCommand::ClearBus {
            bus: "music".into(),
        }],
    );
    assert_eq!(jobs.borrow().cancelled.len(), 1);
    jobs.borrow_mut().finish(true);
    output.update(&assets, 1, Duration::ZERO);
    assert!(trace.borrow().loads.is_empty());
    output.stop();
    assert_eq!(jobs.borrow().cancelled.len(), 2);
    jobs.borrow_mut().finish(true);
    output.dispatch(&assets, 1, &[play("song", true)]);
    drop(output);
    assert_eq!(jobs.borrow().cancelled.len(), 3);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_default_preparer_integrates_with_host_device_without_replaying_warm_voices() {
    let (mut output, trace, assets) = fixture(AudioLimits::default());
    output.preparer = platform_preparer();
    output.dispatch(&assets, 1, &[play("song", true), play("hit", false)]);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while output.stats().plays != 2 {
        assert!(
            std::time::Instant::now() < deadline,
            "native audio jobs did not publish"
        );
        // Keep the one-shot's simulated presentation age below its TTL. Wall
        // clock completion is independent of the authoritative session clock.
        output.update(&assets, 1, Duration::ZERO);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(trace.borrow().attempted, 1);
    assert_eq!(trace.borrow().loads.len(), 2);
    assert_eq!(output.stats().preparation_jobs, 2);
    let before = plays(&trace.borrow());
    for _ in 0..5 {
        output.update(&assets, 1, Duration::from_millis(16));
    }
    assert_eq!(plays(&trace.borrow()), before);
}

#[test]
fn preparation_source_and_pcm_caps_reject_work_without_unbounded_queueing_or_installation() {
    let (mut output, trace, assets, jobs) = deferred(AudioLimits {
        source_bytes: 4,
        ..Default::default()
    });
    output.dispatch(&assets, 1, &[play("song", false)]);
    assert!(jobs.borrow().pending.is_none());
    assert_eq!(output.stats().preparation_jobs, 0);
    output.update(&assets, 1, Duration::from_secs(1));
    assert_eq!(output.stats().dropped_one_shots, 1);
    assert!(trace.borrow().loads.is_empty());
    let (mut output, trace, assets, jobs) = deferred(AudioLimits {
        preparation_pcm_bytes: 4,
        ..Default::default()
    });
    output.dispatch(&assets, 1, &[play("song", true)]);
    jobs.borrow_mut().finish(false);
    output.update(&assets, 1, Duration::ZERO);
    assert!(trace.borrow().loads.is_empty());
    assert!(matches!(output.cache["song"].state, ClipState::Invalid));
    for _ in 0..10 {
        output.update(&assets, 1, Duration::from_secs(1));
    }
    assert_eq!(
        output.stats().preparation_jobs,
        1,
        "a source exceeding fixed preparation limits must not repeatedly consume CPU"
    );
    assert_eq!(output.wake_after(), None);
}

#[test]
fn resident_pcm_budget_retries_only_while_a_voice_still_requires_that_clip() {
    let (mut output, trace, assets) = fixture(AudioLimits::default());
    trace.borrow_mut().install_budget = true;
    output.dispatch(&assets, 1, &[play("song", true)]);
    assert!(matches!(output.cache["song"].state, ClipState::Budget));
    assert_eq!(output.wake_after(), Some(Duration::from_secs(1)));
    output.update(&assets, 1, Duration::from_millis(100));
    assert_eq!(output.stats().preparation_jobs, 1);
    trace.borrow_mut().install_budget = false;
    output.update(&assets, 1, Duration::from_secs(1));
    assert_eq!(plays(&trace.borrow()), vec![play("song", true)]);
    assert_eq!(output.wake_after(), None);
    let (mut output, trace, assets) = fixture(AudioLimits::default());
    trace.borrow_mut().install_budget = true;
    output.dispatch(&assets, 1, &[play("song", true)]);
    output.dispatch(
        &assets,
        1,
        &[AudioCommand::ClearBus {
            bus: "music".into(),
        }],
    );
    assert_eq!(
        output.wake_after(),
        None,
        "an unneeded cache entry must not keep an idle host awake"
    );
}

#[test]
fn unavailable_preparation_retries_with_backoff_and_does_not_become_a_corrupt_source() {
    struct Recovering {
        attempts: Rc<RefCell<usize>>,
        inline: preparation::InlinePreparation,
    }
    impl AudioPreparer for Recovering {
        fn is_idle(&self) -> bool {
            self.inline.is_idle()
        }
        fn start(&mut self, job: PreparationJob) -> Result<(), AudioError> {
            let mut attempts = self.attempts.borrow_mut();
            *attempts += 1;
            if *attempts == 1 {
                return Err(AudioError::PreparationUnavailable("worker offline".into()));
            }
            self.inline.start(job)
        }
        fn poll(&mut self) -> Option<preparation::PreparationResult> {
            self.inline.poll()
        }
        fn cancel(&mut self, id: u64) {
            self.inline.cancel(id);
        }
    }
    let (mut output, trace, assets) = fixture(AudioLimits::default());
    let attempts = Rc::new(RefCell::new(0));
    output.preparer = Box::new(Recovering {
        attempts: attempts.clone(),
        inline: Default::default(),
    });
    output.dispatch(&assets, 1, &[play("song", true)]);
    assert!(plays(&trace.borrow()).is_empty());
    assert_eq!(output.wake_after(), Some(Duration::from_secs(1)));
    for _ in 0..10 {
        output.update(&assets, 1, Duration::from_millis(1));
    }
    assert_eq!(*attempts.borrow(), 1);
    output.update(&assets, 1, Duration::from_secs(1));
    assert_eq!(*attempts.borrow(), 2);
    assert_eq!(plays(&trace.borrow()), vec![play("song", true)]);
    assert_eq!(
        trace.borrow().attempted,
        1,
        "worker recovery must not replace a healthy device"
    );
    assert_eq!(output.wake_after(), None);
}
