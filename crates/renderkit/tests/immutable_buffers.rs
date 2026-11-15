use renderkit::*;
use std::sync::{Arc, mpsc};

#[test]
#[ignore = "requires native GPU; immutable sharing, LRU and queued ownership"]
fn immutable_inputs_share_exact_contents_and_survive_eviction_before_submit() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    context.set_immutable_buffer_cache_limits(32, 4);
    let usage = wgpu::BufferUsages::COPY_SRC;
    let first = context.upload_immutable_buffer(&[1, 2, 3], usage).unwrap();
    let second = context
        .upload_immutable_buffer(&[4, 5, 6, 7], usage)
        .unwrap();
    let vertex = context
        .upload_immutable_buffer(&[1, 2, 3], usage | wgpu::BufferUsages::VERTEX)
        .unwrap();
    assert_ne!(first.buffer(), vertex.buffer(), "usage is part of identity");
    let clone = (*context).clone();
    let shared = clone.upload_immutable_buffer(&[1, 2, 3], usage).unwrap();
    assert!(
        Arc::ptr_eq(&first, &shared),
        "immutable content can share while both owners live"
    );
    let third = context
        .upload_immutable_buffer(&[8, 9, 10, 11], usage)
        .unwrap();
    context.set_immutable_buffer_cache_limits(32, 2);
    let before = context.immutable_buffer_cache_usage();
    let recent = context.upload_immutable_buffer(&[1, 2, 3], usage).unwrap();
    assert!(
        Arc::ptr_eq(&first, &recent),
        "recent hit survives LRU eviction"
    );
    assert_eq!(
        context.immutable_buffer_cache_usage().allocations,
        before.allocations
    );
    assert_eq!(before.gpu_bytes, 8);
    assert_eq!(before.cpu_bytes, 7);
    let invalid_before = context.immutable_buffer_cache_usage();
    for invalid in [
        wgpu::BufferUsages::COPY_DST,
        wgpu::BufferUsages::MAP_WRITE,
        wgpu::BufferUsages::empty(),
    ] {
        assert!(context.upload_immutable_buffer(&[0; 4], invalid).is_err());
    }
    assert_eq!(context.immutable_buffer_cache_usage(), invalid_before);
    let empty = context
        .upload_immutable_buffer(&[], wgpu::BufferUsages::VERTEX)
        .unwrap();
    assert_eq!(empty.buffer().size(), 4);
    let staging = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("explicit immutable input test capture"),
        size: 12,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    for (offset, buffer) in [(0, &first), (4, &second), (8, &third)] {
        encoder.copy_buffer_to_buffer(buffer.buffer(), 0, &staging, offset, 4);
    }
    // Evict and release owners before even submitting; command ownership alone
    // must preserve exact old bytes. No buffer.destroy or cache overwrite.
    context.set_immutable_buffer_cache_limits(0, 0);
    assert_eq!(context.immutable_buffer_cache_usage().gpu_bytes, 0);
    assert_eq!(context.immutable_buffer_cache_usage().cpu_bytes, 0);
    let owners = context.immutable_buffer_cache_usage();
    assert_eq!(owners.live_bytes, 20);
    assert_eq!(owners.live_allocations, 5);
    assert_eq!(owners.peak_live_bytes, 20);
    drop((first, second, third, vertex, shared, recent, empty));
    assert_eq!(context.immutable_buffer_cache_usage().live_bytes, 0);
    assert_eq!(context.immutable_buffer_cache_usage().live_allocations, 0);
    assert_eq!(context.immutable_buffer_cache_usage().peak_live_bytes, 20);
    context.queue.submit([encoder.finish()]);
    let (tx, rx) = mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    assert_eq!(
        &*staging.slice(..).get_mapped_range(),
        &[1, 2, 3, 0, 4, 5, 6, 7, 8, 9, 10, 11]
    );
    staging.unmap();
    let replacement = pollster::block_on(GpuContext::headless()).unwrap();
    let rebuilt = replacement
        .upload_immutable_buffer(&[1, 2, 3], usage)
        .unwrap();
    assert_ne!(rebuilt.device_generation(), context.generation());
    assert_eq!(replacement.immutable_buffer_cache_usage().hits, 0);
    assert_eq!(replacement.immutable_buffer_cache_usage().allocations, 1);
}
