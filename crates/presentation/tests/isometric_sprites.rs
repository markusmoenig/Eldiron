#[allow(dead_code)]
mod isometric_support;
// Actual GPU source blits compared byte-for-byte with the retained iso adapter.
use eldiron_presentation::{
    isometric::{IsometricSourceVersion, IsometricSources},
    screen::ContentRevision,
};
use renderkit::{GpuContext, ReadbackTicket, wgpu};
use rusterix::{
    Assets, Item, Map, PixelSource, RenderSettings, Texture, Tile, Value,
    isometric::{
        self, Projection, SceneCache, SpriteLayout, lighting::Lighting, surface::SurfaceFrame,
    },
};
use scenevm::isometric::{IsometricBlitter, IsometricCanvas, IsometricRenderer, SpriteBlit};
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
    data[3] = 255; // visible source texel at the smallest reference scale
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
    let mut producer =
        eldiron_presentation::isometric::IsometricSceneRenderer::new(gpu.clone()).unwrap();
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
    let mut sources = IsometricSources::new(gpu.clone(), 32 * 1024 * 1024, 512);
    let version = IsometricSourceVersion {
        namespace: theframework::prelude::Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: assets.tiles_revision,
    };
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let mut expected_uploads = None;
    for (pass, (size, origin, unit)) in [
        ((84, 60), [40.3, 32.8], 16.),
        ((84, 60), [-3.6, 20.2], 10.),
        ((93, 67), [51.5, 48.5], 21.),
        ((84, 60), [104., -72.], 128.),
        ((84, 60), [104., -72.], 128.),
    ]
    .into_iter()
    .enumerate()
    {
        if pass == 3 {
            map.items[0].set_attribute("size_2d", Value::Float(0.));
        }
        if pass == 4 {
            map.items[0].set_attribute("size_2d", Value::Float(-0.5));
        }
        let projection = Projection {
            origin: Vec2::from(origin),
            unit,
        };
        let plan = cache.prepare(&map, &assets, 0., &avatars);
        assert!(plan.ground.is_empty() && plan.floors.is_empty() && plan.terrain.is_none());
        let before_sources = gpu.transfers().uploaded_bytes;
        let leases = plan
            .upright
            .iter()
            .map(|sprite| sources.prepare(sprite, version).unwrap().unwrap())
            .collect::<Vec<_>>();
        let source_bytes = sources.resident_bytes();
        assert_eq!(*expected_uploads.get_or_insert(source_bytes), source_bytes);
        assert_eq!(sources.builds(), 4);
        if pass > 0 {
            assert_eq!(
                gpu.transfers().uploaded_bytes,
                before_sources,
                "camera changes retain every GPU source"
            );
        }
        let sprites = plan
            .upright
            .iter()
            .zip(&leases)
            .map(|(s, source)| {
                let image = s.image().unwrap();
                let (at, scale) = s.image_placement(&image, projection);
                SpriteBlit {
                    source,
                    origin: at.into_array(),
                    pixel_scale: scale,
                    base_height: s.height,
                    world_scale: s.scale,
                    opacity: s.opacity,
                    bitmap: image.bitmap,
                    prelit: image.prelit,
                    ground: image.ground,
                    anchor_y: image.anchor.y,
                    pixels_per_unit: if pass == 4 && !image.bitmap {
                        0.
                    } else {
                        image.unit
                    },
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
        isometric_support::check_producer(
            &gpu,
            &mut producer,
            size,
            &eldiron_presentation::isometric::IsometricSceneInput {
                map: &map,
                assets: &assets,
                avatars: &avatars,
                seconds: 0.,
                projection,
                lighting: &lighting,
                particles: &[],
                source_version: version,
            },
            reference.pixels(),
            "assembled native producer",
        );
        assert_eq!(
            actual,
            reference.pixels(),
            "GPU source blits differ at camera/scale pass {pass}"
        );
    }
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_contact_owners_preserve_supported_boots_without_protecting_foreground_props() {
    use rusterix::isometric::scene::ContactPolicy;
    use scenevm::isometric::SpriteContacts;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut producer =
        eldiron_presentation::isometric::IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut floor = Tile::from_texture(Texture::new([190, 45, 30, 255].repeat(32 * 16), 32, 16));
    floor.isometric = Some(SpriteLayout {
        bitmap: true,
        prelit: true,
        ground: true,
        anchor: [16, 8],
        pixels_per_unit: 16.,
        directions: 1,
        frames_per_direction: 1,
        ..Default::default()
    });
    let mut actor_pixels = vec![0; 32 * 32 * 4];
    for y in 26..31 {
        for x in 15..18 {
            actor_pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4]
                .copy_from_slice(&[25, 185, 40, 255]);
        }
    }
    let mut actor = Tile::from_texture(Texture::new(actor_pixels, 32, 32));
    actor.isometric = Some(SpriteLayout {
        bitmap: true,
        prelit: true,
        anchor: [16, 28],
        pixels_per_unit: 16.,
        directions: 1,
        frames_per_direction: 1,
        ..Default::default()
    });
    let mut prop = Tile::from_texture(Texture::new([35, 60, 210, 255].repeat(8 * 8), 8, 8));
    prop.isometric = Some(SpriteLayout {
        bitmap: true,
        prelit: true,
        anchor: [4, 4],
        pixels_per_unit: 16.,
        directions: 1,
        frames_per_direction: 1,
        ..Default::default()
    });
    let mut map = Map::default();
    for (id, p) in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
        .into_iter()
        .enumerate()
    {
        map.vertices
            .push(rusterix::Vertex::new(id as u32, p[0], p[1]));
    }
    for i in 0..4 {
        map.create_linedef_manual(i, (i + 1) % 4);
    }
    let id = map.close_polygon_manual().unwrap();
    let sector = map.find_sector_mut(id).unwrap();
    sector
        .properties
        .set("source", Value::Source(PixelSource::TileId(floor.id)));
    sector.properties.set("floor_height", Value::Float(1.));
    sector.properties.set("iso_walkable", Value::Bool(true));
    let mut entity = rusterix::Entity::default();
    entity.position = Vec3::new(0.8, 1., 0.2);
    entity.set_attribute("source", Value::Source(PixelSource::TileId(actor.id)));
    map.entities.push(entity);
    let mut item = Item::default();
    item.position = Vec3::new(0.7, 0.9, 0.25);
    item.set_attribute("source", Value::Source(PixelSource::TileId(prop.id)));
    map.items.push(item);
    let mut assets = Assets::default();
    assets.set_tiles(
        [floor, actor, prop]
            .into_iter()
            .map(|t| (t.id, t))
            .collect(),
    );
    let version = IsometricSourceVersion {
        namespace: theframework::prelude::Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: assets.tiles_revision,
    };
    let mut sources = IsometricSources::new(gpu.clone(), 1024 * 1024, 16);
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let size = (96, 64);
    let projection = Projection {
        origin: Vec2::new(40., 40.),
        unit: 16.,
    };
    let canvas = IsometricCanvas::new(&gpu, size.0, size.1).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let mut scene = scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1);
    let mut settings = RenderSettings::default();
    settings.sun_enabled = false;
    settings.ambient_strength = 0.;
    let lighting = Lighting::new(&settings, std::iter::empty(), 0);
    for pass in 0..3 {
        if pass == 1 {
            map.items[0].position.z = 1.3;
        }
        if pass == 2 {
            map.entities[0].position.x = 0.75;
            map.entities[0].position.z = 0.25;
        }
        let plan = cache.prepare(&map, &assets, 0., &avatars);
        assert_eq!(plan.upright.len(), 3);
        let policy = ContactPolicy::new(&map, &plan.upright);
        let actor_owner = plan
            .upright
            .iter()
            .zip(&policy.owner_by_instance)
            .find(|(s, _)| {
                matches!(
                    s.owner,
                    Some(rusterix::isometric::scene::SpriteOwner::Entity { .. })
                )
            })
            .unwrap()
            .1;
        let item_owner = plan
            .upright
            .iter()
            .zip(&policy.owner_by_instance)
            .find(|(s, _)| {
                matches!(
                    s.owner,
                    Some(rusterix::isometric::scene::SpriteOwner::Item { .. })
                )
            })
            .unwrap()
            .1;
        let mask = policy.protected.iter().find(|m| !m.is_empty()).unwrap();
        assert!(mask[*actor_owner as usize]);
        assert!(!mask[*item_owner as usize]);
        assert!(policy.protected.iter().any(|m| m.iter().any(|p| *p)));
        assert!(
            policy
                .affected_screen_bounds(&plan.upright, projection)
                .is_some()
        );
        let prepared = plan
            .upright
            .iter()
            .map(|s| sources.prepare(s, version).unwrap().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(sources.builds(), 3);
        let sprites = plan
            .upright
            .iter()
            .zip(&prepared)
            .map(|(s, source)| {
                let image = s.image().unwrap();
                let (at, scale) = s.image_placement(&image, projection);
                SpriteBlit {
                    source,
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
        let contacts = policy
            .owner_by_instance
            .iter()
            .zip(&policy.protected)
            .map(|(id, protected)| SpriteContacts {
                owner: *id,
                protected,
            })
            .collect::<Vec<_>>();
        let light = lighting.gpu_parameters(projection, size, &plan.casters, false, 0);
        let before = gpu.transfers().readback_bytes;
        blitter
            .render_with_contacts(&canvas, [17, 34, 29, 255], &sprites, &contacts)
            .unwrap();
        let output = renderer
            .render(
                &canvas,
                &light.uniforms,
                &light.lights,
                &light.casters,
                &light.columns,
                &[],
                None,
                pass,
            )
            .unwrap();
        assert_eq!(gpu.transfers().readback_bytes, before);
        assert!(!renderer.has_cpu_storage() && !renderer.has_readback_storage());
        let protected = capture(&gpu, output.image);
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
        isometric_support::check_producer(
            &gpu,
            &mut producer,
            size,
            &eldiron_presentation::isometric::IsometricSceneInput {
                map: &map,
                assets: &assets,
                avatars: &avatars,
                seconds: 0.,
                projection,
                lighting: &lighting,
                particles: &[],
                source_version: version,
            },
            reference.pixels(),
            "assembled native producer",
        );
        if protected != reference.pixels() {
            let at = protected
                .iter()
                .zip(reference.pixels())
                .position(|(a, b)| a != b)
                .unwrap();
            panic!(
                "contact pass {pass} differs at byte {at}: GPU {}, reference {}",
                protected[at],
                reference.pixels()[at]
            );
        }
        if pass == 0 {
            let mut evidence = png::Encoder::new(
                std::fs::File::create("/private/tmp/eldiron-native-isometric-contacts.png")
                    .unwrap(),
                size.0,
                size.1,
            );
            evidence.set_color(png::ColorType::Rgba);
            evidence.set_depth(png::BitDepth::Eight);
            evidence
                .write_header()
                .unwrap()
                .write_image_data(&protected)
                .unwrap();

            assert!(
                protected.chunks_exact(4).any(|p| p == [25, 185, 40, 255]),
                "boots remain visible"
            );
            blitter
                .render(&canvas, [17, 34, 29, 255], &sprites)
                .unwrap();
            let unprotected = renderer
                .render(
                    &canvas,
                    &light.uniforms,
                    &light.lights,
                    &light.casters,
                    &light.columns,
                    &[],
                    None,
                    9,
                )
                .unwrap();
            let unprotected = capture(&gpu, unprotected.image);
            assert_ne!(
                protected, unprotected,
                "fixture must expose missing support protection"
            );
        }
    }
}
