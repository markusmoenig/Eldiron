use crate::project::Project;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const PROJECT_ARCHIVE_FORMAT: &str = "eldiron.project";
pub const PROJECT_ARCHIVE_VERSION: u32 = 1;
pub const PROJECT_MANIFEST_PATH: &str = "manifest.json";
pub const PROJECT_JSON_PATH: &str = "project.json";

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_PROJECT_JSON_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_BAKE_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ICON_FRAME_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ICON_FRAME_DIMENSION: u32 = 4096;
const MAX_ICON_FRAMES_PER_STATE: usize = 1024;
const MAX_ARCHIVE_PATH_BYTES: usize = 1024;

struct ProjectBinaryEntry {
    path: String,
    bytes: Vec<u8>,
    compression: CompressionMethod,
}

/// Compression policy for a binary payload inside an Eldiron project archive.
/// Already-compressed formats such as PNG should use `Stored`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectBinaryCompression {
    Stored,
    Deflated,
}

impl ProjectBinaryCompression {
    fn zip_method(self) -> CompressionMethod {
        match self {
            Self::Stored => CompressionMethod::Stored,
            Self::Deflated => CompressionMethod::Deflated,
        }
    }
}

/// Collects validated binary payloads from project domains before the ZIP is
/// written. All newly-authored payloads live below `binaries/`.
#[derive(Default)]
pub struct ProjectBinaryWriter {
    entries: Vec<ProjectBinaryEntry>,
    paths: BTreeSet<String>,
}

impl ProjectBinaryWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(
        &mut self,
        path: impl Into<String>,
        bytes: Vec<u8>,
        compression: ProjectBinaryCompression,
    ) -> Result<String, String> {
        let path = path.into();
        validate_archive_path(&path)?;
        if !path.starts_with("binaries/") {
            return Err(format!(
                "new project binary entry must be below binaries/: {path}"
            ));
        }
        if !self.paths.insert(path.clone()) {
            return Err(format!("duplicate project binary entry: {path}"));
        }
        self.entries.push(ProjectBinaryEntry {
            path: path.clone(),
            bytes,
            compression: compression.zip_method(),
        });
        Ok(path)
    }

    fn into_entries(self) -> Vec<ProjectBinaryEntry> {
        self.entries
    }
}

/// Reads size-limited binary payloads referenced by `project.json`. Legacy
/// pre-`binaries/` paths remain valid so existing version-one archives reopen.
pub struct ProjectBinaryReader<'archive, 'data> {
    archive: &'archive mut ZipArchive<Cursor<&'data [u8]>>,
}

impl<'archive, 'data> ProjectBinaryReader<'archive, 'data> {
    fn new(archive: &'archive mut ZipArchive<Cursor<&'data [u8]>>) -> Self {
        Self { archive }
    }

    pub fn read(&mut self, path: &str, max_size: u64) -> Result<Vec<u8>, String> {
        validate_archive_path(path)?;
        read_archive_entry(self.archive, path, max_size)
    }

    pub fn read_png(&mut self, path: &str, max_size: u64) -> Result<Vec<u8>, String> {
        let bytes = self.read(path, max_size)?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(format!("project archive binary entry {path} is not a PNG"));
        }
        Ok(bytes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectFileFormat {
    LegacyJson,
    ArchiveV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectArchiveManifest {
    pub format: String,
    pub version: u32,
}

impl Default for ProjectArchiveManifest {
    fn default() -> Self {
        Self {
            format: PROJECT_ARCHIVE_FORMAT.to_string(),
            version: PROJECT_ARCHIVE_VERSION,
        }
    }
}

pub fn project_file_format(bytes: &[u8]) -> ProjectFileFormat {
    if bytes.starts_with(b"PK\x03\x04") {
        ProjectFileFormat::ArchiveV1
    } else {
        ProjectFileFormat::LegacyJson
    }
}

pub fn decode_project(bytes: &[u8]) -> Result<Project, String> {
    match project_file_format(bytes) {
        ProjectFileFormat::LegacyJson => serde_json::from_slice(bytes)
            .map_err(|err| format!("invalid legacy Eldiron project JSON: {err}")),
        ProjectFileFormat::ArchiveV1 => decode_project_archive(bytes),
    }
}

pub fn encode_project(project: &Project) -> Result<Vec<u8>, String> {
    let mut archived_project = project.clone();
    let mut binaries = ProjectBinaryWriter::new();
    externalize_bake_entries(&mut archived_project, &mut binaries)?;
    externalize_item_icon_entries(&mut archived_project, &mut binaries)?;
    let project_json = serde_json::to_vec(&archived_project)
        .map_err(|err| format!("failed to serialize project JSON: {err}"))?;
    let manifest = serde_json::to_vec_pretty(&ProjectArchiveManifest::default())
        .map_err(|err| format!("failed to serialize project manifest: {err}"))?;

    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    writer
        .start_file(PROJECT_MANIFEST_PATH, options)
        .map_err(|err| format!("failed to create project manifest entry: {err}"))?;
    writer
        .write_all(&manifest)
        .map_err(|err| format!("failed to write project manifest: {err}"))?;
    writer
        .start_file(PROJECT_JSON_PATH, options)
        .map_err(|err| format!("failed to create project JSON entry: {err}"))?;
    writer
        .write_all(&project_json)
        .map_err(|err| format!("failed to write project JSON: {err}"))?;

    for entry in binaries.into_entries() {
        let entry_options = SimpleFileOptions::default()
            .compression_method(entry.compression)
            .unix_permissions(0o644);
        writer
            .start_file(&entry.path, entry_options)
            .map_err(|err| format!("failed to create project entry {}: {err}", entry.path))?;
        writer
            .write_all(&entry.bytes)
            .map_err(|err| format!("failed to write project entry {}: {err}", entry.path))?;
    }

    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|err| format!("failed to finish Eldiron project archive: {err}"))
}

fn decode_project_archive(bytes: &[u8]) -> Result<Project, String> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|err| format!("invalid Eldiron project archive: {err}"))?;

    let manifest_bytes =
        read_archive_entry(&mut archive, PROJECT_MANIFEST_PATH, MAX_MANIFEST_BYTES)?;
    let manifest: ProjectArchiveManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| format!("invalid Eldiron project manifest: {err}"))?;
    if manifest.format != PROJECT_ARCHIVE_FORMAT {
        return Err(format!(
            "unsupported Eldiron archive format '{}'",
            manifest.format
        ));
    }
    if manifest.version != PROJECT_ARCHIVE_VERSION {
        return Err(format!(
            "unsupported Eldiron archive version {} (expected {})",
            manifest.version, PROJECT_ARCHIVE_VERSION
        ));
    }

    let project_json = read_archive_entry(&mut archive, PROJECT_JSON_PATH, MAX_PROJECT_JSON_BYTES)?;
    let mut project: Project = serde_json::from_slice(&project_json)
        .map_err(|err| format!("invalid project.json in Eldiron archive: {err}"))?;
    let mut binaries = ProjectBinaryReader::new(&mut archive);
    hydrate_bake_entries(&mut project, &mut binaries)?;
    hydrate_item_icon_entries(&mut project, &mut binaries)?;
    Ok(project)
}

fn externalize_bake_entries(
    project: &mut Project,
    binaries: &mut ProjectBinaryWriter,
) -> Result<(), String> {
    for region in &mut project.regions {
        let Some(asset) = region.map.orthographic_bake.as_mut() else {
            continue;
        };
        for (index, tile) in asset.tiles.iter_mut().enumerate() {
            let root = format!(
                "binaries/bakes/{}/{}/tile_{index}_{}_{}",
                region.id, region.map.id, tile.x, tile.y
            );
            externalize_required_payload(
                binaries,
                &mut tile.color_png_base64,
                &mut tile.color_png_path,
                format!("{root}/color.png"),
                ProjectBinaryCompression::Stored,
            )?;
            externalize_optional_payload(
                binaries,
                &mut tile.depth_base64,
                &mut tile.depth_path,
                format!("{root}/depth.f32le"),
                ProjectBinaryCompression::Deflated,
            )?;
            externalize_optional_payload(
                binaries,
                &mut tile.albedo_png_base64,
                &mut tile.albedo_png_path,
                format!("{root}/albedo.png"),
                ProjectBinaryCompression::Stored,
            )?;
            externalize_optional_payload(
                binaries,
                &mut tile.normal_png_base64,
                &mut tile.normal_png_path,
                format!("{root}/normal.png"),
                ProjectBinaryCompression::Stored,
            )?;
            externalize_optional_payload(
                binaries,
                &mut tile.material_png_base64,
                &mut tile.material_png_path,
                format!("{root}/material.png"),
                ProjectBinaryCompression::Stored,
            )?;
        }
    }
    Ok(())
}

fn externalize_required_payload(
    binaries: &mut ProjectBinaryWriter,
    encoded: &mut String,
    stored_path: &mut Option<String>,
    path: String,
    compression: ProjectBinaryCompression,
) -> Result<(), String> {
    if encoded.is_empty() {
        return Err(format!(
            "bake payload {path} is unavailable while saving the project"
        ));
    }
    let bytes = BASE64
        .decode(encoded.as_bytes())
        .map_err(|err| format!("invalid Base64 bake payload for {path}: {err}"))?;
    binaries.add(path.clone(), bytes, compression)?;
    encoded.clear();
    *stored_path = Some(path);
    Ok(())
}

fn externalize_optional_payload(
    binaries: &mut ProjectBinaryWriter,
    encoded: &mut Option<String>,
    stored_path: &mut Option<String>,
    path: String,
    compression: ProjectBinaryCompression,
) -> Result<(), String> {
    let Some(value) = encoded.take() else {
        *stored_path = None;
        return Ok(());
    };
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|err| format!("invalid Base64 bake payload for {path}: {err}"))?;
    binaries.add(path.clone(), bytes, compression)?;
    *stored_path = Some(path);
    Ok(())
}

fn externalize_item_icon_entries(
    project: &mut Project,
    binaries: &mut ProjectBinaryWriter,
) -> Result<(), String> {
    for item in project.items.values_mut() {
        item.icon_frame_paths =
            externalize_item_icon_state(binaries, item.id, "on", &mut item.icon_frames)?;
        item.icon_off_frame_paths =
            externalize_item_icon_state(binaries, item.id, "off", &mut item.icon_off_frames)?;
    }
    Ok(())
}

fn externalize_item_icon_state(
    binaries: &mut ProjectBinaryWriter,
    item_id: theframework::prelude::Uuid,
    state: &str,
    frames: &mut Vec<rusterix::Texture>,
) -> Result<Vec<String>, String> {
    if frames.len() > MAX_ICON_FRAMES_PER_STATE {
        return Err(format!(
            "item {item_id} {state} icon has too many frames ({})",
            frames.len()
        ));
    }

    let mut paths = Vec::with_capacity(frames.len());
    for (index, texture) in frames.iter().enumerate() {
        validate_icon_texture(item_id, state, index, texture)?;
        let bytes = texture.to_rgba().to_png().map_err(|err| {
            format!("failed to encode item {item_id} {state} icon frame {index}: {err}")
        })?;
        let path = binaries.add(
            format!("binaries/items/{item_id}/icons/{state}/{index}.png"),
            bytes,
            ProjectBinaryCompression::Stored,
        )?;
        paths.push(path);
    }
    frames.clear();
    Ok(paths)
}

fn validate_icon_texture(
    item_id: theframework::prelude::Uuid,
    state: &str,
    index: usize,
    texture: &rusterix::Texture,
) -> Result<(), String> {
    if texture.width == 0
        || texture.height == 0
        || texture.width > MAX_ICON_FRAME_DIMENSION as usize
        || texture.height > MAX_ICON_FRAME_DIMENSION as usize
    {
        return Err(format!(
            "item {item_id} {state} icon frame {index} has invalid dimensions {}x{}",
            texture.width, texture.height
        ));
    }
    let expected = texture
        .width
        .checked_mul(texture.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| format!("item {item_id} {state} icon frame {index} is too large"))?;
    if texture.data.len() != expected {
        return Err(format!(
            "item {item_id} {state} icon frame {index} has {} RGBA bytes, expected {expected}",
            texture.data.len()
        ));
    }
    Ok(())
}

fn hydrate_item_icon_entries(
    project: &mut Project,
    binaries: &mut ProjectBinaryReader<'_, '_>,
) -> Result<(), String> {
    for item in project.items.values_mut() {
        hydrate_item_icon_state(
            binaries,
            item.id,
            "on",
            &item.icon_frame_paths,
            &mut item.icon_frames,
        )?;
        hydrate_item_icon_state(
            binaries,
            item.id,
            "off",
            &item.icon_off_frame_paths,
            &mut item.icon_off_frames,
        )?;
    }
    Ok(())
}

fn hydrate_item_icon_state(
    binaries: &mut ProjectBinaryReader<'_, '_>,
    item_id: theframework::prelude::Uuid,
    state: &str,
    paths: &[String],
    frames: &mut Vec<rusterix::Texture>,
) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    if paths.len() > MAX_ICON_FRAMES_PER_STATE {
        return Err(format!(
            "item {item_id} {state} icon references too many frames ({})",
            paths.len()
        ));
    }

    let mut hydrated = Vec::with_capacity(paths.len());
    for (index, path) in paths.iter().enumerate() {
        let bytes = binaries.read_png(path, MAX_ICON_FRAME_ENTRY_BYTES)?;
        validate_icon_png_dimensions(item_id, state, index, &bytes)?;
        let texture = rusterix::Texture::from_image_safe(bytes.as_slice()).ok_or_else(|| {
            format!("could not decode item {item_id} {state} icon frame {index} at {path}")
        })?;
        validate_icon_texture(item_id, state, index, &texture)?;
        hydrated.push(texture);
    }
    *frames = hydrated;
    Ok(())
}

fn validate_icon_png_dimensions(
    item_id: theframework::prelude::Uuid,
    state: &str,
    index: usize,
    bytes: &[u8],
) -> Result<(), String> {
    if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
        return Err(format!(
            "item {item_id} {state} icon frame {index} has an invalid PNG header"
        ));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width == 0
        || height == 0
        || width > MAX_ICON_FRAME_DIMENSION
        || height > MAX_ICON_FRAME_DIMENSION
    {
        return Err(format!(
            "item {item_id} {state} icon frame {index} has invalid dimensions {width}x{height}"
        ));
    }
    Ok(())
}

fn hydrate_bake_entries(
    project: &mut Project,
    binaries: &mut ProjectBinaryReader<'_, '_>,
) -> Result<(), String> {
    for region in &mut project.regions {
        let Some(asset) = region.map.orthographic_bake.as_mut() else {
            continue;
        };
        let pixel_count = u64::from(asset.tile_size)
            .checked_mul(u64::from(asset.tile_size))
            .ok_or_else(|| "bake tile dimensions overflow".to_string())?;
        let png_limit = pixel_count
            .checked_mul(4)
            .and_then(|bytes| bytes.checked_add(1024 * 1024))
            .unwrap_or(MAX_BAKE_ENTRY_BYTES)
            .min(MAX_BAKE_ENTRY_BYTES);
        let depth_size = pixel_count
            .checked_mul(4)
            .ok_or_else(|| "bake depth dimensions overflow".to_string())?;
        if depth_size > MAX_BAKE_ENTRY_BYTES {
            return Err("bake tile dimensions exceed the archive safety limit".into());
        }

        for tile in &mut asset.tiles {
            hydrate_required_payload(
                binaries,
                tile.color_png_path.as_deref(),
                &mut tile.color_png_base64,
                png_limit,
                true,
            )?;
            hydrate_optional_payload(
                binaries,
                tile.depth_path.as_deref(),
                &mut tile.depth_base64,
                depth_size,
                false,
            )?;
            hydrate_optional_payload(
                binaries,
                tile.albedo_png_path.as_deref(),
                &mut tile.albedo_png_base64,
                png_limit,
                true,
            )?;
            hydrate_optional_payload(
                binaries,
                tile.normal_png_path.as_deref(),
                &mut tile.normal_png_base64,
                png_limit,
                true,
            )?;
            hydrate_optional_payload(
                binaries,
                tile.material_png_path.as_deref(),
                &mut tile.material_png_base64,
                png_limit,
                true,
            )?;
        }
    }
    Ok(())
}

fn hydrate_required_payload(
    binaries: &mut ProjectBinaryReader<'_, '_>,
    path: Option<&str>,
    encoded: &mut String,
    max_size: u64,
    require_png: bool,
) -> Result<(), String> {
    let Some(path) = path else {
        if encoded.is_empty() {
            return Err("bake tile is missing its color payload".into());
        }
        return Ok(());
    };
    let bytes = if require_png {
        binaries.read_png(path, max_size)?
    } else {
        binaries.read(path, max_size)?
    };
    *encoded = BASE64.encode(bytes);
    Ok(())
}

fn hydrate_optional_payload(
    binaries: &mut ProjectBinaryReader<'_, '_>,
    path: Option<&str>,
    encoded: &mut Option<String>,
    max_size: u64,
    require_png: bool,
) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    let bytes = if require_png {
        binaries.read_png(path, max_size)?
    } else {
        binaries.read(path, max_size)?
    };
    *encoded = Some(BASE64.encode(bytes));
    Ok(())
}

fn read_archive_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    path: &str,
    max_size: u64,
) -> Result<Vec<u8>, String> {
    let entry = archive
        .by_name(path)
        .map_err(|_| format!("Eldiron project archive is missing {path}"))?;
    if entry.size() > max_size {
        return Err(format!(
            "Eldiron project archive entry {path} is too large ({} bytes)",
            entry.size()
        ));
    }

    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry
        .take(max_size + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read {path} from Eldiron archive: {err}"))?;
    if bytes.len() as u64 > max_size {
        return Err(format!(
            "Eldiron project archive entry {path} exceeds the size limit"
        ));
    }
    Ok(bytes)
}

fn validate_archive_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > MAX_ARCHIVE_PATH_BYTES
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(format!("invalid project archive binary path: {path}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item as ProjectItem;
    use rusterix::map::{OrthographicBakeAsset, OrthographicBakeTile};

    #[test]
    fn archive_round_trip_contains_versioned_project_json() {
        let mut project = Project::new();
        project.name = "Archive Test".to_string();

        let encoded = encode_project(&project).expect("project archive encodes");
        assert_eq!(project_file_format(&encoded), ProjectFileFormat::ArchiveV1);

        let decoded = decode_project(&encoded).expect("project archive decodes");
        assert_eq!(decoded.name, "Archive Test");

        let mut archive = ZipArchive::new(Cursor::new(encoded)).expect("valid ZIP");
        assert!(archive.by_name(PROJECT_MANIFEST_PATH).is_ok());
        assert!(archive.by_name(PROJECT_JSON_PATH).is_ok());
    }

    #[test]
    fn bake_payloads_are_binary_archive_entries_not_project_json_base64() {
        let png = b"\x89PNG\r\n\x1a\ntest-png";
        let depth = 2.5f32.to_le_bytes();
        let mut project = Project::new();
        let region = project.regions.first_mut().unwrap();
        region.map.orthographic_bake = Some(OrthographicBakeAsset {
            version: 4,
            tile_size: 1,
            pixels_per_world_unit: 1.0,
            projected_bounds: [0.0, 1.0, 0.0, 1.0],
            camera_forward: [0.0, 0.0, -1.0],
            camera_right: [1.0, 0.0, 0.0],
            camera_up: [0.0, 1.0, 0.0],
            camera_forward_origin: 0.0,
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: BASE64.encode(png),
                depth_base64: Some(BASE64.encode(depth)),
                albedo_png_base64: Some(BASE64.encode(png)),
                normal_png_base64: Some(BASE64.encode(png)),
                material_png_base64: Some(BASE64.encode(png)),
                color_png_path: None,
                depth_path: None,
                albedo_png_path: None,
                normal_png_path: None,
                material_png_path: None,
            }],
        });

        let encoded = encode_project(&project).expect("project with bake encodes");
        let mut archive = ZipArchive::new(Cursor::new(encoded.as_slice())).expect("valid ZIP");
        let project_json =
            read_archive_entry(&mut archive, PROJECT_JSON_PATH, MAX_PROJECT_JSON_BYTES).unwrap();
        let project_text = String::from_utf8(project_json).unwrap();
        assert!(!project_text.contains(&BASE64.encode(png)));
        assert!(!project_text.contains("png_base64"));
        assert!(!project_text.contains("depth_base64"));
        assert!(project_text.contains("color_png_path"));

        let stored_paths = [
            "color.png",
            "depth.f32le",
            "albedo.png",
            "normal.png",
            "material.png",
        ];
        for suffix in stored_paths {
            assert!(
                archive
                    .file_names()
                    .any(|name| name.starts_with("binaries/bakes/") && name.ends_with(suffix)),
                "missing archived bake payload {suffix}"
            );
        }
        drop(archive);

        let decoded = decode_project(&encoded).expect("project with bake decodes");
        let tile = &decoded.regions[0]
            .map
            .orthographic_bake
            .as_ref()
            .unwrap()
            .tiles[0];
        assert_eq!(BASE64.decode(&tile.color_png_base64).unwrap(), png);
        assert_eq!(
            BASE64
                .decode(tile.depth_base64.as_deref().unwrap())
                .unwrap(),
            depth
        );
    }

    #[test]
    fn item_icon_frames_are_png_binary_entries_not_project_json_arrays() {
        let mut project = Project::new();
        let mut item = ProjectItem::new();
        let item_id = item.id;
        item.name = "Custom Icon Item".into();
        item.icon_frames = vec![rusterix::Texture::new(
            vec![255, 0, 0, 255, 0, 255, 0, 128],
            2,
            1,
        )];
        item.icon_off_frames = vec![rusterix::Texture::new(vec![10, 20, 30, 255], 1, 1)];
        let expected_on = item.icon_frames.clone();
        let expected_off = item.icon_off_frames.clone();
        project.items.insert(item_id, item);

        let encoded = encode_project(&project).expect("project with item icons encodes");
        let mut archive = ZipArchive::new(Cursor::new(encoded.as_slice())).expect("valid ZIP");
        let project_json =
            read_archive_entry(&mut archive, PROJECT_JSON_PATH, MAX_PROJECT_JSON_BYTES).unwrap();
        let project_value: serde_json::Value = serde_json::from_slice(&project_json).unwrap();
        let item_value = &project_value["items"][item_id.to_string()];
        assert!(item_value.get("icon_frames").is_none());
        assert!(item_value.get("icon_off_frames").is_none());
        assert_eq!(item_value["icon_frame_paths"].as_array().unwrap().len(), 1);
        assert_eq!(
            item_value["icon_off_frame_paths"].as_array().unwrap().len(),
            1
        );

        let on_path = format!("binaries/items/{item_id}/icons/on/0.png");
        let off_path = format!("binaries/items/{item_id}/icons/off/0.png");
        assert!(archive.by_name(&on_path).is_ok());
        assert!(archive.by_name(&off_path).is_ok());
        drop(archive);

        let decoded = decode_project(&encoded).expect("project with item icons decodes");
        let item = decoded.items.get(&item_id).unwrap();
        assert_eq!(item.icon_frames, expected_on);
        assert_eq!(item.icon_off_frames, expected_off);
        assert_eq!(item.icon_frame_paths, vec![on_path]);
        assert_eq!(item.icon_off_frame_paths, vec![off_path]);
    }

    #[test]
    fn legacy_bake_entry_paths_remain_readable() {
        let png = b"\x89PNG\r\n\x1a\nlegacy-bake";
        let legacy_path = "bakes/legacy/color.png";
        let mut project = Project::new();
        project.regions[0].map.orthographic_bake = Some(OrthographicBakeAsset {
            version: 4,
            tile_size: 1,
            pixels_per_world_unit: 1.0,
            projected_bounds: [0.0, 1.0, 0.0, 1.0],
            camera_forward: [0.0, 0.0, -1.0],
            camera_right: [1.0, 0.0, 0.0],
            camera_up: [0.0, 1.0, 0.0],
            camera_forward_origin: 0.0,
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: String::new(),
                depth_base64: None,
                albedo_png_base64: None,
                normal_png_base64: None,
                material_png_base64: None,
                color_png_path: Some(legacy_path.into()),
                depth_path: None,
                albedo_png_path: None,
                normal_png_path: None,
                material_png_path: None,
            }],
        });

        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        writer.start_file(PROJECT_MANIFEST_PATH, options).unwrap();
        writer
            .write_all(&serde_json::to_vec(&ProjectArchiveManifest::default()).unwrap())
            .unwrap();
        writer.start_file(PROJECT_JSON_PATH, options).unwrap();
        writer
            .write_all(&serde_json::to_vec(&project).unwrap())
            .unwrap();
        writer.start_file(legacy_path, options).unwrap();
        writer.write_all(png).unwrap();
        let encoded = writer.finish().unwrap().into_inner();

        let decoded = decode_project(&encoded).expect("legacy bake path decodes");
        let tile = &decoded.regions[0]
            .map
            .orthographic_bake
            .as_ref()
            .unwrap()
            .tiles[0];
        assert_eq!(BASE64.decode(&tile.color_png_base64).unwrap(), png);
    }

    #[test]
    fn legacy_json_remains_supported() {
        let mut project = Project::new();
        project.name = "Legacy Test".to_string();
        let mut item = ProjectItem::new();
        let item_id = item.id;
        item.icon_frames = vec![rusterix::Texture::new(vec![1, 2, 3, 255], 1, 1)];
        project.items.insert(item_id, item);
        let encoded = serde_json::to_vec(&project).expect("legacy project encodes");

        assert_eq!(project_file_format(&encoded), ProjectFileFormat::LegacyJson);
        let decoded = decode_project(&encoded).expect("legacy project decodes");
        assert_eq!(decoded.name, "Legacy Test");
        assert_eq!(
            decoded.items[&item_id].icon_frames[0].data,
            vec![1, 2, 3, 255]
        );
    }

    #[test]
    fn binary_writer_rejects_unsafe_and_duplicate_paths() {
        let mut binaries = ProjectBinaryWriter::new();
        binaries
            .add(
                "binaries/items/item/icon.png",
                vec![1],
                ProjectBinaryCompression::Stored,
            )
            .unwrap();
        assert!(
            binaries
                .add(
                    "binaries/items/item/icon.png",
                    vec![2],
                    ProjectBinaryCompression::Stored,
                )
                .unwrap_err()
                .contains("duplicate")
        );
        assert!(
            binaries
                .add(
                    "binaries/../project.json",
                    vec![3],
                    ProjectBinaryCompression::Stored,
                )
                .is_err()
        );
        assert!(
            binaries
                .add(
                    "icons/outside.png",
                    vec![4],
                    ProjectBinaryCompression::Stored,
                )
                .is_err()
        );
    }

    #[test]
    fn archive_requires_supported_manifest() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        writer.start_file(PROJECT_MANIFEST_PATH, options).unwrap();
        writer
            .write_all(br#"{"format":"eldiron.project","version":99}"#)
            .unwrap();
        writer.start_file(PROJECT_JSON_PATH, options).unwrap();
        writer.write_all(br#"{}"#).unwrap();
        let encoded = writer.finish().unwrap().into_inner();

        let error = decode_project(&encoded).expect_err("future versions are rejected");
        assert!(error.contains("unsupported Eldiron archive version 99"));
    }
}
