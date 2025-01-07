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
        // let height = buffer.dim().height as usize;
        let stride = buffer.stride();
        let info_height = 20;

        let bg_color = [50, 50, 50, 255];
        let text_color = [150, 150, 150, 255];
        let sel_text_color = [220, 220, 220, 255];

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
            }

            if i == self.selected_icon_index {
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &[255, 255, 255, 255],
                );
            }

            self.icon_rects.push(rect);
        }
    }

    pub fn clicked(&mut self, x: i32, y: i32, map: &mut Map) -> bool {
        for (i, rect) in self.icon_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                self.selected_icon_index = i as i32;
                return true;
            }
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
                    texture_id = linedef.texture_row1;
                } else if index == 1 {
                    texture_id = linedef.texture_row2;
                } else if index == 2 {
                    texture_id = linedef.texture_row3;
                }
            }
        } else if self.mode == HudMode::Sector {
            if let Some(sector) = map.find_sector(id) {
                if index == 0 {
                    texture_id = sector.floor_texture
                } else if index == 1 {
                    texture_id = sector.ceiling_texture
                } else if index == 2 {
                    texture_id = sector.texture_row1;
                } else if index == 3 {
                    texture_id = sector.texture_row2;
                } else if index == 4 {
                    texture_id = sector.texture_row3;
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
