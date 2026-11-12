use crate::{Assets, Tile};
use theframework_core::Uuid;

pub fn resolve_tile_reference<'a>(assets: &'a Assets, tile_ref: &str) -> Option<&'a Tile> {
    let trimmed = tile_ref.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(id) = Uuid::parse_str(trimmed) {
        return assets.tiles.get(&id);
    }
    let needle = trimmed.to_ascii_lowercase();
    assets.tiles.values().find(|tile| {
        tile.alias
            .split([',', ';'])
            .map(str::trim)
            .filter(|alias| !alias.is_empty())
            .any(|alias| alias.eq_ignore_ascii_case(&needle))
            || tile.alias.eq_ignore_ascii_case(&needle)
    })
}
