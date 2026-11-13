//! Backend-neutral UI command painting. Unsupported legacy widgets fail explicitly; there is
//! no whole-window CPU fallback hidden behind this interface.
use crate::paint::Rect;
use crate::{paint::*, prelude::*, text::TextSystem};
use std::sync::Arc;
#[derive(Debug)]
pub struct UiPaintError(pub String);
impl std::fmt::Display for UiPaintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for UiPaintError {}
pub type UiPaintResult = Result<(), UiPaintError>;
/// Backend service for source images. Paint producers never own a GPU registry,
/// device, texture, surface or destination pixel storage.
pub trait UiImageResources {
    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError>;
    /// Typed editable/source pixels without widget position or display scale.
    /// Native sinks import declared color/alpha metadata directly. The default
    /// keeps existing custom sinks compatible for straight/opaque sRGB assets.
    fn asset(
        &mut self,
        key: &str,
        source: &theframework_core::ImageAsset,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        use theframework_core::{AlphaMode, ColorSpace};
        if source.color_space != ColorSpace::Srgb || source.alpha_mode == AlphaMode::Premultiplied {
            return Err(UiPaintError(
                "image sink does not implement this typed color/alpha source".into(),
            ));
        }
        let width = i32::try_from(source.width())
            .map_err(|_| UiPaintError("source image too wide".into()))?;
        let height = i32::try_from(source.height())
            .map_err(|_| UiPaintError("source image too tall".into()))?;
        let mut buffer = TheRGBABuffer::new(TheDim::sized(width, height));
        buffer.pixels_mut().copy_from_slice(source.pixels());
        if source.alpha_mode == AlphaMode::Opaque {
            for pixel in buffer.pixels_mut().chunks_exact_mut(4) {
                pixel[3] = 255;
            }
        }
        self.image(key, &buffer, changed)
    }
    /// Explicit legacy adapter: RenderView copied these RGB bytes into the
    /// opaque application display instead of alpha-compositing an image asset.
    /// Preserve the source/export bytes; only their display interpretation differs.
    fn copied_framebuffer(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        // Reference/custom resource sinks remain source-compatible. Native GPU
        // sinks override this using metadata, without this temporary byte copy.
        let mut opaque = source.clone();
        for pixel in opaque.pixels_mut().chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        self.image(key, &opaque, changed)
    }
}
pub fn bounds(dim: &TheDim) -> Rect {
    Rect::new(
        dim.x as f32,
        dim.y as f32,
        dim.width.max(0) as f32,
        dim.height.max(0) as f32,
    )
}
pub fn rgba(color: RGBA) -> Color {
    color.map(|v| v as f32 / 255.)
}
pub struct WidgetPainter<'a> {
    pub list: &'a mut PaintList,
    pub text: &'a mut TextSystem,
    pub images: &'a mut dyn UiImageResources,
    pub ui: &'a TheUIContext,
    pub theme: &'a mut dyn TheTheme,
}
impl WidgetPainter<'_> {
    /// Layout nodes use this entry point so an unassigned/hidden rectangle never
    /// emits geometry at the origin. Image-content invalidation is independent.
    pub fn widget(&mut self, widget: &mut dyn TheWidget) -> UiPaintResult {
        if !widget.dim().is_valid() {
            widget.set_needs_redraw(false);
            return Ok(());
        }
        widget.paint(self)
    }

    pub fn icon(&mut self, name: &str, rect: Rect) -> UiPaintResult {
        if let Some(source) = self.ui.icon(name) {
            if source.pixel_width() == 0 || source.pixel_height() == 0 {
                return Ok(());
            }
            let image = self.images.image(&self.ui.icon_resource_key(name), source, false)?;
            self.list.image(image, rect, Sampling::Linear);
        }
        Ok(())
    }

    pub(crate) fn dropdown_chrome(
        &mut self,
        r: Rect,
        state: crate::theui::thewidget::thedropdownchrome::TheDropdownChromeState,
    ) {
        let (role, border) = if state.disabled {
            (DropdownDisabled, TextEditBorder)
        } else if state.pressed {
            (DropdownPressed, ToolbarButtonClickedBorder)
        } else if state.focused {
            (DropdownFocus, SelectedTextEditBorder1)
        } else if state.hovered {
            (DropdownHover, ToolbarButtonHoverBorder)
        } else {
            (DropdownNormal, TextEditBorder)
        };
        let border = if state.disabled {
            *self.theme.color_disabled(border)
        } else {
            *self.theme.color(border)
        };
        self.button(r, self.theme.metric(ControlCornerRadius), role, border);
        let x = r.x + r.width - 9.;
        let y = r.y + (r.height * 0.5).floor();
        self.list.ops.push(PaintOp::Fill {
            path: Arc::new(Path {
                elements: vec![
                    PathElement::Move([x - 3.5, y - 1.5]),
                    PathElement::Line([x + 3.5, y - 1.5]),
                    PathElement::Line([x, y + 2.5]),
                    PathElement::Close,
                ],
            }),
            rule: FillRule::NonZero,
            brush: self.theme_brush(r, DropdownMarker),
        });
    }
    pub fn button(&mut self, rect: Rect, radius: f32, role: TheThemePaints, border: RGBA) {
        self.list.rect(rect, radius, rgba(border));
        self.theme_fill(rect.inset(1.), (radius - 1.).max(0.), role);
    }
    pub fn text_input(
        &mut self,
        rect: Rect,
        focused: bool,
        disabled: bool,
        embedded: bool,
        area: bool,
        border: bool,
    ) -> Rect {
        let inner = self.text_input_frame(rect, focused, disabled, embedded, area, border);
        let radius = if area {
            1.
        } else {
            self.theme.metric(ControlCornerRadius)
        };
        let inset = if !border {
            0.
        } else if area {
            1.
        } else {
            2.
        };
        self.theme_fill(
            inner,
            (radius - inset).max(0.),
            if disabled {
                TextInputDisabled
            } else if focused {
                TextInputFocused
            } else {
                TextInputNormal
            },
        );
        inner
    }
    pub fn text_input_frame(
        &mut self,
        rect: Rect,
        focused: bool,
        disabled: bool,
        embedded: bool,
        area: bool,
        border: bool,
    ) -> Rect {
        let radius = if area {
            1.
        } else {
            self.theme.metric(ControlCornerRadius)
        };
        let inset = if !border {
            0.
        } else if area {
            1.
        } else {
            2.
        };
        if border {
            if focused && !embedded {
                self.theme_fill(rect, radius, Focus);
            }
            let b = if area {
                rect
            } else {
                Rect::new(
                    rect.x + 1.,
                    rect.y + 1.,
                    (rect.width - 2.).max(0.),
                    (rect.height - 2.).max(0.),
                )
            };
            let role = if focused && !embedded && !area {
                SelectedTextEditBorder2
            } else {
                TextEditBorder
            };
            let color = if disabled {
                *self.theme.color_disabled(role)
            } else {
                *self.theme.color(role)
            };
            self.list.rect(
                b,
                (radius - if area { 0. } else { 1. }).max(0.),
                rgba(color),
            );
        }
        let inner = Rect::new(
            rect.x + inset,
            rect.y + inset,
            (rect.width - inset * 2.).max(0.),
            (rect.height - inset * 2.).max(0.),
        );
        inner
    }
    pub fn scrollbar(&mut self, bounds: Rect, thumb: Rect, hovered: bool, pressed: bool) {
        let role = if pressed {
            ScrollbarThumbPressed
        } else if hovered {
            ScrollbarThumbHover
        } else {
            ScrollbarThumbNormal
        };
        let border = if pressed {
            ToolbarButtonClickedBorder
        } else if hovered {
            ToolbarButtonHoverBorder
        } else {
            DefaultWidgetBorder
        };
        self.list.ops.push(PaintOp::PushClip(bounds));
        self.theme_fill(bounds, 0., ScrollbarTrack);
        let radius = thumb.width.min(thumb.height) * 0.5;
        self.list.rect(thumb, radius, self.color(border));
        self.theme_fill(
            Rect::new(
                thumb.x + 1.,
                thumb.y + 1.,
                (thumb.width - 2.).max(0.),
                (thumb.height - 2.).max(0.),
            ),
            (radius - 1.).max(0.),
            role,
        );
        self.list.ops.push(PaintOp::PopClip);
    }
    pub fn color(&self, role: TheThemeColors) -> Color {
        rgba(*self.theme.color(role))
    }
    pub fn fill(&mut self, rect: Rect, role: TheThemeColors) {
        self.list.rect(rect, 0., self.color(role));
    }
    pub fn theme_brush(&self, rect: Rect, role: TheThemePaints) -> Brush {
        match self.theme.paint(
            role,
            ThePixelRect::new(
                rect.x as i32,
                rect.y as i32,
                rect.width as i32,
                rect.height as i32,
            ),
        ) {
            ThePaint::Solid(c) => Brush::Solid(rgba(c)),
            ThePaint::LinearGradient {
                start,
                end,
                start_color,
                end_color,
            } => Brush::LinearGradient {
                start,
                end,
                start_color: rgba(start_color),
                end_color: rgba(end_color),
            },
        }
    }
    pub fn theme_fill(&mut self, rect: Rect, radius: f32, role: TheThemePaints) {
        let brush = self.theme_brush(rect, role);
        self.list.rect(rect, radius, brush);
    }
    pub fn outline(&mut self, rect: Rect, color: RGBA, width: f32) {
        if width <= 0. || rect.width <= 0. || rect.height <= 0. {
            return;
        }
        // Widget borders occupy the inside of their rectangle. Centering the
        // stroke on an integer edge would produce half-covered pixels on both
        // sides at unit density and leak into neighboring controls.
        let rect = rect.inset(width * 0.5);
        let path = Path {
            elements: vec![
                PathElement::Move([rect.x, rect.y]),
                PathElement::Line([rect.x + rect.width, rect.y]),
                PathElement::Line([rect.x + rect.width, rect.y + rect.height]),
                PathElement::Line([rect.x, rect.y + rect.height]),
                PathElement::Close,
            ],
        };
        self.list.ops.push(PaintOp::Stroke {
            path: Arc::new(path),
            stroke: Stroke {
                width,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                brush: rgba(color).into(),
            },
        });
    }
    pub fn label(
        &mut self,
        rect: Rect,
        value: &str,
        size: f32,
        color: RGBA,
        horizontal: TheHorizontalAlign,
        vertical: TheVerticalAlign,
    ) {
        let layout = self.text.label_layout(value.trim_end(), "Roboto", size);
        let x = rect.x
            + match horizontal {
                TheHorizontalAlign::Left => 0.,
                TheHorizontalAlign::Center => ((rect.width - layout.width) * 0.5).floor(),
                TheHorizontalAlign::Right => (rect.width - layout.width).floor(),
            };
        let y = rect.y
            + match vertical {
                TheVerticalAlign::Top => 0.,
                TheVerticalAlign::Center => ((rect.height - layout.height) * 0.5).floor(),
                TheVerticalAlign::Bottom => (rect.height - layout.height).floor(),
            };
        self.list.ops.push(PaintOp::PushClip(rect));
        for run in layout.runs {
            self.list.ops.push(PaintOp::GlyphRun {
                run,
                origin: [x, y],
                color: rgba(color),
            });
        }
        self.list.ops.push(PaintOp::PopClip);
    }
    pub fn copied_framebuffer(
        &mut self,
        key: &str,
        image: &TheRGBABuffer,
        rect: Rect,
        changed: bool,
    ) -> UiPaintResult {
        if image.pixel_width() == 0 || image.pixel_height() == 0 {
            return Ok(());
        }
        let handle = self.images.copied_framebuffer(key, image, changed)?;
        self.list.image(handle, rect, Sampling::Linear);
        Ok(())
    }
    pub fn image(
        &mut self,
        key: &str,
        image: &TheRGBABuffer,
        rect: Rect,
        changed: bool,
        sampling: Sampling,
    ) -> UiPaintResult {
        if image.pixel_width() == 0 || image.pixel_height() == 0 {
            return Ok(());
        }
        let handle = self.images.image(key, image, changed)?;
        self.list.image(handle, rect, sampling);
        Ok(())
    }
}

/// Position transient UI in screen space using the same transform as its owner.
pub fn place_popup(
    ctx: &TheContext,
    id: &TheId,
    dim: &TheDim,
    size: [f32; 2],
) -> (Rect, Vec2<i32>) {
    let transform = ctx
        .ui
        .input_transforms
        .get(&id.uuid)
        .copied()
        .unwrap_or_default();
    let anchor = transform.point([dim.x as f32, dim.y as f32]);
    let width = size[0].min(ctx.width as f32);
    let x = anchor[0].clamp(0., (ctx.width as f32 - width - 5.).max(0.));
    let y = (anchor[1] + 20.).clamp(0., (ctx.height as f32 - size[1] - 5.).max(0.));
    (
        Rect::new(x, y, width, size[1]),
        Vec2::new((anchor[0] - x) as i32, (anchor[1] + 20. - y) as i32),
    )
}
