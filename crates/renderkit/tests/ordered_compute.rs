use renderkit::{GpuContext, OrderedCompute, StatusReadback, wgpu};

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn large_ordered_compute_keeps_storage_order_without_exhausting_backend_commands() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let storage = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ordered-compute-state"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let shader = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ordered-compute-fixture"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@group(0) @binding(0) var<storage, read_write> state: array<u32>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) count: vec3<u32>) {
    if id.x == 0u { state[0] = state[0] * 3u + count.x; }
}
"#
                .into(),
            ),
        });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage.as_entire_binding(),
        }],
    });
    let mut work = OrderedCompute::new(&gpu, "large-ordered-compute-fixture");
    let mut expected = 0u32;
    for index in 0..4101u32 {
        let count = index % 7 + 1;
        expected = expected.wrapping_mul(3).wrapping_add(count);
        let mut pass = work.begin_pass();
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
    assert!(work.finish() > 1);
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut copy = gpu.device.create_command_encoder(&Default::default());
    copy.copy_buffer_to_buffer(&storage, 0, &staging, 0, 4);
    gpu.queue.submit([copy.finish()]);
    let mut result = StatusReadback::map(&gpu, staging).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    assert_eq!(result.try_finish(&gpu).unwrap(), Some(expected));
    assert!(!gpu.is_lost());
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert_eq!(gpu.transfers().status_readback_bytes, 4);
}
