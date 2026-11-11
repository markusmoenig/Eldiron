//! Renderer-independent text editing in grapheme/cursor coordinates. The UI
//! adapters re-export these exact types, preserving saved undo-state JSON.
#[cfg(feature = "text")]
mod policy;
#[cfg(feature = "text")]
pub use policy::{EditorPolicy, EditorTab};
#[cfg(feature = "text")]
mod controller;
#[cfg(feature = "text")]
pub use controller::*;
#[cfg(feature = "text")]
mod view;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
#[cfg(feature = "text")]
pub use view::*;

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct Cursor {
    pub row: usize,
    pub column: usize,
}

impl Cursor {
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    pub fn is_zero(&self) -> bool {
        self.row == 0 && self.column == 0
    }

    pub fn reset(&mut self) {
        self.row = 0;
        self.column = 0;
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

impl Selection {
    pub fn intersects(&self, start: usize, end: usize) -> bool {
        start < self.end && end > self.start
    }

    pub fn is_none(&self) -> bool {
        self.start == self.end
    }

    pub fn reset(&mut self) {
        self.start = 0;
        self.end = 0;
    }
}

// cursor index  0   1   2   3   4
//  glyph index    0   1   2   3
//       cursor  |   |   |   |   |
//         text    a   b   c   \n
#[derive(Serialize, Deserialize, Clone)]
pub struct TextEditState {
    // Use cursor index
    pub cursor: Cursor,
    // Linebreak is not stored here
    pub rows: Vec<String>,
    // Use cursor index
    pub selection: Selection,

    // Options
    pub allow_select_blank: bool,
    pub auto_bracket_completion: bool,
    pub auto_indent: bool,
    pub tab_spaces: usize,
}

impl Default for TextEditState {
    fn default() -> Self {
        Self {
            cursor: Cursor::default(),
            rows: vec![String::default()],
            selection: Selection::default(),

            allow_select_blank: true,
            auto_bracket_completion: false,
            auto_indent: false,
            tab_spaces: 4,
        }
    }
}

impl TextEditState {
    pub fn load(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(TextEditState::default())
    }

    pub fn save(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }

    pub fn copy_text(&mut self) -> String {
        self.get_text(self.selection.start, self.selection.end)
    }

    pub fn cut_text(&mut self) -> String {
        let text = self.get_text(self.selection.start, self.selection.end);
        self.delete_text_by_selection();
        text
    }

    pub fn delete_text(&mut self) -> bool {
        let deleted = if !self.selection.is_none() {
            self.delete_text_by_selection()
        } else {
            self.delete_char_by_cursor()
        };

        if self.rows.is_empty() {
            self.insert_row();
        }

        deleted
    }

    pub fn find_beginning_spaces_of_row(&self, row_number: usize) -> usize {
        self.rows[row_number]
            .chars()
            .position(|c| c != ' ')
            .unwrap_or(self.glyphs_in_row(row_number))
    }

    // Position of cursor in cursor index
    pub fn find_cursor_index(&self) -> usize {
        self.find_start_index_of_row(self.cursor.row) + self.cursor.column
    }

    pub fn find_row_col_of_index(&self, index: usize) -> (usize, usize) {
        let row = self.find_row_number_of_index(index);
        let row_start_index = self.find_start_index_of_row(row);
        let col = index - row_start_index;
        (row, col)
    }

    // Row index of glyph index
    // glyph index  0   1   2   3
    //        text  a   b   c   \n
    //         row  0
    // glyph index  4   5   6   7
    //        text  d   e   f   \n
    //         row  1
    pub fn find_row_number_of_index(&self, index: usize) -> usize {
        let mut left = 0;
        let mut right = self.row_count();
        while left < right {
            let row_number = left + (right - left) / 2;
            let (row_start, row_end) = self.find_range_of_row(row_number);

            if index < row_start {
                right = row_number;
            } else if index >= row_end {
                left = row_number + 1;
            } else {
                return row_number;
            }
        }

        self.row_count() - 1
    }

    // Range of row in cursor index
    // cursor index  0   1   2   3   4
    //         text    a   b   c   \n
    //        range  (0, 4)
    // cursor index  4   5   6   7   8
    //         text    d   e   f   \n
    //        range  (4, 8)
    pub fn find_range_of_row(&self, row_number: usize) -> (usize, usize) {
        let start = self.find_start_index_of_row(row_number);
        let end = start + self.row_len(row_number);
        (start, end)
    }

    // Range of selected glyphs within a row
    pub fn find_selected_range_of_row(&self, row_number: usize) -> Option<(usize, usize)> {
        if self.selection.is_none() {
            return None;
        }

        let (start, end) = self.find_range_of_row(row_number);
        if !self.selection.intersects(start, end) {
            return None;
        }

        // Select the linebreak only
        if self.allow_select_blank && self.selection.start == end - 1 {
            return Some((end - 1, end));
        }

        let left = self.selection.start.max(start);
        let right = self.selection.end.min(
            // If it's an empty row, we select the linebreak
            // Or if it's the last row
            if self.allow_select_blank && start + 1 == end || self.is_last_row(row_number) {
                end
            } else {
                // Eliminate the linebreak if the row is not empty,
                // and it's not the last row
                end - 1
            },
        );
        if left == right {
            None
        } else {
            Some((left, right))
        }
    }

    // Range of selected glyphs within a range
    pub fn find_selected_range_within_range(
        &self,
        start: usize,
        end: usize,
    ) -> Option<(usize, usize)> {
        if self.selection.is_none() {
            return None;
        }

        if !self.selection.intersects(start, end) {
            return None;
        }

        let start_row = self.find_row_number_of_index(start);
        let end_row = self.find_row_number_of_index(end);

        let mut selection_start = end;
        let mut selection_end = start;
        for row_number in start_row..=end_row {
            let (start, end) = self.find_range_of_row(row_number);
            if !self.selection.intersects(start, end) {
                continue;
            }

            let left;
            let right;

            // Select the linebreak only
            if self.allow_select_blank && self.selection.start == end - 1 {
                left = end - 1;
                right = end;
            } else {
                left = self.selection.start.max(start);
                right = self.selection.end.min(
                    // If it's an empty row, we select the linebreak
                    // Or if it's the last row
                    if self.allow_select_blank && start + 1 == end || self.is_last_row(row_number) {
                        end
                    } else {
                        // Eliminate the linebreak if the row is not empty,
                        // and it's not the last row
                        end - 1
                    },
                );
                if left >= right {
                    continue;
                }
            }
            selection_start = selection_start.min(left);
            selection_end = selection_end.max(right);
        }

        let left = selection_start.max(start);
        let right = selection_end.min(end);
        if left >= right {
            None
        } else {
            Some((left, right))
        }
    }

    // Start position of row in cursor index
    pub fn find_start_index_of_row(&self, row_number: usize) -> usize {
        let mut index = 0;
        for i in 0..row_number {
            index += self.row_len(i)
        }
        index
    }

    pub fn goto_char_by_index(&mut self, char_index: usize) -> bool {
        let char_index =
            char_index.min(self.find_range_of_row(self.row_count().saturating_sub(1)).1);
        let (row, column) = self.find_row_col_of_index(char_index);

        let new_cursor = Cursor::new(row, column);
        if self.cursor == new_cursor {
            return false;
        }

        self.cursor = new_cursor;
        true
    }

    pub fn goto_row(&mut self, row_number: usize) -> bool {
        let row_number = row_number.min(self.row_count().saturating_sub(1));
        if row_number == self.cursor.row {
            return false;
        }

        let char_index = self.find_start_index_of_row(row_number);
        self.goto_char_by_index(char_index)
    }

    pub fn indent(&mut self) -> bool {
        if self.selection.is_none() {
            self.rows[self.cursor.row].insert_str(0, &(" ".repeat(self.tab_spaces)));
            self.cursor.column += self.tab_spaces;

            return true;
        }

        let start_row = self.find_row_number_of_index(self.selection.start);
        let end_row = self.find_row_number_of_index(self.selection.end);

        // If all selected lines are empty, no need to indent them
        if self.rows[start_row..=end_row]
            .iter()
            .all(|row| row.is_empty())
        {
            return false;
        }

        // Move selection start and cursor,
        // only if we select from actual text, not beginning spaces
        let select_beginning_spaces =
            self.find_selected_range_of_row(start_row)
                .is_some_and(|(start, _)| {
                    grapheme_string(
                        &self.rows[start_row],
                        0,
                        start - self.find_range_of_row(start_row).0,
                    )
                    .chars()
                    .all(|c| c.is_whitespace())
                });

        if !select_beginning_spaces {
            self.selection.start += self.tab_spaces;
        }

        for row in start_row..=end_row {
            // Skip empty line
            if self.rows[row].is_empty() {
                continue;
            }

            self.rows[row].insert_str(0, &(" ".repeat(self.tab_spaces)));
            self.selection.end += self.tab_spaces;
        }

        if !self.rows[self.cursor.row].is_empty()
            && (!select_beginning_spaces || self.cursor.row != start_row)
        {
            self.cursor.column += self.tab_spaces;
        }

        true
    }

    pub fn insert_char(&mut self, char: char) {
        if self.auto_bracket_completion && matches!(char, '(' | '{' | '[' | '<') {
            self.insert_brackets(char);
        } else {
            self.insert_text(char.to_string());
        }
    }

    pub fn insert_text(&mut self, text: String) -> (usize, usize) {
        if !self.selection.is_none() {
            self.delete_text_by_selection();
        }
        let start = self.find_cursor_index();
        let insert_index = self.byte_offset_of_index(self.cursor.row, self.cursor.column);
        if !text.contains('\n') {
            self.rows[self.cursor.row].insert_str(insert_index, &text);
            // The commit can join neighboring graphemes (combining marks,
            // regional indicators or ZWJ sequences). Count the resulting row,
            // rather than adding the isolated commit's grapheme count.
            self.cursor.column =
                grapheme_column(&self.rows[self.cursor.row], insert_index + text.len());
        } else {
            let leftover = self.rows[self.cursor.row].split_off(insert_index);
            let mut rows = text.split('\n');
            self.rows[self.cursor.row].push_str(rows.next().unwrap());
            for row in rows {
                self.cursor.row += 1;
                self.rows.insert(self.cursor.row, row.to_owned());
            }
            let end = self.rows[self.cursor.row].len();
            self.rows[self.cursor.row].push_str(&leftover);
            self.cursor.column = grapheme_column(&self.rows[self.cursor.row], end);
        }
        (start, self.find_cursor_index())
    }

    pub fn insert_row(&mut self) {
        if !self.selection.is_none() {
            self.delete_text_by_selection();
        }

        let beginning_spaces = if self.auto_indent {
            // We only need to make sure the spaces count match the current row's
            self.find_beginning_spaces_of_row(self.cursor.row)
                .min(self.cursor.column)
        } else {
            0
        };
        let new_row_start = " ".repeat(beginning_spaces);

        // Insert at current row
        if self.cursor.column == 0 {
            self.rows.insert(self.cursor.row, new_row_start);
            // Insert at next row
        } else if self.cursor.column >= self.glyphs_in_row(self.cursor.row) {
            self.rows.insert(self.cursor.row + 1, new_row_start);
            // Insert inside current row
        } else {
            let insert_index = self.byte_offset_of_index(self.cursor.row, self.cursor.column);
            let remaining = self.rows[self.cursor.row].split_off(insert_index);
            let new_text = format!("{new_row_start}{remaining}");
            self.rows.insert(self.cursor.row + 1, new_text);
        }

        self.cursor.column = beginning_spaces;
        self.move_cursor_down();
    }

    pub fn insert_tab(&mut self) -> (usize, usize) {
        self.insert_text(" ".repeat(self.tab_spaces))
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() || (self.rows.len() == 1 && self.rows[0].is_empty())
    }

    pub fn is_last_row(&self, row_number: usize) -> bool {
        row_number == self.row_count() - 1
    }

    pub fn is_row_all_selected(&self, row_number: usize) -> bool {
        #[allow(clippy::unnecessary_map_or)]
        self.find_selected_range_of_row(row_number)
            .map_or(false, |selected_range| {
                let range = self.find_range_of_row(row_number);
                range.0 == selected_range.0 && range.1 == selected_range.1 + 1
            })
    }

    pub fn move_cursor_down(&mut self) -> bool {
        if self.is_last_row(self.cursor.row) {
            return false;
        }

        self.cursor.row += 1;
        self.cursor.column = self.cursor.column.min(self.glyphs_in_row(self.cursor.row));
        true
    }

    pub fn move_cursor_left(&mut self) -> bool {
        if self.cursor.is_zero() {
            return false;
        }

        if self.cursor.column == 0 {
            self.cursor.row -= 1;
            self.cursor.column = self.glyphs_in_row(self.cursor.row);
        } else {
            self.cursor.column -= 1;
        }
        true
    }

    pub fn move_cursor_right(&mut self) -> bool {
        if self.is_last_row(self.cursor.row)
            && self.cursor.column == self.glyphs_in_row(self.cursor.row)
        {
            return false;
        }

        if self.cursor.column == self.glyphs_in_row(self.cursor.row) {
            self.cursor.row += 1;
            self.cursor.column = 0;
        } else {
            self.cursor.column += 1;
        }
        true
    }

    pub fn move_cursor_to_line_end(&mut self) -> bool {
        let glyphs_count = self.glyphs_in_row(self.cursor.row);
        if self.cursor.column == glyphs_count {
            return false;
        }

        self.cursor.column = glyphs_count;
        true
    }

    pub fn move_cursor_to_line_start(&mut self) -> bool {
        if self.cursor.column == 0 {
            return false;
        }

        self.cursor.column = 0;
        true
    }

    pub fn move_cursor_up(&mut self) -> bool {
        if self.cursor.row == 0 {
            return false;
        }

        self.cursor.row -= 1;
        self.cursor.column = self.cursor.column.min(self.glyphs_in_row(self.cursor.row));
        true
    }

    pub fn move_lines_down(&mut self) -> bool {
        if self.selection.is_none() {
            self.move_lines(self.cursor.row, self.cursor.row, 1)
        } else {
            self.move_lines(
                self.find_row_number_of_index(self.selection.start),
                self.find_row_number_of_index(self.selection.end),
                1,
            )
        }
    }

    pub fn move_lines_up(&mut self) -> bool {
        if self.selection.is_none() {
            self.move_lines(self.cursor.row, self.cursor.row, -1)
        } else {
            self.move_lines(
                self.find_row_number_of_index(self.selection.start),
                self.find_row_number_of_index(self.selection.end),
                -1,
            )
        }
    }

    pub fn outdent(&mut self) -> bool {
        let (start_row, end_row) = if self.selection.is_none() {
            (self.cursor.row, self.cursor.row)
        } else {
            let start_row = self.find_row_number_of_index(self.selection.start);
            let end_row = self.find_row_number_of_index(self.selection.end);

            (start_row, end_row)
        };

        let mut updated = false;

        for row in start_row..=end_row {
            let indent_spaces = self.find_beginning_spaces_of_row(row);

            let mut indent_level = indent_spaces / self.tab_spaces;
            if indent_spaces % self.tab_spaces == 0 {
                indent_level = indent_level.saturating_sub(1);
            }

            let leftover = self.rows[row].split_off(indent_spaces);
            let space_count = self.tab_spaces * indent_level;

            self.rows[row] = " ".repeat(space_count);
            self.rows[row].push_str(&leftover);

            if self.cursor.row == row {
                self.cursor.column = self
                    .cursor
                    .column
                    .saturating_sub(indent_spaces - space_count);
            }

            if !self.selection.is_none() {
                if start_row == row {
                    self.selection.start = self
                        .selection
                        .start
                        .saturating_sub(indent_spaces - space_count);

                    let select_beginning_spaces = self
                        .find_selected_range_of_row(start_row)
                        .is_some_and(|(start, _)| {
                            grapheme_string(
                                &self.rows[start_row],
                                0,
                                start - self.find_range_of_row(start_row).0,
                            )
                            .chars()
                            .all(|c| c.is_whitespace())
                        });

                    if select_beginning_spaces {
                        self.selection.start = self
                            .selection
                            .start
                            .max(self.find_range_of_row(start_row).0);
                    }
                }

                self.selection.end = self
                    .selection
                    .end
                    .saturating_sub(indent_spaces - space_count);
            }

            updated = true;
        }

        updated
    }

    pub fn quick_move_cursor_left(&mut self) -> bool {
        if self.cursor.column == 0 {
            return false;
        }

        let spaces = self.find_beginning_spaces_of_row(self.cursor.row);

        if spaces < self.cursor.column {
            self.cursor.column = spaces;
            return true;
        }

        self.cursor.column = 0;
        true
    }

    pub fn quick_move_cursor_right(&mut self) -> bool {
        let glyphs_count = self.glyphs_in_row(self.cursor.row);
        if self.cursor.column == glyphs_count {
            return false;
        };

        let spaces = self.find_beginning_spaces_of_row(self.cursor.row);

        if spaces > self.cursor.column {
            self.cursor.column = spaces;
            return true;
        }

        self.cursor.column = glyphs_count;
        true
    }

    pub fn quick_select(&mut self) {
        let text = &self.rows[self.cursor.row];
        let (row_start, row_end) = self.find_range_of_row(self.cursor.row);

        // Cursor is at the end of the row
        if self.cursor.column >= text.len() {
            // Select the linebreak of previous row
            if self.is_last_row(self.cursor.row) {
                if text.is_empty() {
                    self.selection.start = row_start - 1;
                    self.selection.end = row_start;
                    self.move_cursor_left();
                    return;
                }
            // Select the linebreak at the end of row
            } else {
                self.selection.start = row_end - 1;
                self.selection.end = row_end;
                return;
            }
        }

        // Select the empty space
        let col = self.cursor.column.min(text.len().saturating_sub(1));
        let current_char = text.chars().nth(col).unwrap();
        let (start, end) = if current_char.is_whitespace() {
            find_range(text, col, |char| !char.is_whitespace())
        }
        // Select a word
        else if might_be_word_char(current_char) {
            find_range(text, col, |char| !might_be_word_char(char))
        } else {
            find_range(text, col, |char| {
                char.is_whitespace() || might_be_word_char(char)
            })
        };

        self.selection.start = row_start + start;
        self.selection.end = row_start + end;

        fn find_range<P>(text: &str, index: usize, predicate: P) -> (usize, usize)
        where
            P: Fn(char) -> bool,
        {
            let start = text[..index]
                .char_indices()
                .rev()
                .find(|&(_, c)| predicate(c))
                .map_or(0, |(i, _)| i + 1);

            let end = text[index + 1..]
                .char_indices()
                .find(|&(_, c)| predicate(c))
                .map_or(text.len(), |(i, _)| index + 1 + i);

            (start, end)
        }

        fn might_be_word_char(c: char) -> bool {
            c.is_alphanumeric() || c == '_'
        }
    }

    pub fn reset(&mut self) {
        self.rows = vec![String::default()];
        self.reset_cursor();
        self.reset_selection();
    }

    pub fn reset_cursor(&mut self) {
        self.cursor.reset();
    }

    pub fn reset_selection(&mut self) {
        self.selection.reset();
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn select(&mut self, start: usize, end: usize) {
        self.selection.start = start;
        self.selection.end = end;
    }

    pub fn select_all(&mut self) {
        self.selection.start = 0;
        self.selection.end = self
            .rows
            .iter()
            .enumerate()
            .fold(0, |acc, (i, _)| acc + self.row_len(i));
    }

    pub fn select_row(&mut self) {
        let (start, end) = self.find_range_of_row(self.cursor.row);
        self.select(start, end);
    }

    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

    pub fn set_text(&mut self, text: String) {
        self.rows = text.split('\n').map(|s| s.to_string()).collect();
    }

    pub fn to_text(&self) -> String {
        self.rows.join("\n")
    }

    fn byte_offset_of_index(&self, row_number: usize, index: usize) -> usize {
        grapheme_indices(&self.rows[row_number], index).unwrap_or(self.rows[row_number].len())
    }

    fn delete_char_by_cursor(&mut self) -> bool {
        if self.cursor.is_zero() {
            return false;
        }

        // Delete linebreak and concat with previous row
        if self.cursor.column == 0 {
            self.cursor.column = self.glyphs_in_row(self.cursor.row - 1);
            let text = self.rows.remove(self.cursor.row);
            self.rows[self.cursor.row - 1].push_str(&text);
            self.move_cursor_up();
            return true;
        }

        let char_to_be_deleted = self.rows[self.cursor.row]
            .graphemes(true)
            .nth(self.cursor.column - 1)
            .unwrap();
        // Delete spaces
        // go back to last indent level if no non-space char is ahead of it,
        // or delete until the last non-space char
        if char_to_be_deleted == " " {
            let current_row_text = &self.rows[self.cursor.row];

            let last_non_space_char_column =
                grapheme_string(current_row_text, 0, self.cursor.column)
                    .graphemes(true)
                    .enumerate()
                    .filter(|(_, text)| *text != " ")
                    .last()
                    .map(|(i, _)| i + 1);

            let deletion_start = last_non_space_char_column
                .unwrap_or(((self.cursor.column - 1) / self.tab_spaces) * self.tab_spaces);

            if self.delete_range_of_row(self.cursor.row, deletion_start, self.cursor.column) {
                self.cursor.column = deletion_start;
                return true;
            }
        }

        // Delete normal char
        if self.delete_range_of_row(self.cursor.row, self.cursor.column - 1, self.cursor.column) {
            self.move_cursor_left();
            return true;
        }

        false
    }

    fn delete_range_of_row(&mut self, row_number: usize, start: usize, end: usize) -> bool {
        let left = start.min(end);
        let right = start.max(end).min(self.glyphs_in_row(row_number));
        if left == right {
            return false;
        }

        let left = self.byte_offset_of_index(row_number, left);
        let right = self.byte_offset_of_index(row_number, right);
        let text = &mut self.rows[row_number];
        let remaining = text.split_off(right);
        text.truncate(left);
        text.push_str(&remaining);

        true
    }

    fn delete_text_by_selection(&mut self) -> bool {
        if self.selection.is_none() {
            return false;
        }

        let cursor_index = self.find_cursor_index();
        let start_row = self.find_row_number_of_index(self.selection.start);
        let end_row = self.find_row_number_of_index(self.selection.end);

        // Find selection range of first row,
        // to be used in the future
        let (row_start, row_end) = self.find_range_of_row(start_row);
        let (start, end) = self
            .find_selected_range_of_row(start_row)
            .unwrap_or((row_end, row_end + 1));

        if start_row != end_row {
            // Handle last row
            self.delete_range_of_row(
                end_row,
                0,
                self.selection.end - self.find_start_index_of_row(end_row),
            );
            let text = self.rows.remove(end_row);
            // When only linebreak is selected, manually add a linebreak,
            // so we can delete chars safely later
            if self.selection.end == row_end && self.selection.end - 1 == self.selection.start {
                self.rows[start_row].push('\n');
            }
            self.rows[start_row].push_str(&text);

            // Remove inter rows
            for row_number in (start_row + 1..end_row).rev() {
                self.rows.remove(row_number);
            }
        }

        // Handle first row
        let left = start - row_start;
        let right = end - row_start;
        self.delete_range_of_row(start_row, left, right);

        // Reset cursor
        if cursor_index >= self.selection.start {
            if cursor_index < self.selection.end {
                self.cursor.row = start_row;
                self.cursor.column = left;
            } else {
                let cursor_index = cursor_index - (self.selection.end - self.selection.start);
                let (row, col) = self.find_row_col_of_index(cursor_index);
                self.cursor.row = row;
                self.cursor.column = col;
            }
        }

        self.reset_selection();

        true
    }

    pub(crate) fn get_text(&self, start: usize, end: usize) -> String {
        let (start_row, start_col) = self.find_row_col_of_index(start);
        let (end_row, end_col) = self.find_row_col_of_index(end);
        let end_col = end_col.min(self.rows[end_row].len());

        if start_row == end_row {
            grapheme_string(&self.rows[start_row], start_col, end_col).to_owned()
        } else {
            let mut text =
                grapheme_string(&self.rows[start_row], start_col, self.rows[start_row].len())
                    .to_owned();
            for row in &self.rows[start_row + 1..end_row] {
                text.push('\n');
                text.push_str(row.as_str());
            }
            text.push('\n');
            text.push_str(grapheme_string(&self.rows[end_row], 0, end_col));
            text
        }
    }

    fn glyphs_in_row(&self, row_number: usize) -> usize {
        self.rows[row_number].graphemes(true).count()
    }

    fn insert_brackets(&mut self, left: char) {
        let right = match left {
            '(' => ')',
            '{' => '}',
            '[' => ']',
            '<' => '>',
            _ => unreachable!(),
        };

        if self.selection.is_none() {
            let insert_index = self.byte_offset_of_index(self.cursor.row, self.cursor.column);
            self.rows[self.cursor.row].insert_str(insert_index, &format!("{left}{right}"));
            self.cursor.column += 1;
        } else {
            let insert_stuff = [self.selection.start, self.selection.end]
                .map(|global_index| self.find_row_number_of_index(global_index))
                .into_iter()
                .enumerate()
                .map(|(i, row)| {
                    let (row_start, row_end) = self.find_range_of_row(row);
                    let (start, end) = self
                        .find_selected_range_of_row(row)
                        .unwrap_or((row_end, row_end + 1));

                    if i == 0 {
                        (row, start - row_start, left)
                    } else {
                        (row, end - row_start + 1, right)
                    }
                })
                .collect::<Vec<_>>();

            if insert_stuff[0].0 == self.cursor.row {
                self.cursor.column += 1;
            }

            for (row, column, char) in insert_stuff {
                let insert_index = self.byte_offset_of_index(row, column);
                self.rows[row].insert(insert_index, char);
            }

            self.selection.start += 1;
            self.selection.end += 1;
        }
    }

    // Inclusive on both end
    fn move_lines(&mut self, start: usize, end: usize, vector: isize) -> bool {
        if vector == 0
            || (start as isize) + vector < 0
            || ((end as isize) + vector).abs() >= self.row_count() as isize
        {
            return false;
        }

        if vector < 0 {
            let vector = vector.unsigned_abs();
            if !self.selection.is_none() {
                let row_len = self.glyphs_in_row(start - vector) + 1;
                self.selection.start -= row_len;
                self.selection.end -= row_len;
            }
            for i in start..=end {
                self.rows.swap(i, i - vector);
            }
            self.cursor.row -= vector;
        } else {
            let vector = vector.unsigned_abs();
            if !self.selection.is_none() {
                let row_len = self.glyphs_in_row(end + vector) + 1;
                self.selection.start += row_len;
                self.selection.end += row_len;
            }
            for i in (start..=end).rev() {
                self.rows.swap(i, i + vector);
            }
            self.cursor.row += vector;
        }

        true
    }

    // Length of row in glyphs, linebreak included
    fn row_len(&self, row_number: usize) -> usize {
        // +1 to include the linebreak,
        // except for the last row
        let len = self.glyphs_in_row(row_number);
        if self.is_last_row(row_number) {
            len
        } else {
            len + 1
        }
    }
}

/// Snap a byte position to the following valid grapheme boundary.
fn grapheme_column(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .take_while(|(offset, _)| *offset < byte)
        .count()
}

pub(crate) fn grapheme_indices(text: &str, index: usize) -> Option<usize> {
    text.grapheme_indices(true)
        .nth(index)
        .map(|(byte_offset, _)| byte_offset)
}

pub(crate) fn grapheme_string(text: &str, start: usize, end: usize) -> &str {
    &text[grapheme_indices(text, start).unwrap_or_default()
        ..grapheme_indices(text, end).unwrap_or(text.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_editor_data_keeps_the_existing_cursor_rows_and_selection_schema() {
        let mut state = TextEditState::default();
        state.set_text("É👩‍🚀\n東京".into());
        state.cursor = Cursor::new(1, 1);
        state.select(1, 4);
        let saved = state.save();
        let json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(json["cursor"], serde_json::json!({"row":1,"column":1}));
        assert_eq!(json["selection"], serde_json::json!({"start":1,"end":4}));
        assert_eq!(json["rows"], serde_json::json!(["É👩‍🚀", "東京"]));
        let loaded = TextEditState::load(&saved);
        assert_eq!(loaded.save(), saved);
    }
    #[test]
    fn multiline_copy_uses_grapheme_positions_for_the_last_row() {
        let mut state = TextEditState::default();
        state.set_text("É👩‍🚀\n東京Ω".into());
        state.select(1, 4);
        assert_eq!(state.copy_text(), "👩‍🚀\n東");
    }
    #[test]
    fn committed_text_joining_a_previous_grapheme_keeps_the_cursor_valid() {
        let mut state = TextEditState::default();
        state.insert_text("A".into());
        state.insert_text("\u{301}x".into());
        assert_eq!(state.to_text(), "A\u{301}x");
        assert_eq!(state.cursor.column, 2);
        assert_eq!(state.find_cursor_index(), 2);
        assert!(!state.move_cursor_right());
    }
    #[test]
    fn deleting_spaces_after_a_multibyte_grapheme_does_not_slice_utf8_bytes() {
        let mut state = TextEditState::default();
        state.set_text("東 ".into());
        state.cursor = Cursor::new(0, 2);
        assert!(state.delete_text());
        assert_eq!(state.to_text(), "東");
        assert_eq!(state.cursor.column, 1);
        assert!(state.delete_text());
        assert_eq!(state.to_text(), "");
    }
    #[cfg(feature = "ui")]
    #[test]
    fn widget_cursor_and_editor_state_are_identity_reexports() {
        use crate::theui::thewidget::thetextedit::{TheCursor, TheSelection, TheTextEditState};
        use std::any::TypeId;
        assert_eq!(TypeId::of::<Cursor>(), TypeId::of::<TheCursor>());
        assert_eq!(TypeId::of::<Selection>(), TypeId::of::<TheSelection>());
        assert_eq!(
            TypeId::of::<TextEditState>(),
            TypeId::of::<TheTextEditState>()
        );
    }
}
