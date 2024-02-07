use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Client {
    project: Project,
    // #[serde(skip)]
    // compiler: TheCompiler,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            project: Project::default(),
            // compiler: TheCompiler::new(),
        }
    }

    /// Sets the project
    pub fn set_project(&mut self, project: Project) {
        self.project = project;
    }
}