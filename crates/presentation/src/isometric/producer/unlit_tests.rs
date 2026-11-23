use super::*;
use crate::screen::ContentRevision;
use renderkit::{ReadbackTicket, wgpu};
use rusterix::{PixelSource, RenderSettings, Texture, Value, Vertex};
use scenevm::prelude::Vec2;
use std::time::Duration;
use theframework_core::Uuid;

fn capture(
    gpu: &GpuContext,
    renderer: &mut IsometricSceneRenderer,
    frame: &IsometricPreparedFrame,
    revision: u64,
) -> Vec<u8> {
    let before = gpu.transfers().readback_bytes;
    let output = renderer.render_prepared(frame, revision).unwrap();
    assert_eq!(gpu.transfers().readback_bytes, before);
    let mut ticket = ReadbackTicket::capture(gpu, output.image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}

#[test]
#[ignore = "native GPU; large terrain overview and narrow floor coverage"]
fn valley_overview_bounds_terrain_work_and_preserves_roads() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut renderer = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let grass = Tile::from_texture(Texture::from_color([35, 75, 30, 255]));
    let road_color = [170, 112, 65, 255];
    let road = Tile::from_texture(Texture::from_color(road_color));
    let mut map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
    map.vertices = vec![Vertex::new(0, -320., -320.), Vertex::new(1, 320., 320.)];
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set("terrain_subdivisions", Value::Int(4));
    map.properties.set("default_terrain_tile", Value::Source(PixelSource::TileId(grass.id)));
    let ids = [[-280., -4.], [280., -4.], [280., 4.], [-280., 4.]]
        .map(|[x, z]| map.add_vertex_at(x, z));
    for (a, b) in ids.into_iter().zip(ids.into_iter().cycle().skip(1)) {
        map.create_linedef_manual(a, b);
    }
    let sector_id = map.close_polygon_manual().unwrap();
    let sector = map.find_sector_mut(sector_id).unwrap();
    sector.properties.set("source", Value::Source(PixelSource::TileId(road.id)));
    sector.layer = Some(2);
    let mut assets = Assets::default();
    assets.set_tiles([grass, road].into_iter().map(|tile| (tile.id, tile)).collect());
    let lighting = Lighting::new(&RenderSettings::default(), std::iter::empty(), 0);
    let size = (128, 96);
    let frame = renderer.prepare_view(size, &IsometricSceneInput {
        map: &map,
        assets: &assets,
        avatars: &HashMap::new(),
        seconds: 0.,
        projection: Projection { origin: [64., 48.].into(), unit: 0.25 },
        lighting: &lighting,
        particles: &[],
        source_version: IsometricSourceVersion {
            namespace: Uuid::new_v4(),
            content: ContentRevision(1),
            tiles: assets.tiles_revision,
        },
    }, TerrainConsumer::Editor, None).unwrap();
    let [Command::Surface(surfaces)] = frame.ground.as_slice() else { panic!("overview missing") };
    assert!(surfaces.len() <= 2050);
    assert_eq!(surfaces.iter().filter(|tri| tri.kind == SurfaceKind::Floor).count(), 2);
    assert!(frame.geometry.is_none());
    let image = capture(&gpu, &mut renderer, &frame, 1);
    assert_eq!(&image[(48 * 128 + 64) * 4..][..4], &road_color);
}

#[test]
#[ignore = "native GPU; frozen unlit preview and deferred shadow allocation"]
fn unlit_previews_preserve_rgba_particles_and_later_lit_shadows() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut preview = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    assert_eq!(
        gpu.target_pool_usage().live_bytes,
        8,
        "an unused view needs only two one-pixel shadow bindings"
    );
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
    settings.ambient_strength = 0.05;
    settings.raster_shadow_enabled = true;
    let light = scenevm::Light::new_pointlight(scenevm::prelude::Vec3::new(0., 2., 0.));
    let lighting = Lighting::new(&settings, std::iter::once(&light), 0);
    let avatars = HashMap::new();
    let particles = [Particle {
        position_radius: [0., 1., 0., 0.4],
        color: [1., 0.1, 0.05, 0.8],
    }];
    let mut input = IsometricSceneInput {
        map: &map,
        assets: &assets,
        avatars: &avatars,
        seconds: 0.,
        projection: Projection {
            origin: Vec2::new(63.25, 54.75),
            unit: 17.5,
        },
        lighting: &lighting,
        particles: &particles,
        source_version: IsometricSourceVersion {
            namespace: Uuid::new_v4(),
            content: ContentRevision(1),
            tiles: assets.tiles_revision,
        },
    };
    let frame = preview.prepare_unlit((128, 96), &input).unwrap();
    assert!(frame.geometry.is_none());
    assert!(frame.lighting.lights.is_empty());
    assert!(frame.lighting.casters.is_empty() && frame.lighting.columns.is_empty());
    let actual = capture(&gpu, &mut preview, &frame, 1);
    assert!(gpu.target_pool_usage().live_bytes < 1024 * 1024);
    let mut reference = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut old_frame = reference.prepare((128, 96), &input).unwrap();
    assert!(
        old_frame
            .geometry
            .as_ref()
            .is_some_and(|g| !g.vertices.is_empty())
    );
    assert!(!old_frame.lighting.lights.is_empty());
    // Frozen previous policy: prepare the entire lit scene, then override the
    // shader's unlit flag. Geometry, ordering and particles stay identical.
    old_frame.lighting.uniforms.shadow[3] = -1.;
    assert_eq!(actual, capture(&gpu, &mut reference, &old_frame, 1));
    input.particles = &[];
    let without = preview.prepare_unlit((128, 96), &input).unwrap();
    assert_ne!(actual, capture(&gpu, &mut preview, &without, 2));
    let lit = preview.prepare((128, 96), &input).unwrap();
    assert_ne!(actual, capture(&gpu, &mut preview, &lit, 3));
    assert!(gpu.target_pool_usage().live_bytes >= 2048 * 2048 * 4);
    // Reusing a formerly lit view must preserve unlit output too.
    input.particles = &particles;
    let again = preview.prepare_unlit((128, 96), &input).unwrap();
    assert_eq!(actual, capture(&gpu, &mut preview, &again, 4));
    drop((frame, old_frame, without, lit, again, preview, reference));
    gpu.set_target_cache_limits(0, 0);
    gpu.set_working_buffer_cache_limits(0, 0);
    assert_eq!(gpu.target_pool_usage().live_bytes, 0);
    assert_eq!(gpu.working_buffer_pool_usage().live_bytes, 0);
}
