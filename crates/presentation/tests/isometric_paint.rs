use eldiron_presentation::{
    isometric::{
        GpuTerrainPaintSource, IsometricMaterialSources, IsometricPaintSources,
        IsometricSourceVersion, IsometricSurfaceRenderer, SurfaceKind,
    },
    screen::ContentRevision,
};
use renderkit::GpuContext;
use rusterix::{
    Assets, Map, PixelSource, RenderSettings, Value, Vertex,
    isometric::{
        self, Projection, SceneCache,
        lighting::Lighting,
        materials::SurfaceMaterials,
        paint::{CHUNK_SIZE, TerrainPaint, TerrainPaintChunk},
    },
    material_library::{MaterialDefinition, SEMANTIC_MATERIAL_MARKER},
};
use scenevm::{
    isometric::{IsometricBlitter, IsometricCanvas, IsometricRenderer},
    prelude::Vec2,
};
use std::{collections::HashMap, sync::Arc};
use theframework::prelude::{TheDim, TheRGBABuffer, Uuid};
mod isometric_support;
use isometric_support::*;
fn painted() -> TerrainPaint {
    let id = MaterialDefinition::from_preset_finish("metal", "polished").id();
    let mut paint = TerrainPaint::default();
    paint.ground_key = 1;
    for cy in -2..=1 {
        for cx in -2..=1 {
            let mut chunk = TerrainPaintChunk {
                color: vec![0; (CHUNK_SIZE * CHUNK_SIZE * 4) as usize].into(),
                material: vec![0; (CHUNK_SIZE * CHUNK_SIZE * 4) as usize].into(),
            };
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let mode = (x / 8 + y / 8 + cx + cy).rem_euclid(12);
                    let (marker, a, b, amount, alpha) = match mode {
                        0 => (0, 0, 0, 0, 100),
                        1 => (64, 0x38, 0xa7, 128, 140),
                        2 => (123, 0x38, 0xa7, 96, 190),
                        3 => (128, 0x38, 0xa7, 150, 128),
                        4 => (187, 0x38, 0xa7, 64, 255),
                        5 => (252, 0x38, 0xa7, 255, 0),
                        6 => (253, 0x38, 0xa7, 96, 120),
                        7 => (254, id, 0, 128, 110),
                        8 => (254, id, 1, 255, 180),
                        9 => (254, id, 1, 255, 0),
                        10 => (32, 0, 0, 0, 90),
                        _ => (64, SEMANTIC_MATERIAL_MARKER, id, 128, 170),
                    };
                    let at = ((y * CHUNK_SIZE + x) * 4) as usize;
                    chunk.color[at..at + 4].copy_from_slice(&[
                        170 + (x as u8 % 30),
                        40 + (y as u8 % 50),
                        70,
                        alpha,
                    ]);
                    chunk.material[at..at + 4].copy_from_slice(&[marker, a, b, amount]);
                }
            }
            paint
                .chunks
                .insert([cx * CHUNK_SIZE, cy * CHUNK_SIZE], chunk);
        }
    }
    paint
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_paint_matches_reference_negative_chunks_replacement_erasure_and_material_channels() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut producer =
        eldiron_presentation::isometric::IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let base = material([35, 65, 105], true);
    let id = base.id;
    let mut assets = Assets::default();
    assets.set_tiles([(base.id, base)].into_iter().collect());
    let mut paints = IsometricPaintSources::new(gpu.clone(), 32 * 1024 * 1024, 128);
    let mut materials = IsometricMaterialSources::new(gpu.clone(), 1024 * 1024, 16);
    let mut raster = IsometricSurfaceRenderer::new(gpu.clone()).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let mut scene = scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1);
    let size = (128, 96);
    let canvas = IsometricCanvas::new(&gpu, size.0, size.1).unwrap();
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let mut settings = RenderSettings::default();
    settings.sun_enabled = true;
    settings.sun_direction = [-0.5, -1., 0.7];
    settings.sun_intensity = 0.5;
    settings.ambient_color = [1.; 3];
    settings.ambient_strength = 0.25;
    settings.raster_shadow_enabled = false;
    let lighting = Lighting::new(&settings, std::iter::empty(), 0);
    for terrain in [false, true] {
        let mut map = Map::default();
        if terrain {
            map.vertices = vec![Vertex::new(0, -2., -2.), Vertex::new(1, 2., 2.)];
            let mut peak = Vertex::new(2, 0.5, 0.5);
            peak.z = 0.7;
            peak.properties.set("terrain_control", Value::Bool(true));
            peak.properties.set("smoothness", Value::Float(0.5));
            map.vertices.push(peak);
            map.properties.set("terrain_enabled", Value::Bool(true));
            map.properties.set("terrain_subdivisions", Value::Int(2));
            map.properties.set("terrain_edge_falloff", Value::Float(0.));
            map.properties.set(
                "default_terrain_tile",
                Value::Source(PixelSource::TileId(id)),
            );
        } else {
            sector(&mut map, id);
        }
        assets.terrain_paint.insert(map.id, Arc::new(painted()));
        let paint = &assets.terrain_paint[&map.id];
        let version = IsometricSourceVersion {
            namespace: Uuid::new_v4(),
            content: ContentRevision(1),
            tiles: assets.tiles_revision,
        };
        for (pass, (origin, unit, seconds)) in [
            ([64.5, 60.5], 20., 0.),
            ([57.5, 68.2], 23.5, 0.25),
            ([37.2, 64.1], 28., 0.),
        ]
        .into_iter()
        .enumerate()
        {
            let projection = Projection {
                origin: Vec2::from(origin),
                unit,
            };
            let plan = cache.prepare(&map, &assets, seconds, &avatars);
            let mut prepared = Vec::new();
            let mut paint_sources = Vec::<Option<Arc<GpuTerrainPaintSource>>>::new();
            if let Some(model) = &plan.terrain {
                let policy = SurfaceMaterials::for_terrain(&map, &assets);
                model.triangles(model.bounds, |t| {
                    let center = (t[0] + t[1] + t[2]) / 3.;
                    let cell = [center.x.floor() as i32, center.z.floor() as i32];
                    let selected = policy.cell((cell[0], cell[1])).unwrap();
                    let source = materials
                        .prepare(selected.primary, plan.material_frame, version)
                        .unwrap();
                    prepared.push(Prepared {
                        vertices: t.map(|p| p.into_array()),
                        source,
                        secondary: None,
                        cell: None,
                    });
                    paint_sources.push(paints.prepare(paint, map.id, cell, version).unwrap());
                });
            } else {
                let floor = &map.sectors[0];
                let (points, indices) = floor.generate_geometry(&map).unwrap();
                let tile = &assets.tile_list[assets.tile_index(&id).unwrap() as usize];
                let policy = SurfaceMaterials::for_floor(&map, floor, tile, &assets);
                for z in -1..1 {
                    for x in -1..2 {
                        let selected = policy.cell((x, z)).unwrap();
                        let source = materials
                            .prepare(selected.primary, plan.material_frame, version)
                            .unwrap();
                        let source_paint = paints.prepare(paint, map.id, [x, z], version).unwrap();
                        for (a, b, c) in &indices {
                            prepared.push(Prepared {
                                vertices: [*a, *b, *c].map(|i| [points[i][0], 0., points[i][1]]),
                                source: source.clone(),
                                secondary: None,
                                cell: Some([x, z]),
                            });
                            paint_sources.push(source_paint.clone());
                        }
                    }
                }
            }
            let kind = if terrain {
                SurfaceKind::Terrain
            } else {
                SurfaceKind::Floor
            };
            let mut draws = triangles(&prepared, kind, true, [0.; 4]);
            for (draw, paint) in draws.iter_mut().zip(&paint_sources) {
                draw.paint = paint.as_deref();
            }
            let geometry = cache.shadow_geometry(&map, &assets, &plan);
            let light =
                lighting.gpu_parameters(projection, size, &plan.casters, geometry.is_some(), 0);
            let before = gpu.transfers().readback_bytes;
            blitter
                .render(
                    &canvas,
                    if terrain {
                        [17, 34, 29, 0]
                    } else {
                        [17, 34, 29, 255]
                    },
                    &[],
                )
                .unwrap();
            raster.append(&canvas, projection, &draws).unwrap();
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
                seconds,
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
                    seconds: seconds,
                    projection,
                    lighting: &lighting,
                    particles: &[],
                    source_version: version,
                },
                reference.pixels(),
                "assembled native producer",
            );
            mismatch(
                &actual,
                reference.pixels(),
                &format!("paint terrain {terrain}, pass {pass}"),
            );
            if terrain && pass == 1 {
                save_png(
                    "/private/tmp/eldiron-native-isometric-paint.png",
                    size,
                    &actual,
                );
            }
        }
    }
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn paint_neighborhoods_are_versioned_bounded_and_validate_before_upload() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut cache = IsometricPaintSources::new(gpu.clone(), u64::MAX, 4);
    let mut paint = painted();
    let map = Uuid::new_v4();
    let mut version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: 1,
    };
    let first = cache
        .prepare(&paint, map, [-1, -1], version)
        .unwrap()
        .unwrap();
    let uploaded = gpu.transfers().uploaded_bytes;
    assert!(Arc::ptr_eq(
        &first,
        &cache
            .prepare(&paint, map, [-2, -2], version)
            .unwrap()
            .unwrap()
    ));
    assert_eq!(uploaded, gpu.transfers().uploaded_bytes);
    paint.chunks.get_mut(&[-64, -64]).unwrap().color[0] = 42;
    paint.ground_key += 1;
    let changed = cache
        .prepare(&paint, map, [-1, -1], version)
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    version.content.0 += 1;
    assert!(!Arc::ptr_eq(
        &changed,
        &cache
            .prepare(&paint, map, [-1, -1], version)
            .unwrap()
            .unwrap()
    ));
    assert!(
        cache
            .prepare(&paint, map, [100, 100], version)
            .unwrap()
            .is_none()
    );
    assert_eq!(cache.len(), 4);
    // Missing neighborhoods also obey entry limits, without fake GPU uploads.
    let uploaded = gpu.transfers().uploaded_bytes;
    for x in 101..110 {
        assert!(
            cache
                .prepare(&paint, map, [x * 2, 100], version)
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(cache.len(), 4);
    assert_eq!(cache.resident_bytes(), 0);
    assert_eq!(uploaded, gpu.transfers().uploaded_bytes);
    assert!(first.resident_bytes() > 0); // leases survive eviction
    let mut zero = IsometricPaintSources::new(gpu.clone(), 0, 0);
    let uncached = zero.prepare(&paint, map, [-1, -1], version).unwrap().unwrap();
    let uploads = gpu.transfers();
    let builds = zero.builds();
    for _ in 0..100 {
        let repeated = zero.prepare(&paint, map, [-1, -1], version).unwrap().unwrap();
        assert!(Arc::ptr_eq(&uncached, &repeated));
    }
    assert_eq!(gpu.transfers(), uploads);
    assert_eq!(zero.builds(), builds);
    let retired = Arc::downgrade(&uncached);
    drop(uncached);
    assert!(retired.upgrade().is_none());
    assert!(zero.is_empty());
    assert_eq!(zero.resident_bytes(), 0);
    let uploaded = gpu.transfers().uploaded_bytes;
    paint.ground_key += 1;
    paint.chunks.get_mut(&[-64, -64]).unwrap().material.pop();
    assert!(cache.prepare(&paint, map, [-1, -1], version).is_err());
    assert_eq!(uploaded, gpu.transfers().uploaded_bytes);
    assert_eq!(cache.len(), 4);
    cache.clear();
    assert!(cache.is_empty());
    let unrelated = gpu
        .acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(
            first.resident_bytes(),
        ))
        .unwrap();
    assert_ne!(unrelated.buffer(), first.buffer());
    assert_ne!(unrelated.buffer(), changed.buffer());
    gpu.set_working_buffer_cache_limits(0, 0);
    assert_eq!(
        gpu.working_buffer_pool_usage().live_bytes,
        first.resident_bytes() * 2 + changed.resident_bytes()
    );
    drop((unrelated, first, changed));
    assert_eq!(gpu.working_buffer_pool_usage().live_bytes, 0);
}
