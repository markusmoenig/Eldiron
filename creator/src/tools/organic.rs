use crate::editor::{DOCKMANAGER, RUSTERIX, SCENEMANAGER};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::PixelSource;
use rusterix::Surface;
use scenevm::GeoId;
use std::collections::HashSet;

const PROP_RADIUS: &str = "organic_brush_radius";
const PROP_FLOW: &str = "organic_brush_flow";
const PROP_JITTER: &str = "organic_brush_jitter";
const PROP_DEPTH: &str = "organic_brush_depth";
const PROP_CELL_SIZE: &str = "organic_brush_cell_size";
const PROP_SHAPE_MODE: &str = "organic_brush_shape_mode";
const PROP_SOFTNESS: &str = "organic_brush_softness";
const PROP_SCATTER_COUNT: &str = "organic_brush_scatter_count";
const PROP_SCATTER_JITTER: &str = "organic_brush_scatter_jitter";
const PROP_HEIGHT_FALLOFF: &str = "organic_brush_height_falloff";
const PROP_NOISE_SCALE: &str = "organic_brush_noise_scale";
const PROP_NOISE_STRENGTH: &str = "organic_brush_noise_strength";
const PROP_NOISE_SEED: &str = "organic_brush_noise_seed";
const PROP_CHANNEL: &str = "organic_brush_channel";
const PROP_LINE_LENGTH: &str = "organic_brush_line_length";
const PROP_LINE_WIDTH: &str = "organic_brush_line_width";
const PROP_LINE_SOFTNESS: &str = "organic_brush_line_softness";
const PROP_PALETTE_1: &str = "organic_brush_palette_1";
const PROP_PALETTE_2: &str = "organic_brush_palette_2";
const PROP_PALETTE_3: &str = "organic_brush_palette_3";
const PROP_BORDER_SIZE: &str = "organic_brush_border_size";
const PROP_OPACITY: &str = "organic_brush_opacity";
pub(crate) const PROP_RENDER_ACTIVE: &str = "organic_render_active";
pub(crate) const PROP_LOCK_MODE: &str = "organic_paint_lock_mode";
const ORGANIC_DETAIL_TEXTURE_SIZE: u32 = 128;

#[derive(Clone, Copy)]
struct OrganicTextureRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OrganicBrushShape {
    Blob,
    Streak,
}

#[derive(Clone)]
struct OrganicBrushEval {
    radius: f32,
    flow: f32,
    jitter: f32,
    depth: f32,
    cell_size: f32,
    softness: f32,
    scatter_count: i32,
    scatter_jitter: f32,
    height_falloff: f32,
    noise_scale: f32,
    noise_strength: f32,
    noise_seed: i32,
    channel: i32,
    line_length: f32,
    line_width: f32,
    line_softness: f32,
    shape: OrganicBrushShape,
    border_size: f32,
    palette_indices: Vec<u16>,
}

impl OrganicBrushEval {
    fn uses_line_shape(&self) -> bool {
        matches!(self.shape, OrganicBrushShape::Streak)
    }
}

pub struct OrganicTool {
    id: TheId,
    previous_dock: Option<String>,
    stroke_active: bool,
    stroke_changed: bool,
    stroke_prev_map: Option<Map>,
    stroke_work_map: Option<Map>,
    dirty_chunks: HashSet<(i32, i32)>,
    dirty_terrain_tiles: HashSet<(i32, i32)>,
    last_stroke_hit_pos: Option<Vec3<f32>>,
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
            dirty_terrain_tiles: HashSet::default(),
            last_stroke_hit_pos: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_organic")
    }

    fn icon_name(&self) -> String {
        str!("paint-brush")
    }

    fn accel(&self) -> Option<char> {
        Some('O')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/organic".to_string())
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
                server_ctx.curr_map_tool_type = MapToolType::Sector;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    self.cancel_stroke(map);
                }

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
                true
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    self.cancel_stroke(map);
                }
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
            MapHover(_) => {
                server_ctx.hover_cursor = None;
            }
            MapClicked(_) | MapDragged(_) => {
                let erase = ui.shift;
                self.begin_stroke_if_needed(map);
                if let Some(work_map) = self.stroke_work_map.as_mut() {
                    let changed = Self::apply_stroke(
                        work_map,
                        server_ctx,
                        erase,
                        &mut self.dirty_chunks,
                        &mut self.dirty_terrain_tiles,
                        &mut self.last_stroke_hit_pos,
                    );
                    if changed {
                        self.stroke_changed = true;
                        *map = work_map.clone();
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
    pub(crate) fn render_active(map: &Map) -> bool {
        map.properties.get_bool_default(PROP_RENDER_ACTIVE, true)
    }

    pub(crate) fn locked_mode(map: &Map) -> bool {
        map.properties.get_int_default(PROP_LOCK_MODE, 0) != 0
    }

    fn begin_stroke_if_needed(&mut self, map: &Map) {
        if self.stroke_active {
            return;
        }
        self.stroke_active = true;
        self.stroke_changed = false;
        self.stroke_prev_map = Some(map.clone());
        self.stroke_work_map = Some(map.clone());
        self.dirty_chunks.clear();
        self.dirty_terrain_tiles.clear();
        self.last_stroke_hit_pos = None;
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

        if changed && let (Some(prev), Some(work)) = (prev, work) {
            SCENEMANAGER.write().unwrap().update_map(work.clone());
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
            for surface_id in prev.surfaces.keys().copied().collect::<Vec<_>>() {
                Self::sync_surface_detail_to_vm(&prev, surface_id);
            }
            for &(tile_x, tile_z) in &self.dirty_terrain_tiles {
                Self::sync_terrain_detail_to_vm(&prev, tile_x, tile_z);
            }
            *map = prev;
        }
        self.stroke_active = false;
        self.stroke_changed = false;
        self.stroke_work_map = None;
        self.dirty_chunks.clear();
        self.dirty_terrain_tiles.clear();
        self.last_stroke_hit_pos = None;
    }

    fn apply_stroke(
        map: &mut Map,
        server_ctx: &ServerContext,
        erase: bool,
        dirty_chunks: &mut HashSet<(i32, i32)>,
        dirty_terrain_tiles: &mut HashSet<(i32, i32)>,
        last_stroke_hit_pos: &mut Option<Vec3<f32>>,
    ) -> bool {
        let brush = Self::evaluate_brush(map);
        let hit_pos = server_ctx.hover_cursor_3d.unwrap_or(server_ctx.geo_hit_pos);
        if brush.uses_line_shape() && last_stroke_hit_pos.is_none() {
            *last_stroke_hit_pos = Some(hit_pos);
            return false;
        }

        let step = (brush.radius * 0.45)
            .max(brush.cell_size * 0.75)
            .max(0.05);
        let start = last_stroke_hit_pos.unwrap_or(hit_pos);
        let delta = hit_pos - start;
        let dist = delta.magnitude();

        if brush.uses_line_shape() && dist > 0.0001 {
            Self::mark_dirty_chunks(dirty_chunks, hit_pos, brush.radius.max(brush.depth));
            Self::mark_dirty_chunks(dirty_chunks, start, brush.radius.max(brush.depth));
            let changed = Self::apply_line_segment(
                map,
                server_ctx,
                start,
                hit_pos,
                &brush,
                erase,
                dirty_terrain_tiles,
            );
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
            Self::mark_dirty_chunks(dirty_chunks, sample, brush.radius.max(brush.depth));
            changed |=
                Self::apply_stroke_at(map, server_ctx, sample, &brush, erase, dirty_terrain_tiles);
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
        erase: bool,
        dirty_terrain_tiles: &mut HashSet<(i32, i32)>,
    ) -> bool {
        let Some(surface) = Self::paint_target_surface(map, server_ctx)
        else {
            if !Self::locked_mode(map) && matches!(server_ctx.geo_hit, Some(GeoId::Terrain(_, _))) {
                return Self::apply_terrain_line_segment(
                    map,
                    start_pos,
                    end_pos,
                    brush,
                    erase,
                    dirty_terrain_tiles,
                );
            }
            return false;
        };

        let Some((start_local, end_local)) = map.surfaces.get(&surface.id).map(|surface_ref| {
            (
                surface_ref.uv_to_tile_local(surface_ref.world_to_uv(start_pos), map),
                surface_ref.uv_to_tile_local(surface_ref.world_to_uv(end_pos), map),
            )
        }) else {
            return false;
        };

        let source = map
            .find_sector(surface.sector_id)
            .and_then(|sector| sector.properties.get_default_source().cloned());
        let Some(surface_ref) = map.surfaces.get_mut(&surface.id) else {
            return false;
        };
        let surface_normal = surface_ref.normal();
        let signed_dist = (end_pos - surface_ref.plane.origin).dot(surface_normal);
        let grow_positive = Self::resolve_surface_growth_side(
            signed_dist,
            surface_normal,
            server_ctx.hover_ray_dir_3d,
        );
        let layer = surface_ref.organic_layer_for_cell_size_mut(brush.cell_size);
        let changed = Self::apply_brush_line(
            layer,
            start_local,
            end_local,
            signed_dist,
            brush,
            source,
            grow_positive,
            erase,
        );
        if changed {
            map.changed += 1;
            let radius = brush.radius.max(brush.line_width).max(brush.depth);
            let dirty_min = Vec2::new(start_local.x.min(end_local.x), start_local.y.min(end_local.y))
                - Vec2::broadcast(radius);
            let dirty_max = Vec2::new(start_local.x.max(end_local.x), start_local.y.max(end_local.y))
                + Vec2::broadcast(radius);
            Self::sync_surface_detail_region_to_vm(map, surface.id, dirty_min, dirty_max);
        }
        changed
    }

    fn apply_stroke_at(
        map: &mut Map,
        server_ctx: &ServerContext,
        hit_pos: Vec3<f32>,
        brush: &OrganicBrushEval,
        erase: bool,
        dirty_terrain_tiles: &mut HashSet<(i32, i32)>,
    ) -> bool {
        let Some(surface) = Self::paint_target_surface(map, server_ctx)
        else {
            if !Self::locked_mode(map) && matches!(server_ctx.geo_hit, Some(GeoId::Terrain(_, _))) {
                return Self::apply_terrain_stroke_at(map, hit_pos, brush, erase, dirty_terrain_tiles);
            }
            return false;
        };

        let source = map
            .find_sector(surface.sector_id)
            .and_then(|sector| sector.properties.get_default_source().cloned());
        let Some(local) = map
            .surfaces
            .get(&surface.id)
            .map(|surface_ref| surface_ref.uv_to_tile_local(surface_ref.world_to_uv(hit_pos), map))
        else {
            return false;
        };
        let Some(surface_ref) = map.surfaces.get_mut(&surface.id) else {
            return false;
        };
        let surface_normal = surface_ref.normal();
        let signed_dist = (hit_pos - surface_ref.plane.origin).dot(surface_normal);
        let grow_positive = Self::resolve_surface_growth_side(
            signed_dist,
            surface_normal,
            server_ctx.hover_ray_dir_3d,
        );
        let layer = surface_ref.organic_layer_for_cell_size_mut(brush.cell_size);
        let changed = Self::apply_brush_dabs(
            layer,
            local,
            signed_dist,
            brush,
            source,
            grow_positive,
            erase,
        );
        if changed {
            map.changed += 1;
            let radius = brush.radius.max(brush.depth);
            let dirty_min = local - Vec2::broadcast(radius);
            let dirty_max = local + Vec2::broadcast(radius);
            Self::sync_surface_detail_region_to_vm(map, surface.id, dirty_min, dirty_max);
        }
        changed
    }

    pub(crate) fn sync_surface_detail_to_vm(map: &Map, surface_id: Uuid) {
        let Some(surface) = map.surfaces.get(&surface_id) else {
            return;
        };
        let rgba = surface.organic_detail_texture_rgba(map, ORGANIC_DETAIL_TEXTURE_SIZE);
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix
            .scene_handler
            .vm
            .execute(scenevm::Atom::SetOrganicSurfaceDetail {
                surface_id,
                size: ORGANIC_DETAIL_TEXTURE_SIZE,
                rgba,
            });
        rusterix.set_dirty();
    }

    fn local_bounds_to_texture_rect(
        local_min: Vec2<f32>,
        local_max: Vec2<f32>,
        texture_min: Vec2<f32>,
        texture_max: Vec2<f32>,
    ) -> Option<OrganicTextureRect> {
        let texture_size = Vec2::new(
            (texture_max.x - texture_min.x).max(0.001),
            (texture_max.y - texture_min.y).max(0.001),
        );
        let to_px = |v: f32, min: f32, size: f32| ((v - min) / size) * ORGANIC_DETAIL_TEXTURE_SIZE as f32;
        let x0 = to_px(local_min.x, texture_min.x, texture_size.x)
            .floor()
            .clamp(0.0, ORGANIC_DETAIL_TEXTURE_SIZE as f32);
        let y0 = to_px(local_min.y, texture_min.y, texture_size.y)
            .floor()
            .clamp(0.0, ORGANIC_DETAIL_TEXTURE_SIZE as f32);
        let x1 = to_px(local_max.x, texture_min.x, texture_size.x)
            .ceil()
            .clamp(0.0, ORGANIC_DETAIL_TEXTURE_SIZE as f32);
        let y1 = to_px(local_max.y, texture_min.y, texture_size.y)
            .ceil()
            .clamp(0.0, ORGANIC_DETAIL_TEXTURE_SIZE as f32);
        let x = x0 as u32;
        let y = y0 as u32;
        let width = (x1 as u32).saturating_sub(x).max(1);
        let height = (y1 as u32).saturating_sub(y).max(1);
        if x >= ORGANIC_DETAIL_TEXTURE_SIZE || y >= ORGANIC_DETAIL_TEXTURE_SIZE {
            return None;
        }
        Some(OrganicTextureRect {
            x,
            y,
            width: width.min(ORGANIC_DETAIL_TEXTURE_SIZE - x),
            height: height.min(ORGANIC_DETAIL_TEXTURE_SIZE - y),
        })
    }

    fn sync_surface_detail_region_to_vm(
        map: &Map,
        surface_id: Uuid,
        dirty_local_min: Vec2<f32>,
        dirty_local_max: Vec2<f32>,
    ) {
        let Some(surface) = map.surfaces.get(&surface_id) else {
            return;
        };
        let Some((texture_min, texture_max)) = surface.organic_local_bounds(map) else {
            Self::sync_surface_detail_to_vm(map, surface_id);
            return;
        };
        let Some(rect) = Self::local_bounds_to_texture_rect(
            dirty_local_min,
            dirty_local_max,
            texture_min,
            texture_max,
        ) else {
            return;
        };
        let rgba = surface.organic_detail_texture_rect_rgba(
            map,
            ORGANIC_DETAIL_TEXTURE_SIZE,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
        );
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix
            .scene_handler
            .vm
            .execute(scenevm::Atom::SetOrganicSurfaceDetailRect {
                surface_id,
                size: ORGANIC_DETAIL_TEXTURE_SIZE,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                rgba,
            });
        rusterix.set_dirty();
    }

    fn terrain_tile_texture_rgba(map: &Map, tile_x: i32, tile_z: i32) -> Vec<u8> {
        let size = ORGANIC_DETAIL_TEXTURE_SIZE.max(1);
        let mut rgba = vec![0u8; (size * size * 4) as usize];
        let size_f = size as f32;
        for y in 0..size {
            for x in 0..size {
                let local = Vec2::new(
                    tile_x as f32 + (x as f32 + 0.5) / size_f,
                    tile_z as f32 + (y as f32 + 0.5) / size_f,
                );
                if let Some(cell) = map.terrain_organic_layer.sample(local) {
                    let offset = ((y * size + x) * 4) as usize;
                    rgba[offset] = cell.palette_index;
                    rgba[offset + 3] = cell.coverage;
                }
            }
        }
        rgba
    }

    pub(crate) fn sync_terrain_detail_to_vm(map: &Map, tile_x: i32, tile_z: i32) {
        let rgba = Self::terrain_tile_texture_rgba(map, tile_x, tile_z);
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix
            .scene_handler
            .vm
            .execute(scenevm::Atom::SetOrganicSurfaceDetail {
                surface_id: rusterix::terrain_organic_detail_id(tile_x, tile_z),
                size: ORGANIC_DETAIL_TEXTURE_SIZE,
                rgba,
            });
        rusterix.set_dirty();
    }

    fn terrain_tile_texture_rect_rgba(
        map: &Map,
        tile_x: i32,
        tile_z: i32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let mut rgba = vec![0u8; (width.max(1) * height.max(1) * 4) as usize];
        let size_f = ORGANIC_DETAIL_TEXTURE_SIZE as f32;
        for row in 0..height {
            for col in 0..width {
                let local = Vec2::new(
                    tile_x as f32 + ((x + col) as f32 + 0.5) / size_f,
                    tile_z as f32 + ((y + row) as f32 + 0.5) / size_f,
                );
                if let Some(cell) = map.terrain_organic_layer.sample(local) {
                    let offset = ((row * width + col) * 4) as usize;
                    rgba[offset] = cell.palette_index;
                    rgba[offset + 3] = cell.coverage;
                }
            }
        }
        rgba
    }

    fn sync_terrain_detail_region_to_vm(
        map: &Map,
        tile_x: i32,
        tile_z: i32,
        local_min: Vec2<f32>,
        local_max: Vec2<f32>,
    ) {
        let tile_min = Vec2::new(tile_x as f32, tile_z as f32);
        let tile_max = tile_min + Vec2::new(1.0, 1.0);
        let clipped_min = Vec2::new(local_min.x.max(tile_min.x), local_min.y.max(tile_min.y));
        let clipped_max = Vec2::new(local_max.x.min(tile_max.x), local_max.y.min(tile_max.y));
        if clipped_max.x <= clipped_min.x || clipped_max.y <= clipped_min.y {
            return;
        }
        let Some(rect) =
            Self::local_bounds_to_texture_rect(clipped_min, clipped_max, tile_min, tile_max)
        else {
            return;
        };
        let rgba = Self::terrain_tile_texture_rect_rgba(
            map,
            tile_x,
            tile_z,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
        );
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix
            .scene_handler
            .vm
            .execute(scenevm::Atom::SetOrganicSurfaceDetailRect {
                surface_id: rusterix::terrain_organic_detail_id(tile_x, tile_z),
                size: ORGANIC_DETAIL_TEXTURE_SIZE,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                rgba,
            });
        rusterix.set_dirty();
    }

    pub(crate) fn sync_render_active_to_vm(map: &Map) {
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix
            .scene_handler
            .vm
            .execute(scenevm::Atom::SetOrganicVisible {
                visible: Self::render_active(map),
            });
        rusterix.set_dirty();
    }

    pub(crate) fn terrain_tiles_for_sync(
        layer: &rusterix::OrganicVolumeLayer,
    ) -> HashSet<(i32, i32)> {
        let mut tiles = HashSet::default();
        let page_size = layer.page_size.max(1) as f32;
        let page_span = page_size * layer.cell_size.max(0.01);
        for page in layer.pages.values() {
            let min = Vec2::new(page.page_x as f32 * page_span, page.page_y as f32 * page_span);
            let max = min + Vec2::broadcast(page_span);
            let min_tile_x = min.x.floor() as i32;
            let max_tile_x = (max.x - 0.0001).floor() as i32;
            let min_tile_z = min.y.floor() as i32;
            let max_tile_z = (max.y - 0.0001).floor() as i32;
            for tile_z in min_tile_z..=max_tile_z {
                for tile_x in min_tile_x..=max_tile_x {
                    tiles.insert((tile_x, tile_z));
                }
            }
        }
        tiles
    }

    pub(crate) fn sync_all_detail_to_vm(map: &Map) {
        for surface_id in map.surfaces.keys().copied() {
            Self::sync_surface_detail_to_vm(map, surface_id);
        }
        for (tile_x, tile_z) in Self::terrain_tiles_for_sync(&map.terrain_organic_layer) {
            Self::sync_terrain_detail_to_vm(map, tile_x, tile_z);
        }
        Self::sync_render_active_to_vm(map);
    }

    fn paint_target_surface<'a>(map: &Map, server_ctx: &'a ServerContext) -> Option<&'a Surface> {
        if Self::locked_mode(map) {
            if let Some(surface) = server_ctx.active_detail_surface.as_ref() {
                return Some(surface);
            }
            return server_ctx.hover_surface.as_ref().filter(|surface| {
                map.selected_sectors.contains(&surface.sector_id)
            });
        }
        server_ctx
            .active_detail_surface
            .as_ref()
            .or(server_ctx.hover_surface.as_ref())
    }

    fn mark_dirty_terrain_tiles(
        dirty_tiles: &mut HashSet<(i32, i32)>,
        pos: Vec3<f32>,
        radius: f32,
    ) {
        let min_x = (pos.x - radius).floor() as i32;
        let max_x = (pos.x + radius).floor() as i32;
        let min_z = (pos.z - radius).floor() as i32;
        let max_z = (pos.z + radius).floor() as i32;
        for tile_z in min_z..=max_z {
            for tile_x in min_x..=max_x {
                dirty_tiles.insert((tile_x, tile_z));
            }
        }
    }

    fn apply_terrain_stroke_at(
        map: &mut Map,
        hit_pos: Vec3<f32>,
        brush: &OrganicBrushEval,
        erase: bool,
        dirty_terrain_tiles: &mut HashSet<(i32, i32)>,
    ) -> bool {
        let source = Some(PixelSource::PaletteIndex(
            brush.palette_indices.first().copied().unwrap_or(4),
        ));
        let local = Vec2::new(hit_pos.x, hit_pos.z);
        let changed = {
            let layer = &mut map.terrain_organic_layer;
            Self::apply_brush_dabs(layer, local, 0.0, brush, source, true, erase)
        };
        if changed {
            map.changed += 1;
            Self::mark_dirty_terrain_tiles(dirty_terrain_tiles, hit_pos, brush.radius.max(brush.depth));
            let dirty_min = local - Vec2::broadcast(brush.radius.max(brush.depth));
            let dirty_max = local + Vec2::broadcast(brush.radius.max(brush.depth));
            for &(tile_x, tile_z) in dirty_terrain_tiles.iter() {
                Self::sync_terrain_detail_region_to_vm(map, tile_x, tile_z, dirty_min, dirty_max);
            }
        }
        changed
    }

    fn apply_terrain_line_segment(
        map: &mut Map,
        start_pos: Vec3<f32>,
        end_pos: Vec3<f32>,
        brush: &OrganicBrushEval,
        erase: bool,
        dirty_terrain_tiles: &mut HashSet<(i32, i32)>,
    ) -> bool {
        let source = Some(PixelSource::PaletteIndex(
            brush.palette_indices.first().copied().unwrap_or(4),
        ));
        let start_local = Vec2::new(start_pos.x, start_pos.z);
        let end_local = Vec2::new(end_pos.x, end_pos.z);
        let changed = {
            let layer = &mut map.terrain_organic_layer;
            Self::apply_brush_line(layer, start_local, end_local, 0.0, brush, source, true, erase)
        };
        if changed {
            map.changed += 1;
            Self::mark_dirty_terrain_tiles(dirty_terrain_tiles, start_pos, brush.radius.max(brush.depth));
            Self::mark_dirty_terrain_tiles(dirty_terrain_tiles, end_pos, brush.radius.max(brush.depth));
            let radius = brush.radius.max(brush.line_width).max(brush.depth);
            let dirty_min = Vec2::new(start_local.x.min(end_local.x), start_local.y.min(end_local.y))
                - Vec2::broadcast(radius);
            let dirty_max = Vec2::new(start_local.x.max(end_local.x), start_local.y.max(end_local.y))
                + Vec2::broadcast(radius);
            for &(tile_x, tile_z) in dirty_terrain_tiles.iter() {
                Self::sync_terrain_detail_region_to_vm(map, tile_x, tile_z, dirty_min, dirty_max);
            }
        }
        changed
    }

    fn evaluate_brush(map: &Map) -> OrganicBrushEval {
        let mut palette_indices = vec![
            map.properties.get_int_default(PROP_PALETTE_1, 4).clamp(0, 255) as u16,
            map.properties.get_int_default(PROP_PALETTE_2, 8).clamp(0, 255) as u16,
            map.properties.get_int_default(PROP_PALETTE_3, 10).clamp(0, 255) as u16,
        ];
        palette_indices.retain(|index| *index <= 255);

        OrganicBrushEval {
            radius: map.properties.get_float_default(PROP_RADIUS, 0.6).max(0.05),
            flow: map
                .properties
                .get_float_default(
                    PROP_OPACITY,
                    map.properties.get_float_default(PROP_FLOW, 0.7),
                )
                .clamp(0.05, 1.0),
            jitter: map.properties.get_float_default(PROP_JITTER, 0.15).clamp(0.0, 1.0),
            depth: map.properties.get_float_default(PROP_DEPTH, 0.18).max(0.01),
            cell_size: map.properties.get_float_default(PROP_CELL_SIZE, 0.05).max(0.01),
            softness: map.properties.get_float_default(PROP_SOFTNESS, 0.4).clamp(0.0, 1.0),
            scatter_count: map
                .properties
                .get_int_default(PROP_SCATTER_COUNT, 1)
                .clamp(1, 32),
            scatter_jitter: map
                .properties
                .get_float_default(PROP_SCATTER_JITTER, 0.0)
                .clamp(0.0, 1.0),
            height_falloff: map
                .properties
                .get_float_default(PROP_HEIGHT_FALLOFF, 0.5)
                .clamp(0.0, 1.0),
            noise_scale: map
                .properties
                .get_float_default(PROP_NOISE_SCALE, 0.3)
                .max(0.01),
            noise_strength: map
                .properties
                .get_float_default(PROP_NOISE_STRENGTH, 0.0)
                .clamp(0.0, 1.0),
            noise_seed: map.properties.get_int_default(PROP_NOISE_SEED, 1),
            channel: map.properties.get_int_default(PROP_CHANNEL, 0).clamp(0, 3),
            line_length: map
                .properties
                .get_float_default(PROP_LINE_LENGTH, 1.8)
                .max(0.1),
            line_width: map
                .properties
                .get_float_default(PROP_LINE_WIDTH, 0.22)
                .max(0.02),
            line_softness: map
                .properties
                .get_float_default(PROP_LINE_SOFTNESS, 0.4)
                .clamp(0.0, 1.0),
            shape: if map.properties.get_int_default(PROP_SHAPE_MODE, 0) == 1 {
                OrganicBrushShape::Streak
            } else {
                OrganicBrushShape::Blob
            },
            border_size: map
                .properties
                .get_float_default(PROP_BORDER_SIZE, 0.14)
                .clamp(0.0, 0.48),
            palette_indices,
        }
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

    fn apply_brush_dabs(
        layer: &mut rusterix::OrganicVolumeLayer,
        center: Vec2<f32>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        host_source: Option<PixelSource>,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        let scatter_count = brush.scatter_count.max(1) as usize;
        let base_radius = brush.radius.max(layer.cell_size * 0.5);
        let base_depth = brush.depth.max(layer.cell_size * 0.24);
        let shallow_spread = base_depth <= layer.cell_size * 0.95 && scatter_count > 1;
        let dab_radius = if scatter_count > 1 {
            let scale = if shallow_spread { 0.96 } else { 0.82 };
            (base_radius * scale).max(layer.cell_size * 0.5)
        } else {
            base_radius
        };

        let mut changed = false;
        for index in 0..scatter_count {
            let offset = Self::scatter_offset(index, scatter_count, brush, base_radius);
            let dab_center = center + offset;
            let noise = Self::organic_noise(dab_center, brush);
            let source =
                Self::resolve_brush_source(brush, host_source.clone(), Some(index as i32), dab_center);
            let dab_flow = if scatter_count > 1 {
                if shallow_spread {
                    (brush.flow / (scatter_count as f32).sqrt()).clamp(0.08, 1.0)
                } else {
                    (brush.flow / scatter_count as f32).clamp(0.06, 1.0)
                }
            } else {
                brush.flow
            };
            let dab_depth = if scatter_count > 1 {
                let scale = if shallow_spread { 1.0 } else { 0.92 };
                (base_depth * scale).max(layer.cell_size * 0.24)
            } else {
                base_depth
            } * (1.0 + noise * 0.42);
            let dab_radius = (dab_radius * (1.0 + noise * 0.32)).max(layer.cell_size * 0.5);
            let inner_radius = (dab_radius * (1.0 - brush.border_size).clamp(0.18, 1.0))
                .max(layer.cell_size * 0.35);

            let dab_changed = if erase {
                layer.erase_sphere(
                    dab_center,
                    dab_radius,
                    anchor_offset,
                    dab_depth,
                    brush.softness,
                    brush.height_falloff,
                    grow_positive,
                )
            } else {
                let mut changed = false;
                if let Some(border_source) = Self::fixed_brush_source(brush, host_source.clone(), 1) {
                    if brush.border_size > 0.01 {
                        changed |= layer.paint_sphere(
                            dab_center,
                            dab_radius,
                            anchor_offset,
                            dab_depth,
                            brush.softness,
                            brush.height_falloff,
                            dab_flow,
                            brush.channel,
                            Some(border_source),
                            grow_positive,
                        );
                    }
                }
                changed |= layer.paint_sphere(
                    dab_center,
                    inner_radius,
                    anchor_offset,
                    dab_depth,
                    brush.softness,
                    brush.height_falloff,
                    dab_flow,
                    brush.channel,
                    source,
                    grow_positive,
                );
                if let Some(noise_source) =
                    Self::fixed_brush_source(brush, host_source.clone(), 2)
                    && brush.noise_strength > 0.01
                {
                    let noise_count = ((brush.noise_strength * 4.0).ceil() as usize).clamp(1, 4);
                    for noise_index in 0..noise_count {
                        let noise_offset = Self::noise_offset(
                            dab_center,
                            noise_index,
                            brush,
                            inner_radius,
                        );
                        let noise_radius = (inner_radius
                            * (0.12 + brush.noise_strength * 0.18))
                            .max(layer.cell_size * 0.24);
                        changed |= layer.paint_sphere(
                            dab_center + noise_offset,
                            noise_radius,
                            anchor_offset,
                            dab_depth * 0.8,
                            brush.softness,
                            brush.height_falloff,
                            (dab_flow * brush.noise_strength).clamp(0.03, 1.0),
                            brush.channel,
                            Some(noise_source.clone()),
                            grow_positive,
                        );
                    }
                }
                changed
            };
            changed |= dab_changed;
        }

        changed
    }

    fn apply_brush_line(
        layer: &mut rusterix::OrganicVolumeLayer,
        start: Vec2<f32>,
        end: Vec2<f32>,
        anchor_offset: f32,
        brush: &OrganicBrushEval,
        host_source: Option<PixelSource>,
        grow_positive: bool,
        erase: bool,
    ) -> bool {
        let delta = end - start;
        let dist = delta.magnitude();
        if dist <= 0.0001 {
            return false;
        }

        let dir = delta / dist;
        let target_len = (brush.line_length * brush.radius).max(dist);
        let mid = start + delta * 0.5;
        let half = dir * (target_len * 0.5);
        let line_start = mid - half;
        let line_end = mid + half;

        let width = (brush.line_width * brush.radius).max(layer.cell_size * 0.55);
        let midpoint = mid;
        let noise = Self::organic_noise(midpoint, brush);
        let source = Self::resolve_brush_source(brush, host_source, Some(0), midpoint);
        let depth = brush.depth.max(layer.cell_size * 0.26) * (1.0 + noise * 0.28);
        let radius = (width * (1.0 + noise * 0.14)).max(layer.cell_size * 0.55);
        let inner_radius = (radius * (1.0 - brush.border_size).clamp(0.18, 1.0))
            .max(layer.cell_size * 0.4);

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
            let mut changed = false;
            if let Some(border_source) = Self::fixed_brush_source(brush, None, 1) {
                if brush.border_size > 0.01 {
                    changed |= layer.paint_capsule(
                        line_start,
                        line_end,
                        radius,
                        anchor_offset,
                        depth,
                        brush.line_softness,
                        brush.height_falloff,
                        brush.flow.clamp(0.08, 1.0),
                        brush.channel,
                        Some(border_source),
                        grow_positive,
                    );
                }
            }
            changed |= layer.paint_capsule(
                line_start,
                line_end,
                inner_radius,
                anchor_offset,
                depth,
                brush.line_softness,
                brush.height_falloff,
                brush.flow.clamp(0.08, 1.0),
                brush.channel,
                source,
                grow_positive,
            );
            if let Some(noise_source) = Self::fixed_brush_source(brush, None, 2)
                && brush.noise_strength > 0.01
            {
                let noise_count = ((brush.noise_strength * 5.0).ceil() as usize).clamp(1, 5);
                for noise_index in 0..noise_count {
                    let t = (noise_index as f32 + 0.5) / noise_count as f32;
                    let along = line_start + (line_end - line_start) * t;
                    let lateral = Vec2::new(-(line_end - line_start).y, (line_end - line_start).x)
                        .normalized()
                        * ((Self::scalar_hash(
                            t * 19.0 + brush.noise_seed as f32 * 0.73,
                        ) * 2.0
                            - 1.0)
                            * inner_radius
                            * 0.45);
                    changed |= layer.paint_sphere(
                        along + lateral,
                        (inner_radius * (0.10 + brush.noise_strength * 0.18))
                            .max(layer.cell_size * 0.22),
                        anchor_offset,
                        depth * 0.75,
                        brush.line_softness,
                        brush.height_falloff,
                        (brush.flow * brush.noise_strength).clamp(0.03, 1.0),
                        brush.channel,
                        Some(noise_source.clone()),
                        grow_positive,
                    );
                }
            }
            changed
        }
    }

    fn fixed_brush_source(
        brush: &OrganicBrushEval,
        host_source: Option<PixelSource>,
        palette_slot: usize,
    ) -> Option<PixelSource> {
        if let Some(index) = brush.palette_indices.get(palette_slot).copied() {
            Some(PixelSource::PaletteIndex(index))
        } else if let Some(index) = brush.palette_indices.first().copied() {
            Some(PixelSource::PaletteIndex(index))
        } else {
            host_source
        }
    }

    fn resolve_brush_source(
        brush: &OrganicBrushEval,
        host_source: Option<PixelSource>,
        variant: Option<i32>,
        pos: Vec2<f32>,
    ) -> Option<PixelSource> {
        let _ = (variant, pos);
        if brush.palette_indices.is_empty() {
            return host_source;
        }
        Some(PixelSource::PaletteIndex(brush.palette_indices[0]))
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

    fn scalar_hash(value: f32) -> f32 {
        (value.sin() * 43_758.547).fract().abs()
    }

    fn noise_offset(
        center: Vec2<f32>,
        index: usize,
        brush: &OrganicBrushEval,
        radius: f32,
    ) -> Vec2<f32> {
        let seed = brush.noise_seed as f32 * 0.31 + index as f32 * 1.73;
        let angle = Self::scalar_hash(center.x * 3.17 + center.y * 5.91 + seed)
            * std::f32::consts::TAU;
        let dist = radius
            * (0.12 + Self::scalar_hash(center.x * 7.11 - center.y * 2.47 + seed * 2.0) * 0.48);
        Vec2::new(angle.cos(), angle.sin()) * dist
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
        let angle = (index as f32 * 2.3999632) + brush.jitter * std::f32::consts::PI;
        let ring = ((index + 1) as f32 / count as f32).sqrt();
        let amount = base_radius * brush.scatter_jitter * (0.15 + brush.jitter * 0.35);
        Vec2::new(angle.cos(), angle.sin()) * (ring * amount)
    }

    fn mark_dirty_chunks(
        dirty_chunks: &mut HashSet<(i32, i32)>,
        hit_pos: Vec3<f32>,
        radius: f32,
    ) {
        let chunk_size = 32;
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
