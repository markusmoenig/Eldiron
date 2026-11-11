//! Shared text layout and Swash glyph images for measurement, editing and GPU blits.
use crate::paint::{FontFace, Glyph, GlyphRun, Point, Rect};
use parley::{
    Alignment, AlignmentOptions, FontContext, Layout, LayoutContext, PositionedLayoutItem,
    StyleProperty,
};
use std::{collections::HashMap, sync::Arc};
use swash::{
    CacheKey, FontRef,
    scale::{Render, ScaleContext, Source, StrikeWith, image::Content},
    zeno::{Format, Vector},
};

pub struct TextStyleSpan {
    pub range: std::ops::Range<usize>,
    pub color: crate::paint::Color,
}
pub struct TextLine {
    pub range: std::ops::Range<usize>,
    pub top: f32,
    pub bottom: f32,
    pub baseline: f32,
    pub advance: f32,
}
pub struct TextLayout {
    pub runs: Vec<Arc<GlyphRun>>,
    layout: Layout<usize>,
    pub styled_runs: Vec<(Arc<GlyphRun>, Option<crate::paint::Color>)>,
    pub source: Arc<str>,
}
impl TextLayout {
    pub fn width(&self) -> f32 {
        self.layout.width()
    }
    pub fn height(&self) -> f32 {
        self.layout.height()
    }
    pub fn lines(&self) -> Vec<TextLine> {
        self.layout
            .lines()
            .map(|line| {
                let m = line.metrics();
                TextLine {
                    range: line.text_range(),
                    top: m.block_min_coord,
                    bottom: m.block_max_coord,
                    baseline: m.baseline,
                    advance: m.advance,
                }
            })
            .collect()
    }
    pub fn selection(&self, start: usize, end: usize) -> Vec<Rect> {
        let cursor = |index| {
            parley::Cursor::from_byte_index(&self.layout, index, parley::Affinity::Downstream)
        };
        parley::Selection::new(cursor(start), cursor(end))
            .geometry(&self.layout)
            .into_iter()
            .map(|(r, _)| {
                Rect::new(
                    r.x0 as f32,
                    r.y0 as f32,
                    (r.x1 - r.x0) as f32,
                    (r.y1 - r.y0) as f32,
                )
            })
            .collect()
    }
    pub fn hit_test(&self, point: Point) -> usize {
        parley::Cursor::from_point(&self.layout, point[0], point[1]).index()
    }
    pub fn caret(&self, index: usize) -> Rect {
        let r = parley::Cursor::from_byte_index(&self.layout, index, parley::Affinity::Downstream)
            .geometry(&self.layout, 1.);
        Rect::new(
            r.x0 as f32,
            r.y0 as f32,
            (r.x1 - r.x0) as f32,
            (r.y1 - r.y0) as f32,
        )
    }
}
pub struct TextSystem {
    pub fonts: FontContext,
    layouts: LayoutContext<usize>,
    faces: HashMap<(u64, u32), Arc<FontFace>>,
}
impl Default for TextSystem {
    fn default() -> Self {
        Self {
            fonts: FontContext::new(),
            layouts: LayoutContext::new(),
            faces: HashMap::new(),
        }
    }
}
impl TextSystem {
    pub fn register_font(&mut self, data: Vec<u8>) {
        self.fonts
            .collection
            .register_fonts(parley::fontique::Blob::new(Arc::new(data)), None);
    }
    pub fn layout(
        &mut self,
        text: &str,
        family: &str,
        size: f32,
        width: Option<f32>,
    ) -> TextLayout {
        self.layout_styled(text, family, size, width, &[])
    }
    pub fn layout_styled(
        &mut self,
        text: &str,
        family: &str,
        size: f32,
        width: Option<f32>,
        styles: &[TextStyleSpan],
    ) -> TextLayout {
        let mut builder = self.layouts.ranged_builder(&mut self.fonts, text, 1., true);
        builder.push_default(StyleProperty::FontSize(size));
        builder.push_default(parley::FontFamily::from(family));
        builder.push_default(StyleProperty::Brush(usize::MAX));
        for (i, span) in styles.iter().enumerate() {
            builder.push(StyleProperty::Brush(i), span.range.clone());
        }
        let mut layout: Layout<usize> = builder.build(text);
        layout.break_all_lines(width);
        layout.align(Alignment::Start, AlignmentOptions::default());
        let mut runs = Vec::new();
        let mut styled_runs = Vec::new();
        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(gr) = item {
                    let r = gr.run();
                    let font = r.font();
                    let identity = (font.data.id(), font.index);
                    let face = self
                        .faces
                        .entry(identity)
                        .or_insert_with(|| {
                            Arc::new(FontFace {
                                identity,
                                data: Arc::from(font.data.as_ref()),
                            })
                        })
                        .clone();
                    let glyphs: Vec<_> = gr
                        .positioned_glyphs()
                        .map(|g| Glyph {
                            id: g.id,
                            position: [g.x, g.y],
                            advance: g.advance,
                        })
                        .collect();
                    let run = Arc::new(GlyphRun {
                        font: face,
                        size: r.font_size(),
                        variations: Arc::from(r.normalized_coords()),
                        glyphs: Arc::from(glyphs),
                    });
                    styled_runs.push((run.clone(), styles.get(gr.style().brush).map(|s| s.color)));
                    runs.push(run);
                }
            }
        }
        TextLayout {
            runs,
            styled_runs,
            layout,
            source: Arc::from(text),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font: (u64, u32),
    pub glyph: u32,
    pub size_64: u32,
    pub x_phase: u8,
    pub y_phase: u8,
    pub variations: Arc<[i16]>,
}
#[derive(Debug)]
pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub color: bool,
    pub pixels: Vec<u8>,
}
/// Bounded cache of Swash glyph images. Color is applied at draw time for masks.
pub struct GlyphCache {
    scale: ScaleContext,
    keys: HashMap<(u64, u32), (u32, CacheKey)>,
    images: HashMap<GlyphKey, Arc<GlyphBitmap>>,
    bytes: usize,
    limit: usize,
    pub rasterizations: u64,
}
impl Default for GlyphCache {
    fn default() -> Self {
        Self::new(16 * 1024 * 1024)
    }
}
impl GlyphCache {
    pub fn new(limit: usize) -> Self {
        Self {
            scale: ScaleContext::new(),
            keys: HashMap::new(),
            images: HashMap::new(),
            bytes: 0,
            limit,
            rasterizations: 0,
        }
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn glyph(
        &mut self,
        run: &GlyphRun,
        glyph: u32,
        pixels_per_unit: f32,
        position: Point,
    ) -> Option<(GlyphKey, Arc<GlyphBitmap>)> {
        let size = (run.size * pixels_per_unit * 64.).round().max(1.) as u32;
        let phase = |x: f32| ((x.rem_euclid(1.) * 4.).floor() as u8).min(3);
        let key = GlyphKey {
            font: run.font.identity,
            glyph,
            size_64: size,
            x_phase: phase(position[0]),
            y_phase: phase(position[1]),
            variations: run.variations.clone(),
        };
        if let Some(image) = self.images.get(&key) {
            return Some((key, image.clone()));
        }
        let (offset, font_key) = *self.keys.entry(run.font.identity).or_insert_with(|| {
            let f = FontRef::from_index(&run.font.data, run.font.identity.1 as usize);
            f.map(|f| (f.offset, f.key)).unwrap_or((0, CacheKey::new()))
        });
        let font = FontRef {
            data: &run.font.data,
            offset,
            key: font_key,
        };
        let mut scaler = self
            .scale
            .builder(font)
            .size(size as f32 / 64.)
            .hint(true)
            .normalized_coords(run.variations.iter().copied())
            .build();
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(Vector::new(
            key.x_phase as f32 / 4.,
            key.y_phase as f32 / 4.,
        ))
        .render(&mut scaler, u16::try_from(glyph).ok()?)?;
        self.rasterizations += 1;
        let bitmap = Arc::new(GlyphBitmap {
            width: image.placement.width,
            height: image.placement.height,
            left: image.placement.left,
            top: image.placement.top,
            color: image.content == Content::Color,
            pixels: image.data,
        });
        if self.bytes + bitmap.pixels.len() > self.limit {
            self.images.clear();
            self.bytes = 0;
        }
        if bitmap.pixels.len() <= self.limit {
            self.bytes += bitmap.pixels.len();
            self.images.insert(key.clone(), bitmap.clone());
        }
        Some((key, bitmap))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swash_cache_and_cluster_caret_share_layout() {
        let mut text = TextSystem::default();
        text.register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
        let layout = text.layout("Office e\u{301}", "Roboto", 14., None);
        assert!(layout.width() > 0.);
        assert!(!layout.runs.is_empty());
        let caret = layout.caret(0);
        assert_eq!(layout.hit_test([caret.x, caret.y + caret.height / 2.]), 0);
        let run = &layout.runs[0];
        let glyph = run.glyphs[0];
        let mut cache = GlyphCache::default();
        let first = cache.glyph(run, glyph.id, 1.5, [0., 0.]).unwrap();
        let second = cache.glyph(run, glyph.id, 1.5, [0., 0.]).unwrap();
        assert!(Arc::ptr_eq(&first.1, &second.1));
        assert_eq!(cache.rasterizations, 1);
        assert!(!first.1.pixels.is_empty());
    }
}
