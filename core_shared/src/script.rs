// --- ScriptMessage

#[derive(PartialEq, Clone, Debug)]
pub enum ScriptMessage {
    Status(String),
    Debug(String),
    Error(String),
}

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptMessageCmd {
    pub messages: Vec<ScriptMessage>,
}

impl ScriptMessageCmd {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    pub fn status(&mut self, message: &str) {
        self.messages
            .push(ScriptMessage::Status(message.to_owned()));
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
    engine
        .register_type_with_name::<ScriptMessageCmd>("Messages")
        .register_fn("status", ScriptMessageCmd::status)
        .register_fn("debug", ScriptMessageCmd::debug)
        .register_fn("error", ScriptMessageCmd::error);
}

// Failure Enum

use rhai::plugin::*;
use rhai::Dynamic;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FailureEnum {
    No,
    TooFarAway,
    NoValidTarget,
}

// Create a plugin module with functions constructing the 'MyEnum' variants
#[export_module]
mod failure_module {
    // Constructors
    #[allow(non_upper_case_globals)]
    pub const NoFailure: FailureEnum = FailureEnum::No;
    #[allow(non_upper_case_globals)]
    pub const TooFarAway: FailureEnum = FailureEnum::TooFarAway;
    #[allow(non_upper_case_globals)]
    pub const NoValidTarget: FailureEnum = FailureEnum::NoValidTarget;

    // Printing
    #[rhai_fn(global, name = "to_string", name = "to_debug", pure)]
    pub fn to_string(failure_enum: &mut FailureEnum) -> String {
        format!("{failure_enum:?}")
    }

    // '==' and '!=' operators
    #[rhai_fn(global, name = "==", pure)]
    pub fn eq(failure_enum: &mut FailureEnum, failure_enum2: FailureEnum) -> bool {
        failure_enum == &failure_enum2
    }
    #[rhai_fn(global, name = "!=", pure)]
    pub fn neq(failure_enum: &mut FailureEnum, failure_enum2: FailureEnum) -> bool {
        failure_enum != &failure_enum2
    }
}

pub fn script_register_failure_enum_api(engine: &mut rhai::Engine) {
    engine
        .register_type_with_name::<FailureEnum>("Failure")
        .register_static_module("Failure", exported_module!(failure_module).into());
}
