use crate::{Assets, Map, MsgParser, Tok};
use std::collections::HashMap;
use theframework::prelude::TheTime;

#[derive(Clone, Copy, Default)]
pub struct MessageContext {
    pub sender_entity: Option<u32>,
    pub sender_item: Option<u32>,
    pub receiver_entity: Option<u32>,
    pub world_time: Option<TheTime>,
}

fn normalize_locale(locale: &str) -> String {
    locale
        .trim()
        .replace('-', "_")
        .split('.')
        .next()
        .unwrap_or("en")
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_system_locale() -> Option<String> {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(normalize_locale(value));
            }
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn detect_system_locale() -> Option<String> {
    let window = web_sys::window()?;
    let navigator = window.navigator();
    navigator.language().map(|value| normalize_locale(&value))
}

trait LocaleAdapter {
    fn with_article_item(&self, name: &str, opts: &HashMap<String, String>) -> String;
    fn with_article_entity(&self, name: &str, opts: &HashMap<String, String>) -> String;
}

struct NoopLocale;
impl LocaleAdapter for NoopLocale {
    fn with_article_item(&self, name: &str, _opts: &HashMap<String, String>) -> String {
        name.to_string()
    }
    fn with_article_entity(&self, name: &str, _opts: &HashMap<String, String>) -> String {
        name.to_string()
    }
}

struct EnLocale;
impl EnLocale {
    fn indefinite_article(word: &str) -> &'static str {
        if word.is_empty() {
            return "a";
        }
        let an_ex = ["honest", "honor", "honour", "hour", "heir"];
        for ex in &an_ex {
            if word.starts_with(ex) {
                return "an";
            }
        }
        let a_ex_prefix = ["uni", "use", "euro", "one"]; // unicorn, user, euro, one-off
        for ex in &a_ex_prefix {
            if word.starts_with(ex) {
                return "a";
            }
        }
        match word.chars().next().unwrap_or('a').to_ascii_lowercase() {
            'a' | 'e' | 'i' | 'o' | 'u' => "an",
            _ => "a",
        }
    }
    fn is_pair_item(name: &str) -> bool {
        const PAIRS: [&str; 6] = [
            "trousers", "pants", "gloves", "boots", "scissors", "goggles",
        ];
        PAIRS.iter().any(|p| name.contains(p))
    }
    fn is_mass_item(name: &str) -> bool {
        const MASS: [&str; 8] = [
            "armor",
            "cloth",
            "water",
            "meat",
            "sand",
            "rice",
            "bread",
            "equipment",
        ];
        MASS.iter().any(|m| name.contains(m))
    }
}
impl LocaleAdapter for EnLocale {
    fn with_article_item(&self, name: &str, opts: &HashMap<String, String>) -> String {
        let mut value = name.to_string();
        if let Some(article) = opts.get("article").map(|s| s.as_str()) {
            let article = article.to_ascii_lowercase();
            match article.as_str() {
                "none" => {}
                "def" | "definite" => {
                    value = format!("the {}", value);
                }
                "indef" | "indefinite" | "undef" => {
                    let lower = value.to_ascii_lowercase();
                    if Self::is_pair_item(&lower) {
                        value = format!("a pair of {}", value);
                    } else if Self::is_mass_item(&lower) {
                        value = format!("some {}", value);
                    } else {
                        let art = Self::indefinite_article(&lower);
                        value = format!("{} {}", art, value);
                    }
                }
                _ => {}
            }
        }
        value
    }
    fn with_article_entity(&self, name: &str, opts: &HashMap<String, String>) -> String {
        self.with_article_item(name, opts)
    }
}

pub struct MsgResolver {
    locale: String,
    locales: HashMap<String, Box<dyn LocaleAdapter + Send + Sync>>, // runtime-swappable
}

impl Default for MsgResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl MsgResolver {
    pub fn new() -> Self {
        let mut locales: HashMap<String, Box<dyn LocaleAdapter + Send + Sync>> = HashMap::new();
        locales.insert("en".to_string(), Box::new(EnLocale));
        locales.insert("default".to_string(), Box::new(NoopLocale));
        Self {
            locale: "en".into(),
            locales,
        }
    }

    pub fn set_locale(&mut self, locale: &str) {
        let locale = locale.trim();
        if locale.eq_ignore_ascii_case("auto") {
            if let Some(system_locale) = detect_system_locale() {
                self.locale = system_locale;
            } else {
                self.locale = "en".to_string();
            }
        } else if !locale.is_empty() {
            self.locale = normalize_locale(locale);
        }
    }

    fn adapter(&self) -> &dyn LocaleAdapter {
        if let Some(ad) = self.locales.get(&self.locale) {
            &**ad
        } else if let Some(ad) = self.locales.get("default") {
            &**ad
        } else {
            &NoopLocale
        }
    }

    pub fn resolve(&self, tokens: Vec<Tok>, map: &Map, assets: &Assets) -> String {
        let parser = MsgParser::new();
        self.resolve_tokens(&parser, &tokens, map, assets, 0, false, None)
    }

    pub fn resolve_with_context(
        &self,
        tokens: Vec<Tok>,
        map: &Map,
        assets: &Assets,
        context: MessageContext,
    ) -> String {
        let parser = MsgParser::new();
        self.resolve_tokens(&parser, &tokens, map, assets, 0, false, Some(context))
    }

    fn resolve_tokens(
        &self,
        parser: &MsgParser,
        tokens: &[Tok],
        map: &Map,
        assets: &Assets,
        depth: usize,
        preserve_unknown_keys: bool,
        context: Option<MessageContext>,
    ) -> String {
        let mut string = String::new();
        let mut prev_wordy = false;

        for tok in tokens {
            let rendered = match tok {
                Tok::Plain(s) => s.clone(),
                Tok::TextKey { key, opts } => {
                    if let Some(world) = Self::resolve_world_key(key, context) {
                        Self::apply_case(&world, opts)
                    } else if let Some(base) = self.lookup_locale_text(assets, key) {
                        let substituted = self.apply_template_params(
                            &base, opts, parser, map, assets, depth, context,
                        );
                        let resolved = if depth < 8 && substituted.contains('{') {
                            let nested = parser.parse(&substituted);
                            self.resolve_tokens(
                                parser,
                                &nested,
                                map,
                                assets,
                                depth + 1,
                                true,
                                context,
                            )
                        } else {
                            substituted
                        };
                        Self::apply_case(&resolved, opts)
                    } else if preserve_unknown_keys {
                        Self::apply_case(&format!("{{{}}}", key), opts)
                    } else {
                        Self::apply_case(key, opts)
                    }
                }
                Tok::Num { val, opts } => Self::fmt_num(*val, opts),
                Tok::Float { val, opts } => Self::fmt_float(*val, opts),
                Tok::Entity { id, attr, opts } => {
                    let string = map
                        .entities
                        .iter()
                        .find(|entity| entity.id == *id)
                        .map(|entity| resolve_entity_attr(entity, attr, assets))
                        .unwrap_or_else(|| format!("Entity#{}:{}", id, attr));

                    let with_article = self.adapter().with_article_entity(&string, opts);
                    Self::apply_case(&with_article, opts)
                }
                Tok::Item { id, attr, opts } => {
                    // 1) Resolve base value for the requested attribute
                    let mut value = format!("Item#{}:{}", id, attr);
                    let mut found = false;

                    // Look in the world items first
                    for item in map.items.iter() {
                        if item.id == *id {
                            if let Some(attr_val) = item.attributes.get(&attr) {
                                value = format!("{}", attr_val);
                                found = true;
                                break;
                            }
                        }
                    }

                    // Look in entities' inventories
                    if !found {
                        'outer: for entity in map.entities.iter() {
                            for inv_item in entity.inventory.iter() {
                                if let Some(inv_item) = inv_item {
                                    if inv_item.id == *id {
                                        if let Some(attr_val) = inv_item.attributes.get(&attr) {
                                            value = format!("{}", attr_val);
                                            break 'outer;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let value = self.adapter().with_article_item(&value, opts);
                    Self::apply_case(&value, opts)
                }
            };

            let curr_wordy = Self::is_wordy(&rendered);

            if prev_wordy && curr_wordy {
                string.push(' ');
            }

            string.push_str(&rendered);
            prev_wordy = curr_wordy;
        }

        string
    }

    fn lookup_locale_text(&self, assets: &Assets, key: &str) -> Option<String> {
        for candidate in locale_candidates(&self.locale) {
            if let Some(map) = assets.locales.get(&candidate)
                && let Some(value) = map.get(key)
            {
                return Some(value.clone());
            }
        }
        None
    }

    fn apply_template_params(
        &self,
        template: &str,
        opts: &HashMap<String, String>,
        parser: &MsgParser,
        map: &Map,
        assets: &Assets,
        depth: usize,
        context: Option<MessageContext>,
    ) -> String {
        let mut rendered = template.to_string();
        for (key, value) in opts {
            if key == "case" {
                continue;
            }
            let resolved = self.resolve_option_value(value, parser, map, assets, depth, context);
            rendered = rendered.replace(&format!("{{{}}}", key), &resolved);
        }
        rendered
    }

    fn resolve_option_value(
        &self,
        value: &str,
        parser: &MsgParser,
        map: &Map,
        assets: &Assets,
        depth: usize,
        context: Option<MessageContext>,
    ) -> String {
        let trimmed = value.trim();
        let expr = self.expand_context_alias(trimmed, context);
        if matches_token_syntax(&expr) && depth < 8 {
            let wrapped = format!("{{{}}}", expr);
            let nested = parser.parse(&wrapped);
            return self.resolve_tokens(parser, &nested, map, assets, depth + 1, true, context);
        }
        expr
    }

    fn expand_context_alias(&self, value: &str, context: Option<MessageContext>) -> String {
        let Some(context) = context else {
            return value.to_string();
        };

        if value.len() >= 6 && value[..6].eq_ignore_ascii_case("world.") {
            return value.to_string();
        }

        for (prefix, id) in [
            ("self.", context.receiver_entity),
            ("sender.", context.sender_entity),
            ("attacker.", context.sender_entity),
            ("target.", context.sender_entity),
            ("item.", context.sender_item),
        ] {
            if let Some(rest) = value.strip_prefix(prefix)
                && let Some(id) = id
            {
                if prefix == "item." {
                    return format!("It:{}.{}", id, rest);
                }
                return format!("E:{}.{}", id, rest);
            }
        }

        value.to_string()
    }

    fn resolve_world_key(key: &str, context: Option<MessageContext>) -> Option<String> {
        let time = context?.world_time?;

        if key.eq_ignore_ascii_case("WORLD.HOUR") {
            return Some(time.hours.to_string());
        }
        if key.eq_ignore_ascii_case("WORLD.MINUTE") {
            return Some(format!("{:02}", time.minutes));
        }
        if key.eq_ignore_ascii_case("WORLD.TIME") || key.eq_ignore_ascii_case("WORLD.TIME_24") {
            return Some(format!("{:02}:{:02}", time.hours, time.minutes));
        }
        if key.eq_ignore_ascii_case("WORLD.TIME_12") {
            let mut hour = time.hours % 12;
            if hour == 0 {
                hour = 12;
            }
            let suffix = if time.hours < 12 { "AM" } else { "PM" };
            return Some(format!("{}:{:02} {}", hour, time.minutes, suffix));
        }

        None
    }

    fn apply_case(s: &str, opts: &HashMap<String, String>) -> String {
        let case_opt = opts.get("case").map(|c| c.to_ascii_lowercase());

        if let Some(case) = case_opt {
            match case.as_str() {
                "upper" | "uppercase" => return s.to_uppercase(),
                "lower" | "lowercase" => return s.to_lowercase(),
                "ucfirst" | "first" | "first_upper" => {
                    let mut c = s.chars();
                    if let Some(first) = c.next() {
                        return first.to_uppercase().collect::<String>() + c.as_str();
                    } else {
                        return s.to_string();
                    }
                }
                "title" => {
                    return s
                        .split_whitespace()
                        .map(|word| {
                            let mut c = word.chars();
                            if let Some(first) = c.next() {
                                first.to_uppercase().collect::<String>() + c.as_str()
                            } else {
                                word.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                }
                _ => {}
            }
        }

        // If no "case" key, check boolean-like flags in priority order
        if opts.contains_key("upper") {
            return s.to_uppercase();
        }
        if opts.contains_key("lower") {
            return s.to_lowercase();
        }
        if opts.contains_key("ucfirst")
            || opts.contains_key("first")
            || opts.contains_key("first_upper")
        {
            let mut c = s.chars();
            if let Some(first) = c.next() {
                return first.to_uppercase().collect::<String>() + c.as_str();
            } else {
                return s.to_string();
            }
        }
        if opts.contains_key("title") {
            return s
                .split_whitespace()
                .map(|word| {
                    let mut c = word.chars();
                    if let Some(first) = c.next() {
                        first.to_uppercase().collect::<String>() + c.as_str()
                    } else {
                        word.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
        }

        s.to_string()
    }

    fn is_wordy(s: &str) -> bool {
        // Consider wordy if it contains at least one alphanumeric character
        // and is not only punctuation or whitespace
        s.chars().any(|c| c.is_alphanumeric())
    }

    fn fmt_num(val: i64, opts: &HashMap<String, String>) -> String {
        if let Some(unit) = opts.get("unit") {
            format!("{} {}", val, unit)
        } else {
            val.to_string()
        }
    }

    fn fmt_float(val: f64, opts: &HashMap<String, String>) -> String {
        let precision = opts
            .get("precision")
            .and_then(|p| p.parse::<usize>().ok())
            .unwrap_or(2);
        let unit = opts.get("unit");

        let formatted = format!("{:.*}", precision, val);
        if let Some(unit) = unit {
            format!("{} {}", formatted, unit)
        } else {
            formatted
        }
    }
}

fn configured_slot_names(assets: &Assets, key: &str) -> Vec<String> {
    assets
        .config
        .parse::<toml::Table>()
        .ok()
        .and_then(|config| config.get("game").and_then(toml::Value::as_table).cloned())
        .and_then(|game| game.get(key).cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|slots| {
            slots
                .iter()
                .filter_map(toml::Value::as_str)
                .map(|slot| slot.trim().to_ascii_lowercase())
                .filter(|slot| !slot.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn is_weapon_slot(assets: &Assets, slot: &str) -> bool {
    let normalized = slot.trim().to_ascii_lowercase();
    let configured = configured_slot_names(assets, "weapon_slots");
    if !configured.is_empty() {
        return configured
            .iter()
            .any(|configured| configured == &normalized);
    }

    matches!(
        normalized.as_str(),
        "main_hand" | "mainhand" | "weapon" | "hand_main" | "off_hand" | "offhand" | "hand_off"
    )
}

fn is_gear_slot(assets: &Assets, slot: &str) -> bool {
    let normalized = slot.trim().to_ascii_lowercase();
    let configured = configured_slot_names(assets, "gear_slots");
    if !configured.is_empty() {
        return configured
            .iter()
            .any(|configured| configured == &normalized);
    }

    !is_weapon_slot(assets, slot)
}

fn fmt_scalar(value: f32) -> String {
    if (value - value.round()).abs() <= 0.0001 {
        (value.round() as i32).to_string()
    } else {
        format!("{:.2}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn configured_attr_name(assets: &Assets, key: &str, default: &str) -> String {
    assets
        .config
        .parse::<toml::Table>()
        .ok()
        .and_then(|config| {
            config
                .get("game")
                .and_then(toml::Value::as_table)
                .and_then(|game| game.get(key))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| default.to_string())
}

fn sum_equipped_attr(
    entity: &crate::Entity,
    assets: &Assets,
    attr: &str,
    filter: fn(&Assets, &str) -> bool,
) -> f32 {
    entity
        .equipped
        .iter()
        .filter(|(slot, _)| filter(assets, slot))
        .map(|(_, item)| item.attributes.get_float_default(attr, 0.0))
        .sum()
}

fn sum_all_equipped_attr(entity: &crate::Entity, attr: &str) -> f32 {
    entity
        .equipped
        .values()
        .map(|item| item.attributes.get_float_default(attr, 0.0))
        .sum()
}

fn resolve_entity_attr(entity: &crate::Entity, attr: &str, assets: &Assets) -> String {
    if let Some(inner) = attr.strip_prefix("weapon.") {
        return fmt_scalar(sum_equipped_attr(entity, assets, inner, is_weapon_slot));
    }
    if let Some(inner) = attr.strip_prefix("equipped.") {
        return fmt_scalar(sum_all_equipped_attr(entity, inner));
    }
    if let Some(inner) = attr.strip_prefix("armor.") {
        return fmt_scalar(sum_equipped_attr(entity, assets, inner, is_gear_slot));
    }
    if attr.eq_ignore_ascii_case("ATTACK") {
        return fmt_scalar(sum_equipped_attr(entity, assets, "DMG", is_weapon_slot));
    }
    if attr.eq_ignore_ascii_case("ARMOR") {
        return fmt_scalar(sum_equipped_attr(entity, assets, "ARMOR", is_gear_slot));
    }
    if attr.eq_ignore_ascii_case("LEVEL") {
        let level_attr = configured_attr_name(assets, "level", "LEVEL");
        return format!(
            "{}",
            entity
                .attributes
                .get_float_default(&level_attr, 1.0)
                .round() as i32
        );
    }
    if attr.eq_ignore_ascii_case("EXPERIENCE") || attr.eq_ignore_ascii_case("EXP") {
        let exp_attr = configured_attr_name(assets, "experience", "EXP");
        return format!(
            "{}",
            entity.attributes.get_float_default(&exp_attr, 0.0).round() as i32
        );
    }

    if let Some(attr_val) = entity.attributes.get(attr) {
        format!("{}", attr_val)
    } else {
        format!("Entity#{}:{}", entity.id, attr)
    }
}

fn locale_candidates(locale: &str) -> Vec<String> {
    let normalized = normalize_locale(locale);
    let mut candidates = vec![normalized.clone()];
    if let Some((base, _)) = normalized.split_once('_')
        && base != normalized
    {
        candidates.push(base.to_string());
    }
    if !candidates.iter().any(|candidate| candidate == "en") {
        candidates.push("en".to_string());
    }
    candidates
}

fn matches_token_syntax(value: &str) -> bool {
    ["E:", "I:", "It:", "Item:", "N:", "F:", "WORLD."]
        .iter()
        .any(|prefix| {
            value.len() >= prefix.len() && value[..prefix.len()].eq_ignore_ascii_case(prefix)
        })
}
