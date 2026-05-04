use crate::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};

const TREASURY_RAW_BASE: &str =
    "https://raw.githubusercontent.com/markusmoenig/Eldiron-Treasury/main/";
const TREASURY_GITHUB_CONTENTS_BASE: &str =
    "https://api.github.com/repos/markusmoenig/Eldiron-Treasury/contents/";
const TREASURY_GITHUB_REF: &str = "main";

#[derive(Clone, Debug)]
pub struct TreasuryBuilderGraphTemplate {
    pub summary: TreasuryBuilderGraphSummary,
    pub graph_data: String,
}

#[derive(Debug)]
struct GitHubBuilderGraphFile {
    path: String,
    download_url: String,
}

#[derive(Deserialize)]
struct GitHubContentEntry {
    name: String,
    path: String,
    #[serde(rename = "type")]
    kind: String,
    download_url: Option<String>,
}

#[derive(Default, Deserialize)]
struct TreasuryBuilderGraphManifest {
    #[serde(default)]
    id: Option<Uuid>,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    target: String,
}

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

pub fn fetch_builder_graph_templates() -> Result<Vec<TreasuryBuilderGraphTemplate>, String> {
    fetch_builder_graph_templates_from_index().or_else(|index_err| {
        fetch_builder_graph_templates_from_github().map_err(|github_err| {
            format!(
                "Treasury index failed: {index_err}. GitHub template discovery failed: {github_err}"
            )
        })
    })
}

fn fetch_builder_graph_templates_from_index() -> Result<Vec<TreasuryBuilderGraphTemplate>, String> {
    let text = fetch_url_text(&format!("{TREASURY_RAW_BASE}index.json"))?;
    let index: TreasuryIndex = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut entries = if !index.builder_graphs.is_empty() {
        index.builder_graphs
    } else {
        index.categories.builder_graphs
    };
    entries.retain(|entry| !entry.path.trim().is_empty());
    entries.sort_by(|a, b| {
        a.display_name()
            .to_lowercase()
            .cmp(&b.display_name().to_lowercase())
    });

    let mut templates = Vec::new();
    for summary in entries {
        let graph_data = fetch_url_text(&format!("{TREASURY_RAW_BASE}{}", summary.path))?;
        templates.push(TreasuryBuilderGraphTemplate {
            summary,
            graph_data,
        });
    }

    if templates.is_empty() {
        Err("No Builder Graph templates found in the Treasury index.".to_string())
    } else {
        Ok(templates)
    }
}

fn fetch_builder_graph_templates_from_github() -> Result<Vec<TreasuryBuilderGraphTemplate>, String>
{
    let mut graph_files = Vec::new();
    collect_github_builder_graphs("builders", &mut graph_files)?;
    graph_files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut templates = Vec::new();
    for graph_file in graph_files {
        let graph_data = fetch_url_text(&graph_file.download_url)?;
        let summary = fetch_builder_graph_summary_for_github_file(&graph_file)?;
        templates.push(TreasuryBuilderGraphTemplate {
            summary,
            graph_data,
        });
    }

    if templates.is_empty() {
        Err("No Builder Graph templates found in the Treasury GitHub repo.".to_string())
    } else {
        templates.sort_by(|a, b| {
            a.summary
                .display_name()
                .to_lowercase()
                .cmp(&b.summary.display_name().to_lowercase())
        });
        Ok(templates)
    }
}

fn collect_github_builder_graphs(
    path: &str,
    graph_files: &mut Vec<GitHubBuilderGraphFile>,
) -> Result<(), String> {
    for entry in fetch_github_contents(path)? {
        match entry.kind.as_str() {
            "dir" => collect_github_builder_graphs(&entry.path, graph_files)?,
            "file" if entry.name == "graph.buildergraph" => {
                if let Some(download_url) = entry.download_url {
                    graph_files.push(GitHubBuilderGraphFile {
                        path: entry.path,
                        download_url,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn fetch_builder_graph_summary_for_github_file(
    graph_file: &GitHubBuilderGraphFile,
) -> Result<TreasuryBuilderGraphSummary, String> {
    let dir = graph_file
        .path
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");
    let package_path = if dir.is_empty() {
        "package.toml".to_string()
    } else {
        format!("{dir}/package.toml")
    };
    let package_url = format!("{TREASURY_RAW_BASE}{package_path}");
    let package = fetch_url_text(&package_url)
        .ok()
        .and_then(|text| toml::from_str::<TreasuryBuilderGraphManifest>(&text).ok())
        .unwrap_or_default();
    let slug = if package.slug.is_empty() {
        dir.rsplit('/').next().unwrap_or("").to_string()
    } else {
        package.slug
    };
    Ok(TreasuryBuilderGraphSummary {
        id: package
            .id
            .unwrap_or_else(|| stable_treasury_builder_id(&graph_file.path)),
        slug,
        name: package.name,
        author: package.author,
        version: package.version,
        description: package.description,
        path: graph_file.path.clone(),
        aliases: package.aliases,
        tags: package.tags,
        target: package.target,
    })
}

fn fetch_github_contents(path: &str) -> Result<Vec<GitHubContentEntry>, String> {
    let url = format!("{TREASURY_GITHUB_CONTENTS_BASE}{path}?ref={TREASURY_GITHUB_REF}");
    let text = fetch_url_text(&url)?;
    serde_json::from_str::<Vec<GitHubContentEntry>>(&text).map_err(|e| e.to_string())
}

fn stable_treasury_builder_id(path: &str) -> Uuid {
    let mut first = DefaultHasher::new();
    "eldiron-treasury-builder".hash(&mut first);
    path.hash(&mut first);
    let first = first.finish();

    let mut second = DefaultHasher::new();
    "eldiron-treasury-builder-secondary".hash(&mut second);
    path.hash(&mut second);
    let second = second.finish();

    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&first.to_be_bytes());
    bytes[8..].copy_from_slice(&second.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
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
    let response = ureq::get(url)
        .set("User-Agent", "Eldiron")
        .call()
        .map_err(|e| e.to_string())?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
