use crate::{
    Assets, Batch2D, Map, MapToolType, PixelSource, Rect, Scene, Surface, ValueContainer, WHITE,
    scene_handler::SceneHandler,
};
use scenevm::GeoId;
use vek::{Vec2, Vec3};

pub struct D2ConceptBuilder {
    map_tool_type: MapToolType,
    hover: (Option<u32>, Option<u32>, Option<u32>),
    hover_cursor: Option<Vec2<f32>>,
    camera_pos: Option<Vec3<f32>>,
    look_at: Option<Vec3<f32>>,
    clip_rect: Option<Rect>,
}

impl Default for D2ConceptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2ConceptBuilder {
    pub fn new() -> Self {
        Self {
            map_tool_type: MapToolType::Dungeon,
            hover: (None, None, None),
            hover_cursor: None,
            camera_pos: None,
            look_at: None,
            clip_rect: None,
        }
    }

    pub fn build(
        &mut self,
        map: &Map,
        _assets: &Assets,
        _screen_size: Vec2<f32>,
        _properties: &ValueContainer,
    ) -> Scene {
        let mut scene = Scene::empty();

        if let Some(layer) = map.dungeon.active_layer() {
            let mut fill = Batch2D::empty().source(PixelSource::Pixel([138, 138, 138, 168]));
            let mut floor_inner = Batch2D::empty().source(PixelSource::Pixel([166, 166, 166, 110]));
            let mut outlines = Batch2D::empty()
                .source(PixelSource::Pixel([196, 196, 196, 180]))
                .mode(crate::PrimitiveMode::Lines);
            let mut glyphs = Batch2D::empty()
                .source(PixelSource::Pixel([248, 242, 218, 255]))
                .mode(crate::PrimitiveMode::Lines);

            for cell in &layer.cells {
                fill.add_rectangle(cell.x as f32, cell.y as f32, 1.0, 1.0);
                floor_inner.add_rectangle(cell.x as f32 + 0.18, cell.y as f32 + 0.18, 0.64, 0.64);
                outlines.add_line(
                    Vec2::new(cell.x as f32, cell.y as f32),
                    Vec2::new(cell.x as f32 + 1.0, cell.y as f32),
                    0.05,
                );
                outlines.add_line(
                    Vec2::new(cell.x as f32 + 1.0, cell.y as f32),
                    Vec2::new(cell.x as f32 + 1.0, cell.y as f32 + 1.0),
                    0.05,
                );
                outlines.add_line(
                    Vec2::new(cell.x as f32 + 1.0, cell.y as f32 + 1.0),
                    Vec2::new(cell.x as f32, cell.y as f32 + 1.0),
                    0.05,
                );
                outlines.add_line(
                    Vec2::new(cell.x as f32, cell.y as f32 + 1.0),
                    Vec2::new(cell.x as f32, cell.y as f32),
                    0.05,
                );
                self.add_cell_glyph(&mut glyphs, cell.x as f32, cell.y as f32, cell.kind);
            }

            scene.d2_static.push(fill);
            scene.d2_static.push(floor_inner);
            scene.d2_dynamic.push(outlines);
            scene.d2_dynamic.push(glyphs);
        }

        if let Some(cursor) = self.hover_cursor {
            let mut cursor_batch = Batch2D::empty()
                .source(PixelSource::Pixel(WHITE))
                .mode(crate::PrimitiveMode::Lines);
            let x = cursor.x.floor();
            let y = cursor.y.floor();
            cursor_batch.add_line(Vec2::new(x, y), Vec2::new(x + 1.0, y), 0.04);
            cursor_batch.add_line(Vec2::new(x + 1.0, y), Vec2::new(x + 1.0, y + 1.0), 0.04);
            cursor_batch.add_line(Vec2::new(x + 1.0, y + 1.0), Vec2::new(x, y + 1.0), 0.04);
            cursor_batch.add_line(Vec2::new(x, y + 1.0), Vec2::new(x, y), 0.04);
            scene.d2_dynamic.push(cursor_batch);
        }

        scene
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_entities_items(
        &self,
        map: &Map,
        _assets: &Assets,
        _scene: &mut Scene,
        _screen_size: Vec2<f32>,
        _editing_surface: &Option<Surface>,
        scene_handler: &mut SceneHandler,
        _draw_sectors: bool,
    ) {
        scene_handler.clear_overlay();

        let reference_base = map
            .dungeon
            .active_layer()
            .map(|layer| layer.floor_base)
            .unwrap_or(0.0);
        let reference_band = 0.35;

        for surface in map.surfaces.values() {
            if surface.plane.normal.y.abs() >= 0.25 {
                continue;
            }
            let Some(sector) = map.find_sector(surface.sector_id) else {
                continue;
            };
            if sector
                .properties
                .get_str_default("generated_by", String::new())
                == "dungeon_tool"
            {
                continue;
            }
            let floor_base = sector.properties.get_float_default(
                "floor_height",
                sector.properties.get_float_default("floor_base", 0.0),
            );
            if (floor_base - reference_base).abs() > reference_band {
                continue;
            }
            let Some(world_vertices) = sector.vertices_world(map) else {
                continue;
            };
            let mut points: Vec<Vec2<f32>> = Vec::new();
            for p in &world_vertices {
                let q = Vec2::new(p.x, p.z);
                if points
                    .last()
                    .is_none_or(|last| (*last - q).magnitude() > 0.01)
                {
                    points.push(q);
                }
            }
            if points.len() < 2 {
                continue;
            }

            let mut best = None;
            let mut best_dist = 0.0;
            for i in 0..points.len() {
                for j in i + 1..points.len() {
                    let dist = (points[i] - points[j]).magnitude_squared();
                    if dist > best_dist {
                        best_dist = dist;
                        best = Some((points[i], points[j]));
                    }
                }
            }
            if let Some((start, end)) = best {
                scene_handler.add_overlay_2d_line(
                    GeoId::Unknown(40_000 + surface.sector_id),
                    start,
                    end,
                    scene_handler.gray,
                    8_900,
                );
            }
        }

        if let Some(layer) = map.dungeon.active_layer() {
            for (index, cell) in layer.cells.iter().enumerate() {
                let center = Vec2::new(cell.x as f32 + 0.5, cell.y as f32 + 0.5);
                let left = Vec2::new(cell.x as f32, cell.y as f32);
                let right = Vec2::new(cell.x as f32 + 1.0, cell.y as f32);
                let bottom_right = Vec2::new(cell.x as f32 + 1.0, cell.y as f32 + 1.0);
                let bottom_left = Vec2::new(cell.x as f32, cell.y as f32 + 1.0);
                let inner_top_left = Vec2::new(cell.x as f32 + 0.16, cell.y as f32 + 0.16);
                let inner_top_right = Vec2::new(cell.x as f32 + 0.84, cell.y as f32 + 0.16);
                let inner_bottom_right = Vec2::new(cell.x as f32 + 0.84, cell.y as f32 + 0.84);
                let inner_bottom_left = Vec2::new(cell.x as f32 + 0.16, cell.y as f32 + 0.84);

                scene_handler.overlay_2d.add_square_2d(
                    GeoId::Unknown(5_000 + index as u32),
                    scene_handler.gray,
                    [center.x, center.y],
                    1.0,
                    9_000,
                    true,
                );

                scene_handler.add_overlay_2d_line(
                    GeoId::Unknown(10_000 + index as u32 * 4),
                    inner_top_left,
                    inner_top_right,
                    scene_handler.gray,
                    9_050,
                );
                scene_handler.add_overlay_2d_line(
                    GeoId::Unknown(10_001 + index as u32 * 4),
                    inner_top_right,
                    inner_bottom_right,
                    scene_handler.gray,
                    9_050,
                );
                scene_handler.add_overlay_2d_line(
                    GeoId::Unknown(10_002 + index as u32 * 4),
                    inner_bottom_right,
                    inner_bottom_left,
                    scene_handler.gray,
                    9_050,
                );
                scene_handler.add_overlay_2d_line(
                    GeoId::Unknown(10_003 + index as u32 * 4),
                    inner_bottom_left,
                    inner_top_left,
                    scene_handler.gray,
                    9_050,
                );

                if cell.kind.has_north() {
                    scene_handler.add_overlay_2d_line(
                        GeoId::Unknown(20_000 + index as u32 * 4),
                        left,
                        right,
                        scene_handler.white,
                        9_100,
                    );
                }
                if cell.kind.has_east() {
                    scene_handler.add_overlay_2d_line(
                        GeoId::Unknown(20_001 + index as u32 * 4),
                        right,
                        bottom_right,
                        scene_handler.white,
                        9_100,
                    );
                }
                if cell.kind.has_south() {
                    scene_handler.add_overlay_2d_line(
                        GeoId::Unknown(20_002 + index as u32 * 4),
                        bottom_left,
                        bottom_right,
                        scene_handler.white,
                        9_100,
                    );
                }
                if cell.kind.has_west() {
                    scene_handler.add_overlay_2d_line(
                        GeoId::Unknown(20_003 + index as u32 * 4),
                        left,
                        bottom_left,
                        scene_handler.white,
                        9_100,
                    );
                }
            }
        }

        if let Some(cursor) = self.hover_cursor {
            let x = cursor.x.floor();
            let y = cursor.y.floor();
            let top_left = Vec2::new(x, y);
            let top_right = Vec2::new(x + 1.0, y);
            let bottom_right = Vec2::new(x + 1.0, y + 1.0);
            let bottom_left = Vec2::new(x, y + 1.0);
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(30_000),
                top_left,
                top_right,
                scene_handler.white,
                9_200,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(30_001),
                top_right,
                bottom_right,
                scene_handler.white,
                9_200,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(30_002),
                bottom_right,
                bottom_left,
                scene_handler.white,
                9_200,
            );
            scene_handler.add_overlay_2d_line(
                GeoId::Unknown(30_003),
                bottom_left,
                top_left,
                scene_handler.white,
                9_200,
            );
        }

        scene_handler.set_overlay();
    }

    pub fn set_map_tool_type(&mut self, tool: MapToolType) {
        self.map_tool_type = tool;
    }

    pub fn set_map_hover_info(
        &mut self,
        hover: (Option<u32>, Option<u32>, Option<u32>),
        hover_cursor: Option<Vec2<f32>>,
    ) {
        self.hover = hover;
        self.hover_cursor = hover_cursor;
    }

    pub fn set_camera_info(&mut self, pos: Option<Vec3<f32>>, look_at: Option<Vec3<f32>>) {
        self.camera_pos = pos;
        self.look_at = look_at;
    }

    pub fn set_clip_rect(&mut self, clip_rect: Option<Rect>) {
        self.clip_rect = clip_rect;
    }

    fn add_cell_glyph(&self, batch: &mut Batch2D, x: f32, y: f32, kind: crate::DungeonTileKind) {
        let left = x + 0.18;
        let right = x + 0.82;
        let top = y + 0.18;
        let bottom = y + 0.82;
        if kind.has_north() {
            batch.add_line(Vec2::new(left, top), Vec2::new(right, top), 0.05);
        }
        if kind.has_east() {
            batch.add_line(Vec2::new(right, top), Vec2::new(right, bottom), 0.05);
        }
        if kind.has_south() {
            batch.add_line(Vec2::new(left, bottom), Vec2::new(right, bottom), 0.05);
        }
        if kind.has_west() {
            batch.add_line(Vec2::new(left, top), Vec2::new(left, bottom), 0.05);
        }
    }
}
