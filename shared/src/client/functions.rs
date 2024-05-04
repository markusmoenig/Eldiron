//use crate::prelude::*;
use super::{
    CHARACTER, DRAWSETTINGS, FONTS, IMAGES, PALETTE, REGIONS, RENDERER, TILEDRAWER, UPDATE,
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
                        );
                        zoom_buffer.scaled_into(&mut buffer);
                    } else {
                        tiledrawer.draw_region(
                            &mut buffer,
                            region,
                            &mut update,
                            &mut settings,
                            true,
                        );
                    }
                } else if mode == 1 {
                    // 3D

                    let mut renderer = RENDERER.write().unwrap();

                    let width = buffer.dim().width as usize;
                    let height = buffer.dim().height as usize;

                    renderer.render(
                        &mut buffer,
                        region,
                        &mut update,
                        &mut settings,
                        width,
                        height,
                        true,
                        &PALETTE.read().unwrap(),
                    );
                }
            }

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
                text = t.clone();
            }

            let mut size = 12.0;
            if let Some(TheValue::Float(s)) = stack.pop() {
                size = s;
            }

            let mut font_name = str!("font");
            if let Some(TheValue::Text(t)) = stack.pop() {
                font_name = t.clone();
            }

            if let Some(font) = FONTS.read().unwrap().get(&font_name) {
                buffer.draw_text(font, text.as_str(), size, WHITE);
            }

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "CreateImg".to_string(),
        |stack, _data, _sandbox| {
            let mut source_size = vec2i(0, 0);
            if let Some(TheValue::Int2(v)) = stack.pop() {
                source_size = v;
            }

            let mut source_pos = vec2i(0, 0);
            if let Some(TheValue::Int2(v)) = stack.pop() {
                source_pos = v;
            }

            let mut image_name = str!("image");
            if let Some(TheValue::Text(v)) = stack.pop() {
                image_name = v.clone();
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
        "DrawImg".to_string(),
        |stack, _data, _sandbox| {
            let mut buffer = WIDGETBUFFER.write().unwrap();

            let mut pos = vec2i(0, 0);
            if let Some(TheValue::Int2(v)) = stack.pop() {
                pos = v;
            }

            let mut image_buffer = TheRGBABuffer::default();
            if let Some(TheValue::Image(v)) = stack.pop() {
                image_buffer = v.clone();
            }

            buffer.copy_into(pos.x, pos.y, &image_buffer);

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "ScaleImg".to_string(),
        |stack, _data, _sandbox| {
            let mut size = vec2i(1, 1);
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
