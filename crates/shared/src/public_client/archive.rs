use super::*;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};
const MANIFEST: &str = "manifest.json";
const PAYLOAD: &str = "client.msgpack";
fn hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    max: usize,
) -> Result<Vec<u8>, String> {
    let file = archive.by_name(name).map_err(|e| e.to_string())?;
    if file.size() > max as u64 {
        return Err("Client archive entry exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > max {
        return Err("Client archive entry exceeds size limit".into());
    }
    Ok(bytes)
}
pub fn is_public_archive(bytes: &[u8]) -> Result<bool, String> {
    if !bytes.starts_with(b"PK\x03\x04") {
        return Ok(false);
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let manifest = entry(&mut zip, MANIFEST, 512 * 1024)?;
    let value: serde_json::Value = serde_json::from_slice(&manifest).map_err(|e| e.to_string())?;
    Ok(value.get("format").and_then(|v| v.as_str()) == Some(FORMAT))
}
pub fn encode(
    payload: &Payload,
    content_revision: String,
    protocol: u16,
) -> Result<Vec<u8>, String> {
    encode_with_packs(payload, content_revision, protocol, Vec::new())
}
pub(super) fn encode_with_packs(
    payload: &Payload,
    content_revision: String,
    protocol: u16,
    packs: Vec<streaming::PackRef>,
) -> Result<Vec<u8>, String> {
    if !hex(&content_revision) {
        return Err("Invalid server content revision".into());
    }
    payload.validate()?;
    let bytes = rmp_serde::to_vec_named(payload).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err("Public release exceeds 128 MiB; reduce exported artwork".into());
    }
    let manifest = Manifest {
        format: FORMAT.into(),
        version: if packs.is_empty() { VERSION } else { 2 },
        engine: env!("CARGO_PKG_VERSION").into(),
        protocol,
        content_revision,
        packs,
        payload_sha256: format!("{:x}", Sha256::digest(&bytes)),
    };
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(MANIFEST, options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&serde_json::to_vec(&manifest).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    zip.start_file(PAYLOAD, options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&bytes).map_err(|e| e.to_string())?;
    let bytes = zip.finish().map_err(|e| e.to_string())?.into_inner();
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err("Public archive exceeds 128 MiB".into());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8], protocol: u16) -> Result<Project, String> {
    let (payload, manifest) = decode_payload(bytes, protocol)?;
    payload.into_project(manifest)
}
pub(super) fn decode_payload(bytes: &[u8], protocol: u16) -> Result<(Payload, Manifest), String> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err("Public archive exceeds size limit".into());
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if zip.len() != 2 {
        return Err("Public archive must contain exactly manifest.json and client.msgpack".into());
    }
    let mut names = std::collections::BTreeSet::new();
    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(|e| e.to_string())?;
        if !matches!(file.name(), MANIFEST | PAYLOAD) || !names.insert(file.name().to_owned()) {
            return Err("Unexpected/duplicate public archive entry".into());
        }
    }
    let manifest: Manifest = serde_json::from_slice(&entry(&mut zip, MANIFEST, 512 * 1024)?)
        .map_err(|e| e.to_string())?;
    if manifest.format != FORMAT
        || !matches!(manifest.version, 1 | 2)
        || manifest.engine != env!("CARGO_PKG_VERSION")
        || manifest.protocol != protocol
    {
        return Err("Unsupported public client format, engine or network protocol".into());
    }
    if !hex(&manifest.content_revision) || !hex(&manifest.payload_sha256) {
        return Err("Invalid public release identity".into());
    }
    streaming::validate_manifest(&manifest)?;
    let bytes = entry(&mut zip, PAYLOAD, MAX_PAYLOAD_BYTES)?;
    if manifest.version == 2 && bytes.len() > streaming::MAX_BOOTSTRAP {
        return Err("Streamed bootstrap exceeds size limit".into());
    }
    if format!("{:x}", Sha256::digest(&bytes)) != manifest.payload_sha256 {
        return Err("Public client checksum mismatch".into());
    }
    let mut reader = rmp_serde::Deserializer::new(Cursor::new(&bytes));
    reader.set_max_depth(64);
    let payload = Payload::deserialize(&mut reader).map_err(|e| e.to_string())?;
    if reader.position() != bytes.len() as u64 {
        return Err("Trailing public client data".into());
    }
    payload.validate()?;
    Ok((payload, manifest))
}
