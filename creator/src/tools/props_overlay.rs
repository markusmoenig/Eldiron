//! 3D Props controls live over the viewport, leaving the dock to browse assets.
use super::props_recipe;
use crate::prelude::*;

#[derive(Clone)]
enum Action {
    Event(TheEvent),
    Placement,
    Controls,
}
#[derive(Default)]
pub struct PropsOverlay {
    bounds: TheDim,
    buttons: Vec<(TheDim, Action)>,
    down: bool,
    recipe: Option<(Uuid, Vec<props_recipe::Published>)>,
    resizable: bool,
    subdivisions: f32,
    palette: ThePalette,
    builtin: bool,
    deletable: bool,
}
impl PropsOverlay {
    pub fn sync(&mut self, project: &Project, server: &ServerContext) {
        self.palette = project.palette.clone();
        self.subdivisions = project.get_map(server).map_or(1., |m| m.subdivisions);
        self.builtin = server
            .curr_block_asset_id
            .is_some_and(|id| crate::blocks::block_asset(id).is_some());
        self.recipe = server.curr_block_asset_id.and_then(|id| {
            project
                .procedural_appearances
                .get(&id)
                .and_then(|s| denrim_materials::geometry::Document::parse(s).ok())
                .map(|doc| (id, props_recipe::published(&doc)))
        });
        self.resizable = server.curr_block_asset_id.is_some_and(|id| {
            crate::blocks::block_asset(id).is_some()
                || project
                    .block_props
                    .get(&id)
                    .is_some_and(crate::blocks::prefab_uses_auto_sizing)
        });
        self.deletable = server
            .curr_block_asset_id
            .is_some_and(|id| project.block_props.contains_key(&id));
    }
    pub fn refresh_fields(&self, ui: &mut TheUI) {
        if let Some((asset, controls)) = &self.recipe {
            for (i, field) in controls.iter().enumerate() {
                let c = &field.control;
                if c.expression.is_some() {
                    if let Some(widget) = ui.get_widget(&format!("Props Calculated {i}")) {
                        let text = format!("{:.3}", c.value);
                        if widget.value().to_string().as_deref() != Some(text.as_str()) {
                            widget.set_value(TheValue::Text(text));
                        }
                        widget.set_disabled(true);
                    }
                } else if let Some(widget) = ui.get_widget(&format!("Props Recipe {asset} {i}")) {
                    let current = widget.value();
                    let current = current
                        .to_f32()
                        .or_else(|| current.to_i32().map(|n| n as f32));
                    // Preserve the widget's text/render state when its value already matches.
                    if current.is_none_or(|v| (v - c.value).abs() > 1e-6) {
                        widget.set_value(if c.is_palette() || c.is_integer() {
                            TheValue::Int(c.value as i32)
                        } else {
                            TheValue::Float(c.value)
                        });
                    }
                }
            }
        }
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
        let c = crate::hud::colors();
        let stride = buffer.stride();
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
    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        server: &ServerContext,
    ) {
        self.buttons.clear();
        self.bounds = TheDim::default();
        if !server.editor_view_mode.is_3d() || buffer.dim().width < 340 || buffer.dim().height < 250
        {
            return;
        }
        let width = 420.min(buffer.dim().width - 16);
        let x = 16;
        let y = 62;
        let gap = 6;
        let half = (width - 22) / 2;
        self.bounds = TheDim::rect(8, 26, width, if self.recipe.is_some() { 174 } else { 208 });
        let theme = crate::hud::colors();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &(8, 26, width as usize, self.bounds.height as usize),
            stride,
            &theme.background,
        );
        buffer.draw_rect_outline(&self.bounds, &theme.border);
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(x as usize, 30, (width - 16) as usize, 26),
            stride,
            &format!(
                "{} · {}",
                fl!("tool_blocks"),
                server.curr_block_asset_name.as_deref().unwrap_or("")
            ),
            TheFontSettings {
                size: 13.,
                ..Default::default()
            },
            &theme.selected_text,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        let w = (width - 28) / 3;
        for (i, label) in [
            fl!("block_operation_place"),
            fl!("block_operation_replace"),
            fl!("block_operation_erase"),
        ]
        .iter()
        .enumerate()
        {
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + i as i32 * (w + 6), y, w, 28),
                label,
                server.block_operation == i as i32,
                Action::Event(TheEvent::IndexChanged(
                    TheId::named("Blocks Dock Operation"),
                    i,
                )),
            );
        }
        for (i, label) in [fl!("block_stroke_line"), fl!("block_stroke_rect")]
            .iter()
            .enumerate()
        {
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + i as i32 * (half + gap), y + 34, half, 28),
                label,
                server.block_stroke_mode == i as i32,
                Action::Event(TheEvent::IndexChanged(
                    TheId::named("Blocks Dock Stroke"),
                    i,
                )),
            );
        }
        self.button(
            buffer,
            ctx,
            TheDim::rect(x, y + 68, half, 28),
            &format!(
                "{}: {}°",
                fl!("props_rotate"),
                server.block_rotation_quarters.rem_euclid(4) * 90
            ),
            false,
            Action::Event(TheEvent::ValueChanged(
                TheId::named("Props Rotation"),
                TheValue::Float((server.block_rotation_quarters + 1).rem_euclid(4) as f32),
            )),
        );
        self.button(
            buffer,
            ctx,
            TheDim::rect(x + half + gap, y + 68, half, 28),
            &fl!("props_placement"),
            false,
            Action::Placement,
        );
        let edit = if let Some((asset, _)) = &self.recipe {
            TheEvent::Custom(TheId::named("Open Geometry Recipe"), TheValue::Id(*asset))
        } else {
            TheEvent::Custom(TheId::named("Props Edit"), TheValue::Empty)
        };
        self.button(
            buffer,
            ctx,
            TheDim::rect(x, y + 102, half, 28),
            &if self.recipe.is_some() {
                fl!("props_edit_recipe")
            } else {
                fl!("props_edit_prop")
            },
            false,
            Action::Event(edit),
        );
        if self.recipe.as_ref().is_some_and(|(_, c)| !c.is_empty()) {
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + half + gap, y + 102, half, 28),
                &fl!("props_recipe_controls"),
                false,
                Action::Controls,
            );
        } else if self.recipe.is_none() {
            self.button(
                buffer,
                ctx,
                TheDim::rect(x + half + gap, y + 102, half, 28),
                &fl!("prefab_duplicate"),
                false,
                Action::Event(TheEvent::StateChanged(
                    TheId::named("Blocks Dock Duplicate Prefab"),
                    TheWidgetState::Clicked,
                )),
            );
        }
        if self.recipe.is_none() {
            if server
                .curr_block_asset_id
                .is_some_and(|id| crate::blocks::block_asset(id).is_some())
            {
                self.button(
                    buffer,
                    ctx,
                    TheDim::rect(x, y + 136, half, 28),
                    &if server.block_damage_enabled {
                        fl!("block_damage_damaged")
                    } else {
                        fl!("block_damage_clean")
                    },
                    server.block_damage_enabled,
                    Action::Event(TheEvent::IndexChanged(
                        TheId::named("Blocks Dock Damage"),
                        usize::from(!server.block_damage_enabled),
                    )),
                );
            }
            if self.deletable {
                self.button(
                    buffer,
                    ctx,
                    TheDim::rect(x + half + gap, y + 136, half, 28),
                    &fl!("prefab_delete"),
                    false,
                    Action::Event(TheEvent::StateChanged(
                        TheId::named("Blocks Dock Delete Prefab"),
                        TheWidgetState::Clicked,
                    )),
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
                if let Some((rect, action)) =
                    self.buttons.iter().find(|(r, _)| r.contains(*at)).cloned()
                {
                    match action {
                        Action::Event(event) => ctx.ui.send(event),
                        Action::Placement | Action::Controls => {
                            let mut fields = TheNodeUI::default();
                            let mut rows = 0;
                            if matches!(action, Action::Placement) {
                                let mut numbers = vec![
                                    (
                                        "Grid",
                                        if self.builtin {
                                            fl!("props_grid")
                                        } else {
                                            fl!("props_level_height")
                                        },
                                        server.block_grid_cell_size,
                                        0.05,
                                        16.,
                                    ),
                                    (
                                        "Level",
                                        fl!("props_level"),
                                        server.block_grid_level as f32,
                                        -128.,
                                        128.,
                                    ),
                                ];
                                if !self.builtin {
                                    numbers.push((
                                        "Subdivisions",
                                        fl!("props_subdivisions"),
                                        self.subdivisions,
                                        1.,
                                        16.,
                                    ));
                                }
                                if self.resizable {
                                    numbers.extend([
                                        (
                                            "Height",
                                            fl!("props_height"),
                                            server.block_height_cells,
                                            0.05,
                                            16.,
                                        ),
                                        (
                                            "Width",
                                            fl!("props_width"),
                                            server.block_span_extra_cells,
                                            0.,
                                            16.,
                                        ),
                                        (
                                            "Depth",
                                            fl!("props_depth"),
                                            server.block_depth_extra_cells,
                                            0.,
                                            16.,
                                        ),
                                    ]);
                                }
                                for (key, label, value, min, max) in numbers {
                                    fields.add_item(TheNodeUIItem::FloatEditSlider(
                                        format!("Props {key}"),
                                        label,
                                        String::new(),
                                        value,
                                        min..=max,
                                        true,
                                    ));
                                    rows += 1;
                                }
                            } else if let Some((asset, controls)) = &self.recipe {
                                for (i, field) in controls.iter().enumerate() {
                                    let c = &field.control;
                                    let label = c.label.clone();
                                    if c.expression.is_some() {
                                        fields.add_item(TheNodeUIItem::Text(
                                            format!("Props Calculated {i}"),
                                            label,
                                            c.help.clone(),
                                            format!("{:.3}", c.value),
                                            None,
                                            false,
                                        ));
                                    } else if c.is_palette() {
                                        fields.add_item(TheNodeUIItem::PaletteSlider(
                                            format!("Props Recipe {asset} {i}"),
                                            label,
                                            c.help.clone(),
                                            c.value as i32,
                                            self.palette.clone(),
                                            false,
                                        ));
                                    } else if c.is_integer() {
                                        fields.add_item(TheNodeUIItem::IntEditSlider(
                                            format!("Props Recipe {asset} {i}"),
                                            label,
                                            c.help.clone(),
                                            c.value as i32,
                                            c.min.ceil() as i32..=c.max.floor() as i32,
                                            false,
                                        ));
                                    } else {
                                        fields.add_item(TheNodeUIItem::FloatEditSlider(
                                            format!("Props Recipe {asset} {i}"),
                                            label,
                                            c.help.clone(),
                                            c.value,
                                            c.min..=c.max,
                                            false,
                                        ));
                                    }
                                    rows += 1;
                                }
                            }
                            let mut layout =
                                TheTextLayout::new(TheId::named("Props Overlay Settings"));
                            layout.set_margin(Vec4::new(10, 10, 10, 10));
                            layout.set_text_margin(8);
                            layout.set_fixed_text_width(180);
                            fields.apply_to_text_layout(&mut layout);
                            let mut canvas = TheCanvas::new();
                            canvas
                                .limiter_mut()
                                .set_max_size(Vec2::new(460, (rows * 40 + 20).clamp(70, 420)));
                            canvas.set_layout(layout);
                            if let Some(view) = crate::utils::map_editor_render_view(ui, server) {
                                let dim = *view.dim();
                                ui.show_popover(
                                    TheId::named("Props Overlay Settings"),
                                    TheDim::rect(
                                        dim.x + rect.x,
                                        dim.y + rect.y,
                                        rect.width,
                                        rect.height,
                                    ),
                                    canvas,
                                    ctx,
                                );
                                self.refresh_fields(ui);
                            }
                        }
                    }
                }
                ctx.ui.redraw_all = true;
                true
            }
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
