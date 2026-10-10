use crate::prelude::*;
use rusterix::{Command, Entity, Rusterix, Value};

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
            for (id, texture) in textures {
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
    assets.item_icons.clear();
    assets.project_item_icon_keys.clear();
    match crate::rulesets::bundled_item_icon_states_for_project(&project.config) {
        Ok(states) => {
            for (item_id, state, frames) in states {
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
}

fn insert_project_item_icons(assets: &mut rusterix::server::assets::Assets, project: &Project) {
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

        if !item.icon_frames.is_empty() {
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
        if !item.icon_off_frames.is_empty() {
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
    insert_bundled_ruleset_item_icons(&mut rusterix.assets, project);
    insert_project_item_icons(&mut rusterix.assets, project);
    rusterix.set_overlay_dirty();
}

/// Refresh the visual asset subset used by Creator previews and tree icons.
/// This deliberately does not start or mutate the game server.
pub fn sync_editor_visual_assets(rusterix: &mut Rusterix, project: &Project) {
    rusterix.assets.config = project.config.clone();
    rusterix.assets.rules = project.rules_source().unwrap_or_else(|_| String::new());
    rusterix.assets.ruleset_palette = project.palette.clone();
    rusterix.assets.palette = project.art_palette.clone();
    rusterix.assets.read_rules_metadata();
    rusterix.set_tiles(project.tiles.clone(), true);
    rusterix.set_tile_groups(project.tile_groups.clone());

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

    // Characters
    rusterix.assets.entities.clear();
    rusterix.assets.entity_authoring.clear();
    rusterix.assets.character_maps.clear();
    rusterix.assets.entity_tiles.clear();
    for character in project.characters.values_mut() {
        if debug && !character.source_debug.is_empty() {
            rusterix.assets.entities.insert(
                character.name.clone(),
                (character.source_debug.clone(), character.data.clone()),
            );
        } else {
            rusterix.assets.entities.insert(
                character.name.clone(),
                (character.source.clone(), character.data.clone()),
            );
        }
        rusterix
            .assets
            .entity_authoring
            .insert(character.name.clone(), character.authoring.clone());
        if !character.map.vertices.is_empty() {
            rusterix
                .assets
                .character_maps
                .insert(character.name.clone(), character.map.clone());
        }
    }

    // Items
    rusterix.assets.items.clear();
    rusterix.assets.item_authoring.clear();
    rusterix.assets.item_maps.clear();
    rusterix.assets.item_tiles.clear();
    sync_item_icon_assets(rusterix, project);
    for item in project.items.values_mut() {
        if debug && !item.source_debug.is_empty() {
            rusterix.assets.items.insert(
                item.name.clone(),
                (item.source_debug.clone(), item.data.clone()),
            );
        } else {
            rusterix
                .assets
                .items
                .insert(item.name.clone(), (item.source.clone(), item.data.clone()));
        }
        rusterix
            .assets
            .item_authoring
            .insert(item.name.clone(), item.authoring.clone());
        if !item.map.vertices.is_empty() {
            rusterix
                .assets
                .item_maps
                .insert(item.name.clone(), item.map.clone());
        }
    }

    // Create the regions
    for region in &mut project.regions {
        let region_source = if debug && !region.source_debug.is_empty() {
            region.source_debug.clone()
        } else {
            region.source.clone()
        };
        rusterix
            .assets
            .region_sources
            .insert(region.map.id, region_source);
        let region_config = crate::project::merge_config_toml(&project.config, &region.config);
        rusterix.server.create_region_instance(
            region.name.clone(),
            region.map.clone(),
            &rusterix.assets,
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
    rusterix.assets.ruleset_palette = project.palette.clone();
    rusterix.assets.palette = project.art_palette.clone();
    rusterix.assets.palette_materials = project
        .art_palette_materials
        .iter()
        .map(|m| m.rmoe_values())
        .collect();
    rusterix.assets.palette_material_ids = project
        .art_palette_materials
        .iter()
        .map(|m| m.material_id())
        .collect();
    rusterix.assets.set_procedural_material_sources(
        project
            .procedural_materials
            .iter()
            .map(|(alias, source)| (alias.as_str(), source.as_str())),
    );
    rusterix.assets.set_procedural_sdf_sources(
        project
            .procedural_sdfs
            .iter()
            .map(|(alias, source)| (alias.as_str(), source.as_str())),
    );
    rusterix.assets.maps.clear();
    for region in &project.regions {
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
    fn valid_edits_sync_assets_invalid_drafts_keep_last_valid_and_restore_syncs() {
        let mut assets = rusterix::server::assets::Assets::default();
        let mut project = Project::new();
        assert!(prepare_live_rules(&mut assets, &project).unwrap().is_some());
        let original = assets.rules.clone();
        assert!(!prepare_live_rules(&mut assets, &project).unwrap().is_some());
        let branch = project
            .rules
            .current
            .branches
            .iter_mut()
            .find(|b| crate::rulesets::graph::branch_path(b) == Some("/actions/basic_attack"))
            .unwrap();
        let node = branch
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("rules_definition"))
            .unwrap();
        let theframework::thegraph::GraphControlValue::List { rows, .. } = &mut node.rows[1].value
        else {
            panic!()
        };
        let field = rows
            .iter_mut()
            .find(|r| r[0] == theframework::thegraph::GraphControlValue::Text("name".into()))
            .unwrap();
        field[2] = theframework::thegraph::GraphControlValue::Text("Live Robot Strike".into());
        assert!(prepare_live_rules(&mut assets, &project).unwrap().is_some());
        let accepted = assets.rules.clone();
        assert!(accepted.contains("Live Robot Strike"));
        assert!(assets.textures.contains_key("basic_attack"));
        let tile =
            rusterix::Tile::from_texture(rusterix::Texture::new(vec![20, 80, 140, 255], 1, 1));
        let tile_id = tile.id;
        project.tiles.insert(tile_id, tile);
        let branch = project
            .rules
            .current
            .branches
            .iter_mut()
            .find(|b| crate::rulesets::graph::branch_path(b) == Some("/icons/basic_attack"))
            .unwrap();
        let node = branch
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("rules_definition"))
            .unwrap();
        let theframework::thegraph::GraphControlValue::List { columns, rows, .. } =
            &mut node.rows[1].value
        else {
            panic!()
        };
        let mut cells: Vec<_> = columns.iter().map(|c| c.control.clone()).collect();
        cells[0] = theframework::thegraph::GraphControlValue::Text("texture".into());
        cells[2] = theframework::thegraph::GraphControlValue::Text(tile_id.to_string());
        rows.push(cells);
        crate::rulesets::graph::sync_ports(branch);
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
        project
            .rules
            .current
            .branches
            .push(project.rules.current.branches[0].clone());
        assert!(prepare_live_rules(&mut assets, &project).is_err());
        assert_eq!(assets.rules, accepted);
        project.rules.restore_original();
        assert!(prepare_live_rules(&mut assets, &project).unwrap().is_some());
        assert_eq!(assets.rules, original);
    }
}
