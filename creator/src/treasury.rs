use crate::prelude::*;
use std::io::Read;
use std::path::{Path, PathBuf};

const TREASURY_RAW_BASE: &str =
    "https://raw.githubusercontent.com/markusmoenig/Eldiron-Treasury/main/";

pub fn fetch_tile_packages() -> Result<Vec<TreasuryPackageSummary>, String> {
    let text = fetch_url_text(&format!("{TREASURY_RAW_BASE}index.json"))?;
    let index: TreasuryIndex = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let entries = if !index.tiles.is_empty() {
        index.tiles
    } else {
        index.categories.tiles
    };

    Ok(entries
        .into_iter()
        .filter(|entry| !entry.slug.trim().is_empty())
        .collect())
}

pub fn install_tile_package(
    project: &mut Project,
    package: &TreasuryPackageSummary,
) -> Result<Uuid, String> {
    let text = fetch_url_text(&payload_url(package))?;
    let payload: TreasuryTileCollectionPackage =
        serde_json::from_str(&text).map_err(|e| e.to_string())?;
    import_tile_collection_payload(project, payload, Some(package.metadata()))
}

pub fn export_tile_collection_package(
    project: &Project,
    collection_id: Uuid,
    path: &Path,
) -> Result<PathBuf, String> {
    let payload = build_tile_collection_package(project, collection_id)?;
    let text = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    let path = normalize_package_path(path);
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn export_tile_collection_to_treasury_repo(
    project: &Project,
    collection_id: Uuid,
    repo_root: &Path,
) -> Result<PathBuf, String> {
    let payload = build_tile_collection_package(project, collection_id)?;
    let metadata = payload.metadata.clone();
    let slug = slugify(&metadata.name);
    if slug.is_empty() {
        return Err("Collection needs a name before Treasury export.".to_string());
    }

    let tiles_root = repo_root.join("tiles");
    let package_dir = tiles_root.join(&slug);
    std::fs::create_dir_all(&package_dir).map_err(|e| e.to_string())?;

    let collection_json_path = package_dir.join("collection.json");
    let collection_json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&collection_json_path, collection_json).map_err(|e| e.to_string())?;

    let manifest = TreasuryPackageManifest::tile_collection(&metadata);
    let manifest_toml = toml::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(package_dir.join("package.toml"), manifest_toml).map_err(|e| e.to_string())?;

    let index_path = repo_root.join("index.json");
    let mut index = if index_path.exists() {
        let text = std::fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
        serde_json::from_str::<TreasuryIndex>(&text).unwrap_or_default()
    } else {
        TreasuryIndex::default()
    };

    let summary = TreasuryPackageSummary {
        id: Uuid::new_v4(),
        slug: slug.clone(),
        name: metadata.name.clone(),
        author: metadata.author.clone(),
        version: metadata.version.clone(),
        description: metadata.description.clone(),
    };

    if let Some(existing) = index.tiles.iter_mut().find(|entry| entry.slug == slug) {
        *existing = summary;
    } else {
        index.tiles.push(summary);
    }
    index.tiles.sort_by(|a, b| {
        a.display_name()
            .to_lowercase()
            .cmp(&b.display_name().to_lowercase())
    });

    let index_text = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    std::fs::write(index_path, index_text).map_err(|e| e.to_string())?;

    Ok(package_dir)
}

pub fn import_tile_collection_package(project: &mut Project, path: &Path) -> Result<Uuid, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let payload: TreasuryTileCollectionPackage =
        serde_json::from_str(&text).map_err(|e| e.to_string())?;
    import_tile_collection_payload(project, payload, None)
}

pub fn build_tile_collection_package(
    project: &Project,
    collection_id: Uuid,
) -> Result<TreasuryTileCollectionPackage, String> {
    let Some(collection) = project.tile_collections.get(&collection_id) else {
        return Err("Collection not found.".to_string());
    };

    let mut tiles = IndexMap::default();
    let mut tile_groups = IndexMap::default();
    let mut tile_node_groups = IndexMap::default();

    for entry in &collection.entries {
        match entry {
            TileCollectionEntry::SingleTile(tile_id) => {
                let Some(tile) = project.tiles.get(tile_id) else {
                    continue;
                };
                tiles.insert(*tile_id, tile.clone());
            }
            TileCollectionEntry::TileGroup(group_id) => {
                let Some(group) = project.tile_groups.get(group_id) else {
                    continue;
                };
                for member in &group.members {
                    if let Some(tile) = project.tiles.get(&member.tile_id) {
                        tiles.insert(member.tile_id, tile.clone());
                    }
                }
                tile_groups.insert(*group_id, group.clone());
                if let Some(node_group) = project.tile_node_groups.get(group_id) {
                    tile_node_groups.insert(*group_id, node_group.clone());
                }
            }
        }
    }

    Ok(TreasuryTileCollectionPackage {
        metadata: TreasuryPackageMetadata::from_collection(collection),
        collection: collection.clone(),
        tiles,
        tile_groups,
        tile_node_groups,
    })
}

pub fn import_tile_collection_payload(
    project: &mut Project,
    payload: TreasuryTileCollectionPackage,
    metadata_override: Option<TreasuryPackageMetadata>,
) -> Result<Uuid, String> {
    let mut tile_map: IndexMap<Uuid, Uuid> = IndexMap::default();
    let mut group_map: IndexMap<Uuid, Uuid> = IndexMap::default();

    for old_id in payload.tiles.keys() {
        tile_map.insert(*old_id, Uuid::new_v4());
    }
    for old_id in payload.tile_groups.keys() {
        group_map.insert(*old_id, Uuid::new_v4());
    }

    for (old_id, mut tile) in payload.tiles {
        let Some(new_id) = tile_map.get(&old_id).copied() else {
            continue;
        };
        tile.id = new_id;
        project.tiles.insert(new_id, tile);
    }

    for (old_id, mut group) in payload.tile_groups {
        let Some(new_group_id) = group_map.get(&old_id).copied() else {
            continue;
        };
        group.id = new_group_id;
        for member in &mut group.members {
            if let Some(new_tile_id) = tile_map.get(&member.tile_id).copied() {
                member.tile_id = new_tile_id;
            }
        }
        project.tile_groups.insert(new_group_id, group);
    }

    for (old_group_id, mut node_group) in payload.tile_node_groups {
        let Some(new_group_id) = group_map.get(&old_group_id).copied() else {
            continue;
        };
        node_group.group_id = new_group_id;
        node_group.graph_id = Uuid::new_v4();
        project.tile_node_groups.insert(new_group_id, node_group);
    }

    let mut collection = payload.collection;
    collection.id = Uuid::new_v4();
    for entry in &mut collection.entries {
        match entry {
            TileCollectionEntry::SingleTile(tile_id) => {
                if let Some(new_tile_id) = tile_map.get(tile_id).copied() {
                    *tile_id = new_tile_id;
                }
            }
            TileCollectionEntry::TileGroup(group_id) => {
                if let Some(new_group_id) = group_map.get(group_id).copied() {
                    *group_id = new_group_id;
                }
            }
        }
    }

    let metadata = metadata_override.unwrap_or(payload.metadata);
    if collection.name.is_empty() {
        collection.name = metadata.name;
    }
    if collection.author.is_empty() {
        collection.author = metadata.author;
    }
    if collection.version.is_empty() {
        collection.version = metadata.version;
    }
    if collection.description.is_empty() {
        collection.description = metadata.description;
    }

    let id = collection.id;
    project.tile_collections.insert(id, collection);
    Ok(id)
}

fn payload_url(package: &TreasuryPackageSummary) -> String {
    format!("{TREASURY_RAW_BASE}tiles/{}/collection.json", package.slug)
}

fn normalize_package_path(path: &Path) -> PathBuf {
    if path.extension().is_some() {
        path.to_path_buf()
    } else {
        path.with_extension("eldiron_tiles")
    }
}

pub fn default_treasury_repo_root() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Eldiron-Treasury"))
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in name.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn fetch_url_text(url: &str) -> Result<String, String> {
    let response = ureq::get(url).call().map_err(|e| e.to_string())?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
