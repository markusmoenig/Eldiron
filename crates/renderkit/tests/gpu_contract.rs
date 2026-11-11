use renderkit::*;
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn resources_reject_stale_handles_and_capture_padded_rows() {
    let ctx = pollster::block_on(GpuContext::headless()).unwrap();
    let mut a = ImageRegistry::new(ctx.clone());
    let b = ImageRegistry::new(ctx.clone());
    let source = ImageAsset::rgba(3, 2, vec![17; 24]).unwrap();
    let old = a.upload(&source).unwrap();
    let image = a.get(old).unwrap();
    assert!(b.get(old).is_none());
    assert!(a.remove(old));
    assert!(a.get(old).is_none());
    let new = a.upload(&source).unwrap();
    assert_ne!(old, new);
    assert_eq!(old.slot, new.slot);
    let mut ticket = ReadbackTicket::capture(&ctx, image).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    let capture = ticket.try_finish(&ctx).unwrap().unwrap();
    assert_eq!(capture, source);
    assert!(ticket.try_finish(&ctx).is_err());
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires native Metal; run with --ignored"]
fn rust_surface_accepts_foreign_metal_layer_without_winit() {
    let layer = objc2_quartz_core::CAMetalLayer::new();
    let pointer = (&*layer as *const objc2_quartz_core::CAMetalLayer)
        .cast_mut()
        .cast();
    // The foreign layer is retained locally until after the surface is dropped.
    let mut surface =
        pollster::block_on(unsafe { SurfaceState::from_metal_layer(pointer, (64, 64)) }).unwrap();
    assert_eq!(surface.physical_size(), (64, 64));
    surface.resize(0, 0);
    assert!(matches!(surface.acquire().unwrap(), AcquireFrame::Deferred));
    surface.resize(128, 96);
    surface.suspend();
    assert!(matches!(surface.acquire().unwrap(), AcquireFrame::Deferred));
    surface.resume();
    assert_eq!(surface.physical_size(), (128, 96));
    drop(surface);
    drop(layer);
}
