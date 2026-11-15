use renderkit::*;
use std::sync::mpsc;

#[test]
#[ignore = "requires native GPU; writable leases, reuse and eviction before submission"]
fn writable_leases_reuse_only_after_retirement_and_preserve_queued_contents() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    context.set_working_buffer_cache_limits(64, 4);
    let descriptor = WorkingBufferDescriptor::storage(16);
    let first = context.acquire_working_buffer(descriptor).unwrap();
    let second = context.acquire_working_buffer(descriptor).unwrap();
    assert_ne!(first.buffer(), second.buffer());
    context.queue.write_buffer(first.buffer(), 0, &[1; 16]);
    context.queue.write_buffer(second.buffer(), 0, &[2; 16]);
    let raw_first = first.buffer().clone();
    let held = first.clone();
    drop(first);
    let third = context.acquire_working_buffer(descriptor).unwrap();
    assert_ne!(held.buffer(), third.buffer(), "cloned leases pin storage");
    assert_eq!(context.working_buffer_pool_usage().allocations, 3);
    context.queue.write_buffer(third.buffer(), 0, &[3; 16]);
    drop(held);
    let clone = (*context).clone();
    let reused = clone.acquire_working_buffer(descriptor).unwrap();
    assert_eq!(reused.buffer(), &raw_first);
    assert_eq!(context.working_buffer_pool_usage().allocations, 3);
    assert_eq!(context.working_buffer_pool_usage().hits, 1);
    assert_eq!(reused.device_generation(), context.generation());
    // Reused bytes are deliberately overwritten, never assumed zero/fresh.
    context.queue.write_buffer(reused.buffer(), 0, &[4; 16]);
    let different_usage = context
        .acquire_working_buffer(WorkingBufferDescriptor {
            bytes: 16,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        })
        .unwrap();
    assert_ne!(different_usage.buffer(), reused.buffer());
    let larger = context
        .acquire_working_buffer(WorkingBufferDescriptor::storage(32))
        .unwrap();
    assert_eq!(larger.buffer().size(), 32);
    let usage = context.working_buffer_pool_usage();
    assert!(usage.cached_bytes <= 64 && usage.cached_buffers <= 4 && usage.evictions > 0);
    let staging = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("working buffer ownership test capture"),
        size: 48,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    for (offset, lease) in [(0, &reused), (16, &second), (32, &third)] {
        encoder.copy_buffer_to_buffer(lease.buffer(), 0, &staging, offset, 16);
    }
    // Eviction during encoding cannot destroy resources in active/queued work.
    context.set_working_buffer_cache_limits(0, 0);
    assert_eq!(context.working_buffer_pool_usage().cached_bytes, 0);
    assert_eq!(context.working_buffer_pool_usage().cached_buffers, 0);
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 96);
    assert_eq!(context.working_buffer_pool_usage().live_allocations, 5);
    assert_eq!(context.working_buffer_pool_usage().peak_live_bytes, 96);
    context.queue.submit([encoder.finish()]);
    drop((reused, second, third, different_usage, larger, raw_first));
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
    assert_eq!(context.working_buffer_pool_usage().live_allocations, 0);
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
    {
        let mapped = staging.slice(..).get_mapped_range();
        assert_eq!(&mapped[..16], &[4; 16]);
        assert_eq!(&mapped[16..32], &[2; 16]);
        assert_eq!(&mapped[32..], &[3; 16]);
    }
    staging.unmap();
    let uncached = context.acquire_working_buffer(descriptor).unwrap();
    assert_eq!(context.working_buffer_pool_usage().uncached_allocations, 1);
    let before = context.working_buffer_pool_usage();
    for invalid in [
        WorkingBufferDescriptor::storage(0),
        WorkingBufferDescriptor::storage(3),
        WorkingBufferDescriptor::storage(context.device.limits().max_buffer_size + 4),
        WorkingBufferDescriptor {
            bytes: 16,
            usage: wgpu::BufferUsages::empty(),
        },
        WorkingBufferDescriptor {
            bytes: 16,
            usage: wgpu::BufferUsages::MAP_WRITE,
        },
    ] {
        assert!(context.acquire_working_buffer(invalid).is_err());
    }
    assert_eq!(context.working_buffer_pool_usage(), before);
    let replacement = pollster::block_on(GpuContext::headless()).unwrap();
    let rebuilt = replacement.acquire_working_buffer(descriptor).unwrap();
    assert_ne!(rebuilt.device_generation(), uncached.device_generation());
    assert_eq!(replacement.working_buffer_pool_usage().allocations, 1);
    assert_eq!(replacement.working_buffer_pool_usage().hits, 0);
    replacement.device.destroy();
    for _ in 0..100 {
        let _ = replacement.poll();
        if replacement.is_lost() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(replacement.is_lost());
    let before_loss_request = replacement.working_buffer_pool_usage();
    assert!(replacement.acquire_working_buffer(descriptor).is_err());
    assert_eq!(replacement.working_buffer_pool_usage(), before_loss_request);
}
