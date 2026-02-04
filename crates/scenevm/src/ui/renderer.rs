use super::drawable::Drawable;
use super::style::{StyleParams, StyleRegistry};
use super::text::TextCache;
use crate::{
    Embedded,
    poly2d::Poly2D,
    vm::{Atom, GeoId, VM},
};

/// Renders UI drawables into the 2D layer by emitting quads.
pub struct UiRenderer {
    styles: StyleRegistry,
    text: TextCache,
    next_id: u32,
    tinted_tiles_cache: std::collections::HashMap<(uuid::Uuid, [u8; 16]), uuid::Uuid>, // (base_tile_id, tint_bytes) -> tinted_tile_id
}

impl UiRenderer {
    pub fn new() -> Self {
        let font_bytes = Embedded::get("ui_font.ttf").map(|d| d.data.to_vec());
        Self {
            styles: StyleRegistry::new(),
            text: TextCache::new(font_bytes),
            next_id: 0,
            tinted_tiles_cache: std::collections::HashMap::new(),
        }
    }

    pub fn text_cache(&self) -> &TextCache {
        &self.text
    }

    fn alloc_id(&mut self) -> GeoId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        GeoId::Unknown(id)
    }

    /// Emit drawables into the current chunk as 2D polys.
    pub fn render(&mut self, vm: &mut VM, drawables: &[Drawable]) {
        self.render_internal(vm, drawables, true);
    }

    /// Emit drawables without clearing geometry (for rendering to separate layers).
    /// Used when rendering popups to a different VM layer.
    pub fn render_no_clear(&mut self, vm: &mut VM, drawables: &[Drawable]) {
        self.render_internal(vm, drawables, false);
    }

    fn render_internal(&mut self, vm: &mut VM, drawables: &[Drawable], clear: bool) {
        if clear {
            // Wipe previous UI geometry so we don't accumulate quads across frames.
            vm.execute(Atom::ClearGeometry);
        }

        for d in drawables {
            match d {
                Drawable::Quad {
                    tile_id,
                    rect,
                    uv,
                    layer,
                    tint,
                    ..
                } => {
                    let verts = quad_verts(*rect);

                    // Create a tile with tint stored in material texture
                    // The material will store RGBA tint color that the shader can read
                    let tinted_tile_id = self.ensure_tinted_tile(vm, *tile_id, *tint);

                    let poly = Poly2D::poly(
                        self.alloc_id(),
                        tinted_tile_id,
                        verts,
                        uv.to_vec(),
                        vec![(0, 1, 2), (0, 2, 3)],
                    )
                    .with_layer(*layer);
                    vm.execute(Atom::AddPoly { poly });
                }
                Drawable::Rect {
                    rect,
                    fill,
                    border,
                    radius_px,
                    border_px,
                    layer,
                    ..
                } => {
                    let verts = quad_verts(*rect);
                    let style = StyleParams {
                        fill: *fill,
                        border: *border,
                        radius_px: *radius_px,
                        border_px: *border_px,
                    };
                    let style_id = self.styles.ensure_style(vm, style);
                    let tile_id = self.styles.tile_id(style_id).expect("missing style tile");
                    let uv = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
                    let poly = Poly2D::poly(
                        self.alloc_id(),
                        tile_id,
                        verts,
                        uv,
                        vec![(0, 1, 2), (0, 2, 3)],
                    )
                    .with_layer(*layer);
                    vm.execute(Atom::AddPoly { poly });
                }
                Drawable::Text {
                    id: _,
                    text,
                    origin,
                    px_size,
                    color,
                    layer,
                } => {
                    let glyphs = self.text.layout_positions(text, *px_size);
                    let start_x = origin[0];
                    let start_y = origin[1];
                    for g in glyphs {
                        let Some(entry) = self.text.ensure_glyph(vm, g.parent, *px_size, *color)
                        else {
                            continue;
                        };
                        // Layout gives glyph bounds at (x, y) with width/height.
                        let x0 = start_x + g.x;
                        let y0 = start_y + g.y;
                        let w = g.width as f32;
                        let h = g.height as f32;
                        if w <= 0.0 || h <= 0.0 {
                            continue;
                        }
                        let verts = vec![[x0, y0], [x0 + w, y0], [x0 + w, y0 + h], [x0, y0 + h]];
                        let uv = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
                        let poly = Poly2D::poly(
                            self.alloc_id(),
                            entry.tile_id,
                            verts,
                            uv,
                            vec![(0, 1, 2), (0, 2, 3)],
                        )
                        .with_layer(*layer);
                        vm.execute(Atom::AddPoly { poly });
                    }
                }
            }
        }
        self.styles.build_if_dirty(vm);
        self.text.build_if_dirty(vm);
    }

    /// Create a tinted version of a tile by copying and tinting the pixel data
    fn ensure_tinted_tile(
        &mut self,
        vm: &mut VM,
        base_tile_id: uuid::Uuid,
        tint: vek::Vec4<f32>,
    ) -> uuid::Uuid {
        // If tint is white (no tint), just return the original tile
        if (tint.x - 1.0).abs() < 0.001
            && (tint.y - 1.0).abs() < 0.001
            && (tint.z - 1.0).abs() < 0.001
            && (tint.w - 1.0).abs() < 0.001
        {
            return base_tile_id;
        }

        // Create a cache key from the tint color bytes
        let mut tint_bytes = [0u8; 16];
        tint_bytes[0..4].copy_from_slice(&tint.x.to_le_bytes());
        tint_bytes[4..8].copy_from_slice(&tint.y.to_le_bytes());
        tint_bytes[8..12].copy_from_slice(&tint.z.to_le_bytes());
        tint_bytes[12..16].copy_from_slice(&tint.w.to_le_bytes());

        let cache_key = (base_tile_id, tint_bytes);

        // Check if we already created this tinted tile
        if let Some(&cached_id) = self.tinted_tiles_cache.get(&cache_key) {
            return cached_id;
        }

        // Get the original tile data
        let tile_data = vm.get_tile_data(base_tile_id);
        if tile_data.is_none() {
            return base_tile_id;
        }

        let (width, height, rgba_data) = tile_data.unwrap();

        // Create tinted copy by multiplying each pixel by the tint color
        let mut tinted_data = rgba_data.clone();
        for i in (0..tinted_data.len()).step_by(4) {
            tinted_data[i] = ((tinted_data[i] as f32 / 255.0 * tint.x) * 255.0) as u8; // R
            tinted_data[i + 1] = ((tinted_data[i + 1] as f32 / 255.0 * tint.y) * 255.0) as u8; // G
            tinted_data[i + 2] = ((tinted_data[i + 2] as f32 / 255.0 * tint.z) * 255.0) as u8; // B
            tinted_data[i + 3] = ((tinted_data[i + 3] as f32 / 255.0 * tint.w) * 255.0) as u8; // A
        }

        // Add the tinted tile with a consistent ID
        let tinted_tile_id = uuid::Uuid::new_v4();
        vm.execute(Atom::AddTile {
            id: tinted_tile_id,
            width,
            height,
            frames: vec![tinted_data],
            material_frames: Some(vec![super::create_tile_material(width, height)]),
        });
        vm.execute(Atom::BuildAtlas);

        // Cache the tinted tile ID
        self.tinted_tiles_cache.insert(cache_key, tinted_tile_id);

        tinted_tile_id
    }
}

fn quad_verts(rect: [f32; 4]) -> Vec<[f32; 2]> {
    let [x, y, w, h] = rect;
    vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
}
