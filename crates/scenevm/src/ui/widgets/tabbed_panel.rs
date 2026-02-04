use uuid::Uuid;
use vek::Vec4;

use crate::ui::workspace::NodeId;
use crate::ui::{
    Drawable, UiView, ViewContext,
    event::{UiEvent, UiEventOutcome},
};

/// Style properties for a tabbed panel widget.
#[derive(Debug, Clone)]
pub struct TabbedPanelStyle {
    pub rect: [f32; 4],    // x, y, w, h in pixels
    pub fill: Vec4<f32>,   // Background color
    pub border: Vec4<f32>, // Border color
    pub radius_px: f32,    // Corner radius
    pub border_px: f32,    // Border width
    pub layer: i32,        // Rendering layer
    pub tab_height: f32,   // Height reserved for tab button area
    pub padding: f32,      // Padding inside the panel
}

/// A tabbed panel widget that displays a ButtonGroup for tabs and shows different content per tab.
#[derive(Debug, Clone)]
pub struct TabbedPanel {
    pub id: String,
    render_id: Uuid,
    pub style: TabbedPanelStyle,
    pub tab_button_group: NodeId,  // ButtonGroup for tab selection
    pub tab_contents: Vec<NodeId>, // One content node per tab
    pub active_tab: usize,         // Currently active tab index
}

impl TabbedPanel {
    /// Create a new tabbed panel widget.
    pub fn new(
        style: TabbedPanelStyle,
        tab_button_group: NodeId,
        tab_contents: Vec<NodeId>,
    ) -> Self {
        Self {
            id: String::new(),
            render_id: Uuid::new_v4(),
            style,
            tab_button_group,
            tab_contents,
            active_tab: 0,
        }
    }

    /// Set the widget ID (for lookup).
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the active tab index.
    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tab_contents.len() {
            self.active_tab = index;
        }
    }

    /// Get the currently active content node.
    pub fn active_content(&self) -> Option<NodeId> {
        self.tab_contents.get(self.active_tab).copied()
    }

    /// Get all tab content NodeIds (for workspace attachment).
    pub fn tab_contents(&self) -> &[NodeId] {
        &self.tab_contents
    }

    /// Get the tab button group NodeId.
    pub fn button_group(&self) -> NodeId {
        self.tab_button_group
    }

    /// Get children for layout purposes (button group + all tab contents).
    pub fn children(&self) -> Vec<NodeId> {
        let mut children = vec![self.tab_button_group];
        children.extend_from_slice(&self.tab_contents);
        children
    }

    /// Calculate layout for children: button group at top, active content below.
    pub fn calculate_layout(&self) -> Vec<[f32; 4]> {
        let [x, y, w, h] = self.style.rect;
        let padding = self.style.padding;

        // Button group at the top
        let button_rect = [
            x + padding,
            y + padding,
            w - 2.0 * padding,
            self.style.tab_height,
        ];

        // Content area below the button group
        let content_y = y + padding + self.style.tab_height + padding;
        let content_h = h - 2.0 * padding - self.style.tab_height - padding;
        let content_rect = [x + padding, content_y, w - 2.0 * padding, content_h];

        // Return rects: first is button group, then all tab contents get the same rect
        let mut rects = vec![button_rect];
        for _ in 0..self.tab_contents.len() {
            rects.push(content_rect);
        }
        rects
    }
}

impl UiView for TabbedPanel {
    fn build(&mut self, ctx: &mut ViewContext) {
        let [x, y, w, h] = self.style.rect;
        let padding = self.style.padding;

        // Draw background
        ctx.push(Drawable::Rect {
            id: self.render_id,
            rect: [x, y, w, h],
            fill: self.style.fill,
            border: self.style.border,
            radius_px: self.style.radius_px,
            border_px: self.style.border_px,
            layer: ctx.layer(),
        });

        // Draw separator line under the button group (as a thin rect)
        let separator_y = y + padding + self.style.tab_height + (padding / 2.0);
        ctx.push(Drawable::Rect {
            id: Uuid::new_v4(),
            rect: [x + padding, separator_y, w - 2.0 * padding, 1.0],
            fill: Vec4::new(0.3, 0.3, 0.35, 1.0),
            border: Vec4::new(0.0, 0.0, 0.0, 0.0),
            radius_px: 0.0,
            border_px: 0.0,
            layer: self.style.layer + 1,
        });

        // The ButtonGroup and content widgets are positioned and rendered by workspace
        // The ButtonGroup should be positioned at the top
        // The active content should be positioned below the tabs
    }

    fn handle_event(&mut self, _evt: &UiEvent) -> UiEventOutcome {
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
