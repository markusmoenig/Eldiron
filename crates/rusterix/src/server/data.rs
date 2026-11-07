use crate::{
    Entity, Item, Light, LightType, PixelSource, PlayerCamera, Value,
    server::entity::{EntitySequence, EntitySequenceStep},
};
use indexmap::IndexMap;
use std::sync::{LazyLock, RwLock};
use theframework::prelude::*;
use toml::Table;

static TILE_ALIAS_LOOKUP: LazyLock<RwLock<FxHashMap<String, PixelSource>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

thread_local! {
    static SCOPED_TILE_ALIASES: std::cell::RefCell<Option<FxHashMap<String, PixelSource>>> = const {std::cell::RefCell::new(None)};
}
/// Offscreen snapshots must not overwrite the live server's alias table.
/// Nested scopes restore the caller's lookup even if rendering unwinds.
pub struct TileAliasScope {
    previous: Option<FxHashMap<String, PixelSource>>,
    same_thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl TileAliasScope {
    pub fn new(tiles: &IndexMap<Uuid, crate::Tile>) -> Self {
        let previous =
            SCOPED_TILE_ALIASES.with(|lookup| lookup.replace(Some(FxHashMap::default())));
        rebuild_tile_alias_lookup(tiles);
        Self {
            previous,
            same_thread: std::marker::PhantomData,
        }
    }
}
impl Drop for TileAliasScope {
    fn drop(&mut self) {
        SCOPED_TILE_ALIASES.with(|lookup| *lookup.borrow_mut() = self.previous.take());
    }
}

pub fn rebuild_tile_alias_lookup(tiles: &IndexMap<Uuid, crate::Tile>) {
    let mut lookup = FxHashMap::default();

    for (id, tile) in tiles {
        for alias in tile.alias.split([',', ';', '\n']) {
            let key = alias.trim().to_ascii_lowercase();
            if !key.is_empty() {
                lookup.insert(key, PixelSource::TileId(*id));
            }
        }
    }

    let scoped = SCOPED_TILE_ALIASES.with(|current| {
        let mut current = current.borrow_mut();
        if current.is_some() {
            *current = Some(lookup.clone());
            true
        } else {
            false
        }
    });
    if !scoped {
        *TILE_ALIAS_LOOKUP.write().unwrap() = lookup;
    }
}

fn lookup_tile_alias(value: &str) -> Option<PixelSource> {
    let key = value.trim().to_ascii_lowercase();
    if let Some(result) = SCOPED_TILE_ALIASES
        .with(|lookup| lookup.borrow().as_ref().map(|map| map.get(&key).cloned()))
    {
        return result;
    }
    TILE_ALIAS_LOOKUP
        .read()
        .unwrap()
        .get(&value.trim().to_ascii_lowercase())
        .cloned()
}

fn facing_to_orientation(facing: &str) -> Option<vek::Vec2<f32>> {
    match facing.trim().to_ascii_lowercase().as_str() {
        "front" | "north" | "n" => Some(vek::Vec2::new(0.0, -1.0)),
        "back" | "south" | "s" => Some(vek::Vec2::new(0.0, 1.0)),
        "right" | "east" | "e" => Some(vek::Vec2::new(1.0, 0.0)),
        "left" | "west" | "w" => Some(vek::Vec2::new(-1.0, 0.0)),
        _ => None,
    }
}

fn player_camera_from_str(camera: &str) -> Option<PlayerCamera> {
    match camera.trim().to_ascii_lowercase().as_str() {
        "2d" => Some(PlayerCamera::D2),
        "2d_grid" => Some(PlayerCamera::D2Grid),
        "iso" => Some(PlayerCamera::D3Iso),
        "iso_grid" => Some(PlayerCamera::D2Grid),
        "firstp" => Some(PlayerCamera::D3FirstP),
        "firstp_mouse" => Some(PlayerCamera::D3FirstPMouse),
        "firstp_grid" => Some(PlayerCamera::D3FirstPGrid),
        _ => None,
    }
}

fn parse_tile_source_from_toml(value: &toml::Value) -> Option<PixelSource> {
    if let Some(s) = value.as_str() {
        let trimmed = s.trim();
        if let Ok(uuid) = Uuid::parse_str(trimmed) {
            return Some(PixelSource::TileId(uuid));
        }
        if let Some(source) = lookup_tile_alias(trimmed) {
            return Some(source);
        }
        if let Ok(idx) = trimmed.parse::<u16>() {
            return Some(PixelSource::PaletteIndex(idx));
        }
        return None;
    }
    if let Some(i) = value.as_integer() {
        if i >= 0 && i <= u16::MAX as i64 {
            return Some(PixelSource::PaletteIndex(i as u16));
        }
    }
    None
}

fn parse_entity_sequences_from_toml(map: &Table) -> IndexMap<String, EntitySequence> {
    let mut sequences = IndexMap::new();

    let Some(behavior) = map.get("behavior").and_then(|value| value.as_table()) else {
        return sequences;
    };
    let Some(sequence_map) = behavior.get("sequences").and_then(|value| value.as_table()) else {
        return sequences;
    };

    for (name, value) in sequence_map {
        let Some(sequence_table) = value.as_table() else {
            continue;
        };
        let Some(step_values) = sequence_table
            .get("steps")
            .and_then(|value| value.as_array())
        else {
            continue;
        };

        let mut steps = Vec::new();
        for step_value in step_values {
            let Some(step_table) = step_value.as_table() else {
                continue;
            };
            let action = step_table
                .get("action")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if action.is_empty() {
                continue;
            }

            let target = step_table
                .get("target")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .unwrap_or_default();
            let value = step_table.get("value").and_then(|value| value.as_bool());
            let speed = step_table
                .get("speed")
                .and_then(|value| value.as_float().map(|value| value as f32))
                .or_else(|| {
                    step_table
                        .get("speed")
                        .and_then(|value| value.as_integer().map(|value| value as f32))
                });
            let seconds = step_table
                .get("seconds")
                .and_then(|value| value.as_float().map(|value| value as f32))
                .or_else(|| {
                    step_table
                        .get("seconds")
                        .and_then(|value| value.as_integer().map(|value| value as f32))
                });
            let intent = step_table
                .get("intent")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string());

            steps.push(EntitySequenceStep {
                action,
                target,
                value,
                speed,
                seconds,
                intent,
            });
        }

        if !steps.is_empty() {
            sequences.insert(name.trim().to_string(), EntitySequence { steps });
        }
    }

    sequences
}

pub(crate) fn parse_tile_source_from_str(value: &str) -> Option<PixelSource> {
    let trimmed = value.trim();
    if let Ok(uuid) = Uuid::parse_str(trimmed) {
        return Some(PixelSource::TileId(uuid));
    }
    if let Some(source) = lookup_tile_alias(trimmed) {
        return Some(source);
    }
    if let Ok(idx) = trimmed.parse::<u16>() {
        return Some(PixelSource::PaletteIndex(idx));
    }
    None
}

/// Apply toml data to an Entity.
pub fn apply_entity_data(entity: &mut Entity, toml: &str) {
    match toml.parse::<Table>() {
        Ok(map) => {
            entity.sequences = parse_entity_sequences_from_toml(&map);
            for (attr, v) in map.iter() {
                if attr == "attributes" {
                    if let Some(values) = v.as_table() {
                        for (key, value) in values {
                            if let Some(value) = value.as_array() {
                                let mut values = vec![];
                                for v in value {
                                    values.push(v.to_string().replace("\"", ""));
                                }
                                entity.set_attribute(key, crate::Value::StrArray(values));
                            } else if let Some(value) = value.as_float() {
                                entity.set_attribute(key, crate::Value::Float(value as f32));
                            } else if let Some(value) = value.as_integer() {
                                entity.set_attribute(key, crate::Value::Int(value as i32));
                            } else if let Some(value) = value.as_str() {
                                if key == "tile_id" {
                                    if let Some(source) = parse_tile_source_from_str(value) {
                                        entity.set_attribute("source", Value::Source(source));
                                    } else {
                                        entity.set_attribute(
                                            key,
                                            crate::Value::Str(value.to_string()),
                                        );
                                    }
                                } else if key == "facing" {
                                    entity.set_attribute(key, crate::Value::Str(value.to_string()));
                                    if let Some(orientation) = facing_to_orientation(value) {
                                        entity.orientation = orientation;
                                    }
                                } else if key == "player_camera" {
                                    if let Some(camera) = player_camera_from_str(value) {
                                        entity
                                            .set_attribute(key, crate::Value::PlayerCamera(camera));
                                    } else {
                                        entity.set_attribute(
                                            key,
                                            crate::Value::Str(value.to_string()),
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
                // Support common top-level item keys used by rigging preview/data docs.
                // Runtime should mirror preview behavior for these fields.
                if apply_item_top_level(item, attr, v) {
                    continue;
                }
                if attr == "attributes" {
                    if let Some(values) = v.as_table() {
                        for (key, value) in values {
                            if matches!(key.as_str(), "rig_pivot" | "decal_scale")
                                && let Some(value) = value.as_array()
                                && value.len() == 2
                                && let (Some(x), Some(y)) =
                                    (value[0].as_float(), value[1].as_float())
                            {
                                item.set_attribute(key, crate::Value::Vec2([x as f32, y as f32]));
                            } else if let Some(value) = value.as_array() {
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
                                    if let Some(source) = parse_tile_source_from_str(value) {
                                        item.set_attribute("source", Value::Source(source));
                                    } else {
                                        item.set_attribute(
                                            key,
                                            crate::Value::Str(value.to_string()),
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
                    item.apply_container_attributes();
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

fn apply_item_top_level(item: &mut Item, key: &str, value: &toml::Value) -> bool {
    let tile_keys = [
        "tile_id",
        "tile_id_front",
        "tile_id_front_right",
        "tile_id_right",
        "tile_id_back_right",
        "tile_id_back",
        "tile_id_back_left",
        "tile_id_left",
        "tile_id_front_left",
        "rig_tile_id",
        "rig_tile_id_front",
        "rig_tile_id_front_right",
        "rig_tile_id_right",
        "rig_tile_id_back_right",
        "rig_tile_id_back",
        "rig_tile_id_back_left",
        "rig_tile_id_left",
        "rig_tile_id_front_left",
    ];
    if tile_keys.contains(&key)
        && let Some(source) = parse_tile_source_from_toml(value)
    {
        if key == "tile_id" || key == "rig_tile_id" {
            item.set_attribute("source", Value::Source(source.clone()));
        }
        item.set_attribute(key, Value::Source(source));
        return true;
    }

    if key == "rig_scale"
        && let Some(v) = value.as_float()
    {
        item.set_attribute("rig_scale", Value::Float(v as f32));
        return true;
    }

    if key == "rig_pivot"
        && let Some(v) = value.as_array()
        && v.len() == 2
        && let (Some(x), Some(y)) = (v[0].as_float(), v[1].as_float())
    {
        item.set_attribute("rig_pivot", Value::Vec2([x as f32, y as f32]));
        return true;
    }

    if key == "rig_layer"
        && let Some(v) = value.as_str()
    {
        item.set_attribute("rig_layer", Value::Str(v.to_string()));
        return true;
    }

    if key == "rig_flip_back"
        && let Some(v) = value.as_bool()
    {
        item.set_attribute("rig_flip_back", Value::Bool(v));
        return true;
    }

    if key == "slot"
        && let Some(v) = value.as_str()
    {
        item.set_attribute("slot", Value::Str(v.to_string()));
        return true;
    }

    // Allow spell configuration directly at top-level as well as inside [attributes].
    if key.starts_with("spell_") || key == "on_cast" || key == "is_spell" || key == "reagents" {
        if let Some(v) = value.as_float() {
            item.set_attribute(key, Value::Float(v as f32));
            return true;
        }
        if let Some(v) = value.as_integer() {
            item.set_attribute(key, Value::Int(v as i32));
            return true;
        }
        if let Some(v) = value.as_bool() {
            item.set_attribute(key, Value::Bool(v));
            return true;
        }
        if let Some(v) = value.as_str() {
            item.set_attribute(key, Value::Str(v.to_string()));
            return true;
        }
        if let Some(arr) = value.as_array() {
            let mut values = vec![];
            for v in arr {
                values.push(v.to_string().replace("\"", ""));
            }
            item.set_attribute(key, Value::StrArray(values));
            return true;
        }
    }

    false
}

/// Read a light from the toml
pub fn read_light(light: &mut Light, values: &toml::Value) {
    if let Some(toml::Value::Float(flicker)) = values.get("flicker") {
        light.set_flicker(*flicker as f32);
    }
    if let Some(lift) = values.get("lift").and_then(|value| {
        value
            .as_float()
            .or_else(|| value.as_integer().map(|v| v as f64))
    }) {
        light
            .properties
            .set("lift", crate::Value::Float(lift as f32));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_labels_map_to_authoring_orientation() {
        assert_eq!(
            facing_to_orientation("north"),
            Some(vek::Vec2::new(0.0, -1.0))
        );
        assert_eq!(
            facing_to_orientation("front"),
            Some(vek::Vec2::new(0.0, -1.0))
        );
        assert_eq!(
            facing_to_orientation("south"),
            Some(vek::Vec2::new(0.0, 1.0))
        );
        assert_eq!(
            facing_to_orientation("east"),
            Some(vek::Vec2::new(1.0, 0.0))
        );
        assert_eq!(
            facing_to_orientation("west"),
            Some(vek::Vec2::new(-1.0, 0.0))
        );
    }

    #[test]
    fn entity_light_data_supports_vertical_lift() {
        let values: toml::Value = toml::from_str(
            r##"
[light]
color = "#ffad52"
strength = 4.2
range = 4.8
flicker = 0.1
lift = 1.15
"##,
        )
        .expect("light data parses");
        let mut light = Light::new(LightType::Point);

        read_light(&mut light, &values["light"]);

        assert!((light.get_lift() - 1.15).abs() < 0.0001);
        assert!((light.get_intensity() - 4.2).abs() < 0.0001);
        assert!((light.get_end_distance() - 4.8).abs() < 0.0001);
    }

    #[test]
    fn item_data_preserves_avatar_decal_scale_as_a_vector() {
        let mut item = Item::new();

        apply_item_data(
            &mut item,
            r#"
[attributes]
dye_color = "linen"
decal_scale = [2.0, 3.0]
"#,
        );

        assert_eq!(item.attributes.get_str("dye_color"), Some("linen"));
        assert_eq!(item.attributes.get_vec2("decal_scale"), Some([2.0, 3.0]));
    }

    #[test]
    fn display_defaults_cache_preserves_live_state_and_tracks_authoring_edits() {
        let mut assets = crate::Assets::default();
        assets.rules = eldiron_ruleset::latest_official_ruleset().into();
        for template in eldiron_ruleset::ruleset_item_templates_from_source(&assets.rules).unwrap()
        {
            assets
                .items
                .insert(template.name, (template.source, template.data));
        }
        let mut entity = Entity::default();
        entity.set_attribute(
            "start_equipped_items",
            Value::StrArray(vec!["Linen Shirt".into()]),
        );
        fn serialized(entity: &Entity) -> serde_json::Value {
            fn normalize(value: &mut serde_json::Value) {
                match value {
                    serde_json::Value::Object(fields) => {
                        for (key, value) in fields {
                            if key == "dirty_attributes" {
                                // These serialize from unordered sets.
                                value.as_array_mut().unwrap().sort_by_key(|v| v.to_string());
                            } else {
                                normalize(value);
                            }
                        }
                    }
                    serde_json::Value::Array(values) => values.iter_mut().for_each(normalize),
                    _ => {}
                }
            }
            let mut value = serde_json::to_value(entity).unwrap();
            normalize(&mut value);
            value
        }
        let check = |entity: &Entity, assets: &crate::Assets| {
            let actual = character_for_display(entity, assets);
            let expected = prepare_character_for_display(entity, assets);
            assert_eq!(serialized(&actual), serialized(&expected));
            assert_eq!(actual.interp_target_position, entity.interp_target_position);
            assert_eq!(actual.interp_remaining, entity.interp_remaining);
            assert!(matches!(actual.action, crate::EntityAction::Forward));
            actual
        };
        entity.action = crate::EntityAction::Forward;
        let first = check(&entity, &assets);
        assert!(
            !first.equipped.is_empty(),
            "Exercise actual default equipment"
        );
        assert_eq!(assets.display_defaults.read().unwrap().len(), 1);
        entity.position.x = 8.;
        entity.orientation = vek::Vec2::new(0., -1.);
        entity.interp_target_position = Some(vek::Vec3::new(9., 1., 0.));
        entity.interp_remaining = 0.2;
        entity.inventory.push(Some(Item::default()));
        entity.dirty_flags = 1;
        entity.dirty_attributes.insert("runtime".into());
        check(&entity, &assets);
        assert_eq!(
            assets.display_defaults.read().unwrap().len(),
            1,
            "Motion must reuse setup"
        );
        entity.set_attribute("race", Value::Str("orc".into()));
        check(&entity, &assets);
        entity.equipped = first.equipped;
        check(&entity, &assets);
        // Assets clones share caches, but their edited data must never reuse
        // another snapshot's equipment or ruleset defaults.
        let mut edited = assets.clone();
        edited
            .items
            .get_mut("Linen Shirt")
            .unwrap()
            .1
            .push_str("\n[cache_test]\nvalue = 1\n");
        entity.equipped.clear();
        check(&entity, &edited);
        edited
            .item_template_ids
            .insert(uuid::Uuid::new_v4(), "Linen Shirt".into());
        check(&entity, &edited);
        edited
            .rules
            .push_str("\n[attributes.defaults.cache_test]\nvalue = 2\n");
        check(&entity, &edited);
        check(&entity, &assets);
        assert_eq!(assets.display_defaults.read().unwrap().len(), 6);
    }
}

/// Resolve the character setup for an editor display without spawning gameplay items.
/// The same class defaults, template lookup, permissions and occupied slots are used at startup.
pub fn character_for_display(entity: &Entity, assets: &crate::Assets) -> Entity {
    use super::assets::DisplayDefaults;
    use std::sync::Arc;
    let key = crate::procedural_avatar::fingerprint(&(
        serde_json::to_vec(&(&entity.attributes, &entity.equipped)).unwrap_or_default(),
        &assets.rules,
        assets
            .items
            .iter()
            .map(|(name, (_, data))| (name, data))
            .collect::<Vec<_>>(),
        assets.item_template_ids.iter().collect::<Vec<_>>(),
    ));
    let cached = assets
        .display_defaults
        .read()
        .ok()
        .and_then(|cache| cache.get(&key).cloned());
    let defaults = cached.unwrap_or_else(|| {
        // Cache only setup-derived fields. Motion, position, inventory, actions
        // and current equipment remain owned by the caller on every display.
        let mut clean = entity.clone();
        clean.dirty_flags = 0;
        clean.dirty_attributes.clear();
        let prepared = prepare_character_for_display(&clean, assets);
        let defaults = Arc::new(DisplayDefaults {
            attributes: prepared.attributes,
            equipped: prepared
                .equipped
                .into_iter()
                .filter(|(slot, _)| !entity.equipped.contains_key(slot))
                .collect(),
            dirty_flags: prepared.dirty_flags,
            dirty_attributes: prepared.dirty_attributes,
        });
        if let Ok(mut cache) = assets.display_defaults.write() {
            if cache.len() >= 128 {
                cache.clear();
            }
            cache.insert(key, defaults.clone());
        }
        defaults
    });
    let mut displayed = entity.clone();
    displayed.attributes = defaults.attributes.clone();
    displayed.equipped.extend(defaults.equipped.clone());
    displayed.dirty_flags |= defaults.dirty_flags;
    displayed
        .dirty_attributes
        .extend(defaults.dirty_attributes.iter().cloned());
    displayed
}

fn prepare_character_for_display(entity: &Entity, assets: &crate::Assets) -> Entity {
    let mut entity = entity.clone();
    let Some(rules) = assets.rules_table() else {
        return entity;
    };
    super::region::apply_ruleset_character_defaults(&rules, &mut entity);
    let policy = eldiron_ruleset::resolve_equipment_policy(&rules).unwrap_or_default();
    let names = match entity.attributes.get("start_equipped_items") {
        Some(Value::StrArray(names)) => names.clone(),
        Some(Value::Str(name)) => vec![name.clone()],
        _ => Vec::new(),
    };
    for (index, name) in names.iter().enumerate() {
        let Some(class) = assets.resolve_item_class_name(name) else {
            continue;
        };
        let mut item = Item {
            id: index as u32 + 1,
            creator_id: assets
                .item_template_ids
                .iter()
                .find_map(|(id, name)| (name == &class).then_some(*id))
                .unwrap_or_default(),
            item_type: class.clone(),
            ..Default::default()
        };
        item.set_attribute("name", Value::Str(class.clone()));
        item.set_attribute("class_name", Value::Str(class.clone()));
        apply_item_data(&mut item, &assets.items[&class].1);
        let Some(slot) = item.attributes.get_str("slot").map(str::to_string) else {
            continue;
        };
        let kind = super::region::equipment_item_kind(&policy, &item);
        let category = item.attributes.get_str("category");
        if policy
            .check_item(entity.attributes.get_str("class"), &kind, category, &slot)
            .is_err()
        {
            continue;
        }
        let occupied = policy.occupied_slots(&kind, category, &slot);
        if entity.equipped.iter().any(|(other_slot, other)| {
            let kind = super::region::equipment_item_kind(&policy, other);
            !occupied.is_disjoint(&policy.occupied_slots(
                &kind,
                other.attributes.get_str("category"),
                other_slot,
            ))
        }) {
            continue;
        }
        entity.equipped.insert(slot, item);
    }
    entity
}

#[cfg(test)]
mod tile_alias_scope_tests {
    use super::*;
    #[test]
    fn scepter_offscreen_tile_alias_scopes_are_thread_local_and_restore_after_unwind() {
        let tiles = |id| {
            let mut tiles = IndexMap::default();
            tiles.insert(
                id,
                crate::Tile {
                    id,
                    alias: "scepter-isolation-test".into(),
                    ..Default::default()
                },
            );
            tiles
        };
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let _outer = TileAliasScope::new(&tiles(a));
        assert_eq!(
            lookup_tile_alias("scepter-isolation-test"),
            Some(PixelSource::TileId(a))
        );
        std::thread::spawn(move || {
            assert_eq!(lookup_tile_alias("scepter-isolation-test"), None);
            {
                let _scope = TileAliasScope::new(&tiles(b));
                assert_eq!(
                    lookup_tile_alias("scepter-isolation-test"),
                    Some(PixelSource::TileId(b))
                );
            }
            assert_eq!(lookup_tile_alias("scepter-isolation-test"), None);
        })
        .join()
        .unwrap();
        let _ = std::panic::catch_unwind(|| {
            let _nested = TileAliasScope::new(&tiles(b));
            panic!("Test scope cleanup");
        });
        assert_eq!(
            lookup_tile_alias("scepter-isolation-test"),
            Some(PixelSource::TileId(a))
        );
    }
}
