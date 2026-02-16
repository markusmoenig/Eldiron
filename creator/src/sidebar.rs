use crate::editor::{
    ACTIONLIST, CODEEDITOR, CONFIG, CONFIGEDITOR, DOCKMANAGER, PALETTE, RUSTERIX, SCENEMANAGER,
    SHADEGRIDFX, SIDEBARMODE, TOOLLIST, UNDOMANAGER,
};
use crate::minimap::draw_minimap;
use crate::prelude::*;
use crate::undo::project_helper::*;
use codegridfx::Module;
use rusterix::{Texture, TileRole};

#[derive(PartialEq, Debug)]
pub enum SidebarMode {
    Region,
    Character,
    Item,
    Tilemap,
    Module,
    Screen,
    Asset,
    Shader,
    Action,
    // Node,
    Debug,
    Palette,
}

pub struct Sidebar {
    pub width: i32,

    curr_tilemap_uuid: Option<Uuid>,

    pub startup: bool,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    pub fn new() -> Self {
        Self {
            width: 380,

            curr_tilemap_uuid: None,

            startup: true,
        }
    }

    pub fn init_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // Tree View

        let mut canvas: TheCanvas = TheCanvas::new();

        let mut project_canvas: TheCanvas = TheCanvas::new();
        let mut project_tree_layout = TheTreeLayout::new(TheId::named("Project Tree"));
        let root = project_tree_layout.get_root();

        let mut regions_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("regions"),
            server_ctx.tree_regions_id,
        ));
        regions_node.set_open(true);

        root.add_child(regions_node);

        let characters_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("characters"),
            server_ctx.tree_characters_id,
        ));
        root.add_child(characters_node);

        let items_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("items"),
            server_ctx.tree_items_id,
        ));
        root.add_child(items_node);

        let tilemaps_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("tilesets"),
            server_ctx.tree_tilemaps_id,
        ));
        root.add_child(tilemaps_node);

        let screens_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("screens"),
            server_ctx.tree_screens_id,
        ));
        root.add_child(screens_node);

        let avatars_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("avatars"),
            server_ctx.tree_avatars_id,
        ));
        root.add_child(avatars_node);

        let mut assets_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("assets"),
            server_ctx.tree_assets_id,
        ));

        let fonts_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("fonts"),
            server_ctx.tree_assets_fonts_id,
        ));
        assets_node.add_child(fonts_node);
        root.add_child(assets_node);

        let mut palette_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("palette"),
            server_ctx.tree_palette_id,
        ));

        let mut item = TheTreeIcons::new(TheId::named("Palette Item"));
        item.set_icon_count(256);
        item.set_icons_per_row(17);
        item.set_selected_index(Some(0));
        palette_node.add_widget(Box::new(item));
        root.add_child(palette_node);

        let mut config_node: TheTreeNode = TheTreeNode::new(TheId::named(&fl!("game")));

        let mut config_item = TheTreeItem::new(TheId::named("Project Settings"));
        config_item.set_text(fl!("settings"));
        config_node.add_widget(Box::new(config_item));

        let mut debug_log_item = TheTreeItem::new(TheId::named("Debug Log"));
        debug_log_item.set_text(fl!("debug_log"));
        config_node.add_widget(Box::new(debug_log_item));

        root.add_child(config_node);

        project_canvas.set_layout(project_tree_layout);

        // Tree View Toolbar

        let mut add_button = TheTraybarButton::new(TheId::named("Project Add"));
        add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_status_text(&fl!("status_project_add_button"));
        add_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Add Region".to_string(), TheId::named("Add Region")),
                TheContextMenuItem::new("Add Character".to_string(), TheId::named("Add Character")),
                TheContextMenuItem::new("Add Item".to_string(), TheId::named("Add Item")),
                TheContextMenuItem::new("Add Tileset".to_string(), TheId::named("Add Tileset")),
                TheContextMenuItem::new("Add Screen".to_string(), TheId::named("Add Screen")),
                TheContextMenuItem::new("Add Avatar".to_string(), TheId::named("Add Avatar")),
                TheContextMenuItem::new(
                    "Add Font Asset".to_string(),
                    TheId::named("Add Font Asset"),
                ),
            ],
            ..Default::default()
        }));

        let mut remove_button = TheTraybarButton::new(TheId::named("Project Remove"));
        remove_button.set_icon_name("icon_role_remove".to_string());
        remove_button.set_status_text(&fl!("status_project_remove_button"));

        let mut project_context_text = TheText::new(TheId::named("Project Context"));
        project_context_text.set_text("".to_string());

        let mut import_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Project Import"));
        import_button.set_icon_name("import".to_string());
        import_button.set_status_text(&fl!("status_project_import_button"));
        import_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Import Region".to_string(), TheId::named("Import Region")),
                TheContextMenuItem::new(
                    "Import Character".to_string(),
                    TheId::named("Import Character"),
                ),
                TheContextMenuItem::new("Import Item".to_string(), TheId::named("Import Item")),
                TheContextMenuItem::new(
                    "Import Tileset".to_string(),
                    TheId::named("Import Tileset"),
                ),
                TheContextMenuItem::new("Import Screen".to_string(), TheId::named("Import Screen")),
                TheContextMenuItem::new("Import Avatar".to_string(), TheId::named("Import Avatar")),
                TheContextMenuItem::new(
                    "Import Font Asset".to_string(),
                    TheId::named("Import Font Asset"),
                ),
            ],
            ..Default::default()
        }));

        let mut export_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Project Export"));
        export_button.set_icon_name("export".to_string());
        export_button.set_status_text(&fl!("status_project_export_button"));

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(add_button));
        toolbar_hlayout.add_widget(Box::new(remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(project_context_text));
        toolbar_hlayout.add_widget(Box::new(import_button));
        toolbar_hlayout.add_widget(Box::new(export_button));

        toolbar_hlayout.set_reverse_index(Some(2));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        project_canvas.set_bottom(toolbar_canvas);

        // Shared Layout

        let mut stack_layout = TheStackLayout::new(TheId::named("Tree Stack Layout"));
        stack_layout.add_canvas(project_canvas);

        // canvas.set_top(header);
        // canvas.set_right(sectionbar_canvas);
        // canvas.top_is_expanding = false;
        // canvas.set_layout(stack_layout);

        canvas.set_layout(stack_layout);

        // Multi functional footer canvas

        let mut right_canvas = TheCanvas::new();

        let mut shared_layout = TheSharedVLayout::new(TheId::named("Multi Shared"));

        let mut nodes_minimap_canvas: TheCanvas = TheCanvas::default();
        let mut nodes_minimap_shared = TheSharedVLayout::new(TheId::named("Multi Tab"));
        nodes_minimap_shared.set_shared_ratio(0.5);
        nodes_minimap_shared.set_mode(TheSharedVLayoutMode::Shared);

        let mut minimap_canvas = TheCanvas::default();
        let mut minimap = TheRenderView::new(TheId::named("MiniMap"));
        minimap.limiter_mut().set_max_width(self.width);
        minimap_canvas.set_widget(minimap);

        let mut action_params_canvas = TheCanvas::default();
        let mut textedit = TheTextAreaEdit::new(TheId::named("Action Params TOML"));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_syntax_from_string(source);
            textedit.set_code_type("TOML");
        }
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_theme_from_string(source);
            textedit.set_code_theme("Gruvbox Dark");
        }
        textedit.set_continuous(true);
        textedit.display_line_number(false);
        textedit.use_global_statusbar(true);
        textedit.set_font_size(13.5);
        action_params_canvas.set_widget(textedit);

        // let mut header = TheCanvas::new();
        // let mut switchbar = TheSwitchbar::new(TheId::named("Action Header"));
        // switchbar.set_text("Settings".to_string());
        // header.set_widget(switchbar);

        // nodes_minimap_canvas.set_top(header);

        nodes_minimap_shared.add_canvas(action_params_canvas);
        nodes_minimap_shared.add_canvas(minimap_canvas);
        nodes_minimap_canvas.set_layout(nodes_minimap_shared);

        shared_layout.add_canvas(canvas);
        shared_layout.add_canvas(nodes_minimap_canvas);
        shared_layout.set_mode(TheSharedVLayoutMode::Shared);
        shared_layout.set_shared_ratio(0.6);
        shared_layout.limiter_mut().set_max_width(self.width);

        right_canvas.set_layout(shared_layout);
        right_canvas.top_is_expanding = false;

        // --

        ui.canvas.set_right(right_canvas);

        self.apply_region(ui, ctx, None, &mut Project::default());
        self.apply_screen(ui, ctx, None);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::SnapperStateChanged(id, _layout_id, open) => {
                if *open {
                    // Region
                    if project.contains_region(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Region(id.uuid),
                        );
                        self.apply_region(ui, ctx, Some(id.uuid), project);
                    } else
                    // Character
                    if let Some(_character) = project.characters.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(id.uuid),
                        );
                    } else
                    // Item
                    if let Some(_item) = project.items.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Item(id.uuid),
                        );
                    } else
                    // Tilemap
                    if let Some(_item) = project.get_tilemap(id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.uuid),
                        );
                    } else
                    // Screen
                    if let Some(_item) = project.screens.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(id.uuid),
                        );
                    } else
                    // Asset
                    if let Some(_item) = project.assets.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(id.uuid),
                        );
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Character Region Override" {
                    server_ctx.character_region_override = *index == 1;
                } else if id.name == "Item Region Override" {
                    server_ctx.item_region_override = *index == 1;
                } else if id.name == "Palette Item" {
                    project.palette.current_index = *index as u16;
                } else if id.name == "Avatar Perspective Count" {
                    let new_count = match index {
                        0 => AvatarPerspectiveCount::One,
                        _ => AvatarPerspectiveCount::Four,
                    };
                    if let Some(avatar) = project.avatars.get(&id.references) {
                        let old_count = avatar.perspective_count;
                        if old_count != new_count {
                            let atom = ProjectUndoAtom::EditAvatarPerspectiveCount(
                                id.references,
                                old_count,
                                new_count,
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        }
                    }
                } else if id.name.starts_with("Avatar Perspective Icons ") {
                    // Parse the perspective index from the widget name
                    if let Some(persp_index_str) = id.name.strip_prefix("Avatar Perspective Icons ")
                    {
                        if let Ok(persp_index) = persp_index_str.parse::<usize>() {
                            let anim_id = id.references;
                            let frame_index = *index as usize;

                            // Find the avatar that owns this animation
                            if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                                let avatar_id = avatar.id;

                                server_ctx.editing_ctx = PixelEditingContext::AvatarFrame(
                                    avatar_id,
                                    anim_id,
                                    persp_index,
                                    frame_index,
                                );

                                // Open the tile editor dock in editor mode
                                let mut dm = DOCKMANAGER.write().unwrap();
                                dm.set_dock("Tiles".into(), ui, ctx, project, server_ctx);
                                dm.edit_maximize(ui, ctx, project, server_ctx);
                            }
                        }
                    }
                }
            }
            TheEvent::RenderViewClicked(id, coord)
            | TheEvent::RenderViewDragged(id, coord)
            | TheEvent::RenderViewUp(id, coord) => {
                if id.name == "MiniMap" {
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();

                        // Color selected
                        if *SIDEBARMODE.read().unwrap() == SidebarMode::Palette {
                            if !matches!(event, TheEvent::RenderViewDragged(_, _)) {
                                let buffer = render_view.render_buffer_mut();
                                if let Some(col) = buffer.get_pixel(coord.x, coord.y) {
                                    let color = TheColor::from(col);

                                    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                                        widget.set_value(TheValue::Text(color.to_hex()));
                                    }

                                    if let Some(palette_picker) =
                                        ui.get_palette_picker("Palette Picker")
                                    {
                                        if project.palette[palette_picker.index()]
                                            != Some(color.clone())
                                        {
                                            let prev = project.palette.clone();
                                            palette_picker.set_color(color.clone());
                                            redraw = true;
                                            project.palette[palette_picker.index()] = Some(color);

                                            let undo = PaletteUndoAtom::Edit(
                                                prev,
                                                project.palette.clone(),
                                            );
                                            UNDOMANAGER
                                                .write()
                                                .unwrap()
                                                .add_palette_undo(undo, ctx);
                                        }

                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Soft Update Minimap"),
                                            TheValue::Empty,
                                        ));
                                    }

                                    *PALETTE.write().unwrap() = project.palette.clone();
                                    RUSTERIX.write().unwrap().assets.palette =
                                        project.palette.clone();
                                }
                            }

                            return redraw;
                        }

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let width = dim.width as f32;
                            let height = dim.height as f32;

                            if let Some(mut bbox) = region.map.bounding_box() {
                                if let Some(tbbox) = region.map.terrain.compute_bounds() {
                                    let bbox_min = Vec2::new(bbox.x, bbox.y);
                                    let bbox_max = bbox_min + Vec2::new(bbox.z, bbox.w);

                                    let new_min = bbox_min.map2(tbbox.min, f32::min);
                                    let new_max = bbox_max.map2(tbbox.max, f32::max);

                                    bbox.x = new_min.x;
                                    bbox.y = new_min.y;
                                    bbox.z = new_max.x - new_min.x;
                                    bbox.w = new_max.y - new_min.y;
                                }
                                bbox.x -= 0.5;
                                bbox.y -= 0.5;
                                bbox.z += 1.0;
                                bbox.w += 1.0;

                                let scale_x = width / bbox.z;
                                let scale_y = height / bbox.w;

                                let bbox_center_x = bbox.x + bbox.z / 2.0;
                                let bbox_center_y = bbox.y + bbox.w / 2.0;

                                let offset_x = -bbox_center_x * scale_x;
                                let offset_y = bbox_center_y * scale_y;

                                let grid_x = (coord.x as f32 - width / 2.0 - offset_x) / scale_x;
                                let grid_y = (coord.y as f32 - height / 2.0 + offset_y) / scale_y;

                                // If shift is pressed we move the look_at position
                                if ui.shift && server_ctx.editor_view_mode == EditorViewMode::FirstP
                                {
                                    region.editing_look_at_3d = Vec3::new(
                                        grid_x,
                                        region.map.terrain.sample_height_bilinear(grid_x, grid_y),
                                        grid_y,
                                    );
                                } else {
                                    // We move the camera position
                                    server_ctx.center_map_at_grid_pos(
                                        Vec2::new(width, height),
                                        Vec2::new(grid_x, grid_y),
                                        &mut region.map,
                                    );

                                    // let old_editing_pos = region.editing_position_3d;
                                    region.editing_position_3d = Vec3::new(
                                        grid_x,
                                        region.map.terrain.sample_height_bilinear(grid_x, grid_y),
                                        grid_y,
                                    );
                                    //region.editing_look_at_3d +=
                                    //    region.editing_position_3d - old_editing_pos;
                                }
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Soft Update Minimap"),
                                    TheValue::Empty,
                                ));

                                RUSTERIX.write().unwrap().set_dirty();
                            }

                            /*
                            let region_width = region.width * region.grid_size;
                            let region_height = region.height * region.grid_size;

                            let minimap_width = dim.width;
                            let minimap_height = dim.height;

                            let scale_x = region_width as f32 / minimap_width as f32;
                            let scale_y = region_height as f32 / minimap_height as f32;

                            // Calculate the real-world coordinates by applying scaling
                            let real_x = (coord.x as f32 * scale_x).round();
                            let real_y = (coord.y as f32 * scale_y).round();

                            // Converting real-world coordinates to tile indices
                            let tile_x = real_x / region.grid_size as f32;
                            let tile_y = real_y / region.grid_size as f32;

                            server_ctx.curr_character_instance = None;
                            server_ctx.curr_item_instance = None;
                            region.editing_position_3d = vec3f(tile_x, 0.0, tile_y);
                            server.set_editing_position_3d(region.editing_position_3d);
                            server.update_region(region);

                            region.scroll_offset = vec2i(
                                (tile_x * region.grid_size as f32) as i32,
                                (tile_y * region.grid_size as f32) as i32,
                            );

                            if let Some(rgba_layout) = ui.get_rgba_layout("TerrainMap") {
                                rgba_layout.scroll_to(region.scroll_offset);
                            }

                            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                                rgba_layout.scroll_to_grid(vec2i(tile_x as i32, tile_y as i32));
                            }
                            */
                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::Resize => {
                ctx.ui.redraw_all = true;
                self.show_filtered_materials(ui, ctx, project, server_ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
            }
            TheEvent::WidgetResized(id, dim) => {
                if project.regions.is_empty() && id.name == "PolyView" {
                    if let Some(renderview) = ui.get_render_view("PolyView") {
                        if let Some(buffer) = ctx.ui.icon("eldiron") {
                            let scaled_buffer = buffer.scaled(dim.width, dim.height);
                            *renderview.render_buffer_mut() =
                                TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
                            renderview.render_buffer_mut().fill(BLACK);
                            renderview.render_buffer_mut().copy_into(
                                (dim.width - scaled_buffer.dim().width) / 2,
                                (dim.height - scaled_buffer.dim().height) / 2,
                                &scaled_buffer,
                            );
                            renderview.set_needs_redraw(true);
                        }
                    }
                }
            }
            TheEvent::Custom(id, _value) => {
                if id.name == "Editing Texture Updated" {
                    // Update the avatar perspective icon in the Project Tree
                    if let PixelEditingContext::AvatarFrame(
                        avatar_id,
                        anim_id,
                        persp_index,
                        frame_index,
                    ) = server_ctx.editing_ctx
                    {
                        if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx)
                        {
                            let icon_name = format!("Avatar Perspective Icons {}", persp_index);
                            if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                                if let Some(avatars_node) =
                                    tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id)
                                {
                                    // Find the avatar node
                                    if let Some(avatar_node) = avatars_node
                                        .childs
                                        .iter_mut()
                                        .find(|c| c.id.uuid == avatar_id)
                                    {
                                        // Find the animation node
                                        if let Some(anim_node) = avatar_node
                                            .childs
                                            .iter_mut()
                                            .find(|c| c.id.uuid == anim_id)
                                        {
                                            // Find the perspective node containing our icons widget
                                            for persp_node in &mut anim_node.childs {
                                                for widget in &mut persp_node.widgets {
                                                    if widget.id().name == icon_name {
                                                        if let Some(icons) = widget.as_tree_icons()
                                                        {
                                                            icons.set_icon(
                                                                frame_index,
                                                                texture.to_rgba(),
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
                } else if id.name == "Backup Editing Position" {
                    if let Some(region) = project.get_region_ctx(server_ctx) {
                        server_ctx.editing_pos_buffer = Some(region.editing_position_3d);
                    }
                } else if id.name == "Update Action Parameters" {
                    // Update the current action params (if any)
                    if let Some(curr_action_id) = server_ctx.curr_action_id {
                        if let Some(action) = ACTIONLIST
                            .write()
                            .unwrap()
                            .get_action_by_id_mut(curr_action_id)
                        {
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                action.load_params(map);
                            }
                            action.load_params_project(project, server_ctx);
                            self.show_action_toml_params(ui, ctx, server_ctx, action.as_ref());
                        }
                    }
                } else if id.name == "Update Action List" {
                    // Update the current action params (if any)
                    if let Some(curr_action_id) = server_ctx.curr_action_id {
                        if let Some(_action) = ACTIONLIST
                            .write()
                            .unwrap()
                            .get_action_by_id_mut(curr_action_id)
                        {
                            // if let Some(map) = project.get_map_mut(&server_ctx) {
                            //     action.load_params(map);
                            // }
                            // action.load_params_project(project, server_ctx);
                        }
                    }
                    self.show_actions(ui, ctx, project, server_ctx);
                    if server_ctx.curr_action_id.is_none() {
                        self.show_empty_action_toml(ui, ctx);
                    }
                } else if id.name == "Nodegraph Id Changed" {
                    if let Some(map) = project.get_map(server_ctx) {
                        if let Some(widget) = ui.get_widget("Graph Id Text") {
                            // map.shapefx_graphs.gener
                            if let Some(index) = map.shapefx_graphs.get_index_of(&id.uuid) {
                                widget.set_value(TheValue::Text(format!("({index:02})")));
                            } else {
                                widget.set_value(TheValue::Text("(--)".into()));
                            }
                        }
                    }
                } else if id.name == "Update Minimap" {
                    // Rerenders the minimap
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();
                        let buffer = render_view.render_buffer_mut();
                        buffer.resize(dim.width, dim.height);

                        let mut dock_handled_drawing = false;
                        if let Some(dock) = DOCKMANAGER.read().unwrap().get_active_dock() {
                            // Test if dock is drawing minimap
                            if dock.draw_minimap(buffer, project, ctx, server_ctx) {
                                dock_handled_drawing = true;
                            }
                        }

                        if !dock_handled_drawing {
                            if let Some(region) = project.get_region_ctx_mut(&server_ctx) {
                                draw_minimap(region, buffer, server_ctx, true);
                            }
                        }
                    } else {
                    }
                } else if id.name == "Soft Update Minimap" {
                    // Uses the currently rendered minimap and only updates the
                    // camera markers
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();
                        let buffer = render_view.render_buffer_mut();
                        buffer.resize(dim.width, dim.height);

                        let mut dock_handled_drawing = false;
                        if let Some(dock) = DOCKMANAGER.read().unwrap().get_active_dock() {
                            // Test if dock is drawing minimap
                            if dock.draw_minimap(buffer, project, ctx, server_ctx) {
                                dock_handled_drawing = true;
                            }
                        }

                        if !dock_handled_drawing {
                            if let Some(region) = project.get_region_ctx_mut(&server_ctx) {
                                draw_minimap(region, buffer, server_ctx, false);
                            }
                        }
                    }
                } else if id.name == "Update Tiles" {
                    self.update_tiles(ui, ctx, project);
                } else if id.name == "Show Node Settings" {
                    if let Some(tab) = ui.get_layout("Multi Tab") {
                        if let Some(tab) = tab.as_tab_layout() {
                            tab.set_index(1);
                        }
                    }
                } else if id.name == "Update Content List" {
                    if server_ctx.get_map_context() == MapContext::Region {
                        self.apply_region(ui, ctx, Some(server_ctx.curr_region), project);
                    } else if server_ctx.get_map_context() == MapContext::Screen {
                        self.apply_screen(ui, ctx, project.get_screen_ctx(server_ctx));
                    }
                }
            }
            TheEvent::PaletteIndexChanged(id, index) => {
                if id.name == "Palette Picker" {
                    project.palette.current_index = *index;
                    if let Some(widget) = ui.get_widget("Palette Index Text") {
                        widget.set_value(TheValue::Text(format!("{index:03}")));
                    }
                    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                        if let Some(color) = &project.palette[*index as usize] {
                            widget.set_value(TheValue::Text(color.to_hex()));
                        }
                    }
                    // if let Some(widget) = ui.get_widget("Palette Name Edit") {
                    //     if let Some(color) = &project.palette[*index as usize] {
                    //         widget.set_value(TheValue::Text(color.name.clone()));
                    //     }
                    // }
                    *PALETTE.write().unwrap() = project.palette.clone();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Soft Update Minimap"),
                        TheValue::Empty,
                    ));
                }
            }
            TheEvent::DialogValueOnClose(role, name, uuid, value) => {
                if name == "Add Shader To Library" && *role == TheDialogButtonRole::Accept {
                    let mut material = SHADEGRIDFX.read().unwrap().clone();
                    if let Some(routine) = material.get_selected_routine_mut() {
                        let mut routine_clone = routine.clone();
                        routine.id = Uuid::new_v4();
                        routine_clone.name = "shader".to_string();
                        let mut module: Module = Module::as_type(codegridfx::ModuleType::Shader);
                        if let Some(name) = value.to_string() {
                            module.name = name;
                            module.routines.insert(routine.id, routine_clone);
                            server_ctx.curr_material_id = Some(module.id);
                            project.shaders.insert(module.id, module);
                            self.show_filtered_materials(ui, ctx, project, server_ctx);
                            RUSTERIX.write().unwrap().set_dirty();

                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named_with_id(
                                    "Shader Item",
                                    server_ctx.curr_material_id.unwrap(),
                                ),
                                TheWidgetState::Selected,
                            ));
                        }
                    }
                } else if name == "Rename Region" && *role == TheDialogButtonRole::Accept {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.name = value.describe();
                        region.map.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Character" && *role == TheDialogButtonRole::Accept {
                    if crate::utils::is_valid_python_variable(&value.describe()) {
                        if let Some(character) = project.characters.get_mut(uuid) {
                            character.name = value.describe();
                            ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                        }
                    }
                } else if name == "Rename Item" && *role == TheDialogButtonRole::Accept {
                    if crate::utils::is_valid_python_variable(&value.describe()) {
                        if let Some(item) = project.items.get_mut(uuid) {
                            item.name = value.describe();
                            ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                        }
                    }
                }
                /*else if name == "Rename Module" && *role == TheDialogButtonRole::Accept {
                    if let Some(bundle) = project.codes.get_mut(uuid) {
                        bundle.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } */
                else if name == "Rename Screen" && *role == TheDialogButtonRole::Accept {
                    if let Some(screen) = project.screens.get_mut(uuid) {
                        screen.name = value.describe();
                        screen.map.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                }
                /*else if name == "Rename Widget" && *role == TheDialogButtonRole::Accept {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
                                widget.name = value.describe();
                                ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                            }
                        }
                    }
                }*/
                else if name == "Rename Asset" && *role == TheDialogButtonRole::Accept {
                    if let Some(asset) = project.assets.get_mut(uuid) {
                        asset.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Model" && *role == TheDialogButtonRole::Accept {
                    if let Some(model) = project.models.get_mut(uuid) {
                        model.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Shader" && *role == TheDialogButtonRole::Accept {
                    if let Some(material) = project.shaders.get_mut(uuid) {
                        material.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                }
            }
            TheEvent::ContextMenuSelected(widget_id, item_id) => {
                if item_id.name == "Add Image" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(item_id.name.as_str(), Uuid::new_v4()),
                        "Open Image".into(),
                        TheFileExtension::new(
                            "PNG Image".into(),
                            vec!["png".to_string(), "PNG".to_string()],
                        ),
                    );
                } else if item_id.name == "Add Font" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(item_id.name.as_str(), Uuid::new_v4()),
                        "Open Font".into(),
                        TheFileExtension::new(
                            "Font".into(),
                            vec!["ttf".to_string(), "TTF".to_string()],
                        ),
                    );
                } else if item_id.name == "Rename Region" {
                    if let Some(tilemap) = project.get_region(&server_ctx.curr_region) {
                        open_text_dialog(
                            "Rename Region",
                            "Region Name",
                            tilemap.name.as_str(),
                            server_ctx.curr_region,
                            ui,
                            ctx,
                        );
                    }
                }
                /*else if item_id.name == "Rename Module" {
                    if let Some(module) = project.codes.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Module",
                            "Module Name",
                            module.name.as_str(),
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                }*/
                else if item_id.name == "Rename Character" {
                    if let Some(character) = project.characters.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Character",
                            "Character Class",
                            &character.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Rename Item" {
                    if let Some(item) = project.items.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Item",
                            "Item Class",
                            &item.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Rename Screen" {
                    if let Some(screen) = project.screens.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Screen",
                            "Screen Name",
                            &screen.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                }
                /*else if item_id.name == "Rename Widget" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
                                open_text_dialog(
                                    "Rename Widget",
                                    "Widget Name",
                                    &widget.name,
                                    widget_id,
                                    ui,
                                    ctx,
                                );
                            }
                        }
                    }
                }*/
                else if item_id.name == "Rename Asset" {
                    if let Some(asset) = project.assets.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Asset",
                            "Asset Name",
                            &asset.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Rename Shader" {
                    if let Some(material) = project.shaders.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Shader",
                            "Shader Name",
                            &material.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Duplicate Shader" {
                    if let Some(mut material) = project.shaders.get(&widget_id.uuid).cloned() {
                        material.name = format!("Duplicate of {}", material.name);
                        material.id = Uuid::new_v4();
                        project.shaders.insert(material.id, material);
                        self.show_filtered_materials(ui, ctx, project, server_ctx);
                    }
                }
            }
            TheEvent::DragStarted(id, text, offset) => {
                if id.name == "Shader Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Shader: {text}"));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                } else if id.name == "Character Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Character: {text}"));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                } else if id.name == "Item Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Item: {text}"));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                }
            }
            TheEvent::TileDropped(id, _, _) => {
                if let Some(action_id) = server_ctx.curr_action_id
                    && id.name.starts_with("action")
                {
                    if let Some(action) =
                        ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        if action.handle_event(event, project, ui, ctx, server_ctx) {
                            return true;
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Action Params TOML" {
                    if let Some(action_id) = server_ctx.curr_action_id
                        && let Some(source) = value.to_string()
                        && let Some(action) =
                            ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        let mut nodeui = action.params();
                        if apply_toml_to_nodeui(&mut nodeui, &source).is_ok() {
                            for (key, val) in nodeui_to_value_pairs(&nodeui) {
                                let ev = TheEvent::ValueChanged(TheId::named(&key), val);
                                let _ = action.handle_event(&ev, project, ui, ctx, server_ctx);
                            }

                            if server_ctx.auto_action {
                                ctx.ui.send(TheEvent::StateChanged(
                                    TheId::named("Action Apply"),
                                    TheWidgetState::Clicked,
                                ));
                            }
                        }
                    }
                } else if id.name.starts_with("Region Item Name Edit") {
                    // Rename a region
                    let mut old = String::new();
                    if let Some(region) = project.get_region_mut(&id.uuid) {
                        old = region.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameRegion(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Character Item Name Edit") {
                    // Rename a Character
                    let mut old = String::new();
                    if let Some(character) = project.characters.get(&id.uuid) {
                        old = character.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameCharacter(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Item Item Name Edit") {
                    // Rename an item
                    let mut old = String::new();
                    if let Some(item) = project.items.get(&id.uuid) {
                        old = item.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameItem(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Tilemap Item Name Edit") {
                    // Rename a Tilemap
                    let mut old = String::new();
                    if let Some(tilemap) = project.get_tilemap(id.uuid) {
                        old = tilemap.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameTilemap(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Tilemap Item Grid Edit") {
                    // Edit Tilemap Grid Size
                    let mut old = 0;
                    if let Some(tilemap) = project.get_tilemap(id.references) {
                        old = tilemap.grid_size;
                    }

                    if let Some(size) = value.to_i32()
                        && old != size
                    {
                        let atom = ProjectUndoAtom::EditTilemapGridSize(id.references, old, size);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Screen Item Name Edit") {
                    // Rename a Screen
                    let mut old = String::new();
                    if let Some(screen) = project.screens.get(&id.uuid) {
                        old = screen.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameScreen(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Asset Item Name Edit") {
                    // Rename an Asset
                    let mut old = String::new();
                    if let Some(asset) = project.assets.get(&id.uuid) {
                        old = asset.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameAsset(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Avatar Item Name Edit") {
                    // Rename an Avatar
                    let mut old = String::new();
                    if let Some(avatar) = project.avatars.get(&id.uuid) {
                        old = avatar.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameAvatar(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Avatar Item Resolution Edit") {
                    // Change Avatar Resolution
                    if let Some(new_res) = value.to_i32() {
                        let new_res = new_res.max(1) as u16;
                        if let Some(avatar) = project.avatars.get(&id.references) {
                            let old_res = avatar.resolution;
                            if old_res != new_res {
                                let atom = ProjectUndoAtom::EditAvatarResolution(
                                    id.references,
                                    old_res,
                                    new_res,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Avatar Animation Name Edit" {
                    let anim_id = id.references;
                    if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                        let avatar_id = avatar.id;
                        let old = avatar
                            .animations
                            .iter()
                            .find(|a| a.id == anim_id)
                            .map(|a| a.name.clone())
                            .unwrap_or_default();
                        if let Some(name) = value.to_string() {
                            if old != name {
                                let atom = ProjectUndoAtom::RenameAvatarAnimation(
                                    avatar_id, anim_id, old, name,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Avatar Animation Frame Count Edit" {
                    let anim_id = id.references;
                    if let Some(new_count) = value.to_i32() {
                        let new_count = (new_count.max(1)) as usize;
                        if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                            let avatar_id = avatar.id;
                            let old_count = avatar.get_animation_frame_count(&anim_id);
                            if old_count != new_count {
                                let atom = ProjectUndoAtom::EditAvatarAnimationFrameCount(
                                    avatar_id, anim_id, old_count, new_count,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Avatar Animation Speed Edit" {
                    let anim_id = id.references;
                    if let Some(new_speed) = value.to_f32() {
                        let new_speed = new_speed.clamp(0.01, 100.0);
                        if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                            let avatar_id = avatar.id;
                            let old_speed = avatar
                                .animations
                                .iter()
                                .find(|a| a.id == anim_id)
                                .map(|a| a.speed)
                                .unwrap_or(1.0);
                            if (old_speed - new_speed).abs() > f32::EPSILON {
                                let atom = ProjectUndoAtom::EditAvatarAnimationSpeed(
                                    avatar_id, anim_id, old_speed, new_speed,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if let Some(action_id) = server_ctx.curr_action_id
                    && id.name.starts_with("action")
                {
                    if let Some(action) =
                        ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        if action.handle_event(event, project, ui, ctx, server_ctx) {
                            if server_ctx.auto_action {
                                ctx.ui.send(TheEvent::StateChanged(
                                    TheId::named("Action Apply"),
                                    TheWidgetState::Clicked,
                                ));
                            }
                            return true;
                        }
                    }
                }

                if id.name == "RegionConfigEdit" {
                    if let Some(code) = value.to_string() {
                        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                            apply_region_config(&mut region.map, code.clone());
                            region.config = code;
                        }
                    }
                }
                if id.name == "Palette Hex Edit" {
                    if let Some(hex) = value.to_string() {
                        let color = TheColor::from_hex(&hex);

                        if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                            if project.palette[palette_picker.index()] != Some(color.clone()) {
                                let prev = project.palette.clone();

                                palette_picker.set_color(color.clone());
                                redraw = true;
                                project.palette[palette_picker.index()] = Some(color.clone());
                                let undo = PaletteUndoAtom::Edit(prev, project.palette.clone());
                                UNDOMANAGER.write().unwrap().add_palette_undo(undo, ctx);

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Soft Update Minimap"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }
                    *PALETTE.write().unwrap() = project.palette.clone();
                    RUSTERIX.write().unwrap().assets.palette = project.palette.clone();
                } else if id.name == "Tilemap Filter Edit" || id.name == "Tilemap Filter Role" {
                    if let Some(id) = self.curr_tilemap_uuid {
                        self.show_filtered_tiles(ui, ctx, project.get_tilemap(id).as_deref())
                    }
                } else if id.name == "Shader Filter Edit" {
                    self.show_filtered_materials(ui, ctx, project, server_ctx);
                } else if id.name == "Tilemap Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(layout) = ui.get_rgba_layout("Tilemap Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                        if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                            if let Some(tilemap) = project.get_tilemap_mut(curr_tilemap_uuid) {
                                tilemap.zoom = v;
                            }
                        }
                    }
                } else if id.name == "Region Content Filter Edit"
                    || id.name == "Region Content Dropdown"
                {
                    self.apply_region(ui, ctx, Some(server_ctx.curr_region), project);
                }
            }
            TheEvent::FileRequesterResult(id, paths) => {
                if let Some(action_id) = server_ctx.curr_action_id
                    && id.name.starts_with("action")
                {
                    if let Some(action) =
                        ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        if action.handle_event(event, project, ui, ctx, server_ctx) {
                            return true;
                        }
                    }
                } else if id.name == "Tilemap Add"
                    || id.name == "Add Tileset"
                    || id.name == "Add Image"
                {
                    for p in paths {
                        ctx.ui.decode_image(id.clone(), p.clone());
                    }
                } else if id.name == "Add Font Asset" || id.name == "Add Font" {
                    for p in paths {
                        if let Ok(bytes) = std::fs::read(p) {
                            if fontdue::Font::from_bytes(
                                bytes.clone(),
                                fontdue::FontSettings::default(),
                            )
                            .is_ok()
                            {
                                let asset = Asset {
                                    name: p
                                        .file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    id: Uuid::new_v4(),
                                    buffer: AssetBuffer::Font(bytes),
                                };

                                let atom = ProjectUndoAtom::AddAsset(asset);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Add Font Old" {
                    for p in paths {
                        if let Ok(bytes) = std::fs::read(p) {
                            if fontdue::Font::from_bytes(
                                bytes.clone(),
                                fontdue::FontSettings::default(),
                            )
                            .is_ok()
                            {
                                let asset = Asset {
                                    name: if let Some(n) = p.file_stem() {
                                        n.to_string_lossy().to_string()
                                    } else {
                                        "Font".to_string()
                                    },
                                    buffer: AssetBuffer::Font(bytes),
                                    ..Asset::default()
                                };

                                if let Some(layout) =
                                    ui.canvas.get_layout(Some(&"Asset List".to_string()), None)
                                {
                                    if let Some(list_layout) = layout.as_list_layout() {
                                        let mut item = TheListItem::new(TheId::named_with_id(
                                            "Asset Item",
                                            asset.id,
                                        ));
                                        item.set_text(asset.name.clone());
                                        item.set_state(TheWidgetState::Selected);
                                        item.set_context_menu(Some(TheContextMenu {
                                            items: vec![TheContextMenuItem::new(
                                                "Rename Asset...".to_string(),
                                                TheId::named("Rename Asset"),
                                            )],
                                            ..Default::default()
                                        }));
                                        item.add_value_column(
                                            100,
                                            TheValue::Text("Font".to_string()),
                                        );
                                        list_layout.deselect_all();
                                        let id = item.id().clone();
                                        list_layout.add_item(item, ctx);
                                        ctx.ui.send_widget_state_changed(
                                            &id,
                                            TheWidgetState::Selected,
                                        );

                                        redraw = true;
                                    }
                                }
                                project.add_asset(asset);
                            }
                        }
                    }
                } else if id.name == "Shader Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut module: Module =
                            serde_json::from_str(&contents).unwrap_or(Module::default());
                        module.id = Uuid::new_v4();
                        if module.name.is_empty() {
                            module.name = "Unnamed".into();
                        }

                        project.shaders.insert(module.id, module);
                        self.show_filtered_materials(ui, ctx, project, server_ctx);

                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Shader loaded successfully.".to_string(),
                        ))
                    }
                } else if id.name == "Shader Export" {
                    if let Some(curr_material_id) = server_ctx.curr_material_id {
                        if let Some(material) = project.shaders.get(&curr_material_id) {
                            for p in paths {
                                let json = serde_json::to_string(&material);
                                if let Ok(json) = json {
                                    if std::fs::write(p, json).is_ok() {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Shader saved successfully.".to_string(),
                                        ))
                                    } else {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Unable to save Material!".to_string(),
                                        ))
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "Region Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut region: Region =
                            serde_json::from_str(&contents).unwrap_or(Region::default());

                        region.id = Uuid::new_v4();
                        region.map.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddRegion(region);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Region Export" {
                    if let Some(region) = project.get_region(&id.uuid) {
                        let mut region = region.clone();
                        for p in paths {
                            region.id = Uuid::new_v4();
                            region.map.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&region) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Region saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Region!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Character Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut character: Character =
                            serde_json::from_str(&contents).unwrap_or(Character::default());

                        character.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddCharacter(character);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Avatar Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut avatar: Avatar =
                            serde_json::from_str(&contents).unwrap_or(Avatar::default());

                        avatar.id = Uuid::new_v4();
                        for animation in &mut avatar.animations {
                            animation.id = Uuid::new_v4();
                        }

                        let atom = ProjectUndoAtom::AddAvatar(avatar);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Character Export" {
                    if let Some(character) = project.characters.get(&id.uuid) {
                        let mut character = character.clone();
                        for p in paths {
                            character.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&character) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Character saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Character!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Avatar Export" {
                    if let Some(avatar) = project.avatars.get(&id.uuid) {
                        let mut avatar = avatar.clone();
                        for p in paths {
                            avatar.id = Uuid::new_v4();
                            for animation in &mut avatar.animations {
                                animation.id = Uuid::new_v4();
                            }

                            if let Ok(json) = serde_json::to_string(&avatar) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Avatar saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Avatar!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Item Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut item: Item =
                            serde_json::from_str(&contents).unwrap_or(Item::default());

                        item.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddItem(item);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Item Export" {
                    if let Some(item) = project.items.get(&id.uuid) {
                        let mut item = item.clone();
                        for p in paths {
                            item.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&item) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Item saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Item!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Tileset Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut tilemap: Tilemap =
                            serde_json::from_str(&contents).unwrap_or(Tilemap::default());

                        tilemap.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddTilemap(tilemap);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Tileset Export" {
                    if let Some(tilemap) = project.get_tilemap(id.uuid) {
                        let mut tilemap = tilemap.clone();
                        for p in paths {
                            tilemap.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&tilemap) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Tileset saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Tileset!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Screen Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut screen: Screen =
                            serde_json::from_str(&contents).unwrap_or(Screen::default());

                        screen.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddScreen(screen);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Screen Export" {
                    if let Some(screen) = project.screens.get(&id.uuid) {
                        let mut screen = screen.clone();
                        for p in paths {
                            screen.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&screen) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Screen saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Screen!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Font Asset Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut asset: Asset =
                            serde_json::from_str(&contents).unwrap_or(Asset::default());

                        asset.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddAsset(asset);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Font Asset Export" {
                    if let Some(asset) = project.assets.get(&id.uuid) {
                        let mut asset = asset.clone();
                        for p in paths {
                            asset.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&asset) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Font Asset saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Font Asset!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::ImageDecodeResult(id, name, buffer) => {
                if id.name == "Add Image" {
                    let asset = Asset {
                        name: name.clone(),
                        id: Uuid::new_v4(),
                        buffer: AssetBuffer::Image(buffer.clone()),
                    };

                    if let Some(layout) =
                        ui.canvas.get_layout(Some(&"Asset List".to_string()), None)
                    {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item =
                                TheListItem::new(TheId::named_with_id("Asset Item", asset.id));
                            item.set_text(name.clone());
                            item.set_state(TheWidgetState::Selected);
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Rename Asset...".to_string(),
                                    TheId::named("Rename Asset"),
                                )],
                                ..Default::default()
                            }));
                            item.add_value_column(100, TheValue::Text("Image".to_string()));
                            list_layout.deselect_all();
                            let id = item.id().clone();
                            list_layout.add_item(item, ctx);
                            ctx.ui
                                .send_widget_state_changed(&id, TheWidgetState::Selected);

                            redraw = true;
                        }
                    }
                    project.add_asset(asset);
                } else if id.name == "Tilemap Add" || id.name == "Add Tileset" {
                    let mut tilemap = Tilemap::new();
                    tilemap.name = name.clone();
                    tilemap.id = Uuid::new_v4();
                    tilemap.buffer = buffer.clone();

                    // Use undo system to add tilemap
                    let atom = ProjectUndoAtom::AddTilemap(tilemap);
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);

                    redraw = true;
                }
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                let action_list = ACTIONLIST.write().unwrap();
                let mut needs_scene_redraw: bool = false;
                for action in &action_list.actions {
                    if let Some(accel) = action.accel() {
                        if accel.matches(ui.shift, ui.ctrl, ui.alt, ui.logo, *c) {
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                if action.is_applicable(map, ctx, server_ctx) {
                                    println!("{}", action.id().name);
                                    needs_scene_redraw =
                                        self.apply_action(action, map, ui, ctx, server_ctx, true);
                                }
                            }
                            action.apply_project(project, ui, ctx, server_ctx);
                        }
                    }
                }
                if needs_scene_redraw {
                    crate::utils::scenemanager_render_map(project, server_ctx);
                    TOOLLIST
                        .write()
                        .unwrap()
                        .update_geometry_overlay_3d(project, server_ctx);
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Action Auto" {
                    server_ctx.auto_action = *state == TheWidgetState::Selected;
                } else
                // Iterate actions
                if let Some(action) =
                    ACTIONLIST.write().unwrap().get_action_by_id_mut(id.uuid)
                {
                    if server_ctx.help_mode {
                        let mut name = id.name.to_lowercase().trim().to_string();
                        name = name.replace(" ", "-".into());
                        let url = format!("docs/creator/actions/#{}", name);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Show Help"),
                            TheValue::Text(url),
                        ));
                        return true;
                    }

                    server_ctx.curr_action_id = Some(action.id().uuid);

                    //layout.clear();
                    // if let Some(node) = layout.get_node_by_id_mut(&server_ctx.tree_settings_id) {
                    //     if let Some(action_id) = server_ctx.curr_action_id {
                    //         if let Some(action) = ACTIONLIST.read().unwrap().get_action_by_id(action_id) {
                    //             let nodeui = action.params();
                    //             // nodeui.apply_to_text_layout(layout);
                    //             nodeui.apply_to_tree_node(node);
                    //             ctx.ui.relayout = true;

                    if let Some(map) = project.get_map_mut(&server_ctx) {
                        action.load_params(map);
                    }
                    action.load_params_project(project, server_ctx);
                    self.show_action_toml_params(ui, ctx, server_ctx, action.as_ref());

                    if server_ctx.auto_action {
                        ctx.ui.send(TheEvent::StateChanged(
                            TheId::named("Action Apply"),
                            TheWidgetState::None,
                        ));
                    }
                } else if id.name == "Action Apply" {
                    if let Some(action_id) = server_ctx.curr_action_id {
                        if let Some(action) = ACTIONLIST.read().unwrap().get_action_by_id(action_id)
                        {
                            let mut needs_scene_redraw = false;
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                needs_scene_redraw = self.apply_action(
                                    action,
                                    map,
                                    ui,
                                    ctx,
                                    server_ctx,
                                    !(*state == TheWidgetState::None),
                                );
                            }
                            action.apply_project(project, ui, ctx, server_ctx);

                            if needs_scene_redraw {
                                crate::utils::scenemanager_render_map(project, server_ctx);
                                TOOLLIST
                                    .write()
                                    .unwrap()
                                    .update_geometry_overlay_3d(project, server_ctx);
                            }
                        }
                    }
                } else if id.name == "Shader Item" {
                    let material_id = id.uuid;
                    server_ctx.curr_material_id = Some(material_id);
                    if let Some(material) = project.shaders.get(&id.uuid) {
                        let prev = SHADEGRIDFX.read().unwrap().clone();

                        CODEEDITOR
                            .write()
                            .unwrap()
                            .set_shader_material(ui, ctx, material);

                        let atom = MaterialUndoAtom::ShaderEdit(prev, material.clone());
                        UNDOMANAGER.write().unwrap().add_material_undo(atom, ctx);
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimap"),
                        TheValue::Empty,
                    ));
                } else if id.name == "Palette Clear" {
                    let prev = project.palette.clone();
                    project.palette.clear();
                    if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                        let index = palette_picker.index();

                        palette_picker.set_palette(project.palette.clone());
                        if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                            if let Some(color) = &project.palette[index] {
                                widget.set_value(TheValue::Text(color.to_hex()));
                            }
                        }
                    }
                    redraw = true;

                    let undo = PaletteUndoAtom::Edit(prev, project.palette.clone());
                    UNDOMANAGER.write().unwrap().add_palette_undo(undo, ctx);
                } else if id.name == "Palette Import" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "Palette (*.txt)".into(),
                            vec!["txt".to_string(), "TXT".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Tilemap Import" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "Eldiron Tilemap".into(),
                            vec!["eldiron_tilemap".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Tilemap Export" {
                    if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                        if let Some(tilemap) = project.get_tilemap(curr_tilemap_uuid) {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id(id.name.as_str(), tilemap.id),
                                "Save".into(),
                                TheFileExtension::new(
                                    "Eldiron Tilemap".into(),
                                    vec!["eldiron_tilemap".to_string()],
                                ),
                            );
                            ctx.ui
                                .set_widget_state("Save As".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }
                    }
                } else if id.name == "Add Region" {
                    // Add Region
                    let mut region = Region::default();
                    if let Some(bytes) = crate::Embedded::get("toml/region.toml") {
                        if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                            region.config = source.to_string();
                        }
                    }
                    let atom = ProjectUndoAtom::AddRegion(region);
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Import Region" {
                    if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_region() {
                            ctx.ui.open_file_requester(
                                TheId::named_with_id("Region Import", id),
                                "Import Region".into(),
                                TheFileExtension::new(
                                    "Eldiron Region".into(),
                                    vec!["eldiron_region".to_string()],
                                ),
                            );
                        }
                    }
                } else if id.name == "Add Character" {
                    // Add Character
                    let atom = ProjectUndoAtom::AddCharacter(Character::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Import Character" {
                    if let Some(id) = server_ctx.pc.id() {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id("Character Import", id),
                            "Import Character".into(),
                            TheFileExtension::new(
                                "Eldiron Character".into(),
                                vec!["eldiron_character".to_string()],
                            ),
                        );
                    }
                } else if id.name == "Import Avatar" {
                    if let Some(id) = server_ctx.pc.id() {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id("Avatar Import", id),
                            "Import Avatar".into(),
                            TheFileExtension::new(
                                "Eldiron Avatar".into(),
                                vec!["eldiron_avatar".to_string()],
                            ),
                        );
                    }
                } else if id.name == "Add Item" {
                    // Add Item
                    let atom = ProjectUndoAtom::AddItem(Item::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Add Tileset" {
                    // Add Tileset - open PNG file requester
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Add Tileset", Uuid::new_v4()),
                        "Open PNG Image".into(),
                        TheFileExtension::new(
                            "PNG Image".into(),
                            vec!["png".to_string(), "PNG".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("Add Tileset".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Add Screen" {
                    // Add Screen
                    let atom = ProjectUndoAtom::AddScreen(Screen::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Add Font Asset" {
                    // Add Font Asset - open font file requester
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Add Font Asset", Uuid::new_v4()),
                        "Open Font File".into(),
                        TheFileExtension::new(
                            "Font File".into(),
                            vec!["ttf".to_string(), "otf".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("Add Font Asset".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Add Avatar" {
                    // Add Avatar
                    let atom = ProjectUndoAtom::AddAvatar(Avatar::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Import Item" {
                    if let Some(id) = server_ctx.pc.id() {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id("Item Import", id),
                            "Import Item".into(),
                            TheFileExtension::new(
                                "Eldiron Item".into(),
                                vec!["eldiron_item".to_string()],
                            ),
                        );
                    }
                } else if id.name == "Import Tileset" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Tileset Import", Uuid::new_v4()),
                        "Import Tileset".into(),
                        TheFileExtension::new(
                            "Eldiron Tileset".into(),
                            vec!["eldiron_tileset".to_string()],
                        ),
                    );
                } else if id.name == "Import Screen" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Screen Import", Uuid::new_v4()),
                        "Import Screen".into(),
                        TheFileExtension::new(
                            "Eldiron Screen".into(),
                            vec!["eldiron_screen".to_string()],
                        ),
                    );
                } else if id.name == "Import Font Asset" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Font Asset Import", Uuid::new_v4()),
                        "Import Font Asset".into(),
                        TheFileExtension::new(
                            "Eldiron Font Asset".into(),
                            vec!["eldiron_font_asset".to_string()],
                        ),
                    );
                } else if id.name == "Project Remove" {
                    if server_ctx.pc.is_region() {
                        if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id()
                        {
                            // This is a character instance in the region

                            let mut character = Character::default();
                            let mut index = 0;

                            if let Some(r) = project.get_region_ctx(server_ctx) {
                                if let Some(ind) = r.characters.get_index_of(&instance_id) {
                                    index = ind;
                                }
                                if let Some(char) = r.characters.get(&instance_id) {
                                    character = char.clone();
                                }
                            }

                            let atom = ProjectUndoAtom::RemoveRegionCharacterInstance(
                                index,
                                server_ctx.curr_region,
                                character,
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        } else if let Some(instance_id) =
                            server_ctx.pc.get_region_item_instance_id()
                        {
                            // This is a item instance in the region

                            let mut item = Item::default();
                            let mut index = 0;

                            if let Some(r) = project.get_region_ctx(server_ctx) {
                                if let Some(ind) = r.items.get_index_of(&instance_id) {
                                    index = ind;
                                }
                                if let Some(it) = r.items.get(&instance_id) {
                                    item = it.clone();
                                }
                            }

                            let atom = ProjectUndoAtom::RemoveRegionItemInstance(
                                index,
                                server_ctx.curr_region,
                                item,
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        } else {
                            // Remove Region
                            let mut region = Region::default();
                            if let Some(r) = project.get_region_ctx(server_ctx) {
                                region = r.clone();
                            }

                            if let Some(index) =
                                project.regions.iter().position(|r| r.id == region.id)
                            {
                                let atom = ProjectUndoAtom::RemoveRegion(index, region);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_character() {
                        // Remove Character
                        let mut character: Character = Character::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(c) = project.characters.get(&id) {
                                character = c.clone();
                            }

                            if let Some(index) = project.characters.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveCharacter(index, character);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if let ProjectContext::AvatarAnimation(avatar_id, anim_id, _) =
                        server_ctx.pc
                    {
                        // Remove Avatar Animation
                        if let Some(avatar) = project.avatars.get(&avatar_id)
                            && let Some(index) =
                                avatar.animations.iter().position(|anim| anim.id == anim_id)
                            && let Some(animation) = avatar.animations.get(index)
                        {
                            let atom = ProjectUndoAtom::RemoveAvatarAnimation(
                                avatar_id,
                                index,
                                animation.clone(),
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        }
                    } else if let ProjectContext::Avatar(avatar_id) = server_ctx.pc {
                        // Remove Avatar
                        if let Some(avatar) = project.avatars.get(&avatar_id).cloned()
                            && let Some(index) = project.avatars.get_index_of(&avatar_id)
                        {
                            let atom = ProjectUndoAtom::RemoveAvatar(index, avatar);
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        }
                    } else if server_ctx.pc.is_item() {
                        // Remove Item
                        let mut item: Item = Item::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(c) = project.items.get(&id) {
                                item = c.clone();
                            }

                            if let Some(index) = project.items.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveItem(index, item);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_tilemap() {
                        // Remove Tilemap
                        let mut tilemap: Tilemap = Tilemap::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(t) = project.get_tilemap(id) {
                                tilemap = t.clone();
                            }

                            if let Some(index) =
                                project.tilemaps.iter().position(|r| r.id == tilemap.id)
                            {
                                let atom = ProjectUndoAtom::RemoveTilemap(index, tilemap);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_screen() {
                        // Remove Screen
                        let mut screen: Screen = Screen::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(s) = project.screens.get(&id) {
                                screen = s.clone();
                            }

                            if let Some(index) = project.screens.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveScreen(index, screen);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_asset() {
                        // Remove Asset
                        let mut asset: Asset = Asset::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(a) = project.assets.get(&id) {
                                asset = a.clone();
                            }

                            if let Some(index) = project.assets.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveAsset(index, asset);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Project Export" {
                    if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_region() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Region Export", id),
                                "Export Region".into(),
                                TheFileExtension::new(
                                    "Eldiron Region".into(),
                                    vec!["eldiron_region".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_character() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Character Export", id),
                                "Export Character".into(),
                                TheFileExtension::new(
                                    "Eldiron Character".into(),
                                    vec!["eldiron_character".to_string()],
                                ),
                            );
                        } else if matches!(
                            server_ctx.pc,
                            ProjectContext::Avatar(_) | ProjectContext::AvatarAnimation(_, _, _)
                        ) {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Avatar Export", id),
                                "Export Avatar".into(),
                                TheFileExtension::new(
                                    "Eldiron Avatar".into(),
                                    vec!["eldiron_avatar".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_item() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Item Export", id),
                                "Export Item".into(),
                                TheFileExtension::new(
                                    "Eldiron Item".into(),
                                    vec!["eldiron_item".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_tilemap() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Tileset Export", id),
                                "Export Tileset".into(),
                                TheFileExtension::new(
                                    "Eldiron Tileset".into(),
                                    vec!["eldiron_tileset".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_screen() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Screen Export", id),
                                "Export Screen".into(),
                                TheFileExtension::new(
                                    "Eldiron Screen".into(),
                                    vec!["eldiron_screen".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_asset() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Font Asset Export", id),
                                "Export Font Asset".into(),
                                TheFileExtension::new(
                                    "Eldiron Font Asset".into(),
                                    vec!["eldiron_font_asset".to_string()],
                                ),
                            );
                        }
                    }
                } else if id.name == "Region Item" {
                    server_ctx.editing_pos_buffer = None;
                    server_ctx.curr_region = id.references;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Region(id.references),
                    );
                    let _ = crate::utils::update_region_settings(project, server_ctx);
                    self.apply_region(ui, ctx, Some(id.references), project);
                    redraw = true;
                } else if id.name == "Region Settings Item" {
                    server_ctx.editing_pos_buffer = None;
                    server_ctx.curr_region = id.references;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::RegionSettings(id.references),
                    );
                    let _ = crate::utils::update_region_settings(project, server_ctx);
                    self.apply_region(ui, ctx, Some(id.references), project);
                    redraw = true;
                } else if id.name == "Character Item" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.uuid);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Name Edit" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Visual Code Edit" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::CharacterVisualCode(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Code Edit" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::CharacterCode(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Data Edit" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::CharacterData(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Preview Rigging Edit" {
                    if let Some(character) = project.characters.get_mut(&id.references) {
                        if character.preview_rigging.trim().is_empty() {
                            character.preview_rigging = default_preview_rigging_toml();
                        }
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::CharacterPreviewRigging(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Item Item" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_item = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.uuid);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Item(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Item Item Name Edit" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_item = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Item(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Tilemap Item Name Edit" {
                    if let Some(_tilemap) = project.get_tilemap(id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Tilemap Item Code Edit" || id.name == "Tilemap Item Grid Edit"
                {
                    if let Some(_tilemap) = project.get_tilemap(id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Screen Item" {
                    if let Some(_screen) = project.screens.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Screen Item Name Edit" {
                    if let Some(_screen) = project.screens.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Asset Item" {
                    if let Some(_screen) = project.assets.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Asset Item Name Edit" {
                    if let Some(_screen) = project.assets.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Avatar Item"
                    || id.name == "Avatar Item Name Edit"
                    || id.name == "Avatar Item Resolution Edit"
                {
                    if let Some(_) = project.avatars.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Avatar(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Avatar Animation Item"
                    || id.name == "Avatar Animation Name Edit"
                    || id.name == "Avatar Animation Frame Count Edit"
                    || id.name == "Avatar Animation Speed Edit"
                {
                    let anim_id = id.references;
                    if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::AvatarAnimation(avatar.id, anim_id, 0),
                        );
                        redraw = true;
                    }
                } else if id.name == "Avatar Add Animation" {
                    if let Some(avatar) = project.avatars.get(&id.references) {
                        let resolution = avatar.resolution as usize;
                        let directions: Vec<AvatarDirection> = match avatar.perspective_count {
                            AvatarPerspectiveCount::One => vec![AvatarDirection::Front],
                            AvatarPerspectiveCount::Four => vec![
                                AvatarDirection::Front,
                                AvatarDirection::Back,
                                AvatarDirection::Left,
                                AvatarDirection::Right,
                            ],
                        };
                        let mut anim = AvatarAnimation::default();
                        anim.perspectives = directions
                            .into_iter()
                            .map(|dir| AvatarPerspective {
                                direction: dir,
                                frames: vec![AvatarAnimationFrame::new(Texture::new(
                                    vec![0; resolution * resolution * 4],
                                    resolution,
                                    resolution,
                                ))],
                                weapon_main_anchor: None,
                                weapon_off_anchor: None,
                            })
                            .collect();
                        let atom = ProjectUndoAtom::AddAvatarAnimation(id.references, anim);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        redraw = true;
                    }
                } else if id.name == "Item Item Visual Code Edit" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_character = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ItemVisualCode(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Item Item Code Edit" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_character = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ItemCode(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Item Item Data Edit" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_character = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ItemData(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Project Settings" {
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::ProjectSettings,
                    );
                    redraw = true;
                } else if id.name == "Debug Log" {
                    set_project_context(ctx, ui, project, server_ctx, ProjectContext::DebugLog);
                    redraw = true;
                } else if id.name == "Shader Add" {
                    let mut module: Module = Module::as_type(codegridfx::ModuleType::Shader);
                    module.update_routines();
                    module.name = "New Shader".into();
                    server_ctx.curr_material_id = Some(module.id);
                    project.shaders.insert(module.id, module);
                    self.show_filtered_materials(ui, ctx, project, server_ctx);
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::StateChanged(
                        TheId::named_with_id("Shader Item", server_ctx.curr_material_id.unwrap()),
                        TheWidgetState::Selected,
                    ));
                } else if id.name == "Shader Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Shader List") {
                        if let Some(curr_material) = server_ctx.curr_material_id {
                            project.shaders.shift_remove(&curr_material);
                            list_layout.select_first_item(ctx);
                        }
                    }
                    self.show_filtered_materials(ui, ctx, project, &server_ctx);
                } else if id.name == "Shader Import" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "Eldiron Material".into(),
                            vec!["eldiron_shader".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                }
                if id.name == "Shader Export" {
                    if let Some(curr_tilemap_uuid) = server_ctx.curr_material_id {
                        ctx.ui.save_file_requester(
                            TheId::named_with_id(id.name.as_str(), curr_tilemap_uuid),
                            "Save".into(),
                            TheFileExtension::new(
                                "Eldiron Material".into(),
                                vec!["eldiron_shader".to_string()],
                            ),
                        );
                        ctx.ui
                            .set_widget_state("Save As".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    }
                } else if id.name == "Tileset Item" {
                    // Display the tileset editor
                    if let Some(t) = project.get_tilemap(id.references) {
                        self.curr_tilemap_uuid = Some(t.id);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.references),
                        );
                    }
                    redraw = true;
                } else if id.name == "Screen Item" {
                    if let Some(s) = project.screens.get(&id.uuid) {
                        self.apply_screen(ui, ctx, Some(s));
                        server_ctx.curr_screen = id.uuid;
                        redraw = true;
                        RUSTERIX.write().unwrap().set_dirty();
                    }
                } else if id.name == "Screen Add" {
                    if let Some(list_layout) = ui.get_list_layout("Screen List") {
                        let screen = Screen::default();

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Screen Item", screen.id));
                        item.set_text(screen.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Screen...".to_string(),
                                TheId::named("Rename Screen"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_screen(ui, ctx, Some(&screen));
                        project.add_screen(screen);
                    }
                } else if id.name == "Screen Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Screen List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_screen(&selected.uuid);
                            self.apply_screen(ui, ctx, None);
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Apply th given project to the UI
    pub fn load_from_project(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        _ = RUSTERIX
            .write()
            .unwrap()
            .scene_handler
            .settings
            .read(&project.config);

        // If no colors we load the duel palette: https://lospec.com/palette-list/duel
        if project.palette.is_empty() {
            if let Some(bytes) = crate::Embedded::get("duel.txt") {
                if let Ok(txt) = std::str::from_utf8(bytes.data.as_ref()) {
                    project.palette.load_from_txt(txt.to_string());
                }
            }
        }

        self.apply_regions(ui, ctx, server_ctx, project);
        self.apply_characters(ui, ctx, server_ctx, project);
        self.apply_items(ui, ctx, server_ctx, project);
        self.apply_tilemaps(ui, ctx, server_ctx, project);
        self.apply_screens(ui, ctx, server_ctx, project);
        self.apply_assets(ui, ctx, server_ctx, project);
        apply_palette(ui, ctx, server_ctx, project);
        self.apply_screen(ui, ctx, None);
        self.apply_avatars(ui, ctx, server_ctx, project);

        if let Some(list_layout) = ui.get_list_layout("Screen List") {
            list_layout.clear();
            let list = project.sorted_screens_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Screen Item", id));
                item.set_text(name);
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Screen...".to_string(),
                        TheId::named("Rename Screen"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }
        if let Some(list_layout) = ui.get_list_layout("Asset List") {
            list_layout.clear();
            let list = project.sorted_assets_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Asset Item", id));
                item.set_text(name);
                if let Some(asset) = project.assets.get(&id) {
                    let text = asset.buffer.clone().to_string().to_string();
                    item.add_value_column(100, TheValue::Text(text));
                }
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Asset...".to_string(),
                        TheId::named("Rename Asset"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }

        // Adjust Palette and Color Picker
        if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
            palette_picker.set_palette(project.palette.clone());
            let index = palette_picker.index();

            if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                if let Some(color) = &project.palette[index] {
                    widget.set_value(TheValue::Text(color.to_hex()));
                }
            }
        }

        // ui.select_first_list_item("Region List", ctx);
        // ui.select_first_list_item("Character List", ctx);
        // ui.select_first_list_item("Item List", ctx);
        // ui.select_first_list_item("Tilemap List", ctx);
        // ui.select_first_list_item("Module List", ctx);
        // ui.select_first_list_item("Screen List", ctx);
        // ui.select_first_list_item("Asset List", ctx);

        // ui.set_widget_value("ConfigEdit", ctx, TheValue::Text(project.config.clone()));
        if let Ok(toml) = project.config.parse::<Table>() {
            *CONFIG.write().unwrap() = toml;
        }
        CONFIGEDITOR.write().unwrap().read_defaults();
        RUSTERIX.write().unwrap().assets.palette = project.palette.clone();

        // ctx.ui.send(TheEvent::Custom(
        //     TheId::named("Update Tilepicker"),
        //     TheValue::Empty,
        // ));

        // ctx.ui.send(TheEvent::Custom(
        //     TheId::named("Update Materialpicker"),
        //     TheValue::Empty,
        // ));

        // Set the current material
        let selected_material = if project.shaders.is_empty() {
            None
        } else if let Some((id, _)) = project.shaders.get_index(0) {
            Some(*id)
        } else {
            None
        };

        server_ctx.curr_material_id = selected_material;

        self.show_actions(ui, ctx, project, server_ctx);
        // self.show_filtered_materials(ui, ctx, project, server_ctx);
        self.update_tiles(ui, ctx, project);

        TOOLLIST.write().unwrap().get_current_tool().tool_event(
            ToolEvent::Activate,
            ui,
            ctx,
            project,
            server_ctx,
        );
    }

    /// Apply the given screen to the UI
    pub fn apply_screen(&mut self, ui: &mut TheUI, ctx: &mut TheContext, screen: Option<&Screen>) {
        ui.set_widget_disabled_state("Screen Remove", ctx, screen.is_none());
        ui.set_widget_disabled_state("Screen Settings", ctx, screen.is_none());

        if screen.is_none() {
            ui.set_widget_disabled_state("Widget Add", ctx, true);
            ui.set_widget_disabled_state("Widget Remove", ctx, true);

            if let Some(zoom) = ui.get_widget("Screen Editor Zoom") {
                zoom.set_value(TheValue::Float(1.0));
            }

            if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Screen Editor".into()), None) {
                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                    if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                        rgba_view.set_mode(TheRGBAViewMode::Display);
                        rgba_view.set_zoom(1.0);
                        if let Some(buffer) = ctx.ui.icon("eldiron_map") {
                            rgba_view.set_buffer(buffer.clone());
                        }
                        rgba_view.set_grid(None);
                        ctx.ui.relayout = true;
                    }
                    rgba_layout.scroll_to(Vec2::new(0, 0));
                }
            }
        }

        // if let Some(screen) = screen {
        // ui.set_widget_disabled_state("Widget Add", ctx, false);
        // if !screen.widget_list.is_empty() {
        //     ui.set_widget_disabled_state("Widget Remove", ctx, false);
        // }

        // if let Some(zoom) = ui.get_widget("Screen Editor Zoom") {
        //zoom.set_value(TheValue::Float(screen.zoom));
        // }
        // if let Some(rgba_layout) = ui.get_rgba_layout("Screen Editor") {
        //     if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
        //         //rgba.set_zoom(screen.zoom);
        //         rgba.set_grid(Some(screen.grid_size));
        //     }
        //     rgba_layout.scroll_to(screen.scroll_offset);
        // }
        // }

        // Show the filter region content.

        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Content Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Content Dropdown".to_string()), None)
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

        if let Some(list) = ui.get_list_layout("Screen Content List") {
            list.clear();
            if let Some(screen) = screen {
                if filter_role < 2 {
                    // Show Named Sectors
                    for sector in &screen.map.sectors {
                        if !sector.name.is_empty()
                            && (filter_text.is_empty()
                                || sector.name.to_lowercase().contains(&filter_text))
                        {
                            let mut item = TheListItem::new(TheId::named_with_id(
                                "Screen Content List Item",
                                sector.creator_id,
                            ));
                            item.set_text(sector.name.clone());
                            item.add_value_column(100, TheValue::Text("Widget".to_string()));
                            list.add_item(item, ctx);
                        }
                    }

                    /*
                    for widget in screen.widget_list.iter() {
                        let name: String = widget.name.clone();
                        if filter_text.is_empty() || name.to_lowercase().contains(&filter_text) {
                            let mut item = TheListItem::new(TheId::named_with_id(
                                "Screen Content List Item",
                                widget.id,
                            ));
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Rename Widget...".to_string(),
                                    TheId::named("Rename Widget"),
                                )],
                                ..Default::default()
                            }));
                            item.set_text(name);
                            item.add_value_column(100, TheValue::Text("Widget".to_string()));
                            list.add_item(item, ctx);
                        }
                    }*/
                }
            }

            // Activate the current widget
            // Disabled for now to show screen bundle by default.

            // if let Some(selected) = list.selected() {
            //     ctx.ui
            //         .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
            // } else {
            //     list.select_first_item(ctx);
            // }
        }

        ctx.ui.relayout = true;
    }

    /// Apply the avatars
    pub fn apply_avatars(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(avatar_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id) {
                avatar_node.widgets.clear();
                avatar_node.childs.clear();

                for (_index, avatar) in project.avatars.iter() {
                    let node = gen_avatar_tree_node(avatar);

                    avatar_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current regions to the tree.
    pub fn apply_regions(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        let mut id: Option<Uuid> = None;

        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(region_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id) {
                region_node.widgets.clear();
                region_node.childs.clear();

                for (index, region) in project.regions.iter().enumerate() {
                    let mut node = gen_region_tree_node(region);
                    if index == 0 {
                        id = Some(region.id);
                        node.set_open(true);
                    }

                    region_node.add_child(node);
                }
            }
        }

        if let Some(id) = id {
            server_ctx.curr_region = id;
            set_project_context(ctx, ui, project, server_ctx, ProjectContext::Region(id));
            self.apply_region(ui, ctx, Some(id), project);
        }
    }

    /// Apply the current characters to the tree.
    pub fn apply_characters(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(characters_node) =
                tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
            {
                characters_node.widgets.clear();
                characters_node.childs.clear();

                for (_, character) in project.characters.iter() {
                    let node = gen_character_tree_node(character);

                    characters_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current items to the tree.
    pub fn apply_items(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(items_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id) {
                items_node.widgets.clear();
                items_node.childs.clear();

                for (_, item) in project.items.iter() {
                    let node = gen_item_tree_node(item);
                    items_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current tilemaps to the tree.
    pub fn apply_tilemaps(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(tilema_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_tilemaps_id)
            {
                tilema_node.widgets.clear();
                tilema_node.childs.clear();

                for tilemap in project.tilemaps.iter() {
                    let node = gen_tilemap_tree_node(tilemap);
                    tilema_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current screens to the tree.
    pub fn apply_screens(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(screen_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_screens_id) {
                screen_node.widgets.clear();
                screen_node.childs.clear();

                for (_, screen) in project.screens.iter() {
                    let node = gen_screen_tree_node(screen);
                    screen_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current assets to the tree.
    pub fn apply_assets(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(asset_node) =
                tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_fonts_id)
            {
                asset_node.widgets.clear();
                asset_node.childs.clear();

                for (_, assets) in project.assets.iter() {
                    let node = gen_asset_tree_node(assets);
                    asset_node.add_child(node);
                }
            }
        }
    }

    /// Apply the given item to the UI
    pub fn apply_region(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _region_id: Option<Uuid>,
        _project: &mut Project,
    ) {
        /*
        ui.set_widget_disabled_state("Region Remove", ctx, region_id.is_none());
        ui.set_widget_disabled_state("Region Settings", ctx, region_id.is_none());

        if UNDOMANAGER.read().unwrap().has_undo() {
            ctx.ui.set_enabled("Undo");
            ctx.ui.set_enabled("Redo");
        }

        if region_id.is_none() {
            if let Some(zoom) = ui.get_widget("Region Editor Zoom") {
                zoom.set_value(TheValue::Float(1.0));
            }

            if let Some(renderview) = ui.get_render_view("PolyView") {
                if let Some(buffer) = ctx.ui.icon("eldiron") {
                    let dim = *renderview.dim();
                    let scaled_buffer = buffer.scaled(dim.width, dim.height);
                    renderview.render_buffer_mut().fill(BLACK);
                    renderview.render_buffer_mut().copy_into(
                        (dim.width - scaled_buffer.dim().width) / 2,
                        (dim.height - scaled_buffer.dim().height) / 2,
                        &scaled_buffer,
                    );
                    renderview.set_needs_redraw(true);
                }
            }

            if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Region Editor".into()), None) {
                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                    if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                        rgba_view.set_mode(TheRGBAViewMode::Display);
                        rgba_view.set_zoom(1.0);
                        if let Some(buffer) = ctx.ui.icon("eldiron_map") {
                            rgba_view.set_buffer(buffer.clone());
                        }
                        rgba_view.set_grid(None);
                        ctx.ui.relayout = true;
                    }
                    rgba_layout.scroll_to(Vec2::new(0, 0));
                }
            }
        }*/

        /*
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
            if let Some(region_id) = region_id {
                if let Some(region) = project.get_region(&region_id) {
                    if filter_role < 2 {
                        // Show Characters
                        for (id, character) in region.characters.iter() {
                            let mut name = character.name.clone();

                            if let Some(character_template) =
                                project.characters.get(&character.character_id)
                            {
                                name = character_template.name.clone();
                            }

                            if filter_text.is_empty() || name.to_lowercase().contains(&filter_text)
                            {
                                let mut item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    *id,
                                ));
                                item.set_text(name);
                                item.add_value_column(100, TheValue::Text("Character".to_string()));
                                item.set_context_menu(Some(TheContextMenu {
                                    items: vec![TheContextMenuItem::new(
                                        "Delete Character...".to_string(),
                                        TheId::named("Sidebar Delete Character Instance"),
                                    )],
                                    ..Default::default()
                                }));
                                list.add_item(item, ctx);
                            }
                        }
                    }

                    if filter_role == 0 || filter_role == 3 {
                        // Show Named Sectors
                        for sector in &region.map.sectors {
                            if !sector.name.is_empty()
                                && (filter_text.is_empty()
                                    || sector.name.to_lowercase().contains(&filter_text))
                            {
                                let mut item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    sector.creator_id,
                                ));
                                item.set_text(sector.name.clone());
                                item.add_value_column(100, TheValue::Text("Sector".to_string()));
                                // item.set_context_menu(Some(TheContextMenu {
                                //     items: vec![TheContextMenuItem::new(
                                //         "Delete Character...".to_string(),
                                //         TheId::named("Sidebar Delete Character Instance"),
                                //     )],
                                //     ..Default::default()
                                // }));
                                list.add_item(item, ctx);
                            }
                        }
                    }

                    if filter_role == 0 || filter_role == 3 {
                        // Show Items
                        for (id, item) in region.items.iter() {
                            let mut name = item.name.clone();

                            if let Some(item_template) = project.items.get(&item.item_id) {
                                name = item_template.name.clone();
                            }

                            if filter_text.is_empty() || name.to_lowercase().contains(&filter_text)
                            {
                                let mut item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    *id,
                                ));
                                item.set_text(name);
                                item.add_value_column(100, TheValue::Text("Item".to_string()));
                                item.set_context_menu(Some(TheContextMenu {
                                    items: vec![TheContextMenuItem::new(
                                        "Delete Item...".to_string(),
                                        TheId::named("Sidebar Delete Item Instance"),
                                    )],
                                    ..Default::default()
                                }));
                                list.add_item(item, ctx);
                            }
                        }
                    }
                }
            }
        }*/

        // let mut changed = false;

        // ctx.ui.send(TheEvent::Custom(
        //     TheId::named("Update Minimap"),
        //     TheValue::Empty,
        // ));

        // RUSTERIX.write().unwrap().set_dirty();

        // if let Some(region_id) = region_id {
        //     ctx.ui.send(TheEvent::Custom(
        //         TheId::named("Render SceneManager Map"),
        //         TheValue::Empty,
        //     ));

        // if let Some(region) = project.get_region(&region_id) {
        //     ui.set_widget_value(
        //         "RegionConfigEdit",
        //         ctx,
        //         TheValue::Text(region.config.clone()),
        //     );
        // }
        // }
        /*
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
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Tile Size".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.tile_size.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Tracer Samples Edit".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.pathtracer_samples.to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(region) = region {
            if let Some(zoom) = ui.get_widget("Region Editor Zoom") {
                zoom.set_value(TheValue::Float(region.zoom));
            }
            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    rgba.set_zoom(region.zoom);
                    rgba.set_grid(Some(region.grid_size));
                }
                rgba_layout.scroll_to(region.scroll_offset);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 1".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_1.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 2".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_2.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 3".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_3.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 4".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_4.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        // Apply the region's timeline to the editor.
        if let Some(region) = region {
            if let Some(render_view) = ui.get_render_view("MiniMap") {
                let dim = *render_view.dim();
                let buffer = render_view.render_buffer_mut();
                buffer.resize(dim.width, dim.height);
                draw_minimap(region, buffer);
            }
        }*/
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
                                || tile.role == TileRole::from_index(filter_role as u8 - 1))
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

    fn show_empty_action_toml(&self, ui: &mut TheUI, _ctx: &mut TheContext) {
        if let Some(widget) = ui.get_widget("Action Params TOML")
            && let Some(edit) = widget.as_text_area_edit()
            && !edit.text().is_empty()
        {
            edit.set_text(String::new());
            let mut state = edit.get_state();
            state.cursor.row = 0;
            state.cursor.column = 0;
            state.selection.reset();
            TheTextAreaEditTrait::set_state(edit, state);
        }
    }

    fn show_action_toml_params(
        &self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
        action: &dyn Action,
    ) {
        let toml_text = nodeui_to_toml(&action.params());
        if let Some(widget) = ui.get_widget("Action Params TOML")
            && let Some(edit) = widget.as_text_area_edit()
        {
            let previous = edit.get_state();
            if edit.text() != toml_text {
                edit.set_text(toml_text);

                let mut state = edit.get_state();
                let row_max = state.rows.len().saturating_sub(1);
                let row = previous.cursor.row.min(row_max);
                let col_max = state
                    .rows
                    .get(row)
                    .map(|line| line.chars().count())
                    .unwrap_or(0);

                state.cursor.row = row;
                state.cursor.column = previous.cursor.column.min(col_max);
                state.selection.reset();
                TheTextAreaEditTrait::set_state(edit, state);
            }
        }
    }

    /// Shows the filtered actions for the current selection.
    pub fn show_actions(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Action List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();

                let actions = ACTIONLIST.read().unwrap();
                let mut found_current = false;

                let mut camera_actions: Vec<TheListItem> = vec![];
                let mut editor_actions: Vec<TheListItem> = vec![];
                let mut dock_actions: Vec<TheListItem> = vec![];

                if let Some(map) = project.get_map(server_ctx).or(Some(&Map::default())) {
                    for action in &actions.actions {
                        if action.is_applicable(map, ctx, server_ctx) {
                            let mut item = TheListItem::new(action.id().clone());
                            item.set_text(action.id().name.clone());

                            // let mut accel_text = String::new();
                            // if let Some(accel) = action.accel() {
                            //     accel_text = accel.description();
                            // }
                            // item.add_value_column(110, TheValue::Text(accel_text));
                            //

                            let mut status_text = action.info().to_string();
                            if let Some(accel) = action.accel() {
                                status_text =
                                    format!("{} ( {} )", status_text, accel.description());
                            }
                            item.set_status_text(&status_text);
                            item.set_background_color(TheColor::from(action.role().to_color()));

                            if Some(action.id().uuid) == server_ctx.curr_action_id {
                                found_current = true;
                                item.set_state(TheWidgetState::Selected);
                            }

                            if action.role() == ActionRole::Camera {
                                camera_actions.push(item);
                            } else if action.role() == ActionRole::Editor {
                                editor_actions.push(item);
                            } else {
                                dock_actions.push(item);
                            }
                        }
                    }
                }

                if DOCKMANAGER.read().unwrap().get_state() != DockManagerState::Editor {
                    for item in camera_actions {
                        list_layout.add_item(item, ctx);
                    }
                }
                for item in editor_actions {
                    list_layout.add_item(item, ctx);
                }
                for item in dock_actions {
                    list_layout.add_item(item, ctx);
                }

                if !found_current {
                    server_ctx.curr_action_id = None;
                }
            }
        }

        if let Some(action_id) = server_ctx.curr_action_id {
            if let Some(action) = ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id) {
                if let Some(map) = project.get_map(server_ctx) {
                    action.load_params(map);
                } else {
                    let default_map = Map::default();
                    action.load_params(&default_map);
                }
                action.load_params_project(project, server_ctx);
                self.show_action_toml_params(ui, ctx, server_ctx, action.as_ref());
            }
        } else {
            self.show_empty_action_toml(ui, ctx);
        }
    }

    /// Shows the filtered materials of the project.
    pub fn show_filtered_materials(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Shader Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let _filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Shader Filter Role".to_string()), None)
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

        if let Some(layout) = ui.canvas.get_layout(Some(&"Shader List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();

                for material in project.shaders.values() {
                    if filter_text.is_empty() || material.name.to_lowercase().contains(&filter_text)
                    //&& (filter_role == 0
                    //    || tile.role == TileRole::from_index(filter_role as u8 - 1).unwrap())
                    {
                        let mut item =
                            TheListItem::new(TheId::named_with_id("Shader Item", material.id));
                        item.set_text(material.name.clone());
                        //let sub_text = format!("Index: {index}");
                        // item.set_sub_text(sub_text);
                        // item.set_size(42);
                        if Some(material.id) == server_ctx.curr_material_id {
                            item.set_state(TheWidgetState::Selected);
                        }

                        /*
                        if let Some(Value::Texture(texture)) = material.properties.get("Shader") {
                            let resized = texture.resized(36, 36);
                            let rgba = TheRGBABuffer::from(
                                resized.data.clone(),
                                resized.width as u32,
                                resized.height as u32,
                            );
                            item.set_icon(rgba);
                        }*/

                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![
                                TheContextMenuItem::new(
                                    "Rename Shader...".to_string(),
                                    TheId::named("Rename Shader"),
                                ),
                                TheContextMenuItem::new(
                                    "Duplicate Shader".to_string(),
                                    TheId::named("Duplicate Shader"),
                                ),
                            ],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                    }
                }
            }
        }
        //ui.select_first_list_item("Shader List", ctx);
    }

    /// Apply the given asset to the UI
    pub fn apply_asset(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext, _asset: Option<&Asset>) {}

    /// Deselects the section buttons
    pub fn deselect_sections_buttons(
        &mut self,
        ctx: &mut TheContext,
        ui: &mut TheUI,
        except: String,
    ) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Section Buttons".into()), None) {
            for w in layout.widgets() {
                if !w.id().name.starts_with(&except) {
                    w.set_state(TheWidgetState::None);
                }
            }
        }

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Soft Update Minimap"),
            TheValue::Empty,
        ));
    }

    pub fn select_section_button(&mut self, ui: &mut TheUI, name: String) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Section Buttons".into()), None) {
            for w in layout.widgets() {
                if w.id().name.starts_with(&name) {
                    w.set_state(TheWidgetState::Selected);
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

    /// Clears the debug messages.
    pub fn clear_debug_messages(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Debug List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();

                let mut item = TheListItem::new(TheId::empty());
                item.set_text(fl!("info_server_started"));
                item.add_value_column(100, TheValue::Text("Status".to_string()));
                list_layout.add_item(item, ctx);
            }
        }
    }

    pub fn apply_action(
        &self,
        action: &Box<dyn Action>,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        param_update: bool,
    ) -> bool {
        if let Some(undo_atom) = action.apply(map, ui, ctx, server_ctx) {
            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);

            if server_ctx.editor_view_mode == EditorViewMode::D2
                && server_ctx.profile_view.is_some()
            {
            } else {
                map.update_surfaces();
                return true;
            }
            crate::editor::RUSTERIX.write().unwrap().set_dirty();
        }

        if !param_update {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Action List"),
                TheValue::Empty,
            ));
        }
        false
    }

    /// Tilemaps in the project have been updated, propagate the change to all relevant parties.
    pub fn update_tiles(&mut self, _ui: &mut TheUI, ctx: &mut TheContext, project: &mut Project) {
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.set_tiles(project.tiles.clone(), true);
        SCENEMANAGER.write().unwrap().set_tile_list(
            rusterix.assets.tile_list.clone(),
            rusterix.assets.tile_indices.clone(),
        );

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
    }
}
