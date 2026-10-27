//! Local collection import and export, independent of any online library.
use crate::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default)]
struct CollectionMetadata {
    #[serde(default)]
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
}

#[derive(Serialize, Deserialize)]
struct TileCollectionPackage {
    #[serde(default)]
    metadata: CollectionMetadata,
    collection: TileCollectionAsset,
    #[serde(default)]
    tiles: IndexMap<Uuid, rusterix::Tile>,
}

pub fn export_tile_collection_package(
    project: &Project,
    collection_id: Uuid,
    path: &Path,
) -> Result<PathBuf, String> {
    let collection = project
        .tile_collections
        .get(&collection_id)
        .ok_or("Collection not found.")?;
    let tiles = collection
        .entries
        .iter()
        .filter_map(|TileCollectionEntry::SingleTile(id)| {
            project.tiles.get(id).map(|tile| (*id, tile.clone()))
        })
        .collect();
    let payload = TileCollectionPackage {
        metadata: CollectionMetadata {
            name: collection.name.clone(),
            author: collection.author.clone(),
            version: collection.version.clone(),
            description: collection.description.clone(),
        },
        collection: collection.clone(),
        tiles,
    };
    let mut path = path.to_path_buf();
    if path.extension().is_none() {
        path.set_extension("eldiron_tiles");
    }
    let text = serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?;
    std::fs::write(&path, text).map_err(|err| err.to_string())?;
    Ok(path)
}

pub fn import_tile_collection_package(project: &mut Project, path: &Path) -> Result<Uuid, String> {
    let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| err.to_string())?;
    shared::legacy_tiles::migrate(&mut value)?;
    let payload: TileCollectionPackage =
        serde_json::from_value(value).map_err(|err| err.to_string())?;
    import_payload(project, payload)
}

fn import_payload(project: &mut Project, payload: TileCollectionPackage) -> Result<Uuid, String> {
    let mut collection = payload.collection;
    let mut ids = IndexMap::new();
    for old_id in payload.tiles.keys() {
        ids.insert(*old_id, Uuid::new_v4());
    }
    for TileCollectionEntry::SingleTile(id) in &mut collection.entries {
        *id = *ids.get(id).ok_or("Collection references a missing tile.")?;
    }
    collection.tile_board_tiles = collection
        .tile_board_tiles
        .into_iter()
        .filter_map(|(id, pos)| ids.get(&id).map(|new_id| (*new_id, pos)))
        .collect();
    for (old_id, mut tile) in payload.tiles {
        tile.id = ids[&old_id];
        project.tiles.insert(tile.id, tile);
    }
    collection.id = Uuid::new_v4();
    if collection.name.is_empty() {
        collection.name = payload.metadata.name;
    }
    if collection.author.is_empty() {
        collection.author = payload.metadata.author;
    }
    if collection.version.is_empty() {
        collection.version = payload.metadata.version;
    }
    if collection.description.is_empty() {
        collection.description = payload.metadata.description;
    }
    let id = collection.id;
    project.tile_collections.insert(id, collection);
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_import_remaps_board_positions_and_preserves_empty_cells() {
        let mut project = Project::new();
        let tile = rusterix::Tile::from_texture(rusterix::Texture::from_color([10, 20, 30, 255]));
        let old_id = tile.id;
        let mut collection = TileCollectionAsset::new("Local".into());
        collection
            .entries
            .push(TileCollectionEntry::SingleTile(old_id));
        collection.tile_board_tiles.insert(old_id, Vec2::new(3, 2));
        collection.tile_board_empty_slots.push(Vec2::new(1, 0));
        let mut tiles = IndexMap::new();
        tiles.insert(old_id, tile);
        let id = import_payload(
            &mut project,
            TileCollectionPackage {
                metadata: CollectionMetadata::default(),
                collection,
                tiles,
            },
        )
        .unwrap();
        let collection = &project.tile_collections[&id];
        let TileCollectionEntry::SingleTile(new_id) = collection.entries[0];
        assert_ne!(old_id, new_id);
        assert_eq!(collection.tile_board_tiles[&new_id], Vec2::new(3, 2));
        assert!(!collection.tile_board_tiles.contains_key(&old_id));
        assert_eq!(collection.tile_board_empty_slots, vec![Vec2::new(1, 0)]);
        assert_eq!(project.tiles[&new_id].id, new_id);
    }

    #[test]
    fn collection_import_with_missing_tiles_leaves_project_unchanged() {
        let mut project = Project::new();
        let mut collection = TileCollectionAsset::new("Broken".into());
        collection
            .entries
            .push(TileCollectionEntry::SingleTile(Uuid::new_v4()));
        let before = serde_json::to_value(&project).unwrap();
        assert!(
            import_payload(
                &mut project,
                TileCollectionPackage {
                    metadata: CollectionMetadata::default(),
                    collection,
                    tiles: IndexMap::new()
                }
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }
}
