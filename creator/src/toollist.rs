use crate::actions::geometry_face_ops::surface_segment_points;
use crate::editor::{ACTIONLIST, DOCKMANAGER, RUSTERIX, SCENEMANAGER, SIDEBARMODE, UNDOMANAGER};
use crate::editor_display::{
    EditorDisplay, EditorDisplayOwner, EditorDisplayResult, EditorDisplaySession,
};
use crate::prelude::*;
use crate::shortcuts::{ShortcutAction, ShortcutContext, ShortcutResolution, ShortcutResolver};
use crate::sidebar::SidebarMode;
pub use crate::tools::rect::RectTool;
use rusterix::Assets;
use rusterix::ChunkBuilder;
use rusterix::D3Camera;
use rusterix::PixelSource;
use rusterix::Surface;
use rusterix::TopologyBuilder;
use rusterix::TopologyScene;
use rusterix::chunkbuilder::terrain_generator::{TerrainConfig, TerrainGenerator};
use scenevm::GeoId;
use std::time::Instant;

fn rect_stroke_streams_scene_update(tool_type: MapToolType, view_mode: EditorViewMode) -> bool {
    tool_type == MapToolType::Rect && view_mode == EditorViewMode::D2
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    pub command_id: String,
}

pub struct ToolList {
    pub server_time: TheTime,
    pub render_button_text: String,
    pub authoring_mode: bool,
    pub text_game_mode: bool,
    pub palette_mode: bool,
    pub previous_sidebar_mode: Option<SidebarMode>,
    pub previous_palette_dock: Option<String>,

    pub game_tools: Vec<Box<dyn Tool>>,
    pub curr_game_tool: usize,
    game_tool_descriptors: FxHashMap<Uuid, ToolDescriptor>,
    game_tool_command_ids: FxHashMap<String, Uuid>,
    previous_non_game_tool: Option<usize>,

    // Editor tools for dock editors
    pub editor_tools: Vec<Box<dyn EditorTool>>,
    pub curr_editor_tool: usize,
    pub editor_mode: bool,
    /// Full-screen Prefab editing uses the normal geometry implementations,
    /// but exposes only the tools that make sense inside an isolated asset.
    pub prefab_mode: bool,
    props_3d_available: bool,
    last_3d_hover_pick_at: Option<Instant>,
    last_3d_overlay_update_at: Option<Instant>,
    prefab_preview_wire_rgb: [f32; 3],
    prefab_grid_rgb: [f32; 3],
    box_select_armed: bool,
    pub(crate) native_world_input: bool,
    editor_display_session: Option<EditorDisplaySession>,
    #[cfg(feature = "native-ui")]
    native_display_mode: bool,
    #[cfg(feature = "native-ui")]
    native_document: Option<Uuid>,
    #[cfg(target_os = "macos")]
    macos_primary_orbit_drag: Option<MacOsPrimaryOrbitDrag>,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug)]
struct MacOsPrimaryOrbitDrag {
    origin: Vec2<i32>,
    from_viewcube: bool,
    orbiting: bool,
}

struct GeometryOwnerReplacementPlan {
    owners: FxHashSet<GeoId>,
    chunk_origins: FxHashSet<(i32, i32)>,
    include_terrain: bool,
}

#[derive(Clone, Default)]
struct GeometrySelectionSnapshot {
    objects: Vec<Uuid>,
    vertices: Vec<(Uuid, usize)>,
    faces: Vec<(Uuid, usize)>,
    surface_points: Vec<(Uuid, usize, usize)>,
    surface_segments: Vec<(Uuid, usize, usize)>,
}

impl Default for ToolList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with_rect_cell(x: f32, y: f32) -> Map {
        let mut map = Map::default();
        let v0 = map.add_vertex_at(x, y);
        let v1 = map.add_vertex_at(x, y + 1.0);
        let v2 = map.add_vertex_at(x + 1.0, y + 1.0);
        let v3 = map.add_vertex_at(x + 1.0, y);
        let _ = map.create_linedef_manual(v0, v1);
        let _ = map.create_linedef_manual(v1, v2);
        let _ = map.create_linedef_manual(v2, v3);
        let _ = map.create_linedef_manual(v3, v0);
        let sector_id = map.close_polygon_manual().expect("close rect sector");
        let sector = map.find_sector_mut(sector_id).unwrap();
        sector.properties.set("rect", Value::Bool(true));
        sector
            .properties
            .set("source", Value::Source(PixelSource::TileId(Uuid::new_v4())));
        sector.layer = Some(1);
        map
    }

    #[test]
    fn only_2d_rect_strokes_stream_their_commit_update() {
        assert!(rect_stroke_streams_scene_update(
            MapToolType::Rect,
            EditorViewMode::D2
        ));
        assert!(!rect_stroke_streams_scene_update(
            MapToolType::Rect,
            EditorViewMode::Iso
        ));
        assert!(!rect_stroke_streams_scene_update(
            MapToolType::Selection,
            EditorViewMode::D2
        ));
    }

    #[test]
    fn prefab_context_routes_geometry_tools_to_isolated_map() {
        let mut project = Project::default();
        let region_name = project.regions[0].map.name.clone();
        let mut prefab_map = Map::default();
        prefab_map.name = "Isolated Prefab".to_string();
        project.prefab_editor_map = Some(prefab_map);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Prefab(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Orbit;
        server_ctx.geometry_edit_mode = GeometryEditMode::Detail;

        ToolList::get_tool_map_mut(&mut project, &server_ctx)
            .unwrap()
            .name = "Edited Prefab".to_string();

        assert_eq!(
            project.prefab_editor_map.as_ref().unwrap().name,
            "Edited Prefab"
        );
        assert_eq!(project.regions[0].map.name, region_name);
    }

    #[test]
    fn prefab_palette_contains_geometry_and_paint_tools() {
        assert!(ToolList::is_prefab_tool_command_id("tool.geometry"));
        assert!(ToolList::is_prefab_tool_command_id("tool.vertex"));
        assert!(ToolList::is_prefab_tool_command_id("tool.linedef"));
        assert!(ToolList::is_prefab_tool_command_id("tool.sector"));
        assert!(ToolList::is_prefab_tool_command_id("tool.iso_paint"));
        assert!(ToolList::is_prefab_tool_command_id("tool.tile_picker"));
        assert!(ToolList::is_prefab_tool_command_id("tool.effects"));
        assert!(!ToolList::is_prefab_tool_command_id("tool.wall"));
        assert!(!ToolList::is_prefab_tool_command_id("tool.terrain"));
        assert!(!ToolList::is_prefab_tool_command_id("tool.entity"));
        assert!(!ToolList::is_prefab_tool_command_id("tool.game"));
    }

    #[test]
    fn prefab_authoring_tools_are_only_available_in_prefab_mode() {
        let mut tools = ToolList::new();
        assert!(!tools.game_tool_is_available("tool.tile_picker"));
        assert!(!tools.game_tool_is_available("tool.effects"));
        tools.prefab_mode = true;
        assert!(tools.game_tool_is_available("tool.tile_picker"));
        assert!(tools.game_tool_is_available("tool.effects"));
    }

    #[test]
    fn prefab_geometry_input_uses_the_isolated_render_view() {
        let mut server_ctx = ServerContext::default();
        assert_eq!(
            crate::utils::map_editor_render_view_name(&server_ctx),
            "PolyView"
        );

        server_ctx.pc = ProjectContext::Prefab(Uuid::new_v4());
        assert_eq!(
            crate::utils::map_editor_render_view_name(&server_ctx),
            "PrefabView"
        );
    }

    #[test]
    fn stable_tool_registry_covers_every_game_tool() {
        let tools = ToolList::new();
        assert_eq!(tools.game_tools.len(), tools.game_tool_descriptors.len());
        assert_eq!(tools.game_tools.len(), tools.game_tool_command_ids.len());
        for tool in &tools.game_tools {
            let descriptor = tools.game_tool_descriptor_by_id(tool.id().uuid).unwrap();
            assert!(validate_command_id(&descriptor.command_id).is_ok());
            assert_eq!(
                tools.get_game_tool_uuid_by_command_id(&descriptor.command_id),
                Some(tool.id().uuid)
            );
        }
    }

    #[test]
    fn creator_starts_with_the_sector_face_tool() {
        let tools = ToolList::new();
        assert_eq!(tools.current_game_tool_command_id(), Some("tool.sector"));
    }

    #[test]
    fn terrain_brush_replaces_the_wall_tool_in_the_region_palette() {
        let tools = ToolList::new();
        assert!(tools.game_tool_is_available("tool.terrain"));
        assert!(
            tools
                .get_game_tool_uuid_by_command_id("tool.wall")
                .is_none()
        );
    }

    #[test]
    fn d2_rect_undo_invalidates_added_and_removed_cell_chunks() {
        let empty = Map::default();
        let painted = map_with_rect_cell(-33.0, -33.0);

        let added = ToolList::changed_d2_rect_paint_chunks(&empty, &painted).unwrap();
        let removed = ToolList::changed_d2_rect_paint_chunks(&painted, &empty).unwrap();

        assert!(added.contains(&(-64, -64)));
        assert!(removed.contains(&(-64, -64)));
        assert!(ToolList::changed_d2_rect_paint_chunks(&painted, &painted).is_none());

        let custom_chunks =
            ToolList::changed_d2_rect_paint_chunks_with_size(&empty, &painted, 16).unwrap();
        assert!(custom_chunks.contains(&(-48, -48)));
    }
}

impl ToolList {
    const AUTHORING_BUTTON_NAME: &'static str = "Authoring";
    const TEXT_PLAY_BUTTON_NAME: &'static str = "Text Play";
    const PALETTE_BUTTON_NAME: &'static str = "Palette Mode";
    const DEFAULT_GAME_TOOL_COMMAND_ID: &'static str = "tool.sector";
    const GRID_SUBDIVISIONS: [f32; 6] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0];

    pub fn reset_for_project_switch(&mut self, ctx: &mut TheContext) {
        self.cancel_editor_display_session();
        self.authoring_mode = false;
        self.text_game_mode = false;
        self.palette_mode = false;
        self.previous_sidebar_mode = None;
        self.previous_palette_dock = None;
        self.last_3d_hover_pick_at = None;
        self.last_3d_overlay_update_at = None;
        self.previous_non_game_tool = None;
        self.prefab_mode = false;
        self.box_select_armed = false;
        #[cfg(target_os = "macos")]
        {
            self.macos_primary_orbit_drag = None;
        }
        ctx.ui.set_widget_state(
            Self::AUTHORING_BUTTON_NAME.to_string(),
            TheWidgetState::None,
        );
        ctx.ui.set_widget_state(
            Self::TEXT_PLAY_BUTTON_NAME.to_string(),
            TheWidgetState::None,
        );
        ctx.ui
            .set_widget_state(Self::PALETTE_BUTTON_NAME.to_string(), TheWidgetState::None);
    }

    fn update_editor_display_owner(
        &mut self,
        owner: EditorDisplayOwner,
        expected: &EditorDisplay,
        display: &EditorDisplay,
    ) -> bool {
        match owner {
            EditorDisplayOwner::Action(action_id) => ACTIONLIST
                .write()
                .unwrap()
                .get_action_by_id_mut(action_id)
                .is_some_and(|action| {
                    action.editor_display().as_ref() == Some(expected)
                        && action.update_editor_display(display)
                }),
            EditorDisplayOwner::Tool(tool_id) => self
                .game_tools
                .iter_mut()
                .find(|tool| tool.id().uuid == tool_id)
                .is_some_and(|tool| {
                    tool.editor_display().as_ref() == Some(expected)
                        && tool.update_editor_display(display)
                }),
        }
    }

    fn cancel_editor_display_session(&mut self) {
        if let Some(session) = self.editor_display_session.take() {
            self.update_editor_display_owner(
                session.owner,
                session.working_display(),
                session.original_display(),
            );
        }
    }

    /// Begin or replace the optional full-display editor owned by an action.
    ///
    /// `ProjectContext` is captured so the same host can safely serve the regular map view and
    /// isolated Prefab view without leaking a session between them.
    pub fn begin_action_edit(
        &mut self,
        action_id: Uuid,
        project_context: ProjectContext,
        display: Option<EditorDisplay>,
        ctx: &mut TheContext,
    ) {
        self.begin_editor_display(
            EditorDisplayOwner::Action(action_id),
            project_context,
            display,
            ctx,
        );
    }

    pub fn begin_tool_edit(
        &mut self,
        tool_id: Uuid,
        project_context: ProjectContext,
        display: Option<EditorDisplay>,
        ctx: &mut TheContext,
    ) {
        self.begin_editor_display(
            EditorDisplayOwner::Tool(tool_id),
            project_context,
            display,
            ctx,
        );
    }

    fn begin_editor_display(
        &mut self,
        owner: EditorDisplayOwner,
        project_context: ProjectContext,
        display: Option<EditorDisplay>,
        ctx: &mut TheContext,
    ) {
        if self
            .editor_display_session
            .as_ref()
            .is_some_and(|session| {
                session.is_for(owner, project_context)
                    && display.as_ref() == Some(session.working_display())
            })
        {
            return;
        }
        self.cancel_editor_display_session();
        if let Some(display) = display {
            let status_text = display.status_text().to_string();
            self.editor_display_session = Some(EditorDisplaySession::new_for_owner(
                owner,
                project_context,
                display,
            ));
            ctx.ui.set_focus(&TheId::named(
                crate::utils::map_editor_render_view_name_for_context(project_context),
            ));
            ctx.ui
                .send(TheEvent::SetStatusText(TheId::empty(), status_text));
            ctx.ui.redraw_all = true;
        }
    }

    fn handle_editor_display_event(
        &mut self,
        event: &TheEvent,
        ui: &TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        #[cfg(feature = "native-ui")]
        if self.native_display_mode {
            // Native viewport input has already been routed by the owning
            // document. Keep the real Apply button's established close behavior.
            if self.editor_display_session.as_ref().is_some_and(|session|
                matches!(session.owner, EditorDisplayOwner::Action(_)))
                && matches!(event, TheEvent::StateChanged(id, _)
                    if id.name == "Action Apply" || id.name == "Project Action Apply")
            {
                self.editor_display_session = None;
            }
            return false;
        }
        if self
            .editor_display_session
            .as_ref()
            .is_some_and(|session| session.project_context != server_ctx.pc)
        {
            self.cancel_editor_display_session();
            return false;
        }

        if self.editor_display_session.as_ref().is_some_and(|session| {
            matches!(session.owner, EditorDisplayOwner::Action(_))
                && matches!(
                    event,
                    TheEvent::StateChanged(id, _)
                        if id.name == "Action Apply" || id.name == "Project Action Apply"
                )
        }) {
            self.editor_display_session = None;
            return false;
        }

        let Some((owner, expected, result)) = self.editor_display_session.as_mut().map(|session| {
            let expected = session.working_display().clone();
            if let Some(map) = project.get_map(server_ctx) {
                session.set_grid_subdivisions(map.subdivisions);
            }
            let result = session.handle_event_with_modifiers(
                event,
                crate::utils::map_editor_render_view_name(server_ctx),
                ui.logo || ui.ctrl,
            );
            (session.owner, expected, result)
        }) else {
            return false;
        };
        match result {
            EditorDisplayResult::Ignored => false,
            EditorDisplayResult::Handled => {
                ctx.ui.redraw_all = true;
                true
            }
            EditorDisplayResult::DisplayChanged(display) => {
                if !self.update_editor_display_owner(owner, &expected, &display) {
                    self.editor_display_session = None;
                }
                ctx.ui.redraw_all = true;
                true
            }
            EditorDisplayResult::GridSubdivisionChanged(subdivision) => {
                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.subdivisions = subdivision;
                }
                {
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix.set_dirty();
                    rusterix.set_overlay_dirty();
                }
                ctx.ui.redraw_all = true;
                true
            }
            EditorDisplayResult::Commit => {
                self.editor_display_session = None;
                match owner {
                    EditorDisplayOwner::Action(_) => {
                        ctx.ui.send(TheEvent::StateChanged(
                            TheId::named("Action Apply"),
                            TheWidgetState::Clicked,
                        ));
                    }
                    EditorDisplayOwner::Tool(tool_id) => {
                        if let Some(info) = self
                            .game_tools
                            .iter()
                            .find(|tool| tool.id().uuid == tool_id)
                            .map(|tool| tool.info())
                        {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), info));
                        }
                        ctx.ui.redraw_all = true;
                    }
                }
                true
            }
            EditorDisplayResult::Cancel => {
                let session = self.editor_display_session.take().unwrap();
                self.update_editor_display_owner(
                    session.owner,
                    session.working_display(),
                    session.original_display(),
                );
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Editor display changes cancelled.".to_string(),
                ));
                ctx.ui.redraw_all = true;
                true
            }
        }
    }

    fn handle_action_hud_icon_click(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
        coord: Vec2<i32>,
    ) -> bool {
        if server_ctx.get_map_context() != MapContext::Region {
            return false;
        }
        let Some(map) = project.get_map(server_ctx) else {
            return false;
        };
        let Some(render_view) = crate::utils::map_editor_render_view(ui, server_ctx) else {
            return false;
        };

        let slot_count = if let Some(slots) =
            crate::actions::named_geometry_hud_material_slots_for_selection(map)
        {
            slots.len()
        } else if let Some(slots) =
            crate::actions::prefab_hud_material_slots_for_selected_instances(map)
        {
            slots.len()
        } else if let Some(action_id) = server_ctx.curr_action_id {
            let actionlist = ACTIONLIST.read().unwrap();
            actionlist
                .get_action_by_id(action_id)
                .and_then(|action| action.hud_material_slots(map, server_ctx))
                .map(|slots| slots.len())
                .unwrap_or(0)
        } else {
            0
        };
        if slot_count == 0 {
            return false;
        }

        let icon_size = 40;
        let dim = *render_view.dim();
        let x = dim.width as i32 - icon_size * slot_count as i32 - 1;
        for index in 0..slot_count as i32 {
            let rect = TheDim::rect(x + index * icon_size, 0, icon_size, 60);
            if rect.contains(coord) {
                server_ctx.selected_hud_icon_index = index;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
                ctx.ui.redraw_all = true;
                return true;
            }
        }
        false
    }

    fn handle_viewcube_click(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        coord: Vec2<i32>,
    ) -> bool {
        let Some(face) = self.viewcube_face_at(ui, server_ctx, coord) else {
            return false;
        };

        let region_center = if server_ctx.pc.is_prefab() {
            None
        } else {
            project
                .get_region(&server_ctx.curr_region)
                .map(|region| region.editing_position_3d)
        };
        let camera = {
            let mut edit_camera = crate::editor::EDITCAMERA.write().unwrap();
            crate::viewcube::ViewCube::snap_camera(&mut edit_camera.orbit_camera, face);
            edit_camera.reset_mouse_tracking();
            let mut camera = edit_camera.orbit_camera.clone();
            if let Some(center) = region_center {
                camera.center = center;
            }
            camera
        };
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.client.set_camera_d3(Box::new(camera));
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
        self.update_geometry_overlay_3d(project, server_ctx);
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            fl!("status_viewcube_camera_aligned"),
        ));
        ctx.ui.redraw_all = true;
        true
    }

    pub(crate) fn viewcube_navigation_enabled(&self) -> bool {
        !self.box_select_armed
    }

    fn viewcube_face_at(
        &mut self,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        coord: Vec2<i32>,
    ) -> Option<crate::viewcube::ViewCubeFace> {
        if self.native_world_input {
            return None;
        }
        if server_ctx.editor_view_mode != EditorViewMode::Orbit {
            return None;
        }
        let dim = crate::utils::map_editor_render_view(ui, server_ctx)
            .map(|render_view| *render_view.dim())?;
        let edit_camera = crate::editor::EDITCAMERA.read().unwrap();
        crate::viewcube::ViewCube::hit_test(coord, dim, &edit_camera.orbit_camera)
    }

    fn shortcut_context(map: Option<&Map>, server_ctx: &ServerContext) -> ShortcutContext {
        ShortcutContext {
            editor_view_mode: server_ctx.editor_view_mode,
            current_tool: server_ctx.curr_map_tool_type,
            has_geometry_objects: map
                .map(|map| !map.selected_geometry_objects.is_empty())
                .unwrap_or(false),
            has_geometry_vertices: map
                .map(|map| !map.selected_geometry_vertices.is_empty())
                .unwrap_or(false),
            has_geometry_faces: map
                .map(|map| !map.selected_geometry_faces.is_empty())
                .unwrap_or(false),
            has_surface_detail: map
                .map(|map| {
                    !map.selected_geometry_surface_points.is_empty()
                        || !map.selected_geometry_surface_segments.is_empty()
                })
                .unwrap_or(false),
        }
    }

    fn shortcut_tool_command_id(action: ShortcutAction) -> Option<&'static str> {
        match action {
            ShortcutAction::BoxSelect => None,
            ShortcutAction::ToolObject => Some("tool.geometry"),
            ShortcutAction::ToolVertex => Some("tool.vertex"),
            ShortcutAction::ToolEdge => Some("tool.linedef"),
            ShortcutAction::ToolFace => Some("tool.sector"),
            ShortcutAction::ToolIsoPaint => Some("tool.iso_paint"),
        }
    }

    fn shortcut_tool_uuid(&self, action: ShortcutAction) -> Option<Uuid> {
        self.get_game_tool_uuid_by_command_id(Self::shortcut_tool_command_id(action)?)
    }

    fn set_tool_widget_state_by_uuid(&mut self, uuid: Uuid, ctx: &mut TheContext) {
        if self.editor_mode {
            if self.curr_editor_tool < self.editor_tools.len() {
                ctx.ui.set_widget_state(
                    self.editor_tools[self.curr_editor_tool].id().name,
                    TheWidgetState::None,
                );
            }
            if let Some(tool) = self.editor_tools.iter().find(|tool| tool.id().uuid == uuid) {
                ctx.ui
                    .set_widget_state(tool.id().name, TheWidgetState::Selected);
            }
        } else {
            if self.curr_game_tool < self.game_tools.len() {
                ctx.ui.set_widget_state(
                    self.game_tools[self.curr_game_tool].id().name,
                    TheWidgetState::None,
                );
            }
            if let Some(tool) = self.game_tools.iter().find(|tool| tool.id().uuid == uuid) {
                ctx.ui
                    .set_widget_state(tool.id().name, TheWidgetState::Selected);
            }
        }
    }

    fn grid_subdivision_from_key(c: char) -> Option<f32> {
        match c {
            '1' | '!' => Some(1.0),
            '2' | '@' => Some(2.0),
            '3' | '#' => Some(4.0),
            '4' | '$' => Some(8.0),
            '5' | '%' => Some(16.0),
            '6' | '^' => Some(32.0),
            _ => None,
        }
    }

    fn is_grid_subdivision_key(c: char) -> bool {
        matches!(c, '1'..='6' | '!' | '@' | '#' | '$' | '%' | '^')
    }

    fn step_grid_subdivision(current: f32, delta: i32) -> f32 {
        let current_index = Self::GRID_SUBDIVISIONS
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| ((*a - current).abs()).total_cmp(&((*b - current).abs())))
            .map(|(index, _)| index as i32)
            .unwrap_or(0);
        let next_index =
            (current_index + delta).clamp(0, Self::GRID_SUBDIVISIONS.len() as i32 - 1) as usize;
        Self::GRID_SUBDIVISIONS[next_index]
    }

    fn get_tool_map_mut<'a>(
        project: &'a mut Project,
        server_ctx: &ServerContext,
    ) -> Option<&'a mut Map> {
        if !server_ctx.pc.is_prefab()
            && server_ctx.get_map_context() == MapContext::Region
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

    fn paste_position(server_ctx: &ServerContext) -> Option<Vec2<f32>> {
        server_ctx
            .hover_cursor
            .or_else(|| {
                server_ctx
                    .hover_cursor_3d
                    .map(|pos| Vec2::new(pos.x, pos.z))
            })
            .or_else(|| {
                server_ctx
                    .geo_hit
                    .map(|_| Vec2::new(server_ctx.geo_hit_pos.x, server_ctx.geo_hit_pos.z))
            })
    }

    #[allow(clippy::too_many_arguments)]
    /// End a viewport-owned authoring gesture through the ordinary controller
    /// and undo/source-update path before its presentation scope disappears.
    pub(crate) fn cancel_native_world_gesture(
        &mut self, ui: &mut TheUI, ctx: &mut TheContext,
        project: &mut Project, server: &mut ServerContext,
    ) {
        let undo = self.current_tool_map_event(MapEvent::MapEscape,ui,ctx,project,server);
        self.update_map_context(ui,ctx,project,server,undo);
        #[cfg(target_os="macos")]
        { self.macos_primary_orbit_drag = None; }
        crate::editor::EDITCAMERA.write().unwrap().reset_mouse_tracking();
        ctx.ui.redraw_all = true;
    }
    fn current_tool_map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if server_ctx.pc.is_prefab() {
            let ProjectContext::Prefab(asset_id) = server_ctx.pc else {
                return None;
            };
            let undo_atom = self
                .get_current_tool()
                .prefab_map_event(map_event, ui, ctx, project, asset_id, server_ctx);
            if undo_atom.is_some()
                && let Some(map) = project.prefab_editor_map.as_mut()
            {
                map.changed += 1;
            }
            return undo_atom;
        } else if server_ctx.get_map_context() == MapContext::Region {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                let terrain_drag = matches!(map_event, MapEvent::MapDragged(_))
                    && Self::selected_edit_affects_terrain(&region.map, server_ctx);
                let revision = region.map.changed;
                let undo_atom = self
                    .get_current_tool()
                    .region_map_event(map_event, ui, ctx, region, server_ctx);
                if terrain_drag {
                    // Tool drags may restore their initial map snapshot. Keep the
                    // shared terrain cache revision monotonic for live previews.
                    region.map.changed = revision.wrapping_add(1);
                }
                if undo_atom
                    .as_ref()
                    .is_some_and(|atom| !atom.is_iso_paint_only())
                {
                    region.map.changed += 1;
                }
                return undo_atom;
            }
        } else if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
            let undo_atom = self
                .get_current_tool()
                .map_event(map_event, ui, ctx, map, server_ctx);
            if undo_atom.is_some() {
                map.changed += 1;
            }
            return undo_atom;
        }
        None
    }

    fn geometry_selection_snapshot(map: &Map) -> GeometrySelectionSnapshot {
        GeometrySelectionSnapshot {
            objects: map.selected_geometry_objects.clone(),
            vertices: map.selected_geometry_vertices.clone(),
            faces: map.selected_geometry_faces.clone(),
            surface_points: map.selected_geometry_surface_points.clone(),
            surface_segments: map.selected_geometry_surface_segments.clone(),
        }
    }

    fn push_unique_uuid(ids: &mut Vec<Uuid>, id: Uuid) {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }

    fn push_unique_vertex(vertices: &mut Vec<(Uuid, usize)>, selection: (Uuid, usize)) {
        if !vertices.contains(&selection) {
            vertices.push(selection);
        }
    }

    fn push_unique_face(faces: &mut Vec<(Uuid, usize)>, selection: (Uuid, usize)) {
        if !faces.contains(&selection) {
            faces.push(selection);
        }
    }

    fn valid_geometry_object_ids(map: &Map, snapshot: &GeometrySelectionSnapshot) -> Vec<Uuid> {
        let mut ids = Vec::new();
        for id in &snapshot.objects {
            if map.geometry_objects.iter().any(|object| object.id == *id) {
                Self::push_unique_uuid(&mut ids, *id);
            }
        }
        for (id, _) in &snapshot.faces {
            if map.geometry_objects.iter().any(|object| object.id == *id) {
                Self::push_unique_uuid(&mut ids, *id);
            }
        }
        for (id, _) in &snapshot.vertices {
            if map.geometry_objects.iter().any(|object| object.id == *id) {
                Self::push_unique_uuid(&mut ids, *id);
            }
        }
        for (id, _, _) in &snapshot.surface_points {
            if map.geometry_objects.iter().any(|object| object.id == *id) {
                Self::push_unique_uuid(&mut ids, *id);
            }
        }
        for (id, _, _) in &snapshot.surface_segments {
            if map.geometry_objects.iter().any(|object| object.id == *id) {
                Self::push_unique_uuid(&mut ids, *id);
            }
        }
        ids
    }

    fn geometry_faces_from_snapshot(
        map: &Map,
        snapshot: &GeometrySelectionSnapshot,
        object_ids: &[Uuid],
    ) -> Vec<(Uuid, usize)> {
        let mut faces = Vec::new();
        for (object_id, face_index) in &snapshot.faces {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                && *face_index < object.faces.len()
            {
                Self::push_unique_face(&mut faces, (*object_id, *face_index));
            }
        }
        if faces.is_empty() {
            for object_id in object_ids {
                if let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == *object_id)
                {
                    for face_index in 0..object.faces.len() {
                        Self::push_unique_face(&mut faces, (*object_id, face_index));
                    }
                }
            }
        }
        faces
    }

    fn geometry_vertices_from_snapshot(
        map: &Map,
        snapshot: &GeometrySelectionSnapshot,
        object_ids: &[Uuid],
    ) -> Vec<(Uuid, usize)> {
        let mut vertices = Vec::new();
        for (object_id, vertex_index) in &snapshot.vertices {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                && *vertex_index < object.vertices.len()
            {
                Self::push_unique_vertex(&mut vertices, (*object_id, *vertex_index));
            }
        }
        if vertices.is_empty() {
            for (object_id, face_index) in &snapshot.faces {
                if let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == *object_id)
                    && let Some(face) = object.faces.get(*face_index)
                {
                    for vertex_index in &face.indices {
                        if *vertex_index < object.vertices.len() {
                            Self::push_unique_vertex(&mut vertices, (*object_id, *vertex_index));
                        }
                    }
                }
            }
        }
        if vertices.is_empty() {
            for object_id in object_ids {
                if let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == *object_id)
                {
                    for vertex_index in 0..object.vertices.len() {
                        Self::push_unique_vertex(&mut vertices, (*object_id, vertex_index));
                    }
                }
            }
        }
        vertices
    }

    fn apply_geometry_tool_selection_carryover(
        map: &mut Map,
        target_tool: MapToolType,
        snapshot: &GeometrySelectionSnapshot,
    ) -> bool {
        let object_ids = Self::valid_geometry_object_ids(map, snapshot);
        if object_ids.is_empty() {
            return false;
        }

        match target_tool {
            MapToolType::Selection => {
                map.geometry_selection_mode = 0;
                map.selected_geometry_objects = object_ids;
                map.selected_geometry_faces.clear();
                map.selected_geometry_vertices.clear();
                map.selected_geometry_surface_points.clear();
                map.selected_geometry_surface_segments.clear();
                true
            }
            MapToolType::Sector => {
                let faces = Self::geometry_faces_from_snapshot(map, snapshot, &object_ids);
                if faces.is_empty() {
                    return false;
                }
                map.geometry_selection_mode = 1;
                map.selected_geometry_objects = object_ids;
                map.selected_geometry_faces = faces;
                map.selected_geometry_vertices.clear();
                map.selected_geometry_surface_points.clear();
                map.selected_geometry_surface_segments.clear();
                true
            }
            MapToolType::Vertex | MapToolType::Linedef => {
                let vertices = Self::geometry_vertices_from_snapshot(map, snapshot, &object_ids);
                if vertices.is_empty() {
                    return false;
                }
                map.geometry_selection_mode = if target_tool == MapToolType::Vertex {
                    2
                } else {
                    3
                };
                map.selected_geometry_objects = object_ids;
                map.selected_geometry_vertices = vertices;
                map.selected_geometry_faces.clear();
                map.selected_geometry_surface_points.clear();
                map.selected_geometry_surface_segments.clear();
                true
            }
            _ => false,
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

    fn map_geometry_matches(old_map: &Map, new_map: &Map) -> bool {
        old_map.vertices.len() == new_map.vertices.len()
            && old_map.linedefs.len() == new_map.linedefs.len()
            && old_map.sectors.len() == new_map.sectors.len()
            && old_map
                .vertices
                .iter()
                .zip(new_map.vertices.iter())
                .all(|(old, new)| {
                    old.id == new.id && old.x == new.x && old.y == new.y && old.z == new.z
                })
            && old_map
                .linedefs
                .iter()
                .zip(new_map.linedefs.iter())
                .all(|(old, new)| {
                    old.id == new.id
                        && old.start_vertex == new.start_vertex
                        && old.end_vertex == new.end_vertex
                        && old.sector_ids == new.sector_ids
                })
            && old_map
                .sectors
                .iter()
                .zip(new_map.sectors.iter())
                .all(|(old, new)| old.id == new.id && old.linedefs == new.linedefs)
    }

    fn add_sector_dirty_chunks(map: &Map, sector_id: u32, chunks: &mut FxHashSet<(i32, i32)>) {
        let Some(sector) = map.find_sector(sector_id) else {
            return;
        };

        let mut bbox = sector.bounding_box(map);
        if Self::sector_affects_terrain(sector) {
            let radius = sector
                .properties
                .get_float_default("terrain_shape_blend", 0.)
                .max(4.);
            bbox.expand(Vec2::broadcast(radius * 2.));
        }
        Self::add_bbox_dirty_chunks(bbox, chunks);
    }

    fn add_linedef_dirty_chunks(map: &Map, linedef_id: u32, chunks: &mut FxHashSet<(i32, i32)>) {
        let Some(linedef) = map.find_linedef(linedef_id) else {
            return;
        };

        let mut bbox = linedef.bounding_box(map);
        if Self::linedef_affects_terrain(linedef) {
            let width = linedef
                .properties
                .get_float_default(
                    "terrain_shape_width",
                    linedef.properties.get_float_default("terrain_width", 2.0),
                )
                .max(0.0);
            let falloff = linedef
                .properties
                .get_float_default(
                    "terrain_shape_blend",
                    linedef
                        .properties
                        .get_float_default("terrain_tile_falloff", 1.0),
                )
                .max(0.0);
            bbox.expand(Vec2::broadcast((width + falloff + 8.0) * 2.0));
        } else {
            bbox.expand(Vec2::broadcast(1.0));
        }
        Self::add_bbox_dirty_chunks(bbox, chunks);
    }

    fn vertex_affects_terrain(vertex: &rusterix::Vertex) -> bool {
        vertex.properties.get_bool_default("terrain_control", false)
            || vertex.properties.contains("terrain_source")
    }

    fn linedef_affects_terrain(linedef: &rusterix::Linedef) -> bool {
        linedef.properties.get_bool_default("terrain_shape", false)
            || linedef.properties.contains("terrain_source")
            || linedef.properties.get_bool_default("terrain_smooth", false)
    }

    fn sector_affects_terrain(sector: &rusterix::Sector) -> bool {
        sector.properties.get_bool_default("terrain_shape", false)
            || sector.properties.get_int_default("terrain_mode", 0) != 0
            || sector.properties.contains("terrain_source")
            || sector.properties.get_bool_default("cutout_handle", false)
            || sector.properties.contains("linked_cutout_handle")
    }

    fn expand_cutout_related_sectors(map: &Map, sectors: &mut FxHashSet<u32>) {
        let mut cutout_handles = FxHashSet::default();
        let seeds = sectors.iter().copied().collect::<Vec<_>>();

        for sector_id in &seeds {
            if let Some(sector) = map.find_sector(*sector_id) {
                if sector.properties.get_bool_default("cutout_handle", false) {
                    cutout_handles.insert(*sector_id);
                }
                if let Some(handle_id) = sector.properties.get_int("linked_cutout_handle") {
                    cutout_handles.insert(handle_id as u32);
                }
                if let Some(handle_id) = sector.properties.get_int("host_sector") {
                    sectors.insert(handle_id as u32);
                }
            }
        }

        if cutout_handles.is_empty() {
            return;
        }

        for sector in &map.sectors {
            if let Some(handle_id) = sector.properties.get_int("linked_cutout_handle")
                && cutout_handles.contains(&(handle_id as u32))
            {
                sectors.insert(sector.id);
            }
            if let Some(host_id) = sector.properties.get_int("host_sector")
                && cutout_handles.contains(&sector.id)
            {
                sectors.insert(host_id as u32);
            }
        }
        sectors.extend(cutout_handles);
    }

    fn selected_edit_affects_terrain(map: &Map, server_ctx: &ServerContext) -> bool {
        match server_ctx.curr_map_tool_type {
            MapToolType::Vertex => {
                let topology = rusterix::MapTopology::build(map);
                map.selected_vertices
                    .iter()
                    .filter_map(|id| map.find_vertex(*id))
                    .any(Self::vertex_affects_terrain)
                    || map.linedefs.iter().any(|l| {
                        Self::linedef_affects_terrain(l)
                            && (map.selected_vertices.contains(&l.start_vertex)
                                || map.selected_vertices.contains(&l.end_vertex))
                    })
                    || topology
                        .sectors_for_vertices(map.selected_vertices.iter().copied())
                        .into_iter()
                        .filter_map(|id| map.find_sector(id))
                        .any(Self::sector_affects_terrain)
            }
            MapToolType::Linedef => {
                let topology = rusterix::MapTopology::build(map);
                map.selected_linedefs
                    .iter()
                    .filter_map(|id| map.find_linedef(*id))
                    .any(Self::linedef_affects_terrain)
                    || topology
                        .sectors_for_linedefs(map.selected_linedefs.iter().copied())
                        .into_iter()
                        .filter_map(|id| map.find_sector(id))
                        .any(Self::sector_affects_terrain)
            }
            MapToolType::Sector => map
                .selected_sectors
                .iter()
                .filter_map(|id| map.find_sector(*id))
                .any(Self::sector_affects_terrain),
            _ => false,
        }
    }

    fn changed_elements_affect_terrain(
        old_map: &Map,
        new_map: &Map,
        vertices: &FxHashSet<u32>,
        linedefs: &FxHashSet<u32>,
        sectors: &FxHashSet<u32>,
    ) -> bool {
        vertices.iter().any(|id| {
            old_map
                .find_vertex(*id)
                .is_some_and(Self::vertex_affects_terrain)
                || new_map
                    .find_vertex(*id)
                    .is_some_and(Self::vertex_affects_terrain)
        }) || linedefs.iter().any(|id| {
            old_map
                .find_linedef(*id)
                .is_some_and(Self::linedef_affects_terrain)
                || new_map
                    .find_linedef(*id)
                    .is_some_and(Self::linedef_affects_terrain)
        }) || sectors.iter().any(|id| {
            old_map
                .find_sector(*id)
                .is_some_and(Self::sector_affects_terrain)
                || new_map
                    .find_sector(*id)
                    .is_some_and(Self::sector_affects_terrain)
        })
    }

    fn strip_terrain_owners(owners: &mut FxHashSet<GeoId>) {
        owners.retain(|owner| !matches!(owner, GeoId::Terrain(_, _)));
    }

    fn add_owner_dirty_chunks(map: &Map, owner: GeoId, chunks: &mut FxHashSet<(i32, i32)>) {
        let chunk_size = 32;
        match owner {
            GeoId::Sector(sector_id) => Self::add_sector_dirty_chunks(map, sector_id, chunks),
            GeoId::Linedef(linedef_id) => Self::add_linedef_dirty_chunks(map, linedef_id, chunks),
            GeoId::Terrain(x, z) => {
                chunks.insert((
                    x.div_euclid(chunk_size) * chunk_size,
                    z.div_euclid(chunk_size) * chunk_size,
                ));
            }
            GeoId::GeometryObject(object_id) => {
                if let Some(bbox) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == object_id)
                    .and_then(|object| object.bbox())
                {
                    Self::add_bbox_dirty_chunks(bbox, chunks);
                }
            }
            _ => {}
        }
    }

    fn add_bbox_dirty_chunks(bbox: rusterix::BBox, chunks: &mut FxHashSet<(i32, i32)>) {
        if !bbox.min.x.is_finite()
            || !bbox.min.y.is_finite()
            || !bbox.max.x.is_finite()
            || !bbox.max.y.is_finite()
        {
            return;
        }

        let chunk_size = 32;
        let min_cx = (bbox.min.x / chunk_size as f32).floor() as i32;
        let min_cy = (bbox.min.y / chunk_size as f32).floor() as i32;
        let max_cx = (bbox.max.x / chunk_size as f32).ceil() as i32;
        let max_cy = (bbox.max.y / chunk_size as f32).ceil() as i32;
        for cy in min_cy..max_cy.max(min_cy + 1) {
            for cx in min_cx..max_cx.max(min_cx + 1) {
                chunks.insert((cx * chunk_size, cy * chunk_size));
            }
        }
    }

    fn expand_topology_owners_for_chunks(
        map: &Map,
        chunk_origins: &FxHashSet<(i32, i32)>,
        owners: &mut FxHashSet<GeoId>,
        include_terrain: bool,
    ) {
        let chunk_size = SCENEMANAGER.write().unwrap().chunk_size();
        let scene = TopologyScene::build(map);
        for origin in chunk_origins {
            let bbox = rusterix::BBox::from_pos_size(
                Vec2::new(origin.0 as f32, origin.1 as f32),
                Vec2::broadcast(chunk_size as f32),
            );
            let mut chunk_owners = scene.owners_for_chunk(map, &bbox);
            if !include_terrain {
                Self::strip_terrain_owners(&mut chunk_owners);
            }
            owners.extend(chunk_owners);
        }
    }

    fn expand_index_owners_for_chunks(
        chunk_origins: &FxHashSet<(i32, i32)>,
        owners: &mut FxHashSet<GeoId>,
        include_terrain: bool,
    ) {
        let rusterix = RUSTERIX.read().unwrap();
        for origin in chunk_origins {
            let mut chunk_owners = rusterix
                .scene_handler
                .build_index
                .owners_for_chunk(Vec2::new(origin.0, origin.1));
            if !include_terrain {
                Self::strip_terrain_owners(&mut chunk_owners);
            }
            owners.extend(chunk_owners);
        }
    }

    #[allow(dead_code)]
    fn changed_geometry_replacement_plan(
        old_map: &Map,
        new_map: &Map,
    ) -> Option<GeometryOwnerReplacementPlan> {
        let old_topology = rusterix::MapTopology::build(old_map);
        let new_topology = rusterix::MapTopology::build(new_map);
        let mut affected_sectors: FxHashSet<u32> = FxHashSet::default();
        let mut affected_linedefs: FxHashSet<u32> = FxHashSet::default();
        let mut affected_vertices: FxHashSet<u32> = FxHashSet::default();
        let mut affected_geometry_objects: FxHashSet<Uuid> = FxHashSet::default();
        let mut affected_prefab_instances: FxHashSet<Uuid> = FxHashSet::default();

        let old_vertices = old_map
            .vertices
            .iter()
            .map(|vertex| (vertex.id, vertex))
            .collect::<FxHashMap<_, _>>();
        let new_vertices = new_map
            .vertices
            .iter()
            .map(|vertex| (vertex.id, vertex))
            .collect::<FxHashMap<_, _>>();
        let mut vertex_ids = FxHashSet::default();
        vertex_ids.extend(old_vertices.keys().copied());
        vertex_ids.extend(new_vertices.keys().copied());

        for vertex_id in vertex_ids {
            match (old_vertices.get(&vertex_id), new_vertices.get(&vertex_id)) {
                (Some(old), Some(new))
                    if old.x == new.x
                        && old.y == new.y
                        && old.z == new.z
                        && old.properties == new.properties => {}
                _ => {
                    affected_vertices.insert(vertex_id);
                    for map in [old_map, new_map] {
                        affected_linedefs.extend(
                            map.linedefs
                                .iter()
                                .filter(|l| {
                                    l.start_vertex == vertex_id || l.end_vertex == vertex_id
                                })
                                .map(|l| l.id),
                        );
                    }
                    affected_sectors.extend(old_topology.sectors_for_vertices([vertex_id]));
                    affected_sectors.extend(new_topology.sectors_for_vertices([vertex_id]));
                }
            }
        }

        let old_linedefs = old_map
            .linedefs
            .iter()
            .map(|linedef| (linedef.id, linedef))
            .collect::<FxHashMap<_, _>>();
        let new_linedefs = new_map
            .linedefs
            .iter()
            .map(|linedef| (linedef.id, linedef))
            .collect::<FxHashMap<_, _>>();
        let mut linedef_ids = FxHashSet::default();
        linedef_ids.extend(old_linedefs.keys().copied());
        linedef_ids.extend(new_linedefs.keys().copied());

        for linedef_id in linedef_ids {
            match (old_linedefs.get(&linedef_id), new_linedefs.get(&linedef_id)) {
                (Some(old), Some(new))
                    if old.start_vertex == new.start_vertex
                        && old.end_vertex == new.end_vertex
                        && old.sector_ids == new.sector_ids
                        && old.properties == new.properties => {}
                _ => {
                    affected_linedefs.insert(linedef_id);
                    affected_sectors.extend(old_topology.sectors_for_linedefs([linedef_id]));
                    affected_sectors.extend(new_topology.sectors_for_linedefs([linedef_id]));
                }
            }
        }

        let old_sectors = old_map
            .sectors
            .iter()
            .map(|sector| (sector.id, sector))
            .collect::<FxHashMap<_, _>>();
        let new_sectors = new_map
            .sectors
            .iter()
            .map(|sector| (sector.id, sector))
            .collect::<FxHashMap<_, _>>();
        let mut sector_ids = FxHashSet::default();
        sector_ids.extend(old_sectors.keys().copied());
        sector_ids.extend(new_sectors.keys().copied());

        for sector_id in sector_ids {
            match (old_sectors.get(&sector_id), new_sectors.get(&sector_id)) {
                (Some(old), Some(new))
                    if old.linedefs == new.linedefs
                        && old.properties == new.properties
                        && old.layer == new.layer => {}
                _ => {
                    affected_sectors.insert(sector_id);
                }
            }
        }

        let old_geometry_objects = old_map
            .geometry_objects
            .iter()
            .map(|object| (object.id, object))
            .collect::<FxHashMap<_, _>>();
        let new_geometry_objects = new_map
            .geometry_objects
            .iter()
            .map(|object| (object.id, object))
            .collect::<FxHashMap<_, _>>();
        let mut geometry_object_ids = FxHashSet::default();
        geometry_object_ids.extend(old_geometry_objects.keys().copied());
        geometry_object_ids.extend(new_geometry_objects.keys().copied());

        for object_id in geometry_object_ids {
            match (
                old_geometry_objects.get(&object_id),
                new_geometry_objects.get(&object_id),
            ) {
                (Some(old), Some(new)) if *old == *new => {}
                _ => {
                    affected_geometry_objects.insert(object_id);
                }
            }
        }

        let old_prefab_instances = old_map
            .block_prop_instances
            .iter()
            .map(|instance| (instance.id, instance))
            .collect::<FxHashMap<_, _>>();
        let new_prefab_instances = new_map
            .block_prop_instances
            .iter()
            .map(|instance| (instance.id, instance))
            .collect::<FxHashMap<_, _>>();
        let mut prefab_instance_ids = FxHashSet::default();
        prefab_instance_ids.extend(old_prefab_instances.keys().copied());
        prefab_instance_ids.extend(new_prefab_instances.keys().copied());
        for instance_id in prefab_instance_ids {
            match (
                old_prefab_instances.get(&instance_id),
                new_prefab_instances.get(&instance_id),
            ) {
                (Some(old), Some(new)) if *old == *new => {}
                _ => {
                    affected_prefab_instances.insert(instance_id);
                }
            }
        }

        if affected_sectors.is_empty()
            && affected_linedefs.is_empty()
            && affected_vertices.is_empty()
            && affected_geometry_objects.is_empty()
            && affected_prefab_instances.is_empty()
        {
            return None;
        }

        Self::expand_cutout_related_sectors(old_map, &mut affected_sectors);
        Self::expand_cutout_related_sectors(new_map, &mut affected_sectors);

        let mut chunk_origins: FxHashSet<(i32, i32)> = FxHashSet::default();
        let mut owners = FxHashSet::default();
        owners.extend(old_topology.owners_for_vertices(old_map, affected_vertices.iter().copied()));
        owners.extend(new_topology.owners_for_vertices(new_map, affected_vertices.iter().copied()));
        owners.extend(old_topology.owners_for_linedefs(old_map, affected_linedefs.iter().copied()));
        owners.extend(new_topology.owners_for_linedefs(new_map, affected_linedefs.iter().copied()));
        owners.extend(old_topology.owners_for_sectors(old_map, affected_sectors.iter().copied()));
        owners.extend(new_topology.owners_for_sectors(new_map, affected_sectors.iter().copied()));
        owners.extend(
            affected_geometry_objects
                .iter()
                .copied()
                .map(GeoId::GeometryObject),
        );
        let prefab_assets = RUSTERIX.read().unwrap().assets.block_props.clone();
        let old_changed_instances = old_map
            .block_prop_instances
            .iter()
            .filter(|instance| affected_prefab_instances.contains(&instance.id))
            .cloned()
            .collect::<Vec<_>>();
        let new_changed_instances = new_map
            .block_prop_instances
            .iter()
            .filter(|instance| affected_prefab_instances.contains(&instance.id))
            .cloned()
            .collect::<Vec<_>>();
        let old_prefab_geometry =
            rusterix::resolve_block_prop_geometry(&old_changed_instances, &prefab_assets)
                .geometry_objects;
        let new_prefab_geometry =
            rusterix::resolve_block_prop_geometry(&new_changed_instances, &prefab_assets)
                .geometry_objects;
        owners.extend(
            old_prefab_geometry
                .iter()
                .chain(new_prefab_geometry.iter())
                .map(|object| GeoId::GeometryObject(object.id)),
        );
        let include_terrain = Self::changed_elements_affect_terrain(
            old_map,
            new_map,
            &affected_vertices,
            &affected_linedefs,
            &affected_sectors,
        );
        if !include_terrain {
            Self::strip_terrain_owners(&mut owners);
        }
        {
            let rusterix = RUSTERIX.read().unwrap();
            chunk_origins.extend(
                rusterix
                    .scene_handler
                    .build_index
                    .chunks_for_owners(owners.iter().copied()),
            );
        }

        // Also include old and new bounds. The build index gives us currently
        // streamed stale chunks; bounds cover movement into chunks that did not
        // previously contain this sector.
        for sector_id in &affected_sectors {
            Self::add_sector_dirty_chunks(old_map, *sector_id, &mut chunk_origins);
            Self::add_sector_dirty_chunks(new_map, *sector_id, &mut chunk_origins);
        }
        for linedef_id in &affected_linedefs {
            Self::add_linedef_dirty_chunks(old_map, *linedef_id, &mut chunk_origins);
            Self::add_linedef_dirty_chunks(new_map, *linedef_id, &mut chunk_origins);
        }
        for object_id in &affected_geometry_objects {
            if let Some(bbox) = old_geometry_objects
                .get(object_id)
                .and_then(|object| object.bbox())
            {
                Self::add_bbox_dirty_chunks(bbox, &mut chunk_origins);
            }
            if let Some(bbox) = new_geometry_objects
                .get(object_id)
                .and_then(|object| object.bbox())
            {
                Self::add_bbox_dirty_chunks(bbox, &mut chunk_origins);
            }
        }
        for object in old_prefab_geometry.iter().chain(new_prefab_geometry.iter()) {
            if let Some(bbox) = object.bbox() {
                Self::add_bbox_dirty_chunks(bbox, &mut chunk_origins);
            }
        }
        for owner in owners.iter().copied() {
            Self::add_owner_dirty_chunks(old_map, owner, &mut chunk_origins);
            Self::add_owner_dirty_chunks(new_map, owner, &mut chunk_origins);
        }
        Self::expand_topology_owners_for_chunks(
            old_map,
            &chunk_origins,
            &mut owners,
            include_terrain,
        );
        Self::expand_topology_owners_for_chunks(
            new_map,
            &chunk_origins,
            &mut owners,
            include_terrain,
        );
        Self::expand_index_owners_for_chunks(&chunk_origins, &mut owners, include_terrain);
        if !include_terrain {
            Self::strip_terrain_owners(&mut owners);
        }

        if chunk_origins.is_empty() || (!include_terrain && owners.is_empty()) {
            None
        } else {
            Some(GeometryOwnerReplacementPlan {
                owners,
                chunk_origins,
                include_terrain,
            })
        }
    }

    fn apply_geometry_owner_replacement(new_map: &Map, plan: GeometryOwnerReplacementPlan) -> bool {
        if plan.owners.is_empty() || plan.chunk_origins.is_empty() {
            return false;
        }

        let mut build_map = new_map.clone();
        if !plan.include_terrain {
            build_map.properties.remove("terrain_enabled");
        }
        build_map.changed = build_map.changed.wrapping_add(1);
        build_map.update_surfaces();

        let assets = RUSTERIX.read().unwrap().assets.clone();
        let chunk_size = SCENEMANAGER.write().unwrap().chunk_size();
        let mut builder = TopologyBuilder::new();
        let mut generated_chunks = Vec::new();

        for origin in plan.chunk_origins {
            let origin = Vec2::new(origin.0, origin.1);
            let mut chunk = rusterix::Chunk::new(origin, chunk_size);
            let mut vmchunk = scenevm::Chunk::new(origin, chunk_size);
            builder.build(&build_map, &assets, &mut chunk, &mut vmchunk);
            generated_chunks.push(vmchunk);
        }

        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix
                .scene_handler
                .replace_owner_geometry(&plan.owners, generated_chunks);
        }

        crate::utils::editor_scene_incremental_map_update(build_map, Vec::new());
        true
    }

    fn apply_geometry_scene_update(new_map: &Map, plan: GeometryOwnerReplacementPlan) -> bool {
        if plan
            .owners
            .iter()
            .any(|owner| matches!(owner, GeoId::GeometryObject(_)))
        {
            let mut build_map = new_map.clone();
            build_map.changed = build_map.changed.wrapping_add(1);
            build_map.update_surfaces();
            crate::utils::editor_scene_replace_incremental_map_update(
                build_map,
                plan.chunk_origins.into_iter().collect(),
            );
            return true;
        }
        Self::apply_geometry_owner_replacement(new_map, plan)
    }

    pub(crate) fn try_incremental_map_edit(
        old_map: &Map,
        new_map: &Map,
        server_ctx: &ServerContext,
    ) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            if server_ctx.curr_map_tool_type == MapToolType::Rect
                && let Some(dirty_chunks) = Self::changed_d2_rect_paint_chunks(old_map, new_map)
            {
                crate::utils::editor_scene_replace_incremental_map_update(
                    new_map.clone(),
                    dirty_chunks,
                );
                return true;
            }
            return false;
        }

        if let Some(dirty_chunks) = Self::changed_rect_paint_chunks(old_map, new_map) {
            crate::utils::editor_scene_incremental_map_update(new_map.clone(), dirty_chunks);
            return true;
        }

        if let Some(plan) = Self::changed_geometry_replacement_plan(old_map, new_map) {
            return Self::apply_geometry_scene_update(new_map, plan);
        }

        false
    }

    pub(crate) fn changed_d2_rect_paint_chunks(
        old_map: &Map,
        new_map: &Map,
    ) -> Option<Vec<(i32, i32)>> {
        // Scene streaming uses world-space chunks; this is unrelated to tile resolution.
        let chunk_size = SCENEMANAGER.read().unwrap().chunk_size();
        Self::changed_d2_rect_paint_chunks_with_size(old_map, new_map, chunk_size)
    }

    fn changed_d2_rect_paint_chunks_with_size(
        old_map: &Map,
        new_map: &Map,
        chunk_size: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let chunk_size = chunk_size.max(1);
        let mut dirty_chunks: FxHashSet<(i32, i32)> = FxHashSet::default();

        for (x, y) in Self::changed_terrain_override_keys(old_map, new_map) {
            dirty_chunks.insert((
                x.div_euclid(chunk_size) * chunk_size,
                y.div_euclid(chunk_size) * chunk_size,
            ));
        }

        let mut sector_ids = FxHashSet::default();
        sector_ids.extend(old_map.sectors.iter().map(|sector| sector.id));
        sector_ids.extend(new_map.sectors.iter().map(|sector| sector.id));

        for sector_id in sector_ids {
            let old_sector = old_map.find_sector(sector_id);
            let new_sector = new_map.find_sector(sector_id);
            let unchanged = matches!((old_sector, new_sector), (Some(old), Some(new))
                if old.linedefs == new.linedefs
                    && old.properties == new.properties
                    && old.layer == new.layer);
            if unchanged {
                continue;
            }

            if let Some(sector) = old_sector {
                Self::add_bbox_dirty_chunks_with_size(
                    sector.bounding_box(old_map),
                    chunk_size,
                    &mut dirty_chunks,
                );
            }
            if let Some(sector) = new_sector {
                Self::add_bbox_dirty_chunks_with_size(
                    sector.bounding_box(new_map),
                    chunk_size,
                    &mut dirty_chunks,
                );
            }
        }

        (!dirty_chunks.is_empty()).then(|| dirty_chunks.into_iter().collect())
    }

    fn add_bbox_dirty_chunks_with_size(
        bbox: rusterix::BBox,
        chunk_size: i32,
        chunks: &mut FxHashSet<(i32, i32)>,
    ) {
        if !bbox.min.x.is_finite()
            || !bbox.min.y.is_finite()
            || !bbox.max.x.is_finite()
            || !bbox.max.y.is_finite()
        {
            return;
        }

        let chunk_size = chunk_size.max(1);
        let min_cx = (bbox.min.x / chunk_size as f32).floor() as i32;
        let min_cy = (bbox.min.y / chunk_size as f32).floor() as i32;
        let max_cx = (bbox.max.x / chunk_size as f32).ceil() as i32;
        let max_cy = (bbox.max.y / chunk_size as f32).ceil() as i32;
        for cy in min_cy..max_cy.max(min_cy + 1) {
            for cx in min_cx..max_cx.max(min_cx + 1) {
                chunks.insert((cx * chunk_size, cy * chunk_size));
            }
        }
    }

    fn apply_live_geometry_owner_replacement(map: &Map, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return false;
        }
        if !matches!(
            server_ctx.curr_map_tool_type,
            MapToolType::Vertex
                | MapToolType::Linedef
                | MapToolType::Sector
                | MapToolType::Selection
        ) {
            return false;
        }

        if server_ctx.curr_map_tool_type == MapToolType::Selection {
            let owners: FxHashSet<GeoId> = map
                .selected_geometry_objects
                .iter()
                .copied()
                .map(GeoId::GeometryObject)
                .collect();
            if owners.is_empty() {
                return false;
            }

            let mut chunk_origins = {
                let rusterix = RUSTERIX.read().unwrap();
                rusterix
                    .scene_handler
                    .build_index
                    .chunks_for_owners(owners.iter().copied())
            };
            for id in &map.selected_geometry_objects {
                if let Some(object) = map.geometry_objects.iter().find(|object| object.id == *id)
                    && let Some(bbox) = object.bbox()
                {
                    Self::add_bbox_dirty_chunks(bbox, &mut chunk_origins);
                }
            }
            if chunk_origins.is_empty() {
                return false;
            }

            crate::utils::editor_scene_replace_incremental_map_update(
                map.clone(),
                chunk_origins.into_iter().collect(),
            );
            return true;
        }

        let topology = rusterix::MapTopology::build(map);
        let include_terrain = Self::selected_edit_affects_terrain(map, server_ctx);
        let mut owners = match server_ctx.curr_map_tool_type {
            MapToolType::Vertex => topology.owners_for_vertices(map, map.selected_vertices.clone()),
            MapToolType::Linedef => {
                topology.owners_for_linedefs(map, map.selected_linedefs.clone())
            }
            MapToolType::Sector => {
                topology.owners_for_sectors(map, map.selected_sectors.iter().copied())
            }
            _ => FxHashSet::default(),
        };
        if !include_terrain {
            Self::strip_terrain_owners(&mut owners);
        }
        if owners.is_empty() {
            return false;
        }

        let mut chunk_origins = {
            let rusterix = RUSTERIX.read().unwrap();
            rusterix
                .scene_handler
                .build_index
                .chunks_for_owners(owners.iter().copied())
        };
        for owner in owners.iter().copied() {
            Self::add_owner_dirty_chunks(map, owner, &mut chunk_origins);
        }
        Self::expand_topology_owners_for_chunks(map, &chunk_origins, &mut owners, include_terrain);
        Self::expand_index_owners_for_chunks(&chunk_origins, &mut owners, include_terrain);
        if !include_terrain {
            Self::strip_terrain_owners(&mut owners);
        }

        if chunk_origins.is_empty() {
            return false;
        }

        if include_terrain {
            let mut live_owners = owners.clone();
            Self::strip_terrain_owners(&mut live_owners);

            if !live_owners.is_empty() {
                let mut live_chunk_origins = {
                    let rusterix = RUSTERIX.read().unwrap();
                    rusterix
                        .scene_handler
                        .build_index
                        .chunks_for_owners(live_owners.iter().copied())
                };
                for owner in live_owners.iter().copied() {
                    Self::add_owner_dirty_chunks(map, owner, &mut live_chunk_origins);
                }
                Self::expand_topology_owners_for_chunks(
                    map,
                    &live_chunk_origins,
                    &mut live_owners,
                    false,
                );
                Self::expand_index_owners_for_chunks(&live_chunk_origins, &mut live_owners, false);
                Self::strip_terrain_owners(&mut live_owners);

                if !live_chunk_origins.is_empty() && !live_owners.is_empty() {
                    let _ = Self::apply_geometry_owner_replacement(
                        map,
                        GeometryOwnerReplacementPlan {
                            owners: live_owners,
                            chunk_origins: live_chunk_origins,
                            include_terrain: false,
                        },
                    );
                }
            }

            crate::utils::editor_scene_replace_incremental_map_update(
                map.clone(),
                chunk_origins.into_iter().collect(),
            );
            return true;
        }

        Self::apply_geometry_owner_replacement(
            map,
            GeometryOwnerReplacementPlan {
                owners,
                chunk_origins,
                include_terrain,
            },
        )
    }

    fn changed_rect_paint_chunks(old_map: &Map, new_map: &Map) -> Option<Vec<(i32, i32)>> {
        if !Self::map_geometry_matches(old_map, new_map) {
            return None;
        }

        let chunk_size = 32;
        let mut dirty_chunks: FxHashSet<(i32, i32)> = FxHashSet::default();
        for (x, z) in Self::changed_terrain_override_keys(old_map, new_map) {
            dirty_chunks.insert((
                x.div_euclid(chunk_size) * chunk_size,
                z.div_euclid(chunk_size) * chunk_size,
            ));
        }

        let mut sector_ids = FxHashSet::default();
        for sector in &old_map.sectors {
            sector_ids.insert(sector.id);
        }
        for sector in &new_map.sectors {
            sector_ids.insert(sector.id);
        }
        for sector_id in sector_ids {
            if Self::sector_tile_overrides_changed(old_map, new_map, sector_id) {
                Self::add_sector_dirty_chunks(new_map, sector_id, &mut dirty_chunks);
            }
        }

        if dirty_chunks.is_empty() {
            None
        } else {
            Some(dirty_chunks.into_iter().collect())
        }
    }

    fn sector_tile_overrides_changed(old_map: &Map, new_map: &Map, sector_id: u32) -> bool {
        let old_sector = old_map.find_sector(sector_id);
        let new_sector = new_map.find_sector(sector_id);
        match (old_sector, new_sector) {
            (Some(old), Some(new)) => {
                old.properties.get("tiles") != new.properties.get("tiles")
                    || old.properties.get("blend_tiles") != new.properties.get("blend_tiles")
                    || old.properties.get("source") != new.properties.get("source")
                    || old.properties.get("ceiling_source") != new.properties.get("ceiling_source")
                    || old.properties.get("tile_mode") != new.properties.get("tile_mode")
            }
            (None, Some(new)) => {
                new.properties.get("tiles").is_some()
                    || new.properties.get("blend_tiles").is_some()
                    || new.properties.get("source").is_some()
                    || new.properties.get("ceiling_source").is_some()
                    || new.properties.get("tile_mode").is_some()
            }
            (Some(old), None) => {
                old.properties.get("tiles").is_some()
                    || old.properties.get("blend_tiles").is_some()
                    || old.properties.get("source").is_some()
                    || old.properties.get("ceiling_source").is_some()
                    || old.properties.get("tile_mode").is_some()
            }
            (None, None) => false,
        }
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
        let mut list = Self {
            server_time: TheTime::default(),
            render_button_text: "Finished".to_string(),
            authoring_mode: false,
            text_game_mode: false,
            palette_mode: false,
            previous_sidebar_mode: None,
            previous_palette_dock: None,
            game_tools: Vec::new(),
            curr_game_tool: 0,
            game_tool_descriptors: FxHashMap::default(),
            game_tool_command_ids: FxHashMap::default(),
            previous_non_game_tool: None,

            editor_tools: Vec::new(),
            curr_editor_tool: 0,
            editor_mode: false,
            prefab_mode: false,
            props_3d_available: false,
            last_3d_hover_pick_at: None,
            last_3d_overlay_update_at: None,
            prefab_preview_wire_rgb: [83.0 / 255.0, 151.0 / 255.0, 207.0 / 255.0],
            prefab_grid_rgb: [57.0 / 255.0, 75.0 / 255.0, 105.0 / 255.0],
            box_select_armed: false,
            native_world_input: false,
            editor_display_session: None,
            #[cfg(feature = "native-ui")]
            native_display_mode: false,
            #[cfg(feature = "native-ui")]
            native_document: None,
            #[cfg(target_os = "macos")]
            macos_primary_orbit_drag: None,
        };

        list.register_game_tool("tool.vertex", VertexTool::new());
        list.register_game_tool("tool.linedef", LinedefTool::new());
        list.register_game_tool("tool.sector", SectorTool::new());
        list.register_game_tool("tool.geometry", GeometryTool::new());
        list.register_game_tool("tool.terrain", TerrainTool::new());
        list.register_game_tool("tool.iso_paint", IsoPaintTool::new());
        list.register_game_tool(
            "tool.tile_picker",
            crate::tools::tile_picker::TilePickerTool::new(),
        );
        list.register_game_tool("tool.effects", crate::tools::effects::EffectsTool::new());
        list.register_game_tool("tool.rect", RectTool::new());
        list.register_game_tool("tool.entity", crate::tools::entity::EntityTool::new());
        list.register_game_tool("tool.blocks", crate::tools::blocks::BlockTool::new());
        // The collision probe remains hidden while its workflow is being revised.
        list.register_game_tool("tool.game", GameTool::new());
        let default_tool_id = list
            .get_game_tool_uuid_by_command_id(Self::DEFAULT_GAME_TOOL_COMMAND_ID)
            .expect("default Creator tool must be registered");
        list.curr_game_tool = list
            .game_tools
            .iter()
            .position(|tool| tool.id().uuid == default_tool_id)
            .expect("default Creator tool must have an index");
        list
    }

    /// Keep renderer-owned prefab overlays aligned with the active UI theme.
    pub fn set_overlay_theme(&mut self, theme: &dyn TheTheme) {
        self.prefab_preview_wire_rgb =
            Self::normalized_theme_rgb(*theme.color(ToolListButtonSelectedBorder));
        self.prefab_grid_rgb = Self::normalized_theme_rgb(*theme.color(DefaultSelection));
    }

    fn normalized_theme_rgb(color: RGBA) -> [f32; 3] {
        [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
        ]
    }

    fn overlay_color(rgb: [f32; 3], alpha: f32) -> [f32; 4] {
        [rgb[0], rgb[1], rgb[2], alpha]
    }

    fn register_game_tool<T: Tool + 'static>(&mut self, command_id: &'static str, tool: T) {
        self.register_boxed_game_tool(command_id, Box::new(tool))
            .unwrap_or_else(|error| panic!("invalid built-in tool registration: {error}"));
    }

    /// Register a tool supplied by Creator or a runtime plugin.
    pub fn register_boxed_game_tool(
        &mut self,
        command_id: impl Into<String>,
        tool: Box<dyn Tool>,
    ) -> Result<(), String> {
        let command_id = command_id.into();
        validate_command_id(&command_id)?;
        if self.game_tool_command_ids.contains_key(&command_id) {
            return Err(format!("Duplicate tool command id '{command_id}'."));
        }
        let id = tool.id().uuid;
        if self.game_tool_descriptors.contains_key(&id) {
            return Err(format!("Duplicate tool UUID '{id}'."));
        }
        self.game_tool_command_ids.insert(command_id.clone(), id);
        self.game_tool_descriptors
            .insert(id, ToolDescriptor { command_id });
        self.game_tools.push(tool);
        Ok(())
    }

    fn should_refresh_3d_hover_pick(&mut self) -> bool {
        self.last_3d_hover_pick_at = Some(Instant::now());
        true
    }

    fn should_refresh_3d_overlay(&mut self) -> bool {
        self.last_3d_overlay_update_at = Some(Instant::now());
        true
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
                let command_id = self
                    .game_tool_descriptor_by_id(tool.id().uuid)
                    .map(|descriptor| descriptor.command_id.as_str());
                if !self.prefab_mode
                    && matches!(command_id, Some("tool.tile_picker" | "tool.effects"))
                {
                    continue;
                }
                if self.prefab_mode && !command_id.is_some_and(Self::is_prefab_tool_command_id) {
                    continue;
                }
                let mut b = TheToolListButton::new(tool.id());

                b.set_icon_name(tool.icon_name());
                b.set_status_text(&Self::status_text_with_accel(tool.info(), tool.accel()));
                if command_id == Some("tool.blocks") {
                    b.set_disabled(!self.props_3d_available);
                }
                if command_id == Some("tool.game") {
                    b.set_custom_color(Some(TheColor::from_u8(35, 78, 57, 255)));
                }
                if index == self.curr_game_tool && !(self.prefab_mode && self.palette_mode) {
                    b.set_state(TheWidgetState::Selected);
                }
                list.add_widget(Box::new(b));
            }

            if !self.prefab_mode {
                let mut sep =
                    TheSeparator::new(TheId::named_with_id("Tool Separator", Uuid::new_v4()));
                sep.limiter_mut().set_max_width(46);
                sep.limiter_mut().set_max_height(8);
                list.add_widget(Box::new(sep));

                let mut authoring =
                    TheToolListButton::new(TheId::named(Self::AUTHORING_BUTTON_NAME));
                authoring.set_icon_name("book-open".to_string());
                authoring.set_status_text(&fl!("status_tool_authoring"));
                if self.authoring_mode {
                    authoring.set_state(TheWidgetState::Selected);
                }
                list.add_widget(Box::new(authoring));

                let mut text_play =
                    TheToolListButton::new(TheId::named(Self::TEXT_PLAY_BUTTON_NAME));
                text_play.set_icon_name("terminal".to_string());
                text_play.set_status_text(&fl!("status_tool_text_play"));
                if self.text_game_mode {
                    text_play.set_state(TheWidgetState::Selected);
                }
                list.add_widget(Box::new(text_play));
            }

            let mut sep = TheSeparator::new(TheId::named_with_id(
                "Tool Separator Bottom",
                Uuid::new_v4(),
            ));
            sep.limiter_mut().set_max_width(46);
            sep.limiter_mut().set_max_height(8);
            list.add_widget(Box::new(sep));

            let mut palette = TheToolListButton::new(TheId::named(Self::PALETTE_BUTTON_NAME));
            palette.set_icon_name("palette".to_string());
            palette.set_status_text(&Self::status_text_with_accel(
                fl!("tool_palette"),
                Some('P'),
            ));
            if self.palette_mode {
                palette.set_state(TheWidgetState::Selected);
            }
            list.add_widget(Box::new(palette));
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

    fn is_prefab_tool_command_id(command_id: &str) -> bool {
        matches!(
            command_id,
            "tool.geometry"
                | "tool.vertex"
                | "tool.linedef"
                | "tool.sector"
                | "tool.iso_paint"
                | "tool.tile_picker"
                | "tool.effects"
        )
    }

    fn enforce_palette_dock(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.editor_mode || self.prefab_mode || !self.palette_mode {
            return;
        }
        let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
        if current_dock == "Tiles" {
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Palette".into(), ui, ctx, project, server_ctx);
        }
    }

    fn toggle_palette_mode(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.palette_mode = !self.palette_mode;
        server_ctx.palette_tool_active = self.palette_mode;

        ctx.ui.set_widget_state(
            Self::PALETTE_BUTTON_NAME.to_string(),
            if self.palette_mode {
                TheWidgetState::Selected
            } else {
                TheWidgetState::None
            },
        );

        if self.prefab_mode {
            let current_tool = self.game_tools[self.curr_game_tool].id();
            ctx.ui.set_widget_state(
                current_tool.name,
                if self.palette_mode {
                    TheWidgetState::None
                } else {
                    TheWidgetState::Selected
                },
            );
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Tool Changed"),
                TheValue::Empty,
            ));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Action List"),
                TheValue::Empty,
            ));
            ctx.ui.redraw_all = true;
            return true;
        }

        let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
        if self.palette_mode {
            self.previous_sidebar_mode = Some(*SIDEBARMODE.read().unwrap());
            self.previous_palette_dock = if current_dock == "Palette" {
                None
            } else {
                Some(current_dock)
            };
            *SIDEBARMODE.write().unwrap() = SidebarMode::Palette;
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Palette".into(), ui, ctx, project, server_ctx);
        } else if current_dock == "Palette" {
            if let Some(mode) = self.previous_sidebar_mode.take() {
                *SIDEBARMODE.write().unwrap() = mode;
            }
            let previous_dock = self
                .previous_palette_dock
                .take()
                .unwrap_or_else(|| "Tiles".into());
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock(previous_dock, ui, ctx, project, server_ctx);
        } else if let Some(mode) = self.previous_sidebar_mode.take() {
            *SIDEBARMODE.write().unwrap() = mode;
            self.previous_palette_dock = None;
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));
        true
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
        self.prefab_mode = false;

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

        if self.prefab_mode {
            self.palette_mode = false;
            self.previous_sidebar_mode = None;
            self.previous_palette_dock = None;
            ctx.ui
                .set_widget_state(Self::PALETTE_BUTTON_NAME.to_string(), TheWidgetState::None);
        }
        self.editor_mode = false;
        self.prefab_mode = false;
        self.editor_tools.clear();

        // Update the toolbar
        if let Some(list) = ui.get_vlayout("Tool List Layout") {
            self.set_active_editor(list, ctx);
        }
    }

    /// Keep the geometry tool engine active while presenting a purpose-built
    /// asset-authoring palette instead of the region/game tool list.
    pub fn set_prefab_tools(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if self.editor_mode && self.curr_editor_tool < self.editor_tools.len() {
            self.editor_tools[self.curr_editor_tool].deactivate();
        }
        self.editor_mode = false;
        self.editor_tools.clear();
        self.prefab_mode = true;
        self.palette_mode = false;
        self.previous_sidebar_mode = None;
        self.previous_palette_dock = None;
        server_ctx.palette_tool_active = false;

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
                    if undo_atom.is_iso_paint_only() {
                        ctx.ui.redraw_all = true;
                    } else {
                        let rect_paint_dirty_chunks = if server_ctx.curr_map_tool_type
                            == MapToolType::Rect
                            && server_ctx.editor_view_mode != EditorViewMode::D2
                            && let ProjectUndoAtom::MapEdit(_, old_map, new_map) = &undo_atom
                        {
                            Self::changed_rect_paint_chunks(old_map, new_map)
                        } else {
                            None
                        };
                        let geometry_replacement_plan = if rect_paint_dirty_chunks.is_none()
                            && server_ctx.editor_view_mode != EditorViewMode::D2
                            && let ProjectUndoAtom::MapEdit(_, old_map, new_map) = &undo_atom
                        {
                            Self::changed_geometry_replacement_plan(old_map, new_map)
                        } else {
                            None
                        };
                        if server_ctx.editor_view_mode == EditorViewMode::D2
                            && server_ctx.editing_surface.is_some()
                            || rect_paint_dirty_chunks.is_some()
                            || geometry_replacement_plan.is_some()
                        {
                        } else {
                            self.update_geometry_overlay_3d(project, server_ctx);
                        }
                        // RectTool already streams the touched chunks while painting in 2D.
                        // Rebuilding here would clear that preview and visibly reload the map
                        // as soon as the stroke is committed.
                        let mut used_incremental_scene_update = rect_stroke_streams_scene_update(
                            server_ctx.curr_map_tool_type,
                            server_ctx.editor_view_mode,
                        );
                        if let Some(dirty_chunks) = rect_paint_dirty_chunks
                            && let ProjectUndoAtom::MapEdit(_, _, new_map) = &undo_atom
                        {
                            crate::utils::editor_scene_incremental_map_update(
                                (**new_map).clone(),
                                dirty_chunks,
                            );
                            used_incremental_scene_update = true;
                        }
                        if let Some(plan) = geometry_replacement_plan
                            && let ProjectUndoAtom::MapEdit(_, _, new_map) = &undo_atom
                        {
                            used_incremental_scene_update =
                                Self::apply_geometry_scene_update(new_map, plan);
                        }
                        if !used_incremental_scene_update {
                            crate::utils::editor_scene_full_rebuild(project, server_ctx);
                        }
                    }
                } else if pc.is_prefab() {
                    if let ProjectContext::Prefab(asset_id) = pc {
                        if let Err(message) =
                            crate::block_props::sync_prefab_editor(project, asset_id)
                        {
                            ctx.ui
                                .send(TheEvent::SetStatusText(TheId::empty(), message));
                        } else {
                            let prefabs = project.block_props.clone();
                            RUSTERIX.write().unwrap().set_block_props(prefabs.clone());
                            SCENEMANAGER.write().unwrap().set_block_props(prefabs);
                        }
                    }
                    if let ProjectUndoAtom::MapEdit(_, old_map, new_map) = &undo_atom {
                        if !Self::try_incremental_map_edit(old_map, new_map, server_ctx) {
                            crate::utils::editor_scene_full_rebuild(project, server_ctx);
                        }
                    }
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
        let current_tool_id = self
            .game_tools
            .get(self.curr_game_tool)
            .map(|tool| tool.id().uuid);
        if self.editor_display_session.as_ref().is_some_and(|session| {
            let owner_is_active = match session.owner {
                EditorDisplayOwner::Action(action_id) => {
                    Some(action_id) == server_ctx.curr_action_id
                }
                EditorDisplayOwner::Tool(tool_id) => Some(tool_id) == current_tool_id,
            };
            owner_is_active && session.project_context == server_ctx.pc
        }) {
            if let Some(session) = self.editor_display_session.as_mut() {
                session.set_grid_subdivisions(map.subdivisions);
                session.draw(buffer, ctx);
            }
            return;
        }
        self.game_tools[self.curr_game_tool].draw_hud(buffer, map, ctx, server_ctx, assets);
        crate::hud::Hud::draw_shortcut_guidance(buffer, map, ctx, server_ctx);
        if server_ctx.editor_view_mode == EditorViewMode::Orbit {
            let camera = crate::editor::EDITCAMERA
                .read()
                .unwrap()
                .orbit_camera
                .clone();
            crate::viewcube::ViewCube::draw(buffer, ctx, &camera);
        }
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
        self.sync_props_availability(ui, ctx, project, server_ctx);
        if !self.editor_mode
            && self.current_game_tool_is("tool.blocks")
            && self
                .get_current_tool()
                .handle_event(event, ui, ctx, project, server_ctx)
        {
            return true;
        }
        if self.handle_editor_display_event(event, ui, ctx, project, server_ctx) {
            return true;
        }
        if let TheEvent::ValueChanged(id, value) = event {
            if id.name == "Editor Fill Light" {
                if let Some(strength) = value.to_f32() {
                    let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                    rusterix.editor_preview_fill_strength = strength.clamp(0.0, 1.0);
                    rusterix.set_dirty();
                    ctx.ui.redraw_all = true;
                }
                return true;
            }
        }
        if self.editor_mode && self.curr_editor_tool < self.editor_tools.len() {
            let should_forward_to_tool = match event {
                // Keep tool switching and shortcuts handled by ToolList itself.
                TheEvent::StateChanged(_, _) | TheEvent::KeyDown(_) => false,
                TheEvent::KeyCodeDown(TheValue::KeyCode(code))
                    if server_ctx.polyview_has_focus(ctx)
                        && server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && !server_ctx.game_input_mode
                        && *code == TheKeyCode::Space =>
                {
                    false
                }
                TheEvent::KeyCodeDown(TheValue::KeyCode(code))
                    if server_ctx.polyview_has_focus(ctx)
                        && server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
                        && *code == TheKeyCode::Escape =>
                {
                    false
                }
                TheEvent::KeyCodeUp(TheValue::KeyCode(code))
                    if *code == TheKeyCode::Space
                        && !server_ctx.game_mode
                        && !server_ctx.game_input_mode =>
                {
                    false
                }
                TheEvent::KeyUp(TheValue::Char(c))
                    if server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && !server_ctx.game_mode
                        && !server_ctx.game_input_mode
                        && matches!(c.to_ascii_lowercase(), 'w' | 'a' | 's' | 'd') =>
                {
                    false
                }
                TheEvent::RenderViewContext(id, _)
                    if id.name == "PolyView"
                        && server_ctx.editor_view_mode != EditorViewMode::D2
                        && !server_ctx.game_input_mode =>
                {
                    false
                }
                TheEvent::RenderViewClicked(id, _)
                | TheEvent::RenderViewDragged(id, _)
                | TheEvent::RenderViewUp(id, _)
                | TheEvent::RenderViewHoverChanged(id, _)
                    if id.name == "PolyView"
                        && server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active =>
                {
                    false
                }
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
                    let projection_center = project
                        .get_map(server_ctx)
                        .filter(|m| {
                            prev_mode == EditorViewMode::D2
                                && (*index == 0 || *index == 1)
                                && (m.camera == MapCamera::TwoDIso) != (*index == 1)
                        })
                        .map(|m| {
                            if m.camera == MapCamera::TwoDIso {
                                rusterix::isometric::Projection {
                                    origin: Vec2::new(m.offset.x, -m.offset.y),
                                    unit: m.grid_size,
                                }
                                .ground(Vec2::zero())
                            } else {
                                Vec2::new(-m.offset.x, m.offset.y) / m.grid_size
                            }
                        });
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

                    server_ctx.editor_view_mode = EditorViewMode::from_toolbar_index(*index as i32);
                    let new_mode = server_ctx.editor_view_mode;
                    let new = new_mode.is_3d();
                    self.box_select_armed = false;

                    if new_mode != EditorViewMode::FirstP {
                        server_ctx.editor_fly_nav_active = false;
                        server_ctx.editor_fly_nav_space_down = false;
                        let mut edit_camera = crate::editor::EDITCAMERA.write().unwrap();
                        edit_camera.move_action = None;
                        edit_camera.reset_mouse_tracking();
                    }

                    if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                        region.map.camera = match new_mode {
                            EditorViewMode::D2 => {
                                if *index == 1 {
                                    MapCamera::TwoDIso
                                } else {
                                    MapCamera::TwoD
                                }
                            }
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
                            if let Some(center) = projection_center {
                                server_ctx.center_map_at_grid_pos(
                                    Vec2::zero(),
                                    center,
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

                    self.sync_props_availability(ui, ctx, project, server_ctx);
                    if let Some(editing_pos_buffer) = server_ctx.editing_pos_buffer {
                        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                            region.editing_position_3d = editing_pos_buffer;
                        }
                        server_ctx.editing_pos_buffer = None;
                    }
                    server_ctx.editing_surface = None;

                    RUSTERIX.write().unwrap().client.scene.d2_static.clear();
                    RUSTERIX.write().unwrap().client.scene.d2_dynamic.clear();

                    // Both 2D projections have distinct authoring sources.
                    // Returning from sprite-isometric must replace its dormant
                    // builder map before ordinary top-down picking resumes.
                    if old != new || projection_center.is_some() {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Switch SceneManager Camera"),
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
                let polyview_focused = server_ctx.polyview_has_focus(ctx);
                let text_input_focused =
                    ui.focus_widget_supports_text_input(ctx) && !polyview_focused;
                let plain_key = !ui.ctrl && !ui.logo && !ui.alt;
                if plain_key
                    && polyview_focused
                    && !text_input_focused
                    && !server_ctx.game_mode
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && server_ctx.editor_view_mode == EditorViewMode::FirstP
                {
                    let action = match c.to_ascii_lowercase() {
                        'w' => Some(crate::editcamera::CustomMoveAction::Forward),
                        's' => Some(crate::editcamera::CustomMoveAction::Backward),
                        'a' => Some(crate::editcamera::CustomMoveAction::StrafeLeft),
                        'd' => Some(crate::editcamera::CustomMoveAction::StrafeRight),
                        _ => None,
                    };
                    if let Some(action) = action {
                        crate::editor::EDITCAMERA.write().unwrap().move_action = Some(action);
                        ctx.ui.redraw_all = true;
                        return true;
                    }
                }
                // Iso part orientation belongs to the active brush. Handle it
                // before global comma/period grid-subdivision shortcuts.
                if plain_key
                    && polyview_focused
                    && !text_input_focused
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && !server_ctx.game_mode
                    && !self.editor_mode
                    && self.current_game_tool_is("tool.rect")
                    && project
                        .get_map(server_ctx)
                        .is_some_and(|map| map.camera == MapCamera::TwoDIso)
                    && matches!(c, ',' | '.')
                {
                    let undo = self.current_tool_map_event(
                        MapEvent::MapKey(*c),
                        ui,
                        ctx,
                        project,
                        server_ctx,
                    );
                    self.update_map_context(ui, ctx, project, server_ctx, undo);
                    return true;
                }
                let shortcut_resolution = if plain_key
                    && polyview_focused
                    && !text_input_focused
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && !self.editor_mode
                    && !self.current_game_tool_is("tool.game")
                    && !server_ctx.game_mode
                {
                    let shortcut_context =
                        Self::shortcut_context(project.get_map(server_ctx), server_ctx);
                    ShortcutResolver::from_toml(&project.shortcuts).resolve(*c, shortcut_context)
                } else {
                    None
                };
                let suppress_tool_accel = shortcut_resolution.is_some();

                let gizmo_op = match c.to_ascii_lowercase() {
                    'm' => Some(GeometryGizmoOp::Move),
                    's' => Some(GeometryGizmoOp::Resize),
                    _ => None,
                };
                if plain_key
                    && polyview_focused
                    && !text_input_focused
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && !server_ctx.game_mode
                    && server_ctx.editor_view_mode != EditorViewMode::D2
                    && (server_ctx.curr_map_tool_type == MapToolType::Selection
                        || self.current_game_tool_is("tool.geometry"))
                    && let Some(op) = gizmo_op
                {
                    server_ctx.geometry_gizmo_op = op;
                    self.update_geometry_overlay_3d(project, server_ctx);
                    {
                        let mut rusterix = RUSTERIX.write().unwrap();
                        rusterix.set_dirty();
                        rusterix.set_overlay_dirty();
                    }
                    ctx.ui.redraw_all = true;
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        match op {
                            GeometryGizmoOp::Move => fl!("status_hud_geometry_op_move"),
                            GeometryGizmoOp::Resize => fl!("status_hud_geometry_op_size"),
                        },
                    ));
                    return true;
                }

                if plain_key
                    && polyview_focused
                    && !text_input_focused
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && !self.editor_mode
                    && self.current_game_tool_is("tool.blocks")
                    && matches!(
                        c,
                        'r' | 'R' | 'e' | 'E' | 'h' | 'H' | 'w' | 'W' | '[' | '{' | ']' | '}'
                    )
                {
                    let undo_atom = self.current_tool_map_event(
                        MapEvent::MapKey(*c),
                        ui,
                        ctx,
                        project,
                        server_ctx,
                    );
                    self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                    if server_ctx.editor_view_mode != EditorViewMode::D2 {
                        self.update_geometry_overlay_3d(project, server_ctx);
                    }
                    return true;
                }

                let preserve_geometry_object_shortcut = plain_key
                    && polyview_focused
                    && server_ctx.editor_view_mode != EditorViewMode::D2
                    && server_ctx.curr_map_tool_type == MapToolType::Selection
                    && matches!(c, 'r' | 'R')
                    && project
                        .get_map(server_ctx)
                        .is_some_and(|map| !map.selected_geometry_objects.is_empty());
                let preserve_prefab_instance_shortcut = plain_key
                    && polyview_focused
                    && server_ctx.editor_view_mode != EditorViewMode::D2
                    && server_ctx.curr_map_tool_type == MapToolType::Selection
                    && matches!(c, 'r' | 'R' | 'f' | 'F' | 'd' | 'D' | 'a' | 'A')
                    && project
                        .get_map(server_ctx)
                        .is_some_and(|map| !map.selected_block_prop_instances.is_empty());

                if plain_key
                    && !text_input_focused
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && !preserve_geometry_object_shortcut
                    && !preserve_prefab_instance_shortcut
                    && !suppress_tool_accel
                {
                    let mut tool_uuid = None;
                    if self.editor_mode {
                        for tool in self.editor_tools.iter() {
                            if let Some(acc) = tool.accel()
                                && acc.to_ascii_lowercase() == c.to_ascii_lowercase()
                            {
                                tool_uuid = Some(tool.id().uuid);
                                ctx.ui.set_widget_state(
                                    self.editor_tools[self.curr_editor_tool].id().name,
                                    TheWidgetState::None,
                                );
                                ctx.ui
                                    .set_widget_state(tool.id().name, TheWidgetState::Selected);
                                break;
                            }
                        }
                    } else if !self.current_game_tool_is("tool.game") && !server_ctx.game_mode {
                        for tool in self.game_tools.iter() {
                            if let Some(acc) = tool.accel()
                                && acc.to_ascii_lowercase() == c.to_ascii_lowercase()
                            {
                                tool_uuid = Some(tool.id().uuid);
                                ctx.ui.set_widget_state(
                                    self.game_tools[self.curr_game_tool].id().name,
                                    TheWidgetState::None,
                                );
                                ctx.ui
                                    .set_widget_state(tool.id().name, TheWidgetState::Selected);
                                break;
                            }
                        }
                    }

                    if let Some(uuid) = tool_uuid {
                        return self.set_tool(uuid, ui, ctx, project, server_ctx);
                    }
                }

                if let Some(ShortcutResolution::Run(action)) = shortcut_resolution {
                    if action == ShortcutAction::BoxSelect {
                        if !matches!(
                            server_ctx.curr_map_tool_type,
                            MapToolType::Selection
                                | MapToolType::Vertex
                                | MapToolType::Linedef
                                | MapToolType::Sector
                        ) && let Some(uuid) =
                            self.get_game_tool_uuid_by_command_id("tool.geometry")
                        {
                            self.set_tool_widget_state_by_uuid(uuid, ctx);
                            self.set_tool(uuid, ui, ctx, project, server_ctx);
                        }
                        server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                        self.box_select_armed = true;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_box_select_armed"),
                        ));
                        ctx.ui.redraw_all = true;
                        return true;
                    }
                    if let Some(uuid) = self.shortcut_tool_uuid(action) {
                        self.set_tool_widget_state_by_uuid(uuid, ctx);
                        return self.set_tool(uuid, ui, ctx, project, server_ctx);
                    }
                }

                if let Some(_id) = &ctx.ui.focus {
                    if server_ctx.polyview_has_focus(ctx) {
                        if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                            let plain_grid_shortcut = plain_key
                                && !server_ctx.game_mode
                                && !server_ctx.game_input_mode
                                && server_ctx.curr_map_tool_type != MapToolType::Game;
                            if plain_grid_shortcut
                                && (*c == ',' || *c == '.')
                                && server_ctx.editor_view_mode != EditorViewMode::D2
                            {
                                let delta = if *c == ',' { -1 } else { 1 };
                                map.subdivisions =
                                    Self::step_grid_subdivision(map.subdivisions, delta);
                                {
                                    let mut rusterix = RUSTERIX.write().unwrap();
                                    rusterix.set_dirty();
                                    rusterix.set_overlay_dirty();
                                }
                                self.update_geometry_overlay_3d(project, server_ctx);
                                ctx.ui.redraw_all = true;
                                return true;
                            } else if plain_grid_shortcut && (*c == ',' || *c == '.') {
                                return false;
                            } else if plain_grid_shortcut
                                && let Some(subdivision) = Self::grid_subdivision_from_key(*c)
                            {
                                map.subdivisions = subdivision;
                                {
                                    let mut rusterix = RUSTERIX.write().unwrap();
                                    rusterix.set_dirty();
                                    rusterix.set_overlay_dirty();
                                }
                                self.update_geometry_overlay_3d(project, server_ctx);
                                ctx.ui.redraw_all = true;
                                return true;
                            } else if plain_grid_shortcut
                                && Self::is_grid_subdivision_key(*c)
                                && server_ctx.curr_map_tool_type != MapToolType::Selection
                            {
                                return false;
                            } else if server_ctx.editor_view_mode != EditorViewMode::D2
                                && !server_ctx.game_input_mode
                                && server_ctx.curr_map_tool_type != MapToolType::Game
                            {
                                if (*c == 'd' || *c == 'D') && !preserve_prefab_instance_shortcut {
                                    server_ctx.geometry_edit_mode = GeometryEditMode::Detail;
                                    self.update_geometry_overlay_3d(project, server_ctx);
                                    RUSTERIX.write().unwrap().set_dirty();
                                    return false;
                                }
                            }

                            let undo_atom = self.current_tool_map_event(
                                MapEvent::MapKey(*c),
                                ui,
                                ctx,
                                project,
                                server_ctx,
                            );
                            self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                            if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                self.update_geometry_overlay_3d(project, server_ctx);
                            }
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

                let mut acc = !text_input_focused;
                if self.current_game_tool_is("tool.game")
                    || ui.ctrl
                    || ui.logo
                    || ui.alt
                    || server_ctx.game_input_mode
                    || preserve_geometry_object_shortcut
                    || suppress_tool_accel
                {
                    acc = false;
                }

                if acc {
                    if !self.editor_mode && c.to_ascii_lowercase() == 'p' {
                        return self.toggle_palette_mode(ui, ctx, project, server_ctx);
                    }

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
                                if acc.to_ascii_lowercase() == c.to_ascii_lowercase() {
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
                                if acc.to_ascii_lowercase() == c.to_ascii_lowercase() {
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
                    if self.authoring_mode || current_dock == "Authoring" {
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

                    if self.current_game_tool_is("tool.game") && server_ctx.game_mode {
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
                if id.name == Self::PALETTE_BUTTON_NAME && *state == TheWidgetState::Clicked {
                    redraw = self.toggle_palette_mode(ui, ctx, project, server_ctx);
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
                    redraw = self.set_tool(id.uuid, ui, ctx, project, server_ctx);
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                if self.current_game_tool_is("tool.game") {
                    let key = match code {
                        TheKeyCode::Return => Some("enter"),
                        TheKeyCode::Delete => Some("backspace"),
                        TheKeyCode::Escape => Some("escape"),
                        TheKeyCode::Space => Some("space"),
                        _ => None,
                    };
                    if let Some(key) = key {
                        let mut rusterix = RUSTERIX.write().unwrap();
                        if rusterix.client.focused_text_input_key_down(key) {
                            rusterix.set_overlay_dirty();
                            return true;
                        }
                    }
                }

                if let Some(id) = &ctx.ui.focus {
                    if id.name == "PolyView"
                        || (server_ctx.pc.is_prefab() && id.name == "PrefabView")
                    {
                        if self.box_select_armed && *code == TheKeyCode::Escape {
                            self.box_select_armed = false;
                            if let Some(_map) = Self::get_tool_map_mut(project, server_ctx) {
                                let undo_atom = self.current_tool_map_event(
                                    MapEvent::MapEscape,
                                    ui,
                                    ctx,
                                    project,
                                    server_ctx,
                                );
                                self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                            }
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                fl!("status_box_select_cancelled"),
                            ));
                            ctx.ui.redraw_all = true;
                            return true;
                        }
                        if server_ctx.editor_view_mode == EditorViewMode::FirstP
                            && !server_ctx.game_input_mode
                            && *code == TheKeyCode::Space
                        {
                            if server_ctx.editor_fly_nav_space_down {
                                return true;
                            }
                            server_ctx.editor_fly_nav_space_down = true;
                            server_ctx.editor_fly_nav_mouse_down = false;
                            server_ctx.editor_fly_nav_active = !server_ctx.editor_fly_nav_active;
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .reset_mouse_tracking();
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                if server_ctx.editor_fly_nav_active {
                                    fl!("status_firstp_fly_nav_on")
                                } else {
                                    fl!("status_firstp_fly_nav_off")
                                },
                            ));
                            ctx.ui.redraw_all = true;
                            return true;
                        }

                        let invert_2d_pan = server_ctx.editor_view_mode == EditorViewMode::D2
                            && !cfg!(target_os = "macos");
                        if server_ctx.editor_view_mode == EditorViewMode::D2
                            && *code == TheKeyCode::Up
                        {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.y += if invert_2d_pan { -50.0 } else { 50.0 };
                            }
                            return false;
                        }
                        if server_ctx.editor_view_mode == EditorViewMode::D2
                            && *code == TheKeyCode::Down
                        {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.y += if invert_2d_pan { 50.0 } else { -50.0 };
                            }
                            return false;
                        }
                        if server_ctx.editor_view_mode == EditorViewMode::D2
                            && *code == TheKeyCode::Left
                        {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.x += if invert_2d_pan { 50.0 } else { -50.0 };
                            }
                            return false;
                        }
                        if server_ctx.editor_view_mode == EditorViewMode::D2
                            && *code == TheKeyCode::Right
                        {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.offset.x += if invert_2d_pan { -50.0 } else { 50.0 };
                            }
                            return false;
                        }
                        if *code == TheKeyCode::Escape {
                            if let Some(_map) = Self::get_tool_map_mut(project, server_ctx) {
                                if server_ctx.editor_fly_nav_active {
                                    server_ctx.editor_fly_nav_active = false;
                                    server_ctx.editor_fly_nav_mouse_down = false;
                                    server_ctx.editor_fly_nav_space_down = false;
                                    crate::editor::EDITCAMERA.write().unwrap().move_action = None;
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        fl!("status_firstp_fly_nav_off"),
                                    ));
                                    ctx.ui.redraw_all = true;
                                    return true;
                                }
                                if server_ctx.paste_clipboard.is_some() {
                                    server_ctx.paste_clipboard = None;
                                    return true;
                                }

                                let undo_atom = self.current_tool_map_event(
                                    MapEvent::MapEscape,
                                    ui,
                                    ctx,
                                    project,
                                    server_ctx,
                                );
                                self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                                if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                    self.update_geometry_overlay_3d(project, server_ctx);
                                }
                            }
                        } else if *code == TheKeyCode::Delete {
                            if let Some(_map) = Self::get_tool_map_mut(project, server_ctx) {
                                let undo_atom = self.current_tool_map_event(
                                    MapEvent::MapDelete,
                                    ui,
                                    ctx,
                                    project,
                                    server_ctx,
                                );
                                self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                                if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                    self.update_geometry_overlay_3d(project, server_ctx);
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::KeyUp(TheValue::Char(c)) => {
                if server_ctx.editor_view_mode == EditorViewMode::FirstP
                    && !server_ctx.game_mode
                    && !server_ctx.game_input_mode
                    && matches!(c.to_ascii_lowercase(), 'w' | 'a' | 's' | 'd')
                {
                    crate::editor::EDITCAMERA.write().unwrap().move_action = None;
                    ctx.ui.redraw_all = true;
                    return true;
                }
            }
            TheEvent::KeyCodeUp(TheValue::KeyCode(code)) => {
                if *code == TheKeyCode::Space {
                    server_ctx.editor_fly_nav_space_down = false;
                }
            }
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
                    {
                        return true;
                    }

                    if !server_ctx.game_mode && !server_ctx.game_input_mode {
                        #[cfg(target_os = "macos")]
                        {
                            self.macos_primary_orbit_drag = None;
                            if !self.box_select_armed
                                && self.viewcube_face_at(ui, server_ctx, *coord).is_some()
                            {
                                self.macos_primary_orbit_drag = Some(MacOsPrimaryOrbitDrag {
                                    origin: *coord,
                                    from_viewcube: true,
                                    orbiting: false,
                                });
                                crate::editor::EDITCAMERA
                                    .write()
                                    .unwrap()
                                    .mouse_dragged_orbit(coord);
                                return true;
                            }
                        }
                        #[cfg(not(target_os = "macos"))]
                        if !self.box_select_armed
                            && self.handle_viewcube_click(ui, ctx, project, server_ctx, *coord)
                        {
                            return true;
                        }
                        let native_overlay = self.native_world_input;
                        if !native_overlay
                            && self.handle_action_hud_icon_click(ui, ctx, project, server_ctx, *coord)
                        {
                            return true;
                        }

                        if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                            if coord.y > 20 {
                                // Test for Paste operation
                                if let Some(paste) = &server_ctx.paste_clipboard {
                                    if let Some(hover) = Self::paste_position(server_ctx) {
                                        let prev = map.clone();
                                        let prev_counts = (
                                            map.vertices.len(),
                                            map.linedefs.len(),
                                            map.sectors.len(),
                                            map.geometry_objects.len(),
                                        );
                                        map.paste_at_position(paste, hover);
                                        let post_counts = (
                                            map.vertices.len(),
                                            map.linedefs.len(),
                                            map.sectors.len(),
                                            map.geometry_objects.len(),
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

                                            return true;
                                        }
                                    }
                                }
                            }
                        }

                        if server_ctx.editor_view_mode != EditorViewMode::D2
                            && let Some(render_view) =
                                crate::utils::map_editor_render_view(ui, server_ctx)
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

                        if self.box_select_armed {
                            // Box Select starts a rectangle even when the drag
                            // begins over geometry or a gizmo.
                            server_ctx.geo_hit = None;
                            server_ctx.geo_hit_pos = Vec3::zero();
                            server_ctx.hover = (None, None, None);
                        }

                        #[cfg(target_os = "macos")]
                        if server_ctx.editor_view_mode == EditorViewMode::Orbit
                            && !self.box_select_armed
                            && server_ctx.curr_map_tool_type != MapToolType::Terrain
                            && server_ctx.geo_hit.is_none()
                        {
                            self.macos_primary_orbit_drag = Some(MacOsPrimaryOrbitDrag {
                                origin: *coord,
                                from_viewcube: false,
                                orbiting: false,
                            });
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .mouse_dragged_orbit(coord);
                        }

                        {
                            let undo_atom = self.current_tool_map_event(
                                MapEvent::MapClicked(*coord),
                                ui,
                                ctx,
                                project,
                                server_ctx,
                            );
                            let paint_only_undo = undo_atom
                                .as_ref()
                                .is_some_and(ProjectUndoAtom::is_iso_paint_only);
                            self.update_map_context(ui, ctx, project, server_ctx, undo_atom);

                            if server_ctx.editor_view_mode != EditorViewMode::D2
                                && !paint_only_undo
                                && server_ctx.curr_map_tool_type != MapToolType::Rect
                            {
                                self.update_geometry_overlay_3d(project, server_ctx);
                            }
                            redraw = true;
                        }
                    } else {
                        let current_map = RUSTERIX.read().unwrap().client.current_map.clone();
                        for r in &mut project.regions {
                            if r.map.name == current_map {
                                self.get_current_tool().region_map_event(
                                    MapEvent::MapClicked(*coord),
                                    ui,
                                    ctx,
                                    r,
                                    server_ctx,
                                );
                            }
                        }
                    }
                }
            }
            TheEvent::RenderViewContext(id, coord) => {
                if id.name == "PolyView"
                    && !server_ctx.game_mode
                    && !server_ctx.game_input_mode
                    && server_ctx.editor_view_mode != EditorViewMode::D2
                {
                    crate::editor::EDITCAMERA
                        .write()
                        .unwrap()
                        .reset_mouse_tracking();

                    if server_ctx.editor_view_mode == EditorViewMode::FirstP {
                        server_ctx.editor_fly_nav_active = true;
                        server_ctx.editor_fly_nav_mouse_down = true;
                        server_ctx.editor_fly_nav_space_down = false;
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .mouse_dragged_firstp(region, coord);
                        }
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_firstp_fly_nav_rmb_on"),
                        ));
                    }

                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    ctx.ui.redraw_all = true;
                    return true;
                }
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
                    {
                        if server_ctx.editor_fly_nav_mouse_down {
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                crate::editor::EDITCAMERA
                                    .write()
                                    .unwrap()
                                    .mouse_dragged_firstp(region, coord);
                            }
                        } else if let Some(view_size) =
                            crate::utils::map_editor_render_view(ui, server_ctx).map(|view| {
                                let dim = *view.dim();
                                Vec2::new(dim.width, dim.height)
                            })
                        {
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .set_fly_pointer(coord, view_size);
                        }
                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        ctx.ui.redraw_all = true;
                        return true;
                    }

                    #[cfg(target_os = "macos")]
                    if server_ctx.editor_view_mode == EditorViewMode::Orbit
                        && !ui.right_mouse_down
                        && let Some(mut navigation) = self.macos_primary_orbit_drag
                    {
                        let delta = *coord - navigation.origin;
                        let drag_threshold_squared = if navigation.from_viewcube { 4 } else { 16 };
                        if delta.x * delta.x + delta.y * delta.y >= drag_threshold_squared {
                            navigation.orbiting = true;
                        }
                        self.macos_primary_orbit_drag = Some(navigation);

                        if navigation.orbiting {
                            let prefab_camera = {
                                let mut edit_camera = crate::editor::EDITCAMERA.write().unwrap();
                                if navigation.from_viewcube {
                                    edit_camera.mouse_dragged_viewcube_macos_primary(coord);
                                } else {
                                    edit_camera.mouse_dragged_orbit_macos_primary(coord);
                                }
                                server_ctx
                                    .pc
                                    .is_prefab()
                                    .then(|| edit_camera.orbit_camera.clone())
                            };
                            if let Some(camera) = prefab_camera {
                                crate::editor::RUSTERIX
                                    .write()
                                    .unwrap()
                                    .client
                                    .set_camera_d3(Box::new(camera));
                                self.update_geometry_overlay_3d(project, server_ctx);
                            }
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            ctx.ui.redraw_all = true;
                        }
                        // Do not leak a pending navigation drag into the active
                        // modelling tool while waiting for the drag threshold.
                        return true;
                    }

                    if server_ctx.editor_view_mode != EditorViewMode::D2
                        && !server_ctx.game_mode
                        && !server_ctx.game_input_mode
                    {
                        let orbit_drag = !self.box_select_armed
                            && server_ctx.editor_view_mode == EditorViewMode::Orbit
                            && (ui.right_mouse_down || ui.alt);
                        let pan_drag = !self.box_select_armed
                            && matches!(
                                server_ctx.editor_view_mode,
                                EditorViewMode::Orbit | EditorViewMode::Iso
                            )
                            && (ui.ctrl
                                || ui.logo
                                || (server_ctx.editor_view_mode == EditorViewMode::Iso
                                    && (ui.right_mouse_down || ui.alt)));

                        if orbit_drag {
                            let prefab_camera = {
                                let mut edit_camera = crate::editor::EDITCAMERA.write().unwrap();
                                edit_camera.mouse_dragged_orbit(coord);
                                server_ctx
                                    .pc
                                    .is_prefab()
                                    .then(|| edit_camera.orbit_camera.clone())
                            };
                            if let Some(camera) = prefab_camera {
                                crate::editor::RUSTERIX
                                    .write()
                                    .unwrap()
                                    .client
                                    .set_camera_d3(Box::new(camera));
                                self.update_geometry_overlay_3d(project, server_ctx);
                            }
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            ctx.ui.redraw_all = true;
                            return true;
                        }

                        if pan_drag
                            && let Some(render_view) =
                                crate::utils::map_editor_render_view(ui, server_ctx)
                        {
                            let dim = *render_view.dim();
                            if server_ctx.pc.is_prefab() {
                                let camera = {
                                    let mut edit_camera =
                                        crate::editor::EDITCAMERA.write().unwrap();
                                    edit_camera
                                        .mouse_dragged_pan_prefab(coord, Vec2::new(dim.x, dim.y));
                                    edit_camera.orbit_camera.clone()
                                };
                                crate::editor::RUSTERIX
                                    .write()
                                    .unwrap()
                                    .client
                                    .set_camera_d3(Box::new(camera));
                                self.update_geometry_overlay_3d(project, server_ctx);
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                ctx.ui.redraw_all = true;
                                return true;
                            } else if let Some(region) =
                                project.get_region_mut(&server_ctx.curr_region)
                            {
                                crate::editor::EDITCAMERA
                                    .write()
                                    .unwrap()
                                    .mouse_dragged_pan_3d(
                                        region,
                                        server_ctx,
                                        coord,
                                        Vec2::new(dim.x, dim.y),
                                    );
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                ctx.ui.redraw_all = true;
                                return true;
                            }
                        }
                    }

                    if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        // Map dragging handled by tools.
                    }

                    if server_ctx.editor_view_mode != EditorViewMode::D2
                        && let Some(render_view) =
                            crate::utils::map_editor_render_view(ui, server_ctx)
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

                    {
                        let undo_atom = self.current_tool_map_event(
                            MapEvent::MapDragged(*coord),
                            ui,
                            ctx,
                            project,
                            server_ctx,
                        );
                        if undo_atom.is_none() {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                Self::apply_live_geometry_owner_replacement(map, server_ctx);
                            }
                        }
                        self.update_map_context(ui, ctx, project, server_ctx, undo_atom);

                        if server_ctx.editor_view_mode != EditorViewMode::D2
                            && self.should_refresh_3d_overlay()
                        {
                            if server_ctx.curr_map_tool_type == MapToolType::Rect {
                                self.update_tool_preview_overlay_3d(project, server_ctx);
                            } else {
                                self.update_geometry_overlay_3d(project, server_ctx);
                            }
                        }
                    }

                    redraw = true;
                }
            }
            TheEvent::RenderViewUp(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
                    {
                        if server_ctx.editor_fly_nav_mouse_down {
                            server_ctx.editor_fly_nav_active = false;
                            server_ctx.editor_fly_nav_mouse_down = false;
                            server_ctx.editor_fly_nav_space_down = false;
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .reset_mouse_tracking();
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                fl!("status_firstp_fly_nav_off"),
                            ));
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            ctx.ui.redraw_all = true;
                        }
                        return true;
                    }
                    #[cfg(target_os = "macos")]
                    if let Some(navigation) = self.macos_primary_orbit_drag.take() {
                        crate::editor::EDITCAMERA
                            .write()
                            .unwrap()
                            .reset_mouse_tracking();
                        if navigation.from_viewcube {
                            if !navigation.orbiting {
                                self.handle_viewcube_click(
                                    ui,
                                    ctx,
                                    project,
                                    server_ctx,
                                    navigation.origin,
                                );
                            }
                            return true;
                        }
                    }
                    if server_ctx.editor_view_mode != EditorViewMode::D2 {
                        crate::editor::EDITCAMERA
                            .write()
                            .unwrap()
                            .reset_mouse_tracking();
                    }

                    {
                        let undo_atom = self.current_tool_map_event(
                            MapEvent::MapUp(*coord),
                            ui,
                            ctx,
                            project,
                            server_ctx,
                        );
                        let paint_only_undo = undo_atom
                            .as_ref()
                            .is_some_and(ProjectUndoAtom::is_iso_paint_only);

                        if undo_atom.is_some() && !paint_only_undo {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.update_surfaces();
                            }
                        }
                        self.update_map_context(ui, ctx, project, server_ctx, undo_atom);
                        if server_ctx.editor_view_mode != EditorViewMode::D2
                            && !paint_only_undo
                            && self.should_refresh_3d_overlay()
                        {
                            self.update_geometry_overlay_3d(project, server_ctx);
                        }
                    }
                    self.box_select_armed = false;

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
                                    let (old_surface, new_surface) = server_ctx
                                        .moved_item_surface_placements
                                        .remove(&id)
                                        .unwrap_or((None, None));
                                    move_atoms.push(ProjectUndoAtom::MoveRegionItemInstance(
                                        server_ctx.curr_region,
                                        id,
                                        from,
                                        to,
                                        old_surface,
                                        new_surface,
                                    ));
                                }
                            }
                            server_ctx.moved_item_surface_placements.clear();

                            for atom in move_atoms {
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else {
                        server_ctx.moved_entities.clear();
                        server_ctx.moved_items.clear();
                        server_ctx.moved_item_surface_placements.clear();
                    }

                    redraw = true;
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
                    {
                        if !server_ctx.editor_fly_nav_mouse_down
                            && let Some(view_size) =
                                crate::utils::map_editor_render_view(ui, server_ctx).map(|view| {
                                    let dim = *view.dim();
                                    Vec2::new(dim.width, dim.height)
                                })
                        {
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .set_fly_pointer(coord, view_size);
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            ctx.ui.redraw_all = true;
                        }
                        return true;
                    }

                    if server_ctx.editor_view_mode != EditorViewMode::D2 {
                        if let Some(render_view) =
                            crate::utils::map_editor_render_view(ui, server_ctx)
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
                            // println!("{:?}", server_ctx.geo_hit);
                        }
                    }
                    {
                        let old_hover = server_ctx.hover;
                        let undo_atom = self.current_tool_map_event(
                            MapEvent::MapHover(*coord),
                            ui,
                            ctx,
                            project,
                            server_ctx,
                        );
                        let paint_only_undo = undo_atom
                            .as_ref()
                            .is_some_and(ProjectUndoAtom::is_iso_paint_only);
                        if undo_atom.is_some() && !paint_only_undo {
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                                map.update_surfaces();
                            }
                        }
                        self.update_map_context(ui, ctx, project, server_ctx, undo_atom);

                        if server_ctx.editor_view_mode != EditorViewMode::D2 {
                            let hover_changed = old_hover != server_ctx.hover;
                            let fast_preview_tool = server_ctx.curr_map_tool_type
                                == MapToolType::Rect
                                || paint_only_undo;

                            if server_ctx.block_tool_active && self.should_refresh_3d_overlay() {
                                self.update_geometry_overlay_3d(project, server_ctx);
                            } else if hover_changed && !fast_preview_tool {
                                self.update_geometry_overlay_3d(project, server_ctx);
                            } else if self.should_refresh_3d_overlay() {
                                self.update_tool_preview_overlay_3d(project, server_ctx);
                            }
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
                    if let TheValue::Text(command_id) = value {
                        if let Some(tool_id) = self
                            .get_game_tool_uuid_by_command_id(command_id)
                            .or_else(|| self.get_game_tool_uuid_of_name(command_id))
                        {
                            self.set_tool(tool_id, ui, ctx, project, server_ctx);
                            if let Some(tool) = self
                                .game_tools
                                .iter()
                                .find(|tool| tool.id().uuid == tool_id)
                            {
                                ctx.ui
                                    .set_widget_state(tool.id().name, TheWidgetState::Selected);
                            }
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

        self.enforce_palette_dock(ui, ctx, project, server_ctx);

        redraw
    }

    /// Returns the curently active tool.
    pub fn get_current_tool(&mut self) -> &mut Box<dyn Tool> {
        &mut self.game_tools[self.curr_game_tool]
    }

    pub fn update_current_tool(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some(map) = Self::get_tool_map_mut(project, server_ctx) else {
            return false;
        };
        let changed = self.get_current_tool().update(ui, ctx, map, server_ctx);
        if changed {
            map.changed += 1;
        }
        changed
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

    /// Leave the full-screen Game tool after the runtime stops and restore the
    /// editor tool that was active before Play. Keeping GameTool active with an
    /// Off server leaves its hidden-sidebar/full-screen state installed and
    /// routes every redraw through the wrong editor path.
    pub fn leave_game_tool_after_stop(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if self.editor_mode || !self.current_game_tool_is("tool.game") {
            server_ctx.game_mode = false;
            return false;
        }

        let game_tool_id = self.get_game_tool_uuid_by_command_id("tool.game");
        let target = self
            .previous_non_game_tool
            .take()
            .filter(|index| {
                *index < self.game_tools.len()
                    && Some(self.game_tools[*index].id().uuid) != game_tool_id
            })
            .or_else(|| {
                self.game_tools
                    .iter()
                    .position(|tool| Some(tool.id().uuid) != game_tool_id)
            });
        let Some(target) = target else {
            self.game_tools[self.curr_game_tool].tool_event(
                ToolEvent::DeActivate,
                ui,
                ctx,
                project,
                server_ctx,
            );
            return true;
        };
        let id = self.game_tools[target].id();
        let changed = self.set_tool(id.uuid, ui, ctx, project, server_ctx);
        ctx.ui.set_widget_state(id.name, TheWidgetState::Selected);
        changed
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
        self.props_3d_available = server_ctx.editor_view_mode.is_3d();
        if !self.props_3d_available
            && self.get_game_tool_uuid_by_command_id("tool.blocks") == Some(tool_id)
        {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("props_requires_3d"),
            ));
            ctx.ui
                .set_widget_state("3D Props Tool".into(), TheWidgetState::None);
            ctx.ui.set_widget_state(
                self.game_tools[self.curr_game_tool].id().name,
                TheWidgetState::Selected,
            );
            return false;
        }
        let mut redraw = false;
        let mut switched_tool = false;
        let layout_name = "Game Tool Params";
        let mut old_tool_index = 0;
        let previous_geometry_selection =
            if !self.editor_mode && server_ctx.editor_view_mode != EditorViewMode::D2 {
                project
                    .get_map(server_ctx)
                    .map(Self::geometry_selection_snapshot)
            } else {
                None
            };

        if self.prefab_mode && self.palette_mode {
            self.palette_mode = false;
            server_ctx.palette_tool_active = false;
            ctx.ui
                .set_widget_state(Self::PALETTE_BUTTON_NAME.to_string(), TheWidgetState::None);
        }

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
                self.cancel_editor_display_session();
                if self.current_game_tool_is("tool.game")
                    && !self.game_tool_index_is(old_tool_index, "tool.game")
                {
                    self.previous_non_game_tool = Some(old_tool_index);
                }
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

                // Collapse an editor before activating the next tool. Doing
                // this through a queued event races with tool activation: the
                // late event can reopen the lower dock after a full-screen
                // tool such as Game has already hidden it.
                let keep_prefab_editor_open = {
                    let manager = DOCKMANAGER.read().unwrap();
                    manager.state == DockManagerState::Editor && manager.dock == "Prefabs"
                };
                if !keep_prefab_editor_open {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .minimize_for_tool_switch(ui, ctx, project, server_ctx);
                }
            }

            if let Some(layout) = ui.get_hlayout(layout_name) {
                layout.clear();
                layout.set_reverse_index(None);
                ctx.ui.redraw_all = true;
            }

            self.get_current_tool()
                .tool_event(ToolEvent::Activate, ui, ctx, project, server_ctx);
            if switched_tool {
                let tool_id = self.game_tools[self.curr_game_tool].id().uuid;
                let display = self.game_tools[self.curr_game_tool].editor_display();
                self.begin_tool_edit(tool_id, server_ctx.pc, display, ctx);
            }

            let preserve_surface_detail_host = server_ctx.curr_map_tool_type
                == MapToolType::Linedef
                && previous_geometry_selection
                    .as_ref()
                    .is_some_and(|snapshot| !snapshot.faces.is_empty());
            let switched_to_game_tool = self.current_game_tool_is("tool.game");
            if switched_tool
                && server_ctx.editor_view_mode != EditorViewMode::D2
                && !switched_to_game_tool
            {
                if preserve_surface_detail_host {
                    server_ctx.geometry_edit_mode = GeometryEditMode::Detail;
                } else {
                    server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                    server_ctx.editing_surface = None;
                    server_ctx.editing_surface_hit_pos = None;
                    server_ctx.active_detail_surface = None;
                }
                RUSTERIX.write().unwrap().set_overlay_dirty();
            }
            if switched_tool
                && server_ctx.editor_view_mode != EditorViewMode::D2
                && !preserve_surface_detail_host
                && let Some(snapshot) = previous_geometry_selection.as_ref()
                && let Some(map) = project.get_map_mut(server_ctx)
                && Self::apply_geometry_tool_selection_carryover(
                    map,
                    server_ctx.curr_map_tool_type,
                    snapshot,
                )
            {
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));
            }

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
    pub fn game_tool_descriptor_by_id(&self, id: Uuid) -> Option<&ToolDescriptor> {
        self.game_tool_descriptors.get(&id)
    }

    fn game_tool_index_is(&self, index: usize, command_id: &str) -> bool {
        self.game_tools
            .get(index)
            .and_then(|tool| self.game_tool_descriptor_by_id(tool.id().uuid))
            .is_some_and(|descriptor| descriptor.command_id == command_id)
    }

    pub fn current_game_tool_command_id(&self) -> Option<&str> {
        self.game_tools
            .get(self.curr_game_tool)
            .and_then(|tool| self.game_tool_descriptor_by_id(tool.id().uuid))
            .map(|descriptor| descriptor.command_id.as_str())
    }

    pub fn palette_mode_active(&self) -> bool {
        self.palette_mode
    }

    fn sync_props_availability(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) {
        let available = server.editor_view_mode.is_3d();
        if self.props_3d_available != available {
            self.props_3d_available = available;
            if available {
                ui.set_enabled("3D Props Tool", ctx);
            } else {
                ui.set_disabled("3D Props Tool", ctx);
            }
        }
        if !available && self.current_game_tool_is("tool.blocks") && !self.editor_mode {
            if let Some(id) = self.get_game_tool_uuid_by_command_id("tool.rect") {
                self.set_tool(id, ui, ctx, project, server);
            }
        }
    }

    pub fn game_tool_is_available(&self, command_id: &str) -> bool {
        if self.editor_mode || !self.game_tool_command_ids.contains_key(command_id) {
            return false;
        }
        if command_id == "tool.blocks" && !self.props_3d_available {
            return false;
        }
        if matches!(command_id, "tool.tile_picker" | "tool.effects") {
            return self.prefab_mode;
        }
        !self.prefab_mode || Self::is_prefab_tool_command_id(command_id)
    }

    pub fn current_game_tool_is(&self, command_id: &str) -> bool {
        self.current_game_tool_command_id() == Some(command_id)
    }

    pub fn get_game_tool_uuid_by_command_id(&self, command_id: &str) -> Option<Uuid> {
        self.game_tool_command_ids.get(command_id).copied()
    }

    pub fn get_game_tool_by_command_id(&mut self, command_id: &str) -> Option<&mut Box<dyn Tool>> {
        let id = self.get_game_tool_uuid_by_command_id(command_id)?;
        self.game_tools.iter_mut().find(|tool| tool.id().uuid == id)
    }

    // Legacy display-name lookup retained for old project/UI events.
    pub fn get_game_tool_uuid_of_name(&self, name: &str) -> Option<Uuid> {
        for tool in self.game_tools.iter() {
            if tool.id().name == name {
                return Some(tool.id().uuid);
            }
        }
        None
    }

    // Legacy display-name lookup retained for old call sites.
    pub fn get_game_tool_of_name(&mut self, name: &str) -> Option<&mut Box<dyn Tool>> {
        for tool in self.game_tools.iter_mut() {
            if tool.id().name == name {
                return Some(tool);
            }
        }
        None
    }

    /// Update the overlay geometry.
    /// Reference renderer adapter. Native views supply their own scene below.
    pub fn update_geometry_overlay_3d(
        &mut self,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.native_world_input {
            return;
        }
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return;
        }
        let mut bridge = RUSTERIX.write().unwrap();
        bridge.client.scene.d3_overlay.clear();
        let rusterix::Rusterix {
            assets,
            client,
            scene_handler,
            ..
        } = &mut *bridge;
        self.prepare_geometry_overlay_3d(
            project,
            server_ctx,
            assets,
            client.camera_d3.as_ref(),
            scene_handler,
        );
        if server_ctx.curr_map_tool_type == MapToolType::Terrain {
            bridge.set_overlay_dirty();
        }
    }

    pub fn update_tool_preview_overlay_3d(
        &mut self,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        if self.native_world_input {
            return;
        }
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return;
        }
        let mut bridge = RUSTERIX.write().unwrap();
        let rusterix::Rusterix {
            assets,
            client,
            scene_handler,
            ..
        } = &mut *bridge;
        self.prepare_tool_preview_overlay_3d(
            project,
            server_ctx,
            assets,
            client.camera_d3.as_ref(),
            scene_handler,
        );
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

    fn ground_plane_hover(server_ctx: &ServerContext) -> Option<Vec3<f32>> {
        let ray_origin = server_ctx.hover_ray_origin_3d?;
        let ray_dir = server_ctx.hover_ray_dir_3d?;
        if ray_dir.y.abs() <= 1e-6 {
            return None;
        }
        let t = -ray_origin.y / ray_dir.y;
        (t >= 0.0).then_some(ray_origin + ray_dir * t)
    }

    /// Get the geometry hit at the given screen position.
    fn get_geometry_hit(
        &mut self,
        render_view: &dyn TheRenderViewTrait,
        coord: Vec2<i32>,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> Option<(GeoId, Vec3<f32>)> {
        // The native viewport has already queried its acknowledged, private
        // scene before this controller dispatch. Never repick on the adapter VM.
        if self.native_world_input {
            return server_ctx.geo_hit.map(|geo| (geo, server_ctx.geo_hit_pos));
        }
        if !self.should_refresh_3d_hover_pick() {
            return server_ctx
                .geo_hit
                .map(|geo_id| (geo_id, server_ctx.geo_hit_pos));
        }

        let dim = *render_view.dim();

        let screen_uv = [
            coord.x as f32 / dim.width as f32,
            coord.y as f32 / dim.height as f32,
        ];

        let mut rusterix = RUSTERIX.write().unwrap();

        server_ctx.hover_cursor_3d = None;
        server_ctx.hover_ray_origin_3d = None;
        server_ctx.hover_ray_dir_3d = None;
        server_ctx.hover_surface = None;
        server_ctx.hover_surface_hit_pos = None;
        server_ctx.hover_surface_normal = None;
        if let Some((ray_origin, ray_dir)) = rusterix.scene_handler.vm.ray_from_uv_with_size(
            dim.width as u32,
            dim.height as u32,
            screen_uv,
        ) {
            server_ctx.hover_ray_origin_3d = Some(ray_origin);
            server_ctx.hover_ray_dir_3d = Some(ray_dir);
        }

        if matches!(
            server_ctx.curr_map_tool_type,
            MapToolType::Selection
                | MapToolType::Vertex
                | MapToolType::Linedef
                | MapToolType::Sector
                | MapToolType::Effects
        ) {
            rusterix.scene_handler.vm.set_active_vm(2);
            if let Some((GeoId::Gizmo(axis), pos, _)) = rusterix.scene_handler.vm.pick_geo_id_at_uv(
                dim.width as u32,
                dim.height as u32,
                screen_uv,
                false,
                false,
            ) {
                rusterix.scene_handler.vm.set_active_vm(0);
                return Some((GeoId::Gizmo(axis), pos));
            }
            rusterix.scene_handler.vm.set_active_vm(0);
        }

        rusterix.scene_handler.vm.set_active_vm(0);
        if let Some(raw) = rusterix.scene_handler.vm.pick_geo_id_normal_at_uv(
            dim.width as u32,
            dim.height as u32,
            screen_uv,
            false,
            false,
        ) {
            server_ctx.hover_cursor_3d = Some(raw.1);
            server_ctx.hover_surface_normal = Some(raw.3);
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
            if server_ctx.curr_map_tool_type == MapToolType::IsoPaint {
                return Some((raw.0, raw.1));
            }
            if server_ctx.curr_map_tool_type == MapToolType::Sector
                && server_ctx.geometry_edit_mode != GeometryEditMode::Detail
            {
                return Some((raw.0, raw.1));
            }
            if (server_ctx.block_tool_active
                || matches!(
                    server_ctx.curr_map_tool_type,
                    MapToolType::Selection
                        | MapToolType::Vertex
                        | MapToolType::Linedef
                        | MapToolType::Sector
                ))
                && matches!(raw.0, GeoId::GeometryObject(_))
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

        if let Some((geo_id, pos, _)) = rc {
            return Some((geo_id, pos));
        }

        if let Some(ground_hit) = Self::ground_plane_hover(server_ctx) {
            server_ctx.hover_cursor_3d = Some(ground_hit);
        }

        None
    }
}

#[cfg(feature = "native-ui")]
#[path = "toollist/native_display.rs"]
mod native_display;
#[cfg(feature = "native-ui")]
pub(crate) use native_display::{NativeDisplayBinding, NativeDisplayOutcome};

mod scene_guides;
