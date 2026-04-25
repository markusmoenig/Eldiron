use crate::prelude::*;
use rusterix::DungeonTileKind;
use rusterix::Value;
use rusterix::map::dungeon::DungeonDoorOpenMode;
use rusterix::rebuild_generated_geometry;

const DUNGEON_VIEW_ID: &str = "Dungeon Dock View";
const DUNGEON_SETTINGS_TOML: &str = "Dungeon Dock Settings TOML";
const DUNGEON_CELL_MIN: i32 = 72;
const DUNGEON_CELL_MAX: i32 = 132;
const DUNGEON_GAP: i32 = 12;
const DUNGEON_MARGIN: i32 = 12;
const DUNGEON_SETTINGS_WIDTH: i32 = 280;

#[derive(Clone, Copy)]
struct DungeonPalettePlacement {
    kind: DungeonTileKind,
    rect: Vec4<i32>,
}

pub struct DungeonDock {
    placements: Vec<DungeonPalettePlacement>,
    hovered: Option<DungeonTileKind>,
    scroll_y: i32,
    content_height: i32,
}

impl DungeonDock {
    fn default_render_toml() -> String {
        "[render]\ntransition_seconds = 1.0\nsun_enabled = false\nshadow_enabled = true\nfog_density = 5.0\nfog_color = \"#000000\"\n".to_string()
    }

    fn legacy_render_toml(map: &Map, server_ctx: &ServerContext) -> String {
        format!(
            "[render]\ntransition_seconds = {}\nsun_enabled = {}\nshadow_enabled = {}\nfog_density = {}\nfog_color = \"{}\"\n",
            map.properties.get_float_default(
                "dungeon_render_transition_seconds",
                server_ctx.curr_dungeon_render_transition_seconds,
            ),
            map.properties.get_bool_default(
                "dungeon_render_sun_enabled",
                server_ctx.curr_dungeon_render_sun_enabled,
            ),
            map.properties.get_bool_default(
                "dungeon_render_shadow_enabled",
                server_ctx.curr_dungeon_render_shadow_enabled,
            ),
            map.properties.get_float_default(
                "dungeon_render_fog_density",
                server_ctx.curr_dungeon_render_fog_density,
            ),
            map.properties.get_str_default(
                "dungeon_render_fog_color",
                server_ctx.curr_dungeon_render_fog_color.clone(),
            ),
        )
    }

    fn load_settings_from_map(project: &Project, server_ctx: &mut ServerContext) {
        if let Some(map) = project.get_map(server_ctx) {
            server_ctx.curr_dungeon_floor_base = map
                .properties
                .get_float_default("dungeon_floor_base", server_ctx.curr_dungeon_floor_base);
            server_ctx.curr_dungeon_height = map
                .properties
                .get_float_default("dungeon_height", server_ctx.curr_dungeon_height)
                .max(0.1);
            server_ctx.curr_dungeon_create_floor = map
                .properties
                .get_bool_default("dungeon_create_floor", server_ctx.curr_dungeon_create_floor);
            server_ctx.curr_dungeon_create_ceiling = map.properties.get_bool_default(
                "dungeon_create_ceiling",
                server_ctx.curr_dungeon_create_ceiling,
            );
            server_ctx.curr_dungeon_standalone = map.properties.get_bool_default(
                "dungeon_standalone_default",
                server_ctx.curr_dungeon_standalone,
            );
            server_ctx.curr_dungeon_tile_span = map
                .properties
                .get_int_default("dungeon_tile_span", server_ctx.curr_dungeon_tile_span)
                .max(1);
            server_ctx.curr_dungeon_tile_depth = map
                .properties
                .get_float_default("dungeon_tile_depth", server_ctx.curr_dungeon_tile_depth)
                .max(0.05);
            server_ctx.curr_dungeon_tile_height = map
                .properties
                .get_float_default("dungeon_tile_height", server_ctx.curr_dungeon_tile_height)
                .max(0.5);
            server_ctx.curr_dungeon_tile_open_mode = map.properties.get_int_default(
                "dungeon_tile_open_mode",
                server_ctx.curr_dungeon_tile_open_mode,
            );
            server_ctx.curr_dungeon_tile_item = map.properties.get_str_default(
                "dungeon_tile_item",
                server_ctx.curr_dungeon_tile_item.clone(),
            );
            let legacy_target = map
                .properties
                .get_float_default("dungeon_stair_target_floor_base", f32::NAN);
            server_ctx.curr_dungeon_stair_target_floor_base = map.properties.get_float_default(
                "dungeon_stair_delta",
                if legacy_target.is_finite() {
                    legacy_target - server_ctx.curr_dungeon_floor_base
                } else {
                    server_ctx.curr_dungeon_stair_target_floor_base
                },
            );
            server_ctx.curr_dungeon_stair_steps = map
                .properties
                .get_int_default("dungeon_stair_steps", server_ctx.curr_dungeon_stair_steps)
                .max(1);
            server_ctx.curr_dungeon_stair_tile_id = map.properties.get_str_default(
                "dungeon_stair_tile_id",
                server_ctx.curr_dungeon_stair_tile_id.clone(),
            );
            server_ctx.curr_dungeon_stair_tile_mode = map.properties.get_int_default(
                "dungeon_stair_tile_mode",
                server_ctx.curr_dungeon_stair_tile_mode,
            );
            server_ctx.curr_dungeon_render_toml =
                if let Some(render_toml) = map.properties.get_str("dungeon_render_toml") {
                    render_toml.to_string()
                } else if map
                    .properties
                    .get("dungeon_render_transition_seconds")
                    .is_some()
                    || map.properties.get("dungeon_render_sun_enabled").is_some()
                    || map
                        .properties
                        .get("dungeon_render_shadow_enabled")
                        .is_some()
                    || map.properties.get("dungeon_render_fog_density").is_some()
                    || map.properties.get("dungeon_render_fog_color").is_some()
                {
                    Self::legacy_render_toml(map, server_ctx)
                } else {
                    server_ctx.curr_dungeon_render_toml.clone()
                };
        }
    }

    fn store_settings_to_map(project: &mut Project, server_ctx: &ServerContext) {
        if let Some(map) = project.get_map_mut(server_ctx) {
            map.properties.set(
                "dungeon_floor_base",
                Value::Float(server_ctx.curr_dungeon_floor_base),
            );
            map.properties.set(
                "dungeon_height",
                Value::Float(server_ctx.curr_dungeon_height),
            );
            map.properties.set(
                "dungeon_create_floor",
                Value::Bool(server_ctx.curr_dungeon_create_floor),
            );
            map.properties.set(
                "dungeon_create_ceiling",
                Value::Bool(server_ctx.curr_dungeon_create_ceiling),
            );
            map.properties.set(
                "dungeon_standalone_default",
                Value::Bool(server_ctx.curr_dungeon_standalone),
            );
            map.properties.set(
                "dungeon_tile_span",
                Value::Int(server_ctx.curr_dungeon_tile_span.max(1)),
            );
            map.properties.set(
                "dungeon_tile_depth",
                Value::Float(server_ctx.curr_dungeon_tile_depth.max(0.05)),
            );
            map.properties.set(
                "dungeon_tile_height",
                Value::Float(server_ctx.curr_dungeon_tile_height.max(0.5)),
            );
            map.properties.set(
                "dungeon_tile_open_mode",
                Value::Int(server_ctx.curr_dungeon_tile_open_mode),
            );
            map.properties.set(
                "dungeon_tile_item",
                Value::Str(server_ctx.curr_dungeon_tile_item.clone()),
            );
            map.properties.set(
                "dungeon_stair_delta",
                Value::Float(server_ctx.curr_dungeon_stair_target_floor_base),
            );
            map.properties.set(
                "dungeon_stair_steps",
                Value::Int(server_ctx.curr_dungeon_stair_steps.max(1)),
            );
            map.properties.set(
                "dungeon_stair_tile_id",
                Value::Str(server_ctx.curr_dungeon_stair_tile_id.clone()),
            );
            map.properties.set(
                "dungeon_stair_tile_mode",
                Value::Int(server_ctx.curr_dungeon_stair_tile_mode),
            );
            map.properties.set(
                "dungeon_render_toml",
                Value::Str(server_ctx.curr_dungeon_render_toml.clone()),
            );
        }
    }

    fn settings_nodeui(server_ctx: &ServerContext) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::OpenTree("Dungeon".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "Floor Base".into(),
            "Floor Base".into(),
            "Default floor base for newly painted dungeon cells.".into(),
            server_ctx.curr_dungeon_floor_base,
            -64.0..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "Height".into(),
            "Height".into(),
            "Wall and ceiling height above the floor base for newly painted cells.".into(),
            server_ctx.curr_dungeon_height,
            0.1..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "Floors".into(),
            "Floors".into(),
            "Generate floor surfaces for conceptual dungeon tiles.".into(),
            server_ctx.curr_dungeon_create_floor,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "Ceilings".into(),
            "Ceilings".into(),
            "Generate ceiling surfaces for conceptual dungeon tiles.".into(),
            server_ctx.curr_dungeon_create_ceiling,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "Standalone".into(),
            "Standalone".into(),
            "Newly painted cells stay standalone and will not merge with adjacent dungeon geometry.".into(),
            server_ctx.curr_dungeon_standalone,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);
        if server_ctx.curr_dungeon_tile.is_door() {
            nodeui.add_item(TheNodeUIItem::OpenTree("Tile".into()));
            nodeui.add_item(TheNodeUIItem::IntEditSlider(
                "Door Width".into(),
                "Door Width".into(),
                "Door opening span in tiles for newly painted door tiles.".into(),
                server_ctx.curr_dungeon_tile_span.max(1),
                1..=8,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "Door Depth".into(),
                "Door Depth".into(),
                "Conceptual door depth hint for newly painted door tiles.".into(),
                server_ctx.curr_dungeon_tile_depth.max(0.05),
                0.05..=4.0,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "Door Height".into(),
                "Door Height".into(),
                "Door opening height for newly painted door tiles.".into(),
                server_ctx.curr_dungeon_tile_height.max(0.5),
                0.5..=8.0,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::Selector(
                "Door Open Mode".into(),
                "Open Mode".into(),
                "How the generated door should open in gameplay.".into(),
                DungeonDoorOpenMode::all()
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
                server_ctx.curr_dungeon_tile_open_mode,
            ));
            nodeui.add_item(TheNodeUIItem::Text(
                "Item".into(),
                "Item".into(),
                "Item handler class written into generated dungeon door sectors.".into(),
                server_ctx.curr_dungeon_tile_item.clone(),
                Some("Door Handler".into()),
                false,
            ));
            nodeui.add_item(TheNodeUIItem::CloseTree);
        } else if server_ctx.curr_dungeon_tile.is_stair() {
            nodeui.add_item(TheNodeUIItem::OpenTree("Steps".into()));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "steps_floor_delta".into(),
                "Floor Delta".into(),
                "Relative floor-base change reached by the stair tile. Negative goes down.".into(),
                server_ctx.curr_dungeon_stair_target_floor_base,
                -64.0..=64.0,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::IntEditSlider(
                "steps_steps".into(),
                "Steps".into(),
                "Number of generated steps inside the stair tile.".into(),
                server_ctx.curr_dungeon_stair_steps.max(1),
                1..=16,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::Selector(
                "steps_tile_mode".into(),
                "Tile Mode".into(),
                "How the stair tile source is mapped onto generated stair geometry.".into(),
                vec!["Repeat".into(), "Scale".into()],
                if server_ctx.curr_dungeon_stair_tile_mode == 1 {
                    0
                } else {
                    1
                },
            ));
            nodeui.add_item(TheNodeUIItem::Text(
                "steps_tile_id".into(),
                "Tile Id".into(),
                "Default tile id applied to generated stair geometry.".into(),
                server_ctx.curr_dungeon_stair_tile_id.clone(),
                Some(String::new()),
                false,
            ));
            nodeui.add_item(TheNodeUIItem::CloseTree);
        }
        nodeui
    }

    fn settings_toml(server_ctx: &ServerContext) -> String {
        let mut text = nodeui_to_toml(&Self::settings_nodeui(server_ctx))
            .trim_end()
            .to_string();
        let render = if server_ctx.curr_dungeon_render_toml.trim().is_empty() {
            Self::default_render_toml()
        } else {
            server_ctx.curr_dungeon_render_toml.trim().to_string()
        };
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&render);
        text.push('\n');
        text
    }

    fn sync_settings_ui(&self, ui: &mut TheUI, _ctx: &mut TheContext, server_ctx: &ServerContext) {
        if let Some(widget) = ui.get_widget(DUNGEON_SETTINGS_TOML)
            && let Some(edit) = widget.as_text_area_edit()
        {
            let toml_text = Self::settings_toml(server_ctx);
            if edit.text() != toml_text {
                let previous = edit.get_state();
                edit.set_text(toml_text);

                let mut state = edit.get_state();
                let row_max = state.rows.len().saturating_sub(1);
                let row = previous.cursor.row.min(row_max);
                let col_max = state
                    .rows
                    .get(row)
                    .map(|line| line.chars().count())
                    .unwrap_or(0);

                state.cursor.row = row;
                state.cursor.column = previous.cursor.column.min(col_max);
                state.selection.reset();
                TheTextAreaEditTrait::set_state(edit, state);
            }
        }
    }

    fn clip_rect(
        buffer: &TheRGBABuffer,
        rect: Vec4<i32>,
        inset: i32,
    ) -> Option<(usize, usize, usize, usize)> {
        let x0 = (rect.x + inset).clamp(0, buffer.dim().width);
        let y0 = (rect.y + inset).clamp(0, buffer.dim().height);
        let x1 = (rect.x + rect.z - inset).clamp(0, buffer.dim().width);
        let y1 = (rect.y + rect.w - inset).clamp(0, buffer.dim().height);

        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        Some((
            x0 as usize,
            y0 as usize,
            (x1 - x0) as usize,
            (y1 - y0) as usize,
        ))
    }

    fn palette() -> &'static [DungeonTileKind] {
        DungeonTileKind::all()
    }

    fn draw_panel_rect(
        draw: &TheDraw2D,
        buffer: &mut TheRGBABuffer,
        rect: Vec4<i32>,
        color: &[u8; 4],
    ) {
        if let Some(clipped) = Self::clip_rect(buffer, rect, 0) {
            let stride = buffer.stride();
            draw.rect(buffer.pixels_mut(), &clipped, stride, color);
        }
    }

    fn draw_preview(
        &self,
        draw: &TheDraw2D,
        buffer: &mut TheRGBABuffer,
        rect: Vec4<i32>,
        kind: DungeonTileKind,
        selected: bool,
        hovered: bool,
    ) {
        let stride = buffer.stride();
        let card_color = if selected {
            [64, 76, 92, 255]
        } else if hovered {
            [48, 54, 64, 255]
        } else {
            [36, 40, 48, 255]
        };
        if let Some(inner) = Self::clip_rect(buffer, rect, 0) {
            draw.rect(buffer.pixels_mut(), &inner, stride, &card_color);
        }

        let outline = if selected {
            [255, 255, 255, 255]
        } else if hovered {
            [210, 210, 210, 255]
        } else {
            [84, 84, 84, 255]
        };
        if let Some(inner) = Self::clip_rect(buffer, rect, 0) {
            draw.rect_outline(buffer.pixels_mut(), &inner, stride, &outline);
        }

        let preview = Vec4::new(rect.x + 10, rect.y + 10, rect.z - 20, rect.w - 20);
        let floor_color = [150, 150, 150, 255];
        let floor_outline = [205, 205, 205, 255];
        let wall_color = [255, 255, 255, 255];
        let door_color = [240, 196, 92, 255];
        let stair_color = [182, 220, 255, 255];
        let wall = (preview.z.min(preview.w) / 8).clamp(4, 8);
        let floor_rect = preview;

        Self::draw_panel_rect(draw, buffer, floor_rect, &floor_color);
        if let Some(inner) = Self::clip_rect(buffer, floor_rect, 0) {
            draw.rect_outline(buffer.pixels_mut(), &inner, stride, &floor_outline);
        }
        if kind.has_north() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(preview.x, preview.y, preview.z, wall),
                &wall_color,
            );
        }
        if kind.has_south() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(preview.x, preview.y + preview.w - wall, preview.z, wall),
                &wall_color,
            );
        }
        if kind.has_west() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(preview.x, preview.y, wall, preview.w),
                &wall_color,
            );
        }
        if kind.has_east() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(preview.x + preview.z - wall, preview.y, wall, preview.w),
                &wall_color,
            );
        }
        if kind.has_door_north() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(preview.x + preview.z / 4, preview.y, preview.z / 2, wall),
                &door_color,
            );
        }
        if kind.has_door_south() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(
                    preview.x + preview.z / 4,
                    preview.y + preview.w - wall,
                    preview.z / 2,
                    wall,
                ),
                &door_color,
            );
        }
        if kind.has_door_west() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(preview.x, preview.y + preview.w / 4, wall, preview.w / 2),
                &door_color,
            );
        }
        if kind.has_door_east() {
            Self::draw_panel_rect(
                draw,
                buffer,
                Vec4::new(
                    preview.x + preview.z - wall,
                    preview.y + preview.w / 4,
                    wall,
                    preview.w / 2,
                ),
                &door_color,
            );
        }
        if kind.is_stair() {
            let bands = 4;
            let band_gap = (preview.z.min(preview.w) / 20).max(2);
            for i in 0..bands {
                let t = i as i32;
                if kind.has_stair_north() {
                    let y = preview.y + preview.w - ((t + 1) * preview.w / (bands + 1));
                    Self::draw_panel_rect(
                        draw,
                        buffer,
                        Vec4::new(
                            preview.x + band_gap,
                            y,
                            preview.z - band_gap * 2,
                            wall.max(3),
                        ),
                        &stair_color,
                    );
                } else if kind.has_stair_south() {
                    let y = preview.y + (t + 1) * preview.w / (bands + 1);
                    Self::draw_panel_rect(
                        draw,
                        buffer,
                        Vec4::new(
                            preview.x + band_gap,
                            y,
                            preview.z - band_gap * 2,
                            wall.max(3),
                        ),
                        &stair_color,
                    );
                } else if kind.has_stair_east() {
                    let x = preview.x + (t + 1) * preview.z / (bands + 1);
                    Self::draw_panel_rect(
                        draw,
                        buffer,
                        Vec4::new(
                            x,
                            preview.y + band_gap,
                            wall.max(3),
                            preview.w - band_gap * 2,
                        ),
                        &stair_color,
                    );
                } else if kind.has_stair_west() {
                    let x = preview.x + preview.z - ((t + 1) * preview.z / (bands + 1));
                    Self::draw_panel_rect(
                        draw,
                        buffer,
                        Vec4::new(
                            x,
                            preview.y + band_gap,
                            wall.max(3),
                            preview.w - band_gap * 2,
                        ),
                        &stair_color,
                    );
                }
            }
        }
    }

    fn render_palette(&mut self, ui: &mut TheUI, ctx: &mut TheContext, server_ctx: &ServerContext) {
        let Some(render_view) = ui.get_render_view(DUNGEON_VIEW_ID) else {
            return;
        };
        let dim = *render_view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return;
        }
        if dim.width < 120 || dim.height < 80 {
            *render_view.render_buffer_mut() =
                TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
            render_view.render_buffer_mut().fill(BLACK);
            render_view.set_needs_redraw(true);
            ctx.ui.redraw_all = true;
            return;
        }

        *render_view.render_buffer_mut() =
            TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
        let buffer = render_view.render_buffer_mut();
        buffer.fill(BLACK);
        self.placements.clear();
        self.content_height = 0;

        let palette = Self::palette();
        let cols = ((dim.width - DUNGEON_MARGIN * 2 + DUNGEON_GAP)
            / (DUNGEON_CELL_MIN + DUNGEON_GAP))
            .max(1)
            .min(palette.len() as i32);
        let cell = ((dim.width - DUNGEON_MARGIN * 2 - (cols - 1) * DUNGEON_GAP) / cols)
            .clamp(DUNGEON_CELL_MIN, DUNGEON_CELL_MAX);
        for (index, kind) in palette.iter().copied().enumerate() {
            let col = index as i32 % cols;
            let row = index as i32 / cols;
            let rect = Vec4::new(
                DUNGEON_MARGIN + col * (cell + DUNGEON_GAP),
                DUNGEON_MARGIN + row * (cell + DUNGEON_GAP) - self.scroll_y,
                cell,
                cell,
            );
            self.placements.push(DungeonPalettePlacement { kind, rect });
            self.content_height = self
                .content_height
                .max(DUNGEON_MARGIN * 2 + (row + 1) * cell + row * DUNGEON_GAP);

            if rect.x >= dim.width
                || rect.y >= dim.height
                || rect.x + rect.z <= 0
                || rect.y + rect.w <= 0
            {
                continue;
            }

            self.draw_preview(
                &ctx.draw,
                buffer,
                rect,
                kind,
                server_ctx.curr_dungeon_tile == kind,
                self.hovered == Some(kind),
            );
        }

        render_view.set_needs_redraw(true);
        ctx.ui.redraw_all = true;
    }

    fn kind_at(&self, coord: Vec2<i32>) -> Option<DungeonTileKind> {
        self.placements
            .iter()
            .find(|placement| {
                coord.x >= placement.rect.x
                    && coord.x < placement.rect.x + placement.rect.z
                    && coord.y >= placement.rect.y
                    && coord.y < placement.rect.y + placement.rect.w
            })
            .map(|placement| placement.kind)
    }
}

impl Dock for DungeonDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            placements: Vec::new(),
            hovered: None,
            scroll_y: 0,
            content_height: 0,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut title = TheText::new(TheId::named("Dungeon Dock Title"));
        title.set_text("Dungeon".to_string());
        title.set_text_size(12.5);
        toolbar_hlayout.add_widget(Box::new(title));

        let spacer = TheSpacer::new(TheId::empty());
        toolbar_hlayout.add_widget(Box::new(spacer));
        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut center = TheCanvas::new();
        let mut palette_canvas = TheCanvas::new();
        palette_canvas.set_widget(TheRenderView::new(TheId::named(DUNGEON_VIEW_ID)));
        center.set_center(palette_canvas);

        let mut settings_canvas = TheCanvas::new();
        let mut textedit = TheTextAreaEdit::new(TheId::named(DUNGEON_SETTINGS_TOML));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_syntax_from_string(source);
            textedit.set_code_type("TOML");
        }
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_theme_from_string(source);
            textedit.set_code_theme("Gruvbox Dark");
        }
        textedit.set_continuous(true);
        textedit.display_line_number(false);
        textedit.use_global_statusbar(true);
        textedit.set_font_size(13.5);
        textedit.limiter_mut().set_max_width(DUNGEON_SETTINGS_WIDTH);
        textedit.limiter_mut().set_min_width(DUNGEON_SETTINGS_WIDTH);
        settings_canvas.set_widget(textedit);
        center.set_right(settings_canvas);
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
        Self::load_settings_from_map(project, server_ctx);
        self.sync_settings_ui(ui, ctx, server_ctx);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Render Dungeon Palette"),
            TheValue::Empty,
        ));
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::Custom(id, _) if id.name == "Render Dungeon Palette" => {
                self.render_palette(ui, ctx, server_ctx);
                redraw = true;
            }
            TheEvent::Resize => {
                let view_height = ui
                    .get_render_view(DUNGEON_VIEW_ID)
                    .map(|view| view.dim().height)
                    .unwrap_or(0);
                let max_scroll = (self.content_height - view_height).max(0);
                self.scroll_y = self.scroll_y.clamp(0, max_scroll);
                self.render_palette(ui, ctx, server_ctx);
                redraw = true;
            }
            TheEvent::RenderViewHoverChanged(id, coord) if id.name == DUNGEON_VIEW_ID => {
                self.hovered = self.kind_at(*coord);
                if let Some(kind) = self.hovered {
                    ctx.ui.send(TheEvent::SetStatusText(
                        id.clone(),
                        kind.label().to_string(),
                    ));
                }
                self.render_palette(ui, ctx, server_ctx);
                redraw = true;
            }
            TheEvent::RenderViewLostHover(id) if id.name == DUNGEON_VIEW_ID => {
                self.hovered = None;
                self.render_palette(ui, ctx, server_ctx);
                redraw = true;
            }
            TheEvent::RenderViewClicked(id, coord) if id.name == DUNGEON_VIEW_ID => {
                if let Some(kind) = self.kind_at(*coord) {
                    server_ctx.curr_dungeon_tile = kind;
                    self.sync_settings_ui(ui, ctx, server_ctx);
                    self.render_palette(ui, ctx, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::RenderViewScrollBy(id, delta) if id.name == DUNGEON_VIEW_ID => {
                if !ui.ctrl && !ui.logo {
                    let view_height = ui
                        .get_render_view(DUNGEON_VIEW_ID)
                        .map(|view| view.dim().height)
                        .unwrap_or(0);
                    let max_scroll = (self.content_height - view_height).max(0);
                    self.scroll_y = (self.scroll_y + delta.y).clamp(0, max_scroll);
                    self.render_palette(ui, ctx, server_ctx);
                    if let Some(render_view) = ui.get_render_view(DUNGEON_VIEW_ID) {
                        render_view.set_needs_redraw(true);
                    }
                    ctx.ui.redraw_all = true;
                    redraw = true;
                }
            }
            TheEvent::ValueChanged(id, value) if id.name == DUNGEON_SETTINGS_TOML => {
                if let Some(source) = value.to_string() {
                    let mut nodeui = Self::settings_nodeui(server_ctx);
                    let doc = toml::from_str::<toml::Value>(&source);
                    if doc.is_ok() && apply_toml_to_nodeui(&mut nodeui, &source).is_ok() {
                        let mut rebuild_geometry = false;
                        let mut refresh_preview = false;
                        for (key, val) in nodeui_to_value_pairs(&nodeui) {
                            match (key.as_str(), val) {
                                ("Floor Base", TheValue::Float(v)) => {
                                    refresh_preview |=
                                        (server_ctx.curr_dungeon_floor_base - v).abs() > 0.0001;
                                    server_ctx.curr_dungeon_floor_base = v;
                                }
                                ("Height", TheValue::Float(v)) => {
                                    let v = v.max(0.1);
                                    refresh_preview |=
                                        (server_ctx.curr_dungeon_height - v).abs() > 0.0001;
                                    server_ctx.curr_dungeon_height = v;
                                }
                                ("Floors", TheValue::Bool(v)) => {
                                    rebuild_geometry |= server_ctx.curr_dungeon_create_floor != v;
                                    server_ctx.curr_dungeon_create_floor = v;
                                }
                                ("Ceilings", TheValue::Bool(v)) => {
                                    rebuild_geometry |= server_ctx.curr_dungeon_create_ceiling != v;
                                    server_ctx.curr_dungeon_create_ceiling = v;
                                }
                                ("Standalone", TheValue::Bool(v)) => {
                                    server_ctx.curr_dungeon_standalone = v;
                                }
                                ("Door Width", TheValue::Int(v)) => {
                                    server_ctx.curr_dungeon_tile_span = v.max(1);
                                }
                                ("Door Depth", TheValue::Float(v)) => {
                                    server_ctx.curr_dungeon_tile_depth = v.max(0.05);
                                }
                                ("Door Height", TheValue::Float(v)) => {
                                    server_ctx.curr_dungeon_tile_height = v.max(0.5);
                                }
                                ("Door Open Mode", TheValue::Int(v)) => {
                                    server_ctx.curr_dungeon_tile_open_mode = v.clamp(0, 5);
                                }
                                ("steps_floor_delta", TheValue::Float(v)) => {
                                    server_ctx.curr_dungeon_stair_target_floor_base = v;
                                }
                                ("steps_steps", TheValue::Int(v)) => {
                                    server_ctx.curr_dungeon_stair_steps = v.max(1);
                                }
                                ("steps_tile_mode", TheValue::Int(v)) => {
                                    server_ctx.curr_dungeon_stair_tile_mode =
                                        if v == 0 { 1 } else { 0 };
                                }
                                _ => {}
                            }
                        }
                        if server_ctx.curr_dungeon_tile.is_stair()
                            && let Some(tile_id) = nodeui.get_text_value("steps_tile_id")
                        {
                            server_ctx.curr_dungeon_stair_tile_id = tile_id.trim().to_string();
                        }
                        if let Some(item) = nodeui.get_text_value("Item") {
                            server_ctx.curr_dungeon_tile_item = item.trim().to_string();
                        }
                        if let Ok(doc) = doc
                            && let Some(render_table) =
                                doc.get("render").and_then(toml::Value::as_table)
                        {
                            let render_body =
                                toml::to_string_pretty(render_table).unwrap_or_default();
                            server_ctx.curr_dungeon_render_toml =
                                format!("[render]\n{}", render_body);
                        }

                        Self::store_settings_to_map(_project, server_ctx);

                        if refresh_preview {
                            let layer = _project
                                .get_map_mut(server_ctx)
                                .map(|map| map.dungeon.ensure_active_layer_mut());
                            if let Some(layer) = layer {
                                layer.floor_base = server_ctx.curr_dungeon_floor_base;
                                layer.height = server_ctx.curr_dungeon_height;
                            }
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            ctx.ui.redraw_all = true;
                            redraw = true;
                        }

                        if rebuild_geometry && let Some(map) = _project.get_map_mut(server_ctx) {
                            rebuild_generated_geometry(
                                map,
                                server_ctx.curr_dungeon_create_floor,
                                server_ctx.curr_dungeon_create_ceiling,
                            );
                            map.changed += 1;
                            crate::utils::scenemanager_render_map(_project, server_ctx);
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            redraw = true;
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    fn supports_actions(&self) -> bool {
        false
    }
}
