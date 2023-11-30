use crate::prelude::*;

pub struct Sidebar {
    stack_layout_id: TheId,

    curr_tilemap_uuid: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    pub fn new() -> Self {
        Self {
            stack_layout_id: TheId::empty(),
            curr_tilemap_uuid: None,
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext, _project: &mut Project) {
        let width = 420;

        let mut sectionbar_canvas = TheCanvas::new();

        let mut section_bar_canvas = TheCanvas::new();
        section_bar_canvas.set_widget(TheSectionbar::new(TheId::named("Sectionbar")));
        sectionbar_canvas.set_top(section_bar_canvas);

        let mut region_sectionbar_button = TheSectionbarButton::new(TheId::named("Region Section"));
        region_sectionbar_button.set_text("Region".to_string());
        region_sectionbar_button.set_state(TheWidgetState::Selected);

        let mut character_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Character Section"));
        character_sectionbar_button.set_text("Character".to_string());

        // let mut item_sectionbar_button = TheSectionbarButton::new("Items Section".to_string());
        // item_sectionbar_button.set_text("Items".to_string());

        let mut tile_sectionbar_button = TheSectionbarButton::new(TheId::named("Tilemap Section"));
        tile_sectionbar_button.set_text("Tilemap".to_string());

        let mut vlayout = TheVLayout::new(TheId::named("Section Buttons"));
        vlayout.add_widget(Box::new(region_sectionbar_button));
        vlayout.add_widget(Box::new(character_sectionbar_button));
        //vlayout.add_widget(Box::new(item_sectionbar_button));
        vlayout.add_widget(Box::new(tile_sectionbar_button));
        vlayout.set_margin(vec4i(5, 10, 5, 10));
        vlayout.set_padding(4);
        vlayout.set_background_color(Some(SectionbarBackground));
        vlayout.limiter_mut().set_max_width(90);
        sectionbar_canvas.set_layout(vlayout);

        //

        let mut header = TheCanvas::new();
        let mut switchbar = TheSwitchbar::new(TheId::named("Switchbar Section Header"));
        switchbar.set_text("Region".to_string());
        header.set_widget(switchbar);

        let mut stack_layout = TheStackLayout::new(TheId::named("List Stack Layout"));

        stack_layout.limiter_mut().set_max_width(width);

        self.stack_layout_id = stack_layout.id().clone();

        // Regions

        let mut regions_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Region List"));
        list_layout.limiter_mut().set_max_size(vec2i(width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut region_add_button = TheTraybarButton::new(TheId::named("Region Add"));
        region_add_button.set_icon_name("icon_role_add".to_string());
        let mut region_remove_button = TheTraybarButton::new(TheId::named("Region Remove"));
        region_remove_button.set_icon_name("icon_role_remove".to_string());
        region_remove_button.set_disabled(true);
        let mut region_settings_button = TheTraybarButton::new(TheId::named("Region Settings"));
        region_settings_button.set_text("Settings ...".to_string());
        region_settings_button.set_disabled(true);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(region_add_button));
        toolbar_hlayout.add_widget(Box::new(region_remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(region_settings_button));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut text_layout: TheTextLayout = TheTextLayout::new(TheId::empty());
        text_layout.limiter_mut().set_max_width(width);
        let name_edit = TheTextLineEdit::new(TheId::named("Region Name Edit"));
        text_layout.add_pair("Name".to_string(), Box::new(name_edit));
        let width_edit = TheTextLineEdit::new(TheId::named("Region Width Edit"));
        text_layout.add_pair("Width in Grid".to_string(), Box::new(width_edit));
        let height_edit = TheTextLineEdit::new(TheId::named("Region Height Edit"));
        text_layout.add_pair("Height in Grid".to_string(), Box::new(height_edit));
        let grid_edit = TheTextLineEdit::new(TheId::named("Region Grid Edit"));
        text_layout.add_pair("Grid Size".to_string(), Box::new(grid_edit));

        let mut yellow_canvas = TheCanvas::default();
        let mut yellow_color = TheColorButton::new(TheId::named("Yellow"));
        yellow_color.set_color([255, 255, 0, 255]);
        yellow_color.limiter_mut().set_max_size(vec2i(width, 200));
        yellow_canvas.set_widget(yellow_color);

        regions_canvas.set_top(list_canvas);
        regions_canvas.set_layout(text_layout);
        regions_canvas.set_bottom(yellow_canvas);
        stack_layout.add_canvas(regions_canvas);

        // Character

        let mut character_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Character List"));
        list_layout.limiter_mut().set_max_size(vec2i(width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut regions_add_button = TheTraybarButton::new(TheId::named("Character Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        let mut regions_remove_button = TheTraybarButton::new(TheId::named("Character Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut text_layout = TheTextLayout::new(TheId::empty());
        text_layout.limiter_mut().set_max_width(width);
        let name_edit = TheTextLineEdit::new(TheId::named("Character Name Edit"));
        text_layout.add_pair("Name".to_string(), Box::new(name_edit));

        let mut red_canvas = TheCanvas::default();
        let mut red_color = TheColorButton::new(TheId::named("Red"));
        red_color.set_color([255, 0, 0, 255]);
        red_color.limiter_mut().set_max_size(vec2i(width, 350));
        red_canvas.set_widget(red_color);

        character_canvas.set_top(list_canvas);
        character_canvas.set_layout(text_layout);
        character_canvas.set_bottom(red_canvas);
        stack_layout.add_canvas(character_canvas);

        // Tilemaps

        let mut tiles_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Tilemap List"));
        list_layout.limiter_mut().set_max_size(vec2i(width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut regions_add_button = TheTraybarButton::new(TheId::named("Tilemap Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        let mut regions_remove_button = TheTraybarButton::new(TheId::named("Tilemap Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut text_layout = TheTextLayout::new(TheId::empty());
        text_layout.limiter_mut().set_max_width(width);
        let name_edit = TheTextLineEdit::new(TheId::named("Tilemap Name Edit"));
        text_layout.add_pair("Name".to_string(), Box::new(name_edit));
        let grid_edit = TheTextLineEdit::new(TheId::named("Tilemap Grid Edit"));
        text_layout.add_pair("Grid Size".to_string(), Box::new(grid_edit));

        let mut tiles_list_canvas = TheCanvas::default();

        let mut tiles_list_header_canvas = TheCanvas::default();
        tiles_list_header_canvas.set_widget(TheStatusbar::new(TheId::empty()));
        let mut tiles_list_header_canvas_hlayout = TheHLayout::new(TheId::empty());
        tiles_list_header_canvas_hlayout.set_background_color(None);
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        tiles_list_header_canvas_hlayout.set_margin(vec4i(10, 1, 5, 1));
        tiles_list_header_canvas_hlayout.set_padding(3);
        tiles_list_header_canvas_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Tilemap Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(vec2i(75, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        tiles_list_header_canvas_hlayout.add_widget(Box::new(filter_edit));

        for dir in TileRole::iterator() {
            let mut color_button = TheColorButton::new(TheId::named("Tilemap Filter Character"));
            color_button.limiter_mut().set_max_size(vec2i(17, 17));
            color_button.set_color(dir.to_color().to_u8_array());
            color_button.set_state(TheWidgetState::Selected);
            tiles_list_header_canvas_hlayout.add_widget(Box::new(color_button));
        }

        tiles_list_header_canvas.set_layout(tiles_list_header_canvas_hlayout);

        let mut tile_list_layout = TheListLayout::new(TheId::named("Tilemap Tile List"));
        tile_list_layout.set_item_size(42);
        tile_list_layout
            .limiter_mut()
            .set_max_size(vec2i(width, 360));

        tiles_list_canvas.set_top(tiles_list_header_canvas);
        tiles_list_canvas.set_layout(tile_list_layout);

        tiles_canvas.set_top(list_canvas);
        tiles_canvas.set_layout(text_layout);
        tiles_canvas.set_bottom(tiles_list_canvas);
        stack_layout.add_canvas(tiles_canvas);

        //

        let mut canvas = TheCanvas::new();

        canvas.set_top(header);
        canvas.set_right(sectionbar_canvas);
        canvas.top_is_expanding = false;
        canvas.set_layout(stack_layout);

        ui.canvas.set_right(canvas);

        self.apply_region(ui, ctx, None);
        self.apply_tilemap(ui, ctx, None);
    }

    #[allow(clippy::suspicious_else_formatting)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::ShowContextMenu(id, _coord) => {
                println!("ShowContextMenu {}", id.name);
            }
            TheEvent::TileSelectionChanged(id) => {
                if id.name == "Tilemap Editor View" {
                    // Selection changed in the tilemap editor
                    if let Some(rgba_view) = ui
                        .canvas
                        .get_widget(Some(&"Tilemap Editor View".to_string()), None)
                    {
                        if let Some(rgba_view) = rgba_view.as_rgba_view() {

                            let tile = rgba_view.selection_as_tile();

                            if let Some(icon_view) = ui
                                .canvas
                                .get_widget(Some(&"Tilemap Editor Icon View".to_string()), None)
                            {
                                if let Some(icon_view) = icon_view.as_icon_view() {
                                    icon_view.set_rgba_tile(tile);
                                    redraw = true;
                                }
                            }
                        }
                    }
                }
            }
            // Tiles Add
            TheEvent::FileRequesterResult(id, paths) => {
                if id.name == "Tilemap Add" {
                    for p in paths {
                        ctx.ui.decode_image(id.clone(), p.clone());
                    }
                }
            }
            TheEvent::ImageDecodeResult(id, name, _buffer) => {
                if id.name == "Tilemap Add" {
                    if let Some(layout) = ui
                        .canvas
                        .get_layout(Some(&"Tilemap List".to_string()), None)
                    {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item =
                                TheListItem::new(TheId::named_with_id("Tilemap Item", id.uuid));
                            item.set_text(name.clone());
                            item.set_state(TheWidgetState::Selected);
                            list_layout.deselect_all();
                            let id = item.id().clone();
                            list_layout.add_item(item, ctx);
                            ctx.ui
                                .send_widget_state_changed(&id, TheWidgetState::Selected);

                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                // Regions Add

                if id.name == "Region Add" {
                    if let Some(list_layout) = ui.get_list_layout("Region List") {
                        let region = Region::new();

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Region Item", region.id));
                        item.set_text(region.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        project.regions.push(region);
                    }
                } else if id.name == "Region Item" {
                    for r in &project.regions {
                        if r.id == id.uuid {
                            self.apply_region(ui, ctx, Some(r));
                            redraw = true;
                        }
                    }
                } else if id.name == "Region Settings" {
                    self.show_region_settings(ctx);
                }  else
                // Tilemap Item Handling
                if id.name == "Tilemap Add" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new("PNG Image".into(), vec!["PNG".to_string()]),
                    );
                    ctx.ui
                        .set_widget_state("Tilemap Add".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else
                if id.name == "Tilemap Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_tilemap(selected);
                            self.apply_tilemap(ui, ctx, None);
                            self.curr_tilemap_uuid = None;
                        }
                    }
                } else if id.name == "Tilemap Item" {
                    // Display the tilemap editor
                    for t in &project.tilemaps {
                        if t.id == id.uuid {
                            self.curr_tilemap_uuid = Some(t.id);

                            let mut center = TheCanvas::new();

                            let mut rgba_layout =
                                TheRGBALayout::new(TheId::named("Tilemap Editor"));
                            rgba_layout.set_buffer(t.buffer.clone());
                            rgba_layout.set_scroll_offset(t.scroll_offset);
                            if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                                rgba_view.set_grid(Some(t.grid_size));
                                rgba_view.set_mode(TheRGBAViewMode::TileSelection);
                            }

                            rgba_layout.relayout(ctx);
                            center.set_layout(rgba_layout);

                            //

                            let mut toolbar_canvas = TheCanvas::new();
                            let traybar_widget = TheTraybar::new(TheId::empty());
                            toolbar_canvas.set_widget(traybar_widget);

                            let mut add_button =
                                TheTraybarButton::new(TheId::named("Tilemap Editor Add Selection"));
                            add_button.set_text("Add Tile".to_string());

                            let icon_view =
                                TheIconView::new(TheId::named("Tilemap Editor Icon View"));

                            let mut tile_name_text = TheText::new(TheId::empty());
                            tile_name_text.set_text("Tile Name".to_string());

                            let mut tile_name_edit =
                                TheTextLineEdit::new(TheId::named("Tilemap Editor Name Edit"));
                            tile_name_edit.limiter_mut().set_max_width(150);

                            let mut block_name_text = TheText::new(TheId::empty());
                            block_name_text.set_text("Blocking".to_string());

                            let block_check_button =
                                TheCheckButton::new(TheId::named("Tilemap Editor Block"));

                            let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
                            toolbar_hlayout.set_background_color(None);
                            toolbar_hlayout.set_margin(vec4i(5, 4, 5, 4));

                            toolbar_hlayout.add_widget(Box::new(icon_view));
                            let mut hdivider = TheHDivider::new(TheId::empty());
                            hdivider.limiter_mut().set_max_width(15);
                            toolbar_hlayout.add_widget(Box::new(hdivider));

                            toolbar_hlayout.add_widget(Box::new(tile_name_text));
                            toolbar_hlayout.add_widget(Box::new(tile_name_edit));

                            let mut hdivider = TheHDivider::new(TheId::empty());
                            hdivider.limiter_mut().set_max_width(15);
                            toolbar_hlayout.add_widget(Box::new(hdivider));

                            let mut drop_down = TheDropdownMenu::new(TheId::named("Tilemap Editor Role"));

                            for dir in TileRole::iterator() {
                                drop_down.add_option(dir.to_string().to_string());
                                /*
                                let mut color_button = TheColorButton::new(TheId::named(
                                    "Tilemap Editor Filter Character",
                                ));
                                color_button.limiter_mut().set_max_size(vec2i(19, 19));
                                color_button.set_color(dir.to_color().to_u8_array());
                                if dir == TileRole::Character {
                                    color_button.set_state(TheWidgetState::Selected);
                                }
                                toolbar_hlayout.add_widget(Box::new(color_button));*/
                            }
                            toolbar_hlayout.add_widget(Box::new(drop_down));

                            let mut hdivider = TheHDivider::new(TheId::empty());
                            hdivider.limiter_mut().set_max_width(15);
                            toolbar_hlayout.add_widget(Box::new(hdivider));

                            toolbar_hlayout.add_widget(Box::new(block_name_text));
                            toolbar_hlayout.add_widget(Box::new(block_check_button));

                            let mut hdivider = TheHDivider::new(TheId::empty());
                            hdivider.limiter_mut().set_max_width(15);
                            toolbar_hlayout.add_widget(Box::new(hdivider));

                            toolbar_hlayout.add_widget(Box::new(add_button));

                            toolbar_canvas.set_layout(toolbar_hlayout);
                            center.set_top(toolbar_canvas);
                            ctx.ui.relayout = true;

                            if let Some(browser) =
                                ui.canvas.get_layout(Some(&"Browser".to_string()), None)
                            {
                                if let Some(browser) = browser.as_tab_layout() {
                                    browser.clear();
                                    browser.add_canvas(t.name.clone(), center);
                                }
                            }

                            ctx.ui.relayout = true;
                            self.apply_tilemap(ui, ctx, Some(t));
                        }
                    }
                    redraw = true;
                } else if id.name == "Tilemap Editor Add Selection" {
                    let mut clear_selection = false;

                    if let Some(editor) = ui
                        .canvas
                        .get_layout(Some(&"Tilemap Editor".to_string()), None)
                    {
                        if let Some(editor) = editor.as_rgba_layout() {
                            let sequence = editor
                                .rgba_view_mut()
                                .as_rgba_view()
                                .unwrap()
                                .selection_as_sequence();
                            let mut tile = Tile::new();
                            tile.sequence = sequence;

                            if let Some(text_line_edit) =
                                ui.get_text_line_edit("Tilemap Editor Name Edit")
                            {
                                tile.name = text_line_edit.text();
                            }

                            if let Some(block_widget) = ui
                                .canvas
                                .get_widget(Some(&"Tilemap Editor Block".to_string()), None)
                            {
                                tile.blocking = block_widget.state() == TheWidgetState::Selected;
                            }

                            if let Some(role_widget) = ui
                                .get_drop_down_menu("Tilemap Editor Role")
                            {
                                let index = role_widget.selected_index();
                                tile.role = TileRole::from_index(index as u8).unwrap();
                            }

                            // Only add if non-empty
                            if !tile.name.is_empty() && !tile.sequence.regions.is_empty() {
                                if let Some(layout) = ui
                                    .canvas
                                    .get_layout(Some(&"Tilemap Tile List".to_string()), None)
                                {
                                    let list_layout_id = layout.id().clone();
                                    if let Some(list_layout) = layout.as_list_layout() {
                                        let mut item = TheListItem::new(TheId::named_with_id(
                                            "Tilemap Tile",
                                            tile.id,
                                        ));
                                        item.set_text(tile.name.clone());
                                        let mut sub_text = if tile.blocking {
                                            "Blocking".to_string()
                                        } else {
                                            "Non-Blocking".to_string()
                                        };
                                        sub_text +=
                                            ("  ".to_string() + tile.role.to_string()).as_str();
                                        item.set_sub_text(sub_text);
                                        item.set_state(TheWidgetState::Selected);
                                        item.set_size(42);
                                        item.set_associated_layout(list_layout_id);
                                        if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                                            if let Some(t) = project.get_tilemap(curr_tilemap_uuid)
                                            {
                                                item.set_icon(
                                                    tile.sequence.regions[0].scale(&t.buffer, 36, 36),
                                                );
                                            }
                                        }
                                        list_layout.deselect_all();
                                        let id = item.id().clone();
                                        list_layout.add_item(item, ctx);
                                        ctx.ui.send_widget_state_changed(
                                            &id,
                                            TheWidgetState::Selected,
                                        );

                                        clear_selection = true;
                                        redraw = true;
                                    }
                                }
                            }

                            if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                                if let Some(tilemap) = project.get_tilemap(curr_tilemap_uuid) {
                                    tilemap.tiles.push(tile);
                                }
                            }
                        }
                    }

                    // Clear the selection if successful
                    if clear_selection {
                        if let Some(editor) = ui
                            .canvas
                            .get_layout(Some(&"Tilemap Editor".to_string()), None)
                        {
                            if let Some(editor) = editor.as_rgba_layout() {
                                editor
                                    .rgba_view_mut()
                                    .as_rgba_view()
                                    .unwrap()
                                    .set_selection(FxHashSet::default());
                            }
                        }
                    }
                } else
                // Section Buttons
                if id.name == "Region Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Regions".to_string()));
                    }

                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 0));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Character Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Character".to_string()));
                    }

                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 1));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Tilemap Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Tilemaps".to_string()));
                    }

                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 2));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                }
            }
            _ => {}
        }

        redraw
    }

    pub fn load_from_project(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        if let Some(list_layout) = ui.get_list_layout("Region List") {
            list_layout.clear();
            for region in &project.regions {
                let mut item = TheListItem::new(TheId::named_with_id("Region Item", region.id));
                item.set_text(region.name.clone());
                //item.set_state(TheWidgetState::Selected);
                // list_layout.deselect_all();
                // let id = item.id().clone();
                list_layout.add_item(item, ctx);
                // ctx.ui.send_widget_state_changed(&id, TheWidgetState::Selected);
            }
        }
        if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
            list_layout.clear();
            for tilemap in &project.tilemaps {
                let mut item = TheListItem::new(TheId::named_with_id("Tilemap Item", tilemap.id));
                item.set_text(tilemap.name.clone());
                //item.set_state(TheWidgetState::Selected);
                // list_layout.deselect_all();
                // let id = item.id().clone();
                list_layout.add_item(item, ctx);
                // ctx.ui.send_widget_state_changed(&id, TheWidgetState::Selected);
            }
        }
    }

    /// Apply the given item to the UI
    pub fn apply_region(&mut self, ui: &mut TheUI, ctx: &mut TheContext, region: Option<&Region>) {
        ui.set_widget_disabled_state("Region Remove", ctx, region.is_none());
        ui.set_widget_disabled_state("Region Settings", ctx, region.is_none());

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Name Edit".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.name.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Width Edit".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.width.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Height Edit".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.height.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Grid Edit".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.grid_size.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
    }

    /// Apply the given tilemap item to the UI
    pub fn apply_tilemap(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        tilemap: Option<&Tilemap>,
    ) {
        ui.set_widget_disabled_state("Tilemap Remove", ctx, tilemap.is_none());

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Tilemap Name Edit".to_string()), None)
        {
            if let Some(tilemap) = tilemap {
                widget.set_value(TheValue::Text(tilemap.name.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Tilemap Grid Edit".to_string()), None)
        {
            if let Some(tilemap) = tilemap {
                widget.set_value(TheValue::Text(tilemap.grid_size.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        //
        if let Some(layout) = ui
            .canvas
            .get_layout(Some(&"Tilemap Tile List".to_string()), None)
        {
            let list_layout_id = layout.id().clone();
            if let Some(list_layout) = layout.as_list_layout() {
                if let Some(tilemap) = tilemap {
                    list_layout.clear();
                    for tile in &tilemap.tiles {
                        let mut item =
                            TheListItem::new(TheId::named_with_id("Tilemap Tile", tile.id));
                        item.set_text(tile.name.clone());
                        let mut sub_text = if tile.blocking {
                            "Blocking".to_string()
                        } else {
                            "Non-Blocking".to_string()
                        };
                        sub_text += ("  ".to_string() + tile.role.to_string()).as_str();
                        item.set_sub_text(sub_text);
                        item.set_size(42);
                        item.set_icon(tile.sequence.regions[0].scale(&tilemap.buffer, 36, 36));
                        item.set_state(TheWidgetState::Selected);
                        item.set_associated_layout(list_layout_id.clone());
                        list_layout.deselect_all();
                        list_layout.add_item(item, ctx);
                    }
                } else {
                    list_layout.clear();
                }
            }
        }
    }

    pub fn show_region_settings(&mut self, ctx: &mut TheContext) {

        let width = 400;
        let height = 400;

        let mut canvas = TheCanvas::new();
        canvas.limiter_mut().set_max_size(vec2i(width, height));

        let mut text_layout: TheTextLayout = TheTextLayout::new(TheId::empty());
        text_layout.limiter_mut().set_max_width(width);
        let name_edit = TheTextLineEdit::new(TheId::named("Region Name Edit"));
        text_layout.add_pair("Name".to_string(), Box::new(name_edit));
        let width_edit = TheTextLineEdit::new(TheId::named("Region Width Edit"));
        text_layout.add_pair("Width in Grid".to_string(), Box::new(width_edit));
        let height_edit = TheTextLineEdit::new(TheId::named("Region Height Edit"));
        text_layout.add_pair("Height in Grid".to_string(), Box::new(height_edit));
        let grid_edit = TheTextLineEdit::new(TheId::named("Region Grid Edit"));
        text_layout.add_pair("Grid Size".to_string(), Box::new(grid_edit));


        // let mut yellow_canvas = TheCanvas::default();
        // let mut yellow_color = TheColorButton::new(TheId::named("Yellow"));
        // yellow_color.set_color([255, 255, 0, 255]);
        // yellow_color.limiter_mut().set_max_size(vec2i(width, 200));
        // yellow_canvas.set_widget(yellow_color);

        // regions_canvas.set_top(list_canvas);
        // regions_canvas.set_layout(text_layout);
        // regions_canvas.set_bottom(yellow_canvas);
        // stack_layout.add_canvas(regions_canvas);

        canvas.set_layout(text_layout);
        ctx.ui.show_dialog("Region Settings", canvas);
    }

    /// Deselects the section buttons
    pub fn deselect_sections_buttons(&mut self, ui: &mut TheUI, except: String) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Section Buttons".into()), None) {
            for w in layout.widgets() {
                if !w.id().name.starts_with(&except) {
                    w.set_state(TheWidgetState::None);
                }
            }
        }
    }

    /// Returns the selected id in the given list layout
    pub fn get_selected_in_list_layout(&self, ui: &mut TheUI, layout_name: &str) -> Option<TheId> {
        if let Some(layout) = ui.canvas.get_layout(Some(&layout_name.to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                return list_layout.selected();
            }
        }
        None
    }
}
