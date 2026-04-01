use crate::editor::RUSTERIX;
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Assets;
use rusterix::rebuild_generated_geometry;

pub struct DungeonTool {
    id: TheId,
    hud: Hud,
    stroke_active: bool,
    stroke_changed: bool,
    stroke_prev_map: Option<Map>,
    stroke_anchor: Option<Vec2<i32>>,
    line_axis_horizontal: Option<bool>,
    last_cell: Option<Vec2<i32>>,
}

impl Tool for DungeonTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Dungeon Tool"),
            hud: Hud::new(HudMode::Dungeon),
            stroke_active: false,
            stroke_changed: false,
            stroke_prev_map: None,
            stroke_anchor: None,
            line_axis_horizontal: None,
            last_cell: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_dungeon")
    }

    fn icon_name(&self) -> String {
        "skull".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('U')
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
                ctx.ui
                    .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));

                if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                    layout.set_mode(TheSharedHLayoutMode::Right);
                    ctx.ui.relayout = true;
                }

                server_ctx.editing_surface = None;
                server_ctx.prev_dungeon_view_mode = Some(server_ctx.editor_view_mode);
                ui.set_widget_value(
                    "Editor View Switch",
                    ctx,
                    TheValue::Int(EditorViewMode::D2.to_index()),
                );
                ctx.ui.send(TheEvent::IndexChanged(
                    TheId::named("Editor View Switch"),
                    EditorViewMode::D2.to_index() as usize,
                ));

                server_ctx.curr_map_tool_type = MapToolType::Dungeon;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;

                if let Some(map) = project.get_map_mut(server_ctx) {
                    server_ctx.prev_dungeon_subdivisions = Some(map.subdivisions);
                    map.subdivisions = 1.0;
                    map.selected_vertices.clear();
                    map.selected_linedefs.clear();
                    map.selected_sectors.clear();
                }

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Open Dungeon Dock"),
                    TheValue::Empty,
                ));
                true
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if let Some(map) = project.get_map_mut(server_ctx)
                    && let Some(prev_subdivisions) = server_ctx.prev_dungeon_subdivisions.take()
                {
                    map.subdivisions = prev_subdivisions;
                }
                if let Some(prev_view_mode) = server_ctx.prev_dungeon_view_mode.take() {
                    ui.set_widget_value(
                        "Editor View Switch",
                        ctx,
                        TheValue::Int(prev_view_mode.to_index()),
                    );
                    ctx.ui.send(TheEvent::IndexChanged(
                        TheId::named("Editor View Switch"),
                        prev_view_mode.to_index() as usize,
                    ));
                }
                if let Some(prev) = server_ctx.prev_dungeon_dock.take() {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Restore Previous Dock"),
                        TheValue::Text(prev),
                    ));
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Hide Dungeon Settings"),
                    TheValue::Empty,
                ));
                self.reset_stroke();
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(coord) => {
                self.update_hover_cell(coord, ui, ctx, map, server_ctx);
                self.begin_stroke(map);
                self.apply_at(coord, ui, map, server_ctx);
            }
            MapDragged(coord) => {
                if !self.stroke_active {
                    self.begin_stroke(map);
                }
                self.update_hover_cell(coord, ui, ctx, map, server_ctx);
                self.apply_at(coord, ui, map, server_ctx);
            }
            MapUp(_) => {
                if self.stroke_active {
                    self.last_cell = None;
                    self.stroke_anchor = None;
                    self.line_axis_horizontal = None;
                    self.stroke_active = false;
                    if self.stroke_changed
                        && let Some(prev) = self.stroke_prev_map.take()
                    {
                        self.stroke_changed = false;
                        return Some(ProjectUndoAtom::MapEdit(
                            server_ctx.pc,
                            Box::new(prev),
                            Box::new(map.clone()),
                        ));
                    }
                    self.stroke_changed = false;
                    self.stroke_prev_map = None;
                }
            }
            MapHover(coord) => {
                if self.hud.hovered(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if server_ctx.editor_view_mode == EditorViewMode::D2
                    && let Some(render_view) = ui.get_render_view("PolyView")
                {
                    let dim = *render_view.dim();
                    let cp = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Cursor Pos Changed"),
                        TheValue::Float2(cp),
                    ));
                    let cell_pos = server_ctx.local_to_map_cell(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        1.0,
                    );
                    server_ctx.hover_cursor = Some(cell_pos);
                    RUSTERIX.write().unwrap().set_dirty();
                }
            }
            MapEscape => {
                if self.stroke_active {
                    if let Some(prev) = self.stroke_prev_map.take() {
                        *map = prev;
                    }
                    self.reset_stroke();
                }
                server_ctx.hover_cursor = None;
                RUSTERIX.write().unwrap().set_dirty();
            }
            _ => {}
        }
        None
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        assets: &Assets,
    ) {
        self.hud.draw(buffer, map, ctx, server_ctx, None, assets);
    }
}

impl DungeonTool {
    fn begin_stroke(&mut self, map: &Map) {
        if !self.stroke_active {
            self.stroke_active = true;
            self.stroke_changed = false;
            self.stroke_prev_map = Some(map.clone());
            self.stroke_anchor = None;
            self.line_axis_horizontal = None;
            self.last_cell = None;
        }
    }

    fn reset_stroke(&mut self) {
        self.stroke_active = false;
        self.stroke_changed = false;
        self.stroke_prev_map = None;
        self.stroke_anchor = None;
        self.line_axis_horizontal = None;
        self.last_cell = None;
    }

    fn apply_at(
        &mut self,
        coord: Vec2<i32>,
        ui: &mut TheUI,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) {
        if server_ctx.editor_view_mode != EditorViewMode::D2 {
            return;
        }
        let hovered = if let Some(cp) = server_ctx.hover_cursor {
            cp
        } else if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();
            server_ctx.local_to_map_cell(
                Vec2::new(dim.width as f32, dim.height as f32),
                Vec2::new(coord.x as f32, coord.y as f32),
                map,
                1.0,
            )
        } else {
            return;
        };
        let mut cell = Vec2::new(hovered.x.floor() as i32, hovered.y.floor() as i32);
        let anchor = *self.stroke_anchor.get_or_insert(cell);
        if ui.ctrl {
            if self.line_axis_horizontal.is_none() {
                let dx = (cell.x - anchor.x).abs();
                let dy = (cell.y - anchor.y).abs();
                if dx > 0 || dy > 0 {
                    self.line_axis_horizontal = Some(dx >= dy);
                }
            }
            if let Some(horizontal) = self.line_axis_horizontal {
                if horizontal {
                    cell.y = anchor.y;
                } else {
                    cell.x = anchor.x;
                }
            }
        } else {
            self.line_axis_horizontal = None;
        }
        let stroke_cells = if let Some(last) = self.last_cell {
            Self::cells_between(last, cell)
        } else {
            vec![cell]
        };
        self.last_cell = Some(cell);

        let mut cells = Vec::new();
        for cell in stroke_cells {
            cells.extend(Self::stamp_cells_for_tile(
                cell,
                server_ctx.curr_dungeon_tile,
                server_ctx.curr_dungeon_tile_span,
            ));
        }

        let mut any_changed = false;
        for cell in cells {
            let changed = if ui.shift {
                map.dungeon.remove_active_cell(cell.x, cell.y)
            } else {
                map.dungeon.upsert_active_cell(
                    cell.x,
                    cell.y,
                    server_ctx.curr_dungeon_tile,
                    server_ctx.curr_dungeon_floor_base,
                    server_ctx.curr_dungeon_height,
                    server_ctx.curr_dungeon_standalone,
                    server_ctx.curr_dungeon_tile_span.max(1),
                    server_ctx.curr_dungeon_tile_depth.max(0.05),
                    server_ctx.curr_dungeon_tile_height.max(0.5),
                    server_ctx.curr_dungeon_tile_open_mode,
                    server_ctx.curr_dungeon_tile_item.clone(),
                )
            };

            any_changed |= changed;
        }

        if any_changed {
            rebuild_generated_geometry(
                map,
                server_ctx.curr_dungeon_create_floor,
                server_ctx.curr_dungeon_create_ceiling,
            );
            map.changed += 1;
            self.stroke_changed = true;
            RUSTERIX.write().unwrap().set_dirty();
        }
    }

    fn update_hover_cell(
        &mut self,
        coord: Vec2<i32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &Map,
        server_ctx: &mut ServerContext,
    ) {
        if server_ctx.editor_view_mode != EditorViewMode::D2 {
            return;
        }
        if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();
            let screen_size = Vec2::new(dim.width as f32, dim.height as f32);
            let local = Vec2::new(coord.x as f32, coord.y as f32);
            let cp = server_ctx.local_to_map_grid(screen_size, local, map, map.subdivisions);
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Cursor Pos Changed"),
                TheValue::Float2(cp),
            ));
            let cell_pos = server_ctx.local_to_map_cell(screen_size, local, map, 1.0);
            server_ctx.hover_cursor = Some(cell_pos);
            RUSTERIX.write().unwrap().set_dirty();
        }
    }

    fn cells_between(from: Vec2<i32>, to: Vec2<i32>) -> Vec<Vec2<i32>> {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let steps = dx.abs().max(dy.abs());
        if steps == 0 {
            return vec![to];
        }

        let mut cells = Vec::with_capacity(steps as usize + 1);
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let x = from.x as f32 + dx as f32 * t;
            let y = from.y as f32 + dy as f32 * t;
            let cell = Vec2::new(x.round() as i32, y.round() as i32);
            if cells.last().copied() != Some(cell) {
                cells.push(cell);
            }
        }
        cells
    }

    fn stamp_cells_for_tile(
        origin: Vec2<i32>,
        tile: rusterix::DungeonTileKind,
        span: i32,
    ) -> Vec<Vec2<i32>> {
        let span = span.max(1);
        if !tile.is_door() || span == 1 {
            return vec![origin];
        }

        let mut cells = Vec::with_capacity(span as usize);
        if tile.has_door_north() || tile.has_door_south() {
            let start_x = origin.x - (span - 1) / 2;
            for dx in 0..span {
                cells.push(Vec2::new(start_x + dx, origin.y));
            }
        } else {
            let start_y = origin.y - (span - 1) / 2;
            for dy in 0..span {
                cells.push(Vec2::new(origin.x, start_y + dy));
            }
        }
        cells
    }
}
