pub use eldiron_ruleset::*;

use theframework::prelude::Uuid;

pub fn bundled_avatars_for_project(
    config_src: &str,
) -> Result<Vec<(&'static str, rusterix::Avatar)>, String> {
    let selection = eldiron_ruleset::selected_ruleset_config(config_src);
    if selection.source == "project" {
        return Ok(Vec::new());
    }

    eldiron_ruleset::bundled_avatar_assets_for_ruleset(&selection.id, &selection.version)
        .into_iter()
        .map(|asset| {
            serde_json::from_str::<rusterix::Avatar>(asset.source)
                .map(|avatar| (asset.id, avatar))
                .map_err(|err| {
                    format!(
                        "Bundled ruleset avatar '{}' at '{}' could not be parsed: {}",
                        asset.id, asset.path, err
                    )
                })
        })
        .collect()
}

pub fn bundled_tiles_for_project(config_src: &str) -> Result<Vec<(Uuid, rusterix::Tile)>, String> {
    let selection = eldiron_ruleset::selected_ruleset_config(config_src);
    if selection.source == "project" {
        return Ok(Vec::new());
    }

    eldiron_ruleset::bundled_icon_state_assets_for_ruleset(&selection.id, &selection.version)
        .into_iter()
        .filter(|asset| asset.tile_id.is_some())
        .map(|asset| {
            let id = Uuid::parse_str(asset.tile_id.unwrap()).map_err(|err| {
                format!(
                    "Bundled icon state '{}:{}' has invalid tile id: {}",
                    asset.item_id, asset.state, err
                )
            })?;
            let textures = decode_icon_state_frames(asset)?;
            let mut tile = rusterix::Tile::from_textures(textures);
            tile.id = id;
            tile.alias = asset.tile_alias.to_string();
            tile.role = match asset.tile_role {
                "Character" => rusterix::TileRole::Character,
                "Nature" => rusterix::TileRole::Nature,
                "Mountain" => rusterix::TileRole::Mountain,
                "Road" => rusterix::TileRole::Road,
                "Water" => rusterix::TileRole::Water,
                "Dungeon" => rusterix::TileRole::Dungeon,
                "Effect" => rusterix::TileRole::Effect,
                "Icon" => rusterix::TileRole::Icon,
                "UI" => rusterix::TileRole::UI,
                _ => rusterix::TileRole::ManMade,
            };
            Ok((id, tile))
        })
        .collect()
}

fn decode_icon_state_frames(
    asset: &eldiron_ruleset::BundledIconStateAsset,
) -> Result<Vec<rusterix::Texture>, String> {
    asset
        .frames
        .iter()
        .enumerate()
        .map(|(index, source)| {
            rusterix::Texture::from_image_safe(*source).ok_or_else(|| {
                format!(
                    "Bundled icon state '{}:{}' frame {} at '{}' could not be decoded",
                    asset.item_id, asset.state, index, asset.path
                )
            })
        })
        .collect()
}

pub fn bundled_item_icon_states_for_project(
    config_src: &str,
) -> Result<Vec<(&'static str, &'static str, Vec<rusterix::Texture>)>, String> {
    let selection = eldiron_ruleset::selected_ruleset_config(config_src);
    if selection.source == "project" {
        return Ok(Vec::new());
    }

    eldiron_ruleset::bundled_icon_state_assets_for_ruleset(&selection.id, &selection.version)
        .into_iter()
        .map(|asset| {
            decode_icon_state_frames(asset).map(|frames| (asset.item_id, asset.state, frames))
        })
        .collect()
}

pub fn bundled_textures_for_project(
    config_src: &str,
) -> Result<Vec<(&'static str, rusterix::Texture)>, String> {
    let selection = eldiron_ruleset::selected_ruleset_config(config_src);
    if selection.source == "project" {
        return Ok(Vec::new());
    }

    eldiron_ruleset::bundled_texture_assets_for_ruleset(&selection.id, &selection.version)
        .into_iter()
        .map(|asset| {
            rusterix::Texture::from_image_safe(asset.source)
                .map(|texture| (asset.id, texture))
                .ok_or_else(|| {
                    format!(
                        "Bundled ruleset texture '{}' at '{}' could not be decoded",
                        asset.id, asset.path
                    )
                })
        })
        .collect()
}
