//! Retained editor overlay commands. Source images remain editable CPU assets;
//! the reference adapter is explicit, while native paint never creates pixels.
use crate::prelude::*;
use std::sync::Arc;
use theframework_core::ImageAsset;

type BoxRect = (usize, usize, usize, usize);
#[derive(Clone)]
pub(crate) struct OverlayImage {
    key: String,
    pub(crate) source: Arc<ImageAsset>,
}
struct CachedSource {
    revision: u64,
    source: Arc<ImageAsset>,
}
pub(crate) struct OverlaySources {
    owner: Uuid,
    revision: u64,
    entries: std::collections::HashMap<String, CachedSource>,
    used: std::collections::HashSet<String>,
}
impl OverlaySources {
    pub(crate) fn new(owner: Uuid) -> Self {
        Self {
            owner,
            revision: 0,
            entries: Default::default(),
            used: Default::default(),
        }
    }
    pub(crate) fn begin(&mut self) {
        self.used.clear();
    }
    pub(crate) fn image(
        &mut self,
        role: &str,
        width: usize,
        height: usize,
        pixels: &[u8],
    ) -> Option<OverlayImage> {
        if width == 0 || height == 0 {
            return None;
        }
        let width = u32::try_from(width).ok()?;
        let height = u32::try_from(height).ok()?;
        let same = self.entries.get(role).is_some_and(|entry| {
            entry.source.width() == width
                && entry.source.height() == height
                && entry.source.pixels() == pixels
        });
        if !same {
            let source = ImageAsset::rgba(width, height, pixels.to_vec()).ok()?;
            self.revision = self
                .revision
                .checked_add(1)
                .expect("overlay source revision exhausted");
            self.entries.insert(
                role.into(),
                CachedSource {
                    revision: self.revision,
                    source: Arc::new(source),
                },
            );
        }
        self.used.insert(role.into());
        let entry = &self.entries[role];
        Some(OverlayImage {
            key: format!("editor-overlay:{}:{}:{}", self.owner, role, entry.revision),
            source: entry.source.clone(),
        })
    }
    pub(crate) fn finish(&mut self) {
        self.entries.retain(|role, _| self.used.contains(role));
    }
}

enum Command {
    Rect(BoxRect, RGBA),
    SignedRect(TheDim, RGBA),
    SignedOutline(TheDim, RGBA),
    PixelLine {
        from: [i32; 2],
        to: [i32; 2],
        color: RGBA,
    },
    Outline(BoxRect, RGBA),
    Text {
        position: (usize, usize),
        text: String,
        settings: TheFontSettings,
        color: RGBA,
        background: RGBA,
    },
    Label {
        rect: BoxRect,
        text: String,
        settings: TheFontSettings,
        color: RGBA,
        background: Option<RGBA>,
        horizontal: TheHorizontalAlign,
        vertical: TheVerticalAlign,
    },
    Image {
        rect: BoxRect,
        image: OverlayImage,
    },
    Procedural {
        rect: BoxRect,
        source: procedural::ProceduralIcon,
    },
}
pub(crate) struct EditorOverlay {
    pub(crate) size: (usize, usize),
    commands: Vec<Command>,
}
impl EditorOverlay {
    pub(crate) fn new(size: (usize, usize)) -> Self {
        Self {
            size,
            commands: Vec::new(),
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    pub(crate) fn rect(&mut self, rect: &BoxRect, color: &RGBA) {
        self.commands.push(Command::Rect(*rect, *color));
    }
    pub(crate) fn signed_rect(&mut self, rect: TheDim, color: RGBA) {
        self.commands.push(Command::SignedRect(rect, color));
    }
    pub(crate) fn signed_outline(&mut self, rect: &TheDim, color: &RGBA) {
        self.commands.push(Command::SignedOutline(*rect, *color));
    }
    pub(crate) fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: RGBA) {
        self.commands.push(Command::PixelLine {
            from: [x0, y0],
            to: [x1, y1],
            color,
        });
    }
    pub(crate) fn horizontal_line(&mut self, x0: i32, x1: i32, y: i32, color: RGBA) {
        self.line(x0, y, x1, y, color);
    }
    pub(crate) fn vertical_line(&mut self, x: i32, y0: i32, y1: i32, color: RGBA) {
        self.line(x, y0, x, y1, color);
    }
    pub(crate) fn dim(&self) -> TheDim {
        TheDim::sized(
            i32::try_from(self.size.0).unwrap_or(i32::MAX),
            i32::try_from(self.size.1).unwrap_or(i32::MAX),
        )
    }
    pub(crate) fn rect_outline(&mut self, rect: &BoxRect, color: &RGBA) {
        self.commands.push(Command::Outline(*rect, *color));
    }
    pub(crate) fn text(
        &mut self,
        position: &(usize, usize),
        text: &str,
        settings: TheFontSettings,
        color: &RGBA,
        background: &RGBA,
    ) {
        self.commands.push(Command::Text {
            position: *position,
            text: text.into(),
            settings,
            color: *color,
            background: *background,
        });
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_rect(
        &mut self,
        rect: &BoxRect,
        text: &str,
        settings: TheFontSettings,
        color: &RGBA,
        background: &RGBA,
        horizontal: TheHorizontalAlign,
        vertical: TheVerticalAlign,
    ) {
        self.commands.push(Command::Label {
            rect: *rect,
            text: text.into(),
            settings,
            color: *color,
            background: Some(*background),
            horizontal,
            vertical,
        });
    }
    pub(crate) fn text_rect_blend(
        &mut self,
        rect: &BoxRect,
        text: &str,
        settings: TheFontSettings,
        color: &RGBA,
        horizontal: TheHorizontalAlign,
        vertical: TheVerticalAlign,
    ) {
        self.commands.push(Command::Label {
            rect: *rect,
            text: text.into(),
            settings,
            color: *color,
            background: None,
            horizontal,
            vertical,
        });
    }
    pub(crate) fn image(&mut self, image: OverlayImage, rect: BoxRect) {
        self.commands.push(Command::Image { rect, image });
    }
    pub(crate) fn procedural(&mut self, source: procedural::ProceduralIcon, rect: BoxRect) {
        self.commands.push(Command::Procedural { rect, source });
    }
    /// Explicit reference-only pixel output. No native producer calls this adapter.
    pub(crate) fn draw_reference(&self, buffer: &mut TheRGBABuffer, ctx: &TheContext) {
        let stride = buffer.stride();
        for command in &self.commands {
            match command {
                Command::SignedRect(rect, color) => {
                    // Explicit reference semantics of the original set_pixel square.
                    for y in rect.y..rect.y.saturating_add(rect.height) {
                        for x in rect.x..rect.x.saturating_add(rect.width) {
                            buffer.set_pixel(x, y, color);
                        }
                    }
                }
                Command::SignedOutline(rect, color) => buffer.draw_rect_outline(rect, color),
                Command::PixelLine { from, to, color } => {
                    buffer.draw_line(from[0], from[1], to[0], to[1], *color)
                }
                Command::Rect(rect, color) => {
                    let width = (buffer.dim().width.max(0) as usize)
                        .saturating_sub(rect.0)
                        .min(rect.2);
                    let height = (buffer.dim().height.max(0) as usize)
                        .saturating_sub(rect.1)
                        .min(rect.3);
                    if width > 0 && height > 0 {
                        ctx.draw.rect(
                            buffer.pixels_mut(),
                            &(rect.0, rect.1, width, height),
                            stride,
                            color,
                        );
                    }
                }
                Command::Outline(rect, color) => {
                    if rect.2 == 0 || rect.3 == 0 {
                        continue;
                    }
                    if rect.0.saturating_add(rect.2) <= buffer.dim().width.max(0) as usize
                        && rect.1.saturating_add(rect.3) <= buffer.dim().height.max(0) as usize
                    {
                        ctx.draw
                            .rect_outline(buffer.pixels_mut(), rect, stride, color);
                    } else {
                        // Clip the original border rather than moving its far
                        // edges to the viewport boundary. This is reference-only.
                        buffer.draw_rect_outline(
                            &TheDim::rect(
                                rect.0 as i32,
                                rect.1 as i32,
                                rect.2 as i32,
                                rect.3 as i32,
                            ),
                            color,
                        );
                    }
                }
                Command::Text {
                    position,
                    text,
                    settings,
                    color,
                    background,
                } => ctx.draw.text(
                    buffer.pixels_mut(),
                    position,
                    stride,
                    text,
                    copy_settings(settings),
                    color,
                    background,
                ),
                Command::Label {
                    rect,
                    text,
                    settings,
                    color,
                    background,
                    horizontal,
                    vertical,
                } => {
                    if let Some(background) = background {
                        ctx.draw.text_rect(
                            buffer.pixels_mut(),
                            rect,
                            stride,
                            text,
                            copy_settings(settings),
                            color,
                            background,
                            horizontal.clone(),
                            copy_vertical(vertical),
                        );
                    } else {
                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            rect,
                            stride,
                            text,
                            copy_settings(settings),
                            color,
                            horizontal.clone(),
                            copy_vertical(vertical),
                        );
                    }
                }
                Command::Image { rect, image } => draw_reference_image(
                    buffer,
                    ctx,
                    rect,
                    image.source.width() as usize,
                    image.source.height() as usize,
                    image.source.pixels(),
                ),
                Command::Procedural { rect, source } => {
                    let texture = source.reference_pixels();
                    draw_reference_image(
                        buffer,
                        ctx,
                        rect,
                        texture.width,
                        texture.height,
                        &texture.data,
                    );
                }
            }
        }
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn paint(
        &self,
        text: &mut theframework::text::TextSystem,
        images: &mut dyn OverlayImages,
        density: f32,
    ) -> Result<theframework::paint::PaintList, theframework::theui::paint::UiPaintError> {
        use theframework::{paint::*, theui::paint::UiPaintError};
        if !density.is_finite() || density <= 0. {
            return Err(UiPaintError("invalid overlay density".into()));
        }
        let mut list = PaintList::default();
        list.ops.push(PaintOp::PushClip(Rect::new(
            0.,
            0.,
            self.size.0 as f32,
            self.size.1 as f32,
        )));
        let bounds = |r: &BoxRect| Rect::new(r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32);
        let opaque = |color: &RGBA| {
            [
                color[0] as f32 / 255.,
                color[1] as f32 / 255.,
                color[2] as f32 / 255.,
                1.,
            ]
        };
        for command in &self.commands {
            match command {
                Command::PixelLine { from, to, color } => list.ops.push(PaintOp::PixelLine {
                    from: from.map(|v| v as f32),
                    to: to.map(|v| v as f32),
                    color: opaque(color),
                }),
                Command::SignedRect(rect, color) => {
                    if rect.width > 0 && rect.height > 0 {
                        list.rect(
                            Rect::new(
                                rect.x as f32,
                                rect.y as f32,
                                rect.width as f32,
                                rect.height as f32,
                            ),
                            0.,
                            opaque(color),
                        );
                    }
                }
                Command::SignedOutline(rect, color) => {
                    if rect.width > 0 && rect.height > 0 {
                        let r = Rect::new(
                            rect.x as f32,
                            rect.y as f32,
                            rect.width as f32,
                            rect.height as f32,
                        );
                        for edge in [
                            Rect::new(r.x, r.y, r.width, 1.),
                            Rect::new(r.x, r.y + r.height - 1., r.width, 1.),
                            Rect::new(r.x, r.y, 1., r.height),
                            Rect::new(r.x + r.width - 1., r.y, 1., r.height),
                        ] {
                            list.rect(edge, 0., opaque(color));
                        }
                    }
                }
                Command::Rect(rect, color) => list.rect(bounds(rect), 0., opaque(color)),
                Command::Outline(rect, color) => {
                    let r = bounds(rect);
                    if r.width < 1. || r.height < 1. {
                        continue;
                    }
                    let color = opaque(color);
                    for edge in [
                        Rect::new(r.x, r.y, r.width, 1.),
                        Rect::new(r.x, r.y + r.height - 1., r.width, 1.),
                        Rect::new(r.x, r.y, 1., r.height),
                        Rect::new(r.x + r.width - 1., r.y, 1., r.height),
                    ] {
                        list.rect(edge, 0., color);
                    }
                }
                Command::Text {
                    position,
                    text: value,
                    settings,
                    color,
                    background,
                } => {
                    let layout = text.layout(value, family(settings), settings.size, None);
                    for run in layout.runs {
                        list.ops.push(PaintOp::OpaqueGlyphRun {
                            run,
                            origin: [position.0 as f32, position.1 as f32],
                            color: opaque(color),
                            background: opaque(background),
                        });
                    }
                }
                Command::Label {
                    rect,
                    text: value,
                    settings,
                    color,
                    background,
                    horizontal,
                    vertical,
                } => {
                    let r = bounds(rect);
                    let value = fit_label(value, r.width, text, settings, background.is_some());
                    let layout = text.layout_aligned(
                        &value,
                        theframework::text::TextOptions {
                            family: family(settings),
                            size: settings.size,
                            width: Some(r.width),
                            line_height: None,
                        },
                        &[],
                        match horizontal {
                            TheHorizontalAlign::Left => theframework::text::TextAlignment::Start,
                            TheHorizontalAlign::Center => theframework::text::TextAlignment::Center,
                            TheHorizontalAlign::Right => theframework::text::TextAlignment::End,
                        },
                    );
                    let x = r.x;
                    let y = r.y
                        + match vertical {
                            TheVerticalAlign::Top => 0.,
                            TheVerticalAlign::Center => (r.height - layout.height()) * 0.5,
                            TheVerticalAlign::Bottom => r.height - layout.height(),
                        };
                    list.ops.push(PaintOp::PushClip(r));
                    for run in layout.runs {
                        list.ops.push(if let Some(background) = background {
                            PaintOp::OpaqueGlyphRun {
                                run,
                                origin: [x, y],
                                color: opaque(color),
                                background: opaque(background),
                            }
                        } else {
                            PaintOp::GlyphRun {
                                run,
                                origin: [x, y],
                                color: opaque(color),
                            }
                        });
                    }
                    list.ops.push(PaintOp::PopClip);
                }
                Command::Procedural { rect, source } => {
                    if rect.2 == 0 || rect.3 == 0 {
                        continue;
                    }
                    list.ops.push(PaintOp::PixelImage {
                        image: images.procedural(source)?,
                        rect: bounds(rect),
                        tint: [1.; 4],
                    });
                }
                Command::Image { rect, image } => {
                    if rect.2 == 0 || rect.3 == 0 {
                        continue;
                    }
                    let handle = images.asset(&image.key, &image.source, false)?;
                    list.ops.push(PaintOp::PixelImage {
                        image: handle,
                        rect: bounds(rect),
                        tint: [1.; 4],
                    });
                }
            }
        }
        list.ops.push(PaintOp::PopClip);
        Ok(list)
    }
    #[cfg(all(test, feature = "native-ui"))]
    pub(crate) fn reference_text_bounds(&self, ctx: &TheContext) -> Vec<theframework::paint::Rect> {
        use fontdue::layout::{HorizontalAlign, LayoutSettings, VerticalAlign};
        use theframework::paint::Rect;
        let mut boxes = Vec::new();
        for command in &self.commands {
            let (value, settings, position, options) = match command {
                Command::Text {
                    position,
                    text,
                    settings,
                    ..
                } => (text.clone(), settings, *position, LayoutSettings::default()),
                Command::Label {
                    rect,
                    text,
                    settings,
                    background,
                    horizontal,
                    vertical,
                    ..
                } => {
                    let mut value = if background.is_some() {
                        text.trim_end().replace('\n', "")
                    } else {
                        text.trim_end().into()
                    };
                    if value.trim_end().is_empty() {
                        continue;
                    }
                    let mut width = ctx.draw.get_text_size(&value, settings).0;
                    let mut trail = false;
                    while width >= rect.2 && !value.is_empty() {
                        value.pop();
                        width = ctx.draw.get_text_size(&format!("{value}..."), settings).0;
                        trail = true;
                    }
                    if trail {
                        value.push_str("...");
                    }
                    (
                        value,
                        settings,
                        (rect.0, rect.1),
                        LayoutSettings {
                            max_width: Some(rect.2 as f32),
                            max_height: Some(rect.3 as f32),
                            horizontal_align: match horizontal {
                                TheHorizontalAlign::Left => HorizontalAlign::Left,
                                TheHorizontalAlign::Center => HorizontalAlign::Center,
                                TheHorizontalAlign::Right => HorizontalAlign::Right,
                            },
                            vertical_align: match vertical {
                                TheVerticalAlign::Top => VerticalAlign::Top,
                                TheVerticalAlign::Center => VerticalAlign::Middle,
                                TheVerticalAlign::Bottom => VerticalAlign::Bottom,
                            },
                            ..Default::default()
                        },
                    )
                }
                _ => continue,
            };
            let layout = ctx.draw.get_text_layout(&value, settings, options);
            for glyph in layout.glyphs() {
                if glyph.width == 0 || glyph.height == 0 {
                    continue;
                }
                boxes.push(Rect::new(
                    (position.0 + glyph.x as usize) as f32,
                    (position.1 + glyph.y as usize) as f32,
                    glyph.width as f32,
                    glyph.height as f32,
                ));
            }
        }
        boxes
    }
    #[cfg(test)]
    pub(crate) fn procedurals(&self) -> impl Iterator<Item = &procedural::ProceduralIcon> {
        self.commands.iter().filter_map(|command| match command {
            Command::Procedural { source, .. } => Some(source),
            _ => None,
        })
    }
    #[cfg(test)]
    pub(crate) fn images(&self) -> impl Iterator<Item = &Arc<ImageAsset>> {
        self.commands.iter().filter_map(|command| match command {
            Command::Image { image, .. } => Some(&image.source),
            _ => None,
        })
    }
}
fn copy_settings(settings: &TheFontSettings) -> TheFontSettings {
    TheFontSettings {
        size: settings.size,
        preference: settings.preference.clone(),
    }
}
fn copy_vertical(value: &TheVerticalAlign) -> TheVerticalAlign {
    match value {
        TheVerticalAlign::Top => TheVerticalAlign::Top,
        TheVerticalAlign::Center => TheVerticalAlign::Center,
        TheVerticalAlign::Bottom => TheVerticalAlign::Bottom,
    }
}
#[cfg(feature = "native-ui")]
fn family(settings: &TheFontSettings) -> &'static str {
    match settings.preference {
        TheFontPreference::Default => "Roboto",
        TheFontPreference::Code => "Source Code Pro",
    }
}
#[cfg(feature = "native-ui")]
fn fit_label(
    value: &str,
    width: f32,
    text: &mut theframework::text::TextSystem,
    settings: &TheFontSettings,
    flatten: bool,
) -> String {
    let mut value = if flatten {
        value.trim_end().replace('\n', "")
    } else {
        value.trim_end().into()
    };
    if value.trim_end().is_empty() {
        return String::new();
    }
    if text
        .layout(&value, family(settings), settings.size, None)
        .width()
        < width
    {
        return value;
    }
    while !value.is_empty() {
        value.pop();
        let candidate = format!("{value}...");
        if text
            .layout(&candidate, family(settings), settings.size, None)
            .width()
            < width
        {
            return candidate;
        }
    }
    "...".into()
}

pub(crate) mod panel;

#[cfg(test)]
pub(crate) mod test_support;

pub(crate) mod procedural;

#[cfg(feature = "native-ui")]
pub(crate) trait OverlayImages: theframework::theui::paint::UiImageResources {
    fn procedural(
        &mut self,
        _source: &procedural::ProceduralIcon,
    ) -> Result<theframework_core::ImageHandle, theframework::theui::paint::UiPaintError> {
        Err(theframework::theui::paint::UiPaintError(
            "overlay requires an injected procedural GPU source producer".into(),
        ))
    }
}
#[cfg(feature = "native-render")]
impl OverlayImages for theframework::render_wgpu::UiImages {}
#[cfg(feature = "native-render")]
impl OverlayImages for theframework::render_wgpu::UiFrameImages<'_> {}
#[cfg(feature = "native-render")]
pub(crate) mod procedural_gpu;

fn draw_reference_image(
    buffer: &mut TheRGBABuffer,
    ctx: &TheContext,
    rect: &BoxRect,
    width: usize,
    height: usize,
    pixels_source: &[u8],
) {
    let stride = buffer.stride();
    if (width, height) == (rect.2, rect.3) {
        ctx.draw
            .blend_slice(buffer.pixels_mut(), pixels_source, rect, stride);
    } else {
        // Preserve Texture::resized's original left-edge nearest
        // convention in the reference adapter, including f32 math.
        let mut pixels = vec![0; rect.2 * rect.3 * 4];
        let sx = width as f32 / rect.2 as f32;
        let sy = height as f32 / rect.3 as f32;
        for y in 0..rect.3 {
            for x in 0..rect.2 {
                let source_x = ((x as f32 * sx) as usize).min(width - 1);
                let source_y = ((y as f32 * sy) as usize).min(height - 1);
                let start = (source_y * width + source_x) * 4;
                pixels[(y * rect.2 + x) * 4..(y * rect.2 + x + 1) * 4]
                    .copy_from_slice(&pixels_source[start..start + 4]);
            }
        }
        ctx.draw
            .blend_slice(buffer.pixels_mut(), &pixels, rect, stride);
    }
}
