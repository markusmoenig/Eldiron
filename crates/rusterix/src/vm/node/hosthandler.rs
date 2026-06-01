use crate::vm::{NodeOp, VMValue};

/// Host handler invoked for VM ops that need to touch external context.
pub trait HostHandler {
    fn on_host_call(&mut self, _name: &str, _args: &[VMValue]) -> Option<VMValue> {
        None
    }

    fn on_debug_line(&mut self, _line: usize) {}

    fn on_debug_value(&mut self, _line: usize, _name: &str, _value: &VMValue) {}

    fn on_debug_branch(&mut self, _line: usize, _taken: bool) {}

    /// Dispatch a NodeOp that targets the host layer. Returns true if handled.
    fn handle_host_op(&mut self, op: &NodeOp, stack: &mut Vec<VMValue>) -> bool {
        match op {
            NodeOp::DebugLine(line) => {
                self.on_debug_line(*line);
                true
            }
            NodeOp::DebugValue { line, name } => {
                if let Some(value) = stack.last() {
                    self.on_debug_value(*line, name, value);
                }
                true
            }
            NodeOp::HostCall { name, argc } => {
                let mut args = Vec::with_capacity(*argc as usize);
                for _ in 0..*argc as usize {
                    if let Some(v) = stack.pop() {
                        args.push(v);
                    }
                }
                args.reverse();
                if let Some(ret) = self.on_host_call(name, &args) {
                    stack.push(ret);
                }
                true
            }
            _ => false,
        }
    }
}
