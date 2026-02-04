#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;

use fontdue::layout::{HorizontalAlign, LayoutSettings};
use web_time::Instant;

use crate::prelude::*;

use super::thetextedit::{TheCursor, TheTextEditState, TheTextRenderer};

#[derive(Default, PartialEq)]
enum StatusbarType {
    #[default]
    None,
    Widget(TheDim),
    Global,
}

pub struct TheCodeEditorSettings {
    pub auto_bracket_completion: bool,
    pub auto_indent: bool,
    pub indicate_space: bool,
}

impl Default for TheCodeEditorSettings {
    fn default() -> Self {
        Self {
            auto_bracket_completion: true,
            auto_indent: true,
            indicate_space: true,
        }
    }
}

pub struct TheTextAreaEdit {
    // Widget Basic
    id: TheId,
    limiter: TheSizeLimiter,
    status: Option<String>,

    // Dimension
    dim: TheDim,

    // Edit State
    is_disabled: bool,

    // Text state
    state: TheTextEditState,
    modified_since_last_return: bool,
    modified_since_last_tick: bool,

    // Text render
    renderer: TheTextRenderer,
    ln_area_dim: Option<TheDim>,
    scrollbar_size: usize,
    statusbar_type: StatusbarType,
    debug_line: Option<usize>,
    pending_scroll_row: Option<usize>,
    pending_scroll_centered: bool,

    // Interaction
    auto_scroll_to_cursor: bool,
    drag_start_index: usize,
    hover_coord: Vec2<i32>,
    is_clicking_on_selection: bool,
    last_mouse_down_coord: Vec2<i32>,
    last_mouse_down_time: Instant,
    readonly: bool,

    // Cursor icon
    cursor_icon: Option<TheCursorIcon>,

    // Modifiers
    modifier_alt: bool,
    modifier_ctrl: bool,
    modifier_logo: bool,
    modifier_shift: bool,

    // Scrollbar
    hscrollbar: Box<dyn TheWidget>,
    vscrollbar: Box<dyn TheWidget>,
    is_hscrollbar_clicked: bool,
    is_hscrollbar_hovered: bool,
    is_vscrollbar_clicked: bool,
    is_vscrollbar_hovered: bool,

    is_dirty: bool,
    embedded: bool,

    continuous: bool,

    undo_stack: TheUndoStack,
    supports_undo: bool,
}

impl TheWidget for TheTextAreaEdit {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_width(200);
        limiter.set_max_height(300);

        let hscrollbar = Box::new(TheHorizontalScrollbar::new(TheId::named(
            (id.name.clone() + " Horizontal Scrollbar").as_str(),
        )));
        let vscrollbar = Box::new(TheVerticalScrollbar::new(TheId::named(
            (id.name.clone() + " Vertical Scrollbar").as_str(),
        )));

        Self {
            id,
            limiter,
            status: None,

            dim: TheDim::zero(),

            is_disabled: false,

            state: TheTextEditState::default(),
            modified_since_last_return: false,
            modified_since_last_tick: false,

            renderer: TheTextRenderer::default(),
            ln_area_dim: None,
            scrollbar_size: 13,
            statusbar_type: StatusbarType::None,
            debug_line: None,
            pending_scroll_row: None,
            pending_scroll_centered: false,

            auto_scroll_to_cursor: true,
            drag_start_index: 0,
            hover_coord: Vec2::zero(),
            is_clicking_on_selection: false,
            last_mouse_down_coord: Vec2::zero(),
            last_mouse_down_time: Instant::now(),
            readonly: false,

            modifier_alt: false,
            modifier_ctrl: false,
            modifier_logo: false,
            modifier_shift: false,

            hscrollbar,
            vscrollbar,
            is_hscrollbar_clicked: false,
            is_hscrollbar_hovered: false,
            is_vscrollbar_clicked: false,
            is_vscrollbar_hovered: false,

            is_dirty: false,
            embedded: false,

            continuous: false,

            undo_stack: TheUndoStack::default(),

            cursor_icon: Some(TheCursorIcon::Text),
            supports_undo: true,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
    }

    fn disabled(&self) -> bool {
        self.is_disabled
    }

    fn set_disabled(&mut self, disabled: bool) {
        if disabled != self.is_disabled {
            self.is_disabled = disabled;
            self.is_dirty = true;
        }
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.modified_since_last_tick = true;
            self.is_dirty = true;
        }
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

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn supports_text_input(&self) -> bool {
        true
    }

    fn supports_clipboard(&mut self) -> bool {
        true
    }

    fn supports_undo_redo(&mut self) -> bool {
        self.supports_undo
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if self.is_disabled {
            return false;
        }

        let mut redraw = false;
        let mut update_status = false;
        match event {
            TheEvent::Undo => {
                if self.undo_stack.has_undo() {
                    let (_id, state) = self.undo_stack.undo();
                    self.state = TheTextEditState::load(&state);
                    self.modified_since_last_tick = true;
                    self.is_dirty = true;
                    redraw = true;
                    update_status = true;
                }
            }
            TheEvent::Redo => {
                if self.undo_stack.has_redo() {
                    let (_id, state) = self.undo_stack.redo();
                    self.state = TheTextEditState::load(&state);
                    self.modified_since_last_tick = true;
                    self.is_dirty = true;
                    redraw = true;
                    update_status = true;
                }
            }
            TheEvent::Copy => {
                let text = self.state.copy_text();
                if !text.is_empty() {
                    redraw = true;
                    update_status = true;

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut clipboard = Clipboard::new().unwrap();
                        clipboard.set_text(text.clone()).unwrap();
                    }

                    ctx.ui
                        .send(TheEvent::SetClipboard(TheValue::Text(text), None));
                }
            }
            TheEvent::Cut => {
                let prev_state = self.state.save();
                let text = self.state.cut_text();
                if !text.is_empty() {
                    self.modified_since_last_tick = true;
                    self.is_dirty = true;
                    redraw = true;
                    update_status = true;

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut clipboard = Clipboard::new().unwrap();
                        clipboard.set_text(text.clone()).unwrap();
                    }

                    ctx.ui
                        .send(TheEvent::SetClipboard(TheValue::Text(text), None));

                    let mut undo = TheUndo::new(TheId::named("Cut"));
                    undo.set_undo_data(prev_state);
                    undo.set_redo_data(self.state.save());
                    self.undo_stack.add(undo);

                    if self.continuous {
                        self.emit_value_changed(ctx);
                    }
                }
            }
            TheEvent::Paste(_value, _) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut clipboard = Clipboard::new().unwrap();
                    let text = clipboard.get_text().unwrap();

                    let prev_state = self.state.save();

                    self.state.insert_text(text);
                    self.modified_since_last_tick = true;
                    self.is_dirty = true;
                    redraw = true;
                    update_status = true;

                    // if self.continuous {
                    //     self.emit_value_changed(ctx);
                    // }

                    let mut undo = TheUndo::new(TheId::named("Cut"));
                    undo.set_undo_data(prev_state);
                    undo.set_redo_data(self.state.save());
                    self.undo_stack.add(undo);

                    if self.continuous {
                        self.emit_value_changed(ctx);
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    let prev_state = self.state.save();

                    if let Some(text) = _value.to_string() {
                        self.state.insert_text(text);
                        self.modified_since_last_tick = true;
                        self.is_dirty = true;
                        redraw = true;
                        update_status = true;

                        if self.continuous {
                            self.emit_value_changed(ctx);
                        }

                        let mut undo = TheUndo::new(TheId::named("Cut"));
                        undo.set_undo_data(prev_state);
                        undo.set_redo_data(self.state.save());
                        self.undo_stack.add(undo);
                    }
                }
            }
            TheEvent::ModifierChanged(shift, ctrl, alt, logo) => {
                self.modifier_alt = *alt;
                self.modifier_ctrl = *ctrl;
                self.modifier_logo = *logo;
                self.modifier_shift = *shift
            }
            TheEvent::MouseDown(coord) => {
                if !self.state.is_empty() {
                    let global_coord = coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                    if self.renderer.is_horizontal_overflow()
                        && self.hscrollbar.dim().contains(global_coord)
                    {
                        self.is_hscrollbar_clicked = true;
                        self.hscrollbar.on_event(
                            &TheEvent::MouseDown(self.hscrollbar.dim().to_local(global_coord)),
                            ctx,
                        );
                    } else if self.renderer.is_vertical_overflow()
                        && self.vscrollbar.dim().contains(global_coord)
                    {
                        self.is_vscrollbar_clicked = true;
                        self.vscrollbar.on_event(
                            &TheEvent::MouseDown(self.vscrollbar.dim().to_local(global_coord)),
                            ctx,
                        );
                    } else if self.renderer.dim().contains(global_coord) {
                        {
                            let mut coord = *coord;
                            if let Some(dim) = &self.ln_area_dim {
                                coord.x -= dim.width;
                            }
                            self.drag_start_index = self.renderer.find_cursor_index(&coord);
                            let (cursor_row, cursor_column) =
                                self.state.find_row_col_of_index(self.drag_start_index);
                            self.state
                                .set_cursor(TheCursor::new(cursor_row, cursor_column));
                            update_status = true;
                        }

                        let is_double_click = self.last_mouse_down_time.elapsed().as_millis() < 500
                            && self.last_mouse_down_coord == *coord;
                        if is_double_click {
                            if self.state.selection.is_none() {
                                // Select a word, a whole row or a spacing etc.
                                self.state.quick_select();
                            } else if self.state.is_row_all_selected(self.state.cursor.row) {
                                self.state.reset_selection();
                            } else {
                                self.state.select_row();
                            }
                        } else if self.drag_start_index >= self.state.selection.start
                            && self.drag_start_index < self.state.selection.end
                        {
                            self.is_clicking_on_selection = true;
                        } else {
                            self.state.reset_selection();
                        }
                    }
                }

                ctx.ui.set_focus(self.id());
                self.is_dirty = true;
                redraw = true;

                self.last_mouse_down_coord = *coord;
                self.last_mouse_down_time = Instant::now();
            }
            TheEvent::MouseDragged(coord) => {
                if *coord == self.last_mouse_down_coord {
                    return false;
                }
                self.is_dirty = true;

                if !self.state.is_empty() {
                    if self.is_hscrollbar_clicked {
                        redraw =
                            self.hscrollbar.on_event(
                                &TheEvent::MouseDragged(self.hscrollbar.dim().to_local(
                                    coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y),
                                )),
                                ctx,
                            );
                        if let Some(scrollbar) = self.hscrollbar.as_horizontal_scrollbar() {
                            redraw = self.renderer.scroll(
                                &Vec2::new(
                                    scrollbar.scroll_offset()
                                        - self.renderer.scroll_offset.x as i32,
                                    0,
                                ),
                                false,
                            ) || redraw;
                        }
                    } else if self.is_vscrollbar_clicked {
                        redraw =
                            self.vscrollbar.on_event(
                                &TheEvent::MouseDragged(self.vscrollbar.dim().to_local(
                                    coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y),
                                )),
                                ctx,
                            );
                        if let Some(scrollbar) = self.vscrollbar.as_vertical_scrollbar() {
                            redraw = self.renderer.scroll(
                                &Vec2::new(
                                    0,
                                    scrollbar.scroll_offset()
                                        - self.renderer.scroll_offset.y as i32,
                                ),
                                false,
                            ) || redraw;
                        }
                    } else {
                        let mut coord = *coord;
                        if let Some(dim) = &self.ln_area_dim {
                            coord.x -= dim.width;
                        }

                        let delta_x = if coord.x < 0 {
                            coord.x
                        } else if coord.x > self.dim.width {
                            coord.x - self.dim.width
                        } else {
                            0
                        };
                        let delta_y = if coord.y < 0 {
                            coord.y
                        } else if coord.y > self.dim.height {
                            coord.y - self.dim.height
                        } else {
                            0
                        };

                        if delta_x != 0 || delta_y != 0 {
                            let ratio = if self.last_mouse_down_time.elapsed().as_millis() > 500 {
                                8
                            } else {
                                4
                            };
                            self.renderer
                                .scroll(&Vec2::new(delta_x / ratio, delta_y / ratio), true);
                        }

                        let cursor_index = self.renderer.find_cursor_index(&coord);
                        let (cursor_row, cursor_column) =
                            self.state.find_row_col_of_index(cursor_index);
                        self.state
                            .set_cursor(TheCursor::new(cursor_row, cursor_column));
                        update_status = true;

                        if !self.is_clicking_on_selection {
                            if self.drag_start_index != cursor_index {
                                let start = self.drag_start_index.min(cursor_index);
                                let end = self.drag_start_index.max(cursor_index);
                                self.state.select(start, end);
                            } else {
                                self.state.reset_selection();
                            }
                        }

                        redraw = true;
                    }
                }
            }
            TheEvent::MouseUp(coord) => {
                let global_coord = coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                if self.is_hscrollbar_clicked {
                    self.hscrollbar.on_event(
                        &TheEvent::MouseUp(self.hscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );
                } else if self.is_vscrollbar_clicked {
                    self.vscrollbar.on_event(
                        &TheEvent::MouseUp(self.vscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );
                } else if self.renderer.dim().contains(global_coord)
                    && self.is_clicking_on_selection
                {
                    // Drag selection then cut/paste
                    if !self.readonly {
                        let cursor_index = self.state.find_cursor_index();
                        if cursor_index < self.state.selection.start
                            || cursor_index >= self.state.selection.end
                        {
                            let text = self.state.cut_text();
                            let (start, end) = self.state.insert_text(text);
                            self.state.select(start, end);
                            self.modified_since_last_tick = true;
                        } else {
                            self.state.reset_selection();
                        }
                    } else {
                        self.state.reset_selection();
                    }
                }

                self.is_dirty = true;
                redraw = true;

                self.is_clicking_on_selection = false;
                self.is_hscrollbar_clicked = false;
                self.is_vscrollbar_clicked = false;
                self.drag_start_index = 0;
            }
            TheEvent::MouseWheel(delta) => {
                let global_coord =
                    self.hover_coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                let scrolled = if self.hscrollbar.dim().contains(global_coord) {
                    let delta = if delta.x.abs() > delta.y.abs() {
                        delta.x
                    } else {
                        delta.y
                    };
                    self.renderer.scroll(&Vec2::new(-delta, 0), false)
                } else if self.vscrollbar.dim().contains(global_coord) {
                    let delta = if delta.x.abs() > delta.y.abs() {
                        delta.x
                    } else {
                        delta.y
                    };
                    self.renderer.scroll(&Vec2::new(0, -delta), false)
                } else {
                    self.renderer.scroll(&Vec2::new(-delta.x, -delta.y), false)
                };
                if scrolled {
                    self.is_dirty = true;
                    redraw = true;
                }
            }

            TheEvent::KeyDown(key) => {
                if !self.readonly {
                    let prev_state = self.state.save();
                    if let Some(c) = key.to_char() {
                        if (self.modifier_ctrl || self.modifier_logo) && c == 'a' {
                            self.state.select_all();
                            self.is_dirty = true;
                            redraw = true;
                        } else if (self.modifier_ctrl || self.modifier_logo) && c == '+' {
                            self.renderer.font_size += 1.0;
                            self.is_dirty = true;
                            self.modified_since_last_tick = true;
                            redraw = true;
                        } else if (self.modifier_ctrl || self.modifier_logo) && c == '-' {
                            if self.renderer.font_size > 5.0 {
                                self.renderer.font_size -= 1.0;
                                self.is_dirty = true;
                                self.modified_since_last_tick = true;
                                redraw = true;
                            }
                        } else if (self.modifier_ctrl || self.modifier_logo) && c == '/' {
                            // TODO: Toggle comments
                            // Only works for Python/TOML right now
                            let syntax_name = self
                                .renderer
                                .highlighter
                                .as_ref()
                                .map(|highlighter| highlighter.syntax().to_owned());

                            if syntax_name
                                .map(|syntax| &syntax == "Python" || &syntax == "TOML")
                                .unwrap_or_default()
                            {
                                let (start_row, end_row) = if self.state.selection.is_none() {
                                    (self.state.cursor.row, self.state.cursor.row)
                                } else {
                                    let start_row = self
                                        .state
                                        .find_row_number_of_index(self.state.selection.start);
                                    let end_row = self
                                        .state
                                        .find_row_number_of_index(self.state.selection.end);
                                    (start_row, end_row)
                                };

                                // Should we consider these rows as already commented
                                // If there are multiple lines to be considered, we skip those empty lines
                                let is_all_commented = if start_row != end_row {
                                    self.state.rows[start_row..=end_row]
                                        .iter()
                                        .filter(|row| !row.trim().is_empty())
                                        .all(|row| row.trim_start().starts_with("# "))
                                } else {
                                    self.state.rows[start_row].trim_start().starts_with("# ")
                                };

                                let start_of_start_row =
                                    self.state.find_start_index_of_row(start_row);
                                let should_move_cursor =
                                    self.state.find_cursor_index() > start_of_start_row;

                                let mut modified_line_count = 0;

                                for row_number in start_row..=end_row {
                                    // Remove comments
                                    if is_all_commented {
                                        if let Some((left, right)) =
                                            self.state.rows[row_number].split_once("# ")
                                        {
                                            self.state.rows[row_number] =
                                                format!("{}{}", left, right);

                                            modified_line_count += 1;

                                            if should_move_cursor
                                                && row_number == self.state.cursor.row
                                            {
                                                let beginning_spaces = self
                                                    .state
                                                    .find_beginning_spaces_of_row(row_number);
                                                if beginning_spaces <= self.state.cursor.column {
                                                    self.state.cursor.column -= 2;
                                                }
                                            }
                                        }
                                    // Add comments
                                    } else if start_row == end_row
                                        || !self.state.rows[row_number].trim().is_empty()
                                    {
                                        let beginning_spaces =
                                            self.state.find_beginning_spaces_of_row(row_number);
                                        self.state.rows[row_number]
                                            .insert_str(beginning_spaces, "# ");

                                        modified_line_count += 1;

                                        if should_move_cursor
                                            && row_number == self.state.cursor.row
                                            && beginning_spaces <= self.state.cursor.column
                                        {
                                            self.state.cursor.column += 2;
                                        }
                                    }
                                }

                                if modified_line_count > 0 {
                                    self.state.cursor.column = self.state.cursor.column.max(0);
                                }

                                if !self.state.selection.is_none() {
                                    if is_all_commented {
                                        self.state.selection.start =
                                            self.state.selection.start.saturating_sub(2);
                                        self.state.selection.end = self
                                            .state
                                            .selection
                                            .end
                                            .saturating_sub(2 * modified_line_count);

                                        self.state.selection.start =
                                            self.state.selection.start.max(start_of_start_row);
                                    } else {
                                        if self.state.selection.start > start_of_start_row {
                                            self.state.selection.start += 2;
                                        }
                                        self.state.selection.end += 2 * modified_line_count;
                                    }
                                }

                                self.is_dirty = true;
                                self.modified_since_last_tick = true;
                                if self.continuous {
                                    self.emit_value_changed(ctx);
                                }
                                redraw = true;
                            }
                        } else {
                            self.state.insert_char(c);
                            self.modified_since_last_tick = true;
                            self.is_dirty = true;
                            redraw = true;
                            update_status = true;

                            if self.continuous {
                                self.emit_value_changed(ctx);
                            }
                        }
                    }
                    if self.is_dirty {
                        let mut undo = TheUndo::new(TheId::named("Input"));
                        undo.set_undo_data(prev_state);
                        undo.set_redo_data(self.state.save());
                        self.undo_stack.add(undo);
                    }
                }
            }
            TheEvent::KeyCodeDown(key_code) => {
                let prev_state = self.state.save();
                if let Some(key) = key_code.to_key_code() {
                    if !self.readonly {
                        match key {
                            TheKeyCode::Return => {
                                self.state.insert_row();
                                self.modified_since_last_tick = true;
                                self.is_dirty = true;
                                redraw = true;
                                update_status = true;

                                if self.continuous {
                                    self.emit_value_changed(ctx);
                                }
                            }
                            TheKeyCode::Delete => {
                                if self.state.delete_text() {
                                    self.modified_since_last_tick = true;
                                    self.is_dirty = true;
                                    redraw = true;
                                    update_status = true;

                                    if self.continuous {
                                        self.emit_value_changed(ctx);
                                    }
                                }
                            }
                            TheKeyCode::Up => {
                                if self.modifier_alt {
                                    if self.state.move_lines_up() {
                                        self.modified_since_last_tick = true;
                                        self.is_dirty = true;
                                        redraw = true;
                                    }
                                } else if self.modifier_shift {
                                    let cursor_index = self.state.find_cursor_index();
                                    let is_cursor_at_selection_start =
                                        cursor_index <= self.state.selection.start;
                                    let is_cursor_at_selection_end =
                                        cursor_index >= self.state.selection.end;

                                    if self.state.move_cursor_up() {
                                        let new_cursor_index = self.state.find_cursor_index();

                                        if self.state.selection.is_none() {
                                            self.state.select(new_cursor_index, cursor_index);
                                        } else {
                                            if is_cursor_at_selection_start {
                                                self.state.selection.start = new_cursor_index;
                                            }
                                            if is_cursor_at_selection_end {
                                                if new_cursor_index < self.state.selection.start {
                                                    self.state.select(
                                                        new_cursor_index,
                                                        self.state.selection.end,
                                                    );
                                                }
                                                if new_cursor_index > self.state.selection.start {
                                                    self.state.select(
                                                        self.state.selection.start,
                                                        new_cursor_index,
                                                    );
                                                }
                                            }
                                        }

                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                } else {
                                    let updated = {
                                        if self.state.selection.is_none() {
                                            self.state.move_cursor_up()
                                        } else {
                                            let (row, column) = self
                                                .state
                                                .find_row_col_of_index(self.state.selection.start);
                                            self.state.set_cursor(TheCursor::new(row, column));
                                            self.state.move_cursor_up();
                                            self.state.reset_selection();
                                            true
                                        }
                                    };

                                    if updated {
                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                }
                            }
                            TheKeyCode::Right => {
                                if self.modifier_ctrl || self.modifier_logo {
                                    if self.state.quick_move_cursor_right()
                                        || self.state.move_cursor_right()
                                    {
                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                    }
                                } else if self.modifier_shift {
                                    let cursor_index = self.state.find_cursor_index();
                                    let is_cursor_at_selection_start =
                                        cursor_index == self.state.selection.start;
                                    let is_cursor_at_selection_end =
                                        cursor_index == self.state.selection.end;

                                    if self.state.move_cursor_right() {
                                        if self.state.selection.is_none() {
                                            self.state.select(cursor_index, cursor_index + 1);
                                        } else {
                                            if is_cursor_at_selection_start {
                                                self.state.selection.start = cursor_index + 1;
                                            }
                                            if is_cursor_at_selection_end {
                                                self.state.selection.end = cursor_index + 1;
                                            }
                                        }

                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                } else {
                                    let updated = {
                                        if self.state.selection.is_none() {
                                            self.state.move_cursor_right()
                                        } else {
                                            let (row, column) = self
                                                .state
                                                .find_row_col_of_index(self.state.selection.end);
                                            self.state.set_cursor(TheCursor::new(row, column));
                                            self.state.reset_selection();
                                            true
                                        }
                                    };

                                    if updated {
                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                }
                            }
                            TheKeyCode::Down => {
                                if self.modifier_alt {
                                    if self.state.move_lines_down() {
                                        self.modified_since_last_tick = true;
                                        self.is_dirty = true;
                                        redraw = true;
                                    }
                                } else if self.modifier_shift {
                                    let cursor_index = self.state.find_cursor_index();
                                    let is_cursor_at_selection_start =
                                        cursor_index <= self.state.selection.start;
                                    let is_cursor_at_selection_end =
                                        cursor_index >= self.state.selection.end;

                                    if self.state.move_cursor_down() {
                                        let new_cursor_index = self.state.find_cursor_index();

                                        if self.state.selection.is_none() {
                                            self.state.select(cursor_index, new_cursor_index);
                                        } else {
                                            if is_cursor_at_selection_start {
                                                if new_cursor_index > self.state.selection.end {
                                                    self.state.select(
                                                        self.state.selection.start,
                                                        new_cursor_index,
                                                    );
                                                }
                                                if new_cursor_index < self.state.selection.end {
                                                    self.state.select(
                                                        new_cursor_index,
                                                        self.state.selection.end,
                                                    );
                                                }
                                            }
                                            if is_cursor_at_selection_end {
                                                self.state.selection.end = new_cursor_index;
                                            }
                                        }

                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                } else {
                                    let updated = {
                                        if self.state.selection.is_none() {
                                            self.state.move_cursor_down()
                                        } else {
                                            let (row, column) = self
                                                .state
                                                .find_row_col_of_index(self.state.selection.end);
                                            self.state.set_cursor(TheCursor::new(row, column));
                                            self.state.move_cursor_down();
                                            self.state.reset_selection();
                                            true
                                        }
                                    };

                                    if updated {
                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                }
                            }
                            TheKeyCode::Left => {
                                if self.modifier_ctrl || self.modifier_logo {
                                    if self.state.quick_move_cursor_left()
                                        || self.state.move_cursor_left()
                                    {
                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                    }
                                } else if self.modifier_shift {
                                    let cursor_index = self.state.find_cursor_index();
                                    let is_cursor_at_selection_start =
                                        cursor_index == self.state.selection.start;
                                    let is_cursor_at_selection_end =
                                        cursor_index == self.state.selection.end;

                                    if self.state.move_cursor_left() {
                                        if self.state.selection.is_none() {
                                            self.state.select(cursor_index - 1, cursor_index);
                                        } else {
                                            if is_cursor_at_selection_start {
                                                self.state.selection.start = cursor_index - 1;
                                            }
                                            if is_cursor_at_selection_end {
                                                self.state.selection.end = cursor_index - 1;
                                            }
                                        }

                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                } else {
                                    let updated = {
                                        if self.state.selection.is_none() {
                                            self.state.move_cursor_left()
                                        } else {
                                            let (row, column) = self
                                                .state
                                                .find_row_col_of_index(self.state.selection.start);
                                            self.state.set_cursor(TheCursor::new(row, column));
                                            self.state.reset_selection();
                                            true
                                        }
                                    };

                                    if updated {
                                        if self.auto_scroll_to_cursor {
                                            self.renderer.scroll_to_cursor(
                                                self.state.find_cursor_index(),
                                                self.state.cursor.row,
                                            );
                                        }
                                        self.is_dirty = true;
                                        redraw = true;
                                        update_status = true;
                                    }
                                }
                            }
                            TheKeyCode::Space => {
                                self.state.insert_text(" ".to_owned());
                                self.modified_since_last_tick = true;
                                self.is_dirty = true;
                                redraw = true;
                                update_status = true;

                                if self.continuous {
                                    self.emit_value_changed(ctx);
                                }
                            }
                            TheKeyCode::Tab => {
                                let updated = {
                                    if self.modifier_shift {
                                        self.state.outdent()
                                    } else if self.state.selection.is_none() {
                                        self.state.insert_tab();
                                        true
                                    } else {
                                        let start_row = self
                                            .state
                                            .find_row_number_of_index(self.state.selection.start);
                                        let end_row = self
                                            .state
                                            .find_row_number_of_index(self.state.selection.end);

                                        if start_row == end_row {
                                            self.state.insert_tab();
                                            true
                                        } else {
                                            self.state.indent()
                                        }
                                    }
                                };

                                if updated {
                                    self.modified_since_last_tick = true;
                                    self.is_dirty = true;
                                    redraw = true;
                                    update_status = true;

                                    if self.continuous {
                                        self.emit_value_changed(ctx);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if self.is_dirty {
                    let mut undo = TheUndo::new(TheId::named("Input"));
                    undo.set_undo_data(prev_state);
                    undo.set_redo_data(self.state.save());
                    self.undo_stack.add(undo);
                }
            }
            TheEvent::GainedFocus(_id) => {
                // Set text cursor when gaining focus
                self.cursor_icon = Some(TheCursorIcon::Text);
            }
            TheEvent::LostFocus(_id) => {
                // if self.modified_since_last_return {
                //     self.emit_value_changed(ctx);
                // }

                // Reset cursor to text when losing focus
                self.cursor_icon = Some(TheCursorIcon::Text);
            }
            TheEvent::Hover(coord) => {
                // The hovered widget is always current widget not scrollbars
                // We should manually draw hovered style to scrollbar hovered
                let global_coord = coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                if self.renderer.is_horizontal_overflow() {
                    self.hscrollbar.on_event(
                        &TheEvent::Hover(self.hscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );

                    self.is_hscrollbar_hovered = self.hscrollbar.id().equals(&ctx.ui.hover);
                    redraw = redraw || self.hscrollbar.needs_redraw();
                }
                if self.renderer.is_vertical_overflow() {
                    self.vscrollbar.on_event(
                        &TheEvent::Hover(self.vscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );

                    self.is_vscrollbar_hovered = self.vscrollbar.id().equals(&ctx.ui.hover);
                    redraw = redraw || self.vscrollbar.needs_redraw();
                }

                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }

                // Set text cursor when hovered (only if not already focused)
                if !ctx.ui.has_focus(self.id()) {
                    self.cursor_icon = Some(TheCursorIcon::Text);
                }

                self.hover_coord = *coord;
            }
            _ => {}
        }

        if update_status && self.statusbar_type == StatusbarType::Global {
            ctx.ui.send(TheEvent::SetStatusText(
                self.id().clone(),
                self.statusbar_text(),
            ));
        }

        redraw
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        self.cursor_icon
    }

    fn set_cursor_icon(&mut self, icon: Option<TheCursorIcon>) {
        self.cursor_icon = icon;
    }

    fn value(&self) -> TheValue {
        TheValue::Text(self.state.to_text())
    }

    fn set_value(&mut self, value: TheValue) {
        match value {
            TheValue::Empty => {
                self.state.reset();
                self.modified_since_last_tick = true;
                self.is_dirty = true;
            }
            TheValue::Text(text) => {
                self.state.reset();
                self.state.set_text(text);
                self.modified_since_last_tick = true;
                self.is_dirty = true;
            }
            _ => {}
        }
        self.undo_stack.clear();
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }

        let mut shrinker = TheDimShrinker::zero();
        self.renderer.render_widget(
            &mut shrinker,
            self.is_disabled,
            self.embedded,
            true,
            true,
            self,
            buffer,
            style,
            ctx,
            true,
        );

        if self.modified_since_last_tick || self.renderer.row_count() == 0 {
            self.renderer
                .prepare(&self.state.to_text(), TheFontPreference::Code, &ctx.draw);

            shrinker.shrink_by(
                -(self.renderer.padding.0 as i32),
                -(self.renderer.padding.1 as i32),
                -(self.renderer.padding.2 as i32),
                -(self.renderer.padding.3 as i32),
            );
            let mut outer_area = self.dim.to_buffer_shrunk_utuple(&shrinker);

            shrinker.shrink_by(
                self.renderer.padding.0 as i32,
                self.renderer.padding.1 as i32,
                self.renderer.padding.2 as i32,
                self.renderer.padding.3 as i32,
            );
            let mut visible_area = self.dim.to_buffer_shrunk_utuple(&shrinker);

            if let StatusbarType::Widget(dim) = &mut self.statusbar_type {
                let font_size = self.renderer.font_size;
                let statusbar_height = (1.5 * font_size).round() as usize;
                dim.x = outer_area.0 as i32;
                dim.y = (outer_area.1 + outer_area.3).saturating_sub(statusbar_height) as i32;
                dim.width = outer_area.2 as i32;
                dim.height = statusbar_height as i32;
                dim.set_buffer_offset(dim.x, dim.y);

                outer_area.3 = outer_area.3.saturating_sub(statusbar_height);
                visible_area.3 = visible_area.3.saturating_sub(statusbar_height);
            }

            if let Some(dim) = &mut self.ln_area_dim {
                let font_size = self.renderer.font_size;
                let digit_count = self.state.row_count().to_string().len();
                let line_number_width = ctx
                    .draw
                    // We assume '9' is one of the widest chars within 0-9
                    .get_text_size(
                        &"9".repeat(digit_count),
                        &TheFontSettings {
                            size: font_size,
                            preference: TheFontPreference::Code,
                        },
                    )
                    .0;
                let line_number_area_width = line_number_width + font_size.round() as usize;
                dim.x = outer_area.0 as i32;
                dim.y = outer_area.1 as i32;
                dim.width = line_number_area_width as i32;
                dim.height = outer_area.3 as i32;
                dim.set_buffer_offset(dim.x, dim.y);

                outer_area.0 += line_number_area_width;
                outer_area.2 = outer_area.2.saturating_sub(line_number_area_width);
                visible_area.0 += line_number_area_width;
                visible_area.2 = visible_area.2.saturating_sub(line_number_area_width);
            }

            let content_w = self.renderer.actual_size.x;
            let content_h = self.renderer.actual_size.y;
            let outer_w = visible_area.2;
            let outer_h = visible_area.3;
            let inner_w = outer_w.saturating_sub(self.scrollbar_size);
            let inner_h = outer_h.saturating_sub(self.scrollbar_size);
            let (is_hoverflow, is_voverflow) = if content_w <= outer_w && content_h <= outer_h {
                (false, false)
            } else if content_w > outer_w && content_h > outer_h {
                (true, true)
            } else {
                (content_w > inner_w, content_h > inner_h)
            };
            if is_hoverflow {
                visible_area.3 = inner_h;
            }
            if is_voverflow {
                visible_area.2 = inner_w;
            }
            self.renderer.set_dim(
                visible_area.0,
                visible_area.1,
                visible_area.2,
                visible_area.3,
            );

            if let Some(row) = self.pending_scroll_row.take() {
                if self.pending_scroll_centered {
                    self.renderer.scroll_to_row_centered(row);
                } else {
                    self.renderer.scroll_to_row_with_margin(row, 3);
                }
                self.pending_scroll_centered = false;
            } else if self.auto_scroll_to_cursor {
                self.renderer
                    .scroll_to_cursor(self.state.find_cursor_index(), self.state.cursor.row);
            }

            if is_hoverflow {
                let mut dim = TheDim::new(
                    outer_area.0 as i32,
                    (outer_area.1 + outer_area.3).saturating_sub(self.scrollbar_size) as i32,
                    outer_area
                        .2
                        .saturating_sub(if is_voverflow { self.scrollbar_size } else { 0 })
                        as i32,
                    self.scrollbar_size as i32,
                );
                dim.set_buffer_offset(dim.x, dim.y);
                self.hscrollbar.set_dim(dim, ctx);
            }
            if let Some(scrollbar) = self.hscrollbar.as_horizontal_scrollbar() {
                scrollbar.set_total_width(
                    self.renderer.actual_size.x as i32
                        + self.renderer.padding.0
                        + self.renderer.padding.2,
                );
            }

            if is_voverflow {
                let mut dim = TheDim::new(
                    (outer_area.0 + outer_area.2).saturating_sub(self.scrollbar_size) as i32,
                    outer_area.1 as i32,
                    self.scrollbar_size as i32,
                    outer_area
                        .3
                        .saturating_sub(if is_hoverflow { self.scrollbar_size } else { 0 })
                        as i32,
                );
                dim.set_buffer_offset(dim.x, dim.y);
                self.vscrollbar.set_dim(dim, ctx);
            }
            if let Some(scrollbar) = self.vscrollbar.as_vertical_scrollbar() {
                scrollbar.set_total_height(
                    self.renderer.actual_size.y as i32
                        + self.renderer.padding.1
                        + self.renderer.padding.3,
                );
            }
        }

        self.renderer.render_text(
            &self.state,
            ctx.ui.has_focus(self.id()),
            self.readonly,
            buffer,
            style,
            TheFontPreference::Code,
            &ctx.draw,
        );

        if let StatusbarType::Widget(dim) = &self.statusbar_type {
            let stride = buffer.stride();
            if !self.is_disabled {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &dim.to_buffer_utuple(),
                    stride,
                    style.theme().color(TextEditBackground),
                );
            } else {
                ctx.draw.blend_rect(
                    buffer.pixels_mut(),
                    &dim.to_buffer_utuple(),
                    stride,
                    style.theme().color_disabled_t(TextEditBackground),
                );
            }
            ctx.draw.rect(
                buffer.pixels_mut(),
                &(dim.x as usize, dim.y as usize, dim.width as usize, 1),
                stride,
                style.theme().color(TextEditBorder),
            );

            let font_size = self.renderer.font_size * 0.8;
            let text = self.statusbar_text();
            let text_size = ctx.draw.get_text_size(
                &text,
                &TheFontSettings {
                    size: font_size,
                    preference: TheFontPreference::Code,
                },
            );
            let right = dim.x + dim.width - font_size.ceil() as i32;
            let top = dim.y + (dim.height as f32 * 0.5).round() as i32
                - (text_size.1 as f32 * 0.5).round() as i32;
            ctx.draw.text_rect_blend_clip(
                buffer.pixels_mut(),
                &Vec2::new(right - text_size.0 as i32, top - 1),
                &dim.to_buffer_utuple(),
                stride,
                &text,
                TheFontSettings {
                    size: font_size,
                    preference: TheFontPreference::Code,
                },
                style.theme().color_disabled_t(TextEditTextColor),
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        }

        if let Some(dim) = &self.ln_area_dim {
            let stride = buffer.stride();
            if !self.is_disabled {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &dim.to_buffer_utuple(),
                    stride,
                    style.theme().color(TextEditBackground),
                );
            } else {
                ctx.draw.blend_rect(
                    buffer.pixels_mut(),
                    &dim.to_buffer_utuple(),
                    stride,
                    style.theme().color_disabled_t(TextEditBackground),
                );
            }
            ctx.draw.rect(
                buffer.pixels_mut(),
                &(
                    (dim.x + dim.width - 1) as usize,
                    dim.y as usize,
                    1,
                    dim.height as usize,
                ),
                stride,
                style.theme().color(TextEditBorder),
            );

            if let Some((start_row, end_row)) = self.renderer.visible_rows() {
                let font_size = self.renderer.font_size;
                let text = (start_row..=end_row)
                    .map(|i| format!("{}", i + 1))
                    .collect::<Vec<String>>();
                let layout = ctx.draw.get_text_layout(
                    &text.join("\n"),
                    &TheFontSettings {
                        size: font_size,
                        preference: TheFontPreference::Code,
                    },
                    LayoutSettings {
                        horizontal_align: HorizontalAlign::Right,
                        max_width: Some(dim.width as f32 - font_size),
                        ..LayoutSettings::default()
                    },
                );
                let lines = layout.lines().unwrap();
                let left = dim.x + (0.5 * font_size).ceil() as i32;
                let mut rect = dim.to_buffer_utuple();
                if self.renderer.is_horizontal_overflow() {
                    rect.3 = rect.3.saturating_sub(self.scrollbar_size);
                }
                for i in start_row..=end_row {
                    let line = lines[i - start_row];
                    let top = dim.y - self.renderer.scroll_offset.y as i32
                        + (self.renderer.row_baseline(i) as f32 - line.max_ascent).ceil() as i32;
                    let color = if self.debug_line == Some(i) {
                        style.theme().color(TextEditLineNumberDebugColor)
                    } else if self.state.cursor.row == i {
                        style.theme().color(TextEditLineNumberHighlightColor)
                    } else {
                        style.theme().color_disabled_t(TextEditLineNumberColor)
                    };
                    ctx.draw.text_rect_blend_clip(
                        buffer.pixels_mut(),
                        &Vec2::new(
                            left + layout.glyphs()[line.glyph_start].x.ceil() as i32,
                            top - 1,
                        ),
                        &rect,
                        stride,
                        &text[i - start_row],
                        TheFontSettings {
                            size: font_size,
                            preference: TheFontPreference::Code,
                        },
                        color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
        }

        if self.renderer.is_horizontal_overflow() {
            if let Some(scrollbar) = self.hscrollbar.as_horizontal_scrollbar() {
                scrollbar.set_scroll_offset(self.renderer.scroll_offset.x as i32);

                if self.is_hscrollbar_hovered {
                    ctx.ui.set_hover(self.hscrollbar.id());
                }
                self.hscrollbar.draw(buffer, style, ctx);
                if self.is_hscrollbar_hovered {
                    ctx.ui.set_hover(self.id());
                }
            }
        }
        if self.renderer.is_vertical_overflow() {
            if let Some(scrollbar) = self.vscrollbar.as_vertical_scrollbar() {
                scrollbar.set_scroll_offset(self.renderer.scroll_offset.y as i32);

                if self.is_vscrollbar_hovered {
                    ctx.ui.set_hover(self.vscrollbar.id());
                }
                self.vscrollbar.draw(buffer, style, ctx);
                if self.is_vscrollbar_hovered {
                    ctx.ui.set_hover(self.id());
                }
            }
        }

        self.modified_since_last_return =
            self.modified_since_last_return || self.modified_since_last_tick;
        self.modified_since_last_tick = false;
        self.is_dirty = false;
    }

    fn as_text_area_edit(&mut self) -> Option<&mut dyn TheTextAreaEditTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheTextAreaEditTrait: TheWidget {
    fn text(&self) -> String;
    fn set_text(&mut self, text: String);
    fn set_font_size(&mut self, font_size: f32);
    fn set_embedded(&mut self, embedded: bool);
    fn set_continuous(&mut self, continuous: bool);
    fn as_code_editor(&mut self, code_type: &str, settings: TheCodeEditorSettings);
    fn set_code_type(&mut self, code_type: &str);
    fn add_syntax_from_string(&mut self, syntax: &str);
    fn add_theme_from_string(&mut self, theme: &str);
    fn set_code_theme(&mut self, code_theme: &str);
    fn set_tab_spaces(&mut self, tab_spaces: usize);
    fn auto_scroll_to_cursor(&mut self, auto_scroll_to_cursor: bool);
    fn display_line_number(&mut self, display_line_number: bool);
    fn readonly(&mut self, readonly: bool);
    fn use_statusbar(&mut self, use_statusbar: bool);
    fn use_global_statusbar(&mut self, use_global_statusbar: bool);
    fn set_matches(&mut self, matches: &[(usize, usize)]);
    fn clear_matches(&mut self);
    fn highlight_match(&mut self, highlight_index: usize);
    fn set_errors(&mut self, errors: &[(usize, usize)]);
    fn clear_errors(&mut self);
    fn set_debug_line(&mut self, line_number: Option<usize>);
    fn goto_char_by_index(&mut self, char_index: usize);
    fn goto_line(&mut self, line_number: usize);
    fn set_supports_undo(&mut self, supports_undo: bool);
    fn get_state(&self) -> TheTextEditState;
    fn set_state(&mut self, state: TheTextEditState);
}

impl TheTextAreaEditTrait for TheTextAreaEdit {
    fn text(&self) -> String {
        self.state.to_text()
    }
    fn set_text(&mut self, text: String) {
        self.state.set_text(text);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn set_font_size(&mut self, font_size: f32) {
        self.renderer.set_font_size(font_size);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
    }
    fn set_continuous(&mut self, continuous: bool) {
        self.continuous = continuous;
    }
    fn as_code_editor(&mut self, code_type: &str, settings: TheCodeEditorSettings) {
        self.set_code_type(code_type);
        self.state.auto_bracket_completion = settings.auto_bracket_completion;
        self.state.auto_indent = settings.auto_indent;
        self.renderer.indicate_space = settings.indicate_space;
    }
    fn set_code_type(&mut self, code_type: &str) {
        self.renderer.set_code_type(code_type);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn add_syntax_from_string(&mut self, code_type: &str) {
        self.renderer.add_syntax_from_string(code_type);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn add_theme_from_string(&mut self, theme: &str) {
        self.renderer.add_theme_from_string(theme);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn set_code_theme(&mut self, code_theme: &str) {
        self.renderer.set_code_theme(code_theme);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn set_tab_spaces(&mut self, tab_spaces: usize) {
        self.state.tab_spaces = tab_spaces;
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn auto_scroll_to_cursor(&mut self, auto_scroll_to_cursor: bool) {
        self.auto_scroll_to_cursor = auto_scroll_to_cursor;
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn display_line_number(&mut self, display_line_number: bool) {
        self.ln_area_dim = display_line_number.then_some(TheDim::zero());
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
        self.is_dirty = true;
    }
    fn use_statusbar(&mut self, use_statusbar: bool) {
        if use_statusbar {
            self.statusbar_type = StatusbarType::Widget(TheDim::zero());
        } else {
            self.statusbar_type = StatusbarType::None;
        }
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn use_global_statusbar(&mut self, use_global_statusbar: bool) {
        if use_global_statusbar {
            self.statusbar_type = StatusbarType::Global;
        } else {
            self.statusbar_type = StatusbarType::None;
        }
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn set_matches(&mut self, matches: &[(usize, usize)]) {
        self.renderer.set_matches(matches.to_owned());
    }
    fn clear_matches(&mut self) {
        self.renderer.clear_matches();
    }
    fn highlight_match(&mut self, highlight_index: usize) {
        self.renderer.highlight_match(highlight_index);
    }
    fn set_errors(&mut self, errors: &[(usize, usize)]) {
        self.renderer.set_errors(errors.to_owned());
    }
    fn clear_errors(&mut self) {
        self.renderer.clear_errors();
    }
    fn set_debug_line(&mut self, line_number: Option<usize>) {
        self.debug_line = line_number;
        self.renderer.set_debug_line(line_number);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn goto_char_by_index(&mut self, char_index: usize) {
        if self.state.goto_char_by_index(char_index) {
            self.modified_since_last_tick = true;
            self.is_dirty = true;
        }
    }
    fn goto_line(&mut self, line_number: usize) {
        if self.state.goto_row(line_number) {
            self.pending_scroll_row = Some(self.state.cursor.row);
            self.pending_scroll_centered = true;
            self.modified_since_last_tick = true;
            self.is_dirty = true;
        }
    }
    fn set_supports_undo(&mut self, supports_undo: bool) {
        self.supports_undo = supports_undo;
    }
    fn get_state(&self) -> TheTextEditState {
        self.state.clone()
    }
    fn set_state(&mut self, state: TheTextEditState) {
        self.state = state;
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
}

impl TheTextAreaEdit {
    fn emit_value_changed(&mut self, ctx: &mut TheContext) {
        ctx.ui.send_widget_value_changed(self.id(), self.value());
        self.modified_since_last_return = false;
    }

    fn statusbar_text(&self) -> String {
        let mut text = format!(
            "Ln {}, Col {}",
            self.state.cursor.row + 1,
            self.state.cursor.column + 1
        );
        if let Some(hl) = &self.renderer.highlighter {
            text.push_str(&format!(" {}", hl.syntax()));
        }
        text
    }
}
