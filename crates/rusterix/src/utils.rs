/// Aligns a screen to the grid, making sure the start coordinates are not fractional
pub fn align_screen_to_grid(screen_width: f32, screen_height: f32, grid_size: f32) -> (f32, f32) {
    let half_width = screen_width / 2.0;
    let half_height = screen_height / 2.0;

    // Compute top-left corner of screen
    let top_left_x = 0.0 - half_width;
    let top_left_y = 0.0 - half_height;

    // Snap top-left to grid
    //let snapped_top_left_x = (top_left_x / grid_size).floor(); // * grid_size;
    //let snapped_top_left_y = (top_left_y / grid_size).floor(); // * grid_size;

    // New aligned center
    // let aligned_center_x = snapped_top_left_x + half_width;
    // let aligned_center_y = snapped_top_left_y + half_height;

    // (aligned_center_x, aligned_center_y)
    (top_left_x / grid_size, top_left_y / grid_size)
}

/// Shared authored RGB/RGBA hex decoding; malformed values use opaque white.
pub(crate) fn hex_to_rgba_u8(hex: &str) -> [u8; 4] {
    let hex = hex.trim_start_matches('#');
    if !hex.is_ascii() {
        return [255; 4];
    }
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
