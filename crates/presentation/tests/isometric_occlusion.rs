use eldiron_presentation::{
    isometric::{
        IsometricMaterialSources, IsometricSourceVersion, IsometricSources,
        IsometricSurfaceRenderer, SurfaceKind,
    },
    screen::ContentRevision,
};
use renderkit::GpuContext;
use rusterix::{
    Assets, Map, PixelSource, RenderSettings, Texture, Tile, Value, Vertex,
    isometric::{
        self, Projection, SceneCache, SpriteLayout, lighting::Lighting, materials::SurfaceMaterials,
    },
};
use scenevm::{
    isometric::{
        IsometricBlitter, IsometricCanvas, IsometricRenderer, SpriteBlit, SpriteContacts,
        SpriteEnvironment, SpriteTerrainClip,
    },
    prelude::{Vec2, Vec3},
};
use std::collections::HashMap;
use theframework::prelude::{TheDim, TheRGBABuffer, Uuid};
mod isometric_support;
use isometric_support::*;
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_sprite_terrain_clipping_matches_reference_and_keeps_supporting_feet_visible() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let ground = material([60, 90, 35], false);
    let mut pixels = vec![0; 32 * 32 * 4];
    for y in 12..31 {
        for x in 13..19 {
            pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[25, 185, 40, 255]);
        }
    }
    let mut actor = Tile::from_texture(Texture::new(pixels, 32, 32));
    actor.isometric = Some(SpriteLayout {
        bitmap: true,
        prelit: true,
        anchor: [16, 28],
        pixels_per_unit: 16.,
        directions: 1,
        frames_per_direction: 1,
        ..Default::default()
    });
    let mut map = Map::default();
    map.vertices = vec![Vertex::new(0, -2., -2.), Vertex::new(1, 2., 2.)];
    let mut peak = Vertex::new(2, 0.5, 0.5);
    peak.z = 0.9;
    peak.properties.set("terrain_control", Value::Bool(true));
    peak.properties.set("smoothness", Value::Float(0.5));
    map.vertices.push(peak);
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set("terrain_subdivisions", Value::Int(2));
    map.properties.set("terrain_edge_falloff", Value::Float(0.));
    map.properties.set(
        "default_terrain_tile",
        Value::Source(PixelSource::TileId(ground.id)),
    );
    let mut entity = rusterix::Entity::default();
    entity.set_attribute("source", Value::Source(PixelSource::TileId(actor.id)));
    map.entities.push(entity);
    let mut assets = Assets::default();
    assets.set_tiles([ground, actor].into_iter().map(|t| (t.id, t)).collect());
    let version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: assets.tiles_revision,
    };
    let mut materials = IsometricMaterialSources::new(gpu.clone(), 1024 * 1024, 16);
    let mut sprites = IsometricSources::new(gpu.clone(), 1024 * 1024, 16);
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut raster = IsometricSurfaceRenderer::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let mut scene = scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1);
    let size = (128, 96);
    let canvas = IsometricCanvas::new(&gpu, size.0, size.1).unwrap();
    let projection = Projection {
        origin: Vec2::new(64.3, 60.7),
        unit: 22.,
    };
    let mut settings = RenderSettings::default();
    settings.sun_enabled = false;
    settings.ambient_color = [1.; 3];
    settings.ambient_strength = 0.7;
    settings.raster_shadow_enabled = false;
    let lighting = Lighting::new(&settings, std::iter::empty(), 0);
    let mut observed_occlusion = false;
    for (pass, point) in [[0.5, -0.5], [1.2, 0.], [2.5, -1.5]]
        .into_iter()
        .enumerate()
    {
        let terrain = cache.prepare(&map, &assets, 0., &avatars).terrain.unwrap();
        let pos = Vec2::from(point);
        let height = terrain.height_at(pos).unwrap_or(0.);
        map.entities[0].position = Vec3::new(pos.x, height, pos.y);
        let plan = cache.prepare(&map, &assets, 0., &avatars);
        assert_eq!(plan.upright.len(), 1);
        let policy = SurfaceMaterials::for_terrain(&map, &assets);
        let mut prepared = Vec::new();
        terrain.triangles(terrain.bounds, |t| {
            let center = (t[0] + t[1] + t[2]) / 3.;
            let selected = policy
                .cell((center.x.floor() as i32, center.z.floor() as i32))
                .unwrap();
            let source = materials
                .prepare(selected.primary, plan.material_frame, version)
                .unwrap();
            prepared.push(Prepared {
                vertices: t.map(|p| p.into_array()),
                source,
                secondary: None,
                cell: None,
            });
        });
        let draws = triangles(&prepared, SurfaceKind::Terrain, true, [0.; 4]);
        let actor = &plan.upright[0];
        let image = actor.image().unwrap();
        let source = sprites.prepare(actor, version).unwrap().unwrap();
        let (at, scale) = actor.image_placement(&image, projection);
        let blit = SpriteBlit {
            source: &source,
            origin: at.into_array(),
            pixel_scale: scale,
            base_height: actor.height,
            world_scale: actor.scale,
            opacity: actor.opacity,
            bitmap: image.bitmap,
            prelit: image.prelit,
            ground: image.ground,
            anchor_y: image.anchor.y,
            pixels_per_unit: image.unit,
        };
        let contact = actor.terrain_contact(projection).unwrap();
        let environment = SpriteEnvironment {
            contacts: SpriteContacts {
                owner: 1,
                protected: &[],
            },
            terrain: Some(SpriteTerrainClip {
                foot_y: contact.foot_y,
                foot_height: contact.foot_height,
                foot_depth: contact.foot_depth,
                unit: contact.unit,
            }),
            flat_surface: false,
        };
        let geometry = cache.shadow_geometry(&map, &assets, &plan);
        let light = lighting.gpu_parameters(projection, size, &plan.casters, geometry.is_some(), 0);
        let before = gpu.transfers().readback_bytes;
        blitter.render(&canvas, [17, 34, 29, 0], &[]).unwrap();
        raster.append(&canvas, projection, &draws).unwrap();
        blitter
            .append_with_environment(&canvas, &[blit], &[environment])
            .unwrap();
        let output = renderer
            .render(
                &canvas,
                &light.uniforms,
                &light.lights,
                &light.casters,
                &light.columns,
                &[],
                geometry.as_deref(),
                pass as u64,
            )
            .unwrap();
        assert_eq!(gpu.transfers().readback_bytes, before);
        let actual = capture(&gpu, output.image);
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
        mismatch(
            &actual,
            reference.pixels(),
            &format!("terrain clip pose {pass}"),
        );
        // Deliberately disable the clip to prove the fixture exercises actual hiding.
        blitter.render(&canvas, [17, 34, 29, 0], &[]).unwrap();
        raster.append(&canvas, projection, &draws).unwrap();
        blitter.append(&canvas, &[blit]).unwrap();
        let output = renderer
            .render(
                &canvas,
                &light.uniforms,
                &light.lights,
                &light.casters,
                &light.columns,
                &[],
                geometry.as_deref(),
                9,
            )
            .unwrap();
        let plain = capture(&gpu, output.image);
        let count = |pixels: &[u8]| {
            pixels
                .chunks_exact(4)
                .filter(|p| *p == [25, 185, 40, 255])
                .count()
        };
        assert!(count(&actual) <= count(&plain));
        observed_occlusion |= count(&actual) < count(&plain);
        if pass == 0 {
            assert!(
                count(&actual) > 0,
                "supporting terrain must not erase the whole actor"
            );
            save_png(
                "/private/tmp/eldiron-native-isometric-occlusion.png",
                size,
                &actual,
            );
        }
    }
    assert!(
        observed_occlusion,
        "the fixture must expose absent terrain clipping"
    );
    assert_eq!(sprites.builds(), 1);
    assert_eq!(materials.builds(), 1);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn flat_ground_environment_ignores_source_geometry_and_bad_metadata_rejects_atomically() {
    use scenevm::isometric::{GpuSpriteSource, Uniforms};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut surface = isometric::surface::SurfaceFrame::new(4);
    for i in 0..4 {
        surface.set(i, Vec3::unit_x(), 2., 0.7, 0., 0.);
    }
    let source = GpuSpriteSource::new(
        &gpu,
        2,
        2,
        &[150, 150, 150, 255].repeat(4),
        Some(&surface.data),
    )
    .unwrap();
    let canvas = IsometricCanvas::new(&gpu, 8, 8).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let blit = SpriteBlit {
        source: &source,
        origin: [0.; 2],
        pixel_scale: 4.,
        base_height: 0.,
        world_scale: 1.,
        opacity: 1.,
        bitmap: false,
        prelit: false,
        ground: true,
        anchor_y: 2.,
        pixels_per_unit: 2.,
    };
    let uniforms = Uniforms {
        counts: [8, 8, 0, 0],
        projection: [4., 4., 4., 8.],
        sun: [0., 1., 0., 0.],
        sun_color: [1., 1., 1., 0.],
        ..Default::default()
    };
    blitter.render(&canvas, [0; 4], &[blit]).unwrap();
    let output = renderer
        .render(&canvas, &uniforms, &[], &[], &[], &[], None, 1)
        .unwrap();
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert!(
        capture(&gpu, output.image)
            .chunks_exact(4)
            .all(|p| p == [0, 0, 0, 255]),
        "upright source normals must still control ordinary source lighting"
    );
    let flat = SpriteEnvironment {
        flat_surface: true,
        ..Default::default()
    };
    blitter
        .append_with_environment(&canvas, &[blit], &[flat])
        .unwrap();
    let output = renderer
        .render(&canvas, &uniforms, &[], &[], &[], &[], None, 2)
        .unwrap();
    assert!(
        capture(&gpu, output.image)
            .chunks_exact(4)
            .all(|p| p == [150, 150, 150, 255]),
        "shared ground uses flat up normals and its base plane"
    );
    let before = gpu.transfers();
    assert!(
        blitter
            .append_with_environment(&canvas, &[blit], &[])
            .is_err()
    );
    for case in 0..3 {
        let mut invalid = SpriteTerrainClip {
            foot_y: 4.,
            foot_height: 0.,
            foot_depth: 0.,
            unit: 4.,
        };
        match case {
            0 => invalid.foot_y = f32::NAN,
            1 => invalid.unit = 0.,
            _ => invalid.foot_depth = f32::INFINITY,
        }
        assert!(
            blitter
                .append_with_environment(
                    &canvas,
                    &[blit],
                    &[SpriteEnvironment {
                        terrain: Some(invalid),
                        ..Default::default()
                    }]
                )
                .is_err()
        );
        assert_eq!(gpu.transfers(), before);
    }
    let output = renderer
        .render(&canvas, &uniforms, &[], &[], &[], &[], None, 3)
        .unwrap();
    assert!(
        capture(&gpu, output.image)
            .chunks_exact(4)
            .all(|p| p == [150, 150, 150, 255])
    );
}
