use super::prelude::*;
use crate::prelude::*;
use crate::server::{REGIONS, UPDATES};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionInstance {
    pub id: Uuid,

    pub sandbox: TheCodeSandbox,

    #[serde(skip)]
    areas: FxHashMap<Uuid, TheCodePackage>,

    #[serde(skip)]
    characters: FxHashMap<Uuid, TheCodePackage>,

    #[serde(skip)]
    items: FxHashMap<Uuid, TheCodePackage>,

    #[serde(skip)]
    character_instances: FxHashMap<Uuid, TheCodePackage>,

    #[serde(skip)]
    item_instances: FxHashMap<Uuid, TheCodePackage>,

    /// For fast lookups an array of (character_instance_id, character_id) tuples.
    #[serde(skip)]
    character_ids: Vec<(Uuid, Uuid)>,

    /// For fast lookups an array of (item_instance_id, item_id) tuples.
    #[serde(skip)]
    item_ids: Vec<(Uuid, Uuid)>,

    #[serde(skip)]
    debug_modules: FxHashMap<Uuid, FxHashMap<Uuid, TheDebugModule>>,

    #[serde(skip)]
    daylight: Daylight,

    redraw_ms: u32,
    tick_ms: u32,

    last_tick: i64,

    pub draw_settings: RegionDrawSettings,
    time: TheTime,

    pub palette: ThePalette,
}

impl Default for RegionInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionInstance {
    pub fn new() -> Self {
        let sandbox = TheCodeSandbox::new();

        Self {
            id: Uuid::nil(),

            sandbox,

            areas: FxHashMap::default(),

            characters: FxHashMap::default(),
            character_instances: FxHashMap::default(),
            character_ids: vec![],

            items: FxHashMap::default(),
            item_instances: FxHashMap::default(),
            item_ids: vec![],

            debug_modules: FxHashMap::default(),

            redraw_ms: 1000 / 30,
            tick_ms: 250,

            last_tick: 0,

            draw_settings: RegionDrawSettings::new(),
            time: TheTime::default(),

            daylight: Daylight::default(),
            palette: ThePalette::default(),
        }
    }

    /// Sets up the region instance.
    pub fn setup(
        &mut self,
        id: Uuid,
        project: &Project,
        packages: &FxHashMap<Uuid, TheCodePackage>,
    ) {
        self.id = id;

        self.areas = FxHashMap::default();

        // Set the sandbox id to our region id.
        self.sandbox.id = id;
        self.sandbox.packages.clone_from(packages);

        self.tick_ms = project.tick_ms;
        self.redraw_ms = 1000 / project.target_fps;

        self.draw_settings.delta = self.redraw_ms as f32 / self.tick_ms as f32;
        self.palette = project.palette.clone();
    }

    /// Tick. Compute the next frame.
    pub fn tick(&mut self, time: TheTime) {
        self.debug_modules.clear();
        self.sandbox.clear_debug_messages();
        self.time = time;
        if let Some(region) = REGIONS.read().unwrap().get(&self.id) {
            self.draw_settings.daylight = self.daylight.daylight(
                self.time.total_minutes(),
                region.min_brightness,
                region.max_brightness,
            );
            self.draw_settings.daylight_intensity =
                self.daylight.daylight_intensity(self.time.total_minutes());
            self.draw_settings.sun_direction = self
                .daylight
                .calculate_light_direction(self.time.total_minutes());
        }
        self.draw_settings.time = time;

        if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
            for character in update.characters.values_mut() {
                character.moving = None;
                character.facing_anim = None;
                character.move_delta = 0.0;
            }
            update.daylight = self.draw_settings.daylight;
        }

        // We iterate over all character instances and execute their main function
        // as well as the main function of their character template.
        for (instance_id, character_id) in &mut self.character_ids {
            self.sandbox.clear_runtime_states();
            self.sandbox
                .aliases
                .insert("self".to_string(), *instance_id);

            // if let Some(instance) = self.characters_instances.get_mut(instance_id) {
            //     instance.execute("main".to_string(), &mut self.sandbox);
            // }

            if let Some(instance) = self.characters.get_mut(character_id) {
                instance.execute("main".to_string(), &mut self.sandbox);
            }

            self.debug_modules
                .insert(*instance_id, self.sandbox.debug_modules.clone());
        }

        // We iterate over all areas and execute their main function.
        for (area_id, package) in &mut self.areas {
            self.sandbox.clear_runtime_states();
            self.sandbox.aliases.insert("self".to_string(), *area_id);

            package.execute("main".to_string(), &mut self.sandbox);

            self.debug_modules
                .insert(*area_id, self.sandbox.debug_modules.clone());
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

    // Returns the debug module (if any) for the given module_id.
    // pub fn get_module_debug_module(&self, id: Uuid) -> TheDebugModule {
    //     self.sandbox.get_module_debug_module(id)
    // }

    /// Returns the debug module (if any) for the given codegrid_id.
    pub fn get_entity_debug_data(&self, id: Uuid) -> Option<FxHashMap<Uuid, TheDebugModule>> {
        self.debug_modules.get(&id).cloned()
    }

    /// Renders this instance into the given buffer.
    pub fn render(
        &mut self,
        buffer: &mut TheRGBABuffer,
        renderer: &mut MapRender,
        anim_counter: &usize,
        _ctx: &mut TheContext,
        server_ctx: &ServerContext,
        compute_delta: bool,
    ) {
        let delta = self.redraw_ms as f32 / self.tick_ms as f32;

        self.draw_settings.show_fx_marker = server_ctx.show_fx_marker;

        if let Some(region) = REGIONS.read().unwrap().get(&self.id) {
            renderer.set_region(region);

            if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
                if compute_delta {
                    let server_tick = update.server_tick;
                    if server_tick != self.last_tick {
                        self.draw_settings.delta_in_tick = 0.0;
                        self.last_tick = server_tick;
                    } else {
                        self.draw_settings.delta_in_tick += delta;
                    }
                }

                self.draw_settings.anim_counter = *anim_counter;
                self.draw_settings.center_on_character = server_ctx.curr_character_instance;
                renderer.set_position(region.editing_position_3d);

                //if region.camera_type == CameraType::FirstPerson {
                //     renderer.render(
                //         buffer,
                //         region,
                //         update,
                //         &mut self.draw_settings,
                //         compute_delta,
                //         &self.palette,
                //     );
                // } else {
                renderer.render(
                    buffer,
                    region,
                    update,
                    &mut self.draw_settings,
                    Some(server_ctx),
                    true, //compute_delta,
                    &self.palette,
                );
            }
        }
    }

    /// Draws this instance into the given buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        tiledrawer: &TileDrawer,
        anim_counter: &usize,
        server_ctx: &ServerContext,
        compute_delta: bool,
        offset: Vec2<i32>,
        palette: &ThePalette,
    ) {
        let delta = self.redraw_ms as f32 / self.tick_ms as f32;

        self.draw_settings.show_fx_marker = server_ctx.show_fx_marker;

        if let Some(region) = REGIONS.read().unwrap().get(&self.id) {
            if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
                if compute_delta {
                    let server_tick = update.server_tick;
                    if server_tick != self.last_tick {
                        self.draw_settings.delta_in_tick = 0.0;
                        self.last_tick = server_tick;
                    } else {
                        self.draw_settings.delta_in_tick += delta;
                    }
                }

                self.draw_settings.anim_counter = *anim_counter;
                self.draw_settings.display_offset = offset;

                self.draw_settings.conceptual_display = server_ctx.conceptual_display;
                self.draw_settings.curr_geo_object = server_ctx.curr_geo_object;
                self.draw_settings.curr_geo_node = server_ctx.curr_geo_node;

                tiledrawer.draw_region(
                    buffer,
                    region,
                    update,
                    &mut self.draw_settings,
                    compute_delta,
                    palette,
                );
            }
        }
    }

    /// Draw the current
    #[allow(clippy::too_many_arguments)]
    pub fn draw_selections(
        &mut self,
        _buffer: &mut TheRGBABuffer,
        _tiledrawer: &TileDrawer,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) {
        /*
        if let Some(region) = REGIONS.read().unwrap().get(&self.id) {
            let grid_size = region.grid_size as f32;

            if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
                // Draw selected character outline
                if let Some(curr_character_instance) = server_ctx.curr_character_instance {
                    for (position, _, character_id, _) in &update.characters_pixel_pos {
                        if *character_id == curr_character_instance {
                            tiledrawer.draw_tile_outline_at_pixel(*position, buffer, WHITE, ctx);
                        }
                    }
                }

                // Draw selected item outline
                if let Some(curr_item_instance) = server_ctx.curr_item_instance {
                    for (id, item) in &mut update.items {
                        let draw_pos = vec2i(
                            (item.position.x * grid_size) as i32,
                            (item.position.y * grid_size) as i32,
                        );

                        if *id == curr_item_instance {
                            tiledrawer.draw_tile_outline_at_pixel(draw_pos, buffer, WHITE, ctx);
                        }
                    }
                }
            }

            if let Some(tilearea) = &server_ctx.tile_selection {
                tiledrawer.draw_tile_selection(
                    &tilearea.merged(),
                    buffer,
                    region.grid_size,
                    WHITE,
                    ctx,
                );
            }

            if let Some(area_id) = &server_ctx.curr_area {
                if let Some(area) = region.areas.get(area_id) {
                    tiledrawer.draw_tile_selection(
                        &area.area,
                        buffer,
                        region.grid_size,
                        WHITE,
                        ctx,
                    );
                }
            }
        }*/
    }

    /// Insert an area (TheCodePackage) to the region.
    pub fn insert_area(&mut self, mut area: Area, compiler: &mut TheCompiler) {
        let mut package = TheCodePackage::new();
        package.id = area.id;

        for grid in area.bundle.grids.values_mut() {
            let rc = compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name.clone_from(&grid.name);
                println!(
                    "RegionInstance::insert_area: Compiled grid module: {}",
                    grid.name
                );
                package.insert_module(module.name.clone(), module);
            } else {
                println!(
                    "RegionInstance::insert_area: Failed to compile grid: {}",
                    grid.name
                );
            }
        }

        let mut a = TheCodeObject::new();
        a.id = area.id;

        self.sandbox.clear_runtime_states();
        self.sandbox.aliases.insert("self".to_string(), a.id);

        //package.execute("init".to_string(), &mut self.sandbox);

        self.sandbox.add_area(a);
        self.areas.insert(area.id, package);
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

        for id in &instance_ids {
            self.sandbox.clear_runtime_states();
            self.sandbox.aliases.insert("self".to_string(), *id);
            character.execute("init".to_string(), &mut self.sandbox);
        }

        for id in instance_ids {
            self.create_character_update(id);
        }

        self.characters.insert(character.id, character);
    }

    /// Insert a (TheCodePackage) to the region.
    pub fn insert_item(&mut self, mut item: TheCodePackage) {
        // We collect all instances of this item and execute the init function on them.
        let mut instance_ids = vec![];
        for o in self.sandbox.items.values() {
            if o.package_id == item.id {
                instance_ids.push(o.id);
            }
        }

        for id in &instance_ids {
            self.sandbox.clear_runtime_states();
            self.sandbox.aliases.insert("self".to_string(), *id);
            item.execute("init".to_string(), &mut self.sandbox);
        }

        for id in instance_ids {
            self.create_item_update(id);
        }

        self.items.insert(item.id, item);
    }

    /// Adds a character instance to the region.
    pub fn add_character_instance(
        &mut self,
        mut character: Character,
        compiler: &mut TheCompiler,
        rename_to: Option<String>,
    ) -> Option<Uuid> {
        let mut package = TheCodePackage::new();
        package.id = character.id;

        let mut module_id = None;

        for grid in character.instance.grids.values_mut() {
            let rc = compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name.clone_from(&grid.name);
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
        // Faces north by default
        o.set(
            str!("facing"),
            TheValue::Direction(Vec3::new(0.0, 0.0, -1.0)),
        );
        o.set(str!("_type"), TheValue::Text(str!("Character")));
        o.set(str!("inventory"), TheValue::List(vec![]));

        self.sandbox.clear_runtime_states();
        self.sandbox.aliases.insert("self".to_string(), o.id);

        if let Some(template) = self.characters.get_mut(&character.character_id) {
            o.package_id = template.id;
            self.sandbox.add_object(o);
            template.execute("init".to_string(), &mut self.sandbox);

            if let Some(rename_to) = rename_to {
                // Rename character, used on player instantiaton
                if let Some(object) = self.sandbox.get_self_mut() {
                    object.set(str!("name"), TheValue::Text(str!(rename_to)));
                }
            }
        }

        package.execute("init".to_string(), &mut self.sandbox);

        self.create_character_update(character.id);

        self.character_ids
            .push((character.id, character.character_id));
        self.character_instances.insert(package.id, package);

        module_id
    }

    /// Adds an item instance to the region.
    pub fn add_item_instance(
        &mut self,
        mut item: Item,
        compiler: &mut TheCompiler,
    ) -> Option<Uuid> {
        let mut package = TheCodePackage::new();
        package.id = item.id;

        let mut module_id = None;

        for grid in item.instance.grids.values_mut() {
            let rc = compiler.compile(grid);
            if let Ok(mut module) = rc {
                module.name.clone_from(&grid.name);
                println!(
                    "RegionInstance::add_item_instance: Compiled grid module: {}",
                    grid.name
                );
                module_id = Some(module.id);
                package.insert_module(module.name.clone(), module);
            } else {
                println!(
                    "RegionInstance::add_item_instance: Failed to compile grid: {}",
                    grid.name
                );
            }
        }

        let mut o = TheCodeObject::new();
        o.id = item.id;
        o.set(str!("_type"), TheValue::Text(str!("Item")));

        self.sandbox.clear_runtime_states();
        self.sandbox.aliases.insert("self".to_string(), o.id);

        if let Some(template) = self.items.get_mut(&item.item_id) {
            o.package_id = template.id;
            self.sandbox.add_item(o);
            template.execute("init".to_string(), &mut self.sandbox);
        }

        package.execute("init".to_string(), &mut self.sandbox);

        self.create_item_update(item.id);

        self.item_ids.push((item.id, item.item_id));
        self.item_instances.insert(package.id, package);

        module_id
    }

    /// Updates a character instance.
    pub fn update_character_instance_bundle(
        &mut self,
        character: Uuid,
        mut bundle: TheCodeBundle,
        compiler: &mut TheCompiler,
    ) {
        if let Some(existing_package) = self.character_instances.get_mut(&character) {
            let mut package = TheCodePackage::new();
            package.id = character;

            for grid in bundle.grids.values_mut() {
                let rc = compiler.compile(grid);
                if let Ok(mut module) = rc {
                    module.name.clone_from(&grid.name);
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

            // Clear the inventory
            if let Some(object) = self.sandbox.objects.get_mut(&character) {
                object.set(str!("inventory"), TheValue::List(vec![]));
            }

            package.execute("init".to_string(), &mut self.sandbox);

            *existing_package = package;
        }

        self.create_character_update(character);
    }

    /// Updates an item instance.
    pub fn update_item_instance_bundle(
        &mut self,
        item: Uuid,
        mut bundle: TheCodeBundle,
        compiler: &mut TheCompiler,
    ) {
        if let Some(existing_package) = self.item_instances.get_mut(&item) {
            let mut package = TheCodePackage::new();
            package.id = item;

            for grid in bundle.grids.values_mut() {
                let rc = compiler.compile(grid);
                if let Ok(mut module) = rc {
                    module.name.clone_from(&grid.name);
                    println!(
                        "RegionInstance::update_item_instance: Compiled grid module: {}",
                        grid.name
                    );
                    package.insert_module(module.name.clone(), module);
                } else {
                    println!(
                        "RegionInstance::update_item_instance: Failed to compile grid: {}",
                        grid.name
                    );
                }
            }

            self.sandbox.clear_runtime_states();
            self.sandbox.aliases.insert("self".to_string(), item);

            package.execute("init".to_string(), &mut self.sandbox);

            *existing_package = package;
        }

        self.create_item_update(item);
    }

    /// Updates a package by inserting it into the sandbox.
    pub fn update_package(&mut self, package: TheCodePackage) {
        self.sandbox.packages.insert(package.id, package);
    }

    /// Removes the given area from the region.
    pub fn remove_area(&mut self, area: Uuid) {
        self.areas.remove(&area);
        self.sandbox.areas.remove(&area);
    }

    /// Removes the given character instance from the region.
    pub fn remove_character_instance(&mut self, character: Uuid) {
        self.character_instances.remove(&character);
        self.character_ids
            .retain(|(instance_id, _)| *instance_id != character);
        self.sandbox.objects.remove(&character);
        if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
            update.characters.remove(&character);
        }
    }

    /// Removes the given item instance from the region.
    pub fn remove_item_instance(&mut self, item: Uuid) {
        self.item_instances.remove(&item);
        self.item_ids
            .retain(|(instance_id, _)| *instance_id != item);
        self.sandbox.items.remove(&item);
        if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
            update.items.remove(&item);
        }
    }

    /// Returns the character instance id and the character id for the character at the given position.
    pub fn get_character_at(&self, pos: Vec2<i32>) -> Option<(Uuid, Uuid)> {
        for c in self.sandbox.objects.values() {
            if let Some(TheValue::Position(p)) = c.get(&"position".into()).cloned() {
                if Vec2::new(p.x as i32, p.z as i32) == pos {
                    for (instance_id, character_id) in &self.character_ids {
                        if *instance_id == c.id {
                            return Some((*instance_id, *character_id));
                        }
                    }
                }
            }
        }

        None
    }

    /// Returns the item instance id and the item id for the item at the given position.
    pub fn get_item_at(&self, pos: Vec2<i32>) -> Option<(Uuid, Uuid)> {
        for c in self.sandbox.items.values() {
            if let Some(TheValue::Position(p)) = c.get(&"position".into()).cloned() {
                if Vec2::new(p.x as i32, p.z as i32) == pos {
                    for (instance_id, item_id) in &self.item_ids {
                        if *instance_id == c.id {
                            return Some((*instance_id, *item_id));
                        }
                    }
                }
            }
        }

        None
    }

    /// Returns the value of the given character instance property along with its character id.
    pub fn get_character_property(
        &self,
        character_id: Uuid,
        property: String,
    ) -> Option<(TheValue, Uuid)> {
        for (id, c) in &self.sandbox.objects {
            if *id == character_id {
                if let Some(value) = c.get(&property).cloned() {
                    for (instance_id, character_id) in &self.character_ids {
                        if *instance_id == c.id {
                            return Some((value.clone(), *character_id));
                        }
                    }
                }
            }
        }

        None
    }

    /// Sets the value of the given character instance property.
    pub fn set_character_property(
        &mut self,
        character_id: Uuid,
        property: String,
        value: TheValue,
    ) {
        for (id, c) in &mut self.sandbox.objects {
            if *id == character_id {
                c.set(property, value);
                break;
            }
        }
    }

    /// Returns the value of the given item instance property along with its item id.
    pub fn get_item_property(&self, item_id: Uuid, property: String) -> Option<(TheValue, Uuid)> {
        for (id, c) in &self.sandbox.items {
            if *id == item_id {
                if let Some(value) = c.get(&property).cloned() {
                    for (instance_id, item_id) in &self.item_ids {
                        if *instance_id == c.id {
                            return Some((value.clone(), *item_id));
                        }
                    }
                }
            }
        }

        None
    }

    /// Returns the object of the given character instance property along with its character id.
    pub fn get_character_object(&self, character_id: Uuid) -> Option<(TheCodeObject, Uuid)> {
        for (id, c) in &self.sandbox.objects {
            if *id == character_id {
                for (instance_id, character_id) in &self.character_ids {
                    if *instance_id == c.id {
                        return Some((c.clone(), *character_id));
                    }
                }
            }
        }

        None
    }

    /// Creates a character update.
    fn create_character_update(&mut self, character: Uuid) {
        // Add the character to the update struct.
        if let Some(object) = self.sandbox.objects.get_mut(&character) {
            let mut character_update = CharacterUpdate::new();
            if let Some(TheValue::Position(p)) = object.get(&"position".into()) {
                character_update.position = Vec2::new(p.x, p.z);
            }
            if let Some(TheValue::Text(t)) = object.get(&"name".into()) {
                character_update.name.clone_from(t);
            }
            if let Some(TheValue::Tile(name, _id)) = object.get_mut(&"tile".into()) {
                character_update.tile_name.clone_from(name);
            }

            character_update.id = character;
            if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
                update.characters.insert(character, character_update);
            }
        }
    }

    /// Creates a character update.
    fn create_item_update(&mut self, item: Uuid) {
        // Add the character to the update struct.
        if let Some(item_object) = self.sandbox.items.get_mut(&item) {
            let mut item_update = ItemUpdate::new();
            if let Some(TheValue::Position(p)) = item_object.get(&"position".into()) {
                item_update.position = Vec2::new(p.x, p.z);
            }
            if let Some(TheValue::Text(t)) = item_object.get(&"name".into()) {
                item_update.name.clone_from(t);
            }
            if let Some(TheValue::Tile(name, id)) = item_object.get_mut(&"tile".into()) {
                item_update.tile_name.clone_from(name);
                item_update.tile_id = *id;
            }

            if let Some(update) = UPDATES.write().unwrap().get_mut(&self.id) {
                update.items.insert(item, item_update);
            }
        }
    }

    /// Returns the debug messages in the sandbox.
    pub fn debug_messages(&self) -> Vec<TheDebugMessage> {
        self.sandbox.debug_messages.clone()
    }

    /// If the user changes the time w/o the server running, we have to update the draw settings manually.
    pub fn set_time(&mut self, time: TheTime) {
        self.time = time;
        self.draw_settings.time = time;
        if let Some(region) = REGIONS.read().unwrap().get(&self.id) {
            self.draw_settings.daylight = self.daylight.daylight(
                self.time.total_minutes(),
                region.min_brightness,
                region.max_brightness,
            );
            self.draw_settings.daylight_intensity =
                self.daylight.daylight_intensity(self.time.total_minutes());
            self.draw_settings.sun_direction = self
                .daylight
                .calculate_light_direction(self.time.total_minutes());
        }
    }

    /// Returns the instance id of the character with the given name.
    pub fn get_character_instance_info_by_name(&self, name: String) -> Option<Uuid> {
        for (instance_id, _) in &self.character_ids {
            if let Some(object) = self.sandbox.objects.get(instance_id) {
                if let Some(TheValue::Text(n)) = object.get(&"name".into()) {
                    if n == &name {
                        return Some(*instance_id);
                    }
                }
            }
        }

        None
    }
}
