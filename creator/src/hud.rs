use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::prelude::*;
use theframework::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HudMode {
    Vertex,
    Linedef,
    Sector,
}

pub struct Hud {
    mode: HudMode,

    d2_rect: TheDim,
    d3iso_rect: TheDim,
    d3firstp_rect: TheDim,

    icon_rects: Vec<TheDim>,

    pub selected_icon_index: i32,

    preview_rect: TheDim,
    preview_rect_text: TheDim,

    subdiv_rects: Vec<TheDim>,
}

impl Hud {
    pub fn new(mode: HudMode) -> Self {
        Self {
            mode,
            d2_rect: TheDim::rect(80, 1, 75, 19),
            d3iso_rect: TheDim::rect(155, 1, 50, 19),
            d3firstp_rect: TheDim::rect(205, 1, 75, 19),

            icon_rects: vec![],
            selected_icon_index: 0,

            preview_rect: TheDim::rect(0, 0, 0, 0),
            preview_rect_text: TheDim::rect(0, 0, 0, 0),

            subdiv_rects: vec![],
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
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        let stride = buffer.stride();

        let info_height = 20;
        let bg_color = [50, 50, 50, 255];
        let text_color = [150, 150, 150, 255];
        let sel_text_color = [220, 220, 220, 255];

        self.subdiv_rects = vec![];

        // Material Mode
        if server_ctx.curr_map_context == MapContext::Material {
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
                        &format!("{}, {}", v.x, v.y),
                        &text_color,
                        &bg_color,
                    );
                }
            }

            // Show Subdivs
            let x = 100;
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
                        &if (i + 1) as f32 == map.subdivisions {
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

            // Draw Preview

            let preview_width = 150;
            let preview_height = 150;
            let preview_rect = TheDim::rect(
                width as i32 - preview_width - 1,
                height as i32 - preview_height - 1,
                preview_width,
                preview_height,
            );

            self.preview_rect_text = TheDim::rect(width as i32 - 50 - 1, height as i32 - 1, 50, 20);

            let mut pixels = vec![0; (preview_width * preview_height * 4) as usize];
            pixels.fill(255);
            let mut texture = Texture::new(pixels, preview_width as usize, preview_height as usize);

            let builder = D2MaterialBuilder::new();
            builder.build_texture(map, &RUSTERIX.lock().unwrap().assets.tiles, &mut texture);

            ctx.draw.copy_slice(
                buffer.pixels_mut(),
                &texture.data,
                &preview_rect.to_buffer_utuple(),
                stride,
            );

            self.preview_rect = preview_rect;

            return;
        }

        // Region Mode

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
                    &format!("{}, {}", v.x, v.y),
                    &text_color,
                    &bg_color,
                );
            }

            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &self.d2_rect.to_buffer_utuple(),
                stride,
                font,
                13.0,
                "EDIT 2D",
                if map.camera == MapCamera::TwoD {
                    &sel_text_color
                } else {
                    &text_color
                },
                &bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );

            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &self.d3iso_rect.to_buffer_utuple(),
                stride,
                font,
                13.0,
                "ISO",
                if map.camera == MapCamera::ThreeDIso {
                    &sel_text_color
                } else {
                    &text_color
                },
                &bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );

            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &self.d3firstp_rect.to_buffer_utuple(),
                stride,
                font,
                13.0,
                "FIRSTP",
                if map.camera == MapCamera::ThreeDFirstPerson {
                    &sel_text_color
                } else {
                    &text_color
                },
                &bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        }

        let icon_size = 40;
        let icons = if self.mode == HudMode::Vertex {
            0
        } else if self.mode == HudMode::Linedef {
            3
        } else {
            5
        };

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
                    &self.get_icon_text(i),
                    &text_color,
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }

            if let Some(id) = id {
                if let Some(tile) = self.get_icon(i, map, id) {
                    let texture = tile.textures[0].resized(icon_size as usize, icon_size as usize);
                    // let texture = Texture::checkerboard(icon_size as usize, 20);
                    ctx.draw.copy_slice(
                        buffer.pixels_mut(),
                        &texture.data,
                        &rect.to_buffer_utuple(),
                        stride,
                    );
                }
            } else {
                let r = &rect.to_buffer_utuple();
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2),
                    stride,
                    &[30, 30, 30, 255],
                );
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
        if map.camera == MapCamera::TwoD {
            let x = 330;
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
                        &if (i + 1) as f32 == map.subdivisions {
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

        // ----- Preview

        if map.camera == MapCamera::TwoD {
            let preview_width = (width / 3) as i32;
            let preview_height = (height / 2) as i32;
            let preview_rect = TheDim::rect(
                width as i32 - preview_width - 1,
                height as i32 - preview_height - 1,
                preview_width,
                preview_height,
            );

            self.preview_rect_text =
                TheDim::rect(width as i32 - 50 - 1, height as i32 - 20 - 1, 50, 20);

            let mut text = "ISO".to_string();
            if server_ctx.editing_preview_camera != MapCamera::TwoD {
                let mut rusterix = RUSTERIX.lock().unwrap();

                if server_ctx.editing_preview_camera == MapCamera::ThreeDIso {
                    let p = Vec3::new(
                        server_ctx.editing_camera_position.x,
                        0.0,
                        server_ctx.editing_camera_position.z,
                    );
                    rusterix.client.camera_d3.set_parameter_vec3("center", p);
                    rusterix
                        .client
                        .camera_d3
                        .set_parameter_vec3("position", p + vek::Vec3::new(-10.0, 10.0, 10.0));
                } else if server_ctx.editing_preview_camera == MapCamera::ThreeDFirstPerson {
                    text = "FIRSTP".to_string();

                    let p = Vec3::new(
                        server_ctx.editing_camera_position.x,
                        1.5,
                        server_ctx.editing_camera_position.z,
                    );
                    rusterix.client.camera_d3.set_parameter_vec3("position", p);
                    rusterix
                        .client
                        .camera_d3
                        .set_parameter_vec3("center", p + vek::Vec3::new(0.0, 0.0, -1.0));
                }

                let mut pixels = vec![0; (preview_width * preview_height * 4) as usize];
                rusterix.build_scene_d3(map);
                rusterix.client.draw_d3(
                    &mut pixels[..],
                    preview_width as usize,
                    preview_height as usize,
                );

                ctx.draw.copy_slice(
                    buffer.pixels_mut(),
                    &pixels,
                    &preview_rect.to_buffer_utuple(),
                    stride,
                );
            } else {
                text = "OFF".to_string();
            }

            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.preview_rect_text.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            if let Some(font) = &ctx.ui.font {
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &self.preview_rect_text.to_buffer_utuple(),
                    stride,
                    font,
                    10.0,
                    &text,
                    &text_color,
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }

            self.preview_rect = preview_rect;
        }
    }

    pub fn clicked(
        &mut self,
        x: i32,
        y: i32,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.curr_map_context != MapContext::Region
            && server_ctx.curr_map_context != MapContext::Material
        {
            return false;
        }

        for (i, rect) in self.icon_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                self.selected_icon_index = i as i32;
                return true;
            }
        }
        for (i, rect) in self.subdiv_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                map.subdivisions = (i + 1) as f32;
                return true;
            }
        }
        if self.preview_rect_text.contains(Vec2::new(x, y)) {
            if server_ctx.editing_preview_camera == MapCamera::ThreeDIso {
                server_ctx.editing_preview_camera = MapCamera::ThreeDFirstPerson;
            } else if server_ctx.editing_preview_camera == MapCamera::ThreeDFirstPerson {
                server_ctx.editing_preview_camera = MapCamera::TwoD;
            } else {
                server_ctx.editing_preview_camera = MapCamera::ThreeDIso;
            }
            return true;
        }
        if y < 20 {
            if self.d2_rect.contains(Vec2::new(x, y)) {
                map.camera = MapCamera::TwoD;
            } else if self.d3iso_rect.contains(Vec2::new(x, y)) {
                map.camera = MapCamera::ThreeDIso;
                RUSTERIX.lock().unwrap().client.camera_d3 = Box::new(D3IsoCamera::new())
            } else if self.d3firstp_rect.contains(Vec2::new(x, y)) {
                map.camera = MapCamera::ThreeDFirstPerson;
                RUSTERIX.lock().unwrap().client.camera_d3 = Box::new(D3FirstPCamera::new())
            }
            true
        } else {
            false
        }
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon_text(&self, index: i32) -> String {
        let mut text: String = "".into();
        if self.mode == HudMode::Linedef {
            if index == 0 {
                text = "WALL".into();
            } else if index == 1 {
                text = "R#2".into();
            } else if index == 2 {
                text = "R#3".into();
            }
        } else if self.mode == HudMode::Sector {
            if index == 0 {
                text = "FLOOR".into();
            } else if index == 1 {
                text = "CEIL".into();
            } else if index == 2 {
                text = "WALL".into();
            } else if index == 3 {
                text = "R#2".into();
            } else if index == 4 {
                text = "R#3".into();
            }
        }
        text
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon(&self, index: i32, map: &Map, id: u32) -> Option<rusterix::Tile> {
        let mut texture_id: Option<Uuid> = None;
        if self.mode == HudMode::Linedef {
            if let Some(linedef) = map.find_linedef(id) {
                if index == 0 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &linedef.properties.get("row1_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                } else if index == 1 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &linedef.properties.get("row2_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                } else if index == 2 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &linedef.properties.get("row3_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                }
            }
        } else if self.mode == HudMode::Sector {
            if let Some(sector) = map.find_sector(id) {
                if index == 0 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &sector.properties.get("floor_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                } else if index == 1 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &sector.properties.get("ceiling_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                } else if index == 2 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &sector.properties.get("row1_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                } else if index == 3 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &sector.properties.get("row2_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                } else if index == 4 {
                    if let Some(Value::Source(PixelSource::TileId(tile_id))) =
                        &sector.properties.get("row3_source")
                    {
                        texture_id = Some(*tile_id);
                    }
                }
            }
        }

        if let Some(texture_id) = texture_id {
            if let Some(tile) = RUSTERIX.lock().unwrap().assets.tiles.get(&texture_id) {
                return Some(tile.clone());
            }
        }
        None
    }
}
