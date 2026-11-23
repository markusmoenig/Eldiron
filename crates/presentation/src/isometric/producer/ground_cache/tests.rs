use super::*;
use crate::screen::ContentRevision;
use renderkit::{ReadbackTicket, wgpu};
use rusterix::{PixelSource, RenderSettings, Texture, Value, Vertex};
use scenevm::prelude::Vec2;
use std::time::Duration;
use theframework_core::Uuid;

fn wait(gpu: &GpuContext) {
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
}
fn render(
    gpu: &GpuContext,
    renderer: &mut IsometricSceneRenderer,
    frame: &IsometricPreparedFrame,
    revision: u64,
) -> Vec<u8> {
    let before = gpu.transfers();
    let image = renderer.render_prepared(frame, revision).unwrap().image;
    assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
    assert!(!renderer.has_cpu_storage() && !renderer.has_readback_storage());
    let mut capture = ReadbackTicket::capture(gpu, image).unwrap();
    wait(gpu);
    capture.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
fn planes(gpu: &GpuContext, canvas: &IsometricCanvas) -> Vec<u8> {
    let pixels = u64::from(canvas.size().0) * u64::from(canvas.size().1);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("explicit deferred cache test oracle"),
        size: pixels * 24,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    let mut offset = 0;
    for (source, stride) in [
        (canvas.albedo(), 4),
        (canvas.surfaces(), 8),
        (canvas.owners(), 4),
        (canvas.terrain(), 8),
    ] {
        encoder.copy_buffer_to_buffer(source, 0, &buffer, offset, pixels * stride);
        offset += pixels * stride;
    }
    gpu.queue.submit([encoder.finish()]);
    let (send, ready) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap();
        });
    wait(gpu);
    ready.recv().unwrap().unwrap();
    let bytes = buffer.slice(..).get_mapped_range().to_vec();
    buffer.unmap();
    bytes
}

#[test]
#[ignore = "native GPU; retained terrain parity and camera-independent GPU residency"]
fn retained_terrain_matches_ordered_terrain_while_camera_lighting_and_material_phase_change() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut first = Texture::new(vec![80, 140, 35, 255].repeat(16), 4, 4);
    first.set_materials_all(0.8, 0.1, 1., 0.);
    let mut second = Texture::new(vec![35, 90, 170, 255].repeat(16), 4, 4);
    second.set_materials_all(0.2, 0.7, 1., 0.3);
    let mut tile = Tile::from_texture(first);
    tile.textures.push(second);
    let mut map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
    map.vertices = vec![Vertex::new(0, -4., -3.), Vertex::new(1, 4., 3.)];
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set("terrain_subdivisions", Value::Int(4));
    map.vertices[0]
        .properties
        .set("terrain_control", Value::Bool(true));
    map.vertices[0].z = 1.25;
    map.properties.set(
        "terrain_sculpt",
        Value::HeightPoints(vec![rusterix::value::HeightControlPoint {
            position: [0.25, 0.75],
            height: 0.65,
        }]),
    );
    map.properties.set(
        "default_terrain_tile",
        Value::Source(PixelSource::TileId(tile.id)),
    );
    let mut assets = Assets::default();
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let mut settings = RenderSettings::default();
    settings.sun_enabled = false;
    settings.ambient_color = [1.; 3];
    let avatars = HashMap::new();
    let mut retained = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut reference = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut previous = None;
    let mut resident = None;
    let namespace = Uuid::new_v4();
    for (index, (origin, unit, seconds, ambient)) in [
        ([64., 48.], 15.5, 0., 0.8),
        ([63.25, 49.75], 16.25, 0., 0.3),
        ([64., 48.], 15.5, 0.25, 0.8),
    ]
    .into_iter()
    .enumerate()
    {
        settings.ambient_strength = ambient;
        let lighting = Lighting::new(&settings, std::iter::empty(), 0);
        let input = IsometricSceneInput {
            map: &map,
            assets: &assets,
            avatars: &avatars,
            seconds,
            projection: Projection {
                origin: origin.into(),
                unit,
            },
            lighting: &lighting,
            particles: &[],
            source_version: IsometricSourceVersion {
                namespace,
                content: ContentRevision(1),
                tiles: assets.tiles_revision,
            },
        };
        let actual = retained.prepare((128, 96), &input).unwrap();
        let draw = actual
            .terrain_draw
            .as_ref()
            .expect("retained terrain was eligible");
        if let Some(old) = &resident {
            assert!(
                Arc::ptr_eq(old, &draw.chunks),
                "pan, zoom, and time must reuse terrain"
            );
        }
        resident = Some(draw.chunks.clone());
        let expected = reference
            .prepare_impl((128, 96), &input, false, false, None)
            .unwrap();
        let actual_pixels = render(&gpu, &mut retained, &actual, index as u64);
        let expected_pixels = render(&gpu, &mut reference, &expected, index as u64);
        assert_eq!(pixels_mismatched(&actual_pixels, &expected_pixels), 0);
        assert_eq!(
            planes(&gpu, retained.canvas.as_ref().unwrap()),
            planes(&gpu, reference.canvas.as_ref().unwrap()),
            "terrain work must preserve depth, normal, and material planes"
        );
        if let Some(old) = &previous {
            assert_ne!(old, &actual_pixels);
        }
        previous = Some(actual_pixels);
    }
}

fn pixels_mismatched(a: &[u8], b: &[u8]) -> usize {
    a.chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count()
}

#[test]
#[ignore = "native GPU; shared editor, overview and Play terrain with independent rendering"]
fn overview_and_play_borrow_matching_terrain_and_keep_their_lighting() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let tile = Tile::from_texture(Texture::from_color([100, 165, 35, 255]));
    let mut map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
    map.vertices = vec![Vertex::new(0, -3., -3.), Vertex::new(1, 3., 3.)];
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set(
        "default_terrain_tile",
        Value::Source(PixelSource::TileId(tile.id)),
    );
    let mut assets = Assets::default();
    assets.set_tiles([(tile.id, tile.clone())].into_iter().collect());
    let lighting = Lighting::new(&RenderSettings::default(), std::iter::empty(), 0);
    let avatars = HashMap::new();
    let document = Uuid::new_v4();
    let shared = IsometricTerrainCache::default();
    shared.set_scope(Some((document, map.id, gpu.generation())));
    let mut authoring = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut overview = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let input = IsometricSceneInput {
        map: &map,
        assets: &assets,
        avatars: &avatars,
        seconds: 0.,
        projection: Projection { origin: Vec2::new(64., 48.), unit: 12. },
        lighting: &lighting,
        particles: &[],
        source_version: IsometricSourceVersion {
            namespace: Uuid::new_v4(),
            content: ContentRevision(1),
            tiles: assets.tiles_revision,
        },
    };
    let world = authoring.prepare_view((128, 96), &input, TerrainConsumer::Editor, Some(&shared)).unwrap();
    let mut projected = map.clone();
    projected.changed += 1;
    let mut republished = assets.clone();
    republished.tiles_revision += 1;
    let input = IsometricSceneInput {
        map: &projected,
        assets: &republished,
        projection: Projection { origin: Vec2::new(37., 30.), unit: 6. },
        source_version: IsometricSourceVersion {
            namespace: Uuid::new_v4(),
            content: ContentRevision(2),
            tiles: republished.tiles_revision,
        },
        ..input
    };
    let frame = overview.prepare_view((72, 60), &input, TerrainConsumer::Overview, Some(&shared)).unwrap();
    let chunks = &world.terrain_draw.as_ref().unwrap().chunks;
    assert!(Arc::ptr_eq(chunks, &frame.terrain_draw.as_ref().unwrap().chunks));
    let expected = overview.prepare_unlit((72, 60), &input).unwrap();
    assert_eq!(
        render(&gpu, &mut overview, &frame, 1),
        render(&gpu, &mut overview, &expected, 1),
    );

    let mut play = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut standalone = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut settings = RenderSettings::default();
    settings.apply_hour(22.);
    let light = Lighting::new(&settings, std::iter::empty(), 0);
    let mut runtime_map = projected.clone();
    runtime_map.properties.set("runtime_metadata", Value::Str("gameplay".into()));
    assert!(!map.same_overview_source(&runtime_map));
    let play_input = IsometricSceneInput {
        map: &runtime_map,
        lighting: &light,
        source_version: IsometricSourceVersion { namespace: Uuid::new_v4(), ..input.source_version },
        ..input
    };
    let frame = play.prepare_view((72, 60), &play_input, TerrainConsumer::Play, Some(&shared)).unwrap();
    assert!(Arc::ptr_eq(chunks, &frame.terrain_draw.as_ref().unwrap().chunks));
    let expected = standalone.prepare((72, 60), &play_input).unwrap();
    assert_eq!(
        render(&gpu, &mut play, &frame, 1),
        render(&gpu, &mut standalone, &expected, 1),
    );

    let mut tile = tile;
    tile.textures[0].data[0] ^= 0xff;
    let mut changed = Assets::default();
    changed.set_tiles([(tile.id, tile)].into_iter().collect());
    let edited = IsometricSceneInput { assets: &changed, ..input };
    let fallback = overview.prepare_view((72, 60), &edited, TerrainConsumer::Overview, Some(&shared)).unwrap();
    assert!(fallback.terrain_draw.is_none(), "changed material bytes cannot borrow stale terrain");
    let edited_play = IsometricSceneInput { assets: &changed, ..play_input };
    let mut changed_play = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let frame = changed_play.prepare_view(
        (72, 60), &edited_play, TerrainConsumer::Play, Some(&shared),
    ).unwrap();
    assert!(!Arc::ptr_eq(chunks, &frame.terrain_draw.as_ref().unwrap().chunks));
    let mut terrain_edit = runtime_map.clone();
    terrain_edit.changed += 1;
    terrain_edit.properties.set("terrain_edge_falloff", Value::Float(0.));
    terrain_edit.vertices[0].properties.set("terrain_control", Value::Bool(true));
    terrain_edit.vertices[0].z = 1.;
    let edited_play = IsometricSceneInput { map: &terrain_edit, ..play_input };
    let frame = changed_play.prepare_view(
        (72, 60), &edited_play, TerrainConsumer::Play, Some(&shared),
    ).unwrap();
    assert!(!Arc::ptr_eq(chunks, &frame.terrain_draw.as_ref().unwrap().chunks));
    let frame = overview.prepare_view((72, 60), &input, TerrainConsumer::Overview, Some(&shared)).unwrap();
    assert!(Arc::ptr_eq(chunks, &frame.terrain_draw.as_ref().unwrap().chunks));

    shared.set_scope(Some((Uuid::new_v4(), map.id, gpu.generation())));
    let fallback = overview.prepare_view((72, 60), &input, TerrainConsumer::Overview, Some(&shared)).unwrap();
    assert!(fallback.terrain_draw.is_none(), "another document must publish its own terrain");
}

#[test]
#[ignore = "native GPU; frozen terrain cache, weak ownership and shared admission"]
fn deferred_ground_reuse_preserves_all_planes_live_lighting_and_pressure_fallback() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut texture = Texture::new(
        (0..16)
            .flat_map(|i| [35 + i * 7, 80 + i * 3, 25, 255])
            .collect(),
        4,
        4,
    );
    texture.set_materials_all(0.5, 0.1, 1., 0.);
    let tile = Tile::from_texture(texture);
    let mut map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
    map.vertices = vec![Vertex::new(0, -2., -2.), Vertex::new(1, 2., 2.)];
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set("terrain_subdivisions", Value::Int(2));
    map.properties.set(
        "default_terrain_tile",
        Value::Source(PixelSource::TileId(tile.id)),
    );
    let mut assets = Assets::default();
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let mut settings = RenderSettings::default();
    settings.sun_enabled = false;
    settings.ambient_strength = 0.7;
    settings.ambient_color = [1.; 3];
    let lighting = Lighting::new(&settings, std::iter::empty(), 0);
    let avatars = HashMap::new();
    let input = IsometricSceneInput {
        map: &map,
        assets: &assets,
        avatars: &avatars,
        seconds: 0.,
        projection: Projection {
            origin: Vec2::new(63.25, 54.75),
            unit: 17.5,
        },
        lighting: &lighting,
        particles: &[],
        source_version: IsometricSourceVersion {
            namespace: Uuid::new_v4(),
            content: ContentRevision(1),
            tiles: assets.tiles_revision,
        },
    };
    let mut cached = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut reference = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut frame = cached
        .prepare_impl((128, 96), &input, false, false, None)
        .unwrap();
    let [Command::Surface(surfaces)] = frame.ground.as_slice() else {
        panic!("terrain fixture")
    };
    assert!(!surfaces.is_empty());
    let source = surfaces[0].source.clone();
    let owners = Arc::strong_count(&source);
    // A changing camera does not create snapshots; settle before admission.
    render(&gpu, &mut cached, &frame, 0);
    assert_eq!(cached.ground_cache_usage().gpu_bytes, 0);
    let mut previous = None;
    for step in 0..4 {
        // Lighting changes belong after the retained, unlit deferred ground.
        frame.lighting.uniforms.ambient[0] = 1. - step as f32 * 0.2;
        let actual = render(&gpu, &mut cached, &frame, step);
        reference.ground_cache.clear();
        assert_eq!(actual, render(&gpu, &mut reference, &frame, step));
        if let Some(old) = previous {
            assert!(actual != old, "live lighting must still change output");
        }
        previous = Some(actual);
        assert_eq!(
            Arc::strong_count(&source),
            owners,
            "cached signatures must not pin GPU sources"
        );
    }
    assert_eq!(cached.ground_cache_usage().hits, 3);
    let base = planes(&gpu, &cached.ground_cache.entry.as_ref().unwrap().canvas);
    let canvas = cached.canvas.as_ref().unwrap();
    for buffer in [
        canvas.albedo(),
        canvas.surfaces(),
        canvas.owners(),
        canvas.terrain(),
    ] {
        gpu.queue
            .write_buffer(buffer, 0, &vec![0xAB; buffer.size() as usize]);
    }
    assert!(cached.ground_cache.restore(&gpu, canvas, &frame).unwrap());
    assert_eq!(
        planes(&gpu, canvas),
        base,
        "all four deferred planes must be restored bit-exactly"
    );
    // Subpixel camera and tiny geometry edits invalidate even when they happen
    // to leave the current output pixels unchanged.
    frame.projection.origin.x += 0.125;
    let misses = cached.ground_cache_usage().misses;
    let actual = render(&gpu, &mut cached, &frame, 5);
    reference.ground_cache.clear();
    assert_eq!(actual, render(&gpu, &mut reference, &frame, 5));
    assert_eq!(cached.ground_cache_usage().misses, misses + 1);
    if let Command::Surface(s) = &mut frame.ground[0] {
        s[0].vertices[0][0] += 0.0001;
    }
    let actual = render(&gpu, &mut cached, &frame, 6);
    reference.ground_cache.clear();
    assert_eq!(actual, render(&gpu, &mut reference, &frame, 6));
    assert_eq!(cached.ground_cache_usage().misses, misses + 2);
    let replacement = Tile::from_texture(Texture::from_color([210, 25, 50, 255]));
    let replacement = cached
        .materials
        .prepare(&replacement, 0, input.source_version)
        .unwrap();
    if let Command::Surface(s) = &mut frame.ground[0] {
        s[0].source = replacement;
    }
    let actual = render(&gpu, &mut cached, &frame, 7);
    reference.ground_cache.clear();
    assert_eq!(actual, render(&gpu, &mut reference, &frame, 7));
    assert_eq!(
        cached.ground_cache_usage().misses,
        misses + 3,
        "new immutable publication invalidates terrain"
    );
    let cost = cached.ground_cache_usage().reserved_bytes;
    cached.ground_cache.clear();
    reference.ground_cache.clear();
    wait(&gpu);
    assert_eq!(gpu.retained_cache_usage().bytes, 0);
    gpu.set_retained_cache_limit(cost);
    render(&gpu, &mut cached, &frame, 7);
    let a = render(&gpu, &mut cached, &frame, 7);
    assert_eq!(gpu.retained_cache_usage().reservations, 1);
    assert_eq!(a, render(&gpu, &mut reference, &frame, 7));
    assert_eq!(
        reference.ground_cache_usage().gpu_bytes,
        0,
        "another view falls back under shared pressure"
    );
    cached.invalidate_geometry();
    wait(&gpu);
    assert_eq!(gpu.retained_cache_usage().bytes, 0);
    assert_eq!(a, render(&gpu, &mut reference, &frame, 8));
    assert!(reference.ground_cache_usage().gpu_bytes > 0);
    gpu.set_retained_cache_limit(0);
    assert_eq!(a, render(&gpu, &mut reference, &frame, 9));
    wait(&gpu);
    assert_eq!(gpu.retained_cache_usage().bytes, 0);
    assert_eq!(reference.ground_cache_usage().gpu_bytes, 0);
    // Cached terrain can remain while its original material publication dies.
    gpu.set_retained_cache_limit(64 * 1024 * 1024);
    render(&gpu, &mut cached, &frame, 10);
    render(&gpu, &mut cached, &frame, 11);
    let weak = Arc::downgrade(&source);
    drop(frame);
    drop(source);
    cached.materials.clear();
    assert!(cached.ground_cache_usage().gpu_bytes > 0);
    assert!(
        weak.upgrade().is_none(),
        "cached terrain must not keep original source storage alive"
    );
    cached.ground_cache.clear();
    wait(&gpu);
    assert_eq!(gpu.retained_cache_usage().bytes, 0);
}
