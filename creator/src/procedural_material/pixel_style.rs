//! Project-owned render style, independent from standalone material recipes.
pub(super) const DEFAULT_PIXELS_PER_TILE: u32 = 16;
pub(crate) fn density(config: &str) -> u32 {
    config
        .parse::<toml::Table>()
        .ok()
        .and_then(|t| t.get("materials")?.get("pixels_per_tile")?.as_integer())
        .map(|v| v.clamp(0, 64) as u32)
        .unwrap_or(DEFAULT_PIXELS_PER_TILE)
}
/// Smooth mode keeps the existing bake detail; pixel-art mode stores its actual grid.
pub(super) fn bake_size(pixels: u32) -> usize {
    if pixels == 0 { 64 } else { pixels as usize }
}
pub(crate) fn set_density(config: &str, pixels: u32) -> Result<String, String> {
    let mut document = config
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("Fix Game settings before changing material resolution: {e}"))?;
    if document.get("materials").is_none() {
        document["materials"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let table = document["materials"]
        .as_table_like_mut()
        .ok_or("Game settings [materials] must be a table")?;
    let mut value = toml_edit::Value::from(pixels.min(64) as i64);
    if let Some(old) = table
        .get("pixels_per_tile")
        .and_then(|item| item.as_value())
    {
        *value.decor_mut() = old.decor().clone();
    }
    if let Some(item) = table.get_mut("pixels_per_tile") {
        *item = toml_edit::Item::Value(value);
    } else {
        table.insert("pixels_per_tile", toml_edit::Item::Value(value));
    }
    Ok(document.to_string())
}
pub(super) fn bake_key(bundle: &str, pixels: u32) -> u64 {
    super::cache_key(&format!("palette-tick-v3:{pixels}\n{bundle}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_pixel_style_defaults_and_changes_invalidate_bakes() {
        assert_eq!(density(""), 16);
        assert_eq!(density("[materials]\npixels_per_tile = 32"), 32);
        assert_eq!(density("[materials]\npixels_per_tile = 0"), 0);
        assert_ne!(bake_key("recipe", 0), super::super::cache_key("recipe"));
        assert_ne!(bake_key("recipe", 16), bake_key("recipe", 32));
        assert_ne!(
            bake_key("recipe", 16),
            super::super::cache_key("pixel-grid:16\nrecipe")
        );
        assert_eq!(bake_size(16), 16);
        assert_eq!(bake_size(32), 32);
        assert_eq!(bake_size(0), 64);
    }
    #[test]
    fn resolution_control_preserves_other_settings_and_comments() {
        let config = "# Keep this\n[game]\ntarget_fps = 60 # Frame rate\n\n[materials]\n# Material detail\npixels_per_tile = 16\n";
        let changed = set_density(config, 32).unwrap();
        assert_eq!(density(&changed), 32);
        assert!(changed.contains("# Keep this"));
        assert!(changed.contains("target_fps = 60 # Frame rate"));
        assert!(changed.contains("# Material detail"));
        assert_eq!(
            density(&set_density("[game]\ntarget_fps = 60\n", 24).unwrap()),
            24
        );
        assert!(set_density("[game", 32).is_err());
    }
}
