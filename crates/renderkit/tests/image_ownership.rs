use renderkit::{
    GpuContext, GpuImage, ImageAsset, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu,
};

#[test]
#[ignore = "native GPU; registry/frame/capture ownership of unpooled images"]
fn unpooled_image_charges_follow_last_owner_without_charging_pooled_or_imported_images() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(context.clone());
    let source = ImageAsset::rgba(1, 1, vec![230, 40, 20, 255]).unwrap();
    let original = registry.upload(&source).unwrap();
    let retained = registry.get(original).unwrap();
    let duplicate = registry.insert(retained.clone()).unwrap();
    assert_eq!(context.image_allocation_usage().live_bytes, 4);
    assert_eq!(context.image_allocation_usage().live_images, 1);
    assert_eq!(context.image_allocation_usage().allocations, 1);
    let pooled = GpuImage::render_target(&context, ImageDescriptor::rgba(2, 2)).unwrap();
    assert_eq!(context.image_allocation_usage().live_bytes, 4);
    let imported =
        GpuImage::from_texture(&context, retained.texture.clone(), retained.descriptor).unwrap();
    assert_eq!(context.image_allocation_usage().live_images, 1);
    let linear = registry.upload_linear(&source).unwrap();
    assert_eq!(context.image_allocation_usage().live_bytes, 4 + 8);
    assert_eq!(context.image_allocation_usage().live_images, 2);
    assert_eq!(context.image_allocation_usage().allocations, 2);
    let mut capture = ReadbackTicket::capture(&context, retained.clone()).unwrap();
    assert!(registry.remove(original));
    assert!(registry.get(original).is_none());
    assert!(registry.remove(duplicate));
    assert!(registry.remove(linear));
    drop((retained, registry));
    assert_eq!(context.image_allocation_usage().live_bytes, 4);
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    assert_eq!(
        capture.try_finish(&context).unwrap().unwrap().pixels(),
        &[230, 40, 20, 255]
    );
    assert_eq!(context.image_allocation_usage().live_images, 1);
    drop(capture);
    assert_eq!(context.image_allocation_usage().live_bytes, 0);
    assert_eq!(context.image_allocation_usage().live_images, 0);
    assert_eq!(context.image_allocation_usage().peak_live_bytes, 12);
    // Imported raw clones are deliberately outside allocation-owner totals.
    drop((imported, pooled));
    assert_eq!(context.image_allocation_usage().live_bytes, 0);
}
