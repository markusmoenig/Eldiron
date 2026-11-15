use eldiron_presentation::{
    isometric::{GpuMaterialSource, IsometricMaterialSources, IsometricSourceVersion},
    screen::ContentRevision,
};
use renderkit::{GpuContext, ReadbackTicket, wgpu};
use rusterix::{RepeatMode, SampleMode, Texture, Tile, TileUvPages};
use scenevm::isometric::{IsometricCanvas, IsometricRenderer, Uniforms};
use std::sync::Arc;
use theframework_core::Uuid;
use wgpu::util::DeviceExt;

fn sample(
    gpu: &Arc<GpuContext>,
    source: &GpuMaterialSource,
    queries: &[([f32; 2], bool)],
) -> Vec<u8> {
    assert_eq!(source.device_generation(), gpu.generation());
    let mut shader = String::from(
        r#"
@group(0) @binding(0) var<storage,read> material_a:array<vec4<u32>>;
@group(0) @binding(1) var<storage,read> material_b:array<vec4<u32>>;
@group(0) @binding(2) var<storage,read> queries:array<vec4<u32>>;
@group(0) @binding(3) var<storage,read_write> albedo:array<u32>;
@group(0) @binding(4) var<storage,read_write> surfaces:array<vec2<u32>>;
"#,
    );
    shader.push_str(include_str!("../src/isometric/material_sample.wgsl"));
    shader.push_str(
        r#"
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let n=arrayLength(&queries);if id.x>=n{return;}
    let query=queries[id.x];let value=sample_material(0u,bitcast<vec2<f32>>(query.xy),query.z!=0u);
    albedo[id.x]=value.color;albedo[id.x+n]=pack4x8unorm(vec4<f32>(value.rme,1.0));
    surfaces[id.x]=vec2<u32>(0u,1u);surfaces[id.x+n]=vec2<u32>(0u,1u);
}"#,
    );
    let module = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iso-material-sampling-fixture"),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let words = queries
        .iter()
        .map(|(uv, repeat)| [uv[0].to_bits(), uv[1].to_bits(), u32::from(*repeat), 0])
        .collect::<Vec<_>>();
    let query_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("iso-source-query-metadata"),
            contents: bytemuck::cast_slice(&words),
            usage: wgpu::BufferUsages::STORAGE,
        });
    let canvas = IsometricCanvas::new(gpu, queries.len() as u32, 2).unwrap();
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: source.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: source.buffer().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: query_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: canvas.albedo().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: canvas.surfaces().as_entire_binding(),
            },
        ],
    });
    let before = gpu.transfers().readback_bytes;
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut p = encoder.begin_compute_pass(&Default::default());
        p.set_pipeline(&pipeline);
        p.set_bind_group(0, &bind, &[]);
        p.dispatch_workgroups((queries.len() as u32).div_ceil(64), 1, 1);
    }
    gpu.queue.submit(Some(encoder.finish()));
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let n = queries.len() as u32;
    let output = renderer
        .render(
            &canvas,
            &Uniforms {
                counts: [n, 2, 0, 0],
                projection: [0., 0., 4., n as f32],
                ..Default::default()
            },
            &[],
            &[],
            &[],
            &[],
            None,
            1,
        )
        .unwrap();
    assert!(!renderer.has_cpu_storage() && !renderer.has_readback_storage());
    assert_eq!(gpu.transfers().readback_bytes, before);
    let mut capture = ReadbackTicket::capture(gpu, output.image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    capture.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
fn tile() -> Tile {
    let textures = (0..8)
        .map(|frame| {
            let pixels = (0..12)
                .flat_map(|texel| [25 + frame * 25, 40 + texel * 12, 170 - frame * 10, 255])
                .collect();
            let mut texture = Texture::new(pixels, 4, 3);
            texture.set_materials_all(0.15 + frame as f32 * 0.1, 0.2, 1., 0.1);
            texture
        })
        .collect::<Vec<_>>();
    let mut tile = Tile::from_texture(textures[0].clone());
    tile.baked_material_data = textures.iter().map(Texture::material_bytes).collect();
    tile.textures = textures;
    tile.uv_pages = Some(TileUvPages {
        animation_frames: 2,
        repeat_period: [0.; 2],
        origin: [-1, -1],
        size: [2, 2],
        axes: [[0.; 3]; 2],
        units_per_page: 1,
    });
    tile
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn immutable_material_pages_match_cpu_spatial_animation_clamp_and_repeat_sampling() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut tile: Tile = serde_json::from_str(&serde_json::to_string(&tile()).unwrap()).unwrap();
    let mut sources = IsometricMaterialSources::new(gpu.clone(), 1024 * 1024, 8);
    let mut version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: 1,
    };
    let queries = [
        ([-0.85, -0.75], true),
        ([-0.35, 0.6], true),
        ([0.75, -0.65], true),
        ([0.35, 0.85], true),
        ([-7.2, 4.5], true),
        ([4.5, 9.5], true),
        ([0., 0.], false),
        ([1., 1.], false),
        ([-1., 4.], false),
        ([0.49, 0.26], true),
    ];
    for frame in [0, 1, 2, 1] {
        let source = sources.prepare(&tile, frame, version).unwrap();
        let before = gpu.transfers().uploaded_bytes;
        assert!(Arc::ptr_eq(
            &source,
            &sources.prepare(&tile, frame + 2, version).unwrap()
        ));
        assert_eq!(gpu.transfers().uploaded_bytes, before);
        let pixels = sample(&gpu, &source, &queries);
        let n = queries.len();
        for (i, (uv, repeat)) in queries.iter().enumerate() {
            let expected = tile.sample_2d(
                *uv,
                frame,
                SampleMode::Nearest,
                if *repeat {
                    RepeatMode::RepeatXY
                } else {
                    RepeatMode::ClampXY
                },
            );
            assert_eq!(
                &pixels[i * 4..i * 4 + 4],
                &expected,
                "source color query {i}, phase {frame}"
            );
            if *repeat {
                let (_, rme) = tile.sample_surface_2d(*uv, frame);
                for c in 0..3 {
                    assert!(
                        (pixels[(n + i) * 4 + c] as i32 - (rme[c] * 255.).round() as i32).abs()
                            <= 1,
                        "source material query {i}, phase {frame}, channel {c}"
                    );
                }
            }
        }
    }
    assert_eq!(sources.builds(), 2);
    tile.uv_pages.as_mut().unwrap().repeat_period = [2., 3.];
    version.content.0 += 1;
    let source = sources.prepare(&tile, 1, version).unwrap();
    let pixels = sample(&gpu, &source, &queries);
    for (i, (uv, repeat)) in queries.iter().enumerate() {
        assert_eq!(
            &pixels[i * 4..i * 4 + 4],
            &tile.sample_2d(
                *uv,
                1,
                SampleMode::Nearest,
                if *repeat {
                    RepeatMode::RepeatXY
                } else {
                    RepeatMode::ClampXY
                }
            )
        );
    }
    tile.material.preset = "stone".into();
    tile.material.finish = "wet".into();
    version.content.0 += 1;
    let styled = sources.prepare(&tile, 1, version).unwrap();
    let styled_pixels = sample(&gpu, &styled, &queries);
    for (i, (uv, repeat)) in queries.iter().enumerate().filter(|(_, (_, r))| *r) {
        let (_, rme) = tile.sample_surface_2d(*uv, 1);
        for c in 0..3 {
            assert!(
                (styled_pixels[(queries.len() + i) * 4 + c] as i32
                    - (rme[c] * 255.).round() as i32)
                    .abs()
                    <= 1
            );
        }
        let _ = repeat;
    }
    sources.clear();
    assert_eq!(sources.resident_bytes(), 0);
    assert!(sources.is_empty());
    assert_eq!(sample(&gpu, &source, &queries), pixels);
    assert!(source.resident_bytes() > 0); // caller's immutable GPU lease survives eviction
    let unrelated = gpu
        .acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(
            source.resident_bytes(),
        ))
        .unwrap();
    assert_ne!(unrelated.buffer(), source.buffer());
    gpu.queue.write_buffer(
        unrelated.buffer(),
        0,
        &vec![0; source.resident_bytes() as usize],
    );
    gpu.set_working_buffer_cache_limits(0, 0);
    assert_eq!(
        gpu.working_buffer_pool_usage().live_bytes,
        source.resident_bytes() * 2 + styled.resident_bytes()
    );
    assert_eq!(sample(&gpu, &source, &queries), pixels);
    drop((unrelated, styled));
    assert_eq!(
        gpu.working_buffer_pool_usage().live_bytes,
        source.resident_bytes()
    );
    drop(source);
    assert_eq!(gpu.working_buffer_pool_usage().live_bytes, 0);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn material_residency_handles_missing_pages_and_rejects_malformed_assets_before_upload() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut cache = IsometricMaterialSources::new(gpu.clone(), 0, 0);
    let version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: 1,
    };
    let mut sparse = tile();
    sparse.textures.truncate(5);
    let source = cache.prepare(&sparse, 1, version).unwrap();
    assert!(cache.is_empty());
    assert_eq!(cache.resident_bytes(), 0);
    let builds = cache.builds();
    let uploads = gpu.transfers();
    let allocations = gpu.working_buffer_pool_usage().allocations;
    for _ in 0..100 {
        let repeated = cache.prepare(&sparse, 1, version).unwrap();
        assert!(Arc::ptr_eq(&source, &repeated));
    }
    assert_eq!(cache.builds(), builds);
    assert_eq!(gpu.transfers(), uploads);
    assert_eq!(gpu.working_buffer_pool_usage().allocations, allocations);
    let queries = [
        ([-0.5, -0.5], true),
        ([0.5, -0.5], true),
        ([-0.5, 0.5], true),
        ([0.5, 0.5], true),
    ];
    let output = sample(&gpu, &source, &queries);
    for (i, (uv, _)) in queries.iter().enumerate() {
        assert_eq!(
            &output[i * 4..i * 4 + 4],
            &sparse.sample_surface_2d(*uv, 1).0
        );
    }
    let mut empty = Tile::default();
    empty.textures.clear();
    let blank = cache.prepare(&empty, 0, version).unwrap();
    let output = sample(&gpu, &blank, &queries);
    assert!(output[..queries.len() * 4].iter().all(|v| *v == 0));
    let mut invalid = tile();
    invalid.uv_pages.as_mut().unwrap().size = [u32::MAX, u32::MAX];
    let before = gpu.transfers();
    assert!(cache.prepare(&invalid, 0, version).is_err());
    assert_eq!(gpu.transfers(), before);
    let mut malformed = tile();
    malformed.textures[0].data.truncate(3);
    assert!(cache.prepare(&malformed, 0, version).is_err());
    assert_eq!(gpu.transfers(), before);
    malformed.textures[0].width = usize::MAX;
    malformed.textures[0].height = 2;
    assert!(cache.prepare(&malformed, 0, version).is_err());
    assert_eq!(gpu.transfers(), before);
    assert!(cache.is_empty());
    assert_eq!(cache.resident_bytes(), 0);
    // Invalid repeat-period metadata retains the reference spatial-grid fallback.
    let mut fallback = tile();
    fallback.uv_pages.as_mut().unwrap().repeat_period = [f32::NAN, 2.];
    fallback.uv_pages.as_mut().unwrap().units_per_page = 0;
    let asset = cache.prepare(&fallback, 0, version).unwrap();
    let output = sample(&gpu, &asset, &queries);
    for (i, (uv, _)) in queries.iter().enumerate() {
        assert_eq!(
            &output[i * 4..i * 4 + 4],
            &fallback.sample_surface_2d(*uv, 0).0
        );
    }
    let retired = Arc::downgrade(&source);
    drop(source);
    assert!(retired.upgrade().is_none(), "publication lookup must not own the source");
    let builds = cache.builds();
    let _rebuilt = cache.prepare(&sparse, 1, version).unwrap();
    assert_eq!(cache.builds(), builds + 1);
}
