use crate::editor::{RUSTERIX, SCENEMANAGER};
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::prelude::*;
use std::time::{Duration, Instant};
use vek::{Vec2, Vec3};

const LIVE_PAINT_SYNC_INTERVAL: Duration = Duration::from_millis(75);

pub struct RectTool {
    id: TheId,

    hovered_vertices: Option<[Vec2<f32>; 4]>,
    mode: i32,
    hud: Hud,
    composition_mode_rects: Vec<TheDim>,
    rotation_rects: Vec<(TheDim, f32)>,
    // A gesture beginning on the HUD must never become a map-paint stroke.
    hud_control_active: bool,

    processed: FxHashSet<RectPaintKey>,

    stroke_active: bool,
    stroke_changed: bool,
    stroke_prev_map: Option<Map>,
    stroke_work_map: Option<Map>,
    last_2d_cell: Option<Vec2<i32>>,
    line_start_2d_cell: Option<Vec2<i32>>,
    line_axis_horizontal: Option<bool>,
    last_live_scene_sync_at: Option<Instant>,
    stroke_dirty_chunks: FxHashSet<(i32, i32)>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum RectPaintKey {
    Cell(i32, i32),
    Terrain(i32, i32),
    Sector(u32, i32, i32),
    Geometry(Uuid, usize, i32, i32),
}

impl Tool for RectTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Rect Tool"),

            hovered_vertices: None,
            mode: 0,
            hud: Hud::new(HudMode::Rect),
            composition_mode_rects: Vec::new(),
            rotation_rects: Vec::new(),
            hud_control_active: false,

            processed: FxHashSet::default(),

            stroke_active: false,
            stroke_changed: false,
            stroke_prev_map: None,
            stroke_work_map: None,
            last_2d_cell: None,
            line_start_2d_cell: None,
            line_axis_horizontal: None,
            last_live_scene_sync_at: None,
            stroke_dirty_chunks: FxHashSet::default(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        fl!("tool_rect")
    }
    fn icon_name(&self) -> String {
        str!("square")
    }
    fn accel(&self) -> Option<char> {
        Some('R')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                server_ctx.curr_map_tool_type = MapToolType::Rect;
                server_ctx.rect_composition_target = None;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_vertices.clear();
                    region.map.selected_linedefs.clear();
                    region.map.selected_sectors.clear();
                }

                // self.activate_map_tool_helper(ui, ctx, project, server_ctx);

                return true;
            }
            DeActivate => {
                server_ctx.rect_composition_target = None;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                self.reset_stroke();
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.clear_temp();
                }
                return true;
            }
            _ => {}
        };

        false
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut undo_atom: Option<ProjectUndoAtom> = None;

        fn apply_hover(
            coord: Vec2<i32>,
            ui: &mut TheUI,
            _ctx: &mut TheContext,
            map: &mut Map,
            server_ctx: &mut ServerContext,
        ) -> Option<[Vec2<f32>; 4]> {
            let mut hovered_vertices: Option<[Vec2<f32>; 4]> = None;

            if let Some(render_view) = ui.get_render_view("PolyView") {
                let dim = *render_view.dim();
                server_ctx.hover = (None, None, None);
                let cp = server_ctx.local_to_map_cell(
                    Vec2::new(dim.width as f32, dim.height as f32),
                    Vec2::new(coord.x as f32, coord.y as f32),
                    map,
                    1.0,
                );
                let step = 1.0;
                map.curr_rectangle = Some((cp, cp + step));
                hovered_vertices = Some([
                    cp,
                    cp + Vec2::new(0.0, step),
                    cp + Vec2::new(step, step),
                    cp + Vec2::new(step, 0.0),
                ]);
                server_ctx.hover_cursor = Some(cp);
                if RectTool::terrain_edit_plane(map) {
                    server_ctx.rect_terrain_id = Some((cp.x.floor() as i32, cp.y.floor() as i32));
                } else {
                    server_ctx.rect_terrain_id = None;
                }
            }

            hovered_vertices
        }

        match map_event {
            MapKey(c) => {
                if map.camera == rusterix::MapCamera::TwoDIso && (c == '[' || c == ']') {
                    let step = map
                        .properties
                        .get_float_default("iso_step_height", 0.25)
                        .clamp(0.05, 8.);
                    let h = (map.properties.get_float_default("iso_edit_height", 0.)
                        + if c == ']' { step } else { -step })
                    .clamp(0., 32.);
                    map.properties.set("iso_edit_height", Value::Float(h));
                    map.changed = map.changed.wrapping_add(1);
                    map.curr_rectangle = None;
                    RUSTERIX.write().unwrap().set_dirty();
                    ctx.ui.redraw_all = true;
                    return None;
                }
                if map.camera == rusterix::MapCamera::TwoDIso && (c == ',' || c == '.') {
                    Self::rotate_brush(map, if c == ',' { -90. } else { 90. });
                    RUSTERIX.write().unwrap().set_dirty();
                    ctx.ui.redraw_all = true;
                    return None;
                }
            }
            MapClicked(coord) => {
                self.hud_control_active = false;
                if let Some(delta) = self
                    .rotation_rects
                    .iter()
                    .find(|(rect, _)| rect.contains(coord))
                    .map(|(_, d)| *d)
                {
                    self.reset_stroke();
                    self.hud_control_active = true;
                    Self::rotate_brush(map, delta);
                    RUSTERIX.write().unwrap().set_dirty();
                    ctx.ui.redraw_all = true;
                    return None;
                }

                if map.camera == rusterix::MapCamera::TwoDIso
                    && map.properties.get_bool_default("iso_pick_height", false)
                {
                    if let Some(view) = ui.get_render_view("PolyView") {
                        crate::iso_editor::pick_surface(
                            map,
                            Vec2::new(view.dim().width as f32, view.dim().height as f32),
                            coord.map(|v| v as f32),
                        );
                        RUSTERIX.write().unwrap().set_dirty();
                        ctx.ui.redraw_all = true;
                    }
                    return None;
                }
                if let Some(index) = self
                    .composition_mode_rects
                    .iter()
                    .position(|rect| rect.contains(coord))
                {
                    self.reset_stroke();
                    self.hud_control_active = true;
                    server_ctx.rect_compose_mode = index == 1;
                    server_ctx.rect_composition_target = None;
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    ctx.ui.send(TheEvent::SetStatusText(
                        self.id.clone(),
                        if index == 1 {
                            "Compose: click an individual painted tile to edit its material layers."
                        } else {
                            "Paint: paint tiles normally. Alt-click samples a tile."
                        }
                        .into(),
                    ));
                    ctx.ui.redraw_all = true;
                    RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    self.reset_stroke();
                    self.hud_control_active = true;
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if server_ctx.pc.is_region() && server_ctx.rect_compose_mode && !ui.alt {
                    self.reset_stroke();
                    if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                    } else {
                        self.compute_3d_tile(coord, map, ui, server_ctx);
                    }
                    server_ctx.rect_composition_target = Self::composition_target(map, server_ctx);
                    if let Some(binding) = &server_ctx.rect_composition_target {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named(crate::procedural_material::map_composition::OPEN),
                            TheValue::Text(serde_json::to_string(binding).unwrap()),
                        ));
                    } else {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                        ctx.ui.send(TheEvent::SetStatusText(
                            self.id.clone(),
                            "Click a painted tile to compose its materials.".into(),
                        ));
                    }
                    ctx.ui.redraw_all = true;
                    return None;
                }

                self.processed.clear();
                if ui.alt {
                    if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                    } else {
                        self.compute_3d_tile(coord, map, ui, server_ctx);
                    }

                    if let Some(source) = Self::sample_source_at_current_target(map, server_ctx) {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Pick Tile Source"),
                            Self::encode_tile_source(source),
                        ));
                    }
                    return None;
                }

                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    let source = get_source(ui, server_ctx);
                    let use_terrain_paint = Self::uses_terrain_paint(
                        map,
                        source.as_ref(),
                        &RUSTERIX.read().unwrap().assets,
                    );
                    if let Some(cp) = server_ctx.hover_cursor {
                        self.begin_stroke_if_needed(map);
                        let k = if use_terrain_paint {
                            Vec2::new(cp.x.floor() as i32, cp.y.floor() as i32)
                        } else {
                            Vec2::new(cp.x.floor() as i32, cp.y.floor() as i32)
                        };
                        if ui.ctrl && !use_terrain_paint {
                            self.line_start_2d_cell = Some(k);
                            self.line_axis_horizontal = None;
                        }
                        if let Some(work_map) = self.stroke_work_map.as_mut() {
                            let changed = if use_terrain_paint {
                                server_ctx.rect_terrain_id = Some((k.x, k.y));
                                Self::apply_3d_paint_at_current_target(work_map, ui, server_ctx)
                                    .is_some()
                            } else {
                                let step = 1.0;
                                let x = k.x as f32 * step;
                                let y = k.y as f32 * step;
                                let verts = Some([
                                    Vec2::new(x, y),
                                    Vec2::new(x, y + step),
                                    Vec2::new(x + step, y + step),
                                    Vec2::new(x + step, y),
                                ]);
                                Self::paint_cell(
                                    work_map,
                                    verts,
                                    get_source(ui, server_ctx),
                                    ui.shift,
                                    self.mode,
                                )
                            };
                            if changed {
                                self.stroke_changed = true;
                                Self::add_2d_cell_dirty_chunk(k, &mut self.stroke_dirty_chunks);
                                Self::sync_live_stroke_throttled(
                                    work_map,
                                    server_ctx,
                                    &mut self.last_live_scene_sync_at,
                                    &mut self.stroke_dirty_chunks,
                                    false,
                                );
                            }
                            self.processed.insert(RectPaintKey::Cell(k.x, k.y));
                            self.last_2d_cell = Some(k);
                        }
                    }
                } else {
                    self.compute_3d_tile(coord, map, ui, server_ctx);
                    self.begin_stroke_if_needed(map);
                    if let Some(key) = Self::current_3d_paint_key(server_ctx)
                        && !self.processed.contains(&key)
                        && let Some(work_map) = self.stroke_work_map.as_mut()
                        && Self::apply_3d_paint_at_current_target(work_map, ui, server_ctx)
                            .is_some()
                    {
                        self.stroke_changed = true;
                        Self::sync_live_stroke_throttled(
                            work_map,
                            server_ctx,
                            &mut self.last_live_scene_sync_at,
                            &mut self.stroke_dirty_chunks,
                            false,
                        );
                        self.processed.insert(key);
                    }
                }
            }
            MapDragged(coord) => {
                if self.hud_control_active {
                    return None;
                }
                if server_ctx.pc.is_region() && server_ctx.rect_compose_mode && !ui.alt {
                    if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                    } else {
                        self.compute_3d_tile(coord, map, ui, server_ctx);
                    }
                    return None;
                }
                if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }
                if ui.alt {
                    self.reset_stroke();
                    if server_ctx.editor_view_mode == EditorViewMode::D2 {
                        self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                    } else {
                        self.compute_3d_tile(coord, map, ui, server_ctx);
                    }
                    return None;
                }
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    let source = get_source(ui, server_ctx);
                    let use_terrain_paint = Self::uses_terrain_paint(
                        map,
                        source.as_ref(),
                        &RUSTERIX.read().unwrap().assets,
                    );
                    self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                    if let Some(cp) = server_ctx.hover_cursor {
                        self.begin_stroke_if_needed(map);
                        let k = if use_terrain_paint {
                            Vec2::new(cp.x.floor() as i32, cp.y.floor() as i32)
                        } else {
                            Vec2::new(cp.x.floor() as i32, cp.y.floor() as i32)
                        };
                        if let Some(work_map) = self.stroke_work_map.as_mut() {
                            let line_anchor = self.line_start_2d_cell.unwrap_or(k);
                            let mut to = k;
                            if ui.ctrl && !use_terrain_paint {
                                if self.line_axis_horizontal.is_none() {
                                    let dx = (to.x - line_anchor.x).abs();
                                    let dy = (to.y - line_anchor.y).abs();
                                    if dx > 0 || dy > 0 {
                                        self.line_axis_horizontal = Some(dx >= dy);
                                    }
                                }
                                if let Some(horizontal) = self.line_axis_horizontal {
                                    if horizontal {
                                        to.y = line_anchor.y;
                                    } else {
                                        to.x = line_anchor.x;
                                    }
                                }
                            }

                            let from = self.last_2d_cell.unwrap_or(to);
                            let step = 1.0;
                            let between = Self::cells_between(from, to);
                            for cell in between.iter().copied() {
                                let paint_key = RectPaintKey::Cell(cell.x, cell.y);
                                if self.processed.contains(&paint_key) {
                                    continue;
                                }
                                let changed = if use_terrain_paint {
                                    server_ctx.rect_terrain_id = Some((cell.x, cell.y));
                                    Self::apply_3d_paint_at_current_target(work_map, ui, server_ctx)
                                        .is_some()
                                } else {
                                    let x = cell.x as f32 * step;
                                    let y = cell.y as f32 * step;
                                    let verts = Some([
                                        Vec2::new(x, y),
                                        Vec2::new(x, y + step),
                                        Vec2::new(x + step, y + step),
                                        Vec2::new(x + step, y),
                                    ]);
                                    Self::paint_cell(
                                        work_map,
                                        verts,
                                        get_source(ui, server_ctx),
                                        ui.shift,
                                        self.mode,
                                    )
                                };
                                if changed {
                                    self.stroke_changed = true;
                                    Self::add_2d_cell_dirty_chunk(
                                        cell,
                                        &mut self.stroke_dirty_chunks,
                                    );
                                    Self::sync_live_stroke_throttled(
                                        work_map,
                                        server_ctx,
                                        &mut self.last_live_scene_sync_at,
                                        &mut self.stroke_dirty_chunks,
                                        false,
                                    );
                                }
                                self.processed.insert(paint_key);
                            }
                            self.last_2d_cell = Some(to);
                        }
                    }
                } else {
                    self.compute_3d_tile(coord, map, ui, server_ctx);
                    self.begin_stroke_if_needed(map);
                    if let Some(key) = Self::current_3d_paint_key(server_ctx)
                        && !self.processed.contains(&key)
                        && let Some(work_map) = self.stroke_work_map.as_mut()
                        && Self::apply_3d_paint_at_current_target(work_map, ui, server_ctx)
                            .is_some()
                    {
                        self.stroke_changed = true;
                        Self::sync_live_stroke_throttled(
                            work_map,
                            server_ctx,
                            &mut self.last_live_scene_sync_at,
                            &mut self.stroke_dirty_chunks,
                            false,
                        );
                        self.processed.insert(key);
                    }
                }
            }
            MapUp(_) => {
                if self.hud_control_active {
                    self.hud_control_active = false;
                    return None;
                }
                if self.stroke_active {
                    if self.stroke_changed
                        && let (Some(prev), Some(new_map)) =
                            (self.stroke_prev_map.take(), self.stroke_work_map.take())
                    {
                        let has_persistent_change = server_ctx.editor_view_mode
                            != EditorViewMode::D2
                            || crate::toollist::ToolList::changed_d2_rect_paint_chunks(
                                &prev, &new_map,
                            )
                            .is_some();
                        if has_persistent_change {
                            Self::sync_live_stroke_throttled(
                                &new_map,
                                server_ctx,
                                &mut self.last_live_scene_sync_at,
                                &mut self.stroke_dirty_chunks,
                                true,
                            );
                            *map = new_map;
                            undo_atom = Some(ProjectUndoAtom::MapEdit(
                                server_ctx.pc,
                                Box::new(prev),
                                Box::new(map.clone()),
                            ));
                        }
                    }
                    self.reset_stroke();
                }
            }
            MapHover(coord) => {
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                } else {
                    self.compute_3d_tile(coord, map, ui, server_ctx);
                }
            }
            MapDelete => {}
            MapEscape => {
                map.properties.set("iso_pick_height", Value::Bool(false));
                server_ctx.rect_composition_target = None;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));
                self.reset_stroke();
                map.clear_temp();
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
        }
        undo_atom
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        assets: &Assets,
    ) {
        if server_ctx.editor_view_mode == EditorViewMode::D2
            && let Some((top_left, bottom_right)) = map.curr_rectangle
        {
            let screen_size = Vec2::new(buffer.dim().width as f32, buffer.dim().height as f32);
            if map.camera == rusterix::MapCamera::TwoDIso {
                let center = (top_left + bottom_right) * 0.5;
                let edit_height = map.properties.get_float_default("iso_edit_height", 0.);
                let preview_height = if edit_height.abs() < 0.001 {
                    rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(map)
                        .and_then(|t| t.height_at(center))
                        .unwrap_or(edit_height)
                } else {
                    edit_height
                };
                if let Some(rusterix::TileSource::SingleTile(id)) = server_ctx.curr_tile_source {
                    if let Some(tile) = assets.tiles.get(&id) {
                        rusterix::isometric::draw_tile_preview(
                            buffer,
                            tile,
                            rusterix::isometric::Projection::editor(map, screen_size)
                                .at_height(preview_height),
                            center,
                            map.properties.get_float_default("iso_brush_yaw", 0.),
                        );
                    }
                }
                rusterix::isometric::terrain_rectangle_guide(
                    buffer,
                    map,
                    rusterix::isometric::Projection::editor(map, screen_size)
                        .at_height(map.properties.get_float_default("iso_edit_height", 0.)),
                    top_left,
                    bottom_right,
                    [223, 226, 209, 200],
                );
            } else {
                let a = ServerContext::map_grid_to_local(screen_size, top_left, map);
                let b = ServerContext::map_grid_to_local(screen_size, bottom_right, map);
                let min_x = a.x.min(b.x).round() as i32;
                let min_y = a.y.min(b.y).round() as i32;
                let max_x = a.x.max(b.x).round() as i32;
                let max_y = a.y.max(b.y).round() as i32;
                let rect = TheDim::new(
                    min_x,
                    min_y,
                    (max_x - min_x).max(1) + 1,
                    (max_y - min_y).max(1) + 1,
                );
                buffer.draw_rect_outline(&rect, &[255, 255, 255, 255]);
            }
        }

        let id = if !map.selected_linedefs.is_empty() {
            Some(map.selected_linedefs[0])
        } else {
            None
        };
        self.hud.draw(buffer, map, ctx, server_ctx, id, assets);
        self.draw_composition_controls(buffer, map, ctx, server_ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Apply Map Properties" && *state == TheWidgetState::Clicked {
                    let source = crate::utils::get_source(_ui, server_ctx).map(Value::Source);

                    if let Some(source) = source {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let _prev = map.clone();

                            for linedef_id in map.selected_linedefs.clone() {
                                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                    if self.hud.selected_icon_index == 0 {
                                        linedef.properties.set("row1_source", source.clone());
                                    } else if self.hud.selected_icon_index == 1 {
                                        linedef.properties.set("row2_source", source.clone());
                                    } else if self.hud.selected_icon_index == 2 {
                                        linedef.properties.set("row3_source", source.clone());
                                    } else if self.hud.selected_icon_index == 3 {
                                        linedef.properties.set("row4_source", source.clone());
                                    }
                                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                                }
                            }

                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        }
                    }
                } else if id.name == "Remove Map Properties" && *state == TheWidgetState::Clicked {
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        let _prev = map.clone();

                        for linedef_id in map.selected_linedefs.clone() {
                            if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                if self.hud.selected_icon_index == 0 {
                                    linedef
                                        .properties
                                        .set("row1_source", Value::Source(PixelSource::Off));
                                } else if self.hud.selected_icon_index == 1 {
                                    linedef
                                        .properties
                                        .set("row2_source", Value::Source(PixelSource::Off));
                                } else if self.hud.selected_icon_index == 2 {
                                    linedef
                                        .properties
                                        .set("row3_source", Value::Source(PixelSource::Off));
                                } else if self.hud.selected_icon_index == 3 {
                                    linedef
                                        .properties
                                        .set("row4_source", Value::Source(PixelSource::Off));
                                }
                                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            }
                        }

                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_brush_rotation_changes_direction_without_turning_placed_walls() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        let mut wall = rusterix::Sector::new(1, Vec::new());
        wall.properties.set("iso_yaw", Value::Float(0.));
        map.sectors.push(wall);
        map.selected_sectors = vec![1];
        RectTool::rotate_brush(&mut map, 90.);
        assert_eq!(map.properties.get_float_default("iso_brush_yaw", 0.), 90.);
        RectTool::rotate_brush(&mut map, -180.);
        assert_eq!(map.properties.get_float_default("iso_brush_yaw", 0.), 270.);
        assert_eq!(
            map.sectors[0].properties.get_float_default("iso_yaw", 0.),
            0.
        );
    }

    #[test]
    #[ignore = "requires native GPU for editor paint publication"]
    fn iso_rotation_buttons_capture_drag_until_release_without_placing_tiles() {
        // Exercise both HUD layouts and both directions, including small mouse
        // jitter followed by a drag off the button and onto the map.
        for width in [400, 800] {
            for button in 0..2 {
                let mut map = Map::default();
                map.camera = MapCamera::TwoDIso;
                map.grid_size = 40.;
                let mut ui = TheUI::new();
                let mut ctx = TheContext::new(width, 400, 1.);
                let mut view = TheRenderView::new(TheId::named("PolyView"));
                view.set_dim(TheDim::sized(width as i32, 400), &mut ctx);
                ui.canvas.set_widget(view);
                let mut server = ServerContext::default();
                server.pc = ProjectContext::Region(Uuid::new_v4());
                server.editor_view_mode = EditorViewMode::D2;
                server.curr_map_tool_type = MapToolType::Rect;
                server.curr_tile_source = Some(TileSource::SingleTile(Uuid::new_v4()));
                let mut tool = RectTool::new();
                let mut buffer = TheRGBABuffer::new(TheDim::sized(width as i32, 400));
                tool.draw_composition_controls(&mut buffer, &map, &mut ctx, &server);
                let rect = tool.rotation_rects[button].0;
                let click = Vec2::new(rect.x + rect.width / 2, rect.y + rect.height / 2);
                let paint_at =
                    rusterix::isometric::Projection::editor(&map, Vec2::new(width as f32, 400.))
                        .screen(Vec2::new(0.5, 0.5), 0.)
                        .map(|v| v.round() as i32);
                for event in [
                    MapHover(click),
                    MapClicked(click),
                    MapDragged(click + Vec2::new(1, 1)),
                    MapDragged(paint_at),
                    MapUp(paint_at),
                ] {
                    assert!(
                        tool.map_event(event, &mut ui, &mut ctx, &mut map, &mut server)
                            .is_none()
                    );
                }
                let yaw = if button == 0 { 270. } else { 90. };
                assert_eq!(map.properties.get_float_default("iso_brush_yaw", 0.), yaw);
                assert!(map.vertices.is_empty());
                assert!(map.linedefs.is_empty());
                assert!(map.sectors.is_empty(), "A rotation gesture placed a tile");
                assert!(!map.properties.contains("tiles"));
                assert!(!tool.stroke_active);

                // Mouse-up releases capture: the next deliberate click paints
                // normally, with the newly chosen orientation and one undo.
                tool.map_event(MapHover(paint_at), &mut ui, &mut ctx, &mut map, &mut server);
                tool.map_event(
                    MapClicked(paint_at),
                    &mut ui,
                    &mut ctx,
                    &mut map,
                    &mut server,
                );
                assert!(
                    tool.map_event(MapUp(paint_at), &mut ui, &mut ctx, &mut map, &mut server)
                        .is_some()
                );
                assert_eq!(map.sectors.len(), 1);
                assert_eq!(
                    map.sectors[0].properties.get_float_default("iso_yaw", 0.),
                    yaw
                );
            }
        }
    }
    #[test]
    fn smooth_terrain_brush_preserves_rigid_iso_parts_and_explicit_levels() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        map.properties.set("terrain_enabled", Value::Bool(true));
        let mut ground = rusterix::Tile::from_texture(Texture::new(vec![255; 4], 1, 1));
        ground.isometric = Some(rusterix::isometric::SpriteLayout {
            ground: true,
            ..Default::default()
        });
        let mut part = ground.clone();
        part.id = Uuid::new_v4();
        part.isometric.as_mut().unwrap().ground = false;
        let ground_source = PixelSource::TileId(ground.id);
        let part_source = PixelSource::TileId(part.id);
        let mut assets = Assets::default();
        assets.set_tiles([(ground.id, ground), (part.id, part)].into_iter().collect());
        assert!(RectTool::uses_terrain_paint(
            &map,
            Some(&ground_source),
            &assets
        ));
        assert!(!RectTool::uses_terrain_paint(
            &map,
            Some(&part_source),
            &assets
        ));
        map.properties.set("iso_edit_height", Value::Float(1.));
        assert!(!RectTool::uses_terrain_paint(
            &map,
            Some(&ground_source),
            &assets
        ));
        map.properties.set("iso_brush_walkable", Value::Bool(true));
        assert!(RectTool::paint_cell(
            &mut map,
            cell(2., 3.),
            Some(part_source),
            false,
            0
        ));
        assert_eq!(
            map.sectors[0]
                .properties
                .get_float_default("floor_height", 0.),
            1.
        );
        assert!(
            map.sectors[0]
                .properties
                .get_bool_default("iso_walkable", false)
        );
        assert!(!map.properties.contains("tiles"));
        let mut server = ServerContext::default();
        server.editor_view_mode = EditorViewMode::D2;
        server.hover_cursor = Some(Vec2::new(2., 3.));
        assert!(matches!(
            RectTool::composition_target(&map, &server).unwrap().surface,
            shared::material_composition::CompositionSurface::RectTile { .. }
        ));
    }

    fn cell(x: f32, y: f32) -> Option<[Vec2<f32>; 4]> {
        Some([
            Vec2::new(x, y),
            Vec2::new(x, y + 1.),
            Vec2::new(x + 1., y + 1.),
            Vec2::new(x + 1., y),
        ])
    }

    #[test]
    #[ignore = "native editor paint publication and undo use SceneVM's Metal context"]
    fn iso_ui_controls_and_real_paint_strokes_build_stairs_with_undo() {
        let mut p = Project::default();
        let map = &mut p.regions[0].map;
        *map = Map::default();
        map.camera = rusterix::MapCamera::TwoDIso;
        map.grid_size = 40.;
        map.properties.set("terrain_enabled", Value::Bool(true));
        let recipe = Uuid::new_v4();
        p.procedural_appearances.insert(recipe,r#"{"version":1,"id":"step","label":"Stone step","rows":[{"id":"step","label":"Step","tiles":[{"id":"box","op":{"type":"box","size":[1,0.25,1]}},{"id":"lower","op":{"type":"transform","translation":[0,-0.125,0],"rotation":[0,0,0],"scale":[1,1,1]}}]}],"output":"step"}"#.into());
        assert!(shared::isometric::synchronize(&mut p).is_empty());
        assert!(p.tiles.contains_key(&recipe));
        RUSTERIX
            .write()
            .unwrap()
            .set_tiles_for_maps(p.tiles.clone(), false, [&p.regions[0].map]);
        let mut ctx = TheContext::new(1000, 700, 1.);
        let mut ui = TheUI::new();
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::sized(1000, 700), &mut ctx);
        ui.canvas.set_widget(view);
        let mut server = ServerContext::default();
        server.curr_region = p.regions[0].id;
        server.pc = ProjectContext::Region(server.curr_region);
        server.editor_view_mode = EditorViewMode::D2;
        let mut picker = <crate::docks::tiles::TilesDock as Dock>::new();
        let mut picker_ui = TheUI::new();
        picker_ui.canvas = picker.setup(&mut ctx);
        p.set_tile_board_position(rusterix::TileSource::SingleTile(recipe), Vec2::new(2, 0));
        picker_ui.canvas.set_dim(TheDim::sized(800, 300), &mut ctx);
        picker.activate(&mut picker_ui, &mut ctx, &p, &mut server);
        picker_ui.canvas.set_dim(TheDim::sized(800, 300), &mut ctx);
        picker.handle_event(
            &mut TheEvent::Resize,
            &mut picker_ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        picker.handle_event(
            &mut TheEvent::RenderViewClicked(TheId::named("Tiles Dock View 0"), Vec2::new(150, 30)),
            &mut picker_ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(server.curr_tile_id, Some(recipe));
        for (id, value) in [
            ("Iso Placement Walkable", TheValue::Bool(true)),
            ("Iso Placement Stack", TheValue::Bool(true)),
            ("Iso Placement Step", TheValue::Float(0.25)),
        ] {
            assert!(crate::iso_editor::handle_event(
                &TheEvent::ValueChanged(TheId::named(id), value),
                &mut p,
                &server,
                &mut ctx
            ));
        }
        let mut tool = <RectTool as Tool>::new();
        let mut last = None;
        for x in 1..=8 {
            let height = x as f32 * 0.25;
            crate::iso_editor::handle_event(
                &TheEvent::ValueChanged(
                    TheId::named("Iso Placement Height"),
                    TheValue::Float(height),
                ),
                &mut p,
                &server,
                &mut ctx,
            );
            let map = &mut p.regions[0].map;
            let click = rusterix::isometric::Projection::editor(map, Vec2::new(1000., 700.))
                .screen(Vec2::new(x as f32 + 0.5, 0.5), height)
                .map(|v| v.round() as i32);
            tool.map_event(MapHover(click), &mut ui, &mut ctx, map, &mut server);
            tool.map_event(MapClicked(click), &mut ui, &mut ctx, map, &mut server);
            last = tool.map_event(MapUp(click), &mut ui, &mut ctx, map, &mut server);
            assert!(last.is_some(), "UI stroke {x} did not create undo");
        }
        assert_eq!(p.regions[0].map.sectors.len(), 36);
        let mini = p.regions[0].map.as_mini(&Default::default());
        let (top, blocked) = mini.elevation.move_distance(
            Vec3::new(0.5, 0., 0.5),
            Vec2::new(8., 0.),
            0.22,
            0.35,
            0.5,
        );
        assert!(!blocked);
        assert_eq!(top.y, 2.);
        let undo = last.unwrap();
        undo.undo(&mut p, &mut ui, &mut ctx, &mut server);
        assert_eq!(p.regions[0].map.sectors.len(), 28);
        undo.redo(&mut p, &mut ui, &mut ctx, &mut server);
        assert_eq!(p.regions[0].map.sectors.len(), 36);
        assert!(!p.regions[0].map.properties.contains("tiles"));

        // The same UI paints material cells at the visible hill height while
        // leaving the construction parts intact, with one undo for the stroke.
        let ground = rusterix::Tile::from_texture(Texture::new(vec![80, 110, 60, 255], 1, 1));
        let ground_id = ground.id;
        p.tiles.insert(ground_id, ground);
        let map = &mut p.regions[0].map;
        map.properties.set("iso_edit_height", Value::Float(0.));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        let peak = map.add_vertex_at(2.5, 6.5);
        let peak = map.find_vertex_mut(peak).unwrap();
        peak.z = 1.2;
        peak.properties.set("terrain_control", Value::Bool(true));
        peak.properties.set("smoothness", Value::Float(1.));
        map.changed = map.changed.wrapping_add(1);
        RUSTERIX
            .write()
            .unwrap()
            .set_tiles_for_maps(p.tiles.clone(), false, [&*map]);
        server.curr_tile_id = Some(ground_id);
        server.curr_tile_source = Some(TileSource::SingleTile(ground_id));
        let click =
            ServerContext::map_grid_to_local(Vec2::new(1000., 700.), Vec2::new(2.5, 6.5), map)
                .map(|v| v.round() as i32);
        tool.map_event(MapHover(click), &mut ui, &mut ctx, map, &mut server);
        tool.map_event(MapClicked(click), &mut ui, &mut ctx, map, &mut server);
        let undo = tool
            .map_event(MapUp(click), &mut ui, &mut ctx, map, &mut server)
            .unwrap();
        assert_eq!(map.sectors.len(), 36);
        let Some(Value::TileOverrides(tiles)) = map.properties.get("tiles") else {
            panic!("terrain stroke was not published");
        };
        assert_eq!(tiles.get(&(2, 6)), Some(&PixelSource::TileId(ground_id)));
        undo.undo(&mut p, &mut ui, &mut ctx, &mut server);
        assert!(!p.regions[0].map.properties.contains("tiles"));
        undo.redo(&mut p, &mut ui, &mut ctx, &mut server);
        assert!(p.regions[0].map.properties.contains("tiles"));
        if let Ok(path) = std::env::var("ELDIRON_ISO_AUTHORING_OUTPUT") {
            std::fs::write(path, shared::project_io::encode_project(&p).unwrap()).unwrap();
        }
    }

    #[test]
    fn stroke_cell_paint_preserves_overwrite_stacking_and_erase() {
        let mut map = Map::new();
        let a = PixelSource::TileId(Uuid::new_v4());
        let b = PixelSource::TileId(Uuid::new_v4());
        assert!(RectTool::paint_cell(
            &mut map,
            cell(0., 0.),
            Some(a.clone()),
            false,
            0
        ));
        assert_eq!(map.sectors.len(), 1);
        let counts = (map.vertices.len(), map.linedefs.len());
        assert!(!RectTool::paint_cell(
            &mut map,
            cell(0., 0.),
            Some(a.clone()),
            false,
            0
        ));
        assert!(RectTool::paint_cell(
            &mut map,
            cell(0., 0.),
            Some(b.clone()),
            false,
            0
        ));
        assert_eq!(map.sectors[0].properties.get_default_source(), Some(&b));
        assert_eq!(counts, (map.vertices.len(), map.linedefs.len()));
        assert!(!RectTool::paint_cell(
            &mut map,
            cell(0., 0.),
            Some(a.clone()),
            false,
            1
        ));
        // Existing stacked tiles have distinct rings; deleting their top layer
        // must preserve the cell underneath.
        let base = map.sectors[0].linedefs.clone();
        let mut ring = Vec::new();
        for (i, id) in base.iter().enumerate() {
            let line = map.find_linedef(*id).unwrap();
            let copy = rusterix::Linedef::new(100 + i as u32, line.start_vertex, line.end_vertex);
            ring.push(copy.id);
            map.linedefs.push(copy);
        }
        let mut top = rusterix::Sector::new(100, ring);
        top.layer = Some(2);
        top.properties.set("source", Value::Source(a.clone()));
        map.sectors.push(top);
        map.sanitize();
        assert_eq!(map.sectors.len(), 2);
        assert!(RectTool::paint_cell(&mut map, cell(0., 0.), None, true, 0));
        assert_eq!(map.sectors.len(), 1);
        assert_eq!(map.sectors[0].properties.get_default_source(), Some(&b));
        assert!(!RectTool::paint_cell(
            &mut map,
            cell(0., 0.),
            None,
            false,
            0
        ));
        assert!(RectTool::paint_cell(&mut map, cell(0., 0.), None, true, 0));
        assert!(map.sectors.is_empty());
    }

    #[test]
    #[ignore = "requires ELDIRON_BENCH_PROJECT_PATH"]
    fn benchmark_large_map_rect_paint() {
        let path = std::env::var("ELDIRON_BENCH_PROJECT_PATH").unwrap();
        let project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
        let mut map = project.regions.into_iter().next().unwrap().map;
        map.sanitize();
        let started = Instant::now();
        let snapshot = map.clone();
        println!(
            "Rect stroke snapshot={:.2}ms",
            started.elapsed().as_secs_f64() * 1000.
        );
        let source = PixelSource::TileId(Uuid::new_v4());
        let started = Instant::now();
        for x in 0..16 {
            assert!(RectTool::paint_cell(
                &mut map,
                cell(x as f32, 0.),
                Some(source.clone()),
                false,
                0
            ));
        }
        println!(
            "Rect cell mutation mean={:.2}ms (excludes scene rebuild and stroke snapshots)",
            started.elapsed().as_secs_f64() * 1000. / 16.
        );
        assert_eq!(snapshot.sectors.len(), map.sectors.len());
        for x in 0..16 {
            assert!(!RectTool::paint_cell(
                &mut map,
                cell(x as f32, 0.),
                Some(source.clone()),
                false,
                0
            ));
        }
    }

    #[test]
    fn d2_paint_cells_select_their_scene_chunk_across_boundaries() {
        let mut chunks = FxHashSet::default();
        for cell in [
            Vec2::new(0, 0),
            Vec2::new(31, 31),
            Vec2::new(32, 32),
            Vec2::new(-1, -1),
            Vec2::new(-32, -32),
            Vec2::new(-33, -33),
        ] {
            RectTool::add_2d_cell_dirty_chunk(cell, &mut chunks);
        }

        assert_eq!(chunks.len(), 4);
        assert!(chunks.contains(&(0, 0)));
        assert!(chunks.contains(&(32, 32)));
        assert!(chunks.contains(&(-32, -32)));
        assert!(chunks.contains(&(-64, -64)));

        let mut custom_chunks = FxHashSet::default();
        RectTool::add_2d_cell_dirty_chunk_with_size(Vec2::new(17, -1), 8, &mut custom_chunks);
        assert!(custom_chunks.contains(&(16, -8)));
    }

    #[test]
    fn explicit_recipe_uv_faces_use_stable_paint_cells_for_rect_hits() {
        let mut object = rusterix::GeometryObject::box_from_bounds(
            "Explicit recipe UV wall",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 1.0),
        );
        object.faces.truncate(1);
        let face = &mut object.faces[0];
        face.auto_uv = false;
        face.uvs = face.paint_uvs.iter().map(|uv| *uv / 4.0).collect();
        let object_id = object.id;
        let mut map = Map::default();
        map.geometry_objects.push(object);

        assert_eq!(
            RectTool::geometry_face_target_at_world(&map, object_id, Vec3::new(0.5, 0.5, 0.0),),
            Some((0, (0, 0)))
        );
        assert_eq!(
            RectTool::geometry_face_target_at_world(&map, object_id, Vec3::new(1.5, 0.5, 0.0),),
            Some((0, (1, 0)))
        );
    }
    #[test]
    fn compose_click_selects_one_2d_painted_tile_and_drag_does_not_paint() {
        use shared::material_composition::CompositionSurface;
        let mut map = Map::default();
        let source = PixelSource::TileId(Uuid::new_v4());
        RectTool::paint_cell(&mut map, cell(0., 0.), Some(source.clone()), false, 0);
        RectTool::paint_cell(&mut map, cell(1., 0.), Some(source.clone()), false, 0);
        let first = RectTool::rect_sector_at(&map, [0, 0]).unwrap();
        let before = serde_json::to_string(&map.sectors).unwrap();
        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(400, 400, 1.);
        let (tx, rx) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(tx);
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::new(0, 0, 400, 400), &mut ctx);
        ui.canvas.set_widget(view);
        let mut server = ServerContext::default();
        server.pc = ProjectContext::Region(Uuid::new_v4());
        server.curr_map_tool_type = MapToolType::Rect;
        server.rect_compose_mode = true;
        server.editor_view_mode = EditorViewMode::D2;
        let mut tool = RectTool::new();
        let screen = |p| {
            let p = ServerContext::map_grid_to_local(Vec2::new(400., 400.), p, &map);
            Vec2::new(p.x.round() as i32, p.y.round() as i32)
        };
        let first_click = screen(Vec2::new(0.5, 0.5));
        let second_click = screen(Vec2::new(1.5, 0.5));
        assert!(
            tool.map_event(
                MapClicked(first_click),
                &mut ui,
                &mut ctx,
                &mut map,
                &mut server
            )
            .is_none()
        );
        let binding = server.rect_composition_target.clone().unwrap();
        assert_eq!(
            binding.surface,
            CompositionSurface::RectTile { sector: first }
        );
        assert!(rx.try_iter().any(|event|matches!(event,TheEvent::Custom(id,TheValue::Text(_)) if id.name==crate::procedural_material::map_composition::OPEN)));
        tool.map_event(
            MapDragged(second_click),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        tool.map_event(
            MapUp(second_click),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert_eq!(server.rect_composition_target, Some(binding));
        assert_eq!(serde_json::to_string(&map.sectors).unwrap(), before);
        assert!(!tool.stroke_active);
        tool.map_event(
            MapClicked(second_click),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert_ne!(
            server.rect_composition_target.as_ref().unwrap().surface,
            CompositionSurface::RectTile { sector: first }
        );
        // A mode-button drag must never leak through into Paint mode.
        tool.hud_control_active = true;
        server.rect_compose_mode = false;
        tool.map_event(
            MapDragged(first_click),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        tool.map_event(MapUp(first_click), &mut ui, &mut ctx, &mut map, &mut server);
        assert!(!tool.hud_control_active);
        assert_eq!(serde_json::to_string(&map.sectors).unwrap(), before);
    }
    #[test]
    fn composition_targets_use_rect_paint_3d_cells_and_terrain_coordinates() {
        use shared::material_composition::CompositionSurface;
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_("Wall", Vec3::zero(), Vec3::new(4., 3., 1.));
        let object_id = object.id;
        map.geometry_objects.push(object);
        let mut server = ServerContext::default();
        server.editor_view_mode = EditorViewMode::FirstP;
        server.rect_geometry_face_3d = Some((object_id, 0));
        server.rect_tile_id_3d = (-2, 1);
        assert_eq!(
            RectTool::composition_target(&map, &server).unwrap().surface,
            CompositionSurface::GeometryTile {
                object: object_id,
                face: 0,
                cell: [-2, 1]
            }
        );
        server.rect_tile_id_3d = (-1, 1);
        assert_eq!(
            RectTool::composition_target(&map, &server).unwrap().surface,
            CompositionSurface::GeometryTile {
                object: object_id,
                face: 0,
                cell: [-1, 1]
            }
        );
        map.sectors.push(rusterix::Sector::new(9, vec![]));
        server.rect_geometry_face_3d = None;
        server.rect_sector_id_3d = Some(9);
        server.rect_tile_id_3d = (2, 3);
        assert_eq!(
            RectTool::composition_target(&map, &server).unwrap().surface,
            CompositionSurface::SectorTile {
                sector: 9,
                cell: [2, 3]
            }
        );
        map.properties.set("terrain_enabled", Value::Bool(true));
        server.editor_view_mode = EditorViewMode::D2;
        server.hover_cursor = Some(Vec2::new(-2., 5.));
        let d2 = RectTool::composition_target(&map, &server).unwrap();
        server.editor_view_mode = EditorViewMode::FirstP;
        server.rect_terrain_id = Some((-2, 5));
        assert_eq!(d2, RectTool::composition_target(&map, &server).unwrap());
    }
}

impl RectTool {
    fn terrain_edit_plane(map: &Map) -> bool {
        map.properties.get_bool_default("terrain_enabled", false)
            && (map.camera != MapCamera::TwoDIso
                || map
                    .properties
                    .get_float_default("iso_edit_height", 0.)
                    .abs()
                    < 0.001)
    }

    /// Ground sources paint the shared surface. Rigid iso meshes and explicit
    /// construction levels retain the ordinary placement/stacking workflow.
    fn uses_terrain_paint(map: &Map, source: Option<&PixelSource>, assets: &Assets) -> bool {
        if !Self::terrain_edit_plane(map) {
            return false;
        }
        if map.camera != MapCamera::TwoDIso {
            return true;
        }
        source
            .and_then(|s| s.render_tile_id(assets))
            .and_then(|id| assets.tiles.get(&id))
            .and_then(|t| t.isometric.as_ref())
            .is_none_or(|layout| layout.ground && !layout.bitmap)
    }

    /// Rect composition uses exactly the same painted-cell coordinates as paint
    /// and the eyedropper; hover never changes the clicked editing target.
    fn composition_target(
        map: &Map,
        server: &ServerContext,
    ) -> Option<shared::material_composition::MapMaterialBinding> {
        use shared::material_composition::{CompositionSurface, MapMaterialBinding};
        let surface = if server.editor_view_mode == EditorViewMode::D2 {
            let cursor = server.hover_cursor?;
            let cell = [cursor.x.floor() as i32, cursor.y.floor() as i32];
            if Self::terrain_edit_plane(map) {
                CompositionSurface::TerrainTile { cell }
            } else {
                CompositionSurface::RectTile {
                    sector: Self::rect_sector_at(map, cell)?,
                }
            }
        } else if let Some((x, y)) = server.rect_terrain_id {
            CompositionSurface::TerrainTile { cell: [x, y] }
        } else if let Some((object, face)) = server.rect_geometry_face_3d {
            map.geometry_objects
                .iter()
                .find(|o| o.id == object)?
                .faces
                .get(face)?;
            CompositionSurface::GeometryTile {
                object,
                face,
                cell: [server.rect_tile_id_3d.0, server.rect_tile_id_3d.1],
            }
        } else if let Some(sector) = server.rect_sector_id_3d {
            map.find_sector(sector)?;
            CompositionSurface::SectorTile {
                sector,
                cell: [server.rect_tile_id_3d.0, server.rect_tile_id_3d.1],
            }
        } else {
            return None;
        };
        Some(MapMaterialBinding {
            map: map.id,
            surface,
        })
    }
    fn rect_sector_at(map: &Map, cell: [i32; 2]) -> Option<u32> {
        let (x, y) = (cell[0] as f32, cell[1] as f32);
        if map.camera == rusterix::MapCamera::TwoDIso {
            return map
                .sectors
                .iter()
                .rev()
                .find(|s| {
                    let b = s.bounding_box(map);
                    (b.min - Vec2::new(x, y)).magnitude_squared() < 0.0001
                        && (b.max - b.min - 1.).magnitude_squared() < 0.0001
                        && (s.properties.get_float_default("floor_height", 0.)
                            - map.properties.get_float_default("iso_edit_height", 0.))
                        .abs()
                            < 0.001
                })
                .map(|s| s.id);
        }
        let vertices = [
            map.find_vertex_at(x, y)?,
            map.find_vertex_at(x, y + 1.)?,
            map.find_vertex_at(x + 1., y + 1.)?,
            map.find_vertex_at(x + 1., y)?,
        ];
        map.find_sectors_with_vertex_indices(&vertices)
            .last()
            .copied()
    }
    fn rotate_brush(map: &mut Map, delta: f32) {
        let yaw = (map.properties.get_float_default("iso_brush_yaw", 0.) + delta).rem_euclid(360.);
        map.properties.set("iso_brush_yaw", Value::Float(yaw));
    }
    fn draw_composition_controls(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &Map,
        ctx: &mut TheContext,
        server: &ServerContext,
    ) {
        self.composition_mode_rects.clear();
        self.rotation_rects.clear();
        if buffer.dim().width < 188 || buffer.dim().height < 54 || !server.pc.is_region() {
            return;
        }
        let stride = buffer.stride();
        for (index, (x, width, label)) in [(8, 70, "Paint"), (80, 100, "Compose")]
            .into_iter()
            .enumerate()
        {
            let rect = TheDim::rect(x, 26, width, 22);
            let selected = server.rect_compose_mode == (index == 1);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &if selected {
                    [68, 68, 74, 245]
                } else {
                    [34, 34, 36, 240]
                },
            );
            if selected {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(x as usize + 3, 46, width as usize - 6, 2),
                    stride,
                    &[210, 176, 92, 255],
                );
            }
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                label,
                TheFontSettings {
                    size: 12.,
                    ..Default::default()
                },
                &[230, 230, 232, 255],
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
            self.composition_mode_rects.push(rect);
        }
        if map.camera == MapCamera::TwoDIso {
            let y = if buffer.dim().width >= 490 { 26 } else { 52 };
            let left = if y == 26 { 188 } else { 8 };
            for (i, (label, delta)) in [("Rotate -90 (,)", -90.), ("Rotate +90 (.)", 90.)]
                .into_iter()
                .enumerate()
            {
                let rect = TheDim::rect(left + i as i32 * 116, y, 112, 22);
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &[34, 34, 36, 240],
                );
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    label,
                    TheFontSettings {
                        size: 12.,
                        ..Default::default()
                    },
                    &[230, 230, 232, 255],
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
                self.rotation_rects.push((rect, delta));
            }
            let rect = TheDim::rect(left + 232, y, 64, 22);
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &format!(
                    "{:.0} deg",
                    map.properties.get_float_default("iso_brush_yaw", 0.)
                ),
                TheFontSettings {
                    size: 12.,
                    ..Default::default()
                },
                &[230, 230, 232, 255],
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        }
        if server.editor_view_mode == EditorViewMode::D2 && server.rect_compose_mode {
            use shared::material_composition::CompositionSurface;
            let bounds = server
                .rect_composition_target
                .as_ref()
                .filter(|b| b.map == map.id)
                .and_then(|binding| match binding.surface {
                    CompositionSurface::RectTile { sector } => map.find_sector(sector).map(|s| {
                        let b = s.bounding_box(map);
                        (b.min, b.max)
                    }),
                    CompositionSurface::TerrainTile { cell } => {
                        let p = Vec2::new(cell[0] as f32, cell[1] as f32);
                        Some((p, p + 1.))
                    }
                    _ => None,
                });
            if let Some((a, b)) = bounds {
                let size = Vec2::new(buffer.dim().width as f32, buffer.dim().height as f32);
                if map.camera == rusterix::MapCamera::TwoDIso {
                    rusterix::isometric::terrain_rectangle_guide(
                        buffer,
                        map,
                        rusterix::isometric::Projection::editor(map, size),
                        a,
                        b,
                        [210, 176, 92, 255],
                    );
                } else {
                    let a = ServerContext::map_grid_to_local(size, a, map);
                    let b = ServerContext::map_grid_to_local(size, b, map);
                    buffer.draw_rect_outline(
                        &TheDim::new(
                            a.x.min(b.x).round() as i32,
                            a.y.min(b.y).round() as i32,
                            (b.x - a.x).abs().round() as i32 + 1,
                            (b.y - a.y).abs().round() as i32 + 1,
                        ),
                        &[210, 176, 92, 255],
                    );
                }
            }
        }
    }
    /// Add a tile at the current hover position
    // The stroke owns its undo snapshots; individual cells only report changes.
    fn paint_cell(
        map: &mut Map,
        hovered_vertices: Option<[Vec2<f32>; 4]>,
        source: Option<PixelSource>,
        erase: bool,
        mode: i32,
    ) -> bool {
        if map.camera == rusterix::MapCamera::TwoDIso {
            return hovered_vertices.is_some_and(|v| {
                let rusterix = RUSTERIX.read().unwrap();
                crate::iso_editor::paint_cell_with_tiles(
                    map,
                    v,
                    source,
                    erase,
                    mode,
                    &rusterix.assets.tiles,
                )
            });
        }
        let mut changed = false;
        // let size = 1.0 / map.subdivisions;

        if let Some(vertices) = hovered_vertices {
            let iso = map.camera == rusterix::MapCamera::TwoDIso;
            let yaw = map.properties.get_float_default("iso_brush_yaw", 0.);
            let mut add_it = true;
            let mut layer: u8 = 0;

            if erase {
                // Delete the top tile at the given position if shift is pressed
                if let Some(ev0) = map.find_vertex_at(vertices[0].x, vertices[0].y) {
                    if let Some(ev1) = map.find_vertex_at(vertices[1].x, vertices[1].y) {
                        if let Some(ev2) = map.find_vertex_at(vertices[2].x, vertices[2].y) {
                            if let Some(ev3) = map.find_vertex_at(vertices[3].x, vertices[3].y) {
                                let sectors =
                                    map.find_sectors_with_vertex_indices(&[ev0, ev1, ev2, ev3]);

                                if let Some(sector_id) = sectors.last() {
                                    let mut lines = vec![];
                                    if let Some(s) = map.find_sector(*sector_id) {
                                        lines = s.linedefs.clone();
                                    }
                                    map.delete_elements(&[], &lines, &[*sector_id]);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            } else if let Some(source) = source {
                // Add mode
                // Check if tile already exists with same source
                if let Some(ev0) = map.find_vertex_at(vertices[0].x, vertices[0].y) {
                    if let Some(ev1) = map.find_vertex_at(vertices[1].x, vertices[1].y) {
                        if let Some(ev2) = map.find_vertex_at(vertices[2].x, vertices[2].y) {
                            if let Some(ev3) = map.find_vertex_at(vertices[3].x, vertices[3].y) {
                                let sectors =
                                    map.find_sectors_with_vertex_indices(&[ev0, ev1, ev2, ev3]);

                                for sector_id in sectors {
                                    if let Some(sector) = map.find_sector_mut(sector_id) {
                                        if let Some(sector_floor_source) =
                                            sector.properties.get_default_source()
                                        {
                                            // Assign id to the higher current layer id (+1).
                                            if let Some(l) = &sector.layer {
                                                if *l > layer {
                                                    layer = *l;
                                                }
                                            }

                                            if source == *sector_floor_source {
                                                if iso
                                                    && sector
                                                        .properties
                                                        .get_float_default("iso_yaw", 0.)
                                                        != yaw
                                                {
                                                    sector
                                                        .properties
                                                        .set("iso_yaw", Value::Float(yaw));
                                                    changed = true;
                                                }
                                                // A tile with the same floor_source exists, do not add.
                                                add_it = false;
                                            } else if mode == 0 {
                                                // In overlay mode we just overwrite the source
                                                sector
                                                    .properties
                                                    .set("source", Value::Source(source.clone()));
                                                if iso {
                                                    sector
                                                        .properties
                                                        .set("iso_yaw", Value::Float(yaw));
                                                }
                                                changed = true;

                                                add_it = false;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if add_it {
                    let v0 = map.add_vertex_at(vertices[0].x, vertices[0].y);
                    let v1 = map.add_vertex_at(vertices[1].x, vertices[1].y);
                    let v2 = map.add_vertex_at(vertices[2].x, vertices[2].y);
                    let v3 = map.add_vertex_at(vertices[3].x, vertices[3].y);

                    map.possible_polygon = vec![];
                    let _ = map.create_linedef_manual(v0, v1);
                    let _ = map.create_linedef_manual(v1, v2);
                    let _ = map.create_linedef_manual(v2, v3);
                    let _ = map.create_linedef_manual(v3, v0);
                    let sid = map.close_polygon_manual();

                    if let Some(sector_id) = sid {
                        // Add the info for correct box rendering
                        // if let Some(l) = map.find_linedef_mut(l0.0) {
                        //     l.properties.set("row1_source", source.clone());
                        //     l.properties.set("wall_height", Value::Float(size));
                        // }
                        // if let Some(l) = map.find_linedef_mut(l1.0) {
                        //     l.properties.set("row1_source", source.clone());
                        //     l.properties.set("wall_height", Value::Float(size));
                        // }
                        // if let Some(l) = map.find_linedef_mut(l2.0) {
                        //     l.properties.set("row1_source", source.clone());
                        //     l.properties.set("wall_height", Value::Float(size));
                        // }
                        // if let Some(l) = map.find_linedef_mut(id.0) {
                        //     l.properties.set("row1_source", source.clone());
                        //     l.properties.set("wall_height", Value::Float(size));
                        // }

                        if let Some(sector) = map.find_sector_mut(sector_id) {
                            sector.properties.set("rect", Value::Bool(true));

                            sector.properties.set("source", Value::Source(source));
                            if iso {
                                sector.properties.set("iso_yaw", Value::Float(yaw));
                            }
                            sector.layer = Some(layer + 1);
                        }

                        changed = true;

                        map.selected_vertices.clear();
                        map.selected_linedefs.clear();
                        map.selected_sectors = vec![sector_id];
                    }
                }
            }
        }

        changed
    }

    fn encode_tile_source(source: TileSource) -> TheValue {
        match source {
            TileSource::SingleTile(id) => {
                TheValue::List(vec![TheValue::Text("single".to_string()), TheValue::Id(id)])
            }
        }
    }

    fn tile_source_from_pixel_source(source: &PixelSource) -> Option<TileSource> {
        match source {
            PixelSource::TileId(id) => Some(TileSource::SingleTile(*id)),
            PixelSource::ProceduralTile(id) => Some(TileSource::SingleTile(*id)),
            _ => None,
        }
    }

    fn sample_terrain_source(map: &Map, key: (i32, i32)) -> Option<TileSource> {
        if let Some(Value::TileOverrides(tiles)) = map.properties.get("tiles")
            && let Some(source) = tiles.get(&key)
            && let Some(source) = Self::tile_source_from_pixel_source(source)
        {
            return Some(source);
        }

        if let Some(Value::BlendOverrides(blend_tiles)) = map.properties.get("blend_tiles")
            && let Some((_, source)) = blend_tiles.get(&key)
        {
            return Self::tile_source_from_pixel_source(source);
        }

        map.properties
            .get("default_terrain_tile")
            .and_then(|value| {
                if let Value::Source(source) = value {
                    Self::tile_source_from_pixel_source(source)
                } else {
                    None
                }
            })
    }

    fn sample_sector_source_at_cell(map: &Map, cell: Vec2<i32>) -> Option<TileSource> {
        let x = cell.x as f32;
        let y = cell.y as f32;

        let exact_sector = (|| {
            let ev0 = map.find_vertex_at(x, y)?;
            let ev1 = map.find_vertex_at(x, y + 1.0)?;
            let ev2 = map.find_vertex_at(x + 1.0, y + 1.0)?;
            let ev3 = map.find_vertex_at(x + 1.0, y)?;
            let sectors = map.find_sectors_with_vertex_indices(&[ev0, ev1, ev2, ev3]);
            sectors.last().copied()
        })();

        if let Some(sector_id) = exact_sector
            && let Some(sector) = map.find_sector(sector_id)
            && let Some(source) = sector.properties.get_default_source()
            && let Some(source) = Self::tile_source_from_pixel_source(source)
        {
            return Some(source);
        }

        map.find_sector_at(Vec2::new(x + 0.5, y + 0.5))
            .and_then(|sector| sector.properties.get_default_source())
            .and_then(Self::tile_source_from_pixel_source)
    }

    fn sample_sector_override_source(
        map: &Map,
        sector_id: u32,
        key: (i32, i32),
    ) -> Option<TileSource> {
        let sector = map.find_sector(sector_id)?;

        if let Some(Value::TileOverrides(tiles)) = sector.properties.get("tiles")
            && let Some(source) = tiles.get(&key)
            && let Some(source) = Self::tile_source_from_pixel_source(source)
        {
            return Some(source);
        }

        if let Some(Value::BlendOverrides(blend_tiles)) = sector.properties.get("blend_tiles")
            && let Some((_, source)) = blend_tiles.get(&key)
            && let Some(source) = Self::tile_source_from_pixel_source(source)
        {
            return Some(source);
        }

        sector
            .properties
            .get_default_source()
            .and_then(Self::tile_source_from_pixel_source)
    }

    fn sample_source_at_current_target(
        map: &Map,
        server_ctx: &ServerContext,
    ) -> Option<TileSource> {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            if Self::terrain_edit_plane(map) {
                return server_ctx
                    .rect_terrain_id
                    .and_then(|key| Self::sample_terrain_source(map, key));
            }

            return server_ctx.hover_cursor.and_then(|cp| {
                let cell = Vec2::new(cp.x.floor() as i32, cp.y.floor() as i32);
                Self::sample_sector_source_at_cell(map, cell)
            });
        }

        if let Some(key) = server_ctx.rect_terrain_id {
            return Self::sample_terrain_source(map, key);
        }

        if let Some(sector_id) = server_ctx.rect_sector_id_3d {
            return Self::sample_sector_override_source(map, sector_id, server_ctx.rect_tile_id_3d);
        }

        if let Some((object_id, face_index)) = server_ctx.rect_geometry_face_3d {
            let object = map
                .geometry_objects
                .iter()
                .find(|object| object.id == object_id)?;
            let face = object.faces.get(face_index)?;
            if let Some(source) = face.tiles.get(&server_ctx.rect_tile_id_3d) {
                return Self::tile_source_from_pixel_source(source);
            }
            return face
                .tile
                .as_ref()
                .and_then(Self::tile_source_from_pixel_source);
        }

        None
    }

    fn geometry_object_local_from_world(
        object: &rusterix::GeometryObject,
        world: Vec3<f32>,
    ) -> Vec3<f32> {
        let m = &object.transform;
        let sx = if m[0][0].abs() > 1e-6 { m[0][0] } else { 1.0 };
        let sy = if m[1][1].abs() > 1e-6 { m[1][1] } else { 1.0 };
        let sz = if m[2][2].abs() > 1e-6 { m[2][2] } else { 1.0 };
        Vec3::new(
            (world.x - m[3][0]) / sx,
            (world.y - m[3][1]) / sy,
            (world.z - m[3][2]) / sz,
        )
    }

    fn geometry_face_uv_basis(points: &[Vec3<f32>]) -> Option<Vec3<f32>> {
        if points.len() < 3 {
            return None;
        }
        let mut normal = Vec3::<f32>::zero();
        for index in 1..points.len() - 1 {
            normal += (points[index] - points[0]).cross(points[index + 1] - points[0]);
        }
        (normal.magnitude() > 1e-6).then_some(normal)
    }

    fn geometry_point_auto_uv(point: Vec3<f32>, normal: Vec3<f32>) -> Vec2<f32> {
        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        if abs.y >= abs.x && abs.y >= abs.z {
            Vec2::new(point.x, point.z)
        } else if abs.x >= abs.z {
            Vec2::new(point.z, point.y)
        } else {
            Vec2::new(point.x, point.y)
        }
    }

    fn barycentric_2d(
        p: Vec2<f32>,
        a: Vec2<f32>,
        b: Vec2<f32>,
        c: Vec2<f32>,
    ) -> Option<(f32, f32, f32)> {
        let v0 = b - a;
        let v1 = c - a;
        let v2 = p - a;
        let d00 = v0.dot(v0);
        let d01 = v0.dot(v1);
        let d11 = v1.dot(v1);
        let d20 = v2.dot(v0);
        let d21 = v2.dot(v1);
        let denom = d00 * d11 - d01 * d01;
        if denom.abs() <= 1e-6 {
            return None;
        }
        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;
        (u >= -1e-4 && v >= -1e-4 && w >= -1e-4).then_some((u, v, w))
    }

    fn uv_inside_face(uv: Vec2<f32>, face_uvs: &[Vec2<f32>]) -> bool {
        if face_uvs.len() < 3 {
            return false;
        }
        (1..face_uvs.len() - 1).any(|index| {
            Self::barycentric_2d(uv, face_uvs[0], face_uvs[index], face_uvs[index + 1]).is_some()
        })
    }

    fn interpolate_face_uv(
        uv: Vec2<f32>,
        face_uvs: &[Vec2<f32>],
        target_uvs: &[Vec2<f32>],
    ) -> Option<Vec2<f32>> {
        if face_uvs.len() < 3 || face_uvs.len() != target_uvs.len() {
            return None;
        }
        for index in 1..face_uvs.len() - 1 {
            let Some((u, v, w)) =
                Self::barycentric_2d(uv, face_uvs[0], face_uvs[index], face_uvs[index + 1])
            else {
                continue;
            };
            return Some(target_uvs[0] * u + target_uvs[index] * v + target_uvs[index + 1] * w);
        }
        None
    }

    fn geometry_face_target_at_world(
        map: &Map,
        object_id: Uuid,
        world_hit: Vec3<f32>,
    ) -> Option<(usize, (i32, i32))> {
        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        let local_hit = Self::geometry_object_local_from_world(object, world_hit);

        let mut best: Option<(usize, (i32, i32), f32)> = None;
        for (face_index, face) in object.faces.iter().enumerate() {
            if face.indices.len() < 3 {
                continue;
            }
            let points = face
                .indices
                .iter()
                .filter_map(|vertex_index| object.vertices.get(*vertex_index).copied())
                .collect::<Vec<_>>();
            if points.len() != face.indices.len() {
                continue;
            }
            let Some(normal) = Self::geometry_face_uv_basis(&points) else {
                continue;
            };
            let normal_unit = normal / normal.magnitude();
            let plane_dist = (local_hit - points[0]).dot(normal_unit).abs();
            let projected_uvs = points
                .iter()
                .map(|point| Self::geometry_point_auto_uv(*point, normal))
                .collect::<Vec<_>>();
            let projected_hit = Self::geometry_point_auto_uv(local_hit, normal);
            if !Self::uv_inside_face(projected_hit, &projected_uvs) {
                continue;
            }
            let paint_uvs = if face.paint_uvs.len() == points.len() {
                face.paint_uvs.clone()
            } else {
                projected_uvs.clone()
            };
            let Some(hit_uv) = Self::interpolate_face_uv(projected_hit, &projected_uvs, &paint_uvs)
            else {
                continue;
            };
            let min_uv = paint_uvs
                .iter()
                .fold(Vec2::broadcast(f32::INFINITY), |acc, uv| {
                    Vec2::new(acc.x.min(uv.x), acc.y.min(uv.y))
                });
            let tile = (
                (hit_uv.x - min_uv.x).floor() as i32,
                (hit_uv.y - min_uv.y).floor() as i32,
            );
            if best
                .as_ref()
                .map(|(_, _, best_dist)| plane_dist < *best_dist)
                .unwrap_or(true)
            {
                best = Some((face_index, tile, plane_dist));
            }
        }

        best.map(|(face_index, tile, _)| (face_index, tile))
    }

    fn begin_stroke_if_needed(&mut self, map: &Map) {
        if !self.stroke_active {
            self.stroke_active = true;
            self.stroke_changed = false;
            self.stroke_prev_map = Some(map.clone());
            self.stroke_work_map = Some(map.clone());
            self.processed.clear();
            self.last_live_scene_sync_at = None;
            self.stroke_dirty_chunks.clear();
        }
    }

    fn add_bbox_dirty_chunks(bbox: rusterix::BBox, chunks: &mut Vec<(i32, i32)>) {
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
                chunks.push((cx * chunk_size, cy * chunk_size));
            }
        }
    }

    fn add_surface_tile_dirty_chunks(
        work_map: &Map,
        sector_id: u32,
        tile: (i32, i32),
        chunks: &mut Vec<(i32, i32)>,
    ) -> bool {
        let mut added = false;
        for surface in work_map
            .surfaces
            .values()
            .filter(|surface| surface.sector_id == sector_id)
        {
            let points = surface.tile_outline_world_local(tile, work_map);
            if points.is_empty() {
                continue;
            }

            let first = Vec2::new(points[0].x, points[0].z);
            let mut bbox = rusterix::BBox::new(first, first);
            for p in points.iter().skip(1) {
                let p = Vec2::new(p.x, p.z);
                bbox.min.x = bbox.min.x.min(p.x);
                bbox.min.y = bbox.min.y.min(p.y);
                bbox.max.x = bbox.max.x.max(p.x);
                bbox.max.y = bbox.max.y.max(p.y);
            }
            bbox.expand(Vec2::broadcast(0.05));
            Self::add_bbox_dirty_chunks(bbox, chunks);
            added = true;
        }
        added
    }

    fn live_stroke_dirty_chunks(work_map: &Map, server_ctx: &ServerContext) -> Vec<(i32, i32)> {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            let cell = server_ctx
                .rect_terrain_id
                .map(|(x, y)| Vec2::new(x, y))
                .or_else(|| {
                    server_ctx
                        .hover_cursor
                        .map(|cursor| Vec2::new(cursor.x.floor() as i32, cursor.y.floor() as i32))
                });
            return cell
                .map(|cell| {
                    let mut chunks = FxHashSet::default();
                    Self::add_2d_cell_dirty_chunk(cell, &mut chunks);
                    chunks.into_iter().collect()
                })
                .unwrap_or_default();
        }

        let mut dirty_chunks = Vec::new();
        if let Some((x, z)) = server_ctx.rect_terrain_id {
            let chunk_size = 32;
            dirty_chunks.push((
                x.div_euclid(chunk_size) * chunk_size,
                z.div_euclid(chunk_size) * chunk_size,
            ));
        } else if let Some((object_id, _)) = server_ctx.rect_geometry_face_3d {
            let rusterix = crate::editor::RUSTERIX.read().unwrap();
            dirty_chunks.extend(
                rusterix
                    .scene_handler
                    .build_index
                    .chunks_for_owner(scenevm::GeoId::GeometryObject(object_id)),
            );
        } else if let Some(sector_id) = server_ctx.rect_sector_id_3d
            && let Some(sector) = work_map.find_sector(sector_id)
        {
            {
                let rusterix = crate::editor::RUSTERIX.read().unwrap();
                dirty_chunks.extend(
                    rusterix
                        .scene_handler
                        .build_index
                        .chunks_for_owner(scenevm::GeoId::Sector(sector_id)),
                );
            }
            if !Self::add_surface_tile_dirty_chunks(
                work_map,
                sector_id,
                server_ctx.rect_tile_id_3d,
                &mut dirty_chunks,
            ) {
                Self::add_bbox_dirty_chunks(sector.bounding_box(work_map), &mut dirty_chunks);
            }
        }

        dirty_chunks
    }

    fn add_2d_cell_dirty_chunk(cell: Vec2<i32>, chunks: &mut FxHashSet<(i32, i32)>) {
        // Scene streaming uses world-space chunks; this is unrelated to a tile's pixel size.
        let chunk_size = SCENEMANAGER.read().unwrap().chunk_size();
        Self::add_2d_cell_dirty_chunk_with_size(cell, chunk_size, chunks);
    }

    fn add_2d_cell_dirty_chunk_with_size(
        cell: Vec2<i32>,
        chunk_size: i32,
        chunks: &mut FxHashSet<(i32, i32)>,
    ) {
        let chunk_size = chunk_size.max(1);
        chunks.insert((
            cell.x.div_euclid(chunk_size) * chunk_size,
            cell.y.div_euclid(chunk_size) * chunk_size,
        ));
    }

    fn sync_live_stroke_throttled(
        work_map: &Map,
        server_ctx: &ServerContext,
        last_live_scene_sync_at: &mut Option<Instant>,
        stroke_dirty_chunks: &mut FxHashSet<(i32, i32)>,
        force: bool,
    ) {
        stroke_dirty_chunks.extend(Self::live_stroke_dirty_chunks(work_map, server_ctx));
        let now = Instant::now();
        if force
            || last_live_scene_sync_at
                .map(|last| now.duration_since(last) >= LIVE_PAINT_SYNC_INTERVAL)
                .unwrap_or(true)
        {
            let dirty_chunks = stroke_dirty_chunks.iter().copied().collect::<Vec<_>>();
            if !dirty_chunks.is_empty() {
                crate::utils::editor_scene_replace_incremental_map_update(
                    work_map.clone(),
                    dirty_chunks,
                );
            }
            *last_live_scene_sync_at = Some(now);
        }
    }

    fn reset_stroke(&mut self) {
        self.hud_control_active = false;
        self.stroke_active = false;
        self.stroke_changed = false;
        self.stroke_prev_map = None;
        self.stroke_work_map = None;
        self.last_2d_cell = None;
        self.line_start_2d_cell = None;
        self.line_axis_horizontal = None;
        self.processed.clear();
        self.last_live_scene_sync_at = None;
        self.stroke_dirty_chunks.clear();
    }

    fn cells_between(a: Vec2<i32>, b: Vec2<i32>) -> Vec<Vec2<i32>> {
        let mut out = Vec::new();
        let mut x0 = a.x;
        let mut y0 = a.y;
        let x1 = b.x;
        let y1 = b.y;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            out.push(Vec2::new(x0, y0));
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }

        out
    }

    fn current_3d_paint_key(server_ctx: &ServerContext) -> Option<RectPaintKey> {
        if let Some((x, z)) = server_ctx.rect_terrain_id {
            return Some(RectPaintKey::Terrain(x, z));
        }
        if let Some((object_id, face_index)) = server_ctx.rect_geometry_face_3d {
            let (x, y) = server_ctx.rect_tile_id_3d;
            return Some(RectPaintKey::Geometry(object_id, face_index, x, y));
        }
        if let Some(sector_id) = server_ctx.rect_sector_id_3d {
            let (x, y) = server_ctx.rect_tile_id_3d;
            return Some(RectPaintKey::Sector(sector_id, x, y));
        }
        None
    }

    fn apply_3d_paint_at_current_target(
        map: &mut Map,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
    ) -> Option<Vec2<i32>> {
        let source = crate::utils::get_source(ui, server_ctx);

        if let Some((x, z)) = server_ctx.rect_terrain_id {
            let mut tiles = match map.properties.get("tiles") {
                Some(Value::TileOverrides(existing)) => existing.clone(),
                _ => FxHashMap::default(),
            };

            let mut blend_tiles = match map.properties.get("blend_tiles") {
                Some(Value::BlendOverrides(existing)) => existing.clone(),
                _ => FxHashMap::default(),
            };
            let previous_tiles = tiles.clone();
            let previous_blend_tiles = blend_tiles.clone();

            if ui.shift {
                tiles.remove(&(x, z));
                blend_tiles.remove(&(x, z));
            } else if let Some(source) = source.clone() {
                if server_ctx.rect_blend_preset == VertexBlendPreset::Solid {
                    tiles.insert((x, z), source);
                    blend_tiles.remove(&(x, z));
                } else {
                    blend_tiles.insert((x, z), (server_ctx.rect_blend_preset, source));
                    tiles.remove(&(x, z));
                }
            } else {
                return None;
            }

            if tiles == previous_tiles && blend_tiles == previous_blend_tiles {
                return None;
            }

            if tiles.is_empty() {
                map.properties.remove("tiles");
            } else {
                map.properties.set("tiles", Value::TileOverrides(tiles));
            }
            if blend_tiles.is_empty() {
                map.properties.remove("blend_tiles");
            } else {
                map.properties
                    .set("blend_tiles", Value::BlendOverrides(blend_tiles));
            }

            return Some(Vec2::new(x, z));
        }

        if let Some((object_id, face_index)) = server_ctx.rect_geometry_face_3d {
            let key = server_ctx.rect_tile_id_3d;
            let object = map
                .geometry_objects
                .iter_mut()
                .find(|object| object.id == object_id)?;
            let face = object.faces.get_mut(face_index)?;

            if ui.shift {
                face.tiles.remove(&key)?;
            } else if let Some(source) = source.clone() {
                if face.tiles.get(&key) == Some(&source) {
                    return None;
                }
                face.tiles.insert(key, source);
            } else {
                return None;
            }

            return Some(Vec2::new(key.0, key.1));
        }

        if let Some(sector_id) = server_ctx.rect_sector_id_3d
            && let Some(sector) = map.find_sector_mut(sector_id)
        {
            let key = Vec2::new(server_ctx.rect_tile_id_3d.0, server_ctx.rect_tile_id_3d.1);

            let mut tiles = match sector.properties.get("tiles") {
                Some(Value::TileOverrides(existing)) => existing.clone(),
                _ => FxHashMap::default(),
            };
            let mut blend_tiles = match sector.properties.get("blend_tiles") {
                Some(Value::BlendOverrides(existing)) => existing.clone(),
                _ => FxHashMap::default(),
            };
            let previous_tiles = tiles.clone();
            let previous_blend_tiles = blend_tiles.clone();

            if ui.shift {
                tiles.remove(&server_ctx.rect_tile_id_3d);
                blend_tiles.remove(&server_ctx.rect_tile_id_3d);
            } else if let Some(source) = source {
                if server_ctx.rect_blend_preset == VertexBlendPreset::Solid {
                    tiles.insert(server_ctx.rect_tile_id_3d, source);
                    blend_tiles.remove(&server_ctx.rect_tile_id_3d);
                } else {
                    blend_tiles.insert(
                        server_ctx.rect_tile_id_3d,
                        (server_ctx.rect_blend_preset, source),
                    );
                    tiles.remove(&server_ctx.rect_tile_id_3d);
                }
            } else {
                return None;
            }

            if tiles == previous_tiles && blend_tiles == previous_blend_tiles {
                return None;
            }

            if tiles.is_empty() {
                sector.properties.remove("tiles");
            } else {
                sector.properties.set("tiles", Value::TileOverrides(tiles));
            }
            if blend_tiles.is_empty() {
                sector.properties.remove("blend_tiles");
            } else {
                sector
                    .properties
                    .set("blend_tiles", Value::BlendOverrides(blend_tiles));
            }

            return Some(key);
        }

        None
    }

    fn compute_3d_tile(
        &mut self,
        coord: Vec2<i32>,
        map: &Map,
        ui: &mut TheUI,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();

            let screen_uv = [
                coord.x as f32 / dim.width as f32,
                coord.y as f32 / dim.height as f32,
            ];

            let rusterix = RUSTERIX.read().unwrap();

            let rc = rusterix.scene_handler.vm.pick_geo_id_at_uv(
                dim.width as u32,
                dim.height as u32,
                screen_uv,
                false,
                false,
            );

            let mut found = false;
            server_ctx.rect_geometry_face_3d = None;

            if let Some((scenevm::GeoId::Sector(id), world_hit, _)) = rc {
                let mut best: Option<((i32, i32), f32)> = None;

                for surface in map.surfaces.values() {
                    if surface.sector_id != id {
                        continue;
                    }

                    let n = surface.plane.normal;
                    let n_len = n.magnitude();
                    if n_len <= 1e-6 {
                        continue;
                    }

                    // Choose the surface plane closest to the picked world hit.
                    // This avoids random sector-surface iteration order affecting tile coordinates.
                    let signed_dist = (world_hit - surface.plane.origin).dot(n / n_len);
                    let dist = signed_dist.abs();
                    let tile = surface.world_to_tile_local(world_hit, map);

                    if best
                        .as_ref()
                        .map(|(_, best_dist)| dist < *best_dist)
                        .unwrap_or(true)
                    {
                        best = Some((tile, dist));
                    }
                }

                if let Some((tile, _)) = best {
                    server_ctx.rect_tile_id_3d = tile;
                    server_ctx.rect_sector_id_3d = Some(id);
                    found = true;
                }
            }

            if !found
                && let Some((scenevm::GeoId::GeometryObject(id), world_hit, _)) = rc
                && let Some((face_index, tile)) =
                    Self::geometry_face_target_at_world(map, id, world_hit)
            {
                server_ctx.rect_tile_id_3d = tile;
                server_ctx.rect_geometry_face_3d = Some((id, face_index));
                server_ctx.rect_sector_id_3d = None;
                server_ctx.rect_terrain_id = None;
                found = true;
            }

            if let Some((scenevm::GeoId::Terrain(x, z), _, _)) = rc {
                server_ctx.rect_terrain_id = Some((x, z));
                server_ctx.rect_geometry_face_3d = None;
            } else {
                server_ctx.rect_terrain_id = None;
            }

            if !found {
                server_ctx.rect_sector_id_3d = None;
                server_ctx.rect_geometry_face_3d = None;
            }
        }
    }
}
