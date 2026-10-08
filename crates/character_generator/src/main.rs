use character_generator::{
    BakeOptions, Catalog, GenerationOptions, Result, Shading, StylePreset, bake_atlas, export_glb,
    export_viewer,
};
use std::path::PathBuf;
fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] {
        println!(
            "character-generator <catalog.toml> <character-id> <output-dir> [--size 96] [--frames 8] [--ruleset] [--avatar] [--tick-rate 4] [--style blocky|stylized|natural] [--segments 12] [--shading flat|smooth] [--head-scale 1.0]\nExports character.glb, atlas.png, channels.png, atlas.json, preview.html and viewer3d.html.\n--ruleset resolves an official-ruleset overlay; --avatar adds avatar/character.eldiron_avatar. Both require --features eldiron."
        );
        return Ok(());
    }
    if args.len() < 3 {
        return Err("expected catalog, character ID, and output directory; use --help".into());
    }
    let mut options = BakeOptions::default();
    let mut generation = GenerationOptions::default();
    let mut ruleset = false;
    let mut avatar = false;
    let mut tick_rate = 4.0f32;
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--ruleset" => ruleset = true,
            "--avatar" => avatar = true,
            "--size" | "--frames" | "--tick-rate" | "--style" | "--segments" | "--shading"
            | "--head-scale" => {
                let value = args.get(i + 1).ok_or("option missing value")?;
                match args[i].as_str() {
                    "--size" => options.size = value.parse()?,
                    "--frames" => options.frames = value.parse()?,
                    "--tick-rate" => tick_rate = value.parse()?,
                    "--segments" => generation.segments = Some(value.parse()?),
                    "--head-scale" => generation.head_scale = value.parse()?,
                    "--style" => {
                        generation.style = match value.as_str() {
                            "blocky" => StylePreset::Blocky,
                            "stylized" => StylePreset::Stylized,
                            "natural" => StylePreset::Natural,
                            _ => return Err("style must be blocky, stylized or natural".into()),
                        }
                    }
                    _ => {
                        generation.shading = Some(match value.as_str() {
                            "flat" => Shading::Flat,
                            "smooth" => Shading::Smooth,
                            _ => return Err("shading must be flat or smooth".into()),
                        })
                    }
                };
                i += 1;
            }
            other => return Err(format!("unknown option {other}").into()),
        }
        i += 1;
    }
    #[cfg(not(feature = "eldiron"))]
    if ruleset || avatar {
        return Err("--ruleset and --avatar require cargo --features eldiron".into());
    }
    let source = std::fs::read_to_string(&args[0])?;
    let catalog = if ruleset {
        #[cfg(feature = "eldiron")]
        {
            let resolved = eldiron_ruleset::resolve_project_ruleset(
                eldiron_ruleset::DEFAULT_RULESET_CONFIG,
                &source,
            )?;
            if !resolved.validation().is_ok() {
                return Err(format!(
                    "invalid resolved ruleset: {:?}",
                    resolved.validation().issues
                )
                .into());
            }
            character_generator::ruleset::catalog_from_resolved(&resolved)?
        }
        #[cfg(not(feature = "eldiron"))]
        {
            unreachable!()
        }
    } else {
        Catalog::parse(&source)?
    };
    let asset = catalog.generate_with_options(&args[1], &generation)?;
    let output = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&output)?;
    let atlas = bake_atlas(&asset, options, &output)?;
    export_glb(&asset, output.join("character.glb"))?;
    export_viewer(&asset, output.join("viewer3d.html"))?;
    #[cfg(feature = "eldiron")]
    if avatar {
        character_generator::eldiron::export_avatar(
            &asset,
            options,
            output.join("avatar"),
            tick_rate,
        )?;
        let preview_path = output.join("preview.html");
        let preview = std::fs::read_to_string(&preview_path)?.replace(
            "<small>",
            "<p><a href=\"avatar/character.eldiron_avatar\" download>Download Eldiron Avatar</a> · <a href=\"avatar/preview.html\">Body-only preview</a></p><small>",
        );
        std::fs::write(preview_path, preview)?;
    }
    #[cfg(not(feature = "eldiron"))]
    let _ = tick_rate;
    println!(
        "Generated {}: {} joints, {} triangles, {} atlas frames → {}",
        asset.name,
        asset.joints.len(),
        asset
            .primitives
            .iter()
            .map(|p| p.indices.len() / 3)
            .sum::<usize>(),
        atlas.frames.len(),
        output.display()
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("character-generator: {error}");
        std::process::exit(1);
    }
}
