use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionInstance {
    sandbox: TheCodeSandbox,

    character: FxHashMap<Uuid, TheCodeBundle>,
    #[serde(skip)]
    character_modules: FxHashMap<Uuid, TheCodeModule>,

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

            character: FxHashMap::default(),
            character_modules: FxHashMap::default(),

            region: Region::new(),
        }
    }

    /// Sets up the region instance.
    pub fn setup(&mut self, uuid: Uuid, project: &Project) {
        self.character = project.characters.clone();
        if let Some(region) = project.get_region(&uuid).cloned() {
            self.region = region.clone();
        } else {
            println!("RegionInstance::setup: Region not found: {}", uuid);
        }
    }

    /// Tick. Compute the next frame.
    pub fn tick(& mut self) {
    }

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
    pub fn draw(&self, buffer: &mut TheRGBABuffer, tiledrawer: &TileDrawer, anim_counter: &usize, ctx: &mut TheContext) {
        tiledrawer.draw_region(buffer, &self.region, anim_counter, ctx);
    }

    /// Add a new character (TheCodeBundle) to the region.
    pub fn add_character(&mut self, character: TheCodeBundle) {
        self.character.insert(character.id, character);
    }

    /// Adds a character instance to the region.
    pub fn add_character_instance(&mut self, character: Uuid, _location: Vec2i) {
        if let Some(character) = self.character.get(&character) {
            let mut o = TheCodeObject::new();
            o.module_id = character.id;
            self.sandbox.add_object(o);
        }
    }
}