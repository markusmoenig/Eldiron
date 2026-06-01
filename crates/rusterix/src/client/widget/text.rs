use crate::{
    Assets, Currencies, Map, MsgParser, Pixel, Rect, WHITE,
    client::{
        draw2d,
        resolver::{MessageContext, MsgResolver},
    },
};
use draw2d::Draw2D;
use regex::Regex;
use theframework::prelude::*;

fn substitute_placeholders<F>(input: &str, mut resolver: F) -> String
where
    F: FnMut(&str, &str) -> Option<String>,
{
    let re = Regex::new(r"\{([A-Z_]+)\.([A-Z0-9_\.]+)\}").unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        let category = &caps[1];
        let key = &caps[2];
        resolver(category, key).unwrap_or_else(|| format!("{{{}.{}?}}", category, key))
    })
    .to_string()
}

pub struct TextWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub font: Option<fontdue::Font>,
    pub font_size: f32,
    pub messages: Vec<(String, Pixel)>,
    pub draw2d: Draw2D,
    pub spacing: f32,
    pub tab_width: f32,
    pub table: toml::Table,
    pub text: String,
    pub color: Pixel,
    pub horizontal_align: draw2d::TheHorizontalAlign,
    pub parser: MsgParser,
    pub resolver: MsgResolver,
}

impl Default for TextWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl TextWidget {
    const CORE_RULE_NUMBERS: [&'static str; 16] = [
        "HP", "MAX_HP", "MP", "MAX_MP", "STR", "DEX", "INT", "WIS", "VIT", "DMG", "POWER", "ARMOR",
        "RESIST", "INIT", "SPEED", "LEVEL",
    ];

    pub fn new() -> Self {
        Self {
            name: String::new(),
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            font: None,
            font_size: 20.0,
            messages: vec![],
            draw2d: Draw2D::default(),
            spacing: 1.0,
            tab_width: 120.0,
            table: toml::Table::default(),
            text: String::new(),
            color: WHITE,
            horizontal_align: draw2d::TheHorizontalAlign::Left,
            parser: MsgParser::new(),
            resolver: MsgResolver::default(),
        }
    }

    pub fn init(&mut self, assets: &Assets) {
        let mut font_name = String::new();
        if let Ok(config) = assets.config.parse::<toml::Table>() {
            if let Some(locale) = config
                .get("game")
                .and_then(toml::Value::as_table)
                .and_then(|game| game.get("locale"))
                .and_then(toml::Value::as_str)
            {
                self.resolver.set_locale(locale);
            }
        }
        if let Ok(table) = self.toml_str.parse::<toml::Table>() {
            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("font") {
                    if let Some(v) = value.as_str() {
                        font_name = v.into();
                    }
                }
                if let Some(value) = ui.get("font_size") {
                    if let Some(v) = value.as_float() {
                        self.font_size = v as f32;
                    }
                }
                if let Some(value) = ui.get("spacing") {
                    if let Some(v) = value.as_float() {
                        self.spacing = v as f32;
                    }
                }
                if let Some(value) = ui.get("tab_width") {
                    if let Some(v) = value.as_float() {
                        self.tab_width = v as f32;
                    }
                }
                if let Some(value) = ui.get("text") {
                    if let Some(v) = value.as_str() {
                        self.text = v.into();
                    }
                }
                if let Some(value) = ui.get("color") {
                    if let Some(v) = value.as_str() {
                        self.color = self.hex_to_rgba_u8(v);
                    }
                }
                if let Some(value) = ui
                    .get("align")
                    .or_else(|| ui.get("horizontal_align"))
                    .and_then(toml::Value::as_str)
                {
                    self.horizontal_align = match value.trim().to_ascii_lowercase().as_str() {
                        "center" | "centre" => draw2d::TheHorizontalAlign::Center,
                        "right" => draw2d::TheHorizontalAlign::Right,
                        _ => draw2d::TheHorizontalAlign::Left,
                    };
                }
            }
            self.table = table;
        }

        if let Some(font) = assets.fonts.get(&font_name) {
            self.font = Some(font.clone());
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &Map,
        currencies: &Currencies,
        assets: &Assets,
        time: &TheTime,
        ui_state: &FxHashMap<String, String>,
    ) {
        if let Some(font) = &self.font {
            let stride = buffer.stride();
            let mut y = self.rect.y;
            let player = map.entities.iter().find(|entity| entity.is_player());
            let player_id = player.map(|entity| entity.id);

            let width = buffer.dim().width;
            let height = buffer.dim().height;

            for line in self.text.lines() {
                let legacy = substitute_placeholders(line, |cat, key| {
                    match cat {
                        "PLAYER" => {
                            if let Some(entity) = player {
                                if key == "MONEY" {
                                    return Some(currencies.format_base_amount(
                                        entity.wallet.get_balance(currencies),
                                    ));
                                } else if key == "FUNDS" {
                                    return Some(entity.wallet.get_balance(currencies).to_string());
                                } else if key == "CLASS" || key == "CLASS_NAME" {
                                    return Self::player_attr_value(entity, "class")
                                        .or_else(|| Self::player_attr_value(entity, "class_name"));
                                } else if key == "RACE" {
                                    return Self::player_attr_value(entity, "race");
                                } else if key == "LEVEL" || key == "EXP" || key == "EXPERIENCE" {
                                    let token =
                                        format!("{{E:{}.{} }}", entity.id, key).replace(" }", "}");
                                    return Some(self.resolver.resolve_with_context(
                                        self.parser.parse(&token),
                                        map,
                                        assets,
                                        MessageContext {
                                            sender_entity: player_id,
                                            sender_item: None,
                                            receiver_entity: player_id,
                                            world_time: Some(*time),
                                        },
                                    ));
                                } else if key == "ATTACK" {
                                    return Some(self.resolver.resolve_with_context(
                                        self.parser.parse(&format!("{{E:{}.ATTACK}}", entity.id)),
                                        map,
                                        assets,
                                        MessageContext {
                                            sender_entity: player_id,
                                            sender_item: None,
                                            receiver_entity: player_id,
                                            world_time: Some(*time),
                                        },
                                    ));
                                } else if key == "ARMOR" {
                                    return Some(self.resolver.resolve_with_context(
                                        self.parser.parse(&format!("{{E:{}.ARMOR}}", entity.id)),
                                        map,
                                        assets,
                                        MessageContext {
                                            sender_entity: player_id,
                                            sender_item: None,
                                            receiver_entity: player_id,
                                            world_time: Some(*time),
                                        },
                                    ));
                                } else if key.starts_with("WEAPON.")
                                    || key.starts_with("EQUIPPED.")
                                    || key.starts_with("ARMOR.")
                                {
                                    let token =
                                        format!("{{E:{}.{} }}", entity.id, key).replace(" }", "}");
                                    return Some(self.resolver.resolve_with_context(
                                        self.parser.parse(&token),
                                        map,
                                        assets,
                                        MessageContext {
                                            sender_entity: player_id,
                                            sender_item: None,
                                            receiver_entity: player_id,
                                            world_time: Some(*time),
                                        },
                                    ));
                                } else if let Some(value) = Self::player_attr_value(entity, key) {
                                    return Some(value);
                                } else if Self::CORE_RULE_NUMBERS.contains(&key) {
                                    return Some("0".to_string());
                                }
                            }
                            None
                        }
                        "START" => Self::start_ui_value(assets, ui_state, key),
                        // "WORLD" => map.world.get_value(key),
                        _ => None,
                    }
                });
                let resolved = self.resolver.resolve_with_context(
                    self.parser.parse(&legacy),
                    map,
                    assets,
                    MessageContext {
                        sender_entity: player_id,
                        sender_item: None,
                        receiver_entity: player_id,
                        world_time: Some(*time),
                    },
                );

                if resolved.contains('\t') {
                    for (index, segment) in resolved.split('\t').enumerate() {
                        let x = self.rect.x + self.tab_width * index as f32;
                        let seg_width = (self.rect.width - (x - self.rect.x)).max(0.0);
                        let tuple = (
                            x.floor() as isize,
                            y.floor() as isize,
                            seg_width.floor() as isize,
                            self.font_size as isize,
                        );
                        self.draw2d.text_rect_blend_safe(
                            buffer.pixels_mut(),
                            &tuple,
                            stride,
                            font,
                            self.font_size,
                            segment,
                            &self.color,
                            draw2d::TheHorizontalAlign::Left,
                            draw2d::TheVerticalAlign::Center,
                            &(0, 0, width as isize, height as isize),
                        );
                    }
                } else {
                    let tuple = (
                        self.rect.x as isize,
                        y.floor() as isize,
                        self.rect.width as isize,
                        self.font_size as isize,
                    );

                    self.draw2d.text_rect_blend_safe(
                        buffer.pixels_mut(),
                        &tuple,
                        stride,
                        font,
                        self.font_size,
                        &resolved,
                        &self.color,
                        self.horizontal_align.clone(),
                        draw2d::TheVerticalAlign::Center,
                        &(0, 0, width as isize, height as isize),
                    );
                }

                y += self.font_size + self.spacing;
            }
        }
    }

    fn start_ui_value(
        assets: &Assets,
        ui_state: &FxHashMap<String, String>,
        key: &str,
    ) -> Option<String> {
        let class = ui_state
            .get("start.class")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("Warrior");

        match key {
            "NAME" => ui_state
                .get("start.name")
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            "CLASS" => Some(class.to_string()),
            "CLASS_DESCRIPTION" => Self::ruleset_class_string(assets, class, "description"),
            "CLASS_ROLE" => Self::ruleset_class_string(assets, class, "role")
                .map(|value| Self::humanize(&value)),
            "CLASS_ATTRIBUTES" => Self::ruleset_class_array(assets, class, "primary_attributes")
                .map(|values| {
                    if values.is_empty() {
                        "-".to_string()
                    } else {
                        values.join(", ")
                    }
                }),
            "CLASS_WEAPONS" => {
                Self::ruleset_class_array(assets, class, "allowed_weapons").map(|values| {
                    if values.is_empty() {
                        "None".to_string()
                    } else {
                        Self::human_list(values)
                    }
                })
            }
            "CLASS_ARMOR" => {
                Self::ruleset_class_array(assets, class, "allowed_armor").map(|values| {
                    if values.is_empty() {
                        "None".to_string()
                    } else {
                        Self::human_list(values)
                    }
                })
            }
            "CLASS_ABILITIES" => {
                Self::ruleset_class_array(assets, class, "abilities").map(|values| {
                    if values.is_empty() {
                        "None".to_string()
                    } else {
                        Self::human_list(values)
                    }
                })
            }
            "CLASS_SPELLS" => Self::ruleset_class_array(assets, class, "spells").map(|values| {
                if values.is_empty() {
                    "None".to_string()
                } else {
                    Self::human_list(values)
                }
            }),
            "CLASS_EQUIPMENT" => {
                Self::ruleset_class_loadout(assets, class, &["equipment", "weapons", "armor"]).map(
                    |values| {
                        if values.is_empty() {
                            "None".to_string()
                        } else {
                            Self::human_list(values)
                        }
                    },
                )
            }
            "CLASS_INVENTORY" => {
                Self::ruleset_class_loadout(assets, class, &["inventory", "items"]).map(|values| {
                    if values.is_empty() {
                        "None".to_string()
                    } else {
                        Self::human_list(values)
                    }
                })
            }
            _ => None,
        }
    }

    fn ruleset_class_table(assets: &Assets, class: &str) -> Option<toml::value::Table> {
        let root = toml::from_str::<toml::Value>(&assets.rules).ok()?;
        root.get("classes")?
            .as_table()?
            .get(class.trim())?
            .as_table()
            .cloned()
    }

    fn ruleset_class_string(assets: &Assets, class: &str, key: &str) -> Option<String> {
        Self::ruleset_class_table(assets, class)?
            .get(key)?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn ruleset_class_array(assets: &Assets, class: &str, key: &str) -> Option<Vec<String>> {
        let class = Self::ruleset_class_table(assets, class)?;
        Some(Self::toml_string_array(class.get(key)?))
    }

    fn ruleset_class_loadout(assets: &Assets, class: &str, keys: &[&str]) -> Option<Vec<String>> {
        let class = Self::ruleset_class_table(assets, class)?;
        let loadout = class.get("starting_loadout")?.as_table()?;
        let mut values = Vec::new();
        for key in keys {
            if let Some(value) = loadout.get(*key) {
                for entry in Self::toml_string_array(value) {
                    if !values.iter().any(|existing| existing == &entry) {
                        values.push(entry);
                    }
                }
            }
        }
        Some(values)
    }

    fn human_list(values: Vec<String>) -> String {
        values
            .into_iter()
            .map(|value| Self::humanize(&value))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn humanize(value: &str) -> String {
        value
            .split(['_', '-', '.', ' '])
            .filter(|part| !part.trim().is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => {
                        let mut word = first.to_uppercase().collect::<String>();
                        word.push_str(&chars.as_str().to_ascii_lowercase());
                        word
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn toml_string_array(value: &toml::Value) -> Vec<String> {
        value
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn player_attr_value(entity: &crate::Entity, key: &str) -> Option<String> {
        entity
            .attributes
            .get(key)
            .or_else(|| {
                entity
                    .attributes
                    .keys()
                    .find(|candidate| candidate.eq_ignore_ascii_case(key))
                    .and_then(|candidate| entity.attributes.get(candidate))
            })
            .map(|value| value.to_string())
    }

    /// Converts a hex color string to a [u8; 4] (RGBA).
    /// Accepts "#RRGGBB" or "#RRGGBBAA" formats.
    fn hex_to_rgba_u8(&self, hex: &str) -> [u8; 4] {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_start_class_placeholders_from_rules() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [classes.Warrior]
            role = "martial"
            primary_attributes = ["STR", "VIT"]
            allowed_weapons = ["sword", "axe"]
            allowed_armor = ["cloth", "leather", "chain"]
            abilities = ["basic_attack", "guard"]
            spells = []

            [classes.Warrior.starting_loadout]
            weapons = ["training_sword"]
            armor = ["padded_armor"]
        "#
        .to_string();

        let mut ui_state = FxHashMap::default();
        ui_state.insert("start.class".to_string(), "Warrior".to_string());
        ui_state.insert("start.name".to_string(), "Eldiron".to_string());

        assert_eq!(
            TextWidget::start_ui_value(&assets, &ui_state, "CLASS_ROLE").as_deref(),
            Some("Martial")
        );
        assert_eq!(
            TextWidget::start_ui_value(&assets, &ui_state, "CLASS_ATTRIBUTES").as_deref(),
            Some("STR, VIT")
        );
        assert_eq!(
            TextWidget::start_ui_value(&assets, &ui_state, "CLASS_EQUIPMENT").as_deref(),
            Some("Training Sword, Padded Armor")
        );
    }
}
