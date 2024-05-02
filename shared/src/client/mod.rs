use crate::prelude::*;
use lazy_static::lazy_static;
use std::sync::{Mutex, RwLock};
use theframework::prelude::*;

pub mod functions;

lazy_static! {
    pub static ref REGIONS: RwLock<FxHashMap<Uuid, Region>> = RwLock::new(FxHashMap::default());
    pub static ref RNG: Mutex<rand::rngs::StdRng> = Mutex::new(rand::rngs::StdRng::from_entropy());
    pub static ref TILEDRAWER: RwLock<TileDrawer> = RwLock::new(TileDrawer::new());
    pub static ref KEY_DOWN: RwLock<Option<String>> = RwLock::new(None);
    pub static ref UPDATE: RwLock<RegionUpdate> = RwLock::new(RegionUpdate::default());
    pub static ref CHARACTER: RwLock<Uuid> = RwLock::new(Uuid::nil());
    pub static ref WIDGETBUFFER: RwLock<TheRGBABuffer> = RwLock::new(TheRGBABuffer::empty());
    pub static ref IMAGES: RwLock<FxHashMap<String, TheRGBABuffer>> =
        RwLock::new(FxHashMap::default());
    pub static ref FONTS: RwLock<FxHashMap<String, fontdue::Font>> =
        RwLock::new(FxHashMap::default());
    pub static ref DRAWSETTINGS: RwLock<RegionDrawSettings> =
        RwLock::new(RegionDrawSettings::new());
}

#[derive()]
pub struct Client {
    project: Project,

    sandbox: TheCodeSandbox,

    widgets: FxHashMap<Uuid, TheCodePackage>,

    compiler: TheCompiler,

    redraw_ms: u32,
    tick_ms: u32,

    last_tick: i64,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let mut compiler: TheCompiler = TheCompiler::default();
        functions::add_compiler_client_functions(&mut compiler);

        Self {
            project: Project::default(),
            sandbox: TheCodeSandbox::new(),

            widgets: FxHashMap::default(),

            compiler,

            redraw_ms: 1000 / 30,
            tick_ms: 250,

            last_tick: 0,
        }
    }

    /// Sets the project
    pub fn set_project(&mut self, project: Project) {
        TILEDRAWER
            .write()
            .unwrap()
            .set_tiles(project.extract_tiles());

        let mut regions = REGIONS.write().unwrap();

        regions.clear();

        for region in &project.regions {
            regions.insert(region.id, region.clone());
        }

        self.widgets.clear();
        for screen in project.screens.values() {
            self.compile_script_widgets(screen);
        }

        self.set_assets(project.assets.clone());

        self.tick_ms = project.tick_ms;
        self.redraw_ms = 1000 / project.target_fps;
        DRAWSETTINGS.write().unwrap().delta = self.redraw_ms as f32 / self.tick_ms as f32;
        self.project = project;
    }

    pub fn update_screen(&mut self, screen: &Screen) {
        self.project.screens.insert(screen.id, screen.clone());
        self.compile_script_widgets(screen);
    }

    pub fn update_tiles(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        TILEDRAWER.write().unwrap().set_tiles(tiles);
    }

    pub fn compile_script_widgets(&mut self, screen: &Screen) {
        for widget in &screen.widget_list {
            let mut package = TheCodePackage::new();
            package.id = widget.id;

            let mut bundle = widget.bundle.clone();

            for grid in bundle.grids.values_mut() {
                let rc = self.compiler.compile(grid);
                if let Ok(mut module) = rc {
                    module.name = grid.name.clone();
                    println!("Client::screen_widget: Compiled grid module: {}", grid.name);
                    package.insert_module(module.name.clone(), module);
                } else {
                    println!(
                        "Client::screen_widget: Failed to compile grid: {}",
                        grid.name
                    );
                }
            }

            let mut object = TheCodeObject::new();
            object.id = widget.id;

            self.sandbox.objects.insert(object.id, object);

            self.sandbox.clear_runtime_states();
            self.sandbox.aliases.insert("self".to_string(), widget.id);
            package.execute("init".to_string(), &mut self.sandbox);

            self.widgets.insert(package.id, package);
        }
    }

    pub fn set_character_id(&mut self, character_id: Uuid) {
        *CHARACTER.write().unwrap() = character_id;
    }

    pub fn set_region_update(&mut self, json: String) {
        if let Some(update) = RegionUpdate::from_json(&json) {
            *UPDATE.write().unwrap() = update;
        }
    }

    pub fn tick(&mut self) {
        DRAWSETTINGS.write().unwrap().anim_counter += 1;
    }

    pub fn draw_screen(
        &mut self,
        uuid: &Uuid,
        buffer: &mut TheRGBABuffer,
        _tiledrawer: &TileDrawer,
        _ctx: &mut TheContext,
    ) {
        let delta = self.redraw_ms as f32 / self.tick_ms as f32;

        let server_tick = UPDATE.read().unwrap().server_tick;

        if server_tick != self.last_tick {
            DRAWSETTINGS.write().unwrap().delta_in_tick = 0.0;
            self.last_tick = server_tick;
        } else {
            DRAWSETTINGS.write().unwrap().delta_in_tick += delta;
        }

        buffer.fill(BLACK);

        if let Some(screen) = self.project.screens.get(uuid) {
            for widget in &screen.widget_list {
                let x = (widget.x * screen.grid_size as f32) as i32;
                let y = (widget.y * screen.grid_size as f32) as i32;
                let width = (widget.width * screen.grid_size as f32) as i32;
                let height = (widget.height * screen.grid_size as f32) as i32;

                let dim = TheDim::new(0, 0, width, height);
                *WIDGETBUFFER.write().unwrap() = TheRGBABuffer::new(dim);

                if let Some(package) = self.widgets.get_mut(&widget.id) {
                    self.sandbox.clear_runtime_states();
                    self.sandbox.aliases.insert("self".to_string(), package.id);

                    package.execute("draw".to_string(), &mut self.sandbox);
                }

                for (pos, tiles) in &widget.ui_tiles {
                    for tile in tiles {
                        if let Some(tile) = TILEDRAWER.read().unwrap().get_tile(tile) {
                            WIDGETBUFFER.write().unwrap().copy_into(
                                pos.0 * screen.grid_size,
                                pos.1 * screen.grid_size,
                                &tile.buffer[0],
                            )
                        }
                    }
                }

                buffer.blend_into(x, y, &WIDGETBUFFER.read().unwrap());
            }
        }
    }

    /// Extract the assets and make them available via the static accessors.
    pub fn set_assets(&mut self, assets: FxHashMap<Uuid, Asset>) {
        IMAGES.write().unwrap().clear();
        FONTS.write().unwrap().clear();
        for a in assets.values() {
            match &a.buffer {
                AssetBuffer::Image(buffer) => {
                    IMAGES
                        .write()
                        .unwrap()
                        .insert(a.name.clone(), buffer.clone());
                }
                AssetBuffer::Font(buffer) => {
                    if let Ok(font) =
                        fontdue::Font::from_bytes(buffer.clone(), fontdue::FontSettings::default())
                    {
                        FONTS.write().unwrap().insert(a.name.clone(), font);
                    }
                }
                _ => {}
            }
        }
    }

    /// Returns a mutable reference to the compiler.
    pub fn compiler(&mut self) -> &mut TheCompiler {
        &mut self.compiler
    }
}
