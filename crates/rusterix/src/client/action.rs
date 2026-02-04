use crate::vm::node::hosthandler::HostHandler;
use crate::vm::*;
use crate::{Assets, EntityAction, Value};
use std::str::FromStr;

#[derive(Default)]
struct ClientHostHandler {
    pub action: Option<EntityAction>,
}

impl HostHandler for ClientHostHandler {
    fn on_host_call(&mut self, name: &str, args: &[VMValue]) -> Option<VMValue> {
        match name {
            "action" => {
                if let Some(s) = args.get(0).and_then(|v| v.as_string()) {
                    if let Ok(parsed) = EntityAction::from_str(s) {
                        self.action = Some(parsed);
                    }
                }
            }
            "intent" => {
                if let Some(s) = args.get(0).and_then(|v| v.as_string()) {
                    self.action = Some(EntityAction::Intent(s.to_string()));
                }
            }
            _ => {}
        }
        None
    }
}

pub struct ClientAction {
    vm: VM,
    class_name: String,
    exec: Execution,
    program: Option<Program>,
}

impl Default for ClientAction {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientAction {
    pub fn new() -> Self {
        Self {
            vm: VM::default(),
            class_name: String::new(),
            exec: Execution::new(0),
            program: None,
        }
    }

    /// Init
    pub fn init(&mut self, class_name: String, assets: &Assets) {
        if let Some((entity_source, _)) = assets.entities.get(&class_name) {
            let result = self.vm.prepare_str(entity_source);
            match result {
                Ok(program) => {
                    self.exec.reset(program.globals);
                    self.program = Some(program);
                }
                Err(e) => {
                    eprintln!("Client: error compiling user_event: {}", e)
                }
            }
            self.class_name = class_name;
        }
    }

    /// Execute the user event
    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        if let Some(program) = &self.program {
            if let Some(index) = program.user_functions_name_map.get("user_event").copied() {
                self.exec.reset(program.globals);
                let mut handler = ClientHostHandler::default();
                let args = [VMValue::from_string(event), VMValue::from_value(&value)];
                let _ = self
                    .exec
                    .execute_function_host(&args, index, program, &mut handler);

                if let Some(act) = handler.action {
                    return act;
                }
            }
        }

        EntityAction::Off
    }
}
