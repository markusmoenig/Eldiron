//! Rich display paint delegates text geometry/selection to the common editor.
use super::super::thetextdisplay_native::DisplayOptions;
use super::*;
use crate::{
    paint::PaintOp,
    theui::paint::{UiPaintResult, WidgetPainter, bounds},
};
impl TheMarkdownView {
    pub(super) fn prepare_commands(&mut self, ctx: &mut TheContext, theme: &dyn TheTheme) {
        let styles = self.resolved_styles(theme);
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
                family: "Roboto",
                size: self.renderer.font_size,
            },
            &styles,
            ctx,
        );
    }
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let r = bounds(&self.dim);
        p.list.ops.push(PaintOp::PushClip(r));
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
}
