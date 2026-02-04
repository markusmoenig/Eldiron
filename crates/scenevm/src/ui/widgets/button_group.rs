use uuid::Uuid;
use vek::Vec4;

use crate::ui::{
    drawable::Drawable,
    event::{UiAction, UiEvent, UiEventKind, UiEventOutcome},
    workspace::{UiView, ViewContext},
};

/// Orientation for the button group layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonGroupOrientation {
    Horizontal,
    Vertical,
}

/// A group of toggle buttons where only one can be active at a time
#[derive(Debug, Clone)]
pub struct ButtonGroupStyle {
    pub rect: [f32; 4],           // x, y, w, h for the entire group
    pub button_width: f32,        // Width of each button
    pub button_height: f32,       // Height of each button
    pub spacing: f32,             // Space between buttons
    pub fill: Vec4<f32>,          // Normal button fill
    pub border: Vec4<f32>,        // Normal button border
    pub active_fill: Vec4<f32>,   // Active button fill
    pub active_border: Vec4<f32>, // Active button border
    pub radius_px: f32,           // Corner radius
    pub border_px: f32,           // Border width
    pub layer: i32,
    pub text_color: Vec4<f32>,    // Text label color (from theme)
    pub text_bg_color: Vec4<f32>, // Text background color (from theme)
}

impl Default for ButtonGroupStyle {
    fn default() -> Self {
        Self {
            rect: [10.0, 10.0, 300.0, 44.0],
            button_width: 60.0,
            button_height: 40.0,
            spacing: 4.0,
            fill: Vec4::new(0.15, 0.15, 0.18, 1.0),
            border: Vec4::new(0.25, 0.25, 0.28, 1.0),
            active_fill: Vec4::new(0.3, 0.4, 0.5, 1.0),
            active_border: Vec4::new(0.4, 0.5, 0.6, 1.0),
            radius_px: 4.0,
            border_px: 1.0,
            layer: 10,
            text_color: Vec4::new(0.9, 0.9, 0.95, 1.0),
            text_bg_color: Vec4::new(0.0, 0.0, 0.0, 0.85),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonGroup {
    pub id: String,
    pub name: String, // Name sent in events
    pub style: ButtonGroupStyle,
    pub orientation: ButtonGroupOrientation,
    pub labels: Vec<String>, // Button text labels (optional if textures are used)
    pub textures: Vec<Option<Uuid>>, // Optional texture tile_id for each button
    pub text_labels: Vec<String>, // Optional text below icons
    pub text_color: Vec4<f32>, // Color for text labels
    pub text_size: f32,      // Font size for text labels
    pub text_gap: f32,       // Gap between button and text label
    pub text_background: bool, // Whether to draw semi-transparent background behind text labels
    pub text_background_color: Vec4<f32>, // Background color for text labels
    pub text_background_padding: f32, // Padding around text background
    pub active_index: usize, // Currently active button (0-based)
    active_pointer: Option<u32>,
    hover_index: Option<usize>,
    /// Original relative position (used when ButtonGroup is a child of a popup)
    pub original_rect: Option<[f32; 4]>,
}

impl ButtonGroup {
    pub fn new(name: impl Into<String>, style: ButtonGroupStyle) -> Self {
        let text_color = style.text_color;
        let text_background_color = style.text_bg_color;
        Self {
            id: String::new(),
            name: name.into(),
            style,
            orientation: ButtonGroupOrientation::Horizontal,
            labels: Vec::new(),
            textures: Vec::new(),
            text_labels: Vec::new(),
            text_color,
            text_size: 11.0,
            text_gap: 2.0,
            text_background: true,
            text_background_color,
            text_background_padding: 3.0,
            active_index: 0,
            active_pointer: None,
            hover_index: None,
            original_rect: None,
        }
    }

    pub fn with_orientation(mut self, orientation: ButtonGroupOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn with_textures(mut self, textures: Vec<Option<Uuid>>) -> Self {
        self.textures = textures;
        self
    }

    pub fn with_text_labels(mut self, text_labels: Vec<String>) -> Self {
        self.text_labels = text_labels;
        self
    }

    pub fn with_text_color(mut self, color: Vec4<f32>) -> Self {
        self.text_color = color;
        self
    }

    pub fn with_text_size(mut self, size: f32) -> Self {
        self.text_size = size;
        self
    }

    pub fn with_text_gap(mut self, gap: f32) -> Self {
        self.text_gap = gap;
        self
    }

    pub fn with_text_background(mut self, enabled: bool) -> Self {
        self.text_background = enabled;
        self
    }

    pub fn with_text_background_color(mut self, color: Vec4<f32>) -> Self {
        self.text_background_color = color;
        self
    }

    pub fn with_text_background_padding(mut self, padding: f32) -> Self {
        self.text_background_padding = padding;
        self
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.style.spacing = spacing;
        self
    }

    pub fn add_label(&mut self, label: impl Into<String>) {
        self.labels.push(label.into());
    }

    pub fn add_texture(&mut self, tile_id: Option<Uuid>) {
        self.textures.push(tile_id);
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.button_count() {
            self.active_index = index;
        }
    }

    /// Get the number of buttons (max of labels and textures count)
    fn button_count(&self) -> usize {
        self.labels.len().max(self.textures.len())
    }

    pub fn get_active(&self) -> usize {
        self.active_index
    }

    /// Calculate the total width needed for all buttons
    pub fn calculate_width(&self) -> f32 {
        match self.orientation {
            ButtonGroupOrientation::Horizontal => {
                let button_count = self.button_count() as f32;
                if button_count == 0.0 {
                    return 0.0;
                }
                (button_count * self.style.button_width)
                    + ((button_count - 1.0) * self.style.spacing)
            }
            ButtonGroupOrientation::Vertical => self.style.button_width,
        }
    }

    pub fn calculate_height(&self) -> f32 {
        match self.orientation {
            ButtonGroupOrientation::Horizontal => self.style.button_height,
            ButtonGroupOrientation::Vertical => {
                let button_count = self.button_count() as f32;
                if button_count == 0.0 {
                    return 0.0;
                }
                (button_count * self.style.button_height)
                    + ((button_count - 1.0) * self.style.spacing)
            }
        }
    }

    /// Get the rect for a button at the given index
    fn button_rect(&self, index: usize) -> [f32; 4] {
        let [x, y, _, _] = self.style.rect;

        match self.orientation {
            ButtonGroupOrientation::Horizontal => {
                let btn_x = x + (index as f32 * (self.style.button_width + self.style.spacing));
                let btn_y = y;
                [
                    btn_x,
                    btn_y,
                    self.style.button_width,
                    self.style.button_height,
                ]
            }
            ButtonGroupOrientation::Vertical => {
                let btn_x = x;
                let btn_y = y + (index as f32 * (self.style.button_height + self.style.spacing));
                [
                    btn_x,
                    btn_y,
                    self.style.button_width,
                    self.style.button_height,
                ]
            }
        }
    }

    /// Check if a position is inside a specific button
    fn hit_button(&self, pos: [f32; 2], index: usize) -> bool {
        if index >= self.button_count() {
            return false;
        }
        let [x, y, w, h] = self.button_rect(index);
        pos[0] >= x && pos[0] <= x + w && pos[1] >= y && pos[1] <= y + h
    }

    /// Find which button (if any) contains the position
    fn find_button_at(&self, pos: [f32; 2]) -> Option<usize> {
        for i in 0..self.button_count() {
            if self.hit_button(pos, i) {
                return Some(i);
            }
        }
        None
    }
}

impl UiView for ButtonGroup {
    fn build(&mut self, ctx: &mut ViewContext) {
        let button_count = self.button_count();

        // Draw each button
        for index in 0..button_count {
            let is_active = index == self.active_index;
            let is_hover = self.hover_index == Some(index);
            let is_pressed = self.active_pointer.is_some() && is_hover;

            let (fill, border) = if is_pressed {
                // Pressed state - even darker/more distinct
                let pressed_fill = Vec4::new(
                    self.style.active_fill.x * 0.8,
                    self.style.active_fill.y * 0.8,
                    self.style.active_fill.z * 0.8,
                    self.style.active_fill.w,
                );
                (pressed_fill, self.style.active_border)
            } else if is_active {
                (self.style.active_fill, self.style.active_border)
            } else if is_hover {
                // Hover state - add brightness instead of multiply
                let hover_fill = Vec4::new(
                    (self.style.fill.x + 0.15).min(1.0),
                    (self.style.fill.y + 0.15).min(1.0),
                    (self.style.fill.z + 0.15).min(1.0),
                    self.style.fill.w,
                );
                (hover_fill, self.style.border)
            } else {
                (self.style.fill, self.style.border)
            };

            let button_rect = self.button_rect(index);
            let [btn_x, btn_y, btn_w, btn_h] = button_rect;

            // Draw button background
            ctx.push(Drawable::Rect {
                id: Uuid::new_v4(),
                rect: button_rect,
                fill,
                border,
                radius_px: self.style.radius_px,
                border_px: self.style.border_px,
                layer: self.style.layer,
            });

            // Draw texture if available
            if let Some(Some(tile_id)) = self.textures.get(index) {
                // Add small padding inside button for texture
                let padding = 4.0;
                let tex_rect = [
                    btn_x + padding,
                    btn_y + padding,
                    btn_w - padding * 2.0,
                    btn_h - padding * 2.0,
                ];

                ctx.push(Drawable::Quad {
                    id: Uuid::new_v4(),
                    tile_id: *tile_id,
                    rect: tex_rect,
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    layer: self.style.layer + 1,
                    tint: Vec4::new(1.0, 1.0, 1.0, 1.0),
                });
            }

            // Draw label text (centered) if available
            if let Some(label) = self.labels.get(index) {
                let text_size = 14.0;

                // Rough approximation for text width (better than nothing)
                let approx_text_width = label.len() as f32 * text_size * 0.6;
                let text_x = btn_x + (btn_w - approx_text_width) * 0.5;
                let text_y = btn_y + (btn_h - text_size) * 0.5;

                ctx.push(Drawable::Text {
                    id: Uuid::new_v4(),
                    text: label.clone(),
                    origin: [text_x, text_y],
                    px_size: text_size,
                    color: self.style.text_color,
                    layer: self.style.layer + 2,
                });
            }

            // Draw text label below button if available
            if let Some(text_label) = self.text_labels.get(index) {
                // Draw optional semi-transparent background behind text
                if self.text_background {
                    // Background starts right after the button with text_gap, moved down by 1px
                    let bg_x = btn_x;
                    let bg_y = btn_y + btn_h + self.text_gap + 1.0;
                    let bg_w = btn_w;
                    let bg_h = self.text_size + self.text_background_padding * 2.0;

                    ctx.push(Drawable::Rect {
                        id: Uuid::new_v4(),
                        rect: [bg_x, bg_y, bg_w, bg_h],
                        fill: self.text_background_color,
                        border: Vec4::new(0.0, 0.0, 0.0, 0.0),
                        radius_px: 4.0,
                        border_px: 0.0,
                        layer: self.style.layer + 2,
                    });
                }

                // Rough approximation for text width
                let approx_text_width = text_label.len() as f32 * self.text_size * 0.6;
                let text_x = btn_x + (btn_w - approx_text_width) * 0.5;
                // Position text below the button with configurable gap, centered vertically in background
                let text_y = btn_y + btn_h + self.text_gap + self.text_background_padding;

                ctx.push(Drawable::Text {
                    id: Uuid::new_v4(),
                    text: text_label.clone(),
                    origin: [text_x, text_y],
                    px_size: self.text_size,
                    color: self.text_color,
                    layer: self.style.layer + 3,
                });
            }
        }
    }

    fn handle_event(&mut self, evt: &UiEvent) -> UiEventOutcome {
        match evt.kind {
            UiEventKind::PointerDown => {
                if self.find_button_at(evt.pos).is_some() {
                    self.active_pointer = Some(evt.pointer_id);
                    return UiEventOutcome::dirty();
                }
            }
            UiEventKind::PointerUp => {
                if self.active_pointer == Some(evt.pointer_id) {
                    self.active_pointer = None;
                    if let Some(index) = self.find_button_at(evt.pos) {
                        if index != self.active_index {
                            self.active_index = index;
                            let mut outcome = UiEventOutcome::dirty();
                            outcome.merge(UiEventOutcome::with_action(
                                UiAction::ButtonGroupChanged(self.name.clone(), index),
                            ));
                            return outcome;
                        }
                    }
                    return UiEventOutcome::dirty();
                }
            }
            UiEventKind::PointerMove => {
                let new_hover = self.find_button_at(evt.pos);
                if new_hover != self.hover_index {
                    self.hover_index = new_hover;
                    return UiEventOutcome::dirty();
                }
            }
            _ => {}
        }
        UiEventOutcome::none()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn view_id(&self) -> &str {
        &self.id
    }
}
