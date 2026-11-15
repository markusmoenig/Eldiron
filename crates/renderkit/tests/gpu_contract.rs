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
    let shared = ctx.as_ref().clone();
    assert_eq!(shared.capture_queue_usage().bytes, 512);
    assert_eq!(shared.capture_queue_usage().requests, 1);
    let capacity = shared.capture_queue_usage().byte_limit;
    let full = shared.reserve_capture(capacity - 512).unwrap();
    let transfers = ctx.transfers();
    assert!(ReadbackTicket::capture_asset(&ctx, a.get(new).unwrap()).is_err());
    assert_eq!(
        ctx.transfers(),
        transfers,
        "rejection must precede allocation/submission/accounting"
    );
    drop(full);
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    let capture = ticket.try_finish(&ctx).unwrap().unwrap();
    assert_eq!(capture, source);
    assert!(ticket.try_finish(&ctx).is_err());
    assert_eq!(ctx.capture_queue_usage().bytes, 0);
    assert_eq!(ctx.capture_queue_usage().requests, 0);
    let pending: Vec<_> = (0..ctx.capture_queue_usage().request_limit)
        .map(|_| ReadbackTicket::capture(&ctx, a.get(new).unwrap()).unwrap())
        .collect();
    let full = ctx.capture_queue_usage();
    let transfers = ctx.transfers();
    assert!(ReadbackTicket::capture(&shared, a.get(new).unwrap()).is_err());
    assert_eq!(ctx.capture_queue_usage(), full);
    assert_eq!(ctx.transfers(), transfers);
    drop(pending);
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    assert_eq!(ctx.capture_queue_usage().bytes, 0);
    assert_eq!(ctx.capture_queue_usage().requests, 0);
    // Actual loss must wake idle owners, while retired listener leases do not.
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let wakes = Arc::new(AtomicUsize::new(0));
    let count = wakes.clone();
    let live = ctx.observe_faults(move || {
        count.fetch_add(1, Ordering::SeqCst);
    });
    let count = wakes.clone();
    let retired = ctx.observe_faults(move || {
        count.fetch_add(100, Ordering::SeqCst);
    });
    drop(retired);
    let mut ready_before_loss = ReadbackTicket::capture(&ctx, a.get(new).unwrap()).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    ctx.device.destroy();
    for _ in 0..100 {
        let _ = ctx.poll();
        if ctx.is_lost() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(ctx.is_lost());
    assert!(ctx.device_loss_reason().is_some());
    assert!(
        ready_before_loss
            .try_finish(&ctx)
            .unwrap_err()
            .to_string()
            .contains("lost")
    );
    drop(ready_before_loss);
    assert_eq!(ctx.capture_queue_usage().requests, 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    let count = wakes.clone();
    let late = ctx.observe_faults(move || {
        count.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(
        wakes.load(Ordering::SeqCst),
        2,
        "a late observer must not miss loss"
    );
    drop((live, late));
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
