use crate::{Entity, Item, Light, LightType, PixelSource, Value};
use theframework::prelude::*;
use toml::Table;

/// Apply toml data to an Entity.
pub fn apply_entity_data(entity: &mut Entity, toml: &str) {
    match toml.parse::<Table>() {
        Ok(map) => {
            for (attr, v) in map.iter() {
                if attr == "attributes" {
                    if let Some(values) = v.as_table() {
                        for (key, value) in values {
                            if let Some(value) = value.as_float() {
                                entity.set_attribute(key, crate::Value::Float(value as f32));
                            } else if let Some(value) = value.as_integer() {
                                entity.set_attribute(key, crate::Value::Int(value as i32));
                            } else if let Some(value) = value.as_str() {
                                if key == "tile_id" {
                                    if let Ok(uuid) = Uuid::parse_str(value) {
                                        entity.set_attribute(
                                            "source",
                                            Value::Source(PixelSource::TileId(uuid)),
                                        );
                                    }
                                } else {
                                    entity.set_attribute(key, crate::Value::Str(value.to_string()));
                                }
                            } else if let Some(value) = value.as_bool() {
                                entity.set_attribute(key, crate::Value::Bool(value));
                            }
                        }
                    }
                } else if attr == "light" {
                    let mut light = Light::new(LightType::Point);
                    read_light(&mut light, v);
                    entity.set_attribute("light", crate::Value::Light(light));
                }
            }
        }
        Err(err) => {
            println!("error {:?}", err);
        }
    }
}

/// Apply toml data to an Item.
pub fn apply_item_data(item: &mut Item, toml: &str) {
    match toml.parse::<Table>() {
        Ok(map) => {
            for (attr, v) in map.iter() {
                if attr == "attributes" {
                    if let Some(values) = v.as_table() {
                        for (key, value) in values {
                            if let Some(value) = value.as_array() {
                                let mut values = vec![];
                                for v in value {
                                    values.push(v.to_string().replace("\"", ""));
                                }
                                item.set_attribute(key, crate::Value::StrArray(values));
                            } else if let Some(value) = value.as_float() {
                                item.set_attribute(key, crate::Value::Float(value as f32));
                            } else if let Some(value) = value.as_integer() {
                                item.set_attribute(key, crate::Value::Int(value as i32));
                            } else if let Some(value) = value.as_str() {
                                if key == "tile_id" {
                                    if let Ok(uuid) = Uuid::parse_str(value) {
                                        item.set_attribute(
                                            "source",
                                            Value::Source(PixelSource::TileId(uuid)),
                                        );
                                    }
                                } else if key == "color" {
                                    let color = hex_to_rgb_f32(value);
                                    item.set_attribute(
                                        "color",
                                        Value::Color(TheColor::from(color)),
                                    );
                                } else if key == "animation" {
                                    // Map human-readable animation names to the numeric codes used by billboards
                                    // 0=None, 1=OpenUp, 2=OpenRight, 3=OpenDown, 4=OpenLeft, 5=Fade
                                    let code = match value.to_ascii_lowercase().as_str() {
                                        "up" => 1,
                                        "right" => 2,
                                        "down" => 3,
                                        "left" => 4,
                                        "fade" => 5,
                                        _ => 0, // default/none
                                    };
                                    item.set_attribute(
                                        "billboard_animation",
                                        crate::Value::Int(code),
                                    );
                                } else if key == "animation_clock" {
                                    // "smooth" (render frames) or "frame"/"tick" (animation_frame ticks)
                                    item.set_attribute(
                                        "animation_clock",
                                        crate::Value::Str(value.to_ascii_lowercase()),
                                    );
                                } else if key == "animation_duration" {
                                    if let Ok(secs) = value.parse::<f32>() {
                                        item.set_attribute(
                                            "animation_duration",
                                            crate::Value::Float(secs),
                                        );
                                    }
                                } else {
                                    item.set_attribute(key, crate::Value::Str(value.to_string()));
                                }
                            } else if let Some(value) = value.as_bool() {
                                item.set_attribute(key, crate::Value::Bool(value));
                            } else if let Some(value) = value.as_integer() {
                                if key == "animation_duration" {
                                    item.set_attribute(
                                        "animation_duration",
                                        crate::Value::Float(value as f32),
                                    );
                                }
                            }
                        }
                    }
                } else if attr == "light" {
                    let mut light = Light::new(LightType::Point);
                    read_light(&mut light, v);
                    item.set_attribute("light", crate::Value::Light(light));
                }
            }
        }
        Err(err) => {
            println!("error {:?}", err);
        }
    }
}

/// Read a light from the toml
pub fn read_light(light: &mut Light, values: &toml::Value) {
    if let Some(toml::Value::Float(flicker)) = values.get("flicker") {
        light.set_flicker(*flicker as f32);
    }
    light.set_start_distance(0.0);
    if let Some(toml::Value::Float(range)) = values.get("range") {
        light.set_end_distance(*range as f32);
    }
    if let Some(toml::Value::Float(strength)) = values.get("strength") {
        light.set_intensity(*strength as f32);
    }
    if let Some(toml::Value::String(hex)) = values.get("color") {
        light.set_color(hex_to_rgb_f32(hex));
    }
}

/// Converts a hex color string  to an [f32; 3]
fn hex_to_rgb_f32(hex: &str) -> [f32; 3] {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return [1.0, 1.0, 1.0]; // Return white for invalid input
    }

    match (
        u8::from_str_radix(&hex[0..2], 16),
        u8::from_str_radix(&hex[2..4], 16),
        u8::from_str_radix(&hex[4..6], 16),
    ) {
        (Ok(r), Ok(g), Ok(b)) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
        _ => [1.0, 1.0, 1.0], // Return white for invalid input
    }
}
