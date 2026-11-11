//! Markdown resolves semantic document spans before shared shaped text layout.
use super::*;
use crate::{
    paint::{PaintOp, Rect},
    theui::paint::{UiPaintResult, WidgetPainter, bounds},
};
impl TheMarkdownView {
    pub(super) fn prepare_commands(&mut self, ctx: &mut TheContext, theme: &dyn TheTheme) {
        let border = if self.draw_border { 1. } else { 0. };
        let r = bounds(&self.dim).inset(border);
        let padding = self.renderer.padding;
        let content = crate::theui::text_viewport::text_content_rect(r, padding);
        let bar = self.scrollbar_size as f32;
        let width = if self.word_wrap {
            Some((content.width - bar).max(1.))
        } else {
            None
        };
        let styles = self.resolved_styles(theme);
        let changed = styles != self.command_styles
            || self.is_dirty
            || !self.renderer.commands_ready()
            || width != self.renderer.max_width;
        self.renderer.max_width = width;
        if changed {
            self.renderer.prepare_styled_commands(
                &self.state.to_text(),
                None,
                "Roboto",
                &styles,
                &mut ctx.text,
            );
            self.command_styles = styles;
        }
        let plan = crate::theui::text_viewport::plan_text_viewport(
            r,
            padding,
            [
                self.renderer.actual_size.x as f32,
                self.renderer.actual_size.y as f32,
            ],
            bar,
            self.word_wrap,
        );
        self.renderer.command_viewport(
            plan.viewport,
            [border + padding.0 as f32, border + padding.1 as f32],
        );
        if let Some(r) = plan.horizontal {
            self.hscrollbar.set_dim(
                TheDim::new(r.x as i32, r.y as i32, r.width as i32, r.height as i32),
                ctx,
            );
        }
        if let Some(r) = plan.vertical {
            self.vscrollbar.set_dim(
                TheDim::new(r.x as i32, r.y as i32, r.width as i32, r.height as i32),
                ctx,
            );
        }
        if let Some(scroll) = self.hscrollbar.as_horizontal_scrollbar() {
            scroll.set_total_width(self.renderer.actual_size.x as i32 + padding.0 + padding.2);
            scroll.set_scroll_offset(self.renderer.scroll_offset.x as i32);
        }
        if let Some(scroll) = self.vscrollbar.as_vertical_scrollbar() {
            scroll.set_total_height(self.renderer.actual_size.y as i32 + padding.1 + padding.3);
            scroll.set_scroll_offset(self.renderer.scroll_offset.y as i32);
        }
    }
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let r = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(r));
        if self.draw_background {
            self.renderer_paint_background(r, p);
        } else {
            p.text_input_frame(r, false, true, self.embedded, true, self.draw_border);
        }
        self.renderer.paint_styled_commands(
            &self.state,
            self.id.equals(&p.ui.focus),
            true,
            &self.command_styles,
            p,
        );
        if self.renderer.is_horizontal_overflow() {
            let r = bounds(self.hscrollbar.dim());
            let scroll = self.hscrollbar.as_horizontal_scrollbar().unwrap();
            let width = scroll.scrollbar_thumb_width().clamp(0, r.width as i32) as f32;
            let offset = scroll
                .scrollbar_position()
                .round()
                .clamp(0., (r.width - width).max(0.));
            p.scrollbar(
                r,
                Rect::new(r.x + offset, r.y + 2., width, (r.height - 4.).max(0.)),
                self.is_hscrollbar_hovered,
                self.is_hscrollbar_clicked,
            );
        }
        if self.renderer.is_vertical_overflow() {
            let r = bounds(self.vscrollbar.dim());
            let scroll = self.vscrollbar.as_vertical_scrollbar().unwrap();
            let height = scroll.scrollbar_thumb_height().clamp(0, r.height as i32) as f32;
            let offset = scroll
                .scrollbar_position()
                .round()
                .clamp(0., (r.height - height).max(0.));
            p.scrollbar(
                r,
                Rect::new(r.x + 2., r.y + offset, (r.width - 4.).max(0.), height),
                self.is_vscrollbar_hovered,
                self.is_vscrollbar_clicked,
            );
        }
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
    fn resolved_styles(&self, theme: &dyn TheTheme) -> Vec<(Range<usize>, TheTextStyle)> {
        self.md_nodes
            .iter()
            .enumerate()
            .map(|(index, (range, node))| {
                let color = match node {
                    TheMarkdownNode::Emphasis => self.styles.emphasis.clone(),
                    TheMarkdownNode::Heading => self.styles.heading.clone(),
                    TheMarkdownNode::Strong => self.styles.strong.clone(),
                    TheMarkdownNode::Text => self.styles.text.clone(),
                    TheMarkdownNode::Link(_) => {
                        let hovered = self.link_hovered == Some(index);
                        if hovered {
                            self.styles.link_hovered.clone().or_else(|| {
                                Some(TheColor::from_u8_array(*theme.color(TextLinkHoveredColor)))
                            })
                        } else {
                            self.styles.link.clone().or_else(|| {
                                Some(TheColor::from_u8_array(*theme.color(TextLinkColor)))
                            })
                        }
                    }
                };
                (
                    range.clone(),
                    TheTextStyle {
                        foreground: color.clone(),
                        underline: if node.is_link() { color } else { None },
                        background: None,
                    },
                )
            })
            .filter(|(_, style)| !style.is_empty())
            .collect()
    }
    fn renderer_paint_background(&self, r: Rect, p: &mut WidgetPainter<'_>) {
        p.text_input(r, false, true, self.embedded, true, self.draw_border);
    }
}
