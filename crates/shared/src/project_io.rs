use crate::project::Project;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const PROJECT_ARCHIVE_FORMAT: &str = "eldiron.project";
pub const PROJECT_ARCHIVE_VERSION: u32 = 1;
pub const PROJECT_MANIFEST_PATH: &str = "manifest.json";
pub const PROJECT_JSON_PATH: &str = "project.json";

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_PROJECT_JSON_BYTES: u64 = 1024 * 1024 * 1024;

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
    let project_json = serde_json::to_vec(project)
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
    serde_json::from_slice(&project_json)
        .map_err(|err| format!("invalid project.json in Eldiron archive: {err}"))
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn legacy_json_remains_supported() {
        let mut project = Project::new();
        project.name = "Legacy Test".to_string();
        let encoded = serde_json::to_vec(&project).expect("legacy project encodes");

        assert_eq!(project_file_format(&encoded), ProjectFileFormat::LegacyJson);
        let decoded = decode_project(&encoded).expect("legacy project decodes");
        assert_eq!(decoded.name, "Legacy Test");
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
