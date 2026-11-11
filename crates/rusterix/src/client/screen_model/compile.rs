use super::*;

impl ScreenElements {
    pub fn compile(
        screen: &Map,
        assets: &Assets,
        layout: ScreenLayout,
        background: Pixel,
        ui_state: FxHashMap<String, String>,
        animation_frame: usize,
    ) -> Self {
        let mut elements = Self {
            layout,
            background,
            ui_state,
            animation_frame,
            toolbar_actions_panel_config: CatalogPanelConfig::actions_default(),
            ..Default::default()
        };
        let mut screen = screen.clone();
        // Iterate sectors and apply layer property for sorted drawing
        {
            for sector in screen.sectors.iter_mut() {
                if let Some(crate::Value::Str(data)) = sector.properties.get("data") {
                    if let Ok(table) = data.parse::<Table>() {
                        if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                            if let Some(value) = ui.get("layer") {
                                if let Some(v) = value.as_integer() {
                                    sector.properties.set("layer", Value::Int(v as i32));
                                }
                            }
                        }
                    }
                }
            }
        }

        elements.populate(&screen, assets);
        elements.source = screen;
        elements
    }

    fn populate(&mut self, screen: &Map, assets: &Assets) {
        {
            for widget in screen.sectors.iter() {
                let bb = widget.bounding_box(screen);

                let (start_x, start_y) = crate::utils::align_screen_to_grid(
                    self.layout.reference_viewport.x as f32,
                    self.layout.reference_viewport.y as f32,
                    self.layout.grid_size,
                );

                let mut authored_rect = Rect::new(
                    (bb.min.x - start_x) * self.layout.grid_size,
                    (bb.min.y - start_y) * self.layout.grid_size,
                    bb.size().x * self.layout.grid_size,
                    bb.size().y * self.layout.grid_size,
                );

                let source = widget
                    .properties
                    .get_default_source()
                    .cloned()
                    .unwrap_or(PixelSource::Off);
                let active = widget
                    .properties
                    .get_source("ceiling_source")
                    .cloned()
                    .unwrap_or_else(|| source.clone());
                let artwork = widget::artwork::ScreenArtwork {
                    sources: vec![source, active],
                    repeat: widget.properties.get_int_default("tile_mode", 0) != 0,
                    origin: [bb.min.x, bb.min.y],
                    units_per_pixel: 1. / self.layout.grid_size.max(1.),
                    ..Default::default()
                };
                let textures = Vec::new();

                if let Some(crate::Value::Str(data)) = widget.properties.get("data") {
                    if let Ok(table) = data.parse::<Table>() {
                        let grid_size = self.layout.grid_size;

                        let mut role = "none";
                        if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                            if let Some(value) = ui.get("role") {
                                if let Some(v) = value.as_str() {
                                    role = v;
                                }
                            }
                        }

                        if role == "action_bar" {
                            authored_rect = action_bar_authored_rect(authored_rect, &table);
                        }
                        let rect = self.layout.resolve(authored_rect, role, &table);
                        let (x, y, width, height) = (rect.x, rect.y, rect.width, rect.height);
                        let mut artwork = artwork.clone();
                        artwork.slice = table
                            .get("ui")
                            .and_then(toml::Value::as_table)
                            .and_then(|ui| ui.get("slice"))
                            .and_then(toml::Value::as_integer)
                            .unwrap_or(0)
                            .max(0) as usize;

                        if role == "game" {
                            let mut game_widget = GameWidget {
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::default(),
                                grid_size,
                                ..Default::default()
                            };

                            game_widget.init();
                            self.game_widgets.insert(widget.creator_id, game_widget);
                        } else if role == "button" {
                            let mut command = None;
                            let mut command_slot = None;
                            let mut action = "";
                            let mut intent = None;
                            let mut spell = None;
                            let mut group = None;
                            let mut binding = None;
                            let mut value = None;
                            let mut binding_append = false;
                            let mut binding_separator = " ".to_string();
                            let mut binding_max_parts = None;
                            let mut selection = None;
                            let mut show: Option<Vec<String>> = None;
                            let mut hide: Option<Vec<String>> = None;
                            let mut deactivate: Vec<String> = vec![];
                            let mut camera: Option<PlayerCamera> = None;
                            let mut player_camera: Option<PlayerCamera> = None;
                            let mut camera_target: Option<String> = None;
                            let mut party: Option<String> = None;
                            let mut inventory_index: Option<usize> = None;
                            let mut equipped_slot: Option<String> = None;
                            let mut portrait = false;
                            let mut drag_drop = false;

                            let mut entity_cursor_id = None;
                            let mut entity_clicked_cursor_id = None;
                            let mut item_cursor_id = None;
                            let mut item_clicked_cursor_id = None;
                            let mut border_size: i32 = 0;
                            let mut show_icon = true;
                            let mut border_color: [u8; 4] = [255, 255, 255, 255];
                            let mut label = String::new();
                            let mut label_font = String::new();
                            let mut label_font_size = 18.0;
                            let mut label_color: [u8; 4] = [255, 255, 255, 255];
                            let mut background_color = None;
                            let mut hover_style = ButtonStateStyle::default();
                            let mut selected_style = ButtonStateStyle::default();
                            let mut pressed_style = ButtonStateStyle::default();
                            let mut disabled_style = ButtonStateStyle::default();

                            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                                // Check for command. This is the preferred button API.
                                if let Some(value) = ui.get("command")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        command = Some(trimmed.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("command_slot")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        command_slot = Some(trimmed.to_string());
                                    }
                                }

                                // Check for action
                                if let Some(value) = ui.get("action") {
                                    if let Some(v) = value.as_str() {
                                        action = v;
                                    }
                                }

                                // Check for intent
                                if let Some(value) = ui.get("intent") {
                                    if let Some(v) = value.as_str() {
                                        intent = Some(v.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("spell")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        spell = Some(trimmed.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("group")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        group = Some(trimmed.to_string());
                                    }
                                }
                                if let Some(v) = ui
                                    .get("bind")
                                    .or_else(|| ui.get("binding"))
                                    .and_then(toml::Value::as_str)
                                    .map(str::trim)
                                    .filter(|value| !value.is_empty())
                                {
                                    binding = Some(v.to_string());
                                }
                                if let Some(v) = ui.get("value").and_then(toml::Value::as_str) {
                                    value = Some(v.to_string());
                                }
                                if let Some(v) = ui
                                    .get("append")
                                    .or_else(|| ui.get("binding_append"))
                                    .and_then(toml::Value::as_bool)
                                {
                                    binding_append = v;
                                }
                                if let Some(v) = ui
                                    .get("separator")
                                    .or_else(|| ui.get("binding_separator"))
                                    .and_then(toml::Value::as_str)
                                {
                                    binding_separator = v.to_string();
                                }
                                if let Some(v) = ui
                                    .get("max_parts")
                                    .or_else(|| ui.get("binding_max_parts"))
                                    .and_then(toml::Value::as_integer)
                                    && v > 0
                                {
                                    binding_max_parts = Some(v as usize);
                                }
                                if let Some(v) = ui
                                    .get("selection")
                                    .and_then(toml::Value::as_str)
                                    .map(str::trim)
                                    .filter(|value| !value.is_empty())
                                {
                                    selection = Some(v.to_string());
                                }

                                if let Some(value) = ui.get("label").or_else(|| ui.get("text"))
                                    && let Some(v) = value.as_str()
                                {
                                    label = v.to_string();
                                }
                                if let Some(value) = ui.get("font")
                                    && let Some(v) = value.as_str()
                                {
                                    label_font = v.to_string();
                                }
                                if let Some(value) = ui.get("font_size") {
                                    if let Some(v) = value.as_float() {
                                        label_font_size = v as f32;
                                    } else if let Some(v) = value.as_integer() {
                                        label_font_size = v as f32;
                                    }
                                }
                                if let Some(value) = ui.get("color")
                                    && let Some(v) = value.as_str()
                                {
                                    label_color = hex_to_rgba_u8(v);
                                }
                                if let Some(value) = ui
                                    .get("show_icon")
                                    .or_else(|| ui.get("icon"))
                                    .and_then(toml::Value::as_bool)
                                {
                                    show_icon = value;
                                }
                                if let Some(color) = ui_style_color(ui, None, "text", "color")
                                    .or_else(|| ui_style_color(ui, None, "color", "color"))
                                {
                                    label_color = color;
                                }
                                background_color =
                                    ui_style_color(ui, None, "background", "background_color");
                                hover_style = button_state_style_from_ui(
                                    ui,
                                    "hover",
                                    "hover_background_color",
                                    "hover_border_color",
                                    "hover_color",
                                );
                                selected_style = button_state_style_from_ui(
                                    ui,
                                    "selected",
                                    "selected_background_color",
                                    "selected_border_color",
                                    "selected_color",
                                );
                                pressed_style = button_state_style_from_ui(
                                    ui,
                                    "pressed",
                                    "pressed_background_color",
                                    "pressed_border_color",
                                    "pressed_color",
                                );
                                disabled_style = button_state_style_from_ui(
                                    ui,
                                    "disabled",
                                    "disabled_background_color",
                                    "disabled_border_color",
                                    "disabled_color",
                                );

                                // Check for show
                                if let Some(value) = ui.get("show") {
                                    if let Some(va) = value.as_array() {
                                        let mut c = vec![];
                                        for v in va {
                                            if let Some(v) = v.as_str() {
                                                c.push(v.to_string());
                                            }
                                        }
                                        if !c.is_empty() {
                                            show = Some(c);
                                        }
                                    }
                                }

                                // Check for hide
                                if let Some(value) = ui.get("hide") {
                                    if let Some(va) = value.as_array() {
                                        let mut c = vec![];
                                        for v in va {
                                            if let Some(v) = v.as_str() {
                                                c.push(v.to_string());
                                            }
                                        }
                                        if !c.is_empty() {
                                            hide = Some(c);
                                        }
                                    }
                                }

                                // Check for deactivate
                                if let Some(value) = ui.get("deactivate") {
                                    if let Some(va) = value.as_array() {
                                        let mut c = vec![];
                                        for v in va {
                                            if let Some(v) = v.as_str() {
                                                c.push(v.to_string());
                                            }
                                        }
                                        deactivate = c;
                                    }
                                }

                                // Check camera mode switch for game widget rendering
                                if let Some(value) = ui.get("camera")
                                    && let Some(v) = value.as_str()
                                {
                                    camera = parse_player_camera_mode(v);
                                }

                                // Check player camera mapping switch for server controls.
                                if let Some(value) = ui.get("player_camera")
                                    && let Some(v) = value.as_str()
                                {
                                    player_camera = parse_player_camera_mode(v);
                                }

                                // Optional game widget name target (defaults to all game widgets)
                                if let Some(value) = ui.get("camera_target")
                                    && let Some(v) = value.as_str()
                                    && !v.is_empty()
                                {
                                    camera_target = Some(v.to_string());
                                }

                                if let Some(value) = ui.get("party").and_then(toml::Value::as_str) {
                                    let binding = value.trim();
                                    if !binding.is_empty() {
                                        party = Some(binding.to_string());
                                    }
                                }

                                // Check for active
                                if let Some(active_value) = ui.get("active") {
                                    if let Some(v) = active_value.as_bool()
                                        && v
                                        && binding.as_ref().zip(value.as_ref()).is_none_or(
                                            |(binding, value)| {
                                                self.ui_state.get(binding).is_none_or(|current| {
                                                    current
                                                        .trim()
                                                        .eq_ignore_ascii_case(value.trim())
                                                })
                                            },
                                        )
                                    {
                                        self.activated_widgets.push(widget.id);
                                        self.permanently_activated_widgets.push(widget.id);
                                        if let (Some(binding), Some(bound_value)) =
                                            (binding.as_deref(), value.as_deref())
                                            && !binding.trim().is_empty()
                                        {
                                            self.ui_state
                                                .entry(binding.to_string())
                                                .or_insert_with(|| bound_value.to_string());
                                        }
                                        if let Some(hide) = &hide {
                                            self.widgets_to_hide = hide.clone();
                                        }
                                    }
                                }

                                // Check for inventory
                                if let Some(value) = ui.get("inventory_index") {
                                    if let Some(v) = value.as_integer() {
                                        inventory_index = Some(v as usize);
                                    }
                                }
                                if let Some(value) = ui.get("equipped_slot")
                                    && let Some(v) = value.as_str()
                                {
                                    let slot = v.trim();
                                    if !slot.is_empty() {
                                        equipped_slot = Some(slot.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("drag_drop")
                                    && let Some(v) = value.as_bool()
                                {
                                    drag_drop = v;
                                }
                                if let Some(value) = ui.get("portrait")
                                    && let Some(v) = value.as_bool()
                                {
                                    portrait = v;
                                }

                                if inventory_index.is_some() || equipped_slot.is_some() {
                                    drag_drop = ui
                                        .get("drag_drop")
                                        .and_then(toml::Value::as_bool)
                                        .unwrap_or(true);
                                }

                                // Check for the entity / item cursor ids
                                entity_cursor_id = get_uuid(ui, "entity_cursor_id");
                                entity_clicked_cursor_id = get_uuid(ui, "entity_clicked_cursor_id");
                                item_cursor_id = get_uuid(ui, "item_cursor_id");
                                item_clicked_cursor_id = get_uuid(ui, "item_clicked_cursor_id");

                                // Check for border
                                if let Some(value) = ui.get("border_size") {
                                    if let Some(v) = value.as_integer() {
                                        border_size = v as i32;
                                    }
                                }
                                if let Some(value) = ui.get("border_color") {
                                    if let Some(v) = value.as_str() {
                                        border_color = hex_to_rgba_u8(v);
                                    }
                                }
                                if let Some(color) =
                                    ui_style_color(ui, None, "border", "border_color")
                                {
                                    border_color = color;
                                }

                                command = command_from_legacy_fields(
                                    command.as_deref(),
                                    (!action.trim().is_empty()).then_some(action),
                                    intent.as_deref(),
                                    spell.as_deref(),
                                );
                            }

                            let (
                                border_color,
                                border_size,
                                border_gradient_color,
                                border_gradient_direction,
                                border_radius,
                            ) = table
                                .get("ui")
                                .and_then(toml::Value::as_table)
                                .map(|ui| border_style_from_table(ui, border_color, border_size))
                                .unwrap_or((
                                    border_color,
                                    border_size,
                                    None,
                                    BorderGradientDirection::Vertical,
                                    0.0,
                                ));

                            let button_widget = Widget {
                                name: widget.name.clone(),
                                id: widget.id,
                                rect: Rect::new(x, y, width, height),
                                action: action.into(),
                                command,
                                command_slot,
                                intent,
                                spell,
                                group,
                                binding,
                                value,
                                binding_append,
                                binding_separator,
                                binding_max_parts,
                                selection,
                                show,
                                hide,
                                deactivate,
                                camera,
                                player_camera,
                                camera_target,
                                party,
                                inventory_index,
                                equipped_slot,
                                portrait,
                                drag_drop,
                                chrome_textures: vec![],
                                chrome_slice: 0,
                                frame_textures: vec![],
                                frame_slice: 0,
                                icon_inset: None,
                                artwork: artwork.clone(),
                                chrome_artwork: Default::default(),
                                frame_artwork: Default::default(),
                                textures,
                                entity_cursor_id,
                                entity_clicked_cursor_id,
                                item_cursor_id,
                                item_clicked_cursor_id,
                                border_color,
                                border_size,
                                border_gradient_color,
                                border_gradient_direction,
                                border_radius,
                                border_painter: ThePainter::new(),
                                show_icon,
                                label,
                                label_font,
                                label_font_size,
                                label_color,
                                background_color,
                                hover_style,
                                selected_style,
                                pressed_style,
                                disabled_style,
                            };

                            self.button_widgets.insert(widget.id, button_widget);
                        } else if role == "action_bar" {
                            self.insert_action_bar(
                                screen,
                                widget.id,
                                widget.creator_id,
                                &widget.name,
                                rect,
                                &table,
                                &artwork,
                                assets,
                            );
                        } else if role == "spellbook" || role == "actions" {
                            let Some(ui) = table.get("ui").and_then(toml::Value::as_table) else {
                                continue;
                            };
                            let config = catalog_panel_config(ui, Some(&table), Some(rect), assets);
                            if role == "spellbook" {
                                self.custom_spellbook_config = Some(config);
                            } else {
                                let mut config = config;
                                if !ui.contains_key("show_assign") {
                                    config.show_assign = true;
                                }
                                if !ui.contains_key("show_details") {
                                    config.show_details = false;
                                }
                                if !ui.contains_key("show_names") {
                                    config.show_names = true;
                                }
                                if config.title.is_empty() {
                                    config.title = "Actions".to_string();
                                }
                                self.custom_actions_panel_config = Some(config);
                            }
                        } else if role == "inventory" {
                            let Some(ui) = table.get("ui").and_then(toml::Value::as_table) else {
                                continue;
                            };
                            self.custom_inventory_panel_config =
                                Some(inventory_panel_config(ui, Some(rect)));
                        } else if role == "equipment" {
                            let Some(ui) = table.get("ui").and_then(toml::Value::as_table) else {
                                continue;
                            };
                            self.custom_equipment_panel_config =
                                Some(equipment_panel_config(ui, Some(rect)));
                        } else if role == "tab_bar" || role == "dropdown" {
                            let Some(ui) = table.get("ui").and_then(toml::Value::as_table) else {
                                continue;
                            };
                            let kind = if role == "tab_bar" {
                                ChoiceWidgetKind::TabBar
                            } else {
                                ChoiceWidgetKind::Dropdown
                            };
                            if let Some(mut choice) = ChoiceWidget::from_ui(
                                widget.name.clone(),
                                widget.id,
                                Rect::new(x, y, width, height),
                                kind,
                                ui,
                            ) {
                                if let Some(value) = self.ui_state.get(&choice.binding) {
                                    choice.sync_value(Some(value));
                                } else {
                                    self.ui_state.insert(
                                        choice.binding.clone(),
                                        choice.selected_value().to_string(),
                                    );
                                }
                                self.choice_widgets.insert(widget.id, choice);
                            }
                        } else if role == "input" {
                            let mut binding = widget.name.clone();
                            let mut text = String::new();
                            let mut font = String::new();
                            let mut font_size = 22.0;
                            let mut color: [u8; 4] = [242, 242, 242, 255];
                            let mut background_color: [u8; 4] = [17, 17, 17, 204];
                            let mut border_color: [u8; 4] = [136, 136, 136, 255];
                            let mut border_size: i32 = 1;

                            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                                if let Some(value) = ui
                                    .get("bind")
                                    .or_else(|| ui.get("binding"))
                                    .and_then(toml::Value::as_str)
                                {
                                    let trimmed = value.trim();
                                    if !trimmed.is_empty() {
                                        binding = trimmed.to_string();
                                    }
                                }
                                if let Some(value) = ui
                                    .get("text")
                                    .or_else(|| ui.get("default"))
                                    .and_then(toml::Value::as_str)
                                {
                                    text = value.to_string();
                                }
                                if let Some(value) = ui.get("font").and_then(toml::Value::as_str) {
                                    font = value.to_string();
                                }
                                if let Some(value) =
                                    ui.get("font_size").and_then(toml::Value::as_float)
                                {
                                    font_size = value as f32;
                                }
                                if let Some(value) = ui.get("color").and_then(toml::Value::as_str) {
                                    color = hex_to_rgba_u8(value);
                                }
                                if let Some(value) =
                                    ui.get("background_color").and_then(toml::Value::as_str)
                                {
                                    background_color = hex_to_rgba_u8(value);
                                }
                                if let Some(value) =
                                    ui.get("border_color").and_then(toml::Value::as_str)
                                {
                                    border_color = hex_to_rgba_u8(value);
                                }
                                if let Some(value) =
                                    ui.get("border_size").and_then(toml::Value::as_integer)
                                {
                                    border_size = value as i32;
                                }
                            }

                            if !binding.trim().is_empty() {
                                text = self.ui_state.entry(binding.clone()).or_insert(text).clone();
                            }

                            self.text_input_widgets.insert(
                                widget.id,
                                TextInputWidget {
                                    name: widget.name.clone(),
                                    id: widget.id,
                                    rect: Rect::new(x, y, width, height),
                                    binding,
                                    text,
                                    font,
                                    font_size,
                                    color,
                                    background_color,
                                    border_color,
                                    border_size,
                                },
                            );
                        } else if role == "messages" {
                            let mut widget = MessagesWidget {
                                id: widget.id,
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::default(),
                                ..Default::default()
                            };
                            widget.configure(assets);
                            self.messages_widgets.push(widget);
                        } else if role == "avatar" {
                            let mut avatar_widget = AvatarWidget {
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::default(),
                                ..Default::default()
                            };
                            avatar_widget.init();
                            self.avatar_widgets.insert(widget.creator_id, avatar_widget);
                        } else if role == "profile" {
                            let mut profile_widget = ProfileWidget::new();
                            profile_widget.animation_frame = self.animation_frame;
                            profile_widget.name = widget.name.clone();
                            profile_widget.rect = Rect::new(x, y, width, height);
                            profile_widget.toml_str = data.clone();
                            profile_widget.buffer = TheRGBABuffer::default();
                            profile_widget.init();
                            self.profile_widgets
                                .insert(widget.creator_id, profile_widget);
                        } else if role == "stat" {
                            let mut stat_widget = StatWidget::new();
                            stat_widget.name = widget.name.clone();
                            stat_widget.rect = Rect::new(x, y, width, height);
                            stat_widget.toml_str = data.clone();
                            stat_widget.buffer = TheRGBABuffer::default();
                            stat_widget.init();
                            self.stat_widgets.insert(widget.creator_id, stat_widget);
                        } else if role == "text" {
                            let mut text_widget = TextWidget {
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::default(),
                                ..Default::default()
                            };
                            text_widget.configure(assets);
                            self.text_widgets.insert(widget.creator_id, text_widget);
                        } else if role == "deco" {
                            let mut deco_widget = DecoWidget {
                                color: if artwork
                                    .sources
                                    .iter()
                                    .any(|s| !matches!(s, PixelSource::Off))
                                {
                                    [0; 4]
                                } else {
                                    crate::BLACK
                                },
                                artwork: artwork.clone(),
                                animation_frame: self.animation_frame,
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::default(),
                                ..Default::default()
                            };
                            deco_widget.init(assets);
                            self.deco_widgets.insert(widget.creator_id, deco_widget);
                        }
                    }
                }
            }
        }
        self.apply_bound_button_activations();
    }

    fn insert_action_bar(
        &mut self,
        screen: &Map,
        sector_id: u32,
        creator_id: Uuid,
        name: &str,
        rect: Rect,
        table: &toml::Table,
        assigned_artwork: &widget::artwork::ScreenArtwork,
        assets: &Assets,
    ) {
        let ui = table.get("ui").and_then(toml::Value::as_table);
        let frame = table.get("frame").and_then(toml::Value::as_table);
        let slot = table.get("slot").and_then(toml::Value::as_table);
        let slot_frame = slot
            .and_then(|slot| slot.get("frame"))
            .and_then(toml::Value::as_table);

        if let Some(spellbook) = ui
            .and_then(|ui| ui.get("spellbook"))
            .and_then(toml::Value::as_table)
        {
            self.toolbar_spellbook_config = catalog_panel_config(spellbook, None, None, assets);
        }
        if let Some(inventory) = ui
            .and_then(|ui| ui.get("inventory"))
            .and_then(toml::Value::as_table)
        {
            self.toolbar_inventory_panel_config = inventory_panel_config(inventory, None);
        }
        if let Some(equipment) = ui
            .and_then(|ui| ui.get("equipment"))
            .and_then(toml::Value::as_table)
        {
            self.toolbar_equipment_panel_config = equipment_panel_config(equipment, None);
        }
        if let Some(preferences) = ui
            .and_then(|ui| ui.get("preferences"))
            .and_then(toml::Value::as_table)
        {
            self.toolbar_preferences_panel_config = preferences_panel_config(preferences);
        }
        if let Some(actions) = ui
            .and_then(|ui| ui.get("actions"))
            .and_then(toml::Value::as_table)
        {
            let mut config = catalog_panel_config(actions, None, None, assets);
            if !actions.contains_key("show_assign") {
                config.show_assign = true;
            }
            if !actions.contains_key("show_details") {
                config.show_details = false;
            }
            if !actions.contains_key("show_names") {
                config.show_names = true;
            }
            if config.title.is_empty() {
                config.title = "Actions".to_string();
            }
            self.toolbar_actions_panel_config = config;
        }

        let frame_textures = Vec::new();
        let mut frame_artwork = frame.map(|f| action_bar_artwork(f, 0)).unwrap_or_default();
        if frame_artwork.sources.is_empty() {
            frame_artwork = assigned_artwork.clone();
        }
        let frame_color = frame
            .and_then(|frame| {
                color_from_table(frame, "background_color")
                    .or_else(|| color_from_table(frame, "color"))
            })
            .unwrap_or([21, 18, 13, 204]);
        let (
            frame_border_color,
            frame_border_size,
            frame_border_gradient_color,
            frame_border_gradient_direction,
            frame_border_radius,
        ) = frame
            .map(|frame| border_style_from_table(frame, [143, 117, 70, 255], 2))
            .unwrap_or((
                [143, 117, 70, 255],
                2,
                None,
                BorderGradientDirection::Vertical,
                0.0,
            ));
        let frame_slice = frame
            .and_then(|frame| frame.get("slice"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(0)
            .max(0) as usize;
        let separator = table.get("separator").and_then(toml::Value::as_table);
        let separator_size = separator
            .and_then(|separator| separator.get("size"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(0)
            .max(0) as i32;
        let separator_color = separator
            .and_then(|separator| {
                color_from_table(separator, "from").or_else(|| color_from_table(separator, "color"))
            })
            .unwrap_or([143, 117, 70, 255]);
        let separator_gradient_color =
            separator.and_then(|separator| color_from_table(separator, "to"));
        let separator_margin = separator
            .and_then(|separator| layout_number(separator, "margin"))
            .unwrap_or(8.0)
            .max(0.0);
        let separators = if separator_size > 0 {
            action_bar_group_separators(rect, table)
        } else {
            vec![]
        };
        let top_separator = table.get("top_separator").and_then(toml::Value::as_table);
        let top_separator_size = top_separator
            .and_then(|separator| separator.get("size"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(0)
            .max(0) as i32;
        let top_separator_color = top_separator
            .and_then(|separator| {
                color_from_table(separator, "from").or_else(|| color_from_table(separator, "color"))
            })
            .unwrap_or([143, 117, 70, 255]);
        let top_separator_gradient_color =
            top_separator.and_then(|separator| color_from_table(separator, "to"));
        let top_separator_inset = top_separator
            .and_then(|separator| layout_number(separator, "inset"))
            .unwrap_or(0.0)
            .max(0.0);
        let top_separator_offset = top_separator
            .and_then(|separator| layout_number(separator, "offset"))
            .unwrap_or(0.0);
        let layer = ui
            .and_then(|ui| ui.get("layer"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(0) as i32;
        self.deco_widgets.insert(
            creator_id,
            DecoWidget {
                rect,
                buffer: TheRGBABuffer::default(),
                layer,
                color: frame_color,
                border_color: frame_border_color,
                border_size: frame_border_size,
                border_gradient_color: frame_border_gradient_color,
                border_gradient_direction: frame_border_gradient_direction,
                border_radius: frame_border_radius,
                artwork: {
                    let mut a = frame_artwork;
                    a.slice = frame_slice;
                    a
                },
                animation_frame: self.animation_frame,
                textures: frame_textures,
                texture_slice: frame_slice,
                separators,
                separator_color,
                separator_gradient_color,
                separator_size,
                separator_margin,
                top_separator_color,
                top_separator_gradient_color,
                top_separator_size,
                top_separator_inset,
                top_separator_offset,
                ..Default::default()
            },
        );

        let buttons = action_bar_buttons(table);
        let rects = action_bar_slot_rects(rect, table);
        let slot_textures = action_bar_slot_textures(table, assets);
        let slot_slice = slot
            .and_then(|slot| slot.get("slice"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(0)
            .max(0) as usize;
        let slot_frame_textures = slot_frame
            .map(|frame| action_bar_state_textures(frame, assets))
            .unwrap_or_default();
        let slot_frame_slice = slot_frame
            .and_then(|frame| frame.get("slice"))
            .and_then(toml::Value::as_integer)
            .or_else(|| {
                slot.and_then(|slot| slot.get("frame_slice"))
                    .and_then(toml::Value::as_integer)
            })
            .unwrap_or(0)
            .max(0) as usize;
        let icon_inset = slot
            .and_then(|slot| layout_number(slot, "icon_inset"))
            .map(|value| value.max(0.0));
        let background_color = slot
            .and_then(|slot| {
                color_from_table(slot, "background_color")
                    .or_else(|| color_from_table(slot, "color"))
            })
            .or(Some([33, 28, 21, 238]));
        let (
            border_color,
            border_size,
            border_gradient_color,
            border_gradient_direction,
            border_radius,
        ) = slot
            .map(|slot| border_style_from_table(slot, [119, 100, 69, 255], 1))
            .unwrap_or((
                [119, 100, 69, 255],
                1,
                None,
                BorderGradientDirection::Vertical,
                0.0,
            ));
        let label_color = slot
            .and_then(|slot| {
                color_from_table(slot, "text_color").or_else(|| color_from_table(slot, "color"))
            })
            .unwrap_or(crate::WHITE);
        let label_font = slot
            .and_then(|slot| slot.get("font"))
            .and_then(toml::Value::as_str)
            .unwrap_or("")
            .to_string();
        let label_font_size = slot
            .and_then(|slot| layout_number(slot, "font_size"))
            .unwrap_or(18.0);
        let party = ui
            .and_then(|ui| ui.get("party"))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let default_show_icon = ui
            .and_then(|ui| ui.get("show_icon"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);
        let hover_style = slot
            .map(|slot| {
                button_state_style_from_ui(
                    slot,
                    "hover",
                    "hover_background_color",
                    "hover_border_color",
                    "hover_color",
                )
            })
            .unwrap_or_default();
        let selected_style = slot
            .map(|slot| {
                button_state_style_from_ui(
                    slot,
                    "selected",
                    "selected_background_color",
                    "selected_border_color",
                    "selected_color",
                )
            })
            .unwrap_or_default();
        let pressed_style = slot
            .map(|slot| {
                button_state_style_from_ui(
                    slot,
                    "pressed",
                    "pressed_background_color",
                    "pressed_border_color",
                    "pressed_color",
                )
            })
            .unwrap_or_default();
        let disabled_style = slot
            .map(|slot| {
                button_state_style_from_ui(
                    slot,
                    "disabled",
                    "disabled_background_color",
                    "disabled_border_color",
                    "disabled_color",
                )
            })
            .unwrap_or_default();

        let mut ids = Vec::with_capacity(buttons.len());
        for (index, (config, button_rect)) in buttons.into_iter().zip(rects).enumerate() {
            let id = self.next_action_bar_button_id(screen);
            ids.push(id);
            self.button_widgets.insert(
                id,
                Widget {
                    name: format!("{} Slot {}", name, index + 1),
                    id,
                    rect: button_rect,
                    command: config.command,
                    command_slot: config.command_slot,
                    party: party.clone(),
                    chrome_artwork: slot
                        .map(|s| action_bar_artwork(s, slot_slice))
                        .unwrap_or_default(),
                    frame_artwork: slot_frame
                        .map(|s| action_bar_artwork(s, slot_frame_slice))
                        .unwrap_or_default(),
                    chrome_textures: slot_textures.clone(),
                    chrome_slice: slot_slice,
                    frame_textures: slot_frame_textures.clone(),
                    frame_slice: slot_frame_slice,
                    icon_inset,
                    border_color,
                    border_size,
                    border_gradient_color,
                    border_gradient_direction,
                    border_radius,
                    show_icon: config.show_icon.unwrap_or(default_show_icon),
                    label: config.label,
                    label_font: label_font.clone(),
                    label_font_size,
                    label_color,
                    background_color,
                    hover_style,
                    selected_style,
                    pressed_style,
                    disabled_style,
                    ..Widget::new()
                },
            );
        }
        self.action_bar_button_ids.insert(sector_id, ids);
    }

    fn next_action_bar_button_id(&self, screen: &Map) -> u32 {
        let mut id = u32::MAX;
        while self.button_widgets.contains_key(&id)
            || self.text_input_widgets.contains_key(&id)
            || screen.sectors.iter().any(|sector| sector.id == id)
        {
            id = id.wrapping_sub(1);
        }
        id
    }

    fn apply_bound_button_activations(&mut self) {
        for binding in self.ui_state.keys() {
            sync_bound_button_activation(
                &self.button_widgets,
                &self.ui_state,
                &mut self.activated_widgets,
                &mut self.permanently_activated_widgets,
                binding,
            );
        }
    }
}
