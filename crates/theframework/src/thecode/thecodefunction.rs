use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct TheCodeFunction {
    pub name: String,
    pub nodes: Vec<TheCodeNode>,
    pub local: Vec<TheCodeObject>,

    pub arguments: Vec<String>,
}

impl Default for TheCodeFunction {
    fn default() -> Self {
        TheCodeFunction::new()
    }
}

impl TheCodeFunction {
    pub fn new() -> Self {
        Self {
            name: "main".to_string(),
            local: vec![TheCodeObject::default()],
            nodes: vec![],
            arguments: vec![],
        }
    }

    pub fn named(name: String) -> Self {
        Self {
            name,
            local: vec![TheCodeObject::default()],
            nodes: vec![],
            arguments: vec![],
        }
    }

    /// Add a node.
    pub fn add_node(&mut self, node: TheCodeNode) {
        self.nodes.push(node);
    }

    /// Returns true if the function is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the given local variable by reversing the local stack.
    pub fn get_local(&self, name: &String) -> Option<&TheValue> {
        for local in self.local.iter().rev() {
            if let Some(var) = local.get(name) {
                return Some(var);
            }
        }
        None
    }

    /// Sets a local variable.
    pub fn set_local(&mut self, name: String, value: TheValue) {
        if let Some(f) = self.local.last_mut() {
            f.set(name, value);
        }
    }

    /// Execute the function
    pub fn execute(&mut self, sandbox: &mut TheCodeSandbox) -> Vec<TheValue> {
        let mut stack: Vec<TheValue> = Vec::with_capacity(10);

        for n in &mut self.nodes {
            //println!("{:?}", stack);
            let rc = (n.call)(&mut stack, &mut n.data, sandbox);
            if rc == TheCodeNodeCallResult::Break {
                break;
            }
        }

        stack
    }
}
