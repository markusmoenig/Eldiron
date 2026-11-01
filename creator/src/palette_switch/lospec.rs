//! Lospec's public palette export: https://lospec.com/palettes/api
use serde::{Deserialize, Serialize};
use std::{io::Read, time::Duration};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    #[serde(default)]
    pub author: String,
    pub colors: Vec<String>,
}
pub fn slug(input: &str) -> Result<String, String> {
    let value = input.trim();
    let value = if value.contains("://") {
        value
            .strip_prefix("https://lospec.com/palette-list/")
            .or_else(|| value.strip_prefix("https://www.lospec.com/palette-list/"))
            .or_else(|| value.strip_prefix("http://lospec.com/palette-list/"))
            .ok_or("Use a Lospec palette name or palette-list URL")?
    } else {
        value
    };
    let value = value
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
        .trim_end_matches(".json");
    let result = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_ascii_lowercase();
    if result.is_empty()
        || result.len() > 120
        || !result
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(
            "Enter a palette name such as Endesga 32, Sweetie 16, or its Lospec URL".into(),
        );
    }
    Ok(result)
}
pub fn decode(text: &str) -> Result<Palette, String> {
    let result: Palette =
        serde_json::from_str(text).map_err(|_| "Lospec returned an invalid palette")?;
    if result.colors.is_empty()
        || result.colors.len() > 256
        || result
            .colors
            .iter()
            .any(|s| s.len() != 6 || !s.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        return Err("Lospec palettes must contain 1–256 RGB colors".into());
    }
    Ok(result)
}
pub fn fetch(slug: &str) -> Result<Palette, String> {
    let slug = self::slug(slug)?;
    let url = format!("https://lospec.com/palette-list/{slug}.json");
    let response = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build()
        .get(&url)
        .set("User-Agent", "Eldiron/0.95.6 palette importer")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => format!(
                "Palette '{slug}' was not found. Try its Lospec URL if its display name differs."
            ),
            ureq::Error::Status(429, _) => "Lospec is busy; please try again shortly.".into(),
            _ => format!("Could not load Lospec palette: {e}"),
        })?;
    let mut text = String::new();
    response
        .into_reader()
        .take(1_048_576)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    decode(&text)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_urls_resolve_to_the_public_export_slug() {
        for input in [
            "Endesga 32",
            "endesga-32",
            "https://lospec.com/palette-list/endesga-32",
            "https://www.lospec.com/palette-list/endesga-32.json?download=1",
        ] {
            assert_eq!(slug(input).unwrap(), "endesga-32");
        }
        for bad in ["", "https://example.com/a", "../../a", "db16/../../a"] {
            assert!(slug(bad).is_err());
        }
    }
    #[test]
    fn validates_palette_payload() {
        let p =
            decode(r#"{"name":"Example","author":"Artist","colors":["ff0040","131313"]}"#).unwrap();
        assert_eq!(p.colors.len(), 2);
        for bad in [
            r#"{"error":"file not found"}"#,
            r#"{"name":"Bad","colors":["bad hex"]}"#,
            r#"{"name":"Empty","colors":[]}"#,
        ] {
            assert!(decode(bad).is_err());
        }
    }
    #[test]
    #[ignore = "uses Lospec's public network API"]
    fn downloads_endesga_32() {
        let p = fetch("Endesga 32").unwrap();
        assert_eq!(p.colors.len(), 32);
        assert!(p.name.to_lowercase().contains("endesga"));
    }
}
