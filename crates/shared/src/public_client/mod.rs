//! Versioned, display-only public releases. Authoring documents are never the
//! public payload; the loader installs a non-serializable remote-only identity.
mod archive;
mod projection;
pub mod streaming;
use crate::project::Project;
pub use archive::{decode, encode, is_public_archive};
pub use projection::{PublicPresentation, PublicVisuals};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub const FORMAT: &str = "eldiron.client";
pub const VERSION: u32 = 1;
pub const EXTENSION: &str = "eldiron-client";
pub const MAX_PAYLOAD_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_ARCHIVE_BYTES: usize = 128 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub engine: String,
    pub protocol: u16,
    pub content_revision: String,
    pub payload_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packs: Vec<streaming::PackRef>,
}
#[derive(Clone, Debug)]
pub struct ClientRelease {
    pub manifest: Manifest,
    pub visuals: PublicVisuals,
    pub stream_source: Option<String>,
    pub globals_ready: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub presentation: PublicPresentation,
    pub visuals: PublicVisuals,
}
/// Capture operates on an owned copy. It prepares artwork, never a world/VM.
/// The networking boundary supplies its canonical, deployment-neutral identity.
pub fn capture(
    project: &Project,
    content_revision: String,
    protocol: u16,
) -> Result<Vec<u8>, String> {
    if project.client_release.is_some() {
        return Err("A public client cannot export an authored release".into());
    }
    let payload = capture_payload(project)?;
    encode(&payload, content_revision, protocol)
}
pub fn capture_payload(project: &Project) -> Result<Payload, String> {
    if project.client_release.is_some() {
        return Err("A public client cannot export authored content".into());
    }
    let mut source = project.clone();
    source.migrate_default_ruleset();
    crate::project_play::ProjectPlaySettings::parse(&source.config)?
        .launch(crate::project_play::LaunchPurpose::Player)?;
    if crate::project_play::ProjectPlaySettings::parse(&source.config)?.mode
        != crate::project_play::ProjectMode::Multiplayer
    {
        return Err("Public client export requires a multiplayer project".into());
    }
    let mut assets = rusterix::Assets::default();
    crate::runtime_assets::prepare_client_content(&mut assets, &mut source, false)?;
    let mut presentation = PublicPresentation::capture(&source)?;
    presentation.locales = assets.locales_src.clone();
    let payload = Payload {
        presentation,
        visuals: PublicVisuals::capture(&assets)?,
    };
    payload.validate()?;
    Ok(payload)
}
impl Payload {
    pub fn validate(&self) -> Result<(), String> {
        self.presentation.validate()?;
        self.visuals.validate()
    }
    pub fn into_project(self, manifest: Manifest) -> Result<Project, String> {
        self.validate()?;
        let mut project = self.presentation.into_project()?;
        project.client_release = Some(Arc::new(ClientRelease {
            globals_ready: manifest.packs.iter().all(|p| p.map.is_some()),
            manifest,
            visuals: self.visuals,
            stream_source: None,
        }));
        Ok(project)
    }
}
/// A Player accepts both authored projects (including single player) and public
/// releases. Creator continues using project_io, which rejects client archives.
pub fn load_player(bytes: &[u8], protocol: u16) -> Result<Project, String> {
    if is_public_archive(bytes)? {
        decode(bytes, protocol)
    } else {
        crate::project_io::decode_project(bytes)
    }
}
/// Populate renderer inputs without migrations, template scripts, behavior
/// graphs, source maps or authored instance creation.
pub fn prepare(assets: &mut rusterix::Assets, project: &mut Project) -> Result<(), String> {
    let release = project
        .client_release
        .clone()
        .ok_or("Missing public release identity")?;
    if !release.globals_ready || (release.manifest.version == 2 && release.stream_source.is_none())
    {
        return Err("Streamed client requires verified global packs before startup".into());
    }
    assets.config = project.config.clone();
    assets.palette = project.palette.clone();
    assets.world_source.clear();
    assets.authoring_src.clear();
    assets.region_sources.clear();
    assets.node_behaviors = None;
    assets.item_template_ids = project
        .items
        .values()
        .map(|i| (i.id, i.name.clone()))
        .collect();
    assets.item_wearables.clear();
    for item in project.items.values() {
        let mut runtime_item = rusterix::Item::default();
        rusterix::server::data::apply_item_data(&mut runtime_item, &item.data);
        let worn = rusterix::procedural_avatar::Wearable::from_item(&runtime_item)?;
        let mut keys = vec![item.id.to_string(), item.name.clone()];
        for key in ["ruleset_path", "ruleset_id"] {
            if let Some(v) = runtime_item.attributes.get_str(key) {
                keys.push(v.into());
            }
        }
        for key in keys {
            assets
                .item_wearables
                .insert(key.trim().to_ascii_lowercase(), worn.clone());
        }
    }
    assets.rules = release.visuals.rules.clone();
    assets.read_rules_metadata();
    assets.locales_src = project.locales.clone();
    assets.read_locales();
    crate::runtime_assets::sync_audio_sources(assets, project);
    assets.set_block_props(project.block_props.clone());
    crate::runtime_assets::sync_display_templates(assets, project, false);
    assets.maps = project
        .regions
        .iter()
        .map(|r| (r.name.clone(), r.map.clone()))
        .collect();
    release.visuals.install(assets)?;
    assets.terrain_paint.clear();
    assets.surface_paint.clear();
    assets.stamp_paint.clear();
    for region in &mut project.regions {
        crate::iso_paint_terrain::sync(assets, region.map.id, &mut region.iso_paint);
        crate::surface_paint::sync(assets, region.map.id, &mut region.iso_paint)?;
        crate::stamp_paint::sync(assets, region.map.id, &mut region.iso_paint);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
