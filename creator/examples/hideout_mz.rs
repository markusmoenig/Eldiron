//! Reproducible Hideout2D material experiment. Never modifies the reference project.
use rusterix::{PlayerCamera, Rusterix, client::widget::game::GameWidget};
use shared::{
    project_io::decode_project,
    rusterix_utils::{insert_content_into_maps, sync_editor_visual_assets},
};
use theframework::prelude::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let path = args
        .first()
        .map(String::as_str)
        .unwrap_or("test_projects/Hideout2D.eldiron");
    let out = std::path::PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("/tmp/hideout-mz/original"),
    );
    std::fs::create_dir_all(&out)?;
    let mut project = decode_project(&std::fs::read(path)?)?;
    if args.iter().any(|arg| arg == "--black") {
        let mut config = project.config.parse::<toml_edit::DocumentMut>()?;
        config["materials"]["background_2d"] = toml_edit::value("black");
        project.config = config.to_string();
    }
    if args.iter().any(|arg| arg == "--compute") {
        let mut config = project.config.parse::<toml_edit::DocumentMut>()?;
        config["renderer"]["backend_2d"] = toml_edit::value("compute");
        project.config = config.to_string();
    }
    insert_content_into_maps(&mut project);
    let mut runtime = Rusterix::default();
    sync_editor_visual_assets(&mut runtime, &project);
    runtime.scene_handler.preview_character_setup = true;
    for region in &project.regions {
        if path.contains("Hideout2D-MZ") {
            for entity in &region.map.entities {
                let display =
                    rusterix::server::data::character_for_display(entity, &runtime.assets);
                let model =
                    rusterix::procedural_avatar::Runtime::resolve(&display, &runtime.assets);
                eprintln!(
                    "Ruleset character {:?}: avatar {:?}, resolved {}",
                    display.attributes.get_str("name"),
                    display.attributes.get_str("avatar"),
                    model.is_some()
                );
                assert!(
                    model.is_some(),
                    "Every recreated scene character uses its ruleset model"
                );
            }
        }

        let map = &region.map;
        runtime.set_tiles_for_maps(project.tiles.clone(), true, [map]);
        runtime
            .scene_handler
            .sync_base_render_settings(&project.config);
        let bounds = map.bounding_box().ok_or("Empty map")?;
        let w = ((bounds.z + 2.) * 32.).ceil() as u32;
        let h = ((bounds.w + 2.) * 32.).ceil() as u32;
        eprintln!(
            "{}: {}x{}, {} sectors, {} items, {} characters",
            region.name,
            w,
            h,
            map.sectors.len(),
            map.items.len(),
            map.entities.len()
        );
        let mut widget = GameWidget::new();
        widget.rect.width = w as f32;
        widget.rect.height = h as f32;
        widget.grid_size = 32.;
        widget.player_pos = Vec2::new(bounds.x + bounds.z * 0.5, bounds.y + bounds.w * 0.5);
        widget.buffer = TheRGBABuffer::new(TheDim::sized(w as i32, h as i32));
        widget.set_camera_mode(PlayerCamera::D2);
        for _ in 0..60 {
            widget.prepare_frame(
                map,
                &project.time,
                0,
                &runtime.assets,
                &mut runtime.scene_handler,
            );
            widget.apply_entities(map, &runtime.assets, 0, &mut runtime.scene_handler);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let mut pixels = vec![0; w as usize * h as usize * 4];
        runtime.scene_handler.vm.render_frame(&mut pixels, w, h);
        std::fs::write(
            out.join(format!("{}.png", region.name.to_lowercase())),
            TheRGBABuffer::from(pixels.clone(), w, h).to_png()?,
        )?;
        if args.iter().any(|a| a == "--audit-ground") {
            audit_ground(map, &pixels, w, h, widget.top_left)?;
        }
    }
    Ok(())
}

/// Probe actual rendered terrain, including transparent foregrounds over their
/// separate underlays. Space outside the authored sectors is intentionally void.
fn audit_ground(
    map: &rusterix::Map,
    pixels: &[u8],
    width: u32,
    height: u32,
    origin: Vec2<f32>,
) -> Result<(), String> {
    let mut sampled = 0;
    let mut uncovered = 0;
    let mut holes = vec![];
    for sector in &map.sectors {
        if !sector.intersects_vertical_slice(map, 0., 1.)
            || !matches!(
                sector.properties.get_default_source(),
                Some(rusterix::PixelSource::TileId(_))
            )
        {
            continue;
        }
        let bounds = sector.bounding_box(map);
        let project = |p: Vec2<f32>| (p - origin) * 32.;
        let lo = project(bounds.min);
        let hi = project(bounds.max);
        // Each art texel covers two screen pixels. Stay just inside polygon edges
        // so exterior rasterization coverage is not mistaken for missing ground.
        for y in ((lo.y.ceil() as i32 + 1).max(0)..(hi.y.floor() as i32 - 1).min(height as i32))
            .step_by(2)
        {
            for x in ((lo.x.ceil() as i32 + 1).max(0)..(hi.x.floor() as i32 - 1).min(width as i32))
                .step_by(2)
            {
                let world = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) / 32. + origin;
                if !sector.is_inside(map, world) {
                    continue;
                }
                sampled += 1;
                let at = (y as usize * width as usize + x as usize) * 4;
                if pixels[at..at + 3].iter().all(|v| *v == 0) {
                    uncovered += 1;
                    if holes.len() < 12 {
                        holes.push((sector.id, x, y));
                    }
                }
            }
        }
    }
    eprintln!("Ground audit: {sampled} interior samples; {uncovered} exposed-background samples");
    if sampled == 0 {
        return Err("Ground audit sampled no terrain".into());
    }
    if !holes.is_empty() {
        return Err(format!("Uncovered terrain in sectors/pixels: {holes:?}"));
    }
    Ok(())
}
