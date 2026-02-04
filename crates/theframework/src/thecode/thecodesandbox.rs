use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TheCodeSandbox {
    /// The id of the sandbox.
    pub id: Uuid,

    /// The packages with callable codegrid functions.
    #[serde(skip)]
    pub packages: FxHashMap<Uuid, TheCodePackage>,

    /// The objects with values. These make up the state of an entity.
    pub objects: FxHashMap<Uuid, TheCodeObject>,

    /// The items with values. These make up the state of an entity.
    pub items: FxHashMap<Uuid, TheCodeObject>,

    /// The areas with values. These make up the state of an entity.
    pub areas: FxHashMap<Uuid, TheCodeObject>,

    /// Debug switch.
    pub debug_mode: bool,

    // Runtimes
    /// Redirects object aliases (like Self, Target etc.) to a given Uuid.
    #[serde(skip)]
    pub aliases: FxHashMap<String, Uuid>,

    /// Function return value.
    #[serde(skip)]
    pub func_rc: Option<TheValue>,

    /// The call stack of modules.
    #[serde(skip)]
    pub module_stack: Vec<Uuid>,

    /// The call stack of the codegrid source of the module.
    #[serde(skip)]
    pub codegrid_stack: Vec<Uuid>,

    /// The call stack of functions.
    #[serde(skip)]
    pub call_stack: Vec<TheCodeFunction>,

    /// The call stack of functions.
    #[serde(skip)]
    pub debug_modules: FxHashMap<Uuid, TheDebugModule>,

    /// The debug messages.
    #[serde(skip)]
    pub debug_messages: Vec<TheDebugMessage>,
}

impl Default for TheCodeSandbox {
    fn default() -> Self {
        TheCodeSandbox::new()
    }
}

impl TheCodeSandbox {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            objects: FxHashMap::default(),
            items: FxHashMap::default(),
            areas: FxHashMap::default(),

            packages: FxHashMap::default(),

            debug_mode: false,

            aliases: FxHashMap::default(),

            func_rc: None,
            module_stack: vec![],
            call_stack: vec![],
            codegrid_stack: vec![],
            debug_modules: FxHashMap::default(),
            debug_messages: vec![],
        }
    }

    /// Clear the runtime states.
    pub fn clear_runtime_states(&mut self) {
        self.aliases = FxHashMap::default();
        self.func_rc = None;
        self.call_stack = vec![];
        self.module_stack = vec![];
        self.debug_modules = FxHashMap::default();
        self.codegrid_stack = vec![];
    }

    /// Clear the debug messages.
    pub fn clear_debug_messages(&mut self) {
        self.debug_messages = vec![];
    }

    /// Insert a module into the environment.
    pub fn insert_package(&mut self, package: TheCodePackage) {
        self.packages.insert(package.id, package);
    }

    /// Add an object into the sandbox.
    pub fn add_object(&mut self, object: TheCodeObject) {
        self.objects.insert(object.id, object);
    }

    /// Add an item into the sandbox.
    pub fn add_item(&mut self, item: TheCodeObject) {
        self.items.insert(item.id, item);
    }

    /// Add an area into the sandbox.
    pub fn add_area(&mut self, area: TheCodeObject) {
        self.areas.insert(area.id, area);
    }

    /// Get a clone of the module from the environment. The module is identified by the package id and the codegrid id the module is based on.
    pub fn get_package_module_cloned(
        &self,
        package_id: &Uuid,
        codegrid_id: &Uuid,
    ) -> Option<TheCodeModule> {
        if let Some(package) = self.packages.get(package_id) {
            if let Some(module) = package.get_function_codegrid(codegrid_id) {
                return Some(module.clone());
            }
        }
        None
    }

    /// Returns the given local variable by reversing the local stack.
    pub fn get_local(&self, name: &String) -> Option<&TheValue> {
        if let Some(function) = self.call_stack.last() {
            if let Some(var) = function.get_local(name) {
                return Some(var);
            }
        }
        None
    }

    /// Returns a reference to the aliased object.
    pub fn get_object(&self, name: &String) -> Option<&TheCodeObject> {
        if let Some(id) = self.aliases.get(name) {
            if let Some(object) = self.objects.get(id) {
                return Some(object);
            }
        }
        None
    }

    /// Returns a mutable reference to the aliased object.
    pub fn get_object_mut(&mut self, name: &String) -> Option<&mut TheCodeObject> {
        if let Some(id) = self.aliases.get(name) {
            if let Some(object) = self.objects.get_mut(id) {
                return Some(object);
            } else if let Some(item) = self.items.get_mut(id) {
                return Some(item);
            } else if let Some(area) = self.areas.get_mut(id) {
                return Some(area);
            }
        }
        None
    }

    /// Returns a mutable reference to the current object with an alias of "self".
    pub fn get_self_mut(&mut self) -> Option<&mut TheCodeObject> {
        if let Some(id) = self.aliases.get("self") {
            if let Some(object) = self.objects.get_mut(id) {
                return Some(object);
            } else if let Some(item) = self.items.get_mut(id) {
                return Some(item);
            } else if let Some(area) = self.areas.get_mut(id) {
                return Some(area);
            }
        }
        None
    }

    /// Returns a mutable reference to the current object with an alias of "target".
    pub fn get_target_mut(&mut self) -> Option<&mut TheCodeObject> {
        if let Some(id) = self.aliases.get("target") {
            if let Some(object) = self.objects.get_mut(id) {
                return Some(object);
            }
        }
        None
    }

    /// Returns a mutable reference to the current area with an alias of "self".
    pub fn get_self_area_mut(&mut self) -> Option<&mut TheCodeObject> {
        if let Some(id) = self.aliases.get("self") {
            if let Some(area) = self.areas.get_mut(id) {
                return Some(area);
            }
        }
        None
    }

    /// Returns a mutable reference to the current item with an alias of "self".
    pub fn get_self_item_mut(&mut self) -> Option<&mut TheCodeObject> {
        if let Some(id) = self.aliases.get("self") {
            if let Some(item) = self.items.get_mut(id) {
                return Some(item);
            }
        }
        None
    }

    /// Pushes the current module to the module stack.
    pub fn push_current_module(&mut self, module_id: Uuid, codegrid_id: Uuid) {
        self.module_stack.push(module_id);
        self.codegrid_stack.push(codegrid_id);
        let debug_module = TheDebugModule {
            codegrid_id,
            ..TheDebugModule::default()
        };
        self.debug_modules.insert(module_id, debug_module);
    }

    /// Pops the current module from the module stack.
    pub fn pop_current_module(&mut self) {
        self.module_stack.pop();
        self.codegrid_stack.pop();
    }

    /// Sets a debug value in the current module. An optional top value and a required bottom value.
    /// The top value is used for optional progess debug values while the bottom value is
    /// the actual result value for the location.
    pub fn set_debug_value(&mut self, location: (u16, u16), value: (Option<TheValue>, TheValue)) {
        if let Some(module_id) = self.module_stack.last() {
            if let Some(debug_module) = self.debug_modules.get_mut(module_id) {
                debug_module.values.insert(location, value);
            }
        }
    }

    /// Marks the given location as executed.
    pub fn set_debug_executed(&mut self, location: (u16, u16)) {
        if let Some(module_id) = self.module_stack.last() {
            if let Some(debug_module) = self.debug_modules.get_mut(module_id) {
                debug_module.executed.insert(location);
            }
        }
    }

    /// Returns the debug values for a given module id.
    pub fn get_module_debug_module(&self, module_id: Uuid) -> TheDebugModule {
        if let Some(dv) = self.debug_modules.get(&module_id) {
            dv.clone()
        } else {
            TheDebugModule::default()
        }
    }

    /// Returns the debug values for a given entity id.
    pub fn get_codegrid_debug_module(&self, entity_id: Uuid) -> TheDebugModule {
        for (index, id) in self.codegrid_stack.iter().enumerate() {
            if *id == entity_id {
                if let Some(module_id) = self.module_stack.get(index) {
                    if let Some(dv) = self.debug_modules.get(module_id) {
                        return dv.clone();
                    }
                }
            }
        }
        TheDebugModule::default()
    }

    /// Add a debug message
    pub fn add_debug_message(&mut self, message: String) {
        let mut debug_message = TheDebugMessage::new(TheDebugMessageRole::Debug, message);

        if let Some(object) = self.get_self_mut() {
            if let Some(name) = object.get(&"name".to_string()) {
                debug_message.entity = name.describe();
            }
        }

        self.debug_messages.push(debug_message);
    }

    /// Create an instance from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }

    /// Convert the instance to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TheDebugModule {
    pub codegrid_id: Uuid,
    pub values: FxHashMap<(u16, u16), (Option<TheValue>, TheValue)>,
    pub executed: FxHashSet<(u16, u16)>,
}

impl Default for TheDebugModule {
    fn default() -> Self {
        TheDebugModule::new()
    }
}

impl TheDebugModule {
    pub fn new() -> Self {
        Self {
            codegrid_id: Uuid::nil(),
            values: FxHashMap::default(),
            executed: FxHashSet::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TheDebugMessageRole {
    /// The message is a status message.
    Status,
    /// The message is a debug message.
    Debug,
    /// The message is a warning.
    Warning,
    /// The message is an error.
    Error,
}

/// TheDebugMessage
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TheDebugMessage {
    pub role: TheDebugMessageRole,
    pub message: String,
    pub module: String,
    pub entity: String,
}

impl TheDebugMessage {
    pub fn new(role: TheDebugMessageRole, message: String) -> Self {
        Self {
            role,
            message,
            module: "".to_string(),
            entity: "".to_string(),
        }
    }
}
