use crate::prelude::*;

/// TheCodeModule is a compiled output of a TheCodeGrid source.
#[derive(Clone, Debug)]
pub struct TheCodeModule {
    pub name: String,
    pub id: Uuid,
    /// The id of the codegrid that was used to compile this module.
    pub codegrid_id: Uuid,
    pub function: TheCodeFunction,
}

impl Default for TheCodeModule {
    fn default() -> Self {
        TheCodeModule::new()
    }
}

impl TheCodeModule {
    pub fn new() -> Self {
        Self {
            name: "Unnamed".to_string(),
            id: Uuid::new_v4(),
            codegrid_id: Uuid::nil(),
            function: TheCodeFunction::default(),
        }
    }

    /// Insert a function into the module.
    pub fn set_function(&mut self, function: TheCodeFunction) {
        self.function = function;
    }

    /// Get the function from the module.
    pub fn get_function(&self) -> &TheCodeFunction {
        &self.function
    }

    /// Get the function as mutable from the module.
    pub fn get_function_mut(&mut self) -> &mut TheCodeFunction {
        &mut self.function
    }

    /// Execute the module.
    pub fn execute(&mut self, sandbox: &mut TheCodeSandbox) -> Vec<TheValue> {
        let clone = self.function.clone();

        sandbox.push_current_module(self.id, self.codegrid_id);
        sandbox.call_stack.push(clone);

        let rc = self.function.execute(sandbox);
        sandbox.call_stack.pop();
        sandbox.pop_current_module();

        rc
    }
}
