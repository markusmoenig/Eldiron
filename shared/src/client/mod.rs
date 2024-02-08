use crate::prelude::*;
use theframework::prelude::*;

#[derive()]
pub struct Client {
    project: Project,

    tiledrawer: TileDrawer,
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
            tiledrawer: TileDrawer::new(),
            // compiler: TheCompiler::new(),
        }
    }

    /// Sets the project
    pub fn set_project(&mut self, project: Project) {
        self.tiledrawer.tiles = project.extract_tiles();
        self.project = project;
    }

    pub fn set_region_update(&mut self, _region_update: RegionUpdate) {

    }

    pub fn draw(
        &mut self,
        _buffer: &mut TheRGBABuffer,
    ) {
    }
}