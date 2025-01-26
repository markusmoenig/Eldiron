//use crate::prelude::*;
use super::{
    CHARACTER, DRAWSETTINGS, FONTS, IMAGES, PALETTE, REGIONS, SENDCMD, TILEDRAWER, UPDATE,
    WIDGETBUFFER,
};
use theframework::prelude::*;

pub fn add_compiler_client_functions(compiler: &mut TheCompiler) {
    //
    compiler.add_external_call(
        "DrawGame".to_string(),
        |stack, _data, _sandbox| {
            let mut buffer = WIDGETBUFFER.write().unwrap();
            let mut update = UPDATE.write().unwrap();

            let mut zoom = 1.0;
            if let Some(TheValue::Float(v)) = stack.pop() {
                zoom = v;
            }

            let mut mode = 0;
            if let Some(TheValue::TextList(v, _)) = stack.pop() {
                mode = v;
            }

            /*
            if let Some(region) = REGIONS.read().unwrap().get(&update.id) {
                let mut settings = DRAWSETTINGS.write().unwrap();
                let character_id = CHARACTER.read().unwrap();
                if let Some(_character) = update.characters.get(&character_id) {
                    settings.center_on_character = Some(*character_id);
                } else {
                    settings.center_on_character = None;
                }

                settings.daylight = update.daylight;

                if mode == 0 {
                    // 2D

                    let tiledrawer = TILEDRAWER.read().unwrap();

                    if zoom != 1.0 {
                        let scaled_width = (buffer.dim().width as f32 / zoom) as i32;
                        let scaled_height = (buffer.dim().height as f32 / zoom) as i32;
                        let mut zoom_buffer =
                            TheRGBABuffer::new(TheDim::new(0, 0, scaled_width, scaled_height));
                        tiledrawer.draw_region(
                            &mut zoom_buffer,
                            region,
                            &mut update,
                            &mut settings,
                            true,
                            &PALETTE.read().unwrap(),
                        );
                        zoom_buffer.scaled_into(&mut buffer);
                    } else {
                        tiledrawer.draw_region(
                            &mut buffer,
                            region,
                            &mut update,
                            &mut settings,
                            true,
                            &PALETTE.read().unwrap(),
                        );
                    }
                } else if mode == 1 {
                    // 3D

                    //let mut renderer = RENDERER.write().unwrap();

                    let upscale = 1.0;
                    // TODO: Read from render settings

                    if upscale != 1.0 {
                        let scaled_width = (buffer.dim().width as f32 / upscale) as i32;
                        let scaled_height = (buffer.dim().height as f32 / upscale) as i32;
                        let upscaled_buffer =
                            TheRGBABuffer::new(TheDim::new(0, 0, scaled_width, scaled_height));

                        // renderer.rendered(
                        //     &mut upscaled_buffer,
                        //     region,
                        //     &mut update,
                        //     &mut settings,
                        //     true,
                        //     &PALETTE.read().unwrap(),
                        // );

                        upscaled_buffer.scaled_into(&mut buffer);
                        // upscaled_buffer.scaled_into_linear(&mut buffer);
                    } else {
                        // renderer.rendered(
                        //     &mut buffer,
                        //     region,
                        //     &mut update,
                        //     &mut settings,
                        //     true,
                        //     &PALETTE.read().unwrap(),
                        // );
                    }
                }
            }*/

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "Fill".to_string(),
        |stack, _data, _sandbox| {
            let mut buffer = WIDGETBUFFER.write().unwrap();

            let mut color = TheColor::default();
            if let Some(TheValue::ColorObject(c)) = stack.pop() {
                color = c;
            }

            buffer.fill(color.to_u8_array());

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "DrawText".to_string(),
        |stack, _data, _sandbox| {
            let mut buffer = WIDGETBUFFER.write().unwrap();

            let mut text = str!("text");
            if let Some(TheValue::Text(t)) = stack.pop() {
                text.clone_from(&t);
            }

            let mut size = 12.0;
            if let Some(TheValue::Float(s)) = stack.pop() {
                size = s;
            }

            let mut font_name = str!("font");
            if let Some(TheValue::Text(t)) = stack.pop() {
                font_name.clone_from(&t);
            }

            if let Some(font) = FONTS.read().unwrap().get(&font_name) {
                buffer.draw_text(
                    Vec2::new(0, 0),
                    font,
                    text.as_str(),
                    size,
                    WHITE,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "CreateImg".to_string(),
        |stack, _data, _sandbox| {
            let mut source_size = Vec2::new(0, 0);
            if let Some(TheValue::Int2(v)) = stack.pop() {
                source_size = v;
            }

            let mut source_pos = Vec2::new(0, 0);
            if let Some(TheValue::Int2(v)) = stack.pop() {
                source_pos = v;
            }

            let mut image_name = str!("image");
            if let Some(TheValue::Text(v)) = stack.pop() {
                image_name.clone_from(&v);
            }

            if let Some(image) = IMAGES.read().unwrap().get(&image_name) {
                let dim = TheDim::new(source_pos.x, source_pos.y, source_size.x, source_size.y);
                let img = image.extract(&dim);
                stack.push(TheValue::Image(img));
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "CreateTile".to_string(),
        |stack, _data, _sandbox| {
            let mut tags = "".to_string();
            if let Some(TheValue::Text(t)) = stack.pop() {
                tags = t.to_lowercase();
            }

            let mut category = 0;
            if let Some(TheValue::TextList(c, _)) = stack.pop() {
                category = c;
            }

            if let Some(value) = TILEDRAWER
                .read()
                .unwrap()
                .get_tile_by_tags(category as u8, &tags)
            {
                stack.push(value);
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "SendCmd".to_string(),
        |stack, _data, _sandbox| {
            let mut cmd = "".to_string();
            if let Some(TheValue::Text(t)) = stack.pop() {
                cmd = t.to_lowercase();
            }

            if !cmd.is_empty() {
                SENDCMD.read().unwrap().send(cmd).unwrap();
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "Player".to_string(),
        |stack, _data, _sandbox| {
            let mut new_name = "".to_string();
            if let Some(TheValue::Text(t)) = stack.pop() {
                new_name = t;
            }

            let mut old_name = "".to_string();
            if let Some(TheValue::Text(t)) = stack.pop() {
                old_name = t;
            }

            if !new_name.is_empty() && !old_name.is_empty() {
                SENDCMD
                    .read()
                    .unwrap()
                    .send(format!("instantiate {} {}", old_name, new_name))
                    .unwrap();
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "Start".to_string(),
        |_stack, _data, _sandbox| {
            let cmd = "start server".to_string();

            if !cmd.is_empty() {
                SENDCMD.read().unwrap().send(cmd).unwrap();
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "DrawImg".to_string(),
        |stack, _data, _sandbox| {
            let mut buffer = WIDGETBUFFER.write().unwrap();

            let mut pos = Vec2::new(0, 0);
            if let Some(TheValue::Int2(v)) = stack.pop() {
                pos = v;
            }

            let value = stack.pop();
            if let Some(TheValue::Image(image_buffer)) = value {
                buffer.copy_into(pos.x, pos.y, &image_buffer);
            } else if let Some(TheValue::Tile(_, id)) = value {
                if let Some(image_buffer) = TILEDRAWER.read().unwrap().get_tile(&id) {
                    buffer.blend_into(pos.x, pos.y, &image_buffer.buffer[0]);
                }
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "ScaleImg".to_string(),
        |stack, _data, _sandbox| {
            let mut size = Vec2::new(1, 1);
            if let Some(TheValue::Int2(v)) = stack.pop() {
                size = v;
            }

            if let Some(TheValue::Image(v)) = stack.pop() {
                if size.x > 0 && size.y > 0 {
                    let new_img = v.scaled(size.x, size.y);
                    stack.push(TheValue::Image(new_img));
                }
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );
}
