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
/// Cheap borrowed fan-out. A service which owns one output family can forward
/// the remaining slices without cloning payloads or draining the session again.
#[derive(Clone, Copy, Debug)]
pub struct RegionOutputs<'a> {
    pub messages: &'a [Message],
    pub says: &'a [Say],
    pub choices: &'a [MultipleChoice],
    pub audio: &'a [AudioCommand],
    pub world_text: &'a [(u32, WorldText)],
    pub containers: &'a [OpenContainerRequest],
}
impl<'a> RegionOutputs<'a> {
    pub fn without_containers(self) -> Self {
        Self {
            containers: &[],
            ..self
        }
    }
    pub fn without_audio(self) -> Self {
        Self { audio: &[], ..self }
    }
}
impl RegionEvents {
    pub fn outputs(&self) -> RegionOutputs<'_> {
        RegionOutputs {
            messages: &self.messages,
            says: &self.says,
            choices: &self.choices,
            audio: &self.audio,
            world_text: &self.world_text,
            containers: &self.containers,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_output_fanout_keeps_the_original_payload_and_can_assign_audio_once() {
        let events = RegionEvents {
            messages: vec![(None, None, 7, "Hello".into(), "status".into())],
            audio: vec![AudioCommand::ClearAll],
            containers: vec![(7, None)],
            ..Default::default()
        };
        let outputs = events.outputs();
        let without_containers = outputs.without_containers();
        assert!(without_containers.containers.is_empty());
        assert_eq!(outputs.containers, &[(7, None)]);
        assert!(std::ptr::eq(
            without_containers.audio.as_ptr(),
            outputs.audio.as_ptr()
        ));
        let other = outputs.without_audio();
        assert_eq!(outputs.audio.len(), 1);
        assert!(other.audio.is_empty());
        assert!(std::ptr::eq(
            outputs.messages.as_ptr(),
            other.messages.as_ptr()
        ));
        assert!(std::ptr::eq(
            other.messages.as_ptr(),
            events.messages.as_ptr()
        ));
        assert_eq!(
            events.audio.len(),
            1,
            "borrowing does not drain or copy the session batch"
        );
    }
}
