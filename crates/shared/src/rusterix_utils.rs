use crate::prelude::*;
use rusterix::{Command, Entity, Rusterix, Value};

/// Start the server
pub fn start_server(rusterix: &mut Rusterix, project: &mut Project, debug: bool) {
    rusterix.server.clear();
    rusterix.server.debug_mode = debug;
    rusterix.server.log_changed = true;

    insert_content_into_maps_mode(project, debug);
    rusterix.assets.rules = project.rules.clone();
    rusterix.assets.locales_src = project.locales.clone();
    rusterix.assets.audio_fx_src = project.audio_fx.clone();
    rusterix.assets.authoring_src = project.authoring.clone();
    rusterix.assets.read_locales();

    // Characters
    rusterix.assets.entities.clear();
    rusterix.assets.entity_authoring.clear();
    rusterix.assets.character_maps.clear();
    rusterix.assets.entity_tiles.clear();
    for character in project.characters.values_mut() {
        if !character.module.routines.is_empty() {
            if character.source.is_empty() {
                character.source = character.module.build(false);
            }
            if debug {
                character.source_debug = character.module.build(true);
            }
        }
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
    for item in project.items.values_mut() {
        if !item.module.routines.is_empty() {
            if item.source.is_empty() {
                item.source = item.module.build(false);
            }
            if debug {
                item.source_debug = item.module.build(true);
            }
        }
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
        rusterix.server.create_region_instance(
            region.name.clone(),
            region.map.clone(),
            &rusterix.assets,
            project.config.clone(),
        );
    }

    // Create the avatars
    rusterix.assets.avatars.clear();
    for avatar in &mut project.avatars.values() {
        rusterix
            .assets
            .avatars
            .insert(avatar.name.clone(), avatar.clone());
    }

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
    rusterix.assets.rules = project.rules.clone();
    rusterix.assets.locales_src = project.locales.clone();
    rusterix.assets.audio_fx_src = project.audio_fx.clone();
    rusterix.assets.authoring_src = project.authoring.clone();
    rusterix.assets.read_locales();
    rusterix.assets.palette = project.palette.clone();
    rusterix.assets.maps.clear();
    for region in &project.regions {
        rusterix
            .assets
            .maps
            .insert(region.map.name.clone(), region.map.clone());
    }
    rusterix.assets.screens.clear();
    for (_, screen) in &project.screens {
        let scr = screen.map.clone();
        rusterix.assets.screens.insert(screen.map.name.clone(), scr);
    }
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

pub fn insert_content_into_maps_mode(project: &mut Project, debug: bool) {
    for region in &mut project.regions {
        region.map.entities.clear();
        for instance in region.characters.values_mut() {
            if !instance.module.routines.is_empty() {
                if instance.source.is_empty() {
                    instance.source = instance.module.build(false);
                }
                if debug {
                    instance.source_debug = instance.module.build(true);
                }
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
            }
            region.map.entities.push(entity);
        }

        region.map.items.clear();
        for instance in region.items.values_mut() {
            if !instance.module.routines.is_empty() {
                if instance.source.is_empty() {
                    instance.source = instance.module.build(false);
                }
                if debug {
                    instance.source_debug = instance.module.build(true);
                }
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
            if let Some(character) = project.items.get(&instance.item_id) {
                item.set_attribute("class_name", Value::Str(character.name.clone()));
            }
            region.map.items.push(item);
        }
    }
}
