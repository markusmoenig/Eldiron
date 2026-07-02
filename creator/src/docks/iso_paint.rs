use crate::editor::UNDOMANAGER;
use crate::iso_paint_brush::{self, IsoPaintBrushSample};
use crate::prelude::*;
use rusterix::material_library::{
    MATERIAL_FINISH_NAMES as MATERIAL_FINISH_VALUES,
    MATERIAL_PRESET_NAMES as MATERIAL_PRESET_VALUES, MaterialDefinition,
};

const ISO_PAINT_PRESET_STRIP: &str = "Iso Paint Preset Strip";
const ISO_PAINT_MATERIAL_STRIP: &str = "Iso Paint Material Strip";
const ISO_PAINT_INSPECTOR_PRIMARY: &str = "Iso Paint Inspector Primary";
const ISO_PAINT_INSPECTOR_DETAIL: &str = "Iso Paint Inspector Detail";
const ISO_PAINT_BRUSH_EDITOR: &str = "Iso Paint Brush Editor";
const ISO_PAINT_BRUSH_SHAPE_GROUP: &str = "Iso Paint Brush Shape Group";
const ISO_PAINT_BRUSH_SELECTED: &str = "Iso Paint Brush Selected";
const ISO_PAINT_MATERIAL_PRESET_SELECTED: &str = "Iso Paint Material Preset Selected";
const ISO_PAINT_MATERIAL_FINISH_SELECTED: &str = "Iso Paint Material Finish Selected";
const ISO_PAINT_MATERIAL_MODE: &str = "Iso Paint Material Mode";
const ISO_PAINT_OPERATION_GROUP: &str = "Iso Paint Operation Group";
const ISO_PAINT_LAYER_VISIBLE: &str = "Iso Paint Layer Visible";
const ISO_PAINT_CLEAR_ALL: &str = "Iso Paint Clear All";
const ISO_PAINT_CLIP_GROUP: &str = "Iso Paint Clip Group";
const ISO_PAINT_TOOL_SIZE: &str = "Iso Paint Tool Size";
const ISO_PAINT_TOOL_OPACITY: &str = "Iso Paint Tool Opacity";
const ISO_PAINT_PATTERN_KIND: &str = "Iso Paint Pattern Kind";
const ISO_PAINT_PATTERN_SCALE: &str = "Iso Paint Pattern Scale";
const ISO_PAINT_MORTAR: &str = "Iso Paint Mortar";
const ISO_PAINT_PATTERN_DETAIL: &str = "Iso Paint Pattern Detail";
const ISO_PAINT_PATTERN_VARIATION: &str = "Iso Paint Pattern Variation";
const ISO_PAINT_STAMP_DENSITY: &str = "Iso Paint Stamp Density";
const ISO_PAINT_ACTIVE_BRUSH_COLOR: &str = "Iso Paint Active Brush Color";

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintOperation {
    Draw,
    Erase,
    Pick,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintClipMode {
    None,
    Object,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintPatternKind {
    Tiles,
    Bricks,
}

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintMaterialMode {
    Coat,
    Replace,
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
    ) {
        let (x, y, w, h) = *rect;
        if key == "brick" {
            Self::draw_brick_preview(buffer, ctx, rect, stride, palette);
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
                Self::draw_engine_preview(buffer, ctx, &preview, stride, brush.key, palette);
                if self.selected == index {
                    ctx.draw
                        .rect_outline_border(buffer.pixels_mut(), &outer, stride, &WHITE, 1);
                }

                let text_color = if self.selected == index {
                    [252, 252, 252, 255]
                } else {
                    [226, 226, 226, 255]
                };
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
                    &text_color,
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

        const PAD_X: i32 = 7;
        const PAD_Y: i32 = 4;
        const GAP: i32 = 5;
        let count = IsoPaintDock::BRUSHES.len().max(1) as i32;
        let aw = (self.dim.width - PAD_X * 2).max(1);
        let tile_w = ((aw - (count - 1) * GAP) / count).clamp(46, 78);
        let tile_h = (self.dim.height - PAD_Y * 2).clamp(30, 38);
        self.rectangles.clear();

        for index in 0..IsoPaintDock::BRUSHES.len() {
            let x = PAD_X + index as i32 * (tile_w + GAP);
            let y = PAD_Y;
            if x + tile_w > self.dim.width - PAD_X + 1 {
                break;
            }

            let brush = IsoPaintDock::BRUSHES[index];
            let local_rect = TheDim::new(x, y, tile_w, tile_h);
            let outer = (
                utuple.0 + x as usize,
                utuple.1 + y as usize,
                tile_w as usize,
                tile_h as usize,
            );
            let preview = (
                outer.0 + 2,
                outer.1 + 2,
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
            let palette = self
                .preview_palettes
                .get(index)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            IsoPaintBrushBoard::draw_engine_preview(
                buffer, ctx, &preview, stride, brush.key, palette,
            );
            if self.selected == index {
                ctx.draw
                    .rect_outline_border(buffer.pixels_mut(), &outer, stride, &WHITE, 1);
            } else if self.hovered == Some(index) {
                ctx.draw
                    .rect_outline_border(buffer.pixels_mut(), &outer, stride, bg, 1);
            }
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

    fn set_selected(&mut self, selected: usize) {
        self.selected = selected.min(IsoPaintDock::brush_shape_values().len().saturating_sub(1));
        self.is_dirty = true;
    }

    fn draw_shape_icon(
        buffer: &mut TheRGBABuffer,
        _ctx: &mut TheContext,
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
                let alpha = color[3] as u32;
                let inv_alpha = 255 - alpha;
                pixels[index] =
                    ((color[0] as u32 * alpha + pixels[index] as u32 * inv_alpha) / 255) as u8;
                pixels[index + 1] =
                    ((color[1] as u32 * alpha + pixels[index + 1] as u32 * inv_alpha) / 255) as u8;
                pixels[index + 2] =
                    ((color[2] as u32 * alpha + pixels[index + 2] as u32 * inv_alpha) / 255) as u8;
                pixels[index + 3] = 255;
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
                &[96, 96, 96, 255]
            } else {
                &[56, 56, 56, 255]
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
                ctx,
                &icon,
                stride,
                IsoPaintDock::brush_shape_key_from_index(index),
            );
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &outer,
                stride,
                if self.selected == index {
                    &WHITE
                } else {
                    &[36, 36, 36, 255]
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
    preview_cache: Option<(String, Vec<[u8; 4]>, i32, i32, TheRGBABuffer)>,
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
            |(cached_key, cached_palette, cached_w, cached_h, _)| {
                cached_key == brush_key
                    && cached_palette == &self.preview_palette
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
            );
            self.preview_cache = Some((
                brush_key.to_string(),
                self.preview_palette.clone(),
                rect.2 as i32,
                rect.3 as i32,
                preview,
            ));
        }
        if let Some((_, _, _, _, preview)) = &self.preview_cache {
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
        if self.dim.width < 96 || self.dim.height < 44 {
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
            &[38, 38, 38, 255],
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

    fn neutral_chip_color(selected: bool, hovered: bool) -> [u8; 4] {
        if selected {
            [122, 122, 122, 255]
        } else if hovered {
            [96, 96, 96, 255]
        } else {
            [74, 74, 74, 255]
        }
    }

    fn chip_text_color(selected: bool) -> [u8; 4] {
        if selected {
            [250, 250, 250, 255]
        } else {
            [218, 218, 218, 255]
        }
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
            let color = Self::neutral_chip_color(selected, hovered);
            ctx.draw.rect(buffer.pixels_mut(), &global, stride, &color);
            let border = if selected {
                WHITE
            } else if hovered {
                [210, 210, 210, 255]
            } else {
                [38, 38, 38, 255]
            };
            ctx.draw
                .rect_outline_border(buffer.pixels_mut(), &global, stride, &border, 1);
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(global.0 + 1, global.1, global.2.saturating_sub(2), global.3),
                stride,
                &material_labels[index],
                TheFontSettings {
                    size: 9.5,
                    ..Default::default()
                },
                &Self::chip_text_color(selected),
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
            let color = Self::neutral_chip_color(selected, hovered);
            ctx.draw.rect(buffer.pixels_mut(), &global, stride, &color);
            let border = if selected {
                WHITE
            } else if hovered {
                [210, 210, 210, 255]
            } else {
                [38, 38, 38, 255]
            };
            ctx.draw
                .rect_outline_border(buffer.pixels_mut(), &global, stride, &border, 1);
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(global.0 + 2, global.1, global.2.saturating_sub(4), global.3),
                stride,
                &finish_labels[index],
                TheFontSettings {
                    size: 10.0,
                    ..Default::default()
                },
                &Self::chip_text_color(selected),
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
    selected_brush: usize,
    operation: IsoPaintOperation,
    size: f32,
    opacity: f32,
    brush_shapes: [IsoPaintBrushShape; 7],
    brush_sizes: [f32; 7],
    brush_opacities: [f32; 7],
    brush_color_slots: [[u16; 4]; 7],
    selected_color_slot: usize,
    brush_material_presets: [i32; 7],
    brush_material_finishes: [i32; 7],
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
    nodeui: TheNodeUI,
}

impl IsoPaintDock {
    const BRUSHES: [IsoPaintBrushPreset; 7] = [
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
            key: "crack",
            size: 0.6,
            opacity: 1.0,
            shape: IsoPaintBrushShape::Scratch,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
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
            key: "puddle",
            size: 1.8,
            opacity: 0.68,
            shape: IsoPaintBrushShape::Wash,
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

    fn default_brush_sizes() -> [f32; 7] {
        std::array::from_fn(|index| Self::BRUSHES[index].size)
    }

    fn default_brush_opacities() -> [f32; 7] {
        std::array::from_fn(|index| Self::BRUSHES[index].opacity)
    }

    fn default_brush_shapes() -> [IsoPaintBrushShape; 7] {
        std::array::from_fn(|index| Self::BRUSHES[index].shape)
    }

    fn default_brush_color_slots() -> [[u16; 4]; 7] {
        [
            [0, 0, 0, 0],
            [19, 20, 21, 18],
            [37, 36, 34, 38],
            [1, 2, 4, 7],
            [37, 36, 35, 34],
            [39, 41, 43, 45],
            [18, 17, 27, 16],
        ]
    }

    fn brush_color_slot_count(key: &str) -> usize {
        match key {
            "material" => 1,
            "puddle" => 0,
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

    fn default_brush_material_presets() -> [i32; 7] {
        std::array::from_fn(|index| {
            let key = match Self::BRUSHES[index].key {
                "brick" | "crack" => "stone",
                "moss" | "grass" => "foliage",
                "dirt" => "dirt",
                "puddle" => "water",
                _ => "default",
            };
            Self::material_index_from_key(key)
        })
    }

    fn default_brush_material_finishes() -> [i32; 7] {
        std::array::from_fn(|index| {
            let key = match Self::BRUSHES[index].key {
                "puddle" => "wet",
                "dirt" => "matte",
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
        }
    }

    fn operation_index(operation: IsoPaintOperation) -> i32 {
        match operation {
            IsoPaintOperation::Draw => 0,
            IsoPaintOperation::Erase => 1,
            IsoPaintOperation::Pick => 2,
        }
    }

    fn operation_from_index(index: usize) -> IsoPaintOperation {
        match index {
            1 => IsoPaintOperation::Erase,
            2 => IsoPaintOperation::Pick,
            _ => IsoPaintOperation::Draw,
        }
    }

    fn operation_label(operation: IsoPaintOperation) -> String {
        match operation {
            IsoPaintOperation::Draw => fl!("iso_paint_operation_draw"),
            IsoPaintOperation::Erase => fl!("iso_paint_operation_erase"),
            IsoPaintOperation::Pick => fl!("iso_paint_operation_pick"),
        }
    }

    fn operation_key(operation: IsoPaintOperation) -> &'static str {
        match operation {
            IsoPaintOperation::Draw => "draw",
            IsoPaintOperation::Erase => "erase",
            IsoPaintOperation::Pick => "pick",
        }
    }

    fn clip_key(clip: IsoPaintClipMode) -> &'static str {
        match clip {
            IsoPaintClipMode::None => "none",
            IsoPaintClipMode::Object => "object",
        }
    }

    fn clip_index(clip: IsoPaintClipMode) -> i32 {
        match clip {
            IsoPaintClipMode::None => 0,
            IsoPaintClipMode::Object => 1,
        }
    }

    fn clip_from_index(index: usize) -> IsoPaintClipMode {
        match index {
            0 => IsoPaintClipMode::None,
            _ => IsoPaintClipMode::Object,
        }
    }

    fn clip_label(clip: IsoPaintClipMode) -> String {
        match clip {
            IsoPaintClipMode::None => fl!("iso_paint_clip_none"),
            IsoPaintClipMode::Object => fl!("iso_paint_clip_object"),
        }
    }

    fn clip_from_key(key: &str) -> IsoPaintClipMode {
        match key {
            "none" => IsoPaintClipMode::None,
            _ => IsoPaintClipMode::Object,
        }
    }

    fn pattern_kind_key(pattern_kind: IsoPaintPatternKind) -> &'static str {
        match pattern_kind {
            IsoPaintPatternKind::Tiles => "tile",
            IsoPaintPatternKind::Bricks => "brick",
        }
    }

    fn pattern_kind_from_key(key: &str) -> IsoPaintPatternKind {
        match key {
            "tile" | "tiles" => IsoPaintPatternKind::Tiles,
            _ => IsoPaintPatternKind::Bricks,
        }
    }

    fn material_mode_key(material_mode: IsoPaintMaterialMode) -> &'static str {
        match material_mode {
            IsoPaintMaterialMode::Coat => "coat",
            IsoPaintMaterialMode::Replace => "replace",
        }
    }

    fn material_mode_from_key(key: &str) -> IsoPaintMaterialMode {
        match key {
            "replace" => IsoPaintMaterialMode::Replace,
            _ => IsoPaintMaterialMode::Coat,
        }
    }

    fn material_mode_labels() -> Vec<String> {
        vec![
            fl!("iso_paint_material_mode_coat"),
            fl!("iso_paint_material_mode_replace"),
        ]
    }

    fn pattern_kind_labels() -> Vec<String> {
        vec![
            fl!("iso_paint_pattern_tiles"),
            fl!("iso_paint_pattern_bricks"),
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

    fn brush_palette_tone(brush: &str, slot: usize) -> f32 {
        let tones = match brush {
            "dirt" => [1.0, 0.72, 0.50, 0.34],
            "brick" => [1.0, 0.72, 1.22, 0.42],
            "moss" | "grass" => [1.0, 0.76, 1.18, 0.48],
            "crack" => [1.0, 0.58, 1.32, 0.40],
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
            "grass" => fl!("iso_paint_brush_grass"),
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
            "grass" => fl!("iso_paint_brush_grass_desc"),
            "puddle" => fl!("iso_paint_brush_puddle_desc"),
            "dirt" => "Paint dirt, dust, and grime onto the surface.".to_string(),
            _ => String::new(),
        }
    }

    fn build_nodeui(&self, project: &Project) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_brush")));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ISO_PAINT_TOOL_SIZE.into(),
            fl!("iso_paint_size"),
            fl!("status_iso_paint_size"),
            self.size,
            0.05..=8.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ISO_PAINT_TOOL_OPACITY.into(),
            fl!("iso_paint_opacity"),
            fl!("status_iso_paint_opacity"),
            self.opacity,
            0.0..=1.0,
            true,
        ));

        let brush = self.selected_preset();
        let color_count = Self::brush_color_slot_count(brush.key);
        if color_count > 0 {
            nodeui.add_item(TheNodeUIItem::PaletteIndexRowPicker(
                ISO_PAINT_ACTIVE_BRUSH_COLOR.into(),
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
                ISO_PAINT_PATTERN_KIND.into(),
                fl!("iso_paint_pattern_kind"),
                fl!("status_iso_paint_pattern_kind"),
                Self::pattern_kind_labels(),
                if self.pattern_kind == IsoPaintPatternKind::Tiles {
                    0
                } else {
                    1
                },
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                ISO_PAINT_PATTERN_SCALE.into(),
                fl!("iso_paint_pattern_scale"),
                fl!("status_iso_paint_pattern_scale"),
                self.pattern_scale,
                0.25..=4.0,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                ISO_PAINT_MORTAR.into(),
                fl!("iso_paint_mortar"),
                fl!("status_iso_paint_mortar"),
                self.pattern_mortar,
                0.0..=0.4,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                ISO_PAINT_PATTERN_DETAIL.into(),
                fl!("iso_paint_pattern_detail"),
                fl!("status_iso_paint_pattern_detail"),
                self.pattern_detail,
                0.0..=1.0,
                true,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                ISO_PAINT_PATTERN_VARIATION.into(),
                fl!("iso_paint_pattern_variation"),
                fl!("status_iso_paint_pattern_variation"),
                self.pattern_variation,
                0.0..=1.0,
                true,
            ));
        }
        if brush.key != "puddle" {
            nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_material")));
            nodeui.add_item(TheNodeUIItem::Selector(
                ISO_PAINT_MATERIAL_MODE.into(),
                fl!("iso_paint_material_mode"),
                fl!("status_iso_paint_material_mode"),
                Self::material_mode_labels(),
                if self.material_mode == IsoPaintMaterialMode::Replace {
                    1
                } else {
                    0
                },
            ));
        }

        nodeui
    }

    fn inspector_item_column(item: &TheNodeUIItem) -> usize {
        match item {
            TheNodeUIItem::Separator(name) if name == &fl!("iso_paint_section_brush") => 0,
            TheNodeUIItem::FloatEditSlider(id, _, _, _, _, _)
                if matches!(id.as_str(), ISO_PAINT_TOOL_SIZE | ISO_PAINT_TOOL_OPACITY) =>
            {
                0
            }
            TheNodeUIItem::PaletteIndexRowPicker(id, _, _, _, _)
                if id == ISO_PAINT_ACTIVE_BRUSH_COLOR =>
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
            if Self::inspector_item_column(item) == 0 {
                primary.add_item(item.clone());
            } else {
                detail.add_item(item.clone());
            }
        }
        (primary, detail)
    }

    fn apply_inspector_layouts(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        let (primary, detail) = self.split_inspector_nodeui();
        if let Some(layout) = ui.get_text_layout(ISO_PAINT_INSPECTOR_PRIMARY) {
            primary.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
        if let Some(layout) = ui.get_text_layout(ISO_PAINT_INSPECTOR_DETAIL) {
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

    fn brush_index_from_key(key: &str) -> usize {
        let key = if key == "screen" { "dirt" } else { key };
        Self::BRUSHES
            .iter()
            .position(|brush| brush.key == key)
            .unwrap_or(0)
    }

    fn sync_inspector(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        self.nodeui = self.build_nodeui(project);
        self.sync_toolbar(ui, ctx);
        if let Some(widget) = ui.get_widget(ISO_PAINT_PRESET_STRIP)
            && let Some(strip) = widget.as_any().downcast_mut::<IsoPaintPresetStrip>()
        {
            strip.set_selected(self.selected_brush);
            strip.set_preview_palettes(self.all_brush_palette_colors(project));
        }
        if let Some(widget) = ui.get_widget(ISO_PAINT_MATERIAL_STRIP)
            && let Some(strip) = widget.as_any().downcast_mut::<IsoPaintMaterialStrip>()
        {
            strip.set_material(self.material_preset, self.material_finish);
        }
        if let Some(widget) = ui.get_widget(ISO_PAINT_BRUSH_EDITOR)
            && let Some(editor) = widget.as_any().downcast_mut::<IsoPaintBrushEditor>()
        {
            editor.set_selected_brush(self.selected_brush);
            editor.set_preview_palette(self.current_palette_colors(project));
        }
        if let Some(widget) = ui.get_widget(ISO_PAINT_BRUSH_SHAPE_GROUP)
            && let Some(strip) = widget.as_any().downcast_mut::<IsoPaintBrushShapeStrip>()
        {
            strip.set_selected(Self::brush_shape_index(self.brush_shape).max(0) as usize);
        }
        self.apply_inspector_layouts(ui, ctx);
    }

    fn sync_toolbar(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        ui.set_widget_value(
            ISO_PAINT_OPERATION_GROUP,
            ctx,
            TheValue::Int(Self::operation_index(self.operation)),
        );
        ctx.ui.set_widget_state(
            ISO_PAINT_LAYER_VISIBLE.to_string(),
            if self.paint_visible {
                TheWidgetState::Selected
            } else {
                TheWidgetState::None
            },
        );
        ui.set_widget_value(
            ISO_PAINT_CLIP_GROUP,
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
        let color = if brush.key == "puddle" {
            iso_paint_brush::default_preview_color(brush.key)
        } else {
            palette_colors
                .first()
                .copied()
                .unwrap_or_else(|| Self::selected_palette_color(project))
        };
        let Some(region) = project.get_region_mut(&server_ctx.curr_region) else {
            return;
        };
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
        let material_id = MaterialDefinition::from_preset_finish(material_key, finish_key).id();
        region.iso_paint.set_active_settings(
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
            clip_mode: IsoPaintClipMode::Object,
            pattern_kind: IsoPaintPatternKind::Bricks,
            pattern_scale: Self::BRUSHES[0].pattern_scale,
            pattern_mortar: Self::BRUSHES[0].mortar,
            pattern_detail: 0.65,
            pattern_variation: 0.6,
            stamp_density: Self::BRUSHES[0].density,
            nodeui: TheNodeUI::default(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar = TheHLayout::new(TheId::named("Iso Paint Toolbar"));
        toolbar.set_background_color(None);
        toolbar.set_margin(Vec4::new(10, 2, 6, 2));
        toolbar.set_padding(7);

        let mut operation_group = TheGroupButton::new(TheId::named(ISO_PAINT_OPERATION_GROUP));
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
        operation_group.set_item_width(74);
        operation_group.set_index(Self::operation_index(self.operation));
        toolbar.add_widget(Box::new(operation_group));

        let mut layer_visible = TheTraybarButton::new(TheId::named(ISO_PAINT_LAYER_VISIBLE));
        layer_visible.set_text(fl!("iso_paint_layer_visible"));
        layer_visible.set_status_text(&fl!("status_iso_paint_layer_visible"));
        layer_visible.set_fixed_size(false);
        layer_visible.limiter_mut().set_min_width(72);
        layer_visible.limiter_mut().set_max_width(72);
        if self.paint_visible {
            layer_visible.set_state(TheWidgetState::Selected);
        }
        toolbar.add_widget(Box::new(layer_visible));

        let mut clear_all = TheTraybarButton::new(TheId::named(ISO_PAINT_CLEAR_ALL));
        clear_all.set_text(fl!("iso_paint_clear_all"));
        clear_all.set_status_text(&fl!("status_iso_paint_clear_all"));
        clear_all.set_fixed_size(false);
        clear_all.limiter_mut().set_min_width(82);
        clear_all.limiter_mut().set_max_width(82);
        toolbar.add_widget(Box::new(clear_all));

        let mut clip_group = TheGroupButton::new(TheId::named(ISO_PAINT_CLIP_GROUP));
        clip_group.add_text_status(
            Self::clip_label(IsoPaintClipMode::None),
            fl!("status_iso_paint_clip"),
        );
        clip_group.add_text_status(
            Self::clip_label(IsoPaintClipMode::Object),
            fl!("status_iso_paint_clip"),
        );
        clip_group.set_item_width(76);
        clip_group.set_index(Self::clip_index(self.clip_mode));
        toolbar.add_widget(Box::new(clip_group));

        toolbar.set_reverse_index(Some(3));

        toolbar_canvas.set_layout(toolbar);
        canvas.set_top(toolbar_canvas);

        let mut content = TheCanvas::new();

        let mut preset_canvas = TheCanvas::new();
        preset_canvas.limiter_mut().set_min_height(46);
        preset_canvas.limiter_mut().set_max_height(46);
        let mut preset_strip = IsoPaintPresetStrip::new(TheId::named(ISO_PAINT_PRESET_STRIP));
        preset_strip.set_selected(self.selected_brush);
        preset_canvas.set_widget(preset_strip);
        content.set_top(preset_canvas);

        let mut material_canvas = TheCanvas::new();
        material_canvas.limiter_mut().set_min_height(58);
        material_canvas.limiter_mut().set_max_height(58);
        let mut material_strip = IsoPaintMaterialStrip::new(TheId::named(ISO_PAINT_MATERIAL_STRIP));
        material_strip.set_material(self.material_preset, self.material_finish);
        material_canvas.set_widget(material_strip);
        content.set_bottom(material_canvas);

        let mut center = TheCanvas::new();

        let mut brush_panel = TheCanvas::new();

        let mut brush_editor_canvas = TheCanvas::new();
        let mut brush_editor = IsoPaintBrushEditor::new(TheId::named(ISO_PAINT_BRUSH_EDITOR));
        brush_editor.set_selected_brush(self.selected_brush);
        brush_editor_canvas.set_widget(brush_editor);
        brush_panel.set_center(brush_editor_canvas);

        let mut shape_canvas = TheCanvas::new();
        shape_canvas.limiter_mut().set_min_height(32);
        shape_canvas.limiter_mut().set_max_height(32);
        let mut shape_strip =
            IsoPaintBrushShapeStrip::new(TheId::named(ISO_PAINT_BRUSH_SHAPE_GROUP));
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

        let mut primary_inspector = TheTextLayout::new(TheId::named(ISO_PAINT_INSPECTOR_PRIMARY));
        primary_inspector.set_text_margin(8);
        primary_inspector.set_fixed_text_width(58);
        primary_inspector.set_text_align(TheHorizontalAlign::Right);
        primary_inspector.limiter_mut().set_min_width(200);
        primary_inspector.limiter_mut().set_max_width(200);
        primary_nodeui.apply_to_text_layout(&mut primary_inspector);

        let mut detail_inspector = TheTextLayout::new(TheId::named(ISO_PAINT_INSPECTOR_DETAIL));
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
        self.paint_visible = project
            .get_region(&server_ctx.curr_region)
            .map(|region| {
                self.clip_mode = Self::clip_from_key(&region.iso_paint.active_clip);
                self.material_mode =
                    Self::material_mode_from_key(&region.iso_paint.active_material_mode);
                self.selected_brush = Self::brush_index_from_key(&region.iso_paint.active_brush);
                let preset = self.selected_preset();
                self.size = if matches!(preset.key, "material" | "brick")
                    && region.iso_paint.active_size <= 1.001
                {
                    preset.size
                } else {
                    region.iso_paint.active_size.max(0.05)
                };
                self.opacity = region.iso_paint.active_opacity.clamp(0.0, 1.0);
                self.brush_sizes[self.selected_brush] = self.size;
                self.brush_opacities[self.selected_brush] = self.opacity;
                for (slot, index) in region.iso_paint.active_palette_indices.iter().enumerate() {
                    if slot < self.brush_color_slots[self.selected_brush].len() {
                        self.brush_color_slots[self.selected_brush][slot] = *index;
                    }
                }
                self.brush_shape = Self::brush_shape_from_key(&region.iso_paint.active_brush_shape);
                self.brush_shapes[self.selected_brush] = self.brush_shape;
                self.material_preset =
                    Self::material_index_from_key(&region.iso_paint.active_material);
                self.material_finish = Self::finish_index_from_key(&region.iso_paint.active_finish);
                self.selected_color_slot = self
                    .selected_color_slot
                    .min(Self::brush_color_slot_count(preset.key).saturating_sub(1));
                self.brush_material_presets[self.selected_brush] = self.material_preset;
                self.brush_material_finishes[self.selected_brush] = self.material_finish;
                self.enforce_special_brush_settings();
                self.pattern_kind =
                    Self::pattern_kind_from_key(&region.iso_paint.active_pattern_kind);
                self.pattern_scale = region.iso_paint.active_pattern_scale;
                self.pattern_mortar = region.iso_paint.active_pattern_mortar;
                self.pattern_detail = region.iso_paint.active_pattern_detail;
                self.pattern_variation = region.iso_paint.active_pattern_variation;
                region.iso_paint.visible
            })
            .unwrap_or(true);
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
            TheEvent::IndexChanged(id, index) if id.name == ISO_PAINT_OPERATION_GROUP => {
                self.set_operation(Self::operation_from_index(*index), ui, ctx);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == ISO_PAINT_LAYER_VISIBLE =>
            {
                self.paint_visible = !self.paint_visible;
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.iso_paint.visible = self.paint_visible;
                }
                ctx.ui.set_widget_state(
                    ISO_PAINT_LAYER_VISIBLE.to_string(),
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
                if id.name == ISO_PAINT_CLEAR_ALL =>
            {
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region)
                    && !region.iso_paint.chunks.is_empty()
                {
                    let old_region = region.clone();
                    region.iso_paint.chunks.clear();
                    let undo_atom = ProjectUndoAtom::RegionEdit(
                        ProjectContext::Region(region.id),
                        Box::new(old_region),
                        Box::new(region.clone()),
                    );
                    UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                    ctx.ui.redraw_all = true;
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_iso_paint_clear_all_done"),
                    ));
                }
                return true;
            }
            TheEvent::IndexChanged(id, index) if id.name == ISO_PAINT_CLIP_GROUP => {
                self.set_clip_mode(Self::clip_from_index(*index), ui, ctx);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            TheEvent::IndexChanged(id, index) if id.name == ISO_PAINT_BRUSH_SHAPE_GROUP => {
                self.brush_shape = Self::brush_shape_from_index(*index);
                self.brush_shapes[self.selected_brush] = self.brush_shape;
                self.sync_project_settings(project, server_ctx);
                ctx.ui.redraw_all = true;
                return true;
            }
            TheEvent::Custom(id, TheValue::Int(index)) if id.name == ISO_PAINT_BRUSH_SELECTED => {
                self.select_brush((*index).max(0) as usize, ui, ctx, project);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            TheEvent::PaletteIndexChanged(id, index)
                if Self::brush_color_slot_from_id(&id.name).is_some() =>
            {
                let brush = self.selected_preset();
                let color_count = Self::brush_color_slot_count(brush.key);
                let slot = Self::brush_color_slot_from_id(&id.name)
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
                    id.name.as_str(),
                    ISO_PAINT_MATERIAL_PRESET_SELECTED | ISO_PAINT_MATERIAL_FINISH_SELECTED
                ) =>
            {
                match id.name.as_str() {
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
                        match id.name.as_str() {
                            ISO_PAINT_TOOL_SIZE => {
                                if let Some(value) = value.to_f32() {
                                    self.size = value.clamp(0.05, 8.0);
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
                                    self.material_mode = if *index == 1 {
                                        IsoPaintMaterialMode::Replace
                                    } else {
                                        IsoPaintMaterialMode::Coat
                                    };
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
                                    self.pattern_kind = if *index == 0 {
                                        IsoPaintPatternKind::Tiles
                                    } else {
                                        IsoPaintPatternKind::Bricks
                                    };
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
