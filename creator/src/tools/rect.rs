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

    is_terrain: bool,
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

            is_terrain: false,
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
                    map.subdivisions,
                );
                // The size of the rect is always 1
                let step = 1.0; // / map.subdivisions;
                map.curr_rectangle = Some((cp, cp + step));
                hovered_vertices = Some([
                    cp,
                    cp + Vec2::new(0.0, step),
                    cp + Vec2::new(step, step),
                    cp + Vec2::new(step, 0.0),
                ]);
                server_ctx.hover_cursor = Some(cp);
            }

            hovered_vertices
        }

        match map_event {
            MapKey(c) => {
                match c {
                    '1'..='9' => map.subdivisions = (c as u8 - b'0') as f32,
                    '0' => map.subdivisions = 10.0,
                    _ => {}
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapClicked(coord) => {
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                self.processed.clear();
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    if let Some(cp) = server_ctx.hover_cursor {
                        let k = Vec2::new(cp.x as i32, cp.y as i32);
                        if !self.processed.contains(&k) {
                            undo_atom = add_tile(
                                ui,
                                ctx,
                                map,
                                server_ctx,
                                self.hovered_vertices,
                                self.mode,
                            );
                            self.processed.insert(k);
                        }
                    }
                } else {
                    let prev = map.clone();
                    self.is_terrain = false;

                    if let Some(tile_id) = server_ctx.curr_tile_id {
                        if let Some((x, z)) = server_ctx.rect_terrain_id {
                            // Terrain

                            let prev = map.clone();

                            let mut tiles = match map.properties.get("tiles") {
                                Some(Value::TileOverrides(existing)) => existing.clone(),
                                _ => FxHashMap::default(),
                            };

                            let mut blend_tiles = match map.properties.get("blend_tiles") {
                                Some(Value::BlendOverrides(existing)) => existing.clone(),
                                _ => FxHashMap::default(),
                            };

                            if server_ctx.rect_blend_preset == VertexBlendPreset::Solid {
                                if ui.shift {
                                    tiles.remove(&(x, z));
                                } else {
                                    tiles.insert((x, z), PixelSource::TileId(tile_id));
                                }

                                if tiles.is_empty() {
                                    map.properties.remove("tiles");
                                } else {
                                    map.properties.set("tiles", Value::TileOverrides(tiles));
                                }
                            } else {
                                if ui.shift {
                                    blend_tiles.remove(&(x, z));
                                } else {
                                    blend_tiles.insert(
                                        (x, z),
                                        (
                                            server_ctx.rect_blend_preset,
                                            PixelSource::TileId(tile_id),
                                        ),
                                    );
                                }

                                if blend_tiles.is_empty() {
                                    map.properties.remove("blend_tiles");
                                } else {
                                    map.properties
                                        .set("blend_tiles", Value::BlendOverrides(blend_tiles));
                                }
                            }

                            undo_atom = Some(ProjectUndoAtom::MapEdit(
                                server_ctx.pc,
                                Box::new(prev),
                                Box::new(map.clone()),
                            ));

                            self.is_terrain = true;
                            self.processed.insert(Vec2::new(x, z));
                        } else if let Some(sector_id) = server_ctx.rect_sector_id_3d {
                            if let Some(sector) = map.find_sector_mut(sector_id) {
                                // Sector / Surface

                                let mut tiles = match sector.properties.get("tiles") {
                                    Some(Value::TileOverrides(existing)) => existing.clone(),
                                    _ => FxHashMap::default(),
                                };

                                let mut blend_tiles = match sector.properties.get("blend_tiles") {
                                    Some(Value::BlendOverrides(existing)) => existing.clone(),
                                    _ => FxHashMap::default(),
                                };

                                if server_ctx.rect_blend_preset == VertexBlendPreset::Solid {
                                    if ui.shift {
                                        tiles.remove(&server_ctx.rect_tile_id_3d);
                                    } else {
                                        tiles.insert(
                                            server_ctx.rect_tile_id_3d,
                                            PixelSource::TileId(tile_id),
                                        );
                                    }

                                    if tiles.is_empty() {
                                        sector.properties.remove("tiles");
                                    } else {
                                        sector.properties.set("tiles", Value::TileOverrides(tiles));
                                    }
                                } else {
                                    if ui.shift {
                                        blend_tiles.remove(&server_ctx.rect_tile_id_3d);
                                    } else {
                                        blend_tiles.insert(
                                            server_ctx.rect_tile_id_3d,
                                            (
                                                server_ctx.rect_blend_preset,
                                                PixelSource::TileId(tile_id),
                                            ),
                                        );
                                    }

                                    if blend_tiles.is_empty() {
                                        sector.properties.remove("blend_tiles");
                                    } else {
                                        sector
                                            .properties
                                            .set("blend_tiles", Value::BlendOverrides(blend_tiles));
                                    }
                                }

                                undo_atom = Some(ProjectUndoAtom::MapEdit(
                                    server_ctx.pc,
                                    Box::new(prev),
                                    Box::new(map.clone()),
                                ));

                                self.processed.insert(Vec2::new(
                                    server_ctx.rect_tile_id_3d.0,
                                    server_ctx.rect_tile_id_3d.1,
                                ));
                            }
                        }
                    }
                }
            }
            MapDragged(coord) => {
                if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                    if let Some(cp) = server_ctx.hover_cursor {
                        let k = Vec2::new(cp.x as i32, cp.y as i32);
                        if !self.processed.contains(&k) {
                            undo_atom = add_tile(
                                ui,
                                ctx,
                                map,
                                server_ctx,
                                self.hovered_vertices,
                                self.mode,
                            );
                            self.processed.insert(k);
                        }
                    }
                } else {
                    self.compute_3d_tile(coord, map, ui, server_ctx);
                    if self.is_terrain {
                        if let Some((x, z)) = server_ctx.rect_terrain_id {
                            if !self.processed.contains(&Vec2::new(x, z)) {
                                let prev = map.clone();

                                let mut tiles = match map.properties.get("tiles") {
                                    Some(Value::TileOverrides(existing)) => existing.clone(),
                                    _ => FxHashMap::default(),
                                };

                                let mut blend_tiles = match map.properties.get("blend_tiles") {
                                    Some(Value::BlendOverrides(existing)) => existing.clone(),
                                    _ => FxHashMap::default(),
                                };

                                if let Some(tile_id) = server_ctx.curr_tile_id {
                                    if server_ctx.rect_blend_preset == VertexBlendPreset::Solid {
                                        if ui.shift {
                                            tiles.remove(&(x, z));
                                        } else {
                                            tiles.insert((x, z), PixelSource::TileId(tile_id));
                                        }

                                        if tiles.is_empty() {
                                            map.properties.remove("tiles");
                                        } else {
                                            map.properties
                                                .set("tiles", Value::TileOverrides(tiles));
                                        }
                                    } else {
                                        if ui.shift {
                                            blend_tiles.remove(&(x, z));
                                        } else {
                                            blend_tiles.insert(
                                                (x, z),
                                                (
                                                    server_ctx.rect_blend_preset,
                                                    PixelSource::TileId(tile_id),
                                                ),
                                            );
                                        }

                                        if blend_tiles.is_empty() {
                                            map.properties.remove("blend_tiles");
                                        } else {
                                            map.properties.set(
                                                "blend_tiles",
                                                Value::BlendOverrides(blend_tiles),
                                            );
                                        }
                                    }

                                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                                        server_ctx.pc,
                                        Box::new(prev),
                                        Box::new(map.clone()),
                                    ));

                                    self.processed.insert(Vec2::new(x, z));
                                }
                            }
                        }
                    } else {
                        if !self.processed.contains(&Vec2::new(
                            server_ctx.rect_tile_id_3d.0,
                            server_ctx.rect_tile_id_3d.1,
                        )) {
                            if let Some(sector_id) = server_ctx.rect_sector_id_3d {
                                let prev = map.clone();
                                if let Some(tile_id) = server_ctx.curr_tile_id {
                                    if let Some(sector) = map.find_sector_mut(sector_id) {
                                        let mut tiles = match sector.properties.get("tiles") {
                                            Some(Value::TileOverrides(existing)) => {
                                                existing.clone()
                                            }
                                            _ => FxHashMap::default(),
                                        };

                                        let mut blend_tiles =
                                            match sector.properties.get("blend_tiles") {
                                                Some(Value::BlendOverrides(existing)) => {
                                                    existing.clone()
                                                }
                                                _ => FxHashMap::default(),
                                            };

                                        if server_ctx.rect_blend_preset == VertexBlendPreset::Solid
                                        {
                                            if ui.shift {
                                                tiles.remove(&server_ctx.rect_tile_id_3d);
                                            } else {
                                                tiles.insert(
                                                    server_ctx.rect_tile_id_3d,
                                                    PixelSource::TileId(tile_id),
                                                );
                                            }

                                            if tiles.is_empty() {
                                                sector.properties.remove("tiles");
                                            } else {
                                                sector
                                                    .properties
                                                    .set("tiles", Value::TileOverrides(tiles));
                                            }
                                        } else {
                                            if ui.shift {
                                                blend_tiles.remove(&server_ctx.rect_tile_id_3d);
                                            } else {
                                                blend_tiles.insert(
                                                    server_ctx.rect_tile_id_3d,
                                                    (
                                                        server_ctx.rect_blend_preset,
                                                        PixelSource::TileId(tile_id),
                                                    ),
                                                );
                                            }

                                            if blend_tiles.is_empty() {
                                                sector.properties.remove("blend_tiles");
                                            } else {
                                                sector.properties.set(
                                                    "blend_tiles",
                                                    Value::BlendOverrides(blend_tiles),
                                                );
                                            }
                                        }

                                        undo_atom = Some(ProjectUndoAtom::MapEdit(
                                            server_ctx.pc,
                                            Box::new(prev),
                                            Box::new(map.clone()),
                                        ));

                                        self.processed.insert(Vec2::new(
                                            server_ctx.rect_tile_id_3d.0,
                                            server_ctx.rect_tile_id_3d.1,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            MapUp(_) => {}
            MapHover(coord) => {
                if server_ctx.editor_view_mode == EditorViewMode::D2 {
                    self.hovered_vertices = apply_hover(coord, ui, ctx, map, server_ctx);
                } else {
                    self.compute_3d_tile(coord, map, ui, server_ctx);
                }
            }
            MapDelete => {}
            MapEscape => {
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
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Apply Map Properties" && *state == TheWidgetState::Clicked {
                    let mut source: Option<Value> = None;

                    if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
                        if let Some(id) = server_ctx.curr_tile_id {
                            source = Some(Value::Source(PixelSource::TileId(id)));
                        }
                    } /*else if server_ctx.curr_map_tool_helper == MapToolHelper::ColorPicker {
                    if let Some(palette_picker) = ui.get_palette_picker("Panel Palette Picker")
                    {
                    if let Some(color) = &project.palette.colors[palette_picker.index()] {
                    source = Some(Value::Source(PixelSource::Color(color.clone())));
                    }
                    }
                    }*/

                    if let Some(source) = source {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let prev = map.clone();

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

                            let undo_atom =
                                RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));

                            crate::editor::UNDOMANAGER.write().unwrap().add_region_undo(
                                &server_ctx.curr_region,
                                undo_atom,
                                ctx,
                            );
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        }
                    }
                } else if id.name == "Remove Map Properties" && *state == TheWidgetState::Clicked {
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        let prev = map.clone();

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

                        let undo_atom =
                            RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));

                        crate::editor::UNDOMANAGER.write().unwrap().add_region_undo(
                            &server_ctx.curr_region,
                            undo_atom,
                            ctx,
                        );
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
                for (_, surface) in &map.surfaces {
                    if surface.sector_id == id {
                        server_ctx.rect_tile_id_3d = surface.world_to_tile(world_hit);
                        server_ctx.rect_sector_id_3d = Some(id);
                        found = true;
                    }
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
