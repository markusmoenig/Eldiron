//! Actual GPU source blits compared byte-for-byte with the retained iso adapter.
use renderkit::{GpuContext, ReadbackTicket, wgpu};
use rusterix::{
    Assets, Item, Map, PixelSource, RenderSettings, Texture, Tile, Value,
    isometric::{
        self, Projection, SceneCache, SpriteLayout, lighting::Lighting, surface::SurfaceFrame,
    },
};
use scenevm::isometric::{
    GpuSpriteSource, IsometricBlitter, IsometricCanvas, IsometricRenderer, SpriteBlit,
};
use std::{collections::HashMap, sync::Arc};
use theframework::prelude::{TheDim, TheRGBABuffer, Vec2, Vec3};
fn capture(context: &GpuContext, image: Arc<renderkit::GpuImage>) -> Vec<u8> {
    let mut c = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    c.try_finish(context).unwrap().unwrap().pixels().to_vec()
}
fn tile(bitmap: bool, prelit: bool, ground: bool) -> Tile {
    let mut data = vec![0; 7 * 9 * 4];
    let mut surface = SurfaceFrame::new(7 * 9);
    for y in 0..9 {
        for x in 0..7 {
            let i = y * 7 + x;
            data[i * 4..i * 4 + 4].copy_from_slice(&[
                40 + x as u8 * 25,
                190 - y as u8 * 12,
                75,
                [0, 80, 180, 255][(x + y) % 4],
            ]);
            surface.set(
                i,
                if x < 3 {
                    Vec3::unit_y()
                } else {
                    Vec3::unit_x()
                },
                y as f32 * 0.06,
                0.8,
                0.2,
                0.1,
            );
        }
    }
    let mut tile = Tile::from_texture(Texture::new(data, 7, 9));
    tile.isometric = Some(SpriteLayout {
        bitmap,
        prelit,
        ground,
        anchor: [3, 8],
        pixels_per_unit: 8.,
        directions: 1,
        frames_per_direction: 1,
        surfaces: vec![surface],
        ..Default::default()
    });
    tile
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_sprite_sources_match_reference_alpha_placement_scaling_and_live_geometry() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut map = Map::default();
    let tiles = [
        tile(false, false, false),
        tile(true, true, false),
        tile(true, false, true),
        tile(true, false, false),
    ];
    for (i, tile) in tiles.iter().enumerate() {
        let mut item = Item::default();
        item.id = i as u32;
        item.position = Vec3::new(i as f32 * 0.7 - 1., i as f32 * 0.2, 0.4);
        item.set_attribute("source", Value::Source(PixelSource::TileId(tile.id)));
        item.set_attribute("size_2d", Value::Float(0.7 + i as f32 * 0.3));
        map.items.push(item);
    }
    let mut assets = Assets::default();
    assets.set_tiles(tiles.into_iter().map(|t| (t.id, t)).collect());
    let mut scene = scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1);
    let mut settings = RenderSettings::default();
    settings.sun_enabled = false;
    settings.ambient_color = [1.; 3];
    settings.ambient_strength = 0.17;
    let light = scenevm::Light::new_pointlight(Vec3::new(1., 2., 0.5))
        .with_color(Vec3::new(1., 0.1, 0.3))
        .with_intensity(1.8)
        .with_end_distance(7.);
    let lighting = Lighting::new(&settings, std::iter::once(&light), 0);
    let avatars = HashMap::new();
    let mut cache = SceneCache::default();
    let mut sources = HashMap::new();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let mut expected_uploads = None;
    for (pass, (size, origin, unit)) in [
        ((84, 60), [40.3, 32.8], 16.),
        ((84, 60), [-3.6, 20.2], 10.),
        ((93, 67), [51.5, 48.5], 21.),
    ]
    .into_iter()
    .enumerate()
    {
        let projection = Projection {
            origin: Vec2::from(origin),
            unit,
        };
        let plan = cache.prepare(&map, &assets, 0., &avatars);
        assert!(plan.ground.is_empty() && plan.floors.is_empty() && plan.terrain.is_none());
        for sprite in &plan.upright {
            let image = sprite.image().unwrap();
            let id = sprite.tile.unwrap().id;
            sources.entry(id).or_insert_with(|| {
                GpuSpriteSource::new(
                    &gpu,
                    image.width as u32,
                    image.height as u32,
                    image.data,
                    image.surface.map(|s| s.data.as_slice()),
                )
                .unwrap()
            });
        }
        let source_bytes = sources.values().map(|s| s.resident_bytes()).sum::<u64>();
        assert_eq!(*expected_uploads.get_or_insert(source_bytes), source_bytes);
        let sprites = plan
            .upright
            .iter()
            .map(|s| {
                let image = s.image().unwrap();
                let (at, scale) = s.image_placement(&image, projection);
                SpriteBlit {
                    source: &sources[&s.tile.unwrap().id],
                    origin: at.into_array(),
                    pixel_scale: scale,
                    base_height: s.height,
                    world_scale: s.scale,
                    opacity: s.opacity,
                    bitmap: image.bitmap,
                    prelit: image.prelit,
                    ground: image.ground,
                    anchor_y: image.anchor.y,
                    pixels_per_unit: image.unit,
                }
            })
            .collect::<Vec<_>>();
        let canvas = IsometricCanvas::new(&gpu, size.0, size.1).unwrap();
        let before = gpu.transfers();
        blitter
            .render(&canvas, [17, 34, 29, 255], &sprites)
            .unwrap();
        let lighting_data = lighting.gpu_parameters(projection, size, &plan.casters, false, 0);
        let output = renderer
            .render(
                &canvas,
                &lighting_data.uniforms,
                &lighting_data.lights,
                &lighting_data.casters,
                &lighting_data.columns,
                &[],
                None,
                pass as u64,
            )
            .unwrap();
        assert_eq!(gpu.transfers().capture_requests, before.capture_requests);
        assert!(!renderer.has_cpu_storage() && !renderer.has_readback_storage());
        assert!(
            gpu.transfers().uploaded_bytes - before.uploaded_bytes < (size.0 * size.1) as u64,
            "no viewport upload"
        );
        let actual = capture(&gpu, output.image);
        // Explicit reference-only path: CPU source composition plus the SAME GPU lighting.
        let mut reference = TheRGBABuffer::new(TheDim::sized(size.0 as i32, size.1 as i32));
        isometric::draw_scene_gpu_cached(
            &map,
            &assets,
            &mut reference,
            projection,
            0.,
            None,
            &avatars,
            None,
            &lighting,
            &mut scene,
            &[],
        )
        .unwrap();
        assert_eq!(
            actual,
            reference.pixels(),
            "GPU source blits differ at camera/scale pass {pass}"
        );
    }
}
