//! Content-addressed public packs. Paths are derived from verified lowercase
//! digests; a manifest cannot nominate arbitrary URLs or private filesystem paths.
pub use super::projection::{PublicAppearance, PublicAvatar, PublicRegion, PublicTexture};
use super::*;
use crate::asset::Asset;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
};
use theframework_core::Uuid;
pub const PACK_TARGET: usize = 4 * 1024 * 1024;
pub const MAX_PACK: usize = 16 * 1024 * 1024;
pub const MAX_GLOBAL: usize = 128 * 1024 * 1024;
pub const MAX_REGIONAL: usize = 128 * 1024 * 1024;
pub const MAX_BOOTSTRAP: usize = 4 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackRef {
    pub sha256: String,
    pub bytes: usize,
    pub decoded_bytes: usize,
    pub map: Option<Uuid>,
}
impl PackRef {
    pub fn path(&self) -> String {
        format!("packs/{}.eldiron-pack", self.sha256)
    }
}
pub struct Bundle {
    pub bootstrap: Vec<u8>,
    pub packs: BTreeMap<String, Vec<u8>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Record {
    Tile(Uuid, rusterix::Tile),
    Material(Uuid, rusterix::Tile),
    Texture(String, PublicTexture),
    RulesMaterial(String, PublicTexture),
    Animation(String, (Vec<PublicTexture>, f32)),
    Icon(String, Vec<PublicTexture>),
    Model(String, String),
    Appearance(String, PublicAppearance),
    Avatar(String, PublicAvatar),
    Asset(Asset),
    Prefab(rusterix::BlockPropAsset),
    Paint(Uuid, crate::iso_paint::IsoPaintLayer),
}
impl Record {
    fn key(&self) -> String {
        match self {
            Self::Tile(id, _) => format!("tile:{id}"),
            Self::Material(id, _) => format!("material:{id}"),
            Self::Texture(id, _) => format!("texture:{id}"),
            Self::RulesMaterial(id, _) => format!("rules-material:{id}"),
            Self::Animation(id, _) => format!("animation:{id}"),
            Self::Icon(id, _) => format!("icon:{id}"),
            Self::Model(id, _) => format!("model:{id}"),
            Self::Appearance(id, _) => format!("appearance:{id}"),
            Self::Avatar(id, _) => format!("avatar:{id}"),
            Self::Asset(a) => format!("asset:{}", a.id),
            Self::Prefab(a) => format!("prefab:{}", a.id),
            Self::Paint(id, _) => format!("paint:{id}"),
        }
    }
    fn install(self, p: &mut Payload) {
        match self {
            Self::Tile(id, v) => {
                p.visuals.tiles.insert(id, v);
            }
            Self::Material(id, v) => {
                p.visuals.materials.insert(id, v);
            }
            Self::Texture(id, v) => {
                p.visuals.textures.insert(id, v);
            }
            Self::RulesMaterial(id, v) => {
                p.visuals.ruleset_materials.insert(id, v);
            }
            Self::Animation(id, v) => {
                p.visuals.animations.insert(id, v);
            }
            Self::Icon(id, v) => {
                p.visuals.item_icons.insert(id, v);
            }
            Self::Model(id, v) => {
                p.visuals.item_models.insert(id, v);
            }
            Self::Appearance(id, v) => {
                p.visuals.appearances.insert(id, v);
            }
            Self::Avatar(id, v) => {
                p.visuals.avatars.insert(id, v);
            }
            Self::Asset(v) => p.presentation.assets.push(v),
            Self::Prefab(v) => p.presentation.prefabs.push(v),
            Self::Paint(id, v) => {
                p.presentation.prefab_paint.insert(id, v);
            }
        }
    }
}
#[derive(Serialize, Deserialize)]
pub enum Pack {
    Global(Vec<Record>),
    Region(PublicRegion),
}
pub fn validate_manifest(m: &Manifest) -> Result<(), String> {
    if m.version == 1 {
        return if m.packs.is_empty() {
            Ok(())
        } else {
            Err("Version-one releases cannot reference packs".into())
        };
    }
    if m.packs.is_empty() || m.packs.len() > 1024 {
        return Err("Invalid public pack count".into());
    }
    let (mut global, mut regional) = (0usize, 0usize);
    let mut hashes = BTreeSet::new();
    let mut maps = BTreeSet::new();
    for p in &m.packs {
        if p.sha256.len() != 64
            || !p
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || p.bytes == 0
            || p.bytes > MAX_PACK
            || p.decoded_bytes == 0
            || p.decoded_bytes > MAX_PACK
            || !hashes.insert(&p.sha256)
        {
            return Err("Invalid/duplicate public pack descriptor".into());
        }
        if let Some(map) = p.map {
            if !maps.insert(map) {
                return Err("Repeated public region pack".into());
            }
            regional = regional.saturating_add(p.decoded_bytes);
        } else {
            global = global.saturating_add(p.decoded_bytes);
        }
    }
    if global > MAX_GLOBAL || regional > MAX_REGIONAL {
        return Err("Public release asset budget exceeded".into());
    }
    Ok(())
}
fn encode_pack(pack: &Pack, map: Option<Uuid>) -> Result<(PackRef, Vec<u8>), String> {
    let raw = rmp_serde::to_vec_named(pack).map_err(|e| e.to_string())?;
    if raw.len() > MAX_PACK {
        return Err("An individual public asset/region exceeds the 16 MiB pack limit".into());
    }
    let mut writer = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    writer.write_all(&raw).map_err(|e| e.to_string())?;
    let bytes = writer.finish().map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PACK {
        return Err("Public pack exceeds encoded limit".into());
    }
    Ok((
        PackRef {
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len(),
            decoded_bytes: raw.len(),
            map,
        },
        bytes,
    ))
}
pub fn decode_pack(reference: &PackRef, bytes: &[u8]) -> Result<Pack, String> {
    if bytes.len() != reference.bytes
        || bytes.len() > MAX_PACK
        || reference.decoded_bytes > MAX_PACK
        || format!("{:x}", Sha256::digest(bytes)) != reference.sha256
    {
        return Err("Public asset pack size/checksum mismatch".into());
    }
    let mut reader = flate2::read::DeflateDecoder::new(bytes);
    let mut raw = Vec::new();
    reader
        .by_ref()
        .take(reference.decoded_bytes as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() != reference.decoded_bytes || reader.total_in() != bytes.len() as u64 {
        return Err("Public asset pack decompression bounds mismatch".into());
    }
    let mut decoder = rmp_serde::Deserializer::new(std::io::Cursor::new(&raw));
    decoder.set_max_depth(64);
    let pack = Pack::deserialize(&mut decoder).map_err(|e| e.to_string())?;
    if decoder.position() != raw.len() as u64 {
        return Err("Trailing public pack data".into());
    }
    match &pack {
        Pack::Global(_) if reference.map.is_none() => {}
        Pack::Region(r) if reference.map == Some(r.layout.geometry.id) => {
            r.layout.map(&r.name)?;
            if super::projection::policy::region_config(&r.config)? != r.config {
                return Err("Private region configuration in pack".into());
            }
        }
        _ => return Err("Public asset pack kind/region mismatch".into()),
    };
    Ok(pack)
}
/// Stable collection ordering and fixed raw-byte targets let unchanged packs be
/// reused across deployments. Global art is required; region art is demand-loaded.
pub fn partition(mut p: Payload, revision: String, protocol: u16) -> Result<Bundle, String> {
    p.validate()?;
    let mut records = Vec::new();
    macro_rules! drain {
        ($field:ident,$kind:ident) => {
            for (id, v) in std::mem::take(&mut p.visuals.$field) {
                records.push(Record::$kind(id, v));
            }
        };
    }
    drain!(tiles, Tile);
    drain!(materials, Material);
    drain!(textures, Texture);
    drain!(ruleset_materials, RulesMaterial);
    drain!(animations, Animation);
    drain!(item_icons, Icon);
    drain!(item_models, Model);
    drain!(appearances, Appearance);
    drain!(avatars, Avatar);
    records.extend(
        std::mem::take(&mut p.presentation.assets)
            .into_iter()
            .map(Record::Asset),
    );
    records.extend(
        std::mem::take(&mut p.presentation.prefabs)
            .into_iter()
            .map(Record::Prefab),
    );
    records.extend(
        std::mem::take(&mut p.presentation.prefab_paint)
            .into_iter()
            .map(|(id, v)| Record::Paint(id, v)),
    );
    records.sort_by_key(Record::key);
    let (mut refs, mut packs) = (Vec::new(), BTreeMap::new());
    let mut publish = |pack: Pack, map| -> Result<(), String> {
        let (reference, bytes) = encode_pack(&pack, map)?;
        packs.insert(reference.path(), bytes);
        refs.push(reference);
        Ok(())
    };
    let (mut batch, mut size) = (Vec::new(), 0usize);
    for record in records {
        let next = rmp_serde::to_vec_named(&record)
            .map_err(|e| e.to_string())?
            .len();
        if !batch.is_empty() && size + next > PACK_TARGET {
            publish(Pack::Global(std::mem::take(&mut batch)), None)?;
            size = 0;
        }
        size += next;
        batch.push(record);
    }
    if !batch.is_empty() {
        publish(Pack::Global(batch), None)?;
    }
    for region in &mut p.presentation.regions {
        let mut empty = rusterix::Map::default();
        empty.id = region.layout.geometry.id;
        empty.name = region.name.clone();
        empty.camera = region.layout.geometry.camera;
        empty.grid_size = region.layout.geometry.grid_size;
        empty.subdivisions = region.layout.geometry.subdivisions;
        let placeholder = PublicRegion {
            id: region.id,
            name: region.name.clone(),
            config: region.config.clone(),
            layout: super::projection::PublicLayout {
                geometry: crate::public_geometry::RegionGeometry::capture(&empty)?,
                widgets: Default::default(),
                screen_style: None,
                widget_names: Default::default(),
                orthographic_bake: None,
            },
            paint: Default::default(),
        };
        let full = std::mem::replace(region, placeholder);
        let map = full.layout.geometry.id;
        publish(Pack::Region(full), Some(map))?;
    }
    let bootstrap = super::archive::encode_with_packs(&p, revision, protocol, refs)?;
    if bootstrap.len() > MAX_BOOTSTRAP {
        return Err("Public bootstrap exceeds 4 MiB".into());
    }
    let (_, manifest) = super::archive::decode_payload(&bootstrap, protocol)?;
    validate_manifest(&manifest)?;
    Ok(Bundle { bootstrap, packs })
}
pub struct Assembly {
    payload: Payload,
    manifest: Manifest,
    pending: BTreeSet<String>,
    seen: BTreeSet<String>,
}
impl Assembly {
    pub fn new(bytes: &[u8], protocol: u16) -> Result<Self, String> {
        let (payload, manifest) = super::archive::decode_payload(bytes, protocol)?;
        if manifest.version != 2 {
            return Err("Not a streamed public release".into());
        }
        if !payload.presentation.assets.is_empty()
            || !payload.presentation.prefabs.is_empty()
            || !payload.visuals.tiles.is_empty()
        {
            return Err("Streamed bootstrap must not embed global assets".into());
        }
        let catalog: BTreeSet<_> = payload
            .presentation
            .regions
            .iter()
            .map(|r| r.layout.geometry.id)
            .collect();
        let referenced: BTreeSet<_> = manifest.packs.iter().filter_map(|r| r.map).collect();
        if catalog.len() != payload.presentation.regions.len() || catalog != referenced {
            return Err("Public region catalog and pack index differ".into());
        }
        let v = &payload.visuals;
        if !payload.presentation.prefab_paint.is_empty()
            || !v.materials.is_empty()
            || !v.textures.is_empty()
            || !v.ruleset_materials.is_empty()
            || !v.animations.is_empty()
            || !v.item_icons.is_empty()
            || !v.item_models.is_empty()
            || !v.appearances.is_empty()
            || !v.avatars.is_empty()
        {
            return Err("Streamed bootstrap embeds unindexed global artwork".into());
        }
        let pending = manifest
            .packs
            .iter()
            .filter(|p| p.map.is_none())
            .map(|p| p.sha256.clone())
            .collect();
        Ok(Self {
            payload,
            manifest,
            pending,
            seen: BTreeSet::new(),
        })
    }
    pub fn globals(&self) -> Vec<PackRef> {
        self.manifest
            .packs
            .iter()
            .filter(|p| p.map.is_none())
            .cloned()
            .collect()
    }
    pub fn accept(&mut self, reference: &PackRef, bytes: &[u8]) -> Result<(), String> {
        if !self.manifest.packs.contains(reference) || !self.pending.contains(&reference.sha256) {
            return Err("Unexpected/repeated public global pack".into());
        }
        let Pack::Global(records) = decode_pack(reference, bytes)? else {
            return Err("Expected global assets".into());
        };
        if records.len() > 32768 {
            return Err("Public pack record budget exceeded".into());
        }
        // Stage atomically: failure cannot partially change this assembly.
        let mut seen = self.seen.clone();
        for record in &records {
            if !seen.insert(record.key()) {
                return Err("Duplicate public asset identity across packs".into());
            }
        }
        for record in records {
            record.install(&mut self.payload);
        }
        self.seen = seen;
        self.pending.remove(&reference.sha256);
        Ok(())
    }
    pub fn finish(self, source: String) -> Result<Project, String> {
        if !self.pending.is_empty() {
            return Err("Required global assets are missing".into());
        }
        let mut project = self.payload.into_project(self.manifest)?;
        let release = Arc::make_mut(project.client_release.as_mut().unwrap());
        release.globals_ready = true;
        release.stream_source = Some(source);
        Ok(project)
    }
}
pub fn install_region(
    project: &mut Project,
    assets: &mut rusterix::Assets,
    region: PublicRegion,
) -> Result<(), String> {
    let release = project
        .client_release
        .as_ref()
        .ok_or("Missing public release")?;
    if !release
        .manifest
        .packs
        .iter()
        .any(|p| p.map == Some(region.layout.geometry.id))
    {
        return Err("Region is outside public release".into());
    }
    let index = project
        .regions
        .iter()
        .position(|r| {
            r.map.id == region.layout.geometry.id
                && r.id == region.id
                && r.name == region.name
                && r.config == region.config
        })
        .ok_or("Public region catalog identity mismatch")?;
    let restored = region.into_region()?;
    project.regions[index] = restored;
    assets.mark_visual_sources_changed();
    let r = &mut project.regions[index];
    assets.maps.insert(r.name.clone(), r.map.clone());
    crate::iso_paint_terrain::sync(assets, r.map.id, &mut r.iso_paint);
    crate::surface_paint::sync(assets, r.map.id, &mut r.iso_paint)?;
    crate::stamp_paint::sync(assets, r.map.id, &mut r.iso_paint);
    Ok(())
}
