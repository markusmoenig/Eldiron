#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;

use num_traits::ToPrimitive;
use web_time::Instant;

use crate::prelude::*;

use super::thetextedit::{TheCursor, TheTextEditState, TheTextRenderer};

#[derive(Debug, PartialEq)]
pub enum TheTextLineEditContentType {
    Unknown,
    Text,
    Float,
    Int,
}

pub struct TheTextLineEdit {
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

    // Interaction
    drag_start_index: usize,
    last_mouse_down_coord: Vec2<i32>,
    last_mouse_down_time: Instant,

    // Modifiers
    modifier_ctrl: bool,
    modifier_logo: bool,
    modifier_shift: bool,

    // Range
    range: Option<TheValue>,
    original: String,

    is_dirty: bool,
    embedded: bool,
    parent_id: Option<TheId>,
    cursor_icon: Option<TheCursorIcon>,

    layout_id: Option<TheId>,
    continuous: bool,

    content_type: TheTextLineEditContentType,
    info_text: Option<String>,

    palette: Option<ThePalette>,

    undo_stack: TheUndoStack,
}

impl TheWidget for TheTextLineEdit {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_width(150);
        limiter.set_max_height(20);

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

            drag_start_index: 0,
            last_mouse_down_coord: Vec2::zero(),
            last_mouse_down_time: Instant::now(),

            modifier_ctrl: false,
            modifier_logo: false,
            modifier_shift: false,

            range: None,
            original: "".to_string(),

            is_dirty: false,
            embedded: false,
            parent_id: None,
            cursor_icon: Some(TheCursorIcon::Text),

            layout_id: None,
            continuous: false,

            content_type: TheTextLineEditContentType::Unknown,
            info_text: None,

            palette: None,
            undo_stack: TheUndoStack::default(),
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

    fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
        self.limiter_mut().set_max_height(16);
    }

    fn set_parent_id(&mut self, parent_id: TheId) {
        self.parent_id = Some(parent_id);
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        self.cursor_icon
    }

    fn set_cursor_icon(&mut self, icon: Option<TheCursorIcon>) {
        self.cursor_icon = icon;
    }

    fn parent_id(&self) -> Option<&TheId> {
        self.parent_id.as_ref()
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
        true
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if self.is_disabled {
            return false;
        }

        let mut redraw = false;
        match event {
            TheEvent::ModifierChanged(shift, ctrl, _alt, logo) => {
                self.modifier_ctrl = *ctrl;
                self.modifier_logo = *logo;
                self.modifier_shift = *shift;
            }
            TheEvent::GainedFocus(_id) => {
                // Set text cursor when gaining focus
                self.cursor_icon = Some(TheCursorIcon::Text);
            }

            TheEvent::MouseDown(coord) => {
                if !self.state.is_empty() {
                    let (cursor_row, cursor_column) = self
                        .state
                        .find_row_col_of_index(self.renderer.find_cursor_index(coord));
                    self.state
                        .set_cursor(TheCursor::new(cursor_row, cursor_column));
                    self.drag_start_index = self.state.find_cursor_index();

                    if self.is_range() && self.state.selection.is_none() {
                        self.state.select_row();
                    } else {
                        let is_double_click = self.last_mouse_down_time.elapsed().as_millis() < 500
                            && self.last_mouse_down_coord == *coord;

                        if !self.state.selection.is_none() {
                            if is_double_click {
                                if self.state.is_row_all_selected(self.state.cursor.row) {
                                    self.state.reset_selection();
                                } else {
                                    self.state.select_row();
                                }
                            } else {
                                self.state.reset_selection();
                            }
                        } else if is_double_click {
                            // Select a word, a whole row or a spacing etc.
                            self.state.quick_select();
                        }
                    }
                }

                if !self.embedded {
                    ctx.ui.set_focus(self.id());
                }
                self.is_dirty = true;
                redraw = true;
                if self.is_range() {
                    self.original.clone_from(&self.state.to_text());
                }

                self.last_mouse_down_coord = *coord;
                self.last_mouse_down_time = Instant::now();
            }
            TheEvent::MouseDragged(coord) => {
                self.is_dirty = true;
                redraw = true;

                // If we have an i32 or f32 range, we treat the text line edit as a slider
                if let Some(range) = &self.range {
                    if let Some(range_f32) = range.to_range_f32() {
                        let d = (range_f32.end() - range_f32.start()).abs()
                            * (coord.x.to_f32().unwrap() / (self.dim.width).to_f32().unwrap())
                                .clamp(0.0, 1.0);
                        let v = *range_f32.start() + d;
                        self.state.set_text(format!("{:.3}", v));
                        self.modified_since_last_tick = true;
                    } else if let Some(range_i32) = range.to_range_i32() {
                        let range_diff = range_i32.end() - range_i32.start();
                        let d = (coord.x * range_diff) / (self.dim.width);
                        let v =
                            (*range_i32.start() + d).clamp(*range_i32.start(), *range_i32.end());
                        self.state.set_text(v.to_string());
                        self.modified_since_last_tick = true;
                    }
                    if self.continuous {
                        if let Some(range_f32) = range.to_range_f32() {
                            if let Ok(v) = self.state.to_text().parse::<f32>() {
                                ctx.ui.send_widget_value_changed(
                                    self.id(),
                                    TheValue::FloatRange(v, range_f32),
                                );
                            }
                        } else if let Some(range_i32) = range.to_range_i32() {
                            if let Ok(v) = self.state.to_text().parse::<i32>() {
                                ctx.ui.send_widget_value_changed(
                                    self.id(),
                                    TheValue::IntRange(v, range_i32),
                                );
                            }
                        }
                    }
                } else if !self.state.is_empty() {
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

                    // Always update cursor position during drag for text selection
                    let (cursor_row, cursor_column) = self
                        .state
                        .find_row_col_of_index(self.renderer.find_cursor_index(coord));
                    self.state
                        .set_cursor(TheCursor::new(cursor_row, cursor_column));

                    let cursor_index = self.state.find_cursor_index();
                    if self.drag_start_index != cursor_index {
                        let start = self.drag_start_index.min(cursor_index);
                        let end = self.drag_start_index.max(cursor_index);
                        self.state.select(start, end);
                    }
                    // Don't reset selection when cursor hasn't moved - this preserves the initial selection
                }
            }
            TheEvent::MouseUp(_coord) => {
                self.drag_start_index = 0;
                // Send an event if in slider mode and not continuous
                if self.is_range() && !self.continuous && self.state.to_text() != self.original {
                    if let Some(range) = &self.range {
                        if let Some(range_f32) = range.to_range_f32() {
                            if let Ok(v) = self.state.to_text().parse::<f32>() {
                                ctx.ui.send_widget_value_changed(
                                    self.id(),
                                    TheValue::FloatRange(v, range_f32),
                                );
                            }
                        } else if let Some(range_i32) = range.to_range_i32() {
                            if let Ok(v) = self.state.to_text().parse::<i32>() {
                                ctx.ui.send_widget_value_changed(
                                    self.id(),
                                    TheValue::IntRange(v, range_i32),
                                );
                            }
                        }
                    }
                }
            }
            TheEvent::MouseWheel(delta) => {
                if self
                    .renderer
                    .scroll(&Vec2::new(delta.x / 4, delta.y / 4), true)
                {
                    redraw = true;
                }
            }
            TheEvent::KeyDown(key) => {
                let prev_state = self.state.save();

                if let Some(c) = key.to_char() {
                    if self.modifier_ctrl && c == 'a' {
                        self.state.select_all();
                        self.is_dirty = true;
                        redraw = true;
                    } else {
                        self.state.insert_char(c);
                        self.modified_since_last_tick = true;
                        self.is_dirty = true;
                        redraw = true;

                        if self.continuous {
                            if let Some(layout_id) = &self.layout_id {
                                ctx.ui.send(TheEvent::RedirectWidgetValueToLayout(
                                    layout_id.clone(),
                                    self.id().clone(),
                                    self.value(),
                                ));
                            } else {
                                ctx.ui.send_widget_value_changed(self.id(), self.value());
                            }
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
            TheEvent::KeyCodeDown(key_code) => {
                let prev_state = self.state.save();
                if let Some(key) = key_code.to_key_code() {
                    match key {
                        TheKeyCode::Return => {
                            if self.modified_since_last_return {
                                if let Some(layout_id) = &self.layout_id {
                                    ctx.ui.send(TheEvent::RedirectWidgetValueToLayout(
                                        layout_id.clone(),
                                        self.id().clone(),
                                        self.value(),
                                    ));
                                } else {
                                    ctx.ui.send_widget_value_changed(self.id(), self.value());
                                }
                                if !self.embedded {
                                    ctx.ui.clear_focus();
                                }
                                redraw = true;
                                self.is_dirty = true;
                                self.modified_since_last_return = false;
                                if self.is_range() {
                                    self.original = self.state.to_text();
                                }
                            }
                        }
                        TheKeyCode::Delete => {
                            if self.state.delete_text() {
                                self.modified_since_last_tick = true;
                                self.is_dirty = true;
                                redraw = true;
                            }
                        }
                        TheKeyCode::Right => {
                            if self.modifier_ctrl || self.modifier_logo {
                                if self.state.quick_move_cursor_right()
                                    || self.state.move_cursor_right()
                                {
                                    self.renderer.scroll_to_cursor(
                                        self.state.find_cursor_index(),
                                        self.state.cursor.row,
                                    );
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

                                    self.renderer.scroll_to_cursor(
                                        self.state.find_cursor_index(),
                                        self.state.cursor.row,
                                    );
                                    self.is_dirty = true;
                                    redraw = true;
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
                                    self.renderer.scroll_to_cursor(
                                        self.state.find_cursor_index(),
                                        self.state.cursor.row,
                                    );
                                    self.is_dirty = true;
                                    redraw = true;
                                }
                            }
                        }
                        TheKeyCode::Left => {
                            if self.modifier_ctrl | self.modifier_logo {
                                if self.state.quick_move_cursor_left()
                                    || self.state.move_cursor_left()
                                {
                                    self.renderer.scroll_to_cursor(
                                        self.state.find_cursor_index(),
                                        self.state.cursor.row,
                                    );
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

                                    self.renderer.scroll_to_cursor(
                                        self.state.find_cursor_index(),
                                        self.state.cursor.row,
                                    );
                                    self.is_dirty = true;
                                    redraw = true;
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
                                    self.renderer.scroll_to_cursor(
                                        self.state.find_cursor_index(),
                                        self.state.cursor.row,
                                    );
                                    self.is_dirty = true;
                                    redraw = true;
                                }
                            }
                        }
                        TheKeyCode::Up => {
                            if let Some(range) = &self.range {
                                if let Some(range_f32) = range.to_range_f32() {
                                    if let Ok(v) = self.state.to_text().parse::<f32>() {
                                        if range_f32.contains(&(v + 1.0)) {
                                            ctx.ui.send_widget_value_changed(
                                                self.id(),
                                                TheValue::FloatRange(v + 1.0, range_f32),
                                            );
                                            self.set_value(TheValue::Float(v + 1.0));
                                        }
                                    }
                                } else if let Some(range_i32) = range.to_range_i32() {
                                    if let Ok(v) = self.state.to_text().parse::<i32>() {
                                        if range_i32.contains(&(v + 1)) {
                                            ctx.ui.send_widget_value_changed(
                                                self.id(),
                                                TheValue::IntRange(v + 1, range_i32),
                                            );
                                            self.set_value(TheValue::Int(v + 1));
                                        }
                                    }
                                }
                            }
                        }
                        TheKeyCode::Down => {
                            if let Some(range) = &self.range {
                                if let Some(range_f32) = range.to_range_f32() {
                                    if let Ok(v) = self.state.to_text().parse::<f32>() {
                                        if range_f32.contains(&(v - 1.0)) {
                                            ctx.ui.send_widget_value_changed(
                                                self.id(),
                                                TheValue::FloatRange(v - 1.0, range_f32),
                                            );
                                            self.set_value(TheValue::Float(v - 1.0));
                                        }
                                    }
                                } else if let Some(range_i32) = range.to_range_i32() {
                                    if let Ok(v) = self.state.to_text().parse::<i32>() {
                                        if range_i32.contains(&(v - 1)) {
                                            ctx.ui.send_widget_value_changed(
                                                self.id(),
                                                TheValue::IntRange(v - 1, range_i32),
                                            );
                                            self.set_value(TheValue::Int(v - 1));
                                        }
                                    }
                                }
                            }
                        }
                        TheKeyCode::Space => {
                            self.state.insert_text(" ".to_owned());
                            self.modified_since_last_tick = true;
                            self.is_dirty = true;
                            redraw = true;
                        }
                        _ => {}
                    }

                    if self.continuous {
                        if let Some(layout_id) = &self.layout_id {
                            ctx.ui.send(TheEvent::RedirectWidgetValueToLayout(
                                layout_id.clone(),
                                self.id().clone(),
                                self.value(),
                            ));
                        } else {
                            ctx.ui.send_widget_value_changed(self.id(), self.value());
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
            TheEvent::LostFocus(_id) => {
                // if self.modified_since_last_return {
                //     if let Some(layout_id) = &self.layout_id {
                //         ctx.ui.send(TheEvent::RedirectWidgetValueToLayout(
                //             layout_id.clone(),
                //             self.id().clone(),
                //             self.value(),
                //         ));
                //     } else {
                //         ctx.ui.send_widget_value_changed(self.id(), self.value());
                //     }
                // }
                // Reset cursor when losing focus
                self.cursor_icon = Some(TheCursorIcon::Text);
            }
            TheEvent::Hover(_coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                }
                // Set text cursor when hovered (only if not already focused)
                if !ctx.ui.has_focus(self.id()) {
                    self.cursor_icon = Some(TheCursorIcon::Text);
                }
            }
            TheEvent::Undo => {
                if self.undo_stack.has_undo() {
                    let (_id, state) = self.undo_stack.undo();
                    self.state = TheTextEditState::load(&state);
                    self.modified_since_last_tick = true;
                    self.is_dirty = true;
                    redraw = true;
                }
            }
            TheEvent::Redo => {
                if self.undo_stack.has_redo() {
                    let (_id, state) = self.undo_stack.redo();
                    self.state = TheTextEditState::load(&state);
                    self.modified_since_last_tick = true;
                    self.is_dirty = true;
                    redraw = true;
                }
            }
            TheEvent::Copy => {
                let text = self.state.copy_text();
                if !text.is_empty() {
                    redraw = true;
                    // update_status = true;

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
                        ctx.ui.send_widget_value_changed(self.id(), self.value());
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

                    let mut undo = TheUndo::new(TheId::named("Cut"));
                    undo.set_undo_data(prev_state);
                    undo.set_redo_data(self.state.save());
                    self.undo_stack.add(undo);

                    if self.continuous {
                        ctx.ui.send_widget_value_changed(self.id(), self.value());
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

                        if self.continuous {
                            ctx.ui.send_widget_value_changed(self.id(), self.value());
                        }

                        let mut undo = TheUndo::new(TheId::named("Cut"));
                        undo.set_undo_data(prev_state);
                        undo.set_redo_data(self.state.save());
                        self.undo_stack.add(undo);
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    fn value(&self) -> TheValue {
        if let Some(range) = &self.range {
            if let Some(range_f32) = range.to_range_f32() {
                if let Ok(value) = self.state.to_text().parse::<f32>() {
                    //if range_f32.contains(&value) {
                    return TheValue::FloatRange(value, range_f32);
                    //}
                }
                let original = self.original.clone();
                if let Ok(value) = original.parse::<f32>() {
                    if range_f32.contains(&value) {
                        return TheValue::Float(value);
                    }
                }
            } else if let Some(range_i32) = range.to_range_i32() {
                if let Ok(value) = self.state.to_text().parse::<i32>() {
                    // if range_i32.contains(&value) {
                    return TheValue::IntRange(value, range_i32);
                    // }
                }
                let original = self.original.clone();
                if let Ok(value) = original.parse::<i32>() {
                    if range_i32.contains(&value) {
                        return TheValue::Int(value);
                    }
                }
            }
        }
        if self.content_type == TheTextLineEditContentType::Float {
            if let Ok(value) = self.state.to_text().parse::<f32>() {
                return TheValue::Float(value);
            }
        }
        if self.content_type == TheTextLineEditContentType::Int {
            if let Ok(value) = self.state.to_text().parse::<i32>() {
                return TheValue::Int(value);
            }
        }
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
                self.content_type = TheTextLineEditContentType::Text;
                self.set_text(text);
            }
            TheValue::Int(v) => {
                self.set_text(v.to_string());
                self.content_type = TheTextLineEditContentType::Int;
            }
            TheValue::Float(v) => {
                self.set_text(v.to_string());
                self.content_type = TheTextLineEditContentType::Float;
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

        let should_relayout = self.modified_since_last_tick || self.renderer.row_count() == 0;

        if should_relayout {
            self.renderer
                .prepare(&self.state.to_text(), TheFontPreference::Default, &ctx.draw);
            self.reset_renderer_padding();
        }

        let mut shrinker = TheDimShrinker::zero();
        self.renderer.render_widget(
            &mut shrinker,
            self.is_disabled,
            self.embedded,
            true,
            !self.embedded,
            self,
            buffer,
            style,
            ctx,
            false,
        );

        if should_relayout {
            let visible_area = self.dim.to_buffer_shrunk_utuple(&shrinker);
            self.renderer.set_dim(
                visible_area.0,
                visible_area.1,
                visible_area.2,
                visible_area.3,
            );
            self.renderer
                .scroll_to_cursor(self.state.find_cursor_index(), self.state.cursor.row);
        }

        if self.is_range() && !self.is_disabled {
            shrinker.shrink_by(
                -self.renderer.padding.0.to_i32().unwrap(),
                -self.renderer.padding.1.to_i32().unwrap(),
                -self.renderer.padding.2.to_i32().unwrap(),
                -self.renderer.padding.3.to_i32().unwrap(),
            );
            let rect = self.dim.to_buffer_shrunk_utuple(&shrinker);
            let value = self.range.as_ref().and_then(|range| {
                if let Some(range_f32) = range.to_range_f32() {
                    if let Ok(value) = self.state.to_text().parse::<f32>() {
                        let normalized =
                            (value - range_f32.start()) / (range_f32.end() - range_f32.start());
                        return Some((normalized * rect.2.to_f32().unwrap()).to_usize());
                    }
                } else if let Some(range_i32) = range.to_range_i32() {
                    if let Ok(value) = self.state.to_text().parse::<i32>() {
                        let range_diff = range_i32.end() - range_i32.start();
                        let normalized =
                            (value - range_i32.start()) * rect.2.to_i32().unwrap() / range_diff;
                        return Some(normalized.to_usize());
                    }
                }
                None
            });

            // TODO: CHECK Option<Option<usize>>
            if let Some(Some(value)) = value {
                let pos = value.clamp(0, rect.2);
                let stride = buffer.stride();
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(rect.0, rect.1, pos, rect.3),
                    stride,
                    style.theme().color(TextEditRange),
                );
            }
        }

        // Never scroll vertically
        self.renderer.scroll_offset.y = 0;

        self.renderer.render_text(
            &self.state,
            if !self.embedded {
                ctx.ui.has_focus(self.id())
            } else {
                self.has_parent_focus(ctx)
            },
            false,
            buffer,
            style,
            TheFontPreference::Default,
            &ctx.draw,
        );

        if let Some(palette) = &self.palette {
            if let Some(value) = self.value().to_i32() {
                let stride = buffer.stride();
                shrinker.shrink_by(50, 0, 0, 0);
                let utuple: (usize, usize, usize, usize) =
                    self.dim.to_buffer_shrunk_utuple(&shrinker);

                if let Some(Some(color)) = palette.colors.get(value as usize) {
                    ctx.draw
                        .rect(buffer.pixels_mut(), &utuple, stride, &color.to_u8_array());
                }
            }
        }

        if let Some(info_text) = &self.info_text {
            let stride = buffer.stride();
            shrinker.shrink_by(0, 0, 5, 0);
            let utuple: (usize, usize, usize, usize) = self.dim.to_buffer_shrunk_utuple(&shrinker);

            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &utuple,
                stride,
                info_text,
                TheFontSettings {
                    size: 11.5,
                    ..Default::default()
                },
                style.theme().color(DefaultWidgetDarkBackground),
                TheHorizontalAlign::Right,
                TheVerticalAlign::Center,
            );
        }

        self.modified_since_last_return =
            self.modified_since_last_return || self.modified_since_last_tick;
        self.modified_since_last_tick = false;
        self.is_dirty = false;
    }

    fn as_text_line_edit(&mut self) -> Option<&mut dyn TheTextLineEditTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheTextLineEditTrait: TheWidget {
    fn text(&self) -> String;
    fn set_text(&mut self, text: String);
    fn set_info_text(&mut self, text: Option<String>);
    fn set_font_size(&mut self, font_size: f32);
    fn set_range(&mut self, range: TheValue);
    fn set_associated_layout(&mut self, id: TheId);
    fn set_continuous(&mut self, continuous: bool);
    fn set_palette(&mut self, palette: ThePalette);
}

impl TheTextLineEditTrait for TheTextLineEdit {
    fn text(&self) -> String {
        self.state.to_text()
    }
    fn set_text(&mut self, text: String) {
        self.state.reset();
        if let Some(range) = &self.range {
            if let Some(range) = range.to_range_f32() {
                let v = text.parse::<f32>().unwrap_or(*range.start());
                self.state.set_text(format!("{:.3}", v));
            } else if let Some(range) = range.to_range_i32() {
                let v = text.parse::<i32>().unwrap_or(*range.start());
                self.state.set_text(v.to_string());
            }
        } else {
            self.state.set_text(text);
        }
        self.content_type = TheTextLineEditContentType::Text;
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn set_info_text(&mut self, text: Option<String>) {
        self.info_text = text;
    }
    fn set_font_size(&mut self, font_size: f32) {
        self.renderer.set_font_size(font_size);
        self.modified_since_last_tick = true;
        self.is_dirty = true;
    }
    fn set_range(&mut self, range: TheValue) {
        if Some(range.clone()) != self.range {
            if let Some(range) = range.to_range_f32() {
                let v = self
                    .state
                    .to_text()
                    .parse::<f32>()
                    .unwrap_or(*range.start());
                self.state.set_text(format!("{:.3}", v));
                self.content_type = TheTextLineEditContentType::Float;
            } else if let Some(range) = range.to_range_i32() {
                let v = self
                    .state
                    .to_text()
                    .parse::<i32>()
                    .unwrap_or(*range.start());
                self.state.set_text(v.to_string());
                self.content_type = TheTextLineEditContentType::Int;
            }
            self.range = Some(range);
            self.is_dirty = true;
        }
    }
    fn set_associated_layout(&mut self, layout_id: TheId) {
        self.layout_id = Some(layout_id);
    }
    fn set_continuous(&mut self, continuous: bool) {
        self.continuous = continuous;
    }
    fn set_palette(&mut self, palette: ThePalette) {
        self.palette = Some(palette);
        self.is_dirty = true;
    }
}

impl TheTextLineEdit {
    fn is_range(&self) -> bool {
        self.range.is_some()
    }

    fn reset_renderer_padding(&mut self) {
        if self.dim.height == 0 || self.renderer.actual_size.y == 0 {
            return;
        }

        let padding = ((self.dim.height as f32 - self.renderer.font_size as f32) * 0.5) as i32
            - (self.renderer.actual_size.y as f32 - self.renderer.font_size) as i32;
        self.renderer.padding.1 = padding.max(0);
    }
}
