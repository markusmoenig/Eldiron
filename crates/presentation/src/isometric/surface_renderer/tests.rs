use super::*;
#[test]
#[ignore = "requires native GPU; deterministic individual/batched shader comparison"]
fn batched_surface_parameters_match_individual_gpu_draws_at_fractional_large_world_coordinates() {
    use crate::isometric::{IsometricMaterialSources, IsometricSourceVersion};
    use crate::screen::ContentRevision;
    use rusterix::{RenderSettings, Texture, Tile, isometric::lighting::Lighting};
    use scenevm::isometric::{IsometricBlitter, IsometricRenderer};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut materials = IsometricMaterialSources::new(gpu.clone(), 1024 * 1024, 8);
    let texture = Texture::new(
        (0..64 * 64)
            .flat_map(|i| {
                [
                    (i * 37 % 256) as u8,
                    (i * 71 % 256) as u8,
                    (i * 19 % 256) as u8,
                    if i % 3 == 0 { 110 } else { 255 },
                ]
            })
            .collect(),
        64,
        64,
    );
    let tile = Tile::from_texture(texture);
    let source = materials
        .prepare(
            &tile,
            0,
            IsometricSourceVersion {
                namespace: theframework_core::Uuid::new_v4(),
                content: ContentRevision(1),
                tiles: 1,
            },
        )
        .unwrap();
    let mut other_tile = tile.clone();
    other_tile.id = theframework_core::Uuid::new_v4();
    for pixel in other_tile.textures[0].data.chunks_exact_mut(4) {
        for channel in &mut pixel[..3] {
            *channel = 255 - *channel;
        }
    }
    let secondary = materials
        .prepare(
            &other_tile,
            0,
            IsometricSourceVersion {
                namespace: theframework_core::Uuid::new_v4(),
                content: ContentRevision(1),
                tiles: 1,
            },
        )
        .unwrap();
    let mut vertices = Vec::new();
    for y in 0..12 {
        for x in 0..12 {
            let point = |x: i32, y: i32| {
                [
                    137.125 + x as f32 * 0.75,
                    ((x * 13 + y * 7) as f32).sin() * 0.13,
                    -89.375 + y as f32 * 0.75,
                ]
            };
            let [a, b, c, d] = [
                point(x, y),
                point(x + 1, y),
                point(x + 1, y + 1),
                point(x, y + 1),
            ];
            vertices.extend([[a, b, c], [a, c, d]]);
        }
    }
    for kind in [SurfaceKind::Terrain, SurfaceKind::Floor] {
        for repeat in [true, false] {
            let mut draws = vertices
                .iter()
                .map(|v| SurfaceTriangle {
                    vertices: if kind == SurfaceKind::Floor {
                        v.map(|p| [p[0], 0.37, p[2]])
                    } else {
                        *v
                    },
                    source: &source,
                    secondary: Some((&secondary, [0.05, 0.95, 0.25, 0.8])),
                    paint: None,
                    kind,
                    repeat,
                    uv_bounds: [136., -90., 148., -78.],
                    clip_cell: None,
                    contacts: SpriteContacts::default(),
                })
                .collect::<Vec<_>>();
            // A leading invisible record must not shift any compact GPU record
            // lookup. Keep it valid so the individual reference validates it too.
            draws.insert(
                0,
                SurfaceTriangle {
                    vertices: [
                        [10000., 0., 10000.],
                        [10001., 0., 10000.],
                        [10001., 0., 10001.],
                    ],
                    source: &secondary,
                    secondary: None,
                    paint: None,
                    kind,
                    repeat,
                    uv_bounds: [9999., 9999., 10002., 10002.],
                    clip_cell: None,
                    contacts: SpriteContacts::default(),
                },
            );
            let mut raster = IsometricSurfaceRenderer::new(gpu.clone()).unwrap();
            let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
            let mut output = IsometricRenderer::new(gpu.clone()).unwrap();
            let mut settings = RenderSettings::default();
            settings.sun_enabled = false;
            settings.ambient_strength = 1.;
            settings.ambient_color = [1.; 3];
            let lighting = Lighting::new(&settings, std::iter::empty(), 0);
            for unit in [3.75, 12.5] {
                let projection = Projection {
                    origin: Vec2::new(
                        80. - (137.125 - 89.375 + 9.) * unit,
                        75. - (-89.375 - 137.125) * unit * 0.5,
                    ),
                    unit,
                };
                let light = lighting.gpu_parameters(projection, (160, 120), &[], false, 0);
                let mut images = Vec::new();
                let mut uploads = Vec::new();
                for (batched, without_offscreen) in [(false, false), (true, false), (true, true)] {
                    let canvas = IsometricCanvas::new(&gpu, 160, 120).unwrap();
                    blitter.render(&canvas, [17, 34, 29, 255], &[]).unwrap();
                    let before = gpu.transfers().uploaded_bytes;
                    raster
                        .append_impl(
                            &canvas,
                            projection,
                            &draws[usize::from(without_offscreen)..],
                            batched,
                        )
                        .unwrap();
                    uploads.push(gpu.transfers().uploaded_bytes - before);
                    let image = output
                        .render(
                            &canvas,
                            &light.uniforms,
                            &light.lights,
                            &light.casters,
                            &light.columns,
                            &[],
                            None,
                            1,
                        )
                        .unwrap()
                        .image;
                    let mut capture = renderkit::ReadbackTicket::capture(&gpu, image).unwrap();
                    gpu.device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: Some(std::time::Duration::from_secs(30)),
                        })
                        .unwrap();
                    images.push(capture.try_finish(&gpu).unwrap().unwrap().pixels().to_vec());
                }
                let changed = images[0]
                    .iter()
                    .zip(&images[1])
                    .filter(|(a, b)| a != b)
                    .count();
                assert!(
                    images[0].chunks_exact(4).any(|p| p != &images[0][..4]),
                    "fixture must render visible terrain"
                );
                let max = images[0]
                    .iter()
                    .zip(&images[1])
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap();
                assert_eq!(
                    changed, 0,
                    "kind={kind:?} repeat={repeat} unit={unit} changed_channels={changed} max_delta={max}"
                );
                assert!(
                    images[1] == images[2],
                    "removing an offscreen input must preserve all RGBA"
                );
                assert_eq!(
                    uploads[1], uploads[2],
                    "offscreen records must consume no GPU upload bytes"
                );
            }
        }
    }
}
