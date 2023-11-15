use crate::prelude::*;

pub struct Sidebar {
    list_stack_layout_id: TheId,
    list_toolbar_stack_layout_id: TheId,
    content_stack_layout_id: TheId,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    pub fn new() -> Self {
        Self {
            list_stack_layout_id: TheId::empty(),
            list_toolbar_stack_layout_id: TheId::empty(),
            content_stack_layout_id: TheId::empty(),
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext, project: &mut Project) {
        let width = 420;

        let mut sectionbar_canvas = TheCanvas::new();

        let mut section_bar_canvas = TheCanvas::new();
        section_bar_canvas.set_widget(TheSectionbar::new(TheId::named("Sectionbar")));
        sectionbar_canvas.set_top(section_bar_canvas);

        let mut region_sectionbar_button = TheSectionbarButton::new(TheId::named("Regions Section"));
        region_sectionbar_button.set_text("Regions".to_string());
        region_sectionbar_button.set_state(TheWidgetState::Selected);

        let mut character_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Character Section"));
        character_sectionbar_button.set_text("Character".to_string());

        // let mut item_sectionbar_button = TheSectionbarButton::new("Items Section".to_string());
        // item_sectionbar_button.set_text("Items".to_string());

        let mut tile_sectionbar_button = TheSectionbarButton::new(TheId::named("Tiles Section"));
        tile_sectionbar_button.set_text("Tiles".to_string());

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

        let mut header = TheCanvas::new();
        let mut switchbar = TheSwitchbar::new(TheId::named("Switchbar Section Header"));
        switchbar.set_text("Regions".to_string());
        header.set_widget(switchbar);

        let mut list_canvas = TheCanvas::new();
        let mut list_stack_layout = TheStackLayout::new(TheId::named("List Stack Layout"));
        let mut list_toolbar_stack_layout = TheStackLayout::new(TheId::named("List Stack Layout"));
        let mut content_stack_layout = TheStackLayout::new(TheId::named("Content Stack Layout"));
        list_stack_layout
            .limiter_mut()
            .set_max_size(vec2i(width, 200));
        list_toolbar_stack_layout
            .limiter_mut()
            .set_max_size(vec2i(width, 200));
        self.list_stack_layout_id = list_stack_layout.id().clone();
        self.list_toolbar_stack_layout_id = list_toolbar_stack_layout.id().clone();
        self.content_stack_layout_id = content_stack_layout.id().clone();
        let mut toolbar_canvas = TheCanvas::new();
        let toolbar_widget = TheTraybar::new(TheId::named("Toolbar"));
        toolbar_canvas.set_widget(toolbar_widget);

        list_canvas.set_top(header);

        // Regions

        let mut regions_list_layout = TheListLayout::new(TheId::named("Regions List"));
        // for i in 0..1 {
        //     let mut list_item: TheListItem = TheListItem::new(format!("Region Item {}", i));
        //     list_item.set_text(format!("Region #{}", i));
        //     regions_list_layout.add_item(list_item);
        // }
        regions_list_layout
            .limiter_mut()
            .set_max_size(vec2i(width, 200));
        list_stack_layout.add_layout(Box::new(regions_list_layout));

        let mut regions_add_button = TheToolbarButton::new(TheId::named("Regions Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        let mut regions_remove_button = TheToolbarButton::new(TheId::named("Regions Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Toolbar Layout"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 0));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));

        list_toolbar_stack_layout.add_layout(Box::new(toolbar_hlayout));

        let mut regions_text_layout = TheTextLayout::new(TheId::named("Text Layout"));
        regions_text_layout.limiter_mut().set_max_width(width);
        let regions_name_edit = TheTextLineEdit::new(TheId::named("Regions Name Edit"));
        regions_text_layout.add_pair("Name".to_string(), Box::new(regions_name_edit));
        content_stack_layout.add_layout(Box::new(regions_text_layout));

        // Characters

        let mut character_list_layout = TheListLayout::new(TheId::named("Character List"));
        // for i in 0..1 {
        //     let mut list_item: TheListItem = TheListItem::new(format!("Character Item {}", i));
        //     list_item.set_text(format!("Character #{}", i));
        //     character_list_layout.add_item(list_item);
        // }
        character_list_layout
            .limiter_mut()
            .set_max_size(vec2i(360, 200));
        list_stack_layout.add_layout(Box::new(character_list_layout));

        let mut character_add_button = TheToolbarButton::new(TheId::named("Character Add"));
        character_add_button.set_icon_name("icon_role_add".to_string());
        let mut character_remove_button = TheToolbarButton::new(TheId::named("Character Remove"));
        character_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Toolbar Layout"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 0));
        toolbar_hlayout.add_widget(Box::new(character_add_button));
        toolbar_hlayout.add_widget(Box::new(character_remove_button));

        list_toolbar_stack_layout.add_layout(Box::new(toolbar_hlayout));

        let mut character_text_layout = TheTextLayout::new(TheId::named("Text Layout"));
        character_text_layout.limiter_mut().set_max_width(width);
        let character_name_edit = TheTextLineEdit::new(TheId::named("Regions Name Edit"));
        character_text_layout.add_pair("Name".to_string(), Box::new(character_name_edit));
        content_stack_layout.add_layout(Box::new(character_text_layout));

        // Tiles

        let mut tiles_list_layout = TheListLayout::new(TheId::named("Tiles List"));
        // for i in 0..1 {
        //     let mut list_item: TheListItem = TheListItem::new(format!("Region Item {}", i));
        //     list_item.set_text(format!("Region #{}", i));
        //     regions_list_layout.add_item(list_item);
        // }
        tiles_list_layout
            .limiter_mut()
            .set_max_size(vec2i(width, 200));
        list_stack_layout.add_layout(Box::new(tiles_list_layout));

        let mut tiles_add_button = TheTraybarButton::new(TheId::named("Tiles Add"));
        tiles_add_button.set_icon_name("icon_role_add".to_string());
        let mut tiles_remove_button = TheTraybarButton::new(TheId::named("Tiles Remove"));
        tiles_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Toolbar Layout"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 4, 5, 0));
        toolbar_hlayout.add_widget(Box::new(tiles_add_button));
        toolbar_hlayout.add_widget(Box::new(tiles_remove_button));

        list_toolbar_stack_layout.add_layout(Box::new(toolbar_hlayout));

        let mut tiles_text_layout = TheTextLayout::new(TheId::named("Text Layout"));
        tiles_text_layout.limiter_mut().set_max_width(width);
        let tiles_name_edit = TheTextLineEdit::new(TheId::named("Tiles Name Edit"));
        tiles_text_layout.add_pair("Name".to_string(), Box::new(tiles_name_edit));
        let tiles_grid_edit = TheTextLineEdit::new(TheId::named("Tiles Grid Edit"));
        tiles_text_layout.add_pair("Grid Size".to_string(), Box::new(tiles_grid_edit));
        content_stack_layout.add_layout(Box::new(tiles_text_layout));

        // ---

        list_canvas.set_layout(list_stack_layout);
        toolbar_canvas.set_layout(list_toolbar_stack_layout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut canvas = TheCanvas::new();

        canvas.set_top(list_canvas);
        canvas.set_right(sectionbar_canvas);
        canvas.top_is_expanding = false;
        canvas.set_layout(content_stack_layout);

        ui.canvas.set_right(canvas);
    }

    pub fn handle_event(&mut self, event: &TheEvent, ui: &mut TheUI, ctx: &mut TheContext, project: &mut Project) -> bool {
        let mut redraw = false;
        match event {

            // Add

            TheEvent::FileRequesterResult(id, paths) => {
                if id.name == "Tiles Add" {
                    for p in paths {
                        ctx.ui.decode_image(id.clone(), p.clone());
                    }
                }
            }
            TheEvent::ImageDecodeResult(id, name, buffer) => {
                if id.name == "Tiles Add" {
                    if let Some(layout) = ui.canvas.get_layout(Some(&"Tiles List".to_string()), None) {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item = TheListItem::new(TheId::named_with_id("Tiles Item", id.uuid));
                            item.set_text(name.clone());
                            item.set_state(TheWidgetState::Selected);
                            list_layout.deselect_all();
                            let id = item.id().clone();
                            list_layout.add_item(item, ctx);
                            ctx.ui.send_widget_state_changed(&id, TheWidgetState::Selected);

                            if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Main RGBALayout".into()), None) {
                                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                                    rgba_layout.set_buffer(buffer.clone());
                                }
                            }

                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {

                // Tiles Item Handling

                if id.name == "Tiles Add" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new("PNG Image".into(), vec!["PNG".to_string()]),
                    );
                    ctx.ui
                        .set_widget_state("Tiles Add".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                }

                if id.name == "Tiles Item" {
                    // Display the tilemap editor
                    for t in &project.tilemaps {
                        if t.id == id.uuid {

                            let mut center = TheCanvas::new();

                            let mut rgba_layout = TheRGBALayout::new(TheId::named("Main RGBALayout"));
                            rgba_layout.set_buffer(t.buffer.clone());
                            rgba_layout.set_scroll_offset(t.scroll_offset);
                            if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                                rgba_view.set_grid(Some(t.grid_size));
                            }

                            rgba_layout.relayout(ctx);
                            center.set_layout(rgba_layout);

                            //

                            let mut toolbar_canvas = TheCanvas::new();
                            let traybar_widget = TheTraybar::new(TheId::empty());
                            toolbar_canvas.set_widget(traybar_widget);

                            let mut regions_add_button = TheTraybarButton::new(TheId::named("Regions Add"));
                            // regions_add_button.set_icon_name("icon_role_add".to_string());
                            regions_add_button.set_text("icon_role_add".to_string());
                            let mut regions_remove_button = TheTraybarButton::new(TheId::named("Regions Remove"));
                            regions_remove_button.set_icon_name("icon_role_remove".to_string());

                            let mut regions_name_edit = TheTextLineEdit::new(TheId::named("Regions Name Edit"));
                            regions_name_edit.limiter_mut().set_max_width(150);

                            let mut dropdown =
                                TheDropdownMenu::new(TheId::named(format!("DropDown {}", 1).as_str()));
                            dropdown.add_option("Option #1".to_string());
                            dropdown.add_option("Option #2".to_string());

                            let mut toolbar_hlayout = TheHLayout::new(TheId::named("Toolbar Layout"));
                            toolbar_hlayout.set_background_color(None);
                            toolbar_hlayout.set_margin(vec4i(5, 4, 5, 0));
                            toolbar_hlayout.add_widget(Box::new(regions_add_button));
                            toolbar_hlayout.add_widget(Box::new(regions_remove_button));
                            toolbar_hlayout.add_widget(Box::new(regions_name_edit));
                            toolbar_hlayout.add_widget(Box::new(dropdown));

                            toolbar_canvas.set_layout(toolbar_hlayout);
                            center.set_top(toolbar_canvas);
                            ctx.ui.relayout = true;

                            ui.canvas.set_center(center);

                            ctx.ui.relayout = true;
                            self.apply_tilemap_item(ui, ctx, Some(t));
                        }
                    }
                    redraw = true;
                }

                // Section Buttons

                if id.name == "Regions Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Regions".to_string()));
                    }

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.list_stack_layout_id.clone(),
                        0,
                    ));
                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.list_toolbar_stack_layout_id.clone(),
                        0,
                    ));
                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.content_stack_layout_id.clone(),
                        0,
                    ));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Character Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Character".to_string()));
                    }

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.list_stack_layout_id.clone(),
                        1,
                    ));
                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.list_toolbar_stack_layout_id.clone(),
                        1,
                    ));
                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.content_stack_layout_id.clone(),
                        1,
                    ));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Tiles Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Tiles".to_string()));
                    }

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.list_stack_layout_id.clone(),
                        2,
                    ));
                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.list_toolbar_stack_layout_id.clone(),
                        2,
                    ));
                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.content_stack_layout_id.clone(),
                        2,
                    ));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                }
            }
            _ => {}
        }

        redraw
    }

    pub fn load_from_project(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Tiles List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();
                for t in &project.tilemaps {
                    let mut item = TheListItem::new(TheId::named_with_id("Tiles Item", t.id));
                    item.set_text(t.name.clone());
                    //item.set_state(TheWidgetState::Selected);
                    // list_layout.deselect_all();
                    // let id = item.id().clone();
                    list_layout.add_item(item, ctx);
                    // ctx.ui.send_widget_state_changed(&id, TheWidgetState::Selected);
                }
            }
        }
    }

    /// Apply the given item to the UI
    pub fn apply_tilemap_item(&mut self, ui: &mut TheUI, ctx: &mut TheContext, tilemap: Option<&Tilemap>) {
        if let Some(widget) = ui.canvas.get_widget(Some(&"Tiles Name Edit".to_string()), None) {
            if let Some(tilemap) = tilemap {
                widget.set_value(TheValue::Text(tilemap.name.clone()));
            } else {
                widget.set_value(TheValue::Empty);
            }
        }
        if let Some(widget) = ui.canvas.get_widget(Some(&"Tiles Grid Edit".to_string()), None) {
            if let Some(tilemap) = tilemap {
                widget.set_value(TheValue::Text(tilemap.grid_size.clone().to_string()));
            } else {
                widget.set_value(TheValue::Empty);
            }
        }
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
