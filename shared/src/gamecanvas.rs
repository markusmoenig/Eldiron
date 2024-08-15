use crate::prelude::*;
use theframework::prelude::*;

pub struct GameCanvas {
    pub canvas: TheRGBBuffer,

    pub distance_canvas: TheFlattenedMap<half::f16>,
    pub lights_canvas: TheFlattenedMap<Vec<PreRenderedLight>>,
}

impl Default for GameCanvas {
    fn default() -> Self {
        Self::empty()
    }
}

impl GameCanvas {
    pub fn empty() -> Self {
        Self {
            canvas: TheRGBBuffer::empty(),
            distance_canvas: TheFlattenedMap::new(0, 0),
            lights_canvas: TheFlattenedMap::new(0, 0),
        }
    }

    pub fn stitch_from_ortho_tiles(&mut self, region: &Region) {
        let tile_size = region.tile_size;
        let tile_size_half = tile_size;

        //region.regionfx.cam_render_into(self);

        self.canvas
            .resize(tile_size * region.width / 2, tile_size * region.height / 3);

        let sx = tile_size * region.width / 3 - 4 * tile_size;
        let sy = -tile_size * region.height / 4;

        let mut keys: Vec<Vec2i> = region.prerendered.tiles.keys().cloned().collect();

        keys.sort_by(|a, b| {
            let sum_a = a.x + a.y;
            let sum_b = b.x + b.y;
            if sum_a == sum_b {
                a.x.cmp(&b.x)
            } else {
                sum_a.cmp(&sum_b)
            }
        });

        for key in keys {
            if let Some(tile) = &region.prerendered.tiles.get(&key) {
                let x = sx + (key.x - key.y) * tile_size_half;
                let y = sy + (key.x + key.y) * (tile_size_half / 2);

                self.canvas.copy_into(x, y, &tile.albedo);
            }
        }
    }
}
