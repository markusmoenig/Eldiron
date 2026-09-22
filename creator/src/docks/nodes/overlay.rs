//! A detached, multi-line editor for node values that do not fit one row:
//! dialogue lines, messages and other long strings. The graph keeps the
//! document; this only owns the draft text, caret, selection and scroll while
//! the overlay is open.

use std::ops::Range;
use theframework::thegraph::*;

const PAD: f32 = 14.;
const LINE: f32 = 24.;
const FONT: f32 = 15.;
/// Room kept at the bottom for the owner's hint line, so text never collides
/// with it however small the overlay is.
const HINT: f32 = 24.;

/// Which document value the overlay edits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTarget {
    pub node: GraphId,
    pub row: GraphId,
    pub cell: Option<(usize, usize)>,
}

fn advance(font: &fontdue::Font, text: &str, size: f32) -> f32 {
    text.chars()
        .map(|ch| font.metrics(ch, size).advance_width)
        .sum()
}

pub struct TextOverlay {
    pub target: TextTarget,
    text: String,
    original: String,
    caret: usize,
    anchor: usize,
    scroll: f32,
    /// Whole overlay rectangle in view space.
    pub rect: GraphRect,
    /// Wrapped lines from the last layout: byte range and top offset.
    lines: Vec<(Range<usize>, f32)>,
}

impl TextOverlay {
    pub fn new(target: TextTarget, text: impl Into<String>, rect: GraphRect) -> Self {
        let text = text.into();
        let caret = text.len();
        Self {
            target,
            original: text.clone(),
            text,
            caret,
            anchor: caret,
            scroll: 0.,
            rect,
            lines: vec![(0..0, 0.)],
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    pub fn changed(&self) -> bool {
        self.text != self.original
    }

    /// Wrapped line count from the last layout; used by tests and diagnostics.
    #[cfg(test)]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn contains(&self, point: [f32; 2]) -> bool {
        self.rect.contains(point)
    }

    /// Selected text, or the whole draft when nothing is selected.
    pub fn copy(&self) -> String {
        let selection = self.selection();
        if selection.is_empty() {
            self.text.clone()
        } else {
            self.text[selection].to_string()
        }
    }

    /// Remove and return the selection (or the whole draft when empty).
    pub fn cut(&mut self) -> String {
        let text = self.copy();
        let selection = self.selection();
        if selection.is_empty() {
            self.text.clear();
            self.caret = 0;
            self.anchor = 0;
        } else {
            self.caret = selection.start;
            self.text.replace_range(selection, "");
            self.anchor = self.caret;
        }
        text
    }

    pub fn paste(&mut self, text: &str) {
        let text: String = text
            .chars()
            .filter(|c| *c == '\n' || !c.is_control())
            .collect();
        self.input(GraphTextInput::Insert(text));
    }

    fn prev(text: &str, at: usize) -> usize {
        text[..at]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn next(text: &str, at: usize) -> usize {
        text[at..]
            .char_indices()
            .nth(1)
            .map(|(index, _)| at + index)
            .unwrap_or(text.len())
    }

    pub fn input(&mut self, input: GraphTextInput) {
        match input {
            GraphTextInput::SelectAll => {
                self.anchor = 0;
                self.caret = self.text.len();
            }
            GraphTextInput::Insert(value) => {
                let value: String = value
                    .chars()
                    .filter(|c| *c == '\n' || !c.is_control())
                    .collect();
                let range = self.selection();
                self.caret = range.start + value.len();
                self.text.replace_range(range, &value);
                self.anchor = self.caret;
            }
            GraphTextInput::Backspace | GraphTextInput::Delete => {
                let mut range = self.selection();
                if range.is_empty() {
                    if matches!(input, GraphTextInput::Backspace) {
                        range.start = Self::prev(&self.text, self.caret);
                    } else {
                        range.end = Self::next(&self.text, self.caret);
                    }
                }
                self.caret = range.start;
                self.text.replace_range(range, "");
                self.anchor = self.caret;
            }
            GraphTextInput::Left { extend } => {
                self.caret = if !extend && !self.selection().is_empty() {
                    self.selection().start
                } else {
                    Self::prev(&self.text, self.caret)
                };
                if !extend {
                    self.anchor = self.caret;
                }
            }
            GraphTextInput::Right { extend } => {
                self.caret = if !extend && !self.selection().is_empty() {
                    self.selection().end
                } else {
                    Self::next(&self.text, self.caret)
                };
                if !extend {
                    self.anchor = self.caret;
                }
            }
            GraphTextInput::Home { extend } => {
                let line = self.caret_line();
                self.caret = self
                    .lines
                    .get(line)
                    .map(|(range, _)| range.start)
                    .unwrap_or(0);
                if !extend {
                    self.anchor = self.caret;
                }
            }
            GraphTextInput::End { extend } => {
                let line = self.caret_line();
                self.caret = self
                    .lines
                    .get(line)
                    .map(|(range, _)| range.end)
                    .unwrap_or(self.text.len());
                if !extend {
                    self.anchor = self.caret;
                }
            }
            _ => {}
        }
    }

    pub fn scroll_by(&mut self, dy: f32) {
        self.scroll += dy;
    }

    fn wrap(&mut self, width: f32, painter: &mut dyn GraphPainter) {
        let text = self.text.clone();
        let mut lines = Vec::new();
        let mut base = 0usize;
        let mut y = 0.;
        for paragraph in text.split('\n') {
            let mut start = 0usize;
            let mut x = 0.;
            let mut offset = 0usize;
            for word in paragraph.split_inclusive(' ') {
                let at = offset;
                offset += word.len();
                let w = painter.text_width(word, FONT);
                if x > 0. && x + w > width {
                    lines.push((base + start..base + at, y));
                    y += LINE;
                    start = at;
                    x = 0.;
                }
                x += w;
            }
            lines.push((base + start..base + paragraph.len(), y));
            y += LINE;
            base += paragraph.len() + 1;
        }
        if lines.is_empty() {
            lines.push((0..0, 0.));
        }
        self.lines = lines;
    }

    fn caret_line(&self) -> usize {
        for (index, (range, _)) in self.lines.iter().enumerate() {
            if self.caret <= range.end {
                return index;
            }
        }
        self.lines.len().saturating_sub(1)
    }

    fn inner(&self) -> GraphRect {
        GraphRect {
            origin: [self.rect.origin[0] + PAD, self.rect.origin[1] + PAD],
            size: [
                (self.rect.size[0] - 2. * PAD).max(20.),
                (self.rect.size[1] - 2. * PAD - HINT).max(LINE),
            ],
        }
    }

    /// Place the caret at a view-space point. Lines come from the last paint.
    /// `extend` keeps the anchor, so dragging or shift-clicking selects.
    pub fn caret_at(&mut self, point: [f32; 2], font: &fontdue::Font, extend: bool) {
        if self.lines.is_empty() {
            return;
        }
        let anchor = self.anchor;
        let inner = self.inner();
        let local_y = point[1] - inner.origin[1] + self.scroll;
        let line = (local_y / LINE).floor().max(0.) as usize;
        let line = line.min(self.lines.len() - 1);
        let range = self.lines[line].0.clone();
        let target_x = point[0] - inner.origin[0];
        let mut x = 0.;
        let mut caret = range.start;
        for (index, ch) in self.text[range.clone()].char_indices() {
            let w = advance(font, &ch.to_string(), FONT);
            if x + w * 0.5 > target_x {
                break;
            }
            x += w;
            caret = range.start + index + ch.len_utf8();
        }
        self.caret = caret;
        self.anchor = if extend { anchor } else { caret };
    }

    /// Move the caret a line up or down, keeping its horizontal position.
    pub fn move_vertical(&mut self, delta: i32, font: &fontdue::Font) {
        if self.lines.is_empty() {
            return;
        }
        let line = self.caret_line() as i32;
        let range = self.lines[self.caret_line()].0.clone();
        let want_x = advance(font, &self.text[range.start..self.caret.min(range.end)], FONT);
        let target = (line + delta).clamp(0, self.lines.len() as i32 - 1) as usize;
        if target == self.caret_line() {
            return;
        }
        let range = self.lines[target].0.clone();
        let mut x = 0.;
        let mut caret = range.start;
        for (index, ch) in self.text[range.clone()].char_indices() {
            let w = advance(font, &ch.to_string(), FONT);
            if x + w * 0.5 > want_x {
                break;
            }
            x += w;
            caret = range.start + index + ch.len_utf8();
        }
        self.caret = caret;
        self.anchor = caret;
    }

    fn keep_caret_visible(&mut self, inner: GraphRect) {
        let line = self.caret_line();
        if let Some((_, y)) = self.lines.get(line) {
            let top = *y - self.scroll;
            if top < 0. {
                self.scroll += top;
            } else if top + LINE > inner.size[1] {
                self.scroll += top + LINE - inner.size[1];
            }
        }
        let content = self.lines.len() as f32 * LINE;
        self.scroll = self.scroll.clamp(0., (content - inner.size[1]).max(0.));
    }

    pub fn paint(&mut self, painter: &mut dyn GraphPainter, theme: &GraphTheme) {
        painter.round_rect(self.rect, 6., theme.background);
        painter.round_rect(
            GraphRect {
                origin: [self.rect.origin[0], self.rect.origin[1]],
                size: [self.rect.size[0], 2.],
            },
            0.,
            theme.active,
        );
        let inner = self.inner();
        self.wrap(inner.size[0], painter);
        self.keep_caret_visible(inner);

        let selection = self.selection();
        for (range, y) in self.lines.clone() {
            let top = inner.origin[1] + y - self.scroll;
            if top + LINE < inner.origin[1] || top > inner.origin[1] + inner.size[1] {
                continue;
            }
            if !selection.is_empty() {
                let start = selection.start.max(range.start);
                let end = selection.end.min(range.end);
                if start < end {
                    let x0 = painter.text_width(&self.text[range.start..start], FONT);
                    let x1 = painter.text_width(&self.text[range.start..end], FONT);
                    painter.round_rect(
                        GraphRect {
                            origin: [inner.origin[0] + x0, top],
                            size: [x1 - x0, LINE],
                        },
                        0.,
                        [42, 112, 134, 255],
                    );
                }
            }
            painter.text(
                GraphRect {
                    origin: [inner.origin[0], top],
                    size: [inner.size[0], LINE],
                },
                &self.text[range.clone()],
                FONT,
                theme.text,
            );
        }

        let line = self.caret_line();
        if let Some((range, y)) = self.lines.get(line) {
            let upto = self.caret.clamp(range.start, range.end);
            let x = painter.text_width(&self.text[range.start..upto], FONT);
            painter.round_rect(
                GraphRect {
                    origin: [inner.origin[0] + x, inner.origin[1] + *y - self.scroll],
                    size: [2., LINE],
                },
                0.,
                theme.text,
            );
        }
    }
}
