use super::*;

pub(crate) fn action_bar_tile_texture(
    table: &toml::Table,
    key: &str,
    assets: &Assets,
) -> Option<Texture> {
    let id = table
        .get(key)
        .and_then(toml::Value::as_str)
        .and_then(|value| Uuid::parse_str(value.trim()).ok())?;
    assets
        .tiles
        .get(&id)
        .and_then(|tile| tile.textures.first())
        .cloned()
}

pub(crate) fn action_bar_state_textures(table: &toml::Table, assets: &Assets) -> Vec<Texture> {
    let normal = action_bar_tile_texture(table, "normal_tile", assets)
        .or_else(|| action_bar_tile_texture(table, "tile", assets))
        .or_else(|| action_bar_tile_texture(table, "hover_tile", assets))
        .or_else(|| action_bar_tile_texture(table, "selected_tile", assets))
        .or_else(|| action_bar_tile_texture(table, "pressed_tile", assets))
        .or_else(|| action_bar_tile_texture(table, "disabled_tile", assets));
    let Some(normal) = normal else {
        return vec![];
    };
    let selected =
        action_bar_tile_texture(table, "selected_tile", assets).unwrap_or_else(|| normal.clone());
    let pressed =
        action_bar_tile_texture(table, "pressed_tile", assets).unwrap_or_else(|| selected.clone());
    let disabled =
        action_bar_tile_texture(table, "disabled_tile", assets).unwrap_or_else(|| normal.clone());
    let hover =
        action_bar_tile_texture(table, "hover_tile", assets).unwrap_or_else(|| normal.clone());
    vec![normal, selected, pressed, disabled, hover]
}

pub(crate) fn action_bar_slot_textures(table: &toml::Table, assets: &Assets) -> Vec<Texture> {
    table
        .get("slot")
        .and_then(toml::Value::as_table)
        .map(|slot| action_bar_state_textures(slot, assets))
        .unwrap_or_default()
}

pub(crate) fn action_bar_artwork(
    table: &toml::Table,
    slice: usize,
) -> widget::artwork::ScreenArtwork {
    let source = |key: &str| {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .and_then(|s| Uuid::parse_str(s.trim()).ok())
            .map(PixelSource::TileId)
    };
    let normal = [
        "normal_tile",
        "tile",
        "hover_tile",
        "selected_tile",
        "pressed_tile",
        "disabled_tile",
    ]
    .iter()
    .find_map(|key| source(key));
    let Some(normal) = normal else {
        return Default::default();
    };
    let selected = source("selected_tile").unwrap_or_else(|| normal.clone());
    widget::artwork::ScreenArtwork {
        sources: vec![
            normal.clone(),
            selected.clone(),
            source("pressed_tile").unwrap_or(selected),
            source("disabled_tile").unwrap_or_else(|| normal.clone()),
            source("hover_tile").unwrap_or(normal),
        ],
        slice,
        ..Default::default()
    }
}
