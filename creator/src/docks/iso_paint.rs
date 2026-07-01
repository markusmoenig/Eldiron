use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use rusterix::material_library::{
    MATERIAL_FINISH_NAMES as MATERIAL_FINISH_VALUES,
    MATERIAL_PRESET_NAMES as MATERIAL_PRESET_VALUES, MaterialDefinition,
};

const ISO_PAINT_BRUSH_LIST: &str = "Iso Paint Brush List";
const ISO_PAINT_MATERIAL_STRIP: &str = "Iso Paint Material Strip";
const ISO_PAINT_INSPECTOR: &str = "Iso Paint Inspector";
const ISO_PAINT_BRUSH_SELECTED: &str = "Iso Paint Brush Selected";
const ISO_PAINT_MATERIAL_PRESET_SELECTED: &str = "Iso Paint Material Preset Selected";
const ISO_PAINT_MATERIAL_FINISH_SELECTED: &str = "Iso Paint Material Finish Selected";
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

#[derive(Clone, Copy)]
struct IsoPaintBrushPreset {
    key: &'static str,
    size: f32,
    opacity: f32,
    pattern_scale: f32,
    mortar: f32,
    density: f32,
}

struct IsoPaintBrushBoard {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    selected: usize,
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
            hovered: None,
            rectangles: Vec::new(),
            is_dirty: true,
        }
    }

    fn set_selected(&mut self, selected: usize) {
        self.selected = selected.min(IsoPaintDock::BRUSHES.len().saturating_sub(1));
        self.is_dirty = true;
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

    fn draw_preview(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: &(usize, usize, usize, usize),
        stride: usize,
        key: &str,
    ) {
        let (x, y, w, h) = *rect;
        let base = match key {
            "brick" => [143, 120, 75, 255],
            "moss" => [77, 104, 54, 255],
            "crack" => [102, 102, 99, 255],
            "grass" => [62, 120, 64, 255],
            "puddle" => [58, 77, 86, 255],
            "screen" => [89, 111, 132, 255],
            _ => [128, 110, 83, 255],
        };
        let dark = [28, 31, 30, 255];
        let light = [202, 190, 142, 255];
        let accent = [101, 151, 79, 255];

        let fill = (
            x + 1,
            y + 1,
            w.saturating_sub(2).max(1),
            h.saturating_sub(2).max(1),
        );
        ctx.draw.rect(buffer.pixels_mut(), &fill, stride, &base);

        match key {
            "brick" => {
                let brick_h = (h / 6).max(6);
                let brick_w = (w / 3).max(14);
                let mut yy = y;
                let mut row = 0usize;
                while yy < y + h {
                    buffer.draw_line(x as i32, yy as i32, (x + w) as i32, yy as i32, dark);
                    let next_y = (yy + brick_h).min(y + h);
                    buffer.draw_line(x as i32, next_y as i32, (x + w) as i32, next_y as i32, dark);
                    let offset = if row % 2 == 0 { 0 } else { brick_w / 2 };
                    let mut xx = x.saturating_sub(offset);
                    while xx < x + w {
                        if xx > x {
                            buffer.draw_line(xx as i32, yy as i32, xx as i32, next_y as i32, dark);
                        }
                        xx += brick_w;
                    }
                    yy += brick_h;
                    row += 1;
                }
            }
            "moss" => {
                for i in 0..18usize {
                    let px = x + 4 + (i * 17) % w.saturating_sub(8).max(1);
                    let py = y + 4 + (i * 29) % h.saturating_sub(8).max(1);
                    buffer.draw_line(
                        px as i32,
                        py as i32,
                        (px + 5) as i32,
                        (py + 2) as i32,
                        accent,
                    );
                    buffer.draw_line(
                        px as i32,
                        (py + 1) as i32,
                        (px + 2) as i32,
                        (py + 7) as i32,
                        [42, 72, 41, 255],
                    );
                }
            }
            "crack" => {
                let mut px = x + w / 5;
                let mut py = y + h / 5;
                for (dx, dy) in [(12, 8), (8, 14), (15, 6), (6, 16), (14, 9)] {
                    let nx = (px + dx).min(x + w - 3);
                    let ny = (py + dy).min(y + h - 3);
                    buffer.draw_line(px as i32, py as i32, nx as i32, ny as i32, dark);
                    buffer.draw_line(
                        px as i32,
                        py as i32 + 1,
                        nx as i32,
                        ny as i32 + 1,
                        [14, 16, 17, 255],
                    );
                    px = nx;
                    py = ny;
                }
            }
            "grass" => {
                for i in 0..16usize {
                    let px = x + 5 + (i * 13) % w.saturating_sub(10).max(1);
                    let base_y = y + h - 8 - (i * 7) % 14;
                    buffer.draw_line(
                        px as i32,
                        base_y as i32,
                        (px + 2) as i32,
                        (base_y - 12) as i32,
                        accent,
                    );
                    buffer.draw_line(
                        px as i32,
                        base_y as i32,
                        (px - 3) as i32,
                        (base_y - 8) as i32,
                        [37, 91, 45, 255],
                    );
                }
            }
            "puddle" => {
                let cx = x as i32 + w as i32 / 2;
                let cy = y as i32 + h as i32 / 2;
                let rx = (w as f32 * 0.40).max(6.0);
                let ry = (h as f32 * 0.30).max(5.0);
                let bounds = (x + 3, y + 3, w.saturating_sub(6), h.saturating_sub(6));
                let pixels = buffer.pixels_mut();
                for py in bounds.1..bounds.1 + bounds.3 {
                    for px in bounds.0..bounds.0 + bounds.2 {
                        let fx = (px as i32 - cx) as f32 / rx;
                        let fy = (py as i32 - cy) as f32 / ry;
                        let wobble = ((px as f32 * 0.17).sin() + (py as f32 * 0.23).cos()) * 0.10;
                        let d = fx * fx + fy * fy + wobble;
                        if d >= 1.0 {
                            continue;
                        }
                        let index = (py * stride + px) * 4;
                        if index + 3 >= pixels.len() {
                            continue;
                        }
                        let edge = (1.0 - d).clamp(0.0, 1.0);
                        let alpha = (42.0 + edge * 112.0) as u16;
                        let src = [
                            (68.0 + edge * 38.0) as u8,
                            (92.0 + edge * 46.0) as u8,
                            (104.0 + edge * 62.0) as u8,
                            alpha as u8,
                        ];
                        let keep = 255 - alpha;
                        pixels[index] =
                            ((src[0] as u16 * alpha + pixels[index] as u16 * keep) / 255) as u8;
                        pixels[index + 1] =
                            ((src[1] as u16 * alpha + pixels[index + 1] as u16 * keep) / 255) as u8;
                        pixels[index + 2] =
                            ((src[2] as u16 * alpha + pixels[index + 2] as u16 * keep) / 255) as u8;
                        pixels[index + 3] = pixels[index + 3].max(src[3]);
                    }
                }
                let shine_y = y as i32 + (h as i32 * 2) / 5;
                buffer.draw_line(
                    (x + w / 4) as i32,
                    shine_y,
                    (x + (w * 3) / 4) as i32,
                    shine_y - 4,
                    [215, 232, 238, 190],
                );
                buffer.draw_line(
                    (x + w / 3) as i32,
                    shine_y + 5,
                    (x + (w * 2) / 3) as i32,
                    shine_y + 2,
                    [186, 213, 222, 130],
                );
            }
            "screen" => {
                for i in 0..8usize {
                    let yy = y + 8 + i * h.saturating_sub(16).max(1) / 8;
                    buffer.draw_line(
                        (x + 8) as i32,
                        yy as i32,
                        (x + w - 8) as i32,
                        (yy + 4) as i32,
                        [180, 198, 212, 180],
                    );
                }
            }
            _ => {
                buffer.draw_line(
                    (x + 10) as i32,
                    (y + h - 14) as i32,
                    (x + w - 10) as i32,
                    (y + 12) as i32,
                    light,
                );
                buffer.draw_line(
                    (x + 12) as i32,
                    (y + h - 10) as i32,
                    (x + w - 12) as i32,
                    (y + 16) as i32,
                    dark,
                );
            }
        }

        buffer.draw_rect_outline(
            &TheDim::new(x as i32, y as i32, w as i32, h as i32),
            &[20, 20, 20, 255],
        );
        let _ = stride;
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
                self.draw_preview(buffer, ctx, &preview, stride, brush.key);
                if self.selected == index {
                    ctx.draw
                        .rect_outline_border(buffer.pixels_mut(), &outer, stride, &WHITE, 1);
                }

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
    material_preset: i32,
    material_finish: i32,
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
            size: 1.0,
            opacity: 1.0,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "brick",
            size: 1.0,
            opacity: 1.0,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "moss",
            size: 1.6,
            opacity: 0.75,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.7,
        },
        IsoPaintBrushPreset {
            key: "crack",
            size: 0.6,
            opacity: 1.0,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "grass",
            size: 1.2,
            opacity: 1.0,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "puddle",
            size: 1.8,
            opacity: 0.62,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "screen",
            size: 1.4,
            opacity: 0.7,
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

    fn brush_label(key: &str) -> String {
        match key {
            "material" => fl!("iso_paint_brush_material"),
            "brick" => fl!("iso_paint_brush_brick"),
            "moss" => fl!("iso_paint_brush_moss"),
            "crack" => fl!("iso_paint_brush_crack"),
            "grass" => fl!("iso_paint_brush_grass"),
            "puddle" => fl!("iso_paint_brush_puddle"),
            "screen" => fl!("iso_paint_brush_screen"),
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
            "screen" => fl!("iso_paint_brush_screen_desc"),
            _ => String::new(),
        }
    }

    fn build_nodeui(&self) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();
        let brush = self.selected_preset();

        match brush.key {
            "brick" => {
                nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_pattern")));
                nodeui.add_item(TheNodeUIItem::Selector(
                    ISO_PAINT_PATTERN_KIND.into(),
                    fl!("iso_paint_pattern_kind"),
                    fl!("status_iso_paint_pattern_kind"),
                    Self::pattern_kind_labels(),
                    if self.pattern_kind == IsoPaintPatternKind::Bricks {
                        1
                    } else {
                        0
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
            "moss" | "grass" => {
                nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_stamps")));
                nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                    ISO_PAINT_STAMP_DENSITY.into(),
                    fl!("iso_paint_stamp_density"),
                    fl!("status_iso_paint_stamp_density"),
                    self.stamp_density,
                    0.0..=1.0,
                    true,
                ));
            }
            _ => {}
        }

        nodeui
    }

    fn selected_preset(&self) -> IsoPaintBrushPreset {
        Self::BRUSHES
            .get(self.selected_brush)
            .copied()
            .unwrap_or(Self::BRUSHES[0])
    }

    fn sync_inspector(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.nodeui = self.build_nodeui();
        self.sync_toolbar(ui, ctx);
        ui.set_widget_value(
            ISO_PAINT_TOOL_SIZE,
            ctx,
            TheValue::Float((self.size * 100.0).round() / 100.0),
        );
        ui.set_widget_value(
            ISO_PAINT_TOOL_OPACITY,
            ctx,
            TheValue::Float((self.opacity * 100.0).round() / 100.0),
        );
        if let Some(widget) = ui.get_widget(ISO_PAINT_BRUSH_LIST)
            && let Some(board) = widget.as_any().downcast_mut::<IsoPaintBrushBoard>()
        {
            board.set_selected(self.selected_brush);
        }
        if let Some(widget) = ui.get_widget(ISO_PAINT_MATERIAL_STRIP)
            && let Some(strip) = widget.as_any().downcast_mut::<IsoPaintMaterialStrip>()
        {
            strip.set_material(self.material_preset, self.material_finish);
        }
        if let Some(layout) = ui.get_text_layout(ISO_PAINT_INSPECTOR) {
            self.nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
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

    fn select_brush(&mut self, index: usize, ui: &mut TheUI, ctx: &mut TheContext) {
        self.selected_brush = index.min(Self::BRUSHES.len().saturating_sub(1));
        let brush = self.selected_preset();
        if brush.key == "puddle" {
            if let Some(index) = MATERIAL_PRESET_VALUES
                .iter()
                .position(|value| *value == "water")
            {
                self.material_preset = index as i32;
            }
            if let Some(index) = MATERIAL_FINISH_VALUES
                .iter()
                .position(|value| *value == "wet")
            {
                self.material_finish = index as i32;
            }
        }
        self.sync_inspector(ui, ctx);
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
        let color = if brush.key == "puddle" {
            [86, 124, 142, 255]
        } else {
            Self::selected_palette_color(project)
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
        let material_id = MaterialDefinition::from_preset_finish(material_key, finish_key).id();
        region.iso_paint.set_active_settings(
            Self::operation_key(self.operation),
            brush.key,
            material_key,
            finish_key,
            material_id,
            Self::clip_key(self.clip_mode),
            color,
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
        Self {
            selected_brush: 0,
            operation: IsoPaintOperation::Draw,
            size: Self::BRUSHES[0].size,
            opacity: Self::BRUSHES[0].opacity,
            material_preset: 0,
            material_finish: 0,
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

        let mut size_label = TheText::new(TheId::named("Iso Paint Size Label"));
        size_label.set_text(fl!("iso_paint_size"));
        size_label.set_text_size(12.0);
        size_label.set_text_color([230, 230, 230, 255]);
        size_label.limiter_mut().set_min_width(28);
        size_label.limiter_mut().set_max_width(28);
        toolbar.add_widget(Box::new(size_label));

        let mut size = TheTextLineEdit::new(TheId::named(ISO_PAINT_TOOL_SIZE));
        size.set_value(TheValue::Float(self.size));
        size.set_info_text(None);
        size.set_range(TheValue::RangeF32(0.05..=8.0));
        size.set_continuous(true);
        size.set_status_text(&fl!("status_iso_paint_size"));
        size.limiter_mut().set_min_width(104);
        size.limiter_mut().set_max_width(104);
        toolbar.add_widget(Box::new(size));

        let mut opacity_label = TheText::new(TheId::named("Iso Paint Opacity Label"));
        opacity_label.set_text(fl!("iso_paint_opacity"));
        opacity_label.set_text_size(12.0);
        opacity_label.set_text_color([230, 230, 230, 255]);
        opacity_label.limiter_mut().set_min_width(46);
        opacity_label.limiter_mut().set_max_width(46);
        toolbar.add_widget(Box::new(opacity_label));

        let mut opacity = TheTextLineEdit::new(TheId::named(ISO_PAINT_TOOL_OPACITY));
        opacity.set_value(TheValue::Float(self.opacity));
        opacity.set_info_text(None);
        opacity.set_range(TheValue::RangeF32(0.0..=1.0));
        opacity.set_continuous(true);
        opacity.set_status_text(&fl!("status_iso_paint_opacity"));
        opacity.limiter_mut().set_min_width(104);
        opacity.limiter_mut().set_max_width(104);
        toolbar.add_widget(Box::new(opacity));

        toolbar.set_reverse_index(Some(5));

        toolbar_canvas.set_layout(toolbar);
        canvas.set_top(toolbar_canvas);

        let mut center = TheCanvas::new();

        let mut brush_canvas = TheCanvas::new();

        let mut brush_area = TheCanvas::new();
        let mut board = IsoPaintBrushBoard::new(TheId::named(ISO_PAINT_BRUSH_LIST));
        board.set_selected(self.selected_brush);
        brush_area.set_widget(board);
        brush_canvas.set_center(brush_area);

        let mut material_canvas = TheCanvas::new();
        material_canvas.limiter_mut().set_min_height(58);
        material_canvas.limiter_mut().set_max_height(58);
        let mut material_strip = IsoPaintMaterialStrip::new(TheId::named(ISO_PAINT_MATERIAL_STRIP));
        material_strip.set_material(self.material_preset, self.material_finish);
        material_canvas.set_widget(material_strip);
        brush_canvas.set_bottom(material_canvas);

        let mut inspector_canvas = TheCanvas::new();
        inspector_canvas.limiter_mut().set_min_width(300);
        inspector_canvas.limiter_mut().set_max_width(300);
        let mut inspector = TheTextLayout::new(TheId::named(ISO_PAINT_INSPECTOR));
        inspector.set_text_margin(24);
        inspector.set_text_align(TheHorizontalAlign::Right);
        inspector.limiter_mut().set_min_width(300);
        inspector.limiter_mut().set_max_width(300);
        self.nodeui.apply_to_text_layout(&mut inspector);
        inspector_canvas.set_layout(inspector);

        center.set_right(inspector_canvas);
        center.set_center(brush_canvas);

        canvas.set_center(center);
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
                self.pattern_kind =
                    Self::pattern_kind_from_key(&region.iso_paint.active_pattern_kind);
                self.pattern_scale = region.iso_paint.active_pattern_scale;
                self.pattern_mortar = region.iso_paint.active_pattern_mortar;
                self.pattern_detail = region.iso_paint.active_pattern_detail;
                self.pattern_variation = region.iso_paint.active_pattern_variation;
                region.iso_paint.visible
            })
            .unwrap_or(true);
        self.sync_inspector(ui, ctx);
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
            TheEvent::Custom(id, TheValue::Int(index)) if id.name == ISO_PAINT_BRUSH_SELECTED => {
                self.select_brush((*index).max(0) as usize, ui, ctx);
                self.sync_project_settings(project, server_ctx);
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
                    }
                    ISO_PAINT_MATERIAL_FINISH_SELECTED => {
                        let max = MATERIAL_FINISH_VALUES.len().saturating_sub(1) as i32;
                        self.material_finish = (*index).max(0).min(max);
                    }
                    _ => {}
                }
                self.sync_inspector(ui, ctx);
                self.sync_project_settings(project, server_ctx);
                return true;
            }
            TheEvent::ValueChanged(id, value)
                if matches!(
                    id.name.as_str(),
                    ISO_PAINT_TOOL_SIZE | ISO_PAINT_TOOL_OPACITY
                ) =>
            {
                if let Some(value) = value.to_f32() {
                    match id.name.as_str() {
                        ISO_PAINT_TOOL_SIZE => self.size = value.clamp(0.05, 8.0),
                        ISO_PAINT_TOOL_OPACITY => self.opacity = value.clamp(0.0, 1.0),
                        _ => {}
                    }
                    self.sync_project_settings(project, server_ctx);
                    return true;
                }
            }
            _ => {
                if self.nodeui.handle_event(event) {
                    if let TheEvent::ValueChanged(id, value) = event {
                        match id.name.as_str() {
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
                    }
                    if !matches!(event, TheEvent::ValueChanged(_, _)) {
                        if let Some(layout) = ui.get_text_layout(ISO_PAINT_INSPECTOR) {
                            self.nodeui.apply_to_text_layout(layout);
                        }
                        ctx.ui.relayout = true;
                    }
                    return true;
                }
            }
        }
        false
    }
}
