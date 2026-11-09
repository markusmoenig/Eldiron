use crate::{
    editor::{RUSTERIX, SCENEMANAGER, UNDOMANAGER},
    prelude::*,
};
use rusterix::chunkbuilder::{
    terrain_sculpt::{SculptMode, TerrainSculpt},
    terrain_surface::TerrainSurface,
};
use rusterix::{Assets, MapCamera, PixelSource, Value};
use std::time::{Duration, Instant};

struct Stroke {
    before: Map,
    changed: bool,
    last: Vec2<f32>,
    mode: SculptMode,
    tick: Instant,
    dirty: FxHashSet<(i32, i32)>,
    published: Option<Instant>,
}

pub struct TerrainTool {
    id: TheId,
    mode: SculptMode,
    radius: f32,
    strength: f32,
    hover: Option<Vec2<f32>>,
    stroke: Option<Stroke>,
    panel_down: bool,
}

impl TerrainTool {
    fn panel() -> TheDim {
        TheDim::rect(8, 28, 232, 104)
    }

    fn mode_at(&self, ui: &TheUI) -> SculptMode {
        if ui.shift {
            SculptMode::Lower
        } else {
            self.mode
        }
    }

    fn pointer(
        ui: &mut TheUI,
        ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
    ) -> Option<Vec2<f32>> {
        let surface = TerrainSurface::cached(map);
        if ctx.editor_view_mode == EditorViewMode::D2 {
            let dim = *crate::utils::map_editor_render_view(ui, ctx)?.dim();
            let pixel = coord.map(|v| v as f32);
            let size = Vec2::new(dim.width as f32, dim.height as f32);
            if map.camera == MapCamera::TwoDIso {
                let p = rusterix::isometric::Projection::editor(map, size);
                let base = p.ground(pixel);
                return Some(
                    surface
                        .and_then(|t| {
                            t.raycast(
                                Vec3::new(base.x, 0., base.y),
                                Vec3::new(-1.2247449, 1., 1.2247449),
                                t.max_height,
                            )
                        })
                        .map_or(base, |p| Vec2::new(p.x, p.z)),
                );
            }
            return Some(
                (pixel - size * 0.5 - Vec2::new(map.offset.x, -map.offset.y)) / map.grid_size,
            );
        }
        let origin = ctx.hover_ray_origin_3d?;
        let forward = ctx.hover_ray_dir_3d?;
        if forward.y >= -0.0001 {
            return None;
        }
        let t = -origin.y / forward.y;
        if t < 0. {
            return None;
        }
        let base = origin + forward * t;
        let point = surface
            .and_then(|surface| surface.raycast(base, -forward, surface.max_height / -forward.y));
        Some(point.map_or(Vec2::new(base.x, base.z), |p| Vec2::new(p.x, p.z)))
    }

    fn set_hover(
        &mut self,
        p: Option<Vec2<f32>>,
        map: &Map,
        ctx: &mut ServerContext,
        ui_ctx: &mut TheContext,
    ) {
        self.hover = p;
        ctx.hover_cursor = p;
        let surface = TerrainSurface::cached(map);
        ctx.hover_cursor_3d = p.map(|p| {
            Vec3::new(
                p.x,
                surface.as_ref().and_then(|t| t.height_at(p)).unwrap_or(0.),
                p.y,
            )
        });
        ctx.terrain_brush_radius = self.radius;
        if ctx.editor_view_mode != EditorViewMode::D2 {
            ui_ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Geometry Overlay 3D"),
                TheValue::Empty,
            ));
        }
        ui_ctx.ui.redraw_all = true;
    }

    fn panel_click(&mut self, at: Vec2<i32>) {
        if (36..62).contains(&at.y) {
            self.mode = match ((at.x - 14) / 74).clamp(0, 2) {
                0 => SculptMode::Raise,
                1 => SculptMode::Lower,
                _ => SculptMode::Smooth,
            };
        } else if at.x >= 164 {
            let up = at.x >= 200;
            if (68..94).contains(&at.y) {
                self.radius = (self.radius + if up { 0.5 } else { -0.5 }).clamp(0.5, 16.);
            } else if (100..126).contains(&at.y) {
                self.strength = (self.strength + if up { 0.25 } else { -0.25 }).clamp(0.25, 4.);
            }
        }
    }

    fn ensure_ground(map: &mut Map, server: &ServerContext) {
        if map.properties.contains("default_terrain_tile") {
            return;
        }
        let r = RUSTERIX.read().unwrap();
        let ground = server
            .curr_tile_source
            .map(|s| match s {
                rusterix::TileSource::SingleTile(id) => id,
            })
            .or(server.curr_tile_id)
            .filter(|id| {
                r.assets
                    .tiles
                    .get(id)
                    .is_some_and(|t| t.isometric.as_ref().is_none_or(|s| s.ground))
            })
            .or_else(|| {
                map.sectors.iter().find_map(|s| {
                    s.properties
                        .get_default_source()
                        .and_then(|s| s.render_tile_id(&r.assets))
                        .filter(|id| {
                            r.assets
                                .tiles
                                .get(id)
                                .is_some_and(|t| t.isometric.as_ref().is_some_and(|s| s.ground))
                        })
                })
            });
        // A visible neutral ground also lets a new empty map be sculpted before
        // choosing its final MZ material with the ordinary Rect brush.
        let source = ground
            .map(PixelSource::TileId)
            .unwrap_or(PixelSource::Color([65, 85, 65, 255].into()));
        let neutral = matches!(source, PixelSource::Color(_));
        drop(r);
        map.properties
            .set("default_terrain_tile", Value::Source(source));
        if neutral {
            let (tiles, indices) = {
                let mut r = RUSTERIX.write().unwrap();
                let tiles = r.assets.tiles.clone();
                r.set_tiles_for_maps(tiles, false, [&*map]);
                (r.assets.tile_list.clone(), r.assets.tile_indices.clone())
            };
            SCENEMANAGER.write().unwrap().set_tile_list(tiles, indices);
        }
    }

    fn dab(&mut self, map: &mut Map, center: Vec2<f32>, amount: f32, server: &ServerContext) {
        let Some(stroke) = &mut self.stroke else {
            return;
        };
        if TerrainSculpt::dab(map, center, self.radius, amount, stroke.mode) {
            Self::ensure_ground(map, server);
            stroke.changed = true;
            let chunk = SCENEMANAGER.read().unwrap().chunk_size().max(1);
            let min = (center - self.radius - 1.).map(|v| (v.floor() as i32).div_euclid(chunk));
            let max = (center + self.radius + 1.).map(|v| (v.ceil() as i32).div_euclid(chunk));
            for z in min.y..=max.y {
                for x in min.x..=max.x {
                    stroke.dirty.insert((x * chunk, z * chunk));
                }
            }
            RUSTERIX.write().unwrap().set_dirty();
        }
    }

    fn publish(&mut self, map: &Map, force: bool) {
        let Some(stroke) = &mut self.stroke else {
            return;
        };
        if !stroke.changed {
            return;
        }
        if force
            || stroke
                .published
                .is_none_or(|t| t.elapsed() >= Duration::from_millis(80))
        {
            crate::utils::editor_scene_incremental_map_update(
                map.clone(),
                stroke.dirty.iter().copied().collect(),
            );
            stroke.published = Some(Instant::now());
        }
    }

    fn finish(&mut self, map: &mut Map, pc: ProjectContext) -> Option<ProjectUndoAtom> {
        if self.stroke.as_ref().is_some_and(|s| s.before.id != map.id) {
            self.stroke = None;
            return None;
        }
        self.publish(map, true);
        let stroke = self.stroke.take()?;
        stroke
            .changed
            .then(|| ProjectUndoAtom::MapEdit(pc, Box::new(stroke.before), Box::new(map.clone())))
    }

    fn cancel(&mut self, map: &mut Map) {
        if let Some(stroke) = self.stroke.take() {
            if stroke.changed && stroke.before.id == map.id {
                let revision = map.changed.wrapping_add(1);
                *map = stroke.before;
                map.changed = revision;
                crate::utils::editor_scene_incremental_map_update(
                    map.clone(),
                    stroke.dirty.into_iter().collect(),
                );
                RUSTERIX.write().unwrap().set_dirty();
            }
        }
    }

    pub fn outline(map: &Map, center: Vec2<f32>, radius: f32) -> Vec<Vec3<f32>> {
        let surface = TerrainSurface::cached(map);
        (0..=96)
            .map(|i| {
                let angle = i as f32 / 96. * std::f32::consts::TAU;
                let p = center + Vec2::new(angle.cos(), angle.sin()) * radius;
                Vec3::new(
                    p.x,
                    surface.as_ref().and_then(|t| t.height_at(p)).unwrap_or(0.),
                    p.y,
                )
            })
            .collect()
    }
}

impl Tool for TerrainTool {
    fn new() -> Self {
        Self {
            id: TheId::named("Terrain Tool"),
            mode: SculptMode::Raise,
            radius: 2.,
            strength: 1.,
            hover: None,
            stroke: None,
            panel_down: false,
        }
    }
    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        "Terrain brush: raise, lower and smooth hills. Shift lowers; [ / ] change radius.".into()
    }
    fn icon_name(&self) -> String {
        "terrain".into()
    }
    fn accel(&self) -> Option<char> {
        Some('T')
    }

    fn tool_event(
        &mut self,
        event: ToolEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        match event {
            ToolEvent::Activate => {
                server.curr_map_tool_type = MapToolType::Terrain;
                server.terrain_brush_radius = self.radius;
                server.hover_cursor = None;
                server.hover_cursor_3d = None;
                if let Some(map) = project.get_map_mut(server) {
                    map.clear_selection();
                    map.curr_rectangle = None;
                }
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), self.info()));
                true
            }
            ToolEvent::DeActivate => {
                if let Some(map) = project.get_map_mut(server) {
                    if let Some(atom) = self.finish(map, server.pc) {
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                }
                self.hover = None;
                self.panel_down = false;
                server.hover_cursor = None;
                server.hover_cursor_3d = None;
                server.curr_map_tool_type = MapToolType::General;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match event {
            MapEvent::MapClicked(at) => {
                if Self::panel().contains(at) {
                    self.panel_down = true;
                    self.panel_click(at);
                    self.set_hover(None, map, server, ctx);
                    return None;
                }
                let p = Self::pointer(ui, server, map, at)?;
                self.stroke = Some(Stroke {
                    before: map.clone(),
                    changed: false,
                    last: p,
                    mode: self.mode_at(ui),
                    tick: Instant::now(),
                    dirty: FxHashSet::default(),
                    published: None,
                });
                self.dab(map, p, self.strength * 0.1, server);
                self.publish(map, false);
                self.set_hover(Some(p), map, server, ctx);
            }
            MapEvent::MapDragged(at) => {
                if self.panel_down {
                    return None;
                }
                let Some(p) = Self::pointer(ui, server, map, at) else {
                    return None;
                };
                let mode = self.mode_at(ui);
                if let Some(stroke) = &mut self.stroke {
                    stroke.mode = mode;
                    let from = stroke.last;
                    let spacing = (self.radius * 0.15).clamp(0.1, 0.5);
                    let steps = ((p - from).magnitude() / spacing).ceil().clamp(1., 512.) as usize;
                    let distance = (p - from).magnitude();
                    stroke.last = p;
                    stroke.tick = Instant::now();
                    if distance > 0.001 {
                        for i in 1..=steps {
                            self.dab(
                                map,
                                from + (p - from) * (i as f32 / steps as f32),
                                self.strength * distance / steps as f32 * 0.35,
                                server,
                            );
                        }
                    }
                    self.publish(map, false);
                }
                self.set_hover(Some(p), map, server, ctx);
            }
            MapEvent::MapHover(at) => {
                let p = if Self::panel().contains(at) {
                    None
                } else {
                    Self::pointer(ui, server, map, at)
                };
                self.set_hover(p, map, server, ctx);
            }
            MapEvent::MapUp(_) => {
                self.panel_down = false;
                return self.finish(map, server.pc);
            }
            MapEvent::MapEscape => {
                self.cancel(map);
                self.set_hover(None, map, server, ctx);
                self.panel_down = false;
            }
            MapEvent::MapKey(key) => {
                match key {
                    '1' => self.mode = SculptMode::Raise,
                    '2' => self.mode = SculptMode::Lower,
                    '3' => self.mode = SculptMode::Smooth,
                    '[' => self.radius = (self.radius - 0.5).max(0.5),
                    ']' => self.radius = (self.radius + 0.5).min(16.),
                    _ => return None,
                }
                self.set_hover(self.hover, map, server, ctx);
            }
            _ => {}
        }
        None
    }

    fn update(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server: &mut ServerContext,
    ) -> bool {
        if self.stroke.as_ref().is_some_and(|s| s.before.id != map.id) {
            self.stroke = None;
        }
        let mode = self.mode_at(ui);
        if let Some(stroke) = &mut self.stroke {
            let elapsed = stroke.tick.elapsed().as_secs_f32();
            if elapsed >= 0.06 {
                stroke.tick = Instant::now();
                stroke.mode = mode;
                let at = stroke.last;
                self.dab(map, at, self.strength * elapsed.min(0.15), server);
                self.publish(map, false);
                self.set_hover(Some(at), map, server, ctx);
                return true;
            }
        }
        false
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server: &mut ServerContext,
        _assets: &Assets,
    ) {
        if server.editor_view_mode == EditorViewMode::D2 {
            if let Some(center) = self.hover {
                let size = Vec2::new(buffer.dim().width as f32, buffer.dim().height as f32);
                let points = Self::outline(map, center, self.radius);
                let project = |p: Vec3<f32>| {
                    if map.camera == MapCamera::TwoDIso {
                        rusterix::isometric::Projection::editor(map, size)
                            .screen(Vec2::new(p.x, p.z), p.y)
                    } else {
                        Vec2::new(p.x, p.z) * map.grid_size
                            + size * 0.5
                            + Vec2::new(map.offset.x, -map.offset.y)
                    }
                };
                for pair in points.windows(2) {
                    rusterix::isometric::line(
                        buffer,
                        project(pair[0]),
                        project(pair[1]),
                        [235, 205, 135, 240],
                    );
                }
            }
        }
        if buffer.dim().width < 250 || buffer.dim().height < 150 {
            return;
        }
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &Self::panel().to_buffer_utuple(),
            stride,
            &[24, 29, 28, 245],
        );
        let mut label = |rect: TheDim, text: &str, selected: bool| {
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &if selected {
                    [82, 89, 71, 255]
                } else {
                    [46, 52, 49, 255]
                },
            );
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                text,
                TheFontSettings {
                    size: 12.,
                    ..Default::default()
                },
                &[235, 235, 223, 255],
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        };
        for (i, (name, mode)) in [
            ("Raise", SculptMode::Raise),
            ("Lower", SculptMode::Lower),
            ("Smooth", SculptMode::Smooth),
        ]
        .into_iter()
        .enumerate()
        {
            label(
                TheDim::rect(14 + i as i32 * 74, 36, 70, 26),
                name,
                self.mode == mode,
            );
        }
        for (y, text) in [
            (68, format!("Radius: {:.1}", self.radius)),
            (100, format!("Strength: {:.2}", self.strength)),
        ] {
            label(TheDim::rect(14, y, 144, 26), &text, false);
            label(TheDim::rect(164, y, 30, 26), "−", false);
            label(TheDim::rect(200, y, 30, 26), "+", false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "native editor scene publication and undo require Metal"]
    fn terrain_brush_native_strokes_hold_drag_cancel_and_undo() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../generated");
        let mut project = shared::project_io::decode_project(
            &std::fs::read(root.join("mz-iso2d-woodland/MZWoodland.eldiron")).unwrap(),
        )
        .unwrap();
        let mut server = ServerContext::default();
        server.curr_region = project.regions[0].id;
        server.pc = ProjectContext::Region(server.curr_region);
        server.editor_view_mode = EditorViewMode::D2;
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 700, 1.);
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::sized(1000, 700), &mut ctx);
        ui.canvas.set_widget(view);
        RUSTERIX.write().unwrap().set_tiles_for_maps(
            project.tiles.clone(),
            false,
            [&project.regions[0].map],
        );
        let mut tool = TerrainTool::new();
        tool.tool_event(
            ToolEvent::Activate,
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        let center = Vec2::new(6.5, 13.5);
        let size = Vec2::new(1000., 700.);
        server.center_map_at_grid_pos(size, center, &mut project.regions[0].map);
        let original = project.regions[0].map.clone();
        let height = |map: &Map| {
            TerrainSurface::from_map(map)
                .unwrap()
                .height_at(center)
                .unwrap()
        };
        let click = |map: &Map, point: Vec2<f32>| {
            let h = TerrainSurface::from_map(map)
                .unwrap()
                .height_at(point)
                .unwrap_or(0.);
            rusterix::isometric::Projection::editor(map, size)
                .screen(point, h)
                .map(|v| v.round() as i32)
        };
        let at = click(&original, center);
        let before = height(&original);
        tool.map_event(
            MapEvent::MapClicked(at),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        assert!(height(&project.regions[0].map) > before + 0.09);
        tool.stroke.as_mut().unwrap().tick = Instant::now() - Duration::from_millis(100);
        assert!(tool.update(&mut ui, &mut ctx, &mut project.regions[0].map, &mut server));
        let after_hold = height(&project.regions[0].map);
        assert!(after_hold > before + 0.18);
        let drag_point = center + Vec2::new(1., 0.);
        let drag = click(&project.regions[0].map, drag_point);
        tool.map_event(
            MapEvent::MapDragged(drag),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        let undo = tool
            .map_event(
                MapEvent::MapUp(drag),
                &mut ui,
                &mut ctx,
                &mut project.regions[0].map,
                &mut server,
            )
            .unwrap();
        assert!(tool.stroke.is_none());
        let raised = height(&project.regions[0].map);
        assert!(raised > after_hold);
        assert_eq!(project.regions[0].map.vertices, original.vertices);
        assert_eq!(project.regions[0].map.sectors.len(), original.sectors.len());
        assert_eq!(
            project.regions[0].map.properties.get("tiles"),
            original.properties.get("tiles")
        );
        undo.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert!((height(&project.regions[0].map) - before).abs() < 0.0001);
        undo.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert!((height(&project.regions[0].map) - raised).abs() < 0.0001);
        let at = click(&project.regions[0].map, center);
        ui.shift = true;
        tool.map_event(
            MapEvent::MapClicked(at),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        assert!(height(&project.regions[0].map) < raised - 0.09);
        tool.map_event(
            MapEvent::MapEscape,
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        assert!((height(&project.regions[0].map) - raised).abs() < 0.0001);
        ui.shift = false;
        // Clicking brush settings changes no terrain and creates no undo.
        let snapshot = project.regions[0].map.properties.clone();
        tool.map_event(
            MapEvent::MapClicked(Vec2::new(200, 48)),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        assert_eq!(tool.mode, SculptMode::Smooth);
        assert!(
            tool.map_event(
                MapEvent::MapUp(Vec2::new(200, 48)),
                &mut ui,
                &mut ctx,
                &mut project.regions[0].map,
                &mut server
            )
            .is_none()
        );
        assert_eq!(project.regions[0].map.properties, snapshot);
        let at = click(&project.regions[0].map, center);
        tool.map_event(
            MapEvent::MapClicked(at),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        assert!(height(&project.regions[0].map) < raised);
        assert!(
            tool.map_event(
                MapEvent::MapUp(at),
                &mut ui,
                &mut ctx,
                &mut project.regions[0].map,
                &mut server
            )
            .is_some()
        );

        // 3D picking follows the same sculpted surface without hitting props.
        server.editor_view_mode = EditorViewMode::Iso;
        server.hover_ray_origin_3d = Some(Vec3::new(center.x, 10., center.y));
        server.hover_ray_dir_3d = Some(Vec3::new(0., -1., 0.));
        let picked =
            TerrainTool::pointer(&mut ui, &server, &project.regions[0].map, Vec2::zero()).unwrap();
        assert!((picked - center).magnitude() < 0.0001);
        server.editor_view_mode = EditorViewMode::D2;
        let at = click(&project.regions[0].map, center);
        tool.map_event(
            MapEvent::MapHover(at),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        let mut buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
        let mut assets = Assets::default();
        assets.set_tiles(project.tiles.clone());
        let map = &mut project.regions[0].map;
        rusterix::isometric::draw_scene(
            map,
            &assets,
            &mut buffer,
            rusterix::isometric::Projection::editor(map, size),
            0.,
            None,
        );
        tool.draw_hud(&mut buffer, map, &mut ctx, &mut server, &assets);
        let out = root.join("terrain-brush");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("editor.png"), buffer.to_png().unwrap()).unwrap();
        let encoded = shared::project_io::encode_project(&project).unwrap();
        let reopened = shared::project_io::decode_project(&encoded).unwrap();
        assert!(
            (height(&reopened.regions[0].map) - height(&project.regions[0].map)).abs() < 0.0001
        );
        std::fs::write(out.join("BrushTest.eldiron"), encoded).unwrap();

        // Empty maps get visible ground, without needing a pre-authored vertex.
        let mut empty = Map::default();
        empty.camera = MapCamera::TwoDIso;
        assert!(TerrainSculpt::dab(
            &mut empty,
            Vec2::zero(),
            2.,
            0.2,
            SculptMode::Raise
        ));
        server.curr_tile_source = None;
        server.curr_tile_id = None;
        TerrainTool::ensure_ground(&mut empty, &server);
        let r = RUSTERIX.read().unwrap();
        assert!(
            empty
                .properties
                .get_source("default_terrain_tile")
                .unwrap()
                .tile_from_tile_list(&r.assets)
                .is_some()
        );
    }
}
