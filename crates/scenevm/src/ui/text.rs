use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
use rustc_hash::FxHashMap;
use uuid::Uuid;
use vek::Vec4;

use crate::{Atom, VM};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    ch: char,
    size_px: u32,
    color: [u8; 4],
}

#[derive(Debug, Clone)]
pub struct GlyphEntry {
    pub tile_id: Uuid,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub width: u32,
    pub height: u32,
}

pub struct TextCache {
    font: Option<fontdue::Font>,
    glyphs: FxHashMap<GlyphKey, GlyphEntry>,
    dirty: bool,
    font_missing_logged: bool,
}

impl TextCache {
    pub fn new(font_bytes: Option<Vec<u8>>) -> Self {
        let font = font_bytes.and_then(|bytes| {
            fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
        });
        Self {
            font,
            glyphs: FxHashMap::default(),
            dirty: false,
            font_missing_logged: false,
        }
    }

    fn to_key(ch: char, size_px: f32, color: Vec4<f32>) -> GlyphKey {
        let clamp = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        GlyphKey {
            ch,
            size_px: size_px.max(1.0).round() as u32,
            color: [
                clamp(color.x),
                clamp(color.y),
                clamp(color.z),
                clamp(color.w),
            ],
        }
    }

    pub fn ensure_glyph(
        &mut self,
        vm: &mut VM,
        ch: char,
        size_px: f32,
        color: Vec4<f32>,
    ) -> Option<GlyphEntry> {
        let key = Self::to_key(ch, size_px, color);
        if let Some(entry) = self.glyphs.get(&key) {
            return Some(entry.clone());
        }
        let font = match &self.font {
            Some(f) => f,
            None => {
                if !self.font_missing_logged {
                    eprintln!(
                        "ui text: embedded font 'ui_font.ttf' not found; text will not render"
                    );
                    self.font_missing_logged = true;
                }
                return None;
            }
        };

        let (metrics, bitmap) = font.rasterize(ch, key.size_px as f32);
        // Skip zero-sized glyphs but still record advance for spacing
        let width = metrics.width.max(1) as usize;
        let height = metrics.height.max(1) as usize;
        let mut rgba = Vec::with_capacity(width * height * 4);
        for &a in bitmap.iter() {
            // Use the text color's RGB with the glyph's alpha (coverage)
            rgba.extend_from_slice(&[
                key.color[0],
                key.color[1],
                key.color[2],
                a, // glyph coverage as alpha
            ]);
        }
        if rgba.len() < width * height * 4 {
            rgba.resize(width * height * 4, 0);
        }

        let tile_id = Uuid::new_v4();

        // Create material frame with b=0 to mark as non-style tile
        let mat_pixels = vec![0u8; width * height * 4];

        vm.execute(Atom::AddTile {
            id: tile_id,
            width: metrics.width.max(1) as u32,
            height: metrics.height.max(1) as u32,
            frames: vec![rgba],
            material_frames: Some(vec![mat_pixels]),
        });

        let entry = GlyphEntry {
            tile_id,
            advance: metrics.advance_width,
            bearing_x: metrics.xmin as f32,
            bearing_y: metrics.ymin as f32,
            width: metrics.width as u32,
            height: metrics.height as u32,
        };
        self.glyphs.insert(key, entry.clone());
        self.dirty = true;
        Some(entry)
    }

    pub fn build_if_dirty(&mut self, vm: &mut VM) {
        if self.dirty {
            vm.execute(Atom::BuildAtlas);
            self.dirty = false;
        }
    }

    /// Simple single-line layout using fontdue with PositiveYDown coords.
    pub fn layout_positions(
        &self,
        text: &str,
        size_px: f32,
    ) -> Vec<fontdue::layout::GlyphPosition> {
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            x: 0.0,
            y: 0.0,
            max_width: None,
            max_height: None,
            ..LayoutSettings::default()
        });
        let Some(_) = self.font else {
            return Vec::new();
        };
        layout.append(
            &[self.font.as_ref().unwrap()],
            &TextStyle::new(text, size_px, 0),
        );
        layout.glyphs().to_vec()
    }
}
