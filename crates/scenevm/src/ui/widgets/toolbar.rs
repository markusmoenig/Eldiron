use uuid::Uuid;
use vek::Vec4;

use crate::ui::layouts::{HStack, VStack};
use crate::ui::workspace::NodeId;
use crate::ui::{Drawable, UiView, ViewContext};

/// Orientation for the toolbar layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarOrientation {
    Horizontal,
    Vertical,
}

/// Style properties for a toolbar widget.
#[derive(Debug, Clone)]
pub struct ToolbarStyle {
    pub rect: [f32; 4],    // x, y, w, h in pixels
    pub fill: Vec4<f32>,   // Background color
    pub border: Vec4<f32>, // Border color
    pub radius_px: f32,    // Corner radius
    pub border_px: f32,    // Border width
    pub layer: i32,        // Rendering layer
}

/// Separator style for toolbars.
#[derive(Debug, Clone)]
pub struct ToolbarSeparator {
    pub color: Vec4<f32>,
    pub thickness: f32,
    pub length: f32, // Length of the separator (perpendicular to orientation)
}

impl Default for ToolbarSeparator {
    fn default() -> Self {
        Self {
            color: Vec4::new(0.3, 0.3, 0.35, 1.0),
            thickness: 1.0,
            length: 24.0,
        }
    }
}

/// A toolbar widget that draws a background and uses HStack/VStack for automatic layout.
#[derive(Debug, Clone)]
pub struct Toolbar {
    pub id: String,
    render_id: Uuid,
    pub style: ToolbarStyle,
    pub orientation: ToolbarOrientation,
    pub draw_background: bool,  // Whether to draw the background rect
    pub hstack: Option<HStack>, // Used when orientation is Horizontal
    pub vstack: Option<VStack>, // Used when orientation is Vertical
    pub manual_separators: Vec<(f32, ToolbarSeparator)>, // Manually positioned separators (position, style)
}

impl Toolbar {
    /// Create a new toolbar widget.
    pub fn new(style: ToolbarStyle, orientation: ToolbarOrientation) -> Self {
        let rect = style.rect;

        // Create the appropriate layout based on orientation
        let (hstack, vstack) = match orientation {
            ToolbarOrientation::Horizontal => (
                Some(HStack::new(rect).with_spacing(4.0).with_padding(8.0)),
                None,
            ),
            ToolbarOrientation::Vertical => (
                None,
                Some(VStack::new(rect).with_spacing(4.0).with_padding(8.0)),
            ),
        };

        Self {
            id: String::new(),
            render_id: Uuid::new_v4(),
            style,
            orientation,
            draw_background: true,
            hstack,
            vstack,
            manual_separators: Vec::new(),
        }
    }

    /// Set the widget ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set whether to draw the background.
    pub fn with_background(mut self, draw: bool) -> Self {
        self.draw_background = draw;
        self
    }

    /// Set the spacing between items.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        match self.orientation {
            ToolbarOrientation::Horizontal => {
                if let Some(ref mut hstack) = self.hstack {
                    *hstack = hstack.clone().with_spacing(spacing);
                }
            }
            ToolbarOrientation::Vertical => {
                if let Some(ref mut vstack) = self.vstack {
                    *vstack = vstack.clone().with_spacing(spacing);
                }
            }
        }
        self
    }

    /// Set the padding (offset from edges).
    pub fn with_padding(mut self, padding: f32) -> Self {
        match self.orientation {
            ToolbarOrientation::Horizontal => {
                if let Some(ref mut hstack) = self.hstack {
                    *hstack = hstack.clone().with_padding(padding);
                }
            }
            ToolbarOrientation::Vertical => {
                if let Some(ref mut vstack) = self.vstack {
                    *vstack = vstack.clone().with_padding(padding);
                }
            }
        }
        self
    }

    /// Add a child to the toolbar (will be automatically laid out).
    pub fn add_child(&mut self, child: NodeId) {
        match self.orientation {
            ToolbarOrientation::Horizontal => {
                if let Some(ref mut hstack) = self.hstack {
                    hstack.add_child(child);
                }
            }
            ToolbarOrientation::Vertical => {
                if let Some(ref mut vstack) = self.vstack {
                    vstack.add_child(child);
                }
            }
        }
    }

    /// Get the children (for workspace integration).
    pub fn children(&self) -> &[NodeId] {
        match self.orientation {
            ToolbarOrientation::Horizontal => self
                .hstack
                .as_ref()
                .map(|h| h.children.as_slice())
                .unwrap_or(&[]),
            ToolbarOrientation::Vertical => self
                .vstack
                .as_ref()
                .map(|v| v.children.as_slice())
                .unwrap_or(&[]),
        }
    }

    /// Add a separator at the given position that will be drawn automatically.
    /// For horizontal toolbars: position is x coordinate, separator is vertical
    /// For vertical toolbars: position is y coordinate, separator is horizontal
    pub fn add_separator_at(&mut self, position: f32, separator_style: Option<ToolbarSeparator>) {
        let sep = separator_style.unwrap_or_default();
        self.manual_separators.push((position, sep));
    }

    /// Create a separator drawable at the given position.
    /// For horizontal toolbars: position is x coordinate, separator is vertical
    /// For vertical toolbars: position is y coordinate, separator is horizontal
    pub fn create_separator_at(
        &self,
        position: f32,
        separator_style: Option<ToolbarSeparator>,
    ) -> Drawable {
        let sep = separator_style.unwrap_or_default();
        let [x, y, w, h] = self.style.rect;

        let rect = match self.orientation {
            ToolbarOrientation::Horizontal => {
                // Vertical separator at x position
                let sep_y = y + (h - sep.length) / 2.0;
                [position, sep_y, sep.thickness, sep.length]
            }
            ToolbarOrientation::Vertical => {
                // Horizontal separator at y position
                let sep_x = x + (w - sep.length) / 2.0;
                [sep_x, position, sep.length, sep.thickness]
            }
        };

        Drawable::Rect {
            id: Uuid::new_v4(),
            rect,
            fill: sep.color,
            border: Vec4::new(0.0, 0.0, 0.0, 0.0),
            radius_px: 0.0,
            border_px: 0.0,
            layer: self.style.layer + 1,
        }
    }
}

impl UiView for Toolbar {
    fn build(&mut self, ctx: &mut ViewContext) {
        // Draw the background only if enabled
        if self.draw_background {
            ctx.push(Drawable::Rect {
                id: self.render_id,
                rect: self.style.rect,
                fill: self.style.fill,
                border: self.style.border,
                radius_px: self.style.radius_px,
                border_px: self.style.border_px,
                layer: self.style.layer,
            });
        }

        // Draw manual separators
        for (position, sep) in &self.manual_separators {
            let drawable = self.create_separator_at(*position, Some(sep.clone()));
            ctx.push(drawable);
        }
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
