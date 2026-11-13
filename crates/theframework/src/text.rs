//! Shared text layout and Swash glyph images for measurement, editing and GPU blits.
use crate::paint::{FontFace, Glyph, GlyphRun, Point, Rect};
mod fonts;
use parley::{
    Alignment, AlignmentOptions, FontContext, Layout, LayoutContext, PositionedLayoutItem,
    StyleProperty,
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_TEXT_OWNER: AtomicU64 = AtomicU64::new(1);
use swash::{
    CacheKey, FontRef,
    scale::{Render, ScaleContext, Source, StrikeWith, image::Content},
    zeno::{Format, Vector},
};

/// Layout constraints shared by paint, pagination, selection and hit testing.
pub struct TextOptions<'a> {
    pub family: &'a str,
    pub size: f32,
    pub width: Option<f32>,
    pub line_height: Option<f32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlignment {
    #[default]
    Start,
    Center,
    End,
}
#[derive(Clone, Debug, PartialEq)]
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
/// Paint-only labels retain the reference UI's integral advance and baseline
/// policy. Editing/layout snapshots keep their original shaped caret geometry.
pub(crate) struct LabelLayout {
    pub runs: Vec<Arc<GlyphRun>>,
    pub width: f32,
    pub height: f32,
}
pub struct TextLayout {
    pub runs: Vec<Arc<GlyphRun>>,
    line_runs: Vec<std::ops::Range<usize>>,
    layout: Layout<usize>,
    pub styled_runs: Vec<(Arc<GlyphRun>, Option<crate::paint::Color>)>,
    pub source: Arc<str>,
}
impl TextLayout {
    /// Paint-only colors preserve every original glyph position, line break and
    /// editing caret. A style crossing part of a shaped cluster colors that
    /// cluster as a unit; later overlapping spans take precedence.
    pub fn colorized_runs(
        &self,
        styles: &[TextStyleSpan],
    ) -> Vec<(Arc<GlyphRun>, Option<crate::paint::Color>)> {
        if styles.is_empty() {
            return self.styled_runs.clone();
        }
        let spans = color_intervals(styles, self.source.len());
        let mut source = self.styled_runs.iter();
        let mut colored = Vec::new();
        for line in self.layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(positioned) = item else {
                    continue;
                };
                let (run, original) = source.next().expect("snapshot/layout glyph run identity");
                let mut groups: Vec<(Vec<Glyph>, Option<crate::paint::Color>)> = Vec::new();
                for (index, (_, range)) in positioned.positioned_glyphs_with_clusters().enumerate()
                {
                    let start = spans.partition_point(|(r, _)| r.end <= range.start);
                    let color = spans[start..]
                        .iter()
                        .take_while(|(r, _)| r.start < range.end)
                        .map(|(_, index)| *index)
                        .max()
                        .map(|index| styles[index].color)
                        .or(*original);
                    if let Some((glyphs, _)) =
                        groups.last_mut().filter(|(_, previous)| *previous == color)
                    {
                        glyphs.push(run.glyphs[index]);
                    } else {
                        groups.push((vec![run.glyphs[index]], color));
                    }
                }
                if groups.len() == 1 {
                    colored.push((run.clone(), groups[0].1));
                } else {
                    for (glyphs, color) in groups {
                        colored.push((
                            Arc::new(GlyphRun {
                                font: run.font.clone(),
                                size: run.size,
                                variations: run.variations.clone(),
                                glyphs: Arc::from(glyphs),
                            }),
                            color,
                        ));
                    }
                }
            }
        }
        colored
    }
    pub fn width(&self) -> f32 {
        self.layout.width()
    }
    /// Includes trailing whitespace; editing viewports must keep its caret visible.
    pub fn full_width(&self) -> f32 {
        self.layout.full_width()
    }
    pub fn height(&self) -> f32 {
        self.layout.height()
    }
    pub fn line_count(&self) -> usize {
        self.layout.len()
    }
    pub fn line(&self, index: usize) -> Option<TextLine> {
        let line = self.layout.get(index)?;
        let m = line.metrics();
        Some(TextLine {
            range: line.text_range(),
            top: m.block_min_coord,
            bottom: m.block_max_coord,
            baseline: m.baseline,
            advance: m.advance,
        })
    }
    /// Slice retained glyph runs by visual line without visiting or cloning
    /// runs belonging to another page. Empty lines have empty run ranges.
    pub fn runs_for_lines(&self, lines: std::ops::Range<usize>) -> &[Arc<GlyphRun>] {
        let end = lines.end.min(self.line_runs.len());
        if lines.start >= end {
            return &[];
        }
        &self.runs[self.line_runs[lines.start].start..self.line_runs[end - 1].end]
    }
    /// Locate one visual line without allocating metadata for the full document.
    /// A soft-wrap boundary belongs to the following line (downstream affinity).
    pub fn line_range_at(&self, position: usize) -> Option<std::ops::Range<usize>> {
        if !self.source.is_char_boundary(position) {
            return None;
        }
        let mut low = 0;
        let mut high = self.layout.len();
        while low < high {
            let mid = low + (high - low) / 2;
            if self.layout.get(mid)?.text_range().end <= position {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        self.layout
            .get(low.min(self.layout.len().saturating_sub(1)))
            .map(|line| line.text_range())
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
    /// Indexed visual lines intersecting a viewport. Binary search skips the
    /// document prefix without allocating metadata for every off-screen line.
    pub fn visible_lines(&self, top: f32, bottom: f32) -> Vec<(usize, TextLine)> {
        if !top.is_finite() || !bottom.is_finite() || top >= bottom {
            return Vec::new();
        }
        let mut low = 0;
        let mut high = self.layout.len();
        while low < high {
            let mid = low + (high - low) / 2;
            if self.layout.get(mid).unwrap().metrics().block_max_coord <= top {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        (low..self.layout.len())
            .map_while(|index| {
                let line = self.layout.get(index)?;
                let m = line.metrics();
                (m.block_min_coord < bottom).then(|| {
                    (
                        index,
                        TextLine {
                            range: line.text_range(),
                            top: m.block_min_coord,
                            bottom: m.block_max_coord,
                            baseline: m.baseline,
                            advance: m.advance,
                        },
                    )
                })
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
fn color_intervals(
    styles: &[TextStyleSpan],
    length: usize,
) -> Vec<(std::ops::Range<usize>, usize)> {
    let mut events = Vec::with_capacity(styles.len() * 2);
    for (index, style) in styles.iter().enumerate() {
        let start = style.range.start.min(length);
        let end = style.range.end.min(length);
        if start < end {
            events.push((start, true, index));
            events.push((end, false, index));
        }
    }
    events.sort_unstable();
    let mut active = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    let mut previous = 0;
    for (position, begin, index) in events {
        if previous < position {
            if let Some(index) = active.last().copied() {
                result.push((previous..position, index));
            }
            previous = position;
        }
        if begin {
            active.insert(index);
        } else {
            active.remove(&index);
        }
    }
    result
}
/// The embedded UI and code faces are each other's ordered fallback, matching
/// the reference painter's font preference without depending on installed fonts.
/// Keep the same family stack in shaping and the editing driver so measurement,
/// glyphs, carets and IME geometry all describe the same text.
pub(crate) fn font_family(family: &str) -> parley::FontFamily<'_> {
    use parley::{FontFamily, FontFamilyName};
    static UI: [FontFamilyName<'static>; 2] = [
        FontFamilyName::named("Roboto"),
        FontFamilyName::named("Source Code Pro"),
    ];
    static CODE: [FontFamilyName<'static>; 2] = [
        FontFamilyName::named("Source Code Pro"),
        FontFamilyName::named("Roboto"),
    ];
    match family {
        "Roboto" => FontFamily::List(std::borrow::Cow::Borrowed(&UI)),
        "Source Code Pro" => FontFamily::List(std::borrow::Cow::Borrowed(&CODE)),
        _ => FontFamily::from(family),
    }
}

pub struct TextSystem {
    owner: u64,
    fonts: FontContext,
    font_revision: u64,
    layouts: LayoutContext<usize>,
    faces: HashMap<(u64, u32), Arc<FontFace>>,
    // Collection faces share bytes; weak lookup entries cannot retain replaced sources.
    font_data: HashMap<u64, Weak<[u8]>>,
}
impl Default for TextSystem {
    fn default() -> Self {
        Self {
            owner: NEXT_TEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            fonts: FontContext::new(),
            font_revision: 0,
            layouts: LayoutContext::new(),
            faces: HashMap::new(),
            font_data: HashMap::new(),
        }
    }
}
impl TextSystem {
    /// Stable across moves, distinct across text owners even if registration
    /// counts match. Retained editors use both ownership and font revision.
    pub fn font_stamp(&self) -> (u64, u64) {
        (self.owner, self.font_revision)
    }
    pub(crate) fn editor_driver<'a>(
        &'a mut self,
        editor: &'a mut parley::PlainEditor<usize>,
    ) -> parley::PlainEditorDriver<'a, usize> {
        editor.driver(&mut self.fonts, &mut self.layouts)
    }
    pub(crate) fn editor_snapshot(
        &mut self,
        editor: &mut parley::PlainEditor<usize>,
    ) -> TextLayout {
        let layout = {
            let mut driver = self.editor_driver(editor);
            driver.layout().clone()
        };
        self.snapshot_layout(editor.raw_text(), layout, &[])
    }
    pub fn font_revision(&self) -> u64 {
        self.font_revision
    }
    fn fonts_changed(&mut self) {
        self.font_revision = self.font_revision.wrapping_add(1);
        // Retained layouts keep their Arc faces. The lookup cache must not keep
        // every superseded project font alive forever.
        self.faces.clear();
        self.font_data.retain(|_, data| data.strong_count() > 0);
    }
    fn font_blob(&mut self, data: Arc<[u8]>) -> parley::fontique::Blob<u8> {
        self.font_data.retain(|_, data| data.strong_count() > 0);
        let blob = parley::fontique::Blob::new(Arc::new(data.clone()));
        self.font_data.insert(blob.id(), Arc::downgrade(&data));
        blob
    }
    pub fn register_font(&mut self, data: Vec<u8>) {
        let data = self.font_blob(Arc::from(data));
        if !self.fonts.collection.register_fonts(data, None).is_empty() {
            self.fonts_changed();
        }
    }
    /// Register an authored asset name independently of the font's internal family name.
    pub fn register_font_named(&mut self, name: &str, data: Vec<u8>) -> bool {
        let data = self.font_blob(Arc::from(data));
        let info = parley::fontique::FontInfoOverride {
            family_name: Some(name),
            ..Default::default()
        };
        // Fontique appends faces to existing families. Validate first, then replace
        // the named asset's faces so invalid bytes cannot erase its prior font.
        let mut pending = parley::fontique::Collection::new(parley::fontique::CollectionOptions {
            system_fonts: false,
            shared: false,
        });
        if pending.register_fonts(data.clone(), Some(info)).is_empty() {
            return false;
        }
        if let Some(family) = self.fonts.collection.family_by_name(name) {
            for font in family.fonts() {
                self.fonts.collection.unregister_font(
                    family.id(),
                    font.width(),
                    font.style(),
                    font.weight(),
                );
            }
        }
        let registered = !self
            .fonts
            .collection
            .register_fonts(data, Some(info))
            .is_empty();
        if registered {
            self.fonts_changed();
        }
        registered
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
    pub(crate) fn label_layout(&mut self, text: &str, family: &str, size: f32) -> LabelLayout {
        let shaped = self.layout(text, family, size, None);
        let mut label = LabelLayout {
            runs: Vec::new(),
            width: 0.,
            height: 0.,
        };
        for (index, line) in shaped.layout.lines().enumerate() {
            let runs = shaped.runs_for_lines(index..index + 1);
            let mut ascent = 0.0f32;
            let mut height = 0.0f32;
            for run in runs {
                if let Some(font) =
                    FontRef::from_index(&run.font.data, run.font.identity.1 as usize)
                {
                    let metrics = font.metrics(&run.variations).scale(run.size);
                    ascent = ascent.max(metrics.ascent.ceil());
                    height =
                        height.max((metrics.ascent + metrics.descent + metrics.leading).ceil());
                }
            }
            if height == 0. {
                height = (line.metrics().block_max_coord - line.metrics().block_min_coord).ceil();
            }
            let baseline = label.height + ascent;
            let mut shift = 0.;
            let items = line.items().filter_map(|item| {
                if let PositionedLayoutItem::GlyphRun(run) = item {
                    Some(run)
                } else {
                    None
                }
            });
            for (positioned, run) in items.zip(runs) {
                let clusters = positioned
                    .positioned_glyphs_with_clusters()
                    .map(|(_, range)| range)
                    .collect::<Vec<_>>();
                let mut glyphs = run.glyphs.to_vec();
                let mut start = 0;
                while start < glyphs.len() {
                    let mut end = start + 1;
                    while end < glyphs.len() && clusters[end] == clusters[start] {
                        end += 1;
                    }
                    let advance = glyphs[start..end].iter().map(|g| g.advance).sum::<f32>();
                    for glyph in &mut glyphs[start..end] {
                        glyph.position[0] += shift;
                        glyph.position[1] += baseline - line.metrics().baseline;
                    }
                    // Round whole shaped clusters together so combining marks
                    // and required ligatures retain their relative placement.
                    shift += advance.ceil() - advance;
                    start = end;
                }
                label.runs.push(Arc::new(GlyphRun {
                    font: run.font.clone(),
                    size: run.size,
                    variations: run.variations.clone(),
                    glyphs: Arc::from(glyphs),
                }));
            }
            label.width = label.width.max(line.metrics().advance + shift);
            label.height += height;
        }
        label
    }
    pub fn layout_styled(
        &mut self,
        text: &str,
        family: &str,
        size: f32,
        width: Option<f32>,
        styles: &[TextStyleSpan],
    ) -> TextLayout {
        self.layout_with_options(
            text,
            TextOptions {
                family,
                size,
                width,
                line_height: None,
            },
            styles,
        )
    }
    pub fn layout_with_options(
        &mut self,
        text: &str,
        options: TextOptions<'_>,
        styles: &[TextStyleSpan],
    ) -> TextLayout {
        self.layout_aligned(text, options, styles, TextAlignment::Start)
    }
    /// Alignment is applied by the shaper to each visual line, preserving bidi
    /// order and the same glyph/line metrics used by pagination and clipping.
    pub fn layout_aligned(
        &mut self,
        text: &str,
        options: TextOptions<'_>,
        styles: &[TextStyleSpan],
        alignment: TextAlignment,
    ) -> TextLayout {
        let TextOptions {
            family,
            size,
            width,
            line_height,
        } = options;
        let size = if size.is_finite() && size > 0. {
            size
        } else {
            14.
        };
        let width = width.filter(|w| w.is_finite()).map(|w| w.max(0.));
        let mut builder = self.layouts.ranged_builder(&mut self.fonts, text, 1., true);
        builder.push_default(StyleProperty::FontSize(size));
        if let Some(height) = line_height.filter(|h| h.is_finite() && *h > 0.) {
            builder.push_default(StyleProperty::LineHeight(parley::LineHeight::Absolute(
                height,
            )));
        }
        builder.push_default(font_family(family));
        builder.push_default(StyleProperty::Brush(usize::MAX));
        for (i, span) in styles.iter().enumerate() {
            builder.push(StyleProperty::Brush(i), span.range.clone());
        }
        let mut layout: Layout<usize> = builder.build(text);
        layout.break_all_lines(width);
        layout.align(
            match alignment {
                TextAlignment::Start => Alignment::Start,
                TextAlignment::Center => Alignment::Center,
                TextAlignment::End => Alignment::End,
            },
            AlignmentOptions::default(),
        );
        self.snapshot_layout(text, layout, styles)
    }
    fn snapshot_layout(
        &mut self,
        text: &str,
        layout: Layout<usize>,
        styles: &[TextStyleSpan],
    ) -> TextLayout {
        let mut runs = Vec::new();
        let mut styled_runs = Vec::new();
        let mut line_runs = Vec::new();
        for line in layout.lines() {
            let start = runs.len();
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(gr) = item {
                    let r = gr.run();
                    let font = r.font();
                    let identity = (font.data.id(), font.index);
                    let font_data = &mut self.font_data;
                    let face = self
                        .faces
                        .entry(identity)
                        .or_insert_with(|| {
                            let data = font_data
                                .get(&font.data.id())
                                .and_then(Weak::upgrade)
                                .unwrap_or_else(|| {
                                    let data: Arc<[u8]> = Arc::from(font.data.as_ref());
                                    font_data.insert(font.data.id(), Arc::downgrade(&data));
                                    data
                                });
                            Arc::new(FontFace { identity, data })
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
            line_runs.push(start..runs.len());
        }
        TextLayout {
            runs,
            line_runs,
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
    pub alpha_mode: theframework_core::AlphaMode,
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
            // The reference UI rasterizes unhinted outlines. Keep the same
            // glyph extents; grid fitting changes small label height/weight.
            .hint(false)
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
            // Swash's layered outline blitter composites into zero RGBA using
            // source-over in encoded color. PNG/raw color bitmaps are decoded RGBA.
            alpha_mode: if matches!(image.source, Source::ColorOutline(_)) {
                theframework_core::AlphaMode::Premultiplied
            } else {
                theframework_core::AlphaMode::Straight
            },
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
    fn bundled_ui_symbols_keep_registered_fallback_in_labels_and_editing() {
        const UI: &[u8] = include_bytes!("../embedded/fonts/Roboto-Bold.ttf");
        const CODE: &[u8] = include_bytes!("../embedded/fonts/SourceCodePro-Bold.ttf");
        let ui = FontRef::from_index(UI, 0).unwrap();
        let code = FontRef::from_index(CODE, 0).unwrap();
        let mut fonts = TextSystem::default();
        fonts.register_font(UI.to_vec());
        fonts.register_font(CODE.to_vec());
        for symbol in ['▾', '◉', '○'] {
            assert_eq!(
                ui.charmap().map(symbol),
                0,
                "fixture requires actual fallback"
            );
            assert_ne!(
                code.charmap().map(symbol),
                0,
                "bundled code font supplies {symbol}"
            );
            let source = symbol.to_string();
            let label = fonts.layout(&source, "Roboto", 13., None);
            assert_eq!(
                label.runs.iter().map(|run| run.glyphs.len()).sum::<usize>(),
                1
            );
            assert!(label.width() > 0.);
            assert!(
                label
                    .runs
                    .iter()
                    .flat_map(|run| run.glyphs.iter())
                    .all(|glyph| glyph.id != 0),
                "label {symbol} must not become a missing-glyph box"
            );
            assert!(label.runs.iter().all(|run| run.font.data.as_ref() == CODE));
            let mut editor = crate::text_edit::EditorController::new(false);
            editor.prepare(
                &mut fonts,
                TextOptions {
                    family: "Roboto",
                    size: 13.,
                    width: None,
                    line_height: None,
                },
            );
            editor.set_text(&mut fonts, &source);
            let editable = editor.layout(&mut fonts);
            assert_eq!(
                editable
                    .runs
                    .iter()
                    .map(|run| run.glyphs.len())
                    .sum::<usize>(),
                1
            );
            assert!(
                editable
                    .runs
                    .iter()
                    .flat_map(|run| run.glyphs.iter())
                    .all(|glyph| glyph.id != 0),
                "editor {symbol} must use the same fallback policy"
            );
            assert_eq!(editable.width(), label.width());
            assert!(
                editable
                    .runs
                    .iter()
                    .all(|run| run.font.data.as_ref() == CODE)
            );
        }
        for family in ["Roboto", "Source Code Pro"] {
            let layout = fonts.layout("ABC123", family, 13., None);
            let expected = if family == "Roboto" { UI } else { CODE };
            assert!(
                layout
                    .runs
                    .iter()
                    .all(|run| run.font.data.as_ref() == expected),
                "fallback must preserve primary Latin font"
            );
        }
        // Exercise the same collection policy without any installed fonts. Its
        // regional faces must share one immutable TTC source, including editors.
        let mut fallback = TextSystem::default();
        fallback.fonts.collection =
            parley::fontique::Collection::new(parley::fontique::CollectionOptions {
                system_fonts: false,
                shared: false,
            });
        fallback.register_bundled_fonts();
        let cjk = fonts::cjk_data();
        fallback.register_cjk_fallback(cjk.clone(), Some("zh-TW"));
        let source = "編輯器 简体中文 日本語 ひらがな カタカナ 한국어 Журнал";
        let shaped = fallback.layout(source, "Roboto", 13., None);
        assert!(
            shaped
                .runs
                .iter()
                .flat_map(|run| run.glyphs.iter())
                .all(|glyph| glyph.id != 0),
            "supplied regional fallback must cover shipped CJK/Cyrillic text"
        );
        let mut indices = std::collections::HashSet::new();
        let mut raster = GlyphCache::new(2 * 1024 * 1024);
        for run in &shaped.runs {
            if run.font.data.len() == cjk.len() {
                assert!(
                    Arc::ptr_eq(&run.font.data, &cjk),
                    "regional faces must share the supplied CPU collection"
                );
                indices.insert(run.font.identity.1);
                let ink = run
                    .glyphs
                    .iter()
                    .filter_map(|glyph| raster.glyph(run, glyph.id, 1.25, glyph.position))
                    .any(|(_, image)| image.pixels.iter().any(|value| *value > 0));
                assert!(
                    ink,
                    "fallback must produce real Swash ink, not only glyph IDs"
                );
            }
        }
        assert!(
            indices.contains(&0) && indices.contains(&1) && indices.contains(&3),
            "kana/Korean/default traditional Han retain their own collection faces: {indices:?}"
        );
        let mut editor = crate::text_edit::EditorController::new(false);
        editor.prepare(
            &mut fallback,
            TextOptions {
                family: "Roboto",
                size: 13.,
                width: None,
                line_height: None,
            },
        );
        editor.set_text(&mut fallback, source);
        let editable = editor.layout(&mut fallback);
        assert!(
            editable
                .runs
                .iter()
                .flat_map(|run| run.glyphs.iter())
                .all(|glyph| glyph.id != 0)
        );
        assert_eq!(
            editable.width(),
            shaped.width(),
            "labels and editing must share fallback measurement"
        );
        let mut only_fallback = TextSystem::default();
        only_fallback.fonts.collection =
            parley::fontique::Collection::new(parley::fontique::CollectionOptions {
                system_fonts: false,
                shared: false,
            });
        only_fallback.register_cjk_fallback(cjk, Some("en"));
        let cyrillic = only_fallback.layout("Журнал", "Unavailable Project Font", 13., None);
        assert!(
            !cyrillic.runs.is_empty()
                && cyrillic
                    .runs
                    .iter()
                    .flat_map(|run| run.glyphs.iter())
                    .all(|glyph| glyph.id != 0),
            "Cyrillic fallback must also work when an authored font lacks it"
        );
    }

    #[test]
    fn aligned_pages_keep_bidi_clusters_and_slice_only_the_addressed_visual_lines() {
        let mut fonts = TextSystem::default();
        fonts.register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
        let source = "AB\r\nمرحبا e\u{301}\n\n".repeat(200);
        let layout = fonts.layout_aligned(
            &source,
            TextOptions {
                family: "Roboto",
                size: 15.,
                width: Some(400.),
                line_height: Some(20.),
            },
            &[],
            TextAlignment::Center,
        );
        assert_eq!(layout.source.as_ref(), source);
        assert_eq!(layout.line_count(), 601);
        let runs = layout.runs_for_lines(450..452);
        assert!(!runs.is_empty() && runs.len() < 10);
        for run in runs {
            for glyph in run.glyphs.iter() {
                assert!(glyph.position[1] >= layout.line(450).unwrap().top);
                assert!(glyph.position[1] <= layout.line(451).unwrap().bottom);
            }
        }
        assert!(layout.runs_for_lines(2..3).is_empty());
        assert!(layout.runs_for_lines(700..800).is_empty());
        let first = &layout.runs_for_lines(0..1)[0];
        assert!(
            first.glyphs[0].position[0] > 100.,
            "centering belongs to the shaper, including mixed-script lines"
        );
        let caret = layout.caret(0);
        assert_eq!(layout.hit_test([caret.x, caret.y + caret.height * 0.5]), 0);
    }
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
    #[test]
    fn crlf_width_layout_keeps_one_hard_break_across_style_runs_and_mixed_scripts() {
        let mut fonts = TextSystem::default();
        fonts.register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
        for (source, expected) in [
            ("\r\n", 2),
            ("\r", 2),
            ("\n", 2),
            ("A\r\n\r\nB", 3),
            ("A\rB\nC", 3),
            ("مرحبا\r\n東京", 2),
            ("A\r\nB\r\n", 3),
            ("A\r\n\nB", 3),
        ] {
            let layout = fonts.layout(source, "Roboto", 14., None);
            assert_eq!(layout.source.as_ref(), source);
            assert_eq!(layout.lines().len(), expected, "source {source:?}");
        }
        let source = "ab\r\ncd";
        let mut builder = fonts
            .layouts
            .ranged_builder(&mut fonts.fonts, source, 1., true);
        builder.push_default(StyleProperty::FontFamily(parley::FontFamily::from(
            "Roboto",
        )));
        builder.push_default(StyleProperty::FontSize(14.));
        // Force distinct shaping/style runs on either side of the CR/LF pair.
        builder.push(StyleProperty::FontSize(23.), 3..source.len());
        let mut layout: Layout<usize> = builder.build(source);
        layout.break_all_lines(None);
        let snapshot = fonts.snapshot_layout(source, layout, &[]);
        assert_eq!(
            snapshot
                .lines()
                .iter()
                .map(|line| line.range.clone())
                .collect::<Vec<_>>(),
            vec![0..4, 4..6]
        );
    }
}
