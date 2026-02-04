use crate::{Assets, Map, Tok};
use std::collections::HashMap;

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
        let mut string = String::new();
        let mut prev_wordy = false;

        for tok in &tokens {
            let rendered = match tok {
                Tok::Plain(s) => s.clone(),
                Tok::TextKey { key, opts } => {
                    let base = if let Some(map) = assets.locales.get(&self.locale) {
                        map.get(key).cloned().unwrap_or_else(|| key.clone())
                    } else {
                        key.clone()
                    };
                    Self::apply_case(&base, opts)
                }
                Tok::Num { val, opts } => Self::fmt_num(*val, opts),
                Tok::Float { val, opts } => Self::fmt_float(*val, opts),
                Tok::Entity { id, attr, opts } => {
                    let mut string = format!("Entity#{}:{}", id, attr);

                    for entity in map.entities.iter() {
                        if entity.id == *id {
                            if let Some(attr) = entity.attributes.get(&attr) {
                                string = format!("{}", attr);
                            }
                        }
                    }

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
