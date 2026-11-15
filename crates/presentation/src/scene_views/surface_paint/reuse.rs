//! Immutable CPU backing proves which copied GPU pages remain current. Holding
//! both source owners prevents address reuse and keeps copy-on-write edits safe.
use super::*;
use rusterix::surface_paint::SurfacePaintPage;

pub(super) struct Previous {
    pub source: Arc<SurfacePaint>,
    pub binding: Arc<PaintPageBinding>,
}
pub(super) fn same_page(old: &SurfacePaintPage, new: &SurfacePaintPage) -> bool {
    old.atlas_origin == new.atlas_origin
        && old.color_rgba.len() == new.color_rgba.len()
        && old.material_rgba.len() == new.material_rgba.len()
        && old.color_rgba.as_ptr() == new.color_rgba.as_ptr()
        && old.material_rgba.as_ptr() == new.material_rgba.as_ptr()
}
impl Previous {
    pub fn candidate(old: &BoundSource, new: &SurfacePaint) -> Option<Self> {
        let State::Ready(binding) = &old.state else {
            return None;
        };
        if old.source.width != new.width || old.source.height != new.height || new.pages.is_empty()
        {
            return None;
        }
        // Bounded hints select a useful prior source without scanning all paint
        // in request(). Every skipped upload receives its own exact proof later.
        let related = [0, new.pages.len() / 2, new.pages.len() - 1]
            .into_iter()
            .any(|index| {
                old.source
                    .pages
                    .get(index)
                    .is_some_and(|page| same_page(page, &new.pages[index]))
            });
        related.then(|| Self {
            source: old.source.clone(),
            binding: binding.clone(),
        })
    }
}
