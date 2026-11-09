//! Render a CPU-only contact sheet of the native project's walk cycle at tile joins.
//! cargo run -p eldiron-shared --example mz_iso_contacts -- <project> <output.png>
use eldiron_shared::project_io;
use rusterix::{
    Assets, Entity, PixelSource, Value,
    isometric::{self, Projection},
};
use theframework::prelude::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let p = project_io::decode_project(&std::fs::read(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("generated/mz-iso2d-native/MZIso2D.eldiron"),
    )?)?;
    let mut map = p.regions[0].map.clone();
    map.entities.clear();
    map.items.clear();
    let tile = p
        .tiles
        .iter()
        .find(|(_, t)| t.alias == "MZ Iso / character")
        .ok_or("Character tile missing")?;
    let mut actor = Entity::new();
    actor.set_attribute("source", Value::Source(PixelSource::TileId(*tile.0)));
    actor.set_attribute("avatar_animation", Value::Str("walk".into()));
    map.entities.push(actor);
    let mini = map.as_mini(&Default::default());
    let mut assets = Assets::default();
    assets.set_tiles(p.tiles.clone());
    let (w, h) = (180, 240);
    let mut output = TheRGBABuffer::new(TheDim::sized(w * 8, h * 4));
    for row in 0..4 {
        for frame in 0..8 {
            // Tile interiors and the riser crossing, in both walking directions.
            let (x, z, dir) = match row {
                0 => (13.15 + frame as f32 * 0.1, 4.5, Vec2::unit_x()),
                1 => (13.7 + frame as f32 * 0.1, 4.5, Vec2::unit_x()),
                2 => (6.5, 13.15 + frame as f32 * 0.1, Vec2::unit_y()),
                _ => (6.5, 13.3 - frame as f32 * 0.1, -Vec2::unit_y()),
            };
            let at = Vec2::new(x, z);
            let height = mini.elevation.height_at(at);
            map.entities[0].position = Vec3::new(x, height, z);
            map.entities[0].orientation = dir;
            let mut projection = Projection {
                origin: Vec2::zero(),
                unit: 64.,
            };
            projection.origin =
                Vec2::new(w as f32 / 2., h as f32 - 40.) - projection.screen(at, height);
            let mut image = TheRGBABuffer::new(TheDim::sized(w, h));
            isometric::draw(&map, &assets, &mut image, projection, frame as f32 / 16.);
            for y in 0..h as usize {
                let src = y * w as usize * 4;
                let dst = ((row as usize * h as usize + y) * w as usize * 8
                    + frame as usize * w as usize)
                    * 4;
                output.pixels_mut()[dst..dst + w as usize * 4]
                    .copy_from_slice(&image.pixels()[src..src + w as usize * 4]);
            }
        }
    }
    let path = std::path::Path::new(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("generated/ui-review/iso-feet-contacts.png"),
    );
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, output.to_png()?)?;
    // Profile a moving camera with the player on the stairs, including the
    // contact compositor and the same reusable ground cache as the game widget.
    let walkable: Vec<_> = map
        .sectors
        .iter()
        .map(|s| s.properties.get_bool_default("iso_walkable", false))
        .collect();
    for contacts in [false, true] {
        // Disabling walkability here reproduces centre-only painter sorting for
        // the benchmark; the authoritative heights still come from `mini`.
        for (sector, was_walkable) in map.sectors.iter_mut().zip(&walkable) {
            sector
                .properties
                .set("iso_walkable", Value::Bool(contacts && *was_walkable));
        }
        let mut image = TheRGBABuffer::new(TheDim::sized(1000, 700));
        let mut cache = isometric::GroundCache::default();
        let start = std::time::Instant::now();
        for frame in 0..90 {
            let at = Vec2::new(13.15 + (frame % 30) as f32 * 0.025, 4.5);
            let height = mini.elevation.height_at(at);
            map.entities[0].position = Vec3::new(at.x, height, at.y);
            map.entities[0].orientation = Vec2::unit_x();
            let mut projection = Projection {
                origin: Vec2::zero(),
                unit: 40., // The test project's game widget uses 40, editor grid uses 24.
            };
            // Match GameWidget::prepare_isometric: integer camera positions
            // preserve the ground cache's sampling phase while walking.
            projection.origin =
                (Vec2::new(500., 450.) - projection.screen(at, height)).map(|v| v.round());
            isometric::draw_scene_cached(
                &map,
                &assets,
                &mut image,
                projection,
                frame as f32 / 30.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
        }
        eprintln!(
            "Moving stair render (contacts={contacts}, unit=40): {:.2?}/frame",
            start.elapsed() / 90
        );
    }
    Ok(())
}
