use crate::editor::{CODEEDITOR, SIDEBARMODE, TILEDRAWER};
use crate::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
enum EditorMode {
    Draw,
    Pick,
    Erase,
    Select,
}

pub struct TileEditor {
    editor_mode: EditorMode,

    curr_tile_uuid: Option<Uuid>,

    curr_layer_role: Layer2DRole,

    icon_normal_border_color: RGBA,
    icon_selected_border_color: RGBA,
}

#[allow(clippy::new_without_default)]
impl TileEditor {
    pub fn new() -> Self {
        Self {
            editor_mode: EditorMode::Draw,

            curr_tile_uuid: None,

            curr_layer_role: Layer2DRole::Ground,

            icon_normal_border_color: [100, 100, 100, 255],
            icon_selected_border_color: [255, 255, 255, 255],
        }
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
    ) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut shared_layout = TheSharedHLayout::new(TheId::named("Editor Shared"));

        let mut region_editor = TheRGBALayout::new(TheId::named("Region Editor"));
        if let Some(rgba_view) = region_editor.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::Display);

            if let Some(buffer) = ctx.ui.icon("eldiron_map") {
                rgba_view.set_buffer(buffer.clone());
            }

            rgba_view.set_grid_color([255, 255, 255, 5]);
            rgba_view.set_hover_color(Some([255, 255, 255, 100]));
            rgba_view.set_wheel_scale(-0.2);
        }

        let mut region_editor_canvas = TheCanvas::new();
        region_editor_canvas.set_layout(region_editor);
        shared_layout.add_canvas(region_editor_canvas);

        let mut render_canvas: TheCanvas = TheCanvas::new();
        let render_view = TheRenderView::new(TheId::named("RenderView"));
        render_canvas.set_widget(render_view);
        shared_layout.add_canvas(render_canvas);

        center.set_layout(shared_layout);

        // Picker

        let mut tile_picker = TheCanvas::new();
        let mut vlayout = TheVLayout::new(TheId::named("Editor Icon Layout"));
        vlayout.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        vlayout.limiter_mut().set_max_width(90);
        vlayout.set_margin(vec4i(0, 10, 0, 5));

        let mut icon_preview = TheIconView::new(TheId::named("Icon Preview"));
        icon_preview.set_alpha_mode(false);
        icon_preview.limiter_mut().set_max_size(vec2i(60, 60));
        icon_preview.set_border_color(Some([100, 100, 100, 255]));
        vlayout.add_widget(Box::new(icon_preview));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(5);
        vlayout.add_widget(Box::new(spacer));

        let mut ground_icon = TheIconView::new(TheId::named("Ground Icon"));
        ground_icon.set_text(Some("FLOOR".to_string()));
        ground_icon.set_text_size(10.0);
        ground_icon.set_text_color([200, 200, 200, 255]);
        ground_icon.limiter_mut().set_max_size(vec2i(48, 48));
        ground_icon.set_border_color(Some(self.icon_selected_border_color));

        let mut wall_icon = TheIconView::new(TheId::named("Wall Icon"));
        wall_icon.set_text(Some("WALL".to_string()));
        wall_icon.set_text_size(10.0);
        wall_icon.set_text_color([200, 200, 200, 255]);
        wall_icon.limiter_mut().set_max_size(vec2i(48, 48));
        wall_icon.set_border_color(Some(self.icon_normal_border_color));

        let mut ceiling_icon = TheIconView::new(TheId::named("Ceiling Icon"));
        ceiling_icon.set_text(Some("CEILING".to_string()));
        ceiling_icon.set_text_size(10.0);
        ceiling_icon.set_text_color([200, 200, 200, 255]);
        ceiling_icon.limiter_mut().set_max_size(vec2i(48, 48));
        ceiling_icon.set_border_color(Some(self.icon_normal_border_color));

        let mut cc_icon = TheIconView::new(TheId::named("Tile CC Icon"));
        cc_icon.set_text(Some("CC".to_string()));
        cc_icon.set_text_size(10.0);
        cc_icon.set_text_color([200, 200, 200, 255]);
        cc_icon.limiter_mut().set_max_size(vec2i(48, 48));
        cc_icon.set_border_color(Some(self.icon_normal_border_color));

        vlayout.add_widget(Box::new(ground_icon));
        vlayout.add_widget(Box::new(wall_icon));
        vlayout.add_widget(Box::new(ceiling_icon));
        vlayout.add_widget(Box::new(cc_icon));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(5);
        vlayout.add_widget(Box::new(spacer));

        let mut text = TheText::new(TheId::named("Cursor Position"));
        text.set_text_color([200, 200, 200, 255]);
        vlayout.add_widget(Box::new(text));

        tile_picker.set_layout(vlayout);
        center.set_left(tile_picker);

        // Top Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut gb = TheGroupButton::new(TheId::named("2D3D Group"));
        gb.add_text("2D Map".to_string());
        gb.add_text("Mixed".to_string());
        gb.add_text("3D Map".to_string());

        let mut time_slider = TheTimeSlider::new(TheId::named("Server Time Slider"));
        time_slider.set_continuous(true);
        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_width(70);

        let mut zoom = TheSlider::new(TheId::named("Region Editor Zoom"));
        zoom.set_value(TheValue::Float(1.0));
        zoom.set_range(TheValue::RangeF32(0.5..=3.0));
        zoom.set_continuous(true);
        zoom.limiter_mut().set_max_width(120);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 4, 5, 4));
        toolbar_hlayout.add_widget(Box::new(gb));
        toolbar_hlayout.add_widget(Box::new(spacer));
        toolbar_hlayout.add_widget(Box::new(time_slider));
        toolbar_hlayout.add_widget(Box::new(zoom));
        toolbar_hlayout.set_reverse_index(Some(1));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        // Bottom Toolbar
        let mut bottom_toolbar = TheCanvas::new();
        bottom_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut gb = TheGroupButton::new(TheId::named("Editor Group"));
        gb.add_text_status_icon(
            "Draw".to_string(),
            "Draw tiles in the region.".to_string(),
            "draw".to_string(),
        );
        gb.add_text_status_icon(
            "Pick".to_string(),
            "Pick content in the region.".to_string(),
            "pick".to_string(),
        );
        gb.add_text_status_icon(
            "Erase".to_string(),
            "Delete content in the region.".to_string(),
            "eraser".to_string(),
        );
        gb.add_text_status_icon(
            "Select".to_string(),
            "Select an area in the region.".to_string(),
            "selection".to_string(),
        );
        gb.set_item_width(65);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 4, 5, 4));
        toolbar_hlayout.add_widget(Box::new(gb));

        bottom_toolbar.set_layout(toolbar_hlayout);
        center.set_bottom(bottom_toolbar);

        center
    }

    pub fn load_from_project(&mut self, ui: &mut TheUI, _ctx: &mut TheContext, project: &Project) {
        TILEDRAWER
            .lock()
            .unwrap()
            .set_tiles(project.extract_tiles());
        if let Some(widget) = ui.get_widget("RenderView") {
            if let Some(w) = widget
                .as_any()
                .downcast_mut::<TheRenderView>()
                .map(|external_widget| external_widget as &mut dyn TheRenderViewTrait)
            {
                w.renderer_mut().set_textures(project.extract_tiles());
            }
        }
    }

    #[allow(clippy::suspicious_else_formatting)]
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
            TheEvent::ContextMenuSelected(_widget_id, item_id) => {
                if item_id.name == "Create Area" {
                    open_text_dialog(
                        "New Area Name",
                        "Area Name",
                        "New Area",
                        Uuid::new_v4(),
                        ui,
                        ctx,
                    );
                }
            }
            TheEvent::KeyDown(v) => {
                if let Some(str) = v.to_char() {
                    server.set_key_down(Some(str.to_string()));
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "2D3D Group" {
                    if let Some(shared) = ui.get_sharedhlayout("Editor Shared") {
                        if *index == 0 {
                            shared.set_mode(TheSharedHLayoutMode::Left);
                        } else if *index == 1 {
                            shared.set_mode(TheSharedHLayoutMode::Shared);
                        } else if *index == 2 {
                            shared.set_mode(TheSharedHLayoutMode::Right);
                        }
                        ctx.ui.relayout = true;

                        // Set the region and textures to the RenderView if visible
                        if *index > 0 {
                            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                                if let Some(widget) = ui.get_widget("RenderView") {
                                    if let Some(w) = widget
                                        .as_any()
                                        .downcast_mut::<TheRenderView>()
                                        .map(|external_widget| {
                                            external_widget as &mut dyn TheRenderViewTrait
                                        })
                                    {
                                        w.renderer_mut().set_region(region);
                                        w.renderer_mut().set_textures(project.extract_tiles());
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "Editor Group" {
                    if *index == 0 {
                        self.editor_mode = EditorMode::Draw;
                        server_ctx.tile_selection = None;
                        server_ctx.curr_character_instance = None;
                        server_ctx.curr_item_instance = None;
                        server_ctx.curr_area = None;
                    } else if *index == 1 {
                        self.editor_mode = EditorMode::Pick;
                        server_ctx.tile_selection = None;
                    } else if *index == 2 {
                        self.editor_mode = EditorMode::Erase;
                        server_ctx.tile_selection = None;
                    } else if *index == 3 {
                        ui.set_widget_context_menu(
                            "Region Editor View",
                            Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Create Area...".to_string(),
                                    TheId::named("Create Area"),
                                )],
                                ..Default::default()
                            }),
                        );
                        self.editor_mode = EditorMode::Select;
                    }

                    if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                        || *SIDEBARMODE.lock().unwrap() == SidebarMode::Character
                    {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Set Region Panel"),
                            TheValue::Empty,
                        ));
                    }
                }
            }
            /*
            TheEvent::TileEditorDelete(_id, keys) => {
                if self.editor_mode == EditorMode::Pick {
                    // If there is a character instance at the position we delete the instance.
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        let mut changed = false;
                        let mut undo = TheUndo::new(TheId::named("RegionChanged"));
                        undo.set_undo_data(region.to_json());

                        for k in keys {
                            if let Some(c) =
                                server.get_character_at(server_ctx.curr_region, vec2i(k.0, k.1))
                            {
                                if region.characters.remove(&c.0).is_some() {
                                    changed = true;
                                    server.remove_character_instance(region.id, c.0);
                                    server_ctx.curr_character_instance = None;
                                    server_ctx.curr_character = None;
                                    redraw = true;
                                    // Remove from the content list
                                    if let Some(list) = ui.get_list_layout("Region Content List") {
                                        list.remove(TheId::named_with_id(
                                            "Region Content List Item",
                                            c.0,
                                        ));
                                    }
                                }
                            }
                        }

                        if changed {
                            undo.set_redo_data(region.to_json());
                            ctx.ui.undo_stack.add(undo);
                            server.update_region(region);
                            redraw = true;
                        }
                    }
                } else if self.editor_mode == EditorMode::Draw {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        let mut undo = TheUndo::new(TheId::named("RegionChanged"));
                        undo.set_undo_data(region.to_json());
                        let mut changed = false;
                        let mut p = vec2i(0, 0);
                        for k in keys {
                            if region.tiles.contains_key(k) {
                                region.tiles.remove(k);
                                changed = true;
                                p = vec2i(k.0, k.1);
                            }
                        }
                        if changed {
                            undo.set_redo_data(region.to_json());
                            ctx.ui.undo_stack.add(undo);
                            server.update_region(region);
                            self.set_icon_previews(region, p, ui);
                            redraw = true;
                        }
                    }
                }
            }*/
            TheEvent::TileEditorUp(_id) => {
                if self.editor_mode == EditorMode::Select {
                    if let Some(tilearea) = &mut server_ctx.tile_selection {
                        tilearea.ongoing = false;
                    }
                }
            }
            TheEvent::TileEditorClicked(_id, coord) | TheEvent::TileEditorDragged(_id, coord) => {
                if self.editor_mode == EditorMode::Select {
                    let p = (coord.x, coord.y);

                    if let Some(tilearea) = &mut server_ctx.tile_selection {
                        if !tilearea.ongoing {
                            tilearea.start = p;
                            tilearea.end = p;
                            tilearea.ongoing = true;
                        } else {
                            tilearea.grow_by(p);
                        }
                    } else {
                        let tilearea = TileArea {
                            start: p,
                            end: p,
                            ..Default::default()
                        };
                        server_ctx.tile_selection = Some(tilearea);
                    }
                } else if self.editor_mode == EditorMode::Erase {
                    // If there is a character instance at the position we delete the instance.
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(c) =
                            server.get_character_at(server_ctx.curr_region, vec2i(coord.x, coord.y))
                        {
                            // Delete the character at the given position.

                            if let Some((value, _)) =
                                server.get_character_property(region.id, c.0, "name".to_string())
                            {
                                open_delete_confirmation_dialog(
                                    "Delete Character Instance ?",
                                    format!("Permanently delete '{}' ?", value.describe()).as_str(),
                                    c.0,
                                    ui,
                                    ctx,
                                );
                            }
                        } else if let Some(c) =
                            server.get_item_at(server_ctx.curr_region, vec2i(coord.x, coord.y))
                        {
                            // Delete the item at the given position.

                            if let Some((value, _)) =
                                server.get_character_property(region.id, c.0, "name".to_string())
                            {
                                open_delete_confirmation_dialog(
                                    "Delete Item Instance ?",
                                    format!("Permanently delete '{}' ?", value.describe()).as_str(),
                                    c.0,
                                    ui,
                                    ctx,
                                );
                            }
                        } else {
                            let mut area_id = None;

                            // Check for area at the given position.
                            for area in region.areas.values() {
                                if area.area.contains(&(coord.x, coord.y)) {
                                    // Ask to delete it.
                                    open_delete_confirmation_dialog(
                                        "Delete Area ?",
                                        format!("Permanently delete area '{}' ?", area.name)
                                            .as_str(),
                                        area.id,
                                        ui,
                                        ctx,
                                    );
                                    area_id = Some(area.id);
                                    break;
                                }
                            }

                            if area_id.is_none() {
                                // Delete the tile at the given position.
                                let mut undo = TheUndo::new(TheId::named("RegionChanged"));
                                undo.set_undo_data(region.to_json());

                                if region.tiles.contains_key(&(coord.x, coord.y)) {
                                    region.tiles.remove(&(coord.x, coord.y));
                                }

                                undo.set_redo_data(region.to_json());
                                ctx.ui.undo_stack.add(undo);
                                server.update_region(region);
                                self.set_icon_previews(region, *coord, ui);
                                self.redraw_region(ui, server, ctx, server_ctx);
                                redraw = true;
                            }
                        }
                    }
                } else if self.editor_mode == EditorMode::Pick {
                    // Check for character at the given position.
                    if let Some(c) = server.get_character_at(server_ctx.curr_region, *coord) {
                        server_ctx.curr_character_instance = Some(c.0);
                        server_ctx.curr_character = Some(c.1);
                        server_ctx.curr_area = None;
                        server_ctx.curr_item_instance = None;
                        server_ctx.curr_item = None;

                        if let Some(layout) = ui.get_list_layout("Region Content List") {
                            layout.select_item(c.0, ctx, false);
                        }

                        if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                            || *SIDEBARMODE.lock().unwrap() == SidebarMode::Character
                        {
                            // In Region mode, we need to set the character bundle of the current character instance.
                            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                                if let Some(character) = region.characters.get(&c.0) {
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
                        //else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                        // In Character mode, we need to set the character bundle of the current character.
                        //}
                    }
                    // Check for an item at the given position.
                    else if let Some(c) = server.get_item_at(server_ctx.curr_region, *coord) {
                        server_ctx.curr_item_instance = Some(c.0);
                        server_ctx.curr_item = Some(c.1);
                        server_ctx.curr_area = None;

                        if let Some(layout) = ui.get_list_layout("Region Content List") {
                            layout.select_item(c.0, ctx, false);
                        }

                        if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                            || *SIDEBARMODE.lock().unwrap() == SidebarMode::Item
                        {
                            // In Region mode, we need to set the character bundle of the current character instance.
                            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                                if let Some(item) = region.items.get(&c.0) {
                                    for grid in item.instance.grids.values() {
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
                        //else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                        // In Character mode, we need to set the character bundle of the current character.
                        //}
                    } else if let Some(region) = project.get_region(&server_ctx.curr_region) {
                        let mut found_area = false;

                        // Check for area at the given position.
                        for area in region.areas.values() {
                            if area.area.contains(&(coord.x, coord.y)) {
                                for grid in area.bundle.grids.values() {
                                    if grid.name == "main" {
                                        if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                                            || *SIDEBARMODE.lock().unwrap()
                                                == SidebarMode::Character
                                        {
                                            CODEEDITOR
                                                .lock()
                                                .unwrap()
                                                .set_codegrid(grid.clone(), ui);
                                            ctx.ui.send(TheEvent::Custom(
                                                TheId::named("Set CodeGrid Panel"),
                                                TheValue::Empty,
                                            ));
                                        }
                                        found_area = true;
                                        server_ctx.curr_character_instance = None;
                                        server_ctx.curr_character = None;
                                        server_ctx.curr_area = Some(area.id);
                                        if let Some(layout) =
                                            ui.get_list_layout("Region Content List")
                                        {
                                            layout.select_item(area.id, ctx, false);
                                        }
                                        break;
                                    }
                                }
                            }
                        }

                        if !found_area {
                            // No area, set the tile.
                            server_ctx.curr_character_instance = None;
                            if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
                                for uuid in tile.layers.iter().flatten() {
                                    if TILEDRAWER.lock().unwrap().tiles.contains_key(uuid) {
                                        ctx.ui.send(TheEvent::StateChanged(
                                            TheId::named_with_id("Tilemap Tile", *uuid),
                                            TheWidgetState::Selected,
                                        ));

                                        // Switch mode to draw, disabled for now
                                        // self.editor_mode = EditorMode::Draw;
                                        // if let Some(button) = ui.get_group_button("Editor Group") {
                                        //     button.set_index(0);
                                        //     redraw = true;
                                        // }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                        || *SIDEBARMODE.lock().unwrap() == SidebarMode::Character
                    {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Set Region Panel"),
                            TheValue::Empty,
                        ));
                    }
                } else if self.editor_mode == EditorMode::Draw {
                    if self.curr_layer_role == Layer2DRole::Other {
                        // Paint with color correction
                        // TODO
                    } else if let Some(curr_tile_uuid) = self.curr_tile_uuid {
                        if TILEDRAWER
                            .lock()
                            .unwrap()
                            .tiles
                            .contains_key(&curr_tile_uuid)
                        {
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                let mut undo = TheUndo::new(TheId::named("RegionChanged"));
                                undo.set_undo_data(region.to_json());

                                region.set_tile(
                                    (coord.x, coord.y),
                                    self.curr_layer_role,
                                    self.curr_tile_uuid,
                                );
                                undo.set_redo_data(region.to_json());
                                self.set_icon_previews(region, *coord, ui);

                                server.update_region(region);

                                if let Some(widget) = ui.get_widget("RenderView") {
                                    if let Some(w) = widget
                                        .as_any()
                                        .downcast_mut::<TheRenderView>()
                                        .map(|external_widget| {
                                            external_widget as &mut dyn TheRenderViewTrait
                                        })
                                    {
                                        w.renderer_mut().set_region(region);
                                    }
                                }

                                ctx.ui.undo_stack.add(undo);
                            }
                        }
                        self.redraw_region(ui, server, ctx, server_ctx);
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(_id, coord) => {
                if let Some(text) = ui.get_text("Cursor Position") {
                    text.set_text(format!("({}, {})", coord.x, coord.y));
                    redraw = true;
                    if let Some(layout) = ui.get_layout("Editor Icon Layout") {
                        layout.relayout(ctx);
                    }
                }

                for r in &mut project.regions {
                    if r.id == server_ctx.curr_region {
                        self.set_icon_previews(r, *coord, ui);
                        break;
                    }
                }

                if let Some(widget) = ui.get_widget("RenderView") {
                    if let Some(w) = widget
                        .as_any()
                        .downcast_mut::<TheRenderView>()
                        .map(|external_widget| external_widget as &mut dyn TheRenderViewTrait)
                    {
                        w.renderer_mut().set_position(vec3i(coord.x, 0, coord.y));
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Region Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.zoom = v;
                        }
                        if let Some(layout) = ui.get_rgba_layout("Region Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, _state) => {
                // Region Content List Selection
                if id.name == "Region Content List Item" {
                    if let Some((TheValue::Position(p), character_id)) = server
                        .get_character_property(server_ctx.curr_region, id.uuid, "position".into())
                    {
                        // If it's a character instance, center it in the region editor.
                        server_ctx.curr_character_instance = Some(id.uuid);
                        server_ctx.curr_character = Some(character_id);
                        server_ctx.curr_area = None;

                        self.editor_mode = EditorMode::Pick;
                        if let Some(button) = ui.get_group_button("Editor Group") {
                            button.set_index(1);
                            ctx.ui.send(TheEvent::IndexChanged(button.id().clone(), 1));
                        }

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Set Region Panel"),
                            TheValue::Empty,
                        ));

                        if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                            rgba_layout.scroll_to_grid(vec2i(p.x as i32, p.z as i32));
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                region.scroll_offset = vec2i(
                                    p.x as i32 * region.grid_size,
                                    p.z as i32 * region.grid_size,
                                );
                            }
                        }
                    }
                    if let Some((TheValue::Position(p), character_id)) =
                        server.get_item_property(server_ctx.curr_region, id.uuid, "position".into())
                    {
                        // If it's a character instance, center it in the region editor.
                        server_ctx.curr_item_instance = Some(id.uuid);
                        server_ctx.curr_item = Some(character_id);
                        server_ctx.curr_area = None;

                        self.editor_mode = EditorMode::Pick;
                        if let Some(button) = ui.get_group_button("Editor Group") {
                            button.set_index(1);
                            ctx.ui.send(TheEvent::IndexChanged(button.id().clone(), 1));
                        }

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Set Region Panel"),
                            TheValue::Empty,
                        ));

                        if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                            rgba_layout.scroll_to_grid(vec2i(p.x as i32, p.z as i32));
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                region.scroll_offset = vec2i(
                                    p.x as i32 * region.grid_size,
                                    p.z as i32 * region.grid_size,
                                );
                            }
                        }
                    } else if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(area) = region.areas.get(&id.uuid) {
                            server_ctx.curr_character_instance = None;
                            server_ctx.curr_character = None;
                            server_ctx.curr_area = Some(area.id);

                            self.editor_mode = EditorMode::Pick;
                            if let Some(button) = ui.get_group_button("Editor Group") {
                                button.set_index(1);
                                ctx.ui.send(TheEvent::IndexChanged(button.id().clone(), 1));
                            }

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Set Region Panel"),
                                TheValue::Empty,
                            ));

                            // Add the area to the server
                            server.insert_area(region.id, area.clone());

                            if let Some(p) = area.center() {
                                if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                                    rgba_layout.scroll_to_grid(vec2i(p.0, p.1));
                                    region.scroll_offset =
                                        vec2i(p.0 * region.grid_size, p.1 * region.grid_size);
                                }
                            }
                        }
                    }
                }
                // Region Selection
                else if id.name == "Region Item" {
                    for r in &project.regions {
                        if r.id == id.uuid {
                            if let Some(rgba_layout) =
                                ui.canvas.get_layout(Some(&"Region Editor".into()), None)
                            {
                                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                                    if let Some(rgba_view) =
                                        rgba_layout.rgba_view_mut().as_rgba_view()
                                    {
                                        rgba_view.set_mode(TheRGBAViewMode::TileEditor);
                                        let width = r.width * r.grid_size;
                                        let height = r.height * r.grid_size;
                                        let buffer =
                                            TheRGBABuffer::new(TheDim::new(0, 0, width, height));
                                        rgba_view.set_buffer(buffer);
                                        rgba_view.set_grid(Some(r.grid_size));
                                        ctx.ui.relayout = true;
                                    }
                                    rgba_layout.scroll_to(r.scroll_offset);
                                }
                            }
                            if let Some(widget) = ui.get_widget("RenderView") {
                                if let Some(w) = widget
                                    .as_any()
                                    .downcast_mut::<TheRenderView>()
                                    .map(|external_widget| {
                                        external_widget as &mut dyn TheRenderViewTrait
                                    })
                                {
                                    w.renderer_mut().set_region(r);
                                }
                            }
                            server_ctx.curr_region = r.id;
                            self.redraw_region(ui, server, ctx, server_ctx);
                            redraw = true;
                        }
                    }
                }
                // An item in the tile list was selected
                else if id.name == "Tilemap Tile" {
                    self.curr_tile_uuid = Some(id.uuid);

                    if let Some(t) = TILEDRAWER.lock().unwrap().tiles.get(&id.uuid) {
                        if let Some(icon_view) = ui.get_icon_view("Icon Preview") {
                            icon_view.set_rgba_tile(t.clone());
                        }
                    }
                } else if id.name == "Tilemap Editor Add Selection" {
                    TILEDRAWER.lock().unwrap().tiles = project.extract_tiles();
                    server.update_tiles(project.extract_tiles());
                } else if id.name == "Ground Icon" {
                    self.curr_layer_role = Layer2DRole::Ground;
                    self.set_icon_colors(ui);
                    redraw = true;
                } else if id.name == "Wall Icon" {
                    self.curr_layer_role = Layer2DRole::Wall;
                    self.set_icon_colors(ui);
                    redraw = true;
                } else if id.name == "Ceiling Icon" {
                    self.curr_layer_role = Layer2DRole::Ceiling;
                    self.set_icon_colors(ui);
                    redraw = true;
                } else if id.name == "Tile CC Icon" {
                    self.curr_layer_role = Layer2DRole::Other;
                    self.set_icon_colors(ui);
                    redraw = true;
                }
            }
            _ => {}
        }
        redraw
    }

    fn set_icon_previews(&mut self, region: &mut Region, coord: Vec2i, ui: &mut TheUI) {
        // Ground Icon Preview
        if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
            // Ground
            let mut success = false;
            if let Some(ground) = tile.layers[0] {
                if let Some(tile) = TILEDRAWER.lock().unwrap().tiles.get(&ground) {
                    if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                        icon_view.set_rgba_tile(tile.clone());
                        success = true;
                    }
                }
            }
            if !success {
                if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                    icon_view.set_rgba_tile(TheRGBATile::default());
                }
            }

            // Wall
            success = false;
            if let Some(wall) = tile.layers[1] {
                if let Some(tile) = TILEDRAWER.lock().unwrap().tiles.get(&wall) {
                    if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
                        icon_view.set_rgba_tile(tile.clone());
                        success = true;
                    }
                }
            }
            if !success {
                if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
                    icon_view.set_rgba_tile(TheRGBATile::default());
                }
            }

            // Ceiling
            success = false;
            if let Some(ceiling) = tile.layers[2] {
                if let Some(tile) = TILEDRAWER.lock().unwrap().tiles.get(&ceiling) {
                    if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
                        icon_view.set_rgba_tile(tile.clone());
                        success = true;
                    }
                }
            }
            if !success {
                if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
                    icon_view.set_rgba_tile(TheRGBATile::default());
                }
            }
            // if let Some(overlay) = tile.layers[3] {
            //     if let Some(tile) = self.tiledrawer.tiles.get(&overlay) {
            //         if let Some(icon_view) = ui.get_icon_view("Overlay Icon") {
            //             icon_view.set_rgba_tile(tile.clone());
            //         }
            //     } else if let Some(icon_view) = ui.get_icon_view("Overlay Icon") {
            //         icon_view.set_rgba_tile(TheRGBATile::default());
            //     }
            // }
        } else {
            if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                icon_view.set_rgba_tile(TheRGBATile::default());
            }
            if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
                icon_view.set_rgba_tile(TheRGBATile::default());
            }
            if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
                icon_view.set_rgba_tile(TheRGBATile::default());
            }
            // if let Some(icon_view) = ui.get_icon_view("Overlay Icon") {
            //     icon_view.set_rgba_tile(TheRGBATile::default());
            // }
        }
    }

    fn set_icon_colors(&mut self, ui: &mut TheUI) {
        if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
            icon_view.set_border_color(if self.curr_layer_role == Layer2DRole::Ground {
                Some(self.icon_selected_border_color)
            } else {
                Some(self.icon_normal_border_color)
            });
        }
        if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
            icon_view.set_border_color(if self.curr_layer_role == Layer2DRole::Wall {
                Some(self.icon_selected_border_color)
            } else {
                Some(self.icon_normal_border_color)
            });
        }
        if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
            icon_view.set_border_color(if self.curr_layer_role == Layer2DRole::Ceiling {
                Some(self.icon_selected_border_color)
            } else {
                Some(self.icon_normal_border_color)
            });
        }
        if let Some(icon_view) = ui.get_icon_view("Tile CC Icon") {
            icon_view.set_border_color(if self.curr_layer_role == Layer2DRole::Other {
                Some(self.icon_selected_border_color)
            } else {
                Some(self.icon_normal_border_color)
            });
        }
    }

    /// Redraw the map of the current region on tick.
    pub fn redraw_region(
        &mut self,
        ui: &mut TheUI,
        server: &mut Server,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Region Editor".into()), None) {
            if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    server.draw_region(
                        &server_ctx.curr_region,
                        rgba_view.buffer_mut(),
                        &TILEDRAWER.lock().unwrap(),
                        ctx,
                        server_ctx,
                    );
                    rgba_view.set_needs_redraw(true);
                }
            }
        }
    }
}
