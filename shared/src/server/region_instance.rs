use crate::prelude::*;
use crate::server::{REGIONS, RNG, TILES};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionInstance {
    pub id: Uuid,

    sandbox: TheCodeSandbox,

    #[serde(skip)]
    characters: FxHashMap<Uuid, TheCodePackage>,

    #[serde(skip)]
    characters_instances: FxHashMap<Uuid, TheCodePackage>,

    /// For fast lookups an array of (character_instance_id, character_id) tuples.
    #[serde(skip)]
    characters_ids: Vec<(Uuid, Uuid)>,
}

impl Default for RegionInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionInstance {
    pub fn new() -> Self {
        let mut sandbox = TheCodeSandbox::new();

        sandbox.add_global(
            "RandWalk",
            TheCodeNode::new(
                |_stack, data, sandbox| {
                    if let Some(region) = REGIONS.read().unwrap().get(&sandbox.id) {
                        if let Some(object) = sandbox.get_self_mut() {
                            if let Some(TheValue::Position(p)) = object.get_mut(&"position".into())
                            {
                                let mut x = p.x;
                                let mut y = p.y;

                                let dir = RNG.lock().unwrap().gen_range(0..=4);

                                if dir == 0 {
                                    x += 1.0;
                                } else if dir == 1 {
                                    x -= 1.0;
                                } else if dir == 2 {
                                    y += 1.0;
                                } else if dir == 3 {
                                    y -= 1.0;
                                }

                                if region.can_move_to(vec3f(x, y, p.z), &TILES.read().unwrap()) {
                                    *p = vec3f(x, y, p.z);
                                    if sandbox.debug_mode {
                                        sandbox.set_debug_value(
                                            data.location,
                                            TheValue::Text("True".to_string()),
                                        );
                                    }
                                } else if sandbox.debug_mode {
                                    sandbox.set_debug_value(
                                        data.location,
                                        TheValue::Text("False".to_string()),
                                    );
                                }
                            }
                        }
                    }
                    TheCodeNodeCallResult::Continue
                },
                TheCodeNodeData::values(vec![TheValue::Int(0)]),
            ),
        );

        Self {
            id: Uuid::nil(),

            sandbox,

            characters: FxHashMap::default(),
            characters_instances: FxHashMap::default(),
            characters_ids: vec![],
        }
    }

    /// Sets up the region instance.
    pub fn setup(&mut self, id: Uuid, _project: &Project) {
        self.id = id;
        self.sandbox.id = id;
    }

    /// Tick. Compute the next frame.
    pub fn tick(&mut self) {
        // We iterate over all character instances and execute their main function
        // as well as the main function of their character template.
        for (instance_id, character_id) in &mut self.characters_ids {
            self.sandbox.clear_runtime_states();
            self.sandbox
                .aliases
                .insert("self".to_string(), *instance_id);

            if let Some(instance) = self.characters_instances.get_mut(instance_id) {
                instance.execute("main".to_string(), &mut self.sandbox);
            }

            if let Some(instance) = self.characters.get_mut(character_id) {
                instance.execute("main".to_string(), &mut self.sandbox);

                // println!(
                //     "instance_id: {}, debug {:?}",
                //     character_id,
                //     self.sandbox.get_codegrid_debug_module(*character_id)
                // );
            }
        }
    }

    /// Create an instance from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }

    /// Convert the instance to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }

    /// Sets the debugging mode.
    pub fn set_debug_mode(&mut self, debug_mode: bool) {
        self.sandbox.debug_mode = debug_mode;
    }

    /// Returns the debug module (if any) for the given module_id.
    pub fn get_module_debug_module(&self, id: Uuid) -> TheDebugModule {
        self.sandbox.get_module_debug_module(id)
    }

    /// Returns the debug module (if any) for the given codegrid_id.
    pub fn get_codegrid_debug_module(&self, id: Uuid) -> TheDebugModule {
        self.sandbox.get_codegrid_debug_module(id)
    }

    /// Draws this instance into the given buffer.
    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        tiledrawer: &TileDrawer,
        anim_counter: &usize,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if let Some(region) = REGIONS.read().unwrap().get(&self.id) {
            tiledrawer.draw_region(buffer, region, anim_counter, ctx);

            for c in self.sandbox.objects.values_mut() {
                if let Some(TheValue::Position(p)) = c.get(&"position".into()).cloned() {
                    if let Some(TheValue::Tile(name, id)) = c.get_mut(&"tile".into()) {
                        //println!("p {:?} s {:?}", p, name);

                        if !tiledrawer.draw_tile(
                            vec2i(p.x as i32, p.y as i32),
                            buffer,
                            region.grid_size,
                            *id,
                            anim_counter,
                            ctx,
                        ) {
                            if let Some(found_id) = tiledrawer.get_tile_id_by_name(name.clone()) {
                                *id = found_id;
                                tiledrawer.draw_tile(
                                    vec2i(p.x as i32, p.y as i32),
                                    buffer,
                                    region.grid_size,
                                    found_id,
                                    anim_counter,
                                    ctx,
                                );
                            } else {
                                //println!("RegionInstance::draw: Tile not found: {}", name);
                            }
                        }
                    }
                }

                if Some(c.id) == server_ctx.curr_character_instance {
                    if let Some(TheValue::Position(p)) = c.get(&"position".into()) {
                        tiledrawer.draw_tile_outline(
                            vec2i(p.x as i32, p.y as i32),
                            buffer,
                            region.grid_size,
                            WHITE,
                            ctx,
                        );
                    }
                }
                else if Some(c.id) == server_ctx.curr_character {
                    if let Some(TheValue::Position(p)) = c.get(&"position".into()) {
                        tiledrawer.draw_tile_outline(
                            vec2i(p.x as i32, p.y as i32),
                            buffer,
                            region.grid_size,
                            [128, 128, 128, 255],
                            ctx,
                        );
                    }
                }
            }
        }
    }

    /// Insert a (TheCodePackage) to the region.
    pub fn insert_character(&mut self, mut character: TheCodePackage) {
        // We collect all instances of this character and execute the init function on them.
        let mut instance_ids = vec![];
        for o in self.sandbox.objects.values() {
            if o.package_id == character.id {
                instance_ids.push(o.id);
            }
        }

        for id in instance_ids {
            self.sandbox.clear_runtime_states();
            self.sandbox.aliases.insert("self".to_string(), id);
            character.execute("init".to_string(), &mut self.sandbox);

            if let Some(inst) = self.characters_instances.get_mut(&id) {
                inst.execute("init".to_string(), &mut self.sandbox);
            }
        }

        self.characters.insert(character.id, character);
    }

    /// Adds a character instance to the region.
    pub fn add_character_instance(&mut self, mut character: Character) -> Option<Uuid> {
        let mut package = TheCodePackage::new();
        package.id = character.id;

        let mut module_id = None;

        let mut compiler = TheCompiler::new();

        for grid in character.instance.grids.values_mut() {
            let rc = compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name = grid.name.clone();
                println!(
                    "RegionInstance::add_character_instance: Compiled grid module: {}",
                    grid.name
                );
                module_id = Some(module.id);
                package.insert_module(module.name.clone(), module);
            } else {
                println!(
                    "RegionInstance::add_character_instance: Failed to compile grid: {}",
                    grid.name
                );
            }
        }

        let mut o = TheCodeObject::new();
        o.id = character.id;

        self.sandbox.clear_runtime_states();
        self.sandbox.aliases.insert("self".to_string(), o.id);

        if let Some(template) = self.characters.get_mut(&character.character_id) {
            o.package_id = template.id;
            self.sandbox.add_object(o);
            template.execute("init".to_string(), &mut self.sandbox);
        }

        package.execute("init".to_string(), &mut self.sandbox);

        self.characters_ids
            .push((character.id, character.character_id));
        self.characters_instances.insert(package.id, package);

        module_id
    }

    /// Updates a character instance.
    pub fn update_character_instance_bundle(&mut self, character: Uuid, mut bundle: TheCodeBundle) {
        if let Some(existing_package) = self.characters_instances.get_mut(&character) {
            let mut package = TheCodePackage::new();

            let mut compiler = TheCompiler::new();

            for grid in bundle.grids.values_mut() {
                let rc = compiler.compile(grid);
                if let Ok(mut module) = rc {
                    module.name = grid.name.clone();
                    println!(
                        "RegionInstance::add_character_instance: Compiled grid module: {}",
                        grid.name
                    );
                    package.insert_module(module.name.clone(), module);
                } else {
                    println!(
                        "RegionInstance::add_character_instance: Failed to compile grid: {}",
                        grid.name
                    );
                }
            }

            self.sandbox.clear_runtime_states();
            self.sandbox.aliases.insert("self".to_string(), character);

            package.execute("init".to_string(), &mut self.sandbox);

            *existing_package = package;
        }
    }

    /// Removes the given character instance from the region.
    pub fn remove_character_instance(&mut self, character: Uuid) {
        self.characters_instances.remove(&character);
        self.characters_ids.retain(|(instance_id, _)| *instance_id != character);
    }

    /// Returns the character instance id and the character id for the character at the given position.
    pub fn get_character_at(&self, pos: Vec2i) -> Option<(Uuid, Uuid)> {
        for c in self.sandbox.objects.values() {
            if let Some(TheValue::Position(p)) = c.get(&"position".into()).cloned() {
                if vec2i(p.x as i32, p.y as i32) == pos {
                    for (instance_id, character_id) in &self.characters_ids {
                        if *instance_id == c.id {
                            return Some((*instance_id, *character_id));
                        }
                    }
                }
            }
        }

        None
    }
}
