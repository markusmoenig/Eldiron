use uuid::Uuid;
use vek::Vec4;

use crate::ui::workspace::NodeId;
use crate::ui::{
    PopupAlignment,
    drawable::Drawable,
    event::{UiAction, UiEvent, UiEventKind, UiEventOutcome},
    workspace::{UiView, ViewContext},
};

/// Style properties for a color button widget.
#[derive(Debug, Clone)]
pub struct ColorButtonStyle {
    pub rect: [f32; 4],      // x, y, w, h in logical space
    pub fill: Vec4<f32>,     // Background color
    pub border: Vec4<f32>,   // Border color
    pub radius_px: f32,      // Corner radius
    pub border_px: f32,      // Border width
    pub layer: i32,          // Rendering layer
    pub swatch_padding: f32, // Padding around the color swatch inside the button
}

/// A button that displays a color swatch and opens a popup with a ColorWheel when clicked.
#[derive(Debug, Clone)]
pub struct ColorButton {
    pub id: String,
    swatch_id: Uuid,
    pub style: ColorButtonStyle,
    pub current_color: Vec4<f32>,    // Currently selected color
    pub color_wheel: Option<NodeId>, // ColorWheel popup content
    pub popup_alignment: PopupAlignment,
    popup_visible: bool,
    active_pointer: Option<u32>,
}

impl ColorButton {
    /// Create a new color button with the given style and initial color.
    pub fn new(style: ColorButtonStyle, initial_color: Vec4<f32>) -> Self {
        Self {
            id: String::new(),
            swatch_id: Uuid::new_v4(),
            style,
            current_color: initial_color,
            color_wheel: None,
            popup_alignment: PopupAlignment::Right,
            popup_visible: false,
            active_pointer: None,
        }
    }

    /// Set the widget ID (for lookup).
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the color wheel popup content.
    pub fn with_color_wheel(mut self, color_wheel: NodeId, alignment: PopupAlignment) -> Self {
        self.color_wheel = Some(color_wheel);
        self.popup_alignment = alignment;
        self
    }

    /// Set the current color.
    pub fn set_color(&mut self, color: Vec4<f32>) {
        self.current_color = color;
    }

    /// Get the current color.
    pub fn color(&self) -> Vec4<f32> {
        self.current_color
    }

    /// Show the color wheel popup.
    pub fn show_popup(&mut self) {
        self.popup_visible = true;
    }

    /// Hide the color wheel popup.
    pub fn hide_popup(&mut self) {
        self.popup_visible = false;
    }

    /// Check if popup is visible.
    pub fn is_popup_visible(&self) -> bool {
        self.popup_visible
    }
}

impl UiView for ColorButton {
    fn build(&mut self, ctx: &mut ViewContext) {
        let [x, y, w, h] = self.style.rect;

        // Draw color swatch (no outer rect needed)
        let padding = self.style.swatch_padding;
        ctx.push(Drawable::Rect {
            id: self.swatch_id,
            rect: [
                x + padding,
                y + padding,
                w - 2.0 * padding,
                h - 2.0 * padding,
            ],
            fill: self.current_color,
            border: Vec4::new(0.0, 0.0, 0.0, 1.0), // Solid black border around color swatch
            radius_px: (self.style.radius_px - padding).max(0.0),
            border_px: 1.0,
            layer: self.style.layer + 1,
        });
    }

    fn handle_event(&mut self, evt: &UiEvent) -> UiEventOutcome {
        let [x, y, w, h] = self.style.rect;
        let pos = evt.pos;
        let id = evt.pointer_id;

        match evt.kind {
            UiEventKind::PointerDown => {
                if pos[0] >= x && pos[0] <= x + w && pos[1] >= y && pos[1] <= y + h {
                    self.active_pointer = Some(id);
                    return UiEventOutcome::dirty();
                }
            }
            UiEventKind::PointerUp => {
                if self.active_pointer == Some(id) {
                    self.active_pointer = None;
                    // Check if release is inside button
                    if pos[0] >= x && pos[0] <= x + w && pos[1] >= y && pos[1] <= y + h {
                        // Toggle popup
                        self.popup_visible = !self.popup_visible;
                        return UiEventOutcome::with_action(UiAction::ButtonPressed(
                            self.id.clone(),
                        ));
                    }
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
