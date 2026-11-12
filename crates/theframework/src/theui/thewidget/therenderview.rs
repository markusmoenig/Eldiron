use crate::prelude::*;

/// Presentation content supplied by the application. Images and command lists
/// share the host compositor; neither implies ownership of a GPU or window.
#[cfg(feature = "text")]
enum NativeContent {
    Cleared,
    Image(crate::paint::ImageHandle),
    Commands(std::sync::Arc<crate::paint::PaintList>),
}

pub struct TheRenderView {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    render_buffer: TheRGBABuffer,
    #[cfg(feature = "text")]
    native_content: Option<NativeContent>,
    empty_background: Option<TheThemeColors>,
    has_content: bool,
    wheel_scale: f32,
    aspect_scaled_scroll: bool,
    accumulated_wheel_delta: Vec2<f32>,
    context_menu: Option<TheContextMenu>,

    dim: TheDim,

    auto_focus: bool,
    text_input: bool,

    mouse_is_down: bool,
    is_dirty: bool,
}

impl TheRenderView {
    pub fn set_empty_background(&mut self, role: TheThemeColors) {
        self.empty_background = Some(role);
        self.is_dirty = true;
    }
}
impl TheWidget for TheRenderView {
    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::Sampling;
        use crate::theui::paint::bounds;
        let r = bounds(&self.dim);
        if matches!(self.native_content, Some(NativeContent::Cleared)) {
            if let Some(role) = self.empty_background {
                p.fill(r, role);
            }
        } else if let Some(content) = &self.native_content {
            use crate::paint::{PaintOp, Transform};
            // The view's logical viewport is the authority for clipping. A
            // published list uses local coordinates, just like pointer events;
            // parent scroll/layout transforms are already on the paint stack.
            p.list.ops.push(PaintOp::PushClip(r));
            match content {
                NativeContent::Cleared => {}
                NativeContent::Image(image) => p.list.image(*image, r, Sampling::Linear),
                NativeContent::Commands(list) => {
                    p.list
                        .ops
                        .push(PaintOp::PushTransform(Transform::translate(r.x, r.y)));
                    p.list.ops.extend(list.ops.iter().cloned());
                    p.list.ops.push(PaintOp::PopTransform);
                }
            }
            p.list.ops.push(PaintOp::PopClip);
        } else if self.has_content {
            p.copied_framebuffer(
                &format!("renderview:{}", self.id.uuid),
                &self.render_buffer,
                r,
                self.is_dirty,
            )?;
        } else if let Some(role) = self.empty_background {
            p.fill(r, role);
        } else {
            // The reference canvas starts with zero RGB and copies even an
            // untouched RenderView to the opaque display. Preserve that clear
            // without allocating a placeholder framebuffer or uploading it.
            p.list.rect(r, 0., [0., 0., 0., 1.]);
        }
        if Some(self.id.clone()) == p.ui.focus {
            p.outline(r, *p.theme.color(DefaultSelection), 1.);
        }
        self.is_dirty = false;
        Ok(())
    }

    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(20, 20));
        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            #[cfg(feature = "text")]
            native_content: None,
            render_buffer: TheRGBABuffer::empty(),
            empty_background: None,
            has_content: false,
            wheel_scale: -0.4,
            aspect_scaled_scroll: true,
            accumulated_wheel_delta: Vec2::zero(),
            context_menu: None,

            dim: TheDim::zero(),

            auto_focus: false,
            text_input: false,
            mouse_is_down: false,
            is_dirty: false,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::Context(coord) => {
                if let Some(context_menu) = &self.context_menu {
                    ctx.ui.send(TheEvent::ShowContextMenu(
                        self.id().clone(),
                        *coord,
                        context_menu.clone(),
                    ));
                } else {
                    ctx.ui.set_focus(self.id());
                    ctx.ui
                        .send(TheEvent::RenderViewContext(self.id().clone(), *coord));
                    self.mouse_is_down = true;
                    redraw = true;
                }
            }
            TheEvent::MouseDown(coord) => {
                if self.state == TheWidgetState::Selected {
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                } else if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
                ctx.ui.set_focus(self.id());

                ctx.ui
                    .send(TheEvent::RenderViewClicked(self.id().clone(), *coord));

                self.is_dirty = true;
                self.mouse_is_down = true;
                redraw = true;
            }
            TheEvent::MouseDragged(coord) => {
                if self.mouse_is_down {
                    ctx.ui
                        .send(TheEvent::RenderViewDragged(self.id().clone(), *coord));
                }
            }
            TheEvent::MouseUp(coord) => {
                if self.mouse_is_down {
                    ctx.ui
                        .send(TheEvent::RenderViewUp(self.id().clone(), *coord));
                }
                self.mouse_is_down = false;
            }
            TheEvent::Hover(coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }

                ctx.ui
                    .send(TheEvent::RenderViewHoverChanged(self.id().clone(), *coord));

                if self.auto_focus {
                    if self.state != TheWidgetState::Selected {
                        self.state = TheWidgetState::Selected;
                        ctx.ui.send_widget_state_changed(self.id(), self.state);
                    }
                    ctx.ui.set_focus(self.id());

                    self.is_dirty = true;
                    self.mouse_is_down = true;
                    redraw = true;
                }
            }
            TheEvent::LostHover(_) => {
                ctx.ui
                    .send(TheEvent::RenderViewLostHover(self.id().clone()));
            }
            TheEvent::MouseWheel(delta) => {
                let scale_factor = self.wheel_scale; // * 1.0 / (self.zoom.powf(0.5));

                let aspect_ratio = if self.aspect_scaled_scroll {
                    self.dim().width as f32 / self.dim().height.max(1) as f32
                } else {
                    1.0
                };

                let scale_x = if aspect_ratio > 1.0 {
                    1.0 / aspect_ratio
                } else {
                    1.0
                };
                let scale_y = if aspect_ratio < 1.0 {
                    aspect_ratio
                } else {
                    1.0
                };

                // Update accumulated deltas
                self.accumulated_wheel_delta.x += delta.x as f32 * scale_factor * scale_x;
                self.accumulated_wheel_delta.y += delta.y as f32 * scale_factor * scale_y;

                let minimum_delta_threshold = 2.0;

                // Check if accumulated deltas exceed the threshold
                if self.accumulated_wheel_delta.x.abs() > minimum_delta_threshold
                    || self.accumulated_wheel_delta.y.abs() > minimum_delta_threshold
                {
                    // Convert accumulated deltas to integer and reset
                    let d = Vec2::new(
                        self.accumulated_wheel_delta.x as i32,
                        self.accumulated_wheel_delta.y as i32,
                    );
                    self.accumulated_wheel_delta = Vec2::zero();

                    ctx.ui
                        .send(TheEvent::RenderViewScrollBy(self.id().clone(), d));
                }
            }
            TheEvent::PreciseScroll(delta) => {
                let scale_factor = self.wheel_scale;
                let aspect_ratio = if self.aspect_scaled_scroll {
                    self.dim().width as f32 / self.dim().height.max(1) as f32
                } else {
                    1.0
                };
                let scale_x = if aspect_ratio > 1.0 {
                    1.0 / aspect_ratio
                } else {
                    1.0
                };
                let scale_y = if aspect_ratio < 1.0 {
                    aspect_ratio
                } else {
                    1.0
                };

                self.accumulated_wheel_delta.x += delta.x * scale_factor * scale_x;
                self.accumulated_wheel_delta.y += delta.y * scale_factor * scale_y;

                if self.accumulated_wheel_delta.x.abs() > 2.0
                    || self.accumulated_wheel_delta.y.abs() > 2.0
                {
                    let d = Vec2::new(
                        self.accumulated_wheel_delta.x as i32,
                        self.accumulated_wheel_delta.y as i32,
                    );
                    self.accumulated_wheel_delta = Vec2::zero();
                    ctx.ui
                        .send(TheEvent::RenderViewPreciseScrollBy(self.id().clone(), d));
                }
            }
            TheEvent::Pinch(delta) => {
                ctx.ui
                    .send(TheEvent::RenderViewZoomBy(self.id().clone(), *delta));
                redraw = true;
            }
            TheEvent::Drop(coord, drop) => {
                ctx.ui.send(TheEvent::RenderViewDrop(
                    self.id.clone(),
                    *coord,
                    drop.clone(),
                ));
                redraw = true;
            }
            _ => {}
        }
        redraw
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
            ctx.ui.send(TheEvent::WidgetResized(self.id.clone(), dim));
        }
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
    }

    fn set_context_menu(&mut self, menu: Option<TheContextMenu>) {
        self.context_menu = menu;
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        if !self.has_content
            && let Some(role) = self.empty_background
        {
            let stride = buffer.stride();
            ctx.draw.rect(
                buffer.draw_target(),
                &self.dim.to_buffer_utuple(),
                stride,
                style.theme().color(role),
            );
        } else if self.has_content {
            self.render_buffer.scaled_into_using_dim(buffer, &self.dim);
        }

        let stride = buffer.stride();
        if Some(self.id.clone()) == ctx.ui.focus {
            let tuple = self.dim().to_buffer_utuple();
            ctx.draw.rect_outline(
                buffer.draw_target(),
                &tuple,
                stride,
                style.theme().color(DefaultSelection),
            );
        }
        self.is_dirty = false;
    }

    fn supports_text_input(&self) -> bool {
        self.text_input
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn supports_pinch(&self) -> bool {
        true
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_render_view(&mut self) -> Option<&mut dyn TheRenderViewTrait> {
        Some(self)
    }
}

pub trait TheRenderViewTrait: TheWidget {
    #[cfg(feature = "text")]
    /// Publish a shared GPU image, or clear all view content with `None`.
    /// This releases the former CPU viewport cache. The application owns image
    /// residency and republishes handles after device/resource replacement.
    fn set_image_handle(&mut self, image: Option<crate::paint::ImageHandle>);
    /// Publish an immutable, balanced list in view-local logical coordinates.
    /// The view adds its own clip and translation, preserving parent scrolling.
    /// Validation is atomic: an invalid list leaves the previous content intact.
    /// `None` clears the view without revealing an older CPU/image presentation.
    #[cfg(feature = "text")]
    fn set_paint_list(
        &mut self,
        list: Option<std::sync::Arc<crate::paint::PaintList>>,
    ) -> crate::theui::paint::UiPaintResult;
    fn set_scroll_behavior(&mut self, scale: f32, aspect_scaled: bool);
    fn render_buffer_mut(&mut self) -> &mut TheRGBABuffer;
    fn set_auto_focus(&mut self, auto_focus: bool);
    /// Protect host-rendered inline text controls from global shortcuts.
    fn set_text_input(&mut self, active: bool);
}

impl TheRenderViewTrait for TheRenderView {
    #[cfg(feature = "text")]
    fn set_image_handle(&mut self, image: Option<crate::paint::ImageHandle>) {
        if matches!((&self.native_content, image),
            (Some(NativeContent::Image(current)), Some(next)) if *current == next)
            || (matches!(self.native_content, Some(NativeContent::Cleared)) && image.is_none())
        {
            return;
        }
        self.native_content = Some(image.map_or(NativeContent::Cleared, NativeContent::Image));
        self.render_buffer = TheRGBABuffer::empty();
        self.has_content = false;
        self.is_dirty = true;
    }
    #[cfg(feature = "text")]
    fn set_paint_list(
        &mut self,
        list: Option<std::sync::Arc<crate::paint::PaintList>>,
    ) -> crate::theui::paint::UiPaintResult {
        if let Some(list) = &list {
            list.validate()
                .map_err(|e| crate::theui::paint::UiPaintError(e.into()))?;
        }
        if matches!((&self.native_content, &list),
            (Some(NativeContent::Commands(current)), Some(next)) if std::sync::Arc::ptr_eq(current, next))
            || (matches!(self.native_content, Some(NativeContent::Cleared)) && list.is_none())
        {
            return Ok(());
        }
        self.native_content = Some(list.map_or(NativeContent::Cleared, NativeContent::Commands));
        self.render_buffer = TheRGBABuffer::empty();
        self.has_content = false;
        self.is_dirty = true;
        Ok(())
    }
    fn set_scroll_behavior(&mut self, scale: f32, aspect_scaled: bool) {
        self.wheel_scale = scale;
        self.aspect_scaled_scroll = aspect_scaled;
    }
    fn set_text_input(&mut self, active: bool) {
        self.text_input = active;
    }
    fn render_buffer_mut(&mut self) -> &mut TheRGBABuffer {
        #[cfg(feature = "text")]
        {
            self.native_content = None;
        }
        // Preserve the reference adapter's initial 20px source only when a
        // consumer explicitly opts into CPU pixels. Native view construction
        // and image/command publication allocate no mandatory raster storage.
        if !self.has_content && self.render_buffer.pixels().is_empty() {
            self.render_buffer = TheRGBABuffer::new(TheDim::sized(20, 20));
        }
        self.has_content = true;
        self.is_dirty = true;
        &mut self.render_buffer
    }
    fn set_auto_focus(&mut self, auto_focus: bool) {
        self.auto_focus = auto_focus;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "text")]
    #[test]
    fn native_publication_is_atomic_releases_raster_and_never_resurrects_it() {
        use crate::paint::{PaintList, PaintOp, Rect};
        use std::sync::Arc;
        let mut view = TheRenderView::new(TheId::named("native-view"));
        assert!(view.render_buffer.pixels().is_empty());
        assert_eq!(view.render_buffer_mut().pixels().len(), 20 * 20 * 4);
        *view.render_buffer_mut() = TheRGBABuffer::new(TheDim::sized(320, 180));
        view.render_buffer_mut().fill([255, 0, 0, 255]);
        let mut commands = PaintList::default();
        commands.rect(Rect::new(0., 0., 80., 40.), 0., [0., 1., 0., 1.]);
        let commands = Arc::new(commands);
        view.set_paint_list(Some(commands.clone())).unwrap();
        assert!(view.render_buffer.pixels().is_empty());
        assert!(!view.has_content);
        view.set_needs_redraw(false);
        view.set_paint_list(Some(commands.clone())).unwrap();
        assert!(
            !view.needs_redraw(),
            "republishing the retained list is idle"
        );
        let malformed = Arc::new(PaintList {
            ops: vec![PaintOp::PopTransform],
        });
        assert!(view.set_paint_list(Some(malformed)).is_err());
        assert!(!view.needs_redraw());
        assert!(matches!(&view.native_content,
            Some(NativeContent::Commands(current)) if Arc::ptr_eq(current, &commands)));
        let image = crate::paint::ImageHandle {
            owner: 1,
            slot: 2,
            generation: 3,
        };
        view.set_image_handle(Some(image));
        assert!(
            matches!(view.native_content, Some(NativeContent::Image(current)) if current == image)
        );
        view.set_image_handle(None);
        assert!(matches!(view.native_content, Some(NativeContent::Cleared)));
        assert!(!view.has_content);
        assert!(view.render_buffer.pixels().is_empty());
        view.set_paint_list(Some(commands)).unwrap();
        view.set_paint_list(None).unwrap();
        assert!(matches!(view.native_content, Some(NativeContent::Cleared)));
        assert!(!view.has_content);
        // Explicit reference/image consumers can still opt into owned pixels.
        *view.render_buffer_mut() = TheRGBABuffer::new(TheDim::sized(16, 8));
        assert!(view.native_content.is_none());
        assert!(view.has_content);
        assert_eq!(view.render_buffer.pixels().len(), 16 * 8 * 4);
    }

    #[test]
    fn subpixel_touchpad_motion_accumulates_into_canvas_scroll() {
        let mut ctx = TheContext::new(400, 400, 1.);
        let (sender, receiver) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(sender);
        let mut view = TheRenderView::new(TheId::named("Canvas"));
        view.set_dim(TheDim::new(0, 0, 400, 400), &mut ctx);
        view.set_scroll_behavior(1., false);
        for _ in 0..12 {
            view.on_event(&TheEvent::PreciseScroll(Vec2::new(0., 0.25)), &mut ctx);
        }
        assert!(receiver.try_iter().any(|event| matches!(
            event,
            TheEvent::RenderViewPreciseScrollBy(_, delta) if delta.y >= 2
        )));
    }
}
