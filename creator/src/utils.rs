use crate::prelude::*;
use rusterix::{Entity, Rusterix, Value};

/// Start the server
pub fn start_server(rusterix: &mut Rusterix, project: &mut Project) {
    rusterix.server.clear_log();
    rusterix.server.log_changed = true;

    insert_characters_into_maps(project);

    // Characters
    rusterix.assets.entities = FxHashMap::default();
    for character in project.characters.values() {
        rusterix
            .assets
            .entities
            .insert(character.name.clone(), character.source.clone());
    }

    // Items
    rusterix.assets.items = FxHashMap::default();
    for item in project.items.values() {
        rusterix
            .assets
            .items
            .insert(item.name.clone(), item.source.clone());
    }

    for region in &mut project.regions {
        rusterix.server.create_region_instance(
            region.name.clone(),
            region.map.clone(),
            &rusterix.assets,
        );
    }

    rusterix.server.set_state(rusterix::ServerState::Running);
}

/// Convert the characters into Entities for the rusterix server
pub fn insert_characters_into_maps(project: &mut Project) {
    for region in &mut project.regions {
        region.map.entities.clear();

        for instance in region.characters.values() {
            let mut entity = Entity {
                creator_id: instance.id,
                position: instance.position,
                ..Default::default()
            };
            entity.set_attribute("name", Value::Str(instance.name.clone()));
            entity.set_attribute("setup", Value::Str(instance.source.clone()));
            if let Some(character) = project.characters.get(&instance.character_id) {
                entity.set_attribute("class_name", Value::Str(character.name.clone()));
            }
            region.map.entities.push(entity);
        }
    }
}

/// Sets the code for the code editor based on the current editor mode
pub fn set_code(
    ui: &mut TheUI,
    ctx: &mut TheContext,
    project: &mut Project,
    server_ctx: &ServerContext,
) {
    let mut success = false;

    match server_ctx.cc {
        ContentContext::CharacterInstance(uuid) => {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                if let Some(character_instance) = region.characters.get_mut(&uuid) {
                    ui.set_widget_value(
                        "CodeEdit",
                        ctx,
                        TheValue::Text(character_instance.source.clone()),
                    );
                    success = true;
                }
            }
        }
        ContentContext::Sector(uuid) => {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                for s in &region.map.sectors {
                    if s.creator_id == uuid {
                        ui.set_widget_value("CodeEdit", ctx, TheValue::Text(String::new()));
                        success = true;
                        break;
                    }
                }
            }
        }
        ContentContext::CharacterTemplate(uuid) => {
            if let Some(character) = project.characters.get_mut(&uuid) {
                ui.set_widget_value("CodeEdit", ctx, TheValue::Text(character.source.clone()));
                success = true;
            }
        }
        ContentContext::ItemTemplate(uuid) => {
            if let Some(items) = project.items.get_mut(&uuid) {
                ui.set_widget_value("CodeEdit", ctx, TheValue::Text(items.source.clone()));
                success = true;
            }
        }
        _ => {}
    }

    /*
    let sidebarmode = SIDEBARMODE.read().unwrap();
    if *sidebarmode == SidebarMode::Region {
        if let Some(region_content_id) = server_ctx.curr_region_content {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                // Check for Character Instance
                if let Some(character_instance) = region.characters.get_mut(&region_content_id) {
                    ui.set_widget_value(
                        "CodeEdit",
                        ctx,
                        TheValue::Text(character_instance.source.clone()),
                    );
                    success = true;
                } else {
                    // Check for Sector
                    for s in &region.map.sectors {
                        if s.creator_id == region_content_id {
                            ui.set_widget_value("CodeEdit", ctx, TheValue::Text(String::new()));
                        }
                    }
                }
            }
        }
    } else if *sidebarmode == SidebarMode::Character {
        if let Some(character_id) = &server_ctx.curr_character {
            if let Some(character) = project.characters.get(character_id) {
                ui.set_widget_value("CodeEdit", ctx, TheValue::Text(character.source.clone()));
                success = true;
            }
        }
    }*/

    if !success {
        ui.set_widget_value("CodeEdit", ctx, TheValue::Text(String::new()));
    }
}
