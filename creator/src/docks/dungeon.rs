use crate::prelude::*;
use rusterix::DungeonTileKind;
use rusterix::rebuild_generated_geometry;

const DUNGEON_VIEW_ID: &str = "Dungeon Dock View";
const DUNGEON_SETTINGS_TOML: &str = "Dungeon Dock Settings TOML";
const DUNGEON_CELL_MIN: i32 = 72;
const DUNGEON_CELL_MAX: i32 = 132;
const DUNGEON_GAP: i32 = 12;
const DUNGEON_MARGIN: i32 = 12;

#[derive(Clone, Copy)]
struct DungeonPalettePlacement {
    kind: DungeonTileKind,
    rect: Vec4<i32>,
}

pub struct DungeonDock {
    placements: Vec<DungeonPalettePlacement>,
    hovered: Option<DungeonTileKind>,
}

impl DungeonDock {
    fn settings_nodeui(server_ctx: &ServerContext) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::OpenTree("Dungeon".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "Dungeon Floor Base".into(),
            "Floor Base".into(),
            "Default floor base for newly painted dungeon cells.".into(),
            server_ctx.curr_dungeon_floor_base,
            -64.0..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "Dungeon Height".into(),
            "Height".into(),
            "Wall and ceiling height above the floor base for newly painted cells.".into(),
            server_ctx.curr_dungeon_height,
            0.1..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "Dungeon Floors".into(),
            "Floors".into(),
            "Generate floor surfaces for conceptual dungeon tiles.".into(),
            server_ctx.curr_dungeon_create_floor,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "Dungeon Ceilings".into(),
            "Ceilings".into(),
            "Generate ceiling surfaces for conceptual dungeon tiles.".into(),
            server_ctx.curr_dungeon_create_ceiling,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);
        nodeui
    }

    fn sync_settings_ui(&self, ui: &mut TheUI, _ctx: &mut TheContext, server_ctx: &ServerContext) {
        if let Some(widget) = ui.get_widget(DUNGEON_SETTINGS_TOML)
            && let Some(edit) = widget.as_text_area_edit()
        {
            let toml_text = nodeui_to_toml(&Self::settings_nodeui(server_ctx));
            if edit.text() != toml_text {
                edit.set_text(toml_text);
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
                DUNGEON_MARGIN + row * (cell + DUNGEON_GAP),
                cell,
                cell,
            );
            self.placements.push(DungeonPalettePlacement { kind, rect });

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
        settings_canvas.set_widget(textedit);
        center.set_right(settings_canvas);
        canvas.set_center(center);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &Project,
        server_ctx: &mut ServerContext,
    ) {
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
                    self.render_palette(ui, ctx, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::ValueChanged(id, value) if id.name == DUNGEON_SETTINGS_TOML => {
                if let Some(source) = value.to_string() {
                    let mut nodeui = Self::settings_nodeui(server_ctx);
                    if apply_toml_to_nodeui(&mut nodeui, &source).is_ok() {
                        let mut rebuild_geometry = false;
                        for (key, val) in nodeui_to_value_pairs(&nodeui) {
                            match (key.as_str(), val) {
                                ("Dungeon Floor Base", TheValue::Float(v)) => {
                                    server_ctx.curr_dungeon_floor_base = v;
                                }
                                ("Dungeon Height", TheValue::Float(v)) => {
                                    server_ctx.curr_dungeon_height = v.max(0.1);
                                }
                                ("Dungeon Floors", TheValue::Bool(v)) => {
                                    rebuild_geometry |= server_ctx.curr_dungeon_create_floor != v;
                                    server_ctx.curr_dungeon_create_floor = v;
                                }
                                ("Dungeon Ceilings", TheValue::Bool(v)) => {
                                    rebuild_geometry |= server_ctx.curr_dungeon_create_ceiling != v;
                                    server_ctx.curr_dungeon_create_ceiling = v;
                                }
                                _ => {}
                            }
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
