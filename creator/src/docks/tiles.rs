use crate::prelude::*;
use rusterix::{TileRole, VertexBlendPreset};

pub struct TilesDock {
    pub tile_ids: FxHashMap<(i32, i32), Uuid>,

    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,

    pub curr_tile: Option<Uuid>,

    pub tile_preview_mode: bool,
    pub tile_hover_id: Uuid,

    blend_index: usize,
}

impl Dock for TilesDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            tile_ids: FxHashMap::default(),
            filter: "".to_string(),
            filter_role: 0,
            zoom: 1.5,
            curr_tile: None,

            tile_preview_mode: false,
            tile_hover_id: Uuid::nil(),

            blend_index: 0,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text(fl!("filter"));

        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Tiles Dock Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(Vec2::new(120, 18));
        filter_edit.set_font_size(12.5);
        // filter_edit.set_embedded(true);
        filter_edit.set_status_text(&fl!("status_tiles_filter_edit"));
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Tiles Dock Filter Role"));
        drop_down.add_option(fl!("all"));
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_width(10);
        toolbar_hlayout.add_widget(Box::new(spacer));

        let mut zoom = TheSlider::new(TheId::named("Tiles Dock Zoom"));
        zoom.set_value(TheValue::Float(self.zoom));
        zoom.set_default_value(TheValue::Float(1.5));
        zoom.set_range(TheValue::RangeF32(1.0..=3.0));
        zoom.set_continuous(true);
        zoom.limiter_mut().set_max_width(120);
        toolbar_hlayout.add_widget(Box::new(zoom));
        toolbar_hlayout.set_reverse_index(Some(1));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut rgba_layout = TheRGBALayout::new(TheId::named("Tiles Dock RGBA Layout"));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_supports_external_zoom(true);
            rgba_view.set_background([116, 116, 116, 255]);
            rgba_view.set_grid(Some(24));
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }

        // Bottom toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let size = 24;
        for (index, p) in VertexBlendPreset::ALL.iter().enumerate() {
            let weights = p.weights();
            let buffer = p.preview_vertex_blend(weights, size);
            let rgba = TheRGBABuffer::from(buffer, size as u32, size as u32);
            let mut view = TheIconView::new(TheId::named(&format!("Blend #{}", index)));
            view.set_rgba_tile(TheRGBATile::buffer(rgba));
            if index == 0 {
                view.set_border_color(Some(WHITE));
            }
            toolbar_hlayout.add_widget(Box::new(view));

            if index == 2 || index == 6 || index == 10 || index == 14 {
                let mut spacer = TheSpacer::new(TheId::empty());
                spacer.limiter_mut().set_max_width(4);
                toolbar_hlayout.add_widget(Box::new(spacer));
            }
        }

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_bottom(toolbar_canvas);

        // ---

        canvas.set_layout(rgba_layout);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        self.set_tiles(&project.tiles, ui, ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::WidgetResized(id, _) => {
                if id.name == "Tiles Dock RGBA Layout View" {
                    self.set_tiles(&project.tiles, ui, ctx);
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name.starts_with("Blend #") {
                    if let Ok(index) = id.name.strip_prefix("Blend #").unwrap().parse::<usize>() {
                        if let Some(old_icon) =
                            ui.get_icon_view(&format!("Blend #{}", self.blend_index))
                        {
                            old_icon.set_border_color(None);
                        }
                        if let Some(old_icon) = ui.get_icon_view(&format!("Blend #{}", index)) {
                            old_icon.set_border_color(Some(WHITE));
                        }
                        self.blend_index = index;
                        server_ctx.rect_blend_preset = VertexBlendPreset::from_index(index)
                            .unwrap_or(VertexBlendPreset::Solid);
                    }
                } else if id.name == "Tiles Dock Tile Copy" {
                    if let Some(tile_id) = self.curr_tile {
                        let txt = format!("\"{tile_id}\"");
                        ctx.ui.clipboard = Some(TheValue::Text(txt.clone()));
                        let mut clipboard = arboard::Clipboard::new().unwrap();
                        clipboard.set_text(txt.clone()).unwrap();
                    }
                }
            }
            TheEvent::Resize => {
                self.set_tiles(&project.tiles, ui, ctx);
            }
            TheEvent::TileDragStarted(id, pos, _offset) => {
                if id.name == "Tiles Dock RGBA Layout View" {
                    if let Some(tile_id) = self.tile_ids.get(&(pos.x, pos.y)) {
                        if let Some(tile) = project.tiles.get(tile_id) {
                            let mut drop = TheDrop::new(TheId::named_with_id("Tile", *tile_id));
                            if !tile.is_empty() {
                                let b = TheRGBABuffer::from(
                                    tile.textures[0].data.clone(),
                                    tile.textures[0].width as u32,
                                    tile.textures[0].height as u32,
                                );
                                drop.set_image(b.scaled(
                                    (tile.textures[0].width as f32 * self.zoom) as i32,
                                    (tile.textures[0].height as f32 * self.zoom) as i32,
                                ));
                            }
                            ctx.ui.set_drop(drop);
                        }
                    }
                }
            }
            TheEvent::TilePicked(id, pos) => {
                if id.name == "Tiles Dock RGBA Layout View" {
                    if let Some(tile_id) = self.tile_ids.get(&(pos.x, pos.y)) {
                        server_ctx.curr_tile_id = Some(*tile_id);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Tile Picked"),
                            TheValue::Id(*tile_id),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Action List"),
                            TheValue::Empty,
                        ));
                        self.curr_tile = Some(*tile_id);
                        redraw = true;
                    }
                }
            }
            TheEvent::LostHover(id) => {
                if id.name == "Tiles Dock RGBA Layout View" {
                    self.tile_preview_mode = false;
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Soft Update Minimap"),
                        TheValue::Empty,
                    ));
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == "Tiles Dock RGBA Layout View" {
                    if let Some(tile_id) = self.tile_ids.get(&(pos.x, pos.y)) {
                        if let Some(tile) = project.get_tile(tile_id) {
                            if tile.name.is_empty() {
                                let text = format!(
                                    "{}, Blocking: {}",
                                    tile.role.to_string(),
                                    if tile.blocking { "Yes" } else { "No" },
                                );
                                ctx.ui.send(TheEvent::SetStatusText(id.clone(), text));
                            } else {
                                let text = format!(
                                    "{}, Blocking: {}, Tags: \"{}\"",
                                    tile.role.to_string(),
                                    if tile.blocking { "Yes" } else { "No" },
                                    tile.name
                                );
                                ctx.ui.send(TheEvent::SetStatusText(id.clone(), text));
                            }
                        }

                        self.tile_preview_mode = true;
                        self.tile_hover_id = *tile_id;
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Soft Update Minimap"),
                            TheValue::Empty,
                        ));
                    }
                    redraw = true;
                }
            }
            TheEvent::TileEditorDelete(id, selected) => {
                if id.name == "Tiles Dock RGBA Layout View" {
                    for tile_pos in selected {
                        if let Some(tile_id) = self.tile_ids.get(tile_pos) {
                            project.remove_tile(tile_id);
                        }
                    }
                    self.set_tiles(&project.tiles, ui, ctx);
                }
            }
            TheEvent::Custom(id, _value) => {
                if id.name == "Update Tilepicker" {
                    self.set_tiles(&project.tiles, ui, ctx);
                }
            }
            TheEvent::TileZoomBy(id, delta) => {
                if id.name == "Tiles Dock RGBA Layout View" {
                    self.zoom += *delta * 0.05;
                    self.zoom = self.zoom.clamp(1.0, 3.0);
                    self.set_tiles(&project.tiles, ui, ctx);
                    ui.set_widget_value("Tiles Dock Zoom", ctx, TheValue::Float(self.zoom));
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Tiles Dock Tile Role" {
                    if let Some(tile_id) = self.curr_tile {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Int(role) = value {
                                tile.role = TileRole::from_index(*role as u8);
                            }
                        }
                    }
                } else if id.name == "Tiles Dock Tile Tags" {
                    if let Some(tile_id) = self.curr_tile {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Text(tags) = value {
                                tile.name.clone_from(tags);
                            }
                        }
                    }
                } else if id.name == "Tiles Dock Tile Scale" {
                    if let Some(tile_id) = self.curr_tile {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let Some(value) = value.to_f32() {
                                tile.scale = value;
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Tiles"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }
                } else if id.name == "Tiles Dock Tile Blocking" {
                    if let Some(tile_id) = self.curr_tile {
                        if let Some(tile) = project.get_tile_mut(&tile_id) {
                            if let TheValue::Int(role) = value {
                                tile.blocking = *role == 1;
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Tiles"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }
                } else if id.name == "Tiles Dock Filter Edit" {
                    if let TheValue::Text(filter) = value {
                        self.filter = filter.to_lowercase();
                        self.set_tiles(&project.tiles, ui, ctx);
                    }
                } else if id.name == "Tiles Dock Filter Role" {
                    if let TheValue::Int(filter) = value {
                        self.filter_role = *filter as u8;
                        self.set_tiles(&project.tiles, ui, ctx);
                    }
                } else if id.name == "Tiles Dock Zoom" {
                    if let TheValue::Float(zoom) = value {
                        self.zoom = *zoom;
                        self.set_tiles(&project.tiles, ui, ctx);
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        if !self.tile_preview_mode {
            return false;
        }

        buffer.fill(BLACK);

        if let Some(tile) = project.tiles.get(&self.tile_hover_id) {
            let index = server_ctx.animation_counter % tile.textures.len();

            let stride: usize = buffer.stride();

            let src_pixels = &tile.textures[index].data;
            let src_w = tile.textures[index].width as f32;
            let src_h = tile.textures[index].height as f32;

            let dim = buffer.dim();
            let dst_w = dim.width as f32;
            let dst_h = dim.height as f32;

            // Compute scale
            let scale = (dst_w / src_w).min(dst_h / src_h);

            // Scaled dimensions
            let draw_w = src_w * scale;
            let draw_h = src_h * scale;

            // Center
            let offset_x = ((dst_w - draw_w) * 0.5).round() as usize;
            let offset_y = ((dst_h - draw_h) * 0.5).round() as usize;

            let dst_rect = (
                offset_x,
                offset_y,
                draw_w.round() as usize,
                draw_h.round() as usize,
            );

            ctx.draw.blend_scale_chunk(
                buffer.pixels_mut(),
                &dst_rect,
                stride,
                src_pixels,
                &(src_w as usize, src_h as usize),
            );

            return true;
        }

        false
    }

    fn supports_minimap_animation(&self) -> bool {
        true
    }
}

impl TilesDock {
    /// Set the tiles for the picker.
    pub fn set_tiles(
        &mut self,
        tiles: &IndexMap<Uuid, rusterix::Tile>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.tile_ids.clear();
        if let Some(editor) = ui.get_rgba_layout("Tiles Dock RGBA Layout") {
            let width = editor.dim().width - 16;
            let height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let grid = (24_f32 * self.zoom) as i32;

                rgba_view.set_grid(Some(grid));

                let mut filtered_tiles = vec![];

                for (_, t) in tiles {
                    if t.tags.to_lowercase().contains(&self.filter)
                        && (self.filter_role == 0
                            || t.role == TileRole::from_index(self.filter_role - 1))
                    {
                        filtered_tiles.push(t);
                    }
                }

                if grid == 0 || width <= 0 {
                    return;
                }
                let tiles_per_row = width / grid;
                let lines = filtered_tiles.len() as i32 / tiles_per_row + 1;

                let mut buffer =
                    TheRGBABuffer::new(TheDim::sized(width, (lines * grid).max(height)));

                for (i, tile) in filtered_tiles.iter().enumerate() {
                    let x = i as i32 % tiles_per_row;
                    let y = i as i32 / tiles_per_row;

                    self.tile_ids.insert((x, y), tile.id);
                    if !tile.textures.is_empty() {
                        buffer.copy_into(
                            x * grid,
                            y * grid,
                            &tile.textures[0].to_rgba().scaled(grid, grid),
                        );
                    }
                }

                rgba_view.set_buffer(buffer);
            }
            editor.relayout(ctx);
        }
    }
}
