use crate::D2Builder;
use crate::prelude::*;
use num_traits::zero;
use theframework::prelude::*;
use vek::Vec2;

pub struct ScreenWidget {
    pub builder_d2: D2Builder,

    pub position: Vec2<i32>,
    pub size: Vec2<f32>,

    pub scene: Scene,

    pub buffer: TheRGBABuffer,
    pub offset: Vec2<f32>,

    pub grid_size: f32,
    pub background_color: [u8; 4],
}

impl Default for ScreenWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenWidget {
    pub fn new() -> Self {
        Self {
            builder_d2: D2Builder::new(),

            position: Vec2::zero(),
            size: Vec2::zero(),

            scene: Scene::default(),

            buffer: TheRGBABuffer::default(),
            offset: zero(),

            grid_size: 32.0,
            background_color: [0, 0, 0, 255],
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets) {
        let animation_frame = self.scene.animation_frame;
        self.scene = self.builder_d2.build(
            map,
            assets,
            Vec2::new(
                self.buffer.dim().width as f32,
                self.buffer.dim().height as f32,
            ),
        );
        self.scene.animation_frame = animation_frame;
    }

    pub fn draw(&mut self, map: &Map, time: &TheTime, assets: &Assets) {
        self.draw_d2(map, time, assets);
    }

    pub fn draw_transparent(&mut self, map: &Map, time: &TheTime, assets: &Assets) {
        self.buffer.fill([0, 0, 0, 0]);
        self.draw_d2_with_background(map, time, assets, [0, 0, 0, 0]);
    }

    /// Draw the 2D scene.
    pub fn draw_d2(&mut self, map: &Map, time: &TheTime, assets: &Assets) {
        self.draw_d2_with_background(map, time, assets, self.background_color);
    }

    fn draw_d2_with_background(
        &mut self,
        _map: &Map,
        _time: &TheTime,
        assets: &Assets,
        background_color: [u8; 4],
    ) {
        let width = self.buffer.dim().width as usize;
        let height = self.buffer.dim().height as usize;

        // Widget-only screens have no base geometry. Rasterizing their blank
        // background otherwise allocates color/depth buffers and wakes Rayon
        // workers for every tile on every frame.
        if self.scene.d2_static.is_empty()
            && self.scene.d2_dynamic.is_empty()
            && self.scene.d3_static.is_empty()
            && self.scene.d3_dynamic.is_empty()
            && self.scene.d3_overlay.is_empty()
            && self.scene.chunks.is_empty()
        {
            self.buffer.fill(background_color);
            return;
        }

        // Offset is in grid units; translate so the grid cell at offset lands on pixel (0,0)
        let translation_matrix =
            Mat3::<f32>::translation_2d(Vec2::new(-self.offset.x, -self.offset.y) * self.grid_size);

        let scale_matrix = Mat3::new(
            self.grid_size,
            0.0,
            0.0,
            0.0,
            self.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;

        let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity());
        rast.preserve_transparency = true;
        rast.mapmini = self.scene.mapmini.clone();
        rast.render_mode = RenderMode::render_2d();
        rast.background_color = Some(background_color);

        rast.rasterize(
            &mut self.scene,
            self.buffer.pixels_mut(),
            width,
            height,
            40,
            assets,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_screen_clears_old_pixels_with_exact_background_alpha() {
        let mut widget = ScreenWidget::new();
        widget.buffer.resize(12, 8);
        let map = Map::default();
        let assets = Assets::default();
        for color in [[13, 27, 51, 255], [70, 20, 90, 127], [0, 0, 0, 0]] {
            widget.buffer.fill([255; 4]);
            widget.background_color = color;
            widget.draw(&map, &TheTime::new(), &assets);
            assert!(
                widget
                    .buffer
                    .pixels()
                    .chunks_exact(4)
                    .all(|pixel| pixel == color)
            );
        }
        widget.buffer.fill([255; 4]);
        widget.draw_transparent(&map, &TheTime::new(), &assets);
        assert!(widget.buffer.pixels().iter().all(|value| *value == 0));
    }
}
