use crate::prelude::*;

pub struct RegionWidget {
    pub rect                : (usize, usize, usize, usize),

    editor_rect             : (usize, usize, usize, usize),
    preview_rect            : (usize, usize, usize, usize),

    pub region_id           : Uuid,

    pub layouts             : Vec<HLayout>,

    grid_size               : usize,

    offset                  : (isize, isize),
    screen_offset           : (usize, usize),

    pub tile_selector       : TileSelectorWidget,
    pub character_selector  : CharacterSelectorWidget,
    pub loot_selector       : LootSelectorWidget,

    pub behavior_graph      : Box::<NodeGraph>,

    mouse_wheel_delta       : (isize, isize),

    mouse_hover_pos         : (usize, usize),
    pub clicked             : Option<(isize, isize)>,

    bottom_size             : usize,
    toolbar_size            : usize,

    debug_update            : Option<GameUpdate>,

    selected_range          : Option<(isize, isize, isize, isize)>,
    clipboard               : Option<GameRegionData>,

    undo                    : Option<String>,
    has_changed             : bool,

    preview_button          : AtomWidget,
}

impl EditorContent for RegionWidget {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _behavior_type: BehaviorType, asset: &Asset, context: &ScreenContext) -> Self {

        let toolbar_size = 33;
        let bottom_size = 250;

        let mut layouts = vec![];
        let mut hlayout = HLayout::new((rect.0, rect.1 + rect.3 - bottom_size - toolbar_size, 165, toolbar_size));
        hlayout.margin = (10, 0, 0, 0);

        let mut mode_button = AtomWidget::new(vec!["tiles".to_string(), "area".to_string(), "character".to_string(), "loot".to_string(), "settings".to_string()], AtomWidgetType::IconRow,
        AtomData::new("Mode", Value::Empty()));
        mode_button.atom_data.text = "Mode".to_string();
        mode_button.set_rect((0, 0, 165, toolbar_size), asset, context);
        mode_button.custom_color = Some([217, 64, 51, 255]);

        let mut status_help_vector : Vec<(String, String)> = vec![];
        status_help_vector.push(("Draw Mode".to_string(), "Draw tiles ('D').".to_string()));
        status_help_vector.push(("Area Mode".to_string(), "Edit named areas and their behavior ('E').".to_string()));
        status_help_vector.push(("Character Mode".to_string(), "Place character instances ('A').".to_string()));
        status_help_vector.push(("Item Mode".to_string(), "Place item instances as loot ('L').".to_string()));
        status_help_vector.push(("Settings".to_string(), "Edit the settings of the region ('S').".to_string()));
        mode_button.status_help_vector = Some(status_help_vector);

        hlayout.add(mode_button, 0);
        hlayout.layout();
        layouts.push(hlayout);

        // Preview button

        let mut preview_button = AtomWidget::new(vec!["Preview: Off".to_string(), "Preview: 2D".to_string(), "Preview: 3D".to_string()], AtomWidgetType::SliderButton,
        AtomData::new("Preview", Value::Empty()));
        preview_button.curr_index = 0;
        preview_button.atom_data.text = "Preview".to_string();
        preview_button.set_rect((rect.0 + rect.2 - 190, rect.1, 180, 40), asset, context);
        preview_button.status_help_text = Some("Toggles the preview ('P').".to_string());

        // Tile Selector
        let mut tile_selector = TileSelectorWidget::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), asset, &context);
        tile_selector.set_tile_type(vec![], None, None, &asset);

        let character_selector = CharacterSelectorWidget::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), asset, &context);

        let loot_selector = LootSelectorWidget::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), asset, &context);

        // Graph
        let mut behavior_graph = NodeGraph::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), BehaviorType::Regions, asset, &context);

        behavior_graph.set_mode(GraphMode::Detail, &context);

        // Area Widgets
        let mut area_layout = HLayout::new((rect.0 + 180, rect.1 + rect.3 - bottom_size - toolbar_size, rect.2 - 180, toolbar_size));
        area_layout.margin = (10, 0, 0, 0);
        area_layout.spacing = 0;

        let mut regions_button = AtomWidget::new(vec![], AtomWidgetType::SliderButton,
        AtomData::new("Area", Value::Empty()));
        regions_button.atom_data.text = "Area".to_string();
        regions_button.set_rect((0, 0, 180, 40), asset, context);
        regions_button.status_help_text = Some("Cycles through the current areas.".to_string());
        area_layout.add(regions_button, 0);

        let mut add_area_button = AtomWidget::new(vec!["Add Area".to_string()], AtomWidgetType::Button,
            AtomData::new("Add Area", Value::Empty()));
        add_area_button.set_rect((0, rect.1, 140, 40), asset, context);
        add_area_button.status_help_text = Some("Adds a new, empty area.".to_string());
        area_layout.add(add_area_button, 5);

        let mut del_area_button = AtomWidget::new(vec!["Delete".to_string()], AtomWidgetType::Button,
            AtomData::new("Delete", Value::Empty()));
        del_area_button.state = WidgetState::Disabled;
        del_area_button.set_rect((0, 0, 140, 40), asset, context);
        del_area_button.status_help_text = Some("Deletes the current area.".to_string());
        area_layout.add(del_area_button, 5);

        let mut rename_area_button = AtomWidget::new(vec!["Rename".to_string()], AtomWidgetType::Button,
            AtomData::new("Rename", Value::Empty()));
        rename_area_button.state = WidgetState::Disabled;
        rename_area_button.set_rect((0, 0, 140, 40), asset, context);
        rename_area_button.status_help_text = Some("Renames the current area.".to_string());
        area_layout.add(rename_area_button, 5);

        let mut pick_area_button = AtomWidget::new(vec!["pick".to_string()], AtomWidgetType::EnabledIcon,
        AtomData::new("Pick", Value::Empty()));
        pick_area_button.atom_data.text = "Pick".to_string();
        pick_area_button.set_rect((0, 0, 35, 38), asset, context);
        pick_area_button.status_help_text = Some("Selects the clicked area.".to_string());
        area_layout.add(pick_area_button, 15);

        let mut area_add_tile_button = AtomWidget::new(vec!["add".to_string()], AtomWidgetType::EnabledIcon,
        AtomData::new("Add", Value::Empty()));
        area_add_tile_button.atom_data.text = "Add".to_string();
        area_add_tile_button.checked = true;
        area_add_tile_button.set_rect((0, 0, 35, 38), asset, context);
        area_add_tile_button.status_help_text = Some("Adds the clicked tile to the current area.".to_string());
        area_layout.add(area_add_tile_button, 10);

        let mut area_remove_tile_button = AtomWidget::new(vec!["remove".to_string()], AtomWidgetType::EnabledIcon,
        AtomData::new("Remove", Value::Empty()));
        area_remove_tile_button.atom_data.text = "Remove".to_string();
        area_remove_tile_button.set_rect((0, 0, 40, 38), asset, context);
        area_remove_tile_button.status_help_text = Some("Removes the clicked tile from the current area.".to_string());
        area_layout.add(area_remove_tile_button, 0);

        area_layout.layout();
        layouts.push(area_layout);

        // Character Widgets

        let mut character_layout = HLayout::new((rect.0 + 180, rect.1 + rect.3 - bottom_size - toolbar_size, rect.2 - 180, toolbar_size));
        character_layout.margin = (10, 0, 0, 0);
        character_layout.spacing = 0;

        let mut char_add_tile_button = AtomWidget::new(vec!["add".to_string()], AtomWidgetType::EnabledIcon,
        AtomData::new("Add", Value::Empty()));
        char_add_tile_button.atom_data.text = "Add".to_string();
        char_add_tile_button.checked = true;
        char_add_tile_button.set_rect((rect.0 + 190, rect.1 + rect.3 - bottom_size - toolbar_size - 5, 35, 38), asset, context);
        char_add_tile_button.status_help_text = Some("Adds a character instance.".to_string());
        character_layout.add(char_add_tile_button, 0);

        let mut char_remove_tile_button = AtomWidget::new(vec!["remove".to_string()], AtomWidgetType::EnabledIcon,
        AtomData::new("Remove", Value::Empty()));
        char_remove_tile_button.atom_data.text = "Remove".to_string();
        char_remove_tile_button.set_rect((rect.0 + 190 + 40, rect.1 + rect.3 - bottom_size - toolbar_size - 5, 35, 38), asset, context);
        char_remove_tile_button.status_help_text = Some("Removes a character instance.".to_string());
        character_layout.add(char_remove_tile_button, 0);

        character_layout.layout();
        layouts.push(character_layout);

        // Loot Widgets
        let mut loot_layout = HLayout::new((rect.0 + 180, rect.1 + rect.3 - bottom_size - toolbar_size, rect.2 - 180, toolbar_size));
        loot_layout.margin = (10, 0, 0, 0);
        loot_layout.spacing = 0;

        let mut loot_add_tile_button = AtomWidget::new(vec!["add".to_string()], AtomWidgetType::EnabledIcon,
        AtomData::new("Add", Value::Empty()));
        loot_add_tile_button.atom_data.text = "Add".to_string();
        loot_add_tile_button.checked = true;
        loot_add_tile_button.set_rect((rect.0 + 190, rect.1 + rect.3 - bottom_size - toolbar_size - 5, 35, 38), asset, context);
        loot_add_tile_button.status_help_text = Some("Adds loot.".to_string());
        loot_layout.add(loot_add_tile_button, 0);

        let mut loot_remove_tile_button = AtomWidget::new(vec!["remove".to_string()], AtomWidgetType::EnabledIcon,
        AtomData::new("Remove", Value::Empty()));
        loot_remove_tile_button.atom_data.text = "Remove".to_string();
        loot_remove_tile_button.set_rect((rect.0 + 190 + 40, rect.1 + rect.3 - bottom_size - toolbar_size - 5, 35, 38), asset, context);
        loot_remove_tile_button.status_help_text = Some("Removes loot.".to_string());
        loot_layout.add(loot_remove_tile_button, 0);

        loot_layout.layout();
        layouts.push(loot_layout);

        // Editing Widgets
        let mut editing_layout = HLayout::new((rect.0 + 180, rect.1 + rect.3 - bottom_size - toolbar_size, rect.2 - 180, toolbar_size));
        editing_layout.margin = (10, 0, 0, 0);
        editing_layout.spacing = 0;

        let mut draw_mode_button = AtomWidget::new(vec!["draw".to_string(), "erase".to_string(), "pick".to_string(), "select".to_string()], AtomWidgetType::IconRow,
        AtomData::new("Mode", Value::Empty()));
        draw_mode_button.atom_data.text = "Draw Mode".to_string();
        draw_mode_button.set_rect((rect.0 + 190, rect.1 + rect.3 - bottom_size - toolbar_size - 2, 135, 33), asset, context);
        draw_mode_button.custom_color = Some([217, 64, 51, 255]);

        let mut status_help_vector : Vec<(String, String)> = vec![];
        status_help_vector.push(("Draw Mode".to_string(), "Draw tiles ('D').".to_string()));
        status_help_vector.push(("Clear Mode".to_string(), "Clear / Erase tiles ('C').".to_string()));
        status_help_vector.push(("Pick Mode".to_string(), "Pick tile. ('X').".to_string()));
        status_help_vector.push(("Select Mode".to_string(), "Select multiple tiles ('R').".to_string()));
        draw_mode_button.status_help_vector = Some(status_help_vector);

        editing_layout.add(draw_mode_button, 0);

        let mut cut_button = AtomWidget::new(vec!["cut".to_string()], AtomWidgetType::CheckedIcon,
        AtomData::new("Cut", Value::Empty()));
        cut_button.atom_data.text = "Cut".to_string();
        cut_button.set_rect((rect.0 + 350, rect.1 + rect.3 - bottom_size - toolbar_size - 2, 40, 33), asset, context);
        cut_button.status_help_text = Some("Copies the selection to the clipboard and clears it.".to_string());

        editing_layout.add(cut_button, 20);

        let mut copy_button = AtomWidget::new(vec!["copy".to_string()], AtomWidgetType::CheckedIcon,
        AtomData::new("Copy", Value::Empty()));
        copy_button.atom_data.text = "Copy".to_string();
        copy_button.set_rect((rect.0 + 350 + 35, rect.1 + rect.3 - bottom_size - toolbar_size - 2, 40, 33), asset, context);
        copy_button.status_help_text = Some("Copies the selection to the clipboard.".to_string());

        editing_layout.add(copy_button, 0);

        let mut paste_button = AtomWidget::new(vec!["paste".to_string()], AtomWidgetType::CheckedIcon,
        AtomData::new("Paste", Value::Empty()));
        paste_button.atom_data.text = "Paste".to_string();
        paste_button.set_rect((rect.0 + 350 + 35 + 35, rect.1 + rect.3 - bottom_size - toolbar_size - 2, 40, 33), asset, context);
        paste_button.status_help_text = Some("Paste the content from the clipboard.".to_string());

        editing_layout.add(paste_button, 0);

        editing_layout.layout();
        layouts.push(editing_layout);

        Self {
            rect,

            editor_rect             : (0, 0, 0, 0),
            preview_rect            : (0, 0, 0, 0),

            region_id               : Uuid::new_v4(),
            grid_size               : 32,

            layouts,

            offset                  : (0, 0),
            screen_offset           : (0, 0),

            tile_selector,
            character_selector,
            loot_selector,
            behavior_graph          : Box::new(behavior_graph),

            mouse_wheel_delta       : (0, 0),
            mouse_hover_pos         : (0, 0),
            clicked                 : None,

            bottom_size,
            toolbar_size,

            debug_update            : None,

            selected_range          : None,
            clipboard               : None,

            undo                    : None,
            has_changed             : false,

            preview_button,
        }
    }

    fn resize(&mut self, width: usize, height: usize, context: &mut ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;

        self.layouts[0].set_rect((self.rect.0, self.rect.1 + self.rect.3 - self.bottom_size - self.toolbar_size, 165, self.toolbar_size));
        self.layouts[1].set_rect((self.rect.0 + 180, self.rect.1 + self.rect.3 - self.bottom_size - self.toolbar_size, self.rect.2 - 180, self.toolbar_size));
        self.layouts[2].set_rect((self.rect.0 + 180, self.rect.1 + self.rect.3 - self.bottom_size - self.toolbar_size, self.rect.2 - 180, self.toolbar_size));
        self.layouts[3].set_rect((self.rect.0 + 180, self.rect.1 + self.rect.3 - self.bottom_size - self.toolbar_size, self.rect.2 - 180, self.toolbar_size));
        self.layouts[4].set_rect((self.rect.0 + 180, self.rect.1 + self.rect.3 - self.bottom_size - self.toolbar_size, self.rect.2 - 180, self.toolbar_size));

        self.preview_button.set_rect2((self.rect.0 + self.rect.2 - 190, self.rect.1, 180, 40));

        self.behavior_graph.rect = (self.rect.0, self.rect.1 + self.rect.3 - self.bottom_size, width, self.bottom_size);
        self.behavior_graph.set_mode_and_rect(GraphMode::Detail, self.behavior_graph.rect, context);
        self.tile_selector.rect = (self.rect.0, self.rect.1 + self.rect.3 - self.bottom_size, width, self.bottom_size);
        self.tile_selector.resize(width, self.bottom_size);
        self.character_selector.rect = (self.rect.0, self.rect.1 + self.rect.3 - self.bottom_size, width, self.bottom_size);
        self.character_selector.resize(width, self.bottom_size);
        self.loot_selector.rect = (self.rect.0, self.rect.1 + self.rect.3 - self.bottom_size, width, self.bottom_size);
        self.loot_selector.resize(width, self.bottom_size);
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &[0,0,0,255]);

        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            let mut rect = self.rect.clone();
            rect.3 -= self.bottom_size + self.toolbar_size;

            if self.preview_button.curr_index == 1 ||  self.preview_button.curr_index == 2 {
                rect.2 -= rect.2 / 3;
            }

            self.editor_rect = rect.clone();

            let grid_size = self.grid_size;

            let left_offset = (rect.2 % grid_size) / 2;
            let top_offset = (rect.3 % grid_size) / 2;

            self.screen_offset = (left_offset, top_offset);

            if let Some(region) = context.data.regions.get(&self.region_id) {

                if context.is_running == false {

                    if editor_mode != RegionEditorMode::Characters && editor_mode != RegionEditorMode::Loot {
                        let mut show_overlay = false;

                        if options.get_layer() == 4 {
                            show_overlay = true;
                        }

                        context.draw2d.draw_region(frame, region, &rect, &(-self.offset.0, -self.offset.1), context.width, grid_size, anim_counter, asset, show_overlay);
                    } else {
                        context.draw2d.draw_region_with_behavior(frame, region, &rect, &(-self.offset.0, -self.offset.1), context.width, grid_size, anim_counter, asset, context);
                    }
                } else {
                    if context.debug_render.is_none() {
                        context.debug_render = Some(GameRender::new(context.curr_project_path.clone(), context.player_id ));
                    }

                    if let Some(update) = &self.debug_update {
                        if let Some(render) = &mut context.debug_render {
                            render.process_update(update);
                            render.process_game_draw_2d(rect, anim_counter, update, &mut Some(frame), context.width);
                        }
                    }
                }

                // Preview

                if self.preview_button.curr_index == 1 || self.preview_button.curr_index == 2 {


                    if let Some(render) = &mut context.debug_render {

                        let mut prev_rect = rect.clone();
                        prev_rect.0 += self.rect.2 / 3 * 2;
                        prev_rect.2 = self.rect.2 / 3;

                        self.preview_rect = prev_rect.clone();

                        let mut update = GameUpdate::new();
                        if let Some(id) = self.get_tile_id(self.mouse_hover_pos) {
                            update.position = Some(Position { region: region.data.id, x: id.0, y: id.1 });
                            update.region = Some(region.data.clone());

                            // Add characters
                            for (id, behavior) in &context.data.behaviors {

                                let mut default_position        : Option<Position> = None;
                                let mut default_tile            : Option<TileId> = None;

                                for (_id, node) in &behavior.data.nodes {
                                    if node.behavior_type == BehaviorNodeType::BehaviorType {
                                        if let Some(value )= node.values.get(&"position".to_string()) {
                                            default_position = value.to_position();
                                        }
                                        if let Some(value )= node.values.get(&"tile".to_string()) {
                                            default_tile = value.to_tile_id()
                                        }
                                    }
                                }

                                if default_position.is_some() && default_position.clone().unwrap().region == region.data.id && default_tile.is_some() {
                                    let character = CharacterData {
                                        position                : default_position.clone().unwrap(),
                                        old_position            : None,
                                        max_transition_time     : 0,
                                        curr_transition_time    : 0,
                                        tile                    : default_tile.clone().unwrap(),
                                        name                    : "".to_string(),
                                        id                      : *id,
                                        index                   : 0,
                                        effects                 : vec![],
                                    };
                                    update.characters.push(character);
                                }

                                for inst_arr in &behavior.data.instances {
                                    for i in inst_arr {
                                        let tile            : Option<TileId>;

                                        if i.tile.is_some() {
                                            tile = i.tile.clone();
                                        } else {
                                            tile = default_tile.clone();
                                        }

                                        if i.position.region == region.data.id && tile.is_some() {
                                            let character = CharacterData {
                                                position                : i.position.clone(),
                                                old_position            : None,
                                                max_transition_time     : 0,
                                                curr_transition_time    : 0,
                                                tile                    : tile.clone().unwrap(),
                                                name                    : "".to_string(),
                                                id                      : *id,
                                                index                   : 0,
                                                effects                 : vec![],
                                            };
                                            update.characters.push(character);
                                        }
                                    }
                                }
                            }

                            // Add Loot

                            for (id, behavior) in &context.data.items  {
                                if let Some(instances) = &behavior.data.loot {
                                    for instance in instances {
                                        if instance.position.region != region.data.id { continue; }
                                        let mut loot = LootData {
                                            id          : id.clone(),
                                            item_type   : "gear".to_string(),
                                            name        : Some(behavior.data.name.clone()),
                                            tile        : None,
                                            state       : None,
                                            light       : None,
                                            slot        : None,
                                            amount      : instance.amount,
                                            stackable   : 1,
                                            static_item : false,
                                            price       : 0.0,
                                            weight      : 0.0,
                                        };

                                        for (_index, node) in &behavior.data.nodes {
                                            if node.behavior_type == BehaviorNodeType::BehaviorType {
                                                if let Some(value) = node.values.get(&"tile".to_string()) {
                                                    loot.tile = value.to_tile_data();
                                                }
                                                if let Some(value) = node.values.get(&"settings".to_string()) {
                                                    if let Some(str) = value.to_string() {
                                                        let mut s = PropertySink::new();
                                                        s.load_from_string(str.clone());
                                                        if let Some(static_item) = s.get("static") {
                                                            if let Some(st) = static_item.as_bool() {
                                                                loot.static_item = st;
                                                            }
                                                        }
                                                        if let Some(stackable_item) = s.get("stackable") {
                                                            if let Some(st) = stackable_item.as_int() {
                                                                if st >= 0 {
                                                                    loot.stackable = st;
                                                                }
                                                            }
                                                        }
                                                        if let Some(price_item) = s.get("price") {
                                                            let price = price_item.to_float();
                                                            if price >= 0.0 {
                                                                loot.price = price;
                                                            }
                                                        }
                                                        if let Some(weight_item) = s.get("weight") {
                                                            let weight = weight_item.to_float();
                                                            if weight >= 0.0 {
                                                                loot.weight = weight;
                                                            }
                                                        }
                                                        if let Some(item_type) = s.get("item_type") {
                                                            if let Some(i_type) = item_type.as_string() {
                                                                loot.item_type = i_type;
                                                            }
                                                        }
                                                        if let Some(item_slot) = s.get("slot") {
                                                            if let Some(slot) = item_slot.as_string() {
                                                                loot.slot = Some(slot);
                                                            }
                                                        }
                                                    }
                                                }
                                            } else
                                            if node.behavior_type == BehaviorNodeType::LightItem {
                                                // Insert a light if we found a light node for the item
                                                let light = LightData {
                                                    light_type              : LightType::PointLight,
                                                    position                : (instance.position.x, instance.position.y),
                                                    intensity               : 1,
                                                };
                                                update.lights.push(light);
                                            }
                                        }

                                        update.loot.insert((instance.position.x, instance.position.y), vec![loot]);
                                    }
                                }

                                if self.preview_button.curr_index == 2 {
                                    render.force_display_mode = Some(DisplayMode::ThreeD);
                                }
                                render.process_update(&update);
                                // render.process_game_draw_auto(prev_rect, anim_counter, &update, &mut Some(frame), context.width);

                                if self.preview_button.curr_index == 1 {
                                    render.process_game_draw_2d(prev_rect, anim_counter, &update, &mut Some(frame), context.width);
                                } else {
                                    render.process_game_draw_3d(prev_rect, anim_counter, &update, &mut Some(frame), context.width);
                                }
                            }
                        }
                    }
                }
            }

            context.draw2d.draw_rect(frame, &(rect.0, rect.1 + rect.3, self.rect.2, self.toolbar_size), context.width, &context.color_black);

            self.layouts[0].draw(frame, anim_counter, asset, context);

            if editor_mode == RegionEditorMode::Tiles {
                self.tile_selector.draw(frame, context.width, anim_counter, asset, context);

                self.layouts[4].draw(frame, anim_counter, asset, context);

                // Draw selection if any
                if context.is_running == false && self.selected_range.is_some() && self.layouts[4].widgets[0].curr_index == 3 {

                    let x_tiles = (rect.2 / grid_size) as isize;
                    let y_tiles = (rect.3 / grid_size) as isize;

                    let mut c = context.color_white.clone();
                    c[3] = 100;

                    for y in 0..y_tiles {
                        for x in 0..x_tiles {

                            let rx = x - self.offset.0;
                            let ry = y - self.offset.1;

                            if let Some(range) = self.selected_range {
                                if rx >= range.0 && ry >= range.1 && rx < range.0 + range.2 && ry < range.1 + range.3 {
                                    let pos = (rect.0 + left_offset + (x as usize) * grid_size, rect.1 + top_offset + (y as usize) * grid_size);

                                    context.draw2d.blend_rect(frame, &(pos.0, pos.1, grid_size, grid_size), context.width, &c);
                                }
                            }
                        }
                    }
                }

                // Draw Paste preview is paste button is enabled and clipboard non-empty
                if context.is_running == false && self.layouts[4].widgets[3].checked == true {
                    if let Some(clipboard) = &self.clipboard {
                        if let Some(id) = self.get_tile_id(self.mouse_hover_pos) {

                            let x_tiles = (rect.2 / grid_size) as isize;
                            let y_tiles = (rect.3 / grid_size) as isize;

                            let mut c = context.color_white.clone();
                            c[3] = 100;

                            for y in 0..y_tiles {
                                for x in 0..x_tiles {

                                    let rx = x - self.offset.0;
                                    let ry = y - self.offset.1;

                                    let ix = rx - id.0;
                                    let iy = ry - id.1;

                                    if ix >= 0 && iy >= 0 {
                                        let pos = (rect.0 + left_offset + (x as usize) * grid_size, rect.1 + top_offset + (y as usize) * grid_size);

                                        if let Some(tile) = clipboard.layer1.get(&(ix, iy)) {
                                            if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                                                context.draw2d.draw_animated_tile(frame, &pos, &map, context.width, &(tile.x_off as usize, tile.y_off as usize), anim_counter, self.grid_size);
                                            }
                                        }
                                        if let Some(tile) = clipboard.layer2.get(&(ix, iy)) {
                                            if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                                                context.draw2d.draw_animated_tile(frame, &pos, &map, context.width, &(tile.x_off as usize, tile.y_off as usize), anim_counter, self.grid_size);
                                            }
                                        }
                                        if let Some(tile) = clipboard.layer3.get(&(ix, iy)) {
                                            if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                                                context.draw2d.draw_animated_tile(frame, &pos, &map, context.width, &(tile.x_off as usize, tile.y_off as usize), anim_counter, self.grid_size);
                                            }
                                        }
                                        if let Some(tile) = clipboard.layer4.get(&(ix, iy)) {
                                            if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                                                context.draw2d.draw_animated_tile(frame, &pos, &map, context.width, &(tile.x_off as usize, tile.y_off as usize), anim_counter, self.grid_size);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else
            if editor_mode == RegionEditorMode::Areas {

                self.layouts[1].draw(frame, anim_counter, asset, context);

                if context.is_running == false {
                    if let Some(region) = context.data.regions.get(&self.region_id) {

                        let x_tiles = (rect.2 / grid_size) as isize;
                        let y_tiles = (rect.3 / grid_size) as isize;

                        let curr_area_index = context.curr_region_area_index;

                        for y in 0..y_tiles {
                            for x in 0..x_tiles {

                                let rx = x - self.offset.0;
                                let ry = y - self.offset.1;

                                for area_index in 0..region.data.areas.len() {
                                    if region.data.areas[area_index].area.contains(&(rx, ry)) {
                                        let pos = (rect.0 + left_offset + (x as usize) * grid_size, rect.1 + top_offset + (y as usize) * grid_size);

                                        let mut c = context.color_white.clone();
                                        if curr_area_index == area_index {
                                            c = context.color_red.clone();
                                            c[3] = 100;
                                        } else {
                                            c[3] = 100;
                                        }
                                        context.draw2d.blend_rect(frame, &(pos.0, pos.1, grid_size, grid_size), context.width, &c);
                                    }
                                }
                            }
                        }
                    }
                }
                self.behavior_graph.draw(frame, anim_counter, asset, context, &mut None);
            } else
            if editor_mode == RegionEditorMode::Characters {
                self.layouts[2].draw(frame, anim_counter, asset, context);
                self.character_selector.draw(frame, context.width, anim_counter, asset, context);
            } else
            if editor_mode == RegionEditorMode::Loot {
                self.layouts[3].draw(frame, anim_counter, asset, context);
                self.loot_selector.draw(frame, context.width, anim_counter, asset, context);
            }

            // Draw a white border around the tile under the mouse cursor
            if self.mouse_hover_pos != (0,0) && context.is_running == false {
                if let Some(id) = self.get_tile_id(self.mouse_hover_pos) {
                    let pos = (rect.0 + left_offset + ((id.0 + self.offset.0) as usize) * grid_size, rect.1 + top_offset + ((id.1 + self.offset.1) as usize) * grid_size);
                    if  pos.0 + grid_size < rect.0 + rect.2 && pos.1 + grid_size < rect.1 + rect.3 {
                        context.draw2d.draw_rect_outline(frame, &(pos.0, pos.1, grid_size, grid_size), context.width, context.color_light_white);
                        context.draw2d.draw_rect_outline(frame, &(pos.0 + 1, pos.1 + 1, grid_size - 2, grid_size - 2), context.width, context.color_black);
                    }
                }
            }
        }

        self.preview_button.draw(frame, context.width, anim_counter, asset, context);
    }

    fn debug_data(&mut self, context: &mut ScreenContext, data: BehaviorDebugData) {
        self.behavior_graph.debug_data(context, data);
    }

    fn debug_update(&mut self, update: GameUpdate, _context: &mut ScreenContext) {
        self.debug_update = Some(update);
    }

    fn get_layer_mask(&mut self, context: &mut ScreenContext) -> Option<Vec<Option<TileData>>> {
        if let Some(id) = self.get_tile_id(self.mouse_hover_pos) {
            if let Some(region) = context.data.regions.get(&self.region_id) {
                let mask = region.get_layer_mask(id);
                return Some(mask);
            }
        }
        None
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        if self.preview_button.mouse_down(pos, asset, context) {
            return true;
        }

        let mut consumed = false;

        let mut rect = self.rect.clone();
        rect.3 -= self.bottom_size + self.toolbar_size;
        rect.2 -= rect.2 / 3;

        if let Some(options) = options {

            if let Some(_id) = self.layouts[0].mouse_down(pos, asset, context) {
                return true;
            }

            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Tiles {
                if self.tile_selector.mouse_down(pos, asset, context) {
                    consumed = true;
                    if let Some(selected) = &self.tile_selector.selected {
                        context.curr_region_tile = Some(selected.clone());
                    } else {
                        context.curr_region_tile = None;
                    }
                }
                if let Some(id) = self.layouts[4].mouse_down(pos, asset, context) {

                    if id.1 == "Cut".to_string() || id.1 == "Copy".to_string() {
                        if let Some(region) = context.data.regions.get_mut(&self.region_id) {

                            if let Some(range) = self.selected_range {
                                let mut clipboard = GameRegionData::new();

                                for y in 0..range.3 {
                                    for x in 0..range.2 {
                                        if let Some(l1) = region.data.layer1.get(&(x + range.0, y + range.1)) {
                                            clipboard.layer1.insert((x, y), l1.clone());
                                        }
                                        if let Some(l2) = region.data.layer2.get(&(x + range.0, y + range.1)) {
                                            clipboard.layer2.insert((x, y), l2.clone());
                                        }
                                        if let Some(l3) = region.data.layer3.get(&(x + range.0, y + range.1)) {
                                            clipboard.layer3.insert((x, y), l3.clone());
                                        }
                                        if let Some(l4) = region.data.layer4.get(&(x + range.0, y + range.1)) {
                                            clipboard.layer4.insert((x, y), l4.clone());
                                        }
                                    }
                                }

                                clipboard.max_pos = (range.2, range.3);

                                self.selected_range = None;
                                self.clipboard = Some(clipboard);

                                if id.1 == "Cut" {
                                    for y in 0..range.3 {
                                        for x in 0..range.2 {
                                            region.clear_value((x + range.0, y + range.1));
                                        }
                                    }
                                    self.has_changed = true;
                                }
                            }
                        }
                        self.layouts[4].widgets[id.0].checked = false;
                        return true;
                    }
                }
            } else
            if editor_mode == RegionEditorMode::Areas {
                if context.contains_pos_for(pos, self.behavior_graph.rect) {
                    consumed = self.behavior_graph.mouse_down(pos, asset, context, &mut None, &mut None);
                    return consumed;
                } else {
                    if let Some(id) = self.layouts[1].mouse_down(pos, asset, context) {

                        if id.1 == "Pick".to_string() {
                            self.layouts[1].widgets[5].checked = false;
                            self.layouts[1].widgets[5].dirty = true;
                            self.layouts[1].widgets[6].checked = false;
                            self.layouts[1].widgets[6].dirty = true;
                        } else
                        if id.1 == "Add".to_string() {
                            self.layouts[1].widgets[4].checked = false;
                            self.layouts[1].widgets[4].dirty = true;
                            self.layouts[1].widgets[6].checked = false;
                            self.layouts[1].widgets[6].dirty = true;
                        } else
                        if id.1 == "Remove".to_string() {
                            self.layouts[0].widgets[4].checked = false;
                            self.layouts[0].widgets[4].dirty = true;
                            self.layouts[0].widgets[5].checked = false;
                            self.layouts[0].widgets[5].dirty = true;
                        }

                        return true;
                    }
                }
            } else
            if editor_mode == RegionEditorMode::Characters {
                if self.character_selector.mouse_down(pos, asset, context) {
                    consumed = true;
                } else {
                    if let Some(id) = self.layouts[2].mouse_down(pos, asset, context) {
                        if id.1 == "Add".to_string() {
                            self.layouts[2].widgets[1].checked = false;
                            self.layouts[2].widgets[1].dirty = true;
                        } else
                        if id.1 == "Remove".to_string() {
                            self.layouts[2].widgets[0].checked = false;
                            self.layouts[2].widgets[0].dirty = true;
                        }
                        return true;
                    }
                }
            } else
            if editor_mode == RegionEditorMode::Loot {
                if self.loot_selector.mouse_down(pos, asset, context) {
                    consumed = true;
                } else {
                    if let Some(id) = self.layouts[3].mouse_down(pos, asset, context) {
                        if id.1 == "Add".to_string() {
                            self.layouts[3].widgets[1].checked = false;
                            self.layouts[3].widgets[1].dirty = true;
                        } else
                        if id.1 == "Remove".to_string() {
                            self.layouts[3].widgets[0].checked = false;
                            self.layouts[3].widgets[0].dirty = true;
                        }
                        return true;
                    }
                }
            }

            // Click inside the editor
            if consumed == false && context.contains_pos_for(pos, rect) {
                if let Some(id) = self.get_tile_id(pos) {
                    self.clicked = Some(id);
                    let editor_mode = options.get_editor_mode();

                    if editor_mode == RegionEditorMode::Tiles {

                        if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                            self.undo = Some(region.get_data());
                        }

                        // Copy from Clipboard
                        if self.clipboard.is_some() && self.layouts[4].widgets[3].checked == true {
                            if let Some(clipboard) = &self.clipboard {
                                if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                    for y in 0..clipboard.max_pos.1 {
                                        for x in 0..clipboard.max_pos.0 {

                                            let ix = id.0 + x;
                                            let iy = id.1 + y;

                                            region.clear_value((ix, iy));

                                            if let Some(tile) = clipboard.layer1.get(&(x, y)) {
                                                region.data.layer1.insert((ix, iy), tile.clone());
                                            }
                                            if let Some(tile) = clipboard.layer2.get(&(x, y)) {
                                                region.data.layer2.insert((ix, iy), tile.clone());
                                            }
                                            if let Some(tile) = clipboard.layer3.get(&(x, y)) {
                                                region.data.layer3.insert((ix, iy), tile.clone());
                                            }
                                            if let Some(tile) = clipboard.layer4.get(&(x, y)) {
                                                region.data.layer4.insert((ix, iy), tile.clone());
                                            }
                                            self.has_changed = true;
                                        }
                                    }
                                }
                            }
                        } else
                        if self.layouts[4].widgets[0].curr_index == 0 {
                            // Draw selected tile
                            if let Some(selected) = &self.tile_selector.selected {
                                if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                    region.set_value(options.get_layer(), id, selected.clone());
                                    self.has_changed = true;
                                }
                            }
                        } else
                        if self.layouts[4].widgets[0].curr_index == 1 {
                            // Clear
                            if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                region.clear_layer_value(options.get_layer(), id);
                                //region.clear_value(id);
                                self.has_changed = true;
                            }
                        } else
                        if self.layouts[4].widgets[0].curr_index == 2 {
                            // Pick selected tile
                            if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                let layer_index = options.get_layer();

                                if layer_index == 1 {
                                    if let Some(tile) = region.data.layer1.get(&id) {
                                        self.tile_selector.selected = Some(tile.clone());
                                    }
                                }
                                if layer_index == 2 {
                                    if let Some(tile) = region.data.layer2.get(&id) {
                                        self.tile_selector.selected = Some(tile.clone());
                                    }
                                }
                                if layer_index == 3 {
                                    if let Some(tile) = region.data.layer3.get(&id) {
                                        self.tile_selector.selected = Some(tile.clone());
                                    }
                                }
                                if layer_index == 4 {
                                    if let Some(tile) = region.data.layer4.get(&id) {
                                        self.tile_selector.selected = Some(tile.clone());
                                    }
                                }
                            }
                            self.layouts[4].widgets[0].curr_index = 0;
                            self.layouts[4].widgets[0].dirty = true;
                        } else
                        if self.layouts[4].widgets[0].curr_index == 3 {
                            // Select range
                            self.selected_range = Some((id.0, id.1, 1, 1));
                        }
                    } else
                    if editor_mode == RegionEditorMode::Areas {
                        let mut update_graph = false;
                        if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                            if region.data.areas.len() > 0 {
                                let area = &mut region.data.areas[context.curr_region_area_index];

                                //

                                let mut mode = 0;

                                if self.layouts[1].widgets[4].checked {
                                    mode = 2;
                                } else
                                if self.layouts[1].widgets[6].checked {
                                    mode = 1;
                                }

                                if mode == 0 {
                                    // Add
                                    if area.area.contains(&id) == false {
                                        area.area.push(id);
                                    }
                                } else
                                if mode == 1 {
                                    // Remove
                                    if area.area.contains(&id) == true {
                                        let index = area.area.iter().position(|&r| r == id).unwrap();
                                        area.area.remove(index);
                                    }
                                } else
                                if mode == 2 {
                                    // Pick
                                    for (index, area) in region.data.areas.iter().enumerate() {
                                        if area.area.contains(&id) {
                                            self.layouts[1].widgets[0].curr_index = index;
                                            self.layouts[1].widgets[0].dirty = true;
                                            context.curr_region_area_index = index;
                                            update_graph = true;
                                            break;
                                        }
                                    }
                                }
                                region.save_data();
                            }
                        }

                        // Set a new area graph if the user picked one via the mouse event
                        if update_graph {
                            if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                                if let Some(graph) = self.get_behavior_graph() {
                                    graph.set_behavior_id(region.behaviors[context.curr_region_area_index].data.id, context);
                                }
                            }
                        }
                    } else
                    if editor_mode == RegionEditorMode::Characters {
                        if let Some(id) = self.get_tile_id(pos) {
                            if let Some(meta) = self.character_selector.selected.clone() {

                                let alignment = context.data.get_behavior_default_alignment(meta.id);

                                if let Some(behavior) = context.data.get_mut_behavior(meta.id, BehaviorType::Behaviors) {
                                    if behavior.data.instances.is_none() {
                                        behavior.data.instances = Some(vec![]);
                                    }

                                    let mode = self.layouts[2].widgets[0].checked;

                                    if mode{
                                        // Add
                                        let index = behavior.data.instances.as_ref().unwrap().iter().position(|r| r.position == Position::new(self.region_id, id.0, id.1));

                                        if index.is_none() {
                                            let instance     = CharacterInstanceData {
                                                position    : Position::new(self.region_id, id.0, id.1),
                                                name        : None,
                                                tile        : None,
                                                alignment   : alignment };
                                            behavior.data.instances.as_mut().unwrap().push(instance);
                                        }
                                    } else {
                                        // Remove
                                        if let Some(index) = behavior.data.instances.as_ref().unwrap().iter().position(|r| r.position == Position::new(self.region_id, id.0, id.1)) {
                                            behavior.data.instances.as_mut().unwrap().remove(index);
                                        }
                                    }
                                    behavior.save_data();
                                }
                            }
                        }
                    } else
                    if editor_mode == RegionEditorMode::Loot {
                        if let Some(id) = self.get_tile_id(pos) {
                            if let Some(meta) = self.loot_selector.selected.clone() {

                                let amount = 1;//context.data.get_behavior_default_alignment(meta.id);
                                let mode = self.layouts[3].widgets[0].checked;

                                if mode {
                                    // Add
                                    if let Some(behavior) = context.data.get_mut_behavior(meta.id, BehaviorType::Items) {

                                        if behavior.data.loot.is_none() {
                                            behavior.data.loot = Some(vec![]);
                                        }



                                        let index = behavior.data.loot.as_ref().unwrap().iter().position(|r| r.position == Position::new(self.region_id, id.0, id.1));

                                            if index.is_none() {
                                                let loot = LootInstanceData {
                                                    position    : Position::new(self.region_id, id.0, id.1),
                                                    name        : None,
                                                    tile        : None,
                                                    amount      : amount };
                                                behavior.data.loot.as_mut().unwrap().push(loot);
                                                behavior.save_data();
                                            }
                                        }
                                } else {
                                    // Remove loot from all items at the given position
                                    for (_id, behavior) in &mut context.data.items {

                                        let mut to_remove = vec![];
                                        if let Some(loot) = &behavior.data.loot {
                                            for (index, l) in loot.iter().enumerate() {
                                                if l.position.region == self.region_id && l.position.x == id.0 && l.position.y == id.1 {
                                                    to_remove.push(index);
                                                }
                                            }
                                        }

                                        for index in to_remove {
                                            behavior.data.loot.as_mut().unwrap().remove(index);
                                            behavior.save_data();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        if self.preview_button.mouse_up(pos, asset, context) {
            return true;
        }

        self.clicked = None;

        let mut consumed = false;
        if let Some(options) = options {

            if let Some(_id) = self.layouts[0].mouse_up(pos, asset, context) {
                context.code_editor_is_active = false;

                let curr_index = self.layouts[0].widgets[0].curr_index;
                if curr_index == 0 {
                    options.set_editor_mode(RegionEditorMode::Tiles);
                } else
                if curr_index == 1 {
                    options.set_editor_mode(RegionEditorMode::Areas);
                } else
                if curr_index == 2 {
                    options.set_editor_mode(RegionEditorMode::Characters);
                    self.character_selector.collect(context);
                } else
                if curr_index == 3 {
                    options.set_editor_mode(RegionEditorMode::Loot);
                    self.loot_selector.collect(context);
                } else
                if curr_index == 4 {
                    options.set_editor_mode(RegionEditorMode::Settings);
                    let value;
                    if let Some(region) = context.data.regions.get(&self.get_region_id()) {
                        value = Value::String(region.data.settings.to_string(generate_region_sink_descriptions()));
                    } else {
                        return false;
                    }
                    let id = context.create_property_id("region_settings");
                    context.code_editor_mode = CodeEditorMode::Settings;
                    context.open_code_editor(id,  value, false);
                }

                return true;
            }

            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Areas {

                if context.contains_pos_for(pos, self.behavior_graph.rect) {
                    consumed = self.behavior_graph.mouse_up(pos, asset, context, &mut None, &mut None);
                } else {
                    if let Some(id) = self.layouts[1].mouse_up(pos, asset, context) {
                        if id.1 == "Area".to_string() {
                            self.update_area_ui(context);
                            if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                                if let Some(graph) = self.get_behavior_graph() {
                                    graph.set_behavior_id(region.behaviors[context.curr_region_area_index].data.id, context);
                                }
                            }
                        } else
                        if id.1 == "Add Area".to_string() {
                            if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                                let id = region.create_area();
                                self.layouts[1].widgets[0].curr_index = region.behaviors.len() - 1;
                                if let Some(graph) = self.get_behavior_graph() {
                                    graph.set_behavior_id(id, context);
                                }
                            }

                            self.update_area_ui(context);
                        } else
                        if id.1 == "Delete".to_string() {
                            if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                                region.delete_area(context.curr_region_area_index);
                            }

                            self.update_area_ui(context);
                        } else
                        if id.1 == "Rename".to_string() {
                            context.dialog_state = DialogState::Opening;
                            context.dialog_height = 0;
                            context.target_fps = 60;
                            context.dialog_entry = DialogEntry::NewName;
                            if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                                context.dialog_new_name = region.get_area_names()[context.curr_region_area_index].clone();
                            }
                            self.update_area_ui(context);
                        }
                    }
                }
            }
            if editor_mode == RegionEditorMode::Tiles {
                if let Some(_id) = self.layouts[4].mouse_up(pos, asset, context) {
                    consumed = true;
                }
            }
            if editor_mode == RegionEditorMode::Characters {
                if let Some(_id) = self.layouts[2].mouse_up(pos, asset, context) {
                    consumed = true;
                }
            }
        }

        // Set undo point

        if self.has_changed {
            if let Some(undo) = &self.undo {
                if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                    region.undo.add(undo.clone(), region.get_data());
                    region.save_data();
                }
            }
        }

        self.has_changed = false;
        self.undo = None;

        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        if self.preview_button.mouse_hover(pos, asset, context) {
            return true;
        }

        if let Some(_id) = self.layouts[0].mouse_hover(pos, asset, context) {
            return true;
        }

        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();
            if editor_mode == RegionEditorMode::Areas {


                if let Some(_id) = self.layouts[1].mouse_hover(pos, asset, context) {
                    return true;
                }
            } else
            if editor_mode == RegionEditorMode::Tiles {
                if let Some(_id) = self.layouts[4].mouse_hover(pos, asset, context) {
                    return true;
                }
            }
            if editor_mode == RegionEditorMode::Characters {
                if let Some(_id) = self.layouts[2].mouse_hover(pos, asset, context) {
                    return true;
                }
            }
        }

        if context.contains_pos_for(pos, self.editor_rect) {
            self.mouse_hover_pos = pos.clone();
        }
        true
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        let mut consumed = false;
        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Areas {
                if context.contains_pos_for(pos, self.behavior_graph.rect) {
                    consumed = self.behavior_graph.mouse_dragged(pos, asset, context, &mut None, &mut None);
                    return consumed;
                }
            }

            if consumed == false && context.contains_pos_for(pos, self.rect) {
                if let Some(id) = self.get_tile_id(pos) {
                    if self.clicked != Some(id) {

                        self.clicked = Some(id);
                        let editor_mode = options.get_editor_mode();

                        if editor_mode == RegionEditorMode::Tiles {

                            // Copy from Clipboard
                            if self.clipboard.is_some() && self.layouts[4].widgets[3].checked == true {
                                if let Some(clipboard) = &self.clipboard {
                                    if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                        for y in 0..clipboard.max_pos.1 {
                                            for x in 0..clipboard.max_pos.0 {

                                                let ix = id.0 + x;
                                                let iy = id.1 + y;

                                                region.clear_value((ix, iy));

                                                if let Some(tile) = clipboard.layer1.get(&(x, y)) {
                                                    region.data.layer1.insert((ix, iy), tile.clone());
                                                }
                                                if let Some(tile) = clipboard.layer2.get(&(x, y)) {
                                                    region.data.layer2.insert((ix, iy), tile.clone());
                                                }
                                                if let Some(tile) = clipboard.layer3.get(&(x, y)) {
                                                    region.data.layer3.insert((ix, iy), tile.clone());
                                                }
                                                if let Some(tile) = clipboard.layer4.get(&(x, y)) {
                                                    region.data.layer4.insert((ix, iy), tile.clone());
                                                }
                                                self.has_changed = true;
                                            }
                                        }
                                    }
                                }
                            } else
                            if self.layouts[4].widgets[0].curr_index == 0 {
                                // Draw selected tile
                                if let Some(selected) = &self.tile_selector.selected {
                                    if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                        region.set_value(options.get_layer(), id, selected.clone());
                                        self.has_changed = true;
                                    }
                                }
                            } else
                            if self.layouts[4].widgets[0].curr_index == 1 {
                                // Clear
                                if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                    region.clear_value(id);
                                    self.has_changed = true;
                                }
                            } else
                            if self.layouts[4].widgets[0].curr_index == 2 {
                                // Pick selected tile
                                if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                    let s = region.get_value(id);
                                    if s.len() > 0 {
                                        self.tile_selector.selected = Some(s[0].clone());
                                    }
                                }
                            } else
                            if self.layouts[4].widgets[0].curr_index == 3 {
                                // Select range
                                if let Some(mut range) = self.selected_range {
                                    range.2 = (id.0 - range.0 + 1).max(1);
                                    range.3 = (id.1 - range.1 + 1).max(1);
                                    self.selected_range = Some(range);
                                }
                            }
                        }
                    }
                }

                consumed = true;
            }
        }
        consumed
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        let mut consumed = false;
        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Tiles {
                if context.contains_pos_for(self.mouse_hover_pos, self.tile_selector.rect) && self.tile_selector.mouse_wheel(delta, asset, context) {
                    consumed = true;
                }
            } else
            if editor_mode == RegionEditorMode::Areas {
                if context.contains_pos_for(self.mouse_hover_pos, self.behavior_graph.rect) && self.behavior_graph.mouse_wheel(delta, asset, context, &mut None, &mut None) {
                    consumed = true;
                }
            } else
            if editor_mode == RegionEditorMode::Characters {
                if context.contains_pos_for(self.mouse_hover_pos, self.character_selector.rect) && self.character_selector.mouse_wheel(delta, asset, context) {
                    consumed = true;
                }
            } else
            if editor_mode == RegionEditorMode::Loot {
                if context.contains_pos_for(self.mouse_hover_pos, self.loot_selector.rect) && self.loot_selector.mouse_wheel(delta, asset, context) {
                    consumed = true;
                }
            }

            if consumed == false {
                if context.contains_pos_for(self.mouse_hover_pos, self.editor_rect) {
                    self.mouse_wheel_delta.0 += delta.0;
                    self.mouse_wheel_delta.1 += delta.1;

                    self.offset.0 += self.mouse_wheel_delta.0 / self.grid_size as isize;
                    self.offset.1 += self.mouse_wheel_delta.1 / self.grid_size as isize;

                    self.mouse_wheel_delta.0 -= (self.mouse_wheel_delta.0 / self.grid_size as isize) * self.grid_size as isize;
                    self.mouse_wheel_delta.1 -= (self.mouse_wheel_delta.1 / self.grid_size as isize) * self.grid_size as isize;

                    if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                        region.data.editor_offset = Some(self.offset.clone());
                        region.save_data();
                    }
                }
            }
        }
        true
    }

    /// Key down
    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, _asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        if let Some(key) = key {
            if key == WidgetKey::Left {
                self.offset.0 -= 1;
                return true;
            } else
            if key == WidgetKey::Right {
                self.offset.0 += 1;
                return true;
            } else
            if key == WidgetKey::Up {
                self.offset.1 -= 1;
                return true;
            } else
            if key == WidgetKey::Down {
                self.offset.1 += 1;
                return true;
            }
        }

        if let Some(options) = options {
            if let Some(char) = char {

                if char == '1' {
                    options.set_layer(1);
                    return true;
                } else
                if char == '2' {
                    options.set_layer(2);
                    return true;
                } else
                if char == '3' {
                    options.set_layer(3);
                    return true;
                } else
                if char == '4' {
                    options.set_layer(4);
                    return true;
                }

                if char == 'd' {
                    self.layouts[0].widgets[0].curr_index = 0;
                    self.layouts[0].widgets[0].dirty = true;
                    options.set_editor_mode(RegionEditorMode::Tiles);
                    self.layouts[4].widgets[0].curr_index = 0;
                    self.layouts[4].widgets[0].dirty = true;
                    return true;
                } else
                if char == 'e' {
                    self.layouts[0].widgets[0].curr_index = 1;
                    self.layouts[0].widgets[0].dirty = true;
                    options.set_editor_mode(RegionEditorMode::Areas);
                    return true;
                } else
                if char == 'a' {
                    self.layouts[0].widgets[0].curr_index = 2;
                    self.layouts[0].widgets[0].dirty = true;
                    options.set_editor_mode(RegionEditorMode::Characters);
                    self.character_selector.collect(context);
                    return true;
                } else
                if char == 'l' {
                    self.layouts[0].widgets[0].curr_index = 3;
                    self.layouts[0].widgets[0].dirty = true;
                    options.set_editor_mode(RegionEditorMode::Loot);
                    self.loot_selector.collect(context);
                    return true;
                } else
                if char == 's' {
                    self.layouts[0].widgets[0].curr_index = 4;
                    self.layouts[0].widgets[0].dirty = true;
                    options.set_editor_mode(RegionEditorMode::Settings);
                    let value;
                    if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                        value = Value::String(region.data.settings.to_string(generate_region_sink_descriptions()));
                    } else {
                        return false;
                    }
                    let id = context.create_property_id("region_settings");
                    context.code_editor_mode = CodeEditorMode::Settings;
                    context.open_code_editor(id, value, false);
                    return true;
                } else
                if char == 'c' {
                    self.layouts[4].widgets[0].curr_index = 1;
                    self.layouts[4].widgets[0].dirty = true;
                    return true;
                } else
                if char == 'x' {
                    self.layouts[4].widgets[0].curr_index = 2;
                    self.layouts[4].widgets[0].dirty = true;
                    return true;
                } else
                if char == 'r' {
                    self.layouts[4].widgets[0].curr_index = 3;
                    self.layouts[4].widgets[0].dirty = true;
                    return true;
                }
                if char == 'o' {
                    if self.grid_size > 0 {
                        self.grid_size -= 2;
                    }
                    return true;
                } else
                if char == 'i' {
                    if self.grid_size < 100 {
                        self.grid_size += 2;
                    }
                    return true;
                } else
                if char == 'p' {
                    if self.preview_button.curr_index == 0 {
                        self.preview_button.curr_index = 1;
                    } else
                    if self.preview_button.curr_index == 1 {
                        self.preview_button.curr_index = 2;
                    } else {
                        self.preview_button.curr_index = 0;
                    }
                    self.preview_button.dirty = true;
                    return true;
                }
            }
        }
        false
    }

    /// Sets a region id
    fn set_region_id(&mut self, id: Uuid, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) {
        self.region_id = id;
        if let Some(region) = context.data.regions.get_mut(&self.region_id) {

            if let Some(editor_offset) = region.data.editor_offset {
                self.offset = editor_offset.clone();
            } else {
                self.offset = (0, 0);
            }

            // Make sure we have the renderer for preview
            if context.debug_render.is_none() {
                context.debug_render = Some(GameRender::new(context.curr_project_path.clone(), context.player_id ));
            }

            self.layouts[1].widgets[0].text = region.get_area_names();
            self.layouts[1].widgets[0].dirty = true;

            if context.curr_region_area_index >= region.data.areas.len() {
                context.curr_region_area_index = 0;
            }
            if region.behaviors.len() > 0 {
                self.behavior_graph.set_behavior_id(region.behaviors[context.curr_region_area_index].data.id, context);
            }
        }

        if let Some(options) = options {
            let mode = options.get_editor_mode();
            if mode == RegionEditorMode::Settings {
                let value;
                if let Some(region) = context.data.regions.get_mut(&id) {
                    value = Value::String(region.data.settings.to_string(generate_region_sink_descriptions()));
                } else {
                    return
                }

                let id = context.create_property_id("region_settings");
                context.code_editor_mode = CodeEditorMode::Settings;
                context.open_code_editor(id, value, false);
            }
        }

        self.update_area_ui(context);
    }

    /// Get the tile id
    fn get_tile_id(&self, pos: (usize, usize)) -> Option<(isize, isize)> {
        let grid_size = self.grid_size;
        if pos.0 > self.rect.0 + self.screen_offset.0 && pos.1 > self.rect.1 + self.screen_offset.1
        && pos.0 < self.rect.0 + self.rect.2 - self.screen_offset.0  && pos.1 < self.rect.1 + self.rect.3 - self.screen_offset.1 - self.bottom_size
        {
            let x = ((pos.0 - self.rect.0 - self.screen_offset.0) / grid_size) as isize - self.offset.0;
            let y = ((pos.1 - self.rect.1 - self.screen_offset.0) / grid_size) as isize - self.offset.1;
            return Some((x, y));
        }
        None
    }

    /// Returns the selected tile
    fn get_selected_tile(&self) -> Option<TileData> {
        self.tile_selector.selected.clone()
    }

    /// Return the tile_selector
    fn get_tile_selector(&mut self) -> Option<&mut TileSelectorWidget> {
        Some(&mut self.tile_selector)
    }

    /// Return the behavior graph
    fn get_behavior_graph(&mut self) -> Option<&mut NodeGraph> {
        Some(&mut self.behavior_graph)
    }

    /// Returns the region_id
    fn get_region_id(&self) -> Uuid {
        self.region_id
    }

    /// Returns the rect for DnD
    fn get_rect(&self) -> (usize, usize, usize, usize) {
        self.behavior_graph.rect.clone()
    }

    /// Adds the given node to the behavior graph (after DnD)
    fn add_node_of_name(&mut self, name: String, position: (isize, isize), context: &mut ScreenContext) {
        self.behavior_graph.add_node_of_name(name, position, context);
    }

    /// Update based on changes
    fn update_from_dialog(&mut self, id: (Uuid, Uuid, String), value: Value, asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) {

        //println!("{:?} {:?}", id, value);

        if id.2 == "region_settings" {
            let mut sink = PropertySink::new();
            if sink.load_from_string(context.code_editor_value.clone()) {
                context.code_editor_error = None;
                let id = self.get_region_id();
                if let Some(region) = context.data.regions.get_mut(&id) {
                    region.data.settings = sink;
                    region.save_data();
                }
            } else {
                context.code_editor_error = Some((sink.error.clone().unwrap().1, Some(sink.error.unwrap().0)));
            }
        } else {
            self.behavior_graph.update_from_dialog(id, value, asset, context, options);
        }
    }

    /// Update the area ui
    fn update_area_ui(&mut self, context: &mut ScreenContext) {
        if let Some(region) = context.data.regions.get(&self.get_region_id()) {

            let area_count = region.data.areas.len();

            if area_count == 0 {
                self.layouts[1].widgets[0].text = vec![];
                self.layouts[1].widgets[0].curr_index = 0;
                self.layouts[1].widgets[0].state = WidgetState::Disabled;
                self.layouts[1].widgets[2].state = WidgetState::Disabled;
                self.layouts[1].widgets[3].state = WidgetState::Disabled;
            } else {
                self.layouts[1].widgets[0].text = region.get_area_names();
                if self.layouts[1].widgets[0].curr_index >= self.layouts[1].widgets[0].text.len() {
                    self.layouts[1].widgets[0].curr_index = 0;
                }
                self.layouts[1].widgets[0].state = WidgetState::Normal;
                self.layouts[1].widgets[1].state = WidgetState::Normal;
                self.layouts[1].widgets[2].state = WidgetState::Normal;
                self.layouts[1].widgets[3].state = WidgetState::Normal;
            }

            for a in &mut self.layouts[1].widgets {
                a.dirty = true;
            }

            context.curr_region_area_index = self.layouts[1].widgets[0].curr_index;

            region.save_data();
        }
    }

    /// Sets a new name for the current area
    fn set_area_name(&mut self, name: String, context: &mut ScreenContext) {
        if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
            region.data.areas[context.curr_region_area_index].name = name;
            self.update_area_ui(context);
        }
    }

    // Undo / Redo

    fn is_undo_available(&self, context: &ScreenContext) -> bool {
        if self.layouts[0].widgets[0].curr_index == 0 {
            // Tiles
            if let Some(region) = context.data.regions.get(&self.get_region_id()) {
                return region.is_undo_available();
            }
        }
        false
    }
    fn is_redo_available(&self, context: &ScreenContext) -> bool {
        if self.layouts[0].widgets[0].curr_index == 0 {
            // Tiles
            if let Some(region) = context.data.regions.get(&self.get_region_id()) {
                return region.is_redo_available();
            }
        }
        false
    }

    fn undo(&mut self, context: &mut ScreenContext) {
        if self.layouts[0].widgets[0].curr_index == 0 {
            // Tiles
            if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                region.undo();
            }
        }
    }

    fn redo(&mut self, context: &mut ScreenContext) {
        if self.layouts[0].widgets[0].curr_index == 0 {
            // Tiles
            if let Some(region) = context.data.regions.get_mut(&self.get_region_id()) {
                region.redo();
            }
        }
    }

}