pub use eldiron_ruleset::*;

use theframework::prelude::Uuid;

pub fn bundled_avatars_for_project(
    config_src: &str,
) -> Result<Vec<(&'static str, rusterix::Avatar)>, String> {
    let (id, version, source) = eldiron_ruleset::selected_ruleset(config_src);
    if source == "project" {
        return Ok(Vec::new());
    }

    eldiron_ruleset::bundled_avatar_assets_for_ruleset(&id, &version)
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
    let (id, version, source) = eldiron_ruleset::selected_ruleset(config_src);
    if source == "project" {
        return Ok(Vec::new());
    }

    eldiron_ruleset::bundled_tile_assets_for_ruleset(&id, &version)
        .into_iter()
        .map(|asset| {
            serde_json::from_str::<rusterix::Tile>(asset.source)
                .map(|tile| (tile.id, tile))
                .map_err(|err| {
                    format!(
                        "Bundled ruleset tile '{}' at '{}' could not be parsed: {}",
                        asset.id, asset.path, err
                    )
                })
        })
        .collect()
}

pub fn bundled_textures_for_project(
    config_src: &str,
) -> Result<Vec<(&'static str, rusterix::Texture)>, String> {
    let (id, version, source) = eldiron_ruleset::selected_ruleset(config_src);
    if source == "project" {
        return Ok(Vec::new());
    }

    eldiron_ruleset::bundled_texture_assets_for_ruleset(&id, &version)
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
