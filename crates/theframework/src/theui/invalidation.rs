//! CPU UI invalidation, independent of a renderer or window host.

/// The owner of a change declares which preparation it invalidates. Scene-only
/// frames preserve retained controls; resource changes retire their commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiInvalidation {
    Layout,
    Paint,
    Resources,
    Scene,
}

const LAYOUT: u8 = 1;
const PAINT: u8 = 2;
const RESOURCES: u8 = 4;
const SCENE: u8 = 8;

#[derive(Default)]
pub(super) struct UiInvalidationState {
    pending: u8,
}

impl UiInvalidationState {
    pub fn invalidate(&mut self, change: UiInvalidation) {
        self.pending |= match change {
            UiInvalidation::Layout => LAYOUT | PAINT,
            UiInvalidation::Paint => PAINT,
            UiInvalidation::Resources => RESOURCES | PAINT,
            UiInvalidation::Scene => SCENE,
        };
    }

    pub fn needs_layout(&self) -> bool {
        self.pending & LAYOUT != 0
    }

    pub fn needs_full_paint(&self) -> bool {
        self.pending & (PAINT | RESOURCES) != 0
    }

    pub fn needs_frame(&self) -> bool {
        self.pending != 0
    }

    pub fn layout_completed(&mut self) {
        self.pending &= !LAYOUT;
    }

    pub fn paint_completed(&mut self) {
        self.pending &= !(PAINT | RESOURCES);
    }

    pub fn scene_completed(&mut self) {
        self.pending &= !SCENE;
    }
}
