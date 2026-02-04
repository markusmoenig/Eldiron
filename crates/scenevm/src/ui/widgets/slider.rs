use uuid::Uuid;
use vek::Vec4;

use crate::ui::{
    drawable::Drawable,
    event::{UiAction, UiEvent, UiEventKind, UiEventOutcome},
    workspace::{UiView, ViewContext},
};

#[derive(Debug, Clone)]
pub struct SliderStyle {
    pub rect: [f32; 4], // x, y, w, h
    pub track_color: Vec4<f32>,
    pub fill_color: Vec4<f32>,
    pub thumb_color: Vec4<f32>,
    pub thumb_radius: f32,
    pub track_height: f32,
    pub layer: i32,
    pub value_color: Vec4<f32>, // Value text color (from theme)
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            rect: [10.0, 10.0, 200.0, 32.0],
            track_color: Vec4::new(0.2, 0.2, 0.22, 1.0),
            fill_color: Vec4::new(0.4, 0.5, 0.7, 1.0),
            thumb_color: Vec4::new(0.9, 0.9, 0.95, 1.0),
            thumb_radius: 12.0,
            track_height: 6.0,
            layer: 10,
            value_color: Vec4::new(0.6, 0.6, 0.65, 1.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Slider {
    pub id: String,
    render_id: Uuid, // For drawable tracking
    pub style: SliderStyle,
    pub value: f32, // 0.0 to 1.0
    pub min: f32,
    pub max: f32,
    dragging: bool,
    active_pointer: Option<u32>,
    original_value: f32,                   // Value when drag started (for undo)
    pub show_value: bool,                  // Whether to show value text
    pub value_color: Vec4<f32>,            // Color for value text
    pub value_size: f32,                   // Font size for value text
    pub value_precision: usize,            // Decimal places for value display
    pub thumb_roundness: f32, // Corner radius in pixels (defaults to thumb_radius for circle)
    pub value_labels: Option<Vec<String>>, // Custom labels for discrete values
}

impl Slider {
    pub fn new(style: SliderStyle, min: f32, max: f32) -> Self {
        let thumb_roundness = style.thumb_radius; // Default to circle
        let value_color = style.value_color;
        Self {
            id: String::new(),
            render_id: Uuid::new_v4(),
            style,
            value: 0.5,
            min,
            max,
            dragging: false,
            active_pointer: None,
            original_value: 0.5,
            show_value: false,
            value_color,
            value_size: 12.0,
            value_precision: 1,
            thumb_roundness,
            value_labels: None,
        }
    }

    pub fn with_show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    pub fn with_value_precision(mut self, precision: usize) -> Self {
        self.value_precision = precision;
        self
    }

    pub fn with_value_color(mut self, color: Vec4<f32>) -> Self {
        self.value_color = color;
        self
    }

    pub fn with_value_labels(mut self, labels: Vec<String>) -> Self {
        self.value_labels = Some(labels);
        self
    }

    pub fn with_value_size(mut self, size: f32) -> Self {
        self.value_size = size;
        self
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = (value - self.min) / (self.max - self.min).max(0.001);
        self.value = self.value.clamp(0.0, 1.0);
        self
    }

    pub fn get_value(&self) -> f32 {
        self.min + self.value * (self.max - self.min)
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
    }

    pub fn set_rect(&mut self, rect: [f32; 4]) {
        self.style.rect = rect;
    }

    pub fn with_thumb_roundness(mut self, roundness: f32) -> Self {
        self.thumb_roundness = roundness;
        self
    }

    fn thumb_position(&self) -> [f32; 2] {
        let [x, y, w, h] = self.style.rect;
        // Use same center adjustment as in build() for consistency
        let center_y = y + h * 0.5 + (h * 0.075);
        let thumb_x = x + self.value * w;
        [thumb_x, center_y]
    }

    fn hit_thumb(&self, pos: [f32; 2]) -> bool {
        let [tx, ty] = self.thumb_position();
        let r = self.style.thumb_radius;
        let dx = pos[0] - tx;
        let dy = pos[1] - ty;
        (dx * dx + dy * dy) <= r * r
    }

    fn hit_track(&self, pos: [f32; 2]) -> bool {
        let [x, y, w, h] = self.style.rect;
        pos[0] >= x && pos[0] <= x + w && pos[1] >= y && pos[1] <= y + h
    }

    fn update_value_from_pos(&mut self, pos: [f32; 2]) -> bool {
        let [x, _y, w, _h] = self.style.rect;
        let new_value = ((pos[0] - x) / w).clamp(0.0, 1.0);
        if (new_value - self.value).abs() > 0.001 {
            self.value = new_value;
            true
        } else {
            false
        }
    }
}

impl UiView for Slider {
    fn build(&mut self, ctx: &mut ViewContext) {
        let [x, y, w, h] = self.style.rect;
        // Align track with text center (text baseline is lower than mathematical center)
        // Moving center down slightly to align track with visual text center
        let center_y = y + h * 0.5 + (h * 0.075);
        let track_h = self.style.track_height;
        let half_track = track_h * 0.5;

        // Draw value text if enabled (positioned to the right of the slider track)
        if self.show_value {
            let value_text = if let Some(ref labels) = self.value_labels {
                // Use custom labels - convert slider value to index
                let current_value = self.get_value();
                let index = current_value.round() as usize;
                labels.get(index).cloned().unwrap_or_else(|| {
                    format!("{:.prec$}", current_value, prec = self.value_precision)
                })
            } else {
                format!("{:.prec$}", self.get_value(), prec = self.value_precision)
            };
            // Position to the right of the slider track
            let text_x = x + w + 8.0; // 8px to the right of the track
            // Center text vertically - need to use original mathematical center for text
            // since text positioning is different from track positioning
            let text_center_y = y + h * 0.5;
            let text_y = text_center_y - (self.value_size / 2.0);
            ctx.push(Drawable::Text {
                id: Uuid::new_v4(),
                text: value_text,
                origin: [text_x, text_y],
                px_size: self.value_size,
                color: self.value_color,
                layer: self.style.layer, // Same layer as track background
            });
        }

        // Draw background track (pill shape - radius = half height in pixels)
        let track_radius_px = track_h * 0.5;
        ctx.push(Drawable::Rect {
            id: Uuid::new_v4(),
            rect: [x, center_y - half_track, w, track_h],
            fill: self.style.track_color,
            border: Vec4::new(0.0, 0.0, 0.0, 0.0),
            radius_px: track_radius_px,
            border_px: 0.0,
            layer: self.style.layer,
        });

        // Draw filled track (up to thumb position) with same rounding
        let fill_w = self.value * w;
        if fill_w > 0.0 {
            ctx.push(Drawable::Rect {
                id: Uuid::new_v4(),
                rect: [x, center_y - half_track, fill_w, track_h],
                fill: self.style.fill_color,
                border: Vec4::new(0.0, 0.0, 0.0, 0.0),
                radius_px: track_radius_px, // Same pixel radius as background
                border_px: 0.0,
                layer: self.style.layer + 1,
            });
        }

        // Draw thumb
        let [tx, ty] = self.thumb_position();
        let r = self.style.thumb_radius;
        let thumb_size = r * 2.0;
        ctx.push(Drawable::Rect {
            id: self.render_id,
            rect: [tx - r, ty - r, thumb_size, thumb_size],
            fill: self.style.thumb_color,
            border: Vec4::new(0.0, 0.0, 0.0, 0.0),
            radius_px: self.thumb_roundness, // Use roundness from slider instance
            border_px: 0.0,
            layer: self.style.layer + 2,
        });
    }

    fn handle_event(&mut self, evt: &UiEvent) -> UiEventOutcome {
        match evt.kind {
            UiEventKind::PointerDown => {
                if self.hit_thumb(evt.pos) || self.hit_track(evt.pos) {
                    self.dragging = true;
                    self.active_pointer = Some(evt.pointer_id);
                    self.original_value = self.get_value(); // Capture original value for undo
                    let changed = self.update_value_from_pos(evt.pos);
                    let mut outcome = UiEventOutcome::dirty();
                    if changed {
                        outcome.merge(UiEventOutcome::with_action(UiAction::SliderChanged(
                            self.id.clone(),
                            self.get_value(),
                            self.original_value,
                            false, // Not final - dragging just started
                        )));
                    }
                    return outcome;
                }
            }
            UiEventKind::PointerMove => {
                if self.dragging && self.active_pointer == Some(evt.pointer_id) {
                    let changed = self.update_value_from_pos(evt.pos);
                    if changed {
                        let mut outcome = UiEventOutcome::dirty();
                        outcome.merge(UiEventOutcome::with_action(UiAction::SliderChanged(
                            self.id.clone(),
                            self.get_value(),
                            self.original_value,
                            false, // Not final - still dragging
                        )));
                        return outcome;
                    }
                }
            }
            UiEventKind::PointerUp => {
                if self.active_pointer == Some(evt.pointer_id) {
                    self.dragging = false;
                    self.active_pointer = None;
                    // Emit final value change for undo/redo
                    return UiEventOutcome::with_action(UiAction::SliderChanged(
                        self.id.clone(),
                        self.get_value(),
                        self.original_value,
                        true, // Final - mouse released
                    ));
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
