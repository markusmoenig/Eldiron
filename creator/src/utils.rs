use crate::editor::CONFIGEDITOR;
use crate::editor::PALETTE;
use crate::prelude::*;
use rusterix::{Command, Entity, PixelSource, Rusterix, Value, ValueContainer};

/// Start the server
pub fn start_server(rusterix: &mut Rusterix, project: &mut Project) {
    rusterix.server.clear();
    rusterix.server.log_changed = true;

    insert_content_into_maps(project);

    // Characters
    rusterix.assets.entities = FxHashMap::default();
    for character in project.characters.values() {
        rusterix.assets.entities.insert(
            character.name.clone(),
            (character.source.clone(), character.data.clone()),
        );
    }

    // Items
    rusterix.assets.items = FxHashMap::default();
    for item in project.items.values() {
        rusterix
            .assets
            .items
            .insert(item.name.clone(), (item.source.clone(), item.data.clone()));
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

    // Wait for the region to be created
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
            if let Some(map) = project.get_map_mut(server_ctx) {
                for s in &map.sectors {
                    if s.creator_id == uuid {
                        if let Some(Value::Str(source)) = s.properties.get("source") {
                            ui.set_widget_value("CodeEdit", ctx, TheValue::Text(source.clone()));
                            success = true;
                        }
                        if let Some(Value::Str(data)) = s.properties.get("data") {
                            ui.set_widget_value("DataEdit", ctx, TheValue::Text(data.clone()));
                            success = true;
                        }
                        break;
                    }
                }
            }
        }
        ContentContext::CharacterTemplate(uuid) => {
            if let Some(character) = project.characters.get_mut(&uuid) {
                ui.set_widget_value("CodeEdit", ctx, TheValue::Text(character.source.clone()));
                ui.set_widget_value("DataEdit", ctx, TheValue::Text(character.data.clone()));
                success = true;
            }
        }
        ContentContext::ItemTemplate(uuid) => {
            if let Some(items) = project.items.get_mut(&uuid) {
                ui.set_widget_value("CodeEdit", ctx, TheValue::Text(items.source.clone()));
                ui.set_widget_value("DataEdit", ctx, TheValue::Text(items.data.clone()));
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

/// Returns the currently active source
pub fn get_source(ui: &mut TheUI, server_ctx: &ServerContext) -> Option<Value> {
    let mut source: Option<Value> = None;

    if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
        if let Some(id) = server_ctx.curr_tile_id {
            source = Some(Value::Source(PixelSource::TileId(id)));
        }
    } else if server_ctx.curr_map_tool_helper == MapToolHelper::ColorPicker {
        if let Some(palette_picker) = ui.get_palette_picker("Panel Palette Picker") {
            if let Some(color) = &PALETTE.read().unwrap().colors[palette_picker.index()] {
                source = Some(Value::Source(PixelSource::Color(color.clone())));
            }
        }
    }

    source
}

pub fn extract_build_values_from_config(values: &mut ValueContainer) {
    let config = CONFIGEDITOR.read().unwrap();
    let sample_mode = config.get_string_default("render", "sample_mode", "nearest");
    if sample_mode == "linear" {
        values.set(
            "sample_mode",
            Value::SampleMode(rusterix::SampleMode::Linear),
        );
    } else {
        values.set(
            "sample_mode",
            Value::SampleMode(rusterix::SampleMode::Nearest),
        );
    }
}
