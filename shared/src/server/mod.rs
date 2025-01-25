use crate::prelude::*;
use lazy_static::lazy_static;
use std::sync::{mpsc, RwLock};
use theframework::prelude::*;

pub mod daylight;
pub mod execute;
pub mod functions;
pub mod region_instance;
pub mod world;

pub mod prelude {
    pub use super::daylight::Daylight;
    pub use super::execute::*;
    pub use super::region_instance::RegionInstance;
    pub use super::world::World;
    pub use super::{ActivePlayer, Server};
}

lazy_static! {
    pub static ref REGIONS: RwLock<FxHashMap<Uuid, Region>> = RwLock::new(FxHashMap::default());
    // pub static ref RNG: Mutex<rand::rngs::StdRng> = Mutex::new(rand::rngs::StdRng::from_entropy());
    pub static ref TILES: RwLock<FxHashMap<Uuid, TheRGBATile>> = RwLock::new(FxHashMap::default());
    pub static ref KEY_DOWN: RwLock<Option<String>> = RwLock::new(None);
    pub static ref UPDATES: RwLock<FxHashMap<Uuid, RegionUpdate>> =
        RwLock::new(FxHashMap::default());
    pub static ref ITEMS: RwLock<FxHashMap<Uuid, TheCodePackage>> =
        RwLock::new(FxHashMap::default());
    pub static ref CHARACTERS: RwLock<FxHashMap<Uuid, TheCodePackage>> =
        RwLock::new(FxHashMap::default());
    pub static ref PLAYERCHARACTERS: RwLock<FxHashMap<Uuid, String>> =
        RwLock::new(FxHashMap::default());
    pub static ref INTERACTIONS: RwLock<Vec<Interaction>> = RwLock::new(Vec::default());
    pub static ref SENDCMD: RwLock<mpsc::Sender<ServerMessage>> = {
        let (tx, _rx) = mpsc::channel::<ServerMessage>();
        RwLock::new(tx)
    };
}

use prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ServerState {
    Running,
    Stopped,
    Paused,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ActivePlayer {
    pub id: Uuid,
    pub region_id: Uuid,
}

// Default function for server_messages
fn default_receiver() -> mpsc::Receiver<ServerMessage> {
    let (_tx, rx) = mpsc::channel();
    rx
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
    pub state: ServerState,

    project: Project,
    #[serde(skip)]
    compiler: TheCompiler,

    /// The region instances
    instances: FxHashMap<Uuid, RegionInstance>,

    pub debug_mode: bool,
    pub world: World,

    pub anim_counter: usize,

    pub active_players: FxHashMap<Uuid, ActivePlayer>,

    #[serde(skip)]
    #[serde(default = "default_receiver")]
    pub server_messages: mpsc::Receiver<ServerMessage>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        let mut compiler: TheCompiler = TheCompiler::default();
        functions::add_compiler_functions(&mut compiler);

        let (tx, rx): (mpsc::Sender<ServerMessage>, mpsc::Receiver<ServerMessage>) =
            mpsc::channel();
        *SENDCMD.write().unwrap() = tx;

        Self {
            state: ServerState::Stopped,

            project: Project::default(),
            compiler,

            instances: FxHashMap::default(),

            debug_mode: false,
            world: World::default(),

            anim_counter: 0,

            active_players: FxHashMap::default(),

            server_messages: rx,
        }
    }

    /// Retrieves all messages for the clients.
    pub fn get_client_messages(&self) -> Vec<ServerMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = self.server_messages.try_recv() {
            messages.push(message);
        }
        messages
    }

    /// Returns a mutable reference to the compiler.
    pub fn compiler(&mut self) -> &mut TheCompiler {
        &mut self.compiler
    }

    /// Sets the current project. Resets the server.
    pub fn set_project(&mut self, project: Project) -> FxHashMap<Uuid, TheCodePackage> {
        let mut regions = FxHashMap::default();
        let mut updates = FxHashMap::default();
        for region in &project.regions {
            regions.insert(region.id, region.clone());
            updates.insert(
                region.id,
                RegionUpdate {
                    id: region.id,
                    ..Default::default()
                },
            );
        }

        *CHARACTERS.write().unwrap() = FxHashMap::default();
        *ITEMS.write().unwrap() = FxHashMap::default();

        *REGIONS.write().unwrap() = regions;
        *UPDATES.write().unwrap() = updates;
        *TILES.write().unwrap() = project.extract_tiles();

        self.world.reset();
        self.anim_counter = 0;

        let packages = self.compile_bundles(project.codes.clone());
        self.compiler.set_packages(packages.clone());

        self.setup_regions(&project, &packages);

        self.project = project;

        packages
    }

    /// Setup all regions in the project and create their instances.
    pub fn setup_regions(&mut self, project: &Project, packages: &FxHashMap<Uuid, TheCodePackage>) {
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
            instance.setup(uuid, project, packages);

            for area in region.areas.values() {
                instance.insert_area(area.clone(), &mut self.compiler);
            }

            instance.set_time(self.world.time);
            self.instances.insert(uuid, instance);
        }

        // Add all characters
        // for bundle in project.characters.values() {
        //     //self.insert_character(bundle.clone());
        // }

        // .. and items
        // for bundle in project.items.values() {
        //     //self.insert_item(bundle.clone());
        // }

        // Second pass we just create the region character and item instances.
        for region in &project.regions {
            for character in region.characters.values() {
                // Only add non player characters. Player characters have to be instantiated by the client.
                if !PLAYERCHARACTERS
                    .read()
                    .unwrap()
                    .contains_key(&character.character_id)
                {
                    self.add_character_instance_to_region(region.id, character.clone(), None);
                }
            }
            for item in region.items.values() {
                self.add_item_instance_to_region(region.id, item.clone());
            }
        }
    }

    /// Compiles all bundles into packages.
    fn compile_bundles(
        &mut self,
        mut bundles: FxHashMap<Uuid, TheCodeBundle>,
    ) -> FxHashMap<Uuid, TheCodePackage> {
        let mut packages = FxHashMap::default();
        for bundle in bundles.values_mut() {
            let mut package = TheCodePackage::new();
            package.name.clone_from(&bundle.name);
            package.id = bundle.id;

            for grid in bundle.grids.values_mut() {
                let rc = self.compiler.compile(grid);
                if let Ok(mut module) = rc {
                    module.name.clone_from(&grid.name);
                    package.insert_module(module.name.clone(), module);
                } else if let Err(e) = rc {
                    println!(
                        "Error in {}.{}: {} at {:?}.",
                        bundle.name, grid.name, e.message, e.location
                    );
                }
            }
            packages.insert(package.id, package);
        }

        packages
    }

    /// Updates a code bundle and provides it to all instances and the compiler.
    pub fn update_bundle(&mut self, mut bundle: TheCodeBundle) {
        let mut package = TheCodePackage::new();
        package.name.clone_from(&bundle.name);
        package.id = bundle.id;

        for grid in bundle.grids.values_mut() {
            let rc = self.compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name.clone_from(&grid.name);
                package.insert_module(module.name.clone(), module);
            } else if let Err(e) = rc {
                println!(
                    "Error in {}.{}: {} at {:?}.",
                    bundle.name, grid.name, e.message, e.location
                );
            }
        }

        // Update the package in all instances.
        for instance in self.instances.values_mut() {
            instance.update_package(package.clone());
        }

        self.compiler.update_package(package);
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
    pub fn tick(&mut self) -> Vec<TheDebugMessage> {
        self.world.tick();
        self.anim_counter = self.anim_counter.wrapping_add(1);
        INTERACTIONS.write().unwrap().clear();

        let (sender, receiver) = mpsc::channel();
        let mut join_handles = vec![];

        for (key, mut instance) in self.instances.drain() {
            let sender = sender.clone();
            let time = self.world.time;
            let handle = std::thread::spawn(move || {
                instance.tick(time);
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

        let mut debug_messages = vec![];

        for instance in self.instances.values_mut() {
            debug_messages.append(&mut instance.debug_messages());
        }

        // Update the server tick for all region updates.
        let mut updates = UPDATES.write().unwrap();
        for region_update in updates.values_mut() {
            region_update.server_tick = self.world.tick_counter
        }

        debug_messages
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
        self.set_time(self.world.time);
    }

    /// Draws the given region instance into the given buffer. This drawing routine is only used by the editor.
    pub fn draw_region(
        &mut self,
        uuid: &Uuid,
        buffer: &mut TheRGBABuffer,
        tiledrawer: &TileDrawer,
        server_ctx: &ServerContext,
        compute_delta: bool,
        offset: Vec2<i32>,
    ) {
        if let Some(instance) = self.instances.get_mut(uuid) {
            instance.draw(
                buffer,
                tiledrawer,
                &self.anim_counter,
                server_ctx,
                compute_delta,
                offset,
                &PALETTE.read().unwrap(),
            );
        }
    }

    /// Draw the region selections into the buffer.
    pub fn draw_region_selections(
        &mut self,
        uuid: &Uuid,
        buffer: &mut TheRGBABuffer,
        tiledrawer: &TileDrawer,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if let Some(instance) = self.instances.get_mut(uuid) {
            instance.draw_selections(buffer, tiledrawer, ctx, server_ctx);
        }
    }

    /// Renders the given region instance into the given buffer. This drawing routine is only used by the editor.
    ///
    pub fn render_region(
        &mut self,
        uuid: &Uuid,
        buffer: &mut TheRGBABuffer,
        renderer: &mut MapRender,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
        compute_delta: bool,
    ) {
        if let Some(instance) = self.instances.get_mut(uuid) {
            instance.render(
                buffer,
                renderer,
                &self.anim_counter,
                ctx,
                server_ctx,
                compute_delta,
            );
        }
    }

    /// Add a new character (TheCodeBundle) to the server.
    pub fn insert_character(&mut self, mut character: TheCodeBundle) -> Option<String> {
        let mut package = TheCodePackage::new();
        package.id = character.id;

        for grid in character.grids.values_mut() {
            let rc = self.compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name.clone_from(&grid.name);
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

        let mut name: Option<String> = None;

        if let Some(init) = package.get_function_mut(&"init".to_string()) {
            let mut sandbox = TheCodeSandbox::new();
            let mut object = TheCodeObject::new();
            object.id = character.id;
            sandbox.add_object(object);
            sandbox.aliases.insert("self".to_string(), character.id);
            init.execute(&mut sandbox);
            if let Some(object) = sandbox.get_self_mut() {
                if let Some(name_value) = object.get(&"name".to_string()) {
                    name = Some(name_value.describe());
                    if let Some(TheValue::Bool(v)) = object.get(&str!("player")) {
                        if *v {
                            PLAYERCHARACTERS
                                .write()
                                .unwrap()
                                .insert(character.id, name_value.describe());
                        }
                    }
                }
            }
        }

        for instance in self.instances.values_mut() {
            instance.insert_character(package.clone());
        }

        CHARACTERS.write().unwrap().insert(package.id, package);

        name
    }

    /// Add a new item (TheCodeBundle) to the server.
    pub fn insert_item(&mut self, mut item: TheCodeBundle) -> Option<String> {
        let mut package = TheCodePackage::new();
        package.id = item.id;

        for grid in item.grids.values_mut() {
            let rc = self.compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name.clone_from(&grid.name);
                println!(
                    "RegionInstance::insert_item: Compiled grid module: {}",
                    grid.name
                );
                package.insert_module(module.name.clone(), module);
            } else {
                println!(
                    "RegionInstance::insert_item: Failed to compile grid: {}",
                    grid.name
                );
            }
        }

        let mut name: Option<String> = None;

        if let Some(init) = package.get_function_mut(&"init".to_string()) {
            let mut sandbox = TheCodeSandbox::new();
            let mut object = TheCodeObject::new();
            object.id = item.id;
            sandbox.add_object(object);
            sandbox.aliases.insert("self".to_string(), item.id);
            init.execute(&mut sandbox);
            if let Some(object) = sandbox.get_self_mut() {
                if let Some(name_value) = object.get(&"name".to_string()) {
                    name = Some(name_value.describe());
                    package.name = name_value.describe();
                }
            }
        }

        for instance in self.instances.values_mut() {
            instance.insert_item(package.clone());
        }

        ITEMS.write().unwrap().insert(package.id, package);

        name
    }

    // Get the debug module for the given module id.
    // pub fn get_region_debug_module(&mut self, region: Uuid, module_id: Uuid) -> TheDebugModule {
    //     if let Some(instance) = self.instances.get_mut(&region) {
    //         instance.get_module_debug_module(module_id)
    //     } else {
    //         TheDebugModule::default()
    //     }
    // }

    /// Get the debug module for the given entity id.
    pub fn get_entity_debug_data(
        &mut self,
        region: Uuid,
        entity_id: Uuid,
    ) -> Option<FxHashMap<Uuid, TheDebugModule>> {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.get_entity_debug_data(entity_id)
        } else {
            None
        }
    }

    /// Adds a new character instance to the given region and returns its module id (for debugging).
    pub fn add_character_instance_to_region(
        &mut self,
        region: Uuid,
        character: Character,
        rename_to: Option<String>,
    ) -> Option<Uuid> {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.add_character_instance(character, &mut self.compiler, rename_to)
        } else {
            None
        }
    }

    /// Adds a new item instance to the given region and returns its module id (for debugging).
    pub fn add_item_instance_to_region(&mut self, region: Uuid, item: Item) -> Option<Uuid> {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.add_item_instance(item, &mut self.compiler)
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
            instance.update_character_instance_bundle(character, bundle, &mut self.compiler);
        }
    }

    /// Updates an item instance.
    pub fn update_item_instance_bundle(&mut self, region: Uuid, item: Uuid, bundle: TheCodeBundle) {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.update_item_instance_bundle(item, bundle, &mut self.compiler);
        }
    }

    /// Remove the character instance from the given region.
    pub fn remove_character_instance(&mut self, region: Uuid, character: Uuid) {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.remove_character_instance(character);
        }
    }

    /// Remove the item instance from the given region.
    pub fn remove_item_instance(&mut self, region: Uuid, item: Uuid) {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.remove_item_instance(item);
        }
    }

    /// Returns the character instance id and the character id for the character at the given position for the given region.
    pub fn get_character_at(&self, region: Uuid, pos: Vec2<i32>) -> Option<(Uuid, Uuid)> {
        if let Some(instance) = self.instances.get(&region) {
            instance.get_character_at(pos)
        } else {
            None
        }
    }

    /// Returns the item instance id and the item id for the item at the given position for the given region.
    pub fn get_item_at(&self, region: Uuid, pos: Vec2<i32>) -> Option<(Uuid, Uuid)> {
        if let Some(instance) = self.instances.get(&region) {
            instance.get_item_at(pos)
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

    /// Sets the property value of the character instance for the given region.
    pub fn set_character_property(
        &mut self,
        region: Uuid,
        character_id: Uuid,
        property: String,
        value: TheValue,
    ) {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.set_character_property(character_id, property, value);
        }
    }

    /// Returns the property of the item instance for the given region along with its item id.
    pub fn get_item_property(
        &self,
        region: Uuid,
        item_id: Uuid,
        property: String,
    ) -> Option<(TheValue, Uuid)> {
        if let Some(instance) = self.instances.get(&region) {
            instance.get_item_property(item_id, property)
        } else {
            None
        }
    }

    /// Returns the instance id of the character with the given name and the region id it is in.
    pub fn get_character_instance_info_by_name(&self, name: String) -> Option<(Uuid, Uuid)> {
        for (region, instance) in self.instances.iter() {
            if let Some(instance_id) = instance.get_character_instance_info_by_name(name.clone()) {
                return Some((*region, instance_id));
            }
        }
        None
    }

    /// Returns the object of the character instance for the given region along with its character id.
    pub fn get_character_object(
        &self,
        region: Uuid,
        character_id: Uuid,
    ) -> Option<(TheCodeObject, Uuid)> {
        if let Some(instance) = self.instances.get(&region) {
            instance.get_character_object(character_id)
        } else {
            None
        }
    }

    /// Insert the area into the given region.
    pub fn insert_area(&mut self, region: Uuid, area: Area) {
        if let Some(region) = REGIONS.write().unwrap().get_mut(&region) {
            if let Some(instance) = self.instances.get_mut(&region.id) {
                instance.insert_area(area.clone(), &mut self.compiler);
            }
            region.areas.insert(area.id, area);
        }
    }

    /// Remove the area from the given region.
    pub fn remove_area(&mut self, region: Uuid, area: Uuid) {
        // Remove the area from the instance.
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.remove_area(area);
        }
        // Remove the area data from the region.
        if let Some(region) = REGIONS.write().unwrap().get_mut(&region) {
            region.areas.remove(&area);
        }
    }

    /// Returns the draw settings of the given region instance.
    pub fn get_instance_draw_settings(&mut self, region: Uuid) -> RegionDrawSettings {
        if let Some(instance) = self.instances.get_mut(&region) {
            instance.draw_settings.clone()
        } else {
            RegionDrawSettings::new()
        }
    }

    /// Get the update for the given region as json.
    pub fn get_region_update_json(&self, region_id: Uuid) -> Option<String> {
        UPDATES
            .read()
            .unwrap()
            .get(&region_id)
            .map(|update| update.to_json())
    }

    /// Gets the server time.
    pub fn get_time(&self) -> TheTime {
        self.world.time
    }

    /// Sets the time of the server.
    pub fn set_time(&mut self, time: TheTime) {
        self.world.set_time(time);
        for instance in self.instances.values_mut() {
            instance.set_time(time);
        }
    }

    /// Sets the current 3d editing postion.
    pub fn set_editing_position_3d(&mut self, position: Vec3<f32>) {
        for region in REGIONS.write().unwrap().values_mut() {
            region.editing_position_3d = position;
        }
    }

    /// Get the current interactions.
    pub fn get_interactions(&self) -> Vec<Interaction> {
        INTERACTIONS.read().unwrap().clone()
    }

    /// Set an updated palette
    pub fn set_palette(&mut self, palette: &ThePalette) {
        for instance in self.instances.values_mut() {
            instance.palette = palette.clone();
        }
    }

    /// Set the current zoom level for the given region.
    pub fn set_zoom(&mut self, region: Uuid, zoom: f32) {
        if let Some(region) = REGIONS.write().unwrap().get_mut(&region) {
            region.zoom = zoom;
        }
    }

    /*
    pub fn clear_prerendered(&mut self, region: Uuid) {
        if let Some(region) = REGIONS.write().unwrap().get_mut(&region) {
            region.prerendered.clear();
        }
    }

    pub fn set_prerendered_tile(
        &mut self,
        region: Uuid,
        tile: &Vec2i,
        sample: u16,
        tile_data: &PreRenderedTileData,
    ) {
        if let Some(region) = REGIONS.write().unwrap().get_mut(&region) {
            region
                .prerendered
                .merge_tile_data(region.tile_size, tile, sample, tile_data);
        }
    }

    pub fn clear_prerendered_tile(&mut self, region: Uuid, tile: &Vec2i) {
        if let Some(region) = REGIONS.write().unwrap().get_mut(&region) {
            region.prerendered.clear_tile_albedo(tile);
        }
    }*/

    /// Executes the given client command.
    pub fn execute_client_cmd(&mut self, client_id: Uuid, cmd: String) {
        //println!("received client ({}) cmd: {}", client_id, cmd);

        let mut parts = cmd.split_whitespace();

        if let Some(cmd_cmd) = parts.next() {
            if cmd_cmd == "instantiate" {
                if let Some(name) = parts.next() {
                    if let Some(new_name) = parts.next() {
                        // Get the character id of the name
                        let mut character_id = None;
                        for (id, character_name) in PLAYERCHARACTERS.read().unwrap().iter() {
                            if name == character_name {
                                character_id = Some(*id);
                                break;
                            }
                        }

                        // Get region id and character to create
                        if let Some(character_id) = character_id {
                            let mut region_id = None;
                            let mut character_to_create = Character::default();
                            for region in &self.project.regions {
                                for character in region.characters.values() {
                                    if character.character_id == character_id {
                                        region_id = Some(region.id);
                                        character_to_create = character.clone();
                                        break;
                                    }
                                }
                            }

                            // Add the character to the region
                            if let Some(region_id) = region_id {
                                self.active_players.insert(
                                    client_id,
                                    ActivePlayer {
                                        id: character_to_create.id,
                                        region_id,
                                    },
                                );

                                SENDCMD
                                    .write()
                                    .unwrap()
                                    .send(ServerMessage::PlayerJoined(
                                        client_id,
                                        character_to_create.id,
                                        region_id,
                                    ))
                                    .unwrap();

                                self.add_character_instance_to_region(
                                    region_id,
                                    character_to_create,
                                    Some(new_name.to_string()),
                                );
                            }
                        }
                    }
                }
            } else if let Some(mut player) = self.active_players.get(&client_id).cloned() {
                // Executes the command
                if let Some(instance) = self.instances.get_mut(&player.region_id) {
                    if execute(&client_id, &cmd, &mut player, &mut instance.sandbox) {
                        self.active_players.insert(client_id, player);
                    }
                }
            }
        }
    }
}
