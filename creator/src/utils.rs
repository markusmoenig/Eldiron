use crate::editor::{CODEEDITOR, CONFIGEDITOR, PALETTE, SCENEMANAGER};
use crate::prelude::*;
use codegridfx::{Module, ModuleType};
use rusteria::{RenderBuffer, Rusteria};
use rusterix::{PixelSource, Value, ValueContainer, pixel_to_vec4};
use std::str::FromStr;
use toml::*;

pub fn default_preview_rigging_toml() -> String {
    if let Some(bytes) = crate::Embedded::get("toml/preview_rigging.toml")
        && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
    {
        return source.to_string();
    }
    "# Editor-only preview. Runtime equipment/animation comes from server scripts.\nanimation = \"Idle\"\nperspective = \"Front\"\nplay = true\nspeed = 1.0\n\n[slots]\n# main_hand = \"Item Name\"\n# off_hand = \"Item Name\"\n".to_string()
}

pub fn update_region_settings(
    project: &mut Project,
    server_ctx: &ServerContext,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut changed = false;
    if let Some(id) = server_ctx.pc.id() {
        if server_ctx.pc.is_region() {
            if let Some(region) = project.get_region_mut(&id) {
                let parsed: toml::Value = toml::from_str(&region.config)?;
                // Parse [render] section
                if let Some(section) = parsed.get("terrain") {
                    if let Some(enabled) = section.get("enabled") {
                        let enabled = enabled.as_bool().unwrap_or(false);
                        // let existing = region
                        //     .map
                        //     .properties
                        //     .get_bool_default("terain_enabled", false);
                        // if enabled != existing {
                        region
                            .map
                            .properties
                            .set("terrain_enabled", Value::Bool(enabled));
                        changed = true;
                        // }
                    }

                    let mut got_tile_id = false;
                    if let Some(enabled) = section.get("tile_id") {
                        if let Some(tile_id) = enabled.as_str() {
                            if let Ok(id) = Uuid::from_str(tile_id) {
                                region.map.properties.set(
                                    "default_terrain_tile",
                                    Value::Source(PixelSource::TileId(id)),
                                );
                                got_tile_id = true;
                            }
                        }
                    }
                    if !got_tile_id {
                        region.map.properties.remove("default_terrain_tile");
                    }
                }
            }
        }
    }

    Ok(changed)
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

                    CODEEDITOR.write().unwrap().set_module_character_instance(
                        ui,
                        ctx,
                        character_instance,
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
            CODEEDITOR.write().unwrap().clear_module(ui, ctx);
        }
        ContentContext::CharacterTemplate(uuid) => {
            if let Some(character) = project.characters.get_mut(&uuid) {
                ui.set_widget_value("CodeEdit", ctx, TheValue::Text(character.source.clone()));
                ui.set_widget_value("DataEdit", ctx, TheValue::Text(character.data.clone()));
                character
                    .module
                    .set_module_type(ModuleType::CharacterTemplate);
                CODEEDITOR
                    .write()
                    .unwrap()
                    .set_module_character(ui, ctx, character);
                success = true;
            }
        }
        ContentContext::ItemTemplate(uuid) => {
            if let Some(item) = project.items.get_mut(&uuid) {
                ui.set_widget_value("CodeEdit", ctx, TheValue::Text(item.source.clone()));
                ui.set_widget_value("DataEdit", ctx, TheValue::Text(item.data.clone()));
                item.module.set_module_type(ModuleType::ItemTemplate);
                CODEEDITOR.write().unwrap().set_module_item(ui, ctx, item);
                success = true;
            }
        }
        ContentContext::ItemInstance(uuid) => {
            let mut temp_id = None;

            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                if let Some(item_inst) = region.items.get_mut(&uuid) {
                    temp_id = Some(item_inst.item_id);
                }
            }

            if let Some(temp_id) = temp_id {
                if let Some(item) = project.items.get_mut(&temp_id) {
                    ui.set_widget_value("CodeEdit", ctx, TheValue::Text(item.source.clone()));
                    ui.set_widget_value("DataEdit", ctx, TheValue::Text(item.data.clone()));
                    item.module.set_module_type(ModuleType::ItemTemplate);
                    CODEEDITOR.write().unwrap().set_module_item(ui, ctx, item);
                    success = true;
                }
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
pub fn get_source(_ui: &mut TheUI, server_ctx: &ServerContext) -> Option<PixelSource> {
    let mut source: Option<PixelSource> = None;

    if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
        if let Some(id) = server_ctx.curr_tile_id {
            source = Some(PixelSource::TileId(id));
        }
    } /*else if server_ctx.curr_map_tool_helper == MapToolHelper::MaterialPicker {
    if let Some(id) = server_ctx.curr_material {
    source = Some(Value::Source(PixelSource::MaterialId(id)));
    }
    }
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
}

/// Reads map relevant region settings from the TOML config and stores it in the map.
pub fn apply_region_config(map: &mut Map, config: String) {
    if let Ok(table) = config.parse::<Table>() {
        if let Some(rendering) = table.get("rendering").and_then(toml::Value::as_table) {
            // Daylight
            if let Some(value) = rendering.get("receives_daylight") {
                if let Some(v) = value.as_bool() {
                    map.properties.set("receives_daylight", Value::Bool(v));
                }
            }

            // Fog
            if let Some(value) = rendering.get("fog_enabled") {
                if let Some(v) = value.as_bool() {
                    map.properties.set("fog_enabled", Value::Bool(v));
                }
            }
            if let Some(value) = rendering.get("fog_start_distance") {
                if let Some(v) = value.as_float() {
                    map.properties
                        .set("fog_start_distance", Value::Float(v as f32));
                }
            }
            if let Some(value) = rendering.get("fog_end_distance") {
                if let Some(v) = value.as_float() {
                    map.properties
                        .set("fog_end_distance", Value::Float(v as f32));
                }
            }
            let mut fog_color = Vec4::zero();
            if let Some(value) = rendering.get("fog_color") {
                if let Some(v) = value.as_str() {
                    let c = hex_to_rgba_u8(v);
                    fog_color = pixel_to_vec4(&c);
                }
            }
            map.properties.set(
                "fog_color",
                Value::Vec4([fog_color.x, fog_color.y, fog_color.z, fog_color.w]),
            );
        }
    }
}

/// Converts an hex string to a vec4 color
pub fn hex_to_rgba_u8(hex: &str) -> [u8; 4] {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        6 => match (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            (Ok(r), Ok(g), Ok(b)) => [r, g, b, 255],
            _ => [255, 255, 255, 255],
        },
        8 => match (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
            u8::from_str_radix(&hex[6..8], 16),
        ) {
            (Ok(r), Ok(g), Ok(b), Ok(a)) => [r, g, b, a],
            _ => [255, 255, 255, 255],
        },
        _ => [255, 255, 255, 255],
    }
}

/// Checks if the string is a valid python variable name
pub fn is_valid_python_variable(name: &str) -> bool {
    // Must not be empty, must start with a letter or underscore, and only contain letters, digits, or underscores
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
        _ => return false,
    }
    if name.is_empty() {
        return false;
    }
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        // Python keywords are not valid variable names
        const PYTHON_KEYWORDS: &[&str] = &[
            "False", "None", "True", "and", "as", "assert", "break", "class", "continue", "def",
            "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
            "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try",
            "while", "with", "yield",
        ];
        !PYTHON_KEYWORDS.contains(&name)
    } else {
        false
    }
}

/// Draws the shader into the given buffer
pub fn draw_shader_into(module: &Module, buffer: &mut TheRGBABuffer) {
    use std::sync::{Arc, Mutex};

    let mut rs = Rusteria::default();
    let code = module.build_shader();

    let _module = match rs.parse_str(&code) {
        Ok(module) => match rs.compile(&module) {
            Ok(()) => {
                println!("Module '{}' compiled successfully.", module.name);
            }
            Err(e) => {
                eprintln!("Error compiling module: {e}");
                return;
            }
        },
        Err(e) => {
            eprintln!("Error parsing module: {e}");
            return;
        }
    };

    let width = buffer.dim().width as usize;
    let height = buffer.dim().height as usize;

    if let Some(shade_index) = rs.context.program.shade_index {
        let mut rbuffer = Arc::new(Mutex::new(RenderBuffer::new(width, height)));
        let t0 = rs.get_time();
        rs.shade(&mut rbuffer, shade_index, &PALETTE.read().unwrap());
        let t1 = rs.get_time();
        println!("Rendered in {}ms", t1 - t0);

        let b = rbuffer.lock().unwrap().as_rgba_bytes();
        *buffer = TheRGBABuffer::from(b, width as u32, height as u32)
    }
}

/// Renders the current map in the scenemanager depending on the current viewmode.
pub fn scenemanager_render_map(project: &Project, server_ctx: &ServerContext) {
    if server_ctx.editor_view_mode == EditorViewMode::D2 {
        // In 2D we render the current map, profile or base
        if let Some(map) = project.get_map(server_ctx) {
            SCENEMANAGER.write().unwrap().set_map(map.clone());
        }
    } else {
        // In 3D we always only render the base map
        if let Some(id) = server_ctx.pc.id() {
            if let Some(region) = project.get_region(&id) {
                SCENEMANAGER.write().unwrap().set_map(region.map.clone());
            }
        }
    }
}
