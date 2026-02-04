use crate::ui::workspace::{NodeId, UiView, ViewContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

/// Trait for views that can be positioned by a layout
pub trait Layoutable {
    /// Set the rect for this view (called by layout during build)
    fn set_layout_rect(&mut self, rect: [f32; 4]);

    /// Get the desired size for this view (width, height)
    /// Returns None if the view should fill available space
    fn get_desired_size(&self) -> Option<[f32; 2]>;
}

#[derive(Debug, Clone)]
pub struct VStack {
    pub children: Vec<NodeId>,
    pub rect: [f32; 4], // x, y, w, h
    pub spacing: f32,
    pub padding: f32,
    pub alignment: Alignment,
    computed_rects: Vec<[f32; 4]>, // Computed child rects
}

impl VStack {
    pub fn new(rect: [f32; 4]) -> Self {
        Self {
            children: Vec::new(),
            rect,
            spacing: 4.0,
            padding: 8.0,
            alignment: Alignment::Start,
            computed_rects: Vec::new(),
        }
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }

    /// Calculate layout positions for all children
    /// Returns computed rects for each child
    pub fn calculate_layout(
        &self,
        child_sizes: &[[f32; 2]],
        _flexible_indices: &[usize],
    ) -> Vec<[f32; 4]> {
        let [x, y, width, _height] = self.rect;
        let mut rects = Vec::new();

        if child_sizes.is_empty() {
            return rects;
        }

        // Current y position for laying out children
        let mut current_y = y + self.padding;

        // Note: VStack doesn't yet support flexible spacers (could be added later)
        for &[child_width, child_height] in child_sizes {
            // Calculate x position based on alignment
            let x_pos = match self.alignment {
                Alignment::Start => x + self.padding,
                Alignment::Center => x + (width - child_width) / 2.0,
                Alignment::End => x + width - self.padding - child_width,
            };

            rects.push([x_pos, current_y, child_width, child_height]);
            current_y += child_height + self.spacing;
        }

        rects
    }

    /// Get the computed rect for a specific child index
    pub fn get_child_rect(&self, index: usize) -> Option<[f32; 4]> {
        self.computed_rects.get(index).copied()
    }
}

impl UiView for VStack {
    fn build(&mut self, _ctx: &mut ViewContext) {
        // VStack doesn't draw anything itself, it just arranges children
        // Child layout is calculated by the workspace during build
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct HStack {
    pub children: Vec<NodeId>,
    pub rect: [f32; 4], // x, y, w, h
    pub spacing: f32,
    pub padding: f32,
    pub alignment: Alignment,
    computed_rects: Vec<[f32; 4]>, // Computed child rects
}

impl HStack {
    pub fn new(rect: [f32; 4]) -> Self {
        Self {
            children: Vec::new(),
            rect,
            spacing: 4.0,
            padding: 8.0,
            alignment: Alignment::Center,
            computed_rects: Vec::new(),
        }
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }

    /// Calculate layout positions for all children
    /// Returns computed rects for each child
    pub fn calculate_layout(
        &self,
        child_sizes: &[[f32; 2]],
        flexible_indices: &[usize],
    ) -> Vec<[f32; 4]> {
        let [x, y, width, height] = self.rect;
        let mut rects = Vec::new();

        if child_sizes.is_empty() {
            return rects;
        }

        // Calculate total width of fixed-size children and spacing
        let total_spacing = (child_sizes.len() as f32 - 1.0).max(0.0) * self.spacing;
        let mut fixed_width = 0.0;
        for (i, &[child_width, _]) in child_sizes.iter().enumerate() {
            if !flexible_indices.contains(&i) {
                fixed_width += child_width;
            }
        }

        // Calculate width for flexible spacers
        let available_width = width - 2.0 * self.padding - total_spacing - fixed_width;
        let flexible_count = flexible_indices.len().max(1) as f32;
        let flexible_width = (available_width / flexible_count).max(0.0);

        // Current x position for laying out children
        let mut current_x = x + self.padding;

        for (i, &[child_width, child_height]) in child_sizes.iter().enumerate() {
            let actual_width = if flexible_indices.contains(&i) {
                flexible_width
            } else {
                child_width
            };

            // Calculate y position based on alignment
            let y_pos = match self.alignment {
                Alignment::Start => y + self.padding,
                Alignment::Center => y + (height - child_height) / 2.0,
                Alignment::End => y + height - self.padding - child_height,
            };

            rects.push([current_x, y_pos, actual_width, child_height]);
            current_x += actual_width + self.spacing;
        }

        rects
    }

    /// Get the computed rect for a specific child index
    pub fn get_child_rect(&self, index: usize) -> Option<[f32; 4]> {
        self.computed_rects.get(index).copied()
    }
}

impl UiView for HStack {
    fn build(&mut self, _ctx: &mut ViewContext) {
        // HStack doesn't draw anything itself, it just arranges children
        // Child layout is calculated by the workspace during build
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
