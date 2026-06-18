use image::{DynamicImage, ImageFormat, imageops::FilterType};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

const DEFAULT_MANIFEST: &str = "crates/ruleset/rulesets/eldiron/v1/icons.toml";
const DEFAULT_OUT_DIR: &str = "crates/ruleset/rulesets/eldiron/v1/assets/icons";
const GAME_ICONS_BASE: &str = "https://game-icons.net/icons/ffffff/transparent/1x1";

#[derive(Debug, Deserialize)]
struct IconManifest {
    icons: BTreeMap<String, IconEntry>,
}

#[derive(Debug, Deserialize)]
struct IconEntry {
    source: String,
    name: String,
    author: String,
    author_slug: Option<String>,
    license: Option<String>,
}

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let manifest_path = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MANIFEST));
    let out_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR));

    let manifest_src = fs::read_to_string(&manifest_path).map_err(|err| {
        format!(
            "Could not read icon manifest '{}': {err}",
            manifest_path.display()
        )
    })?;
    let manifest = toml::from_str::<IconManifest>(&manifest_src)
        .map_err(|err| format!("Could not parse '{}': {err}", manifest_path.display()))?;

    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("Could not create '{}': {err}", out_dir.display()))?;

    for (id, icon) in &manifest.icons {
        build_icon(id, icon, &out_dir)?;
    }
    write_attribution(&manifest, &out_dir)?;

    println!(
        "Built {} icon mask(s) into {}",
        manifest.icons.len(),
        out_dir.display()
    );
    Ok(())
}

fn build_icon(id: &str, icon: &IconEntry, out_dir: &Path) -> Result<(), String> {
    if icon.source.trim() != "game-icons" {
        return Err(format!(
            "Icon '{id}' uses unsupported source '{}'",
            icon.source
        ));
    }

    let author_slug = icon
        .author_slug
        .as_deref()
        .unwrap_or(icon.author.as_str())
        .trim()
        .to_ascii_lowercase();
    let url = format!("{GAME_ICONS_BASE}/{author_slug}/{}.png", icon.name.trim());
    let response = ureq::get(&url)
        .call()
        .map_err(|err| format!("Could not download '{id}' from {url}: {err}"))?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| format!("Could not read downloaded icon '{id}': {err}"))?;

    let image = image::load_from_memory(&bytes)
        .map_err(|err| format!("Could not decode downloaded icon '{id}': {err}"))?;
    let output = normalize_icon_mask(image);
    let out_path = out_dir.join(format!("{id}.png"));
    output
        .save_with_format(&out_path, ImageFormat::Png)
        .map_err(|err| format!("Could not write '{}': {err}", out_path.display()))?;
    Ok(())
}

fn normalize_icon_mask(image: DynamicImage) -> DynamicImage {
    let mut resized = image.resize_exact(32, 32, FilterType::Nearest).to_rgba8();
    for pixel in resized.pixels_mut() {
        if pixel[3] > 0 {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
        }
    }
    DynamicImage::ImageRgba8(resized)
}

fn write_attribution(manifest: &IconManifest, out_dir: &Path) -> Result<(), String> {
    let mut text = String::from(
        "# Eldiron Ruleset Icons\n\n\
         These bundled icon masks are generated from `crates/ruleset/rulesets/eldiron/v1/icons.toml`.\n\
         Source icons are adapted from [Game-icons.net](https://game-icons.net/) \
         under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).\n\n\
         Runtime tinting, dithering, and visual treatment are applied by Eldiron, not by this builder.\n\n\
         ## Icons\n\n",
    );

    for (id, icon) in &manifest.icons {
        let author_slug = icon
            .author_slug
            .as_deref()
            .unwrap_or(icon.author.as_str())
            .trim()
            .to_ascii_lowercase();
        let license = icon.license.as_deref().unwrap_or("CC BY 3.0");
        text.push_str(&format!(
            "- `{id}`: `{}` by {} ({license}) - https://game-icons.net/1x1/{author_slug}/{}.html\n",
            icon.name, icon.author, icon.name
        ));
    }

    let out_path = out_dir.join("ATTRIBUTION.md");
    fs::write(&out_path, text).map_err(|err| format!("Could not write attribution: {err}"))?;
    Ok(())
}
