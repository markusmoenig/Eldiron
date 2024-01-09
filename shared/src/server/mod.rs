use crate::prelude::*;
use lazy_static::lazy_static;
use std::sync::{mpsc, Mutex, RwLock};
use theframework::prelude::*;

pub mod context;
pub mod region_instance;
pub mod world;

pub mod prelude {
    pub use super::context::ServerContext;
    pub use super::region_instance::RegionInstance;
    pub use super::world::World;
    pub use super::Server;
}

lazy_static! {
    pub static ref REGIONS: RwLock<FxHashMap<Uuid, Region>> = RwLock::new(FxHashMap::default());
    pub static ref RNG: Mutex<rand::rngs::StdRng> = Mutex::new(rand::rngs::StdRng::from_entropy());
    pub static ref TILES: RwLock<FxHashMap<Uuid, TheRGBATile>> = RwLock::new(FxHashMap::default());
}

use prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ServerState {
    Running,
    Stopped,
    Paused,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Server {
    pub state: ServerState,

    project: Project,
    #[serde(skip)]
    compiler: TheCompiler,

    instances: FxHashMap<Uuid, RegionInstance>,

    #[serde(skip)]
    characters: FxHashMap<Uuid, TheCodePackage>,

    pub debug_mode: bool,
    pub world: World,

    pub anim_counter: usize,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            state: ServerState::Stopped,

            project: Project::default(),
            compiler: TheCompiler::new(),

            instances: FxHashMap::default(),

            characters: FxHashMap::default(),

            debug_mode: false,
            world: World::default(),

            anim_counter: 0,
        }
    }

    /// Sets the current project. Resets the server.
    pub fn set_project(&mut self, project: Project) {
        let mut regions = FxHashMap::default();
        for region in &project.regions {
            regions.insert(region.id, region.clone());
        }

        self.characters = FxHashMap::default();

        *REGIONS.write().unwrap() = regions;
        *TILES.write().unwrap() = project.extract_tiles();

        self.world.reset();
        self.anim_counter = 0;

        self.setup_regions(&project);
        self.project = project;
    }

    /// Setup all regions in the project and create their instances.
    pub fn setup_regions(&mut self, project: &Project) {
        self.instances = FxHashMap::default();

        /*
        let (sender, receiver) = mpsc::channel();
        let mut join_handles = vec![];

        for region in &self.project.regions {//region_map.drain() {
            let sender = sender.clone();

            let uuid = region.id;
            let mut instance = RegionInstance::new();
            let project = self.project.clone();

            let handle = std::thread::spawn(move || {
                instance.setup(uuid, &project);
                sender.send((uuid, instance)).unwrap();
            });

            join_handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in join_handles {
            handle.join().unwrap();
        }

        drop(sender);
        for (key, instance) in receiver {
            self.instances.insert(key, instance);
        }*/

        // Syncronous version. Slower but has the advantage not to clone project for each thread.

        // First pass we just create the region instances.
        for region in &project.regions {
            let uuid = region.id;
            let mut instance = RegionInstance::new();

            instance.set_debug_mode(self.debug_mode);
            instance.setup(uuid, project);

            self.instances.insert(uuid, instance);
        }

        // Add all characters
        for bundle in project.characters.values() {
            self.insert_character(bundle.clone());
        }

        // Second pass we just create the region character instances.
        for region in &project.regions {
            for character in region.characters.values() {
                self.add_character_instance_to_region(region.id, character.clone());
            }
        }
    }

    /// Starts the server.
    pub fn start(&mut self) {
        self.state = ServerState::Running;
    }

    /// Stops the server.
    pub fn stop(&mut self) {
        self.state = ServerState::Stopped;
    }

    /// Tick. Compute the next frame.
    pub fn tick(&mut self) {
        self.world.tick();
        self.anim_counter = self.anim_counter.wrapping_add(1);

        let (sender, receiver) = mpsc::channel();
        let mut join_handles = vec![];

        for (key, mut instance) in self.instances.drain() {
            let sender = sender.clone();
            let handle = std::thread::spawn(move || {
                instance.tick();
                sender.send((key, instance)).unwrap();
            });

            join_handles.push(handle);
        }

        for handle in join_handles {
            handle.join().unwrap();
        }

        drop(sender);
        for (key, instance) in receiver {
            self.instances.insert(key, instance);
        }
    }

    /// Updates the tiles in the server. Called after live tilemap updates from the editor.
    pub fn update_tiles(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        *TILES.write().unwrap() = tiles;
    }

    /// Update the region instance for the region. Called after live updates from the editor.
    pub fn update_region(&mut self, region: &Region) {
        if let Ok(r) = &mut REGIONS.write() {
            r.insert(region.id, region.clone());
        }
    }

    /// Draws the given region instance into the given buffer. This drawing routine is only used by the editor.
    pub fn draw_region(
        &mut self,
        uuid: &Uuid,
        buffer: &mut TheRGBABuffer,
        tiledrawer: &TileDrawer,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if let Some(instance) = self.instances.get_mut(uuid) {
            instance.draw(buffer, tiledrawer, &self.anim_counter, ctx, server_ctx);
        }
    }

    /// Add a new character (TheCodeBundle) to the server.
    pub fn insert_character(&mut self, mut character: TheCodeBundle) {
        let mut package = TheCodePackage::new();
        package.id = character.id;

        for grid in character.grids.values_mut() {
            let rc = self.compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name = grid.name.clone();
                println!(
                    "RegionInstance::add_character: Compiled grid module: {}",
                    grid.name
                );
                package.insert_module(module.name.clone(), module);
            } else {
                println!(
                    "RegionInstance::add_character: Failed to compile grid: {}",
                    grid.name
                );
            }
        }

        for instance in self.instances.values_mut() {
            instance.insert_character(package.clone());
        }

        self.characters.insert(package.id, package);
    }

    /// Get the debug module for the given module id.
    pub fn get_region_debug_module(&mut self, region: Uuid, module_id: Uuid) -> TheDebugModule {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.get_module_debug_module(module_id)
        } else {
            TheDebugModule::default()
        }
    }

    /// Get the debug module for the given entity id.
    pub fn get_region_debug_codegrid(&mut self, region: Uuid, entity_id: Uuid) -> TheDebugModule {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.get_codegrid_debug_module(entity_id)
        } else {
            TheDebugModule::default()
        }
    }

    /// Adds a new character instance to the given region and returns its module id (for debugging).
    pub fn add_character_instance_to_region(
        &mut self,
        region: Uuid,
        character: Character,
    ) -> Option<Uuid> {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.add_character_instance(character)
        } else {
            None
        }
    }

    /// Updates a character instance.
    pub fn update_character_instance_bundle(
        &mut self,
        region: Uuid,
        character: Uuid,
        bundle: TheCodeBundle,
    ) {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.update_character_instance_bundle(character, bundle);
        }
    }

    /// Remove the character instance from the given region.
    pub fn remove_character_instance(&mut self, region: Uuid, character: Uuid) {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.remove_character_instance(character);
        }
    }

    /// Returns the character instance id and the character id for the character at the given position for the given region.
    pub fn get_character_at(&self, region: Uuid, pos: Vec2i) -> Option<(Uuid, Uuid)> {
        if let Some(instance) = self.instances.get(&region) {
            instance.get_character_at(pos)
        } else {
            None
        }
    }

    /// Returns the property of the character instance for the given region along with its character id.
    pub fn get_character_property(
        &self,
        region: Uuid,
        character_id: Uuid,
        property: String,
    ) -> Option<(TheValue, Uuid)> {
        if let Some(instance) = self.instances.get(&region) {
            instance.get_character_property(character_id, property)
        } else {
            None
        }
    }
}
