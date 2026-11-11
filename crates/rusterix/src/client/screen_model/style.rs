use super::*;

pub(crate) fn parse_player_camera_mode(camera: &str) -> Option<PlayerCamera> {
    match camera.to_ascii_lowercase().as_str() {
        "2d" => Some(PlayerCamera::D2),
        "2d_grid" => Some(PlayerCamera::D2Grid),
        "2d_iso" => Some(PlayerCamera::D2Iso),
        "2d_iso_grid" => Some(PlayerCamera::D2IsoGrid),
        "iso" => Some(PlayerCamera::D3Iso),
        "iso_grid" => Some(PlayerCamera::D2Grid),
        "firstp" => Some(PlayerCamera::D3FirstP),
        "firstp_mouse" => Some(PlayerCamera::D3FirstPMouse),
        "firstp_grid" => Some(PlayerCamera::D3FirstPGrid),
        _ => None,
    }
}

pub(crate) fn get_uuid(map: &toml::map::Map<String, toml::Value>, key: &str) -> Option<Uuid> {
    map.get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

pub(crate) fn hex_to_rgba_u8(hex: &str) -> [u8; 4] {
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

pub(crate) fn border_style_from_table(
    table: &toml::value::Table,
    default_color: [u8; 4],
    default_size: i32,
) -> ([u8; 4], i32, Option<[u8; 4]>, BorderGradientDirection, f32) {
    let mut color = color_from_table(table, "border_color").unwrap_or(default_color);
    let mut size = table
        .get("border_size")
        .and_then(toml::Value::as_integer)
        .map(|value| value.max(0) as i32)
        .unwrap_or(default_size.max(0));
    let mut gradient_color = None;
    let mut direction = BorderGradientDirection::Vertical;
    let mut radius = 0.0;

    if let Some(border) = table.get("border").and_then(toml::Value::as_table) {
        size = border
            .get("size")
            .and_then(toml::Value::as_integer)
            .map(|value| value.max(0) as i32)
            .unwrap_or(size);
        color = color_from_table(border, "from")
            .or_else(|| color_from_table(border, "color"))
            .unwrap_or(color);
        gradient_color = color_from_table(border, "to");
        direction = border
            .get("direction")
            .and_then(toml::Value::as_str)
            .map(BorderGradientDirection::parse)
            .unwrap_or_default();
        radius = layout_number(border, "radius").unwrap_or(0.0).max(0.0);
    }

    (color, size, gradient_color, direction, radius)
}

pub(crate) fn ui_style_color(
    ui: &toml::value::Table,
    state: Option<&str>,
    key: &str,
    flat_key: &str,
) -> Option<[u8; 4]> {
    if let Some(color) = color_from_table(ui, flat_key) {
        return Some(color);
    }

    let style = ui.get("style").and_then(toml::Value::as_table)?;
    match state {
        Some(state) => style
            .get(state)
            .and_then(toml::Value::as_table)
            .and_then(|state_style| color_from_table(state_style, key)),
        None => color_from_table(style, key),
    }
}

pub(crate) fn button_state_style_from_ui(
    ui: &toml::value::Table,
    state: &str,
    background_key: &str,
    border_key: &str,
    label_key: &str,
) -> ButtonStateStyle {
    ButtonStateStyle {
        background_color: ui_style_color(ui, Some(state), "background", background_key),
        border_color: ui_style_color(ui, Some(state), "border", border_key),
        label_color: ui_style_color(ui, Some(state), "text", label_key)
            .or_else(|| ui_style_color(ui, Some(state), "color", label_key)),
    }
}

pub(crate) fn color_from_table(table: &toml::value::Table, key: &str) -> Option<[u8; 4]> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(hex_to_rgba_u8)
}
