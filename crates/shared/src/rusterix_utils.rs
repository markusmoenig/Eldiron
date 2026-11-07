use crate::prelude::*;
use rusterix::{Command, Entity, Rusterix, Value};

/// Bind ruleset identities to editable recipes. Compiled data is a derived cache.
pub fn sync_procedural_avatar_assets(
    assets: &mut rusterix::server::assets::Assets,
    project: &Project,
) {
    use rusterix::procedural_avatar::{Appearance, fingerprint};
    use std::sync::Arc;
    use theframework::prelude::Uuid;
    assets.palette = project.palette.clone();
    let old_materials = std::mem::take(&mut assets.ruleset_materials);
    let old_animations = std::mem::take(&mut assets.material_animations);
    for def in crate::ruleset_materials::definitions(&project.config) {
        let uri = format!("ruleset:material/{}", def.id);
        match crate::ruleset_materials::baked_frames(project, def.id) {
            Ok(frames) => {
                let texture = frames[0].clone();
                let duration = project
                    .procedural_materials
                    .get(&crate::rulesets::material_asset_id(def.id))
                    .and_then(|b| rusterix::ruleset_material::EditableAsset::reopen(b).ok())
                    .and_then(|a| {
                        a.root
                            .project
                            .materials
                            .into_iter()
                            .find(|m| m.id == a.selected_material)
                    })
                    .map_or(frames.len() as f32 * 0.25, |m| m.animation.duration);
                let animated = (frames, duration);
                assets
                    .material_animations
                    .insert(uri.clone(), animated.clone());
                assets.material_animations.insert(
                    crate::rulesets::material_asset_id(def.id).to_string(),
                    animated,
                );
                assets.ruleset_materials.insert(
                    crate::rulesets::material_asset_id(def.id).to_string(),
                    texture.clone(),
                );
                assets.ruleset_materials.insert(uri, texture);
            }
            Err(e) => {
                eprintln!("Ruleset material {}: {e}", def.id);
                if let Some(texture) = old_materials.get(&uri) {
                    assets
                        .ruleset_materials
                        .insert(uri.clone(), texture.clone());
                    if let Some(animation) = old_animations.get(&uri) {
                        assets.material_animations.insert(uri, animation.clone());
                    }
                }
            }
        }
    }
    let mut worn_ids = project
        .items
        .values()
        .flat_map(|item| crate::ruleset_materials::worn_material_sources(project, &item.data))
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect::<std::collections::BTreeSet<_>>();
    fn geometry_material_ids(
        doc: &denrim_materials::geometry::Document,
        ids: &mut std::collections::BTreeSet<Uuid>,
    ) {
        use denrim_materials::geometry::Operation;
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            match &tile.op {
                Operation::Texture { source, .. } => {
                    if let Ok(id) = Uuid::parse_str(source) {
                        ids.insert(id);
                    }
                }
                Operation::Asset { document, .. } | Operation::Group { document, .. } => {
                    geometry_material_ids(document, ids)
                }
                _ => {}
            }
        }
    }
    for source in project.procedural_appearances.values() {
        if let Ok(doc) = serde_json::from_str::<denrim_materials::geometry::Document>(source) {
            geometry_material_ids(&doc, &mut worn_ids);
        }
    }
    for (id, bundle) in &project.procedural_materials {
        let worn = worn_ids.contains(id);
        if id.as_u128() >> 64 != 0x4d5a49434f4e4544 && !worn {
            continue;
        }
        if let Ok(a) = rusterix::ruleset_material::EditableAsset::reopen(bundle) {
            let animation = a
                .root
                .project
                .materials
                .iter()
                .find(|m| m.id == a.selected_material)
                .map(|m| m.animation.clone())
                .unwrap_or_default();
            let size = if worn { 64 } else { 32 };
            // Existing project bakes are authoritative. Do not rebake every worn
            // material each time an entity or map changes.
            let frames = project
                .tiles
                .get(id)
                .filter(|t| !t.textures.is_empty())
                .map(|t| {
                    t.textures
                        .iter()
                        .enumerate()
                        .take(animation.frames.max(1) as usize)
                        .map(|(frame, texture)| {
                            let mut texture = texture.clone();
                            if let Some(bytes) = t.baked_material_data.get(frame) {
                                texture.apply_material_bytes(bytes);
                            }
                            texture.resized(size, size)
                        })
                        .collect::<Vec<_>>()
                })
                .map(Ok)
                .unwrap_or_else(|| {
                    (0..animation.frames.max(1))
                        .map(|frame| {
                            crate::ruleset_materials::bake_at_time(
                                &a,
                                &project.palette,
                                size as u32,
                                frame as f32 / animation.frames.max(1) as f32,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()
                });
            if let Ok(frames) = frames {
                assets
                    .ruleset_materials
                    .insert(id.to_string(), frames[0].clone());
                assets
                    .material_animations
                    .insert(id.to_string(), (frames, animation.duration));
            }
        }
    }
    let previous = std::mem::take(&mut assets.procedural_avatars);
    let selection = crate::rulesets::selected_ruleset_config(&project.config);
    let mut sources = Vec::new();
    if selection.source != "project" {
        for asset in crate::rulesets::bundled_appearance_assets_for_ruleset(
            &selection.id,
            &selection.version,
        ) {
            // Explicit bitmap avatars still take precedence over bundled defaults.
            if project
                .avatars
                .values()
                .any(|a| a.name.eq_ignore_ascii_case(asset.id))
            {
                continue;
            }
            let id = Uuid::parse_str(asset.asset_id).unwrap();
            let source = project
                .procedural_appearances
                .get(&id)
                .map_or(asset.source, String::as_str);
            sources.push((asset.id.to_string(), source, asset.equipment));
        }
    }
    let bundled =
        crate::rulesets::bundled_appearance_assets_for_ruleset(&selection.id, &selection.version);
    let profiles = project
        .characters
        .values()
        .filter_map(|character| {
            let data = character.data.parse::<toml::Table>().ok()?;
            let attrs = data.get("attributes")?;
            Some((
                attrs.get("avatar")?.as_str()?.to_owned(),
                attrs.get("appearance_profile")?.as_str()?.to_owned(),
            ))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for (id, source) in &project.procedural_appearances {
        // A project body variant retains the ruleset's explicit attachment-slot contract.
        let profile = profiles.get(&id.to_string());
        let equipment = profile
            .and_then(|name| bundled.iter().find(|asset| asset.id == name.as_str()))
            .map_or(&[][..], |asset| asset.equipment);
        sources.push((id.to_string(), source.as_str(), equipment));
    }
    let palette = serde_json::to_string(&project.palette.colors).unwrap_or_default();
    // Ruleset/palette changes invalidate baked sprites and material tiles too.
    let resources = assets
        .item_icons
        .iter()
        .map(|(k, v)| (k, v.iter().map(|t| &t.data).collect::<Vec<_>>()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let material_maps = assets
        .ruleset_materials
        .iter()
        .map(|(id, t)| (id, (&t.data, &t.data_ext)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let wardrobe = assets
        .item_wearables
        .iter()
        .map(|(key, w)| (key, w.as_ref().map(|w| w.signature())))
        .collect::<std::collections::BTreeMap<_, _>>();
    let animation_maps = assets
        .material_animations
        .iter()
        .map(|(id, (frames, duration))| {
            (
                id,
                (
                    frames
                        .iter()
                        .map(|t| (&t.data, &t.data_ext))
                        .collect::<Vec<_>>(),
                    duration.to_bits(),
                ),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let resource_key = fingerprint(&(resources, material_maps, animation_maps, wardrobe));
    for (id, source, equipment) in sources {
        let resolved = denrim_materials::geometry::Document::parse(source)
            .map_err(|e| e.to_string())
            .and_then(|doc| crate::appearance_assets::resolve_dependencies(project, &doc))
            .and_then(|doc| serde_json::to_string_pretty(&doc).map_err(|e| e.to_string()));
        let source = match resolved {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Geometry dependency: {e}");
                if let Some(old) = previous.get(&id) {
                    if let Some(avatar) = assets.avatars.get(&id) {
                        assets
                            .procedural_avatars
                            .insert(avatar.id.to_string(), old.clone());
                    }
                    assets.procedural_avatars.insert(id, old.clone());
                }
                continue;
            }
        };
        let signature = fingerprint(&(&source, &palette, resource_key, equipment));
        let asset = if let Some(old) = previous.get(&id).filter(|a| a.signature == signature) {
            old.clone()
        } else {
            match Appearance::new(&source, equipment, signature) {
                Ok(a) => Arc::new(a),
                Err(e) => {
                    eprintln!("Invalid procedural appearance {id}: {e}");
                    if let Some(old) = previous.get(&id) {
                        old.clone()
                    } else {
                        continue;
                    }
                }
            }
        };
        if let Some(avatar) = assets.avatars.get(&id) {
            assets
                .procedural_avatars
                .insert(avatar.id.to_string(), asset.clone());
        }
        assets.procedural_avatars.insert(id, asset);
    }
}

#[cfg(test)]
mod procedural_appearance_tests {
    use super::*;
    use theframework::prelude::Uuid;
    use vek::Vec2;
    #[test]
    fn integration_pass_unchanged_geometry_reuses_appearances_and_invalid_edits_keep_last_good() {
        let mut project = Project::new();
        project.config = "[ruleset]\nsource = \"project\"\n".into();
        let id = Uuid::new_v4();
        project
            .procedural_appearances
            .insert(id, denrim_materials::geometry::SWORD_EXAMPLE.into());
        let mut assets = rusterix::Assets::default();
        sync_procedural_avatar_assets(&mut assets, &project);
        let first = assets.procedural_avatars[&id.to_string()].clone();
        sync_procedural_avatar_assets(&mut assets, &project);
        assert!(std::sync::Arc::ptr_eq(
            &first,
            &assets.procedural_avatars[&id.to_string()]
        ));
        let mut invalid =
            denrim_materials::geometry::Document::parse(denrim_materials::geometry::SWORD_EXAMPLE)
                .unwrap();
        invalid.output = "missing-row".into();
        project
            .procedural_appearances
            .insert(id, serde_json::to_string(&invalid).unwrap());
        sync_procedural_avatar_assets(&mut assets, &project);
        assert!(std::sync::Arc::ptr_eq(
            &first,
            &assets.procedural_avatars[&id.to_string()]
        ));
    }
    #[test]
    fn integration_pass_cached_mesh_surfaces_restore_saved_pbr_planes() {
        let mut project = Project::new();
        project.config = "[ruleset]\nsource = \"project\"\n".into();
        let material = Uuid::new_v4();
        let asset=rusterix::ruleset_material::EditableAsset::parse(&material.to_string(),"project 1\nmaterial \"m\" called \"Surface\"\n  mode pbr\n  row \"r\"\n    use \"Color\" id \"color\"\n  output base color from row \"r\" xyz\n","m").unwrap();
        project
            .procedural_materials
            .insert(material, asset.save().unwrap());
        let mut texture = rusterix::Texture::new(vec![255; 32 * 32 * 4], 32, 32);
        texture.set_materials_all(0.9, 0.4, 1., 0.3);
        let packed = texture.material_bytes();
        texture.set_materials_all(0.5, 0., 1., 0.);
        let mut tile = rusterix::Tile::from_texture(texture);
        tile.id = material;
        tile.baked_material_data = vec![packed.clone()];
        project.tiles.insert(material, tile);
        let mut doc =
            denrim_materials::geometry::Document::load(denrim_materials::geometry::SWORD_EXAMPLE)
                .unwrap();
        doc.rows
            .iter_mut()
            .find(|r| r.id == doc.output)
            .unwrap()
            .tiles
            .push(denrim_materials::geometry::Tile {
                id: "surface".into(),
                op: denrim_materials::geometry::Operation::Texture {
                    preserve_uv: true,
                    parts: vec![],
                    source: material.to_string(),
                    bundle: None,
                    size: 32,
                    axes: [0, 1],
                    origin: [0.; 2],
                    span: [1.; 2],
                },
            });
        project
            .procedural_appearances
            .insert(Uuid::new_v4(), doc.save().unwrap());
        let mut assets = rusterix::Assets::default();
        sync_procedural_avatar_assets(&mut assets, &project);
        let surface = &assets.ruleset_materials[&material.to_string()];
        assert_eq!(&surface.material_bytes()[..4], &packed[..4]);
        assert_eq!(
            &assets.material_animations[&material.to_string()].0[0].material_bytes()[..4],
            &packed[..4]
        );
    }
    #[test]
    fn all_ruleset_inventory_icons_and_torch_states_use_saved_palette_bound_recipes() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let bytes = crate::project_io::encode_project(&project).unwrap();
        let mut project = crate::project_io::decode_project(&bytes).unwrap();
        let state_tiles = crate::ruleset_materials::icon_state_tiles(&project)
            .unwrap()
            .into_keys()
            .map(|id| (id, project.tiles[&id].clone()))
            .collect::<Vec<_>>();
        project.sync_ruleset_items().unwrap();
        for (id, saved) in state_tiles {
            assert_eq!(
                project.tiles[&id], saved,
                "State tile should not change just from reopening"
            );
        }
        let mut assets = rusterix::Assets::default();
        insert_bundled_ruleset_item_icons(&mut assets, &project);
        insert_project_item_icons(&mut assets, &project);
        let mut checked = 0;
        for item in project.items.values() {
            let mut runtime = rusterix::Item::default();
            rusterix::server::data::apply_item_data(&mut runtime, &item.data);
            runtime.creator_id = item.id;
            let Some(name) = runtime.attributes.get_str("ruleset_id") else {
                continue;
            };
            let on = runtime.attributes.get_bool_default("active", true);
            let expected = crate::ruleset_materials::baked_frames(
                &project,
                &format!("icon/{name}/{}", if on { "on" } else { "off" }),
            )
            .unwrap();
            let (size, rgba) =
                rusterix::client::widget::Widget::item_generated_icon_square(&assets, &runtime)
                    .unwrap();
            assert_eq!(size, 32, "{name}");
            assert_eq!(rgba, expected[0].data, "{name}");
            checked += 1;
        }
        assert_eq!(
            checked,
            crate::rulesets::ruleset_item_templates_from_source(
                crate::rulesets::latest_official_ruleset()
            )
            .unwrap()
            .len()
        );
        let lit = crate::ruleset_materials::baked_frames(&project, "icon/torch/on").unwrap();
        let unlit = crate::ruleset_materials::baked_frames(&project, "icon/torch/off").unwrap();
        assert_eq!((lit.len(), unlit.len()), (4, 1));
        assert!(lit.windows(2).all(|f| f[0].data != f[1].data));
        assert_ne!(lit[0].data, unlit[0].data);
        for state in crate::rulesets::bundled_icon_state_assets_for_ruleset(
            crate::rulesets::OFFICIAL_RULESET_ID,
            crate::rulesets::OFFICIAL_RULESET_VERSION,
        ) {
            let id = Uuid::parse_str(state.tile_id.unwrap()).unwrap();
            let expected = if state.state == "on" { &lit } else { &unlit };
            assert_eq!(assets.palette_tile_overrides[&id].textures, *expected);
            assert_eq!(
                project.tiles[&id]
                    .textures
                    .iter()
                    .map(|t| &t.data)
                    .collect::<Vec<_>>(),
                expected.iter().map(|t| &t.data).collect::<Vec<_>>()
            );
            assert_eq!(
                project.tiles[&id].to_material_array(),
                expected
                    .iter()
                    .map(|t| t.data_ext.clone().unwrap())
                    .collect::<Vec<_>>()
            );
        }
        // Swapping palette entries changes derived pixels while retaining the authored indices.
        let before = crate::ruleset_materials::baked(&project, "icon/gold_coin/on").unwrap();
        let id = crate::rulesets::material_asset_id("icon/gold_coin/on");
        let authored = project.procedural_materials[&id].clone();
        project.palette.colors.swap(24, 42);
        let after = crate::ruleset_materials::baked(&project, "icon/gold_coin/on").unwrap();
        assert_ne!(before.data, after.data);
        assert_eq!(project.procedural_materials[&id], authored);
    }
    #[test]
    fn creature_defaults_and_clothing_icons_resolve_from_saved_ruleset_assets() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let bytes = crate::project_io::encode_project(&project).unwrap();
        let project = crate::project_io::decode_project(&bytes).unwrap();
        let mut assets = rusterix::Assets::default();
        insert_bundled_ruleset_avatars(&mut assets, &project);
        insert_bundled_ruleset_item_icons(&mut assets, &project);
        insert_project_item_icons(&mut assets, &project);
        sync_procedural_avatar_assets(&mut assets, &project);
        let rules = project
            .rules_source()
            .unwrap()
            .parse::<toml::Table>()
            .unwrap();
        for (race, name) in [
            ("Human", "humanoid"),
            ("Orc", "orc"),
            ("Skeleton", "skeleton"),
        ] {
            assert_eq!(rules["races"][race]["default_avatar"].as_str(), Some(name));
            let mut entity = Entity::default();
            entity.set_attribute("avatar_id", Value::Id(assets.avatars[name].id));
            let resolved = rusterix::procedural_avatar::Runtime::resolve(&entity, &assets).unwrap();
            assert!(std::sync::Arc::ptr_eq(
                resolved,
                &assets.procedural_avatars[name]
            ));
        }
        for name in [
            "linen_shirt",
            "wool_trousers",
            "leather_shoes",
            "padded_armor",
            "leather_vest",
            "chain_shirt",
            "cleric_vestments",
        ] {
            let source = project
                .items
                .values()
                .find(|i| i.data.contains(&format!("ruleset_id = \"{name}\"")))
                .unwrap();
            let expected_id = crate::rulesets::material_asset_id(&format!("icon/{name}/on"));
            assert_eq!(
                crate::ruleset_materials::icon_id(&source.data),
                Some(expected_id)
            );
            assert!(project.procedural_materials.contains_key(&expected_id));
            let mut item = rusterix::Item::default();
            rusterix::server::data::apply_item_data(&mut item, &source.data);
            item.creator_id = source.id;
            let (size, pixels) =
                rusterix::client::widget::Widget::item_generated_icon_square(&assets, &item)
                    .unwrap();
            assert_eq!(size, 32);
            assert_eq!(
                pixels,
                assets.ruleset_materials[&expected_id.to_string()].data
            );
            assert!(
                rusterix::procedural_avatar::Runtime::resolve_wearable(&item, &assets).is_some()
            );
        }
    }
    #[test]
    fn worn_materials_follow_live_item_edits_palette_changes_and_project_save() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let shirt = project
            .items
            .values()
            .find(|i| i.data.contains("ruleset_id = \"linen_shirt\""))
            .unwrap()
            .clone();
        let mut runtime_item = rusterix::Item::default();
        rusterix::server::data::apply_item_data(&mut runtime_item, &shirt.data);
        runtime_item.creator_id = shirt.id;
        let mut entity = Entity::default();
        entity.orientation = Vec2::new(0., 1.);
        entity.equipped.insert("torso".into(), runtime_item.clone());
        let mut assets = rusterix::Assets::default();
        insert_project_item_icons(&mut assets, &project);
        sync_procedural_avatar_assets(&mut assets, &project);
        let original = assets.procedural_avatars["humanoid"]
            .sprite(&entity, &assets, "idle", 0.)
            .unwrap();
        let original_map = assets.ruleset_materials["ruleset:material/linen-shirt"].clone();
        let geometry_id = crate::ruleset_materials::worn_geometry_id(&shirt.data).unwrap();
        let human_id = Uuid::parse_str(crate::rulesets::HUMANOID_APPEARANCE_ASSET_ID).unwrap();
        let body_source = project.procedural_appearances[&human_id].clone();
        let mut garment = denrim_materials::geometry::Document::load(
            &project.procedural_appearances[&geometry_id],
        )
        .unwrap();
        for t in garment.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            if let denrim_materials::geometry::Operation::SurfaceOffset { distance } = &mut t.op {
                *distance = 0.035;
            }
        }
        let edited_geometry = garment.save().unwrap();
        project
            .procedural_appearances
            .insert(geometry_id, edited_geometry.clone());
        sync_procedural_avatar_assets(&mut assets, &project);
        assert_ne!(
            original,
            assets.procedural_avatars["humanoid"]
                .sprite(&entity, &assets, "idle", 0.)
                .unwrap()
        );
        assert_eq!(project.procedural_appearances[&human_id], body_source);
        project.palette.colors[23] = project.palette.colors[42].clone();
        sync_procedural_avatar_assets(&mut assets, &project);
        assert_ne!(
            original_map.data,
            assets.ruleset_materials["ruleset:material/linen-shirt"].data
        );
        assert_ne!(
            original,
            assets.procedural_avatars["humanoid"]
                .sprite(&entity, &assets, "idle", 0.)
                .unwrap()
        );
        // A running entity still has its original item attributes. Project bindings
        // must override them when the author edits the worn appearance.
        let mut data = shirt.data.parse::<toml::Table>().unwrap();
        data["attributes"].as_table_mut().unwrap().insert(
            "appearance_materials".into(),
            toml::Value::Array(vec![toml::Value::String(
                "body-torso=ruleset:material/leather-vest".into(),
            )]),
        );
        project.items.get_mut(&shirt.id).unwrap().data = toml::to_string(&data).unwrap();
        project.sync_ruleset_items().unwrap();
        insert_project_item_icons(&mut assets, &project);
        sync_procedural_avatar_assets(&mut assets, &project);
        let worn =
            rusterix::procedural_avatar::Runtime::resolve_wearable(&runtime_item, &assets).unwrap();
        assert_eq!(
            worn.materials["body-torso"],
            "ruleset:material/leather-vest"
        );
        let bytes = crate::project_io::encode_project(&project).unwrap();
        let reopened = crate::project_io::decode_project(&bytes).unwrap();
        assert_eq!(
            crate::ruleset_materials::worn_materials(&reopened.items[&shirt.id].data),
            worn.materials
        );
        assert_eq!(reopened.palette.colors, project.palette.colors);
        assert_eq!(
            reopened.procedural_appearances[&geometry_id],
            edited_geometry
        );
        data["attributes"]["appearance_materials"] = toml::Value::Array(vec![]);
        data["attributes"]["appearance_worn_geometry"] = toml::Value::String(String::new());
        project.items.get_mut(&shirt.id).unwrap().data = toml::to_string(&data).unwrap();
        project.sync_ruleset_items().unwrap();
        insert_project_item_icons(&mut assets, &project);
        assert!(
            rusterix::procedural_avatar::Runtime::resolve_wearable(&runtime_item, &assets)
                .is_none()
        );
    }
    #[test]
    fn item_icons_models_and_custom_equipment_share_the_project_bindings() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let template = project
            .items
            .values()
            .find(|i| i.data.contains("ruleset_id = \"training_sword\""))
            .unwrap()
            .clone();
        let mut item = rusterix::Item::default();
        assert!(
            template.data.contains("appearance_geometry"),
            "{}",
            template.data
        );
        rusterix::server::data::apply_item_data(&mut item, &template.data);
        item.creator_id = template.id;
        let mut assets = rusterix::Assets::default();
        insert_bundled_ruleset_item_icons(&mut assets, &project);
        insert_project_item_icons(&mut assets, &project);
        sync_procedural_avatar_assets(&mut assets, &project);
        let (size, pixels) =
            rusterix::client::widget::Widget::item_generated_icon_square(&assets, &item).unwrap();
        assert_eq!(size, 32);
        assert_eq!(
            pixels,
            crate::ruleset_materials::baked(&project, "icon/training_sword/on")
                .unwrap()
                .data
        );
        let icon_id = crate::rulesets::material_asset_id("icon/training_sword/on");
        assert_eq!(assets.ruleset_materials[&icon_id.to_string()].width, 32);
        assert_eq!(
            assets.ruleset_materials["ruleset:material/icon/training_sword/on"].data,
            pixels
        );
        assert!(
            rusterix::procedural_avatar::Runtime::resolve_item(&item, &assets).is_some(),
            "model {:?}; bindings {:?}; sources {:?}",
            item.attributes.get_str("appearance_geometry"),
            assets.item_models,
            assets.procedural_avatars.keys().collect::<Vec<_>>()
        );
        let mut entity = rusterix::Entity::default();
        entity.orientation = Vec2::new(1., 0.);
        entity.equipped.insert("main_hand".into(), item.clone());
        let human = assets.procedural_avatars["humanoid"].clone();
        let before = human.sprite(&entity, &assets, "idle", 0.).unwrap();
        let original = crate::rulesets::geometry_asset_id("training_sword");
        let custom = Uuid::new_v4();
        let mut doc =
            denrim_materials::geometry::Document::load(&project.procedural_appearances[&original])
                .unwrap();
        doc.id = custom.to_string();
        let denrim_materials::geometry::Operation::Group { controls, .. } =
            &mut doc.rows[0].tiles[0].op
        else {
            panic!()
        };
        controls
            .iter_mut()
            .find(|c| c.id == "blade-length")
            .unwrap()
            .value = 1.15;
        project
            .procedural_appearances
            .insert(custom, doc.save().unwrap());
        let mut data = template.data.parse::<toml::Table>().unwrap();
        data["attributes"].as_table_mut().unwrap().insert(
            "appearance_geometry".into(),
            toml::Value::String(custom.to_string()),
        );
        project.items.get_mut(&template.id).unwrap().data = toml::to_string_pretty(&data).unwrap();
        project.sync_ruleset_items().unwrap();
        assert!(
            project.items[&template.id]
                .data
                .contains(&custom.to_string()),
            "Rules refresh must preserve appearance edits"
        );
        insert_project_item_icons(&mut assets, &project);
        sync_procedural_avatar_assets(&mut assets, &project);
        assert!(
            rusterix::procedural_avatar::Runtime::resolve_item(&item, &assets)
                .unwrap()
                .source
                .contains(&custom.to_string())
        );
        assert_ne!(
            before,
            assets.procedural_avatars["humanoid"]
                .sprite(&entity, &assets, "idle", 0.)
                .unwrap()
        );
        // A different weapon category must use its own model in the same socket.
        let axe_template = project
            .items
            .values()
            .find(|i| i.data.contains("ruleset_id = \"hand_axe\""))
            .unwrap();
        let mut axe = rusterix::Item::default();
        rusterix::server::data::apply_item_data(&mut axe, &axe_template.data);
        axe.creator_id = axe_template.id;
        assert!(
            rusterix::procedural_avatar::Runtime::resolve_item(&axe, &assets)
                .unwrap()
                .source
                .contains("hand_axe")
        );
        entity.equipped.insert("main_hand".into(), axe);
        assert_ne!(
            before,
            assets.procedural_avatars["humanoid"]
                .sprite(&entity, &assets, "idle", 0.)
                .unwrap()
        );
    }
    #[test]
    fn ruleset_model_is_editable_cached_and_does_not_overwrite_project_bitmaps() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let asset = crate::rulesets::bundled_appearance_assets_for_ruleset(
            crate::rulesets::OFFICIAL_RULESET_ID,
            crate::rulesets::OFFICIAL_RULESET_VERSION,
        )
        .remove(0);
        let id = theframework::prelude::Uuid::parse_str(asset.asset_id).unwrap();
        assert!(project.procedural_appearances.contains_key(&id));
        let mut assets = rusterix::Assets::default();
        insert_bundled_ruleset_avatars(&mut assets, &project);
        sync_procedural_avatar_assets(&mut assets, &project);
        let original = assets.procedural_avatars["humanoid"].clone();
        let bitmap_id = assets.avatars["humanoid"].id.to_string();
        assert!(std::sync::Arc::ptr_eq(
            &original,
            &assets.procedural_avatars[&bitmap_id]
        ));
        sync_procedural_avatar_assets(&mut assets, &project);
        assert!(std::sync::Arc::ptr_eq(
            &original,
            &assets.procedural_avatars["humanoid"]
        ));
        let edited =
            project.procedural_appearances[&id].replace("Ruleset humanoid", "Edited humanoid");
        project.procedural_appearances.insert(id, edited.clone());
        project.sync_ruleset_items().unwrap();
        assert_eq!(project.procedural_appearances[&id], edited);
        sync_procedural_avatar_assets(&mut assets, &project);
        assert!(!std::sync::Arc::ptr_eq(
            &original,
            &assets.procedural_avatars["humanoid"]
        ));
        let bitmap = assets.avatars["humanoid"].clone();
        project.avatars.insert(bitmap.id, bitmap);
        sync_procedural_avatar_assets(&mut assets, &project);
        assert!(!assets.procedural_avatars.contains_key("humanoid"));
        assert!(assets.procedural_avatars.contains_key(&id.to_string()));
    }
}

fn insert_bundled_ruleset_avatars(
    assets: &mut rusterix::server::assets::Assets,
    project: &Project,
) {
    match crate::rulesets::bundled_avatars_for_project(&project.config) {
        Ok(avatars) => {
            for (id, avatar) in avatars {
                assets.avatars.insert(id.to_string(), avatar);
            }
        }
        Err(err) => eprintln!("Ruleset avatar load error: {}", err),
    }
}

fn insert_bundled_ruleset_textures(
    assets: &mut rusterix::server::assets::Assets,
    project: &Project,
) {
    match crate::rulesets::bundled_textures_for_project(&project.config) {
        Ok(textures) => {
            for (id, mut texture) in textures {
                crate::palette::fit_rgba(&mut texture.data, &project.palette);
                assets.textures.insert(id.to_string(), texture);
            }
        }
        Err(err) => eprintln!("Ruleset texture load error: {}", err),
    }
}

fn insert_bundled_ruleset_item_icons(
    assets: &mut rusterix::server::assets::Assets,
    project: &Project,
) {
    assets.palette_tile_overrides.clear();
    if let Ok(ids) = crate::rulesets::bundled_tile_ids_for_project(&project.config) {
        for id in ids {
            if let Some(original) = project.tiles.get(&id) {
                let mut tile = original.clone();
                for frame in &mut tile.textures {
                    crate::palette::fit_rgba(&mut frame.data, &project.palette);
                }
                assets.palette_tile_overrides.insert(id, tile);
            }
        }
    }
    assets.item_icons.clear();
    assets.project_item_icon_keys.clear();
    match crate::rulesets::bundled_item_icon_states_for_project(&project.config) {
        Ok(states) => {
            for (item_id, state, mut frames) in states {
                for frame in &mut frames {
                    crate::palette::fit_rgba(&mut frame.data, &project.palette);
                }
                let item_id = item_id.trim().to_ascii_lowercase();
                let state = state.trim().to_ascii_lowercase();
                assets
                    .item_icons
                    .insert(format!("{item_id}:{state}"), frames.clone());
                if state == "on" {
                    assets.item_icons.insert(item_id, frames);
                }
            }
        }
        Err(err) => eprintln!("Ruleset item icon state load error: {}", err),
    }
    insert_procedural_ruleset_icons(assets, project);
}

/// Publish the same palette-bound MZ output to inventory, commands, ground items
/// and geometry resource lookup. Project-authored bitmap frames still override it.
pub fn insert_procedural_ruleset_icons(assets: &mut rusterix::Assets, project: &Project) {
    for def in crate::ruleset_materials::definitions(&project.config) {
        let Some(path) = def.id.strip_prefix("icon/") else {
            continue;
        };
        let Some((name, state)) = path.rsplit_once('/') else {
            continue;
        };
        match crate::ruleset_materials::baked_frames(project, def.id) {
            Ok(frames) => {
                assets
                    .item_icons
                    .insert(format!("{name}:{state}"), frames.clone());
                if state == "on" {
                    assets.item_icons.insert(name.into(), frames.clone());
                    if let Some(texture) = frames.first() {
                        assets.textures.insert(name.into(), texture.clone());
                    }
                }
            }
            Err(e) => eprintln!("Ruleset procedural icon {name}: {e}"),
        }
    }
    match crate::ruleset_materials::icon_state_tiles(project) {
        Ok(states) => {
            for (id, frames) in states {
                if let Some(tile) = project.tiles.get(&id) {
                    let mut tile = tile.clone();
                    tile.textures = frames;
                    tile.baked_material_data = tile
                        .textures
                        .iter()
                        .map(rusterix::Texture::material_bytes)
                        .collect();
                    assets.palette_tile_overrides.insert(id, tile);
                }
            }
        }
        Err(e) => eprintln!("Procedural state tile: {e}"),
    }
}

fn insert_project_item_icons(assets: &mut rusterix::server::assets::Assets, project: &Project) {
    assets.item_template_ids = project
        .items
        .values()
        .map(|i| (i.id, i.name.clone()))
        .collect();
    assets.item_models.clear();
    assets.item_wearables.clear();
    for item in project.items.values() {
        let mut runtime_item = rusterix::Item::default();
        rusterix::server::data::apply_item_data(&mut runtime_item, &item.data);
        let mut keys = vec![
            item.id.to_string().to_ascii_lowercase(),
            item.name.trim().to_ascii_lowercase(),
        ];
        for key in ["ruleset_path", "ruleset_id", "class_name", "name"] {
            if let Some(value) = runtime_item.attributes.get_str(key) {
                keys.push(value.trim().to_ascii_lowercase());
            }
        }
        for region in &project.regions {
            keys.extend(
                region
                    .items
                    .values()
                    .filter(|instance| instance.item_id == item.id)
                    .map(|instance| instance.id.to_string().to_ascii_lowercase()),
            );
        }
        keys.sort_unstable();
        keys.dedup();

        if let Some(model) = runtime_item.attributes.get_str("appearance_geometry") {
            for key in &keys {
                assets.item_models.insert(key.clone(), model.to_owned());
            }
        }
        let worn =
            rusterix::procedural_avatar::Wearable::from_item(&runtime_item).unwrap_or_else(|e| {
                eprintln!("Worn appearance for {}: {e}", item.name);
                None
            });
        for key in &keys {
            assets.item_wearables.insert(key.clone(), worn.clone());
        }
        let mut procedural = [false; 2];
        for (index, on) in [true, false].into_iter().enumerate() {
            if let Ok(Some(frames)) = crate::ruleset_materials::frames_for_item(project, item, on) {
                procedural[index] = true;
                for key in &keys {
                    let state_key = format!("{key}:{}", if on { "on" } else { "off" });
                    assets.project_item_icon_keys.insert(state_key.clone());
                    assets.item_icons.insert(state_key, frames.clone());
                    if on {
                        assets.project_item_icon_keys.insert(key.clone());
                        assets.item_icons.insert(key.clone(), frames.clone());
                    }
                }
            }
        }

        if !procedural[0] && !item.icon_frames.is_empty() {
            for key in &keys {
                assets.project_item_icon_keys.insert(key.clone());
                assets.project_item_icon_keys.insert(format!("{key}:on"));
                assets
                    .item_icons
                    .insert(key.clone(), item.icon_frames.clone());
                assets
                    .item_icons
                    .insert(format!("{key}:on"), item.icon_frames.clone());
            }
        }
        if !procedural[1] && !item.icon_off_frames.is_empty() {
            for key in &keys {
                assets.project_item_icon_keys.insert(format!("{key}:off"));
                assets
                    .item_icons
                    .insert(format!("{key}:off"), item.icon_off_frames.clone());
            }
        }
    }
}

/// Rebuild the item-icon registry from bundled defaults and project-owned
/// overrides. Project frames are inserted last and therefore remain the
/// authoritative artwork for both ruleset and custom items.
pub fn sync_item_icon_assets(rusterix: &mut Rusterix, project: &Project) {
    insert_bundled_ruleset_textures(&mut rusterix.assets, project);
    insert_bundled_ruleset_item_icons(&mut rusterix.assets, project);
    insert_project_item_icons(&mut rusterix.assets, project);
    rusterix.set_overlay_dirty();
}

/// Refresh the visual asset subset used by Creator previews and tree icons.
/// This deliberately does not start or mutate the game server.
pub fn sync_editor_visual_assets(rusterix: &mut Rusterix, project: &Project) {
    rusterix.assets.config = project.config.clone();
    rusterix.assets.rules = project.rules_source().unwrap_or_else(|_| String::new());
    rusterix.assets.palette = project.palette.clone();
    rusterix.assets.read_rules_metadata();

    rusterix.assets.avatars.clear();
    insert_bundled_ruleset_avatars(&mut rusterix.assets, project);
    for avatar in project.avatars.values() {
        rusterix
            .assets
            .avatars
            .insert(avatar.name.clone(), avatar.clone());
    }
    sync_item_icon_assets(rusterix, project);
    rusterix.set_tiles(project.tiles.clone(), true);
    sync_procedural_avatar_assets(&mut rusterix.assets, project);
}

/// Publish only valid, semantically changed rules. Invalid drafts leave the
/// running regions and preview assets on their last accepted ruleset.
pub fn sync_live_rules(rusterix: &mut Rusterix, project: &Project) -> Result<bool, String> {
    let Some(table) = prepare_live_rules(&mut rusterix.assets, project)? else {
        return Ok(false);
    };
    rusterix::server::publish_rules(table);
    // Creator stops regular region redraws while paused. Drain the edit now;
    // the paused region path handles messages without advancing gameplay.
    if rusterix.server.state == rusterix::ServerState::Paused {
        rusterix.server.redraw_tick();
    }
    rusterix.set_overlay_dirty();
    Ok(true)
}

/// Update rules and icon caches independently of the GPU renderer. Validation
/// finishes before touching the last accepted asset snapshot.
pub fn prepare_live_rules(
    assets: &mut rusterix::server::assets::Assets,
    project: &Project,
) -> Result<Option<toml::Table>, String> {
    let source = project.rules_source()?;
    if assets.rules == source {
        return Ok(None);
    }
    let table = source
        .parse()
        .map_err(|error| format!("Rules compilation: {error}"))?;
    assets.rules = source;
    assets.config = project.config.clone();
    assets.read_rules_metadata();
    insert_bundled_ruleset_textures(assets, project);
    insert_bundled_ruleset_avatars(assets, project);
    insert_bundled_ruleset_item_icons(assets, project);
    insert_project_item_icons(assets, project);
    sync_procedural_avatar_assets(assets, project);
    for (id, tile) in &project.tiles {
        assets.tiles.insert(*id, tile.clone());
    }
    Ok(Some(table))
}

/// Start the server
pub fn start_server(
    rusterix: &mut Rusterix,
    project: &mut Project,
    debug: bool,
) -> Result<(), String> {
    let rules_source = project.rules_source()?;
    for error in crate::geometry_recipe::synchronize(project).1 {
        eprintln!("MZ geometry recipe: {error}");
    }
    crate::entity_graph::synchronize(project)
        .map_err(|error| format!("Entity configuration: {error}"))?;
    rusterix.server.clear();
    rusterix.server.debug_mode = debug;
    rusterix.server.log_changed = true;
    // Change only behavior execution. Keep the established region/client startup,
    // screen configuration and character-selection registration path intact.
    rusterix.assets.node_behaviors = Some(rusterix::server::nodes::region::BehaviorAssets {
        graphs: project
            .node_graphs
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        characters: project
            .characters
            .values()
            .map(|c| (c.name.clone(), c.id))
            .collect(),
        items: project
            .items
            .values()
            .map(|i| (i.name.clone(), i.id))
            .collect(),
        regions: project.regions.iter().map(|r| (r.map.id, r.id)).collect(),
    });

    insert_content_into_maps_mode(project, debug);
    rusterix.assets.rules = rules_source;
    rusterix.assets.read_rules_metadata();
    rusterix.assets.locales_src =
        crate::rulesets::resolve_project_locales(&project.config, &project.locales).unwrap_or_else(
            |err| {
                eprintln!("Ruleset locale resolution error: {}", err);
                project.locales.clone()
            },
        );
    rusterix.assets.audio_fx_src = project.audio_fx.clone();
    rusterix.assets.authoring_src = project.authoring.clone();
    if debug && !project.world_source_debug.is_empty() {
        rusterix.assets.world_source = project.world_source_debug.clone();
    } else {
        rusterix.assets.world_source = project.world_source.clone();
    }
    rusterix.assets.region_sources.clear();
    rusterix.set_block_props(project.block_props.clone());
    rusterix.assets.read_locales();

    sync_item_icon_assets(rusterix, project);
    sync_display_templates(&mut rusterix.assets, project, debug);

    // Publish all region sources before taking the shared startup snapshot.
    for region in &project.regions {
        let region_source = if debug && !region.source_debug.is_empty() {
            region.source_debug.clone()
        } else {
            region.source.clone()
        };
        rusterix
            .assets
            .region_sources
            .insert(region.map.id, region_source);
    }
    let region_assets = rusterix::server::assets::RegionAssets::for_regions(&mut rusterix.assets);
    // Create the regions without cloning textures and behavior graphs per map.
    for region in &project.regions {
        let region_config = crate::project::merge_config_toml(&project.config, &region.config);
        rusterix.server.create_region_instance_with_shared_assets(
            region.name.clone(),
            region.map.clone(),
            &region_assets,
            region_config,
        );
    }

    // Create the avatars
    rusterix.assets.avatars.clear();
    insert_bundled_ruleset_avatars(&mut rusterix.assets, project);
    for avatar in &mut project.avatars.values() {
        rusterix
            .assets
            .avatars
            .insert(avatar.name.clone(), avatar.clone());
    }
    insert_bundled_ruleset_textures(&mut rusterix.assets, project);
    sync_procedural_avatar_assets(&mut rusterix.assets, project);

    // Wait for the region to be created
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(std::time::Duration::from_millis(10));
    // Set the time for each region to the project time
    for region in &mut project.regions {
        rusterix.server.set_time(&region.map.id, project.time);
    }

    rusterix.server.set_state(rusterix::ServerState::Running);
    // Force dynamic overlays (lights/billboards/avatars) to rebuild after restarts.
    rusterix.scene_handler.mark_dynamics_dirty();
    Ok(())
}

/// Let freshly queued startup work settle after client commands create the local player.
pub fn warmup_runtime(rusterix: &mut Rusterix, project: &mut Project, ticks: usize) {
    for _ in 0..ticks {
        rusterix.server.system_tick();
        rusterix.server.redraw_tick();

        if let Some(new_region_name) = rusterix.update_server() {
            rusterix.client.current_map = new_region_name;
        }

        for region in &mut project.regions {
            rusterix.server.apply_entities_items(&mut region.map);
            if let Some(time) = rusterix.server.get_time(&region.map.id) {
                rusterix.client.set_server_time(time);
                project.time = time;
            }
        }
    }
}

/// Rendering needs the same template data as server startup to resolve outfits
/// and item appearances. This does not create region instances or run behavior.
pub fn sync_display_templates(assets: &mut rusterix::Assets, project: &Project, debug: bool) {
    // Characters
    assets.entities.clear();
    assets.entity_authoring.clear();
    assets.character_maps.clear();
    assets.entity_tiles.clear();
    for character in project.characters.values() {
        if debug && !character.source_debug.is_empty() {
            assets.entities.insert(
                character.name.clone(),
                (character.source_debug.clone(), character.data.clone()),
            );
        } else {
            assets.entities.insert(
                character.name.clone(),
                (character.source.clone(), character.data.clone()),
            );
        }
        assets
            .entity_authoring
            .insert(character.name.clone(), character.authoring.clone());
        if !character.map.vertices.is_empty() {
            assets
                .character_maps
                .insert(character.name.clone(), character.map.clone());
        }
    }

    // Items
    assets.items.clear();
    assets.item_authoring.clear();
    assets.item_maps.clear();
    assets.item_tiles.clear();
    for item in project.items.values() {
        if debug && !item.source_debug.is_empty() {
            assets.items.insert(
                item.name.clone(),
                (item.source_debug.clone(), item.data.clone()),
            );
        } else {
            assets
                .items
                .insert(item.name.clone(), (item.source.clone(), item.data.clone()));
        }
        assets
            .item_authoring
            .insert(item.name.clone(), item.authoring.clone());
        if !item.map.vertices.is_empty() {
            assets.item_maps.insert(item.name.clone(), item.map.clone());
        }
    }
}

/// Setup the client
pub fn setup_client(rusterix: &mut Rusterix, project: &mut Project) -> Vec<Command> {
    rusterix.assets.config = project.config.clone();
    rusterix.assets.world_source = project.world_source.clone();
    rusterix.assets.rules = project.rules_source().unwrap_or_else(|err| {
        eprintln!("Ruleset resolution error: {}", err);
        String::new()
    });
    rusterix.assets.read_rules_metadata();
    rusterix.assets.locales_src =
        crate::rulesets::resolve_project_locales(&project.config, &project.locales).unwrap_or_else(
            |err| {
                eprintln!("Ruleset locale resolution error: {}", err);
                project.locales.clone()
            },
        );
    rusterix.assets.audio_fx_src = project.audio_fx.clone();
    rusterix.assets.authoring_src = project.authoring.clone();
    rusterix.assets.region_sources.clear();
    rusterix.set_block_props(project.block_props.clone());
    rusterix.assets.read_locales();
    rusterix.assets.palette = project.palette.clone();
    rusterix.assets.palette_materials = project
        .palette_materials
        .iter()
        .map(|m| m.rmoe_values())
        .collect();
    rusterix.assets.palette_material_ids = project
        .palette_materials
        .iter()
        .map(|m| m.material_id())
        .collect();

    rusterix.assets.maps.clear();
    for region in &mut project.regions {
        region.map.name.clone_from(&region.name);
        rusterix
            .assets
            .maps
            .insert(region.map.name.clone(), region.map.clone());
        rusterix
            .assets
            .region_sources
            .insert(region.map.id, region.source.clone());
    }
    rusterix.assets.screens.clear();
    rusterix.assets.screen_settings.clear();
    for (_, screen) in &project.screens {
        let scr = screen.map.clone();
        let name = screen.map.name.clone();
        rusterix.assets.screens.insert(name.clone(), scr);
        rusterix
            .assets
            .screen_settings
            .insert(name, screen.settings.clone());
    }
    rusterix.assets.avatars.clear();
    insert_bundled_ruleset_avatars(&mut rusterix.assets, project);
    for avatar in project.avatars.values() {
        rusterix
            .assets
            .avatars
            .insert(avatar.name.clone(), avatar.clone());
    }
    insert_bundled_ruleset_textures(&mut rusterix.assets, project);
    sync_item_icon_assets(rusterix, project);
    sync_procedural_avatar_assets(&mut rusterix.assets, project);
    rusterix.assets.fonts.clear();
    rusterix.assets.audio.clear();
    for (_, asset) in project.assets.iter() {
        if let AssetBuffer::Font(bytes) = &asset.buffer {
            if let Ok(font) =
                fontdue::Font::from_bytes(bytes.clone(), fontdue::FontSettings::default())
            {
                rusterix.assets.fonts.insert(asset.name.clone(), font);
            }
        } else if let AssetBuffer::Audio(bytes) = &asset.buffer {
            rusterix
                .assets
                .audio
                .insert(asset.name.clone(), bytes.clone());
        }
    }
    // Client setup can swap maps/widgets while keeping SceneVM alive.
    // Invalidate dynamic caches so first game frame always reuploads lights/dynamics.
    rusterix.scene_handler.mark_dynamics_dirty();
    rusterix.setup_client()
}

/// Convert the characters and items into Entities / Items for the rusterix server
pub fn insert_content_into_maps(project: &mut Project) {
    insert_content_into_maps_mode(project, false);
}

fn is_legacy_python_instance_setup(source: &str) -> bool {
    source.trim_start().starts_with("def setup")
}

pub fn insert_content_into_maps_mode(project: &mut Project, debug: bool) {
    if let Err(error) = crate::entity_graph::synchronize(project) {
        eprintln!("Entity configuration: {error}");
    }
    let block_props = &project.block_props;
    for region in &mut project.regions {
        // Imports can replace the map without updating its internal name.
        region.map.name.clone_from(&region.name);
        region.map.entities.clear();
        for instance in region.characters.values_mut() {
            if is_legacy_python_instance_setup(&instance.source) {
                instance.source.clear();
                instance.source_debug.clear();
            }
            let mut entity = Entity {
                creator_id: instance.id,
                position: instance.position,
                orientation: instance.orientation,
                ..Default::default()
            };
            entity.set_attribute("name", Value::Str(instance.name.clone()));
            if let Some(character_template) = project.characters.get(&instance.character_id) {
                entity.set_attribute("name", Value::Str(character_template.name.clone()));
            }
            entity.set_attribute(
                "setup",
                Value::Str(if debug && !instance.source_debug.is_empty() {
                    instance.source_debug.clone()
                } else {
                    instance.source.clone()
                }),
            );
            if let Some(character) = project.characters.get(&instance.character_id) {
                entity.set_attribute("class_name", Value::Str(character.name.clone()));
                rusterix::server::data::apply_entity_data(&mut entity, &character.data);
            }
            // Resolve instance configuration before race/class derivation and startup.
            rusterix::server::data::apply_entity_data(&mut entity, &instance.data);
            entity.set_attribute("_entity_configuration", Value::Str(instance.data.clone()));
            let template_data = project
                .characters
                .get(&instance.character_id)
                .map(|c| c.data.as_str())
                .unwrap_or("");
            if let Ok(bindings) = crate::entity_graph::merged_input(template_data, &instance.data) {
                entity.set_attribute("_input_bindings", Value::Str(bindings));
            }
            region.map.entities.push(entity);
        }

        region.map.items.clear();
        for instance in region.items.values_mut() {
            if is_legacy_python_instance_setup(&instance.source) {
                instance.source.clear();
                instance.source_debug.clear();
            }
            let mut item = rusterix::Item {
                creator_id: instance.id,
                position: instance.position,
                ..Default::default()
            };
            item.set_attribute("name", Value::Str(instance.name.clone()));
            if let Some(item_template) = project.items.get(&instance.item_id) {
                item.set_attribute("name", Value::Str(item_template.name.clone()));
            }
            item.set_attribute(
                "setup",
                Value::Str(if debug && !instance.source_debug.is_empty() {
                    instance.source_debug.clone()
                } else {
                    instance.source.clone()
                }),
            );
            if let Some(item_template) = project.items.get(&instance.item_id) {
                item.set_attribute("class_name", Value::Str(item_template.name.clone()));
                item.set_attribute(
                    "creator_template_id",
                    Value::Str(item_template.id.to_string()),
                );
                rusterix::server::data::apply_item_data(&mut item, &item_template.data);
            }
            rusterix::server::data::apply_item_data(&mut item, &instance.data);
            item.set_attribute("_entity_configuration", Value::Str(instance.data.clone()));
            region.map.items.push(item);
        }
        rusterix::sync_block_prop_surface_prop_transforms(
            &mut region.map.block_prop_instances,
            &region.map.block_prop_surface_placements,
            block_props,
        );
        rusterix::sync_block_prop_surface_item_positions(
            &region.map.block_prop_instances,
            &region.map.block_prop_surface_placements,
            &mut region.map.items,
            block_props,
        );
    }
}

#[cfg(test)]
mod live_rules_tests {
    use super::*;

    #[test]
    fn runtime_content_uses_region_name_after_map_replacement() {
        let mut project = Project::new();
        assert_eq!(project.regions[0].map.name, project.regions[0].name);
        project.regions[0].name = "Britannia".into();
        project.regions[0].map = rusterix::Map::default();
        let map_id = project.regions[0].map.id;
        insert_content_into_maps(&mut project);
        assert_eq!(project.regions[0].map.name, "Britannia");
        assert_eq!(project.regions[0].map.id, map_id);
        project.regions[0].name = "Dungeon".into();
        insert_content_into_maps(&mut project);
        assert_eq!(project.regions[0].map.name, "Dungeon");
    }

    #[test]
    fn valid_edits_sync_assets_invalid_drafts_keep_last_valid_and_restore_syncs() {
        let mut assets = rusterix::server::assets::Assets::default();
        let mut project = Project::new();
        assert!(prepare_live_rules(&mut assets, &project).unwrap().is_some());
        let original = assets.rules.clone();
        assert!(!prepare_live_rules(&mut assets, &project).unwrap().is_some());
        project
            .rules_document
            .edit(|table| {
                table["actions"]["basic_attack"]
                    .as_table_mut()
                    .unwrap()
                    .insert(
                        "name".into(),
                        crate::rulesets::document::RulesValue::String("Live Robot Strike".into()),
                    );
                Ok(())
            })
            .unwrap();
        assert!(prepare_live_rules(&mut assets, &project).unwrap().is_some());
        let accepted = assets.rules.clone();
        assert!(accepted.contains("Live Robot Strike"));
        assert!(assets.textures.contains_key("basic_attack"));
        let tile =
            rusterix::Tile::from_texture(rusterix::Texture::new(vec![20, 80, 140, 255], 1, 1));
        let tile_id = tile.id;
        project.tiles.insert(tile_id, tile);
        project
            .rules_document
            .edit(|table| {
                table["icons"]["basic_attack"]
                    .as_table_mut()
                    .unwrap()
                    .insert(
                        "texture".into(),
                        crate::rulesets::document::RulesValue::String(tile_id.to_string()),
                    );
                Ok(())
            })
            .unwrap();
        assert!(prepare_live_rules(&mut assets, &project).unwrap().is_some());
        let mut item = rusterix::Item::default();
        item.attributes
            .set("icon", Value::Str("basic_attack".into()));
        assert_eq!(
            rusterix::client::widget::Widget::item_generated_icon_square(&assets, &item)
                .unwrap()
                .1,
            vec![20, 80, 140, 255]
        );
        let accepted = assets.rules.clone();
        project.rules_document.current = "[invalid".into();
        assert!(prepare_live_rules(&mut assets, &project).is_err());
        assert_eq!(assets.rules, accepted);
        project.rules_document.restore_original();
        assert!(prepare_live_rules(&mut assets, &project).unwrap().is_some());
        assert_eq!(assets.rules, original);
    }
}

#[cfg(test)]
mod unified_palette_tests {
    use super::*;
    use theframework::prelude::{TheColor, ThePalette};
    #[test]
    fn bundled_icons_and_world_tiles_fit_palette_without_mutating_source_or_custom_art() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let sources = project.tiles.clone();
        project.palette = ThePalette::empty_256();
        project.palette.colors[0] = Some(TheColor::from_u8(0, 0, 0, 255));
        project.palette.colors[1] = Some(TheColor::from_u8(255, 255, 255, 255));
        let mut assets = rusterix::Assets::default();
        insert_bundled_ruleset_item_icons(&mut assets, &project);
        assert!(!assets.palette_tile_overrides.is_empty());
        let first = assets.palette_tile_overrides.clone();
        for tile in first.values() {
            for image in &tile.textures {
                assert!(
                    image
                        .data
                        .chunks_exact(4)
                        .filter(|p| p[3] > 0)
                        .all(|p| p[..3] == [0, 0, 0] || p[..3] == [255, 255, 255])
                );
            }
        }
        assert_eq!(project.tiles, sources);
        project.palette.colors[0] = Some(TheColor::from_u8(255, 0, 0, 255));
        insert_bundled_ruleset_item_icons(&mut assets, &project);
        project.palette.colors[0] = Some(TheColor::from_u8(0, 0, 0, 255));
        insert_bundled_ruleset_item_icons(&mut assets, &project);
        assert_eq!(
            assets.palette_tile_overrides, first,
            "Switches rebuild from source pixels, not a quantized copy"
        );
        let item = project.items.values_mut().next().unwrap();
        item.icon_frames = vec![rusterix::Texture::from_color([1, 45, 200, 128])];
        let key = item.id.to_string().to_ascii_lowercase();
        insert_project_item_icons(&mut assets, &project);
        assert_eq!(assets.item_icons[&key][0].data, [1, 45, 200, 128]);
    }
}

#[cfg(test)]
mod scepter_display_template_tests {
    use super::*;
    #[test]
    fn offscreen_templates_resolve_the_same_equipment_without_starting_a_server() {
        let mut project = Project::default();
        for template in crate::rulesets::ruleset_item_templates_from_source(
            crate::rulesets::latest_official_ruleset(),
        )
        .unwrap()
        {
            let item = crate::item::Item {
                name: template.name,
                source: template.source,
                data: template.data,
                ..Default::default()
            };
            project.items.insert(item.id, item);
        }
        let mut assets = rusterix::Assets::default();
        assets.rules = project.rules_source().unwrap();
        assets.item_template_ids = project
            .items
            .values()
            .map(|i| (i.id, i.name.clone()))
            .collect();
        sync_display_templates(&mut assets, &project, false);
        let mut entity = rusterix::Entity::default();
        entity.set_attribute(
            "start_equipped_items",
            rusterix::Value::StrArray(vec!["Linen Shirt".into()]),
        );
        let displayed = rusterix::server::data::character_for_display(&entity, &assets);
        assert!(!displayed.equipped.is_empty());
        assert_eq!(assets.items.len(), project.items.len());
        assert!(
            entity.equipped.is_empty(),
            "Preview resolution must not mutate the authored instance"
        );
    }
}
