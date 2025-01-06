use crate::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HudMode {
    Vertex,
    Linedef,
    Sector,
}

pub struct Hud {
    mode: HudMode,
}

impl Hud {
    pub fn new(mode: HudMode) -> Self {
        Self { mode }
    }

    pub fn draw(
        &self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
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

        let mut y = 1;
        if let Some(font) = &ctx.ui.font {
            if let Some(v) = server_ctx.hover_cursor {
                ctx.draw.text(
                    buffer.pixels_mut(),
                    &(10, y),
                    stride,
                    font,
                    13.0,
                    &format!("{}, {}", v.x, v.y),
                    &text_color,
                    &bg_color,
                );
            }

            let mut x = 80;
            y += 1;
            ctx.draw.text(
                buffer.pixels_mut(),
                &(x, y),
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
            );

            x += 75;
            ctx.draw.text(
                buffer.pixels_mut(),
                &(x, y),
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
            );

            x += 50;
            ctx.draw.text(
                buffer.pixels_mut(),
                &(x, y),
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
            );
        }
    }
}
