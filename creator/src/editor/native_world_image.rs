//! Display registration belongs to the viewport, independently of a scene or
//! atlas being replaced. Retain complete pixels until a replacement is used.
use super::*;
use renderkit::{ImageHandle, ImageRegistry};

type Scope = (
    (Uuid, ProjectContext, Uuid, Option<Uuid>),
    Uuid,
    EditorViewMode,
    MapCamera,
    (usize, usize),
    u32,
);
struct Display {
    source: ImageHandle,
    image: ImageHandle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderkit::{GpuContext, GpuImage, ImageDescriptor};

    #[test]
    #[ignore = "requires a native GPU"]
    fn viewport_display_survives_scene_registration_retirement() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut images = ImageRegistry::new(gpu.clone());
        let pixels = GpuImage::new(&gpu, ImageDescriptor::rgba(8, 8)).unwrap();
        let source = images.insert(pixels.clone()).unwrap();
        let mut owner = Owner::default();
        let mut scope = (
            (
                Uuid::new_v4(),
                ProjectContext::Unknown,
                Uuid::new_v4(),
                None,
            ),
            Uuid::new_v4(),
            EditorViewMode::Iso,
            MapCamera::ThreeDIso,
            (8, 8),
            1,
        );
        owner.scope(scope);
        let display = owner.publish(source, &mut images).unwrap();
        assert_ne!(source, display);
        assert_eq!(owner.publish(source, &mut images).unwrap(), display);
        assert!(images.remove(source));
        assert!(Arc::ptr_eq(&pixels, &images.get(display).unwrap()));
        // Empty/pending production cannot retire the current complete image.
        owner.finish(&mut images, &PaintList::default());
        assert_eq!(owner.current(), Some(display));
        assert!(images.get(display).is_some());
        // A changed viewport size revokes reuse, but an outstanding root pins it.
        scope.4 = (16, 8);
        owner.scope(scope);
        assert!(owner.current().is_none());
        let mut root = PaintList::default();
        root.image(display, Rect::new(0., 0., 8., 8.), Sampling::Nearest);
        owner.finish(&mut images, &root);
        assert!(images.get(display).is_some());
        owner.finish(&mut images, &PaintList::default());
        assert!(images.get(display).is_none());
    }
}
#[derive(Default)]
pub(super) struct Owner {
    scope: Option<Scope>,
    current: Option<Display>,
    retired: Vec<ImageHandle>,
}
impl Owner {
    pub fn revoke(&mut self) {
        if let Some(old) = self.current.take() {
            self.retired.push(old.image);
        }
    }
    pub fn scope(&mut self, scope: Scope) {
        if self.scope != Some(scope) {
            self.revoke();
            self.scope = Some(scope);
        }
    }
    pub fn current(&self) -> Option<ImageHandle> {
        self.current.as_ref().map(|display| display.image)
    }
    pub fn publish(
        &mut self,
        source: ImageHandle,
        images: &mut ImageRegistry,
    ) -> Result<ImageHandle, String> {
        if let Some(display) = &self.current
            && display.source == source
        {
            return Ok(display.image);
        }
        let pixels = images.get(source).ok_or("completed scene image missing")?;
        let image = images.insert(pixels).map_err(|error| error.to_string())?;
        if let Some(old) = self.current.replace(Display { source, image }) {
            self.retired.push(old.image);
        }
        Ok(image)
    }
    pub fn finish(&mut self, images: &mut ImageRegistry, root: &PaintList) {
        self.retired.retain(|image| {
            if root.image_handles().any(|used| used == *image) {
                true
            } else {
                images.remove(*image);
                false
            }
        });
    }
}
