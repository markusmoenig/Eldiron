//! Theme system for consistent UI styling

use vek::Vec4;

use crate::ui::{
    ButtonGroupStyle, ButtonStyle, ColorButtonStyle, DropdownListStyle, ParamListStyle,
    SliderStyle, TabbedPanelStyle, ToolbarStyle,
};

/// A UI theme that defines colors and styling for all widgets
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    // Background colors
    pub background: Vec4<f32>,
    pub surface: Vec4<f32>,
    pub surface_variant: Vec4<f32>,

    // Interactive element colors
    pub primary: Vec4<f32>,
    pub primary_hover: Vec4<f32>,
    pub primary_active: Vec4<f32>,

    // Border colors
    pub border: Vec4<f32>,
    pub border_subtle: Vec4<f32>,

    // Text colors
    pub text: Vec4<f32>,
    pub text_secondary: Vec4<f32>,

    // Accent colors
    pub accent: Vec4<f32>,
    pub accent_hover: Vec4<f32>,

    // Spacing and sizing
    pub radius_px: f32,
    pub border_px: f32,
}

impl Theme {
    /// Dark theme - distinctive look with rich blacks and vibrant accents
    pub fn dark() -> Self {
        Self {
            name: "Dark".into(),

            // Backgrounds - deep blacks
            background: Vec4::new(0.015, 0.015, 0.02, 1.0), // Near black
            surface: Vec4::new(0.06, 0.06, 0.07, 1.0),      // Deep charcoal
            surface_variant: Vec4::new(0.1, 0.1, 0.11, 1.0), // Slight lift for panels

            // Primary interactive elements - darker, more subtle
            primary: Vec4::new(0.16, 0.16, 0.18, 1.0),
            primary_hover: Vec4::new(0.2, 0.2, 0.22, 1.0),
            primary_active: Vec4::new(0.25, 0.25, 0.27, 1.0), // Brighter for pressed state in dark mode

            // Borders - very subtle, almost invisible
            border: Vec4::new(0.16, 0.16, 0.18, 1.0),
            border_subtle: Vec4::new(0.1, 0.1, 0.11, 1.0),

            // Text - crisp white
            text: Vec4::new(1.0, 1.0, 1.0, 1.0),
            text_secondary: Vec4::new(0.72, 0.72, 0.75, 1.0),

            // Accent - deeper blue
            accent: Vec4::new(0.08, 0.42, 0.9, 1.0), // Slightly darker
            accent_hover: Vec4::new(0.18, 0.5, 0.98, 1.0),

            // Rounded corners
            radius_px: 10.0, // Rounded corners
            border_px: 0.0,  // No visible borders
        }
    }

    /// Light theme - clean, bright appearance with high contrast
    pub fn light() -> Self {
        Self {
            name: "Light".into(),

            // Backgrounds - nearly white for maximum contrast with dark text
            background: Vec4::new(0.98, 0.98, 0.99, 1.0), // Nearly white canvas
            surface: Vec4::new(0.95, 0.95, 0.96, 1.0),    // Very light cards/panels
            surface_variant: Vec4::new(0.92, 0.92, 0.93, 1.0), // Inputs/secondary surfaces

            // Primary interactive elements
            primary: Vec4::new(0.88, 0.88, 0.89, 1.0),
            primary_hover: Vec4::new(0.84, 0.84, 0.85, 1.0),
            primary_active: Vec4::new(0.8, 0.8, 0.81, 1.0),

            // Borders - clearly visible
            border: Vec4::new(0.7, 0.7, 0.71, 1.0),
            border_subtle: Vec4::new(0.8, 0.8, 0.81, 1.0),

            // Text - black for maximum contrast on light surfaces
            text: Vec4::new(0.0, 0.0, 0.0, 1.0),
            text_secondary: Vec4::new(0.3, 0.3, 0.31, 1.0),

            // Accent - deeper blue for titles/active states
            accent: Vec4::new(0.02, 0.5, 0.94, 1.0), // Darker but saturated to stand out
            accent_hover: Vec4::new(0.0, 0.58, 1.0, 1.0),

            // Rounded corners
            radius_px: 8.0,
            border_px: 2.0,
        }
    }

    // Style factory methods

    /// Create a button style with the given rect
    pub fn button(&self, rect: [f32; 4]) -> ButtonStyle {
        // Nuanced icon tint - not pure white/black
        let icon_tint = if self.name == "Dark" {
            Vec4::new(0.9, 0.9, 0.95, 1.0) // Light gray with blue hint for dark theme
        } else {
            Vec4::new(0.2, 0.2, 0.2, 1.0) // Dark gray for light theme
        };

        ButtonStyle {
            rect,
            fill: self.surface_variant,
            border: self.border,
            pressed_fill: self.primary_active,
            pressed_border: self.border,
            radius_px: self.radius_px,
            border_px: self.border_px,
            layer: 15, // Higher layer than toolbar to ensure buttons draw on top
            text_color: self.text,
            icon_tint,
        }
    }

    /// Create a toolbar style with the given rect
    pub fn toolbar(&self, rect: [f32; 4]) -> ToolbarStyle {
        ToolbarStyle {
            rect,
            fill: self.surface,
            border: self.border_subtle,
            radius_px: self.radius_px,
            border_px: self.border_px,
            layer: 10,
        }
    }

    /// Create a button group style with the given rect and button dimensions
    pub fn button_group(
        &self,
        rect: [f32; 4],
        button_width: f32,
        button_height: f32,
    ) -> ButtonGroupStyle {
        // For dark theme: dark background with light text
        // For light theme: light background with dark text
        let text_bg_color = if self.name == "Light" {
            Vec4::new(1.0, 1.0, 1.0, 0.85) // Light semi-transparent background
        } else {
            Vec4::new(0.0, 0.0, 0.0, 0.85) // Dark semi-transparent background
        };

        ButtonGroupStyle {
            rect,
            button_width,
            button_height,
            spacing: 4.0,
            fill: self.surface_variant,
            border: self.border,
            active_fill: self.accent,
            active_border: self.accent_hover,
            radius_px: self.radius_px,
            border_px: self.border_px,
            layer: 15, // Higher layer than toolbar
            text_color: self.text,
            text_bg_color,
        }
    }

    /// Create a slider style with the given rect
    pub fn slider(&self, rect: [f32; 4]) -> SliderStyle {
        let track_color = if self.name == "Light" {
            self.primary_active // Darker track on light panels
        } else {
            Vec4::new(0.05, 0.05, 0.06, 1.0) // Dark mode: clearly darker than panel
        };

        let value_color = if self.name == "Light" {
            Vec4::new(0.2, 0.2, 0.22, 1.0) // Dark text for light mode
        } else {
            Vec4::new(0.6, 0.6, 0.65, 1.0) // Light text for dark mode
        };

        SliderStyle {
            rect,
            track_color,
            fill_color: self.accent,
            thumb_color: self.accent_hover,
            thumb_radius: 6.0,
            track_height: 4.0,
            layer: 11,
            value_color,
        }
    }

    /// Create a param list style with the given rect
    pub fn param_list(&self, rect: [f32; 4]) -> ParamListStyle {
        // Use accent color for title in both themes for emphasis
        let title_color = self.accent;

        ParamListStyle {
            rect,
            fill: self.surface_variant, // Match other widget surfaces for consistency
            border: self.border_subtle,
            radius_px: self.radius_px,
            border_px: self.border_px,
            layer: 10,
            title_color,
            title_size: 16.0,
            label_color: self.text, // Labels should always use theme text color
        }
    }

    /// Create a tabbed panel style with the given rect
    pub fn tabbed_panel(&self, rect: [f32; 4]) -> TabbedPanelStyle {
        TabbedPanelStyle {
            rect,
            fill: self.surface_variant,
            border: self.border_subtle,
            radius_px: self.radius_px,
            border_px: self.border_px,
            layer: 10,
            tab_height: 32.0,
            padding: 8.0,
        }
    }

    /// Create a dropdown list style with the given rect
    pub fn dropdown_list(&self, rect: [f32; 4]) -> DropdownListStyle {
        DropdownListStyle {
            rect,
            fill: self.surface_variant,
            border: self.border,
            hover_fill: self.primary_hover,
            // Panel uses a deeper shade and a stronger border to stand out against ParamList
            panel_fill: self.surface, // slightly darker than surface_variant
            panel_border: self.border,
            text_color: self.text,
            text_size: 14.0,
            radius_px: self.radius_px,
            border_px: self.border_px,
            layer: 15,
            item_height: 36.0,
            max_visible_items: 8,
        }
    }

    /// Create a color button style with the given rect
    pub fn color_button(&self, rect: [f32; 4]) -> ColorButtonStyle {
        ColorButtonStyle {
            rect,
            fill: self.surface_variant,
            border: Vec4::new(self.border.x, self.border.y, self.border.z, 1.0), // Full opacity border
            radius_px: self.radius_px,
            border_px: 1.0, // Standard border width
            layer: 10,
            swatch_padding: 4.0,
        }
    }

    /// Get the background color for this theme (for VM clear color)
    pub fn background_color(&self) -> [f32; 4] {
        [
            self.background.x,
            self.background.y,
            self.background.z,
            self.background.w,
        ]
    }
}
