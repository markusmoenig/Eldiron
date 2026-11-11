//! Rect-style in-viewport Paint controls. Artwork stays in project MZ recipes.
use crate::prelude::*;
use shared::paint_source::Spec;

#[derive(Clone)]
pub struct Controls {
    pub title: String,
    pub source: Option<Uuid>,
    pub spec: Option<Spec>,
    pub operation: String,
    pub mode: String,
    pub shape: String,
    pub clip: String,
    pub rotation: f32,
    pub size: f32,
    pub opacity: f32,
    pub density: f32,
    pub jitter: f32,
    pub material: u8,
    pub colors: Vec<[u8; 4]>,
    pub palette: ThePalette,
    pub visible: bool,
}
impl Controls {
    pub fn read(project: &mut Project, server: &ServerContext) -> Option<Self> {
        let paint = super::paint_picker::layer(project, server)?;
        let source = paint.active_source;
        let mut controls = Self {
            title: String::new(),
            source,
            spec: None,
            operation: paint.active_operation.clone(),
            mode: paint.active_material_mode.clone(),
            shape: paint.active_brush_shape.clone(),
            clip: paint.active_clip.clone(),
            rotation: paint.active_stamp_rotation_jitter,
            size: paint.active_size,
            opacity: paint.active_opacity,
            density: paint.active_stamp_density,
            jitter: paint.active_stamp_size_jitter,
            material: paint.active_material_id.min(59),
            colors: paint.active_palette_colors.clone(),
            palette: ThePalette::default(),
            visible: paint.visible,
        };
        controls.palette = project.palette.clone();
        controls.title = source
            .and_then(|id| project.tiles.get(&id))
            .map_or_else(|| fl!("iso_paint_color"), |t| t.alias.clone());
        controls.spec = source
            .and_then(|id| crate::procedural_material::paint_assets::selected_spec(project, id));
        if let Some(spec) = &controls.spec {
            controls.colors = spec.colors.clone();
            controls.material = spec.material_id;
            controls.shape = spec.shape.clone();
        }
        Some(controls)
    }
}

#[derive(Clone)]
enum Action {
    Mode(usize),
    Number(&'static str, f32),
    Input(&'static str, f32),
    Settings,
    Choice(&'static str, Vec<String>, usize),
    Color(usize),
    Command(&'static str),
}
#[derive(Default)]
pub struct PaintOverlay {
    controls: Option<Controls>,
    buttons: Vec<(TheDim, Action)>,
    bounds: TheDim,
    down: bool,
}
impl PaintOverlay {
    pub fn is_active(&self) -> bool {
        self.controls.is_some()
    }
    pub fn sync(&mut self, project: &mut Project, server: &ServerContext) {
        self.controls = Controls::read(project, server);
    }
    fn button(
        &mut self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: TheDim,
        label: &str,
        selected: bool,
        action: Action,
    ) {
        let stride = buffer.stride();
        let c = crate::hud::colors();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &(
                rect.x as usize,
                rect.y as usize,
                rect.width as usize,
                rect.height as usize,
            ),
            stride,
            &if selected { c.selected } else { c.panel },
        );
        buffer.draw_rect_outline(&rect, &if selected { c.accent } else { c.border });
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(
                rect.x as usize + 4,
                rect.y as usize,
                rect.width as usize - 8,
                rect.height as usize,
            ),
            stride,
            label,
            TheFontSettings {
                size: 13.,
                ..Default::default()
            },
            &c.selected_text,
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );
        self.buttons.push((rect, action));
    }
    fn step(
        &mut self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        x: i32,
        y: i32,
        w: i32,
        label: String,
        key: &'static str,
        value: f32,
        step: f32,
    ) {
        let stride = buffer.stride();
        let c = crate::hud::colors();
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(x as usize, y as usize, (w - 60) as usize, 28),
            stride,
            &label,
            TheFontSettings {
                size: 13.,
                ..Default::default()
            },
            &c.selected_text,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        self.buttons
            .push((TheDim::rect(x, y, w - 62, 28), Action::Input(key, value)));
        self.button(
            buffer,
            ctx,
            TheDim::rect(x + w - 58, y, 26, 28),
            "−",
            false,
            Action::Number(key, value - step),
        );
        self.button(
            buffer,
            ctx,
            TheDim::rect(x + w - 28, y, 26, 28),
            "+",
            false,
            Action::Number(key, value + step),
        );
    }
    pub fn draw(&mut self, buffer: &mut TheRGBABuffer, ctx: &mut TheContext) {
        let stride = buffer.stride();
        let Some(c) = self.controls.clone() else {
            return;
        };
        if buffer.dim().width < 340 || buffer.dim().height < 260 {
            return;
        }
        let width = 504.min(buffer.dim().width - 16);
        let x = 16;
        let y = 34;
        let inner = width - 16;
        let gap = 6;
        let stamp = c.mode == "stamp";
        let pattern = c.spec.as_ref().is_some_and(|s| s.brush == "brick");
        let height = if pattern { 274 } else { 240 };
        self.bounds = TheDim::rect(8, 26, width, height);
        let theme = crate::hud::colors();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &(
                self.bounds.x as usize,
                self.bounds.y as usize,
                self.bounds.width as usize,
                self.bounds.height as usize,
            ),
            stride,
            &theme.background,
        );
        buffer.draw_rect_outline(&self.bounds, &theme.border);
        self.buttons.clear();
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(x as usize, y as usize, (inner - 230) as usize, 28),
            stride,
            &c.title,
            TheFontSettings {
                size: 14.,
                ..Default::default()
            },
            &theme.selected_text,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        self.button(
            buffer,
            ctx,
            TheDim::rect(x + inner - 224, y, 36, 28),
            if c.visible { "◉" } else { "○" },
            c.visible,
            Action::Command("Visible"),
        );
        self.button(
            buffer,
            ctx,
            TheDim::rect(x + inner - 182, y, 98, 28),
            &fl!("paint_overlay_settings"),
            false,
            Action::Settings,
        );
        self.button(
            buffer,
            ctx,
            TheDim::rect(x + inner - 78, y, 78, 28),
            &fl!("paint_overlay_advanced"),
            false,
            Action::Command("Edit"),
        );
        let labels = [
            fl!("paint_picker_paint"),
            fl!("paint_picker_scatter"),
            fl!("iso_paint_operation_erase"),
            fl!("iso_paint_operation_pick"),
            fl!("iso_paint_operation_select"),
        ];
        let selected = match c.operation.as_str() {
            "erase" => 2,
            "pick" => 3,
            "select" => 4,
            _ if stamp => 1,
            _ => 0,
        };
        let cell = (inner - 4 * gap) / 5;
        for (i, label) in labels.iter().enumerate() {
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + i as i32 * (cell + gap), y + 34, cell, 30),
                label,
                selected == i,
                Action::Mode(i),
            );
        }
        let half = (inner - gap) / 2;
        self.step(
            buffer,
            ctx,
            x,
            y + 70,
            half,
            format!("{}: {:.2}", fl!("paint_picker_size"), c.size),
            "Size",
            c.size,
            0.1,
        );
        self.step(
            buffer,
            ctx,
            x + half + gap,
            y + 70,
            half,
            format!("{}: {:.0}%", fl!("iso_paint_opacity"), c.opacity * 100.),
            "Opacity",
            c.opacity,
            0.05,
        );
        let palette_label = fl!("iso_paint_section_colors");
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(x as usize, (y + 105) as usize, 74, 28),
            stride,
            &palette_label,
            TheFontSettings {
                size: 13.,
                ..Default::default()
            },
            &theme.selected_text,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        for (i, color) in c.colors.iter().take(8).enumerate() {
            let rect = TheDim::rect(x + 82 + i as i32 * 36, y + 105, 30, 28);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &(
                    rect.x as usize,
                    rect.y as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                stride,
                color,
            );
            buffer.draw_rect_outline(&rect, &theme.border);
            self.buttons.push((rect, Action::Color(i)));
        }
        let families = crate::docks::iso_paint::IsoPaintDock::material_preset_labels();
        let finishes = crate::docks::iso_paint::IsoPaintDock::material_finish_labels();
        self.button(
            buffer,
            ctx,
            TheDim::rect(x, y + 140, half, 28),
            &format!("{} ▾", families[c.material as usize / 4]),
            false,
            Action::Choice("Material", families, c.material as usize / 4),
        );
        self.button(
            buffer,
            ctx,
            TheDim::rect(x + half + gap, y + 140, half, 28),
            &format!("{} ▾", finishes[c.material as usize % 4]),
            false,
            Action::Choice("Finish", finishes, c.material as usize % 4),
        );
        if stamp {
            self.step(
                buffer,
                ctx,
                x,
                y + 174,
                half,
                format!(
                    "{}: {:.0}%",
                    fl!("iso_paint_stamp_density"),
                    c.density * 100.
                ),
                "Density",
                c.density,
                0.05,
            );
            self.step(
                buffer,
                ctx,
                x + half + gap,
                y + 174,
                half,
                format!("{}: {:.0}%", fl!("paint_picker_jitter"), c.jitter * 100.),
                "Jitter",
                c.jitter,
                0.05,
            );
        } else if selected == 4 {
            self.button(
                buffer,
                ctx,
                TheDim::rect(x, y + 174, half, 28),
                &fl!("iso_paint_fill_selection"),
                false,
                Action::Command("Fill"),
            );
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + half + gap, y + 174, half, 28),
                &fl!("iso_paint_clear_selection"),
                false,
                Action::Command("Clear"),
            );
        } else {
            let shapes = vec![
                fl!("iso_paint_brush_shape_solid"),
                fl!("iso_paint_brush_shape_soft"),
                fl!("iso_paint_brush_shape_dirt"),
                fl!("iso_paint_brush_shape_speckle"),
                fl!("iso_paint_brush_shape_jagged"),
                fl!("iso_paint_brush_shape_scratch"),
                fl!("iso_paint_brush_shape_wash"),
            ];
            let index = [
                "solid", "soft", "dirt", "speckle", "jagged", "scratch", "wash",
            ]
            .iter()
            .position(|s| *s == c.shape)
            .unwrap_or(0);
            self.button(
                buffer,
                ctx,
                TheDim::rect(x, y + 174, half, 28),
                &format!("{} ▾", shapes[index]),
                false,
                Action::Choice("Shape", shapes, index),
            );
            let quarter = (half - gap) / 2;
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + half + gap, y + 174, quarter, 28),
                &fl!("iso_paint_material_mode_coat"),
                c.mode == "coat",
                Action::Command("Coat"),
            );
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + half + gap + quarter + gap, y + 174, quarter, 28),
                &fl!("iso_paint_material_mode_replace"),
                c.mode == "replace",
                Action::Command("Replace"),
            );
        }
        if pattern {
            if let Some(s) = &c.spec {
                self.step(
                    buffer,
                    ctx,
                    x,
                    y + 208,
                    half,
                    format!("{}: {:.2}", fl!("iso_paint_pattern_scale"), s.scale),
                    "Scale",
                    s.scale,
                    0.1,
                );
                self.step(
                    buffer,
                    ctx,
                    x + half + gap,
                    y + 208,
                    half,
                    format!("{}: {:.2}", fl!("iso_paint_mortar"), s.mortar),
                    "Mortar",
                    s.mortar,
                    0.02,
                );
            }
        }
    }
    pub fn map_event(
        &mut self,
        event: &MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server: &ServerContext,
    ) -> bool {
        match event {
            MapEvent::MapClicked(at) if self.bounds.contains(*at) => {
                self.down = true;
                let Some(c) = self.controls.as_ref() else {
                    return true;
                };
                let action = self.buttons.iter().find(|(r, _)| r.contains(*at)).cloned();
                if let Some((rect, action)) = action {
                    match action {
                        Action::Mode(index) => ctx.ui.send(TheEvent::IndexChanged(
                            TheId::named(super::paint_picker::MODE),
                            index,
                        )),
                        Action::Number(key, value) => ctx.ui.send(TheEvent::ValueChanged(
                            TheId::named(&format!("Paint Overlay {key}")),
                            TheValue::Float(value),
                        )),
                        Action::Settings => {
                            let mut fields = TheNodeUI::default();
                            fields.add_item(TheNodeUIItem::Selector(
                                "Paint Overlay Clip".into(),
                                fl!("iso_paint_clip"),
                                String::new(),
                                vec![fl!("iso_paint_clip_none"), fl!("iso_paint_clip_object")],
                                if c.clip == "surface" { 1 } else { 0 },
                            ));
                            if let Some(spec) = &c.spec {
                                if spec.brush == "brick" {
                                    fields.add_item(TheNodeUIItem::Selector(
                                        "Paint Overlay Pattern".into(),
                                        fl!("iso_paint_pattern_kind"),
                                        String::new(),
                                        vec![
                                            fl!("iso_paint_pattern_tiles"),
                                            fl!("iso_paint_pattern_bricks"),
                                            fl!("iso_paint_pattern_arch"),
                                        ],
                                        ["tiles", "bricks", "arch"]
                                            .iter()
                                            .position(|p| *p == spec.pattern)
                                            .unwrap_or(1)
                                            as i32,
                                    ));
                                    for (key, label, value, min, max) in [
                                        (
                                            "Scale",
                                            fl!("iso_paint_pattern_scale"),
                                            spec.scale,
                                            0.25,
                                            4.,
                                        ),
                                        ("Mortar", fl!("iso_paint_mortar"), spec.mortar, 0., 0.4),
                                        (
                                            "Detail",
                                            fl!("iso_paint_pattern_detail"),
                                            spec.detail,
                                            0.,
                                            1.,
                                        ),
                                        (
                                            "Variation",
                                            fl!("iso_paint_pattern_variation"),
                                            spec.variation,
                                            0.,
                                            1.,
                                        ),
                                    ] {
                                        fields.add_item(TheNodeUIItem::FloatEditSlider(
                                            format!("Paint Overlay {key}"),
                                            label,
                                            String::new(),
                                            value,
                                            min..=max,
                                            true,
                                        ));
                                    }
                                }
                                if spec.stamp && !IsoPaintStamp::keeps_upright(&spec.brush) {
                                    fields.add_item(TheNodeUIItem::FloatEditSlider(
                                        "Paint Overlay Rotate".into(),
                                        fl!("paint_picker_rotate"),
                                        String::new(),
                                        c.rotation,
                                        0.0..=1.0,
                                        true,
                                    ));
                                }
                            }
                            let mut layout =
                                TheTextLayout::new(TheId::named("Paint Overlay Settings"));
                            layout.set_margin(Vec4::new(10, 10, 10, 10));
                            layout.set_text_margin(8);
                            layout.set_fixed_text_width(120);
                            fields.apply_to_text_layout(&mut layout);
                            let mut canvas = TheCanvas::new();
                            canvas.limiter_mut().set_max_size(Vec2::new(
                                420,
                                if c.spec.as_ref().is_some_and(|s| s.brush == "brick") {
                                    240
                                } else {
                                    96
                                },
                            ));
                            canvas.set_layout(layout);
                            let dim = *crate::utils::map_editor_render_view(ui, server)
                                .unwrap()
                                .dim();
                            ui.show_popover(
                                TheId::named("Paint Overlay Settings"),
                                TheDim::rect(
                                    dim.x + rect.x,
                                    dim.y + rect.y,
                                    rect.width,
                                    rect.height,
                                ),
                                canvas,
                                ctx,
                            );
                        }
                        Action::Input(key, value) => {
                            let dim = *crate::utils::map_editor_render_view(ui, server)
                                .unwrap()
                                .dim();
                            let mut field =
                                TheTextLineEdit::new(TheId::named(&format!("Paint Overlay {key}")));
                            field.set_value(TheValue::Float(value));
                            let range = match key {
                                "Size" => 0.05..=16.,
                                "Scale" => 0.25..=4.,
                                "Mortar" => 0.0..=0.4,
                                _ => 0.0..=1.,
                            };
                            field.set_range(TheValue::RangeF32(range));
                            field.limiter_mut().set_max_width(180);
                            let mut row = TheHLayout::new(TheId::empty());
                            row.set_margin(Vec4::new(8, 8, 8, 8));
                            row.add_widget(Box::new(field));
                            let mut canvas = TheCanvas::new();
                            canvas.limiter_mut().set_max_size(Vec2::new(204, 42));
                            canvas.set_layout(row);
                            ui.show_popover(
                                TheId::named("Paint Overlay Number"),
                                TheDim::rect(
                                    dim.x + rect.x,
                                    dim.y + rect.y,
                                    rect.width,
                                    rect.height,
                                ),
                                canvas,
                                ctx,
                            );
                        }
                        Action::Command(key) => ctx.ui.send(TheEvent::Custom(
                            TheId::named("Paint Overlay Command"),
                            TheValue::Text(key.into()),
                        )),
                        Action::Choice(key, choices, index) => {
                            let items = choices
                                .into_iter()
                                .enumerate()
                                .map(|(i, label)| {
                                    TheContextMenuItem::new(
                                        format!("{}{}", if i == index { "✓ " } else { "" }, label),
                                        TheId::named(&format!("Paint Overlay Choice {key} {i}")),
                                    )
                                })
                                .collect();
                            let dim = *crate::utils::map_editor_render_view(ui, server)
                                .unwrap()
                                .dim();
                            ctx.ui.send(TheEvent::ShowContextMenu(
                                TheId::named("Paint Overlay Choices"),
                                Vec2::new(dim.x + rect.x, dim.y + rect.y + rect.height),
                                TheContextMenu {
                                    items,
                                    ..Default::default()
                                },
                            ));
                        }
                        Action::Color(slot) => {
                            let dim = *crate::utils::map_editor_render_view(ui, server)
                                .unwrap()
                                .dim();
                            let mut picker = ThePalettePicker::new(TheId::named_with_id(
                                &format!("Paint Overlay Palette {slot}"),
                                c.source.unwrap_or_default(),
                            ));
                            picker.set_palette(c.palette.clone());
                            picker.set_rows_columns(16, 16);
                            picker.set_adaptive_cell_size(true);
                            picker.set_index(
                                c.spec
                                    .as_ref()
                                    .and_then(|s| s.palette.get(slot))
                                    .copied()
                                    .flatten()
                                    .unwrap_or(0) as usize,
                            );
                            picker.limiter_mut().set_max_size(Vec2::new(352, 352));
                            let mut canvas = TheCanvas::new();
                            canvas.limiter_mut().set_max_size(Vec2::new(352, 352));
                            canvas.set_widget(picker);
                            ui.show_popover(
                                TheId::named("Paint Overlay Colors"),
                                TheDim::rect(
                                    dim.x + rect.x,
                                    dim.y + rect.y,
                                    rect.width,
                                    rect.height,
                                ),
                                canvas,
                                ctx,
                            );
                        }
                    }
                }
                ctx.ui.redraw_all = true;
                true
            }
            // A popover can consume the release of the click that opened it.
            // The next scene press must always start a fresh painting gesture.
            MapEvent::MapClicked(_) => {
                self.down = false;
                false
            }
            MapEvent::MapDragged(_) | MapEvent::MapUp(_) if self.down => {
                if matches!(event, MapEvent::MapUp(_)) {
                    self.down = false;
                }
                true
            }
            MapEvent::MapHover(at) if self.bounds.contains(*at) => true,
            MapEvent::MapEscape => {
                self.down = false;
                false
            }
            _ => false,
        }
    }
}
