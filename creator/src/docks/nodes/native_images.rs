//! Resolve only preview requests emitted by visible graph/picker controls. The
//! held Arc identities detect replacement without hashing/copying all assets.
use super::*;
use std::{cell::RefCell, collections::HashSet, sync::Arc};
use theframework::{
    paint::ImageHandle,
    theui::paint::{UiImageResources, UiPaintError},
};
pub(super) struct NativeImageFrame<'a> {
    prefix: String,
    assets: &'a tiles::TileControls,
    ui: RefCell<&'a mut dyn UiImageResources>,
    previous: RefCell<&'a mut HashMap<String, Arc<TheRGBABuffer>>>,
    requested: RefCell<HashSet<String>>,
    handles: RefCell<HashMap<String, ImageHandle>>,
    error: RefCell<Option<UiPaintError>>,
}
impl<'a> NativeImageFrame<'a> {
    pub(super) fn new(
        prefix: String,
        assets: &'a tiles::TileControls,
        ui: &'a mut dyn UiImageResources,
        previous: &'a mut HashMap<String, Arc<TheRGBABuffer>>,
    ) -> Self {
        Self {
            prefix,
            assets,
            ui: RefCell::new(ui),
            previous: RefCell::new(previous),
            requested: RefCell::new(HashSet::new()),
            handles: RefCell::new(HashMap::new()),
            error: RefCell::new(None),
        }
    }
    pub(super) fn finish(self) -> Result<(), UiPaintError> {
        let requested = self.requested.into_inner();
        self.previous
            .into_inner()
            .retain(|key, _| requested.contains(key));
        self.error.into_inner().map_or(Ok(()), Err)
    }
}
impl GraphImageHandles for NativeImageFrame<'_> {
    fn paint_preview(
        &self,
        key: &str,
        rect: theframework::paint::Rect,
        paint: &mut theframework::paint::PaintList,
    ) {
        if let Some(layer) = self.assets.radials.get(key) {
            paint
                .ops
                .push(theframework::paint::PaintOp::PixelRadialLayer {
                    rect,
                    layer: layer.clone(),
                });
        } else if let Some(image) = self.image_handle(key) {
            paint.image(image, rect, theframework::paint::Sampling::Linear);
        }
    }
    fn image_handle(&self, key: &str) -> Option<ImageHandle> {
        if let Some(handle) = self.handles.borrow().get(key) {
            return Some(*handle);
        }
        let source = self.assets.image_arc(key)?;
        self.requested.borrow_mut().insert(key.to_owned());
        let changed = self
            .previous
            .borrow()
            .get(key)
            .is_none_or(|old| !Arc::ptr_eq(old, &source));
        match self.ui.borrow_mut().image(
            &format!("{}:{key}", self.prefix),
            source.as_ref(),
            changed,
        ) {
            Ok(handle) => {
                self.previous.borrow_mut().insert(key.to_owned(), source);
                self.handles.borrow_mut().insert(key.to_owned(), handle);
                Some(handle)
            }
            Err(error) => {
                self.error.borrow_mut().get_or_insert(error);
                None
            }
        }
    }
}
