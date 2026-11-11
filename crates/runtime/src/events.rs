use rusterix::{
    MultipleChoice,
    server::{Message, OpenContainerRequest, Say, message::AudioCommand},
    world_text::WorldText,
};
/// A host declares which outputs it consumes. Unsubscribed channels are still
/// drained once from the engine, so an inactive presentation cannot accumulate
/// an unbounded audio/dialog/world-text backlog inside the simulation.
#[derive(Clone, Copy, Debug)]
pub struct EventInterests {
    pub messages: bool,
    pub says: bool,
    pub choices: bool,
    pub audio: bool,
    pub world_text: bool,
    pub containers: bool,
}
impl Default for EventInterests {
    fn default() -> Self {
        Self {
            messages: true,
            says: true,
            choices: true,
            audio: true,
            world_text: true,
            containers: true,
        }
    }
}
impl EventInterests {
    pub fn terminal() -> Self {
        Self {
            messages: true,
            says: true,
            choices: false,
            audio: false,
            world_text: false,
            containers: false,
        }
    }
}
#[derive(Default, Debug)]
pub struct RegionEvents {
    pub messages: Vec<Message>,
    pub says: Vec<Say>,
    pub choices: Vec<MultipleChoice>,
    pub audio: Vec<AudioCommand>,
    pub world_text: Vec<(u32, WorldText)>,
    pub containers: Vec<OpenContainerRequest>,
}
impl RegionEvents {
    pub(crate) fn collect(
        &mut self,
        server: &mut rusterix::Server,
        id: &theframework_core::Uuid,
        interest: EventInterests,
    ) {
        let messages = server.get_messages(id);
        if interest.messages {
            self.messages.extend(messages);
        }
        let says = server.get_says(id);
        if interest.says {
            self.says.extend(says);
        }
        let choices = server.get_choices(id);
        if interest.choices {
            self.choices.extend(choices);
        }
        let audio = server.get_audio_commands(id);
        if interest.audio {
            self.audio.extend(audio);
        }
        let text = server.get_world_texts(id);
        if interest.world_text {
            self.world_text.extend(text);
        }
        let containers = server.get_open_container_requests(id);
        if interest.containers {
            self.containers.extend(containers);
        }
    }
}
