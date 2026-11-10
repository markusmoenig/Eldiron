//! Add editable terrain examples to the maintained project and capture/measure them.
//! cargo run -p eldiron-shared --release --example shape_terrain -- <project> <ignored-dir> [--review]
use eldiron_shared::{project::Project, project_io};
use rusterix::chunkbuilder::{
    geometry_object_builder::GeometryObjectBuilder, terrain_surface::TerrainSurface,
};
use rusterix::{ChunkBuilder, Map, Value};
use std::{path::Path, time::Instant};
use theframework::prelude::*;

fn area(map: &mut Map, name: &str, points: &[[f32; 2]], height: f32, hill: bool) {
    map.clear_temp();
    let vertices: Vec<_> = points
        .iter()
        .map(|p| map.add_vertex_at(p[0], p[1]))
        .collect();
    for i in 0..vertices.len() {
        map.create_linedef_manual(vertices[i], vertices[(i + 1) % vertices.len()]);
    }
    let id = map.close_polygon_manual().unwrap();
    let s = map.find_sector_mut(id).unwrap();
    s.name = name.into();
    s.properties.set("terrain_example", Value::Bool(true));
    s.properties.set("terrain_shape", Value::Bool(true));
    s.properties
        .set("terrain_shape_mode", Value::Int(i32::from(hill)));
    s.properties
        .set("terrain_shape_height", Value::Float(height));
    s.properties.set("terrain_shape_blend", Value::Float(2.5));
}
fn examples(project: &mut Project) -> Result<(), Box<dyn std::error::Error>> {
    let map = &mut project.regions[0].map;
    let existing = map
        .sectors
        .iter()
        .any(|s| s.properties.get_bool_default("terrain_example", false));
    if existing
        && map
            .properties
            .get_bool_default("terrain_examples_ready", false)
    {
        return Ok(());
    }
    if !existing {
        area(
            map,
            "Terrain Plateau",
            &[[19., 11.], [24., 11.], [25., 14.], [23., 16.], [19., 16.]],
            1.2,
            false,
        );
        area(
            map,
            "Terrain Hill",
            &[
                [18., 21.],
                [21., 18.],
                [25., 19.],
                [27., 23.],
                [24., 27.],
                [20., 26.],
            ],
            1.2,
            true,
        );
        let route = [
            ([12.5, 12.5], 0.0),
            ([14.5, 16.5], 0.1),
            ([18.5, 15.5], 0.45),
            ([21.5, 14.5], 1.2),
            ([24.5, 18.5], 0.1),
            ([24.5, 21.5], 0.8),
        ];
        for (i, pair) in route.windows(2).enumerate() {
            let a = Vec2::from(pair[0].0);
            let b = Vec2::from(pair[1].0);
            let av = map.add_vertex_at(a.x, a.y);
            let bv = map.add_vertex_at(b.x, b.y);
            let id = map.create_linedef_manual(av, bv);
            map.clear_temp();
            let line = map.find_linedef_mut(id).unwrap();
            line.name = format!("Terrain approach {}", i + 1);
            line.properties.set("terrain_example", Value::Bool(true));
            line.properties.set("terrain_shape", Value::Bool(true));
            line.properties
                .set("terrain_start_height", Value::Float(pair[0].1));
            line.properties
                .set("terrain_end_height", Value::Float(pair[1].1));
            line.properties
                .set("terrain_shape_width", Value::Float(1.8));
            line.properties
                .set("terrain_shape_blend", Value::Float(2.5));
        }
    }
    // Surface painting is authored as an editable continuous MZ composition by
    // woodland_demo::terrain_art::update_unending_night_terrain_art. Corner
    // blend presets cannot represent a continuous winding path across cells.
    map.properties
        .set("terrain_examples_ready", Value::Bool(true));
    map.changed = map.changed.wrapping_add(1);
    map.selected_sectors.clear();
    map.selected_linedefs.clear();
    map.selected_vertices.clear();
    project.time = TheTime::new_time(11, 0)?;
    let center = Vec2::new(15., 14.);
    map.grid_size = 25.;
    let screen = rusterix::isometric::Projection {
        origin: Vec2::zero(),
        unit: map.grid_size,
    }
    .screen(center, 0.);
    map.offset = Vec2::new(-screen.x, screen.y);
    project.regions[0].editing_look_at_3d = Vec3::new(20., 0.5, 18.);
    project.regions[0].editing_position_3d = Vec3::new(10., 12., 28.);
    project.regions[0].editing_position_iso_3d = Some(project.regions[0].editing_position_3d);
    project.regions[0].editing_look_at_iso_3d = Some(project.regions[0].editing_look_at_3d);
    project.regions[0].editing_iso_scale = Some(9.);
    Ok(())
}
fn verify(project: &Project, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let map = &project.regions[0].map;
    let field = TerrainSurface::from_map(map).unwrap();
    let mut peak = 0_f32;
    for line in map
        .linedefs
        .iter()
        .filter(|l| l.properties.get_bool_default("terrain_example", false))
    {
        let a = map.get_vertex(line.start_vertex).unwrap();
        let b = map.get_vertex(line.end_vertex).unwrap();
        let mut previous = field.height_at(a).unwrap();
        for step in 0..=100 {
            let p = a + (b - a) * (step as f32 / 100.);
            let h = field.height_at(p).unwrap();
            assert!(
                (h - previous).abs() < 0.1,
                "discontinuous approach at {p:?}"
            );
            previous = h;
            peak = peak.max(h);
        }
    }
    assert!(peak > 1.1);
    let blocking = project
        .tiles
        .iter()
        .filter(|(_, tile)| tile.blocking)
        .map(|(id, _)| *id)
        .collect();
    let mini = map.as_mini(&blocking);
    let elevation = &mini.elevation;
    for line in map
        .linedefs
        .iter()
        .filter(|l| l.properties.get_bool_default("terrain_example", false))
    {
        let a = map.get_vertex(line.start_vertex).unwrap();
        let b = map.get_vertex(line.end_vertex).unwrap();
        let mut foot = a;
        for _ in 0..100 {
            let (next, blocked) = mini.move_distance(foot, (b - a) / 100., 0.22);
            assert!(
                !blocked,
                "static collision blocks the example route at {foot:?}"
            );
            foot = next;
        }
        for _ in 0..100 {
            let (next, blocked) = mini.move_distance(foot, (a - b) / 100., 0.22);
            assert!(
                !blocked,
                "static collision blocks the return route at {foot:?}"
            );
            foot = next;
        }
        let mut p = Vec3::new(a.x, field.height_at(a).unwrap(), a.y);
        for _ in 0..100 {
            let (next, blocked) = elevation.move_distance(p, (b - a) / 100., 0.22, 0.35, 0.5);
            assert!(!blocked, "approach walking blocked at {p:?}");
            p = next;
        }
        assert!((Vec2::new(p.x, p.z) - b).magnitude() < 0.001);
        for _ in 0..100 {
            let (next, blocked) = elevation.move_distance(p, (a - b) / 100., 0.22, 0.35, 0.5);
            assert!(!blocked);
            p = next;
        }
    }
    let mut assets = rusterix::Assets::default();
    assets.set_tiles(project.tiles.clone());
    let mut build_ms = Vec::new();
    let mut cells = 0;
    let mut polygons = 0;
    let mut triangles = 0;
    for _ in 0..5 {
        let mut generation_seconds = 0.;
        cells = 0;
        polygons = 0;
        triangles = 0;
        for z in [-16, -8, 0, 8, 16, 24, 32] {
            for x in [-16, -8, 0, 8, 16, 24, 32] {
                let mut chunk = rusterix::Chunk::new(Vec2::new(x, z), 8);
                let mut vm = scenevm::Chunk::new(Vec2::new(x, z), 8);
                let started = Instant::now();
                GeometryObjectBuilder::new().build(map, &assets, &mut chunk, &mut vm);
                generation_seconds += started.elapsed().as_secs_f64();
                for (id, polys) in &vm.polys3d_map {
                    if matches!(id, scenevm::GeoId::Terrain(..)) {
                        cells += 1;
                        polygons += polys.len();
                        triangles += polys.iter().map(|p| p.indices.len()).sum::<usize>();
                        for poly in polys {
                            for v in &poly.vertices {
                                if let Some(h) = field.height_at(Vec2::new(v[0], v[2])) {
                                    assert!((v[1] - h).abs() < 0.0001);
                                }
                            }
                        }
                    }
                }
            }
        }
        build_ms.push(generation_seconds * 1000.);
    }
    build_ms.sort_by(f64::total_cmp);
    let report = format!(
        "Shared terrain verified in actual 3D GeometryObjectBuilder chunks.\n{cells} cells, {polygons} polygons, {triangles} triangles.\nWhole-map chunk generation (height verification excluded): median {:.2} ms (5 runs).\nApproach traversed uphill and downhill on every segment; static prop collision checked.\n",
        build_ms[2]
    );
    eprintln!("{report}");
    std::fs::write(out.join("terrain-3d-profile.txt"), report)?;
    Ok(())
}
#[cfg(feature = "graphics")]
fn review(project: &Project, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use rusterix::{
        D3Camera, D3IsoCamera, PlayerCamera, Rusterix, client::widget::game::GameWidget,
    };
    let mut runtime = Rusterix::default();
    let mut map = project.regions[0].map.clone();
    runtime.assets.palette = project.palette.clone();
    runtime.set_tiles_for_maps(project.tiles.clone(), false, [&map]);
    runtime.scene_handler.settings.read(&project.config)?;
    runtime.scene_handler.settings.apply_hour(12.);
    let mut iso = TheRGBABuffer::new(TheDim::sized(1280, 900));
    let settings = &runtime.scene_handler.settings;
    let lights = rusterix::isometric::lighting::Lighting::new(settings, std::iter::empty(), 0);
    rusterix::isometric::draw_scene_gpu_cached(
        &map,
        &runtime.assets,
        &mut iso,
        rusterix::isometric::Projection::editor(&map, Vec2::new(1280., 900.)),
        0.,
        None,
        &Default::default(),
        None,
        &lights,
        &mut runtime.scene_handler.vm,
        &[],
    )?;
    std::fs::write(out.join("shape-terrain-iso.png"), iso.to_png()?)?;
    map.camera = rusterix::MapCamera::ThreeDIso;
    let mut widget = GameWidget::new();
    widget.rect.width = 1280.;
    widget.rect.height = 900.;
    widget.buffer = TheRGBABuffer::new(TheDim::sized(1280, 900));
    widget.set_camera_mode(PlayerCamera::D3Iso);
    let mut camera = D3IsoCamera::new();
    camera.scale = 10.;
    camera.set_parameter_vec3("center", Vec3::new(20., 0.5, 18.));
    widget.camera_d3 = Box::new(camera);
    let mut time = project.time.clone();
    time.hours = 12;
    for frame in 0..80 {
        widget.prepare_frame(
            &map,
            &time,
            frame / 12,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let mut pixels = vec![0; 1280 * 900 * 4];
    for _ in 0..5 {
        runtime
            .scene_handler
            .vm
            .render_frame(&mut pixels, 1280, 900);
    }
    let mut times = Vec::new();
    for _ in 0..30 {
        let now = Instant::now();
        runtime
            .scene_handler
            .vm
            .render_frame(&mut pixels, 1280, 900);
        times.push(now.elapsed().as_secs_f64() * 1000.);
    }
    times.sort_by(f64::total_cmp);
    std::fs::write(
        out.join("shape-terrain-3d.png"),
        TheRGBABuffer::from(pixels, 1280, 900).to_png()?,
    )?;
    let frame_report = format!(
        "Native 3D terrain VM render/readback at 1280x900: median {:.2} ms, p95 {:.2} ms (30 frames after 5 warmups; editor UI and game ticks excluded).\n",
        times[15], times[28]
    );
    eprintln!("{frame_report}");
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(out.join("terrain-3d-profile.txt"))?
        .write_all(frame_report.as_bytes())?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    std::fs::create_dir_all(out)?;
    let backup = out.join("UnendingNight.before-shape-terrain.eldiron");
    if !backup.exists() {
        std::fs::copy(path, backup)?;
    }
    let mut project = project_io::decode_project(&std::fs::read(path)?)?;
    examples(&mut project)?;
    let bytes = project_io::encode_project(&project)?;
    let reopened = project_io::decode_project(&bytes)?;
    verify(&reopened, out)?;
    std::fs::write(path, bytes)?;
    #[cfg(feature = "graphics")]
    if args.iter().any(|a| a == "--review") {
        review(&reopened, out)?;
    }
    Ok(())
}
