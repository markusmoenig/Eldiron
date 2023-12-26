use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionInstance {
    sandbox: TheCodeSandbox,

    #[serde(skip)]
    characters: FxHashMap<Uuid, TheCodePackage>,

    #[serde(skip)]
    characters_custom: FxHashMap<Uuid, TheCodePackage>,

    region: Region,
}

impl Default for RegionInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionInstance {
    pub fn new() -> Self {
        Self {
            sandbox: TheCodeSandbox::new(),

            characters: FxHashMap::default(),
            characters_custom: FxHashMap::default(),

            region: Region::new(),
        }
    }

    /// Sets up the region instance.
    pub fn setup(&mut self, uuid: Uuid, project: &Project) {
        if let Some(region) = project.get_region(&uuid).cloned() {
            self.region = region.clone();
        } else {
            println!("RegionInstance::setup: Region not found: {}", uuid);
        }
    }

    /// Tick. Compute the next frame.
    pub fn tick(&mut self) {}

    /// Create an instance from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }

    /// Convert the instance to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }

    /// Updates the region instance. This is called when the region has been edited in the editor.
    pub fn update(&mut self, region: &Region) {
        self.region = region.clone();
        //println!("RegionInstance::update: {:?}", self.region);
    }

    /// Draws this instance into the given buffer.
    pub fn draw(
        &self,
        buffer: &mut TheRGBABuffer,
        tiledrawer: &TileDrawer,
        anim_counter: &usize,
        ctx: &mut TheContext,
        server_ctx: &ServerContext
    ) {
        tiledrawer.draw_region(buffer, &self.region, anim_counter, ctx);

        for c in self.sandbox.objects.values() {
            //println!("RegionInstance::draw: Object: {:?}", c);
            if let Some(TheValue::Position(p)) = c.get(&"position".into()) {
                if let Some(TheValue::Tile(name, _id)) = c.get(&"tile".into()) {
                    println!("p {:?} s {:?}", p, name);
                }
            }

            if Some(c.id) == server_ctx.curr_character_instance || Some(c.package_id) == server_ctx.curr_character {
                if let Some(TheValue::Position(p)) = c.get(&"position".into()) {
                    tiledrawer.draw_tile_outline(
                        vec2i(p.x as i32, p.y as i32),
                        buffer,
                        self.region.grid_size,
                        ctx,
                    );
                }
            }
        }
    }

    /// Insert a (TheCodeBundle) to the region.
    pub fn insert_character(&mut self, character: TheCodePackage) {
        self.characters.insert(character.id, character);
    }

    /// Adds a character instance to the region.
    pub fn add_character_instance(&mut self, mut character: Character) {
        let mut package = TheCodePackage::new();
        package.id = character.id;

        let mut compiler = TheCompiler::new();

        for grid in character.custom.grids.values_mut() {
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

        self.characters_custom.insert(package.id, package);
    }

    pub fn update_character_bundle(&mut self, character: Uuid, mut bundle: TheCodeBundle) {

        if let Some(existing_package) = self.characters_custom.get_mut(&character) {

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

            println!("updated package");
            *existing_package = package;
        }
    }
}
