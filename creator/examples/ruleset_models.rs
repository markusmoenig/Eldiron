//! Review every MZ item model against its inventory icon and equipped accessories.
//! cargo run -p eldiron-creator --example ruleset_models -- <output-dir> [palette-size]
use rusterix::{Assets, Entity, Item, Value, procedural_avatar::Runtime};
use shared::{project::Project, ruleset_materials, rulesets, rusterix_utils};
use theframework::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or("/tmp/eldiron-ruleset-models".into()));
    std::fs::create_dir_all(&out)?;
    let mut project = Project::default();
    let options = args.collect::<Vec<_>>();
    if let Some(size) = options.first().filter(|s| s.as_str() != "--focus") {
        project.palette.colors.truncate(size.parse()?);
    }
    project.sync_ruleset_items()?;
    let mut assets = Assets::default();
    rusterix_utils::insert_procedural_ruleset_icons(&mut assets, &project);
    rusterix_utils::sync_procedural_avatar_assets(&mut assets, &project);
    let templates =
        rulesets::ruleset_item_templates_from_source(rulesets::latest_official_ruleset())?;
    let mut entries = Vec::new();
    for t in &templates {
        let mut item = Item::default();
        item.item_type = t.id.clone();
        rusterix::server::data::apply_item_data(&mut item, &t.data);
        item.attributes.set("active", Value::Bool(true));
        entries.push((t.name.clone(), t.id.clone(), item));
    }
    let mut off = entries
        .iter()
        .find(|(_, id, _)| id == "torch")
        .unwrap()
        .2
        .clone();
    off.attributes.set("active", Value::Bool(false));
    entries.push(("Torch (off)".into(), "torch_off".into(), off));
    let mut index = vec![];
    for (label, id, item) in entries {
        if options.iter().any(|s| s == "--focus")
            && !matches!(id.as_str(), "small_bag" | "sunward_charm" | "ritual_censer")
        {
            continue;
        }
        let appearance = Runtime::resolve_item(&item, &assets).ok_or(format!("No model: {id}"))?;
        let doc = denrim_materials::geometry::Document::load(&appearance.source)?;
        let mesh = doc.evaluate()?.mesh;
        let icon = ruleset_materials::baked(
            &project,
            &format!(
                "icon/{}/{}",
                if id == "torch_off" { "torch" } else { &id },
                if id == "torch_off" { "off" } else { "on" }
            ),
        )?;
        std::fs::write(out.join(format!("{id}-icon.png")), icon.to_rgba().to_png()?)?;
        let mut sheet = vec![0; 192 * 64 * 4];
        for (i, angle) in [0_f32, 45., 90.].into_iter().enumerate() {
            let mut entity = Entity::default();
            entity.orientation = Vec2::new(angle.to_radians().sin(), angle.to_radians().cos());
            let pixels = appearance.sprite(&entity, &assets, "idle", 0.)?;
            assert!(
                pixels.chunks_exact(4).any(|p| p[3] != 0),
                "{id}: blank model"
            );
            for y in 0..64 {
                let to = (y * 192 + i * 64) * 4;
                sheet[to..to + 256].copy_from_slice(&pixels[y * 256..y * 256 + 256]);
            }
        }
        std::fs::write(
            out.join(format!("{id}.png")),
            TheRGBABuffer::from(sheet, 192, 64).to_png()?,
        )?;
        index.push(serde_json::json!({"id":id,"label":label,"bounds":mesh.bounds(),"vertices":mesh.vertex_count()}));
    }
    let make_item = |name: &str| {
        let mut item = Item::default();
        rusterix::server::data::apply_item_data(
            &mut item,
            &templates.iter().find(|t| t.id == name).unwrap().data,
        );
        item
    };
    let mut sheet = vec![0; 64 * 9 * 64 * 4 * 4];
    for (race_index, race) in ["humanoid", "orc", "skeleton"].into_iter().enumerate() {
        for (outfit_index, focus) in ["ritual_censer", "sunward_charm", "none"]
            .into_iter()
            .enumerate()
        {
            let mut entity = Entity::default();
            for (slot, name) in [
                ("container", "small_bag"),
                ("torso", "linen_shirt"),
                ("legs", "wool_trousers"),
                ("feet", "leather_shoes"),
                ("main_hand", "training_sword"),
                ("off_hand", "round_shield"),
            ] {
                entity.equipped.insert(slot.into(), make_item(name));
            }
            if focus != "none" {
                entity.equipped.insert("focus".into(), make_item(focus));
            }
            let model = &assets.procedural_avatars[race];
            for (row, (angle, clip, time)) in [
                (0_f32, "idle", 0.),
                (90., "idle", 0.),
                (180., "idle", 0.),
                (0., "walk", 0.3),
            ]
            .into_iter()
            .enumerate()
            {
                entity.orientation = Vec2::new(angle.to_radians().sin(), angle.to_radians().cos());
                let pixels = model.sprite(&entity, &assets, clip, time)?;
                for y in 0..64 {
                    let to = ((row * 64 + y) * 576 + (race_index * 3 + outfit_index) * 64) * 4;
                    sheet[to..to + 256].copy_from_slice(&pixels[y * 256..y * 256 + 256]);
                }
            }
        }
    }
    std::fs::write(
        out.join("accessories.png"),
        TheRGBABuffer::from(sheet, 576, 256).to_png()?,
    )?;
    std::fs::write(
        out.join("index.json"),
        serde_json::to_string_pretty(&index)?,
    )?;
    eprintln!("Reviewed {} item states in {}", index.len(), out.display());
    Ok(())
}
