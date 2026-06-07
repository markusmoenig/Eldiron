use crate::prelude::*;

const BG: [u8; 4] = [24, 26, 31, 255];
const PANEL: [u8; 4] = [35, 38, 46, 255];
const TEXT: [u8; 4] = [230, 235, 245, 255];
const MUTED: [u8; 4] = [135, 145, 160, 255];
const BORDER: [u8; 4] = [92, 104, 124, 255];
const FRAME_BG: [u8; 4] = [0, 0, 0, 0];

const FONT_W: usize = 5;
const FONT_H: usize = 7;
const FONT_SCALE: usize = 2;
const CHAR_W: usize = FONT_W * FONT_SCALE + 2;
const LINE_H: usize = FONT_H * FONT_SCALE + 5;
const MARGIN: usize = 16;
const CELL_GAP: usize = 12;
const ROW_GAP: usize = 14;
const BORDER_SIZE: usize = 2;
const TITLE_LINES: usize = 2;
const PALETTE_COLUMNS: usize = 3;
const PALETTE_COLUMN_W: usize = 350;
const PALETTE_SWATCH: usize = 14;

#[derive(Clone, Copy, Debug)]
struct AtlasRow {
    animation_index: usize,
    perspective_index: usize,
    y: usize,
}

#[derive(Clone, Debug)]
struct AtlasLayout {
    width: usize,
    height: usize,
    cell: usize,
    rows: Vec<AtlasRow>,
}

const MARKERS: [(&str, [u8; 4]); 9] = [
    ("LIGHT_SKIN", [255, 0, 255, 255]),
    ("DARK_SKIN", [200, 0, 200, 255]),
    ("TORSO", [0, 0, 255, 255]),
    ("ARMS", [0, 120, 255, 255]),
    ("LEGS", [0, 255, 0, 255]),
    ("HAIR", [255, 255, 0, 255]),
    ("EYES", [0, 255, 255, 255]),
    ("HANDS", [255, 128, 0, 255]),
    ("FEET", [255, 80, 0, 255]),
];

pub fn export_avatar_atlas(avatar: &rusterix::Avatar) -> Result<TheRGBABuffer, String> {
    let layout = atlas_layout(avatar);
    let mut buffer =
        TheRGBABuffer::new(TheDim::new(0, 0, layout.width as i32, layout.height as i32));
    fill_rect(&mut buffer, 0, 0, layout.width, layout.height, BG);

    draw_text(
        &mut buffer,
        MARGIN,
        MARGIN,
        &format!(
            "AVATAR ATLAS: {} | {}X{} | {} PERSPECTIVES",
            avatar.name.to_uppercase(),
            avatar.resolution,
            avatar.resolution,
            avatar.perspective_count.directions().len()
        ),
        TEXT,
    );
    draw_text(
        &mut buffer,
        MARGIN,
        MARGIN + LINE_H,
        "EDIT FRAME PIXELS ONLY. TEXT, BORDERS, AND GUIDES ARE IGNORED ON IMPORT.",
        MUTED,
    );

    draw_marker_palette(&mut buffer, MARGIN, MARGIN + LINE_H * TITLE_LINES + 8);

    for row in &layout.rows {
        let animation = avatar
            .animations
            .get(row.animation_index)
            .ok_or_else(|| "Avatar atlas row references a missing animation.".to_string())?;
        let perspective = animation
            .perspectives
            .get(row.perspective_index)
            .ok_or_else(|| "Avatar atlas row references a missing perspective.".to_string())?;
        let frame_count = perspective.frames.len();
        draw_text(
            &mut buffer,
            MARGIN,
            row.y,
            &format!(
                "{} - {} - {} {}",
                animation.name.to_uppercase(),
                perspective.direction.label().to_uppercase(),
                frame_count,
                if frame_count == 1 { "FRAME" } else { "FRAMES" }
            ),
            TEXT,
        );

        let frame_y = row.y + LINE_H;
        for (frame_index, frame) in perspective.frames.iter().enumerate() {
            let x = frame_x(frame_index, layout.cell);
            draw_frame_cell(&mut buffer, x, frame_y, layout.cell);
            blit_texture(
                &mut buffer,
                &frame.texture,
                x + BORDER_SIZE,
                frame_y + BORDER_SIZE,
                layout.cell - BORDER_SIZE * 2,
            );
            draw_text(
                &mut buffer,
                x + BORDER_SIZE,
                frame_y + layout.cell + 3,
                &format!("F{}", frame_index),
                MUTED,
            );
        }
    }

    Ok(buffer)
}

pub fn import_avatar_atlas(
    avatar: &mut rusterix::Avatar,
    atlas: &TheRGBABuffer,
) -> Result<usize, String> {
    let layout = atlas_layout(avatar);
    let dim = atlas.dim();
    if dim.width < layout.width as i32 || dim.height < layout.height as i32 {
        return Err(format!(
            "Atlas is {}x{}, expected at least {}x{} for avatar '{}'.",
            dim.width, dim.height, layout.width, layout.height, avatar.name
        ));
    }

    let mut imported = 0usize;
    for row in layout.rows {
        let animation = avatar
            .animations
            .get_mut(row.animation_index)
            .ok_or_else(|| "Avatar atlas row references a missing animation.".to_string())?;
        let perspective = animation
            .perspectives
            .get_mut(row.perspective_index)
            .ok_or_else(|| "Avatar atlas row references a missing perspective.".to_string())?;
        let frame_y = row.y + LINE_H + BORDER_SIZE;
        for (frame_index, frame) in perspective.frames.iter_mut().enumerate() {
            let frame_x = frame_x(frame_index, layout.cell) + BORDER_SIZE;
            frame.texture.data =
                extract_rgba(atlas, frame_x, frame_y, layout.cell - BORDER_SIZE * 2)?;
            frame.texture.width = avatar.resolution as usize;
            frame.texture.height = avatar.resolution as usize;
            frame.texture.data_ext = None;
            imported += 1;
        }
    }

    Ok(imported)
}

fn atlas_layout(avatar: &rusterix::Avatar) -> AtlasLayout {
    let resolution = avatar.resolution.max(1) as usize;
    let cell = resolution + BORDER_SIZE * 2;
    let max_frames = avatar
        .animations
        .iter()
        .flat_map(|animation| animation.perspectives.iter().map(|p| p.frames.len()))
        .max()
        .unwrap_or(1)
        .max(1);
    let row_count: usize = avatar
        .animations
        .iter()
        .map(|animation| animation.perspectives.len())
        .sum();
    let palette_height = palette_height();
    let rows_start = MARGIN + LINE_H * TITLE_LINES + 8 + palette_height + ROW_GAP;
    let row_height = LINE_H + cell + LINE_H + ROW_GAP;

    let mut rows = Vec::with_capacity(row_count);
    let mut y = rows_start;
    for (animation_index, animation) in avatar.animations.iter().enumerate() {
        for perspective_index in 0..animation.perspectives.len() {
            rows.push(AtlasRow {
                animation_index,
                perspective_index,
                y,
            });
            y += row_height;
        }
    }

    let frames_width = MARGIN * 2 + max_frames * cell + max_frames.saturating_sub(1) * CELL_GAP;
    let header_width = MARGIN * 2 + 86 * CHAR_W;
    let palette_width = MARGIN * 2 + PALETTE_COLUMNS * PALETTE_COLUMN_W;
    let height = if rows.is_empty() {
        rows_start + MARGIN
    } else {
        y + MARGIN - ROW_GAP
    };

    AtlasLayout {
        width: frames_width.max(header_width).max(palette_width),
        height,
        cell,
        rows,
    }
}

fn palette_height() -> usize {
    LINE_H + 3 * (PALETTE_SWATCH + 10) + 12
}

fn frame_x(frame_index: usize, cell: usize) -> usize {
    MARGIN + frame_index * (cell + CELL_GAP)
}

fn draw_marker_palette(buffer: &mut TheRGBABuffer, x: usize, y: usize) {
    draw_text(buffer, x, y, "MARKER PALETTE", TEXT);
    let start_y = y + LINE_H;
    for (index, (name, color)) in MARKERS.iter().enumerate() {
        let col = index % PALETTE_COLUMNS;
        let row = index / PALETTE_COLUMNS;
        let px = x + col * PALETTE_COLUMN_W;
        let py = start_y + row * (PALETTE_SWATCH + 10);
        fill_rect(buffer, px, py, PALETTE_SWATCH, PALETTE_SWATCH, *color);
        stroke_rect(buffer, px, py, PALETTE_SWATCH, PALETTE_SWATCH, BORDER);
        draw_text(
            buffer,
            px + PALETTE_SWATCH + 8,
            py,
            &format!("{} RGB({},{},{})", name, color[0], color[1], color[2]),
            TEXT,
        );
    }
}

fn draw_frame_cell(buffer: &mut TheRGBABuffer, x: usize, y: usize, cell: usize) {
    fill_rect(buffer, x, y, cell, cell, PANEL);
    stroke_rect(buffer, x, y, cell, cell, BORDER);
    stroke_rect(
        buffer,
        x + 1,
        y + 1,
        cell.saturating_sub(2),
        cell.saturating_sub(2),
        BORDER,
    );
    let inner = cell.saturating_sub(BORDER_SIZE * 2);
    fill_rect(
        buffer,
        x + BORDER_SIZE,
        y + BORDER_SIZE,
        inner,
        inner,
        FRAME_BG,
    );
}

fn blit_texture(
    buffer: &mut TheRGBABuffer,
    texture: &rusterix::Texture,
    dst_x: usize,
    dst_y: usize,
    size: usize,
) {
    if size == 0 || texture.width == 0 || texture.height == 0 {
        return;
    }
    for y in 0..size {
        let src_y = (y * texture.height / size).min(texture.height - 1);
        for x in 0..size {
            let src_x = (x * texture.width / size).min(texture.width - 1);
            let src = (src_y * texture.width + src_x) * 4;
            let color = [
                texture.data[src],
                texture.data[src + 1],
                texture.data[src + 2],
                texture.data[src + 3],
            ];
            buffer.set_pixel((dst_x + x) as i32, (dst_y + y) as i32, &color);
        }
    }
}

fn extract_rgba(atlas: &TheRGBABuffer, x: usize, y: usize, size: usize) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; size * size * 4];
    for yy in 0..size {
        for xx in 0..size {
            let Some(color) = atlas.get_pixel((x + xx) as i32, (y + yy) as i32) else {
                return Err("Atlas frame crop is outside the image bounds.".to_string());
            };
            let dst = (yy * size + xx) * 4;
            out[dst..dst + 4].copy_from_slice(&color);
        }
    }
    Ok(out)
}

fn fill_rect(buffer: &mut TheRGBABuffer, x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
    for yy in y..y.saturating_add(h) {
        for xx in x..x.saturating_add(w) {
            buffer.set_pixel(xx as i32, yy as i32, &color);
        }
    }
}

fn stroke_rect(buffer: &mut TheRGBABuffer, x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
    if w == 0 || h == 0 {
        return;
    }
    for xx in x..x + w {
        buffer.set_pixel(xx as i32, y as i32, &color);
        buffer.set_pixel(xx as i32, (y + h - 1) as i32, &color);
    }
    for yy in y..y + h {
        buffer.set_pixel(x as i32, yy as i32, &color);
        buffer.set_pixel((x + w - 1) as i32, yy as i32, &color);
    }
}

fn draw_text(buffer: &mut TheRGBABuffer, x: usize, y: usize, text: &str, color: [u8; 4]) {
    let mut cursor_x = x;
    for ch in text.to_uppercase().chars() {
        draw_char(buffer, cursor_x, y, ch, color);
        cursor_x += CHAR_W;
    }
}

fn draw_char(buffer: &mut TheRGBABuffer, x: usize, y: usize, ch: char, color: [u8; 4]) {
    let glyph = glyph(ch);
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..FONT_W {
            if bits & (1 << (FONT_W - 1 - col)) != 0 {
                fill_rect(
                    buffer,
                    x + col * FONT_SCALE,
                    y + row * FONT_SCALE,
                    FONT_SCALE,
                    FONT_SCALE,
                    color,
                );
            }
        }
    }
}

fn glyph(ch: char) -> [u8; FONT_H] {
    match ch {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '|' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '_' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000,
        ],
        '(' => [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
        ')' => [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
        '\'' => [
            0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        ' ' => [0; FONT_H],
        _ => [
            0b11111, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_avatar() -> rusterix::Avatar {
        let texture = rusterix::Texture::new(
            vec![255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64, 12, 34, 56, 0],
            2,
            2,
        );
        rusterix::Avatar {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            resolution: 2,
            perspective_count: rusterix::AvatarPerspectiveCount::One,
            animations: vec![rusterix::AvatarAnimation {
                id: Uuid::new_v4(),
                name: "Idle".to_string(),
                speed: 1.0,
                perspectives: vec![rusterix::AvatarPerspective {
                    direction: rusterix::AvatarDirection::Front,
                    frames: vec![rusterix::AvatarAnimationFrame::new(texture)],
                    weapon_main_anchor: Some((1, 2)),
                    weapon_off_anchor: None,
                }],
            }],
        }
    }

    #[test]
    fn avatar_atlas_round_trips_frame_pixels_and_keeps_anchors() {
        let avatar = test_avatar();
        let atlas = export_avatar_atlas(&avatar).unwrap();
        let mut imported = avatar.clone();
        imported.animations[0].perspectives[0].frames[0]
            .texture
            .data = vec![0; 16];

        let count = import_avatar_atlas(&mut imported, &atlas).unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            imported.animations[0].perspectives[0].frames[0]
                .texture
                .data,
            avatar.animations[0].perspectives[0].frames[0].texture.data
        );
        assert_eq!(
            imported.animations[0].perspectives[0].weapon_main_anchor,
            Some((1, 2))
        );
    }
}
