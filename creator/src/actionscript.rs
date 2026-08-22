use rusterix::vm::{Execution, HostHandler, VM, VMValue};

pub const EDITOR_ACTION_FUNCTION: &str = "editor_action";
pub const EDITOR_TOOL_FUNCTION: &str = "editor_tool";

/// A validated command emitted by an Eldrin editor-action script.
///
/// Parameters use the same TOML representation as the sidebar action editor. This keeps the
/// scripting bridge independent of localized labels and avoids inventing a second parameter
/// format while Eldrin values are still intentionally small.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorActionRequest {
    pub command_id: String,
    pub parameters_toml: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditorAutomationRequest {
    Action(EditorActionRequest),
    SelectTool { command_id: String },
}

#[derive(Default)]
struct EditorActionHost {
    requests: Vec<EditorAutomationRequest>,
    error: Option<String>,
}

impl HostHandler for EditorActionHost {
    fn on_host_call(&mut self, name: &str, args: &[VMValue]) -> Option<VMValue> {
        match name {
            EDITOR_ACTION_FUNCTION => {
                let Some(command_id) = args.first().and_then(VMValue::as_string) else {
                    self.error = Some("editor_action command id must be a string".to_string());
                    return Some(VMValue::from_bool(false));
                };
                let Some(parameters_toml) = args.get(1).and_then(VMValue::as_string) else {
                    self.error = Some("editor_action parameters must be a TOML string".to_string());
                    return Some(VMValue::from_bool(false));
                };

                self.requests
                    .push(EditorAutomationRequest::Action(EditorActionRequest {
                        command_id: command_id.to_string(),
                        parameters_toml: parameters_toml.to_string(),
                    }));
                Some(VMValue::from_bool(true))
            }
            EDITOR_TOOL_FUNCTION => {
                let Some(command_id) = args.first().and_then(VMValue::as_string) else {
                    self.error = Some("editor_tool command id must be a string".to_string());
                    return Some(VMValue::from_bool(false));
                };
                self.requests.push(EditorAutomationRequest::SelectTool {
                    command_id: command_id.to_string(),
                });
                Some(VMValue::from_bool(true))
            }
            _ => None,
        }
    }
}

/// Parses and executes the pure Eldrin portion of an editor-action script.
///
/// The host only records requests. Creator applies them afterwards on the UI thread through the
/// normal action implementation, preserving applicability checks and undo behavior.
pub fn collect_editor_automation_requests(
    source: &str,
) -> Result<Vec<EditorAutomationRequest>, String> {
    let mut vm = VM::default();
    vm.register_host_function(EDITOR_ACTION_FUNCTION, 2)?;
    vm.register_host_function(EDITOR_TOOL_FUNCTION, 1)?;
    let program = vm.prepare_str(source).map_err(|error| error.to_string())?;
    let mut execution = Execution::new(program.globals);
    let mut host = EditorActionHost::default();
    execution.execute_host(&program.body, &program, &mut host);

    if let Some(error) = host.error {
        Err(error)
    } else {
        Ok(host.requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eldrin_collects_ordered_editor_action_requests() {
        let requests = collect_editor_automation_requests(
            r#"
                editor_tool("tool.geometry");
                editor_action("camera.isometric", "");
                editor_action("face.extrude", "amount = 2");
            "#,
        )
        .unwrap();

        assert_eq!(
            requests,
            vec![
                EditorAutomationRequest::SelectTool {
                    command_id: "tool.geometry".to_string(),
                },
                EditorAutomationRequest::Action(EditorActionRequest {
                    command_id: "camera.isometric".to_string(),
                    parameters_toml: String::new(),
                }),
                EditorAutomationRequest::Action(EditorActionRequest {
                    command_id: "face.extrude".to_string(),
                    parameters_toml: "amount = 2".to_string(),
                }),
            ]
        );
    }

    #[test]
    fn eldrin_rejects_non_string_editor_action_arguments() {
        let error =
            collect_editor_automation_requests(r#"editor_action("face.extrude", 2);"#).unwrap_err();
        assert!(error.contains("TOML string"));
    }
}
