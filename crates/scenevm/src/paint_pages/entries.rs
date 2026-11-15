//! Immutable key backing may already be a published Vec. Keep that allocation
//! intact rather than copying every key again when a residency owner starts.
use crate::Raster3DSurfacePaintEntry;
use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub(crate) enum PaintEntryStorage {
    Slice(Arc<[Raster3DSurfacePaintEntry]>),
    Vec(Arc<Vec<Raster3DSurfacePaintEntry>>),
}
impl PaintEntryStorage {
    pub fn same_storage(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Slice(a), Self::Slice(b)) => Arc::ptr_eq(a, b),
            (Self::Vec(a), Self::Vec(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
    pub fn copy_from(entries: &[Raster3DSurfacePaintEntry]) -> Self {
        Self::Slice(Arc::from(entries))
    }
}
impl AsRef<[Raster3DSurfacePaintEntry]> for PaintEntryStorage {
    fn as_ref(&self) -> &[Raster3DSurfacePaintEntry] {
        match self {
            Self::Slice(entries) => entries,
            Self::Vec(entries) => entries,
        }
    }
}
impl Deref for PaintEntryStorage {
    type Target = [Raster3DSurfacePaintEntry];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
