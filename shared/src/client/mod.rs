use crate::prelude::*;
use lazy_static::lazy_static;
use std::sync::{Mutex, RwLock};
use theframework::prelude::*;

pub mod functions;

lazy_static! {
    pub static ref REGIONS: RwLock<FxHashMap<Uuid, Region>> = RwLock::new(FxHashMap::default());
    pub static ref RNG: Mutex<rand::rngs::StdRng> = Mutex::new(rand::rngs::StdRng::from_entropy());
    pub static ref TILEDRAWER: RwLock<TileDrawer> = RwLock::new(TileDrawer::new());
    pub static ref PALETTE: RwLock<ThePalette> = RwLock::new(ThePalette::default());
    pub static ref RENDERER: RwLock<Renderer> = RwLock::new(Renderer::new());
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

    // The screen package
    package: TheCodePackage,

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
            package: TheCodePackage::default(),

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

        RENDERER
            .write()
            .unwrap()
            .set_textures(project.extract_tiles());

        *PALETTE.write().unwrap() = project.palette.clone();

        let mut regions = REGIONS.write().unwrap();
        regions.clear();

        for region in &project.regions {
            regions.insert(region.id, region.clone());
        }

        self.set_assets(&project);

        self.widgets.clear();
        for screen in project.screens.values() {
            self.compile_screen(screen);
        }

        self.tick_ms = project.tick_ms;
        self.redraw_ms = 1000 / project.target_fps;
        DRAWSETTINGS.write().unwrap().delta = self.redraw_ms as f32 / self.tick_ms as f32;
        self.project = project;
    }

    pub fn update_screen(&mut self, screen: &Screen) {
        self.project.screens.insert(screen.id, screen.clone());
        self.compile_screen(screen);
    }

    pub fn update_tiles(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        TILEDRAWER.write().unwrap().set_tiles(tiles.clone());
        RENDERER.write().unwrap().set_textures(tiles);
    }

    pub fn compile_screen(&mut self, screen: &Screen) {
        // Compile the screen scripts
        let mut package = TheCodePackage::new();
        package.id = screen.id;

        let mut bundle = screen.bundle.clone();

        for grid in bundle.grids.values_mut() {
            let rc = self.compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name.clone_from(&grid.name);
                println!("Client::screen: Compiled grid module: {}", grid.name);
                package.insert_module(module.name.clone(), module);
            } else {
                println!("Client::screen: Failed to compile grid: {}", grid.name);
            }
        }

        let mut object = TheCodeObject::new();
        object.id = screen.id;

        self.sandbox.objects.insert(object.id, object);

        self.sandbox.clear_runtime_states();
        self.sandbox.aliases.insert("self".to_string(), screen.id);
        package.execute("init".to_string(), &mut self.sandbox);
        self.package = package;

        // Compile the widget scripts
        for widget in &screen.widget_list {
            let mut package = TheCodePackage::new();
            package.id = widget.id;

            let mut bundle = widget.bundle.clone();

            for grid in bundle.grids.values_mut() {
                let rc = self.compiler.compile(grid);
                if let Ok(mut module) = rc {
                    module.name.clone_from(&grid.name);
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

    pub fn set_region(&mut self, region: &Uuid) {
        if let Some(region) = REGIONS.write().unwrap().get_mut(region) {
            // TODO: Only do this once per region.
            for (key, model) in region.models.iter_mut() {
                model.create_voxels(
                    region.grid_size as u8,
                    &vec3f(key.0 as f32, key.1 as f32, key.2 as f32),
                    &self.project.palette,
                );
            }

            RENDERER.write().unwrap().set_region(region);
        }
    }

    pub fn tick(&mut self) {
        DRAWSETTINGS.write().unwrap().anim_counter += 1;
    }

    pub fn draw_screen(&mut self, uuid: &Uuid, buffer: &mut TheRGBABuffer) {
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
            // Draw background tiles
            for (pos, tiles) in &screen.tiles {
                for tile in tiles {
                    if let Some(tile) = TILEDRAWER.read().unwrap().get_tile(tile) {
                        buffer.copy_into(
                            pos.0 * screen.grid_size,
                            pos.1 * screen.grid_size,
                            &tile.buffer[0],
                        )
                    }
                }
            }

            // Draw screen
            let x = 0;
            let y = 0;
            let width = screen.width * screen.grid_size;
            let height = screen.height * screen.grid_size;

            let dim = TheDim::new(0, 0, width, height);
            *WIDGETBUFFER.write().unwrap() = TheRGBABuffer::new(dim);

            self.sandbox.clear_runtime_states();
            self.sandbox
                .aliases
                .insert("self".to_string(), self.package.id);

            self.package.execute("draw".to_string(), &mut self.sandbox);

            buffer.blend_into(x, y, &WIDGETBUFFER.read().unwrap());

            for widget in &screen.widget_list {
                let x = (widget.x * screen.grid_size as f32) as i32;
                let y = (widget.y * screen.grid_size as f32) as i32;
                let width = (widget.width * screen.grid_size as f32) as i32;
                let height = (widget.height * screen.grid_size as f32) as i32;

                let dim = TheDim::new(0, 0, width, height);
                *WIDGETBUFFER.write().unwrap() = TheRGBABuffer::new(dim);

                // Draw Images
                if let Some(package) = self.widgets.get_mut(&widget.id) {
                    self.sandbox.clear_runtime_states();
                    self.sandbox.aliases.insert("self".to_string(), package.id);

                    package.execute("draw".to_string(), &mut self.sandbox);
                }

                // Draw Buttons
                if let Some(object) = self.sandbox.objects.get_mut(&widget.id) {
                    for (_, value) in object.values.iter() {
                        if let TheValue::CodeObject(obj) = value {
                            if let Some(TheValue::Text(typ)) = obj.get(&"_type".to_string()) {
                                if typ == "Button" {
                                    if let Some(TheValue::Position(p)) =
                                        obj.get(&"position".to_string())
                                    {
                                        if let Some(TheValue::Image(image)) =
                                            obj.get(&"normal".to_string())
                                        {
                                            buffer.copy_into(p.x as i32, p.z as i32, image);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                buffer.blend_into(x, y, &WIDGETBUFFER.read().unwrap());
            }

            // Draw foreground tiles
            for (pos, tiles) in &screen.foreground_tiles {
                for tile in tiles {
                    if let Some(tile) = TILEDRAWER.read().unwrap().get_tile(tile) {
                        buffer.copy_into(
                            pos.0 * screen.grid_size,
                            pos.1 * screen.grid_size,
                            &tile.buffer[0],
                        )
                    }
                }
            }
        }
    }

    /// Extract the assets and make them available via the static accessors.
    pub fn set_assets(&mut self, project: &Project) {
        IMAGES.write().unwrap().clear();
        FONTS.write().unwrap().clear();

        for tilemap in project.tilemaps.iter() {
            println!("adding {}", tilemap.name);
            IMAGES
                .write()
                .unwrap()
                .insert(tilemap.name.clone(), tilemap.buffer.clone());
        }

        for a in project.assets.values() {
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

    /// Touch down event.
    pub fn touch_down(&mut self, uuid: &Uuid, pos: Vec2i) {
        println!("Client: Touch down {}", pos);

        if let Some(screen) = self.project.screens.get(uuid) {
            for widget in &screen.widget_list {
                // let x = (widget.x * screen.grid_size as f32) as i32;
                // let y = (widget.y * screen.grid_size as f32) as i32;
                // let width = (widget.width * screen.grid_size as f32) as i32;
                // let height = (widget.height * screen.grid_size as f32) as i32;

                if let Some(object) = self.sandbox.objects.get_mut(&widget.id) {
                    for (_name, value) in object.values.iter_mut() {
                        if let TheValue::CodeObject(obj) = value {
                            if let Some(TheValue::Text(typ)) = obj.get(&"_type".to_string()) {
                                if typ == "Button" {
                                    if let Some(TheValue::Position(position)) =
                                        obj.get(&"position".to_string())
                                    {
                                        if let Some(TheValue::Image(image)) =
                                            obj.get(&"normal".to_string())
                                        {
                                            let x = position.x as i32;
                                            let y = position.z as i32;
                                            if pos.x >= x
                                                && pos.x <= x + image.dim().width
                                                && pos.y >= y
                                                && pos.y <= y + image.dim().height
                                            {
                                                obj.set(
                                                    str!("state"),
                                                    TheValue::Text(str!("clicked")),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
