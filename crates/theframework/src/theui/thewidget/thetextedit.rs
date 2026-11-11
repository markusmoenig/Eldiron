#[cfg(feature = "text")]
#[path = "thetextedit_paint.rs"]
mod paint;
use std::ops::Range;

use fontdue::layout::LayoutSettings;
use num_traits::ToPrimitive;
use unicode_segmentation::UnicodeSegmentation;

use crate::prelude::*;
use crate::text_edit::grapheme_string;
pub use crate::text_edit::{
    Cursor as TheCursor, Selection as TheSelection, TextEditState as TheTextEditState,
};

struct TheGlyph {
    parent: char,
    x: f32,
    width: usize,
}

struct TheRowInfo {
    top: usize,
    left: usize,
    bottom: usize,
    right: usize,

    baseline: usize,
    glyph_start: usize,
    glyph_end: usize,

    highlights: Option<Vec<(TheColor, TheColor, usize)>>,
}

#[derive(Clone, Debug)]
struct TheTextDebugLine {
    row: usize,
    branch_taken: Option<bool>,
    values: Vec<String>,
}

pub struct TheTextRenderer {
    #[cfg(feature = "text")]
    command_layout: Option<crate::text::TextLayout>,
    #[cfg(feature = "text")]
    command_graphemes: Vec<usize>,
    #[cfg(feature = "text")]
    command_hit_inset: [f32; 2],
    #[cfg(feature = "text")]
    pub composition: Option<(String, Option<(usize, usize)>)>,
    // Dim
    left: usize,
    top: usize,
    width: usize,
    height: usize,

    // Options
    cursor_width: usize,
    pub font_size: f32,
    pub indicate_space: bool,
    pub max_width: Option<f32>,
    pub padding: (i32, i32, i32, i32), // left top right bottom
    row_extend: usize,

    // State
    pub actual_size: Vec2<usize>,
    glyphs: Vec<TheGlyph>,
    pub highlighter: Option<Box<dyn TheCodeHighlighterTrait>>,
    row_info: Vec<TheRowInfo>,
    pub scroll_offset: Vec2<usize>,

    // Matches
    matches: Vec<(usize, usize)>,
    highlighted_match: usize,

    // Errors
    errors: Vec<(usize, usize)>,

    // Debugging
    debug_line: Option<usize>,
    debug_lines: Vec<TheTextDebugLine>,
}

impl Default for TheTextRenderer {
    fn default() -> Self {
        Self {
            #[cfg(feature = "text")]
            command_layout: None,
            #[cfg(feature = "text")]
            command_graphemes: Vec::new(),
            #[cfg(feature = "text")]
            command_hit_inset: [0.; 2],
            #[cfg(feature = "text")]
            composition: None,
            left: 0,
            top: 0,
            width: 0,
            height: 0,

            cursor_width: 2,
            font_size: 14.0,
            indicate_space: false,
            max_width: None,
            padding: (5, 0, 5, 0),
            row_extend: 1,

            actual_size: Vec2::zero(),
            glyphs: vec![],
            highlighter: None,
            row_info: vec![],
            scroll_offset: Vec2::zero(),

            matches: vec![],
            highlighted_match: 0,

            errors: vec![],

            debug_line: None,
            debug_lines: vec![],
        }
    }
}

impl TheTextRenderer {
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    pub fn clear_matches(&mut self) {
        self.matches.clear();
    }

    pub fn dim(&self) -> TheDim {
        TheDim::new(
            self.left as i32 - self.padding.0,
            self.top as i32 - self.padding.1,
            self.width as i32 + self.padding.0 + self.padding.2,
            self.height as i32 + self.padding.1 + self.padding.3,
        )
    }

    pub fn find_cursor_index(&self, coord: &Vec2<i32>) -> usize {
        #[cfg(feature = "text")]
        if let Some(layout) = &self.command_layout {
            let index = layout.hit_test([
                coord.x as f32 + self.scroll_offset.x as f32 - self.command_hit_inset[0],
                coord.y as f32 + self.scroll_offset.y as f32 - self.command_hit_inset[1],
            ]);
            return self
                .command_graphemes
                .partition_point(|b| *b <= index)
                .saturating_sub(1);
        }

        let coord = Vec2::new(
            coord.x + self.scroll_offset.x as i32 - self.padding.0 as i32,
            coord.y + self.scroll_offset.y as i32 - self.padding.1 as i32,
        );

        if (coord.x < 0 && coord.y < 0) || self.glyphs.is_empty() {
            // Cursor is at the start of all the text
            return 0;
        }

        for (row_number, row) in self.row_info.iter().enumerate() {
            if coord.y <= row.bottom as i32 {
                let start_index = self.row_info[row_number].glyph_start;
                let end_index = self.row_info[row_number].glyph_end;
                let mut cursor_column = end_index - start_index;
                if self.glyphs[end_index].parent != '\n' {
                    cursor_column += 1;
                }

                for i in start_index..=end_index {
                    let glyph = &self.glyphs[i];
                    if (glyph.x + glyph.width.to_f32().unwrap()).to_i32().unwrap() > coord.x {
                        cursor_column = i - start_index;
                        break;
                    }
                }

                return self.find_glyph_index(row_number, cursor_column);
            }
        }

        // Cursor is at the end of all the text
        let cursor_row = self.row_count() - 1;
        let mut cursor_column =
            self.row_info[cursor_row].glyph_end - self.row_info[cursor_row].glyph_start;
        if self.glyphs.last().unwrap().parent != '\n' {
            cursor_column += 1;
        }
        self.find_glyph_index(cursor_row, cursor_column)
    }

    pub fn highlight_match(&mut self, highlight_index: usize) {
        self.highlighted_match = highlight_index;
    }

    pub fn is_horizontal_overflow(&self) -> bool {
        self.actual_size.x > self.width
    }

    pub fn is_vertical_overflow(&self) -> bool {
        self.actual_size.y > self.height
    }

    pub fn viewport_height(&self) -> usize {
        self.height
    }

    pub fn prepare(&mut self, text: &str, font_preference: TheFontPreference, draw: &TheDraw2D) {
        self.prepare_with_highlight_source(text, None, font_preference, draw);
    }

    /// Prepares visible text while allowing syntax highlighting to use a parallel source.
    ///
    /// This lets presentation-only output keep control markers in the highlight source without
    /// exposing them in rendered, selected, or copied text. Both strings should contain the same
    /// number of logical lines. When they differ, each source line's dominant syntax style is
    /// projected over the corresponding visible line.
    pub fn prepare_with_highlight_source(
        &mut self,
        text: &str,
        highlight_source: Option<&str>,
        font_preference: TheFontPreference,
        draw: &TheDraw2D,
    ) {
        self.actual_size = Vec2::zero();
        self.glyphs.clear();
        self.row_info.clear();

        let mut text = text.to_owned();
        // Indicate a new line, for render and interaction only
        if text.ends_with('\n') || text.is_empty() {
            text.push('\n');
        }

        let layout = draw.get_text_layout(
            &text,
            &TheFontSettings {
                size: self.font_size,
                preference: font_preference.clone(),
            },
            LayoutSettings {
                max_width: self.max_width,
                ..Default::default()
            },
        );
        let glyph_positions = layout.glyphs();

        self.glyphs = glyph_positions
            .iter()
            .map(|glyph| TheGlyph {
                parent: glyph.parent,
                x: glyph.x,
                width: glyph.width,
            })
            .collect();

        // Hack: to get the width of a normal space,
        // for that fontdue will render the tailing space with zero width
        let space_width = {
            let layout = draw.get_text_layout(
                "  ",
                &TheFontSettings {
                    size: self.font_size,
                    preference: font_preference,
                },
                LayoutSettings::default(),
            );
            layout.glyphs().last().unwrap().x
                - layout.glyphs().first().unwrap().x
                - layout.glyphs().first().unwrap().width.to_f32().unwrap()
        };
        // Manually set space width
        self.glyphs.iter_mut().for_each(|glyph| {
            if glyph.parent == ' ' {
                glyph.width = space_width.ceil() as usize;
            }
        });

        self.row_info = layout
            .lines()
            .unwrap()
            .iter()
            .map(|line| {
                let top = (line.baseline_y - line.max_ascent).ceil() as usize;
                let left = self.glyphs.get(line.glyph_start).unwrap().x.ceil() as usize;
                let bottom = (line.baseline_y - line.min_descent).ceil() as usize;
                let right = {
                    let last_glyph = self.glyphs.get_mut(line.glyph_end).unwrap();
                    (last_glyph.x + last_glyph.width.to_f32().unwrap()).ceil() as usize
                };

                self.actual_size.x = self.actual_size.x.max(right);
                self.actual_size.y = self.actual_size.y.max(bottom);

                TheRowInfo {
                    top,
                    left,
                    bottom,
                    right,
                    baseline: line.baseline_y.ceil() as usize,
                    glyph_start: line.glyph_start,
                    glyph_end: line.glyph_end,
                    highlights: None,
                }
            })
            .collect();

        if let Some(highlighter) = &self.highlighter {
            let mut h = syntect::easy::HighlightLines::new(
                highlighter.syntect_syntax(),
                highlighter.syntect_theme(),
            );

            let mut highlighted_lines = Vec::new();
            if let Some(source) = highlight_source {
                let mut source_lines = source.split('\n');
                for visible_segment in text.split_inclusive('\n') {
                    let visible_line = visible_segment
                        .strip_suffix('\n')
                        .unwrap_or(visible_segment);
                    let source_line = source_lines.next().unwrap_or(visible_line);
                    let source_highlights = highlighter.highlight_line(source_line, &mut h);
                    if let Some((foreground, background, _)) =
                        source_highlights.into_iter().max_by_key(|(_, _, len)| *len)
                    {
                        highlighted_lines.push((
                            foreground,
                            background,
                            visible_segment.graphemes(true).count(),
                        ));
                    } else if visible_segment.ends_with('\n') {
                        highlighted_lines.push((
                            TheColor::default(),
                            highlighter.background().unwrap_or_default(),
                            1,
                        ));
                    }
                }
            } else {
                for segment in text.split_inclusive('\n') {
                    let line = segment.strip_suffix('\n').unwrap_or(segment);
                    let mut line_highlights = highlighter.highlight_line(line, &mut h);
                    if segment.ends_with('\n') {
                        let (foreground, background) = line_highlights
                            .last()
                            .map(|(foreground, background, _)| {
                                (foreground.clone(), background.clone())
                            })
                            .unwrap_or_else(|| {
                                (
                                    TheColor::default(),
                                    highlighter.background().unwrap_or_default(),
                                )
                            });
                        line_highlights.push((foreground, background, 1));
                    }
                    highlighted_lines.extend(line_highlights);
                }
            }
            let mut highlighted_lines = highlighted_lines.into_iter();
            let mut leftover: Option<(TheColor, TheColor, usize)> = None;

            for row_info in &mut self.row_info {
                let mut remaining = row_info.glyph_end + 1 - row_info.glyph_start;
                let mut highlights = vec![];

                while remaining > 0 {
                    let Some((fg_color, bg_color, token_len)) =
                        leftover.take().or_else(|| highlighted_lines.next())
                    else {
                        break;
                    };
                    let visible_len = token_len.min(remaining);
                    if visible_len > 0 {
                        highlights.push((fg_color.clone(), bg_color.clone(), visible_len));
                        remaining -= visible_len;
                    }
                    if token_len > visible_len {
                        leftover = Some((fg_color, bg_color, token_len - visible_len));
                    }
                }

                if !highlights.is_empty() {
                    row_info.highlights = Some(highlights);
                }
            }
        }

        // Re-calculate scroll offset
        self.scroll(&Vec2::zero(), false);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_text(
        &self,
        state: &TheTextEditState,
        focused: bool,
        readonly: bool,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        font_preference: TheFontPreference,
        draw: &TheDraw2D,
    ) {
        self.render_text_with_styles(
            state,
            focused,
            readonly,
            buffer,
            style,
            font_preference,
            &[],
            draw,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_text_with_styles(
        &self,
        state: &TheTextEditState,
        focused: bool,
        readonly: bool,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        font_preference: TheFontPreference,
        styles: &[(Range<usize>, TheTextStyle)],
        draw: &TheDraw2D,
    ) {
        if let Some((start_row, end_row)) = self.visible_rows() {
            for i in start_row..=end_row {
                self.render_row(
                    state,
                    i,
                    buffer,
                    style,
                    font_preference.clone(),
                    styles,
                    draw,
                );
            }

            if focused && !readonly {
                self.render_cursor(
                    &state.cursor,
                    state.find_cursor_index(),
                    buffer,
                    style,
                    draw,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_widget(
        &self,
        shrinker: &mut TheDimShrinker,
        disabled: bool,
        embedded: bool,
        background: bool,
        border: bool,
        widget: &dyn TheWidget,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
        is_text_area: bool,
    ) {
        if border {
            if is_text_area {
                style.draw_text_area_border(buffer, widget, shrinker, ctx, embedded, disabled);
            } else {
                style.draw_text_edit_border(buffer, widget, shrinker, ctx, embedded, disabled);
            }
        }

        if background {
            let rect = ThePixelRect::new(
                widget.dim().buffer_x.saturating_add(shrinker.left),
                widget.dim().buffer_y.saturating_add(shrinker.top),
                widget
                    .dim()
                    .width
                    .saturating_sub(shrinker.left)
                    .saturating_sub(shrinker.right),
                widget
                    .dim()
                    .height
                    .saturating_sub(shrinker.top)
                    .saturating_sub(shrinker.bottom),
            );
            let paint = if let Some(background) = self
                .highlighter
                .as_ref()
                .and_then(|highlighter| highlighter.background())
            {
                ThePaint::solid(background.to_u8_array())
            } else {
                let role = if disabled {
                    TextInputDisabled
                } else if widget.id().equals(&ctx.ui.focus) {
                    TextInputFocused
                } else {
                    TextInputNormal
                };
                style.theme().paint(role, rect)
            };
            let radius = if is_text_area {
                0.0
            } else {
                (style.theme().metric(ControlCornerRadius) - shrinker.left.max(0) as f32).max(0.0)
            };
            let width = buffer.dim().width.max(0) as usize;
            let height = buffer.dim().height.max(0) as usize;
            if let Ok(mut surface) = TheSurfaceMut::new(buffer.draw_target(), width, height) {
                surface.set_clip(rect);
                ctx.painter
                    .fill_round_rect(&mut surface, rect, radius, &paint);
            }
        }

        shrinker.shrink_by(
            self.padding.0 as i32,
            self.padding.1 as i32,
            self.padding.2 as i32,
            self.padding.3 as i32,
        );
    }

    pub fn row_baseline(&self, row_number: usize) -> usize {
        self.row_info[row_number].baseline
    }

    pub fn row_count(&self) -> usize {
        self.row_info.len()
    }

    #[cfg(test)]
    pub(crate) fn highlighted_rows_cover_text(&self) -> bool {
        self.row_info.iter().all(|row| {
            row.highlights
                .as_ref()
                .map(|highlights| highlights.iter().map(|(_, _, len)| len).sum::<usize>())
                .unwrap_or(0)
                == row.glyph_end + 1 - row.glyph_start
        })
    }

    pub fn scroll(&mut self, delta: &Vec2<i32>, visible_constrained: bool) -> bool {
        if self.row_info.is_empty() {
            self.scroll_offset = Vec2::zero();
            return true;
        }

        let previous_offset = self.scroll_offset;

        #[allow(clippy::obfuscated_if_else)]
        let max_width = visible_constrained
            .then(|| {
                self.visible_rows()
                    .and_then(|(start_row, end_row)| {
                        self.row_info[start_row..=end_row]
                            .iter()
                            .max_by_key(|row| row.right)
                    })
                    .map(|row| row.right)
                    .unwrap_or(self.actual_size.x)
            })
            .unwrap_or(self.actual_size.x);
        let rightmost = max_width.saturating_sub(self.width);
        self.scroll_offset.x = (self.scroll_offset.x as i32 + delta.x)
            .max(0)
            .to_usize()
            .unwrap()
            .min(rightmost);

        let downmost = self.actual_size.y.saturating_sub(self.height);
        self.scroll_offset.y = (self.scroll_offset.y as i32 + delta.y)
            .max(0)
            .to_usize()
            .unwrap()
            .min(downmost);

        previous_offset != self.scroll_offset
    }

    pub fn scroll_to_cursor(&mut self, cursor_index: usize, cursor_row: usize) {
        #[cfg(feature = "text")]
        if let Some(layout) = &self.command_layout {
            let c = layout.caret(self.layout_byte(cursor_index));
            let x = c.x.max(0.) as usize;
            let y = c.y.max(0.) as usize;
            self.scroll_offset.x = self
                .scroll_offset
                .x
                .min(x)
                .max((x + self.cursor_width).saturating_sub(self.width));
            self.scroll_offset.y = self
                .scroll_offset
                .y
                .min(y)
                .max((y + c.height.ceil() as usize).saturating_sub(self.height));
            return;
        }

        let text_width_before_cursor = self.get_text_left(cursor_index);

        // Check if the widget should be scrolled in order to display the cursor
        // Scroll right
        let leftmost = text_width_before_cursor;
        self.scroll_offset.x = self.scroll_offset.x.min(leftmost);
        // Scroll left
        let rightmost = (text_width_before_cursor + self.cursor_width).saturating_sub(self.width);
        self.scroll_offset.x = self.scroll_offset.x.max(rightmost);
        // Scroll down
        let upmost = self.row_info[cursor_row].top;
        self.scroll_offset.y = self.scroll_offset.y.min(upmost);
        // Scroll up
        let downmost = self.row_info[cursor_row].bottom.saturating_sub(self.height);
        self.scroll_offset.y = self.scroll_offset.y.max(downmost);
    }

    pub fn set_dim(&mut self, left: usize, top: usize, width: usize, height: usize) {
        self.left = left;
        self.top = top;
        self.width = width;
        self.height = height;
    }

    pub fn set_code_type(&mut self, code_type: &str) {
        if let Some(highlighter) = self.highlighter.as_mut() {
            highlighter.set_syntax_by_name(code_type);
        } else {
            let mut highlighter = TheCodeHighlighter::default();
            highlighter.set_syntax_by_name(code_type);
            self.highlighter = Some(Box::new(highlighter));
        }
    }

    pub fn add_syntax_from_string(&mut self, syntax: &str) {
        if let Some(highlighter) = self.highlighter.as_mut() {
            _ = highlighter.add_syntax_from_string(syntax);
        } else {
            let mut highlighter = TheCodeHighlighter::default();
            _ = highlighter.add_syntax_from_string(syntax);
            self.highlighter = Some(Box::new(highlighter));
        }
    }

    pub fn add_theme_from_string(&mut self, theme: &str) {
        if let Some(highlighter) = self.highlighter.as_mut() {
            _ = highlighter.add_theme_from_string(theme);
        } else {
            let mut highlighter = TheCodeHighlighter::default();
            _ = highlighter.add_theme_from_string(theme);
            self.highlighter = Some(Box::new(highlighter));
        }
    }

    pub fn set_code_theme(&mut self, code_theme: &str) {
        if let Some(highlighter) = self.highlighter.as_mut() {
            highlighter.set_theme(code_theme);
        } else {
            let mut highlighter = TheCodeHighlighter::default();
            highlighter.set_theme(code_theme);
            self.highlighter = Some(Box::new(highlighter));
        }
    }

    pub fn set_errors(&mut self, errors: Vec<(usize, usize)>) {
        self.errors = errors;
    }

    pub fn set_debug_line(&mut self, debug_line: Option<usize>) {
        self.debug_line = debug_line;
    }

    pub fn set_debug_lines(&mut self, lines: &[(usize, Option<bool>, Vec<String>)]) {
        self.debug_lines = lines
            .iter()
            .map(|(row, branch_taken, values)| TheTextDebugLine {
                row: *row,
                branch_taken: *branch_taken,
                values: values.clone(),
            })
            .collect();
    }

    pub fn has_debug_line(&self, row_number: usize) -> bool {
        self.debug_line == Some(row_number)
            || self
                .debug_lines
                .iter()
                .any(|debug_line| debug_line.row == row_number)
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
    }

    pub fn set_matches(&mut self, matches: Vec<(usize, usize)>) {
        self.matches = matches;
    }

    pub fn scroll_to_row_centered(&mut self, row_number: usize) {
        if row_number >= self.row_info.len() {
            return;
        }

        let row = &self.row_info[row_number];
        let row_mid = (row.top + row.bottom) / 2;
        let mut target = row_mid.saturating_sub(self.height / 2);

        if self.actual_size.y > self.height {
            let downmost = self.actual_size.y - self.height;
            target = target.min(downmost);
        } else {
            target = 0;
        }

        self.scroll_offset.y = target;
    }

    pub fn scroll_to_row_with_margin(&mut self, row_number: usize, margin_rows: usize) {
        if row_number >= self.row_info.len() {
            return;
        }

        let row = &self.row_info[row_number];
        let row_height = self.row_height(row_number);
        let margin = margin_rows * row_height;

        let upmost = row.top.saturating_sub(margin);
        let downmost = self.actual_size.y.saturating_sub(self.height);

        self.scroll_offset.y = upmost.min(downmost);
    }

    // Inclusive on both end
    pub fn visible_rows(&self) -> Option<(usize, usize)> {
        if self.row_count() == 0 {
            return None;
        }

        let start_row = self
            .row_info
            .iter()
            .enumerate()
            .find(|(_, row)| row.bottom > self.scroll_offset.y)
            .map(|(idx, _)| idx)
            .unwrap_or_default();
        let end_row = if start_row < self.row_count() - 1 {
            self.row_info[start_row + 1..]
                .iter()
                .enumerate()
                .find(|(_, row)| row.top > self.height + self.scroll_offset.y)
                .map(|(idx, _)| idx + start_row)
                .unwrap_or(self.row_count() - 1)
        } else {
            start_row
        };

        Some((start_row, end_row))
    }

    fn find_glyph_index(&self, row: usize, column: usize) -> usize {
        self.row_info[row].glyph_start + column
    }

    fn get_text_left(&self, index: usize) -> usize {
        #[cfg(feature = "text")]
        if let Some(layout) = &self.command_layout {
            return layout.caret(self.layout_byte(index)).x.max(0.).ceil() as usize;
        }

        if self.glyphs.is_empty() {
            return 0;
        }

        if let Some(glyph) = self.glyphs.get(index) {
            return glyph.x.ceil().to_usize().unwrap();
        }

        let last_glyph = &self.glyphs[self.glyphs.len() - 1];
        last_glyph.x.ceil().to_usize().unwrap() + last_glyph.width
    }

    // Support single row only
    // Inclusive on both end
    // Make sure start and end are on the same row
    fn get_text_width(&self, start: usize, end: usize) -> usize {
        if self.glyphs.is_empty() {
            return 0;
        }

        if start == end {
            return self.glyphs[start].width;
        }

        let left = start.min(end);
        let right = start.max(end);
        let last_glyph = &self.glyphs[right];
        let last_glyph_end = last_glyph.x + last_glyph.width.to_f32().unwrap();

        (last_glyph_end - self.glyphs[left].x)
            .ceil()
            .to_usize()
            .unwrap()
    }

    fn fit_debug_label(
        &self,
        label: &str,
        max_width: usize,
        draw: &TheDraw2D,
        font_preference: &TheFontPreference,
        font_size: f32,
    ) -> (String, usize) {
        let text_max_width = max_width.saturating_sub(14).max(12);
        let font_settings = TheFontSettings {
            size: font_size,
            preference: font_preference.clone(),
        };

        let text_size = draw.get_text_size(label, &font_settings);
        if text_size.0 <= text_max_width {
            return (label.to_string(), text_size.0 + 14);
        }

        let ellipsis = "...";
        let mut chars: Vec<char> = label.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let trimmed = chars.iter().collect::<String>();
            let candidate = format!("{}{}", trimmed.trim_end(), ellipsis);
            let text_size = draw.get_text_size(&candidate, &font_settings);
            if text_size.0 <= text_max_width {
                return (candidate, text_size.0 + 14);
            }
        }

        let text_size = draw.get_text_size(ellipsis, &font_settings);
        (ellipsis.to_string(), text_size.0 + 14)
    }

    fn is_rect_out_of_visible_area(
        &self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) -> bool {
        top >= self.scroll_offset.y.saturating_add(self.height)
            || top.saturating_add(height) <= self.scroll_offset.y
            || left >= self.scroll_offset.x.saturating_add(self.width)
            || left.saturating_add(width) <= self.scroll_offset.x
    }

    fn render_cursor(
        &self,
        cursor: &TheCursor,
        cursor_index: usize,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        draw: &TheDraw2D,
    ) {
        let row_height = self.row_height(cursor.row);

        let left = self.get_text_left(cursor_index).to_i32().unwrap()
            - (self.cursor_width / 2).to_i32().unwrap();
        let top = self.row_info[cursor.row].bottom.to_i32().unwrap() - row_height.to_i32().unwrap();
        if self.is_rect_out_of_visible_area(
            left.max(0).to_usize().unwrap(),
            top.max(0).to_usize().unwrap(),
            self.cursor_width,
            row_height,
        ) {
            return;
        }

        let left = (self.left.to_i32().unwrap() + left - self.scroll_offset.x.to_i32().unwrap())
            .max(0)
            .to_usize()
            .unwrap()
            .max(self.left);
        let top = self.top.to_i32().unwrap() + top - self.scroll_offset.y.to_i32().unwrap();

        let bottom = (top + row_height.to_i32().unwrap())
            .max(0)
            .to_usize()
            .unwrap()
            .min(self.top + self.height);

        let top = top.max(0).to_usize().unwrap().max(self.top);

        let stride = buffer.stride();
        let color = &self
            .highlighter
            .as_ref()
            .and_then(|hl| hl.caret())
            .map(|color| color.to_u8_array())
            .unwrap_or(*style.theme().color(TextEditCursorColor));
        draw.rect(
            buffer.draw_target(),
            &(left, top, self.cursor_width, bottom - top),
            stride,
            color,
        );
    }

    fn render_errors(
        &self,
        row_number: usize,
        visible_range: (usize, usize),
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        draw: &TheDraw2D,
    ) {
        for (start, end) in &self.errors {
            if *end < visible_range.0 {
                continue;
            }
            if *start > visible_range.1 {
                break;
            }

            let row = &self.row_info[row_number];

            let start = (*start).max(visible_range.0);
            let end = (*end).min(visible_range.1);

            if start == end {
                return;
            }

            let width = self.get_text_width(start, end - 1);

            let left = (self.left + self.get_text_left(start)) as i32 - self.scroll_offset.x as i32;
            let base = (self.top + row.bottom) as i32 - self.scroll_offset.y as i32;

            let right = (left + width.to_i32().unwrap())
                .max(0)
                .to_usize()
                .unwrap()
                .min(self.left + self.width);
            let length = right - left.max(0).to_usize().unwrap().max(self.left);

            let color = self
                .highlighter
                .as_ref()
                .and_then(|hl| hl.misspelling())
                .map(|color| color.to_u8_array())
                .unwrap_or(*style.theme().color(Red));

            let stride = buffer.stride();
            draw.wavy_line(
                buffer.draw_target(),
                left,
                base,
                length,
                2.0,
                1.0,
                stride,
                &color,
            );
        }
    }

    fn render_matches(
        &self,
        row_number: usize,
        visible_range: (usize, usize),
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        draw: &TheDraw2D,
    ) {
        for (index, (start, end)) in self.matches.iter().enumerate() {
            if *end < visible_range.0 {
                continue;
            }
            if *start > visible_range.1 {
                break;
            }

            let start = (*start).max(visible_range.0);
            let end = (*end).min(visible_range.1);

            let color = if index == self.highlighted_match {
                self.highlighter
                    .as_ref()
                    .and_then(|hl| hl.active_match_background())
                    .map(|color| color.to_u8_array())
                    // This is temporary
                    .unwrap_or(*style.theme().color(NodeBorderSelected))
            } else {
                self.highlighter
                    .as_ref()
                    .and_then(|hl| hl.match_background())
                    .map(|color| color.to_u8_array())
                    .unwrap_or(*style.theme().color(DefaultSelection))
            };

            self.render_text_background(row_number, start, end, buffer, &color, draw);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_row(
        &self,
        state: &TheTextEditState,
        row_number: usize,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        font_preference: TheFontPreference,
        styles: &[(Range<usize>, TheTextStyle)],
        draw: &TheDraw2D,
    ) {
        let row = &self.row_info[row_number];
        if self.is_rect_out_of_visible_area(
            row.left,
            row.top,
            row.right - row.left,
            row.bottom - row.top,
        ) {
            return;
        }

        let logical_row_number = state.find_row_number_of_index(row.glyph_start);
        let debug_entry = self
            .debug_lines
            .iter()
            .rev()
            .find(|debug_line| debug_line.row == logical_row_number);
        let is_latest_debug_row = self
            .debug_lines
            .last()
            .is_some_and(|debug_line| debug_line.row == logical_row_number);

        if self.debug_line == Some(logical_row_number) || debug_entry.is_some() {
            let (mut color, rail_color) = if debug_entry
                .is_some_and(|debug_line| debug_line.branch_taken == Some(true))
            {
                ([255, 243, 163, 48], [92, 220, 148, 230])
            } else if debug_entry.is_some_and(|debug_line| debug_line.branch_taken == Some(false)) {
                ([255, 243, 163, 38], [92, 176, 236, 210])
            } else if is_latest_debug_row {
                ([255, 243, 163, 84], [255, 212, 92, 210])
            } else if self.debug_line == Some(logical_row_number) {
                (
                    *style.theme().color(TextEditDebugLineBackground),
                    [255, 212, 92, 180],
                )
            } else {
                ([255, 243, 163, 42], [255, 212, 92, 160])
            };
            if color[3] > 110 {
                color[3] = 110;
            }
            let row_height = self.row_height(row_number);
            let top =
                (self.top + row.bottom) as i32 - row_height as i32 - self.scroll_offset.y as i32;
            let bottom = (top + row_height as i32)
                .max(0)
                .to_usize()
                .unwrap()
                .min(self.top + self.height);
            let top = top.max(0).to_usize().unwrap().max(self.top);
            if bottom > top {
                let stride = buffer.stride();
                draw.blend_rect(
                    buffer.draw_target(),
                    &(self.left, top, self.width, bottom - top),
                    stride,
                    &color,
                );
                draw.blend_rect(
                    buffer.draw_target(),
                    &(self.left, top, 3.min(self.width), bottom - top),
                    stride,
                    &rail_color,
                );
            }
        }

        // Find the visible text
        let glyph_start = row.glyph_start;
        let glyph_end = row.glyph_end + 1; // Convert glyph index to cursor index
        let glyphs_count = glyph_end - glyph_start;
        let mut visible_text_start_index = 0;
        let mut visible_text_end_index = glyphs_count;
        let mut is_start_index_found = false;
        let mut chars_acc_width = 0;
        for i in 0..glyphs_count {
            if is_start_index_found && chars_acc_width >= self.scroll_offset.x + self.width {
                visible_text_end_index = i;
                break;
            }
            chars_acc_width = self.get_text_width(glyph_start, glyph_start + i);
            if !is_start_index_found && chars_acc_width >= self.scroll_offset.x {
                visible_text_start_index = i;
                is_start_index_found = true;
            }
        }

        // Render text and clip
        // Make sure row x start at 0 TODO
        let left = self.left.to_i32().unwrap()
            - self.scroll_offset.x.to_i32().unwrap()
            - self.get_text_left(glyph_start).to_i32().unwrap();
        let top = self.top.to_i32().unwrap() - self.scroll_offset.y.to_i32().unwrap()
            + row.top.to_i32().unwrap();

        // Selections
        let selected_range = state.find_selected_range_within_range(glyph_start, glyph_end);
        if let Some((start, end)) = selected_range {
            self.render_selection(row_number, start, end, buffer, style, draw);
        }

        // Tokens
        let text = &state.get_text(glyph_start, glyph_end);
        let stride = buffer.stride();
        if let Some(highlights) = &row.highlights {
            // Matches
            self.render_matches(
                row_number,
                (
                    glyph_start + visible_text_start_index,
                    glyph_start + visible_text_end_index,
                ),
                buffer,
                style,
                draw,
            );

            // Errors
            self.render_errors(
                row_number,
                (
                    glyph_start + visible_text_start_index,
                    glyph_start + visible_text_end_index,
                ),
                buffer,
                style,
                draw,
            );

            let widget_bg = self
                .highlighter
                .as_ref()
                .and_then(|h| h.background())
                .map(|c| c.to_u8_array())
                .unwrap_or(*style.theme().color(TextEditBackground));

            let mut token_end_in_row = 0;
            for (fg_color, bg_color, token_len) in highlights {
                let token_start_in_row = token_end_in_row;
                if token_start_in_row > visible_text_end_index {
                    break;
                }
                token_end_in_row = (token_start_in_row + token_len).min(text.len());
                if token_end_in_row < visible_text_start_index {
                    continue;
                }

                let token_bg_start = glyph_start + token_start_in_row;
                let token_bg_end = glyph_start + token_end_in_row;
                let selected_range_in_token = selected_range.and_then(|(start, end)| {
                    (token_bg_start < end && token_bg_end > start)
                        .then_some((start.max(token_bg_start), end.min(token_bg_end)))
                });
                let bg_color = bg_color.to_u8_array();
                if widget_bg != bg_color {
                    // Render original text background,
                    // and blend selection background if needed
                    self.render_text_background(
                        row_number,
                        token_bg_start,
                        token_bg_end,
                        buffer,
                        &bg_color,
                        draw,
                    );
                    if let Some((start, end)) = selected_range_in_token {
                        let mut color = self
                            .highlighter
                            .as_ref()
                            .and_then(|hl| hl.selection_background())
                            .map(|color| color.to_u8_array())
                            .unwrap_or(*style.theme().color(DefaultSelection));
                        color[3] = 180;
                        self.render_text_background(row_number, start, end, buffer, &color, draw);
                    }
                }

                if self.indicate_space {
                    let mut chars_to_rendered: Vec<char> = vec![];
                    for (char_index, char) in
                        grapheme_string(text, token_start_in_row, token_end_in_row).char_indices()
                    {
                        if let Some(ch) = chars_to_rendered.first() {
                            if ch.is_whitespace() == char.is_whitespace() {
                                chars_to_rendered.push(char);
                            } else {
                                let left = left
                                    + self
                                        .get_text_left(
                                            glyph_start + token_start_in_row + char_index
                                                - chars_to_rendered.len(),
                                        )
                                        .to_i32()
                                        .unwrap();
                                let (text_to_rendered, fg_color) = if ch.is_whitespace() {
                                    (
                                        "·".repeat(chars_to_rendered.len()),
                                        self.highlighter
                                            .as_ref()
                                            .and_then(|hl| hl.guide())
                                            .map(|color| color.to_u8_array())
                                            .unwrap_or_else(|| {
                                                let mut color =
                                                    *style.theme().color(TextEditTextColor);
                                                color[3] = 50;
                                                color
                                            }),
                                    )
                                } else {
                                    (
                                        String::from_iter(&chars_to_rendered),
                                        fg_color.to_u8_array(),
                                    )
                                };

                                draw.text_rect_blend_clip(
                                    buffer.draw_target(),
                                    &Vec2::new(left, top - 1),
                                    &(self.left, self.top, self.width, self.height),
                                    stride,
                                    &text_to_rendered,
                                    TheFontSettings {
                                        size: self.font_size,
                                        preference: font_preference.clone(),
                                    },
                                    &fg_color,
                                    TheHorizontalAlign::Center,
                                    TheVerticalAlign::Center,
                                );

                                chars_to_rendered.clear();
                                chars_to_rendered.push(char);
                            }
                        } else {
                            chars_to_rendered.push(char);
                        }
                    }

                    if !chars_to_rendered.is_empty() {
                        let left = left
                            + self
                                .get_text_left(
                                    glyph_start + token_end_in_row - chars_to_rendered.len(),
                                )
                                .to_i32()
                                .unwrap();
                        let (text_to_rendered, fg_color) = if chars_to_rendered[0].is_whitespace() {
                            (
                                "·".repeat(chars_to_rendered.len()),
                                self.highlighter
                                    .as_ref()
                                    .and_then(|hl| hl.guide())
                                    .map(|color| color.to_u8_array())
                                    .unwrap_or_else(|| {
                                        let mut color = *style.theme().color(TextEditTextColor);
                                        color[3] = 50;
                                        color
                                    }),
                            )
                        } else {
                            (String::from_iter(chars_to_rendered), fg_color.to_u8_array())
                        };

                        draw.text_rect_blend_clip(
                            buffer.draw_target(),
                            &Vec2::new(left, top - 1),
                            &(self.left, self.top, self.width, self.height),
                            stride,
                            &text_to_rendered,
                            TheFontSettings {
                                size: self.font_size,
                                preference: font_preference.clone(),
                            },
                            &fg_color,
                            TheHorizontalAlign::Center,
                            TheVerticalAlign::Center,
                        );
                    }
                } else {
                    let left = left + self.get_text_left(token_bg_start).to_i32().unwrap();

                    draw.text_rect_blend_clip(
                        buffer.draw_target(),
                        &Vec2::new(left, top - 1),
                        &(self.left, self.top, self.width, self.height),
                        stride,
                        grapheme_string(text, token_start_in_row, token_end_in_row),
                        TheFontSettings {
                            size: self.font_size,
                            preference: font_preference.clone(),
                        },
                        &fg_color.to_u8_array(),
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }

            // Render linebreak selection if needed
            if let Some((_, end)) = selected_range {
                if glyph_start + token_end_in_row < end {
                    self.render_selection(row_number, end - 1, end, buffer, style, draw);
                }
            }
        } else {
            let mut rendered_text_ranges = vec![];
            for (range, text_style) in styles {
                if range.start >= glyph_start + visible_text_end_index
                    || range.end <= glyph_start + visible_text_start_index.saturating_sub(1)
                {
                    continue;
                }

                let token_start = (glyph_start + visible_text_start_index).max(range.start);
                let token_end = (glyph_start + visible_text_end_index).min(range.end);

                if let Some(color) = &text_style.background {
                    self.render_text_background(
                        row_number,
                        token_start,
                        token_end,
                        buffer,
                        &color.to_u8_array(),
                        draw,
                    );

                    let selected_range_in_token = selected_range.and_then(|(start, end)| {
                        (token_start < end && token_end > start)
                            .then_some((start.max(token_start), end.min(token_end)))
                    });
                    if let Some((start, end)) = selected_range_in_token {
                        let mut color = self
                            .highlighter
                            .as_ref()
                            .and_then(|hl| hl.selection_background())
                            .map(|color| color.to_u8_array())
                            .unwrap_or(*style.theme().color(DefaultSelection));
                        color[3] = 180;
                        self.render_text_background(row_number, start, end, buffer, &color, draw);
                    }
                }
                if let Some(color) = &text_style.foreground {
                    let left = left + self.get_text_left(token_start).to_i32().unwrap() + 0;
                    draw.text_rect_blend_clip(
                        buffer.draw_target(),
                        &Vec2::new(left, top - 1),
                        &(self.left, self.top, self.width, self.height),
                        stride,
                        grapheme_string(
                            text,
                            token_start - glyph_start,
                            token_end - glyph_start + 1,
                        ),
                        TheFontSettings {
                            size: self.font_size,
                            preference: font_preference.clone(),
                        },
                        &color.to_u8_array(),
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                    rendered_text_ranges.push(token_start..token_end);
                }
                if let Some(color) = &text_style.underline {
                    let left = left + self.get_text_left(token_start).to_i32().unwrap();
                    let left = left.max(0).to_usize().unwrap().max(self.left);

                    let top = top + self.row_height(row_number) as i32 - 1;
                    let top = top.max(0).to_usize().unwrap().max(self.top);

                    let width = self.get_text_width(token_start, token_end - 1);
                    let right = (left + width).min(self.left + self.width);
                    if top < self.top + self.height && right > left {
                        draw.blend_rect(
                            buffer.draw_target(),
                            &(left, top, right - left, 1),
                            stride,
                            &color.to_u8_array(),
                        );
                    }
                }
            }

            let mut text_ranges_to_render = vec![];
            let mut start = glyph_start + visible_text_start_index;
            for range in rendered_text_ranges {
                if start < range.start {
                    text_ranges_to_render.push((start, range.start - 1));
                }
                start = range.end + 1;
            }
            if start < glyph_start + visible_text_end_index {
                text_ranges_to_render.push((start, glyph_start + visible_text_end_index));
            }

            for (start, end) in text_ranges_to_render {
                let left = left + self.get_text_left(start).to_i32().unwrap();
                draw.text_rect_blend_clip(
                    buffer.draw_target(),
                    &Vec2::new(left, top - 1),
                    &(self.left, self.top, self.width, self.height),
                    stride,
                    grapheme_string(text, start - glyph_start, end - glyph_start),
                    TheFontSettings {
                        size: self.font_size,
                        preference: font_preference.clone(),
                    },
                    style.theme().color(TextEditTextColor),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
        }

        if let Some(debug_entry) = debug_entry
            && !debug_entry.values.is_empty()
        {
            let row_height = self.row_height(row_number);
            let label = debug_entry
                .values
                .iter()
                .rev()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("   ");
            let font_size = (self.font_size - 1.5).max(10.0);
            let max_width = (self.width / 2).clamp(120, 420);
            let (label, badge_width) =
                self.fit_debug_label(&label, max_width, draw, &font_preference, font_size);
            let badge_width = badge_width.min(max_width).max(34);
            let badge_height = (font_size.ceil() as usize + 8).min(row_height.saturating_sub(2));
            if badge_width + 16 < self.width && badge_height > 8 {
                let code_right = self.left as i32
                    - self.scroll_offset.x as i32
                    - self.get_text_left(row.glyph_start) as i32
                    + row.right as i32;
                let min_x = self.left + 8;
                let max_x = self.left + self.width - badge_width - 8;
                let preferred_x = (code_right + 14)
                    .max(min_x as i32)
                    .to_usize()
                    .unwrap_or(min_x);
                let badge_x = preferred_x.min(max_x).max(min_x);
                let row_top = self.top as i32 + row.top as i32 - self.scroll_offset.y as i32;
                let badge_y = row_top
                    + ((row_height.saturating_sub(badge_height)) / 2)
                        .to_i32()
                        .unwrap_or_default();
                let badge_y = badge_y.max(self.top as i32).to_usize().unwrap();
                let badge_bottom = (badge_y + badge_height).min(self.top + self.height);
                if badge_bottom > badge_y {
                    draw.rounded_rect(
                        buffer.draw_target(),
                        &(badge_x, badge_y, badge_width, badge_bottom - badge_y),
                        stride,
                        &[34, 38, 43, 224],
                        &(5.0, 5.0, 5.0, 5.0),
                    );
                    draw.text_rect_blend_clip(
                        buffer.draw_target(),
                        &Vec2::new(badge_x as i32 + 7, badge_y as i32),
                        &(self.left, self.top, self.width, self.height),
                        stride,
                        &label,
                        TheFontSettings {
                            size: font_size,
                            preference: font_preference,
                        },
                        &[255, 243, 163, 255],
                        TheHorizontalAlign::Left,
                        TheVerticalAlign::Center,
                    );
                }
            }
        }
    }

    fn render_selection(
        &self,
        row_number: usize,
        start: usize,
        end: usize,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        draw: &TheDraw2D,
    ) {
        let color = &self
            .highlighter
            .as_ref()
            .and_then(|hl| hl.selection_background())
            .map(|color| color.to_u8_array())
            .unwrap_or(*style.theme().color(DefaultSelection));
        self.render_text_background(row_number, start, end, buffer, color, draw);
    }

    fn render_text_background(
        &self,
        row_number: usize,
        start: usize,
        end: usize,
        buffer: &mut TheRGBABuffer,
        color: &[u8; 4],
        draw: &TheDraw2D,
    ) {
        if start == end {
            return;
        }

        let row = &self.row_info[row_number];
        let row_height = self.row_height(row_number);
        let row_width = row.right - row.left;
        if self.is_rect_out_of_visible_area(row.left, row.top, row_width, row_height) {
            return;
        }

        let mut width = if start == end - 1 && row.glyph_end == start {
            // Linebreak
            (self.font_size * 0.5).ceil().to_usize().unwrap()
        } else {
            self.get_text_width(start, end - 1)
        };

        let mut left = self.get_text_left(start);
        // If leftmost is the first glyph of current row,
        // we expand the left of the text background to 0
        if row.glyph_start == start {
            width += left;
            left = 0;
        }
        // If rightmost is not the last glyph of current row,
        // we expand the right of the text background to
        // left of the next glyph to avoid possible gap
        if row.glyph_end > end - 1 {
            width = self.get_text_left(end) - left;
        }

        let left = (self.left + left) as i32 - self.scroll_offset.x as i32;
        let top = (self.top + row.bottom) as i32 - row_height as i32 - self.scroll_offset.y as i32;

        let right = (left + width.to_i32().unwrap())
            .max(0)
            .to_usize()
            .unwrap()
            .min(self.left + self.width);
        let bottom = (top + row_height.to_i32().unwrap())
            .max(0)
            .to_usize()
            .unwrap()
            .min(self.top + self.height);

        let left = left.max(0).to_usize().unwrap().max(self.left);
        let top = top.max(0).to_usize().unwrap().max(self.top);

        if right > left && bottom > top {
            let stride = buffer.stride();
            draw.blend_rect(
                buffer.draw_target(),
                &(left, top, right - left, bottom - top),
                stride,
                color,
            );
        }
    }

    fn row_height(&self, row_number: usize) -> usize {
        let row = &self.row_info[row_number];
        row.bottom - row.top + self.row_extend
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct TheTextStyle {
    pub foreground: Option<TheColor>,
    pub background: Option<TheColor>,
    pub underline: Option<TheColor>,
}

impl TheTextStyle {
    pub fn is_empty(&self) -> bool {
        self.foreground.is_none() && self.background.is_none() && self.underline.is_none()
    }
}
