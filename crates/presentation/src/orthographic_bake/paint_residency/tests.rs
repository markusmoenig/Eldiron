use super::*;
use shared::iso_paint::{IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner};
fn finish(context: &GpuContext) {
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
}
#[test]
#[ignore = "native GPU; bounded paint slots, pinned pressure and original uploads"]
fn bounded_pages_upload_originals_without_mutating_pinned_neighbors_or_other_devices() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut layer = IsoPaintLayer::default();
    for index in 0..3 {
        let mut page =
            IsoPaintBakedChunk::new(IsoPaintOwner::Sector(7), [7, 0, 0, 1], [index * 64, 0]);
        for pixel in page.color_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[index as u8 * 70 + 20, 80, 160, 255]);
        }
        layer.baked_chunks.insert(index.to_string(), page);
    }
    let source = Arc::new(PaintPageIndex::from_layer(&layer, 1024).unwrap());
    let keys: Vec<_> = (0..3)
        .map(|index| source.slot([7, 0, 0, 1], [index * 64, 0]).unwrap())
        .collect();
    let mut pages =
        PaintPageResidency::new(context.clone(), source.clone(), PAGE_BYTES * 2).unwrap();
    assert_eq!(pages.usage().gpu_plane_bytes, PAGE_BYTES * 2);
    assert_eq!(pages.request(keys[0]).unwrap(), PaintPageRequest::Loaded);
    assert_eq!(pages.request(keys[1]).unwrap(), PaintPageRequest::Loaded);
    assert_eq!(
        pages.request(keys[2]).unwrap(),
        PaintPageRequest::WorkingSetFull
    );
    assert_eq!(pages.usage().page_uploads, 2);
    assert_eq!(pages.usage().pinned, 2);
    pages.begin_dispatch();
    assert_eq!(pages.request(keys[2]).unwrap(), PaintPageRequest::Loaded);
    assert_eq!(pages.request(keys[1]).unwrap(), PaintPageRequest::Resident);
    assert_eq!(pages.usage().resident, 2);
    assert_eq!(pages.usage().uploaded_bytes, PAGE_BYTES * 3);
    // Returning to an ordinary CPU paint source of the same dimensions must
    // allocate its own upload storage instead of overwriting the shared atlas.
    let mut vm = scenevm::SceneVM::new_with_context_and_atlas(
        context.clone(),
        1,
        1,
        scenevm::SharedAtlas::new(64, 64),
    );
    vm.set_paint_page_binding(&context, pages.binding().unwrap())
        .unwrap();
    vm.execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute3D));
    let mut target = scenevm::render_output::SceneRenderTarget::new(context.clone(), 1, 1).unwrap();
    vm.render_gpu(&mut target, 1).unwrap();
    vm.set_raster3d_surface_paint(
        128,
        64,
        vec![255; 128 * 64 * 4],
        vec![0; 128 * 64 * 4],
        source
            .resident_entries([128, 64], &[(keys[0], [0, 0]), (keys[1], [64, 0])])
            .unwrap(),
        Vec::new(),
    );
    vm.render_gpu(&mut target, 2).unwrap();
    let mut capture =
        renderkit::ReadbackTicket::capture_asset(&context, pages.color.clone()).unwrap();
    finish(&context);
    let image = capture.try_finish(&context).unwrap().unwrap();
    for y in 0..64 {
        for x in 0..128 {
            let at = (y * 128 + x) * 4;
            assert_eq!(
                &image.pixels()[at..at + 4],
                if x < 64 {
                    &[160, 80, 160, 255]
                } else {
                    &[90, 80, 160, 255]
                }
            );
        }
    }
    assert!(pages.request(u32::MAX).is_err());
    // A final partial source row can consume spare budget instead of silently
    // reducing usable capacity below the complete supported source count.
    let columns = context.device.limits().max_texture_dimension_2d / PAGE;
    let mut many = IsoPaintLayer::default();
    let prototype = layer.baked_chunks.values().next().unwrap();
    for at in 0..=columns {
        let mut page = prototype.clone();
        page.origin = [at as i32 * 64, 0];
        many.baked_chunks.insert(at.to_string(), page);
    }
    let many = Arc::new(PaintPageIndex::from_layer(&many, 1024 * 1024).unwrap());
    let padded =
        PaintPageResidency::new(context.clone(), many, PAGE_BYTES * u64::from(columns) * 2)
            .unwrap();
    assert!(padded.usage().capacity > columns as usize);
    assert!(padded.usage().gpu_plane_bytes <= PAGE_BYTES * u64::from(columns) * 2);
    let other = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut foreign = scenevm::SceneVM::new_with_context_and_atlas(
        other.clone(),
        1,
        1,
        scenevm::SharedAtlas::new(64, 64),
    );
    assert!(
        foreign
            .set_paint_page_binding(&other, pages.binding().unwrap())
            .is_err()
    );
    assert!(PaintPageResidency::new(context.clone(), source, PAGE_BYTES - 1).is_err());
}
