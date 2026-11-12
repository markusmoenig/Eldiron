//! Authored speech policy shared by reference and native presentation.
use crate::{Pixel, utils::hex_to_rgba_u8};
use toml::Table;
pub fn ticks_per_minute(config: &Table) -> u32 {
    config
        .get("game")
        .and_then(|v| v.get("ticks_per_minute"))
        .and_then(toml::Value::as_integer)
        .unwrap_or(4)
        .clamp(1, u32::MAX as i64) as u32
}
pub fn duration_minutes(say: &Table) -> Option<f32> {
    say.get("duration")
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .map(|v| (v as f32).max(0.))
}
pub fn duration_ticks(config: &Table, local: Option<&Table>) -> i64 {
    let minutes = local
        .and_then(duration_minutes)
        .or_else(|| {
            config
                .get("say")
                .and_then(toml::Value::as_table)
                .and_then(duration_minutes)
        })
        .unwrap_or(1.);
    ((minutes * ticks_per_minute(config) as f32).round() as i64).max(1)
}
fn category_color(say: &Table, category: &str) -> Option<Pixel> {
    [category, "default", ""]
        .into_iter()
        .find_map(|key| say.get(key).and_then(toml::Value::as_str))
        .map(hex_to_rgba_u8)
}
pub fn color(config: &Table, local: Option<&Table>, category: &str, fallback: Pixel) -> Pixel {
    local
        .and_then(|say| category_color(say, category))
        .or_else(|| {
            config
                .get("say")
                .and_then(toml::Value::as_table)
                .and_then(|say| category_color(say, category))
        })
        .unwrap_or(fallback)
}
pub fn background_enabled(config: &Table, local: Option<&Table>) -> bool {
    local
        .and_then(|say| say.get("background_enabled"))
        .and_then(toml::Value::as_bool)
        .or_else(|| {
            config
                .get("say")
                .and_then(|say| say.get("background_enabled"))
                .and_then(toml::Value::as_bool)
        })
        .unwrap_or(true)
}
fn background_from_table(say: &Table) -> Option<Pixel> {
    ["background_color", "background"]
        .into_iter()
        .find_map(|key| say.get(key).and_then(toml::Value::as_str))
        .map(hex_to_rgba_u8)
}
pub fn background_color(config: &Table, local: Option<&Table>) -> Pixel {
    local
        .and_then(background_from_table)
        .or_else(|| {
            config
                .get("say")
                .and_then(toml::Value::as_table)
                .and_then(background_from_table)
        })
        .unwrap_or([0, 0, 0, 128])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speech_styles_keep_local_category_defaults_alpha_background_and_tick_duration() {
        let global: Table = "[game]\nticks_per_minute=8\n[say]\nduration=1.5\nwarning='#ff000080'\ndefault='#abcdef'\nbackground='#10203040'".parse().unwrap();
        let local: Table = "duration=0.25\ndefault='#12345678'\nbackground_enabled=false"
            .parse()
            .unwrap();
        assert_eq!(color(&global, None, "warning", [1; 4]), [255, 0, 0, 128]);
        assert_eq!(
            color(&global, Some(&local), "warning", [1; 4]),
            [18, 52, 86, 120]
        );
        assert_eq!(duration_ticks(&global, None), 12);
        assert_eq!(duration_ticks(&global, Some(&local)), 2);
        assert_eq!(background_color(&global, Some(&local)), [16, 32, 48, 64]);
        assert!(!background_enabled(&global, Some(&local)));
        assert_eq!(color(&Table::new(), None, "warning", [42; 4]), [42; 4]);
        assert_eq!(crate::utils::hex_to_rgba_u8("ééé"), [255; 4]);
    }
}
