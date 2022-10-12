// --- ScriptMessage

#[derive(PartialEq, Clone, Debug)]
pub enum ScriptMessage {
    Status(String),
    Debug(String),
    Error(String),
}

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptMessageCmd {
    pub messages            : Vec<ScriptMessage>
}

impl ScriptMessageCmd {
    pub fn new() -> Self {
        Self {
            messages        : vec![],
        }
    }

    pub fn status(&mut self, message: &str) {
        self.messages.push(ScriptMessage::Status(message.to_owned()));
    }

    pub fn debug(&mut self, message: &str) {
        self.messages.push(ScriptMessage::Debug(message.to_owned()));
    }

    pub fn error(&mut self, message: &str) {
        self.messages.push(ScriptMessage::Error(message.to_owned()));
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

pub fn script_register_message_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<ScriptMessageCmd>("Messages")
        .register_fn("status", ScriptMessageCmd::status)
        .register_fn("debug", ScriptMessageCmd::debug)
        .register_fn("error", ScriptMessageCmd::error);
}

