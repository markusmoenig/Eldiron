use crate::D2Builder;
use crate::prelude::*;
use num_traits::zero;
use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
};
use theframework::prelude::*;
use vek::Vec2;

struct CompositeLayer {
    key: u64,
    pixels: Vec<u8>,
    // Runs stay within a row; fully transparent areas have no run.
    spans: Vec<(usize, usize, bool)>,
}
impl CompositeLayer {
    fn new(key: u64, pixels: &[u8], width: usize) -> Self {
        let mut spans = Vec::new();
        for (y, row) in pixels.chunks_exact(width * 4).enumerate() {
            let mut x = 0;
            while x < width {
                if row[x * 4 + 3] == 0 {
                    x += 1;
                    continue;
                }
                let start = x;
                let opaque = row[x * 4 + 3] == 255;
                x += 1;
                while x < width && row[x * 4 + 3] != 0 && (row[x * 4 + 3] == 255) == opaque {
                    x += 1;
                }
                spans.push(((y * width + start) * 4, (x - start) * 4, opaque));
            }
        }
        Self {
            key,
            pixels: pixels.to_vec(),
            spans,
        }
    }
    fn blend(&self, target: &mut TheRGBABuffer) {
        let dest = target.pixels_mut();
        for &(offset, bytes, opaque) in &self.spans {
            let src = &self.pixels[offset..offset + bytes];
            let dest = &mut dest[offset..offset + bytes];
            if opaque {
                dest.copy_from_slice(src);
            } else {
                for (dest, src) in dest.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                    let mixed = theframework::thergbabuffer::source_over(
                        (&*dest).try_into().unwrap(),
                        src.try_into().unwrap(),
                        src[3] as f32 / 255.,
                    );
                    dest.copy_from_slice(&mixed);
                }
            }
        }
    }
}

pub struct ScreenWidget {
    pub builder_d2: D2Builder,

    pub position: Vec2<i32>,
    pub size: Vec2<f32>,

    pub scene: Scene,

    pub buffer: TheRGBABuffer,
    pub offset: Vec2<f32>,

    pub grid_size: f32,
    pub background_color: [u8; 4],
    // Base and foreground share this widget. Retain at most two raster layers;
    // copies into `buffer` keep caller hole-punching/overlays out of the cache.
    raster_cache: VecDeque<(u64, Vec<u8>)>,
    composite_cache: VecDeque<CompositeLayer>,
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
            raster_cache: VecDeque::new(),
            composite_cache: VecDeque::new(),
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

    /// Retain the screen's transparent holes and alpha runs as well as its raster.
    /// The target changes every frame, so only source pixels/coverage are retained.
    pub(crate) fn composite_cached(
        &mut self,
        target: &mut TheRGBABuffer,
        assets: &Assets,
        stamp: &impl Hash,
        prepare: impl FnOnce(&mut TheRGBABuffer),
    ) {
        let key = self
            .raster_key(assets, self.background_color)
            .filter(|_| {
                target.dim().width == self.buffer.dim().width
                    && target.dim().height == self.buffer.dim().height
                    && self.buffer.dim().width > 0
                    && target.render_scale() == 1.0
                    && self.buffer.render_scale() == 1.0
            })
            .map(|base| {
                let mut hash = rustc_hash::FxHasher::default();
                base.hash(&mut hash);
                stamp.hash(&mut hash);
                hash.finish()
            });
        if let Some(key) = key {
            if let Some(layer) = self.composite_cache.iter().find(|layer| layer.key == key) {
                self.buffer.pixels_mut().copy_from_slice(&layer.pixels);
                layer.blend(target);
                return;
            }
            prepare(&mut self.buffer);
            let layer =
                CompositeLayer::new(key, self.buffer.pixels(), self.buffer.dim().width as usize);
            layer.blend(target);
            if self.composite_cache.len() == 2 {
                self.composite_cache.pop_front();
            }
            self.composite_cache.push_back(layer);
        } else {
            prepare(&mut self.buffer);
            target.blend_into(0, 0, &self.buffer);
        }
    }

    fn raster_key(&self, assets: &Assets, background: [u8; 4]) -> Option<u64> {
        // Lighting can depend on animated flicker and occluders. Keep those
        // uncommon screen scenes on the normal raster path.
        if !self.scene.lights.is_empty()
            || !self.scene.dynamic_lights.is_empty()
            || !self.scene.d3_static.is_empty()
            || !self.scene.d3_dynamic.is_empty()
            || !self.scene.d3_overlay.is_empty()
            || !self.scene.chunks.is_empty()
        {
            return None;
        }
        let mut hash = rustc_hash::FxHasher::default();
        (
            self.buffer.dim().width,
            self.buffer.dim().height,
            background,
        )
            .hash(&mut hash);
        self.buffer.render_scale().to_bits().hash(&mut hash);
        self.buffer.len().hash(&mut hash);
        self.grid_size.to_bits().hash(&mut hash);
        self.offset.x.to_bits().hash(&mut hash);
        self.offset.y.to_bits().hash(&mut hash);
        for batches in [&self.scene.d2_static, &self.scene.d2_dynamic] {
            batches.len().hash(&mut hash);
            for batch in batches {
                // Projection/edge caches are outputs of rasterization, not inputs.
                std::mem::discriminant(&batch.mode).hash(&mut hash);
                std::mem::discriminant(&batch.repeat_mode).hash(&mut hash);
                hash.write(&serde_json::to_vec(&batch.source).ok()?);
                for v in batch.vertices.iter().chain(batch.uvs.iter()) {
                    v[0].to_bits().hash(&mut hash);
                    v[1].to_bits().hash(&mut hash);
                }
                batch.vertices.len().hash(&mut hash);
                batch.uvs.len().hash(&mut hash);
                batch.indices.hash(&mut hash);
                for v in batch.transform.as_col_slice() {
                    v.to_bits().hash(&mut hash);
                }
                let tile = match batch.source {
                    PixelSource::StaticTileIndex(index) => assets.tile_list.get(index as usize),
                    PixelSource::DynamicTileIndex(index) => {
                        self.scene.dynamic_textures.get(index as usize)
                    }
                    _ => return None,
                };
                if let Some(tile) = tile {
                    tile.id.hash(&mut hash);
                    tile.frame_index(self.scene.animation_frame).hash(&mut hash);
                    hash.write(&serde_json::to_vec(&tile.uv_pages).ok()?);
                    // Assets permit in-place pixel edits. A pointer/revision-only
                    // key would miss palette rebakes and refreshed screen materials.
                    tile.textures.len().hash(&mut hash);
                    for texture in &tile.textures {
                        (texture.width, texture.height).hash(&mut hash);
                        hash.write(&texture.data);
                    }
                }
            }
        }
        Some(hash.finish())
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

        let key = self.raster_key(assets, background_color);
        if let Some(key) = key
            && let Some((_, pixels)) = self.raster_cache.iter().find(|(cached, _)| *cached == key)
        {
            self.buffer.pixels_mut().copy_from_slice(pixels);
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
        if let Some(key) = key {
            if self.raster_cache.len() == 2 {
                self.raster_cache.pop_front();
            }
            self.raster_cache
                .push_back((key, self.buffer.pixels().to_vec()));
        }
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

#[cfg(test)]
mod raster_cache_tests {
    use super::*;
    #[test]
    fn sparse_composite_matches_source_over_and_reuses_prepared_holes() {
        let mut widget = ScreenWidget::new();
        widget.buffer.resize(16, 8);
        widget
            .scene
            .d2_static
            .push(Batch2D::from_rectangle(0., 0., 1., 1.).source(PixelSource::StaticTileIndex(0)));
        let mut assets = Assets::default();
        assets.tile_list.push(Tile::from_texture(Texture::white()));
        for alpha in 0..=255u8 {
            let color = [17, 81, 203, alpha];
            let mut reference = TheRGBABuffer::new(TheDim::sized(16, 8));
            reference.fill(color);
            let mut source = widget.buffer.clone();
            for (i, pixel) in source.pixels_mut().chunks_exact_mut(4).enumerate() {
                pixel.copy_from_slice(&[31, 127, 231, (i as u8).wrapping_mul(13)]);
            }
            source.pixels_mut()[8..64].fill(0);
            widget.buffer = source.clone();
            reference.blend_into(0, 0, &source);
            let mut actual = TheRGBABuffer::new(TheDim::sized(16, 8));
            actual.fill(color);
            widget.composite_cache.clear();
            widget.composite_cached(&mut actual, &assets, &0, |_| {});
            assert_eq!(actual.pixels(), reference.pixels());
            actual.fill(color);
            widget.composite_cached(&mut actual, &assets, &0, |_| {
                panic!("prepared layer should be reused")
            });
            assert_eq!(actual.pixels(), reference.pixels());
        }
        let mut actual = TheRGBABuffer::new(TheDim::sized(16, 8));
        widget.composite_cached(&mut actual, &assets, &1, |buf| buf.fill([0; 4]));
        assert_eq!(widget.composite_cache.len(), 2);
        assert!(actual.pixels().iter().all(|p| *p == 0));
    }

    #[test]
    fn retained_screen_invalidates_for_live_art_animation_layout_and_compositing() {
        let mut assets = Assets::default();
        assets.tile_list.push(Tile::from_textures(vec![
            Texture::from_color([80, 140, 200, 128]),
            Texture::from_color([220, 110, 40, 192]),
        ]));
        let mut widget = ScreenWidget::new();
        widget.buffer.resize(16, 16);
        widget.grid_size = 8.;
        widget
            .scene
            .d2_static
            .push(Batch2D::from_rectangle(0., 0., 2., 2.).source(PixelSource::StaticTileIndex(0)));
        let map = Map::default();
        let time = TheTime::new();
        widget.draw(&map, &time, &assets);
        let original = widget.buffer.pixels().to_vec();
        assert_eq!(widget.raster_cache.len(), 1);
        // Caller hole punching/overlays must never contaminate retained pixels.
        widget.buffer.fill([255; 4]);
        widget.draw(&map, &time, &assets);
        assert_eq!(widget.raster_cache.len(), 1);
        assert_eq!(widget.buffer.pixels(), &original);
        for change in 0..7 {
            match change {
                0 => widget.scene.animation_frame += 1,
                1 => assets.tile_list[0].textures[0]
                    .data
                    .copy_from_slice(&[11, 222, 33, 100]),
                2 => widget.offset.x = 0.5,
                3 => widget.scene.d2_static[0].vertices[0][0] = 0.25,
                4 => widget.background_color = [100, 30, 80, 60],
                5 => widget.buffer.resize(24, 16),
                _ => {
                    widget.buffer.set_render_scale(2.0);
                }
            }
            let before = widget.raster_key(&assets, widget.background_color).unwrap();
            assert!(!widget.raster_cache.iter().any(|(key, _)| *key == before));
            widget.draw(&map, &time, &assets);
            let retained = widget.buffer.pixels().to_vec();
            widget.raster_cache.clear();
            widget.draw(&map, &time, &assets);
            assert_eq!(retained, widget.buffer.pixels());
        }
        widget.draw_transparent(&map, &time, &assets);
        let transparent = widget.buffer.pixels().to_vec();
        widget.draw(&map, &time, &assets);
        widget.draw_transparent(&map, &time, &assets);
        assert_eq!(widget.buffer.pixels(), &transparent);
        assert_eq!(widget.raster_cache.len(), 2);
    }
}
