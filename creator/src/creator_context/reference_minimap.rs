//! CPU cache retained only by explicit reference minimap drawing. Native views
//! own independent GPU frames and presented navigation geometry.
use theframework::prelude::{TheRGBABuffer, Vec4};

pub(crate) struct ReferenceMinimap {
    pub(crate) image: TheRGBABuffer,
    pub(crate) bbox: Vec4<f32>,
    pub(crate) key: u64,
}
impl Default for ReferenceMinimap {
    fn default() -> Self {
        Self {
            image: TheRGBABuffer::default(),
            bbox: Vec4::one(),
            key: 0,
        }
    }
}
