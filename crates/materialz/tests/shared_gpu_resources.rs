#![cfg(feature = "shared-gpu")]
use denrim_materials::{gpu::*, integration::*};
use renderkit::GpuContext;
use std::{
    sync::mpsc,
    time::Duration,
};
struct Capture {
    buffer: wgpu::Buffer,
    stride: u32,
    width_bytes: u32,
    height: u32,
}
fn capture(
    context: &GpuContext,
    encoder: &mut wgpu::CommandEncoder,
    surface: &Surface,
) -> Vec<Capture> {
    surface
        .maps
        .iter()
        .map(|map| {
            let width_bytes =
                surface.resolution[0] * map.layout.format.block_copy_size(None).unwrap();
            let stride = width_bytes.div_ceil(256) * 256;
            let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("explicit shared resource comparison"),
                size: u64::from(stride) * u64::from(surface.resolution[1]),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                map.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(surface.resolution[1]),
                    },
                },
                map.texture.size(),
            );
            Capture {
                buffer,
                stride,
                width_bytes,
                height: surface.resolution[1],
            }
        })
        .collect()
}
fn finish(context: &GpuContext, captures: Vec<Capture>, mut ticket: RenderTicket) -> Vec<Vec<u8>> {
    ticket.submitted();
    let (done, ready) = mpsc::channel();
    ticket.map_diagnostics(move |result| {
        done.send(result).unwrap();
    });
    let mut receivers = Vec::new();
    for capture in &captures {
        let (done, ready) = mpsc::channel();
        capture
            .buffer
            .map_async(wgpu::MapMode::Read, .., move |result| {
                done.send(result).unwrap();
            });
        receivers.push(ready);
    }
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    ready.recv().unwrap().unwrap();
    captures
        .into_iter()
        .zip(receivers)
        .map(|(capture, ready)| {
            ready.recv().unwrap().unwrap();
            let data = capture.buffer.get_mapped_range(..);
            let mut pixels = Vec::new();
            for row in 0..capture.height {
                let start = (row * capture.stride) as usize;
                pixels.extend_from_slice(&data[start..start + capture.width_bytes as usize]);
            }
            drop(data);
            capture.buffer.unmap();
            pixels
        })
        .collect()
}
async fn compile(runtime: &MaterialZ) -> MaterialInstance {
    let asset = EditableMaterialAsset::parse(
        "shared-resource-fixture",
        include_str!("../examples/filtered-basketweave.material"),
        "filtered-basketweave",
    )
    .unwrap();
    runtime
        .compile(&asset, InstanceRecord::new("shared-resource-fixture"))
        .await
        .unwrap()
}
#[test]
#[ignore = "native GPU; shared MaterialZ lease, zeroing and pixel contract"]
fn shared_material_maps_and_work_survive_pending_submission_and_reuse() {
    pollster::block_on(async {
        let context = GpuContext::headless().await.unwrap();
        let legacy = MaterialZ::new(&context.device, &context.queue)
            .await
            .unwrap();
        let baseline_material = compile(&legacy).await;
        let runtime = MaterialZ::new_with_context(context.clone()).await.unwrap();
        let material = compile(&runtime).await;
        let mut baseline = Vec::new();
        for time in [0f32, 0.8] {
            let request = RenderRequest {
                time,
                resolution: [16, 16],
                domain: SurfaceDomain::rect([-0.5; 2], [1.; 2]),
            };
            let surface = legacy.create_surface(&request).unwrap();
            let mut encoder = context.device.create_command_encoder(&Default::default());
            let ticket = legacy
                .encode(&mut encoder, &baseline_material, &request, &surface)
                .unwrap();
            let captured = capture(&context, &mut encoder, &surface);
            context.queue.submit([encoder.finish()]);
            baseline.push(finish(&context, captured, ticket));
        }
        let mut pending = Vec::new();
        for time in [0f32, 0.8] {
            let request = RenderRequest {
                time,
                resolution: [16, 16],
                domain: SurfaceDomain::rect([-0.5; 2], [1.; 2]),
            };
            let surface = runtime.create_surface(&request).unwrap();
            let mut encoder = context.device.create_command_encoder(&Default::default());
            let ticket = runtime
                .encode(&mut encoder, &material, &request, &surface)
                .unwrap();
            let captured = capture(&context, &mut encoder, &surface);
            // The ticket must retain every output and scratch lease even after
            // the source surface is gone and another encode is prepared.
            drop(surface);
            pending.push((encoder, ticket, captured));
        }
        assert_eq!(context.target_pool_usage().allocations, 10);
        assert_eq!(context.target_pool_usage().hits, 0);
        let work_allocations = context.working_buffer_pool_usage().allocations;
        assert!(work_allocations >= 2);
        let second = pending.pop().unwrap();
        let first = pending.pop().unwrap();
        context.queue.submit([second.0.finish(), first.0.finish()]);
        assert_eq!(finish(&context, first.2, first.1), baseline[0]);
        assert_eq!(finish(&context, second.2, second.1), baseline[1]);
        // Recycle now-idle maps/work. Old atomic flags and partially written
        // intermediate data must be cleared, preserving the standalone bytes.
        let request = RenderRequest {
            time: 0.,
            resolution: [16, 16],
            domain: SurfaceDomain::rect([-0.5; 2], [1.; 2]),
        };
        let surface = runtime.create_surface(&request).unwrap();
        let mut encoder = context.device.create_command_encoder(&Default::default());
        let ticket = runtime
            .encode(&mut encoder, &material, &request, &surface)
            .unwrap();
        let captured = capture(&context, &mut encoder, &surface);
        assert_eq!(context.target_pool_usage().allocations, 10);
        assert_eq!(
            context.working_buffer_pool_usage().allocations,
            work_allocations
        );
        context.queue.submit([encoder.finish()]);
        assert_eq!(finish(&context, captured, ticket), baseline[0]);
        drop(surface);
        context.set_target_cache_limits(0, 0);
        context.set_working_buffer_cache_limits(0, 0);
        context.set_immutable_buffer_cache_limits(0, 0);
        assert_eq!(context.target_pool_usage().live_bytes, 0);
        assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
        assert_eq!(context.immutable_buffer_cache_usage().live_bytes, 0);
        assert!(context.immutable_buffer_cache_usage().peak_live_bytes > 0);
        let replacement = GpuContext::headless().await.unwrap();
        let rebuilt = MaterialZ::new_with_context(replacement.clone())
            .await
            .unwrap();
        let rebuilt_material = compile(&rebuilt).await;
        let surface = rebuilt.create_surface(&request).unwrap();
        let mut encoder = replacement
            .device
            .create_command_encoder(&Default::default());
        let ticket = rebuilt
            .encode(&mut encoder, &rebuilt_material, &request, &surface)
            .unwrap();
        let captured = capture(&replacement, &mut encoder, &surface);
        replacement.queue.submit([encoder.finish()]);
        assert_eq!(finish(&replacement, captured, ticket), baseline[0]);
        assert_ne!(replacement.generation(), context.generation());
    });
}
