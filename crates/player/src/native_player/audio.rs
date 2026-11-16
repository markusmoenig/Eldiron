//! One audio owner outside the GPU/view lifetime. Other application services
//! receive borrowed non-audio outputs, so composing services cannot replay it.
use super::*;
use rusterix::audio::{
    AudioEngine,
    output::{AudioDevice, AudioOutput},
};
pub struct AudioServices<S, D: AudioDevice = AudioEngine> {
    pub audio: AudioOutput<D>,
    pub inner: S,
    revision: u64,
    state: Option<ServerState>,
    session: Option<Uuid>,
}
impl<S> AudioServices<S, AudioEngine> {
    pub fn new(inner: S) -> Self {
        Self::with_output(inner, AudioOutput::default())
    }
}
impl<S, D: AudioDevice> AudioServices<S, D> {
    pub fn with_output(inner: S, audio: AudioOutput<D>) -> Self {
        Self {
            audio,
            inner,
            revision: 0,
            state: None,
            session: None,
        }
    }
    fn bind_session(&mut self, session: Uuid) {
        if self.session.is_some_and(|old| old != session) {
            self.audio.reset();
            self.state = None;
        }
        self.session = Some(session);
    }
}
impl<S: PlayerServices, D: AudioDevice> PlayerServices for AudioServices<S, D> {
    fn update(&mut self, world: &RuntimeView<'_>, revision: u64, elapsed: Duration) {
        self.bind_session(world.session_id);
        self.revision = revision;
        if world.state == ServerState::Off && self.state != Some(ServerState::Off) {
            self.audio.stop();
        }
        self.state = Some(world.state);
        self.audio.update(world.assets, revision, elapsed);
        self.inner.update(world, revision, elapsed);
    }
    fn user_gesture(&mut self, world: &RuntimeView<'_>, revision: u64) {
        self.bind_session(world.session_id);
        self.revision = revision;
        self.audio.user_gesture(world.assets, revision);
        self.inner.user_gesture(world, revision);
    }
    fn set_suspended(&mut self, suspended: bool) {
        self.audio.set_suspended(suspended);
        self.inner.set_suspended(suspended);
    }
    fn wake_after(&self) -> Option<Duration> {
        self.audio
            .wake_after()
            .into_iter()
            .chain(self.inner.wake_after())
            .min()
    }
    fn region_events(&mut self, region: Uuid, world: &RuntimeView<'_>, events: RegionOutputs<'_>) {
        self.bind_session(world.session_id);
        if world.state != ServerState::Off {
            self.audio
                .dispatch(world.assets, self.revision, events.audio);
        }
        self.inner
            .region_events(region, world, events.without_audio());
    }
    fn screen_effect(
        &mut self,
        world: &RuntimeView<'_>,
        effect: &ScreenEffect,
    ) -> std::result::Result<(), String> {
        self.inner.screen_effect(world, effect)
    }
}
