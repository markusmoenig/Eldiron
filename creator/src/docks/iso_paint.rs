use crate::prelude::*;

const ISO_PAINT_BRUSH_LIST: &str = "Iso Paint Brush List";
const ISO_PAINT_MATERIAL_STRIP: &str = "Iso Paint Material Strip";
const ISO_PAINT_INSPECTOR: &str = "Iso Paint Inspector";
const ISO_PAINT_BRUSH_SELECTED: &str = "Iso Paint Brush Selected";
const ISO_PAINT_MATERIAL_PRESET_SELECTED: &str = "Iso Paint Material Preset Selected";
const ISO_PAINT_MATERIAL_FINISH_SELECTED: &str = "Iso Paint Material Finish Selected";
const ISO_PAINT_TOOL_DRAW: &str = "Iso Paint Tool Draw";
const ISO_PAINT_TOOL_ERASE: &str = "Iso Paint Tool Erase";
const ISO_PAINT_TOOL_PICK: &str = "Iso Paint Tool Pick";
const ISO_PAINT_TOOL_SOFTEN: &str = "Iso Paint Tool Soften";
const ISO_PAINT_TOOL_SIZE: &str = "Iso Paint Tool Size";
const ISO_PAINT_TOOL_OPACITY: &str = "Iso Paint Tool Opacity";
const ISO_PAINT_AMOUNT: &str = "Iso Paint Amount";
const ISO_PAINT_PATTERN_SCALE: &str = "Iso Paint Pattern Scale";
const ISO_PAINT_MORTAR: &str = "Iso Paint Mortar";
const ISO_PAINT_STAMP_DENSITY: &str = "Iso Paint Stamp Density";
const MATERIAL_PRESET_VALUES: [&str; 11] = [
    "default", "stone", "wood", "metal", "glass", "water", "mirror", "emissive", "dirt", "fabric",
    "plastic",
];
const MATERIAL_FINISH_VALUES: [&str; 4] = ["natural", "matte", "polished", "wet"];

#[derive(Clone, Copy, PartialEq)]
enum IsoPaintOperation {
    Draw,
    Erase,
    Pick,
    Soften,
}

#[derive(Clone, Copy)]
struct IsoPaintBrushPreset {
    key: &'static str,
    size: f32,
    opacity: f32,
    amount: f32,
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
                let brick_h = (h / 5).max(6);
                let brick_w = (w / 3).max(12);
                let mut yy = y + brick_h;
                let mut row = 0usize;
                while yy < y + h {
                    buffer.draw_line(x as i32, yy as i32, (x + w) as i32, yy as i32, dark);
                    let offset = if row % 2 == 0 { 0 } else { brick_w / 2 };
                    let mut xx = x + offset;
                    while xx < x + w {
                        buffer.draw_line(xx as i32, y as i32, xx as i32, (y + h) as i32, dark);
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

    fn material_color(index: usize) -> [u8; 4] {
        match MATERIAL_PRESET_VALUES
            .get(index)
            .copied()
            .unwrap_or("default")
        {
            "stone" => [112, 115, 111, 255],
            "wood" => [126, 78, 42, 255],
            "metal" => [125, 135, 142, 255],
            "glass" => [122, 190, 205, 255],
            "water" => [45, 107, 178, 255],
            "mirror" => [204, 209, 213, 255],
            "emissive" => [127, 196, 73, 255],
            "dirt" => [95, 72, 48, 255],
            "fabric" => [148, 61, 78, 255],
            "plastic" => [89, 106, 156, 255],
            _ => [132, 132, 128, 255],
        }
    }

    fn finish_color(index: usize) -> [u8; 4] {
        match MATERIAL_FINISH_VALUES
            .get(index)
            .copied()
            .unwrap_or("natural")
        {
            "matte" => [105, 105, 101, 255],
            "polished" => [178, 184, 190, 255],
            "wet" => [82, 119, 154, 255],
            _ => [137, 128, 108, 255],
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

        for index in 0..MATERIAL_PRESET_VALUES.len() {
            let x = PAD_X + index as i32 * (preset_w + GAP);
            let rect = TheDim::new(x, preset_y, preset_w, preset_h);
            let global = (
                utuple.0 + x as usize,
                utuple.1 + preset_y as usize,
                preset_w as usize,
                preset_h as usize,
            );
            let color = Self::material_color(index);
            ctx.draw.rect(buffer.pixels_mut(), &global, stride, &color);
            if MATERIAL_PRESET_VALUES[index] == "emissive" {
                buffer.draw_line(
                    global.0 as i32 + 3,
                    global.1 as i32 + 3,
                    (global.0 + global.2 - 4) as i32,
                    (global.1 + global.3 - 4) as i32,
                    [235, 245, 122, 255],
                );
            }
            let border = if self.material_preset == index {
                WHITE
            } else if self.hovered == Some(("preset", index)) {
                [210, 210, 210, 255]
            } else {
                [38, 38, 38, 255]
            };
            ctx.draw
                .rect_outline_border(buffer.pixels_mut(), &global, stride, &border, 1);
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
            let mut color = Self::finish_color(index);
            if self.material_finish != index {
                color = [
                    (color[0] as f32 * 0.64) as u8,
                    (color[1] as f32 * 0.64) as u8,
                    (color[2] as f32 * 0.64) as u8,
                    255,
                ];
            }
            ctx.draw.rect(buffer.pixels_mut(), &global, stride, &color);
            let border = if self.material_finish == index {
                WHITE
            } else if self.hovered == Some(("finish", index)) {
                [210, 210, 210, 255]
            } else {
                [38, 38, 38, 255]
            };
            ctx.draw
                .rect_outline_border(buffer.pixels_mut(), &global, stride, &border, 1);
            let luminance =
                (color[0] as u16 * 30 + color[1] as u16 * 59 + color[2] as u16 * 11) / 100;
            let text_color = if luminance > 150 {
                [28, 28, 28, 255]
            } else {
                [238, 238, 238, 255]
            };
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(global.0 + 2, global.1, global.2.saturating_sub(4), global.3),
                stride,
                &finish_labels[index],
                TheFontSettings {
                    size: 10.0,
                    ..Default::default()
                },
                &text_color,
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
    nodeui: TheNodeUI,
}

impl IsoPaintDock {
    const BRUSHES: [IsoPaintBrushPreset; 6] = [
        IsoPaintBrushPreset {
            key: "material",
            size: 1.0,
            opacity: 1.0,
            amount: 0.65,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "brick",
            size: 1.0,
            opacity: 1.0,
            amount: 0.85,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "moss",
            size: 1.6,
            opacity: 0.75,
            amount: 0.5,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.7,
        },
        IsoPaintBrushPreset {
            key: "crack",
            size: 0.6,
            opacity: 1.0,
            amount: 0.8,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "grass",
            size: 1.2,
            opacity: 1.0,
            amount: 0.6,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
        IsoPaintBrushPreset {
            key: "screen",
            size: 1.4,
            opacity: 0.7,
            amount: 0.45,
            pattern_scale: 1.0,
            mortar: 0.08,
            density: 0.6,
        },
    ];

    fn material_preset_labels() -> Vec<String> {
        vec![
            fl!("material_preset_default"),
            fl!("material_preset_stone"),
            fl!("material_preset_wood"),
            fl!("material_preset_metal"),
            fl!("material_preset_glass"),
            fl!("material_preset_water"),
            fl!("material_preset_mirror"),
            fl!("material_preset_emissive"),
            fl!("material_preset_dirt"),
            fl!("material_preset_fabric"),
            fl!("material_preset_plastic"),
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

    fn operation_button_ids() -> [(&'static str, IsoPaintOperation); 4] {
        [
            (ISO_PAINT_TOOL_DRAW, IsoPaintOperation::Draw),
            (ISO_PAINT_TOOL_ERASE, IsoPaintOperation::Erase),
            (ISO_PAINT_TOOL_PICK, IsoPaintOperation::Pick),
            (ISO_PAINT_TOOL_SOFTEN, IsoPaintOperation::Soften),
        ]
    }

    fn operation_label(operation: IsoPaintOperation) -> String {
        match operation {
            IsoPaintOperation::Draw => fl!("iso_paint_operation_draw"),
            IsoPaintOperation::Erase => fl!("iso_paint_operation_erase"),
            IsoPaintOperation::Pick => fl!("iso_paint_operation_pick"),
            IsoPaintOperation::Soften => fl!("iso_paint_operation_soften"),
        }
    }

    fn brush_label(key: &str) -> String {
        match key {
            "material" => fl!("iso_paint_brush_material"),
            "brick" => fl!("iso_paint_brush_brick"),
            "moss" => fl!("iso_paint_brush_moss"),
            "crack" => fl!("iso_paint_brush_crack"),
            "grass" => fl!("iso_paint_brush_grass"),
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
            "screen" => fl!("iso_paint_brush_screen_desc"),
            _ => String::new(),
        }
    }

    fn build_nodeui(brush: IsoPaintBrushPreset) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_brush")));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ISO_PAINT_AMOUNT.into(),
            fl!("iso_paint_strength"),
            fl!("status_iso_paint_strength"),
            brush.amount,
            0.0..=1.0,
            true,
        ));

        match brush.key {
            "brick" => {
                nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_pattern")));
                nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                    ISO_PAINT_PATTERN_SCALE.into(),
                    fl!("iso_paint_pattern_scale"),
                    fl!("status_iso_paint_pattern_scale"),
                    brush.pattern_scale,
                    0.25..=4.0,
                    true,
                ));
                nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                    ISO_PAINT_MORTAR.into(),
                    fl!("iso_paint_mortar"),
                    fl!("status_iso_paint_mortar"),
                    brush.mortar,
                    0.0..=0.4,
                    true,
                ));
            }
            "moss" | "grass" => {
                nodeui.add_item(TheNodeUIItem::Separator(fl!("iso_paint_section_stamps")));
                nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                    ISO_PAINT_STAMP_DENSITY.into(),
                    fl!("iso_paint_stamp_density"),
                    fl!("status_iso_paint_stamp_density"),
                    brush.density,
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
        self.nodeui = Self::build_nodeui(self.selected_preset());
        self.sync_toolbar(ctx);
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

    fn sync_toolbar(&self, ctx: &mut TheContext) {
        for (id, operation) in Self::operation_button_ids() {
            ctx.ui.set_widget_state(
                id.to_string(),
                if operation == self.operation {
                    TheWidgetState::Selected
                } else {
                    TheWidgetState::None
                },
            );
        }
    }

    fn select_brush(&mut self, index: usize, ui: &mut TheUI, ctx: &mut TheContext) {
        self.selected_brush = index.min(Self::BRUSHES.len().saturating_sub(1));
        let brush = self.selected_preset();
        self.size = brush.size;
        self.opacity = brush.opacity;
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
        self.sync_toolbar(ctx);
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            format!(
                "{}: {}",
                fl!("iso_paint_operation"),
                Self::operation_label(operation)
            ),
        ));
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
            nodeui: Self::build_nodeui(Self::BRUSHES[0]),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar = TheHLayout::new(TheId::named("Iso Paint Toolbar"));
        toolbar.set_background_color(None);
        toolbar.set_margin(Vec4::new(10, 2, 6, 2));
        toolbar.set_padding(4);

        let buttons = [
            (
                ISO_PAINT_TOOL_DRAW,
                IsoPaintOperation::Draw,
                "paint-brush",
                fl!("iso_paint_operation_draw"),
                fl!("status_iso_paint_operation_draw"),
            ),
            (
                ISO_PAINT_TOOL_ERASE,
                IsoPaintOperation::Erase,
                "eraser",
                fl!("iso_paint_operation_erase"),
                fl!("status_iso_paint_operation_erase"),
            ),
            (
                ISO_PAINT_TOOL_PICK,
                IsoPaintOperation::Pick,
                "eyedropper-sample",
                fl!("iso_paint_operation_pick"),
                fl!("status_iso_paint_operation_pick"),
            ),
            (
                ISO_PAINT_TOOL_SOFTEN,
                IsoPaintOperation::Soften,
                "square_half",
                fl!("iso_paint_operation_soften"),
                fl!("status_iso_paint_operation_soften"),
            ),
        ];

        for (id, operation, _icon, label, status) in buttons {
            let mut button = TheTraybarButton::new(TheId::named(id));
            button.set_text(label);
            button.set_status_text(&status);
            button.set_fixed_size(false);
            button.limiter_mut().set_min_width(72);
            button.limiter_mut().set_max_width(72);
            if operation == self.operation {
                button.set_state(TheWidgetState::Selected);
            }
            toolbar.add_widget(Box::new(button));
        }

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
        size.limiter_mut().set_min_width(76);
        size.limiter_mut().set_max_width(76);
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
        opacity.limiter_mut().set_min_width(76);
        opacity.limiter_mut().set_max_width(76);
        toolbar.add_widget(Box::new(opacity));
        toolbar.set_reverse_index(Some(4));

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
        _project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        self.sync_inspector(ui, ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if matches!(
                    id.name.as_str(),
                    ISO_PAINT_TOOL_DRAW
                        | ISO_PAINT_TOOL_ERASE
                        | ISO_PAINT_TOOL_PICK
                        | ISO_PAINT_TOOL_SOFTEN
                ) =>
            {
                let operation = match id.name.as_str() {
                    ISO_PAINT_TOOL_ERASE => IsoPaintOperation::Erase,
                    ISO_PAINT_TOOL_PICK => IsoPaintOperation::Pick,
                    ISO_PAINT_TOOL_SOFTEN => IsoPaintOperation::Soften,
                    _ => IsoPaintOperation::Draw,
                };
                self.set_operation(operation, ui, ctx);
                return true;
            }
            TheEvent::Custom(id, TheValue::Int(index)) if id.name == ISO_PAINT_BRUSH_SELECTED => {
                self.select_brush((*index).max(0) as usize, ui, ctx);
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
                    return true;
                }
            }
            _ => {
                if self.nodeui.handle_event(event) {
                    if let Some(layout) = ui.get_text_layout(ISO_PAINT_INSPECTOR) {
                        self.nodeui.apply_to_text_layout(layout);
                    }
                    ctx.ui.relayout = true;
                    return true;
                }
            }
        }
        false
    }
}
