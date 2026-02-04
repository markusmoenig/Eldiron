use crate::ui::ViewContext;
use crate::ui::layouts::Layoutable;
use crate::ui::workspace::UiView;

/// A spacer widget that takes up space but doesn't draw anything.
/// Useful for creating gaps in layouts.
#[derive(Debug, Clone)]
pub struct Spacer {
    pub rect: [f32; 4], // x, y, w, h
    pub flexible: bool, // If true, this spacer expands to fill available space
}

impl Spacer {
    /// Create a fixed-size spacer
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            rect: [0.0, 0.0, width, height],
            flexible: false,
        }
    }

    /// Create a flexible spacer that fills available space
    pub fn flexible() -> Self {
        Self {
            rect: [0.0, 0.0, 0.0, 0.0], // Size will be calculated by layout
            flexible: true,
        }
    }
}

impl UiView for Spacer {
    fn build(&mut self, _ctx: &mut ViewContext) {
        // Spacer doesn't draw anything
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Layoutable for Spacer {
    fn set_layout_rect(&mut self, rect: [f32; 4]) {
        self.rect = rect;
    }

    fn get_desired_size(&self) -> Option<[f32; 2]> {
        let [_x, _y, w, h] = self.rect;
        Some([w, h])
    }
}
