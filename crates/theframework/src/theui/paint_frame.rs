use super::paint::{UiImageResources, UiPaintError, WidgetPainter, bounds};
use super::*;
use crate::paint::{PaintList, PaintOp, Rect};

impl TheUI {
    pub fn needs_paint(&mut self, ctx: &TheContext) -> bool {
        let reason = if self.paint_font_revision != ctx.text.font_revision() {
            Some("fonts")
        } else if self.is_dirty {
            Some("UI")
        } else if ctx.ui.needs_full_paint() {
            Some("redraw_all")
        } else if ctx.ui.needs_layout() {
            Some("relayout")
        } else if ctx.ui.needs_frame() {
            Some("scene")
        } else if self.canvas.needs_redraw() {
            Some("canvas")
        } else if self.dialog.as_mut().is_some_and(|c| c.needs_redraw()) {
            Some("dialog")
        } else if self.popover.as_mut().is_some_and(|p| p.canvas.needs_redraw()) {
            Some("popover")
        } else {
            None
        };
        if let Some(reason) = reason
            && std::env::var_os("ELDIRON_PROFILE_INVALIDATION").is_some()
        {
            eprintln!("UI paint requested by {reason}");
        }
        reason.is_some()
    }

    pub fn prepare_paint_layout(&mut self, ctx: &mut TheContext) {
        if self.paint_font_revision != ctx.text.font_revision() {
            self.paint_font_revision = ctx.text.font_revision();
            ctx.ui.invalidate_layout();
            ctx.ui.invalidate_resources();
        }
        if !ctx.ui.command_paint {
            ctx.ui.command_paint = true;
            ctx.ui.invalidate_layout();
            self.canvas.release_raster_cache();
            if let Some(dialog) = &mut self.dialog {
                dialog.release_raster_cache();
            }
            if let Some(popover) = &mut self.popover {
                popover.canvas.release_raster_cache();
            }
        }
        if ctx.width == 0 || ctx.height == 0 {
            return;
        }
        if self.canvas.resize(ctx.width as i32, ctx.height as i32, ctx) {
            ctx.ui.send(TheEvent::Resize);
            ctx.ui.layout_completed();
        }
        if ctx.ui.needs_layout() {
            self.relayout(ctx);
        }
        ctx.ui.input_transforms.clear();
        let theme_owner = self.style.theme();
        let _theme = theme_owner.as_ref();
        self.canvas.prepare_paint(ctx, _theme);
        if let Some(dialog) = &mut self.dialog {
            dialog.prepare_paint(ctx, _theme);
        }
        if let Some(popover) = &mut self.popover {
            popover.canvas.prepare_paint(ctx, _theme);
        }
        if let Some(menu) = &mut self.context_menu {
            menu.prepare_commands_layout(ctx);
        }
        self.synchronize_editor_scope(ctx);
    }

    /// Prepare and paint the UI tree directly. Unsupported widget families return
    /// an explicit error instead of allocating a root CPU framebuffer fallback.
    pub fn paint_commands(
        &mut self,
        ctx: &mut TheContext,
        images: &mut dyn UiImageResources,
    ) -> Result<PaintList, UiPaintError> {
        self.prepare_paint_layout(ctx);
        if ctx.width == 0 || ctx.height == 0 {
            return Ok(PaintList::default());
        }
        let mut list = PaintList::default();
        let theme_owner = self.style.theme();
        let theme = theme_owner.as_mut();
        let frame_key = theme
            .paint_cache_key()
            .zip(images.retained_domain())
            .map(|(theme, images)| super::retained_paint::FrameKey {
                theme,
                fonts: ctx.text.font_stamp(),
                images,
                density: [ctx.scale_factor.to_bits(), ctx.ui_render_scale.to_bits()],
                icons: ctx.ui.icon_revision,
            });
        let retained_frame = super::retained_paint::RetainedPaint::begin(
            &ctx.ui.retained_paint,
            frame_key,
            ctx.ui.needs_full_paint(),
        );
        let mut p = WidgetPainter {
            list: &mut list,
            text: &mut ctx.text,
            images,
            ui: &ctx.ui,
            theme,
        };
        self.canvas.paint(&mut p)?;
        if let Some(dialog) = &mut self.dialog {
            let r = bounds(&dialog.dim);
            let header = p.theme.metric(PopupHeaderHeight);
            let border = p.theme.metric(PopupBorderWidth);
            let radius = p.theme.metric(PopupCornerRadius);
            let outer = Rect::new(
                r.x - border,
                r.y - header,
                r.width + border * 2.,
                r.height + header + border,
            );
            p.theme_fill(outer, radius, PopupFrame);
            p.theme_fill(
                Rect::new(
                    outer.x + border,
                    outer.y + border,
                    outer.width - border * 2.,
                    outer.height - border * 2.,
                ),
                (radius - border).max(0.),
                PopupBody,
            );
            let title = Rect::new(r.x, outer.y + border, r.width, header - border);
            p.theme_fill(title, 0., PopupHeader);
            p.theme_fill(
                Rect::new(title.x, title.y + title.height - 1., title.width, 1.),
                0.,
                PopupSeparator,
            );
            p.label(
                Rect::new(
                    title.x + 10.,
                    title.y,
                    (title.width - 20.).max(0.),
                    title.height,
                ),
                &self.dialog_text,
                15.,
                *p.theme.color(MenuText),
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
            dialog.paint(&mut p)?;
        }
        if let Some(popover) = &mut self.popover {
            let r = bounds(&popover.canvas.dim);
            let border = p.theme.metric(PopoverBorderWidth);
            let radius = p.theme.metric(PopoverCornerRadius);
            let frame = Rect::new(
                r.x - border,
                r.y - border,
                r.width + border * 2.,
                r.height + border * 2.,
            );
            p.theme_fill(
                Rect::new(frame.x + 4., frame.y + 5., frame.width, frame.height),
                radius + 2.,
                PopoverShadow,
            );
            p.theme_fill(frame, radius, PopoverFrame);
            p.theme_fill(r, (radius - border).max(0.), PopoverBody);
            popover.canvas.paint(&mut p)?;
        }
        // Overlays are outside canvas clips and must remain above embedded scenes.
        if let Some(overlay) = &p.ui.overlay {
            let widget = if let Some(dialog) = &mut self.dialog {
                dialog.get_widget(None, Some(&overlay.uuid))
            } else if let Some(popover) = &mut self.popover {
                popover.canvas.get_widget(None, Some(&overlay.uuid))
            } else {
                self.canvas.get_widget(None, Some(&overlay.uuid))
            };
            if let Some(widget) = widget {
                widget.paint_overlay(&mut p)?;
            }
        }
        if let Some(help) = self.hover_help.as_ref().filter(|h| h.visible) {
            let max_width = (ctx.width as f32 - 26.).clamp(1., 340.);
            let text = p.text.layout(&help.text, "Roboto", 12.5, Some(max_width));
            let d = Self::hover_help_rect(
                help.anchor,
                text.width().ceil() as i32 + 18,
                text.height().ceil() as i32 + 10,
                ctx.width as i32,
                ctx.height as i32,
            );
            let r = bounds(&d);
            p.fill(r, ContextMenuBackground);
            p.outline(r, *p.theme.color(ContextMenuBorder), 1.);
            p.list.ops.push(PaintOp::PushClip(r));
            for run in text.runs {
                p.list.ops.push(PaintOp::GlyphRun {
                    run,
                    origin: [r.x + 9., r.y + 5.],
                    color: p.color(ContextMenuTextNormal),
                });
            }
            p.list.ops.push(PaintOp::PopClip);
        }
        if let Some(drop) = &p.ui.drop
            && let Some(position) = drop.position
        {
            let r = Rect::new(
                (position.x - drop.offset.x) as f32,
                (position.y - drop.offset.y) as f32,
                drop.image.dim().width as f32,
                drop.image.dim().height as f32,
            );
            if drop.image.pixels().is_empty() {
                p.button(
                    r.inset(2.),
                    2.,
                    crate::prelude::TheThemePaints::ControlNormal,
                    *p.theme.color(DropItemBorder),
                );
                p.list.rect(r.inset(3.), 1., p.color(DropItemBackground));
                p.label(
                    r,
                    &drop.title,
                    12.5,
                    *p.theme.color(DropItemText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            } else {
                p.image(
                    &format!("drag:{}", drop.id.uuid),
                    &drop.image,
                    r,
                    false,
                    crate::paint::Sampling::Linear,
                )?;
            }
        }
        if let Some(menu) = &self.context_menu {
            menu.paint_commands(&mut p)?;
        }
        drop(p);
        drop(retained_frame);
        list.validate().map_err(|e| UiPaintError(e.into()))?;
        ctx.ui.paint_completed();
        self.is_dirty = false;
        Ok(list)
    }
}
impl TheContextMenu {
    fn prepare_commands_layout(&mut self, ctx: &mut TheContext) {
        let mut y = self.dim.y + 8;
        for item in &mut self.items {
            let height = if item.name.is_empty() {
                self.item_height / 2
            } else {
                self.item_height
            };
            if let Some(menu) = &mut item.sub_menu {
                menu.is_open =
                    Some(item.id.clone()) == self.hovered && !ctx.ui.is_disabled(&item.id.name);
                if menu.is_open {
                    menu.set_position(Vec2::new(self.dim.x + self.width - 1, y), ctx);
                    menu.cascading_y_offset = y - self.dim.y;
                    menu.prepare_commands_layout(ctx);
                }
            }
            y += height;
        }
    }
    fn paint_commands(&self, p: &mut WidgetPainter<'_>) -> Result<(), UiPaintError> {
        let outer = bounds(&self.dim);
        p.fill(outer, ContextMenuBorder);
        p.fill(
            Rect::new(
                outer.x + 1.,
                outer.y + 1.,
                (outer.width - 2.).max(0.),
                (outer.height - 2.).max(0.),
            ),
            ContextMenuBackground,
        );
        let mut y = outer.y + 8.;
        for item in &self.items {
            let height = if item.name.is_empty() {
                self.item_height as f32 / 2.
            } else {
                self.item_height as f32
            };
            let r = Rect::new(outer.x + 1., y, (self.width - 2).max(0) as f32, height);
            let disabled = p.ui.is_disabled(&item.id.name);
            let hover = Some(item.id.clone()) == self.hovered && !disabled;
            let mut color = item
                .text_color
                .unwrap_or(*p.theme.color(ContextMenuTextNormal));
            if disabled {
                color = *p.theme.color(ContextMenuTextDisabled);
            } else if hover {
                p.fill(r, ContextMenuHighlight);
                color = *p.theme.color(ContextMenuTextHighlight);
            }
            if item.name.is_empty() {
                p.fill(
                    Rect::new(r.x, r.y + r.height / 2., r.width, 1.),
                    ContextMenuSeparator,
                );
            } else {
                p.label(
                    Rect::new(r.x + 16., r.y, (r.width - 16.).max(0.), r.height),
                    &item.name,
                    13.5,
                    color,
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }
            if let Some(accel) = &item.accel {
                p.label(
                    Rect::new(r.x, r.y, (r.width - 6.).max(0.), r.height),
                    &accel.description(),
                    12.,
                    *p.theme.color(ContextMenuTextDisabled),
                    TheHorizontalAlign::Right,
                    TheVerticalAlign::Center,
                );
            } else if item.sub_menu.is_some() {
                p.label(
                    Rect::new(r.x + r.width - 18., r.y, 12., r.height),
                    "›",
                    14.,
                    color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
            y += height;
        }
        for item in &self.items {
            if let Some(menu) = &item.sub_menu
                && menu.is_open
            {
                menu.paint_commands(p)?;
            }
        }
        Ok(())
    }
}
