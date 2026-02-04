use crate::prelude::*;

/// TheCodePackage is a collection of modules and is the compiled equivalent of a TheCodeBundle.
#[derive(Clone, Debug)]
pub struct TheCodePackage {
    pub name: String,
    pub id: Uuid,
    pub modules: FxHashMap<String, TheCodeModule>,
}

impl Default for TheCodePackage {
    fn default() -> Self {
        TheCodePackage::new()
    }
}

impl TheCodePackage {
    pub fn new() -> Self {
        Self {
            name: "Unnamed".to_string(),
            id: Uuid::new_v4(),
            modules: FxHashMap::default(),
        }
    }

    /// Insert a module into the package.
    pub fn insert_module(&mut self, name: String, module: TheCodeModule) {
        self.modules.insert(name, module);
    }

    /// Get a module from the package.
    pub fn get_module(&self, name: &String) -> Option<&TheCodeModule> {
        self.modules.get(name)
    }

    /// Get a module from the package based on the codegrid id.
    pub fn get_function_codegrid(&self, codegrid_id: &Uuid) -> Option<&TheCodeModule> {
        self.modules
            .values()
            .find(|module| module.codegrid_id == *codegrid_id)
    }

    /// Get a mutable module from the package based on the codegrid id.
    pub fn get_function_codegrid_mut(&mut self, codegrid_id: &Uuid) -> Option<&mut TheCodeModule> {
        self.modules
            .values_mut()
            .find(|module| module.codegrid_id == *codegrid_id)
    }

    /// Get a mutable m module from the package.
    pub fn get_function_mut(&mut self, name: &String) -> Option<&mut TheCodeModule> {
        self.modules.get_mut(name)
    }

    /// Execute the module of the given name.
    pub fn execute(&mut self, name: String, sandbox: &mut TheCodeSandbox) {
        if let Some(module) = self.modules.get_mut(&name) {
            module.execute(sandbox);
        }
    }
}
