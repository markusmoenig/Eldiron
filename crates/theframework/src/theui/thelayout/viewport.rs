//! One clip predicate for collection paint, preparation and editor scopes.
use crate::prelude::TheDim;

#[derive(Clone, Copy)]
pub(super) struct ScrolledViewport {
    clip: TheDim,
    pub offset: [i32; 2],
}
impl ScrolledViewport {
    pub fn new(clip: TheDim, offset: [i32; 2]) -> Self {
        Self { clip, offset }
    }
    pub fn intersects(&self, item: &TheDim) -> bool {
        if !self.clip.is_valid() || !item.is_valid() {
            return false;
        }
        let left = i64::from(item.x) - i64::from(self.offset[0]);
        let top = i64::from(item.y) - i64::from(self.offset[1]);
        left < i64::from(self.clip.x) + i64::from(self.clip.width)
            && left + i64::from(item.width) > i64::from(self.clip.x)
            && top < i64::from(self.clip.y) + i64::from(self.clip.height)
            && top + i64::from(item.height) > i64::from(self.clip.y)
    }
}
