use crate::editor::CONFIGEDITOR;
use crate::prelude::*;
use rusterix::{PixelSource, Value, ValueContainer};
use toml::*;

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
pub fn get_source(_ui: &mut TheUI, server_ctx: &ServerContext) -> Option<Value> {
    let mut source: Option<Value> = None;

    if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
        if let Some(id) = server_ctx.curr_tile_id {
            source = Some(Value::Source(PixelSource::TileId(id)));
        }
    } else if server_ctx.curr_map_tool_helper == MapToolHelper::MaterialPicker {
        if let Some(id) = server_ctx.curr_material {
            source = Some(Value::Source(PixelSource::MaterialId(id)));
        }
    } /*
    else if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
    if let Some(palette_picker) = ui.get_palette_picker("Panel Palette Picker") {
    if let Some(color) = &PALETTE.read().unwrap().colors[palette_picker.index()] {
    source = Some(Value::Source(PixelSource::Color(color.clone())));
    }
    }
    }*/

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

    values.set("tile_size", Value::Int(config.tile_size));
}

/// Reads map relevant region settings from the TOML config and stores it in the map.
pub fn apply_region_config(map: &mut Map, config: String) {
    if let Ok(table) = config.parse::<Table>() {
        if let Some(rendering) = table.get("rendering").and_then(toml::Value::as_table) {
            if let Some(value) = rendering.get("receives_daylight") {
                if let Some(v) = value.as_bool() {
                    map.properties.set("receives_daylight", Value::Bool(v));
                }
            }
        }
    }
}
