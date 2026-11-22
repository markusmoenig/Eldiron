//! Editable ruleset MZ library. Internal swatches adapt the existing material editor;
//! geometry and runtime use stable ruleset URIs rather than those swatch tiles.
use crate::project::Project;
use rusterix::{Texture, Tile};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use theframework_core::prelude::Uuid;

pub fn definitions(config: &str) -> Vec<crate::rulesets::BundledMaterialAsset> {
    let s = crate::rulesets::selected_ruleset_config(config);
    if s.source == "project" {
        return vec![];
    }
    let mut defs = crate::rulesets::bundled_material_assets_for_ruleset(&s.id, &s.version);
    defs.extend(crate::rulesets::bundled_icon_material_assets_for_ruleset(
        &s.id, &s.version,
    ));
    defs
}
pub fn definition(config: &str, id: Uuid) -> Option<crate::rulesets::BundledMaterialAsset> {
    if id.as_u128() >> 64 != 0x52554c454d415453 {
        return None;
    }
    definitions(config)
        .into_iter()
        .find(|def| crate::rulesets::material_asset_id(def.id) == id)
}

pub fn editable_id(project: &Project, id: Uuid) -> Result<rusterix::ruleset_material::EditableAsset, String> {
    if let Some(def) = definition(&project.config, id) {
        return rusterix::ruleset_material::editable(&def, &project.palette);
    }
    let source = project.procedural_materials.get(&id).ok_or("Material is missing")?;
    rusterix::ruleset_material::EditableAsset::reopen(source).map_err(|e| e.to_string())
}

pub fn is_library_tile(_config: &str, id: &Uuid) -> bool {
    matches!(id.as_u128() >> 64, 0x4d5a49434f4e4544 | 0x52554c454d415453)
}
pub fn new_icon_id() -> Uuid {
    Uuid::from_u128((0x4d5a49434f4e4544u128 << 64) | (Uuid::new_v4().as_u128() & u64::MAX as u128))
}
pub fn is_icon(config: &str, id: &Uuid) -> bool {
    id.as_u128() >> 64 == 0x4d5a49434f4e4544
        || definitions(config)
            .iter()
            .any(|d| d.id.starts_with("icon/") && crate::rulesets::material_asset_id(d.id) == *id)
}
pub fn resolution(config: &str, id: &Uuid) -> Option<u32> {
    if id.as_u128() >> 64 == 0x4d5a49434f4e4544 {
        return Some(32);
    }
    definitions(config)
        .iter()
        .find(|d| crate::rulesets::material_asset_id(d.id) == *id)
        .map(|d| d.resolution)
}
pub fn icon_id(data: &str) -> Option<Uuid> {
    icon_id_for_state(data, true)
}
/// Explicit mesh-part bindings, shared by the item browser and runtime authoring.
pub fn worn_materials(data: &str) -> std::collections::BTreeMap<String, String> {
    let Ok(root) = data.parse::<toml::Table>() else {
        return Default::default();
    };
    root.get("attributes")
        .and_then(toml::Value::as_table)
        .and_then(|attrs| attrs.get("appearance_materials"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .filter_map(|entry| entry.split_once('='))
        .map(|(part, source)| (part.trim().into(), source.trim().into()))
        .collect()
}
pub fn worn_geometry_id(data: &str) -> Option<Uuid> {
    let root = data.parse::<toml::Table>().ok()?;
    let source = root
        .get("attributes")?
        .get("appearance_worn_geometry")?
        .as_str()?;
    if source.trim().is_empty() {
        return None;
    }
    crate::appearance_assets::asset_id(source)
        .or_else(|_| crate::appearance_assets::asset_id(&format!("ruleset:geometry/{source}")))
        .ok()
}
pub fn worn_material_sources(project: &Project, data: &str) -> std::collections::BTreeSet<String> {
    use denrim_materials::geometry::{Document, Operation, WEARER_SOURCE};
    let mut sources = worn_materials(data)
        .into_values()
        .collect::<std::collections::BTreeSet<_>>();
    fn visit(
        doc: &Document,
        project: &Project,
        sources: &mut std::collections::BTreeSet<String>,
        depth: usize,
    ) {
        if depth > 6 {
            return;
        }
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            match &tile.op {
                Operation::Texture { source, .. } => {
                    sources.insert(source.clone());
                }
                Operation::Group { document, .. } => visit(document, project, sources, depth + 1),
                Operation::Asset { source, document } if source != WEARER_SOURCE => {
                    let current = crate::appearance_assets::asset_id(source)
                        .ok()
                        .and_then(|id| crate::appearance_assets::source(project, &id))
                        .and_then(|text| serde_json::from_str::<Document>(text).ok());
                    visit(
                        current.as_ref().unwrap_or(document),
                        project,
                        sources,
                        depth + 1,
                    );
                }
                _ => {}
            }
        }
    }
    if let Some(doc) = worn_geometry_id(data)
        .and_then(|id| crate::appearance_assets::source(project, &id))
        .and_then(|text| serde_json::from_str::<Document>(text).ok())
    {
        visit(&doc, project, &mut sources, 0);
    }
    sources
}

pub fn icon_id_for_state(data: &str, on: bool) -> Option<Uuid> {
    let data = data.parse::<toml::Table>().ok()?;
    let key = if on {
        "appearance_icon"
    } else {
        "appearance_icon_off"
    };
    let value = data
        .get("attributes")
        .and_then(toml::Value::as_table)
        .and_then(|attrs| attrs.get(key))
        .or_else(|| data.get(key))?
        .as_str()?;
    Some(if value.starts_with("icon/") {
        crate::rulesets::material_asset_id(value)
    } else {
        Uuid::parse_str(value).ok()?
    })
}
pub fn editable(
    project: &Project,
    name: &str,
) -> Result<rusterix::ruleset_material::EditableAsset, String> {
    let def = definitions(&project.config)
        .into_iter()
        .find(|d| d.id == name)
        .ok_or("Ruleset material is missing")?;
    rusterix::ruleset_material::editable(&def, &project.palette)
}

pub fn baked(project: &Project, name: &str) -> Result<Texture, String> {
    let def = definitions(&project.config)
        .into_iter()
        .find(|d| d.id == name)
        .ok_or("Ruleset material is missing")?;
    if let Some(frames) = saved_frames(project, &def) {
        return Ok(frames[0].clone());
    }
    let asset = editable(project, name)?;
    bake_asset(&asset, &project.palette, def.resolution)
}

/// The same animation frames feed inventory, editor previews and state tiles.
pub fn baked_frames(project: &Project, name: &str) -> Result<Vec<Texture>, String> {
    let def = definitions(&project.config)
        .into_iter()
        .find(|d| d.id == name)
        .ok_or("Ruleset material is missing")?;
    if let Some(frames) = saved_frames(project, &def) {
        return Ok(frames);
    }
    // Reopening an editable bundle compiles its dependency library. Cache above
    // that operation, rather than only caching the final per-frame evaluator.
    static CACHE: OnceLock<Mutex<HashMap<u64, Vec<Texture>>>> = OnceLock::new();
    let key = rusterix::procedural_avatar::fingerprint(&(
        def.id,
        def.source,
        def.palette,
        def.output,
        def.resolution,
        serde_json::to_string(&project.palette.colors).unwrap_or_default(),
    ));
    let cache = CACHE.get_or_init(Default::default);
    if let Some(frames) = cache.lock().unwrap().get(&key) {
        return Ok(frames.clone());
    }
    let frames = animation_frames(&editable(project, name)?, &project.palette, def.resolution)?;
    let mut cache = cache.lock().unwrap();
    if cache.len() >= 256 {
        if let Some(key) = cache.keys().next().copied() {
            cache.remove(&key);
        }
    }
    cache.insert(key, frames.clone());
    Ok(frames)
}

/// Read transport metadata without compiling the editable material library.
pub fn animation_settings(bundle: &str) -> Option<denrim_materials::format::Animation> {
    let data: serde_json::Value = serde_json::from_str(bundle).ok()?;
    let selected = data.get("selected_material")?.as_str()?;
    let recipe = data.pointer("/root/recipe")?.as_str()?;
    denrim_materials::format::parse(recipe)
        .ok()?
        .materials
        .into_iter()
        .find(|m| m.id == selected)
        .map(|m| m.animation)
}

pub fn icon_state_tiles(project: &Project) -> Result<HashMap<Uuid, Vec<Texture>>, String> {
    let selection = crate::rulesets::selected_ruleset_config(&project.config);
    if selection.source == "project" {
        return Ok(HashMap::new());
    }
    let definitions = definitions(&project.config);
    let mut tiles = HashMap::new();
    for state in
        crate::rulesets::bundled_icon_state_assets_for_ruleset(&selection.id, &selection.version)
    {
        let Some(id) = state.tile_id else { continue };
        let name = format!("icon/{}/{}", state.item_id, state.state);
        if definitions.iter().any(|d| d.id == name) {
            tiles.insert(
                Uuid::parse_str(id).map_err(|e| e.to_string())?,
                baked_frames(project, &name)?,
            );
        }
    }
    Ok(tiles)
}

/// Share resolved maps with the geometry worker without moving the whole project
/// or a GPU device into the runtime asset loader.
pub fn bake_asset(
    asset: &rusterix::ruleset_material::EditableAsset,
    palette: &theframework_core::prelude::ThePalette,
    resolution: u32,
) -> Result<Texture, String> {
    bake_at_time(asset, palette, resolution, 0.)
}
pub fn frames_for_item(
    project: &Project,
    item: &crate::item::Item,
    on: bool,
) -> Result<Option<Vec<Texture>>, String> {
    let Some(id) = icon_id_for_state(&item.data, on) else {
        return Ok(None);
    };
    if let Some(def) = definitions(&project.config)
        .into_iter()
        .find(|d| d.id.starts_with("icon/") && crate::rulesets::material_asset_id(d.id) == id)
    {
        if !item.icon_frames_for_state(on).is_empty() {
            return Ok(None);
        }
        // The inventory/map icon lookup must share the persisted library bake,
        // rather than rebuilding the same recipes during project activation.
        return baked_frames(project, def.id).map(Some);
    }
    let bundle = project
        .procedural_materials
        .get(&id)
        .ok_or("Missing procedural item icon")?;
    let asset =
        rusterix::ruleset_material::EditableAsset::reopen(bundle).map_err(|e| e.to_string())?;
    animation_frames(&asset, &project.palette, 32).map(Some)
}

fn animation_frames(
    asset: &rusterix::ruleset_material::EditableAsset,
    palette: &theframework_core::prelude::ThePalette,
    resolution: u32,
) -> Result<Vec<Texture>, String> {
    let frames = asset
        .root
        .project
        .materials
        .iter()
        .find(|m| m.id == asset.selected_material)
        .ok_or("Missing icon output")?
        .animation
        .frames;
    (0..frames)
        .map(|phase| bake_at_time(asset, palette, resolution, phase as f32 / frames as f32))
        .collect::<Result<Vec<_>, _>>()
}
pub(crate) fn bake_at_time(
    asset: &rusterix::ruleset_material::EditableAsset,
    palette: &theframework_core::prelude::ThePalette,
    resolution: u32,
    time: f32,
) -> Result<Texture, String> {
    static CACHE: OnceLock<Mutex<HashMap<String, Texture>>> = OnceLock::new();
    let key = format!(
        "{}:{}:{}:{}",
        time.to_bits(),
        resolution,
        asset.save().map_err(|e| e.to_string())?,
        serde_json::to_string(&palette.colors).map_err(|e| e.to_string())?
    );
    let cache = CACHE.get_or_init(Default::default);
    if let Some(texture) = cache.lock().unwrap().get(&key) {
        return Ok(texture.clone());
    }
    let texture = rusterix::ruleset_material::bake_at_time(asset, palette, resolution, time)?;
    let mut cache = cache.lock().unwrap();
    // The complete official surface/icon library exceeds 64 entries. Keep a
    // palette's frames resident instead of evicting them during every sync.
    if cache.len() >= 256 {
        cache.clear();
    }
    cache.insert(key, texture.clone());
    Ok(texture)
}
// Saved bakes survive process restarts. Content, palette and resolution own the
// key; the application release does not change a material's pixels.
fn saved_key(bundle: &str, palette: &theframework_core::prelude::ThePalette, resolution: u32) -> u64 {
    let mut document: serde_json::Value =
        serde_json::from_str(bundle).unwrap_or_else(|_| serde_json::Value::String(bundle.into()));
    if let Some(version) = document.get_mut("source_library") {
        if let Some(fingerprint) = version.as_str().and_then(|s| s.rsplit('-').next()) {
            *version = serde_json::Value::String(fingerprint.to_owned());
        }
    }
    format!(
        "ruleset-bake-v1:{resolution}:{}:{}",
        document,
        serde_json::to_string(&palette.colors).unwrap()
    )
    .bytes()
    .fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
fn canonical_key(def: &crate::rulesets::BundledMaterialAsset, project: &Project) -> u64 {
    format!("ruleset-bake-v2:{}:{}", definition_key(def),
        serde_json::to_string(&project.palette.colors).unwrap_or_default())
        .bytes()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
}

fn saved_tile<'a>(project: &'a Project, def: &crate::rulesets::BundledMaterialAsset) -> Option<&'a Tile> {
    let id = crate::rulesets::material_asset_id(def.id);
    if project.procedural_material_cache_keys.get(&id) != Some(&canonical_key(def, project)) {
        return None;
    }
    sized_tile(project, id, def.resolution)
}

fn sized_tile(project: &Project, id: Uuid, resolution: u32) -> Option<&Tile> {
    let tile = project.tiles.get(&id)?;
    if tile.textures.is_empty()
        || tile.textures.len() != tile.baked_material_data.len()
        || tile
            .textures
            .iter()
            .zip(&tile.baked_material_data)
            .any(|(t, bytes)| {
                t.width != resolution as usize
                    || t.height != resolution as usize
                    || bytes.len() != t.width * t.height * 2
            })
    {
        return None;
    }
    Some(tile)
}

pub fn has_saved_library_bakes(project: &Project) -> bool {
    definitions(&project.config)
        .iter()
        .all(|def| saved_tile(project, def).is_some())
}

fn saved_frames(project: &Project, def: &crate::rulesets::BundledMaterialAsset) -> Option<Vec<Texture>> {
    let tile = saved_tile(project, def)?;
    let mut frames = tile.textures.clone();
    for (texture, bytes) in frames.iter_mut().zip(&tile.baked_material_data) {
        texture.apply_material_bytes(bytes);
    }
    Some(frames)
}
/// Install only content-matching derived defaults. This never overwrites authored
/// overrides; stale palette, definition or resolution keys fall back to baking.
fn definition_key(def: &crate::rulesets::BundledMaterialAsset) -> String {
    let library = denrim_materials::integration::source_library_version();
    let text = format!(
        "eldiron-default-v1:{}:{}:{}:{}:{}:{}",
        def.id,
        def.source,
        def.palette,
        def.output,
        def.resolution,
        library.rsplit('-').next().unwrap_or(&library)
    );
    format!(
        "{:016x}",
        text.bytes()
            .fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b))
                .wrapping_mul(0x100000001b3))
    )
}
/// Maintainer metadata for an already validated default-bake archive. It lets
/// normal startup compare source bytes without compiling 98 editable recipes.
pub fn stamp_seed_sources(cache: &mut Project) -> Result<(), String> {
    let mut config = cache
        .config
        .parse::<toml::Table>()
        .map_err(|e| e.to_string())?;
    let mut sources = toml::Table::new();
    for def in definitions(&cache.config) {
        sources.insert(def.id.into(), toml::Value::String(definition_key(&def)));
    }
    config.insert(
        "embedded_material_bakes".into(),
        toml::Value::Table(sources),
    );
    cache.config = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn seed_bakes(project: &mut Project, cache: &Project) -> Result<usize, String> {
    if project.palette.colors != cache.palette.colors {
        return Ok(0);
    }
    let manifest = cache
        .config
        .parse::<toml::Table>()
        .ok()
        .and_then(|mut table| table.remove("embedded_material_bakes"))
        .and_then(|v| v.as_table().cloned())
        .unwrap_or_default();
    let mut reused = 0;
    for def in definitions(&project.config) {
        let id = crate::rulesets::material_asset_id(def.id);
        if manifest.get(def.id).and_then(toml::Value::as_str)
            != Some(definition_key(&def).as_str())
        {
            continue;
        }
        let cached = saved_tile(cache, &def).or_else(|| {
            let bundle = cache.procedural_materials.get(&id)?;
            let key = saved_key(bundle, &cache.palette, def.resolution);
            (cache.procedural_material_cache_keys.get(&id) == Some(&key))
                .then(|| sized_tile(cache, id, def.resolution)).flatten()
        });
        if let Some(cached) = cached {
            let mut tile = project
                .tiles
                .get(&id)
                .cloned()
                .unwrap_or_else(|| cached.clone());
            tile.textures = cached.textures.clone();
            tile.baked_material_data = cached.baked_material_data.clone();
            tile.uv_pages = cached.uv_pages.clone();
            project.tiles.insert(id, tile);
            project.procedural_material_cache_keys.insert(id, canonical_key(&def, project));
            reused += 1;
        }
    }
    Ok(reused)
}

/// Promote only legacy tiles whose bake key matches the current shared recipe.
pub(crate) fn validate_legacy_bakes(project: &mut Project) {
    if project.procedural_materials.is_empty() {
        return;
    }
    for def in definitions(&project.config) {
        let id = crate::rulesets::material_asset_id(def.id);
        if !project.procedural_materials.contains_key(&id)
            || saved_tile(project, &def).is_some()
            || sized_tile(project, id, def.resolution).is_none()
        {
            continue;
        }
        let Some(key) = project.procedural_material_cache_keys.get(&id) else {
            continue;
        };
        let Ok(bundle) = rusterix::ruleset_material::editable(&def, &project.palette)
            .and_then(|asset| asset.save().map_err(|error| error.to_string()))
        else {
            continue;
        };
        if *key == saved_key(&bundle, &project.palette, def.resolution) {
            project.procedural_material_cache_keys.insert(id, canonical_key(&def, project));
        }
    }
}

pub fn install(project: &mut Project) -> Result<(), String> {
    validate_legacy_bakes(project);
    let defs = definitions(&project.config);
    install_definitions(project, &defs)?;
    for def in defs {
        let id = crate::rulesets::material_asset_id(def.id);
        if project.procedural_materials.contains_key(&id) {
            project.procedural_materials.shift_remove(&id);
        }
    }
    Ok(())
}
fn install_definitions(
    project: &mut Project,
    definitions: &[crate::rulesets::BundledMaterialAsset],
) -> Result<(), String> {
    let bake = |def: &crate::rulesets::BundledMaterialAsset| -> Result<_, String> {
        let id = crate::rulesets::material_asset_id(def.id);
        if saved_tile(project, def).is_some() {
            return Ok(None);
        }
        let frames = baked_frames(project, def.id)?;
        let mut tile = project
            .tiles
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Tile::from_texture(frames[0].clone()));
        tile.id = id;
        tile.alias = format!("Ruleset · {}", def.id);
        tile.baked_material_data = frames.iter().map(Texture::material_bytes).collect();
        tile.textures = frames;
        // Match Texture's persisted representation to avoid false changes on reopen.
        for texture in &mut tile.textures {
            texture.set_materials_all(0.5, 0., 1., 0.);
        }
        Ok(Some((id, tile, canonical_key(def, project))))
    };
    // Independent MZ recipes can bake concurrently. Collect in library order and
    // publish only after all workers finish, so picker order stays stable and
    // no partially baked library is exposed. Browser builds keep the serial path.
    #[cfg(not(target_arch = "wasm32"))]
    let baked: Result<Vec<_>, String> = {
        use rayon::prelude::*;
        definitions.par_iter().map(bake).collect()
    };
    #[cfg(target_arch = "wasm32")]
    let baked: Result<Vec<_>, String> = definitions.iter().map(bake).collect();
    for (id, tile, key) in baked?.into_iter().flatten() {
        project.tiles.insert(id, tile);
        project.procedural_material_cache_keys.insert(id, key);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_library_bakes_match_serial_pixels_materials_and_insertion_order() {
        let mut project = Project::default();
        let definitions: Vec<_> = definitions(&project.config)
            .into_iter()
            .filter(|d| matches!(d.id, "tunic" | "leather"))
            .collect();
        assert_eq!(definitions.len(), 2);
        // Use a custom palette, as an old project may not match the embedded cache.
        project.palette.colors[0] = Some(theframework_core::prelude::TheColor::from_hex("#8b6254"));
        let expected: Vec<_> = definitions
            .iter()
            .map(|def| {
                let asset = rusterix::ruleset_material::editable(def, &project.palette).unwrap();
                let texture =
                    rusterix::ruleset_material::bake(&asset, &project.palette, def.resolution)
                        .unwrap();
                (crate::rulesets::material_asset_id(def.id), texture)
            })
            .collect();
        install_definitions(&mut project, &definitions).unwrap();
        for (def, (id, texture)) in definitions.iter().zip(&expected) {
            let actual = saved_frames(&project, def).unwrap();
            assert_eq!(actual[0].data, texture.data);
            assert_eq!(actual[0].material_bytes(), texture.material_bytes());
            assert_eq!(project.tiles[id].id, *id);
        }
        let order: Vec<_> = project
            .tiles
            .keys()
            .filter(|id| expected.iter().any(|(e, _)| e == *id))
            .copied()
            .collect();
        assert_eq!(
            order,
            expected.iter().map(|(id, _)| *id).collect::<Vec<_>>()
        );
        let tiles = project.tiles.clone();
        let keys = project.procedural_material_cache_keys.clone();
        install_definitions(&mut project, &definitions).unwrap();
        assert_eq!(tiles, project.tiles);
        assert_eq!(keys, project.procedural_material_cache_keys);
    }

    #[test]
    fn saved_library_bakes_survive_reopen_and_reject_palette_or_recipe_changes() {
        let mut project = Project::default();
        let id = crate::rulesets::material_asset_id("tunic");
        let def = definition(&project.config, id).unwrap();
        install_definitions(&mut project, std::slice::from_ref(&def)).unwrap();
        let expected = saved_frames(&project, &def).unwrap()[0].material_bytes();
        let bytes = crate::project_io::encode_project(&project).unwrap();
        let mut reopened = crate::project_io::decode_project(&bytes).unwrap();
        assert_eq!(saved_frames(&reopened, &def).unwrap()[0].material_bytes(), expected);
        assert!(!reopened.procedural_materials.contains_key(&id));

        let mut changed = def.clone();
        changed.source = "Updated material recipe";
        assert!(saved_frames(&reopened, &changed).is_none());
        reopened.palette.colors[0] = reopened.palette.colors[1].clone();
        assert!(saved_frames(&reopened, &def).is_none());
    }

    #[test]
    fn legacy_library_materials_do_not_override_the_shared_recipe() {
        let mut project = Project::default();
        let id = crate::rulesets::material_asset_id("tunic");
        let expected = baked(&project, "tunic").unwrap();
        project.procedural_materials.insert(id, "stale bundle".into());
        assert_eq!(baked(&project, "tunic").unwrap(), expected);
        let bytes = crate::project_io::encode_project(&project).unwrap();
        let reopened = crate::project_io::decode_project(&bytes).unwrap();
        assert!(!reopened.procedural_materials.contains_key(&id));
        assert_eq!(baked(&reopened, "tunic").unwrap(), expected);
    }
}
