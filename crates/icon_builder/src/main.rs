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

    let mut imported = 0;
    for (id, icon) in &manifest.icons {
        if import_missing_icon(id, icon, &out_dir)? {
            imported += 1;
        }
    }
    write_attribution(&manifest, &out_dir)?;

    println!(
        "Imported {imported} missing icon(s); preserved {} existing artist-editable PNG(s) in {}",
        manifest.icons.len() - imported,
        out_dir.display()
    );
    Ok(())
}

fn import_missing_icon(id: &str, icon: &IconEntry, out_dir: &Path) -> Result<bool, String> {
    let out_path = out_dir.join(id).join("on").join("0.png");
    if out_path.exists() {
        return Ok(false);
    }

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
    let output = normalize_imported_icon(image);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create '{}': {err}", parent.display()))?;
    }
    output
        .save_with_format(&out_path, ImageFormat::Png)
        .map_err(|err| format!("Could not write '{}': {err}", out_path.display()))?;
    Ok(true)
}

fn normalize_imported_icon(image: DynamicImage) -> DynamicImage {
    let mut resized = image.resize_exact(32, 32, FilterType::Nearest).to_rgba8();
    for pixel in resized.pixels_mut() {
        if pixel[3] > 0 {
            // New imports receive the old neutral item-icon default as actual
            // RGBA pixels. From this point onward the PNG is edited and used as-is.
            pixel[0] = 216;
            pixel[1] = 216;
            pixel[2] = 216;
        }
    }
    DynamicImage::ImageRgba8(resized)
}

fn write_attribution(manifest: &IconManifest, out_dir: &Path) -> Result<(), String> {
    let mut text = String::from(
        "# Eldiron Ruleset Icons\n\n\
         The PNG files under `<id>/<state>/<frame>.png` are the authoritative, artist-editable icon artwork.\n\
         `crates/ruleset/rulesets/eldiron/v1/icons.toml` records the upstream sources and licenses for the Game-icons-derived subset. Eldiron-authored item icons are not derived from those upstream files.\n\
         Source icons are adapted from [Game-icons.net](https://game-icons.net/) \
         under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).\n\n\
         `eldiron-icon-builder` only imports missing files and never overwrites existing PNG artwork.\n\n\
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
