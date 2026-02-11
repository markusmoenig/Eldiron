use crate::prelude::*;
use rusterix::{Command, Entity, Rusterix, Value};

/// Start the server
pub fn start_server(rusterix: &mut Rusterix, project: &mut Project, debug: bool) {
    rusterix.server.clear();
    rusterix.server.debug_mode = debug;
    rusterix.server.log_changed = true;
    rusterix.assets.global = project.render_graph.clone();

    insert_content_into_maps(project);

    // Characters
    rusterix.assets.entities.clear();
    rusterix.assets.character_maps.clear();
    rusterix.assets.entity_tiles.clear();
    for character in project.characters.values() {
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
        if !character.map.vertices.is_empty() {
            rusterix
                .assets
                .character_maps
                .insert(character.name.clone(), character.map.clone());
        }
    }

    // Items
    rusterix.assets.items.clear();
    rusterix.assets.item_maps.clear();
    rusterix.assets.item_tiles.clear();
    for item in project.items.values() {
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
}

/// Setup the client
pub fn setup_client(rusterix: &mut Rusterix, project: &mut Project) -> Vec<Command> {
    rusterix.assets.config = project.config.clone();
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
    for (_, asset) in project.assets.iter() {
        if let AssetBuffer::Font(bytes) = &asset.buffer {
            if let Ok(font) =
                fontdue::Font::from_bytes(bytes.clone(), fontdue::FontSettings::default())
            {
                rusterix.assets.fonts.insert(asset.name.clone(), font);
            }
        }
    }
    rusterix.setup_client()
}

/// Convert the characters and items into Entities / Items for the rusterix server
pub fn insert_content_into_maps(project: &mut Project) {
    for region in &mut project.regions {
        region.map.entities.clear();
        for instance in region.characters.values() {
            let mut entity = Entity {
                creator_id: instance.id,
                position: instance.position,
                ..Default::default()
            };
            entity.set_attribute("name", Value::Str(instance.name.clone()));
            if let Some(character_template) = project.characters.get(&instance.character_id) {
                entity.set_attribute("name", Value::Str(character_template.name.clone()));
            }
            entity.set_attribute("setup", Value::Str(instance.source.clone()));
            if let Some(character) = project.characters.get(&instance.character_id) {
                entity.set_attribute("class_name", Value::Str(character.name.clone()));
            }
            region.map.entities.push(entity);
        }

        region.map.items.clear();
        for instance in region.items.values() {
            let mut item = rusterix::Item {
                creator_id: instance.id,
                position: instance.position,
                ..Default::default()
            };
            item.set_attribute("name", Value::Str(instance.name.clone()));
            if let Some(item_template) = project.items.get(&instance.item_id) {
                item.set_attribute("name", Value::Str(item_template.name.clone()));
            }
            item.set_attribute("setup", Value::Str(instance.source.clone()));
            if let Some(character) = project.items.get(&instance.item_id) {
                item.set_attribute("class_name", Value::Str(character.name.clone()));
            }
            region.map.items.push(item);
        }
    }
}
