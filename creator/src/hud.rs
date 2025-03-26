use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::prelude::*;
use theframework::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HudMode {
    Vertex,
    Linedef,
    Sector,
    Effects,
    Rect,
}

pub struct Hud {
    mode: HudMode,

    icon_rects: Vec<TheDim>,

    pub selected_icon_index: i32,

    subdiv_rects: Vec<TheDim>,

    state_rects: Vec<TheDim>,
    add_state_rect: TheDim,

    play_button_rect: TheDim,
    timeline_rect: TheDim,

    rect_geo_rect: TheDim,

    mouse_pos: Vec2<i32>,

    is_playing: bool,
    light_icon: Option<TheRGBABuffer>,
}

impl Hud {
    pub fn new(mode: HudMode) -> Self {
        Self {
            mode,

            icon_rects: vec![],
            selected_icon_index: 0,

            subdiv_rects: vec![],

            state_rects: vec![],
            add_state_rect: TheDim::rect(0, 0, 0, 0),

            play_button_rect: TheDim::rect(0, 0, 0, 0),
            timeline_rect: TheDim::rect(0, 0, 0, 0),

            rect_geo_rect: TheDim::zero(),

            mouse_pos: Vec2::zero(),

            is_playing: false,
            light_icon: None,
        }
    }

    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        id: Option<u32>,
    ) {
        if (self.mode == HudMode::Linedef || self.mode == HudMode::Sector)
            && self.light_icon.is_none()
        {
            if let Some(li) = ctx.ui.icon("light_small") {
                self.light_icon = Some(li.clone());
            }
        }

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        let stride = buffer.stride();

        let info_height = 20;
        let bg_color = [50, 50, 50, 255];
        let dark_bg_color = [30, 30, 30, 255];
        let text_color = [150, 150, 150, 255];
        let sel_text_color = [220, 220, 220, 255];

        self.state_rects.clear();
        self.subdiv_rects = vec![];

        ctx.draw.rect(
            buffer.pixels_mut(),
            &(0, 0, width, info_height),
            stride,
            &bg_color,
        );

        if let Some(font) = &ctx.ui.font {
            if let Some(v) = server_ctx.hover_cursor {
                ctx.draw.text(
                    buffer.pixels_mut(),
                    &(10, 2),
                    stride,
                    font,
                    13.0,
                    &format!("{:.2}, {:.2}", v.x, v.y),
                    &text_color,
                    &bg_color,
                );
            }
        }

        // States
        if self.mode != HudMode::Effects {
            let x = 0;
            let state_width = 70;
            let state_height = 25_i32;
            let mut y =
                height - state_height as usize - map.animation.states.len() * state_height as usize;

            // Base State
            let rect = TheDim::rect(x, y as i32, state_width, state_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &dark_bg_color,
            );

            if let Some(font) = &ctx.ui.font {
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    font,
                    11.5,
                    "Base State",
                    if map.animation.current_state.is_none() {
                        &sel_text_color
                    } else {
                        &text_color
                    },
                    &dark_bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }

            self.state_rects.push(rect);

            // Animation States
            y += state_height as usize;
            for i in 0..map.animation.states.len() {
                let rect = TheDim::rect(x, y as i32, state_width, state_height);
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &dark_bg_color,
                );
                if let Some(font) = &ctx.ui.font {
                    ctx.draw.text_rect(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        font,
                        11.5,
                        &map.animation.states[i].state_name,
                        if map.animation.current_state == Some(i)
                            || map.animation.loop_states.contains(&i)
                        {
                            &sel_text_color
                        } else {
                            &text_color
                        },
                        &dark_bg_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }

                self.state_rects.push(rect);
                y += state_height as usize;
            }

            // Plus buttton
            y -= state_height as usize;
            let rect = TheDim::rect(state_width, y as i32, state_height, state_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            if let Some(font) = &ctx.ui.font {
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    font,
                    11.5,
                    "+",
                    if map.animation.current_state.is_none() {
                        &sel_text_color
                    } else {
                        &text_color
                    },
                    &dark_bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }

            self.add_state_rect = rect;

            // Play button and timeline

            let rect = TheDim::rect(150, y as i32, state_height, state_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            if let Some(font) = &ctx.ui.font {
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    font,
                    11.5,
                    "P",
                    if map.animation.current_state.is_none() {
                        &sel_text_color
                    } else {
                        &text_color
                    },
                    &dark_bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }

            self.play_button_rect = rect;

            if self.is_playing {
                map.tick(1000.0 / 30.0);
            }

            let rect = TheDim::rect(150 + state_height, y as i32, 150, state_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &dark_bg_color,
            );

            self.timeline_rect = rect;
        }

        // Icons

        let icon_size = 40;
        let mut icons = 0;

        if server_ctx.curr_map_context == MapContext::Region {
            icons = if self.mode == HudMode::Vertex {
                0
            } else if self.mode == HudMode::Linedef {
                4
            } else {
                2
            };
        } else if server_ctx.curr_map_context == MapContext::Material {
            icons = 1;
        }

        if self.mode == HudMode::Effects || self.mode == HudMode::Rect {
            icons = 0;
        }

        let x = width as i32 - (icon_size * icons) - 1;
        for i in 0..icons {
            let rect = TheDim::rect(x + (i * icon_size), 20, icon_size, icon_size);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            if let Some(font) = &ctx.ui.font {
                let r = rect.to_buffer_utuple();
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(r.0, 1, r.2, 19),
                    stride,
                    font,
                    10.0,
                    &self.get_icon_text(i, server_ctx),
                    &text_color,
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }

            let r = &rect.to_buffer_utuple();
            ctx.draw.rect(
                buffer.pixels_mut(),
                &(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2),
                stride,
                &[30, 30, 30, 255],
            );

            if let Some(id) = id {
                let (tile, has_light) = self.get_icon(i, map, id, icon_size as usize);
                if let Some(tile) = tile {
                    let texture = tile.textures[0].resized(icon_size as usize, icon_size as usize);
                    ctx.draw.copy_slice(
                        buffer.pixels_mut(),
                        &texture.data,
                        &rect.to_buffer_utuple(),
                        stride,
                    );
                }
                if has_light {
                    if let Some(light_icon) = &self.light_icon {
                        ctx.draw.blend_slice(
                            buffer.pixels_mut(),
                            light_icon.pixels(),
                            &(
                                rect.x as usize + 1,
                                rect.y as usize + 1,
                                light_icon.dim().width as usize,
                                light_icon.dim().height as usize,
                            ),
                            stride,
                        );
                    }
                }
            }

            if i == self.selected_icon_index {
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &sel_text_color,
                );
            }

            self.icon_rects.push(rect);
        }

        // Show Subdivs
        if map.camera == MapCamera::TwoD
            || server_ctx.curr_map_context == MapContext::Material
            || server_ctx.curr_map_context == MapContext::Screen
        {
            let x = 150;

            let size = 20;
            for i in 0..10 {
                let rect = TheDim::rect(x + (i * size), 0, size, size);

                if let Some(font) = &ctx.ui.font {
                    let r = rect.to_buffer_utuple();
                    ctx.draw.text_rect(
                        buffer.pixels_mut(),
                        &(r.0, 1, r.2, 19),
                        stride,
                        font,
                        13.0,
                        &(i + 1).to_string(),
                        &if (i + 1) as f32 == map.subdivisions || rect.contains(self.mouse_pos) {
                            sel_text_color
                        } else {
                            text_color
                        },
                        &bg_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
                self.subdiv_rects.push(rect);
            }
        }

        if map.camera == MapCamera::TwoD {
            let x = 400;
            let size = 20;
            self.rect_geo_rect = TheDim::rect(x, 0, 100, size);

            if let Some(font) = &ctx.ui.font {
                let r = self.rect_geo_rect.to_buffer_utuple();
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(r.0, 1, r.2, 19),
                    stride,
                    font,
                    13.0,
                    if server_ctx.no_rect_geo_on_map {
                        "SHOW RECTS"
                    } else {
                        "HIDE RECTS"
                    },
                    &if self.rect_geo_rect.contains(self.mouse_pos) {
                        sel_text_color
                    } else {
                        text_color
                    },
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
        }

        // ----- Preview

        if server_ctx.curr_map_context == MapContext::Material {
            let preview_width = 150;
            let preview_height = 150;
            let preview_rect = TheDim::rect(
                width as i32 - preview_width - 1,
                height as i32 - preview_height - 1,
                preview_width,
                preview_height,
            );

            let mut pixels = vec![0; (preview_width * preview_height * 4) as usize];
            pixels.fill(255);
            let mut texture = Texture::new(pixels, preview_width as usize, preview_height as usize);

            let builder = D2MaterialBuilder::new();
            builder.build_texture(map, &RUSTERIX.read().unwrap().assets, &mut texture);

            ctx.draw.copy_slice(
                buffer.pixels_mut(),
                &texture.data,
                &preview_rect.to_buffer_utuple(),
                stride,
            );
        }
    }

    pub fn clicked(
        &mut self,
        x: i32,
        y: i32,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.curr_map_context != MapContext::Region
            && server_ctx.curr_map_context != MapContext::Material
            && server_ctx.curr_map_context != MapContext::Screen
        {
            return false;
        }

        for (i, rect) in self.icon_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                self.selected_icon_index = i as i32;
                if self.mode == HudMode::Linedef {
                    server_ctx.selected_wall_row = Some(i as i32);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
                return true;
            }
        }
        for (i, rect) in self.subdiv_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                map.subdivisions = (i + 1) as f32;
                return true;
            }
        }

        // Parse States
        for (i, rect) in self.state_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                if i == 0 {
                    map.animation.current_state = None;
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Base Anim State Selected"),
                        TheValue::Int(i as i32 - 1),
                    ));
                } else if ui.shift && !map.animation.loop_states.contains(&i) {
                    // map.animation.loop_states.push(i - 1);
                    map.animation.next_state = Some(i - 1);
                } else {
                    map.animation.current_state = Some(i - 1);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Anim State Selected"),
                        TheValue::Int(i as i32 - 1),
                    ));
                }

                return true;
            }
        }
        // Add State
        if self.add_state_rect.contains(Vec2::new(x, y)) {
            let offset =
                map.animation
                    .add_state("New State", vec![], state::InterpolationType::Linear);

            map.animation.current_state = Some(offset);
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Anim State Selected"),
                TheValue::Int(offset as i32),
            ));
            return true;
        }
        // Play Button
        if self.play_button_rect.contains(Vec2::new(x, y)) {
            //self.is_playing = !self.is_playing;
            return true;
        }
        // Timeline
        if self.timeline_rect.contains(Vec2::new(x, y)) {
            let offset = x - self.timeline_rect.x;
            let progress = offset as f32 / self.timeline_rect.width as f32;
            map.animation.transition_progress = progress;
            println!("{:?}", map.animation);
            return true;
        }

        if self.rect_geo_rect.contains(Vec2::new(x, y)) {
            server_ctx.no_rect_geo_on_map = !server_ctx.no_rect_geo_on_map;
            return true;
        }

        if map.camera == MapCamera::TwoD && y < 20 {
            return true;
        }

        false
    }

    pub fn dragged(
        &mut self,
        x: i32,
        y: i32,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if self.timeline_rect.contains(Vec2::new(x, y)) {
            let offset = x - self.timeline_rect.x;
            let progress = offset as f32 / self.timeline_rect.width as f32;
            map.animation.transition_progress = progress;
            return true;
        }

        false
    }

    pub fn hovered(
        &mut self,
        x: i32,
        y: i32,
        _map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.mouse_pos = Vec2::new(x, y);
        false
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon_text(&self, index: i32, server_ctx: &mut ServerContext) -> String {
        let mut text: String = "".into();
        if server_ctx.curr_map_context == MapContext::Region {
            if self.mode == HudMode::Linedef {
                if index == 0 {
                    text = "WALL".into();
                } else if index == 1 {
                    text = "ROW2".into();
                } else if index == 2 {
                    text = "ROW3".into();
                } else if index == 3 {
                    text = "ROW4".into();
                }
            } else if self.mode == HudMode::Sector {
                if index == 0 {
                    text = "FLOOR".into();
                } else if index == 1 {
                    text = "CEIL".into();
                }
            }
        } else if server_ctx.curr_map_context == MapContext::Material {
            if index == 0 {
                text = "IN 1".into();
            } else if index == 1 {
                text = "IN 2".into();
            }
        }

        text
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon(
        &self,
        index: i32,
        map: &Map,
        id: u32,
        icon_size: usize,
    ) -> (Option<rusterix::Tile>, bool) {
        if self.mode == HudMode::Linedef {
            if let Some(linedef) = map.find_linedef(id) {
                if index == 0 {
                    let has_light = linedef.properties.get("row1_light").is_some();
                    if let Some(Value::Source(pixelsource)) = &linedef.properties.get("row1_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &linedef.properties,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                } else if index == 1 {
                    let has_light = linedef.properties.get("row2_light").is_some();
                    if let Some(Value::Source(pixelsource)) = &linedef.properties.get("row2_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.write().unwrap().assets,
                            icon_size,
                            &linedef.properties,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                } else if index == 2 {
                    let has_light = linedef.properties.get("row3_light").is_some();
                    if let Some(Value::Source(pixelsource)) = &linedef.properties.get("row3_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &linedef.properties,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                } else if index == 3 {
                    let has_light = linedef.properties.get("row4_light").is_some();
                    if let Some(Value::Source(pixelsource)) = &linedef.properties.get("row4_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &linedef.properties,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                }
            }
        } else if self.mode == HudMode::Sector {
            if let Some(sector) = map.find_sector(id) {
                if index == 0 {
                    let has_light = sector.properties.get("floor_light").is_some();
                    if let Some(Value::Source(pixelsource)) = &sector.properties.get("floor_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &sector.properties,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                } else if index == 1 {
                    let has_light = sector.properties.get("ceiling_light").is_some();
                    if let Some(Value::Source(pixelsource)) =
                        &sector.properties.get("ceiling_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &sector.properties,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                }
            }
        }

        (None, false)
    }
}
