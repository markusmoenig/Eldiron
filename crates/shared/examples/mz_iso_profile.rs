//! End-to-end iso timings, including GPU uploads and readback with --gpu.
use eldiron_shared::{isometric, project_io, rusterix_utils};
use rusterix::Assets;
use std::time::Instant;
use theframework::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or("generated/mz-iso2d-native/MZIso2D.eldiron".into());
    let args = std::env::args().collect::<Vec<_>>();
    let option = |name: &str| {
        args.iter()
            .position(|s| s == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let hour = option("--hour")
        .map(|s| s.parse::<f32>().unwrap())
        .unwrap_or(12.);
    let size = option("--size").unwrap_or("1000x700".into());
    let (width, height) = size
        .split_once('x')
        .map(|(w, h)| (w.parse::<i32>().unwrap(), h.parse::<i32>().unwrap()))
        .unwrap();
    let walking = args.iter().any(|s| s == "--walking");
    let view_scale = option("--view-scale")
        .map(|s| s.parse::<f32>().unwrap())
        .unwrap_or(1.);
    let frames = option("--frames")
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(90)
        .max(1);
    let warmup = option("--warmup")
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(5);
    let extra_count = option("--extra-lights")
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(0);
    let use_gpu = std::env::args().any(|s| s == "--gpu");
    #[cfg(feature = "graphics")]
    let mut scene = rusterix::SceneHandler::default();
    let lit = std::env::args().any(|s| s == "--lit");
    let bytes = std::fs::read(path)?;
    let start = Instant::now();
    let mut project = project_io::decode_project(&bytes)?;
    eprintln!("decode: {:.2?}", start.elapsed());
    let start = Instant::now();
    let defaults = project_io::decode_project(include_bytes!(
        "../../../creator/embedded/materials/default-bakes.eldiron"
    ))?;
    eldiron_shared::ruleset_materials::seed_bakes(&mut project, &defaults)?;
    eprintln!("seed persisted material bakes: {:.2?}", start.elapsed());
    let start = Instant::now();
    assert!(isometric::synchronize(&mut project).is_empty());
    eprintln!("iso cache validation: {:.2?}", start.elapsed());
    let mut assets = Assets::default();
    for pass in 0..2 {
        let start = Instant::now();
        rusterix_utils::sync_procedural_avatar_assets(&mut assets, &project);
        eprintln!("avatar assets {pass}: {:.2?}", start.elapsed());
    }
    // Match runtime's tile lookup/index construction.
    assets.set_tiles(project.tiles.clone());
    let mut owned_map = project.regions[0].map.clone();
    let terrain = rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(&owned_map);
    if walking {
        if let Some(id) = project
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "MZ Iso / character")
            .map(|(id, _)| *id)
        {
            let mut actor = rusterix::Entity::new();
            actor.set_attribute(
                "source",
                rusterix::Value::Source(rusterix::PixelSource::TileId(id)),
            );
            actor.set_attribute("player", rusterix::Value::Bool(true));
            owned_map.entities.push(actor);
        }
    }
    let map = &mut owned_map;
    let mut settings = rusterix::RenderSettings::default();
    settings.read(&project.config)?;
    settings.apply_hour(hour);
    if let Some(softness) = option("--shadow-softness") {
        settings.point_shadow_softness = softness.parse::<f32>()?.clamp(0., 1.);
    }
    if args.iter().any(|s| s == "--no-shadows") {
        settings.raster_shadow_enabled = false;
    }
    let lights = map
        .lights
        .iter()
        .filter(|l| l.active)
        .map(|l| {
            let l = l.compile();
            scenevm::Light::new_pointlight(l.position)
                .with_color(vek::Vec3::from(l.color))
                .with_intensity(l.intensity)
                .with_start_distance(l.start_distance)
                .with_end_distance(l.end_distance)
        })
        .collect::<Vec<_>>();
    let lighting = rusterix::isometric::lighting::Lighting::new(&settings, lights.iter(), 0);
    let mut buffer = TheRGBABuffer::new(TheDim::sized(width, height));
    let mut samples = Vec::new();
    let mut ground_cache = rusterix::isometric::GroundCache::default();
    let mut light_count = 0;
    let mut particle_count = 0;
    for frame in 0..warmup + frames {
        // Repeat the same camera/actor path for comparable long runs.
        let path_frame = frame % 95;
        let start = Instant::now();
        if walking && let Some(actor) = map.entities.last_mut() {
            let x = 9. + path_frame as f32 * 0.012;
            let z = 12.3;
            actor.position = Vec3::new(
                x,
                terrain
                    .as_ref()
                    .and_then(|t| t.height_at(Vec2::new(x, z)))
                    .unwrap_or(0.),
                z,
            );
            actor.orientation = Vec2::new(1., 0.);
            actor.set_attribute("moving", rusterix::Value::Bool(true));
        }
        let projection = rusterix::isometric::Projection {
            origin: Vec2::new(width as f32, height as f32) * 0.5
                + (rusterix::isometric::Projection::editor(
                    map,
                    Vec2::new(width as f32, height as f32),
                )
                .origin
                    - Vec2::new(width as f32, height as f32) * 0.5
                    + Vec2::new(path_frame as f32, 0.))
                    * view_scale,
            unit: map.grid_size * view_scale,
        };
        if use_gpu {
            #[cfg(feature = "graphics")]
            {
                scene.settings = settings.clone();
                scene.build_dynamics_2d(map, frame, &assets, &Default::default());
                let particles = scene.isometric_particles(map, &assets, frame);
                let mut live_lights = scene.vm.vm.lights.values().cloned().collect::<Vec<_>>();
                // Benchmark-only overlapping courtyard lights; never modify the project.
                for i in 0..extra_count {
                    let angle = std::f32::consts::TAU * i as f32 / extra_count as f32;
                    live_lights.push(
                        scenevm::Light::new_pointlight(Vec3::new(
                            9.5 + angle.cos() * 2.5,
                            1.8,
                            10. + angle.sin() * 2.5,
                        ))
                        .with_color(Vec3::new(1., 0.72, 0.38))
                        .with_intensity(1.35)
                        .with_start_distance(0.5)
                        .with_end_distance(4.5),
                    );
                }
                light_count = live_lights
                    .iter()
                    .filter(|l| l.emitting && l.intensity > 0. && l.end_distance > 0.)
                    .count();
                particle_count = particles.len();
                let lighting = rusterix::isometric::lighting::Lighting::new(
                    &settings,
                    live_lights.iter(),
                    frame,
                );
                rusterix::isometric::draw_scene_gpu_cached(
                    map,
                    &assets,
                    &mut buffer,
                    projection,
                    frame as f32 / 30.,
                    None,
                    &Default::default(),
                    Some(&mut ground_cache),
                    &lighting,
                    &mut scene.vm,
                    &particles,
                )?;
            }
            #[cfg(not(feature = "graphics"))]
            panic!("Build with --features graphics for GPU profiling");
        } else {
            rusterix::isometric::draw_scene_lit_cached(
                map,
                &assets,
                &mut buffer,
                projection,
                frame as f32 / 30.,
                None,
                &Default::default(),
                Some(&mut ground_cache),
                lit.then_some(&lighting),
            );
        }
        if frame >= warmup {
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
    }
    samples.sort_by(f64::total_cmp);
    eprintln!(
        "{width}x{height}, hour {hour}, walking {walking}, GPU {use_gpu}, softness {}, lights {light_count}, particles {particle_count}: mean {:.2} ms, median {:.2} ms, p95 {:.2} ms ({frames} frames after {warmup} warmup; includes uploads, shadows, lighting, particles and readback)",
        settings.point_shadow_softness,
        samples.iter().sum::<f64>() / samples.len() as f64,
        samples[samples.len() / 2],
        samples[samples.len() * 95 / 100]
    );
    if let Some(path) = option("--json") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "width": width, "height": height, "hour": hour, "walking": walking,
                "view_scale": view_scale,
                "gpu": use_gpu, "softness": settings.point_shadow_softness,
                "shadows": settings.raster_shadow_enabled, "lights": light_count,
                "particles": particle_count, "frames": frames, "warmup": warmup,
                "mean_ms": samples.iter().sum::<f64>() / samples.len() as f64,
                "median_ms": samples[samples.len() / 2],
                "p95_ms": samples[samples.len() * 95 / 100], "samples_ms": samples,
            }))?,
        )?;
    }
    if let Some(path) = option("--save") {
        std::fs::write(path, buffer.to_png().unwrap())?;
    }
    Ok(())
}
