use crate::editor::{CODEEDITOR, SIDEBARMODE, TILEMAPEDITOR};
use crate::prelude::*;

#[derive(PartialEq, Debug)]
pub enum SidebarMode {
    Region,
    Character,
    Item,
    Tilemap,
    Code,
}

pub struct Sidebar {
    pub width: i32,

    stack_layout_id: TheId,

    curr_tilemap_uuid: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    pub fn new() -> Self {
        Self {
            width: 380,

            stack_layout_id: TheId::empty(),
            curr_tilemap_uuid: None,
        }
    }

    pub fn init_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server: &mut Server,
    ) {
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

        let mut item_sectionbar_button = TheSectionbarButton::new(TheId::named("Item Section"));
        item_sectionbar_button.set_text("Item".to_string());

        let mut tilemap_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Tilemap Section"));
        tilemap_sectionbar_button.set_text("Tilemap".to_string());

        let mut code_sectionbar_button = TheSectionbarButton::new(TheId::named("Code Section"));
        code_sectionbar_button.set_text("Code".to_string());

        let mut vlayout = TheVLayout::new(TheId::named("Section Buttons"));
        vlayout.add_widget(Box::new(region_sectionbar_button));
        vlayout.add_widget(Box::new(character_sectionbar_button));
        vlayout.add_widget(Box::new(item_sectionbar_button));
        vlayout.add_widget(Box::new(tilemap_sectionbar_button));
        vlayout.add_widget(Box::new(code_sectionbar_button));
        vlayout.set_margin(vec4i(5, 10, 5, 5));
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

        stack_layout.limiter_mut().set_max_width(self.width);

        self.stack_layout_id = stack_layout.id().clone();

        // Regions

        let mut regions_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Region List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut region_add_button = TheTraybarButton::new(TheId::named("Region Add"));
        region_add_button.set_icon_name("icon_role_add".to_string());
        region_add_button.set_status_text("Add a new region.");

        let mut region_remove_button = TheTraybarButton::new(TheId::named("Region Remove"));
        region_remove_button.set_icon_name("icon_role_remove".to_string());
        region_remove_button.set_status_text("Remove the selected region.");
        region_remove_button.set_disabled(true);
        let mut name_edit = TheTextLineEdit::new(TheId::named("Region Name Edit"));
        name_edit.limiter_mut().set_max_width(200);
        name_edit.set_status_text("Edit the name of the region.");

        // let mut region_settings_button = TheTraybarButton::new(TheId::named("Region Settings"));
        // region_settings_button.set_text("Settings ...".to_string());
        // region_settings_button.set_disabled(true);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(region_add_button));
        toolbar_hlayout.add_widget(Box::new(region_remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(name_edit));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut region_canvas = TheCanvas::new();
        let mut region_tab = TheTabLayout::new(TheId::named("Region Tab Layout"));

        // Region Content

        let mut list_layout = TheListLayout::new(TheId::named("Region Content List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 250));
        let mut content_canvas = TheCanvas::default();
        content_canvas.set_layout(list_layout);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Region Content Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(vec2i(85, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Region Content Dropdown"));
        drop_down.add_option("All".to_string());
        drop_down.add_option("Character".to_string());
        drop_down.add_option("Areas".to_string());
        drop_down.add_option("Item".to_string());
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        content_canvas.set_top(toolbar_canvas);

        region_tab.add_canvas("Content".to_string(), content_canvas);

        // Region Settings

        let mut settings_canvas = TheCanvas::default();

        let mut text_layout: TheTextLayout = TheTextLayout::new(TheId::empty());
        text_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 250));
        let mut drop_down = TheDropdownMenu::new(TheId::named("Region Settings Dropdown"));
        drop_down.add_option("Top / Left".to_string());
        drop_down.add_option("Top / Right".to_string());
        drop_down.add_option("Bottom / Left".to_string());
        drop_down.add_option("Bottom / Right".to_string());
        drop_down.set_status_text(
            "On region size changes the region will grow or shrink from the given corner.",
        );
        text_layout.add_pair("Grow / Shrink From".to_string(), Box::new(drop_down));
        let mut width_edit = TheTextLineEdit::new(TheId::named("Region Width Edit"));
        width_edit.set_range(TheValue::RangeI32(1..=100000));
        width_edit.set_status_text("The width of the region in grid units.");
        text_layout.add_pair("Width (Grid)".to_string(), Box::new(width_edit));
        let mut height_edit = TheTextLineEdit::new(TheId::named("Region Height Edit"));
        height_edit.set_range(TheValue::RangeI32(1..=100000));
        height_edit.set_status_text("The height of the region in grid units.");
        text_layout.add_pair("Height (Grid)".to_string(), Box::new(height_edit));
        let mut grid_edit = TheTextLineEdit::new(TheId::named("Region Grid Edit"));
        grid_edit.set_range(TheValue::RangeI32(1..=1000));
        grid_edit.set_status_text("The size of the region grid in pixels.");
        text_layout.add_pair("Grid Size".to_string(), Box::new(grid_edit));

        settings_canvas.set_layout(text_layout);
        region_tab.add_canvas("Settings".to_string(), settings_canvas);

        region_canvas.set_layout(region_tab);
        regions_canvas.set_top(list_canvas);
        //regions_canvas.set_layout(text_layout);
        regions_canvas.set_bottom(region_canvas);
        stack_layout.add_canvas(regions_canvas);

        // Character

        let mut character_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Character List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 400));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut regions_add_button = TheTraybarButton::new(TheId::named("Character Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        let mut regions_remove_button = TheTraybarButton::new(TheId::named("Character Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());
        let mut name_edit = TheTextLineEdit::new(TheId::named("Character Name Edit"));
        name_edit.limiter_mut().set_max_width(200);
        name_edit.set_status_text("Edit the name of the character.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(name_edit));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        character_canvas.set_top(list_canvas);
        stack_layout.add_canvas(character_canvas);

        // Item

        let mut item_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Item List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 400));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut item_add_button = TheTraybarButton::new(TheId::named("Item Add"));
        item_add_button.set_icon_name("icon_role_add".to_string());
        let mut item_remove_button = TheTraybarButton::new(TheId::named("Item Remove"));
        item_remove_button.set_icon_name("icon_role_remove".to_string());
        let mut name_edit = TheTextLineEdit::new(TheId::named("Item Name Edit"));
        name_edit.limiter_mut().set_max_width(200);
        name_edit.set_status_text("Edit the name of the item.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(item_add_button));
        toolbar_hlayout.add_widget(Box::new(item_remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(name_edit));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        item_canvas.set_top(list_canvas);
        stack_layout.add_canvas(item_canvas);

        // Tilemaps

        let mut tiles_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Tilemap List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut regions_add_button = TheTraybarButton::new(TheId::named("Tilemap Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        let mut regions_remove_button = TheTraybarButton::new(TheId::named("Tilemap Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());

        let mut name_edit = TheTextLineEdit::new(TheId::named("Tilemap Name Edit"));
        name_edit.limiter_mut().set_max_width(150);
        name_edit.set_status_text("Edit the name of the tilemap.");
        let mut grid_edit = TheTextLineEdit::new(TheId::named("Tilemap Grid Edit"));
        grid_edit.limiter_mut().set_max_width(50);
        grid_edit.set_status_text("Edit the grid size of the tilemap.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(name_edit));
        toolbar_hlayout.add_widget(Box::new(grid_edit));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

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
        filter_edit.set_status_text("Show tiles containing the given text.");
        filter_edit.set_continuous(true);
        tiles_list_header_canvas_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Tilemap Filter Role"));
        drop_down.add_option("All".to_string());
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        tiles_list_header_canvas_hlayout.add_widget(Box::new(drop_down));
        // for dir in TileRole::iterator() {
        //     let mut color_button = TheColorButton::new(TheId::named("Tilemap Filter Character"));
        //     color_button.limiter_mut().set_max_size(vec2i(17, 17));
        //     color_button.set_color(dir.to_color().to_u8_array());
        //     color_button.set_state(TheWidgetState::Selected);
        //     color_button.set_status_text(format!("Show \"{}\" tiles.", dir.to_string()).as_str());
        //     tiles_list_header_canvas_hlayout.add_widget(Box::new(color_button));
        // }

        tiles_list_header_canvas.set_layout(tiles_list_header_canvas_hlayout);

        let mut tile_list_layout = TheListLayout::new(TheId::named("Tilemap Tile List"));
        tile_list_layout.set_item_size(42);
        tiles_list_canvas.set_top(tiles_list_header_canvas);
        tiles_list_canvas.set_layout(tile_list_layout);

        tiles_canvas.set_top(list_canvas);
        tiles_canvas.set_bottom(tiles_list_canvas);
        stack_layout.add_canvas(tiles_canvas);

        // Code

        let mut code_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Code List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 400));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut code_add_button = TheTraybarButton::new(TheId::named("Code Add"));
        code_add_button.set_icon_name("icon_role_add".to_string());
        let mut code_remove_button = TheTraybarButton::new(TheId::named("Code Remove"));
        code_remove_button.set_icon_name("icon_role_remove".to_string());
        let mut name_edit = TheTextLineEdit::new(TheId::named("Code Name Edit"));
        name_edit.limiter_mut().set_max_width(200);
        name_edit.set_status_text("Edit the name of the code.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(code_add_button));
        toolbar_hlayout.add_widget(Box::new(code_remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(name_edit));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        code_canvas.set_top(list_canvas);
        stack_layout.add_canvas(code_canvas);

        //

        let mut canvas = TheCanvas::new();

        canvas.set_top(header);
        canvas.set_right(sectionbar_canvas);
        canvas.top_is_expanding = false;
        canvas.set_layout(stack_layout);

        ui.canvas.set_right(canvas);

        self.apply_region(ui, ctx, None, server);
        self.apply_character(ui, ctx, None);
        self.apply_item(ui, ctx, None);
        self.apply_tilemap(ui, ctx, None);
        self.apply_code(ui, ctx, None);
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::ShowContextMenu(id, _coord) => {
                println!("ShowContextMenu {}", id.name);
            }
            TheEvent::DragStarted(id, text, offset) => {
                if id.name == "Character Item" {
                    let mut drop = TheDrop::new(id.clone());
                    //drop.set_data(atom.to_json());
                    drop.set_title(format!("Character: {}", text));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Region Grid Edit" {
                    if let Some(v) = value.to_i32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.grid_size = v;
                            server.update_region(region);

                            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                                if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
                                    rgba.set_grid(Some(v));
                                }
                            }
                        }
                    }
                } else if id.name == "Tilemap Filter Edit" || id.name == "Tilemap Filter Role" {
                    if let Some(id) = self.curr_tilemap_uuid {
                        self.show_filtered_tiles(ui, ctx, project.get_tilemap(id).as_deref())
                    }
                } else if id.name == "Tilemap Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(layout) = ui.get_rgba_layout("Tilemap Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                    }
                } else if id.name == "Region Content Filter Edit"
                    || id.name == "Region Content Dropdown"
                {
                    self.apply_region(ui, ctx, project.get_region(&server_ctx.curr_region), server);
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
                } else if id.name == "Region Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Region List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_region(&selected.uuid);
                            self.apply_region(ui, ctx, None, server);
                        }
                    }
                } else if id.name == "Region Item" {
                    for r in &project.regions {
                        if r.id == id.uuid {
                            self.apply_region(ui, ctx, Some(r), server);
                            redraw = true;
                        }
                    }
                } else if id.name == "Region Settings" {
                    self.show_region_settings(ui, ctx);
                } else if id.name == "Character Add" {
                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        let mut bundle = TheCodeBundle::new();

                        let mut init = TheCodeGrid {
                            name: "init".into(),
                            ..Default::default()
                        };
                        init.insert_atom(
                            (0, 0),
                            TheCodeAtom::ObjectSet("self".to_string(), "name".to_string()),
                        );
                        init.insert_atom((1, 0), TheCodeAtom::Assignment("=".to_string()));
                        init.insert_atom(
                            (2, 0),
                            TheCodeAtom::Value(TheValue::Text("Unnamed".to_string())),
                        );

                        init.insert_atom(
                            (0, 2),
                            TheCodeAtom::ObjectSet("self".to_string(), "tile".to_string()),
                        );
                        init.insert_atom((1, 2), TheCodeAtom::Assignment("=".to_string()));
                        init.insert_atom(
                            (2, 2),
                            TheCodeAtom::Value(TheValue::Tile("Name".to_string(), Uuid::nil())),
                        );

                        bundle.insert_grid(init);

                        let main = TheCodeGrid {
                            name: "main".into(),
                            ..Default::default()
                        };
                        bundle.insert_grid(main);

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Character Item", bundle.id));
                        item.set_text(bundle.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_character(ui, ctx, Some(&bundle));
                        server.insert_character(bundle.clone());
                        project.add_character(bundle);
                    }
                } else if id.name == "Character Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_character(&selected.uuid);
                            self.apply_character(ui, ctx, None);
                        }
                    }
                } else if id.name == "Character Item" {
                    if let Some(c) = project.characters.get(&id.uuid) {
                        server_ctx.curr_character = Some(id.uuid);
                        //server_ctx.curr_character_instance = None;
                        self.apply_character(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Item Add" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        let bundle = TheCodeBundle::new();

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Item Item", bundle.id));
                        item.set_text(bundle.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_item(ui, ctx, Some(&bundle));
                        project.add_item(bundle);
                    }
                } else if id.name == "Item Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_item(&selected.uuid);
                            self.apply_item(ui, ctx, None);
                        }
                    }
                } else if id.name == "Item Item" {
                    if let Some(c) = project.items.get(&id.uuid) {
                        self.apply_item(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Code Add" {
                    if let Some(list_layout) = ui.get_list_layout("Code List") {
                        let bundle = TheCodeBundle::new();

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Code Item", bundle.id));
                        item.set_text(bundle.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_code(ui, ctx, Some(&bundle));
                        project.add_code(bundle);
                    }
                } else if id.name == "Code Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_code(&selected.uuid);
                            self.apply_code(ui, ctx, None);
                        }
                    }
                } else if id.name == "Code Item" {
                    if let Some(c) = project.codes.get(&id.uuid) {
                        self.apply_code(ui, ctx, Some(c));
                        redraw = true;
                    }
                }
                // Tilemap Item Handling
                else if id.name == "Tilemap Add" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "PNG Image".into(),
                            vec!["png".to_string(), "PNG".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("Tilemap Add".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Tilemap Remove" {
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

                            TILEMAPEDITOR.lock().unwrap().set_tilemap(t, ui, ctx);
                            self.apply_tilemap(ui, ctx, Some(t));
                            ctx.ui.relayout = true;
                        }
                    }
                    redraw = true;
                } else if id.name == "Tilemap Editor Clear Selection" {
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

                            if let Some(role_widget) = ui.get_drop_down_menu("Tilemap Editor Role")
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
                                                    tile.sequence.regions[0]
                                                        .scale(&t.buffer, 36, 36),
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

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Tilepicker"),
                                TheValue::Empty,
                            ));

                            if let Some(widget) = ui.get_widget("RenderView") {
                                if let Some(w) = widget
                                    .as_any()
                                    .downcast_mut::<TheRenderView>()
                                    .map(|external_widget| {
                                        external_widget as &mut dyn TheRenderViewTrait
                                    })
                                {
                                    w.renderer_mut().set_textures(project.extract_tiles());
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
                }
                // Section Buttons
                else if id.name == "Region Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Regions".to_string()));
                    }

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Set Region Panel"),
                        TheValue::Empty,
                    ));

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Region;

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

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Set CodeGrid Panel"),
                        TheValue::Empty,
                    ));

                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Character;

                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 1));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Item Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Item".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Item;

                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 2));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Tilemap Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Tilemaps".to_string()));
                    }

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Set Tilemap Panel"),
                        TheValue::Empty,
                    ));

                    if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Tilemap;

                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 3));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Code Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Code".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Code List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Code;

                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 4));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Compile" {
                    // Compile button in the editor. Compile the code and send it to the server if successful.
                    // We do not need to store it in the project because thats already done in the
                    // CodeBundleChanged event.

                    if let Some(layout) = ui.get_code_layout("Code Editor") {
                        if let Some(code_view) = layout.code_view_mut().as_code_view() {
                            let grid = code_view.codegrid_mut();

                            let rc = server.compiler().compile(grid);

                            if let Ok(_module) = rc {
                                let bundle: TheCodeBundle = CODEEDITOR.lock().unwrap().get_bundle();
                                CODEEDITOR.lock().unwrap().set_compiled(true, ui, ctx);

                                // Successfully compiled, transfer the bundle to the server.

                                if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region {
                                    if let Some(character_instance) =
                                        server_ctx.curr_character_instance
                                    {
                                        server.update_character_instance_bundle(
                                            server_ctx.curr_region,
                                            character_instance,
                                            CODEEDITOR.lock().unwrap().get_bundle(),
                                        );
                                    }
                                } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                                    server.insert_character(bundle);
                                }
                            } else {
                                code_view.set_debug_module(TheDebugModule::new());
                            }
                        }
                    }
                }
            }
            TheEvent::CodeBundleChanged(bundle, _) => {
                ctx.ui.relayout = true;
                if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region {
                    if let Some(character_instance) = server_ctx.curr_character_instance {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(character) = region.characters.get_mut(&character_instance)
                            {
                                // Update the character instance
                                character.instance = bundle.clone();
                            }
                        }
                    }
                } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        if let Some(selected) = list_layout.selected() {
                            if selected.uuid == bundle.id {
                                if let Some(character) = project.characters.get_mut(&bundle.id) {
                                    *character = bundle.clone();
                                }
                                redraw = true;
                            }
                        }
                    }
                } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Item {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            if selected.uuid == bundle.id {
                                if let Some(item) = project.items.get_mut(&bundle.id) {
                                    *item = bundle.clone();
                                }
                                redraw = true;
                            }
                        }
                    }
                } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Code {
                    if let Some(list_layout) = ui.get_list_layout("Code List") {
                        if let Some(selected) = list_layout.selected() {
                            if selected.uuid == bundle.id {
                                if let Some(code) = project.codes.get_mut(&bundle.id) {
                                    *code = bundle.clone();
                                }
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Editor Group" {
                    if *index == 0 {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Set Region Panel"),
                            TheValue::Empty,
                        ));
                    } else if *index == 1 {
                        if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region {
                            if let Some(character_instance) = server_ctx.curr_character_instance {
                                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                                    if let Some(character) =
                                        region.characters.get(&character_instance)
                                    {
                                        for grid in character.instance.grids.values() {
                                            if grid.name == "init" {
                                                CODEEDITOR
                                                    .lock()
                                                    .unwrap()
                                                    .set_codegrid(grid.clone(), ui);
                                                ctx.ui.send(TheEvent::Custom(
                                                    TheId::named("Set CodeGrid Panel"),
                                                    TheValue::Empty,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Apply th given project to the UI
    pub fn load_from_project(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        if let Some(list_layout) = ui.get_list_layout("Region List") {
            list_layout.clear();
            for region in &project.regions {
                let mut item = TheListItem::new(TheId::named_with_id("Region Item", region.id));
                item.set_text(region.name.clone());
                list_layout.add_item(item, ctx);
            }
        }
        self.apply_character(ui, ctx, None);
        if let Some(list_layout) = ui.get_list_layout("Character List") {
            list_layout.clear();
            let list = project.sorted_character_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Character Item", id));
                item.set_text(name);
                list_layout.add_item(item, ctx);
            }
        }
        if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
            list_layout.clear();
            for tilemap in &project.tilemaps {
                let mut item = TheListItem::new(TheId::named_with_id("Tilemap Item", tilemap.id));
                item.set_text(tilemap.name.clone());
                list_layout.add_item(item, ctx);
            }
        }
        ui.select_first_list_item("Region List", ctx);
        ui.select_first_list_item("Character List", ctx);
        ui.select_first_list_item("Tilemap List", ctx);

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
    }

    /// Apply the given character to the UI
    pub fn apply_character(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        character: Option<&TheCodeBundle>,
    ) {
        ui.set_widget_disabled_state("Character Remove", ctx, character.is_none());
        ui.set_widget_disabled_state("Character Name Edit", ctx, character.is_none());

        let compiled: bool = CODEEDITOR.lock().unwrap().compiled(ui);

        // Set the character bundle.
        if let Some(character) = character {
            let char_list_canvas: TheCanvas =
                CODEEDITOR
                    .lock()
                    .unwrap()
                    .set_bundle(character.clone(), ctx, self.width);

            if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                if let Some(canvas) = stack_layout.canvas_at_mut(1) {
                    canvas.set_bottom(char_list_canvas);
                }
            }
        } else if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            if let Some(canvas) = stack_layout.canvas_at_mut(1) {
                let mut empty = TheCanvas::new();
                empty.set_layout(TheVLayout::new(TheId::empty()));
                canvas.set_bottom(empty);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Character Name Edit".to_string()), None)
        {
            if let Some(character) = character {
                widget.set_value(TheValue::Text(character.name.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        CODEEDITOR.lock().unwrap().set_compiled(compiled, ui, ctx);
        ctx.ui.relayout = true;
    }

    /// Apply the given item to the UI
    pub fn apply_item(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        item: Option<&TheCodeBundle>,
    ) {
        ui.set_widget_disabled_state("Item Remove", ctx, item.is_none());
        ui.set_widget_disabled_state("Item Name Edit", ctx, item.is_none());

        // Set the Item bundle.
        if let Some(item) = item {
            let item_list_canvas: TheCanvas =
                CODEEDITOR
                    .lock()
                    .unwrap()
                    .set_bundle(item.clone(), ctx, self.width);

            if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                if let Some(canvas) = stack_layout.canvas_at_mut(2) {
                    canvas.set_bottom(item_list_canvas);
                }
            }
        } else if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            if let Some(canvas) = stack_layout.canvas_at_mut(2) {
                let mut empty = TheCanvas::new();
                empty.set_layout(TheVLayout::new(TheId::empty()));
                canvas.set_bottom(empty);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Item Name Edit".to_string()), None)
        {
            if let Some(item) = item {
                widget.set_value(TheValue::Text(item.name.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        ctx.ui.relayout = true;
    }

    /// Apply the given item to the UI
    pub fn apply_code(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        code: Option<&TheCodeBundle>,
    ) {
        ui.set_widget_disabled_state("Code Remove", ctx, code.is_none());
        ui.set_widget_disabled_state("Code Name Edit", ctx, code.is_none());

        // Set the Item bundle.
        if let Some(code) = code {
            let code_list_canvas: TheCanvas =
                CODEEDITOR
                    .lock()
                    .unwrap()
                    .set_bundle(code.clone(), ctx, self.width);

            if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                if let Some(canvas) = stack_layout.canvas_at_mut(4) {
                    canvas.set_bottom(code_list_canvas);
                }
            }
        } else if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            if let Some(canvas) = stack_layout.canvas_at_mut(4) {
                let mut empty = TheCanvas::new();
                empty.set_layout(TheVLayout::new(TheId::empty()));
                canvas.set_bottom(empty);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Code Name Edit".to_string()), None)
        {
            if let Some(item) = code {
                widget.set_value(TheValue::Text(item.name.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        ctx.ui.relayout = true;
    }

    /// Apply the given item to the UI
    pub fn apply_region(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        region: Option<&Region>,
        server: &mut Server,
    ) {
        ui.set_widget_disabled_state("Region Remove", ctx, region.is_none());
        ui.set_widget_disabled_state("Region Settings", ctx, region.is_none());

        // Show the filter region content.

        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Content Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Content Dropdown".to_string()), None)
        {
            if let Some(drop_down_menu) = widget.as_drop_down_menu() {
                drop_down_menu.selected_index()
            } else {
                0
            }
        } else {
            0
        };

        filter_text = filter_text.to_lowercase();

        if let Some(list) = ui.get_list_layout("Region Content List") {
            list.clear();
            if let Some(region) = region {
                if filter_role < 2 {
                    // Show Characters
                    for (id, _) in region.characters.iter() {
                        let mut name = "Character".to_string();
                        if let Some((TheValue::Text(text), _)) =
                            server.get_character_property(region.id, *id, "name".to_string())
                        {
                            name = text;
                        }
                        if filter_text.is_empty() || name.to_lowercase().contains(&filter_text) {
                            let mut item = TheListItem::new(TheId::named_with_id(
                                "Region Content List Item",
                                *id,
                            ));
                            item.set_text(name);
                            item.add_value_column(100, TheValue::Text("Character".to_string()));
                            list.add_item(item, ctx);
                        }
                    }
                }
            }
        }

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

        if let Some(region) = region {
            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    rgba.set_grid(Some(region.grid_size));
                }
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
        self.show_filtered_tiles(ui, ctx, tilemap);
    }

    /// Shows the filtered tiles of the given tilemap.
    pub fn show_filtered_tiles(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        tilemap: Option<&Tilemap>,
    ) {
        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Tilemap Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Tilemap Filter Role".to_string()), None)
        {
            if let Some(drop_down_menu) = widget.as_drop_down_menu() {
                drop_down_menu.selected_index()
            } else {
                0
            }
        } else {
            0
        };

        filter_text = filter_text.to_lowercase();

        if let Some(layout) = ui
            .canvas
            .get_layout(Some(&"Tilemap Tile List".to_string()), None)
        {
            if let Some(list_layout) = layout.as_list_layout() {
                if let Some(tilemap) = tilemap {
                    list_layout.clear();
                    for tile in &tilemap.tiles {
                        if (filter_text.is_empty()
                            || tile.name.to_lowercase().contains(&filter_text))
                            && (filter_role == 0
                                || tile.role
                                    == TileRole::from_index(filter_role as u8 - 1).unwrap())
                        {
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
                            list_layout.add_item(item, ctx);
                        }
                    }
                } else {
                    list_layout.clear();
                }
            }
        }
        ui.select_first_list_item("Tilemap Tile List", ctx);
    }

    pub fn show_region_settings(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
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

        canvas.set_layout(text_layout);
        ui.show_dialog("Region Settings", canvas, ctx);
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

    /// Deselects all items in the given list layout.
    pub fn deselect_all(&self, layout_name: &str, ui: &mut TheUI) {
        if let Some(layout) = ui.canvas.get_layout(Some(&layout_name.to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.deselect_all();
            }
        }
    }
}
