use crate::prelude::*;

pub struct BrushList {
    pub brushes: IndexMap<Uuid, Box<dyn Brush>>,
}

impl Default for BrushList {
    fn default() -> Self {
        Self::new()
    }
}

impl BrushList {
    pub fn new() -> Self {
        let mut brushes: IndexMap<Uuid, Box<dyn Brush>> = IndexMap::default();

        let brush = Box::new(RectBrush::new());
        brushes.insert(brush.id().uuid, brush);
        let brush = Box::new(DiscBrush::new());
        brushes.insert(brush.id().uuid, brush);

        Self { brushes }
    }
}
