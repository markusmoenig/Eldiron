use crate::editor::{DOCKMANAGER, RUSTERIX, SCENEMANAGER, UNDOMANAGER};
use crate::prelude::*;
pub use crate::tools::rect::RectTool;
use rusterix::Assets;
use rusterix::D3Camera;
use rusterix::PixelSource;
use rusterix::Surface;
use rusterix::chunkbuilder::terrain_generator::{TerrainConfig, TerrainGenerator};
use scenevm::GeoId;

pub struct ToolList {
    pub server_time: TheTime,
    pub render_button_text: String,
    pub authoring_mode: bool,
    pub text_game_mode: bool,

    pub game_tools: Vec<Box<dyn Tool>>,
    pub curr_game_tool: usize,

    // Editor tools for dock editors
    pub editor_tools: Vec<Box<dyn EditorTool>>,
    pub curr_editor_tool: usize,
    pub editor_mode: bool,
}

impl Default for ToolList {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolList {
    const AUTHORING_BUTTON_NAME: &'static str = "Authoring";
    const TEXT_PLAY_BUTTON_NAME: &'static str = "Text Play";

    fn get_tool_map_mut<'a>(
        project: &'a mut Project,
        server_ctx: &ServerContext,
    ) -> Option<&'a mut Map> {
        if server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
        {
            project
                .get_region_mut(&server_ctx.curr_region)
                .map(|region| &mut region.map)
        } else {
            project.get_map_mut(server_ctx)
        }
    }

    fn collect_terrain_tile_overrides(map: &Map) -> FxHashMap<(i32, i32), PixelSource> {
        match map.properties.get("tiles") {
            Some(Value::TileOverrides(tiles)) => tiles.clone(),
            _ => FxHashMap::default(),
        }
    }

    fn collect_terrain_blend_overrides(
        map: &Map,
    ) -> FxHashMap<(i32, i32), (VertexBlendPreset, PixelSource)> {
        match map.properties.get("blend_tiles") {
            Some(Value::BlendOverrides(tiles)) => tiles.clone(),
            _ => FxHashMap::default(),
        }
    }

    fn changed_terrain_override_keys(old_map: &Map, new_map: &Map) -> FxHashSet<(i32, i32)> {
        let old_tiles = Self::collect_terrain_tile_overrides(old_map);
        let new_tiles = Self::collect_terrain_tile_overrides(new_map);
        let old_blends = Self::collect_terrain_blend_overrides(old_map);
        let new_blends = Self::collect_terrain_blend_overrides(new_map);

        let mut keys = FxHashSet::default();
        for k in old_tiles.keys() {
            keys.insert(*k);
        }
        for k in new_tiles.keys() {
            keys.insert(*k);
        }
        for k in old_blends.keys() {
            keys.insert(*k);
        }
        for k in new_blends.keys() {
            keys.insert(*k);
        }

        let mut changed = FxHashSet::default();
        for key in keys {
            if old_tiles.get(&key) != new_tiles.get(&key)
                || old_blends.get(&key) != new_blends.get(&key)
            {
                changed.insert(key);
            }
        }
        changed
    }

    fn apply_editor_rgba_mode(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if !self.editor_mode || self.curr_editor_tool >= self.editor_tools.len() {
            return;
        }

        if let Some(mode) = self.editor_tools[self.curr_editor_tool].rgba_view_mode()
            && let Some(layout) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
            && let Some(rgba_view) = layout.rgba_view_mut().as_rgba_view()
        {
            let is_selection_mode = mode == TheRGBAViewMode::TileSelection;
            rgba_view.set_mode(mode);
            rgba_view.set_rectangular_selection(is_selection_mode);
            layout.relayout(ctx);
        }
    }

    pub fn new() -> Self {
        let game_tools: Vec<Box<dyn Tool>> = vec![
            Box::new(VertexTool::new()),
            Box::new(LinedefTool::new()),
            Box::new(SectorTool::new()),
            Box::new(RectTool::new()),
            Box::new(crate::tools::dungeon::DungeonTool::new()),
            Box::new(crate::tools::builder::BuilderTool::new()),
            Box::new(crate::tools::organic::OrganicTool::new()),
            Box::new(crate::tools::entity::EntityTool::new()),
            // Box::new(RenderTool::new()),
            // Box::new(TerrainTool::new()),
            // Box::new(CodeTool::new()),
            // Box::new(DataTool::new()),
            // Box::new(TilesetTool::new()),
            // Box::new(ConfigTool::new()),
            // Box::new(InfoTool::new()),
            Box::new(GameTool::new()),
        ];
        Self {
            server_time: TheTime::default(),
            render_button_text: "Finished".to_string(),
            authoring_mode: false,
            text_game_mode: false,
            game_tools,
            curr_game_tool: 2,

            editor_tools: Vec::new(),
            curr_editor_tool: 0,
            editor_mode: false,
        }
    }

    /// Build the UI
    pub fn set_active_editor(&mut self, list: &mut dyn TheVLayoutTrait, ctx: &mut TheContext) {
        list.clear();
        ctx.ui.relayout = true;

        if self.editor_mode {
            // Show editor tools
            for (index, tool) in self.editor_tools.iter().enumerate() {
                let mut b = TheToolListButton::new(tool.id());

                b.set_icon_name(tool.icon_name());
                b.set_status_text(&Self::status_text_with_accel(tool.info(), tool.accel()));
                if index == self.curr_editor_tool {
                    b.set_state(TheWidgetState::Selected);
                }
                list.add_widget(Box::new(b));
            }
        } else {
            // Show game tools
            for (index, tool) in self.game_tools.iter().enumerate() {
                let mut b = TheToolListButton::new(tool.id());

                b.set_icon_name(tool.icon_name());
                b.set_status_text(&Self::status_text_with_accel(tool.info(), tool.accel()));
                if index == self.curr_game_tool {
                    b.set_state(TheWidgetState::Selected);
                }
                list.add_widget(Box::new(b));
            }

            let mut sep = TheSeparator::new(TheId::named_with_id("Tool Separator", Uuid::new_v4()));
            sep.limiter_mut().set_max_width(46);
            sep.limiter_mut().set_max_height(8);
            list.add_widget(Box::new(sep));

            let mut authoring = TheToolListButton::new(TheId::named(Self::AUTHORING_BUTTON_NAME));
            authoring.set_icon_name("book-open".to_string());
            authoring.set_status_text(&fl!("status_tool_authoring"));
            if self.authoring_mode {
                authoring.set_state(TheWidgetState::Selected);
            }
            list.add_widget(Box::new(authoring));

            let mut text_play = TheToolListButton::new(TheId::named(Self::TEXT_PLAY_BUTTON_NAME));
            text_play.set_icon_name("terminal".to_string());
            text_play.set_status_text(&fl!("status_tool_text_play"));
            if self.text_game_mode {
                text_play.set_state(TheWidgetState::Selected);
            }
            list.add_widget(Box::new(text_play));
        }
    }

    fn status_text_with_accel(info: String, accel: Option<char>) -> String {
        if let Some(accel) = accel {
            let marker = format!("({accel})");
            if info.contains(&marker) {
                info
            } else {
                format!("{info} ({accel})")
            }
        } else {
            info
        }
    }

    fn enforce_builder_dock(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.editor_mode {
            return;
        }
        if self.game_tools[self.curr_game_tool].id().name != "Builder Tool" {
            return;
        }
        let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
        if current_dock == "Tiles" {
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Builder".into(), ui, ctx, project, server_ctx);
        }
    }

    /// Switch to editor tools mode
    pub fn set_editor_tools(
        &mut self,
        tools: Vec<Box<dyn EditorTool>>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.editor_tools = tools;
        self.curr_editor_tool = 0;
        self.editor_mode = true;

        // Activate first tool
        if !self.editor_tools.is_empty() {
            self.editor_tools[0].activate();
            self.apply_editor_rgba_mode(ui, ctx);
        }

        // Update the toolbar
        if let Some(list) = ui.get_vlayout("Tool List Layout") {
            self.set_active_editor(list, ctx);
        }
    }

    /// Switch back to game tools mode
    pub fn set_game_tools(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        // Deactivate current editor tool
        if self.editor_mode && self.curr_editor_tool < self.editor_tools.len() {
            self.editor_tools[self.curr_editor_tool].deactivate();
        }

        self.editor_mode = false;
        self.editor_tools.clear();

        // Update the toolbar
        if let Some(list) = ui.get_vlayout("Tool List Layout") {
            self.set_active_editor(list, ctx);
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// If the map has been changed, update its context and add an undo.
    fn update_map_context(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        undo_atom: Option<ProjectUndoAtom>,
    ) {
        if let Some(undo_atom) = undo_atom {
            if let Some(pc) = undo_atom.pc() {
                if pc.is_region() {
                    if server_ctx.editor_view_mode == EditorViewMode::D2
                        && server_ctx.editing_surface.is_some()
                    {
                    } else {
                        self.update_geometry_overlay_3d(project, server_ctx);
                    }
                    let mut used_incremental_terrain_update = false;
                    if server_ctx.curr_map_tool_type == MapToolType::Rect
                        && server_ctx.editor_view_mode != EditorViewMode::D2
                        && let ProjectUndoAtom::MapEdit(_, old_map, new_map) = &undo_atom
                    {
                        let changed_keys = Self::changed_terrain_override_keys(old_map, new_map);

                        if !changed_keys.is_empty() {
                            let chunk_size = new_map.terrain.chunk_size.max(1);
                            let mut dirty_chunks: FxHashSet<(i32, i32)> = FxHashSet::default();
                            for (x, z) in changed_keys {
                                let cx = x.div_euclid(chunk_size) * chunk_size;
                                let cz = z.div_euclid(chunk_size) * chunk_size;
                                dirty_chunks.insert((cx, cz));
                            }

                            let mut sm = SCENEMANAGER.write().unwrap();
                            sm.update_map((**new_map).clone());
                            sm.add_dirty(dirty_chunks.into_iter().collect());
                            used_incremental_terrain_update = true;
                        }
                    }

                    if !used_incremental_terrain_update {
                        crate::utils::scenemanager_render_map(project, server_ctx);
                    }
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
            }
            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
        }
    }

    pub fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        assets: &Assets,
    ) {
        self.game_tools[self.curr_game_tool].draw_hud(buffer, map, ctx, server_ctx, assets);
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
        if self.editor_mode && self.curr_editor_tool < self.editor_tools.len() {
            let should_forward_to_tool = match event {
                // Keep tool switching and shortcuts handled by ToolList itself.
                TheEvent::StateChanged(_, _) | TheEvent::KeyDown(_) => false,
                TheEvent::Custom(id, _) if id.name == "Set Tool" => false,
                _ => true,
            };
            if should_forward_to_tool {
                return self.editor_tools[self.curr_editor_tool]
                    .handle_event(event, ui, ctx, project, server_ctx);
            }
        }

        let mut redraw = false;
        match event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Editor View Switch" {
                    let prev_mode = server_ctx.editor_view_mode;
                    let old = prev_mode.is_3d();

                    // Persist region camera anchors before switching.
                    if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                        if prev_mode == EditorViewMode::D2 {
                            server_ctx.store_edit_view_2d_for_map(
                                region.map.id,
                                region.map.offset,
                                region.map.grid_size,
                            );
                        } else {
                            server_ctx.store_edit_view_for_map(
                                region.map.id,
                                prev_mode,
                                region.editing_position_3d,
                                region.editing_look_at_3d,
                            );
                            if prev_mode == EditorViewMode::Iso {
                                let iso_scale =
                                    crate::editor::EDITCAMERA.read().unwrap().iso_camera.scale();
                                server_ctx
                                    .store_edit_view_iso_scale_for_map(region.map.id, iso_scale);
                            }
                            match prev_mode {
                                EditorViewMode::Iso => {
                                    region.editing_position_iso_3d =
                                        Some(region.editing_position_3d);
                                    region.editing_look_at_iso_3d = Some(region.editing_look_at_3d);
                                    let iso_scale = crate::editor::EDITCAMERA
                                        .read()
                                        .unwrap()
                                        .iso_camera
                                        .scale();
                                    region.editing_iso_scale = Some(iso_scale);
                                }
                                EditorViewMode::Orbit => {
                                    region.editing_position_orbit_3d =
                                        Some(region.editing_position_3d);
                                    region.editing_look_at_orbit_3d =
                                        Some(region.editing_look_at_3d);
                                    region.editing_orbit_distance = Some(
                                        crate::editor::EDITCAMERA
                                            .read()
                                            .unwrap()
                                            .orbit_camera
                                            .distance,
                                    );
                                }
                                EditorViewMode::FirstP => {
                                    region.editing_position_firstp_3d =
                                        Some(region.editing_position_3d);
                                    region.editing_look_at_firstp_3d =
                                        Some(region.editing_look_at_3d);
                                }
                                EditorViewMode::D2 => {}
                            }
                        }
                    }

                    server_ctx.editor_view_mode = EditorViewMode::from_index(*index as i32);
                    let new_mode = server_ctx.editor_view_mode;
                    let new = new_mode.is_3d();

                    if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                        region.map.camera = match new_mode {
                            EditorViewMode::D2 => MapCamera::TwoD,
                            EditorViewMode::Iso | EditorViewMode::Orbit => MapCamera::ThreeDIso,
                            EditorViewMode::FirstP => MapCamera::ThreeDFirstPerson,
                        };
                    }

                    // Restore region camera anchor for the selected view mode.
                    if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                        if new_mode == EditorViewMode::D2 {
                            if let Some((offset, grid_size)) =
                                server_ctx.load_edit_view_2d_for_map(region.map.id)
                            {
                                region.map.offset = offset;
                                region.map.grid_size = grid_size;
                            } else {
                                server_ctx.center_map_at_grid_pos(
                                    Vec2::zero(),
                                    Vec2::new(0.0, -1.0),
                                    &mut region.map,
                                );
                            }
                        } else if let Some((pos, look)) =
                            server_ctx.load_edit_view_for_map(region.map.id, new_mode)
                        {
                            region.editing_position_3d = pos;
                            region.editing_look_at_3d = look;
                            if new_mode == EditorViewMode::Iso
                                && let Some(iso_scale) =
                                    server_ctx.load_edit_view_iso_scale_for_map(region.map.id)
                            {
                                crate::editor::EDITCAMERA
                                    .write()
                                    .unwrap()
                                    .iso_camera
                                    .set_parameter_f32("scale", iso_scale);
                            }
                            if new_mode == EditorViewMode::Orbit
                                && let Some(distance) = region.editing_orbit_distance
                            {
                                crate::editor::EDITCAMERA
                                    .write()
                                    .unwrap()
                                    .orbit_camera
                                    .set_parameter_f32("distance", distance);
                            }
                        } else {
                            match new_mode {
                                EditorViewMode::Iso => {
                                    if let (Some(pos), Some(look)) = (
                                        region.editing_position_iso_3d,
                                        region.editing_look_at_iso_3d,
                                    ) {
                                        region.editing_position_3d = pos;
                                        region.editing_look_at_3d = look;
                                    }
                                    if let Some(iso_scale) = region.editing_iso_scale {
                                        crate::editor::EDITCAMERA
                                            .write()
                                            .unwrap()
                                            .iso_camera
                                            .set_parameter_f32("scale", iso_scale);
                                    }
                                }
                                EditorViewMode::Orbit => {
                                    if let (Some(pos), Some(look)) = (
                                        region.editing_position_orbit_3d,
                                        region.editing_look_at_orbit_3d,
                                    ) {
                                        region.editing_position_3d = pos;
                                        region.editing_look_at_3d = look;
                                    }
                                    if let Some(distance) = region.editing_orbit_distance {
                                        crate::editor::EDITCAMERA
                                            .write()
                                            .unwrap()
                                            .orbit_camera
                                            .set_parameter_f32("distance", distance);
                                    }
                                }
                                EditorViewMode::FirstP => {
                                    if let (Some(pos), Some(look)) = (
                                        region.editing_position_firstp_3d,
                                        region.editing_look_at_firstp_3d,
                                    ) {
                                        region.editing_position_3d = pos;
                                        region.editing_look_at_3d = look;
                                    }
                                }
                                EditorViewMode::D2 => {}
                            }
                        }
                    }

                    if let Some(editing_pos_buffer) = server_ctx.editing_pos_buffer {
                        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                            region.editing_position_3d = editing_pos_buffer;
                        }
                        server_ctx.editing_pos_buffer = None;
                    }
                    server_ctx.editing_surface = None;

                    RUSTERIX.write().unwrap().client.scene.d2_static.clear();
                    RUSTERIX.write().unwrap().client.scene.d2_dynamic.clear();

                    if old != new {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Render SceneManager Map"),
                            TheValue::Empty,
                        ));
                    } else if new {
                        self.update_geometry_overlay_3d(project, server_ctx);
                    }
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Action List"),
                        TheValue::Empty,
                    ));
                }
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                if let Some(id) = &ctx.ui.focus {
                    if id.name == "PolyView" {
                        if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                            if *c == ',' {
                                map.grid_size -= 2.0;
                                return false;
                            } else if *c == '.' {
                                map.grid_size += 2.0;
                                return false;
                            } else if server_ctx.editor_view_mode != EditorViewMode::D2
                                && !server_ctx.game_input_mode
                                && server_ctx.curr_map_tool_type != MapToolType::Game
                            {
                                if *c == 'g' || *c == 'G' {
                                    server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                                    self.update_geometry_overlay_3d(project, server_ctx);
                                    RUSTERIX.write().unwrap().set_dirty();
                                    return false;
                                } else if *c == 'd' || *c == 'D' {
                                    server_ctx.geometry_edit_mode = GeometryEditMode::Detail;
                                    self.update_geometry_overlay_3d(project, server_ctx);
                                    RUSTERIX.write().unwrap().set_dirty();
                                    return false;
                                }
                            }

                            let undo_atom = self.get_current_tool().map_event(
                                MapEvent::MapKey(*c),
                                ui,
                                ctx,
                                map,
                                server_ctx,
                            );
                            if undo_atom.is_some() {
                                map.changed += 1;
                            }
                            self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                        }

                        if server_ctx.get_map_context() == MapContext::Region
                            && !server_ctx.rotated_entities.is_empty()
                            && let Some(region) = project.get_region_mut(&server_ctx.curr_region)
                        {
                            for (id, (_from, to)) in server_ctx.rotated_entities.drain() {
                                if let Some(instance) = region.characters.get_mut(&id) {
                                    instance.orientation = to;
                                }
                                if let Some(entity) =
                                    region.map.entities.iter_mut().find(|e| e.creator_id == id)
                                {
                                    entity.orientation = to;
                                }
                            }
                        } else {
                            server_ctx.rotated_entities.clear();
                        }
                    }
                }

                let mut acc = !ui.focus_widget_supports_text_input(ctx);
                if self.get_current_tool().id().name == "Game Tool"
                    || ui.ctrl
                    || ui.logo
                    || ui.alt
                    || server_ctx.game_input_mode
                {
                    acc = false;
                }

                if acc {
                    /*
                    if (*c == '-' || *c == '=' || *c == '+') && (ui.ctrl || ui.logo) {
                        // Global Zoom In / Zoom Out
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if *c == '=' || *c == '+' {
                                region.zoom += 0.2;
                            } else {
                                region.zoom -= 0.2;
                            }
                            region.zoom = region.zoom.clamp(1.0, 5.0);
                            if let Some(layout) = ui.get_rgba_layout("Region Editor") {
                                layout.set_zoom(region.zoom);
                                layout.relayout(ctx);
                            }
                            if let Some(edit) = ui.get_text_line_edit("Editor Zoom") {
                                edit.set_value(TheValue::Float(region.zoom));
                            }
                            return true;
                        }
                    }*/

                    let mut tool_uuid = None;

                    if self.editor_mode {
                        // Check editor tool accelerators
                        for tool in self.editor_tools.iter() {
                            if let Some(acc) = tool.accel() {
                                if acc.to_ascii_lowercase() == *c {
                                    tool_uuid = Some(tool.id().uuid);
                                    ctx.ui.set_widget_state(
                                        self.editor_tools[self.curr_editor_tool].id().name,
                                        TheWidgetState::None,
                                    );
                                    ctx.ui
                                        .set_widget_state(tool.id().name, TheWidgetState::Selected);
                                }
                            }
                        }
                    } else {
                        // Check game tool accelerators
                        for tool in self.game_tools.iter() {
                            if let Some(acc) = tool.accel() {
                                if acc.to_ascii_lowercase() == *c {
                                    tool_uuid = Some(tool.id().uuid);
                                    ctx.ui.set_widget_state(
                                        self.game_tools[self.curr_game_tool].id().name,
                                        TheWidgetState::None,
                                    );
                                    ctx.ui
                                        .set_widget_state(tool.id().name, TheWidgetState::Selected);
                                }
                            }
                        }
                    }

                    if let Some(uuid) = tool_uuid {
                        self.set_tool(uuid, ui, ctx, project, server_ctx);
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == Self::AUTHORING_BUTTON_NAME && *state == TheWidgetState::Clicked {
                    self.authoring_mode = !self.authoring_mode;
                    let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                    if current_dock == "Tiles" || current_dock == "Authoring" {
                        let dock = if self.authoring_mode {
                            "Authoring"
                        } else {
                            "Tiles"
                        };
                        DOCKMANAGER.write().unwrap().set_dock(
                            dock.into(),
                            ui,
                            ctx,
                            project,
                            server_ctx,
                        );
                    }
                    ctx.ui.set_widget_state(
                        Self::AUTHORING_BUTTON_NAME.to_string(),
                        if self.authoring_mode {
                            TheWidgetState::Selected
                        } else {
                            TheWidgetState::None
                        },
                    );
                    redraw = true;
                    return redraw;
                }
                if id.name == Self::TEXT_PLAY_BUTTON_NAME && *state == TheWidgetState::Clicked {
                    self.text_game_mode = !self.text_game_mode;
                    server_ctx.text_game_mode = self.text_game_mode;
                    ctx.ui.set_widget_state(
                        Self::TEXT_PLAY_BUTTON_NAME.to_string(),
                        if self.text_game_mode {
                            TheWidgetState::Selected
                        } else {
                            TheWidgetState::None
                        },
                    );

                    if self.get_current_tool().id().name == "Game Tool" && server_ctx.game_mode {
                        if let Some(stack) = ui.get_stack_layout("Game Output Stack") {
                            stack.set_index(if self.text_game_mode { 1 } else { 0 });
                        }
                        if self.text_game_mode {
                            crate::editor::TEXTGAME.write().unwrap().activate(ui, ctx);
                        } else if let Some(widget) = ui.get_widget("PolyView") {
                            let id = widget.id().clone();
                            ctx.ui.set_focus(&id);
                        }
                    }
                    redraw = true;
                    return redraw;
                }
                if id.name == "Editor View Switch"
                    && *state == TheWidgetState::Clicked
                    && server_ctx.editor_view_mode == EditorViewMode::D2
                    && server_ctx.editing_surface.is_some()
                {
                    // Re-clicking 2D while editing a profile/surface should exit surface mode.
                    server_ctx.editing_surface = None;
                    RUSTERIX.write().unwrap().client.scene.d2_static.clear();
                    RUSTERIX.write().unwrap().client.scene.d2_dynamic.clear();
                    RUSTERIX.write().unwrap().set_dirty();
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Render SceneManager Map"),
                        TheValue::Empty,
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Action List"),
                        TheValue::Empty,
                    ));
                    redraw = true;
                }
                if id.name.contains("Tool") && *state == TheWidgetState::Selected {
                    if server_ctx.help_mode {
                        if self.editor_mode {
                            for tool in self.editor_tools.iter() {
                                if tool.id().uuid == id.uuid {
                                    if let Some(url) = tool.help_url() {
                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Show Help"),
                                            TheValue::Text(url),
                                        ));
                                    }
                                }
                            }
                        } else {
                            for tool in self.game_tools.iter() {
                                if tool.id().uuid == id.uuid {
                                    if tool.id().uuid == id.uuid {
                                        if let Some(url) = tool.help_url() {
                                            ctx.ui.send(TheEvent::Custom(
                                                TheId::named("Show Help"),
                                                TheValue::Text(url),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    redraw = self.set_tool(id.uuid, ui, ctx, project, server_ctx);
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                if let Some(id) = &ctx.ui.focus {
                    if id.name == "PolyView" {
                        if *code == TheKeyCode::Up {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.y += 50.0;
                            }
                            return false;
                        }
                        if *code == TheKeyCode::Down {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.y -= 50.0;
                            }
                            return false;
                        }
                        if *code == TheKeyCode::Left {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.x -= 50.0;
                            }
                            return false;
                        }
                        if *code == TheKeyCode::Right {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.x += 50.0;
                            }
                            return false;
                        }
                        if *code == TheKeyCode::Escape {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                if server_ctx.paste_clipboard.is_some() {
                                    server_ctx.paste_clipboard = None;
                                    return true;
                                }

                                let undo_atom = self.get_current_tool().map_event(
                                    MapEvent::MapEscape,
                                    ui,
                                    ctx,
                                    map,
                                    server_ctx,
                                );
                                if undo_atom.is_some() {
                                    map.changed += 1;
                                }
                                self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                                if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                    self.update_geometry_overlay_3d(project, server_ctx);
                                }
                            }
                        } else if *code == TheKeyCode::Delete {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                let undo_atom = self.get_current_tool().map_event(
                                    MapEvent::MapDelete,
                                    ui,
                                    ctx,
                                    map,
                                    server_ctx,
                                );
                                if undo_atom.is_some() {
                                    map.changed += 1;
                                }
                                self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                                if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                    self.update_geometry_overlay_3d(project, server_ctx);
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "PolyView" {
                    if !server_ctx.game_mode && !server_ctx.game_input_mode {
                        if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                            if coord.y > 20 {
                                // Test for Paste operation
                                if let Some(paste) = &server_ctx.paste_clipboard {
                                    if let Some(hover) = server_ctx.hover_cursor {
                                        let prev = map.clone();
                                        let prev_counts = (
                                            map.vertices.len(),
                                            map.linedefs.len(),
                                            map.sectors.len(),
                                        );
                                        map.paste_at_position(paste, hover);
                                        let post_counts = (
                                            map.vertices.len(),
                                            map.linedefs.len(),
                                            map.sectors.len(),
                                        );
                                        let inserted = post_counts != prev_counts;

                                        if inserted {
                                            if server_ctx.curr_map_tool_type == MapToolType::Vertex
                                            {
                                                map.selected_linedefs.clear();
                                                map.selected_sectors.clear();
                                            } else if server_ctx.curr_map_tool_type
                                                == MapToolType::Linedef
                                            {
                                                map.selected_vertices.clear();
                                                map.selected_sectors.clear();
                                            } else if server_ctx.curr_map_tool_type
                                                == MapToolType::Sector
                                            {
                                                map.selected_vertices.clear();
                                                map.selected_linedefs.clear();
                                            }

                                            // Paste bypasses normal tool finalize paths; rebuild
                                            // associations + surfaces so overlays and rendering
                                            // use a fully consistent map immediately.
                                            map.sanitize();
                                            map.changed += 1;
                                            server_ctx.paste_clipboard = None;

                                            let undo_atom = ProjectUndoAtom::MapEdit(
                                                server_ctx.pc,
                                                Box::new(prev),
                                                Box::new(map.clone()),
                                            );

                                            // We bypass normal tool click/drag flow during paste.
                                            // Reset any stale drag state in the active tool so a
                                            // following drag/up event cannot restore an old map snapshot.
                                            let _ = self.get_current_tool().map_event(
                                                MapEvent::MapUp(*coord),
                                                ui,
                                                ctx,
                                                map,
                                                server_ctx,
                                            );

                                            self.update_map_context(
                                                ui,
                                                ctx,
                                                project,
                                                server_ctx,
                                                Some(undo_atom),
                                            );
                                            ctx.ui.send(TheEvent::Custom(
                                                TheId::named("Map Selection Changed"),
                                                TheValue::Empty,
                                            ));
                                            ctx.ui.send(TheEvent::Custom(
                                                TheId::named("Render SceneManager Map"),
                                                TheValue::Empty,
                                            ));

                                            return true;
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                            let undo_atom = self.get_current_tool().map_event(
                                MapEvent::MapClicked(*coord),
                                ui,
                                ctx,
                                map,
                                server_ctx,
                            );
                            if undo_atom.is_some() {
                                map.changed += 1;
                            }
                            self.update_map_context(ui, ctx, project, server_ctx, undo_atom);

                            if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                self.update_geometry_overlay_3d(project, server_ctx);
                            }
                            redraw = true;
                        }
                    } else {
                        let current_map = RUSTERIX.read().unwrap().client.current_map.clone();
                        for r in &mut project.regions {
                            if r.map.name == current_map {
                                self.get_current_tool().map_event(
                                    MapEvent::MapClicked(*coord),
                                    ui,
                                    ctx,
                                    &mut r.map,
                                    server_ctx,
                                );
                            }
                        }
                    }
                }
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        // Map dragging handled by tools.
                    }

                    if server_ctx.editor_view_mode != EditorViewMode::D2
                        && let Some(render_view) = ui.get_render_view("PolyView")
                    {
                        if let Some(rc) =
                            self.get_geometry_hit(render_view, *coord, project, server_ctx)
                        {
                            server_ctx.geo_hit = Some(rc.0);
                            server_ctx.geo_hit_pos = rc.1;
                        } else {
                            server_ctx.geo_hit = None;
                            server_ctx.geo_hit_pos = Vec3::zero();
                        }
                    }

                    if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                        let undo_atom = self.get_current_tool().map_event(
                            MapEvent::MapDragged(*coord),
                            ui,
                            ctx,
                            map,
                            server_ctx,
                        );
                        if undo_atom.is_some() {
                            map.changed += 1;
                            // if server_ctx.get_map_context() == MapContext::Shader {
                            // }
                        }
                        self.update_map_context(ui, ctx, project, server_ctx, undo_atom);

                        if server_ctx.editor_view_mode != EditorViewMode::D2 {
                            self.update_geometry_overlay_3d(project, server_ctx);
                        }
                    }

                    redraw = true;
                }
            }
            TheEvent::RenderViewUp(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                        let undo_atom = self.get_current_tool().map_event(
                            MapEvent::MapUp(*coord),
                            ui,
                            ctx,
                            map,
                            server_ctx,
                        );

                        if undo_atom.is_some() {
                            map.changed += 1;
                            map.update_surfaces();
                        }
                        self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                        if server_ctx.editor_view_mode != EditorViewMode::D2 {
                            self.update_geometry_overlay_3d(project, server_ctx);
                        }
                    }

                    if server_ctx.get_map_context() == MapContext::Region {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let mut move_atoms: Vec<ProjectUndoAtom> = Vec::new();

                            for (id, (from, to)) in server_ctx.moved_entities.drain() {
                                if from != to {
                                    if let Some(instance) = region.characters.get_mut(&id) {
                                        instance.position = to;
                                    }
                                    move_atoms.push(ProjectUndoAtom::MoveRegionCharacterInstance(
                                        server_ctx.curr_region,
                                        id,
                                        from,
                                        to,
                                    ));
                                }
                            }
                            for (id, (from, to)) in server_ctx.moved_items.drain() {
                                if from != to {
                                    if let Some(instance) = region.items.get_mut(&id) {
                                        instance.position = to;
                                    }
                                    move_atoms.push(ProjectUndoAtom::MoveRegionItemInstance(
                                        server_ctx.curr_region,
                                        id,
                                        from,
                                        to,
                                    ));
                                }
                            }

                            for atom in move_atoms {
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else {
                        server_ctx.moved_entities.clear();
                        server_ctx.moved_items.clear();
                    }

                    redraw = true;
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.editor_view_mode != EditorViewMode::D2 {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            if let Some(rc) =
                                self.get_geometry_hit(render_view, *coord, project, server_ctx)
                            {
                                server_ctx.geo_hit = Some(rc.0);
                                server_ctx.geo_hit_pos = rc.1;
                            } else {
                                server_ctx.geo_hit = None;
                                server_ctx.geo_hit_pos = Vec3::zero();
                            }
                            // println!("{:?}", server_ctx.geo_hit);
                        }
                    }
                    if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                        let undo_atom = self.get_current_tool().map_event(
                            MapEvent::MapHover(*coord),
                            ui,
                            ctx,
                            map,
                            server_ctx,
                        );
                        if undo_atom.is_some() {
                            map.changed += 1;
                            map.update_surfaces();
                        }
                        self.update_map_context(ui, ctx, project, server_ctx, undo_atom);

                        if server_ctx.editor_view_mode != EditorViewMode::D2 {
                            self.update_geometry_overlay_3d(project, server_ctx);
                        }
                    }
                    redraw = false;
                }
            }
            // TheEvent::RenderViewScrollBy(id, coord) => { TODO
            //     if id.name == "PolyView" {
            //         if server_ctx.editor_view_mode == EditorViewMode::Iso {
            //             if ui.ctrl || ui.logo {
            //                 EDITCAMERA.write().unwrap().scroll_by(coord.y as f32);
            //             }
            //         }
            //     }
            // }
            /*
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "Region Editor View"
                    || id.name == "Screen Editor View"
                    || id.name == "TerrainMap View"
                {
                    let mut coord_f = Vec2f::from(*coord);
                    if id.name == "Region Editor View" {
                        if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                coord_f = rgba_view.float_pos();
                            }
                        }
                    }

                    self.get_current_tool().tool_event(
                        ToolEvent::TileDown(*coord, coord_f),
                        ToolContext::TwoD,
                        ui,
                        ctx,
                        project,
                        server,
                        client,
                        server_ctx,
                    );
                }
            }
            TheEvent::TileEditorDragged(id, coord) => {
                if id.name == "Region Editor View"
                    || id.name == "Screen Editor View"
                    || id.name == "TerrainMap View"
                {
                    let mut coord_f = Vec2f::from(*coord);
                    if id.name == "Region Editor View" {
                        if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                coord_f = rgba_view.float_pos();
                            }
                        }
                    }

                    self.get_current_tool().tool_event(
                        ToolEvent::TileDrag(*coord, coord_f),
                        ToolContext::TwoD,
                        ui,
                        ctx,
                        project,
                        server,
                        client,
                        server_ctx,
                    );
                }
            }
            TheEvent::TileEditorUp(id) => {
                if id.name == "Region Editor View"
                    || id.name == "Screen Editor View"
                    || id.name == "TerrainMap View"
                {
                    self.get_current_tool().tool_event(
                        ToolEvent::TileUp,
                        ToolContext::TwoD,
                        ui,
                        ctx,
                        project,
                        server,
                        client,
                        server_ctx,
                    );
                }
            }
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "PolyView" {
                    // if let Some(render_view) = ui.get_render_view("PolyView") {
                    // let dim = render_view.dim();
                    // if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    // let pos = RENDERER.lock().unwrap().get_hit_position_at(
                    //     *coord,
                    //     region,
                    //     &mut server.get_instance_draw_settings(server_ctx.curr_region),
                    //     dim.width as usize,
                    //     dim.height as usize,
                    // );
                    //
                    let pos = Some((*coord, *coord));

                    if let Some((pos, _)) = pos {
                        redraw = self.get_current_tool().tool_event(
                            ToolEvent::TileDown(
                                vec2i(pos.x, pos.y),
                                vec2f(pos.x as f32, pos.y as f32),
                            ),
                            ToolContext::ThreeD,
                            ui,
                            ctx,
                            project,
                            server,
                            client,
                            server_ctx,
                        );
                    }
                    // }
                    // }
                }
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView" {
                    //if let Some(render_view) = ui.get_render_view("RenderView") {
                    //let dim = render_view.dim();
                    //if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    // let pos = RENDERER.lock().unwrap().get_hit_position_at(
                    //     *coord,
                    //     region,
                    //     &mut server.get_instance_draw_settings(server_ctx.curr_region),
                    //     dim.width as usize,
                    //     dim.height as usize,
                    // );

                    let pos = Some((*coord, *coord));

                    if let Some((pos, _)) = pos {
                        redraw = self.get_current_tool().tool_event(
                            ToolEvent::TileDrag(
                                vec2i(pos.x, pos.y),
                                vec2f(pos.x as f32, pos.y as f32),
                            ),
                            ToolContext::ThreeD,
                            ui,
                            ctx,
                            project,
                            server,
                            client,
                            server_ctx,
                        );
                    }
                    //}
                    //}
                }
            }*/
            // TheEvent::ContextMenuSelected(widget_id, item_id) => {
            //     if widget_id.name == "Render Button" {
            //         if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            //             if item_id.name == "Start Renderer" {
            //                 PRERENDERTHREAD.lock().unwrap().set_paused(false);
            //             } else if item_id.name == "Pause Renderer" {
            //                 PRERENDERTHREAD.lock().unwrap().set_paused(true);
            //             } else if item_id.name == "Restart Renderer" {
            //                 PRERENDERTHREAD.lock().unwrap().set_paused(false);
            //                 PRERENDERTHREAD
            //                     .lock()
            //                     .unwrap()
            //                     .render_region(region.clone(), None);
            //             }
            //             redraw = true;
            //         }
            //     }
            // }
            TheEvent::Custom(id, value) => {
                if id.name == "Set Tool" {
                    if let TheValue::Text(name) = value {
                        if let Some(tool_id) = self.get_game_tool_uuid_of_name(name) {
                            self.set_tool(tool_id, ui, ctx, project, server_ctx);
                            ctx.ui
                                .set_widget_state(name.into(), TheWidgetState::Selected);
                        }
                    }
                } else if id.name == "Update Geometry Overlay 3D" {
                    self.update_geometry_overlay_3d(project, server_ctx);
                    redraw = true;
                }
            }
            _ => {}
        }

        if !redraw {
            redraw = self
                .get_current_tool()
                .handle_event(event, ui, ctx, project, server_ctx);
        }

        self.enforce_builder_dock(ui, ctx, project, server_ctx);

        redraw
    }

    /// Returns the curently active tool.
    pub fn get_current_tool(&mut self) -> &mut Box<dyn Tool> {
        &mut self.game_tools[self.curr_game_tool]
    }

    /// Returns the curent editor tool.
    pub fn get_current_editor_tool(&mut self) -> &mut Box<dyn EditorTool> {
        &mut self.editor_tools[self.curr_editor_tool]
    }

    #[allow(clippy::too_many_arguments)]
    pub fn deactivte_tool(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        self.game_tools[self.curr_game_tool].tool_event(
            ToolEvent::DeActivate,
            ui,
            ctx,
            project,
            server_ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_tool(
        &mut self,
        tool_id: Uuid,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        let mut switched_tool = false;
        let layout_name = "Game Tool Params";
        let mut old_tool_index = 0;

        if self.editor_mode {
            // Handle editor tool switching
            for (index, tool) in self.editor_tools.iter().enumerate() {
                if tool.id().uuid == tool_id && index != self.curr_editor_tool {
                    switched_tool = true;
                    old_tool_index = self.curr_editor_tool;
                    self.curr_editor_tool = index;
                    redraw = true;
                }
            }
            if switched_tool {
                for (index, tool) in self.editor_tools.iter().enumerate() {
                    let state = if index == self.curr_editor_tool {
                        TheWidgetState::Selected
                    } else {
                        TheWidgetState::None
                    };
                    ctx.ui.set_widget_state(tool.id().name.clone(), state);
                }

                self.editor_tools[old_tool_index].deactivate();
                self.editor_tools[self.curr_editor_tool].activate();
                self.apply_editor_rgba_mode(ui, ctx);
            }
        } else {
            // Handle game tool switching
            for (index, tool) in self.game_tools.iter().enumerate() {
                if tool.id().uuid == tool_id && index != self.curr_game_tool {
                    switched_tool = true;
                    old_tool_index = self.curr_game_tool;
                    self.curr_game_tool = index;
                    redraw = true;
                }
            }
            if switched_tool {
                server_ctx.hover = (None, None, None);
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    if let Some(surface) = server_ctx.active_detail_surface.as_ref()
                        && let Some(profile_id) = surface.profile
                        && let Some(profile_map) = map.profiles.get_mut(&profile_id)
                    {
                        profile_map.clear_temp();
                    }
                    map.curr_grid_pos = None;
                    map.curr_grid_pos_3d = None;
                    map.clear_temp();
                }
                for tool in self.game_tools.iter() {
                    if tool.id().uuid != tool_id {
                        ctx.ui
                            .set_widget_state(tool.id().name.clone(), TheWidgetState::None);
                    }
                }
                self.game_tools[old_tool_index].tool_event(
                    ToolEvent::DeActivate,
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );

                // Switching game tools should collapse any maximized dock/editor view.
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Minimize Dock"),
                    TheValue::Empty,
                ));
            }

            if let Some(layout) = ui.get_hlayout(layout_name) {
                layout.clear();
                layout.set_reverse_index(None);
                ctx.ui.redraw_all = true;
            }

            self.get_current_tool()
                .tool_event(ToolEvent::Activate, ui, ctx, project, server_ctx);

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Tool Changed"),
                TheValue::Empty,
            ));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Mark Rusterix Dirty"),
                TheValue::Empty,
            ));
        }

        /*
        if let Some(layout) = ui.get_hlayout(layout_name) {
            if layout.widgets().is_empty() {
                // Add default widgets

                // let mut gb = TheGroupButton::new(TheId::named("2D3D Group"));
                // gb.add_text("2D Map".to_string());
                // gb.add_text("Mixed".to_string());
                // gb.add_text("3D Map".to_string());

                // match *RENDERMODE.lock().unwrap() {
                //     EditorDrawMode::Draw2D => gb.set_index(0),
                //     EditorDrawMode::DrawMixed => gb.set_index(1),
                //     EditorDrawMode::Draw3D => gb.set_index(2),
                // }

                // let mut time_slider = TheTimeSlider::new(TheId::named("Server Time Slider"));
                // time_slider.set_continuous(true);
                // time_slider.limiter_mut().set_max_width(400);
                // time_slider.set_value(TheValue::Time(self.server_time));

                let mut spacer = TheSpacer::new(TheId::empty());
                spacer.limiter_mut().set_max_width(30);

                let mut render_button = TheTraybarButton::new(TheId::named("Render Button"));
                render_button.set_text(self.render_button_text.clone());
                render_button.set_status_text("Controls the 3D background renderer. During rendering it displays how many tiles are left to render.");
                render_button.set_fixed_size(true);
                render_button.limiter_mut().set_max_width(80);

                render_button.set_context_menu(Some(TheContextMenu {
                    items: vec![
                        TheContextMenuItem::new(
                            "Start Renderer".to_string(),
                            TheId::named("Start Renderer"),
                        ),
                        TheContextMenuItem::new(
                            "Pause".to_string(),
                            TheId::named("Pause Renderer"),
                        ),
                        TheContextMenuItem::new(
                            "Restart".to_string(),
                            TheId::named("Restart Renderer"),
                        ),
                    ],
                    ..Default::default()
                }));

                //layout.add_widget(Box::new(gb));
                layout.add_widget(Box::new(spacer));
                //layout.add_widget(Box::new(time_slider));
                layout.add_widget(Box::new(render_button));
                layout.set_reverse_index(Some(1));
            }
        }*/

        ctx.ui.relayout = true;

        redraw
    }

    // Return the uuid given game tool.
    pub fn get_game_tool_uuid_of_name(&self, name: &str) -> Option<Uuid> {
        for tool in self.game_tools.iter() {
            if tool.id().name == name {
                return Some(tool.id().uuid);
            }
        }
        None
    }

    // Return the tool of the given name
    pub fn get_game_tool_of_name(&mut self, name: &str) -> Option<&mut Box<dyn Tool>> {
        for tool in self.game_tools.iter_mut() {
            if tool.id().name == name {
                return Some(tool);
            }
        }
        None
    }

    /// Update the overlay geometry.
    pub fn update_geometry_overlay_3d(
        &mut self,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return;
        }

        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.scene_handler.clear_overlay();
        // rusterix.scene_handler.vm.set_layer_activity_logging(true);

        // basis_vectors returns (forward, right, up)
        let (cam_forward, cam_right, cam_up) = rusterix.client.camera_d3.basis_vectors();
        let view_right = cam_right;
        let view_up = cam_up;
        let view_nudge = cam_forward * -0.002; // small toward-camera nudge to avoid z-fighting
        rusterix.client.scene.d3_overlay.clear();
        let thickness = 0.15;

        if let Some(region) = project.get_region_ctx(&server_ctx) {
            let map = &region.map;

            // Helper to draw a single world-space line into the overlay
            let push_line = |id: GeoId,
                             rusterix: &mut rusterix::Rusterix,
                             mut a: Vec3<f32>,
                             mut b: Vec3<f32>,
                             normal: Vec3<f32>,
                             selected: bool,
                             hovered: bool| {
                // Z-fight mitigation: nudge along CAMERA FORWARD, not the line normal
                if selected {
                    let extra_nudge = cam_forward * -0.004; // toward camera
                    a += extra_nudge;
                    b += extra_nudge;
                }

                let tile_id = if selected || hovered {
                    rusterix.scene_handler.selected
                } else {
                    rusterix.scene_handler.white
                };

                rusterix
                    .scene_handler
                    .overlay_3d
                    .add_line_3d(id, tile_id, a, b, thickness, normal, 100);
            };

            // Rect tool previews

            if let Some((top_left, bottom_right)) = map.curr_rectangle {
                let mut index = 0;
                let min = Vec2::new(
                    top_left.x.min(bottom_right.x),
                    top_left.y.min(bottom_right.y),
                );
                let max = Vec2::new(
                    top_left.x.max(bottom_right.x),
                    bottom_right.y.max(top_left.y),
                );

                let corners = [
                    Vec2::new(min.x, min.y),
                    Vec2::new(max.x, min.y),
                    Vec2::new(max.x, max.y),
                    Vec2::new(min.x, max.y),
                ];
                let color = rusterix.scene_handler.white;

                // Draw 4 edges (close the loop by wrapping 3→0) in 2D overlay
                for i in 0..4 {
                    let a = corners[i];
                    let b = corners[(i + 1) % 4];
                    rusterix.scene_handler.add_overlay_2d_line(
                        GeoId::Gizmo(index),
                        a,
                        b,
                        color,
                        100,
                    );
                    index += 1;
                }
            }

            if server_ctx.curr_map_tool_type == MapToolType::Rect {
                if let Some(terrain_id) = server_ctx.rect_terrain_id {
                    let mut index = 0;
                    let config = TerrainConfig::default();
                    let corners = TerrainGenerator::tile_outline_world(map, terrain_id, &config);
                    let n = TerrainGenerator::tile_normal(map, terrain_id, &config);

                    // Draw 4 edges (close the loop by wrapping 3→0)
                    for i in 0..4 {
                        let a = corners[i] + view_nudge;
                        let b = corners[(i + 1) % 4] + view_nudge;
                        push_line(GeoId::Unknown(index), &mut rusterix, a, b, n, false, false);
                        index += 1;
                    }
                } else if let Some(sector_id) = server_ctx.rect_sector_id_3d {
                    let mut index = 0;
                    for (_, surface) in &map.surfaces {
                        if surface.sector_id == sector_id {
                            let corners =
                                surface.tile_outline_world_local(server_ctx.rect_tile_id_3d, map);
                            let n = surface.plane.normal;

                            // Draw 4 edges (close the loop by wrapping 3→0)
                            for i in 0..4 {
                                let a = corners[i] + view_nudge;
                                let b = corners[(i + 1) % 4] + view_nudge;
                                push_line(
                                    GeoId::Unknown(index),
                                    &mut rusterix,
                                    a,
                                    b,
                                    n,
                                    false,
                                    false,
                                );
                                index += 1;
                            }
                        }
                    }
                }
            }

            if !server_ctx.show_editing_geometry {
                rusterix.scene_handler.set_overlay();
                return;
            }

            // Helper to draw a single vertex as a camera-facing billboard in the overlay
            let vertex_size_world = 0.24_f32; // slightly larger for visibility
            let push_vertex =
                |id: GeoId, p: Vec3<f32>, selected: bool, rusterix: &mut rusterix::Rusterix| {
                    let tile_id = if selected {
                        rusterix.scene_handler.selected
                    } else {
                        rusterix.scene_handler.white
                    };
                    rusterix.scene_handler.overlay_3d.add_billboard_3d(
                        id,
                        tile_id,
                        p,
                        view_right,
                        view_up,
                        vertex_size_world,
                        true,
                    );
                };

            let hover_point = server_ctx.hover_cursor_3d.map(|p| {
                if server_ctx.curr_map_tool_type == MapToolType::Linedef
                    || server_ctx.curr_map_tool_type == MapToolType::Vertex
                    || server_ctx.curr_map_tool_type == MapToolType::Sector
                {
                    server_ctx.snap_world_point_for_edit(map, p)
                } else {
                    p
                }
            });

            if server_ctx.curr_map_tool_type != MapToolType::Rect {
                if let Some(pos) = hover_point {
                    let yellow = rusterix.scene_handler.yellow;
                    rusterix.scene_handler.overlay_3d.add_billboard_3d(
                        GeoId::Triangle(1000),
                        yellow,
                        pos + view_nudge,
                        view_right,
                        view_up,
                        vertex_size_world,
                        true,
                    );
                }
            }

            let show_builder_selected_vertices =
                server_ctx.builder_tool_active && !map.selected_vertices.is_empty();
            if server_ctx.curr_map_tool_type == MapToolType::Vertex
                || show_builder_selected_vertices
            {
                let detail_surface = server_ctx
                    .active_detail_surface
                    .as_ref()
                    .or(server_ctx.hover_surface.as_ref())
                    .or(server_ctx.editing_surface.as_ref())
                    .cloned();
                let detail_vertex_mode = server_ctx.curr_map_tool_type == MapToolType::Vertex
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && detail_surface.is_some();

                if detail_vertex_mode {
                    if let Some(surface) = detail_surface
                        && let Some(profile_id) = surface.profile
                        && let Some(profile_map) = map.profiles.get(&profile_id)
                    {
                        for v in &profile_map.vertices {
                            let mut pos = surface.uv_to_world(Vec2::new(v.x, -v.y)) + view_nudge;
                            pos += Vec3::zero();
                            let selected = profile_map.selected_vertices.contains(&v.id)
                                || server_ctx.hover.0 == Some(v.id);
                            push_vertex(GeoId::Vertex(v.id), pos, selected, &mut rusterix);
                        }
                    }
                } else {
                    for v in map.vertices.iter() {
                        let Some(world_pos) = map.get_vertex_3d(v.id) else {
                            continue;
                        };
                        let mut pos = Vec3::new(world_pos.x, world_pos.y, world_pos.z);
                        pos += view_nudge;
                        let selected = map.selected_vertices.contains(&v.id)
                            || server_ctx.hover.0 == Some(v.id);

                        push_vertex(GeoId::Vertex(v.id), pos, selected, &mut rusterix);
                    }
                }
            } else {
                let detail_surface = server_ctx
                    .active_detail_surface
                    .as_ref()
                    .or(server_ctx.hover_surface.as_ref())
                    .or(server_ctx.editing_surface.as_ref())
                    .cloned();
                let skip_world_linedef_overlay = server_ctx.curr_map_tool_type
                    == MapToolType::Linedef
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && detail_surface.is_some();
                // Linedefs
                let show_builder_selected_linedefs =
                    server_ctx.builder_tool_active && !map.selected_linedefs.is_empty();
                if server_ctx.curr_map_tool_type == MapToolType::Linedef
                    || show_builder_selected_linedefs
                {
                    if server_ctx.curr_map_tool_type == MapToolType::Linedef
                        && let (Some(start), Some(end)) = (map.curr_grid_pos_3d, hover_point)
                    {
                        if start != end {
                            push_line(
                                GeoId::Unknown(5000),
                                &mut rusterix,
                                start + view_nudge,
                                end + view_nudge,
                                cam_forward,
                                false,
                                true,
                            );
                        }
                    }

                    if server_ctx.curr_map_tool_type == MapToolType::Linedef
                        && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    {
                        if let Some(surface) = detail_surface.clone() {
                            if let Some(profile_id) = surface.profile
                                && let Some(profile_map) = map.profiles.get(&profile_id)
                            {
                                for linedef in &profile_map.linedefs {
                                    let Some(start_vertex) = profile_map
                                        .vertices
                                        .iter()
                                        .find(|vertex| vertex.id == linedef.start_vertex)
                                    else {
                                        continue;
                                    };
                                    let Some(end_vertex) = profile_map
                                        .vertices
                                        .iter()
                                        .find(|vertex| vertex.id == linedef.end_vertex)
                                    else {
                                        continue;
                                    };

                                    let a = surface
                                        .uv_to_world(Vec2::new(start_vertex.x, -start_vertex.y))
                                        + view_nudge;
                                    let b = surface
                                        .uv_to_world(Vec2::new(end_vertex.x, -end_vertex.y))
                                        + view_nudge;

                                    let is_selected =
                                        profile_map.selected_linedefs.contains(&linedef.id);
                                    let is_hovered = server_ctx.hover.1 == Some(linedef.id);

                                    push_line(
                                        GeoId::Linedef(linedef.id),
                                        &mut rusterix,
                                        a,
                                        b,
                                        cam_forward,
                                        is_selected,
                                        is_hovered,
                                    );
                                }
                            }
                        }
                    }

                    if !skip_world_linedef_overlay {
                        for linedef in &map.linedefs {
                            let is_selected = map.selected_linedefs.contains(&linedef.id);
                            let is_hovered = server_ctx.hover.1 == Some(linedef.id);
                            let show_in_builder =
                                server_ctx.builder_tool_active && (is_selected || is_hovered);
                            if !linedef.sector_ids.is_empty() && !show_in_builder {
                                continue;
                            }

                            if let (Some(vs), Some(ve)) = (
                                map.get_vertex_3d(linedef.start_vertex),
                                map.get_vertex_3d(linedef.end_vertex),
                            ) {
                                let a = Vec3::new(vs.x, vs.y, vs.z) + view_nudge;
                                let b = Vec3::new(ve.x, ve.y, ve.z) + view_nudge;
                                let normal = cam_forward;

                                push_line(
                                    GeoId::Linedef(linedef.id),
                                    &mut rusterix,
                                    a,
                                    b,
                                    normal,
                                    is_selected,
                                    is_hovered,
                                );
                            }
                        }
                    }
                }

                // Sectors
                use std::collections::HashMap;
                #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
                struct EdgeKey {
                    v0: u32,
                    v1: u32,
                }
                impl EdgeKey {
                    fn from_vertices(a: u32, b: u32) -> Self {
                        if a < b {
                            EdgeKey { v0: a, v1: b }
                        } else {
                            EdgeKey { v0: b, v1: a }
                        }
                    }
                }
                #[derive(Clone)]
                struct EdgeInfo {
                    a: Vec3<f32>,
                    b: Vec3<f32>,
                    selected: bool,
                    hovered: bool,
                    rep_ld_id: u32, // representative linedef id for picking/hit-testing
                }
                let mut edge_accum: HashMap<EdgeKey, EdgeInfo> = HashMap::new();

                if server_ctx.curr_map_tool_type == MapToolType::Sector
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && let Some(surface) = server_ctx
                        .active_detail_surface
                        .as_ref()
                        .or(server_ctx.hover_surface.as_ref())
                        .or(server_ctx.editing_surface.as_ref())
                        .cloned()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get(&profile_id)
                {
                    for sector in &profile_map.sectors {
                        let sector_is_selected = profile_map.selected_sectors.contains(&sector.id);
                        let sector_is_hovered = server_ctx.hover.2 == Some(sector.id);
                        for &ld_id in &sector.linedefs {
                            let Some(linedef) = profile_map.find_linedef(ld_id) else {
                                continue;
                            };
                            let Some(start_vertex) = profile_map.find_vertex(linedef.start_vertex)
                            else {
                                continue;
                            };
                            let Some(end_vertex) = profile_map.find_vertex(linedef.end_vertex)
                            else {
                                continue;
                            };
                            let a = surface.uv_to_world(Vec2::new(start_vertex.x, -start_vertex.y))
                                + view_nudge;
                            let b = surface.uv_to_world(Vec2::new(end_vertex.x, -end_vertex.y))
                                + view_nudge;
                            push_line(
                                GeoId::Sector(sector.id),
                                &mut rusterix,
                                a,
                                b,
                                cam_forward,
                                sector_is_selected,
                                sector_is_hovered,
                            );
                        }
                    }
                }

                let skip_world_sector_overlay = server_ctx.curr_map_tool_type
                    == MapToolType::Sector
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && server_ctx.active_detail_surface.is_some();

                for surface in map.surfaces.values() {
                    if skip_world_sector_overlay {
                        continue;
                    }
                    let sector_id = surface.sector_id;
                    let Some(sector) = map.find_sector(sector_id) else {
                        continue;
                    };
                    let sector_is_selected = map.selected_sectors.contains(&sector_id);

                    if sector.properties.contains("rect") && server_ctx.no_rect_geo_on_map {
                        continue;
                    }

                    let nudge = view_nudge; // consistent camera-side nudge avoids opposite-face z-fight

                    if let Some(points3) = sector.vertices_world(map) {
                        let n_pts = points3.len();
                        let n_ld = sector.linedefs.len();
                        let n = n_pts.min(n_ld);
                        if n >= 2 {
                            for i in 0..n {
                                let a = points3[i] + nudge;
                                let b = points3[(i + 1) % n_pts] + nudge;
                                let ld_id = sector.linedefs[i];

                                let mut line_is_selected = false;

                                if server_ctx.curr_map_tool_type == MapToolType::Linedef
                                    || server_ctx.curr_map_tool_type == MapToolType::Selection
                                {
                                    line_is_selected = map.selected_linedefs.contains(&ld_id)
                                        || server_ctx.hover.1 == Some(ld_id);
                                } else if server_ctx.curr_map_tool_type == MapToolType::Sector {
                                    line_is_selected =
                                        sector_is_selected || server_ctx.hover.2 == Some(sector_id);
                                };

                                // Build unordered edge key from linedef vertices, fallback if not found
                                let key = if let Some(ld_ref) = map.find_linedef(ld_id) {
                                    EdgeKey::from_vertices(ld_ref.start_vertex, ld_ref.end_vertex)
                                } else {
                                    // Fallback: build a key from the nearest map vertices to a/b (should be rare)
                                    continue;
                                };

                                edge_accum
                                    .entry(key)
                                    .and_modify(|e| {
                                        e.selected |= line_is_selected;
                                        e.hovered |= server_ctx.hover.1 == Some(ld_id);
                                        e.a = a;
                                        e.b = b; // keep latest endpoints
                                    })
                                    .or_insert(EdgeInfo {
                                        a,
                                        b,
                                        selected: line_is_selected,
                                        hovered: server_ctx.hover.1 == Some(ld_id),
                                        rep_ld_id: ld_id,
                                    });
                            }
                        }
                    }
                }

                // Emit deduplicated edges
                for (_key, e) in edge_accum.into_iter() {
                    push_line(
                        // &mut overlay_batches,
                        // GeometrySource::Linedef(e.rep_ld_id),
                        GeoId::Linedef(e.rep_ld_id),
                        &mut rusterix,
                        e.a,
                        e.b,
                        cam_forward,
                        e.selected,
                        e.hovered,
                    );
                }
            }

            // Flush final overlay batches: draw normal overlays first, then highlighted front overlays last
            // for batch in overlay_batches.drain(..) {
            //     rusterix.client.scene.d3_overlay.push(batch);
            // }
            // for batch in overlay_batches_front.drain(..) {
            //     rusterix.client.scene.d3_overlay.push(batch);
            // }
        }

        rusterix.scene_handler.set_overlay();
    }
    /*
    pub fn hitpoint_to_editing_coord(
        &mut self,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        hp: Vec3<f32>,
    ) -> Option<Vec2<f32>> {
        let mut rc = None;

        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.client.scene.d3_overlay.clear();

        if let Some(region) = project.get_region_ctx(&server_ctx) {
            // Meta provides world-space normal and the span (region 2D) for wall profiles
            //let (_, span) = server_ctx.get_region_map_meta_data(region);

            if span.is_none() {
                rc = Some(Vec2::new(hp.x, hp.z));
            } else {
                // PROFILE MAP: convert world hitpoint to (x,y) in profile space
                // 1) Find owning linedef
                let mut owner_linedef_opt = None;
                for ld in &region.map.linedefs {
                    if Some(ld.id) == server_ctx.profile_view {
                        owner_linedef_opt = Some(ld);
                        break;
                    }
                }
                if owner_linedef_opt.is_none() {
                    return rc;
                }
                let linedef = owner_linedef_opt.unwrap();

                // 2) Span basis
                let (p0, p1) = span.unwrap();
                let delta = p1 - p0;
                let len = delta.magnitude();
                if len <= 1e-6 {
                    return rc;
                }
                let dir = delta / len; // along wall (world XZ)

                // 3) Base elevation from front sector (default 0.0)
                let base_elevation = if let Some(front_id) = linedef.front_sector {
                    if let Some(front) = region.map.sectors.get(front_id as usize) {
                        front.properties.get_float_default("floor_height", 0.0)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                // 4) Inward offset used during placement; subtract before projecting
                let inward = Vec2::new(-dir.y, dir.x);
                let eps = linedef
                    .properties
                    .get_float("profile_wall_epsilon")
                    .unwrap_or(0.001);
                let offset2 = if linedef.front_sector.is_some() {
                    inward * eps
                } else if linedef.back_sector.is_some() {
                    inward * -eps
                } else {
                    Vec2::new(0.0, 0.0)
                };

                // 5) Determine profile left/right anchors
                let profile = &linedef.profile;
                let mut left_x = f32::INFINITY;
                let mut right_x = f32::NEG_INFINITY;
                for v in &profile.vertices {
                    if let Some(id) = v.properties.get_int("profile_id") {
                        match id {
                            1 | 2 => left_x = left_x.min(v.x),
                            0 | 3 => right_x = right_x.max(v.x),
                            _ => {}
                        }
                    }
                }
                if !left_x.is_finite() || !right_x.is_finite() {
                    left_x = f32::INFINITY;
                    right_x = f32::NEG_INFINITY;
                    for v in &profile.vertices {
                        left_x = left_x.min(v.x);
                        right_x = right_x.max(v.x);
                    }
                }
                let width = (right_x - left_x).max(1e-6);

                // 6) Project hitpoint onto span to get t in [0,1]
                let pos2 = Vec2::new(hp.x, hp.z) - offset2; // undo inward offset
                let t = ((pos2 - p0).dot(dir) / len).clamp(0.0, 1.0);
                let x2d = left_x + t * width;

                // 7) Y in profile space
                let y2d = hp.y - base_elevation;

                rc = Some(Vec2::new(x2d, y2d));
            }
        }

        rc
    }*/

    /// Get the geometry hit at the given screen position.
    fn get_geometry_hit(
        &self,
        render_view: &dyn TheRenderViewTrait,
        coord: Vec2<i32>,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> Option<(GeoId, Vec3<f32>)> {
        let dim = *render_view.dim();

        let screen_uv = [
            coord.x as f32 / dim.width as f32,
            coord.y as f32 / dim.height as f32,
        ];

        let mut rusterix = RUSTERIX.write().unwrap();

        server_ctx.hover_cursor_3d = None;
        server_ctx.hover_ray_dir_3d = None;
        server_ctx.hover_surface = None;
        server_ctx.hover_surface_hit_pos = None;
        if let Some((_, ray_dir)) = rusterix.scene_handler.vm.ray_from_uv_with_size(
            dim.width as u32,
            dim.height as u32,
            screen_uv,
        ) {
            server_ctx.hover_ray_dir_3d = Some(ray_dir);
        }

        rusterix.scene_handler.vm.set_active_vm(0);
        if let Some(raw) = rusterix.scene_handler.vm.pick_geo_id_at_uv(
            dim.width as u32,
            dim.height as u32,
            screen_uv,
            false,
            false,
        ) {
            server_ctx.hover_cursor_3d = Some(raw.1);
            if let Some(map) = project.get_map(server_ctx) {
                let mut best_surface: Option<(Surface, f32)> = None;
                for surface in map.surfaces.values() {
                    let n = surface.plane.normal;
                    let n_len = n.magnitude();
                    if n_len <= 1e-6 {
                        continue;
                    }

                    let signed_dist = (raw.1 - surface.plane.origin).dot(n / n_len);
                    let dist = signed_dist.abs();
                    if best_surface
                        .as_ref()
                        .map(|(_, best_dist)| dist < *best_dist)
                        .unwrap_or(true)
                    {
                        best_surface = Some((surface.clone(), dist));
                    }
                }
                server_ctx.hover_surface = best_surface.map(|(surface, _)| surface);
                server_ctx.hover_surface_hit_pos = Some(raw.1);
            }
            if server_ctx.curr_map_tool_type == MapToolType::Sector
                && server_ctx.geometry_edit_mode != GeometryEditMode::Detail
            {
                return Some((raw.0, raw.1));
            }
        }

        if server_ctx.curr_map_tool_type != MapToolType::Sector {
            rusterix.scene_handler.vm.set_active_vm(2);
        }

        let rc = rusterix.scene_handler.vm.pick_geo_id_at_uv(
            dim.width as u32,
            dim.height as u32,
            screen_uv,
            false,
            false,
        );

        rusterix.scene_handler.vm.set_active_vm(0);

        rc.map(|(geo_id, pos, _)| (geo_id, pos))
    }
}
