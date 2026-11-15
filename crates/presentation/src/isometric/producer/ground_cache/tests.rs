use super::*;
use crate::screen::ContentRevision;
use renderkit::{ReadbackTicket, wgpu};
use rusterix::{PixelSource, RenderSettings, Texture, Value, Vertex};
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
    let mut frame = cached.prepare((128, 96), &input).unwrap();
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
