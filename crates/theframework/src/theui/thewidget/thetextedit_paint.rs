//! Parley is the common source of shaped paint, grapheme hit testing and carets.
use super::*;
use crate::{
    paint::{PaintOp, Rect, Transform},
    text::{TextStyleSpan, TextSystem},
    theui::paint::{WidgetPainter, rgba},
};
impl TheTextRenderer {
    pub(super) fn layout_byte(&self, index: usize) -> usize {
        self.command_graphemes
            .get(index)
            .copied()
            .unwrap_or_else(|| {
                self.command_layout
                    .as_ref()
                    .map(|l| l.source.len())
                    .unwrap_or(0)
            })
    }
    pub fn commands_ready(&self) -> bool {
        self.command_layout.is_some()
    }
    pub fn prepare_commands(
        &mut self,
        text: &str,
        highlight_source: Option<&str>,
        family: &str,
        fonts: &mut TextSystem,
    ) {
        let mut styles = Vec::new();
        if let Some(highlighter) = &self.highlighter {
            let mut h = syntect::easy::HighlightLines::new(
                highlighter.syntect_syntax(),
                highlighter.syntect_theme(),
            );
            let mut offset = 0;
            let mut sources = highlight_source.map(|s| s.split('\n'));
            for visible in text.split_inclusive('\n') {
                let line = visible.strip_suffix('\n').unwrap_or(visible);
                let source = sources.as_mut().and_then(|s| s.next()).unwrap_or(line);
                let tokens = highlighter.highlight_line(source, &mut h);
                if highlight_source.is_some() {
                    if let Some((color, _, _)) = tokens.into_iter().max_by_key(|v| v.2) {
                        styles.push(TextStyleSpan {
                            range: offset..offset + line.len(),
                            color: rgba(color.to_u8_array()),
                        });
                    }
                } else {
                    let mut indices: Vec<_> = line.grapheme_indices(true).map(|v| v.0).collect();
                    indices.push(line.len());
                    let mut g = 0;
                    for (color, _, count) in tokens {
                        let end = (g + count).min(indices.len() - 1);
                        styles.push(TextStyleSpan {
                            range: offset + indices[g]..offset + indices[end],
                            color: rgba(color.to_u8_array()),
                        });
                        g = end;
                    }
                }
                offset += visible.len();
            }
        }
        let layout = fonts.layout_styled(text, family, self.font_size, self.max_width, &styles);
        self.command_graphemes = text.grapheme_indices(true).map(|v| v.0).collect();
        self.command_graphemes.push(text.len());
        self.glyphs = self
            .command_graphemes
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let c = layout.caret(*b);
                let end = layout.caret(self.command_graphemes.get(i + 1).copied().unwrap_or(*b));
                TheGlyph {
                    parent: text[*b..].chars().next().unwrap_or('\n'),
                    x: c.x,
                    width: (end.x - c.x).abs().ceil() as usize,
                }
            })
            .collect();
        self.row_info = layout
            .lines()
            .into_iter()
            .map(|line| {
                let start = self
                    .command_graphemes
                    .partition_point(|b| *b < line.range.start)
                    .min(self.glyphs.len() - 1);
                let end = self
                    .command_graphemes
                    .partition_point(|b| *b < line.range.end)
                    .saturating_sub(1)
                    .max(start)
                    .min(self.glyphs.len() - 1);
                TheRowInfo {
                    top: line.top.max(0.).floor() as usize,
                    bottom: line.bottom.max(0.).ceil() as usize,
                    left: 0,
                    right: line.advance.ceil() as usize,
                    baseline: line.baseline.ceil() as usize,
                    glyph_start: start,
                    glyph_end: end,
                    highlights: None,
                }
            })
            .collect();
        self.actual_size = Vec2::new(
            layout.width().ceil() as usize,
            layout.height().ceil() as usize,
        );
        self.command_layout = Some(layout);
        self.scroll(&Vec2::zero(), false);
    }
    pub fn command_viewport(&mut self, rect: Rect, inset: [f32; 2]) {
        self.set_dim(
            rect.x.max(0.) as usize,
            rect.y.max(0.) as usize,
            rect.width.max(0.) as usize,
            rect.height.max(0.) as usize,
        );
        self.command_hit_inset = inset;
    }
    pub fn paint_commands(
        &self,
        state: &TheTextEditState,
        focused: bool,
        readonly: bool,
        p: &mut WidgetPainter<'_>,
    ) {
        let Some(layout) = &self.command_layout else {
            return;
        };
        let viewport = Rect::new(
            self.left as f32,
            self.top as f32,
            self.width as f32,
            self.height as f32,
        );
        p.list.ops.push(PaintOp::PushClip(viewport));
        p.list.ops.push(PaintOp::PushTransform(Transform::translate(
            viewport.x - self.scroll_offset.x as f32,
            viewport.y - self.scroll_offset.y as f32,
        )));
        if let Some(line) = self.debug_line.and_then(|i| self.row_info.get(i)) {
            p.fill(
                Rect::new(
                    0.,
                    line.top as f32,
                    self.actual_size.x.max(self.width) as f32,
                    (line.bottom - line.top) as f32,
                ),
                TextEditDebugLineBackground,
            );
        }
        for (index, (start, end)) in self.matches.iter().enumerate() {
            let color = if index == self.highlighted_match {
                [0.9, 0.65, 0.1, 0.65]
            } else {
                [0.7, 0.5, 0.1, 0.35]
            };
            for r in layout.selection(self.layout_byte(*start), self.layout_byte(*end)) {
                p.list.rect(r, 0., color);
            }
        }
        if !state.selection.is_none() {
            let color = self
                .highlighter
                .as_ref()
                .and_then(|h| h.selection_background())
                .map(|c| rgba(c.to_u8_array()))
                .unwrap_or_else(|| p.color(DefaultSelection));
            for r in layout.selection(
                self.layout_byte(state.selection.start),
                self.layout_byte(state.selection.end),
            ) {
                p.list.rect(r, 0., color);
            }
        }
        for (run, color) in &layout.styled_runs {
            p.list.ops.push(PaintOp::GlyphRun {
                run: run.clone(),
                origin: [0.; 2],
                color: color.unwrap_or_else(|| p.color(TextEditTextColor)),
            });
        }
        for (start, end) in &self.errors {
            for r in layout.selection(self.layout_byte(*start), self.layout_byte(*end)) {
                p.list.rect(
                    Rect::new(r.x, r.y + r.height - 1., r.width, 1.),
                    0.,
                    [1., 0.2, 0.2, 1.],
                );
            }
        }
        if self.indicate_space {
            for (i, glyph) in self.glyphs.iter().enumerate() {
                if glyph.parent == ' ' {
                    let r = layout.caret(self.layout_byte(i));
                    p.list.rect(
                        Rect::new(r.x + 2., r.y + r.height * 0.6, 1., 1.),
                        0.,
                        p.color(TextEditLineNumberColor),
                    );
                }
            }
        }
        if focused && !readonly {
            let c = layout.caret(self.layout_byte(state.find_cursor_index()));
            if let Some((text, cursor)) = &self.composition {
                let preedit = p.text.layout(text, "Roboto", self.font_size, None);
                for run in &preedit.runs {
                    p.list.ops.push(PaintOp::GlyphRun {
                        run: run.clone(),
                        origin: [c.x, c.y],
                        color: p.color(TextEditTextColor),
                    });
                }
                p.list.rect(
                    Rect::new(c.x, c.y + preedit.height() - 1., preedit.width(), 1.),
                    0.,
                    p.color(TextEditCursorColor),
                );
                let caret = preedit.caret(cursor.map(|v| v.1).unwrap_or(text.len()));
                p.fill(
                    Rect::new(c.x + caret.x, c.y, 1., caret.height),
                    TextEditCursorColor,
                );
            } else {
                p.fill(
                    Rect::new(c.x, c.y, self.cursor_width as f32, c.height),
                    TextEditCursorColor,
                );
            }
        }
        p.list.ops.push(PaintOp::PopTransform);
        p.list.ops.push(PaintOp::PopClip);
    }
}
