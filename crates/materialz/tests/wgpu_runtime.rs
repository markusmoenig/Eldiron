#![cfg(feature = "wgpu")]
use denrim_materials::{cpu::Evaluator, gpu::*, integration::*, ir::Channel};
use std::{sync::mpsc, time::Duration};
const SOURCE: &str = "project 1\nmaterial \"m\" called \"PBR\"\n  mode pbr\n  tiling raw\n  default opacity to 0.25\n  default metallic to 0.7\n  default emission to 0.1 0.2 0.3\n  default emission strength to 3\n  row \"color\"\n    use \"Color\" id \"pigment\"\n      set \"color\" to 0.2 0.3 0.4 1\n  row \"noise\"\n    use \"Perlin Noise\" id \"noise\"\n  output base color from row \"color\" xyz\n  output roughness from row \"noise\" x\n";
fn poll(device: &wgpu::Device) {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
}
async fn setup() -> (wgpu::Device, wgpu::Queue, MaterialZ) {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&Default::default())
        .await
        .expect("GPU adapter required");
    let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
    let runtime = MaterialZ::new(&device, &queue).await.unwrap();
    (device, queue, runtime)
}
fn capture(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    surface: &Surface,
) -> Vec<(wgpu::Buffer, u32)> {
    capture_maps(device, encoder, &surface.maps, surface.resolution)
}
fn capture_maps(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    maps: &[SurfaceMap],
    resolution: [u32; 2],
) -> Vec<(wgpu::Buffer, u32)> {
    maps.iter()
        .map(|map| {
            let pixel = if map.layout.format == wgpu::TextureFormat::Rgba16Float {
                8
            } else {
                4
            };
            let stride = (resolution[0] * pixel).div_ceil(256) * 256;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("test readback"),
                size: u64::from(stride) * u64::from(resolution[1]),
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
                        rows_per_image: Some(resolution[1]),
                    },
                },
                map.texture.size(),
            );
            (buffer, stride)
        })
        .collect()
}
fn collect(
    device: &wgpu::Device,
    buffers: Vec<(wgpu::Buffer, u32)>,
    size: [u32; 2],
    ticket: RenderTicket,
) -> Vec<Vec<[f32; 4]>> {
    let (tx, rx) = mpsc::channel();
    ticket.map_diagnostics(move |r| tx.send(r).unwrap());
    let mut callbacks = vec![];
    for (b, _) in &buffers {
        let (tx, rx) = mpsc::channel();
        b.map_async(wgpu::MapMode::Read, .., move |r| tx.send(r).unwrap());
        callbacks.push(rx);
    }
    poll(device);
    rx.recv().unwrap().unwrap();
    for rx in callbacks {
        rx.recv().unwrap().unwrap();
    }
    buffers
        .into_iter()
        .enumerate()
        .map(|(index, (buffer, stride))| {
            let data = buffer.get_mapped_range(..);
            let mut pixels = vec![];
            let rgb = matches!(index, 0 | 4 | 6);
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let at = (y * stride + x * if rgb { 8 } else { 4 }) as usize;
                    let value = if rgb {
                        std::array::from_fn(|i| {
                            half::f16::from_bits(u16::from_le_bytes(
                                data[at + i * 2..at + i * 2 + 2].try_into().unwrap(),
                            ))
                            .to_f32()
                        })
                    } else {
                        [
                            f32::from_le_bytes(data[at..at + 4].try_into().unwrap()),
                            0.,
                            0.,
                            1.,
                        ]
                    };
                    pixels.push(value);
                }
            }
            drop(data);
            buffer.unmap();
            pixels
        })
        .collect()
}
#[test]
#[ignore = "requires a WGPU device"]
fn authoring_height_and_normal_maps_render_without_changing_the_runtime_profile() {
    pollster::block_on(async {
        let (device, queue, runtime) = setup().await;
        let source = format!("{SOURCE}  default height to 0.65\n  default normal to 0.25 0.75 1\n");
        let asset = EditableMaterialAsset::parse("asset", &source, "m").unwrap();
        let material = runtime
            .compile_preview(&asset, InstanceRecord::new("preview"))
            .await
            .unwrap();
        assert!(material.diagnostics().is_empty());
        let request = RenderRequest {
            time: 0.,
            resolution: [8, 8],
            domain: SurfaceDomain::rect([0., 0.], [1., 1.]),
        };
        let surface = runtime.create_preview_surface(&request).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        let ticket = runtime
            .encode_preview(&mut encoder, &material, &request, &surface)
            .unwrap();
        let buffers = capture_maps(&device, &mut encoder, &surface.maps, surface.resolution);
        queue.submit([encoder.finish()]);
        let maps = collect(&device, buffers, request.resolution, ticket);
        assert_eq!(maps.len(), 7);
        assert!(maps[5].iter().all(|pixel| (pixel[0] - 0.65).abs() < 0.0001));
        for pixel in &maps[6] {
            for i in 0..3 {
                assert!((pixel[i] - [0.25, 0.75, 1.][i]).abs() < 0.001);
            }
        }
        let baked = render(&runtime, &device, &queue, &material, request);
        assert_eq!(baked.len(), 5);
        let minimal = runtime
            .compile(&asset, InstanceRecord::new("runtime"))
            .await
            .unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        assert!(
            runtime
                .encode_preview(&mut encoder, &minimal, &request, &surface)
                .err()
                .unwrap()
                .message
                .contains("compile_preview")
        );
    });
}
fn render(
    runtime: &MaterialZ,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    material: &MaterialInstance,
    request: RenderRequest,
) -> Vec<Vec<[f32; 4]>> {
    let surface = runtime.create_surface(&request).unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    let ticket = runtime
        .encode(&mut encoder, material, &request, &surface)
        .unwrap();
    let buffers = capture(device, &mut encoder, &surface);
    queue.submit([encoder.finish()]);
    collect(device, buffers, request.resolution, ticket)
}
#[test]
#[ignore = "requires a WGPU device"]
fn pbr_maps_regions_live_updates_and_queued_snapshots() {
    pollster::block_on(async {
        let (device, queue, runtime) = setup().await;
        let asset = EditableMaterialAsset::parse("asset", SOURCE, "m").unwrap();
        let record = InstanceRecord::new("placement");
        let mut material = runtime.compile(&asset, record.clone()).await.unwrap();
        let generation = material.pipeline_generation();
        let request = RenderRequest {
            time: 0.,
            resolution: [32, 16],
            domain: SurfaceDomain::rect([0., 0.], [1., 1.]),
        };
        let first = render(&runtime, &device, &queue, &material, request);
        for pixel in &first[0] {
            for i in 0..3 {
                assert!((pixel[i] - [0.2, 0.3, 0.4][i]).abs() < 0.001);
            }
            assert_eq!(pixel[3], 1.);
        }
        assert!(first[1].iter().all(|p| (p[0] - 0.25).abs() < 1e-6));
        assert!(first[3].iter().all(|p| (p[0] - 0.7).abs() < 1e-6));
        for pixel in &first[4] {
            for i in 0..3 {
                assert!((pixel[i] - [0.3, 0.6, 0.9][i]).abs() < 0.001);
            }
        }
        let p = asset.program(&record).unwrap();
        let cpu = Evaluator::new(&p);
        for y in 0..16 {
            for x in 0..32 {
                let expected = cpu
                    .sample(
                        "m",
                        Channel::Roughness,
                        [(x as f32 + 0.5) / 32., (y as f32 + 0.5) / 16.],
                    )
                    .unwrap()[0];
                assert!((first[2][y * 32 + x][0] - expected.clamp(0., 1.)).abs() < 0.003);
            }
        }
        let wide = render(
            &runtime,
            &device,
            &queue,
            &material,
            RenderRequest {
                time: 0.,
                resolution: [64, 16],
                domain: SurfaceDomain::rect([0., 0.], [2., 1.]),
            },
        );
        let right = render(
            &runtime,
            &device,
            &queue,
            &material,
            RenderRequest {
                time: 0.,
                domain: SurfaceDomain::rect([1., 0.], [1., 1.]),
                ..request
            },
        );
        for y in 0..16 {
            for x in 0..64 {
                let small = if x < 32 { &first } else { &right };
                assert!((wide[2][y * 64 + x][0] - small[2][y * 32 + x % 32][0]).abs() < 1e-6);
            }
        }
        let before = runtime.create_surface(&request).unwrap();
        let after = runtime.create_surface(&request).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        let a = runtime
            .encode(&mut encoder, &material, &request, &before)
            .unwrap();
        material
            .set_parameter(
                ParameterKey::new("asset", "pigment", "color"),
                vec![0.8, 0.1, 0.2, 1.],
            )
            .unwrap();
        assert_eq!(generation, material.pipeline_generation());
        let b = runtime
            .encode(&mut encoder, &material, &request, &after)
            .unwrap();
        let a_buffers = capture(&device, &mut encoder, &before);
        let b_buffers = capture(&device, &mut encoder, &after);
        queue.submit([encoder.finish()]);
        let a = collect(&device, a_buffers, request.resolution, a);
        let b = collect(&device, b_buffers, request.resolution, b);
        assert!((a[0][0][0] - 0.2).abs() < 0.001);
        assert!((b[0][0][0] - 0.8).abs() < 0.001);
        let snapshot = material.snapshot();
        let error = material
            .set_parameter(
                ParameterKey::new("asset", "pigment", "color"),
                vec![f32::NAN],
            )
            .unwrap_err();
        assert_eq!(error.location.tile.as_deref(), Some("pigment"));
        assert_eq!(error.location.row.as_deref(), Some("color"));
        assert_eq!(material.snapshot(), snapshot);
        let mut edited = asset.clone();
        edited.root.project.materials[0]
            .defaults
            .iter_mut()
            .find(|(name, _)| name == "opacity")
            .unwrap()
            .1 = vec![0.];
        material.update_asset(&edited).unwrap();
        assert_eq!(material.pipeline_generation(), generation);
        let transparent = render(&runtime, &device, &queue, &material, request);
        assert!(transparent[1].iter().all(|p| p[0] == 0.));
        // A structural source edit must not replace the last compatible live state.
        edited.root.project.materials[0].rows[0].tiles[0].code = Some(format!(
            "{}\nimage = vec4(0.1)\n",
            denrim_materials::tilelang::source(denrim_materials::metadata::Kernel::Color)
        ));
        assert!(material.update_asset(&edited).is_err());
        assert_eq!(material.pipeline_generation(), generation);
    });
}
#[test]
#[ignore = "requires a WGPU device"]
fn cached_filters_render_with_halos_and_mips() {
    pollster::block_on(async {
        let (device, queue, runtime) = setup().await;
        let source = include_str!("../examples/filtered-basketweave.material");
        let asset =
            EditableMaterialAsset::parse("basketweave", source, "filtered-basketweave").unwrap();
        let material = runtime
            .compile(&asset, InstanceRecord::new("one"))
            .await
            .unwrap();
        let data = render(
            &runtime,
            &device,
            &queue,
            &material,
            RenderRequest {
                time: 0.,
                resolution: [128, 128],
                domain: SurfaceDomain::rect([0., 0.], [1., 1.]),
            },
        );
        assert!(data[0].iter().all(|p| p.iter().all(|v| v.is_finite())));
        let min = data[0].iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let max = data[0]
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.02, "cached output range {min}..{max}");
        let program = asset.program(&InstanceRecord::new("reference")).unwrap();
        let cpu = Evaluator::new(&program);
        for (x, y) in [(13, 17), (57, 69), (91, 35), (117, 111)] {
            let expected = cpu
                .sample_with_footprint(
                    "filtered-basketweave",
                    Channel::BaseColor,
                    [(x as f32 + 0.5) / 128., (y as f32 + 0.5) / 128.],
                    1. / 128.,
                )
                .unwrap();
            for c in 0..3 {
                assert!(
                    (data[0][y * 128 + x][c] - expected[c]).abs() < 0.035,
                    "cached GPU vs CPU at {x},{y}: {:?} vs {expected:?}",
                    data[0][y * 128 + x]
                );
            }
        }
    });
}
