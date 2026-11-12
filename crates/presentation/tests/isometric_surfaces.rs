use eldiron_presentation::{
    isometric::{
        IsometricMaterialSources, IsometricSourceVersion, IsometricSurfaceRenderer, SurfaceKind,
        SurfaceTriangle,
    },
    screen::ContentRevision,
};
use renderkit::GpuContext;
use rusterix::{
    Assets, Map, PixelSource, RenderSettings, Texture, Tile, Value, Vertex, VertexBlendPreset,
    isometric::{
        self, Projection, SceneCache, SpriteLayout, lighting::Lighting, materials::SurfaceMaterials,
    },
};
use scenevm::{
    isometric::{IsometricBlitter, IsometricCanvas, IsometricRenderer, SpriteContacts},
    prelude::{Vec2, Vec3},
};
use std::collections::HashMap;
use theframework::prelude::{TheDim, TheRGBABuffer, Uuid};
mod isometric_support;
use isometric_support::*;

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_floor_triangles_match_reference_pages_overrides_blends_clamp_and_height() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let base = material([30, 50, 120], true);
    let paint = material([155, 35, 25], false);
    let blend = material([25, 140, 45], false);
    let ids = [base.id, paint.id, blend.id];
    let mut map = Map::default();
    sector(&mut map, base.id);
    map.properties.set(
        "tiles",
        Value::TileOverrides(
            [((0, 0), PixelSource::TileId(paint.id))]
                .into_iter()
                .collect(),
        ),
    );
    map.sectors[0].properties.set(
        "blend_tiles",
        Value::BlendOverrides(
            [(
                (-1, -1),
                (VertexBlendPreset::Left, PixelSource::TileId(blend.id)),
            )]
            .into_iter()
            .collect(),
        ),
    );
    let mut assets = Assets::default();
    assets.set_tiles(
        [base, paint, blend]
            .into_iter()
            .map(|t| (t.id, t))
            .collect(),
    );
    let version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: assets.tiles_revision,
    };
    let mut materials = IsometricMaterialSources::new(gpu.clone(), 16 * 1024 * 1024, 64);
    let mut raster = IsometricSurfaceRenderer::new(gpu.clone()).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let size = (128, 96);
    let canvas = IsometricCanvas::new(&gpu, size.0, size.1).unwrap();
    let mut scene = scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1);
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let mut settings = RenderSettings::default();
    settings.sun_enabled = false;
    settings.ambient_strength = 0.7;
    settings.ambient_color = [1.; 3];
    let lighting = Lighting::new(&settings, std::iter::empty(), 0);
    for (pass, (height, repeat, origin, unit, seconds)) in [
        (0., true, [48.3, 45.8], 20., 0.),
        (0., true, [48.3, 45.8], 20., 0.25),
        (1.25, true, [56.5, 70.5], 24., 0.25),
        (0., false, [61.2, 38.4], 17.5, 0.5),
        (0., true, [-5.6, 60.7], 28., 0.),
    ]
    .into_iter()
    .enumerate()
    {
        map.sectors[0]
            .properties
            .set("floor_height", Value::Float(height));
        map.sectors[0]
            .properties
            .set("tile_mode", Value::Int(i32::from(repeat)));
        map.changed = map.changed.wrapping_add(1);
        let projection = Projection {
            origin: Vec2::from(origin),
            unit,
        };
        let plan = cache.prepare(&map, &assets, seconds, &avatars);
        let floor = &map.sectors[0];
        let tile = &assets.tile_list[assets.tile_index(&ids[0]).unwrap() as usize];
        let policy = SurfaceMaterials::for_floor(&map, floor, tile, &assets);
        let bounds = floor.bounding_box(&map);
        let bounds = [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y];
        let (points, indices) = floor.generate_geometry(&map).unwrap();
        let mut prepared = Vec::new();
        for z in -1..1 {
            for x in -1..2 {
                let selection = policy.cell((x, z)).unwrap();
                let source = materials
                    .prepare(selection.primary, plan.material_frame, version)
                    .unwrap();
                let secondary = selection.secondary.map(|(weights, tile)| {
                    (
                        materials
                            .prepare(tile, plan.material_frame, version)
                            .unwrap(),
                        weights,
                    )
                });
                for (a, b, c) in &indices {
                    prepared.push(Prepared {
                        vertices: [*a, *b, *c].map(|i| [points[i][0], height, points[i][1]]),
                        source: source.clone(),
                        secondary: secondary.clone(),
                        cell: Some([x, z]),
                    });
                }
            }
        }
        let draws = triangles(&prepared, SurfaceKind::Floor, repeat, bounds);
        let before = gpu.transfers().readback_bytes;
        blitter.render(&canvas, [17, 34, 29, 255], &[]).unwrap();
        raster.append(&canvas, projection, &draws).unwrap();
        let light = lighting.gpu_parameters(projection, size, &plan.casters, false, 0);
        let output = renderer
            .render(
                &canvas,
                &light.uniforms,
                &light.lights,
                &light.casters,
                &light.columns,
                &[],
                None,
                pass as u64,
            )
            .unwrap();
        assert_eq!(gpu.transfers().readback_bytes, before);
        assert!(!renderer.has_cpu_storage() && !renderer.has_readback_storage());
        let pixels = capture(&gpu, output.image);
        if pass == 0 {
            save_png(
                "/private/tmp/eldiron-native-isometric-floor.png",
                size,
                &pixels,
            );
        }
        let mut reference = TheRGBABuffer::new(TheDim::sized(size.0 as i32, size.1 as i32));
        isometric::draw_scene_gpu_cached(
            &map,
            &assets,
            &mut reference,
            projection,
            seconds,
            None,
            &avatars,
            None,
            &lighting,
            &mut scene,
            &[],
        )
        .unwrap();
        mismatch(&pixels, reference.pixels(), &format!("floor pass {pass}"));
    }
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_terrain_triangles_match_reference_materials_depth_and_live_lighting() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let base = material([50, 85, 35], true);
    let paint = material([135, 65, 25], false);
    let blend = material([35, 60, 135], false);
    let mut map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
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
        Value::Source(PixelSource::TileId(base.id)),
    );
    map.properties.set(
        "tiles",
        Value::TileOverrides(
            [((0, 0), PixelSource::TileId(paint.id))]
                .into_iter()
                .collect(),
        ),
    );
    map.properties.set(
        "blend_tiles",
        Value::BlendOverrides(
            [(
                (-1, 0),
                (VertexBlendPreset::Left, PixelSource::TileId(blend.id)),
            )]
            .into_iter()
            .collect(),
        ),
    );
    let mut assets = Assets::default();
    assets.set_tiles(
        [base, paint, blend]
            .into_iter()
            .map(|t| (t.id, t))
            .collect(),
    );
    let version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: assets.tiles_revision,
    };
    let mut materials = IsometricMaterialSources::new(gpu.clone(), 16 * 1024 * 1024, 64);
    let mut raster = IsometricSurfaceRenderer::new(gpu.clone()).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let mut scene = scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1);
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let size = (128, 96);
    let canvas = IsometricCanvas::new(&gpu, size.0, size.1).unwrap();
    let mut settings = RenderSettings::default();
    settings.sun_enabled = true;
    settings.sun_direction = [-0.5, -1., 0.7];
    settings.sun_intensity = 0.6;
    settings.sun_color = [1.; 3];
    settings.ambient_strength = 0.22;
    settings.ambient_color = [1.; 3];
    settings.raster_shadow_enabled = false;
    let point = scenevm::Light::new_pointlight(Vec3::new(0.2, 2., 0.5))
        .with_color(Vec3::new(1., 0.2, 0.1))
        .with_intensity(0.9)
        .with_end_distance(7.);
    for (pass, (origin, unit, seconds)) in [
        ([62.3, 61.7], 20., 0.),
        ([57.5, 68.2], 23.5, 0.25),
        ([30.8, 62.5], 28., 0.5),
    ]
    .into_iter()
    .enumerate()
    {
        let lighting = Lighting::new(&settings, std::iter::once(&point), 0);
        let projection = Projection {
            origin: Vec2::from(origin),
            unit,
        };
        let plan = cache.prepare(&map, &assets, seconds, &avatars);
        let terrain = plan.terrain.as_ref().unwrap();
        let policy = SurfaceMaterials::for_terrain(&map, &assets);
        let mut prepared = Vec::new();
        terrain.triangles(terrain.bounds, |triangle| {
            let center = (triangle[0] + triangle[1] + triangle[2]) / 3.;
            let selection = policy
                .cell((center.x.floor() as i32, center.z.floor() as i32))
                .unwrap();
            let source = materials
                .prepare(selection.primary, plan.material_frame, version)
                .unwrap();
            let secondary = selection.secondary.map(|(weights, tile)| {
                (
                    materials
                        .prepare(tile, plan.material_frame, version)
                        .unwrap(),
                    weights,
                )
            });
            prepared.push(Prepared {
                vertices: triangle.map(|p| p.into_array()),
                source,
                secondary,
                cell: None,
            });
        });
        let geometry = cache.shadow_geometry(&map, &assets, &plan);
        let draws = triangles(&prepared, SurfaceKind::Terrain, true, [0.; 4]);
        let before = gpu.transfers().readback_bytes;
        blitter.render(&canvas, [17, 34, 29, 0], &[]).unwrap();
        raster.append(&canvas, projection, &draws).unwrap();
        let light = lighting.gpu_parameters(projection, size, &plan.casters, geometry.is_some(), 0);
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
        assert!(!renderer.has_cpu_storage() && !renderer.has_readback_storage());
        let pixels = capture(&gpu, output.image);
        if pass == 1 {
            save_png(
                "/private/tmp/eldiron-native-isometric-terrain.png",
                size,
                &pixels,
            );
        }
        let mut reference = TheRGBABuffer::new(TheDim::sized(size.0 as i32, size.1 as i32));
        isometric::draw_scene_gpu_cached(
            &map,
            &assets,
            &mut reference,
            projection,
            seconds,
            None,
            &avatars,
            None,
            &lighting,
            &mut scene,
            &[],
        )
        .unwrap();
        mismatch(&pixels, reference.pixels(), &format!("terrain pass {pass}"));
    }
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_surface_and_sprite_appends_preserve_supported_actors_and_existing_ground() {
    use eldiron_presentation::isometric::IsometricSources;
    use isometric::scene::ContactPolicy;
    use scenevm::isometric::{GpuSpriteSource, SpriteBlit};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let floor = material([105, 70, 30], false);
    let floor_id = floor.id;
    let mut image = vec![0; 32 * 32 * 4];
    for y in 26..31 {
        for x in 15..18 {
            image[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[25, 185, 40, 255]);
        }
    }
    let mut actor = Tile::from_texture(Texture::new(image, 32, 32));
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
    sector(&mut map, floor.id);
    map.sectors[0]
        .properties
        .set("floor_height", Value::Float(1.25));
    let mut entity = rusterix::Entity::default();
    entity.position = Vec3::new(0.8, 1.25, -0.2);
    entity.set_attribute("source", Value::Source(PixelSource::TileId(actor.id)));
    map.entities.push(entity);
    let mut assets = Assets::default();
    assets.set_tiles([floor, actor].into_iter().map(|t| (t.id, t)).collect());
    let version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: assets.tiles_revision,
    };
    let mut sprites = IsometricSources::new(gpu.clone(), 1024 * 1024, 16);
    let mut materials = IsometricMaterialSources::new(gpu.clone(), 1024 * 1024, 16);
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let plan = cache.prepare(&map, &assets, 0., &avatars);
    let policy = ContactPolicy::new(&map, &plan.upright);
    assert_eq!(plan.upright.len(), 2);
    assert!(plan.upright[0].floor.is_none() && plan.upright[1].floor.is_some());
    assert!(policy.protected[1][policy.owner_by_instance[0] as usize]);
    let size = (128, 96);
    let projection = Projection {
        origin: Vec2::new(48., 70.),
        unit: 24.,
    };
    let canvas = IsometricCanvas::new(&gpu, size.0, size.1).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut raster = IsometricSurfaceRenderer::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
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
    let contacts = SpriteContacts {
        owner: policy.owner_by_instance[0],
        protected: &policy.protected[0],
    };
    blitter.render(&canvas, [17, 34, 29, 255], &[]).unwrap();
    blitter
        .append_with_contacts(&canvas, &[blit], &[contacts])
        .unwrap();
    let f = &map.sectors[0];
    let floor = &assets.tile_list[assets.tile_index(&floor_id).unwrap() as usize];
    let src = materials
        .prepare(floor, plan.material_frame, version)
        .unwrap();
    let (points, indices) = f.generate_geometry(&map).unwrap();
    let prepared = indices
        .into_iter()
        .map(|(a, b, c)| Prepared {
            vertices: [a, b, c].map(|i| [points[i][0], 1.25, points[i][1]]),
            source: src.clone(),
            secondary: None,
            cell: None,
        })
        .collect::<Vec<_>>();
    let bounds = f.bounding_box(&map);
    let mut draws = triangles(
        &prepared,
        SurfaceKind::Floor,
        true,
        [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y],
    );
    for draw in &mut draws {
        draw.contacts = SpriteContacts {
            owner: 0,
            protected: &policy.protected[1],
        };
    }
    raster.append(&canvas, projection, &draws).unwrap();
    let mut settings = RenderSettings::default();
    settings.sun_enabled = false;
    settings.ambient_color = [1.; 3];
    settings.ambient_strength = 0.7;
    let lighting = Lighting::new(&settings, std::iter::empty(), 0);
    let light = lighting.gpu_parameters(projection, size, &plan.casters, false, 0);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let output = renderer
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
        .unwrap();
    let actual = capture(&gpu, output.image);
    let mut reference = TheRGBABuffer::new(TheDim::sized(size.0 as i32, size.1 as i32));
    let mut scene = scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1);
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
    mismatch(&actual, reference.pixels(), "mixed surface/sprite appends");
    assert!(actual.chunks_exact(4).any(|p| p == [25, 185, 40, 255]));
    let marker = GpuSpriteSource::new(&gpu, 1, 1, &[230, 30, 150, 255], None).unwrap();
    blitter
        .append(
            &canvas,
            &[SpriteBlit {
                source: &marker,
                origin: [124., 92.],
                pixel_scale: 4.,
                base_height: 0.,
                world_scale: 1.,
                opacity: 1.,
                bitmap: true,
                prelit: true,
                ground: true,
                anchor_y: 1.,
                pixels_per_unit: 1.,
            }],
        )
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
            2,
        )
        .unwrap();
    let appended = capture(&gpu, output.image);
    for y in 0..size.1 {
        for x in 0..size.0 {
            let at = ((y * size.0 + x) * 4) as usize;
            if x >= 124 && y >= 92 {
                assert_eq!(&appended[at..at + 4], &[230, 30, 150, 255]);
            } else {
                assert_eq!(
                    &appended[at..at + 4],
                    &actual[at..at + 4],
                    "append/load must retain prior ground and actor pixels"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn surface_batches_reject_bad_geometry_and_foreign_resources_before_touching_the_canvas() {
    use scenevm::isometric::{GpuSpriteSource, SpriteBlit};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let other = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut sources = IsometricMaterialSources::new(gpu.clone(), 1024 * 1024, 8);
    let mut foreign_sources = IsometricMaterialSources::new(other.clone(), 1024 * 1024, 8);
    let tile = material([180, 20, 10], false);
    let version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: 1,
    };
    let source = sources.prepare(&tile, 0, version).unwrap();
    let foreign = foreign_sources.prepare(&tile, 0, version).unwrap();
    let canvas = IsometricCanvas::new(&gpu, 8, 8).unwrap();
    let foreign_canvas = IsometricCanvas::new(&other, 8, 8).unwrap();
    let marker = GpuSpriteSource::new(&gpu, 1, 1, &[33, 66, 99, 255], None).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    blitter
        .render(
            &canvas,
            [0; 4],
            &[SpriteBlit {
                source: &marker,
                origin: [0.; 2],
                pixel_scale: 8.,
                base_height: 0.,
                world_scale: 1.,
                opacity: 1.,
                bitmap: true,
                prelit: true,
                ground: true,
                anchor_y: 1.,
                pixels_per_unit: 1.,
            }],
        )
        .unwrap();
    let mut raster = IsometricSurfaceRenderer::new(gpu.clone()).unwrap();
    let projection = Projection {
        origin: Vec2::new(4., 4.),
        unit: 4.,
    };
    let triangle = |source| SurfaceTriangle {
        vertices: [[0., 0., 0.], [0., 0., 1.], [1., 0., 0.]],
        source,
        secondary: None,
        paint: None,
        kind: SurfaceKind::Floor,
        repeat: true,
        uv_bounds: [0., 0., 1., 1.],
        clip_cell: None,
        contacts: SpriteContacts {
            owner: 0,
            protected: &[],
        },
    };
    for case in 0..4 {
        let mut invalid = triangle(&source);
        match case {
            0 => invalid.vertices[0][0] = f32::NAN,
            1 => invalid.uv_bounds[0] = f32::INFINITY,
            2 => invalid.secondary = Some((&source, [f32::NAN; 4])),
            _ => invalid.vertices[2][0] = f32::MAX,
        }
        let before = gpu.transfers();
        assert!(
            raster
                .append(&canvas, projection, &[triangle(&source), invalid])
                .is_err()
        );
        assert_eq!(before, gpu.transfers());
    }
    let before = gpu.transfers();
    assert!(
        raster
            .append(
                &canvas,
                Projection {
                    unit: 0.,
                    ..projection
                },
                &[triangle(&source)]
            )
            .is_err()
    );
    assert!(
        raster
            .append(&foreign_canvas, projection, &[triangle(&source)])
            .is_err()
    );
    assert!(
        raster
            .append(&canvas, projection, &[triangle(&foreign)])
            .is_err()
    );
    let mut mixed = triangle(&source);
    mixed.secondary = Some((&foreign, [1.; 4]));
    assert!(raster.append(&canvas, projection, &[mixed]).is_err());
    assert_eq!(before, gpu.transfers());
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let output = renderer
        .render(
            &canvas,
            &scenevm::isometric::Uniforms {
                counts: [8, 8, 0, 0],
                projection: [4., 4., 4., 8.],
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
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert!(
        capture(&gpu, output.image)
            .chunks_exact(4)
            .all(|p| p == [33, 66, 99, 255]),
        "invalid batches must preserve all previous canvas pixels"
    );
}
