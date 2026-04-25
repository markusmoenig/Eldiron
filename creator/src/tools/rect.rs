use crate::editor::RUSTERIX;
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::prelude::*;
use vek::Vec2;

pub struct RectTool {
    id: TheId,

    hovered_vertices: Option<[Vec2<f32>; 4]>,
    mode: i32,
    hud: Hud,

    processed: FxHashSet<Vec2<i32>>,

    stroke_active: bool,
    stroke_changed: bool,
    stroke_prev_map: Option<Map>,
    stroke_work_map: Option<Map>,
    last_2d_cell: Option<Vec2<i32>>,
    line_start_2d_cell: Option<Vec2<i32>>,
    line_axis_horizontal: Option<bool>,
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

            processed: FxHashSet::default(),

            stroke_active: false,
            stroke_changed: false,
            stroke_prev_map: None,
            stroke_work_map: None,
            last_2d_cell: None,
            line_start_2d_cell: None,
            line_axis_horizontal: None,
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

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/rect".to_string())
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

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_vertices.clear();
                    region.map.selected_linedefs.clear();
                    region.map.selected_sectors.clear();
                }

                // self.activate_map_tool_helper(ui, ctx, project, server_ctx);

                return true;
            }
            DeActivate => {
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

        /// Add a tile at the current hover position
        fn add_tile(
            ui: &mut TheUI,
            _ctx: &mut TheContext,
            map: &mut Map,
            server_ctx: &mut ServerContext,
            hovered_vertices: Option<[Vec2<f32>; 4]>,
            mode: i32,
        ) -> Option<ProjectUndoAtom> {
            let mut undo_atom: Option<ProjectUndoAtom> = None;
            // let size = 1.0 / map.subdivisions;

            if let Some(vertices) = hovered_vertices {
                let mut add_it = true;
                let mut layer: u8 = 0;

                if ui.shift {
                    // Delete the top tile at the given position if shift is pressed
                    if let Some(ev0) = map.find_vertex_at(vertices[0].x, vertices[0].y) {
                        if let Some(ev1) = map.find_vertex_at(vertices[1].x, vertices[1].y) {
                            if let Some(ev2) = map.find_vertex_at(vertices[2].x, vertices[2].y) {
                                if let Some(ev3) = map.find_vertex_at(vertices[3].x, vertices[3].y)
                                {
                                    let sectors =
                                        map.find_sectors_with_vertex_indices(&[ev0, ev1, ev2, ev3]);

                                    if let Some(sector_id) = sectors.last() {
                                        let prev = map.clone();
                                        let mut lines = vec![];
                                        if let Some(s) = map.find_sector(*sector_id) {
                                            lines = s.linedefs.clone();
                                        }
                                        map.delete_elements(&[], &lines, &[*sector_id]);
                                        undo_atom = Some(ProjectUndoAtom::MapEdit(
                                            server_ctx.pc,
                                            Box::new(prev),
                                            Box::new(map.clone()),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(source) = get_source(ui, server_ctx) {
                    // Add mode
                    // Check if tile already exists with same source
                    if let Some(ev0) = map.find_vertex_at(vertices[0].x, vertices[0].y) {
                        if let Some(ev1) = map.find_vertex_at(vertices[1].x, vertices[1].y) {
                            if let Some(ev2) = map.find_vertex_at(vertices[2].x, vertices[2].y) {
                                if let Some(ev3) = map.find_vertex_at(vertices[3].x, vertices[3].y)
                                {
                                    let sectors =
                                        map.find_sectors_with_vertex_indices(&[ev0, ev1, ev2, ev3]);

                                    let prev = map.clone();
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
                                                    // A tile with the same floor_source exists, do not add.
                                                    add_it = false;
                                                } else if mode == 0 {
                                                    // In overlay mode we just overwrite the source
                                                    sector.properties.set(
                                                        "source",
                                                        Value::Source(source.clone()),
                                                    );

                                                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                                                        server_ctx.pc,
                                                        Box::new(prev),
                                                        Box::new(map.clone()),
                                                    ));

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

                            let prev = map.clone();
                            if let Some(sector) = map.find_sector_mut(sector_id) {
                                sector.properties.set("rect", Value::Bool(true));

                                sector.properties.set("source", Value::Source(source));
                                sector.layer = Some(layer + 1);
                            }

                            undo_atom = Some(ProjectUndoAtom::MapEdit(
                                server_ctx.pc,
                                Box::new(prev),
                                Box::new(map.clone()),
                            ));

                            map.selected_vertices.clear();
                            map.selected_linedefs.clear();
                            map.selected_sectors = vec![sector_id];
                        }
                    }
                }
            }

            undo_atom
        }

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
                if map.properties.get_bool_default("terrain_enabled", false) {
                    server_ctx.rect_terrain_id = Some((cp.x.floor() as i32, cp.y.floor() as i32));
                } else {
                    server_ctx.rect_terrain_id = None;
                }
            }

            hovered_vertices
        }

        match map_event {
            MapKey(_c) => {}
            MapClicked(coord) => {
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
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
                    let use_terrain_paint =
                        map.properties.get_bool_default("terrain_enabled", false);
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
                                add_tile(ui, ctx, work_map, server_ctx, verts, self.mode).is_some()
                            };
                            if changed {
                                self.stroke_changed = true;
                            }
                            self.processed.insert(k);
                            self.last_2d_cell = Some(k);
                        }
                    }
                } else {
                    self.compute_3d_tile(coord, map, ui, server_ctx);
                    self.begin_stroke_if_needed(map);
                    if let Some(work_map) = self.stroke_work_map.as_mut()
                        && let Some(key) =
                            Self::apply_3d_paint_at_current_target(work_map, ui, server_ctx)
                        && !self.processed.contains(&key)
                    {
                        self.stroke_changed = true;
                        self.processed.insert(key);
                    }
                }
            }
            MapDragged(coord) => {
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
                    let use_terrain_paint =
                        map.properties.get_bool_default("terrain_enabled", false);
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
                                if self.processed.contains(&cell) {
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
                                    add_tile(ui, ctx, work_map, server_ctx, verts, self.mode)
                                        .is_some()
                                };
                                if changed {
                                    self.stroke_changed = true;
                                }
                                self.processed.insert(cell);
                            }
                            self.last_2d_cell = Some(to);
                        }
                    }
                } else {
                    self.compute_3d_tile(coord, map, ui, server_ctx);
                    self.begin_stroke_if_needed(map);
                    if let Some(work_map) = self.stroke_work_map.as_mut()
                        && let Some(key) =
                            Self::apply_3d_paint_at_current_target(work_map, ui, server_ctx)
                        && !self.processed.contains(&key)
                    {
                        self.stroke_changed = true;
                        self.processed.insert(key);
                    }
                }
            }
            MapUp(_) => {
                if self.stroke_active {
                    if self.stroke_changed
                        && let (Some(prev), Some(new_map)) =
                            (self.stroke_prev_map.take(), self.stroke_work_map.take())
                    {
                        *map = new_map;
                        undo_atom = Some(ProjectUndoAtom::MapEdit(
                            server_ctx.pc,
                            Box::new(prev),
                            Box::new(map.clone()),
                        ));
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
        let id = if !map.selected_linedefs.is_empty() {
            Some(map.selected_linedefs[0])
        } else {
            None
        };
        self.hud.draw(buffer, map, ctx, server_ctx, id, assets);
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

impl RectTool {
    fn encode_tile_source(source: TileSource) -> TheValue {
        match source {
            TileSource::SingleTile(id) => {
                TheValue::List(vec![TheValue::Text("single".to_string()), TheValue::Id(id)])
            }
            TileSource::TileGroup(id) => {
                TheValue::List(vec![TheValue::Text("group".to_string()), TheValue::Id(id)])
            }
            TileSource::TileGroupMember {
                group_id,
                member_index,
            } => TheValue::List(vec![
                TheValue::Text("group_member".to_string()),
                TheValue::Id(group_id),
                TheValue::Int(member_index as i32),
            ]),
            TileSource::Procedural(id) => TheValue::List(vec![
                TheValue::Text("procedural".to_string()),
                TheValue::Id(id),
            ]),
        }
    }

    fn tile_source_from_pixel_source(source: &PixelSource) -> Option<TileSource> {
        match source {
            PixelSource::TileId(id) => Some(TileSource::SingleTile(*id)),
            PixelSource::TileGroup(id) => Some(TileSource::TileGroup(*id)),
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => Some(TileSource::TileGroupMember {
                group_id: *group_id,
                member_index: *member_index,
            }),
            PixelSource::ProceduralTile(id) => Some(TileSource::Procedural(*id)),
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
            if map.properties.get_bool_default("terrain_enabled", false) {
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

        None
    }

    fn begin_stroke_if_needed(&mut self, map: &Map) {
        if !self.stroke_active {
            self.stroke_active = true;
            self.stroke_changed = false;
            self.stroke_prev_map = Some(map.clone());
            self.stroke_work_map = Some(map.clone());
            self.processed.clear();
        }
    }

    fn reset_stroke(&mut self) {
        self.stroke_active = false;
        self.stroke_changed = false;
        self.stroke_prev_map = None;
        self.stroke_work_map = None;
        self.last_2d_cell = None;
        self.line_start_2d_cell = None;
        self.line_axis_horizontal = None;
        self.processed.clear();
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

    fn apply_3d_paint_at_current_target(
        map: &mut Map,
        ui: &TheUI,
        server_ctx: &ServerContext,
    ) -> Option<Vec2<i32>> {
        let curr_tile_id = server_ctx.curr_tile_id;

        if let Some((x, z)) = server_ctx.rect_terrain_id {
            let mut tiles = match map.properties.get("tiles") {
                Some(Value::TileOverrides(existing)) => existing.clone(),
                _ => FxHashMap::default(),
            };

            let mut blend_tiles = match map.properties.get("blend_tiles") {
                Some(Value::BlendOverrides(existing)) => existing.clone(),
                _ => FxHashMap::default(),
            };

            if ui.shift {
                tiles.remove(&(x, z));
                blend_tiles.remove(&(x, z));
            } else if let Some(tile_id) = curr_tile_id {
                if server_ctx.rect_blend_preset == VertexBlendPreset::Solid {
                    tiles.insert((x, z), PixelSource::TileId(tile_id));
                    blend_tiles.remove(&(x, z));
                } else {
                    blend_tiles.insert(
                        (x, z),
                        (server_ctx.rect_blend_preset, PixelSource::TileId(tile_id)),
                    );
                    tiles.remove(&(x, z));
                }
            } else {
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

            if ui.shift {
                tiles.remove(&server_ctx.rect_tile_id_3d);
                blend_tiles.remove(&server_ctx.rect_tile_id_3d);
            } else if let Some(tile_id) = curr_tile_id {
                if server_ctx.rect_blend_preset == VertexBlendPreset::Solid {
                    tiles.insert(server_ctx.rect_tile_id_3d, PixelSource::TileId(tile_id));
                    blend_tiles.remove(&server_ctx.rect_tile_id_3d);
                } else {
                    blend_tiles.insert(
                        server_ctx.rect_tile_id_3d,
                        (server_ctx.rect_blend_preset, PixelSource::TileId(tile_id)),
                    );
                    tiles.remove(&server_ctx.rect_tile_id_3d);
                }
            } else {
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

            if let Some((scenevm::GeoId::Terrain(x, z), _, _)) = rc {
                server_ctx.rect_terrain_id = Some((x, z));
            } else {
                server_ctx.rect_terrain_id = None;
            }

            if !found {
                server_ctx.rect_sector_id_3d = None;
            }
        }
    }
}
