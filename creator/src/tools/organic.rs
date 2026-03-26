use crate::editor::{DOCKMANAGER, RUSTERIX, SCENEMANAGER};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Surface;
use rusterix::chunkbuilder::terrain_generator::{TerrainConfig, TerrainGenerator};
use rusterix::{OrganicNodeKind, PixelSource};
use scenevm::GeoId;
use std::collections::HashSet;

#[derive(Clone)]
struct OrganicBrushEval {
    valid: bool,
    radius: f32,
    flow: f32,
    jitter: f32,
    depth: f32,
    cell_size: f32,
    circle_radius: f32,
    circle_softness: f32,
    canopy_lobes: i32,
    canopy_spread: f32,
    use_canopy_shape: bool,
    bush_height: f32,
    bush_layers: i32,
    bush_taper: f32,
    bush_breakup: f32,
    use_bush_shape: bool,
    line_length: f32,
    line_width: f32,
    line_softness: f32,
    use_line_shape: bool,
    scatter_count: i32,
    scatter_jitter: f32,
    height_depth: f32,
    height_falloff: f32,
    noise_scale: f32,
    noise_strength: f32,
    noise_seed: i32,
    channel: i32,
    material_source: Option<PixelSource>,
    palette_start: i32,
    palette_count: i32,
    palette_mode: i32,
    stroke_seed: i32,
}

pub struct OrganicTool {
    id: TheId,
    previous_dock: Option<String>,
    stroke_active: bool,
    stroke_changed: bool,
    stroke_prev_map: Option<Map>,
    stroke_work_map: Option<Map>,
    dirty_chunks: HashSet<(i32, i32)>,
    last_stroke_hit_pos: Option<Vec3<f32>>,
    stroke_seed: i32,
    vine_seq: i32,
}

impl Tool for OrganicTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Organic Paint Tool"),
            previous_dock: None,
            stroke_active: false,
            stroke_changed: false,
            stroke_prev_map: None,
            stroke_work_map: None,
            dirty_chunks: HashSet::default(),
            last_stroke_hit_pos: None,
            stroke_seed: 0,
            vine_seq: 0,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_organic")
    }

    fn icon_name(&self) -> String {
        str!("tree")
    }

    fn accel(&self) -> Option<char> {
        Some('O')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/overview".to_string())
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                self.cancel_stroke(map_from_project(project, server_ctx));

                let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Organic" {
                    self.previous_dock = if current_dock.is_empty() {
                        None
                    } else {
                        Some(current_dock)
                    };
                }
                DOCKMANAGER.write().unwrap().set_dock(
                    "Organic".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );
                Self::sync_active_graph_to_map(map_from_project(project, server_ctx));
                true
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                self.cancel_stroke(map_from_project(project, server_ctx));
                if DOCKMANAGER.read().unwrap().dock == "Organic"
                    && let Some(prev) = self.previous_dock.take()
                {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .set_dock(prev, ui, ctx, project, server_ctx);
                }
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            if matches!(map_event, MapUp(_) | MapEscape) {
                self.cancel_stroke(map);
            }
            return None;
        }

        match map_event {
            MapHover(_pos) => {
                server_ctx.hover_cursor = None;
            }
            MapClicked(_coord) => {
                let erase = ui.shift;
                self.begin_stroke_if_needed(map);
                eprintln!("{}", Self::hover_debug_text(map, server_ctx));
                if let Some(work_map) = self.stroke_work_map.as_mut() {
                    let changed = Self::apply_stroke(
                        work_map,
                        server_ctx,
                        erase,
                        self.stroke_seed,
                        &mut self.vine_seq,
                        &mut self.dirty_chunks,
                        &mut self.last_stroke_hit_pos,
                    );
                    if changed {
                        self.stroke_changed = true;
                        *map = work_map.clone();
                        return None;
                    }
                }
            }
            MapDragged(_coord) => {
                let erase = ui.shift;
                self.begin_stroke_if_needed(map);
                eprintln!("{}", Self::hover_debug_text(map, server_ctx));
                if let Some(work_map) = self.stroke_work_map.as_mut() {
                    let changed = Self::apply_stroke(
                        work_map,
                        server_ctx,
                        erase,
                        self.stroke_seed,
                        &mut self.vine_seq,
                        &mut self.dirty_chunks,
                        &mut self.last_stroke_hit_pos,
                    );
                    if changed {
                        self.stroke_changed = true;
                        *map = work_map.clone();
                        return None;
                    }
                }
            }
            MapUp(_) => {
                server_ctx.hover_cursor = None;
                return self.finish_stroke(map, server_ctx);
            }
            MapEscape => {
                server_ctx.hover_cursor = None;
                self.cancel_stroke(map);
            }
            MapDelete | MapKey(_) => {}
        }
        None
    }
}

impl OrganicTool {
    fn begin_stroke_if_needed(&mut self, map: &Map) {
        if self.stroke_active {
            return;
        }
        self.stroke_active = true;
        self.stroke_changed = false;
        self.stroke_prev_map = Some(map.clone());
        self.stroke_work_map = Some(map.clone());
        self.dirty_chunks.clear();
        self.last_stroke_hit_pos = None;
        self.stroke_seed = (map.changed as i32)
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.vine_seq = 0;
    }

    fn finish_stroke(
        &mut self,
        map: &mut Map,
        server_ctx: &ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if !self.stroke_active {
            return None;
        }

        self.stroke_active = false;
        let prev = self.stroke_prev_map.take();
        let work = self.stroke_work_map.take();
        let changed = self.stroke_changed;
        self.stroke_changed = false;

        if changed && let (Some(prev), Some(mut work)) = (prev, work) {
            Self::finalize_vine_stroke_caps(&mut work, self.stroke_seed);
            if !self.dirty_chunks.is_empty() {
                let mut sm = SCENEMANAGER.write().unwrap();
                sm.update_map(work.clone());
                sm.add_dirty(self.dirty_chunks.iter().copied().collect());
                RUSTERIX.write().unwrap().set_dirty();
            }
            *map = work.clone();
            return Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
                Box::new(prev),
                Box::new(work),
            ));
        }

        None
    }

    fn cancel_stroke(&mut self, map: &mut Map) {
        if self.stroke_active
            && let Some(prev) = self.stroke_prev_map.take()
        {
            *map = prev;
        }
        self.stroke_active = false;
        self.stroke_changed = false;
        self.stroke_work_map = None;
        self.dirty_chunks.clear();
        self.last_stroke_hit_pos = None;
    }

    fn apply_stroke(
        map: &mut Map,
        server_ctx: &ServerContext,
        erase: bool,
        stroke_seed: i32,
        vine_seq: &mut i32,
        dirty_chunks: &mut HashSet<(i32, i32)>,
        last_stroke_hit_pos: &mut Option<Vec3<f32>>,
    ) -> bool {
        let mut brush = Self::evaluate_brush(map);
        brush.stroke_seed = stroke_seed;
        let hit_pos = server_ctx.hover_cursor_3d.unwrap_or(server_ctx.geo_hit_pos);
        if brush.use_line_shape && last_stroke_hit_pos.is_none() {
            *last_stroke_hit_pos = Some(hit_pos);
            return false;
        }
        let step = (brush.radius * brush.circle_radius * 0.45)
            .max(brush.cell_size * 0.75)
            .max(0.05);
        let start = last_stroke_hit_pos.unwrap_or(hit_pos);
        let delta = hit_pos - start;
        let dist = delta.magnitude();
        if brush.use_line_shape && dist > 0.0001 {
            Self::mark_dirty_chunks(dirty_chunks, map, start, brush.radius.max(brush.depth));
            Self::mark_dirty_chunks(dirty_chunks, map, hit_pos, brush.radius.max(brush.depth));
            let changed =
                Self::apply_line_segment(map, server_ctx, start, hit_pos, &brush, vine_seq, erase);
            *last_stroke_hit_pos = Some(hit_pos);
            return changed;
        }
        let steps = (dist / step).ceil().max(1.0) as usize;
        let mut changed = false;

        for i in 0..=steps {
            let t = if steps == 0 {
                1.0
            } else {
                i as f32 / steps as f32
            };
            let sample = start + delta * t;
            Self::mark_dirty_chunks(dirty_chunks, map, sample, brush.radius.max(brush.depth));
            let stroke_dir_world = (dist > 0.0001).then_some(delta / dist);
            changed |=
                Self::apply_stroke_at(map, server_ctx, sample, &brush, stroke_dir_world, erase);
        }

        *last_stroke_hit_pos = Some(hit_pos);
        changed
    }

    fn apply_line_segment(
        map: &mut Map,
        server_ctx: &ServerContext,
        start_pos: Vec3<f32>,
        end_pos: Vec3<f32>,
        brush: &OrganicBrushEval,
        vine_seq: &mut i32,
        erase: bool,
    ) -> bool {
        if let Some(surface) = server_ctx
            .active_detail_surface
            .as_ref()
            .or(server_ctx.hover_surface.as_ref())
        {
            if let Some(surface_ref) = map.surfaces.get(&surface.id)
                && Self::should_redirect_surface_to_terrain(map, surface_ref)
            {
                return Self::paint_terrain_line(
                    map,
                    start_pos,
                    end_pos,
                    brush.clone(),
                    vine_seq,
                    erase,
                );
            }
            return Self::paint_surface_line(
                map,
                server_ctx,
                surface.id,
                surface.sector_id,
                start_pos,
                end_pos,
                brush.clone(),
                vine_seq,
                erase,
            );
        }

        match server_ctx.geo_hit {
            Some(GeoId::Terrain(_, _)) => {
                Self::paint_terrain_line(map, start_pos, end_pos, brush.clone(), vine_seq, erase)
            }
            Some(GeoId::Sector(_)) => false,
            _ => Self::paint_terrain_line(map, start_pos, end_pos, brush.clone(), vine_seq, erase),
        }
    }

    fn apply_stroke_at(
        map: &mut Map,
        server_ctx: &ServerContext,
        hit_pos: Vec3<f32>,
        brush: &OrganicBrushEval,
        stroke_dir_world: Option<Vec3<f32>>,
        erase: bool,
    ) -> bool {
        if let Some(surface) = server_ctx
            .active_detail_surface
            .as_ref()
            .or(server_ctx.hover_surface.as_ref())
        {
            if let Some(surface_ref) = map.surfaces.get(&surface.id)
                && Self::should_redirect_surface_to_terrain(map, surface_ref)
            {
                return Self::paint_terrain(map, hit_pos, brush.clone(), stroke_dir_world, erase);
            }
            return Self::paint_surface(
                map,
                server_ctx,
                surface.id,
                surface.sector_id,
                hit_pos,
                brush.clone(),
                stroke_dir_world,
                erase,
            );
        }

        match server_ctx.geo_hit {
            Some(GeoId::Terrain(_, _)) => {
                Self::paint_terrain(map, hit_pos, brush.clone(), stroke_dir_world, erase)
            }
            Some(GeoId::Sector(_)) => false,
            _ => Self::paint_terrain(map, hit_pos, brush.clone(), stroke_dir_world, erase),
        }
    }

    fn evaluate_brush(map: &Map) -> OrganicBrushEval {
        let mut out = OrganicBrushEval {
            valid: true,
            radius: Self::output_radius(map),
            flow: Self::output_flow(map),
            jitter: Self::output_jitter(map),
            depth: Self::output_depth(map),
            cell_size: Self::output_cell_size(map),
            circle_radius: 1.0,
            circle_softness: 0.35,
            canopy_lobes: 6,
            canopy_spread: 0.5,
            use_canopy_shape: false,
            bush_height: 1.0,
            bush_layers: 4,
            bush_taper: 0.5,
            bush_breakup: 0.2,
            use_bush_shape: false,
            line_length: 1.2,
            line_width: 0.18,
            line_softness: 0.35,
            use_line_shape: false,
            scatter_count: 1,
            scatter_jitter: 0.0,
            height_depth: 1.0,
            height_falloff: 0.5,
            noise_scale: 0.3,
            noise_strength: 0.0,
            noise_seed: 1,
            channel: 0,
            material_source: None,
            palette_start: 0,
            palette_count: 1,
            palette_mode: 0,
            stroke_seed: 0,
        };

        if let Some(id) = Self::active_graph_id(map)
            && let Some(graph) = map.organic_brush_graphs.get(&id)
        {
            if let Some(output_index) = Self::active_output_node_index(graph) {
                if let OrganicNodeKind::OutputVolume {
                    radius,
                    flow,
                    jitter,
                    depth,
                    cell_size,
                } = graph.nodes[output_index].kind
                {
                    out.radius = radius.max(0.05);
                    out.flow = flow.clamp(0.05, 1.0);
                    out.jitter = jitter.clamp(0.0, 1.0);
                    out.depth = depth.max(0.01);
                    out.cell_size = cell_size.max(0.05);
                }

                let shape_nodes = Self::collect_branch_nodes(graph, output_index, 0, |kind| {
                    matches!(
                        kind,
                        OrganicNodeKind::CircleMask { .. }
                            | OrganicNodeKind::BushShape { .. }
                            | OrganicNodeKind::CanopyShape { .. }
                            | OrganicNodeKind::LineShape { .. }
                            | OrganicNodeKind::Noise { .. }
                            | OrganicNodeKind::Scatter { .. }
                    )
                });
                let material_nodes = Self::collect_branch_nodes(graph, output_index, 1, |kind| {
                    matches!(
                        kind,
                        OrganicNodeKind::Material { .. } | OrganicNodeKind::PaletteRange { .. }
                    )
                });
                let growth_nodes = Self::collect_branch_nodes(graph, output_index, 2, |kind| {
                    matches!(kind, OrganicNodeKind::HeightProfile { .. })
                });

                out.valid = !shape_nodes.is_empty();

                for node_index in shape_nodes {
                    match graph.nodes[node_index].kind {
                        OrganicNodeKind::CircleMask { radius, softness } => {
                            out.circle_radius = radius.max(0.05);
                            out.circle_softness = softness.clamp(0.0, 1.0);
                        }
                        OrganicNodeKind::BushShape {
                            radius,
                            height,
                            layers,
                            taper,
                            breakup,
                            softness,
                        } => {
                            out.use_bush_shape = true;
                            out.circle_radius = radius.max(0.05);
                            out.circle_softness = softness.clamp(0.0, 1.0);
                            out.bush_height = height.max(0.1);
                            out.bush_layers = layers.max(2);
                            out.bush_taper = taper.clamp(0.0, 1.0);
                            out.bush_breakup = breakup.clamp(0.0, 1.0);
                        }
                        OrganicNodeKind::CanopyShape {
                            radius,
                            lobes,
                            spread,
                            softness,
                        } => {
                            out.use_canopy_shape = true;
                            out.circle_radius = radius.max(0.05);
                            out.circle_softness = softness.clamp(0.0, 1.0);
                            out.canopy_lobes = lobes.max(3);
                            out.canopy_spread = spread.clamp(0.0, 1.0);
                        }
                        OrganicNodeKind::LineShape {
                            length,
                            width,
                            softness,
                        } => {
                            out.use_line_shape = true;
                            out.line_length = length.max(0.1);
                            out.line_width = width.max(0.02);
                            out.line_softness = softness.clamp(0.0, 1.0);
                        }
                        OrganicNodeKind::Noise {
                            scale,
                            strength,
                            seed,
                        } => {
                            out.noise_scale = scale.max(0.01);
                            out.noise_strength = strength.clamp(0.0, 1.0);
                            out.noise_seed = seed;
                        }
                        OrganicNodeKind::Scatter { count, jitter } => {
                            out.scatter_count = count.max(1);
                            out.scatter_jitter = jitter.clamp(0.0, 1.0);
                        }
                        _ => {}
                    }
                }
                for node_index in growth_nodes {
                    if let OrganicNodeKind::HeightProfile { depth, falloff } =
                        graph.nodes[node_index].kind
                    {
                        out.height_depth = depth.max(0.05);
                        out.height_falloff = falloff.clamp(0.0, 1.0);
                    }
                }
                for node_index in material_nodes {
                    match &graph.nodes[node_index].kind {
                        OrganicNodeKind::Material { channel } => {
                            out.channel = (*channel).clamp(0, 3);
                        }
                        OrganicNodeKind::PaletteRange { start, count, mode } => {
                            out.palette_start = (*start).clamp(0, 255);
                            out.palette_count = (*count).clamp(1, 16);
                            out.palette_mode = (*mode).clamp(0, 2);
                            out.material_source =
                                Some(PixelSource::PaletteIndex(out.palette_start as u16));
                        }
                        _ => {}
                    }
                }
            }
        }
        out
    }

    fn active_output_node_index(graph: &rusterix::OrganicBrushGraph) -> Option<usize> {
        if let Some(index) = graph.selected_node
            && matches!(
                graph.nodes.get(index).map(|node| &node.kind),
                Some(OrganicNodeKind::OutputVolume { .. })
            )
        {
            return Some(index);
        }

        graph
            .nodes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, node)| match node.kind {
                OrganicNodeKind::OutputVolume { .. } => Some(index),
                _ => None,
            })
    }

    fn collect_branch_nodes<F>(
        graph: &rusterix::OrganicBrushGraph,
        dst_node: usize,
        dst_terminal: u8,
        allow: F,
    ) -> HashSet<usize>
    where
        F: Fn(&OrganicNodeKind) -> bool + Copy,
    {
        let mut visited = HashSet::default();
        Self::collect_upstream_nodes_filtered(graph, dst_node, dst_terminal, allow, &mut visited);
        visited
    }

    fn collect_upstream_nodes_filtered<F>(
        graph: &rusterix::OrganicBrushGraph,
        dst_node: usize,
        dst_terminal: u8,
        allow: F,
        visited: &mut HashSet<usize>,
    ) where
        F: Fn(&OrganicNodeKind) -> bool + Copy,
    {
        for (from_node, _, to_node, to_terminal) in &graph.connections {
            if *to_node as usize != dst_node || *to_terminal != dst_terminal {
                continue;
            }
            let from_index = *from_node as usize;
            let Some(node) = graph.nodes.get(from_index) else {
                continue;
            };
            if !allow(&node.kind) || !visited.insert(from_index) {
                continue;
            }
            for input_index in 0..Self::organic_node_input_count(&node.kind) {
                Self::collect_upstream_nodes_filtered(
                    graph,
                    from_index,
                    input_index as u8,
                    allow,
                    visited,
                );
            }
        }
    }

    fn organic_node_input_count(kind: &OrganicNodeKind) -> usize {
        match kind {
            OrganicNodeKind::SurfaceInput => 0,
            OrganicNodeKind::CircleMask { .. } => 0,
            OrganicNodeKind::BushShape { .. } => 0,
            OrganicNodeKind::CanopyShape { .. } => 0,
            OrganicNodeKind::LineShape { .. } => 0,
            OrganicNodeKind::Noise { .. } => 1,
            OrganicNodeKind::Scatter { .. } => 2,
            OrganicNodeKind::HeightProfile { .. } => 1,
            OrganicNodeKind::PaletteRange { .. } => 0,
            OrganicNodeKind::Material { .. } => 1,
            OrganicNodeKind::OutputVolume { .. } => 3,
        }
    }

    fn hover_debug_text(map: &Map, server_ctx: &ServerContext) -> String {
        let hit_pos = server_ctx.hover_cursor_3d.unwrap_or(server_ctx.geo_hit_pos);
        if let Some(surface) = server_ctx
            .active_detail_surface
            .as_ref()
            .or(server_ctx.hover_surface.as_ref())
            && let Some(surface_ref) = map.surfaces.get(&surface.id)
        {
            let surface_normal = surface_ref.normal();
            if Self::should_redirect_surface_to_terrain(map, surface_ref) {
                let terrain_height = TerrainGenerator::sample_height_at(
                    map,
                    Vec2::new(hit_pos.x, hit_pos.z),
                    &TerrainConfig::default(),
                );
                return format!(
                    "Organic debug: redirect down-facing surface sector={} to terrain h={:.3} hit=({:.2},{:.2},{:.2})",
                    surface_ref.sector_id, terrain_height, hit_pos.x, hit_pos.y, hit_pos.z
                );
            }
            let signed_dist = (hit_pos - surface_ref.plane.origin).dot(surface_normal);
            let grow_positive = Self::resolve_surface_growth_side(
                signed_dist,
                surface_normal,
                server_ctx.hover_ray_dir_3d,
            );
            let side = if grow_positive { "+" } else { "-" };
            return format!(
                "Organic debug: surface sector={} n=({:.2},{:.2},{:.2}) d={:.3} side={} anchor={:.3} depth={:.3}",
                surface_ref.sector_id,
                surface_normal.x,
                surface_normal.y,
                surface_normal.z,
                signed_dist,
                side,
                signed_dist,
                Self::output_depth(map)
            );
        }

        match server_ctx.geo_hit {
            Some(GeoId::Terrain(_, _)) | None => {
                let height = TerrainGenerator::sample_height_at(
                    map,
                    Vec2::new(hit_pos.x, hit_pos.z),
                    &TerrainConfig::default(),
                );
                format!(
                    "Organic debug: terrain h={:.3} hit=({:.2},{:.2},{:.2})",
                    height, hit_pos.x, hit_pos.y, hit_pos.z
                )
            }
            Some(geo) => format!("Organic debug: hit {:?}", geo),
        }
    }

    fn should_redirect_surface_to_terrain(map: &Map, surface: &Surface) -> bool {
        if surface.normal().y >= -0.7 {
            return false;
        }
        let Some(sector) = map.find_sector(surface.sector_id) else {
            return false;
        };
        let terrain_mode = sector.properties.get_int_default("terrain_mode", 0);
        let water_enabled = sector
            .properties
            .get_bool_default("ridge_water_enabled", false);
        terrain_mode == 2 || water_enabled
    }

    fn resolve_surface_growth_side(
        signed_dist: f32,
        surface_normal: Vec3<f32>,
        hover_ray_dir: Option<Vec3<f32>>,
    ) -> bool {
        if signed_dist.abs() > 0.01 {
            signed_dist >= 0.0
        } else if let Some(ray_dir) = hover_ray_dir {
            surface_normal.dot(-ray_dir) >= 0.0
        } else {
            true
        }
    }

    fn paint_surface(
        map: &mut Map,
        server_ctx: &ServerContext,
        surface_id: Uuid,
        sector_id: u32,
        hit_pos: Vec3<f32>,
        brush: OrganicBrushEval,
        stroke_dir_world: Option<Vec3<f32>>,
        erase: bool,
    ) -> bool {
        let source = map
            .find_sector(sector_id)
            .and_then(|sector| sector.properties.get_default_source().cloned());
        let Some(local) = map
            .surfaces
            .get(&surface_id)
            .map(|surface| surface.uv_to_tile_local(surface.world_to_uv(hit_pos), map))
        else {
            return false;
        };
        let local_dir = stroke_dir_world.and_then(|world_dir| {
            map.surfaces.get(&surface_id).and_then(|surface| {
                let p0 = surface.uv_to_tile_local(surface.world_to_uv(hit_pos), map);
                let p1 = surface.uv_to_tile_local(surface.world_to_uv(hit_pos + world_dir), map);
                let delta = p1 - p0;
                let mag = delta.magnitude();
                (mag > 0.0001).then_some(delta / mag)
            })
        });
        let Some(surface) = map.surfaces.get_mut(&surface_id) else {
            return false;
        };
        let surface_normal = surface.normal();
        let signed_dist = (hit_pos - surface.plane.origin).dot(surface_normal);
        let grow_positive = Self::resolve_surface_growth_side(
            signed_dist,
            surface_normal,
            server_ctx.hover_ray_dir_3d,
        );
        let anchor_offset = signed_dist;
        if brush.use_bush_shape {
            let changed = Self::paint_surface_bush(
                surface,
                local,
                anchor_offset,
                &brush,
                source,
                grow_positive,
                erase,
            );
            if changed {
                map.changed += 1;
            }
            return changed;
        }
        let layer = surface.organic_layer_for_cell_size_mut(brush.cell_size);
        let changed = Self::apply_graph_dabs(
            layer,
            local,
            local_dir,
            anchor_offset,
            &brush,
            source,
            grow_positive,
            erase,
        );
        if changed {
            map.changed += 1;
        }
        changed
    }

    fn paint_surface_line(
        map: &mut Map,
        server_ctx: &ServerContext,
        surface_id: Uuid,
        sector_id: u32,
        start_pos: Vec3<f32>,
        end_pos: Vec3<f32>,
        brush: OrganicBrushEval,
        vine_seq: &mut i32,
        erase: bool,
    ) -> bool {
        let source = map
            .find_sector(sector_id)
            .and_then(|sector| sector.properties.get_default_source().cloned());
        let Some((start_local, end_local)) = map.surfaces.get(&surface_id).map(|surface| {
            (
                surface.uv_to_tile_local(surface.world_to_uv(start_pos), map),
                surface.uv_to_tile_local(surface.world_to_uv(end_pos), map),
            )
        }) else {
            return false;
        };
        let Some(surface) = map.surfaces.get_mut(&surface_id) else {
            return false;
        };
        let surface_normal = surface.normal();
        let signed_dist = (end_pos - surface.plane.origin).dot(surface_normal);
        let grow_positive = Self::resolve_surface_growth_side(
            signed_dist,
            surface_normal,
            server_ctx.hover_ray_dir_3d,
        );
        let anchor_offset = signed_dist;
        if brush.use_line_shape {
            let changed = Self::paint_surface_vine(
                surface,
                start_local,
                end_local,
                anchor_offset,
                &brush,
                source,
                vine_seq,
                grow_positive,
                erase,
            );
            if changed {
                map.changed += 1;
            }
            return changed;
        }
        let layer = surface.organic_layer_for_cell_size_mut(brush.cell_size);
        let changed = Self::apply_graph_line(
            layer,
            start_local,
            end_local,
            anchor_offset,
            &brush,
            source,
            grow_positive,
            erase,
        );
        if changed {
            map.changed += 1;
        }
        changed
    }

    fn paint_terrain(
        map: &mut Map,
        hit_pos: Vec3<f32>,
        brush: OrganicBrushEval,
        stroke_dir_world: Option<Vec3<f32>>,
        erase: bool,
    ) -> bool {
        let world = Vec2::new(hit_pos.x, hit_pos.z);
        let cell = Vec2::new(world.x.floor() as i32, world.y.floor() as i32);
        let source = map.terrain.get_source(cell.x, cell.y).cloned();
        let chunk = map.terrain.get_or_create_chunk_mut(cell.x, cell.y);
        let origin = chunk.origin.map(|v| v as f32);
        if brush.use_bush_shape {
            let changed = Self::paint_terrain_bush(chunk, world - origin, &brush, source, erase);
            if changed {
                chunk.mark_dirty();
                map.changed += 1;
            }
            return changed;
        }
        let layer = chunk.organic_layer_for_cell_size_mut(brush.cell_size);
        let local_dir = stroke_dir_world.and_then(|dir| {
            let delta = Vec2::new(dir.x, dir.z);
            let mag = delta.magnitude();
            (mag > 0.0001).then_some(delta / mag)
        });
        let changed = Self::apply_graph_dabs(
            layer,
            world - origin,
            local_dir,
            0.0,
            &brush,
            source,
            true,
            erase,
        );
        if changed {
            chunk.mark_dirty();
            map.changed += 1;
        }
        changed
    }

    fn paint_terrain_line(
        map: &mut Map,
        start_pos: Vec3<f32>,
        end_pos: Vec3<f32>,
        brush: OrganicBrushEval,
        vine_seq: &mut i32,
        erase: bool,
    ) -> bool {
        let start_world = Vec2::new(start_pos.x, start_pos.z);
        let end_world = Vec2::new(end_pos.x, end_pos.z);
        if brush.use_line_shape {
            let changed = Self::paint_terrain_vine_world(
                map,
                start_world,
                end_world,
                &brush,
                vine_seq,
                erase,
            );
            if changed {
                map.changed += 1;
            }
            return changed;
        }
        let cell = Vec2::new(end_world.x.floor() as i32, end_world.y.floor() as i32);
        let source = map.terrain.get_source(cell.x, cell.y).cloned();
        let chunk = map.terrain.get_or_create_chunk_mut(cell.x, cell.y);
        let origin = chunk.origin.map(|v| v as f32);
        let layer = chunk.organic_layer_for_cell_size_mut(brush.cell_size);
        let changed = Self::apply_graph_line(
            layer,
            start_world - origin,
            end_world - origin,
            0.0,
            &brush,
            source,
            true,
            erase,
        );
        if changed {
            chunk.mark_dirty();
            map.changed += 1;
        }
        changed
    }

    fn paint_terrain_vine_world(
        map: &mut Map,
        start_world: Vec2<f32>,
        end_world: Vec2<f32>,
        brush: &OrganicBrushEval,
        vine_seq: &mut i32,
        erase: bool,
    ) -> bool {
        let delta = end_world - start_world;
        let dist = delta.magnitude();
        if dist <= 0.0001 {
            return false;
        }

        let width = (brush.line_width * brush.radius).max(0.02);
        let segment_len = (width * 2.5).max(0.10);
        let segments = (dist / segment_len).ceil().max(1.0) as usize;
        let mut changed = false;

        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;
            let seg_start_world = start_world + delta * t0;
            let seg_end_world = start_world + delta * t1;
            let mid_world = (seg_start_world + seg_end_world) * 0.5;
            let cell = Vec2::new(mid_world.x.floor() as i32, mid_world.y.floor() as i32);
            let source = map.terrain.get_source(cell.x, cell.y).cloned();
            let chunk = map.terrain.get_or_create_chunk_mut(cell.x, cell.y);
            let origin = chunk.origin.map(|v| v as f32);
            let local_start = seg_start_world - origin;
            let local_end = seg_end_world - origin;
            let seg_changed = Self::paint_terrain_vine(
                chunk,
                local_start,
                local_end,
                brush,
                source,
                vine_seq,
                erase,
            );
            if seg_changed {
                chunk.mark_dirty();
                changed = true;
            }
        }

        changed
    }

    fn paint_surface_vine(
        surface: &mut Surface,
        start: Vec2<f32>,
        end: Vec2<f32>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        source: Option<PixelSource>,
        vine_seq: &mut i32,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        Self::paint_vine_segments(
            &mut surface.organic_vine_strokes,
            start,
            end,
            anchor_offset,
            brush,
            source,
            vine_seq,
            grow_positive,
            erase,
        )
    }

    fn paint_surface_bush(
        surface: &mut Surface,
        center: Vec2<f32>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        source: Option<PixelSource>,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        Self::paint_bush_clusters(
            &mut surface.organic_bush_clusters,
            center,
            anchor_offset,
            brush,
            source,
            grow_positive,
            erase,
        )
    }

    fn paint_terrain_bush(
        chunk: &mut rusterix::TerrainChunk,
        center: Vec2<f32>,
        brush: &OrganicBrushEval,
        source: Option<PixelSource>,
        erase: bool,
    ) -> bool {
        Self::paint_bush_clusters(
            &mut chunk.organic_bush_clusters,
            center,
            0.0,
            brush,
            source,
            true,
            erase,
        )
    }

    fn paint_bush_clusters(
        clusters: &mut Vec<rusterix::OrganicBushCluster>,
        center: Vec2<f32>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        source: Option<PixelSource>,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        let radius = (brush.radius * brush.circle_radius).max(0.08);
        let height = (brush.depth * brush.bush_height).max(0.12);
        if erase {
            let before = clusters.len();
            let threshold = radius * 1.25;
            clusters.retain(|cluster| (cluster.center - center).magnitude() > threshold);
            return before != clusters.len();
        }

        let resolved_source =
            Self::resolve_brush_source(brush, source, Some(clusters.len() as i32), center);
        clusters.push(rusterix::OrganicBushCluster {
            center,
            anchor_offset,
            radius,
            height,
            layers: brush.bush_layers.max(2),
            taper: brush.bush_taper,
            breakup: brush.bush_breakup,
            channel: brush.channel,
            source: resolved_source,
            grow_positive,
        });
        true
    }

    fn paint_terrain_vine(
        chunk: &mut rusterix::TerrainChunk,
        start: Vec2<f32>,
        end: Vec2<f32>,
        brush: &OrganicBrushEval,
        source: Option<PixelSource>,
        vine_seq: &mut i32,
        erase: bool,
    ) -> bool {
        Self::paint_vine_segments(
            &mut chunk.organic_vine_strokes,
            start,
            end,
            0.0,
            brush,
            source,
            vine_seq,
            true,
            erase,
        )
    }

    fn paint_vine_segments(
        strokes: &mut Vec<rusterix::OrganicVineStroke>,
        start: Vec2<f32>,
        end: Vec2<f32>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        source: Option<PixelSource>,
        vine_seq: &mut i32,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        if (end - start).magnitude() <= 0.0001 {
            return false;
        }
        let width = (brush.line_width * brush.radius).max(0.02);
        let depth = (brush.depth * brush.height_depth).max(0.02);
        if erase {
            let before = strokes.len();
            let threshold = width * 1.5;
            strokes.retain(|stroke| {
                let a = Self::point_segment_distance(stroke.start, start, end);
                let b = Self::point_segment_distance(stroke.end, start, end);
                let c = Self::point_segment_distance(start, stroke.start, stroke.end);
                let d = Self::point_segment_distance(end, stroke.start, stroke.end);
                a.max(b).min(c.max(d)) > threshold
            });
            return before != strokes.len();
        }
        let delta = end - start;
        let dist = delta.magnitude();
        let dir = delta / dist;
        let segment_len = (width * 2.5).max(0.10);
        let segments = (dist / segment_len).ceil().max(1.0) as usize;
        let mut changed = false;
        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;
            let seg_start = start + dir * (dist * t0);
            let seg_end = start + dir * (dist * t1);
            let resolved_source = Self::resolve_brush_source(
                brush,
                source.clone(),
                Some(strokes.len() as i32 + i as i32),
                (seg_start + seg_end) * 0.5,
            );
            strokes.push(rusterix::OrganicVineStroke {
                stroke_id: brush.stroke_seed,
                seq: *vine_seq,
                start: seg_start,
                end: seg_end,
                anchor_offset,
                width,
                depth,
                channel: brush.channel,
                source: resolved_source,
                grow_positive,
                cap_start: false,
                cap_end: false,
            });
            *vine_seq += 1;
            changed = true;
        }
        changed
    }

    fn finalize_vine_stroke_caps(map: &mut Map, stroke_seed: i32) {
        enum VineLoc {
            Surface(Uuid, usize),
            Terrain((i32, i32), usize),
        }

        let mut first: Option<(VineLoc, i32)> = None;
        let mut last: Option<(VineLoc, i32)> = None;

        for (surface_id, surface) in &map.surfaces {
            for (idx, stroke) in surface.organic_vine_strokes.iter().enumerate() {
                if stroke.stroke_id != stroke_seed {
                    continue;
                }
                if first.as_ref().is_none_or(|(_, seq)| stroke.seq < *seq) {
                    first = Some((VineLoc::Surface(*surface_id, idx), stroke.seq));
                }
                if last.as_ref().is_none_or(|(_, seq)| stroke.seq > *seq) {
                    last = Some((VineLoc::Surface(*surface_id, idx), stroke.seq));
                }
            }
        }
        for (origin, terrain_chunk) in &map.terrain.chunks {
            for (idx, stroke) in terrain_chunk.organic_vine_strokes.iter().enumerate() {
                if stroke.stroke_id != stroke_seed {
                    continue;
                }
                if first.as_ref().is_none_or(|(_, seq)| stroke.seq < *seq) {
                    first = Some((VineLoc::Terrain(*origin, idx), stroke.seq));
                }
                if last.as_ref().is_none_or(|(_, seq)| stroke.seq > *seq) {
                    last = Some((VineLoc::Terrain(*origin, idx), stroke.seq));
                }
            }
        }

        for surface in map.surfaces.values_mut() {
            for stroke in &mut surface.organic_vine_strokes {
                if stroke.stroke_id == stroke_seed {
                    stroke.cap_start = false;
                    stroke.cap_end = false;
                }
            }
        }
        for terrain_chunk in map.terrain.chunks.values_mut() {
            for stroke in &mut terrain_chunk.organic_vine_strokes {
                if stroke.stroke_id == stroke_seed {
                    stroke.cap_start = false;
                    stroke.cap_end = false;
                }
            }
        }

        if let Some((loc, _)) = first {
            match loc {
                VineLoc::Surface(id, idx) => {
                    if let Some(surface) = map.surfaces.get_mut(&id)
                        && let Some(stroke) = surface.organic_vine_strokes.get_mut(idx)
                    {
                        stroke.cap_start = true;
                    }
                }
                VineLoc::Terrain(origin, idx) => {
                    if let Some(chunk) = map.terrain.chunks.get_mut(&origin)
                        && let Some(stroke) = chunk.organic_vine_strokes.get_mut(idx)
                    {
                        stroke.cap_start = true;
                    }
                }
            }
        }
        if let Some((loc, _)) = last {
            match loc {
                VineLoc::Surface(id, idx) => {
                    if let Some(surface) = map.surfaces.get_mut(&id)
                        && let Some(stroke) = surface.organic_vine_strokes.get_mut(idx)
                    {
                        stroke.cap_end = true;
                    }
                }
                VineLoc::Terrain(origin, idx) => {
                    if let Some(chunk) = map.terrain.chunks.get_mut(&origin)
                        && let Some(stroke) = chunk.organic_vine_strokes.get_mut(idx)
                    {
                        stroke.cap_end = true;
                    }
                }
            }
        }
    }

    fn point_segment_distance(p: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> f32 {
        let ab = b - a;
        let len_sq = ab.magnitude_squared();
        if len_sq <= f32::EPSILON {
            return (p - a).magnitude();
        }
        let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
        (p - (a + ab * t)).magnitude()
    }

    fn output_depth(map: &Map) -> f32 {
        if let Some(id) = Self::active_graph_id(map)
            && let Some(graph) = map.organic_brush_graphs.get(&id)
        {
            for node in graph.nodes.iter().rev() {
                if let OrganicNodeKind::OutputVolume { depth, .. } = node.kind {
                    return depth.max(0.01);
                }
            }
        }
        map.properties
            .get_float_default("organic_brush_depth", 0.45)
    }

    fn output_radius(map: &Map) -> f32 {
        if let Some(id) = Self::active_graph_id(map)
            && let Some(graph) = map.organic_brush_graphs.get(&id)
        {
            for node in graph.nodes.iter().rev() {
                if let OrganicNodeKind::OutputVolume { radius, .. } = node.kind {
                    return radius.max(0.05);
                }
            }
        }
        map.properties
            .get_float_default("organic_brush_radius", 0.6)
    }

    fn output_flow(map: &Map) -> f32 {
        if let Some(id) = Self::active_graph_id(map)
            && let Some(graph) = map.organic_brush_graphs.get(&id)
        {
            for node in graph.nodes.iter().rev() {
                if let OrganicNodeKind::OutputVolume { flow, .. } = node.kind {
                    return flow.clamp(0.05, 1.0);
                }
            }
        }
        map.properties.get_float_default("organic_brush_flow", 1.0)
    }

    fn output_jitter(map: &Map) -> f32 {
        if let Some(id) = Self::active_graph_id(map)
            && let Some(graph) = map.organic_brush_graphs.get(&id)
        {
            for node in graph.nodes.iter().rev() {
                if let OrganicNodeKind::OutputVolume { jitter, .. } = node.kind {
                    return jitter.clamp(0.0, 1.0);
                }
            }
        }
        map.properties
            .get_float_default("organic_brush_jitter", 0.15)
    }

    fn resolve_brush_source(
        brush: &OrganicBrushEval,
        host_source: Option<PixelSource>,
        variant: Option<i32>,
        _pos: Vec2<f32>,
    ) -> Option<PixelSource> {
        if brush.palette_count <= 1 {
            return brush.material_source.clone().or(host_source);
        }
        let count = brush.palette_count.max(1);
        let pick = match brush.palette_mode {
            1 => {
                let seed = brush.stroke_seed ^ brush.noise_seed;
                seed.rem_euclid(count)
            }
            2 => {
                let seed = variant.unwrap_or(0) ^ brush.noise_seed;
                seed.rem_euclid(count)
            }
            _ => 0,
        };
        Some(PixelSource::PaletteIndex(
            (brush.palette_start + pick).clamp(0, 255) as u16,
        ))
    }

    fn output_cell_size(map: &Map) -> f32 {
        let active_id = match map.properties.get("organic_brush_active_graph") {
            Some(Value::Id(id)) => Some(*id),
            _ => None,
        };
        if let Some(id) = active_id
            && let Some(graph) = map.organic_brush_graphs.get(&id)
        {
            for node in graph.nodes.iter().rev() {
                if let OrganicNodeKind::OutputVolume { cell_size, .. } = node.kind {
                    return cell_size.max(0.05);
                }
            }
        }
        0.25
    }

    fn active_graph_id(map: &Map) -> Option<Uuid> {
        match map.properties.get("organic_brush_active_graph") {
            Some(Value::Id(id)) => Some(*id),
            _ => None,
        }
    }

    fn apply_graph_dabs(
        layer: &mut rusterix::OrganicVolumeLayer,
        center: Vec2<f32>,
        stroke_dir_local: Option<Vec2<f32>>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        host_source: Option<PixelSource>,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        if !brush.valid {
            return false;
        }
        let scatter_count = brush.scatter_count.max(1) as usize;
        let base_radius = (brush.radius * brush.circle_radius).max(layer.cell_size * 0.5);
        let base_depth = (brush.depth * brush.height_depth).max(layer.cell_size * 0.20);
        let shallow_spread = base_depth <= layer.cell_size * 0.95 && scatter_count > 1;
        let dab_radius = if scatter_count > 1 {
            let scale = if shallow_spread { 0.92 } else { 0.72 };
            (base_radius * scale).max(layer.cell_size * 0.5)
        } else {
            base_radius
        };
        let mut changed = false;
        let line_dir = stroke_dir_local.unwrap_or(Vec2::new(1.0, 0.0)).normalized();

        if brush.use_bush_shape && !brush.use_line_shape {
            let bush_radius = base_radius;
            let bush_height = (brush.depth * brush.bush_height).max(layer.cell_size * 0.6);
            let layer_count = brush.bush_layers.max(2) as usize;
            for index in 0..layer_count {
                let t = if layer_count <= 1 {
                    0.0
                } else {
                    index as f32 / (layer_count - 1) as f32
                };
                let noise_seed = center + Vec2::new(index as f32 * 0.37, index as f32 * 0.23);
                let noise = Self::organic_noise(noise_seed, brush);
                let source = Self::resolve_brush_source(
                    brush,
                    host_source.clone(),
                    Some(index as i32),
                    center,
                );
                let radius_scale = (1.0 - t * brush.bush_taper * 0.65).max(0.28);
                let lateral = bush_radius * brush.bush_breakup * 0.32;
                let lateral_offset = Vec2::new(
                    ((index as f32 * 1.618) + brush.jitter).cos(),
                    ((index as f32 * 1.618) + brush.jitter).sin(),
                ) * lateral
                    * (0.6 + noise * 0.4);
                let layer_center = center + lateral_offset;
                let layer_radius =
                    (bush_radius * radius_scale * (1.0 + noise * 0.10)).max(layer.cell_size * 0.65);
                let layer_depth = (base_depth * (0.92 - t * 0.18)).max(layer.cell_size * 0.24);
                let layer_anchor = anchor_offset + bush_height * t * 0.72;
                let layer_flow = (brush.flow / layer_count as f32).clamp(0.08, 0.4);
                changed |= if erase {
                    layer.erase_sphere(
                        layer_center,
                        layer_radius,
                        layer_anchor,
                        layer_depth,
                        brush.circle_softness,
                        brush.height_falloff,
                        grow_positive,
                    )
                } else {
                    layer.paint_sphere(
                        layer_center,
                        layer_radius,
                        layer_anchor,
                        layer_depth,
                        brush.circle_softness,
                        brush.height_falloff,
                        layer_flow,
                        brush.channel,
                        source,
                        grow_positive,
                    )
                };
            }
            return changed;
        }

        if brush.use_bush_shape && !brush.use_line_shape {
            let source = Self::resolve_brush_source(brush, host_source.clone(), Some(0), center);
            let bush_radius = base_radius;
            let bush_height = (brush.depth * brush.bush_height).max(layer.cell_size * 0.8);
            changed |= if erase {
                layer.erase_sphere(
                    center,
                    bush_radius,
                    anchor_offset,
                    bush_height,
                    brush.circle_softness,
                    brush.height_falloff,
                    grow_positive,
                )
            } else {
                layer.paint_bush_cluster(
                    center,
                    bush_radius,
                    bush_height,
                    anchor_offset,
                    brush.bush_layers,
                    brush.bush_taper,
                    brush.bush_breakup,
                    brush.circle_softness,
                    brush.flow.clamp(0.1, 1.0),
                    brush.channel,
                    source,
                    grow_positive,
                )
            };
            return changed;
        }

        if brush.use_canopy_shape && !brush.use_line_shape {
            let canopy_radius = base_radius;
            let center_source =
                Self::resolve_brush_source(brush, host_source.clone(), Some(-1), center);
            let center_depth = (base_depth * 0.95).max(layer.cell_size * 0.25);
            changed |= if erase {
                layer.erase_sphere(
                    center,
                    canopy_radius * 0.95,
                    anchor_offset,
                    center_depth,
                    brush.circle_softness,
                    brush.height_falloff,
                    grow_positive,
                )
            } else {
                layer.paint_sphere(
                    center,
                    canopy_radius * 0.95,
                    anchor_offset,
                    center_depth,
                    brush.circle_softness,
                    brush.height_falloff,
                    brush.flow.clamp(0.12, 1.0),
                    brush.channel,
                    center_source,
                    grow_positive,
                )
            };

            let lobe_count = brush.canopy_lobes.max(3) as usize;
            let lobe_ring = canopy_radius * (0.28 + brush.canopy_spread * 0.34);
            let lobe_radius =
                (canopy_radius * (0.72 - brush.canopy_spread * 0.16)).max(layer.cell_size * 0.65);
            for index in 0..lobe_count {
                let angle = index as f32 / lobe_count as f32 * std::f32::consts::TAU
                    + brush.jitter * std::f32::consts::PI;
                let offset = Vec2::new(angle.cos(), angle.sin()) * lobe_ring
                    + Self::scatter_offset(index, lobe_count, brush, canopy_radius * 0.35);
                let lobe_center = center + offset;
                let noise = Self::organic_noise(lobe_center, brush);
                let source = Self::resolve_brush_source(
                    brush,
                    host_source.clone(),
                    Some(index as i32),
                    lobe_center,
                );
                let lobe_depth = (base_depth * (0.82 + noise * 0.18)).max(layer.cell_size * 0.22);
                let lobe_size = (lobe_radius * (1.0 + noise * 0.12)).max(layer.cell_size * 0.55);
                changed |= if erase {
                    layer.erase_sphere(
                        lobe_center,
                        lobe_size,
                        anchor_offset,
                        lobe_depth,
                        brush.circle_softness,
                        brush.height_falloff,
                        grow_positive,
                    )
                } else {
                    layer.paint_sphere(
                        lobe_center,
                        lobe_size,
                        anchor_offset,
                        lobe_depth,
                        brush.circle_softness,
                        brush.height_falloff,
                        (brush.flow / lobe_count as f32).clamp(0.06, 0.4),
                        brush.channel,
                        source,
                        grow_positive,
                    )
                };
            }
            return changed;
        }

        for index in 0..scatter_count {
            let offset = if brush.use_line_shape {
                Vec2::zero()
            } else {
                Self::scatter_offset(index, scatter_count, brush, base_radius)
            };
            let dab_center = center + offset;
            let noise = Self::organic_noise(dab_center, brush);
            let source = Self::resolve_brush_source(
                brush,
                host_source.clone(),
                Some(index as i32),
                dab_center,
            );
            let dab_flow = if scatter_count > 1 {
                if shallow_spread {
                    (brush.flow / (scatter_count as f32).sqrt()).clamp(0.03, 1.0)
                } else {
                    (brush.flow / scatter_count as f32).clamp(0.02, 1.0)
                }
            } else {
                brush.flow
            };
            let dab_depth = if scatter_count > 1 {
                let scale = if shallow_spread { 0.95 } else { 0.85 };
                (base_depth * scale).max(layer.cell_size * 0.20)
            } else {
                base_depth
            } * (1.0 + noise * 0.35);
            let dab_radius = (dab_radius * (1.0 + noise * 0.25)).max(layer.cell_size * 0.5);

            if brush.use_line_shape {
                let half_len = (brush.line_length * brush.radius * 0.5).max(layer.cell_size * 0.5);
                let width = (brush.line_width * brush.radius).max(layer.cell_size * 0.5);
                let steps = ((half_len * 2.0) / (width * 0.75).max(layer.cell_size * 0.5))
                    .ceil()
                    .max(1.0) as usize;
                for step_index in 0..=steps {
                    let t = if steps == 0 {
                        0.0
                    } else {
                        step_index as f32 / steps as f32
                    };
                    let along = -half_len + t * (half_len * 2.0);
                    let line_center = dab_center + line_dir * along;
                    let line_changed = if erase {
                        layer.erase_sphere(
                            line_center,
                            width,
                            anchor_offset,
                            dab_depth,
                            brush.line_softness,
                            brush.height_falloff,
                            grow_positive,
                        )
                    } else {
                        layer.paint_sphere(
                            line_center,
                            width,
                            anchor_offset,
                            dab_depth,
                            brush.line_softness,
                            brush.height_falloff,
                            dab_flow / (steps as f32 + 1.0),
                            brush.channel,
                            source.clone(),
                            grow_positive,
                        )
                    };
                    changed |= line_changed;
                }
            } else {
                let dab_changed = if erase {
                    layer.erase_sphere(
                        dab_center,
                        dab_radius,
                        anchor_offset,
                        dab_depth,
                        brush.circle_softness,
                        brush.height_falloff,
                        grow_positive,
                    )
                } else {
                    layer.paint_sphere(
                        dab_center,
                        dab_radius,
                        anchor_offset,
                        dab_depth,
                        brush.circle_softness,
                        brush.height_falloff,
                        dab_flow,
                        brush.channel,
                        source,
                        grow_positive,
                    )
                };
                changed |= dab_changed;
            }
        }

        changed
    }

    fn apply_graph_line(
        layer: &mut rusterix::OrganicVolumeLayer,
        start: Vec2<f32>,
        end: Vec2<f32>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        host_source: Option<PixelSource>,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        if !brush.valid {
            return false;
        }
        let delta = end - start;
        let dist = delta.magnitude();
        if dist <= 0.0001 {
            return Self::apply_graph_dabs(
                layer,
                end,
                None,
                anchor_offset,
                brush,
                host_source,
                grow_positive,
                erase,
            );
        }

        let dir = delta / dist;
        let width = (brush.line_width * brush.radius).max(layer.cell_size * 0.5);
        let cap = (brush.line_length * brush.radius * 0.1).max(width * 0.5);
        let start_inset = (width * 0.5).min(dist * 0.5);
        let line_start = start + dir * start_inset;
        let line_end = end + dir * cap;
        let total = (line_end - line_start).magnitude();
        if total <= 0.0001 {
            return Self::apply_graph_dabs(
                layer,
                end,
                Some(dir),
                anchor_offset,
                brush,
                host_source,
                grow_positive,
                erase,
            );
        }
        let midpoint = line_start + (line_end - line_start) * 0.5;
        let noise = Self::organic_noise(midpoint, brush);
        let source = Self::resolve_brush_source(brush, host_source, Some(0), midpoint);
        let depth =
            (brush.depth * brush.height_depth).max(layer.cell_size * 0.20) * (1.0 + noise * 0.20);
        let radius = (width * (1.0 + noise * 0.08)).max(layer.cell_size * 0.5);

        if erase {
            layer.erase_capsule(
                line_start,
                line_end,
                radius,
                anchor_offset,
                depth,
                brush.line_softness,
                brush.height_falloff,
                grow_positive,
            )
        } else {
            layer.paint_capsule(
                line_start,
                line_end,
                radius,
                anchor_offset,
                depth,
                brush.line_softness,
                brush.height_falloff,
                brush.flow.clamp(0.02, 1.0),
                brush.channel,
                source,
                grow_positive,
            )
        }
    }

    fn sync_active_graph_to_map(map: &mut Map) {
        let active = match map.properties.get("organic_brush_active_graph") {
            Some(Value::Id(id)) if map.organic_brush_graphs.contains_key(id) => Some(*id),
            _ => map.organic_brush_graphs.first().map(|(id, _)| *id),
        };
        if let Some(id) = active {
            map.properties
                .set("organic_brush_active_graph", Value::Id(id));
        }
    }

    fn organic_noise(pos: Vec2<f32>, brush: &OrganicBrushEval) -> f32 {
        if brush.noise_strength <= 0.001 {
            return 0.0;
        }
        let scale = brush.noise_scale.max(0.01);
        let seed = brush.noise_seed as f32 * 0.137;
        let value =
            ((pos.x * scale + seed).sin() * 12.9898 + (pos.y * scale - seed).cos() * 78.233).sin();
        value * brush.noise_strength.clamp(0.0, 1.0)
    }

    fn scatter_offset(
        index: usize,
        count: usize,
        brush: &OrganicBrushEval,
        base_radius: f32,
    ) -> Vec2<f32> {
        if count <= 1 {
            return Vec2::zero();
        }
        let angle = (index as f32 * 2.3999632) + brush.jitter * 3.14159;
        let ring = ((index + 1) as f32 / count as f32).sqrt();
        let amount = base_radius * brush.scatter_jitter * (0.15 + brush.jitter * 0.35);
        Vec2::new(angle.cos(), angle.sin()) * (ring * amount)
    }

    fn mark_dirty_chunks(
        dirty_chunks: &mut HashSet<(i32, i32)>,
        map: &Map,
        hit_pos: Vec3<f32>,
        radius: f32,
    ) {
        let chunk_size = map.terrain.chunk_size.max(1);
        let reach = radius.max(1.0) + 1.0;
        let min_x = ((hit_pos.x - reach).floor() as i32).div_euclid(chunk_size) * chunk_size;
        let max_x = ((hit_pos.x + reach).ceil() as i32).div_euclid(chunk_size) * chunk_size;
        let min_z = ((hit_pos.z - reach).floor() as i32).div_euclid(chunk_size) * chunk_size;
        let max_z = ((hit_pos.z + reach).ceil() as i32).div_euclid(chunk_size) * chunk_size;

        let mut cz = min_z;
        while cz <= max_z {
            let mut cx = min_x;
            while cx <= max_x {
                dirty_chunks.insert((cx, cz));
                cx += chunk_size;
            }
            cz += chunk_size;
        }
    }
}

fn map_from_project<'a>(project: &'a mut Project, server_ctx: &ServerContext) -> &'a mut Map {
    project
        .get_map_mut(server_ctx)
        .expect("organic tool requires an active map")
}
