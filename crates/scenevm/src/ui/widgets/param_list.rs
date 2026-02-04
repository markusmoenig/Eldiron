use uuid::Uuid;
use vek::Vec4;

use crate::ui::workspace::NodeId;
use crate::ui::{Drawable, UiView, ViewContext};

/// Style properties for a parameter list widget.
#[derive(Debug, Clone)]
pub struct ParamListStyle {
    pub rect: [f32; 4],         // x, y, w, h in pixels
    pub fill: Vec4<f32>,        // Background color
    pub border: Vec4<f32>,      // Border color
    pub radius_px: f32,         // Corner radius
    pub border_px: f32,         // Border width
    pub layer: i32,             // Rendering layer
    pub title_color: Vec4<f32>, // Default title color
    pub title_size: f32,        // Default title font size
    pub label_color: Vec4<f32>, // Label text color (from theme)
}

/// A row in the parameter list. Either a normal widget row or a text separator.
#[derive(Debug, Clone)]
pub enum ParamListEntry {
    Item {
        label: String,
        widget: NodeId,
        reserve_value_space: bool,
    },
    Separator {
        text: String,
    },
}

/// A parameter list widget that displays labels on the left and widgets on the right.
/// Arranges items vertically with automatic layout.
#[derive(Debug, Clone)]
pub struct ParamList {
    pub id: String,
    render_id: Uuid,
    pub style: ParamListStyle,
    pub title: Option<String>,        // Optional title text
    pub title_color: Vec4<f32>,       // Title text color
    pub title_size: f32,              // Title font size
    pub title_height: f32,            // Height reserved for title area
    pub item_height: f32,             // Height of each row
    pub spacing: f32,                 // Vertical spacing between rows
    pub label_width: f32,             // Width of the label column
    pub padding: f32,                 // Padding inside the list
    pub label_offset: f32,            // Horizontal offset for labels from left edge
    pub entries: Vec<ParamListEntry>, // Ordered rows (widgets or separators)
    pub label_color: Vec4<f32>,       // Color for labels
    pub label_size: f32,              // Font size for labels
}

impl ParamList {
    /// Create a new parameter list widget.
    pub fn new(style: ParamListStyle) -> Self {
        let title_color = style.title_color;
        let title_size = style.title_size;
        let label_color = style.label_color;

        Self {
            id: String::new(),
            render_id: Uuid::new_v4(),
            style,
            title: None,
            title_color,
            title_size,
            title_height: 30.0,
            item_height: 32.0,
            spacing: 4.0,
            label_width: 100.0,
            padding: 8.0,
            label_offset: 8.0,
            entries: Vec::new(),
            label_color,
            label_size: 14.0,
        }
    }

    /// Set the widget ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the height of each row.
    pub fn with_item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }

    /// Set the spacing between rows.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the width of the label column.
    pub fn with_label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    /// Set the padding inside the list.
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the horizontal offset for labels.
    pub fn with_label_offset(mut self, offset: f32) -> Self {
        self.label_offset = offset;
        self
    }

    /// Set the label color.
    pub fn with_label_color(mut self, color: Vec4<f32>) -> Self {
        self.label_color = color;
        self
    }

    /// Set the label font size.
    pub fn with_label_size(mut self, size: f32) -> Self {
        self.label_size = size;
        self
    }

    /// Set the title text.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the title color.
    pub fn with_title_color(mut self, color: Vec4<f32>) -> Self {
        self.title_color = color;
        self
    }

    /// Set the title font size.
    pub fn with_title_size(mut self, size: f32) -> Self {
        self.title_size = size;
        self
    }

    /// Set the title area height.
    pub fn with_title_height(mut self, height: f32) -> Self {
        self.title_height = height;
        self
    }

    /// Add a parameter item (label and widget).
    pub fn add_item(&mut self, label: impl Into<String>, widget: NodeId) {
        self.add_item_with_value_space(label, widget, true);
    }

    /// Add a parameter item but skip reserving extra space on the right
    /// (useful for widgets that use their entire width, like dropdowns).
    pub fn add_item_full_width(&mut self, label: impl Into<String>, widget: NodeId) {
        self.add_item_with_value_space(label, widget, false);
    }

    /// Add a parameter item with explicit control over value-text spacing.
    pub fn add_item_with_value_space(
        &mut self,
        label: impl Into<String>,
        widget: NodeId,
        reserve_value_space: bool,
    ) {
        self.entries.push(ParamListEntry::Item {
            label: label.into(),
            widget,
            reserve_value_space,
        });
        // Auto-update height based on content
        self.update_height();
    }

    /// Add a text separator row (uses the title styling).
    pub fn add_separator(&mut self, text: impl Into<String>) {
        self.entries
            .push(ParamListEntry::Separator { text: text.into() });
        // Auto-update height based on content
        self.update_height();
    }

    /// Get the children (widget NodeIds) for workspace integration.
    pub fn children(&self) -> Vec<NodeId> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                ParamListEntry::Item { widget, .. } => Some(*widget),
                ParamListEntry::Separator { .. } => None,
            })
            .collect()
    }

    /// Update the ParamList height based on its content.
    pub fn update_height(&mut self) {
        let height = self.calculate_total_height();
        self.style.rect[3] = height;
    }

    /// Get the Y offset for content (accounts for title if present).
    fn content_offset_y(&self) -> f32 {
        if self.title.is_some() {
            self.title_height
        } else {
            0.0
        }
    }

    /// Calculate the minimum width needed based on label width, widget space, and padding.
    /// Returns the calculated width.
    pub fn calculate_min_width(&self, widget_width: f32) -> f32 {
        // padding + label_offset + label_width + widget_width + value_text_space + padding
        self.padding + self.label_offset + self.label_width + widget_width + 40.0 + self.padding
    }

    /// Set the width based on the expected widget width.
    pub fn with_auto_width(mut self, widget_width: f32) -> Self {
        let width = self.calculate_min_width(widget_width);
        self.style.rect[2] = width;
        self
    }

    /// Calculate layout positions for all widget children.
    /// Returns computed rects for each widget.
    pub fn calculate_layout(&self, child_sizes: &[[f32; 2]]) -> Vec<[f32; 4]> {
        let [x, _y, w, _] = self.style.rect;
        let mut rects = Vec::new();

        let mut widget_index = 0;
        for (row_index, entry) in self.entries.iter().enumerate() {
            let ParamListEntry::Item {
                reserve_value_space,
                ..
            } = entry
            else {
                continue;
            };

            let Some(&[child_width, _]) = child_sizes.get(widget_index) else {
                break;
            };

            let widget_x = x + self.padding + self.label_width;
            let widget_y = self.row_y(row_index);
            let widget_h = self.item_height;

            // Reserve space on the right for value text if desired. Otherwise,
            // use the full available width for the widget.
            let value_space = if *reserve_value_space { 40.0 } else { 0.0 };
            let available_width = w - self.padding * 2.0 - self.label_width - value_space;
            let final_width = if *reserve_value_space {
                child_width.min(available_width)
            } else {
                available_width
            };

            rects.push([widget_x, widget_y, final_width, widget_h]);
            widget_index += 1;
        }

        rects
    }

    /// Get the position for a label at the given index.
    /// Returns [x, y] for the label origin.
    pub fn get_label_position(&self, row_index: usize) -> [f32; 2] {
        let [x, _, _, _] = self.style.rect;
        let label_x = x + self.padding + self.label_offset;
        // Calculate the center of the row
        let row_center_y = self.row_y(row_index) + (self.item_height / 2.0);
        // Position text so its vertical center aligns with row center
        // Text origin is at top-left, so we subtract half the font size
        let label_y = row_center_y - (self.label_size / 2.0);
        [label_x, label_y]
    }

    /// Get the rect for a widget at the given index.
    /// Returns [x, y, w, h] for the widget.
    pub fn get_widget_rect(&self, index: usize, widget_width: f32) -> [f32; 4] {
        let Some(row_index) = self.row_index_for_widget(index) else {
            return [0.0, 0.0, 0.0, 0.0];
        };

        let [x, _y, w, _] = self.style.rect;
        let widget_x = x + self.padding + self.label_width;
        let widget_y = self.row_y(row_index);
        let widget_h = self.item_height;
        let value_space = self
            .entry_at_row(row_index)
            .map(|entry| match entry {
                ParamListEntry::Item {
                    reserve_value_space,
                    ..
                } if *reserve_value_space => 40.0,
                _ => 0.0,
            })
            .unwrap_or(0.0);
        let available_width = w - self.padding * 2.0 - self.label_width - value_space;
        let final_width = if value_space > 0.0 {
            widget_width.min(available_width)
        } else {
            available_width
        };
        [widget_x, widget_y, final_width, widget_h]
    }

    /// Calculate the total height needed for all items.
    pub fn calculate_total_height(&self) -> f32 {
        let content_y_offset = self.content_offset_y();
        let row_count = self.entries.len();
        if row_count == 0 {
            content_y_offset + self.padding * 2.0
        } else {
            content_y_offset
                + self.padding * 2.0
                + (row_count as f32 * self.item_height)
                + ((row_count - 1) as f32 * self.spacing)
        }
    }

    /// Set the position of the ParamList (useful for popups).
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.style.rect[0] = x;
        self.style.rect[1] = y;
    }

    /// Get the size of the ParamList [width, height].
    pub fn get_size(&self) -> [f32; 2] {
        [self.style.rect[2], self.style.rect[3]]
    }

    /// Number of widget rows (excludes separators).
    pub fn widget_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e, ParamListEntry::Item { .. }))
            .count()
    }

    /// Get the Y coordinate for the top of a row.
    fn row_y(&self, row_index: usize) -> f32 {
        let [_, y, _, _] = self.style.rect;
        y + self.content_offset_y()
            + self.padding
            + (row_index as f32 * (self.item_height + self.spacing))
    }

    /// Map a widget index (only items) to its row index including separators.
    fn row_index_for_widget(&self, widget_index: usize) -> Option<usize> {
        let mut current_widget = 0;
        for (row_index, entry) in self.entries.iter().enumerate() {
            if let ParamListEntry::Item { .. } = entry {
                if current_widget == widget_index {
                    return Some(row_index);
                }
                current_widget += 1;
            }
        }
        None
    }

    fn entry_at_row(&self, row_index: usize) -> Option<&ParamListEntry> {
        self.entries.get(row_index)
    }
}

impl UiView for ParamList {
    fn build(&mut self, ctx: &mut ViewContext) {
        // Draw the background
        ctx.push(Drawable::Rect {
            id: self.render_id,
            rect: self.style.rect,
            fill: self.style.fill,
            border: self.style.border,
            radius_px: self.style.radius_px,
            border_px: self.style.border_px,
            layer: self.style.layer,
        });

        // Draw title if present
        if let Some(ref title_text) = self.title {
            let [x, y, w, _] = self.style.rect;
            let title_x = x + self.padding;
            let title_y = y + (self.title_height - self.title_size) / 2.0;

            ctx.push(Drawable::Text {
                id: Uuid::new_v4(),
                text: title_text.clone(),
                origin: [title_x, title_y],
                px_size: self.title_size,
                color: self.title_color,
                layer: self.style.layer + 1,
            });

            // Draw separator line below title
            let separator_y = y + self.title_height - 1.0;
            ctx.push(Drawable::Rect {
                id: Uuid::new_v4(),
                rect: [x + self.padding, separator_y, w - self.padding * 2.0, 1.0],
                fill: Vec4::new(0.3, 0.3, 0.35, 1.0),
                border: Vec4::new(0.0, 0.0, 0.0, 0.0),
                radius_px: 0.0,
                border_px: 0.0,
                layer: self.style.layer + 1,
            });
        }

        // Draw labels
        for (row_index, entry) in self.entries.iter().enumerate() {
            match entry {
                ParamListEntry::Item { label, .. } => {
                    let [label_x, label_y] = self.get_label_position(row_index);
                    ctx.push(Drawable::Text {
                        id: Uuid::new_v4(),
                        text: label.clone(),
                        origin: [label_x, label_y],
                        px_size: self.label_size,
                        color: self.label_color,
                        layer: self.style.layer + 1,
                    });
                }
                ParamListEntry::Separator { text } => {
                    let [x, _y, _w, _h] = self.style.rect;
                    let separator_x = x + self.padding;
                    let separator_y =
                        self.row_y(row_index) + (self.item_height - self.title_size) * 0.5;
                    ctx.push(Drawable::Text {
                        id: Uuid::new_v4(),
                        text: text.clone(),
                        origin: [separator_x, separator_y],
                        px_size: self.title_size,
                        color: self.title_color,
                        layer: self.style.layer + 1,
                    });
                }
            }
        }

        // Note: Widgets are positioned automatically by the workspace layout system
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
