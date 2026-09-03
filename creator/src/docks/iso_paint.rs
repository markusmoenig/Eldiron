use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use rusterix::material_library::{
    MATERIAL_FINISH_NAMES as MATERIAL_FINISH_VALUES,
    MATERIAL_PRESET_NAMES as MATERIAL_PRESET_VALUES, MaterialDefinition,
};
use shared::iso_paint_brush::{self, IsoPaintBrushSample};

const ISO_PAINT_PRESET_STRIP: &str = "3D Paint Preset Strip";
const ISO_PAINT_MATERIAL_STRIP: &str = "3D Paint Material Strip";
const ISO_PAINT_INSPECTOR_PRIMARY: &str = "3D Paint Inspector Primary";
const ISO_PAINT_INSPECTOR_DETAIL: &str = "3D Paint Inspector Detail";
const ISO_PAINT_BRUSH_EDITOR: &str = "3D Paint Brush Editor";
const ISO_PAINT_BRUSH_SHAPE_GROUP: &str = "3D Paint Brush Shape Group";
const ISO_PAINT_BRUSH_SELECTED: &str = "3D Paint Brush Selected";
const ISO_PAINT_MATERIAL_PRESET_SELECTED: &str = "3D Paint Material Preset Selected";
const ISO_PAINT_MATERIAL_FINISH_SELECTED: &str = "3D Paint Material Finish Selected";
const ISO_PAINT_MATERIAL_MODE: &str = "3D Paint Material Mode";
const ISO_PAINT_OPERATION_GROUP: &str = "3D Paint Operation Group";
const ISO_PAINT_LAYER_VISIBLE: &str = "3D Paint Layer Visible";
const ISO_PAINT_CLEAR_ALL: &str = "3D Paint Clear All";
const ISO_PAINT_FILL_SELECTION: &str = "3D Paint Fill Selection";
const ISO_PAINT_CLEAR_SELECTION: &str = "3D Paint Clear Selection";
const ISO_PAINT_CLIP_GROUP: &str = "3D Paint Clip Group";
const ISO_PAINT_TOOL_SIZE: &str = "3D Paint Tool Size";
const ISO_PAINT_TOOL_OPACITY: &str = "3D Paint Tool Opacity";
const ISO_PAINT_PATTERN_KIND: &str = "3D Paint Pattern Kind";
const ISO_PAINT_PATTERN_SCALE: &str = "3D Paint Pattern Scale";
const ISO_PAINT_MORTAR: &str = "3D Paint Mortar";
const ISO_PAINT_PATTERN_DETAIL: &str = "3D Paint Pattern Detail";
const ISO_PAINT_PATTERN_VARIATION: &str = "3D Paint Pattern Variation";
const ISO_PAINT_STAMP_DENSITY: &str = "3D Paint Stamp Density";
const ISO_PAINT_STAMP_SIZE_JITTER: &str = "3D Paint Stamp Size Jitter";
const ISO_PAINT_STAMP_ROTATION_JITTER: &str = "3D Paint Stamp Rotation Jitter";
const ISO_PAINT_FLOWER_TYPE: &str = "3D Paint Flower Type";
const ISO_PAINT_ACTIVE_BRUSH_COLOR: &str = "3D Paint Active Brush Color";
const ISO_PAINT_BRUSH_COUNT: usize = 17;
const ISO_PAINT_SHAPE_STRIP_TILE_WIDTH: i32 = 44;
const ISO_PAINT_SHAPE_STRIP_ICON_PADDING: i32 = 2;
const ISO_PAINT_SHAPE_STRIP_TILE_HEIGHT: i32 = 24;
const ISO_PAINT_MIN_BRUSH_SIZE: f32 = 0.05;
const ISO_PAINT_MAX_PAINT_BRUSH_SIZE: f32 = 16.0;
const ISO_PAINT_MAX_STAMP_BRUSH_SIZE: f32 = 8.0;

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintOperation {
    Draw,
    Erase,
    Pick,
    Select,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintClipMode {
    None,
    Surface,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintPatternKind {
    Tiles,
    Bricks,
    Arch,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintMaterialMode {
    Coat,
    Replace,
    Stamp,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintPreviewMode {
    Paint,
    Stamp,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintBrushShape {
    Solid,
    Soft,
    Dirt,
    Speckle,
    Jagged,
    Scratch,
    Wash,
}

#[derive(Clone, Copy)]
struct IsoPaintBrushPreset {
    key: &'static str,
    size: f32,
    opacity: f32,
    shape: IsoPaintBrushShape,
    pattern_scale: f32,
    mortar: f32,
    density: f32,
}

struct IsoPaintBrushBoard {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    selected: usize,
    preview_palettes: Vec<Vec<[u8; 4]>>,
    hovered: Option<usize>,
    rectangles: Vec<(usize, TheDim)>,
    is_dirty: bool,
}

impl IsoPaintBrushBoard {
    fn new(id: TheId) -> Self {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(i32::MAX, i32::MAX));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            selected: 0,
            preview_palettes: vec![Vec::new(); IsoPaintDock::BRUSHES.len()],
            hovered: None,
            rectangles: Vec::new(),
            is_dirty: true,
        }
    }

    fn layout(&self) -> (i32, i32, i32, i32) {
        const PAD: i32 = 8;
        const GAP: i32 = 6;
        const MIN_TILE: i32 = 70;
        const MAX_TILE: i32 = 88;

        let count = IsoPaintDock::BRUSHES.len().max(1) as i32;
        let aw = (self.dim.width - PAD * 2).max(MIN_TILE);
        let max_cols = ((aw + GAP) / (MIN_TILE + GAP)).clamp(1, count);
        let mut best_cols = 1;
        let mut best_tile = MIN_TILE;
        for cols in 1..=max_cols {
            let tile = ((aw - (cols - 1) * GAP) / cols).min(MAX_TILE);
            if tile >= MIN_TILE && tile >= best_tile {
                best_cols = cols;
                best_tile = tile;
            }
        }
        let rows = (count + best_cols - 1) / best_cols;
        (best_cols, rows, best_tile, GAP)
    }

    fn draw_engine_preview(
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: &(usize, usize, usize, usize),
        stride: usize,
        key: &str,
        palette: &[[u8; 4]],
        preview_mode: IsoPaintPreviewMode,
    ) {
        let (x, y, w, h) = *rect;
        if w < 3 || h < 3 {
            return;
        }
        if key == "brick" {
            Self::draw_brick_preview(buffer, ctx, rect, stride, palette);
            return;
        }
        if key == "puddle"
            || (preview_mode == IsoPaintPreviewMode::Stamp
                && matches!(
                    key,
                    "grass"
                        | "rubble"
                        | "leaves"
                        | "flowers"
                        | "vines"
                        | "roots"
                        | "bushes"
                        | "tree"
                        | "candles"
                        | "footprints"
                        | "mud"
                ))
        {
            Self::draw_icon_preview(buffer, ctx, rect, stride, key, palette);
            return;
        }

        let bg = match key {
            "puddle" => [45, 54, 54, 255],
            "crack" => [90, 90, 84, 255],
            "grass" | "moss" => [38, 55, 37, 255],
            _ => [68, 63, 54, 255],
        };
        let fill = (
            x + 1,
            y + 1,
            w.saturating_sub(2).max(1),
            h.saturating_sub(2).max(1),
        );
        ctx.draw.rect(buffer.pixels_mut(), &fill, stride, &bg);

        let radius = ((w.max(h) as f32) * 4.0).round().max(4.0) as i32;
        let shape = iso_paint_brush::default_shape_for_brush(key);
        let color = palette
            .first()
            .copied()
            .unwrap_or_else(|| iso_paint_brush::default_preview_color(key));
        let cx = x as i32 + w as i32 / 2;
        let cy = y as i32 + h as i32 / 2;
        let pixels = buffer.pixels_mut();
        let sample = IsoPaintBrushSample {
            brush: key,
            shape,
            color,
            palette,
            opacity: 1.0,
            radius,
            seed: 0x51a7_9123,
        };

        for py in y..y + h {
            for px in x..x + w {
                let ox = px as i32 - cx;
                let oy = py as i32 - cy;
                let Some(mut sample_color) = iso_paint_brush::sample_pixel(&sample, ox, oy) else {
                    continue;
                };
                let index = (py * stride + px) * 4;
                if index + 3 >= pixels.len() {
                    continue;
                }
                if key == "puddle" {
                    sample_color[3] = sample_color[3].max(90);
                }
                let alpha = sample_color[3] as u32;
                let inv_alpha = 255 - alpha;
                pixels[index] = ((sample_color[0] as u32 * alpha
                    + pixels[index] as u32 * inv_alpha)
                    / 255) as u8;
                pixels[index + 1] = ((sample_color[1] as u32 * alpha
                    + pixels[index + 1] as u32 * inv_alpha)
                    / 255) as u8;
                pixels[index + 2] = ((sample_color[2] as u32 * alpha
                    + pixels[index + 2] as u32 * inv_alpha)
                    / 255) as u8;
                pixels[index + 3] = 255;
            }
        }

        buffer.draw_rect_outline(
            &TheDim::new(x as i32, y as i32, w as i32, h as i32),
            &[20, 20, 20, 255],
        );
    }

    fn preview_color(palette: &[[u8; 4]], index: usize, fallback: [u8; 4]) -> [u8; 4] {
        let mut color = palette.get(index).copied().unwrap_or(fallback);
        color[3] = 255;
        color
    }

    fn preview_wood_color(palette: &[[u8; 4]], index: usize, fallback: [u8; 4]) -> [u8; 4] {
        let mut color = palette
            .get(index)
            .copied()
            .filter(|color| {
                color[0] >= color[1].saturating_add(10)
                    && color[1] >= color[2].saturating_add(4)
                    && color[0] >= 54
            })
            .unwrap_or(fallback);
        color[3] = 255;
        color
    }

    fn blend_preview_pixel(pixels: &mut [u8], stride: usize, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || stride == 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= stride {
            return;
        }
        let index = (y * stride + x) * 4;
        if index + 3 >= pixels.len() {
            return;
        }
        let alpha = color[3] as u32;
        let inv_alpha = 255 - alpha;
        pixels[index] = ((color[0] as u32 * alpha + pixels[index] as u32 * inv_alpha) / 255) as u8;
        pixels[index + 1] =
            ((color[1] as u32 * alpha + pixels[index + 1] as u32 * inv_alpha) / 255) as u8;
        pixels[index + 2] =
            ((color[2] as u32 * alpha + pixels[index + 2] as u32 * inv_alpha) / 255) as u8;
        pixels[index + 3] = 255;
    }

    fn draw_preview_line(
        buffer: &mut TheRGBABuffer,
        stride: usize,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: [u8; 4],
    ) {
        let pixels = buffer.pixels_mut();
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            Self::blend_preview_pixel(pixels, stride, x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn draw_preview_ellipse(
        buffer: &mut TheRGBABuffer,
        stride: usize,
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        color: [u8; 4],
    ) {
        let rx = rx.max(1);
        let ry = ry.max(1);
        let rx2 = (rx * rx) as f32;
        let ry2 = (ry * ry) as f32;
        let pixels = buffer.pixels_mut();
        for y in -ry..=ry {
            for x in -rx..=rx {
                let edge = x as f32 * x as f32 / rx2 + y as f32 * y as f32 / ry2;
                if edge <= 1.0 {
                    Self::blend_preview_pixel(pixels, stride, cx + x, cy + y, color);
                }
            }
        }
    }

    fn draw_preview_leaf_mass(
        buffer: &mut TheRGBABuffer,
        stride: usize,
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        seed: u32,
        dark: [u8; 4],
        mid: [u8; 4],
        light: [u8; 4],
    ) {
        let rx = rx.max(2);
        let ry = ry.max(2);
        let rx2 = (rx * rx) as f32;
        let ry2 = (ry * ry) as f32;
        for y in -ry - 1..=ry + 1 {
            for x in -rx - 1..=rx + 1 {
                let edge = x as f32 * x as f32 / rx2 + y as f32 * y as f32 / ry2;
                let hash = iso_paint_brush::hash_u32(cx + x, cy + y, seed);
                let wobble = (((hash >> 8) & 0xff) as f32 / 255.0 - 0.5) * 0.34;
                if edge > 0.94 + wobble || (edge > 0.62 && ((hash >> 18) & 7) == 0) {
                    continue;
                }
                let noise = (hash & 0xff) as f32 / 255.0;
                let mut color = if y < -ry / 4 && noise > 0.34 {
                    light
                } else if y > ry / 5 || edge > 0.72 {
                    dark
                } else {
                    mid
                };
                if edge > 0.76 {
                    color[3] = ((color[3] as f32) * (0.62 + noise * 0.28)).round() as u8;
                }
                Self::blend_preview_pixel(buffer.pixels_mut(), stride, cx + x, cy + y, color);
            }
        }
    }

    fn draw_icon_preview(
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: &(usize, usize, usize, usize),
        stride: usize,
        key: &str,
        palette: &[[u8; 4]],
    ) {
        let (x, y, w, h) = *rect;
        let bg = match key {
            "grass" => [30, 47, 34, 255],
            "rubble" => [58, 55, 49, 255],
            "leaves" => [55, 48, 36, 255],
            "flowers" => [35, 52, 35, 255],
            "vines" => [28, 43, 29, 255],
            "roots" => [46, 36, 28, 255],
            "bushes" => [30, 48, 29, 255],
            "tree" => [28, 45, 29, 255],
            "candles" => [47, 39, 33, 255],
            "footprints" => [86, 73, 56, 255],
            "mud" => [45, 34, 27, 255],
            "puddle" => [34, 50, 58, 255],
            _ => [68, 63, 54, 255],
        };
        let fill = (
            x + 1,
            y + 1,
            w.saturating_sub(2).max(1),
            h.saturating_sub(2).max(1),
        );
        ctx.draw.rect(buffer.pixels_mut(), &fill, stride, &bg);

        let left = x as i32;
        let top = y as i32;
        let width = w as i32;
        let height = h as i32;
        let cx = left + width / 2;
        let cy = top + height / 2;
        let scale = width.min(height).max(1) as f32 / 48.0;
        let s = |value: f32| (value * scale).round().max(1.0) as i32;

        match key {
            "grass" => {
                let colors = [
                    Self::preview_color(palette, 0, [64, 124, 55, 255]),
                    Self::preview_color(palette, 1, [93, 158, 70, 255]),
                    Self::preview_color(palette, 2, [135, 173, 82, 255]),
                ];
                for i in 0..9 {
                    let base_x = left + s(12.0) + i * s(5.0);
                    let base_y = top + height - s(9.0);
                    let lean = ((i % 3) - 1) * s(3.0);
                    let tip_y = top + s(14.0 + (i % 4) as f32 * 3.0);
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        base_x,
                        base_y,
                        base_x + lean,
                        tip_y,
                        colors[i as usize % colors.len()],
                    );
                    if i % 2 == 0 {
                        Self::draw_preview_line(
                            buffer,
                            stride,
                            base_x + 1,
                            base_y,
                            base_x + lean + 1,
                            tip_y + s(2.0),
                            colors[(i as usize + 1) % colors.len()],
                        );
                    }
                }
            }
            "rubble" => {
                let colors = [
                    Self::preview_color(palette, 0, [98, 92, 80, 255]),
                    Self::preview_color(palette, 1, [140, 132, 112, 255]),
                    Self::preview_color(palette, 2, [54, 51, 46, 255]),
                ];
                let stones = [
                    (-18, -4, 7, 5),
                    (-7, 8, 9, 5),
                    (7, -7, 8, 6),
                    (18, 7, 6, 4),
                    (0, 0, 6, 4),
                ];
                for (i, (ox, oy, rx, ry)) in stones.iter().enumerate() {
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        cx + s(*ox as f32),
                        cy + s(*oy as f32),
                        s(*rx as f32),
                        s(*ry as f32),
                        colors[i % colors.len()],
                    );
                }
            }
            "leaves" => {
                let colors = [
                    Self::preview_color(palette, 0, [118, 82, 34, 255]),
                    Self::preview_color(palette, 1, [168, 116, 42, 255]),
                    Self::preview_color(palette, 2, [76, 112, 50, 255]),
                ];
                let leaves = [(-16, -6, 11, 5), (0, 7, 13, 5), (16, -4, 10, 4)];
                for (i, (ox, oy, rx, ry)) in leaves.iter().enumerate() {
                    let color = colors[i % colors.len()];
                    let leaf_cx = cx + s(*ox as f32);
                    let leaf_cy = cy + s(*oy as f32);
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        leaf_cx,
                        leaf_cy,
                        s(*rx as f32),
                        s(*ry as f32),
                        color,
                    );
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        leaf_cx - s(*rx as f32 / 2.0),
                        leaf_cy,
                        leaf_cx + s(*rx as f32 / 2.0),
                        leaf_cy,
                        [38, 29, 18, 120],
                    );
                }
            }
            "flowers" => {
                let stem = Self::preview_color(palette, 0, [64, 124, 55, 255]);
                let flowers = [
                    Self::preview_color(palette, 1, [255, 235, 91, 255]),
                    Self::preview_color(palette, 2, [246, 129, 135, 255]),
                    Self::preview_color(palette, 3, [253, 210, 237, 255]),
                ];
                let stems = [
                    (-16, 13, -13, -4),
                    (-4, 14, -2, -11),
                    (8, 13, 9, -5),
                    (17, 14, 16, -13),
                ];
                let clusters: &[(i32, i32)] = if w > 120 || h > 70 {
                    &[(-64, 10), (-34, -12), (0, 7), (34, -9), (64, 12)]
                } else {
                    &[(0, 0)]
                };
                for (cluster_index, (coff_x, coff_y)) in clusters.iter().enumerate() {
                    for (i, (x0, y0, x1, y1)) in stems.iter().enumerate() {
                        let base = [
                            cx + s(*coff_x as f32) + s(*x0 as f32),
                            cy + s(*coff_y as f32) + s(*y0 as f32),
                        ];
                        let tip = [
                            cx + s(*coff_x as f32) + s(*x1 as f32),
                            cy + s(*coff_y as f32) + s(*y1 as f32),
                        ];
                        Self::draw_preview_line(
                            buffer, stride, base[0], base[1], tip[0], tip[1], stem,
                        );
                        let bloom = flowers[(i + cluster_index) % flowers.len()];
                        Self::draw_preview_ellipse(
                            buffer,
                            stride,
                            tip[0],
                            tip[1],
                            s(3.0),
                            s(3.0),
                            bloom,
                        );
                        let center = Self::preview_color(palette, 3, [255, 246, 174, 230]);
                        Self::draw_preview_ellipse(
                            buffer,
                            stride,
                            tip[0],
                            tip[1],
                            s(1.2),
                            s(1.2),
                            center,
                        );
                    }
                }
            }
            "vines" => {
                let stem = Self::preview_color(palette, 0, [47, 105, 48, 255]);
                let leaf = Self::preview_color(palette, 1, [87, 145, 64, 255]);
                let highlight = Self::preview_color(palette, 2, [128, 170, 73, 230]);
                let paths = [
                    (-19, 13, -8, -3, -17),
                    (-4, 15, 2, -13, 10),
                    (14, 13, 20, -8, -8),
                ];
                for (i, (x0, y0, x1, y1, bend)) in paths.iter().enumerate() {
                    let start = [cx + s(*x0 as f32), cy + s(*y0 as f32)];
                    let mid = [
                        cx + s((*x0 + *x1) as f32 * 0.5 + *bend as f32 * 0.35),
                        cy + s((*y0 + *y1) as f32 * 0.5),
                    ];
                    let end = [cx + s(*x1 as f32), cy + s(*y1 as f32)];
                    Self::draw_preview_line(
                        buffer, stride, start[0], start[1], mid[0], mid[1], stem,
                    );
                    Self::draw_preview_line(buffer, stride, mid[0], mid[1], end[0], end[1], stem);
                    for t in 0..3 {
                        let px = start[0] + (end[0] - start[0]) * (t + 1) / 4;
                        let py = start[1] + (end[1] - start[1]) * (t + 1) / 4;
                        let side = if (i + t as usize) % 2 == 0 { -1 } else { 1 };
                        Self::draw_preview_ellipse(
                            buffer,
                            stride,
                            px + side * s(3.0),
                            py,
                            s(4.0),
                            s(2.0),
                            if t == 1 { highlight } else { leaf },
                        );
                    }
                }
            }
            "roots" => {
                let root = Self::preview_color(palette, 0, [92, 58, 36, 255]);
                let dark = Self::preview_color(palette, 1, [48, 31, 22, 240]);
                let light = Self::preview_color(palette, 2, [130, 88, 55, 220]);
                let base_y = cy + s(8.0);
                let paths = [
                    (-20, 5, -5, -4, 14),
                    (-7, 7, 8, 0, -11),
                    (3, 10, 20, -5, 9),
                    (-2, 8, -18, 15, -6),
                ];
                for (i, (x0, y0, x1, y1, bend)) in paths.iter().enumerate() {
                    let start = [cx + s(*x0 as f32), base_y + s(*y0 as f32)];
                    let mid = [
                        cx + s((*x0 + *x1) as f32 * 0.5 + *bend as f32 * 0.25),
                        base_y + s((*y0 + *y1) as f32 * 0.5),
                    ];
                    let end = [cx + s(*x1 as f32), base_y + s(*y1 as f32)];
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        start[0],
                        start[1] + 1,
                        mid[0],
                        mid[1] + 1,
                        dark,
                    );
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        mid[0],
                        mid[1] + 1,
                        end[0],
                        end[1] + 1,
                        dark,
                    );
                    Self::draw_preview_line(
                        buffer, stride, start[0], start[1], mid[0], mid[1], root,
                    );
                    Self::draw_preview_line(buffer, stride, mid[0], mid[1], end[0], end[1], root);
                    if i % 2 == 0 {
                        Self::draw_preview_line(
                            buffer,
                            stride,
                            mid[0],
                            mid[1],
                            mid[0] + s(if *bend > 0 { 6.0 } else { -6.0 }),
                            mid[1] - s(5.0),
                            light,
                        );
                    }
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        start[0],
                        start[1],
                        s(2.2),
                        s(1.6),
                        light,
                    );
                }
            }
            "bushes" => {
                let dark = Self::preview_color(palette, 0, [42, 86, 42, 255]);
                let mid = Self::preview_color(palette, 1, [72, 126, 56, 255]);
                let light = Self::preview_color(palette, 2, [118, 156, 69, 235]);
                let branch = Self::preview_wood_color(palette, 3, [74, 49, 28, 255]);
                let bark_dark = [30, 22, 15, 210];
                let base_y = cy + s(15.0);
                for (i, (x0, x1, y1, side)) in [
                    (-6.0, -9.0, -17.0, -1.0),
                    (0.0, -1.0, -22.0, 1.0),
                    (6.0, 8.0, -18.0, -1.0),
                ]
                .iter()
                .enumerate()
                {
                    let start_x = cx + s(*x0);
                    let top_x = cx + s(*x1);
                    let top_y = base_y + s(*y1);
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        start_x + 1,
                        base_y,
                        top_x + 1,
                        top_y,
                        bark_dark,
                    );
                    Self::draw_preview_line(buffer, stride, start_x, base_y, top_x, top_y, branch);
                    let mid_x = (start_x + top_x) / 2 + s(*side * 2.5);
                    let mid_y = (base_y + top_y) / 2;
                    Self::draw_preview_leaf_mass(
                        buffer,
                        stride,
                        mid_x,
                        mid_y,
                        s(3.2),
                        s(5.0),
                        0xb05b_1001 ^ i as u32,
                        dark,
                        mid,
                        light,
                    );
                    Self::draw_preview_leaf_mass(
                        buffer,
                        stride,
                        top_x + s(*side * 1.4),
                        top_y,
                        s(3.5),
                        s(5.5),
                        0xb05b_2001 ^ i as u32,
                        dark,
                        mid,
                        light,
                    );
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        start_x + 1,
                        base_y,
                        top_x + 1,
                        top_y,
                        bark_dark,
                    );
                    Self::draw_preview_line(buffer, stride, start_x, base_y, top_x, top_y, branch);
                }
                Self::draw_preview_line(
                    buffer,
                    stride,
                    cx - s(9.0),
                    base_y,
                    cx + s(9.0),
                    base_y,
                    bark_dark,
                );
            }
            "tree" => {
                let leaf_dark = Self::preview_color(palette, 0, [38, 83, 41, 255]);
                let leaf_mid = Self::preview_color(palette, 1, [75, 128, 54, 255]);
                let trunk = Self::preview_color(palette, 2, [92, 58, 36, 255]);
                let leaf_light = Self::preview_color(palette, 3, [128, 166, 75, 235]);
                Self::draw_preview_ellipse(
                    buffer,
                    stride,
                    cx,
                    cy + s(15.0),
                    s(10.0),
                    s(4.0),
                    [8, 8, 6, 85],
                );
                for dx in -s(2.0)..=s(2.0) {
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        cx + dx,
                        cy + s(14.0),
                        cx + dx,
                        cy - s(8.0),
                        trunk,
                    );
                }
                for (i, (ox, oy, rx, ry)) in [
                    (-10, -14, 11, 10),
                    (4, -20, 13, 12),
                    (15, -9, 10, 9),
                    (-1, -5, 16, 11),
                    (-16, -2, 9, 8),
                    (0, -25, 8, 8),
                ]
                .iter()
                .enumerate()
                {
                    Self::draw_preview_leaf_mass(
                        buffer,
                        stride,
                        cx + s(*ox as f32),
                        cy + s(*oy as f32),
                        s(*rx as f32),
                        s(*ry as f32),
                        0x7eee_0001 ^ i as u32,
                        leaf_dark,
                        leaf_mid,
                        leaf_light,
                    );
                }
            }
            "candles" => {
                let wax = Self::preview_color(palette, 0, [226, 204, 154, 255]);
                let shade = Self::preview_color(palette, 1, [178, 132, 91, 230]);
                let flame = Self::preview_color(palette, 2, [255, 167, 57, 230]);
                let core = Self::preview_color(palette, 3, [255, 240, 142, 245]);
                for (ox, height) in [(-11, 21), (2, 27), (13, 17)] {
                    let base_x = cx + s(ox as f32);
                    let base_y = cy + s(13.0);
                    let top_y = base_y - s(height as f32);
                    for dx in -s(3.0)..=s(3.0) {
                        let color = if dx > s(1.0) { shade } else { wax };
                        Self::draw_preview_line(
                            buffer,
                            stride,
                            base_x + dx,
                            base_y,
                            base_x + dx,
                            top_y,
                            color,
                        );
                    }
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        base_x,
                        base_y,
                        s(4.0),
                        s(2.0),
                        shade,
                    );
                    Self::draw_preview_line(
                        buffer,
                        stride,
                        base_x,
                        top_y,
                        base_x,
                        top_y - s(3.0),
                        [28, 22, 18, 190],
                    );
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        base_x,
                        top_y - s(7.0),
                        s(4.0),
                        s(6.0),
                        flame,
                    );
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        base_x,
                        top_y - s(6.0),
                        s(2.0),
                        s(3.0),
                        core,
                    );
                }
            }
            "footprints" => {
                let print = Self::preview_color(palette, 0, [48, 34, 25, 255]);
                for (i, (ox, oy)) in [(-8, -10), (8, 7)].iter().enumerate() {
                    let foot_cx = cx + s(*ox as f32);
                    let foot_cy = cy + s(*oy as f32);
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        foot_cx,
                        foot_cy,
                        s(5.0),
                        s(11.0),
                        print,
                    );
                    let toe_y = foot_cy - s(10.0);
                    let toe_dir = if i == 0 { -1 } else { 1 };
                    for toe in -1..=1 {
                        Self::draw_preview_ellipse(
                            buffer,
                            stride,
                            foot_cx + toe * s(3.0) + toe_dir * s(1.0),
                            toe_y,
                            s(2.0),
                            s(2.0),
                            print,
                        );
                    }
                }
            }
            "mud" => {
                let mud = Self::preview_color(palette, 0, [57, 39, 27, 220]);
                Self::draw_preview_ellipse(buffer, stride, cx - s(6.0), cy, s(18.0), s(10.0), mud);
                Self::draw_preview_ellipse(
                    buffer,
                    stride,
                    cx + s(12.0),
                    cy + s(4.0),
                    s(13.0),
                    s(8.0),
                    Self::preview_color(palette, 1, [80, 54, 35, 210]),
                );
                for (ox, oy, r) in [(-11, -4, 4), (3, 3, 3), (15, -2, 3)] {
                    Self::draw_preview_ellipse(
                        buffer,
                        stride,
                        cx + s(ox as f32),
                        cy + s(oy as f32),
                        s(r as f32),
                        s((r - 1) as f32),
                        [112, 86, 58, 180],
                    );
                    Self::blend_preview_pixel(
                        buffer.pixels_mut(),
                        stride,
                        cx + s(ox as f32) - s(1.0),
                        cy + s(oy as f32) - s(1.0),
                        [220, 226, 210, 150],
                    );
                }
            }
            "puddle" => {
                let water = Self::preview_color(palette, 0, [45, 91, 122, 210]);
                Self::draw_preview_ellipse(buffer, stride, cx, cy, s(23.0), s(13.0), water);
                Self::draw_preview_ellipse(
                    buffer,
                    stride,
                    cx + s(10.0),
                    cy - s(2.0),
                    s(10.0),
                    s(6.0),
                    [80, 140, 168, 160],
                );
                Self::draw_preview_line(
                    buffer,
                    stride,
                    cx - s(15.0),
                    cy - s(5.0),
                    cx - s(2.0),
                    cy - s(8.0),
                    [210, 236, 242, 170],
                );
            }
            _ => {}
        }

        buffer.draw_rect_outline(
            &TheDim::new(x as i32, y as i32, w as i32, h as i32),
            &[20, 20, 20, 255],
        );
    }

    fn draw_brick_preview(
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: &(usize, usize, usize, usize),
        stride: usize,
        palette: &[[u8; 4]],
    ) {
        let (x, y, w, h) = *rect;
        let mortar = palette
            .get(3)
            .or_else(|| palette.last())
            .copied()
            .unwrap_or([55, 48, 42, 255]);
        ctx.draw.rect(buffer.pixels_mut(), rect, stride, &mortar);

        let brick_palette: Vec<[u8; 4]> = if palette.is_empty() {
            vec![
                [140, 72, 48, 255],
                [172, 91, 58, 255],
                [113, 55, 43, 255],
                [192, 113, 72, 255],
            ]
        } else {
            palette.iter().take(4).copied().collect()
        };
        let brick_w = (w as i32 / 3).clamp(16, 28);
        let brick_h = (h as i32 / 5).clamp(8, 14);
        let gap = 2i32;
        let rows = (h as i32 + brick_h - 1) / brick_h + 1;

        for row in 0..rows {
            let offset = if row % 2 == 0 { 0 } else { brick_w / 2 };
            let mut bx = -(offset + brick_w);
            while bx < w as i32 + brick_w {
                let by = row * brick_h;
                let rx = x as i32 + bx + gap;
                let ry = y as i32 + by + gap;
                let rw = brick_w - gap * 2;
                let rh = brick_h - gap * 2;
                if rw > 0 && rh > 0 && rx < x as i32 + w as i32 && ry < y as i32 + h as i32 {
                    let color = brick_palette
                        [((row * 5 + bx / brick_w).unsigned_abs() as usize) % brick_palette.len()];
                    let shade =
                        0.82 + iso_paint_brush::noise01(bx / brick_w, row, 0x51a7_4011) * 0.34;
                    let shaded = [
                        (color[0] as f32 * shade).clamp(0.0, 255.0) as u8,
                        (color[1] as f32 * shade).clamp(0.0, 255.0) as u8,
                        (color[2] as f32 * shade).clamp(0.0, 255.0) as u8,
                        255,
                    ];
                    let clipped_x = rx.max(x as i32) as usize;
                    let clipped_y = ry.max(y as i32) as usize;
                    let clipped_w = (rx + rw).min(x as i32 + w as i32) - clipped_x as i32;
                    let clipped_h = (ry + rh).min(y as i32 + h as i32) - clipped_y as i32;
                    if clipped_w > 0 && clipped_h > 0 {
                        ctx.draw.rect(
                            buffer.pixels_mut(),
                            &(clipped_x, clipped_y, clipped_w as usize, clipped_h as usize),
                            stride,
                            &shaded,
                        );
                    }
                }
                bx += brick_w;
            }
        }

        buffer.draw_rect_outline(
            &TheDim::new(x as i32, y as i32, w as i32, h as i32),
            &[20, 20, 20, 255],
        );
    }
}

impl TheWidget for IsoPaintBrushBoard {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self::new(id)
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(coord) => {
                for (index, rect) in &self.rectangles {
                    if rect.contains(*coord) {
                        self.selected = *index;
                        ctx.ui.set_focus(self.id());
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named(ISO_PAINT_BRUSH_SELECTED),
                            TheValue::Int(*index as i32),
                        ));
                        self.is_dirty = true;
                        return true;
                    }
                }
            }
            TheEvent::Hover(coord) => {
                let mut redraw = false;
                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    self.is_dirty = true;
                    redraw = true;
                }
                let hovered = self
                    .rectangles
                    .iter()
                    .find_map(|(index, rect)| rect.contains(*coord).then_some(*index));
                if hovered != self.hovered {
                    self.hovered = hovered;
                    let text = hovered
                        .and_then(|index| IsoPaintDock::BRUSHES.get(index))
                        .map(|brush| {
                            format!(
                                "{}: {}",
                                IsoPaintDock::brush_label(brush.key),
                                IsoPaintDock::brush_description(brush.key)
                            )
                        })
                        .unwrap_or_default();
                    ctx.ui.send(TheEvent::SetStatusText(self.id.clone(), text));
                    self.is_dirty = true;
                    return true;
                }
                if redraw {
                    return true;
                }
            }
            TheEvent::LostHover(_) => {
                self.hovered = None;
                ctx.ui
                    .send(TheEvent::SetStatusText(self.id.clone(), String::new()));
                self.is_dirty = true;
                return true;
            }
            _ => {}
        }
        false
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        let (cols, _, tile, gap) = self.layout();
        let preview_h = (tile - 20).max(36);
        let mut index = 0usize;
        self.rectangles.clear();

        for row in 0.. {
            let y = 8 + row * (tile + gap);
            if y + tile > self.dim.height - 4 {
                break;
            }
            for col in 0..cols {
                if index >= IsoPaintDock::BRUSHES.len() {
                    break;
                }

                let x = 8 + col * (tile + gap);
                let local_rect = TheDim::new(x, y, tile, tile);
                let brush = IsoPaintDock::BRUSHES[index];
                let outer = (
                    utuple.0 + x as usize,
                    utuple.1 + y as usize,
                    tile as usize,
                    tile as usize,
                );
                let preview = (
                    utuple.0 + x as usize + 5,
                    utuple.1 + y as usize + 5,
                    (tile - 10) as usize,
                    (preview_h - 7) as usize,
                );

                let bg = if self.selected == index {
                    style.theme().color(ListItemSelected)
                } else if self.hovered == Some(index) {
                    style.theme().color(ListItemHover)
                } else {
                    style.theme().color(ListItemNormal)
                };
                ctx.draw.rect(buffer.pixels_mut(), &outer, stride, bg);
                let palette = self
                    .preview_palettes
                    .get(index)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                Self::draw_engine_preview(
                    buffer,
                    ctx,
                    &preview,
                    stride,
                    brush.key,
                    palette,
                    IsoPaintPreviewMode::Paint,
                );
                let border = if self.selected == index {
                    style.theme().color(DefaultSelection)
                } else if self.hovered == Some(index) {
                    style.theme().color(ListItemHover)
                } else {
                    style.theme().color(ListItemIconBorder)
                };
                ctx.draw
                    .rect_outline_border(buffer.pixels_mut(), &outer, stride, border, 1);

                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &(
                        outer.0 + 3,
                        outer.1 + preview_h as usize,
                        tile.saturating_sub(6) as usize,
                        18,
                    ),
                    stride,
                    &IsoPaintDock::brush_label(brush.key),
                    TheFontSettings {
                        size: 10.5,
                        ..Default::default()
                    },
                    style.theme().color(ListItemText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );

                self.rectangles.push((index, local_rect));
                index += 1;
            }
            if index >= IsoPaintDock::BRUSHES.len() {
                break;
            }
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

struct IsoPaintPresetStrip {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    selected: usize,
    preview_palettes: Vec<Vec<[u8; 4]>>,
    hovered: Option<usize>,
    rectangles: Vec<(usize, TheDim)>,
    scroll_offset: i32,
    drag_anchor: Option<Vec2<i32>>,
    drag_start_scroll: i32,
    is_dragging: bool,
    is_dirty: bool,
}

impl IsoPaintPresetStrip {
    fn new(id: TheId) -> Self {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_min_height(46);
        limiter.set_max_height(46);
        limiter.set_max_size(Vec2::new(i32::MAX, 46));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            selected: 0,
            preview_palettes: vec![Vec::new(); IsoPaintDock::BRUSHES.len()],
            hovered: None,
            rectangles: Vec::new(),
            scroll_offset: 0,
            drag_anchor: None,
            drag_start_scroll: 0,
            is_dragging: false,
            is_dirty: true,
        }
    }

    fn set_selected(&mut self, selected: usize) {
        self.selected = selected.min(IsoPaintDock::BRUSHES.len().saturating_sub(1));
        self.is_dirty = true;
    }

    fn set_preview_palettes(&mut self, palettes: Vec<Vec<[u8; 4]>>) {
        self.preview_palettes = palettes;
        self.is_dirty = true;
    }

    fn content_width(&self) -> i32 {
        const PAD_X: i32 = 7;
        const GAP: i32 = 5;
        let count = IsoPaintDock::BRUSHES.len() as i32;
        if count == 0 {
            0
        } else {
            PAD_X * 2 + count * (Self::tile_width() + GAP) - GAP
        }
    }

    fn tile_width() -> i32 {
        64
    }

    fn max_scroll(&self) -> i32 {
        (self.content_width() - self.dim.width).max(0)
    }

    fn clamp_scroll(&mut self) {
        self.scroll_offset = self.scroll_offset.clamp(0, self.max_scroll());
    }

    fn scroll_by(&mut self, delta: i32) -> bool {
        let old = self.scroll_offset;
        self.scroll_offset = (self.scroll_offset + delta).clamp(0, self.max_scroll());
        if old != self.scroll_offset {
            self.is_dirty = true;
            true
        } else {
            false
        }
    }
}

impl TheWidget for IsoPaintPresetStrip {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self::new(id)
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.clamp_scroll();
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(coord) => {
                self.drag_anchor = Some(*coord);
                self.drag_start_scroll = self.scroll_offset;
                self.is_dragging = false;
                ctx.ui.set_hover(self.id());
                ctx.ui.set_focus(self.id());
                return true;
            }
            TheEvent::MouseDragged(coord) => {
                if let Some(anchor) = self.drag_anchor {
                    let dx = coord.x - anchor.x;
                    if dx.abs() > 2 {
                        self.is_dragging = true;
                    }
                    let old = self.scroll_offset;
                    self.scroll_offset = (self.drag_start_scroll - dx).clamp(0, self.max_scroll());
                    if self.scroll_offset != old {
                        self.is_dirty = true;
                        return true;
                    }
                }
                return false;
            }
            TheEvent::MouseUp(coord) => {
                let clicked = !self.is_dragging;
                self.drag_anchor = None;
                self.is_dragging = false;
                if clicked {
                    for (index, rect) in &self.rectangles {
                        if rect.contains(*coord) {
                            self.selected = *index;
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named(ISO_PAINT_BRUSH_SELECTED),
                                TheValue::Int(*index as i32),
                            ));
                            self.is_dirty = true;
                            return true;
                        }
                    }
                }
                return self.is_dirty;
            }
            TheEvent::MouseWheel(delta) => {
                if self.scroll_by(-delta.x - delta.y) {
                    return true;
                }
                return false;
            }
            TheEvent::Hover(coord) => {
                let mut redraw = false;
                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    self.is_dirty = true;
                    redraw = true;
                }
                let hovered = self
                    .rectangles
                    .iter()
                    .find_map(|(index, rect)| rect.contains(*coord).then_some(*index));
                if hovered != self.hovered {
                    self.hovered = hovered;
                    let text = hovered
                        .and_then(|index| IsoPaintDock::BRUSHES.get(index))
                        .map(|brush| {
                            format!(
                                "{}: {}",
                                IsoPaintDock::brush_label(brush.key),
                                IsoPaintDock::brush_description(brush.key)
                            )
                        })
                        .unwrap_or_default();
                    ctx.ui.send(TheEvent::SetStatusText(self.id.clone(), text));
                    self.is_dirty = true;
                    return true;
                }
                if redraw {
                    return true;
                }
            }
            TheEvent::LostHover(_) => {
                self.hovered = None;
                ctx.ui
                    .send(TheEvent::SetStatusText(self.id.clone(), String::new()));
                self.is_dirty = true;
                return true;
            }
            _ => {}
        }
        false
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        self.clamp_scroll();
        const PAD_X: i32 = 7;
        const PAD_Y: i32 = 4;
        const GAP: i32 = 5;
        let tile_w = Self::tile_width();
        let tile_h = (self.dim.height - PAD_Y * 2).clamp(30, 38);
        self.rectangles.clear();

        for index in 0..IsoPaintDock::BRUSHES.len() {
            let x = PAD_X + index as i32 * (tile_w + GAP) - self.scroll_offset;
            let y = PAD_Y;
            if x + tile_w <= 0 || x >= self.dim.width {
                continue;
            }

            let brush = IsoPaintDock::BRUSHES[index];
            let local_rect = TheDim::new(x, y, tile_w, tile_h);
            let mut tile_buffer = TheRGBABuffer::new(TheDim::sized(tile_w, tile_h));
            let tile_stride = tile_buffer.stride();
            let outer = (0, 0, tile_w as usize, tile_h as usize);
            let preview = (
                2,
                2,
                tile_w.saturating_sub(4) as usize,
                tile_h.saturating_sub(4) as usize,
            );
            let bg = if self.selected == index {
                style.theme().color(ListItemSelected)
            } else if self.hovered == Some(index) {
                style.theme().color(ListItemHover)
            } else {
                style.theme().color(ListItemNormal)
            };
            ctx.draw
                .rect(tile_buffer.pixels_mut(), &outer, tile_stride, bg);
            let palette = self
                .preview_palettes
                .get(index)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            IsoPaintBrushBoard::draw_engine_preview(
                &mut tile_buffer,
                ctx,
                &preview,
                tile_stride,
                brush.key,
                palette,
                if IsoPaintDock::brush_supports_stamp(brush.key) {
                    IsoPaintPreviewMode::Stamp
                } else {
                    IsoPaintPreviewMode::Paint
                },
            );
            let border = if self.selected == index {
                style.theme().color(DefaultSelection)
            } else if self.hovered == Some(index) {
                style.theme().color(ListItemHover)
            } else {
                style.theme().color(ListItemIconBorder)
            };
            ctx.draw
                .rect_outline_border(tile_buffer.pixels_mut(), &outer, tile_stride, border, 1);
            buffer.copy_into(utuple.0 as i32 + x, utuple.1 as i32 + y, &tile_buffer);
            self.rectangles.push((index, local_rect));
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

struct IsoPaintBrushShapeStrip {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    selected: usize,
    hovered: Option<usize>,
    rectangles: Vec<(usize, TheDim)>,
    is_dirty: bool,
}

impl IsoPaintBrushShapeStrip {
    fn new(id: TheId) -> Self {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_min_height(32);
        limiter.set_max_height(32);
        limiter.set_max_size(Vec2::new(i32::MAX, 32));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            selected: 0,
            hovered: None,
            rectangles: Vec::new(),
            is_dirty: true,
        }
    }

    fn draw_shape_icon(
        buffer: &mut TheRGBABuffer,
        rect: &(usize, usize, usize, usize),
        stride: usize,
        shape: &str,
    ) {
        let (x, y, w, h) = *rect;
        let radius = ((w.min(h) as f32) * 0.42).round().max(4.0) as i32;
        let cx = x as i32 + w as i32 / 2;
        let cy = y as i32 + h as i32 / 2;
        let sample = IsoPaintBrushSample {
            brush: "material",
            shape,
            color: [246, 246, 238, 255],
            palette: &[],
            opacity: 1.0,
            radius,
            seed: 0x7812_5101,
        };
        let pixels = buffer.pixels_mut();
        for py in y..y + h {
            for px in x..x + w {
                let ox = px as i32 - cx;
                let oy = py as i32 - cy;
                let Some(color) = iso_paint_brush::sample_pixel(&sample, ox, oy) else {
                    continue;
                };
                let index = (py * stride + px) * 4;
                if index + 3 >= pixels.len() {
                    continue;
                }
                let src_alpha = color[3] as u32;
                let dst_alpha = pixels[index + 3] as u32;
                let inv_src_alpha = 255 - src_alpha;
                let out_alpha = src_alpha + (dst_alpha * inv_src_alpha) / 255;
                if out_alpha == 0 {
                    continue;
                }
                pixels[index] = ((color[0] as u32 * src_alpha
                    + pixels[index] as u32 * dst_alpha * inv_src_alpha / 255)
                    / out_alpha) as u8;
                pixels[index + 1] = ((color[1] as u32 * src_alpha
                    + pixels[index + 1] as u32 * dst_alpha * inv_src_alpha / 255)
                    / out_alpha) as u8;
                pixels[index + 2] = ((color[2] as u32 * src_alpha
                    + pixels[index + 2] as u32 * dst_alpha * inv_src_alpha / 255)
                    / out_alpha) as u8;
                pixels[index + 3] = out_alpha as u8;
            }
        }
    }
}

impl TheWidget for IsoPaintBrushShapeStrip {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self::new(id)
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(coord) => {
                for (index, rect) in &self.rectangles {
                    if rect.contains(*coord) {
                        self.selected = *index;
                        ctx.ui.set_focus(self.id());
                        ctx.ui
                            .send(TheEvent::IndexChanged(self.id.clone(), self.selected));
                        self.is_dirty = true;
                        return true;
                    }
                }
            }
            TheEvent::Hover(coord) => {
                let hovered = self
                    .rectangles
                    .iter()
                    .find_map(|(index, rect)| rect.contains(*coord).then_some(*index));
                if hovered != self.hovered {
                    self.hovered = hovered;
                    let text = hovered
                        .map(|index| {
                            IsoPaintDock::brush_shape_label(IsoPaintDock::brush_shape_from_index(
                                index,
                            ))
                        })
                        .unwrap_or_default();
                    ctx.ui.send(TheEvent::SetStatusText(self.id.clone(), text));
                    self.is_dirty = true;
                    return true;
                }
            }
            TheEvent::LostHover(_) => {
                self.hovered = None;
                ctx.ui
                    .send(TheEvent::SetStatusText(self.id.clone(), String::new()));
                self.is_dirty = true;
                return true;
            }
            _ => {}
        }
        false
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        const PAD_X: i32 = 8;
        const PAD_Y: i32 = 4;
        const GAP: i32 = 6;
        let count = IsoPaintDock::brush_shape_values().len() as i32;
        let aw = (self.dim.width - PAD_X * 2).max(1);
        let tile_w = ((aw - (count - 1) * GAP) / count).clamp(30, 46);
        let tile_h = (self.dim.height - PAD_Y * 2).clamp(22, 28);
        self.rectangles.clear();

        for index in 0..count as usize {
            let x = PAD_X + index as i32 * (tile_w + GAP);
            let y = PAD_Y;
            if x + tile_w > self.dim.width - PAD_X + 1 {
                break;
            }
            let local_rect = TheDim::new(x, y, tile_w, tile_h);
            let outer = (
                utuple.0 + x as usize,
                utuple.1 + y as usize,
                tile_w as usize,
                tile_h as usize,
            );
            let bg = if self.selected == index {
                style.theme().color(ListItemSelected)
            } else if self.hovered == Some(index) {
                style.theme().color(ListItemHover)
            } else {
                style.theme().color(ListItemNormal)
            };
            ctx.draw.rect(buffer.pixels_mut(), &outer, stride, bg);
            let icon = (
                outer.0 + 2,
                outer.1 + 2,
                outer.2.saturating_sub(4),
                outer.3.saturating_sub(4),
            );
            Self::draw_shape_icon(
                buffer,
                &icon,
                stride,
                IsoPaintDock::brush_shape_key_from_index(index),
            );
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &outer,
                stride,
                if self.selected == index {
                    style.theme().color(DefaultSelection)
                } else if self.hovered == Some(index) {
                    style.theme().color(ListItemHover)
                } else {
                    style.theme().color(ListItemIconBorder)
                },
                1,
            );
            self.rectangles.push((index, local_rect));
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

struct IsoPaintBrushEditor {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    selected_brush: usize,
    preview_palette: Vec<[u8; 4]>,
    preview_mode: IsoPaintPreviewMode,
    preview_cache: Option<(
        String,
        Vec<[u8; 4]>,
        IsoPaintPreviewMode,
        i32,
        i32,
        TheRGBABuffer,
    )>,
    is_dirty: bool,
}

impl IsoPaintBrushEditor {
    fn new(id: TheId) -> Self {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(i32::MAX, i32::MAX));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            selected_brush: 0,
            preview_palette: Vec::new(),
            preview_mode: IsoPaintPreviewMode::Paint,
            preview_cache: None,
            is_dirty: true,
        }
    }

    fn set_selected_brush(&mut self, selected_brush: usize) {
        let selected_brush = selected_brush.min(IsoPaintDock::BRUSHES.len().saturating_sub(1));
        if self.selected_brush != selected_brush {
            self.selected_brush = selected_brush;
            self.preview_cache = None;
        }
        self.is_dirty = true;
    }

    fn set_preview_palette(&mut self, palette: Vec<[u8; 4]>) {
        if self.preview_palette != palette {
            self.preview_palette = palette;
            self.preview_cache = None;
            self.is_dirty = true;
        }
    }

    fn set_preview_mode(&mut self, preview_mode: IsoPaintPreviewMode) {
        if self.preview_mode != preview_mode {
            self.preview_mode = preview_mode;
            self.preview_cache = None;
            self.is_dirty = true;
        }
    }

    fn selected_key(&self) -> &'static str {
        IsoPaintDock::BRUSHES
            .get(self.selected_brush)
            .map(|brush| brush.key)
            .unwrap_or("material")
    }

    fn draw_text(
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: &(usize, usize, usize, usize),
        stride: usize,
        text: &str,
        size: f32,
        color: &[u8; 4],
        align: TheHorizontalAlign,
    ) {
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            rect,
            stride,
            text,
            TheFontSettings {
                size,
                ..Default::default()
            },
            color,
            align,
            TheVerticalAlign::Center,
        );
    }

    fn draw_preview_cached(
        &mut self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: &(usize, usize, usize, usize),
        stride: usize,
        brush_key: &str,
    ) {
        let cache_valid = self.preview_cache.as_ref().is_some_and(
            |(cached_key, cached_palette, cached_mode, cached_w, cached_h, _)| {
                cached_key == brush_key
                    && cached_palette == &self.preview_palette
                    && *cached_mode == self.preview_mode
                    && *cached_w == rect.2 as i32
                    && *cached_h == rect.3 as i32
            },
        );
        if !cache_valid {
            let mut preview = TheRGBABuffer::new(TheDim::sized(rect.2 as i32, rect.3 as i32));
            IsoPaintBrushBoard::draw_engine_preview(
                &mut preview,
                ctx,
                &(0, 0, rect.2, rect.3),
                rect.2,
                brush_key,
                &self.preview_palette,
                self.preview_mode,
            );
            self.preview_cache = Some((
                brush_key.to_string(),
                self.preview_palette.clone(),
                self.preview_mode,
                rect.2 as i32,
                rect.3 as i32,
                preview,
            ));
        }
        if let Some((_, _, _, _, _, preview)) = &self.preview_cache {
            ctx.draw
                .blend_slice(buffer.pixels_mut(), preview.pixels(), rect, stride);
        }
    }
}

impl TheWidget for IsoPaintBrushEditor {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self::new(id)
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn supports_hover(&mut self) -> bool {
        false
    }

    fn on_event(&mut self, _event: &TheEvent, _ctx: &mut TheContext) -> bool {
        false
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );
        if self.dim.width < 96 || self.dim.height < 72 {
            self.is_dirty = false;
            return;
        }

        let brush_key = self.selected_key();

        let pad = 8i32;
        let content_w = (self.dim.width - pad * 2).max(1);
        let preview_w = content_w;
        let preview_h = (self.dim.height - pad * 2).max(1);
        let preview_panel = (
            utuple.0 + pad as usize,
            utuple.1 + pad as usize,
            preview_w as usize,
            preview_h as usize,
        );
        ctx.draw.rect(
            buffer.pixels_mut(),
            &preview_panel,
            stride,
            style.theme().color(ListItemNormal),
        );
        ctx.draw.rect_outline_border(
            buffer.pixels_mut(),
            &preview_panel,
            stride,
            style.theme().color(ListItemIconBorder),
            1,
        );

        let title_h = 24usize;
        Self::draw_text(
            buffer,
            ctx,
            &(
                preview_panel.0 + 8,
                preview_panel.1,
                preview_panel.2.saturating_sub(16),
                title_h,
            ),
            stride,
            &IsoPaintDock::brush_label(brush_key),
            11.0,
            style.theme().color(ListItemText),
            TheHorizontalAlign::Left,
        );

        let preview_rect = (
            preview_panel.0 + 7,
            preview_panel.1 + title_h + 8,
            preview_panel.2.saturating_sub(14).max(1),
            preview_panel.3.saturating_sub(title_h + 16).max(1),
        );
        self.draw_preview_cached(buffer, ctx, &preview_rect, stride, brush_key);

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

struct IsoPaintMaterialStrip {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    material_preset: usize,
    material_finish: usize,
    hovered: Option<(&'static str, usize)>,
    preset_rects: Vec<(usize, TheDim)>,
    finish_rects: Vec<(usize, TheDim)>,
    is_dirty: bool,
}

impl IsoPaintMaterialStrip {
    fn new(id: TheId) -> Self {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_min_height(58);
        limiter.set_max_height(58);
        limiter.set_max_size(Vec2::new(i32::MAX, 58));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            material_preset: 0,
            material_finish: 0,
            hovered: None,
            preset_rects: Vec::new(),
            finish_rects: Vec::new(),
            is_dirty: true,
        }
    }

    fn set_material(&mut self, material_preset: i32, material_finish: i32) {
        self.material_preset =
            (material_preset.max(0) as usize).min(MATERIAL_PRESET_VALUES.len().saturating_sub(1));
        self.material_finish =
            (material_finish.max(0) as usize).min(MATERIAL_FINISH_VALUES.len().saturating_sub(1));
        self.is_dirty = true;
    }

    fn material_labels_short() -> Vec<String> {
        MATERIAL_PRESET_VALUES
            .iter()
            .map(|value| match *value {
                "default" => "Def",
                "stone" => "Stn",
                "dirt" => "Dirt",
                "wood" => "Wood",
                "metal" => "Met",
                "glass" => "Gls",
                "water" => "Wat",
                "mirror" => "Mir",
                "emissive" => "Em",
                "fabric" => "Fab",
                "plastic" => "Pla",
                "foliage" => "Fol",
                "skin" => "Skin",
                "bone" => "Bone",
                "wax" => "Wax",
                _ => "?",
            })
            .map(str::to_string)
            .collect()
    }

    fn finish_labels_short() -> Vec<String> {
        vec![
            fl!("material_finish_natural"),
            fl!("material_finish_matte"),
            fl!("material_finish_polished"),
            fl!("material_finish_wet"),
        ]
    }
}

impl TheWidget for IsoPaintMaterialStrip {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self::new(id)
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(coord) => {
                for (index, rect) in &self.preset_rects {
                    if rect.contains(*coord) {
                        self.material_preset = *index;
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named(ISO_PAINT_MATERIAL_PRESET_SELECTED),
                            TheValue::Int(*index as i32),
                        ));
                        self.is_dirty = true;
                        return true;
                    }
                }
                for (index, rect) in &self.finish_rects {
                    if rect.contains(*coord) {
                        self.material_finish = *index;
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named(ISO_PAINT_MATERIAL_FINISH_SELECTED),
                            TheValue::Int(*index as i32),
                        ));
                        self.is_dirty = true;
                        return true;
                    }
                }
            }
            TheEvent::Hover(coord) => {
                let hovered = self
                    .preset_rects
                    .iter()
                    .find_map(|(index, rect)| rect.contains(*coord).then_some(("preset", *index)))
                    .or_else(|| {
                        self.finish_rects.iter().find_map(|(index, rect)| {
                            rect.contains(*coord).then_some(("finish", *index))
                        })
                    });
                if hovered != self.hovered {
                    self.hovered = hovered;
                    let text = hovered
                        .map(|(kind, index)| {
                            let (kind_label, label) = if kind == "preset" {
                                IsoPaintDock::material_preset_labels()
                                    .get(index)
                                    .cloned()
                                    .map(|label| (fl!("material_preset"), label))
                                    .unwrap_or_default()
                            } else {
                                IsoPaintDock::material_finish_labels()
                                    .get(index)
                                    .cloned()
                                    .map(|label| (fl!("material_finish"), label))
                                    .unwrap_or_default()
                            };
                            format!("{kind_label}: {label}")
                        })
                        .unwrap_or_default();
                    ctx.ui.send(TheEvent::SetStatusText(self.id.clone(), text));
                    self.is_dirty = true;
                    return true;
                }
            }
            TheEvent::LostHover(_) => {
                self.hovered = None;
                ctx.ui
                    .send(TheEvent::SetStatusText(self.id.clone(), String::new()));
                self.is_dirty = true;
                return true;
            }
            _ => {}
        }
        false
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }

        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        self.preset_rects.clear();
        self.finish_rects.clear();

        const PAD_X: i32 = 8;
        const PAD_Y: i32 = 5;
        const GAP: i32 = 3;
        let aw = (self.dim.width - PAD_X * 2).max(1);
        let preset_count = MATERIAL_PRESET_VALUES.len() as i32;
        let preset_w = ((aw - (preset_count - 1) * GAP) / preset_count).max(12);
        let preset_h = 21;
        let preset_y = PAD_Y;
        let material_labels = Self::material_labels_short();

        for index in 0..MATERIAL_PRESET_VALUES.len() {
            let x = PAD_X + index as i32 * (preset_w + GAP);
            let rect = TheDim::new(x, preset_y, preset_w, preset_h);
            let global = (
                utuple.0 + x as usize,
                utuple.1 + preset_y as usize,
                preset_w as usize,
                preset_h as usize,
            );
            let selected = self.material_preset == index;
            let hovered = self.hovered == Some(("preset", index));
            let color = if selected {
                style.theme().color(GroupButtonSelectedBackground)
            } else if hovered {
                style.theme().color(GroupButtonHoverBackground)
            } else {
                style.theme().color(GroupButtonNormalBackground)
            };
            ctx.draw.rect(buffer.pixels_mut(), &global, stride, color);
            let border = if selected {
                style.theme().color(GroupButtonSelectedBorder)
            } else if hovered {
                style.theme().color(GroupButtonHoverBorder)
            } else {
                style.theme().color(GroupButtonNormalBorder)
            };
            ctx.draw
                .rect_outline_border(buffer.pixels_mut(), &global, stride, border, 1);
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(global.0 + 1, global.1, global.2.saturating_sub(2), global.3),
                stride,
                &material_labels[index],
                TheFontSettings {
                    size: 9.5,
                    ..Default::default()
                },
                style.theme().color(ListItemText),
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
            self.preset_rects.push((index, rect));
        }

        let finish_count = MATERIAL_FINISH_VALUES.len() as i32;
        let finish_w = ((aw - (finish_count - 1) * GAP) / finish_count).max(38);
        let finish_h = 22;
        let finish_y = PAD_Y + preset_h + 5;
        let finish_labels = Self::finish_labels_short();

        for index in 0..MATERIAL_FINISH_VALUES.len() {
            let x = PAD_X + index as i32 * (finish_w + GAP);
            let rect = TheDim::new(x, finish_y, finish_w, finish_h);
            let global = (
                utuple.0 + x as usize,
                utuple.1 + finish_y as usize,
                finish_w as usize,
                finish_h as usize,
            );
            let selected = self.material_finish == index;
            let hovered = self.hovered == Some(("finish", index));
            let color = if selected {
                style.theme().color(GroupButtonSelectedBackground)
            } else if hovered {
                style.theme().color(GroupButtonHoverBackground)
            } else {
                style.theme().color(GroupButtonNormalBackground)
            };
            ctx.draw.rect(buffer.pixels_mut(), &global, stride, color);
            let border = if selected {
                style.theme().color(GroupButtonSelectedBorder)
            } else if hovered {
                style.theme().color(GroupButtonHoverBorder)
            } else {
                style.theme().color(GroupButtonNormalBorder)
            };
            ctx.draw
                .rect_outline_border(buffer.pixels_mut(), &global, stride, border, 1);
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(global.0 + 2, global.1, global.2.saturating_sub(4), global.3),
                stride,
                &finish_labels[index],
                TheFontSettings {
                    size: 10.0,
                    ..Default::default()
                },
                style.theme().color(ListItemText),
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
            self.finish_rects.push((index, rect));
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct IsoPaintDock {
    target: IsoPaintTarget,
    ui_prefix: String,
    selected_brush: usize,
    operation: IsoPaintOperation,
    size: f32,
    opacity: f32,
    brush_shapes: [IsoPaintBrushShape; ISO_PAINT_BRUSH_COUNT],
    brush_sizes: [f32; ISO_PAINT_BRUSH_COUNT],
    brush_opacities: [f32; ISO_PAINT_BRUSH_COUNT],
    brush_color_slots: [[u16; 4]; ISO_PAINT_BRUSH_COUNT],
    selected_color_slot: usize,
    brush_material_presets: [i32; ISO_PAINT_BRUSH_COUNT],
    brush_material_finishes: [i32; ISO_PAINT_BRUSH_COUNT],
    material_preset: i32,
    material_finish: i32,
    material_mode: IsoPaintMaterialMode,
    brush_shape: IsoPaintBrushShape,
    paint_visible: bool,
    clip_mode: IsoPaintClipMode,
    pattern_kind: IsoPaintPatternKind,
    pattern_scale: f32,
    pattern_mortar: f32,
    pattern_detail: f32,
    pattern_variation: f32,
    stamp_density: f32,
    stamp_size_jitter: f32,
    stamp_rotation_jitter: f32,
    flower_type: i32,
    nodeui: TheNodeUI,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum IsoPaintTarget {
    #[default]
    Region,
    Prefab,
}

impl IsoPaintDock {
    pub fn new_prefab() -> Self {
        let mut dock = <Self as Dock>::new();
        dock.target = IsoPaintTarget::Prefab;
        dock.ui_prefix = "Prefab ".to_string();
        dock
    }

    fn ui_name(&self, name: &str) -> String {
        format!("{}{name}", self.ui_prefix)
    }

    fn ui_id(&self, name: &str) -> TheId {
        TheId::named(&self.ui_name(name))
    }

    fn base_ui_name<'a>(&self, name: &'a str) -> &'a str {
        name.strip_prefix(&self.ui_prefix).unwrap_or(name)
    }

    fn is_ui_event(&self, name: &str, expected: &str) -> bool {
        self.base_ui_name(name) == expected
    }

    fn paint_layer<'a>(
        &self,
        project: &'a Project,
        server_ctx: &ServerContext,
    ) -> Option<&'a IsoPaintLayer> {
        match self.target {
            IsoPaintTarget::Region => project
                .get_region(&server_ctx.curr_region)
                .map(|region| &region.iso_paint),
            IsoPaintTarget::Prefab => match server_ctx.pc {
                ProjectContext::Prefab(asset_id) => project.block_prop_paint.get(&asset_id),
                _ => None,
            },
        }
    }

    fn paint_layer_mut<'a>(
        &self,
        project: &'a mut Project,
        server_ctx: &ServerContext,
    ) -> Option<&'a mut IsoPaintLayer> {
        match self.target {
            IsoPaintTarget::Region => project
                .get_region_mut(&server_ctx.curr_region)
                .map(|region| &mut region.iso_paint),
            IsoPaintTarget::Prefab => match server_ctx.pc {
                ProjectContext::Prefab(asset_id) => {
                    Some(project.block_prop_paint.entry(asset_id).or_default())
                }
                _ => None,
            },
        }
    }

    const BRUSHES: [IsoPaintBrushPreset; ISO_PAINT_BRUSH_COUNT] = [
        IsoPaintBrushPreset {
            key: "material",
            size: 8.0,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Solid,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "brick",
            size: 8.0,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Solid,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "moss",
            size: 1.6,
            opacity: 0.75,
            shape: IsoPaintBrushShape::Dirt,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.7,
        },
        IsoPaintBrushPreset {
            key: "grass",
            size: 1.2,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Speckle,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "dirt",
            size: 1.4,
            opacity: 0.82,
            shape: IsoPaintBrushShape::Dirt,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "crack",
            size: 0.6,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Scratch,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "puddle",
            size: 1.8,
            opacity: 0.68,
            shape: IsoPaintBrushShape::Wash,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "mud",
            size: 1.1,
            opacity: 0.82,
            shape: IsoPaintBrushShape::Soft,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.62,
        },
        IsoPaintBrushPreset {
            key: "rubble",
            size: 1.0,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Jagged,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.65,
        },
        IsoPaintBrushPreset {
            key: "leaves",
            size: 1.0,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Speckle,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.7,
        },
        IsoPaintBrushPreset {
            key: "flowers",
            size: 1.0,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Speckle,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.58,
        },
        IsoPaintBrushPreset {
            key: "vines",
            size: 1.15,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Speckle,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.54,
        },
        IsoPaintBrushPreset {
            key: "roots",
            size: 1.15,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Scratch,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.56,
        },
        IsoPaintBrushPreset {
            key: "bushes",
            size: 1.85,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Speckle,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.45,
        },
        IsoPaintBrushPreset {
            key: "tree",
            size: 2.35,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Speckle,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.38,
        },
        IsoPaintBrushPreset {
            key: "candles",
            size: 1.0,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Soft,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.45,
        },
        IsoPaintBrushPreset {
            key: "footprints",
            size: 1.0,
            opacity: 0.9,
            shape: IsoPaintBrushShape::Soft,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.55,
        },
    ];

    fn material_preset_labels() -> Vec<String> {
        vec![
            fl!("material_preset_default"),
            fl!("material_preset_stone"),
            fl!("material_preset_dirt"),
            fl!("material_preset_wood"),
            fl!("material_preset_metal"),
            fl!("material_preset_glass"),
            fl!("material_preset_water"),
            fl!("material_preset_mirror"),
            fl!("material_preset_emissive"),
            fl!("material_preset_fabric"),
            fl!("material_preset_plastic"),
            fl!("material_preset_foliage"),
            fl!("material_preset_skin"),
            fl!("material_preset_bone"),
            fl!("material_preset_wax"),
        ]
    }

    fn material_finish_labels() -> Vec<String> {
        vec![
            fl!("material_finish_natural"),
            fl!("material_finish_matte"),
            fl!("material_finish_polished"),
            fl!("material_finish_wet"),
        ]
    }

    fn default_brush_sizes() -> [f32; ISO_PAINT_BRUSH_COUNT] {
        std::array::from_fn(|index| Self::BRUSHES[index].size)
    }

    fn default_brush_opacities() -> [f32; ISO_PAINT_BRUSH_COUNT] {
        std::array::from_fn(|index| Self::BRUSHES[index].opacity)
    }

    fn default_brush_shapes() -> [IsoPaintBrushShape; ISO_PAINT_BRUSH_COUNT] {
        std::array::from_fn(|index| Self::BRUSHES[index].shape)
    }

    fn default_brush_color_slots() -> [[u16; 4]; ISO_PAINT_BRUSH_COUNT] {
        [
            [6, 6, 6, 6],     // material
            [19, 20, 21, 18], // brick
            [37, 36, 34, 38], // moss
            [37, 36, 35, 34], // grass
            [18, 17, 27, 16], // dirt
            [1, 2, 4, 7],     // crack
            [39, 41, 43, 45], // puddle
            [18, 16, 17, 27], // mud
            [4, 7, 2, 1],     // rubble
            [34, 37, 18, 20], // leaves
            [37, 32, 46, 47], // flowers
            [37, 36, 34, 18], // vines
            [18, 17, 16, 27], // roots
            [37, 36, 34, 35], // bushes
            [37, 36, 18, 34], // tree
            [46, 47, 21, 32], // candles
            [18, 17, 16, 27], // footprints
        ]
    }

    fn brush_color_slot_count(key: &str) -> usize {
        match key {
            "material" => 1,
            "puddle" => 1,
            "crack" => 3,
            _ => 4,
        }
    }

    fn brush_shape_key(shape: IsoPaintBrushShape) -> &'static str {
        match shape {
            IsoPaintBrushShape::Solid => "solid",
            IsoPaintBrushShape::Soft => "soft",
            IsoPaintBrushShape::Dirt => "dirt",
            IsoPaintBrushShape::Speckle => "speckle",
            IsoPaintBrushShape::Jagged => "jagged",
            IsoPaintBrushShape::Scratch => "scratch",
            IsoPaintBrushShape::Wash => "wash",
        }
    }

    fn brush_shape_from_key(key: &str) -> IsoPaintBrushShape {
        match key {
            "soft" => IsoPaintBrushShape::Soft,
            "dirt" => IsoPaintBrushShape::Dirt,
            "speckle" => IsoPaintBrushShape::Speckle,
            "jagged" => IsoPaintBrushShape::Jagged,
            "scratch" => IsoPaintBrushShape::Scratch,
            "wash" => IsoPaintBrushShape::Wash,
            _ => IsoPaintBrushShape::Solid,
        }
    }

    fn brush_shape_index(shape: IsoPaintBrushShape) -> i32 {
        match shape {
            IsoPaintBrushShape::Solid => 0,
            IsoPaintBrushShape::Soft => 1,
            IsoPaintBrushShape::Dirt => 2,
            IsoPaintBrushShape::Speckle => 3,
            IsoPaintBrushShape::Jagged => 4,
            IsoPaintBrushShape::Scratch => 5,
            IsoPaintBrushShape::Wash => 6,
        }
    }

    fn brush_shape_from_index(index: usize) -> IsoPaintBrushShape {
        match index {
            1 => IsoPaintBrushShape::Soft,
            2 => IsoPaintBrushShape::Dirt,
            3 => IsoPaintBrushShape::Speckle,
            4 => IsoPaintBrushShape::Jagged,
            5 => IsoPaintBrushShape::Scratch,
            6 => IsoPaintBrushShape::Wash,
            _ => IsoPaintBrushShape::Solid,
        }
    }

    fn brush_shape_label(shape: IsoPaintBrushShape) -> String {
        match shape {
            IsoPaintBrushShape::Solid => fl!("iso_paint_brush_shape_solid"),
            IsoPaintBrushShape::Soft => fl!("iso_paint_brush_shape_soft"),
            IsoPaintBrushShape::Dirt => fl!("iso_paint_brush_shape_dirt"),
            IsoPaintBrushShape::Speckle => fl!("iso_paint_brush_shape_speckle"),
            IsoPaintBrushShape::Jagged => fl!("iso_paint_brush_shape_jagged"),
            IsoPaintBrushShape::Scratch => fl!("iso_paint_brush_shape_scratch"),
            IsoPaintBrushShape::Wash => fl!("iso_paint_brush_shape_wash"),
        }
    }

    fn brush_shape_values() -> [IsoPaintBrushShape; 7] {
        [
            IsoPaintBrushShape::Solid,
            IsoPaintBrushShape::Soft,
            IsoPaintBrushShape::Dirt,
            IsoPaintBrushShape::Speckle,
            IsoPaintBrushShape::Jagged,
            IsoPaintBrushShape::Scratch,
            IsoPaintBrushShape::Wash,
        ]
    }

    fn brush_shape_key_from_index(index: usize) -> &'static str {
        Self::brush_shape_key(Self::brush_shape_from_index(index))
    }

    fn brush_shape_icon_items() -> Vec<TheScrollableIconRowItem> {
        Self::brush_shape_values()
            .iter()
            .enumerate()
            .map(|(index, shape)| {
                let label = Self::brush_shape_label(*shape);
                let icon_w =
                    ISO_PAINT_SHAPE_STRIP_TILE_WIDTH - ISO_PAINT_SHAPE_STRIP_ICON_PADDING * 2;
                let icon_h =
                    ISO_PAINT_SHAPE_STRIP_TILE_HEIGHT - ISO_PAINT_SHAPE_STRIP_ICON_PADDING * 2;
                let mut icon = TheRGBABuffer::new(TheDim::sized(icon_w, icon_h));
                let stride = icon.stride();
                IsoPaintBrushShapeStrip::draw_shape_icon(
                    &mut icon,
                    &(0, 0, icon_w as usize, icon_h as usize),
                    stride,
                    Self::brush_shape_key_from_index(index),
                );
                TheScrollableIconRowItem {
                    label: label.clone(),
                    status: label,
                    icon: Some(icon),
                }
            })
            .collect()
    }

    fn brush_color_slot_from_id(name: &str) -> Option<usize> {
        if name == ISO_PAINT_ACTIVE_BRUSH_COLOR {
            return Some(0);
        }
        name.strip_prefix(ISO_PAINT_ACTIVE_BRUSH_COLOR)
            .and_then(|suffix| suffix.trim().parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
    }

    fn material_index_from_key(key: &str) -> i32 {
        MATERIAL_PRESET_VALUES
            .iter()
            .position(|value| *value == key)
            .unwrap_or(0) as i32
    }

    fn finish_index_from_key(key: &str) -> i32 {
        MATERIAL_FINISH_VALUES
            .iter()
            .position(|value| *value == key)
            .unwrap_or(0) as i32
    }

    fn default_brush_material_presets() -> [i32; ISO_PAINT_BRUSH_COUNT] {
        std::array::from_fn(|index| {
            let key = match Self::BRUSHES[index].key {
                "brick" | "crack" | "rubble" => "stone",
                "moss" | "grass" | "leaves" | "flowers" | "vines" | "bushes" | "tree" => "foliage",
                "roots" => "wood",
                "candles" => "emissive",
                "dirt" | "footprints" | "mud" => "dirt",
                "puddle" => "water",
                _ => "default",
            };
            Self::material_index_from_key(key)
        })
    }

    fn default_brush_material_finishes() -> [i32; ISO_PAINT_BRUSH_COUNT] {
        std::array::from_fn(|index| {
            let key = match Self::BRUSHES[index].key {
                "puddle" => "wet",
                "dirt" => "matte",
                "mud" => "wet",
                _ => "natural",
            };
            Self::finish_index_from_key(key)
        })
    }

    fn enforce_special_brush_settings(&mut self) {
        if self.selected_preset().key == "puddle" {
            self.material_preset = Self::material_index_from_key("water");
            self.material_finish = Self::finish_index_from_key("wet");
            self.material_mode = IsoPaintMaterialMode::Coat;
            if self.opacity >= 0.99 {
                self.opacity = Self::BRUSHES[self.selected_brush].opacity;
                self.brush_opacities[self.selected_brush] = self.opacity;
            }
            if self.brush_shape == IsoPaintBrushShape::Solid {
                self.brush_shape = IsoPaintBrushShape::Wash;
                self.brush_shapes[self.selected_brush] = self.brush_shape;
            }
            self.brush_material_presets[self.selected_brush] = self.material_preset;
            self.brush_material_finishes[self.selected_brush] = self.material_finish;
        } else if !Self::brush_supports_stamp(self.selected_preset().key)
            && self.material_mode == IsoPaintMaterialMode::Stamp
        {
            self.material_mode = IsoPaintMaterialMode::Coat;
        }
    }

    fn operation_index(operation: IsoPaintOperation) -> i32 {
        match operation {
            IsoPaintOperation::Draw => 0,
            IsoPaintOperation::Erase => 1,
            IsoPaintOperation::Pick => 2,
            IsoPaintOperation::Select => 3,
        }
    }

    fn operation_from_index(index: usize) -> IsoPaintOperation {
        match index {
            1 => IsoPaintOperation::Erase,
            2 => IsoPaintOperation::Pick,
            3 => IsoPaintOperation::Select,
            _ => IsoPaintOperation::Draw,
        }
    }

    fn operation_from_key(key: &str) -> IsoPaintOperation {
        match key {
            "erase" => IsoPaintOperation::Erase,
            "pick" => IsoPaintOperation::Pick,
            "select" => IsoPaintOperation::Select,
            _ => IsoPaintOperation::Draw,
        }
    }

    fn operation_label(operation: IsoPaintOperation) -> String {
        match operation {
            IsoPaintOperation::Draw => fl!("iso_paint_operation_draw"),
            IsoPaintOperation::Erase => fl!("iso_paint_operation_erase"),
            IsoPaintOperation::Pick => fl!("iso_paint_operation_pick"),
            IsoPaintOperation::Select => fl!("iso_paint_operation_select"),
        }
    }

    fn operation_key(operation: IsoPaintOperation) -> &'static str {
        match operation {
            IsoPaintOperation::Draw => "draw",
            IsoPaintOperation::Erase => "erase",
            IsoPaintOperation::Pick => "pick",
            IsoPaintOperation::Select => "select",
        }
    }

    fn clip_key(clip: IsoPaintClipMode) -> &'static str {
        match clip {
            IsoPaintClipMode::None => "none",
            IsoPaintClipMode::Surface => "surface",
        }
    }

    fn clip_index(clip: IsoPaintClipMode) -> i32 {
        match clip {
            IsoPaintClipMode::None => 0,
            IsoPaintClipMode::Surface => 1,
        }
    }

    fn clip_from_index(index: usize) -> IsoPaintClipMode {
        match index {
            0 => IsoPaintClipMode::None,
            _ => IsoPaintClipMode::Surface,
        }
    }

    fn clip_label(clip: IsoPaintClipMode) -> String {
        match clip {
            IsoPaintClipMode::None => fl!("iso_paint_clip_none"),
            IsoPaintClipMode::Surface => fl!("iso_paint_clip_object"),
        }
    }

    fn clip_from_key(key: &str) -> IsoPaintClipMode {
        match key {
            "none" => IsoPaintClipMode::None,
            _ => IsoPaintClipMode::Surface,
        }
    }

    fn pattern_kind_key(pattern_kind: IsoPaintPatternKind) -> &'static str {
        match pattern_kind {
            IsoPaintPatternKind::Tiles => "tile",
            IsoPaintPatternKind::Bricks => "brick",
            IsoPaintPatternKind::Arch => "arch",
        }
    }

    fn pattern_kind_from_key(key: &str) -> IsoPaintPatternKind {
        match key {
            "tile" | "tiles" => IsoPaintPatternKind::Tiles,
            "arch" | "trim" => IsoPaintPatternKind::Arch,
            _ => IsoPaintPatternKind::Bricks,
        }
    }

    fn pattern_kind_index(pattern_kind: IsoPaintPatternKind) -> i32 {
        match pattern_kind {
            IsoPaintPatternKind::Tiles => 0,
            IsoPaintPatternKind::Bricks => 1,
            IsoPaintPatternKind::Arch => 2,
        }
    }

    fn pattern_kind_from_index(index: i32) -> IsoPaintPatternKind {
        match index {
            0 => IsoPaintPatternKind::Tiles,
            2 => IsoPaintPatternKind::Arch,
            _ => IsoPaintPatternKind::Bricks,
        }
    }

    fn material_mode_key(material_mode: IsoPaintMaterialMode) -> &'static str {
        match material_mode {
            IsoPaintMaterialMode::Coat => "coat",
            IsoPaintMaterialMode::Replace => "replace",
            IsoPaintMaterialMode::Stamp => "stamp",
        }
    }

    fn material_mode_from_key(key: &str) -> IsoPaintMaterialMode {
        match key {
            "replace" => IsoPaintMaterialMode::Replace,
            "stamp" => IsoPaintMaterialMode::Stamp,
            _ => IsoPaintMaterialMode::Coat,
        }
    }

    fn material_mode_labels(allow_stamp: bool) -> Vec<String> {
        let mut labels = vec![
            fl!("iso_paint_material_mode_coat"),
            fl!("iso_paint_material_mode_replace"),
        ];
        if allow_stamp {
            labels.push("Stamp".to_string());
        }
        labels
    }

    fn material_mode_index(&self, allow_stamp: bool) -> i32 {
        match self.material_mode {
            IsoPaintMaterialMode::Replace => 1,
            IsoPaintMaterialMode::Stamp if allow_stamp => 2,
            _ => 0,
        }
    }

    fn flower_type_labels() -> Vec<String> {
        vec![
            "Wildflowers".to_string(),
            "Daisies".to_string(),
            "Poppies".to_string(),
            "Bluebells".to_string(),
        ]
    }

    fn flower_type_key(index: i32) -> &'static str {
        match index {
            1 => "daisies",
            2 => "poppies",
            3 => "bluebells",
            _ => "wildflowers",
        }
    }

    fn flower_type_index(key: &str) -> i32 {
        match key {
            "daisies" => 1,
            "poppies" => 2,
            "bluebells" => 3,
            _ => 0,
        }
    }

    fn selected_flower_type_key(&self) -> &'static str {
        Self::flower_type_key(self.flower_type)
    }

    fn pattern_kind_labels() -> Vec<String> {
        vec![
            fl!("iso_paint_pattern_tiles"),
            fl!("iso_paint_pattern_bricks"),
            fl!("iso_paint_pattern_arch"),
        ]
    }

    fn selected_material_key(&self) -> &'static str {
        MATERIAL_PRESET_VALUES
            .get(self.material_preset.max(0) as usize)
            .copied()
            .unwrap_or(MATERIAL_PRESET_VALUES[0])
    }

    fn selected_finish_key(&self) -> &'static str {
        MATERIAL_FINISH_VALUES
            .get(self.material_finish.max(0) as usize)
            .copied()
            .unwrap_or(MATERIAL_FINISH_VALUES[0])
    }

    fn selected_palette_color(project: &Project) -> [u8; 4] {
        let mut color = project
            .art_palette
            .get_current_color()
            .map(|color| color.to_u8_array())
            .unwrap_or([132, 132, 128, 255]);
        color[3] = 255;
        color
    }

    fn palette_color(project: &Project, index: u16) -> [u8; 4] {
        let mut color = project
            .art_palette
            .colors
            .get(index as usize)
            .and_then(|entry| entry.as_ref())
            .map(|color| color.to_u8_array())
            .unwrap_or_else(|| Self::selected_palette_color(project));
        color[3] = 255;
        color
    }

    fn material_color_needs_gray(index: u16, color: [u8; 4]) -> bool {
        let average = (color[0] as u16 + color[1] as u16 + color[2] as u16) / 3;
        index == 0 || average < 58 || (color[0] > 150 && color[1] < 90 && color[2] < 90)
    }

    fn neutral_material_palette_index(project: &Project) -> u16 {
        Self::nearest_palette_index(project, [132, 132, 128], &[])
            .unwrap_or(Self::default_brush_color_slots()[0][0])
    }

    fn brush_palette_tone(brush: &str, slot: usize) -> f32 {
        let tones = match brush {
            "dirt" => [1.0, 0.72, 0.50, 0.34],
            "brick" => [1.0, 0.72, 1.22, 0.42],
            "moss" | "grass" => [1.0, 0.76, 1.18, 0.48],
            "crack" => [1.0, 0.58, 1.32, 0.40],
            "rubble" => [1.0, 0.72, 1.18, 0.46],
            "leaves" => [1.0, 0.72, 1.28, 0.44],
            "flowers" => [1.0, 1.28, 1.55, 1.42],
            "footprints" => [1.0, 0.78, 0.58, 0.36],
            "mud" => [1.0, 0.76, 0.54, 1.24],
            _ => [1.0, 0.78, 1.16, 0.52],
        };
        tones[slot.min(tones.len().saturating_sub(1))]
    }

    fn palette_tone_target(base: [u8; 4], tone: f32) -> [u8; 3] {
        if tone >= 1.0 {
            let mix = (tone - 1.0).clamp(0.0, 0.7);
            [
                (base[0] as f32 + (255.0 - base[0] as f32) * mix).round() as u8,
                (base[1] as f32 + (255.0 - base[1] as f32) * mix).round() as u8,
                (base[2] as f32 + (255.0 - base[2] as f32) * mix).round() as u8,
            ]
        } else {
            [
                (base[0] as f32 * tone).round().clamp(0.0, 255.0) as u8,
                (base[1] as f32 * tone).round().clamp(0.0, 255.0) as u8,
                (base[2] as f32 * tone).round().clamp(0.0, 255.0) as u8,
            ]
        }
    }

    fn nearest_palette_index(project: &Project, target: [u8; 3], used: &[u16]) -> Option<u16> {
        let mut best_any: Option<(u16, u32)> = None;
        let mut best_unused: Option<(u16, u32)> = None;
        for (index, entry) in project.art_palette.colors.iter().enumerate() {
            let Some(color) = entry.as_ref() else {
                continue;
            };
            let color = color.to_u8_array();
            let dr = color[0] as i32 - target[0] as i32;
            let dg = color[1] as i32 - target[1] as i32;
            let db = color[2] as i32 - target[2] as i32;
            let score = (dr * dr + dg * dg + db * db) as u32;
            let index = index as u16;
            if best_any.map_or(true, |(_, best)| score < best) {
                best_any = Some((index, score));
            }
            if !used.contains(&index) && best_unused.map_or(true, |(_, best)| score < best) {
                best_unused = Some((index, score));
            }
        }
        best_unused.or(best_any).map(|(index, _)| index)
    }

    fn repick_related_brush_colors(&mut self, project: &Project, brush: &str, base_index: u16) {
        let count = Self::brush_color_slot_count(brush);
        if count <= 1 {
            return;
        }

        let base = Self::palette_color(project, base_index);
        let slots = &mut self.brush_color_slots[self.selected_brush];
        slots[0] = base_index;
        let mut used = vec![base_index];
        for slot in 1..count.min(slots.len()) {
            let tone = Self::brush_palette_tone(brush, slot);
            let target = Self::palette_tone_target(base, tone);
            if let Some(index) = Self::nearest_palette_index(project, target, &used) {
                slots[slot] = index;
                used.push(index);
            }
        }
    }

    fn current_palette_indices(&self) -> Vec<u16> {
        let brush = self.selected_preset();
        let count = Self::brush_color_slot_count(brush.key);
        self.brush_color_slots[self.selected_brush][..count].to_vec()
    }

    fn current_palette_colors(&self, project: &Project) -> Vec<[u8; 4]> {
        self.current_palette_indices()
            .into_iter()
            .map(|index| Self::palette_color(project, index))
            .collect()
    }

    fn all_brush_palette_colors(&self, project: &Project) -> Vec<Vec<[u8; 4]>> {
        Self::BRUSHES
            .iter()
            .enumerate()
            .map(|(brush_index, brush)| {
                let count = Self::brush_color_slot_count(brush.key);
                self.brush_color_slots[brush_index][..count]
                    .iter()
                    .map(|index| Self::palette_color(project, *index))
                    .collect()
            })
            .collect()
    }

    fn brush_label(key: &str) -> String {
        match key {
            "material" => fl!("iso_paint_brush_material"),
            "brick" => fl!("iso_paint_brush_brick"),
            "moss" => fl!("iso_paint_brush_moss"),
            "crack" => fl!("iso_paint_brush_crack"),
            "grass" => "Grass".to_string(),
            "rubble" => "Rubble".to_string(),
            "leaves" => "Leaves".to_string(),
            "flowers" => "Flowers".to_string(),
            "vines" => "Vines".to_string(),
            "roots" => "Roots".to_string(),
            "bushes" => "Bushes".to_string(),
            "tree" => "Tree".to_string(),
            "candles" => "Candles".to_string(),
            "footprints" => "Footprints".to_string(),
            "mud" => "Mud".to_string(),
            "puddle" => fl!("iso_paint_brush_puddle"),
            "dirt" => fl!("material_preset_dirt"),
            _ => key.to_string(),
        }
    }

    fn brush_description(key: &str) -> String {
        match key {
            "material" => fl!("iso_paint_brush_material_desc"),
            "brick" => fl!("iso_paint_brush_brick_desc"),
            "moss" => fl!("iso_paint_brush_moss_desc"),
            "crack" => fl!("iso_paint_brush_crack_desc"),
            "grass" => "Paint a randomized green ground foliage brush.".to_string(),
            "rubble" => "Place loose stone chips as scene-aware stamps.".to_string(),
            "leaves" => "Scatter fallen leaves as scene-aware stamps.".to_string(),
            "flowers" => "Place small wildflower clusters as scene-aware stamps.".to_string(),
            "vines" => "Place creeping vine tendrils as scene-aware stamps.".to_string(),
            "roots" => "Place branching exposed roots as scene-aware stamps.".to_string(),
            "bushes" => "Place larger leafy bush clusters as scene-aware stamps.".to_string(),
            "tree" => "Place a small painted tree stamp with trunk and canopy.".to_string(),
            "candles" => "Place small candle clusters as scene-aware stamps.".to_string(),
            "footprints" => "Place muddy paired footprints as scene-aware stamps.".to_string(),
            "mud" => "Place glossy mud bubbles as scene-aware stamps.".to_string(),
            "puddle" => fl!("iso_paint_brush_puddle_desc"),
            "dirt" => "Paint dirt, dust, and grime onto the surface.".to_string(),
            _ => String::new(),
        }
    }

    fn build_nodeui(&self, project: &Project) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();
        let brush = self.selected_preset();
        let size_max = self.selected_size_max();

        nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_brush")));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            self.ui_name(ISO_PAINT_TOOL_SIZE),
            fl!("iso_paint_size"),
            fl!("status_iso_paint_size"),
            self.size.clamp(ISO_PAINT_MIN_BRUSH_SIZE, size_max),
            ISO_PAINT_MIN_BRUSH_SIZE..=size_max,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            self.ui_name(ISO_PAINT_TOOL_OPACITY),
            fl!("iso_paint_opacity"),
            fl!("status_iso_paint_opacity"),
            self.opacity,
            0.0..=1.0,
            true,
        ));

        let color_count = Self::brush_color_slot_count(brush.key);
        if color_count > 0 {
            nodeui.add_item(TheNodeUIItem::PaletteIndexRowPicker(
                self.ui_name(ISO_PAINT_ACTIVE_BRUSH_COLOR),
                fl!("iso_paint_color"),
                fl!("status_iso_paint_color_slot"),
                self.brush_color_slots[self.selected_brush][..color_count]
                    .iter()
                    .map(|index| *index as i32)
                    .collect(),
                project.art_palette.clone(),
            ));
        }
        if brush.key == "brick" {
            nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_pattern_kind")));
            nodeui.add_item(TheNodeUIItem::Selector(
                self.ui_name(ISO_PAINT_PATTERN_KIND),
                fl!("iso_paint_pattern_kind"),
                fl!("status_iso_paint_pattern_kind"),
                Self::pattern_kind_labels(),
                Self::pattern_kind_index(self.pattern_kind),
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                self.ui_name(ISO_PAINT_PATTERN_SCALE),
                fl!("iso_paint_pattern_scale"),
                fl!("status_iso_paint_pattern_scale"),
                self.pattern_scale,
                0.25..=4.0,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                self.ui_name(ISO_PAINT_MORTAR),
                fl!("iso_paint_mortar"),
                fl!("status_iso_paint_mortar"),
                self.pattern_mortar,
                0.0..=0.4,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                self.ui_name(ISO_PAINT_PATTERN_DETAIL),
                fl!("iso_paint_pattern_detail"),
                fl!("status_iso_paint_pattern_detail"),
                self.pattern_detail,
                0.0..=1.0,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                self.ui_name(ISO_PAINT_PATTERN_VARIATION),
                fl!("iso_paint_pattern_variation"),
                fl!("status_iso_paint_pattern_variation"),
                self.pattern_variation,
                0.0..=1.0,
                true,
            ));
        }
        let allow_stamp_mode = Self::brush_supports_stamp(brush.key);
        let stamp_mode = allow_stamp_mode && self.material_mode == IsoPaintMaterialMode::Stamp;
        if stamp_mode {
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                self.ui_name(ISO_PAINT_STAMP_DENSITY),
                "Density".to_string(),
                fl!("status_iso_paint_stamp_density"),
                self.stamp_density,
                0.0..=1.0,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                self.ui_name(ISO_PAINT_STAMP_SIZE_JITTER),
                "Radius".to_string(),
                "Randomize stamp size.".to_string(),
                self.stamp_size_jitter,
                0.0..=1.0,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                self.ui_name(ISO_PAINT_STAMP_ROTATION_JITTER),
                "Rotate".to_string(),
                "Randomize stamp rotation.".to_string(),
                self.stamp_rotation_jitter,
                0.0..=1.0,
                true,
            ));
        }
        if brush.key != "puddle" {
            nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_material")));
            nodeui.add_item(TheNodeUIItem::Selector(
                self.ui_name(ISO_PAINT_MATERIAL_MODE),
                fl!("iso_paint_material_mode"),
                fl!("status_iso_paint_material_mode"),
                Self::material_mode_labels(allow_stamp_mode),
                self.material_mode_index(allow_stamp_mode),
            ));
            if brush.key == "flowers" && stamp_mode {
                nodeui.add_item(TheNodeUIItem::Selector(
                    self.ui_name(ISO_PAINT_FLOWER_TYPE),
                    "Flower Type".to_string(),
                    "Choose the flower stamp variety.".to_string(),
                    Self::flower_type_labels(),
                    self.flower_type,
                ));
            }
        }

        nodeui
    }

    fn inspector_item_column(&self, item: &TheNodeUIItem) -> usize {
        match item {
            TheNodeUIItem::Separator(name) if name == &fl!("iso_paint_section_brush") => 0,
            TheNodeUIItem::FloatEditSlider(id, _, _, _, _, _)
                if matches!(
                    self.base_ui_name(id),
                    ISO_PAINT_TOOL_SIZE | ISO_PAINT_TOOL_OPACITY
                ) =>
            {
                0
            }
            TheNodeUIItem::PaletteIndexRowPicker(id, _, _, _, _)
                if self.base_ui_name(id) == ISO_PAINT_ACTIVE_BRUSH_COLOR =>
            {
                0
            }
            TheNodeUIItem::FloatEditSlider(id, _, _, _, _, _)
                if matches!(
                    self.base_ui_name(id),
                    ISO_PAINT_STAMP_DENSITY
                        | ISO_PAINT_STAMP_SIZE_JITTER
                        | ISO_PAINT_STAMP_ROTATION_JITTER
                ) =>
            {
                0
            }
            _ => 1,
        }
    }

    fn split_inspector_nodeui(&self) -> (TheNodeUI, TheNodeUI) {
        let mut primary = TheNodeUI::default();
        let mut detail = TheNodeUI::default();
        for (_, item) in self.nodeui.list_items() {
            if matches!(item, TheNodeUIItem::Separator(_)) {
                continue;
            }
            if self.inspector_item_column(item) == 0 {
                primary.add_item(item.clone());
            } else {
                detail.add_item(item.clone());
            }
        }
        (primary, detail)
    }

    fn apply_inspector_layouts(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        let (primary, detail) = self.split_inspector_nodeui();
        if let Some(layout) = ui.get_text_layout(&self.ui_name(ISO_PAINT_INSPECTOR_PRIMARY)) {
            primary.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
        if let Some(layout) = ui.get_text_layout(&self.ui_name(ISO_PAINT_INSPECTOR_DETAIL)) {
            detail.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
    }

    fn selected_preset(&self) -> IsoPaintBrushPreset {
        Self::BRUSHES
            .get(self.selected_brush)
            .copied()
            .unwrap_or(Self::BRUSHES[0])
    }

    fn brush_supports_stamp(key: &str) -> bool {
        matches!(
            key,
            "grass"
                | "grass_stamp"
                | "rubble"
                | "leaves"
                | "flowers"
                | "vines"
                | "roots"
                | "bushes"
                | "tree"
                | "candles"
                | "footprints"
                | "mud"
        )
    }

    fn size_max_for(brush: &str, material_mode: IsoPaintMaterialMode) -> f32 {
        if material_mode == IsoPaintMaterialMode::Stamp && Self::brush_supports_stamp(brush) {
            ISO_PAINT_MAX_STAMP_BRUSH_SIZE
        } else {
            ISO_PAINT_MAX_PAINT_BRUSH_SIZE
        }
    }

    fn selected_size_max(&self) -> f32 {
        Self::size_max_for(self.selected_preset().key, self.material_mode)
    }

    fn clamp_size_for_selection(&self, size: f32) -> f32 {
        size.clamp(ISO_PAINT_MIN_BRUSH_SIZE, self.selected_size_max())
    }

    fn brush_index_from_key(key: &str) -> usize {
        let key = match key {
            "screen" => "dirt",
            "grass_stamp" => "grass",
            _ => key,
        };
        Self::BRUSHES
            .iter()
            .position(|brush| brush.key == key)
            .unwrap_or(0)
    }

    fn sync_inspector(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        self.nodeui = self.build_nodeui(project);
        self.sync_toolbar(ui, ctx);
        let editor_preview_mode = if self.material_mode == IsoPaintMaterialMode::Stamp {
            IsoPaintPreviewMode::Stamp
        } else {
            IsoPaintPreviewMode::Paint
        };
        if let Some(widget) = ui.get_widget(&self.ui_name(ISO_PAINT_PRESET_STRIP))
            && let Some(strip) = widget.as_any().downcast_mut::<IsoPaintPresetStrip>()
        {
            strip.set_selected(self.selected_brush);
            strip.set_preview_palettes(self.all_brush_palette_colors(project));
        }
        if let Some(widget) = ui.get_widget(&self.ui_name(ISO_PAINT_MATERIAL_STRIP))
            && let Some(strip) = widget.as_any().downcast_mut::<IsoPaintMaterialStrip>()
        {
            strip.set_material(self.material_preset, self.material_finish);
        }
        if let Some(widget) = ui.get_widget(&self.ui_name(ISO_PAINT_BRUSH_EDITOR))
            && let Some(editor) = widget.as_any().downcast_mut::<IsoPaintBrushEditor>()
        {
            editor.set_selected_brush(self.selected_brush);
            editor.set_preview_palette(self.current_palette_colors(project));
            editor.set_preview_mode(editor_preview_mode);
        }
        if let Some(widget) = ui.get_widget(&self.ui_name(ISO_PAINT_BRUSH_SHAPE_GROUP))
            && let Some(strip) = widget.as_any().downcast_mut::<TheScrollableIconRow>()
        {
            strip.set_selected(Self::brush_shape_index(self.brush_shape).max(0) as usize);
        }
        self.apply_inspector_layouts(ui, ctx);
    }

    fn sync_toolbar(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        ui.set_widget_value(
            &self.ui_name(ISO_PAINT_OPERATION_GROUP),
            ctx,
            TheValue::Int(Self::operation_index(self.operation)),
        );
        ctx.ui.set_widget_state(
            self.ui_name(ISO_PAINT_LAYER_VISIBLE),
            if self.paint_visible {
                TheWidgetState::Selected
            } else {
                TheWidgetState::None
            },
        );
        ui.set_widget_value(
            &self.ui_name(ISO_PAINT_CLIP_GROUP),
            ctx,
            TheValue::Int(Self::clip_index(self.clip_mode)),
        );
    }

    fn select_brush(
        &mut self,
        index: usize,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) {
        let current = self
            .selected_brush
            .min(Self::BRUSHES.len().saturating_sub(1));
        self.brush_sizes[current] = self.size;
        self.brush_opacities[current] = self.opacity;
        self.brush_shapes[current] = self.brush_shape;
        self.brush_material_presets[current] = self.material_preset;
        self.brush_material_finishes[current] = self.material_finish;
        self.selected_brush = index.min(Self::BRUSHES.len().saturating_sub(1));
        let color_count = Self::brush_color_slot_count(self.selected_preset().key);
        self.selected_color_slot = self.selected_color_slot.min(color_count.saturating_sub(1));
        let brush = self.selected_preset();
        self.size = self.brush_sizes[self.selected_brush];
        self.opacity = self.brush_opacities[self.selected_brush];
        self.brush_shape = self.brush_shapes[self.selected_brush];
        self.material_preset = self.brush_material_presets[self.selected_brush];
        self.material_finish = self.brush_material_finishes[self.selected_brush];
        if matches!(
            brush.key,
            "rubble"
                | "leaves"
                | "flowers"
                | "vines"
                | "roots"
                | "bushes"
                | "tree"
                | "candles"
                | "footprints"
                | "mud"
        ) {
            self.material_mode = IsoPaintMaterialMode::Stamp;
        }
        self.size = self.clamp_size_for_selection(self.size);
        self.brush_sizes[self.selected_brush] = self.size;
        self.enforce_special_brush_settings();
        self.sync_inspector(ui, ctx, project);
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            format!(
                "{}: {} ({})",
                fl!("iso_paint_selected_brush"),
                Self::brush_label(brush.key),
                Self::operation_label(self.operation)
            ),
        ));
    }

    fn set_operation(
        &mut self,
        operation: IsoPaintOperation,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.operation = operation;
        self.sync_toolbar(_ui, ctx);
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            format!(
                "{}: {}",
                fl!("iso_paint_operation"),
                Self::operation_label(operation)
            ),
        ));
    }

    fn set_clip_mode(&mut self, clip_mode: IsoPaintClipMode, ui: &mut TheUI, ctx: &mut TheContext) {
        self.clip_mode = clip_mode;
        self.sync_toolbar(ui, ctx);
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            format!("{}: {}", fl!("iso_paint_clip"), Self::clip_label(clip_mode)),
        ));
    }

    fn sync_project_settings(&self, project: &mut Project, server_ctx: &ServerContext) {
        let brush = self.selected_preset();
        let palette_indices = self.current_palette_indices();
        let palette_colors = self.current_palette_colors(project);
        let color = palette_colors
            .first()
            .copied()
            .unwrap_or_else(|| Self::selected_palette_color(project));
        let material_key = if brush.key == "puddle" {
            "water"
        } else {
            self.selected_material_key()
        };
        let finish_key = if brush.key == "puddle" {
            "wet"
        } else {
            self.selected_finish_key()
        };
        let material_mode_key = if brush.key == "puddle" {
            "coat"
        } else {
            Self::material_mode_key(self.material_mode)
        };
        let stamp_mode = Self::brush_supports_stamp(brush.key)
            && self.material_mode == IsoPaintMaterialMode::Stamp;
        let stamp_variant_key = if stamp_mode && brush.key == "flowers" {
            self.selected_flower_type_key()
        } else {
            "wildflowers"
        };
        let material_id = MaterialDefinition::from_preset_finish(material_key, finish_key).id();
        let Some(paint) = self.paint_layer_mut(project, server_ctx) else {
            return;
        };
        paint.set_active_settings(
            Self::operation_key(self.operation),
            brush.key,
            Self::brush_shape_key(self.brush_shape),
            material_key,
            finish_key,
            material_id,
            material_mode_key,
            Self::clip_key(self.clip_mode),
            color,
            palette_indices,
            palette_colors,
            Self::pattern_kind_key(self.pattern_kind),
            self.pattern_scale,
            self.pattern_mortar,
            self.pattern_detail,
            self.pattern_variation,
            self.stamp_density,
            self.stamp_size_jitter,
            self.stamp_rotation_jitter,
            stamp_variant_key,
            self.size,
            self.opacity,
        );
    }
}

impl Dock for IsoPaintDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        let brush_material_presets = Self::default_brush_material_presets();
        let brush_material_finishes = Self::default_brush_material_finishes();
        let brush_shapes = Self::default_brush_shapes();
        let brush_color_slots = Self::default_brush_color_slots();
        Self {
            target: IsoPaintTarget::Region,
            ui_prefix: String::new(),
            selected_brush: 0,
            operation: IsoPaintOperation::Draw,
            size: Self::BRUSHES[0].size,
            opacity: Self::BRUSHES[0].opacity,
            brush_shapes,
            brush_sizes: Self::default_brush_sizes(),
            brush_opacities: Self::default_brush_opacities(),
            brush_color_slots,
            selected_color_slot: 0,
            brush_material_presets,
            brush_material_finishes,
            material_preset: brush_material_presets[0],
            material_finish: brush_material_finishes[0],
            material_mode: IsoPaintMaterialMode::Coat,
            brush_shape: brush_shapes[0],
            paint_visible: true,
            clip_mode: IsoPaintClipMode::Surface,
            pattern_kind: IsoPaintPatternKind::Bricks,
            pattern_scale: Self::BRUSHES[0].pattern_scale,
            pattern_mortar: Self::BRUSHES[0].mortar,
            pattern_detail: 0.65,
            pattern_variation: 0.6,
            stamp_density: Self::BRUSHES[0].density,
            stamp_size_jitter: 0.25,
            stamp_rotation_jitter: 1.0,
            flower_type: 0,
            nodeui: TheNodeUI::default(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar = TheHLayout::new(TheId::named("3D Paint Toolbar"));
        toolbar.set_background_color(None);
        toolbar.set_margin(Vec4::new(10, 2, 6, 2));
        toolbar.set_padding(7);

        let mut operation_group = TheGroupButton::new(self.ui_id(ISO_PAINT_OPERATION_GROUP));
        operation_group.add_text_status(
            fl!("iso_paint_operation_draw"),
            fl!("status_iso_paint_operation_draw"),
        );
        operation_group.add_text_status(
            fl!("iso_paint_operation_erase"),
            fl!("status_iso_paint_operation_erase"),
        );
        operation_group.add_text_status(
            fl!("iso_paint_operation_pick"),
            fl!("status_iso_paint_operation_pick"),
        );
        operation_group.add_text_status(
            fl!("iso_paint_operation_select"),
            fl!("status_iso_paint_operation_select"),
        );
        operation_group.set_item_width(66);
        operation_group.set_index(Self::operation_index(self.operation));
        toolbar.add_widget(Box::new(operation_group));

        let mut fill_selection = TheTraybarButton::new(self.ui_id(ISO_PAINT_FILL_SELECTION));
        fill_selection.set_text(fl!("iso_paint_fill_selection"));
        fill_selection.set_status_text(&fl!("status_iso_paint_fill_selection"));
        fill_selection.set_fixed_size(false);
        fill_selection.limiter_mut().set_min_width(54);
        fill_selection.limiter_mut().set_max_width(54);
        toolbar.add_widget(Box::new(fill_selection));

        let mut clear_selection = TheTraybarButton::new(self.ui_id(ISO_PAINT_CLEAR_SELECTION));
        clear_selection.set_text(fl!("iso_paint_clear_selection"));
        clear_selection.set_status_text(&fl!("status_iso_paint_clear_selection"));
        clear_selection.set_fixed_size(false);
        clear_selection.limiter_mut().set_min_width(64);
        clear_selection.limiter_mut().set_max_width(64);
        toolbar.add_widget(Box::new(clear_selection));

        let mut layer_visible = TheTraybarButton::new(self.ui_id(ISO_PAINT_LAYER_VISIBLE));
        layer_visible.set_text(fl!("iso_paint_layer_visible"));
        layer_visible.set_status_text(&fl!("status_iso_paint_layer_visible"));
        layer_visible.set_fixed_size(false);
        layer_visible.limiter_mut().set_min_width(72);
        layer_visible.limiter_mut().set_max_width(72);
        if self.paint_visible {
            layer_visible.set_state(TheWidgetState::Selected);
        }
        toolbar.add_widget(Box::new(layer_visible));

        let mut clear_all = TheTraybarButton::new(self.ui_id(ISO_PAINT_CLEAR_ALL));
        clear_all.set_text(fl!("iso_paint_clear_all"));
        clear_all.set_status_text(&fl!("status_iso_paint_clear_all"));
        clear_all.set_fixed_size(false);
        clear_all.limiter_mut().set_min_width(82);
        clear_all.limiter_mut().set_max_width(82);
        toolbar.add_widget(Box::new(clear_all));

        let mut clip_group = TheGroupButton::new(self.ui_id(ISO_PAINT_CLIP_GROUP));
        clip_group.add_text_status(
            Self::clip_label(IsoPaintClipMode::None),
            fl!("status_iso_paint_clip"),
        );
        clip_group.add_text_status(
            Self::clip_label(IsoPaintClipMode::Surface),
            fl!("status_iso_paint_clip"),
        );
        clip_group.set_item_width(76);
        clip_group.set_index(Self::clip_index(self.clip_mode));
        toolbar.add_widget(Box::new(clip_group));

        toolbar.set_reverse_index(Some(5));

        toolbar_canvas.set_layout(toolbar);
        canvas.set_top(toolbar_canvas);

        let mut content = TheCanvas::new();

        let mut preset_canvas = TheCanvas::new();
        preset_canvas.limiter_mut().set_min_height(46);
        preset_canvas.limiter_mut().set_max_height(46);
        let mut preset_strip = IsoPaintPresetStrip::new(self.ui_id(ISO_PAINT_PRESET_STRIP));
        preset_strip.set_selected(self.selected_brush);
        preset_canvas.set_widget(preset_strip);
        content.set_top(preset_canvas);

        let mut material_canvas = TheCanvas::new();
        material_canvas.limiter_mut().set_min_height(58);
        material_canvas.limiter_mut().set_max_height(58);
        let mut material_strip = IsoPaintMaterialStrip::new(self.ui_id(ISO_PAINT_MATERIAL_STRIP));
        material_strip.set_material(self.material_preset, self.material_finish);
        material_canvas.set_widget(material_strip);
        content.set_bottom(material_canvas);

        let mut center = TheCanvas::new();

        let mut brush_panel = TheCanvas::new();

        let mut brush_editor_canvas = TheCanvas::new();
        let mut brush_editor = IsoPaintBrushEditor::new(self.ui_id(ISO_PAINT_BRUSH_EDITOR));
        brush_editor.set_selected_brush(self.selected_brush);
        brush_editor_canvas.set_widget(brush_editor);
        brush_panel.set_center(brush_editor_canvas);

        let mut shape_canvas = TheCanvas::new();
        shape_canvas.limiter_mut().set_min_height(32);
        shape_canvas.limiter_mut().set_max_height(32);
        let mut shape_strip = TheScrollableIconRow::new(self.ui_id(ISO_PAINT_BRUSH_SHAPE_GROUP));
        shape_strip.limiter_mut().set_min_height(32);
        shape_strip.limiter_mut().set_max_height(32);
        shape_strip
            .limiter_mut()
            .set_max_size(Vec2::new(i32::MAX, 32));
        shape_strip.set_tile_width(ISO_PAINT_SHAPE_STRIP_TILE_WIDTH);
        shape_strip.set_icon_padding(ISO_PAINT_SHAPE_STRIP_ICON_PADDING);
        shape_strip.set_items(Self::brush_shape_icon_items());
        shape_strip.set_selected(Self::brush_shape_index(self.brush_shape).max(0) as usize);
        shape_canvas.set_widget(shape_strip);
        brush_panel.set_bottom(shape_canvas);
        center.set_center(brush_panel);

        let mut inspector_column = TheCanvas::new();
        inspector_column.limiter_mut().set_min_width(520);
        inspector_column.limiter_mut().set_max_width(520);
        let mut inspector_sizer = TheSpacer::new(TheId::empty());
        inspector_sizer.limiter_mut().set_max_width(520);
        inspector_sizer.limiter_mut().set_max_height(i32::MAX);
        inspector_column.set_widget(inspector_sizer);

        let (primary_nodeui, detail_nodeui) = self.split_inspector_nodeui();

        let mut primary_inspector = TheTextLayout::new(self.ui_id(ISO_PAINT_INSPECTOR_PRIMARY));
        primary_inspector.set_text_margin(8);
        primary_inspector.set_fixed_text_width(58);
        primary_inspector.set_text_align(TheHorizontalAlign::Right);
        primary_inspector.limiter_mut().set_min_width(200);
        primary_inspector.limiter_mut().set_max_width(200);
        primary_nodeui.apply_to_text_layout(&mut primary_inspector);

        let mut detail_inspector = TheTextLayout::new(self.ui_id(ISO_PAINT_INSPECTOR_DETAIL));
        detail_inspector.set_text_margin(8);
        detail_inspector.set_fixed_text_width(118);
        detail_inspector.set_text_align(TheHorizontalAlign::Right);
        detail_inspector.limiter_mut().set_min_width(320);
        detail_inspector.limiter_mut().set_max_width(320);
        detail_nodeui.apply_to_text_layout(&mut detail_inspector);

        let mut primary_canvas = TheCanvas::new();
        primary_canvas.limiter_mut().set_min_width(200);
        primary_canvas.limiter_mut().set_max_width(200);
        primary_canvas.set_layout(primary_inspector);

        let mut detail_canvas = TheCanvas::new();
        detail_canvas.set_layout(detail_inspector);

        inspector_column.set_left(primary_canvas);
        inspector_column.set_center(detail_canvas);
        center.set_right(inspector_column);

        content.set_center(center);
        canvas.set_center(content);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        let paint = self
            .paint_layer(project, server_ctx)
            .cloned()
            .unwrap_or_default();
        self.operation = Self::operation_from_key(&paint.active_operation);
        self.clip_mode = Self::clip_from_key(&paint.active_clip);
        self.material_mode = Self::material_mode_from_key(&paint.active_material_mode);
        self.selected_brush = Self::brush_index_from_key(&paint.active_brush);
        if paint.active_brush == "grass_stamp" {
            self.material_mode = IsoPaintMaterialMode::Stamp;
        }
        let preset = self.selected_preset();
        self.size = if matches!(preset.key, "material" | "brick") && paint.active_size <= 1.001 {
            preset.size
        } else {
            paint
                .active_size
                .clamp(ISO_PAINT_MIN_BRUSH_SIZE, self.selected_size_max())
        };
        self.opacity = paint.active_opacity.clamp(0.0, 1.0);
        self.brush_sizes[self.selected_brush] = self.size;
        self.brush_opacities[self.selected_brush] = self.opacity;
        for (slot, index) in paint.active_palette_indices.iter().enumerate() {
            if slot < self.brush_color_slots[self.selected_brush].len() {
                self.brush_color_slots[self.selected_brush][slot] = *index;
            }
        }
        if preset.key == "material" {
            let slot = self.brush_color_slots[self.selected_brush][0];
            let color = Self::palette_color(project, slot);
            if Self::material_color_needs_gray(slot, color) {
                self.brush_color_slots[self.selected_brush][0] =
                    Self::neutral_material_palette_index(project);
            }
        }
        self.brush_shape = Self::brush_shape_from_key(&paint.active_brush_shape);
        self.brush_shapes[self.selected_brush] = self.brush_shape;
        self.material_preset = Self::material_index_from_key(&paint.active_material);
        self.material_finish = Self::finish_index_from_key(&paint.active_finish);
        self.selected_color_slot = self
            .selected_color_slot
            .min(Self::brush_color_slot_count(preset.key).saturating_sub(1));
        self.brush_material_presets[self.selected_brush] = self.material_preset;
        self.brush_material_finishes[self.selected_brush] = self.material_finish;
        self.enforce_special_brush_settings();
        self.pattern_kind = Self::pattern_kind_from_key(&paint.active_pattern_kind);
        self.pattern_scale = paint.active_pattern_scale;
        self.pattern_mortar = paint.active_pattern_mortar;
        self.pattern_detail = paint.active_pattern_detail;
        self.pattern_variation = paint.active_pattern_variation;
        self.stamp_density = paint.active_stamp_density;
        self.stamp_size_jitter = paint.active_stamp_size_jitter;
        self.stamp_rotation_jitter = paint.active_stamp_rotation_jitter;
        self.flower_type = Self::flower_type_index(&paint.active_stamp_variant);
        self.paint_visible = paint.visible;
        self.sync_inspector(ui, ctx, project);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::IndexChanged(id, index)
                if self.is_ui_event(&id.name, ISO_PAINT_OPERATION_GROUP) =>
            {
                self.set_operation(Self::operation_from_index(*index), ui, ctx);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if self.is_ui_event(&id.name, ISO_PAINT_LAYER_VISIBLE) =>
            {
                self.paint_visible = !self.paint_visible;
                if let Some(paint) = self.paint_layer_mut(project, server_ctx) {
                    paint.visible = self.paint_visible;
                }
                ctx.ui.set_widget_state(
                    self.ui_name(ISO_PAINT_LAYER_VISIBLE),
                    if self.paint_visible {
                        TheWidgetState::Selected
                    } else {
                        TheWidgetState::None
                    },
                );
                ctx.ui.redraw_all = true;
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if self.is_ui_event(&id.name, ISO_PAINT_CLEAR_ALL) =>
            {
                let target_id = match self.target {
                    IsoPaintTarget::Region => Some(server_ctx.curr_region),
                    IsoPaintTarget::Prefab => match server_ctx.pc {
                        ProjectContext::Prefab(asset_id) => Some(asset_id),
                        _ => None,
                    },
                };
                if let Some(target_id) = target_id
                    && let Some(paint) = self.paint_layer_mut(project, server_ctx)
                    && (!paint.surface_commit_strokes.is_empty()
                        || !paint.chunks.is_empty()
                        || !paint.baked_chunks.is_empty())
                {
                    let old_paint = paint.clone();
                    paint.surface_commit_strokes.clear();
                    paint.chunks.clear();
                    paint.baked_chunks.clear();
                    let active_selection = paint.active_selection_id;
                    paint
                        .selection_masks
                        .retain(|id, _| Some(*id) == active_selection);
                    let new_paint = paint.clone();
                    let undo_atom = match self.target {
                        IsoPaintTarget::Region => ProjectUndoAtom::RegionPaintEdit(
                            ProjectContext::Region(target_id),
                            target_id,
                            Box::new(old_paint),
                            Box::new(new_paint),
                        ),
                        IsoPaintTarget::Prefab => ProjectUndoAtom::PrefabPaintEdit(
                            ProjectContext::Prefab(target_id),
                            target_id,
                            Box::new(old_paint),
                            Box::new(new_paint),
                        ),
                    };
                    UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                    ctx.ui.redraw_all = true;
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_iso_paint_clear_all_done"),
                    ));
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if self.is_ui_event(&id.name, ISO_PAINT_FILL_SELECTION) =>
            {
                let target_id = match self.target {
                    IsoPaintTarget::Region => Some(server_ctx.curr_region),
                    IsoPaintTarget::Prefab => match server_ctx.pc {
                        ProjectContext::Prefab(asset_id) => Some(asset_id),
                        _ => None,
                    },
                };
                if let Some(target_id) = target_id
                    && let Some(paint) = self.paint_layer_mut(project, server_ctx)
                {
                    let old_paint = paint.clone();
                    if paint.fill_active_selection() {
                        let new_paint = paint.clone();
                        let undo_atom = match self.target {
                            IsoPaintTarget::Region => ProjectUndoAtom::RegionPaintEdit(
                                ProjectContext::Region(target_id),
                                target_id,
                                Box::new(old_paint),
                                Box::new(new_paint),
                            ),
                            IsoPaintTarget::Prefab => ProjectUndoAtom::PrefabPaintEdit(
                                ProjectContext::Prefab(target_id),
                                target_id,
                                Box::new(old_paint),
                                Box::new(new_paint),
                            ),
                        };
                        UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                        ctx.ui.redraw_all = true;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_iso_paint_fill_selection_done"),
                        ));
                    }
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if self.is_ui_event(&id.name, ISO_PAINT_CLEAR_SELECTION) =>
            {
                let target_id = match self.target {
                    IsoPaintTarget::Region => Some(server_ctx.curr_region),
                    IsoPaintTarget::Prefab => match server_ctx.pc {
                        ProjectContext::Prefab(asset_id) => Some(asset_id),
                        _ => None,
                    },
                };
                if let Some(target_id) = target_id
                    && let Some(paint) = self.paint_layer_mut(project, server_ctx)
                {
                    let old_paint = paint.clone();
                    if paint.clear_active_selection() {
                        let new_paint = paint.clone();
                        let undo_atom = match self.target {
                            IsoPaintTarget::Region => ProjectUndoAtom::RegionPaintEdit(
                                ProjectContext::Region(target_id),
                                target_id,
                                Box::new(old_paint),
                                Box::new(new_paint),
                            ),
                            IsoPaintTarget::Prefab => ProjectUndoAtom::PrefabPaintEdit(
                                ProjectContext::Prefab(target_id),
                                target_id,
                                Box::new(old_paint),
                                Box::new(new_paint),
                            ),
                        };
                        UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                        ctx.ui.redraw_all = true;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_iso_paint_clear_selection_done"),
                        ));
                    }
                }
                return true;
            }
            TheEvent::IndexChanged(id, index)
                if self.is_ui_event(&id.name, ISO_PAINT_CLIP_GROUP) =>
            {
                self.set_clip_mode(Self::clip_from_index(*index), ui, ctx);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            TheEvent::IndexChanged(id, index)
                if self.is_ui_event(&id.name, ISO_PAINT_BRUSH_SHAPE_GROUP) =>
            {
                self.brush_shape = Self::brush_shape_from_index(*index);
                self.brush_shapes[self.selected_brush] = self.brush_shape;
                self.sync_project_settings(project, server_ctx);
                ctx.ui.redraw_all = true;
                return true;
            }
            TheEvent::Custom(id, TheValue::Int(index))
                if self.is_ui_event(&id.name, ISO_PAINT_BRUSH_SELECTED) =>
            {
                self.select_brush((*index).max(0) as usize, ui, ctx, project);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            TheEvent::PaletteIndexChanged(id, index)
                if Self::brush_color_slot_from_id(self.base_ui_name(&id.name)).is_some() =>
            {
                let brush = self.selected_preset();
                let color_count = Self::brush_color_slot_count(brush.key);
                let slot = Self::brush_color_slot_from_id(self.base_ui_name(&id.name))
                    .unwrap_or(0)
                    .min(color_count.saturating_sub(1));
                self.selected_color_slot = slot;
                if slot < self.brush_color_slots[self.selected_brush].len() {
                    if slot == 0 && color_count > 1 {
                        self.repick_related_brush_colors(project, brush.key, *index);
                    } else {
                        self.brush_color_slots[self.selected_brush][slot] = *index;
                    }
                }
                self.sync_inspector(ui, ctx, project);
                self.sync_project_settings(project, server_ctx);
                ctx.ui.redraw_all = true;
                return true;
            }
            TheEvent::Custom(id, TheValue::Int(index))
                if matches!(
                    self.base_ui_name(&id.name),
                    ISO_PAINT_MATERIAL_PRESET_SELECTED | ISO_PAINT_MATERIAL_FINISH_SELECTED
                ) =>
            {
                match self.base_ui_name(&id.name) {
                    ISO_PAINT_MATERIAL_PRESET_SELECTED => {
                        let max = MATERIAL_PRESET_VALUES.len().saturating_sub(1) as i32;
                        self.material_preset = (*index).max(0).min(max);
                        if MATERIAL_PRESET_VALUES
                            .get(self.material_preset as usize)
                            .is_some_and(|preset| *preset == "default")
                        {
                            self.material_finish = 0;
                        }
                        self.brush_material_presets[self.selected_brush] = self.material_preset;
                        self.brush_material_finishes[self.selected_brush] = self.material_finish;
                    }
                    ISO_PAINT_MATERIAL_FINISH_SELECTED => {
                        let max = MATERIAL_FINISH_VALUES.len().saturating_sub(1) as i32;
                        self.material_finish = (*index).max(0).min(max);
                        self.brush_material_finishes[self.selected_brush] = self.material_finish;
                    }
                    _ => {}
                }
                self.sync_inspector(ui, ctx, project);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            _ => {
                if self.nodeui.handle_event(event) {
                    let mut refresh_inspector = false;
                    if let TheEvent::ValueChanged(id, value) = event {
                        match self.base_ui_name(&id.name) {
                            ISO_PAINT_TOOL_SIZE => {
                                if let Some(value) = value.to_f32() {
                                    self.size = self.clamp_size_for_selection(value);
                                    self.brush_sizes[self.selected_brush] = self.size;
                                }
                            }
                            ISO_PAINT_TOOL_OPACITY => {
                                if let Some(value) = value.to_f32() {
                                    self.opacity = value.clamp(0.0, 1.0);
                                    self.brush_opacities[self.selected_brush] = self.opacity;
                                }
                            }
                            ISO_PAINT_MATERIAL_MODE => {
                                if let TheValue::Int(index) = value {
                                    self.material_mode = match *index {
                                        1 => IsoPaintMaterialMode::Replace,
                                        2 if Self::brush_supports_stamp(
                                            self.selected_preset().key,
                                        ) =>
                                        {
                                            IsoPaintMaterialMode::Stamp
                                        }
                                        _ => IsoPaintMaterialMode::Coat,
                                    };
                                    self.size = self.clamp_size_for_selection(self.size);
                                    self.brush_sizes[self.selected_brush] = self.size;
                                    refresh_inspector = true;
                                }
                            }
                            ISO_PAINT_FLOWER_TYPE => {
                                if let TheValue::Int(index) = value {
                                    self.flower_type = (*index).clamp(0, 3);
                                    refresh_inspector = true;
                                }
                            }
                            name if Self::brush_color_slot_from_id(name).is_some() => {
                                if let TheValue::Int(index) = value {
                                    let brush = self.selected_preset();
                                    let color_count = Self::brush_color_slot_count(brush.key);
                                    let slot = Self::brush_color_slot_from_id(name)
                                        .unwrap_or(0)
                                        .min(color_count.saturating_sub(1));
                                    self.selected_color_slot = slot;
                                    if slot < self.brush_color_slots[self.selected_brush].len() {
                                        let index = (*index).clamp(0, 255) as u16;
                                        if slot == 0 && color_count > 1 {
                                            self.repick_related_brush_colors(
                                                project, brush.key, index,
                                            );
                                        } else {
                                            self.brush_color_slots[self.selected_brush][slot] =
                                                index;
                                        }
                                        refresh_inspector = true;
                                    }
                                }
                            }
                            ISO_PAINT_PATTERN_KIND => {
                                if let TheValue::Int(index) = value {
                                    self.pattern_kind = Self::pattern_kind_from_index(*index);
                                }
                            }
                            ISO_PAINT_PATTERN_SCALE => {
                                if let Some(value) = value.to_f32() {
                                    self.pattern_scale = value.clamp(0.25, 4.0);
                                }
                            }
                            ISO_PAINT_MORTAR => {
                                if let Some(value) = value.to_f32() {
                                    self.pattern_mortar = value.clamp(0.0, 0.4);
                                }
                            }
                            ISO_PAINT_PATTERN_DETAIL => {
                                if let Some(value) = value.to_f32() {
                                    self.pattern_detail = value.clamp(0.0, 1.0);
                                }
                            }
                            ISO_PAINT_PATTERN_VARIATION => {
                                if let Some(value) = value.to_f32() {
                                    self.pattern_variation = value.clamp(0.0, 1.0);
                                }
                            }
                            ISO_PAINT_STAMP_DENSITY => {
                                if let Some(value) = value.to_f32() {
                                    self.stamp_density = value.clamp(0.0, 1.0);
                                }
                            }
                            ISO_PAINT_STAMP_SIZE_JITTER => {
                                if let Some(value) = value.to_f32() {
                                    self.stamp_size_jitter = value.clamp(0.0, 1.0);
                                }
                            }
                            ISO_PAINT_STAMP_ROTATION_JITTER => {
                                if let Some(value) = value.to_f32() {
                                    self.stamp_rotation_jitter = value.clamp(0.0, 1.0);
                                }
                            }
                            _ => {}
                        }
                        self.sync_project_settings(project, server_ctx);
                        if refresh_inspector {
                            self.sync_inspector(ui, ctx, project);
                            ctx.ui.redraw_all = true;
                        }
                    }
                    if !matches!(event, TheEvent::ValueChanged(_, _)) {
                        self.apply_inspector_layouts(ui, ctx);
                    }
                    return true;
                }
            }
        }
        false
    }
}
