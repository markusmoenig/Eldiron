use renderkit::{GpuContext, StatusReadback, wgpu};
fn submitted(gpu: &GpuContext, value: u32) -> StatusReadback {
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("status contract"),
        size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue.write_buffer(&buffer, 0, &value.to_le_bytes());
    gpu.queue.submit([]);
    StatusReadback::map(gpu, buffer).unwrap()
}
fn finish(gpu: &GpuContext) {
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
}
#[test]
#[ignore = "requires native GPU"]
fn status_words_are_async_cancellable_isolated_and_counted_separately_from_color() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut read = submitted(&gpu, 0x10203040);
    finish(&gpu);
    assert_eq!(read.try_finish(&gpu).unwrap(), Some(0x10203040));
    assert!(read.try_finish(&gpu).is_err());
    // Cancel both an in-flight and a completed-but-unconsumed callback. Neither
    // callback may read a buffer after its owner has unmapped/dropped it.
    drop(submitted(&gpu, 1));
    finish(&gpu);
    let old = submitted(&gpu, 2);
    finish(&gpu);
    drop(old);
    let mut current = submitted(&gpu, 3);
    finish(&gpu);
    assert_eq!(current.try_finish(&gpu).unwrap(), Some(3));
    let mut replaced = submitted(&gpu, 4);
    let other = pollster::block_on(GpuContext::headless()).unwrap();
    assert!(
        replaced
            .try_finish(&other)
            .unwrap_err()
            .to_string()
            .contains("replacement")
    );
    drop(replaced);
    finish(&gpu);
    let counters = gpu.transfers();
    assert_eq!(counters.status_readback_bytes, 20);
    assert_eq!(counters.readback_bytes, 0);
    assert_eq!(counters.capture_requests, 0);
    let invalid = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 8,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    assert!(StatusReadback::map(&gpu, invalid).is_err());
    assert_eq!(gpu.transfers(), counters);
}
