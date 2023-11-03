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
            list_stack_layout_id: TheId::new("".to_string()),
            list_toolbar_stack_layout_id: TheId::new("".to_string()),
            content_stack_layout_id: TheId::new("".to_string()),
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) {
        let width = 420;

        let mut sectionbar_canvas = TheCanvas::new();

        let mut section_bar_canvas = TheCanvas::new();
        section_bar_canvas.set_widget(TheSectionbar::new("Sectionbar".to_string()));
        sectionbar_canvas.set_top(section_bar_canvas);

        let mut region_sectionbar_button = TheSectionbarButton::new("Regions Section".to_string());
        region_sectionbar_button.set_text("Regions".to_string());
        region_sectionbar_button.set_state(TheWidgetState::Selected);

        let mut character_sectionbar_button =
            TheSectionbarButton::new("Character Section".to_string());
        character_sectionbar_button.set_text("Character".to_string());

        // let mut item_sectionbar_button = TheSectionbarButton::new("Items Section".to_string());
        // item_sectionbar_button.set_text("Items".to_string());

        let mut tile_sectionbar_button = TheSectionbarButton::new("Tiles Section".to_string());
        tile_sectionbar_button.set_text("Tiles".to_string());

        let mut vlayout = TheVLayout::new("Section Buttons".to_string());
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
        let mut switchbar = TheSwitchbar::new("Switchbar Section Header".to_string());
        switchbar.set_text("Regions".to_string());
        header.set_widget(switchbar);

        let mut list_canvas = TheCanvas::new();
        let mut list_stack_layout = TheStackLayout::new("List Stack Layout".to_string());
        let mut list_toolbar_stack_layout = TheStackLayout::new("List Stack Layout".to_string());
        let mut content_stack_layout = TheStackLayout::new("Content Stack Layout".to_string());
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
        let toolbar_widget = TheToolbar::new("Toolbar".to_string());
        toolbar_canvas.set_widget(toolbar_widget);

        list_canvas.set_top(header);

        // Regions

        let mut regions_list_layout = TheListLayout::new("Regions List".to_string());
        // for i in 0..1 {
        //     let mut list_item: TheListItem = TheListItem::new(format!("Region Item {}", i));
        //     list_item.set_text(format!("Region #{}", i));
        //     regions_list_layout.add_item(list_item);
        // }
        regions_list_layout
            .limiter_mut()
            .set_max_size(vec2i(width, 200));
        list_stack_layout.add_layout(Box::new(regions_list_layout));

        let mut regions_add_button = TheToolbarButton::new("Regions Add".to_string());
        regions_add_button.set_icon_name("icon_role_add".to_string());
        let mut regions_remove_button = TheToolbarButton::new("Regions Remove".to_string());
        regions_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new("Toolbar Layout".to_string());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 0));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));

        list_toolbar_stack_layout.add_layout(Box::new(toolbar_hlayout));

        let mut regions_text_layout = TheTextLayout::new("Text Layout".to_string());
        regions_text_layout.limiter_mut().set_max_width(width);
        let regions_name_edit = TheTextLineEdit::new("Regions Name Edit".to_string());
        regions_text_layout.add_pair("Name".to_string(), Box::new(regions_name_edit));
        content_stack_layout.add_layout(Box::new(regions_text_layout));

        // Characters

        let mut character_list_layout = TheListLayout::new("Character List".to_string());
        // for i in 0..1 {
        //     let mut list_item: TheListItem = TheListItem::new(format!("Character Item {}", i));
        //     list_item.set_text(format!("Character #{}", i));
        //     character_list_layout.add_item(list_item);
        // }
        character_list_layout
            .limiter_mut()
            .set_max_size(vec2i(360, 200));
        list_stack_layout.add_layout(Box::new(character_list_layout));

        let mut character_add_button = TheToolbarButton::new("Character Add".to_string());
        character_add_button.set_icon_name("icon_role_add".to_string());
        let mut character_remove_button = TheToolbarButton::new("Character Remove".to_string());
        character_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new("Toolbar Layout".to_string());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 0));
        toolbar_hlayout.add_widget(Box::new(character_add_button));
        toolbar_hlayout.add_widget(Box::new(character_remove_button));

        list_toolbar_stack_layout.add_layout(Box::new(toolbar_hlayout));

        let mut character_text_layout = TheTextLayout::new("Text Layout".to_string());
        character_text_layout.limiter_mut().set_max_width(width);
        let character_name_edit = TheTextLineEdit::new("Regions Name Edit".to_string());
        character_text_layout.add_pair("Name".to_string(), Box::new(character_name_edit));
        content_stack_layout.add_layout(Box::new(character_text_layout));

        // Tiles

        let mut tiles_list_layout = TheListLayout::new("Tiles List".to_string());
        // for i in 0..1 {
        //     let mut list_item: TheListItem = TheListItem::new(format!("Region Item {}", i));
        //     list_item.set_text(format!("Region #{}", i));
        //     regions_list_layout.add_item(list_item);
        // }
        tiles_list_layout
            .limiter_mut()
            .set_max_size(vec2i(width, 200));
        list_stack_layout.add_layout(Box::new(tiles_list_layout));

        let mut tiles_add_button = TheToolbarButton::new("Tiles Add".to_string());
        tiles_add_button.set_icon_name("icon_role_add".to_string());
        let mut tiles_remove_button = TheToolbarButton::new("Tiles Remove".to_string());
        tiles_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut toolbar_hlayout = TheHLayout::new("Toolbar Layout".to_string());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 0));
        toolbar_hlayout.add_widget(Box::new(tiles_add_button));
        toolbar_hlayout.add_widget(Box::new(tiles_remove_button));

        list_toolbar_stack_layout.add_layout(Box::new(toolbar_hlayout));

        let mut tiles_text_layout = TheTextLayout::new("Text Layout".to_string());
        tiles_text_layout.limiter_mut().set_max_width(width);
        let tiles_name_edit = TheTextLineEdit::new("Regions Name Edit".to_string());
        tiles_text_layout.add_pair("Name".to_string(), Box::new(tiles_name_edit));
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

    pub fn handle_event(&mut self, event: &TheEvent, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
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
                    //println!("{} {:?}", name.clone(), buffer.dim().width);
                    if let Some(layout) = ui.canvas.get_layout(Some(&"Tiles List".to_string()), None) {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item = TheListItem::new("Tiles Item".to_string());
                            item.set_text(name.clone());
                            list_layout.add_item(item);
                        }
                    }

                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Tiles Add" {
                    ctx.ui.open_file_requester(
                        TheId::new(id.name.clone()),
                        "Open".into(),
                        vec![],
                    );
                    ctx.ui
                        .set_widget_state("Tiles Add".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
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
                }
            }
            _ => {}
        }

        false
    }

    pub fn deselect_sections_buttons(&mut self, ui: &mut TheUI, except: String) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Section Buttons".into()), None) {
            for w in layout.widgets() {
                if !w.id().name.starts_with(&except) {
                    w.set_state(TheWidgetState::None);
                }
            }
        }
    }
}
