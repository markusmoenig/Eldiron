//! CPU visibility coverage, independent of a destination framebuffer or renderer.
//! Reference pixels and GPU paint consume the same ordered logical rectangles.
use crate::{Map, MapMini};
use vek::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct VisibilityView2D {
    pub size: (u32, u32),
    pub grid_size: f32,
    pub top_left: Vec2<f32>,
    pub player_pos: Vec2<f32>,
    pub range: f32,
}
impl VisibilityView2D {
    pub fn editor(map: &Map, size: (u32, u32), range: f32) -> Self {
        let screen = Vec2::new(size.0 as f32, size.1 as f32);
        Self {
            size,
            grid_size: map.grid_size,
            top_left: Vec2::new(
                (-screen.x / 2.0 - map.offset.x) / map.grid_size,
                (map.offset.y - screen.y / 2.0) / map.grid_size,
            ),
            player_pos: map
                .entities
                .iter()
                .find(|entity| entity.is_player())
                .map(|entity| entity.get_pos_xz())
                .unwrap_or_else(Vec2::zero),
            range,
        }
    }
    /// Overlapping floor/ceil cells intentionally remain separate: each performs
    /// one blend in the reference path, including at fractional grid boundaries.
    pub fn visit_hidden_rects(&self, mapmini: &MapMini, mut visit: impl FnMut([u32; 4])) {
        let Self {
            size: (width, height),
            grid_size,
            top_left,
            player_pos,
            range,
        } = *self;
        if width == 0
            || height == 0
            || grid_size <= 0.
            || range <= 0.
            || !grid_size.is_finite()
            || !range.is_finite()
            || !top_left.x.is_finite()
            || !top_left.y.is_finite()
            || !player_pos.x.is_finite()
            || !player_pos.y.is_finite()
        {
            return;
        }
        let start_x = top_left.x.floor() as i32 - 1;
        let start_y = top_left.y.floor() as i32 - 1;
        let end_x = (top_left.x + width as f32 / grid_size).ceil() as i32 + 1;
        let end_y = (top_left.y + height as f32 / grid_size).ceil() as i32 + 1;
        let from_tile = player_pos.map(|v| v.floor() as i32);

        for tile_y in start_y..end_y {
            for tile_x in start_x..end_x {
                let center = Vec2::new(tile_x as f32 + 0.5, tile_y as f32 + 0.5);
                let to_tile = Vec2::new(tile_x, tile_y);
                if (center - player_pos).magnitude() <= range
                    && mapmini.is_tile_visible(from_tile, to_tile)
                {
                    continue;
                }

                let x0 = (((tile_x as f32) - top_left.x) * grid_size).floor() as i32;
                let y0 = (((tile_y as f32) - top_left.y) * grid_size).floor() as i32;
                let x1 = ((((tile_x + 1) as f32) - top_left.x) * grid_size).ceil() as i32;
                let y1 = ((((tile_y + 1) as f32) - top_left.y) * grid_size).ceil() as i32;

                let x0 = x0.clamp(0, width as i32) as u32;
                let y0 = y0.clamp(0, height as i32) as u32;
                let x1 = x1.clamp(0, width as i32) as u32;
                let y1 = y1.clamp(0, height as i32) as u32;

                if x0 >= x1 || y0 >= y1 {
                    continue;
                }

                visit([x0, y0, x1 - x0, y1 - y0]);
            }
        }
    }
}
