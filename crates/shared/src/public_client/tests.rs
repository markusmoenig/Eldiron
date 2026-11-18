use super::*;
use crate::{
    asset::{Asset, AssetBuffer},
    character::Character,
    item::Item,
};
use rusterix::{Texture, Tile, Value};
use std::io::{Cursor, Read, Write};

fn source() -> Project {
    let mut p = Project::new();
    p.config="[game]\nmode='multiplayer'\n[multiplayer]\nendpoint='wss://example.com/play'\ndeployment_id='demo-1'\n[ruleset]\nsource='project'\n[private]\npassword='PRIVATE_CONFIG'\n".into();
    p.world_source = "PRIVATE_WORLD_SCRIPT".into();
    p.world_source_debug = "PRIVATE_DEBUG_SCRIPT".into();
    let r = &mut p.regions[0];
    r.source = "PRIVATE_REGION_SCRIPT".into();
    r.module = serde_json::json!({"secret":"PRIVATE_REGION_MODULE"});
    r.config = "[private]\ntoken='PRIVATE_REGION_CONFIG'".into();
    r.map
        .properties
        .set("server_secret", Value::Str("PRIVATE_MAP_DATA".into()));
    let mut c = Character::new();
    c.name = "Public Hero".into();
    c.source = "PRIVATE_TEMPLATE_SCRIPT".into();
    c.data = "[attributes]\navatar='public-avatar'\nHP=999\nsecret='PRIVATE_TEMPLATE_DATA'".into();
    r.characters.insert(c.id, c.clone());
    p.characters.insert(c.id, c);
    let mut i = Item::new();
    i.name = "Public Item".into();
    i.source = "PRIVATE_ITEM_SCRIPT".into();
    r.items.insert(i.id, i.clone());
    p.items.insert(i.id, i);
    let mut tex = Texture::from_color([13, 30, 80, 255]);
    tex.set_materials_all(0.2, 0.8, 0.4, 0.7);
    tex.set_normal(0, 0, 0.2, -0.3);
    let mut tile = Tile::from_texture(tex);
    tile.gameplay_tags.push("PRIVATE_TILE_TAG".into());
    p.tiles.insert(tile.id, tile);
    let mut audio = Asset::new();
    audio.name = "Public Sound".into();
    audio.buffer = AssetBuffer::Audio(b"RIFF public audio".to_vec());
    p.assets.insert(audio.id, audio);
    p
}
fn raw_payload(bytes: &[u8]) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut b = vec![];
    zip.by_name("client.msgpack")
        .unwrap()
        .read_to_end(&mut b)
        .unwrap();
    b
}
fn repack(bytes: &[u8], extra: bool, corrupt: bool) -> Vec<u8> {
    let mut old = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(vec![]));
    for name in ["manifest.json", "client.msgpack"] {
        let mut b = vec![];
        old.by_name(name).unwrap().read_to_end(&mut b).unwrap();
        if corrupt && name == "client.msgpack" {
            b[0] ^= 1;
        }
        out.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(&b).unwrap();
    }
    if extra {
        out.start_file(
            "../private.eldiron",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        out.write_all(b"private").unwrap();
    }
    out.finish().unwrap().into_inner()
}
#[test]
fn public_archive_excludes_authority_and_preserves_visuals_and_identity() {
    let project = source();
    let original = serde_json::to_value(&project).unwrap();
    let bytes = capture(&project, "a".repeat(64), 4).unwrap();
    let raw = raw_payload(&bytes);
    assert!(!raw.windows(8).any(|s| s == b"PRIVATE_"));
    assert_eq!(
        serde_json::to_value(&project).unwrap(),
        original,
        "export must not mutate authoring source"
    );
    let mut loaded = load_player(&bytes, 4).unwrap();
    assert!(loaded.world_source.is_empty());
    assert!(loaded.world_source_debug.is_empty());
    assert!(loaded.node_graphs.is_empty());
    assert!(loaded.regions.iter().all(|r| r.source.is_empty()
        && r.module.is_null()
        && r.characters.is_empty()
        && r.items.is_empty()
        && r.map.entities.is_empty()
        && r.map.items.is_empty()));
    assert!(
        loaded
            .characters
            .values()
            .all(|c| c.source.is_empty() && !c.data.contains("HP"))
    );
    assert_eq!(
        loaded
            .client_release
            .as_ref()
            .unwrap()
            .manifest
            .content_revision,
        "a".repeat(64)
    );
    assert_eq!(loaded.client_release.as_ref().unwrap().manifest.protocol, 4);
    assert!(!loaded.migrate_default_ruleset());
    assert_eq!(loaded.sync_ruleset_items().unwrap(), 0);
    assert!(crate::project_io::decode_project(&bytes).is_err());
    assert!(crate::project_io::encode_project(&loaded).is_err());
    assert!(capture(&loaded, "a".repeat(64), 4).is_err());
    let mut assets = rusterix::Assets::default();
    prepare(&mut assets, &mut loaded).unwrap();
    assert!(
        assets.world_source.is_empty()
            && assets.region_sources.is_empty()
            && assets.node_behaviors.is_none()
    );
    let tile = project.tiles.values().next().unwrap();
    assert_eq!(
        assets.tiles[&tile.id].textures[0].data_ext, tile.textures[0].data_ext,
        "packed material and normal pixels must survive serialization"
    );
    let index = assets.tile_index(&tile.id).unwrap() as usize;
    assert_eq!(
        assets.tile_list[index].textures[0].data_ext, tile.textures[0].data_ext,
        "indexed renderer sources must carry the same material pixels"
    );
    assert_eq!(assets.audio["Public Sound"].as_ref(), b"RIFF public audio");
}
#[test]
fn corrupt_incompatible_or_extra_archive_data_is_rejected() {
    let bytes = capture(&source(), "b".repeat(64), 4).unwrap();
    assert!(decode(&bytes, 999).unwrap_err().contains("Unsupported"));
    assert!(
        decode(&repack(&bytes, false, true), 4)
            .unwrap_err()
            .contains("checksum")
    );
    assert!(
        decode(&repack(&bytes, true, false), 4)
            .unwrap_err()
            .contains("exactly")
    );
    for end in [0, 4, bytes.len() / 2, bytes.len() - 1] {
        assert!(decode(&bytes[..end], 4).is_err());
    }
}
#[test]
fn private_or_malformed_payloads_cannot_be_published() {
    let bytes = capture(&source(), "c".repeat(64), 4).unwrap();
    let mut payload: Payload = rmp_serde::from_slice(&raw_payload(&bytes)).unwrap();
    let clean = payload.clone();
    payload
        .presentation
        .config
        .push_str("\n[server]\npassword='secret'\n");
    assert!(payload.validate().is_err());
    payload = clean.clone();
    payload
        .presentation
        .regions
        .push(payload.presentation.regions[0].clone());
    assert!(payload.validate().is_err());
    payload = clean.clone();
    payload.presentation.regions[0]
        .layout
        .geometry
        .properties
        .set("script", Value::Str("secret".into()));
    assert!(payload.validate().is_err());
    payload = clean;
    let tile = payload.visuals.tiles.values_mut().next().unwrap();
    tile.textures[0].width = 8193;
    assert!(payload.validate().is_err());
}
#[test]
fn authored_single_player_loading_is_unchanged_and_export_requires_multiplayer() {
    let mut p = source();
    p.config = "[game]\nmode='single_player'\n[ruleset]\nsource='project'".into();
    assert!(
        capture(&p, "a".repeat(64), 4)
            .unwrap_err()
            .contains("multiplayer")
    );
    let bytes = crate::project_io::encode_project(&p).unwrap();
    let loaded = load_player(&bytes, 4).unwrap();
    assert!(loaded.client_release.is_none());
    assert_eq!(loaded.world_source, p.world_source);
}
#[test]
fn ui_metadata_keeps_action_catalog_requirements_without_rule_effects() {
    let source = crate::rulesets::latest_official_ruleset();
    let public = projection::policy::rules(source).unwrap();
    let root = public.parse::<toml::Table>().unwrap();
    let original = source.parse::<toml::Table>().unwrap();
    for id in original["actions"].as_table().unwrap().keys() {
        let full = crate::rulesets::resolve_action(&original, id)
            .unwrap()
            .unwrap();
        let ui = crate::rulesets::resolve_action(&root, id).unwrap().unwrap();
        assert_eq!(full.requirements, ui.requirements, "{id}");
        assert_eq!(full.resource_costs, ui.resource_costs, "{id}");
        assert_eq!(full.cooldown_seconds, ui.cooldown_seconds, "{id}");
        assert_eq!(full.intent, ui.intent, "{id}");
    }
    #[cfg(feature = "graphics")]
    {
        let mut a = rusterix::Assets::default();
        a.rules = source.into();
        let mut b = rusterix::Assets::default();
        b.rules = public.clone();
        assert_eq!(
            rusterix::client::rules_ui::action_catalog(&a, None),
            rusterix::client::rules_ui::action_catalog(&b, None)
        );
    }
    assert_eq!(
        crate::rulesets::resolve_attribute_roles(&original).unwrap(),
        crate::rulesets::resolve_attribute_roles(&root).unwrap()
    );
    assert!(root["actions"]["basic_attack"].get("result").is_none());
    assert!(root.get("world_resources").is_none());
}

#[test]
fn saved_generated_tiles_keep_explicit_baked_material_pixels() {
    let mut project = source();
    let tile = project.tiles.values_mut().next().unwrap();
    let id = tile.id;
    let original = tile.textures[0].data_ext.clone();
    tile.baked_material_data = vec![tile.textures[0].material_bytes()];
    // Authored Texture serialization deliberately retains normals alone.
    tile.textures[0] =
        serde_json::from_str(&serde_json::to_string(&tile.textures[0]).unwrap()).unwrap();
    assert_ne!(tile.textures[0].data_ext, original);
    let bytes = capture(&project, "d".repeat(64), 4).unwrap();
    let mut loaded = decode(&bytes, 4).unwrap();
    let mut assets = rusterix::Assets::default();
    prepare(&mut assets, &mut loaded).unwrap();
    assert_eq!(assets.tiles[&id].textures[0].data_ext, original);
    assert_eq!(
        assets.tile_list[assets.tile_index(&id).unwrap() as usize].textures[0].data_ext,
        original
    );
}

#[test]
fn streamed_release_loads_globals_and_only_requested_region_art() {
    use streaming::*;
    let source = source();
    let original = source.regions[0].map.geometry_objects.len();
    let bundle = partition(capture_payload(&source).unwrap(), "e".repeat(64), 4).unwrap();
    assert!(bundle.bootstrap.len() < MAX_BOOTSTRAP);
    let incomplete = decode(&bundle.bootstrap, 4).unwrap();
    let mut incomplete = incomplete;
    assert!(prepare(&mut rusterix::Assets::default(), &mut incomplete).is_err());
    let mut assembly = Assembly::new(&bundle.bootstrap, 4).unwrap();
    for p in assembly.globals() {
        assembly.accept(&p, &bundle.packs[&p.path()]).unwrap();
    }
    let mut project = assembly
        .finish("https://example.com/releases/a/game.eldiron-client".into())
        .unwrap();
    assert!(project.regions[0].map.geometry_objects.is_empty());
    let mut assets = rusterix::Assets::default();
    prepare(&mut assets, &mut project).unwrap();
    assert!(!assets.tiles.is_empty());
    let reference = project
        .client_release
        .as_ref()
        .unwrap()
        .manifest
        .packs
        .iter()
        .find(|p| p.map.is_some())
        .unwrap();
    let Pack::Region(region) = decode_pack(reference, &bundle.packs[&reference.path()]).unwrap()
    else {
        panic!()
    };
    install_region(&mut project, &mut assets, region).unwrap();
    assert_eq!(project.regions[0].map.geometry_objects.len(), original);
    assert!(project.regions[0].characters.is_empty());
    assert_eq!(
        project
            .client_release
            .as_ref()
            .unwrap()
            .manifest
            .content_revision,
        "e".repeat(64)
    );
}
#[test]
fn streamed_pack_integrity_missing_parts_and_publication_reuse() {
    use streaming::*;
    let p = capture_payload(&source()).unwrap();
    let a = partition(p.clone(), "f".repeat(64), 4).unwrap();
    let b = partition(p, "f".repeat(64), 4).unwrap();
    assert_eq!(
        a.packs, b.packs,
        "same public assets keep immutable paths/bytes"
    );
    let mut assembly = Assembly::new(&a.bootstrap, 4).unwrap();
    let globals = assembly.globals();
    assert!(!globals.is_empty());
    let reference = &globals[0];
    let mut corrupted = a.packs[&reference.path()].clone();
    corrupted[0] ^= 1;
    assert!(assembly.accept(reference, &corrupted).is_err());
    assembly
        .accept(reference, &a.packs[&reference.path()])
        .unwrap();
    assert!(
        assembly
            .accept(reference, &a.packs[&reference.path()])
            .is_err()
    );
    let missing = Assembly::new(&a.bootstrap, 4).unwrap();
    assert!(
        missing
            .finish("https://example.com/game.eldiron-client".into())
            .is_err()
    );
    let mut huge = reference.clone();
    huge.decoded_bytes = MAX_PACK + 1;
    assert!(decode_pack(&huge, &a.packs[&reference.path()]).is_err());
    let bytes = &a.packs[&reference.path()];
    assert!(decode_pack(reference, &bytes[..bytes.len() - 1]).is_err());
}
