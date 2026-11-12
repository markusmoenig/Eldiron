use super::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Trace {
    attempted: usize,
    fail: bool,
    ready: bool,
    healthy: bool,
    commands: Vec<AudioCommand>,
    loads: Vec<(String, Vec<u8>)>,
    unloaded: Vec<String>,
    stopped: Vec<String>,
    suspensions: Vec<bool>,
    unlocks: usize,
    resets: usize,
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
    fn load(&mut self, name: &str, bytes: &[u8], budget: usize) -> Result<(), AudioError> {
        if bytes.len() > budget {
            return Err(AudioError::PcmBudget);
        }
        if bytes == b"invalid" {
            return Err(AudioError::Decode("bad clip".into()));
        }
        self.0
            .borrow_mut()
            .loads
            .push((name.into(), bytes.to_vec()));
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
fn fixture(limits: AudioLimits) -> (AudioOutput<Device>, Rc<RefCell<Trace>>, Assets) {
    let trace = Rc::new(RefCell::new(Trace {
        ready: true,
        healthy: true,
        ..Default::default()
    }));
    let factory = trace.clone();
    let output = AudioOutput::new(limits, move || {
        let mut t = factory.borrow_mut();
        t.attempted += 1;
        if t.fail {
            Err("device unavailable".into())
        } else {
            t.healthy = true;
            Ok(Device(factory.clone()))
        }
    });
    let mut assets = Assets::default();
    assets.audio.insert("song".into(), vec![1, 2, 3]);
    assets.audio.insert("hit".into(), vec![4, 5, 6]);
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
    assets.audio.insert("song".into(), vec![7, 8, 9]);
    output.update(&assets, 2, Duration::ZERO);
    assert_eq!(trace.borrow().stopped, vec!["song"]);
    assert_eq!(
        plays(&trace.borrow()),
        vec![play("song", true), play("hit", false), play("song", true)]
    );
    assert_eq!(trace.borrow().loads.last().unwrap().1, vec![7, 8, 9]);
    assets.audio.remove("song");
    output.update(&assets, 3, Duration::ZERO);
    assert_eq!(plays(&trace.borrow()).len(), 3);
    assets.audio.insert("song".into(), vec![9]);
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
    assets.audio.insert("bad".into(), b"invalid".to_vec());
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
            .filter(|e| e.contains("bad clip"))
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
    assert!(trace.borrow().loads[0].1.starts_with(b"RIFF"));
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
    assets.audio.insert("song".into(), vec![9, 8, 7]);
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
