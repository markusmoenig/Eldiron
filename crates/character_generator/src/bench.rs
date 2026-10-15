//! Generate a ruleset-driven, live 3D inspection bench without sprite baking.
use character_generator::*;
use serde_json::json;
use std::path::PathBuf;

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "character-bench [ruleset-overlay.toml] [output-directory]\nEdit the overlay and rerun to regenerate live mesh previews and rigged GLBs."
        );
        return Ok(());
    }
    if args.len() > 2 {
        return Err("expected at most a ruleset overlay and output directory".into());
    }
    let input = args
        .first()
        .map(String::as_str)
        .unwrap_or("crates/character_generator/examples/bench-ruleset.toml");
    let output = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("target/character-generator/bench"),
    );
    let source = std::fs::read_to_string(input)?;
    let resolved =
        eldiron_ruleset::resolve_project_ruleset(eldiron_ruleset::DEFAULT_RULESET_CONFIG, &source)?;
    if !resolved.validation().is_ok() {
        return Err(format!("invalid ruleset: {:?}", resolved.validation().issues).into());
    }
    let mut catalog = ruleset::catalog_from_resolved(&resolved)?;
    std::fs::create_dir_all(&output)?;
    let ids: Vec<_> = catalog.characters.keys().cloned().collect();
    let mut cases = Vec::new();
    for (number, id) in ids.iter().enumerate() {
        let definition = catalog.characters.get_mut(id).unwrap();
        let outfit = std::mem::take(&mut definition.outfit);
        let equipment = std::mem::take(&mut definition.equipment);
        for style in [
            StylePreset::Blocky,
            StylePreset::Stylized,
            StylePreset::Natural,
        ] {
            for loadout in ["anatomy", "clothing", "equipped", "helmet"] {
                let definition = catalog.characters.get_mut(id).unwrap();
                definition.outfit = if loadout == "anatomy" {
                    vec![]
                } else {
                    outfit
                        .iter()
                        .filter(|item| {
                            loadout == "helmet"
                                || catalog.items[*item].generator != ItemKind::Helmet
                        })
                        .cloned()
                        .collect()
                };
                definition.equipment = if matches!(loadout, "equipped" | "helmet") {
                    equipment.clone()
                } else {
                    vec![]
                };
                let asset = catalog.generate_with_options(
                    id,
                    &GenerationOptions {
                        style,
                        ..Default::default()
                    },
                )?;
                let style_name = match style {
                    StylePreset::Blocky => "blocky",
                    StylePreset::Stylized => "stylized",
                    StylePreset::Natural => "natural",
                };
                // Use numeric case directories so arbitrary ruleset IDs never become paths.
                let dir = format!("case-{number}-{style_name}-{loadout}");
                std::fs::create_dir_all(output.join(&dir))?;
                export_viewer(&asset, output.join(&dir).join("viewer3d.html"))?;
                export_glb(&asset, output.join(&dir).join("character.glb"))?;
                cases.push(json!({"id":id,"species":format!("{:?}",catalog.characters[id].species),"style":style_name,"loadout":loadout,"path":dir,"joints":asset.joints.len(),"triangles":asset.primitives.iter().map(|p|p.indices.len()/3).sum::<usize>()}));
            }
        }
        let definition = catalog.characters.get_mut(id).unwrap();
        definition.outfit = outfit;
        definition.equipment = equipment;
    }
    let items: Vec<_> = catalog.items.iter().map(|(id,item)| json!({"id":id,"generator":format!("{:?}",item.generator),"attachment":format!("{:?}",item.attachment),"color":item.color})).collect();
    let payload = serde_json::to_string(&json!({"cases":cases,"input":input,"source":source,"items":items, "motions": ([Motion::Idle, Motion::Walk, Motion::Cast, Motion::Death, Motion::Sit, Motion::Use, Motion::Attack, Motion::Parry].map(|motion| (motion.name(),motion.duration())).into_iter().collect::<std::collections::BTreeMap<_,_>>())}))?
        .replace('<', "\\u003c");
    std::fs::write(
        output.join("index.html"),
        include_str!("bench.html").replace("__BENCH__", &payload),
    )?;
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_string_pretty(&cases)?,
    )?;
    println!(
        "Created {} live 3D cases in {}",
        cases.len(),
        output.join("index.html").display()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("character-bench: {e}");
        std::process::exit(1);
    }
}
