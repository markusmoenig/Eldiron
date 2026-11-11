//! Rich display paint delegates text geometry/selection to the common editor.
use super::super::thetextdisplay_native::DisplayOptions;
use super::*;
use crate::{
    paint::PaintOp,
    theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba},
};
impl TheTextView {
    pub(super) fn prepare_commands(&mut self, ctx: &mut TheContext) {
        let mut styles = self.styled_ranges.clone();
        if let Some(index) = self.hovered_interaction {
            if let Some((range, interaction)) = self.interactive_ranges.get(index) {
                if let Some(bg) = &interaction.hover_background {
                    apply_interaction_background(&mut styles, range, bg);
                }
            }
        }
        if self.pending_scroll_to_bottom {
            self.native.request_scroll(true);
            self.pending_scroll_to_bottom = false;
        }
        let outer = bounds(&self.dim).inset(if self.draw_border { 1. } else { 0. });
        self.native.prepare(
            &self.id,
            &self.state,
            DisplayOptions {
                outer,
                padding: self.renderer.padding,
                scrollbar: self.scrollbar_size as f32,
                wrap: self.word_wrap,
                selectable: self.selectable,
                family: match self.font_preference {
                    TheFontPreference::Default => "Roboto",
                    TheFontPreference::Code => "Source Code Pro",
                },
                size: self.renderer.font_size,
            },
            &styles,
            ctx,
        );
    }
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let r = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(r));
        if let Some(bg) = &self.background_override {
            p.list.rect(r, 0., rgba(bg.to_u8_array()));
        }
        if self.draw_background {
            p.text_input(r, false, true, self.embedded, true, self.draw_border);
        } else {
            p.text_input_frame(r, false, true, self.embedded, true, self.draw_border);
        }
        self.native.paint(p);
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
}
