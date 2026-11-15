#![cfg(feature = "gpu")]
use renderkit::{GpuContext, ReadbackTicket, wgpu};
use scenevm::isometric::{IsometricCanvas, IsometricRenderer, Particle, Uniforms};
use std::sync::Arc;

// Deliberately create the deferred inputs with GPU work, never viewport bytes.
fn produce(context: &GpuContext, canvas: &IsometricCanvas, color: u32, prelit: bool, height: f32) {
    let shader = context
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iso-gpu-input-fixture"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    r#"
@group(0) @binding(0) var<storage,read_write> albedo:array<u32>;
@group(0) @binding(1) var<storage,read_write> surfaces:array<vec2<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {{
    if id.x >= arrayLength(&albedo) {{ return; }}
    albedo[id.x]={color}u;
    surfaces[id.x]=vec2<u32>(128u | (255u<<8u) | (14u<<16u), {mode}u);
}}"#,
                    mode = (height.to_bits() & 0xffffff00) | u32::from(prelit)
                )
                .into(),
            ),
        });
    let pipeline = context
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("iso-gpu-input-fixture"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let bind = context
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: canvas.albedo().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: canvas.surfaces().as_entire_binding(),
                },
            ],
        });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    {
        let mut p = encoder.begin_compute_pass(&Default::default());
        p.set_pipeline(&pipeline);
        p.set_bind_group(0, &bind, &[]);
        let (w, h) = canvas.size();
        p.dispatch_workgroups((w * h).div_ceil(64), 1, 1);
    }
    context.queue.submit(Some(encoder.finish()));
}
fn uniforms(canvas: &IsometricCanvas) -> Uniforms {
    let (w, h) = canvas.size();
    Uniforms {
        counts: [w, h, 0, 0],
        projection: [w as f32 * 0.5, h as f32 * 0.5, 8., w as f32],
        ..Default::default()
    }
}
fn pixels(context: &GpuContext, image: Arc<renderkit::GpuImage>) -> Vec<u8> {
    let mut capture = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    capture
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_produced_isometric_canvases_light_without_implicit_cpu_storage_or_capture() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let ca = IsometricCanvas::new(&context, 17, 9).unwrap();
    let cb = IsometricCanvas::new(&context, 11, 7).unwrap();
    assert_eq!(context.working_buffer_pool_usage().live_allocations, 8);
    assert_eq!(
        context.working_buffer_pool_usage().live_bytes,
        24 * (17 * 9 + 11 * 7)
    );
    let mut a = IsometricRenderer::new(context.clone()).unwrap();
    let mut b = IsometricRenderer::new(context.clone()).unwrap();
    produce(&context, &ca, 0xff0000ff, true, 0.);
    produce(&context, &cb, 0xff00ff00, true, 0.);
    let oa = a
        .render(&ca, &uniforms(&ca), &[], &[], &[], &[], None, 1)
        .unwrap();
    let ob = b
        .render(&cb, &uniforms(&cb), &[], &[], &[], &[], None, 2)
        .unwrap();
    let repeated = a
        .render(&ca, &uniforms(&ca), &[], &[], &[], &[], None, 3)
        .unwrap();
    assert!(Arc::ptr_eq(&oa.image, &repeated.image));
    assert!(!Arc::ptr_eq(&oa.image, &ob.image));
    assert_eq!(repeated.revision, 3);
    assert!(!a.has_cpu_storage() && !a.has_readback_storage());
    assert!(!b.has_cpu_storage() && !b.has_readback_storage());
    assert_eq!(context.transfers().readback_bytes, 0);
    assert_eq!(context.transfers().capture_requests, 0);
    assert_eq!(
        context.transfers().uploaded_bytes,
        3 * (std::mem::size_of::<Uniforms>() as u64 + 96),
        "only small light metadata is uploaded; canvas bytes were GPU-produced"
    );
    let large = IsometricCanvas::new(&context, 19, 13).unwrap();
    produce(&context, &large, 0xffff0000, true, 0.);
    let resized = a
        .render(&large, &uniforms(&large), &[], &[], &[], &[], None, 4)
        .unwrap();
    assert_eq!(
        (
            resized.image.descriptor.width,
            resized.image.descriptor.height
        ),
        (19, 13)
    );
    assert!(!Arc::ptr_eq(&oa.image, &resized.image));
    assert!(!a.has_cpu_storage() && !a.has_readback_storage());
    assert_eq!(context.transfers().readback_bytes, 0);
    assert!(
        pixels(&context, oa.image)
            .chunks_exact(4)
            .all(|p| p == [255, 0, 0, 255])
    );
    assert!(
        pixels(&context, ob.image)
            .chunks_exact(4)
            .all(|p| p == [0, 255, 0, 255])
    );
    assert!(
        pixels(&context, resized.image)
            .chunks_exact(4)
            .all(|p| p == [0, 0, 255, 255])
    );
    // Same GPU-produced surface uses the shared live lighting implementation.
    produce(&context, &ca, 0xffa0a0a0, false, 0.);
    let dark = a
        .render(&ca, &uniforms(&ca), &[], &[], &[], &[], None, 5)
        .unwrap();
    assert!(
        pixels(&context, dark.image)
            .chunks_exact(4)
            .all(|p| p == [0, 0, 0, 255])
    );
    let mut lit = uniforms(&ca);
    lit.ambient = [1., 1., 1., 0.];
    let bright = a.render(&ca, &lit, &[], &[], &[], &[], None, 6).unwrap();
    assert!(
        pixels(&context, bright.image)
            .chunks_exact(4)
            .all(|p| p == [160, 160, 160, 255])
    );
    let mut invalid = uniforms(&ca);
    invalid.counts[2] = 1;
    assert!(
        a.render(&ca, &invalid, &[], &[], &[], &[], None, 7)
            .is_err()
    );
    assert!(
        a.render(&ca, &uniforms(&cb), &[], &[], &[], &[], None, 7)
            .is_err()
    );
    assert!(IsometricCanvas::new(&context, 0, 1).is_err());
    assert!(IsometricCanvas::new(&context, u32::MAX, u32::MAX).is_err());
    let other = pollster::block_on(GpuContext::headless()).unwrap();
    let foreign = IsometricCanvas::new(&other, 17, 9).unwrap();
    assert!(
        a.render(&foreign, &uniforms(&foreign), &[], &[], &[], &[], None, 7)
            .is_err()
    );
    let particle_canvas = IsometricCanvas::new(&context, 32, 32).unwrap();
    let mut particle_uniforms = uniforms(&particle_canvas);
    particle_uniforms.projection = [16., 24., 8., 32.];
    particle_uniforms.shadow[2] = 1.;
    let particle = Particle {
        position_radius: [0., 0.8, 0., 0.5],
        color: [0., 1., 0., 1.],
    };
    produce(&context, &particle_canvas, 0xff0000ff, true, 0.);
    let visible = a
        .render(
            &particle_canvas,
            &particle_uniforms,
            &[],
            &[],
            &[],
            &[particle],
            None,
            8,
        )
        .unwrap();
    let visible = pixels(&context, visible.image);
    let center = (16 * 32 + 16) * 4;
    assert!(
        visible[center + 1] > 150,
        "particle must render in front of native canvas geometry"
    );
    produce(&context, &particle_canvas, 0xff0000ff, true, 5.);
    let hidden = a
        .render(
            &particle_canvas,
            &particle_uniforms,
            &[],
            &[],
            &[],
            &[particle],
            None,
            9,
        )
        .unwrap();
    let hidden = pixels(&context, hidden.image);
    assert_eq!(
        &hidden[center..center + 4],
        &[255, 0, 0, 255],
        "native packed surfaces must occlude particles"
    );
    assert!(!a.has_cpu_storage() && !a.has_readback_storage());
    drop((ca, cb, large, particle_canvas, a, b, repeated));
    context.set_working_buffer_cache_limits(0, 0);
    context.set_target_cache_limits(0, 0);
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
    assert_eq!(context.target_pool_usage().live_bytes, 0);
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn sprite_blits_reject_invalid_sources_and_domains_before_modifying_the_canvas() {
    use scenevm::isometric::{GpuSpriteSource, IsometricBlitter, SpriteBlit};
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    assert!(GpuSpriteSource::new(&context, 0, 1, &[], None).is_err());
    assert!(GpuSpriteSource::new(&context, u32::MAX, u32::MAX, &[], None).is_err());
    assert!(GpuSpriteSource::new(&context, 1, 1, &[255; 3], None).is_err());
    assert!(GpuSpriteSource::new(&context, 1, 1, &[255; 4], Some(&[0; 7])).is_err());
    let source = GpuSpriteSource::new(&context, 1, 1, &[255, 0, 0, 255], None).unwrap();
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 12);
    assert_eq!(context.working_buffer_pool_usage().live_allocations, 2);
    let unrelated = context
        .acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(4))
        .unwrap();
    context
        .queue
        .write_buffer(unrelated.buffer(), 0, &[0, 255, 0, 255]);
    context.set_working_buffer_cache_limits(0, 0);
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 16);
    let canvas = IsometricCanvas::new(&context, 8, 8).unwrap();
    let mut blitter = IsometricBlitter::new(context.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(context.clone()).unwrap();
    let placement = |source| SpriteBlit {
        source,
        origin: [0., 0.],
        pixel_scale: 8.,
        base_height: 0.,
        world_scale: 1.,
        opacity: 1.,
        bitmap: true,
        prelit: true,
        ground: true,
        anchor_y: 1.,
        pixels_per_unit: 1.,
    };
    blitter
        .render(&canvas, [0; 4], &[placement(&source)])
        .unwrap();
    for i in 0..4 {
        let mut invalid = placement(&source);
        match i {
            0 => invalid.origin[0] = f32::NAN,
            1 => invalid.pixel_scale = 0.,
            2 => invalid.opacity = 1.1,
            _ => invalid.world_scale = f32::NAN,
        }
        let before = context.transfers();
        assert!(
            blitter
                .render(&canvas, [0, 255, 0, 255], &[placement(&source), invalid])
                .is_err()
        );
        assert_eq!(
            before,
            context.transfers(),
            "failed frames do not upload/clear a partially valid batch"
        );
    }
    let before = context.transfers();
    assert!(
        blitter
            .render_with_contacts(&canvas, [0, 255, 0, 255], &[placement(&source)], &[])
            .is_err()
    );
    assert_eq!(context.transfers(), before);
    let other = pollster::block_on(GpuContext::headless()).unwrap();
    let foreign = GpuSpriteSource::new(&other, 1, 1, &[0, 255, 0, 255], None).unwrap();
    let foreign_canvas = IsometricCanvas::new(&other, 8, 8).unwrap();
    assert!(
        blitter
            .render(&canvas, [0; 4], &[placement(&foreign)])
            .is_err()
    );
    assert!(
        blitter
            .render(&foreign_canvas, [0; 4], &[placement(&source)])
            .is_err()
    );
    let result = renderer
        .render(&canvas, &uniforms(&canvas), &[], &[], &[], &[], None, 1)
        .unwrap();
    assert!(
        pixels(&context, result.image)
            .chunks_exact(4)
            .all(|p| p == [255, 0, 0, 255])
    );
    let instances = context.device.limits().max_uniform_buffer_binding_size
        / u64::from(context.device.limits().min_uniform_buffer_offset_alignment)
        + 1;
    let batch = vec![placement(&source); instances as usize];
    blitter.render(&canvas, [0; 4], &batch).unwrap();
    assert!(
        context.working_buffer_pool_usage().live_bytes
            > u64::from(context.device.limits().max_uniform_buffer_binding_size)
    );
    let allocations = context.working_buffer_pool_usage().allocations;
    blitter.render(&canvas, [0; 4], &batch).unwrap();
    assert_eq!(context.working_buffer_pool_usage().allocations, allocations);
    let large = renderer
        .render(&canvas, &uniforms(&canvas), &[], &[], &[], &[], None, 2)
        .unwrap();
    assert!(
        pixels(&context, large.image)
            .chunks_exact(4)
            .all(|p| p == [255, 0, 0, 255])
    );
    drop(batch);
    drop((source, unrelated));
    drop((canvas, blitter, renderer));
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
}

#[test]
#[ignore = "native GPU; injected mesh shadow/source/array lease ownership"]
fn injected_mesh_shadows_preserve_alpha_and_retire_pooled_storage() {
    use scenevm::isometric::{PointLight, ShadowGeometry};
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let canvas = IsometricCanvas::new(&context, 1, 1).unwrap();
    produce(&context, &canvas, 0xffa0a0a0, false, 0.);
    let mut renderer = IsometricRenderer::new(context.clone()).unwrap();
    let mut u = Uniforms {
        projection: [0.5, 0.5, 1., 1.],
        sun: [0.70710677, 0.70710677, 0., 0.],
        sun_color: [1., 1., 1., 0.],
        counts: [1, 1, 0, 0],
        shadow: [10., 1., 0., 1.],
        ..Default::default()
    };
    let mut mesh = ShadowGeometry {
        revision: 1,
        ..Default::default()
    };
    mesh.triangle([[0.3, 1., -1.], [1.5, 1., -1.], [0.3, 1., 1.]]);
    let sample = |renderer: &mut IsometricRenderer,
                  u: &Uniforms,
                  mesh: &ShadowGeometry,
                  lights: &[PointLight]| {
        let output = renderer
            .render(&canvas, u, lights, &[], &[], &[], Some(mesh), mesh.revision)
            .unwrap();
        pixels(&context, output.image)[0]
    };
    let open = sample(&mut renderer, &u, &mesh, &[]);
    assert!(open > 100);
    mesh.triangle([[1.5, 1., -1.], [1.5, 1., 1.], [0.3, 1., 1.]]);
    mesh.revision += 1;
    let blocked = sample(&mut renderer, &u, &mesh, &[]);
    assert!(blocked < open / 2);
    let allocations = (
        context.target_pool_usage().allocations,
        context.working_buffer_pool_usage().allocations,
    );
    let uploaded = context.transfers().uploaded_bytes;
    assert_eq!(sample(&mut renderer, &u, &mesh, &[]), blocked);
    assert_eq!(
        context.transfers().uploaded_bytes - uploaded,
        std::mem::size_of::<Uniforms>() as u64 + 96 + 256
    );
    assert_eq!(
        (
            context.target_pool_usage().allocations,
            context.working_buffer_pool_usage().allocations
        ),
        allocations
    );
    mesh.alpha = vec![0];
    for vertex in &mut mesh.vertices {
        vertex.uv_alpha = [0.5, 0.5, 0., f32::from_bits(1 | 1 << 16)];
    }
    mesh.revision += 1;
    assert!(sample(&mut renderer, &u, &mesh, &[]) > open / 2);
    mesh.alpha[0] = 1;
    mesh.revision += 1;
    assert!(sample(&mut renderer, &u, &mesh, &[]) < open / 2);
    let light = PointLight {
        position: [2., 2., 0., 0.],
        color: [1., 0., 0., 0.],
        parameters: [20., 10., 0., 0.],
    };
    u.sun_color = [0.; 4];
    u.counts[2] = 1;
    let blocked = sample(&mut renderer, &u, &mesh, &[light]);
    mesh.alpha[0] = 0;
    mesh.revision += 1;
    let local = sample(&mut renderer, &u, &mesh, &[light]);
    assert!(local > 100 && blocked < local / 2);
    context.set_target_cache_limits(0, 0);
    context.set_working_buffer_cache_limits(0, 0);
    assert_eq!(context.target_pool_usage().live_allocations, 3);
    assert_eq!(
        context.target_pool_usage().live_bytes,
        (2048 * 2048 + 6 * 512 * 512) * 4 + 4
    );
    assert_eq!(sample(&mut renderer, &u, &mesh, &[light]), local);
    drop((renderer, canvas));
    assert_eq!(context.target_pool_usage().live_bytes, 0);
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
    assert!(!scenevm::SceneVM::compatibility_gpu_initialized());
}
