use crate::actions::geometry_face_ops::surface_segment_points;
use crate::editor::{DOCKMANAGER, RUSTERIX, SCENEMANAGER, SIDEBARMODE, UNDOMANAGER};
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

pub struct ToolList {
    pub server_time: TheTime,
    pub render_button_text: String,
    pub authoring_mode: bool,
    pub text_game_mode: bool,
    pub palette_mode: bool,
    pub previous_sidebar_mode: Option<SidebarMode>,

    pub game_tools: Vec<Box<dyn Tool>>,
    pub curr_game_tool: usize,

    // Editor tools for dock editors
    pub editor_tools: Vec<Box<dyn EditorTool>>,
    pub curr_editor_tool: usize,
    pub editor_mode: bool,
    last_3d_hover_pick_at: Option<Instant>,
    last_3d_overlay_update_at: Option<Instant>,
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

impl ToolList {
    const AUTHORING_BUTTON_NAME: &'static str = "Authoring";
    const TEXT_PLAY_BUTTON_NAME: &'static str = "Text Play";
    const PALETTE_BUTTON_NAME: &'static str = "Palette Mode";
    const GRID_SUBDIVISIONS: [f32; 6] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0];

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

    fn shortcut_tool_name(action: ShortcutAction) -> &'static str {
        match action {
            ShortcutAction::ToolObject => "Object Tool",
            ShortcutAction::ToolVertex => "Vertex Tool",
            ShortcutAction::ToolEdge => "Linedef / Edge Tool",
            ShortcutAction::ToolFace => "Sector / Face Tool",
        }
    }

    fn shortcut_tool_uuid(&self, action: ShortcutAction) -> Option<Uuid> {
        let tool_name = Self::shortcut_tool_name(action);
        self.game_tools
            .iter()
            .find(|tool| tool.id().name == tool_name)
            .map(|tool| tool.id().uuid)
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
            bbox.expand(Vec2::broadcast(8.0));
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
                .get_float_default("terrain_width", 2.0)
                .max(0.0);
            let falloff = linedef
                .properties
                .get_float_default("terrain_tile_falloff", 1.0)
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
        linedef.properties.contains("terrain_source")
            || linedef.properties.get_bool_default("terrain_smooth", false)
    }

    fn sector_affects_terrain(sector: &rusterix::Sector) -> bool {
        sector.properties.get_int_default("terrain_mode", 0) != 0
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
                        && old.shader == new.shader
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

        if affected_sectors.is_empty()
            && affected_linedefs.is_empty()
            && affected_vertices.is_empty()
            && affected_geometry_objects.is_empty()
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
        let game_tools: Vec<Box<dyn Tool>> = vec![
            Box::new(VertexTool::new()),
            Box::new(LinedefTool::new()),
            Box::new(SectorTool::new()),
            Box::new(GeometryTool::new()),
            Box::new(RectTool::new()),
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
            palette_mode: false,
            previous_sidebar_mode: None,
            game_tools,
            curr_game_tool: 2,

            editor_tools: Vec::new(),
            curr_editor_tool: 0,
            editor_mode: false,
            last_3d_hover_pick_at: None,
            last_3d_overlay_update_at: None,
        }
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

    fn enforce_palette_dock(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.editor_mode || !self.palette_mode {
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

        let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
        if self.palette_mode {
            self.previous_sidebar_mode = Some(*SIDEBARMODE.read().unwrap());
            *SIDEBARMODE.write().unwrap() = SidebarMode::Palette;
            if current_dock == "Tiles" || current_dock == "Palette" {
                DOCKMANAGER.write().unwrap().set_dock(
                    "Palette".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );
            }
        } else if current_dock == "Palette" {
            if let Some(mode) = self.previous_sidebar_mode.take() {
                *SIDEBARMODE.write().unwrap() = mode;
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        } else if let Some(mode) = self.previous_sidebar_mode.take() {
            *SIDEBARMODE.write().unwrap() = mode;
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
                    let mut used_incremental_terrain_update = false;
                    if let Some(dirty_chunks) = rect_paint_dirty_chunks
                        && let ProjectUndoAtom::MapEdit(_, _, new_map) = &undo_atom
                    {
                        crate::utils::editor_scene_incremental_map_update(
                            (**new_map).clone(),
                            dirty_chunks,
                        );
                        used_incremental_terrain_update = true;
                    }
                    if let Some(plan) = geometry_replacement_plan
                        && let ProjectUndoAtom::MapEdit(_, _, new_map) = &undo_atom
                    {
                        used_incremental_terrain_update =
                            Self::apply_geometry_scene_update(new_map, plan);
                    }
                    if !used_incremental_terrain_update {
                        crate::utils::editor_scene_full_rebuild(project, server_ctx);
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
                TheEvent::KeyCodeDown(TheValue::KeyCode(code))
                    if ctx
                        .ui
                        .focus
                        .as_ref()
                        .is_some_and(|id| id.name == "PolyView")
                        && server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && !server_ctx.game_input_mode
                        && *code == TheKeyCode::Space =>
                {
                    false
                }
                TheEvent::KeyCodeDown(TheValue::KeyCode(code))
                    if ctx
                        .ui
                        .focus
                        .as_ref()
                        .is_some_and(|id| id.name == "PolyView")
                        && server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
                        && *code == TheKeyCode::Escape =>
                {
                    false
                }
                TheEvent::KeyCodeUp(TheValue::KeyCode(code)) if *code == TheKeyCode::Space => false,
                TheEvent::KeyUp(TheValue::Char(c))
                    if ctx
                        .ui
                        .focus
                        .as_ref()
                        .is_some_and(|id| id.name == "PolyView")
                        && server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
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

                    if new_mode != EditorViewMode::FirstP {
                        server_ctx.editor_fly_nav_active = false;
                        server_ctx.editor_fly_nav_space_down = false;
                        let mut edit_camera = crate::editor::EDITCAMERA.write().unwrap();
                        edit_camera.move_action = None;
                        edit_camera.reset_mouse_tracking();
                    }

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
                let polyview_focused = ctx
                    .ui
                    .focus
                    .as_ref()
                    .is_some_and(|id| id.name == "PolyView");
                let text_input_focused =
                    ui.focus_widget_supports_text_input(ctx) && !polyview_focused;
                let plain_key = !ui.ctrl && !ui.logo && !ui.alt;
                let shortcut_resolution = if plain_key
                    && polyview_focused
                    && !text_input_focused
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && !self.editor_mode
                    && self.get_current_tool().id().name != "Game Tool"
                    && !server_ctx.game_mode
                {
                    let shortcut_context =
                        Self::shortcut_context(project.get_map(server_ctx), server_ctx);
                    ShortcutResolver::from_toml(&project.shortcuts).resolve(*c, shortcut_context)
                } else {
                    None
                };
                let suppress_tool_accel = shortcut_resolution.is_some();

                let preserve_geometry_object_shortcut = plain_key
                    && polyview_focused
                    && server_ctx.editor_view_mode != EditorViewMode::D2
                    && server_ctx.curr_map_tool_type == MapToolType::Selection
                    && matches!(c, 'r' | 'R')
                    && project
                        .get_map(server_ctx)
                        .is_some_and(|map| !map.selected_geometry_objects.is_empty());

                if plain_key
                    && !text_input_focused
                    && !server_ctx.game_input_mode
                    && !server_ctx.text_game_mode
                    && !preserve_geometry_object_shortcut
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
                    } else if self.get_current_tool().id().name != "Game Tool"
                        && !server_ctx.game_mode
                    {
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
                    if let Some(uuid) = self.shortcut_tool_uuid(action) {
                        self.set_tool_widget_state_by_uuid(uuid, ctx);
                        return self.set_tool(uuid, ui, ctx, project, server_ctx);
                    }
                }

                if let Some(id) = &ctx.ui.focus {
                    if id.name == "PolyView" {
                        if server_ctx.editor_view_mode == EditorViewMode::FirstP
                            && server_ctx.editor_fly_nav_active
                            && !server_ctx.game_input_mode
                        {
                            let action = match c.to_ascii_lowercase() {
                                'w' => Some(crate::editcamera::CustomMoveAction::Forward),
                                's' => Some(crate::editcamera::CustomMoveAction::Backward),
                                'a' => Some(crate::editcamera::CustomMoveAction::StrafeLeft),
                                'd' => Some(crate::editcamera::CustomMoveAction::StrafeRight),
                                _ => None,
                            };
                            if let Some(action) = action {
                                crate::editor::EDITCAMERA.write().unwrap().move_action =
                                    Some(action);
                                ctx.ui.redraw_all = true;
                                return true;
                            }
                        }

                        if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
                            let plain_grid_shortcut = plain_key;
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
                                if server_ctx.curr_map_tool_type == MapToolType::Selection {
                                    let op = match c.to_ascii_lowercase() {
                                        'm' => Some(GeometryGizmoOp::Move),
                                        's' => Some(GeometryGizmoOp::Resize),
                                        _ => None,
                                    };
                                    if let Some(op) = op {
                                        server_ctx.geometry_gizmo_op = op;
                                        self.update_geometry_overlay_3d(project, server_ctx);
                                        RUSTERIX.write().unwrap().set_dirty();
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            match op {
                                                GeometryGizmoOp::Move => {
                                                    fl!("status_hud_geometry_op_move")
                                                }
                                                GeometryGizmoOp::Resize => {
                                                    fl!("status_hud_geometry_op_size")
                                                }
                                            },
                                        ));
                                        return false;
                                    }
                                }

                                if *c == 'd' || *c == 'D' {
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

                let mut acc = !text_input_focused;
                if self.get_current_tool().id().name == "Game Tool"
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
                            if !server_ctx.editor_fly_nav_active {
                                crate::editor::EDITCAMERA.write().unwrap().move_action = None;
                            }
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
                            if let Some(map) = Self::get_tool_map_mut(project, server_ctx) {
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
            TheEvent::KeyUp(TheValue::Char(c)) => {
                if let Some(id) = &ctx.ui.focus
                    && id.name == "PolyView"
                    && server_ctx.editor_view_mode == EditorViewMode::FirstP
                    && server_ctx.editor_fly_nav_active
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

                                            return true;
                                        }
                                    }
                                }
                            }
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

                            if server_ctx.editor_view_mode != EditorViewMode::D2
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
                        } else if let Some(view_size) = ui.get_render_view("PolyView").map(|view| {
                            let dim = *view.dim();
                            Vec2::new(dim.width, dim.height)
                        }) {
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .set_fly_pointer(coord, view_size);
                        }
                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        ctx.ui.redraw_all = true;
                        return true;
                    }

                    if server_ctx.editor_view_mode != EditorViewMode::D2
                        && !server_ctx.game_mode
                        && !server_ctx.game_input_mode
                    {
                        let orbit_drag = server_ctx.editor_view_mode == EditorViewMode::Orbit
                            && (ui.right_mouse_down || ui.alt);
                        let pan_drag = matches!(
                            server_ctx.editor_view_mode,
                            EditorViewMode::Orbit | EditorViewMode::Iso
                        ) && (ui.ctrl
                            || ui.logo
                            || (server_ctx.editor_view_mode == EditorViewMode::Iso
                                && (ui.right_mouse_down || ui.alt)));

                        if orbit_drag {
                            crate::editor::EDITCAMERA
                                .write()
                                .unwrap()
                                .mouse_dragged_orbit(coord);
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            ctx.ui.redraw_all = true;
                            return true;
                        }

                        if pan_drag
                            && let Some(region) = project.get_region_mut(&server_ctx.curr_region)
                            && let Some(render_view) = ui.get_render_view("PolyView")
                        {
                            let dim = *render_view.dim();
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
                        if undo_atom.is_none() {
                            Self::apply_live_geometry_owner_replacement(map, server_ctx);
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
                            crate::editor::EDITCAMERA.write().unwrap().move_action = None;
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
                    if server_ctx.editor_view_mode != EditorViewMode::D2 {
                        crate::editor::EDITCAMERA
                            .write()
                            .unwrap()
                            .reset_mouse_tracking();
                    }

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
                        if server_ctx.editor_view_mode != EditorViewMode::D2
                            && self.should_refresh_3d_overlay()
                        {
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
                    if server_ctx.editor_view_mode == EditorViewMode::FirstP
                        && server_ctx.editor_fly_nav_active
                    {
                        if !server_ctx.editor_fly_nav_mouse_down
                            && let Some(view_size) = ui.get_render_view("PolyView").map(|view| {
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
                        let old_hover = server_ctx.hover;
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
                            let hover_changed = old_hover != server_ctx.hover;
                            let fast_preview_tool =
                                server_ctx.curr_map_tool_type == MapToolType::Rect;

                            if hover_changed && !fast_preview_tool {
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
        self.enforce_palette_dock(ui, ctx, project, server_ctx);

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
        let previous_geometry_selection =
            if !self.editor_mode && server_ctx.editor_view_mode != EditorViewMode::D2 {
                project
                    .get_region(&server_ctx.curr_region)
                    .map(|region| Self::geometry_selection_snapshot(&region.map))
            } else {
                None
            };

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

            let preserve_surface_detail_host = server_ctx.curr_map_tool_type
                == MapToolType::Linedef
                && previous_geometry_selection
                    .as_ref()
                    .is_some_and(|snapshot| !snapshot.faces.is_empty());
            if switched_tool && server_ctx.editor_view_mode != EditorViewMode::D2 {
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
                && let Some(region) = project.get_region_mut(&server_ctx.curr_region)
                && Self::apply_geometry_tool_selection_carryover(
                    &mut region.map,
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

        if !server_ctx.show_editing_geometry {
            drop(rusterix);
            self.update_tool_preview_overlay_3d(project, server_ctx);
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.scene_handler.set_overlay();
            return;
        }

        // basis_vectors returns (forward, right, up)
        let (cam_forward, cam_right, cam_up) = rusterix.client.camera_d3.basis_vectors();
        let view_right = cam_right;
        let view_up = cam_up;
        let view_nudge = cam_forward * -0.002; // small toward-camera nudge to avoid z-fighting
        let camera_position = rusterix.client.camera_d3.position();
        let camera_scale = rusterix.client.camera_d3.scale();
        let camera_fov = rusterix.client.camera_d3.fov().to_radians();
        let overlay_world_size = |point: Vec3<f32>| -> f32 {
            if server_ctx.editor_view_mode == EditorViewMode::Iso {
                (camera_scale * 0.035).clamp(0.055, 0.28)
            } else {
                let distance = (point - camera_position).magnitude().max(0.1);
                (distance * (camera_fov * 0.5).tan() * 0.045).clamp(0.055, 0.28)
            }
        };
        rusterix.client.scene.d3_overlay.clear();
        let thickness = 0.15;

        if let Some(region) = project.get_region_ctx(&server_ctx) {
            let map = &region.map;
            let dungeon_only = server_ctx.editing_geo_filter == EditingGeoFilter::DungeonOnly;
            let mut visible_sector_ids: FxHashSet<u32> = FxHashSet::default();
            let mut visible_linedef_ids: FxHashSet<u32> = FxHashSet::default();
            let mut visible_vertex_ids: FxHashSet<u32> = FxHashSet::default();

            let sector_is_dungeon = |sector: &rusterix::Sector| {
                sector
                    .properties
                    .get_str_default("generated_by", String::new())
                    == "dungeon_tool"
            };
            let linedef_has_dungeon_host = |linedef_id: u32| {
                if map.sectors.iter().any(|sector| {
                    sector_is_dungeon(sector) && sector.linedefs.contains(&linedef_id)
                }) {
                    return true;
                }
                let Some(linedef) = map.find_linedef(linedef_id) else {
                    return false;
                };
                let (Some(v0), Some(v1)) = (
                    map.find_vertex(linedef.start_vertex),
                    map.find_vertex(linedef.end_vertex),
                ) else {
                    return false;
                };
                let midpoint = Vec2::new((v0.x + v1.x) * 0.5, (v0.y + v1.y) * 0.5);
                map.sectors.iter().any(|sector| {
                    if !sector_is_dungeon(sector) {
                        return false;
                    }
                    let mut bbox = sector.bounding_box(map);
                    bbox.expand(Vec2::new(0.25, 0.25));
                    bbox.contains(midpoint)
                })
            };
            let vertex_has_dungeon_host = |vertex_id: u32| {
                if map.sectors.iter().any(|sector| {
                    sector_is_dungeon(sector)
                        && sector.linedefs.iter().any(|linedef_id| {
                            map.find_linedef(*linedef_id).is_some_and(|linedef| {
                                linedef.start_vertex == vertex_id || linedef.end_vertex == vertex_id
                            })
                        })
                }) {
                    return true;
                }
                let Some(vertex) = map.find_vertex(vertex_id) else {
                    return false;
                };
                let pos = Vec2::new(vertex.x, vertex.y);
                map.sectors.iter().any(|sector| {
                    if !sector_is_dungeon(sector) {
                        return false;
                    }
                    let mut bbox = sector.bounding_box(map);
                    bbox.expand(Vec2::new(0.25, 0.25));
                    bbox.contains(pos)
                })
            };

            for sector in &map.sectors {
                let is_dungeon = sector_is_dungeon(sector);
                if dungeon_only && !is_dungeon {
                    continue;
                }
                if server_ctx.dungeon_no_ceiling && is_dungeon {
                    let dungeon_part = sector
                        .properties
                        .get_str_default("dungeon_part", String::new());
                    if dungeon_part == "ceiling" || dungeon_part == "stair_ceiling" {
                        continue;
                    }
                }
                visible_sector_ids.insert(sector.id);
                for linedef_id in &sector.linedefs {
                    visible_linedef_ids.insert(*linedef_id);
                    if let Some(linedef) = map.find_linedef(*linedef_id) {
                        visible_vertex_ids.insert(linedef.start_vertex);
                        visible_vertex_ids.insert(linedef.end_vertex);
                    }
                }
            }

            if dungeon_only {
                for linedef in &map.linedefs {
                    if linedef_has_dungeon_host(linedef.id) {
                        visible_linedef_ids.insert(linedef.id);
                        visible_vertex_ids.insert(linedef.start_vertex);
                        visible_vertex_ids.insert(linedef.end_vertex);
                    }
                }
                for vertex in &map.vertices {
                    if vertex_has_dungeon_host(vertex.id) {
                        visible_vertex_ids.insert(vertex.id);
                    }
                }
            }

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

            let grid_bbox = map.bbox().expanded(Vec2::new(16.0, 16.0));
            let grid_step = ServerContext::edit_grid_step(map.subdivisions);
            let min_x_step = (grid_bbox.min.x.min(-8.0) / grid_step).floor() as i32;
            let max_x_step = (grid_bbox.max.x.max(8.0) / grid_step).ceil() as i32;
            let min_z_step = (grid_bbox.min.y.min(-8.0) / grid_step).floor() as i32;
            let max_z_step = (grid_bbox.max.y.max(8.0) / grid_step).ceil() as i32;
            let min_x = min_x_step as f32 * grid_step;
            let max_x = max_x_step as f32 * grid_step;
            let min_z = min_z_step as f32 * grid_step;
            let max_z = max_z_step as f32 * grid_step;
            let grid_y = 0.012;
            let mut grid_index = 0u32;

            for x_step in min_x_step..=max_x_step {
                let x = x_step as f32 * grid_step;
                if x.abs() <= 0.0001 {
                    continue;
                }
                let whole_unit = x.round() as i32;
                let is_whole = (x - whole_unit as f32).abs() <= 0.0001;
                let is_major = is_whole && whole_unit % 4 == 0;
                rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                    GeoId::Unknown(0xE300_0000u32.wrapping_add(grid_index)),
                    Vec3::new(x, grid_y, min_z),
                    Vec3::new(x, grid_y, max_z),
                    if is_major {
                        [0.15, 0.15, 0.15, 0.36]
                    } else if is_whole {
                        [0.11, 0.11, 0.11, 0.28]
                    } else {
                        [0.09, 0.09, 0.09, 0.18]
                    },
                    10,
                );
                grid_index = grid_index.wrapping_add(1);
            }

            for z_step in min_z_step..=max_z_step {
                let z = z_step as f32 * grid_step;
                if z.abs() <= 0.0001 {
                    continue;
                }
                let whole_unit = z.round() as i32;
                let is_whole = (z - whole_unit as f32).abs() <= 0.0001;
                let is_major = is_whole && whole_unit % 4 == 0;
                rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                    GeoId::Unknown(0xE301_0000u32.wrapping_add(grid_index)),
                    Vec3::new(min_x, grid_y, z),
                    Vec3::new(max_x, grid_y, z),
                    if is_major {
                        [0.15, 0.15, 0.15, 0.36]
                    } else if is_whole {
                        [0.11, 0.11, 0.11, 0.28]
                    } else {
                        [0.09, 0.09, 0.09, 0.18]
                    },
                    10,
                );
                grid_index = grid_index.wrapping_add(1);
            }

            rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                GeoId::Unknown(0xE302_0000),
                Vec3::new(min_x, grid_y + 0.004, 0.0),
                Vec3::new(max_x, grid_y + 0.004, 0.0),
                [0.15, 0.15, 0.15, 0.42],
                11,
            );
            rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                GeoId::Unknown(0xE302_0001),
                Vec3::new(0.0, grid_y + 0.004, min_z),
                Vec3::new(0.0, grid_y + 0.004, max_z),
                [0.15, 0.15, 0.15, 0.42],
                11,
            );

            if !map.linedefs.is_empty() {
                let reference_y = grid_y + 0.010;
                let mut reference_index = 0u32;
                for linedef in &map.linedefs {
                    let Some(a) = map.find_vertex(linedef.start_vertex) else {
                        continue;
                    };
                    let Some(b) = map.find_vertex(linedef.end_vertex) else {
                        continue;
                    };
                    let is_selected = map.selected_linedefs.contains(&linedef.id)
                        || map.sectors.iter().any(|sector| {
                            map.selected_sectors.contains(&sector.id)
                                && sector.linedefs.contains(&linedef.id)
                        });
                    rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE303_0000u32.wrapping_add(reference_index)),
                        Vec3::new(a.x, reference_y, a.y),
                        Vec3::new(b.x, reference_y, b.y),
                        if is_selected {
                            [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 0.88]
                        } else {
                            [0.74, 0.74, 0.74, 0.44]
                        },
                        12,
                    );
                    reference_index = reference_index.wrapping_add(1);
                }

                for (vertex_index, vertex) in map.vertices.iter().enumerate() {
                    if !map.selected_vertices.contains(&vertex.id) {
                        continue;
                    }
                    let center = Vec3::new(vertex.x, reference_y + 0.004, vertex.y);
                    let half = 0.08;
                    rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE304_0000u32.wrapping_add(vertex_index as u32)),
                        center - Vec3::new(half, 0.0, 0.0),
                        center + Vec3::new(half, 0.0, 0.0),
                        [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 0.88],
                        13,
                    );
                    rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE305_0000u32.wrapping_add(vertex_index as u32)),
                        center - Vec3::new(0.0, 0.0, half),
                        center + Vec3::new(0.0, 0.0, half),
                        [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 0.88],
                        13,
                    );
                }
            }

            let direct_geometry_tool = matches!(
                server_ctx.curr_map_tool_type,
                MapToolType::Selection
                    | MapToolType::Vertex
                    | MapToolType::Linedef
                    | MapToolType::Sector
            );
            let selected_tile = rusterix.scene_handler.selected;
            let white_tile = rusterix.scene_handler.white;
            let push_cube_marker = |rusterix: &mut rusterix::Rusterix,
                                    marker_id: GeoId,
                                    selected: bool,
                                    center: Vec3<f32>,
                                    size: f32,
                                    opacity: f32,
                                    layer: i32| {
                let mut cube = scenevm::Poly3D::cube(
                    marker_id,
                    if selected { selected_tile } else { white_tile },
                    center,
                    size,
                )
                .with_opacity(opacity);
                cube.layer = layer;
                rusterix.scene_handler.overlay_3d.add_3d(cube);
            };
            let push_handle_rect = |rusterix: &mut rusterix::Rusterix,
                                    outline_id_base: u32,
                                    hit_id: GeoId,
                                    selected: bool,
                                    center: Vec3<f32>,
                                    size: f32,
                                    color: [f32; 4],
                                    opacity: f32| {
                let half = size * 0.5;
                let p0 = center - view_right * half - view_up * half;
                let p1 = center + view_right * half - view_up * half;
                let p2 = center + view_right * half + view_up * half;
                let p3 = center - view_right * half + view_up * half;
                let mut fill = scenevm::Poly3D::poly(
                    hit_id,
                    if selected { selected_tile } else { white_tile },
                    vec![
                        [p0.x, p0.y, p0.z, 1.0],
                        [p1.x, p1.y, p1.z, 1.0],
                        [p2.x, p2.y, p2.z, 1.0],
                        [p3.x, p3.y, p3.z, 1.0],
                    ],
                    vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    vec![(0, 1, 2), (0, 2, 3)],
                )
                .with_opacity(opacity);
                fill.layer = 39;
                rusterix.scene_handler.overlay_3d.add_3d(fill);

                for (index, (a, b)) in [(p0, p1), (p1, p2), (p2, p3), (p3, p0)]
                    .into_iter()
                    .enumerate()
                {
                    rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(outline_id_base.wrapping_add(index as u32)),
                        a,
                        b,
                        color,
                        40,
                    );
                }
            };
            for object in &map.geometry_objects {
                let selected = map.selected_geometry_objects.contains(&object.id);
                let hovered = server_ctx.geo_hit == Some(GeoId::GeometryObject(object.id));
                if !selected && !hovered {
                    continue;
                }
                let id_salt = (object.id.as_u128() as u32) & 0x0000_F000;

                let mut min = Vec3::broadcast(f32::INFINITY);
                let mut max = Vec3::broadcast(f32::NEG_INFINITY);
                let mut found = false;
                for vertex in &object.vertices {
                    let world = object.transform_point(*vertex) + view_nudge;
                    if !world.x.is_finite() || !world.y.is_finite() || !world.z.is_finite() {
                        continue;
                    }
                    min.x = min.x.min(world.x);
                    min.y = min.y.min(world.y);
                    min.z = min.z.min(world.z);
                    max.x = max.x.max(world.x);
                    max.y = max.y.max(world.y);
                    max.z = max.z.max(world.z);
                    found = true;
                }
                if !found {
                    continue;
                }

                if direct_geometry_tool {
                    let mut gizmo_min = Vec3::broadcast(f32::INFINITY);
                    let mut gizmo_max = Vec3::broadcast(f32::NEG_INFINITY);
                    let mut gizmo_found = false;
                    let mut has_mode_selection =
                        server_ctx.curr_map_tool_type == MapToolType::Selection;
                    if server_ctx.curr_map_tool_type != MapToolType::Selection {
                        for (_, vertex_index) in map
                            .selected_geometry_vertices
                            .iter()
                            .filter(|(id, _)| *id == object.id)
                        {
                            let Some(vertex) = object.vertices.get(*vertex_index) else {
                                continue;
                            };
                            let world = object.transform_point(*vertex) + view_nudge;
                            gizmo_min.x = gizmo_min.x.min(world.x);
                            gizmo_min.y = gizmo_min.y.min(world.y);
                            gizmo_min.z = gizmo_min.z.min(world.z);
                            gizmo_max.x = gizmo_max.x.max(world.x);
                            gizmo_max.y = gizmo_max.y.max(world.y);
                            gizmo_max.z = gizmo_max.z.max(world.z);
                            gizmo_found = true;
                            if matches!(
                                server_ctx.curr_map_tool_type,
                                MapToolType::Vertex | MapToolType::Linedef
                            ) {
                                has_mode_selection = true;
                            }
                        }
                        for (_, face_index) in map
                            .selected_geometry_faces
                            .iter()
                            .filter(|(id, _)| *id == object.id)
                        {
                            let Some(face) = object.faces.get(*face_index) else {
                                continue;
                            };
                            for vertex_index in &face.indices {
                                let Some(vertex) = object.vertices.get(*vertex_index) else {
                                    continue;
                                };
                                let world = object.transform_point(*vertex) + view_nudge;
                                gizmo_min.x = gizmo_min.x.min(world.x);
                                gizmo_min.y = gizmo_min.y.min(world.y);
                                gizmo_min.z = gizmo_min.z.min(world.z);
                                gizmo_max.x = gizmo_max.x.max(world.x);
                                gizmo_max.y = gizmo_max.y.max(world.y);
                                gizmo_max.z = gizmo_max.z.max(world.z);
                                gizmo_found = true;
                            }
                            if server_ctx.curr_map_tool_type == MapToolType::Sector {
                                has_mode_selection = true;
                            }
                        }
                    }
                    let gizmo_min = if gizmo_found { gizmo_min } else { min };
                    let gizmo_max = if gizmo_found { gizmo_max } else { max };
                    let center = (gizmo_min + gizmo_max) * 0.5;
                    let base_overlay_size = overlay_world_size(center);
                    let axis_len = (base_overlay_size * 7.0).clamp(0.38, 1.75);
                    let move_handle_size = (base_overlay_size * 1.15).clamp(0.08, 0.22);
                    let resize_handle_size = (base_overlay_size * 0.95).clamp(0.07, 0.18);
                    let draw_selection_gizmo = server_ctx.curr_map_tool_type
                        != MapToolType::Selection
                        || map.selected_geometry_objects.len() <= 1
                        || map
                            .selected_geometry_objects
                            .first()
                            .is_some_and(|selected_id| *selected_id == object.id);
                    if selected && has_mode_selection && draw_selection_gizmo {
                        let show_move_gizmo = server_ctx.curr_map_tool_type
                            != MapToolType::Selection
                            || server_ctx.geometry_gizmo_op == GeometryGizmoOp::Move;
                        let show_resize_gizmo = server_ctx.curr_map_tool_type
                            == MapToolType::Selection
                            && server_ctx.geometry_gizmo_op == GeometryGizmoOp::Resize;
                        if show_move_gizmo {
                            for (axis_id, delta, color) in [
                                (1, Vec3::new(axis_len, 0.0, 0.0), [0.86, 0.22, 0.22, 1.0]),
                                (2, Vec3::new(0.0, axis_len, 0.0), [0.28, 0.78, 0.32, 1.0]),
                                (3, Vec3::new(0.0, 0.0, axis_len), [0.32, 0.48, 0.94, 1.0]),
                            ]
                            .into_iter()
                            {
                                let handle_center = center + delta + view_nudge;
                                rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                                    GeoId::Gizmo(axis_id),
                                    center + view_nudge,
                                    handle_center,
                                    color,
                                    42 + axis_id as i32,
                                );
                                push_handle_rect(
                                    &mut rusterix,
                                    0xE3A0_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add(axis_id << 4),
                                    GeoId::Gizmo(axis_id),
                                    false,
                                    handle_center + cam_forward * -0.012,
                                    move_handle_size,
                                    color,
                                    0.88,
                                );
                            }
                        }
                        if show_resize_gizmo {
                            for (axis_id, handle_center, color) in [
                                (
                                    101,
                                    Vec3::new(gizmo_min.x, center.y, center.z),
                                    [0.86, 0.22, 0.22, 1.0],
                                ),
                                (
                                    102,
                                    Vec3::new(gizmo_max.x, center.y, center.z),
                                    [0.86, 0.22, 0.22, 1.0],
                                ),
                                (
                                    103,
                                    Vec3::new(center.x, gizmo_min.y, center.z),
                                    [0.28, 0.78, 0.32, 1.0],
                                ),
                                (
                                    104,
                                    Vec3::new(center.x, gizmo_max.y, center.z),
                                    [0.28, 0.78, 0.32, 1.0],
                                ),
                                (
                                    105,
                                    Vec3::new(center.x, center.y, gizmo_min.z),
                                    [0.32, 0.48, 0.94, 1.0],
                                ),
                                (
                                    106,
                                    Vec3::new(center.x, center.y, gizmo_max.z),
                                    [0.32, 0.48, 0.94, 1.0],
                                ),
                            ] {
                                push_handle_rect(
                                    &mut rusterix,
                                    0xE3B0_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add(axis_id << 4),
                                    GeoId::Gizmo(axis_id),
                                    false,
                                    handle_center + view_nudge + cam_forward * -0.014,
                                    resize_handle_size,
                                    color,
                                    0.72,
                                );
                            }
                        }
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Selection {
                    let corners = [
                        Vec3::new(min.x, min.y, min.z),
                        Vec3::new(max.x, min.y, min.z),
                        Vec3::new(max.x, max.y, min.z),
                        Vec3::new(min.x, max.y, min.z),
                        Vec3::new(min.x, min.y, max.z),
                        Vec3::new(max.x, min.y, max.z),
                        Vec3::new(max.x, max.y, max.z),
                        Vec3::new(min.x, max.y, max.z),
                    ];
                    let edges = [
                        (0, 1),
                        (1, 2),
                        (2, 3),
                        (3, 0),
                        (4, 5),
                        (5, 6),
                        (6, 7),
                        (7, 4),
                        (0, 4),
                        (1, 5),
                        (2, 6),
                        (3, 7),
                    ];
                    let color = if selected {
                        [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                    } else {
                        [1.0, 1.0, 1.0, 0.78]
                    };
                    for (edge_index, (a, b)) in edges.iter().enumerate() {
                        rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                            GeoId::Unknown(
                                0xE340_0000u32
                                    .wrapping_add(id_salt)
                                    .wrapping_add(edge_index as u32),
                            ),
                            corners[*a],
                            corners[*b],
                            color,
                            20,
                        );
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Vertex {
                    for (vertex_index, vertex) in object.vertices.iter().enumerate() {
                        let is_selected = map
                            .selected_geometry_vertices
                            .contains(&(object.id, vertex_index));
                        push_cube_marker(
                            &mut rusterix,
                            GeoId::GeometryObject(object.id),
                            is_selected,
                            object.transform_point(*vertex) + view_nudge + cam_forward * -0.01,
                            overlay_world_size(object.transform_point(*vertex))
                                * if is_selected { 0.88 } else { 0.62 },
                            if is_selected { 0.94 } else { 0.66 },
                            39,
                        );
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Linedef {
                    let mut edge_index = 0u32;
                    let mut surface_line_index = 0u32;
                    let mut drawn_geometry_vertices = FxHashSet::default();
                    for (face_index, face) in object.faces.iter().enumerate() {
                        for index in 0..face.indices.len() {
                            let Some(a) = object.vertices.get(face.indices[index]) else {
                                continue;
                            };
                            let Some(b) = object
                                .vertices
                                .get(face.indices[(index + 1) % face.indices.len()])
                            else {
                                continue;
                            };
                            let a_selected = map
                                .selected_geometry_vertices
                                .contains(&(object.id, face.indices[index]));
                            let b_selected = map.selected_geometry_vertices.contains(&(
                                object.id,
                                face.indices[(index + 1) % face.indices.len()],
                            ));
                            let edge_selected = a_selected && b_selected;
                            rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE370_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add(edge_index),
                                ),
                                object.transform_point(*a) + view_nudge,
                                object.transform_point(*b) + view_nudge,
                                if edge_selected {
                                    [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                                } else {
                                    [1.0, 1.0, 1.0, 0.82]
                                },
                                20,
                            );
                            for (vertex_index, vertex, is_selected) in [
                                (face.indices[index], a, a_selected),
                                (
                                    face.indices[(index + 1) % face.indices.len()],
                                    b,
                                    b_selected,
                                ),
                            ] {
                                if is_selected && drawn_geometry_vertices.insert(vertex_index) {
                                    push_cube_marker(
                                        &mut rusterix,
                                        GeoId::GeometryObject(object.id),
                                        true,
                                        object.transform_point(*vertex)
                                            + view_nudge
                                            + cam_forward * -0.01,
                                        overlay_world_size(object.transform_point(*vertex)) * 0.72,
                                        0.92,
                                        39,
                                    );
                                }
                            }
                            edge_index = edge_index.wrapping_add(1);
                        }

                        let normal = if face.indices.len() >= 3 {
                            let first = object.transform_point(
                                *object
                                    .vertices
                                    .get(face.indices[0])
                                    .unwrap_or(&Vec3::zero()),
                            );
                            let mut normal = Vec3::<f32>::zero();
                            for index in 1..face.indices.len().saturating_sub(1) {
                                let Some(a) = object.vertices.get(face.indices[index]) else {
                                    continue;
                                };
                                let Some(b) = object.vertices.get(face.indices[index + 1]) else {
                                    continue;
                                };
                                normal += (object.transform_point(*a) - first)
                                    .cross(object.transform_point(*b) - first);
                            }
                            normal.try_normalized().unwrap_or(cam_forward)
                        } else {
                            cam_forward
                        };
                        let mut drawn_surface_points = FxHashSet::default();
                        for (segment_index, segment) in face.surface_segments.iter().enumerate() {
                            let Some(points) = surface_segment_points(face, segment, normal, 8)
                            else {
                                continue;
                            };
                            let segment_selected = map
                                .selected_geometry_surface_segments
                                .contains(&(object.id, face_index, segment_index));
                            let world_points = points
                                .iter()
                                .map(|point| {
                                    object.transform_point(*point) + view_nudge + normal * 0.014
                                })
                                .collect::<Vec<_>>();
                            let side = world_points
                                .windows(2)
                                .find_map(|points| {
                                    let dir = (points[1] - points[0]).try_normalized()?;
                                    normal.cross(dir).try_normalized()
                                })
                                .unwrap_or_else(|| {
                                    cam_right.try_normalized().unwrap_or(Vec3::unit_x())
                                });
                            let line_offset = 0.018;
                            let base_id = 0xE390_0000u32
                                .wrapping_add(id_salt)
                                .wrapping_add((face_index as u32) << 11)
                                .wrapping_add(surface_line_index.wrapping_mul(128));
                            let strokes = if segment_selected {
                                [
                                    (-line_offset, [0.30, 0.12, 0.43, 0.88], 23),
                                    (0.0, [0.98, 0.72, 1.0, 1.0], 24),
                                    (line_offset, [1.0, 0.92, 1.0, 0.86], 25),
                                ]
                            } else {
                                [
                                    (-line_offset, [0.03, 0.03, 0.03, 0.62], 23),
                                    (0.0, [1.0, 1.0, 1.0, 0.98], 24),
                                    (line_offset, [0.44, 0.78, 1.0, 0.72], 25),
                                ]
                            };
                            for (stroke_index, (offset, color, layer)) in
                                strokes.into_iter().enumerate()
                            {
                                let delta = side * offset;
                                for (piece_index, piece) in world_points.windows(2).enumerate() {
                                    let offset_id = stroke_index as u32 * 32 + piece_index as u32;
                                    rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                                        GeoId::Unknown(base_id.wrapping_add(offset_id)),
                                        piece[0] + delta,
                                        piece[1] + delta,
                                        color,
                                        layer,
                                    );
                                }
                            }
                            for (point_index, point) in [
                                (segment.start, world_points[0]),
                                (
                                    segment.end,
                                    *world_points.last().unwrap_or(&world_points[0]),
                                ),
                            ] {
                                drawn_surface_points.insert(point_index);
                                let point_selected = map
                                    .selected_geometry_surface_points
                                    .contains(&(object.id, face_index, point_index))
                                    || segment_selected;
                                push_cube_marker(
                                    &mut rusterix,
                                    GeoId::GeometryObject(object.id),
                                    point_selected,
                                    point,
                                    overlay_world_size(point)
                                        * if point_selected { 0.66 } else { 0.50 },
                                    if point_selected { 0.9 } else { 0.48 },
                                    39,
                                );
                            }
                            surface_line_index = surface_line_index.wrapping_add(1);
                        }
                        for (point_index, surface_point) in face.surface_points.iter().enumerate() {
                            if drawn_surface_points.contains(&point_index) {
                                continue;
                            }
                            let point = object.transform_point(surface_point.position)
                                + view_nudge
                                + normal * 0.014;
                            let point_selected = map.selected_geometry_surface_points.contains(&(
                                object.id,
                                face_index,
                                point_index,
                            ));
                            push_cube_marker(
                                &mut rusterix,
                                GeoId::GeometryObject(object.id),
                                point_selected,
                                point,
                                overlay_world_size(point)
                                    * if point_selected { 0.66 } else { 0.50 },
                                if point_selected { 0.9 } else { 0.48 },
                                39,
                            );
                        }
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Sector {
                    for (face_index, face) in object.faces.iter().enumerate() {
                        let is_selected = map
                            .selected_geometry_faces
                            .contains(&(object.id, face_index));
                        if is_selected && face.indices.len() >= 3 {
                            let vertices = face
                                .indices
                                .iter()
                                .filter_map(|vertex_index| object.vertices.get(*vertex_index))
                                .map(|vertex| {
                                    let point = object.transform_point(*vertex)
                                        + view_nudge
                                        + cam_forward * -0.012;
                                    [point.x, point.y, point.z, 1.0]
                                })
                                .collect::<Vec<_>>();
                            if vertices.len() >= 3 {
                                let uv_len = vertices.len();
                                let mut indices = Vec::with_capacity(vertices.len() - 2);
                                for index in 1..vertices.len() - 1 {
                                    indices.push((0, index, index + 1));
                                }
                                let mut fill = scenevm::Poly3D::poly(
                                    GeoId::GeometryObject(object.id),
                                    selected_tile,
                                    vertices,
                                    vec![[0.0, 0.0]; uv_len],
                                    indices,
                                )
                                .with_opacity(0.32);
                                fill.layer = 38;
                                rusterix.scene_handler.overlay_3d.add_3d(fill);
                            }
                        }
                        for edge_index in 0..face.indices.len() {
                            let Some(a) = object.vertices.get(face.indices[edge_index]) else {
                                continue;
                            };
                            let Some(b) = object
                                .vertices
                                .get(face.indices[(edge_index + 1) % face.indices.len()])
                            else {
                                continue;
                            };
                            rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE380_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add((face_index as u32) << 8)
                                        .wrapping_add(edge_index as u32),
                                ),
                                object.transform_point(*a) + view_nudge,
                                object.transform_point(*b) + view_nudge,
                                if is_selected {
                                    [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                                } else {
                                    [1.0, 1.0, 1.0, 0.82]
                                },
                                20,
                            );
                        }
                    }
                }

                if selected {
                    for (selection_index, (_, face_index)) in map
                        .selected_geometry_faces
                        .iter()
                        .filter(|(id, _)| *id == object.id)
                        .enumerate()
                    {
                        let Some(face) = object.faces.get(*face_index) else {
                            continue;
                        };
                        if face.indices.len() < 2 {
                            continue;
                        }
                        for edge_index in 0..face.indices.len() {
                            let Some(a) = object.vertices.get(face.indices[edge_index]) else {
                                continue;
                            };
                            let Some(b) = object
                                .vertices
                                .get(face.indices[(edge_index + 1) % face.indices.len()])
                            else {
                                continue;
                            };
                            rusterix.scene_handler.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE350_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add((selection_index as u32) << 8)
                                        .wrapping_add(edge_index as u32),
                                ),
                                object.transform_point(*a) + view_nudge,
                                object.transform_point(*b) + view_nudge,
                                [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0],
                                21,
                            );
                        }
                    }

                    for (_, vertex_index) in map
                        .selected_geometry_vertices
                        .iter()
                        .filter(|(id, _)| *id == object.id)
                    {
                        let Some(vertex) = object.vertices.get(*vertex_index) else {
                            continue;
                        };
                        push_cube_marker(
                            &mut rusterix,
                            GeoId::GeometryObject(object.id),
                            true,
                            object.transform_point(*vertex) + view_nudge + cam_forward * -0.012,
                            overlay_world_size(object.transform_point(*vertex)) * 0.88,
                            0.94,
                            39,
                        );
                    }
                }
            }

            if direct_geometry_tool && !map.geometry_objects.is_empty() {
                drop(rusterix);
                self.update_tool_preview_overlay_3d(project, server_ctx);
                let mut rusterix = RUSTERIX.write().unwrap();
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
                        if dungeon_only && !visible_vertex_ids.contains(&v.id) && !selected {
                            continue;
                        }

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
                            if dungeon_only
                                && !visible_linedef_ids.contains(&linedef.id)
                                && !is_selected
                                && !is_hovered
                            {
                                continue;
                            }
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
                    let sector_is_hovered = server_ctx.hover.2 == Some(sector_id);
                    if dungeon_only
                        && !visible_sector_ids.contains(&sector_id)
                        && !sector_is_selected
                        && !sector_is_hovered
                    {
                        continue;
                    }

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
                                    line_is_selected = sector_is_selected || sector_is_hovered;
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

        drop(rusterix);
        self.update_tool_preview_overlay_3d(project, server_ctx);
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.scene_handler.set_overlay();
    }

    pub fn update_tool_preview_overlay_3d(
        &mut self,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return;
        }

        let Some(region) = project.get_region_ctx(server_ctx) else {
            return;
        };
        let map = &region.map;

        let mut rusterix = RUSTERIX.write().unwrap();
        if rusterix.scene_handler.vm.vm_layer_count() < 4 {
            rusterix.scene_handler.clear_overlay();
        }

        rusterix.scene_handler.tool_overlay_3d = scenevm::Chunk::default();
        rusterix.scene_handler.tool_overlay_3d.priority = 1;

        let (cam_forward, _, _) = rusterix.client.camera_d3.basis_vectors();
        let view_nudge = cam_forward * -0.002;
        let thickness = 0.15;
        let white = rusterix.scene_handler.white;

        if server_ctx.curr_map_tool_type == MapToolType::Rect {
            if let Some(terrain_id) = server_ctx.rect_terrain_id {
                let config = TerrainConfig::default();
                let corners = TerrainGenerator::tile_outline_world(map, terrain_id, &config);
                let n = TerrainGenerator::tile_normal(map, terrain_id, &config);
                for i in 0..4 {
                    rusterix.scene_handler.tool_overlay_3d.add_line_3d(
                        GeoId::Unknown(i as u32),
                        white,
                        corners[i] + view_nudge,
                        corners[(i + 1) % 4] + view_nudge,
                        thickness,
                        n,
                        100,
                    );
                }
            } else if let Some(sector_id) = server_ctx.rect_sector_id_3d {
                let mut index = 0;
                for surface in map.surfaces.values() {
                    if surface.sector_id == sector_id {
                        let corners =
                            surface.tile_outline_world_local(server_ctx.rect_tile_id_3d, map);
                        let n = surface.plane.normal;
                        for i in 0..4 {
                            rusterix.scene_handler.tool_overlay_3d.add_line_3d(
                                GeoId::Unknown(index),
                                white,
                                corners[i] + view_nudge,
                                corners[(i + 1) % 4] + view_nudge,
                                thickness,
                                n,
                                100,
                            );
                            index += 1;
                        }
                    }
                }
            } else if let Some((object_id, face_index)) = server_ctx.rect_geometry_face_3d
                && let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == object_id)
                && let Some(face) = object.faces.get(face_index)
                && face.indices.len() >= 3
            {
                let points = face
                    .indices
                    .iter()
                    .filter_map(|vertex_index| object.vertices.get(*vertex_index).copied())
                    .collect::<Vec<_>>();
                if points.len() == face.indices.len() {
                    let first = points[0];
                    let mut normal = Vec3::<f32>::zero();
                    for index in 1..points.len().saturating_sub(1) {
                        normal += (points[index] - first).cross(points[index + 1] - first);
                    }
                    if let Some(local_normal) = normal.try_normalized() {
                        let abs = Vec3::new(
                            local_normal.x.abs(),
                            local_normal.y.abs(),
                            local_normal.z.abs(),
                        );
                        let to_uv = |point: Vec3<f32>| {
                            if abs.y >= abs.x && abs.y >= abs.z {
                                Vec2::new(point.x, point.z)
                            } else if abs.x >= abs.z {
                                Vec2::new(point.z, point.y)
                            } else {
                                Vec2::new(point.x, point.y)
                            }
                        };
                        let from_uv = |uv: Vec2<f32>| {
                            if abs.y >= abs.x && abs.y >= abs.z {
                                Vec3::new(uv.x, first.y, uv.y)
                            } else if abs.x >= abs.z {
                                Vec3::new(first.x, uv.y, uv.x)
                            } else {
                                Vec3::new(uv.x, uv.y, first.z)
                            }
                        };
                        let min_uv = points
                            .iter()
                            .map(|point| to_uv(*point))
                            .fold(Vec2::broadcast(f32::INFINITY), |acc, uv| {
                                Vec2::new(acc.x.min(uv.x), acc.y.min(uv.y))
                            });
                        let tile = server_ctx.rect_tile_id_3d;
                        let tile_min = min_uv + Vec2::new(tile.0 as f32, tile.1 as f32);
                        let local_corners = [
                            from_uv(tile_min),
                            from_uv(tile_min + Vec2::new(1.0, 0.0)),
                            from_uv(tile_min + Vec2::new(1.0, 1.0)),
                            from_uv(tile_min + Vec2::new(0.0, 1.0)),
                        ];
                        let world_corners =
                            local_corners.map(|point| object.transform_point(point) + view_nudge);
                        let world_normal = {
                            let origin = object.transform_point(Vec3::zero());
                            (object.transform_point(local_normal) - origin)
                                .try_normalized()
                                .unwrap_or(local_normal)
                        };
                        for i in 0..4 {
                            rusterix.scene_handler.tool_overlay_3d.add_line_3d(
                                GeoId::Unknown(0xE3D0_0000u32.wrapping_add(i as u32)),
                                white,
                                world_corners[i],
                                world_corners[(i + 1) % 4],
                                thickness,
                                world_normal,
                                100,
                            );
                        }
                    }
                }
            }
        } else if server_ctx.curr_map_tool_type == MapToolType::Linedef {
            if let Some(start) = map.curr_grid_pos_3d {
                let target = server_ctx.hover_cursor_3d.or_else(|| {
                    matches!(server_ctx.geo_hit, Some(GeoId::GeometryObject(_)))
                        .then_some(server_ctx.geo_hit_pos)
                });
                if let Some(target) = target {
                    rusterix.scene_handler.tool_overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE3C0_0000),
                        start + view_nudge + cam_forward * -0.012,
                        target + view_nudge + cam_forward * -0.012,
                        [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0],
                        100,
                    );
                }
            }
        }

        let id = rusterix.scene_handler.tool_overlay_3d_id;
        let chunk = rusterix.scene_handler.tool_overlay_3d.clone();
        rusterix.scene_handler.vm.set_active_vm(3);
        rusterix
            .scene_handler
            .vm
            .execute(scenevm::Atom::AddChunk { id, chunk });
        rusterix.scene_handler.vm.set_active_vm(0);
        rusterix.scene_handler.mark_dynamics_dirty();
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
            if matches!(
                server_ctx.curr_map_tool_type,
                MapToolType::Selection
                    | MapToolType::Vertex
                    | MapToolType::Linedef
                    | MapToolType::Sector
            ) && matches!(raw.0, GeoId::GeometryObject(_))
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
