use renderkit::*;
use std::sync::Arc;

#[test]
#[ignore = "requires native GPU; shared-page cache budget and external owners"]
fn range_budget_counts_shared_pages_once_and_preserves_external_owners() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let usage = wgpu::BufferUsages::VERTEX;
    let first = gpu.upload_immutable_range(&[1; 3], usage).unwrap();
    let second = gpu.upload_immutable_range(&[2; 5], usage).unwrap();
    assert_eq!(first.buffer(), second.buffer());
    assert_eq!((first.size(), second.size()), (4, 8));
    let before = gpu.immutable_range_cache_usage();
    assert_eq!(before.live_allocations, 1);
    assert_eq!(before.gpu_bytes, first.buffer().size());
    assert_eq!(before.cpu_bytes, 8);
    assert_eq!(before.uploaded_bytes, 12);
    gpu.set_immutable_range_cache_limits(before.gpu_bytes + 5, 1);
    let trimmed = gpu.immutable_range_cache_usage();
    assert_eq!(trimmed.entries, 1);
    assert_eq!(trimmed.gpu_bytes, before.gpu_bytes);
    assert_eq!(trimmed.cpu_bytes, 5);
    assert_eq!(trimmed.live_allocations, 1);
    assert!(Arc::ptr_eq(
        &second,
        &gpu.upload_immutable_range(&[2; 5], usage).unwrap()
    ));
    gpu.set_immutable_range_cache_limits(0, 0);
    let evicted = gpu.immutable_range_cache_usage();
    assert_eq!(
        (evicted.entries, evicted.gpu_bytes, evicted.cpu_bytes),
        (0, 0, 0)
    );
    assert_eq!(evicted.live_bytes, before.live_bytes);
    drop(first);
    assert_eq!(gpu.immutable_range_cache_usage().live_allocations, 1);
    drop(second);
    assert_eq!(gpu.immutable_range_cache_usage().live_bytes, 0);
}

#[test]
#[ignore = "requires native GPU; shared input allocation and queued ownership"]
fn immutable_ranges_pack_exact_bytes_and_survive_eviction_without_reusing_published_storage() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC;
    let mut ranges = Vec::new();
    for word in 0..2048u32 {
        let data = [word, word ^ 0x12345678, 0, 17];
        let bytes = data
            .into_iter()
            .flat_map(u32::to_ne_bytes)
            .collect::<Vec<_>>();
        let range = gpu.upload_immutable_range(&bytes, usage).unwrap();
        assert_eq!(
            range.offset() % u64::from(gpu.device.limits().min_uniform_buffer_offset_alignment),
            0
        );
        assert_eq!(range.device_generation(), gpu.generation());
        ranges.push(range);
    }
    let usage_before = gpu.immutable_range_cache_usage();
    assert!(
        usage_before.live_allocations <= 16,
        "small inputs must share physical allocations: {usage_before:?}"
    );
    assert_eq!(usage_before.entries, 2048);
    assert!(usage_before.gpu_bytes + usage_before.cpu_bytes <= usage_before.byte_limit);
    let bytes = [0u32, 0x12345678, 0, 17]
        .into_iter()
        .flat_map(u32::to_ne_bytes)
        .collect::<Vec<_>>();
    let shared = gpu.upload_immutable_range(&bytes, usage).unwrap();
    assert!(Arc::ptr_eq(&shared, &ranges[0]));
    let invalid_before = gpu.immutable_range_cache_usage();
    for invalid in [
        wgpu::BufferUsages::COPY_DST,
        wgpu::BufferUsages::MAP_WRITE,
        wgpu::BufferUsages::empty(),
    ] {
        assert!(gpu.upload_immutable_range(&[0; 4], invalid).is_err());
    }
    assert_eq!(gpu.immutable_range_cache_usage(), invalid_before);
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 32,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    for (at, index) in [(0, 0), (16, 2047)] {
        encoder.copy_buffer_to_buffer(
            ranges[index].buffer(),
            ranges[index].offset(),
            &staging,
            at,
            16,
        );
    }
    let old_buffer = ranges.last().unwrap().buffer().clone();
    gpu.set_immutable_range_cache_limits(0, 0);
    assert_eq!(gpu.immutable_range_cache_usage().gpu_bytes, 0);
    drop((ranges, shared));
    assert_eq!(gpu.immutable_range_cache_usage().live_bytes, 0);
    let replacement = gpu.upload_immutable_range(&[99; 16], usage).unwrap();
    assert_ne!(
        old_buffer,
        *replacement.buffer(),
        "queued raw handles must never be repurposed as an empty page"
    );
    gpu.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging.map_async(wgpu::MapMode::Read, .., move |result| {
        tx.send(result).unwrap();
    });
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = staging.get_mapped_range(..);
    assert_eq!(u32::from_ne_bytes(data[0..4].try_into().unwrap()), 0);
    assert_eq!(u32::from_ne_bytes(data[16..20].try_into().unwrap()), 2047);
    drop(data);
    staging.unmap();
}

#[test]
#[ignore = "requires native GPU; dynamic range bindings and immutable offsets"]
fn one_binding_selects_distinct_immutable_ranges_after_cache_retirement() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let input = |word: u32| {
        gpu.upload_immutable_range(
            &[word, 0, 0, 0]
                .into_iter()
                .flat_map(u32::to_ne_bytes)
                .collect::<Vec<_>>(),
            wgpu::BufferUsages::UNIFORM,
        )
        .unwrap()
    };
    let first = input(7);
    let second = input(99);
    let keep = input(10);
    assert_eq!(first.buffer(), second.buffer());
    let output = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let layout = gpu
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: first.dynamic_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl("@group(0) @binding(0) var<uniform> input: vec4<u32>; @group(0) @binding(1) var<storage,read_write> output: array<u32>; @compute @workgroup_size(1) fn main() { output[0] = output[0] * 3u + input.x; }".into()),
    });
    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    for offset in [first.offset(), second.offset()] {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[offset as u32]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    let end = keep.offset() + keep.size();
    gpu.set_immutable_range_cache_limits(0, 0);
    drop((first, second));
    let later = input(123);
    assert_eq!(keep.buffer(), later.buffer());
    assert!(
        later.offset() >= end,
        "retired slices on a live allocation must never be overwritten"
    );
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, 4);
    gpu.queue.submit([encoder.finish()]);
    let mut result = StatusReadback::map(&gpu, staging).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    assert_eq!(result.try_finish(&gpu).unwrap(), Some(120));
}
