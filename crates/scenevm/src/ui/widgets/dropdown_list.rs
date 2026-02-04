use uuid::Uuid;
use vek::Vec4;

use crate::ui::{
    drawable::Drawable,
    event::{UiAction, UiEvent, UiEventKind, UiEventOutcome},
    workspace::{UiView, ViewContext},
};

/// Style for the dropdown list widget
#[derive(Debug, Clone)]
pub struct DropdownListStyle {
    pub rect: [f32; 4],          // x, y, w, h for the button
    pub fill: Vec4<f32>,         // Background color
    pub border: Vec4<f32>,       // Border color
    pub hover_fill: Vec4<f32>,   // Hover background color
    pub panel_fill: Vec4<f32>,   // Background for the open dropdown panel
    pub panel_border: Vec4<f32>, // Border for the open dropdown panel
    pub text_color: Vec4<f32>,   // Text color
    pub text_size: f32,          // Font size
    pub radius_px: f32,          // Corner radius
    pub border_px: f32,          // Border width
    pub layer: i32,
    pub item_height: f32,         // Height of each dropdown item
    pub max_visible_items: usize, // Maximum items to show before scrolling
}

impl Default for DropdownListStyle {
    fn default() -> Self {
        Self {
            rect: [10.0, 10.0, 200.0, 40.0],
            fill: Vec4::new(0.15, 0.15, 0.18, 1.0),
            border: Vec4::new(0.25, 0.25, 0.28, 1.0),
            hover_fill: Vec4::new(0.25, 0.25, 0.28, 1.0),
            panel_fill: Vec4::new(0.1, 0.1, 0.12, 1.0),
            panel_border: Vec4::new(0.35, 0.35, 0.4, 1.0),
            text_color: Vec4::new(0.9, 0.9, 0.95, 1.0),
            text_size: 14.0,
            radius_px: 4.0,
            border_px: 1.0,
            layer: 10,
            item_height: 36.0,
            max_visible_items: 8,
        }
    }
}

/// A dropdown list that shows options when clicked
#[derive(Debug, Clone)]
pub struct DropdownList {
    pub id: String,
    pub name: String, // Name sent in events
    pub style: DropdownListStyle,
    pub items: Vec<String>,      // List of items
    pub selected_index: usize,   // Currently selected item
    pub is_open: bool,           // Whether dropdown is open
    hover_index: Option<usize>,  // Hovered item in dropdown
    hover_button: bool,          // Whether button is hovered
    active_pointer: Option<u32>, // Active pointer ID
}

impl DropdownList {
    pub fn new(name: impl Into<String>, style: DropdownListStyle) -> Self {
        Self {
            id: String::new(),
            name: name.into(),
            style,
            items: Vec::new(),
            selected_index: 0,
            is_open: false,
            hover_index: None,
            hover_button: false,
            active_pointer: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        if index < self.items.len() {
            self.selected_index = index;
        }
        self
    }

    pub fn set_selected(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = index;
        }
    }

    pub fn get_selected(&self) -> usize {
        self.selected_index
    }

    pub fn get_selected_text(&self) -> Option<&str> {
        self.items.get(self.selected_index).map(|s| s.as_str())
    }

    /// Get the rect for the dropdown panel
    fn dropdown_rect(&self) -> [f32; 4] {
        let [x, y, w, h] = self.style.rect;
        let visible_items = self.items.len().min(self.style.max_visible_items);
        let dropdown_height = visible_items as f32 * self.style.item_height;
        [x, y + h, w, dropdown_height]
    }

    /// Check if a position is inside the button
    fn hit_button(&self, pos: [f32; 2]) -> bool {
        let [x, y, w, h] = self.style.rect;
        pos[0] >= x && pos[0] <= x + w && pos[1] >= y && pos[1] <= y + h
    }

    /// Check if a position is inside the dropdown panel
    fn hit_dropdown(&self, pos: [f32; 2]) -> bool {
        if !self.is_open {
            return false;
        }
        let [x, y, w, h] = self.dropdown_rect();
        pos[0] >= x && pos[0] <= x + w && pos[1] >= y && pos[1] <= y + h
    }

    /// Find which item in the dropdown is at the position
    fn find_item_at(&self, pos: [f32; 2]) -> Option<usize> {
        if !self.hit_dropdown(pos) {
            return None;
        }
        let [_x, y, _w, _h] = self.dropdown_rect();
        let relative_y = pos[1] - y;
        let index = (relative_y / self.style.item_height) as usize;
        if index < self.items.len() {
            Some(index)
        } else {
            None
        }
    }
}

impl UiView for DropdownList {
    fn build(&mut self, ctx: &mut ViewContext) {
        let [x, y, w, h] = self.style.rect;

        // Draw button background
        let button_fill = if self.hover_button {
            self.style.hover_fill
        } else {
            self.style.fill
        };

        ctx.push(Drawable::Rect {
            id: Uuid::new_v4(),
            rect: self.style.rect,
            fill: button_fill,
            border: self.style.border,
            radius_px: self.style.radius_px,
            border_px: self.style.border_px,
            layer: self.style.layer,
        });

        // Draw selected item text
        if let Some(selected_text) = self.get_selected_text() {
            let text_x = x + 8.0;
            let text_y = y + (h - self.style.text_size) * 0.5;

            ctx.push(Drawable::Text {
                id: Uuid::new_v4(),
                text: selected_text.to_string(),
                origin: [text_x, text_y],
                px_size: self.style.text_size,
                color: self.style.text_color,
                layer: self.style.layer + 1,
            });
        }

        // Draw dropdown arrow
        let arrow_x = x + w - 20.0;
        let arrow_y = y + h * 0.5;
        let arrow = if self.is_open { "▲" } else { "▼" };

        ctx.push(Drawable::Text {
            id: Uuid::new_v4(),
            text: arrow.to_string(),
            origin: [arrow_x, arrow_y - self.style.text_size * 0.5],
            px_size: self.style.text_size * 0.8,
            color: self.style.text_color,
            layer: self.style.layer + 1,
        });

        // Draw dropdown panel if open
        if self.is_open {
            let [dx, dy, dw, dh] = self.dropdown_rect();

            // Draw dropdown background
            ctx.push(Drawable::Rect {
                id: Uuid::new_v4(),
                rect: [dx, dy, dw, dh],
                fill: self.style.panel_fill,
                border: self.style.panel_border,
                radius_px: self.style.radius_px,
                border_px: self.style.border_px,
                layer: self.style.layer + 10, // Higher layer so it appears on top
            });

            // Draw each item
            for (i, item) in self.items.iter().enumerate() {
                let item_y = dy + (i as f32 * self.style.item_height);
                let item_rect = [dx, item_y, dw, self.style.item_height];

                // Highlight hovered or selected item
                let is_hovered = self.hover_index == Some(i);
                let is_selected = i == self.selected_index;

                if is_hovered || is_selected {
                    let highlight_color = if is_hovered {
                        self.style.hover_fill
                    } else {
                        Vec4::new(
                            self.style.fill.x * 1.3,
                            self.style.fill.y * 1.3,
                            self.style.fill.z * 1.3,
                            self.style.fill.w,
                        )
                    };

                    ctx.push(Drawable::Rect {
                        id: Uuid::new_v4(),
                        rect: item_rect,
                        fill: highlight_color,
                        border: Vec4::new(0.0, 0.0, 0.0, 0.0),
                        radius_px: 0.0,
                        border_px: 0.0,
                        layer: self.style.layer + 11,
                    });
                }

                // Draw item text
                let text_x = dx + 8.0;
                let text_y = item_y + (self.style.item_height - self.style.text_size) * 0.5;

                ctx.push(Drawable::Text {
                    id: Uuid::new_v4(),
                    text: item.clone(),
                    origin: [text_x, text_y],
                    px_size: self.style.text_size,
                    color: self.style.text_color,
                    layer: self.style.layer + 12,
                });
            }
        }
    }

    fn handle_event(&mut self, evt: &UiEvent) -> UiEventOutcome {
        match evt.kind {
            UiEventKind::PointerDown => {
                if self.hit_button(evt.pos) {
                    self.active_pointer = Some(evt.pointer_id);
                    self.is_open = !self.is_open;
                    return UiEventOutcome::dirty();
                }
            }
            UiEventKind::PointerUp => {
                if self.active_pointer == Some(evt.pointer_id) {
                    self.active_pointer = None;

                    // Check if we clicked an item in the dropdown
                    if let Some(index) = self.find_item_at(evt.pos) {
                        if index != self.selected_index {
                            self.selected_index = index;
                            self.is_open = false;
                            let mut outcome = UiEventOutcome::dirty();
                            outcome.merge(UiEventOutcome::with_action(UiAction::DropdownChanged(
                                self.name.clone(),
                                index,
                            )));
                            return outcome;
                        } else {
                            // Clicked same item, just close
                            self.is_open = false;
                            return UiEventOutcome::dirty();
                        }
                    }

                    // Close dropdown if released anywhere (outside or on button)
                    if self.is_open {
                        self.is_open = false;
                        return UiEventOutcome::dirty();
                    }
                }
            }
            UiEventKind::PointerMove => {
                let new_hover_button = self.hit_button(evt.pos);
                let new_hover_index = if self.is_open {
                    self.find_item_at(evt.pos)
                } else {
                    None
                };

                if new_hover_button != self.hover_button || new_hover_index != self.hover_index {
                    self.hover_button = new_hover_button;
                    self.hover_index = new_hover_index;
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
