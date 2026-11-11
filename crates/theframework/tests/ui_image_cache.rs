#![cfg(all(feature = "ui", feature = "render-wgpu"))]
use renderkit::*;
use theframework::{
    paint::*,
    prelude::TheRGBABuffer,
    render_wgpu::{PaintRenderer, UiImageCache},
    theui::paint::UiImageResources,
};

fn source(color: [u8; 4]) -> TheRGBABuffer {
    TheRGBABuffer::from(color.to_vec(), 1, 1)
}
fn list(handles: &[ImageHandle]) -> PaintList {
    let mut list = PaintList::default();
    for (index, handle) in handles.iter().enumerate() {
        list.image(
            *handle,
            Rect::new(index as f32 * 8., 0., 8., 8.),
            Sampling::Nearest,
        );
    }
    list
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn image_versions_remain_pinned_until_the_complete_paint_retires_them() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = UiImageCache::default();
    let red = cache
        .frame(&mut registry)
        .image("tile", &source([255, 0, 0, 255]), false)
        .unwrap();
    let blue = cache
        .frame(&mut registry)
        .image("tile", &source([0, 0, 255, 255]), true)
        .unwrap();
    assert_ne!(red, blue);
    assert!(
        registry.get(red).is_some(),
        "an earlier command can still sample the old version"
    );
    let paint = list(&[red, blue]);
    cache.finish_frame(&mut registry, &paint).unwrap();
    assert_eq!(cache.stats().resident_images, 2);
    assert_eq!(cache.stats().frame_pinned_images, 2);
    assert_eq!(cache.stats().resident_bytes, 16); // normalized RGBA16F, not source RGBA8
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(16, 8)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &paint,
            &registry,
            &output.view,
            output.descriptor.format,
            (16, 8),
            1.,
            [0.; 4],
        )
        .unwrap();
    // GPU submission owns its bindings. Source retirement need not block/map it.
    cache.finish_frame(&mut registry, &list(&[blue])).unwrap();
    assert!(registry.get(red).is_none());
    assert!(registry.get(blue).is_some());
    assert_eq!(cache.stats().evictions, 1);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = capture.try_finish(&gpu).unwrap().unwrap();
    assert_eq!(
        &capture.pixels()[(4 * 16 + 4) * 4..(4 * 16 + 4) * 4 + 4],
        [255, 0, 0, 255]
    );
    assert_eq!(
        &capture.pixels()[(4 * 16 + 12) * 4..(4 * 16 + 12) * 4 + 4],
        [0, 0, 255, 255]
    );
    cache.clear(&mut registry);
    assert!(registry.get(blue).is_none());
    assert_eq!(cache.stats(), Default::default());
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn retention_is_lru_bounded_and_retained_lists_pin_without_new_source_requests() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = UiImageCache::with_budget(16, 2);
    let source = source([255; 4]);
    let a = cache
        .frame(&mut registry)
        .image("a", &source, false)
        .unwrap();
    let b = cache
        .frame(&mut registry)
        .image("b", &source, false)
        .unwrap();
    cache.finish_frame(&mut registry, &list(&[a, b])).unwrap();
    let c = cache
        .frame(&mut registry)
        .image("c", &source, false)
        .unwrap();
    // No source request for A: a retained graph/view list is sufficient to pin it.
    cache.finish_frame(&mut registry, &list(&[a, c])).unwrap();
    assert!(registry.get(a).is_some());
    assert!(registry.get(c).is_some());
    assert!(registry.get(b).is_none());
    assert_eq!(cache.stats().resident_bytes, 16);
    assert!(!cache.stats().over_budget);
    let d = cache
        .frame(&mut registry)
        .image("d", &source, false)
        .unwrap();
    cache
        .finish_frame(&mut registry, &list(&[a, c, d]))
        .unwrap();
    assert_eq!(cache.stats().frame_pinned_images, 3);
    assert_eq!(cache.stats().resident_images, 3);
    assert!(
        cache.stats().over_budget,
        "active working sets are reported, never made stale"
    );
    // Touch A after D and C, then omit every source. The least recently used
    // source C goes first; byte and image limits both return to their budget.
    let uploaded = gpu.transfers().uploaded_bytes;
    assert_eq!(
        cache
            .frame(&mut registry)
            .image("a", &source, false)
            .unwrap(),
        a
    );
    cache
        .finish_frame(&mut registry, &PaintList::default())
        .unwrap();
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
    assert!(registry.get(a).is_some());
    assert!(registry.get(d).is_some());
    assert!(registry.get(c).is_none());
    assert_eq!(cache.stats().resident_images, 2);
    assert_eq!(cache.stats().frame_pinned_images, 0);
    assert!(!cache.stats().over_budget);
    // Reusing an evicted key uploads once with a new generation/slot identity.
    let c2 = cache
        .frame(&mut registry)
        .image("c", &source, false)
        .unwrap();
    assert_ne!(c, c2);
    cache.finish_frame(&mut registry, &list(&[c2])).unwrap();
    assert!(cache.stats().resident_images <= 2);
    assert!(cache.stats().resident_bytes <= 16);
    assert_eq!(cache.stats().uploads, 5);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn effects_meshes_and_opacity_groups_pin_every_source_and_invalid_finish_is_atomic() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = UiImageCache::with_budget(0, 0);
    let mut handles = Vec::new();
    for key in ["image", "filter", "outline", "mesh"] {
        handles.push(
            cache
                .frame(&mut registry)
                .image(key, &source([255; 4]), false)
                .unwrap(),
        );
    }
    let mut paint = list(&handles[..1]);
    paint.ops.push(PaintOp::PushOpacity(0.5));
    paint.ops.push(PaintOp::FilteredImage {
        image: handles[1],
        rect: Rect::new(0., 0., 8., 8.),
        uv: Rect::new(0., 0., 1., 1.),
        sampling: Sampling::Nearest,
        tint: [1.; 4],
        filter: ImageColorFilter::default(),
    });
    paint.ops.push(PaintOp::ImageOutline {
        image: handles[2],
        rect: Rect::new(0., 0., 8., 8.),
        uv: Rect::new(0., 0., 1., 1.),
        sampling: Sampling::Nearest,
        width: 1.,
        color: [1.; 4],
    });
    paint.ops.push(PaintOp::ImageMesh {
        image: handles[3],
        vertices: vec![
            ImageVertex {
                position: [0., 0.],
                uv: [0., 0.],
            },
            ImageVertex {
                position: [8., 0.],
                uv: [1., 0.],
            },
            ImageVertex {
                position: [0., 8.],
                uv: [0., 1.],
            },
        ]
        .into(),
        indices: vec![0, 1, 2].into(),
        sampling: Sampling::Nearest,
        tint: [1.; 4],
    });
    paint.ops.push(PaintOp::PopOpacity);
    cache.finish_frame(&mut registry, &paint).unwrap();
    assert_eq!(cache.stats().frame_pinned_images, 4);
    assert_eq!(cache.stats().resident_images, 4);
    let before = cache.stats();
    let malformed = PaintList {
        ops: vec![PaintOp::PopClip],
    };
    assert!(cache.finish_frame(&mut registry, &malformed).is_err());
    assert_eq!(cache.stats(), before);
    let mut foreign = ImageRegistry::new(gpu);
    let alien = foreign
        .upload_linear(&ImageAsset::rgba(1, 1, vec![255; 4]).unwrap())
        .unwrap();
    assert!(cache.finish_frame(&mut registry, &list(&[alien])).is_err());
    assert_eq!(cache.stats(), before);
    assert!(handles.iter().all(|handle| registry.get(*handle).is_some()));
    cache
        .finish_frame(&mut registry, &PaintList::default())
        .unwrap();
    assert_eq!(cache.stats().resident_images, 0);
    assert_eq!(cache.stats().resident_bytes, 0);
    assert_eq!(cache.stats().evictions, 4);
    assert!(foreign.get(alien).is_some());
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn dimension_changes_invalid_uploads_byte_budget_and_registry_replacement_are_explicit() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = UiImageCache::with_budget(8, 512);
    let small = cache
        .frame(&mut registry)
        .image("preview", &source([255; 4]), false)
        .unwrap();
    let large = TheRGBABuffer::from(vec![255; 4 * 4 * 4], 4, 4);
    let enlarged = cache
        .frame(&mut registry)
        .image("preview", &large, false)
        .unwrap();
    assert_ne!(
        small, enlarged,
        "changed dimensions cannot reuse an obsolete texture"
    );
    let before = cache.stats();
    assert!(
        cache
            .frame(&mut registry)
            .image("preview", &TheRGBABuffer::empty(), true)
            .is_err()
    );
    assert_eq!(cache.stats(), before);
    cache
        .finish_frame(&mut registry, &list(&[enlarged]))
        .unwrap();
    assert_eq!(cache.stats().resident_bytes, 128);
    assert_eq!(cache.stats().resident_images, 1);
    assert!(cache.stats().over_budget);
    assert!(registry.get(small).is_none());
    cache
        .finish_frame(&mut registry, &PaintList::default())
        .unwrap();
    assert_eq!(
        cache.stats().resident_bytes,
        0,
        "byte limit applies independently of entry count"
    );
    let old = cache
        .frame(&mut registry)
        .image("preview", &source([255; 4]), false)
        .unwrap();
    let mut replacement = ImageRegistry::new(gpu);
    let new = cache
        .frame(&mut replacement)
        .image("preview", &source([255; 4]), false)
        .unwrap();
    assert_ne!(old.owner, new.owner);
    cache.finish_frame(&mut replacement, &list(&[new])).unwrap();
    assert_eq!(cache.stats().uploads, 1);
    assert_eq!(cache.stats().resident_bytes, 8);
    cache.clear(&mut replacement);
    assert!(replacement.get(new).is_none());
    assert!(
        registry.get(old).is_some(),
        "the old domain is owned and destroyed by its host"
    );
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn ui_application_finalizes_after_all_custom_contributions() {
    use theframework::{platform::*, render_wgpu::UiFrameImages};
    struct App {
        versions: Option<(ImageHandle, ImageHandle)>,
        frames: usize,
    }
    impl UiApplication for App {
        fn initialize(&mut self, _: &mut UiSession, _: &mut UiFrameImages<'_>) -> Result<()> {
            Ok(())
        }
        fn frame(
            &mut self,
            ui: &mut UiSession,
            images: &mut UiFrameImages<'_>,
        ) -> Result<PaintList> {
            // UI and multiple custom contributors can refer to different
            // revisions of one source. Finalization belongs above all of them.
            let mut paint = ui.paint(images).map_err(|e| RenderError(e.to_string()))?;
            let (old, new) = if let Some(versions) = self.versions {
                versions
            } else {
                let old = images
                    .image("shared", &source([255, 0, 0, 255]), false)
                    .map_err(|e| RenderError(e.to_string()))?;
                let new = images
                    .image("shared", &source([0, 0, 255, 255]), true)
                    .map_err(|e| RenderError(e.to_string()))?;
                self.versions = Some((old, new));
                (old, new)
            };
            if self.frames == 0 {
                paint.ops.extend(list(&[old]).ops);
            }
            paint.ops.extend(list(&[new]).ops);
            self.frames += 1;
            Ok(paint)
        }
    }
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu);
    let mut host = HostContext::new((100, 80));
    let mut app = UiApplicationAdapter::new(App {
        versions: None,
        frames: 0,
    });
    app.initialize(&mut host, &mut registry).unwrap();
    let first = app.frame(&mut host, &mut registry).unwrap();
    let (old, new) = app.application.versions.unwrap();
    assert_eq!(first.image_handles().count(), 2);
    assert!(registry.get(old).is_some());
    assert!(registry.get(new).is_some());
    let second = app.frame(&mut host, &mut registry).unwrap();
    assert_eq!(second.image_handles().collect::<Vec<_>>(), vec![new]);
    assert!(registry.get(old).is_none());
    assert!(registry.get(new).is_some());
    assert!(app.session.ui.canvas.buffer.pixels().is_empty());
}
