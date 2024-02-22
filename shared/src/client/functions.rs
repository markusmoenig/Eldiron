//use crate::prelude::*;
use super::{CHARACTER, DRAWSETTINGS, FONTS, REGIONS, TILEDRAWER, UPDATE, WIDGETBUFFER};
use theframework::prelude::*;

pub fn add_compiler_client_functions(compiler: &mut TheCompiler) {
    //
    compiler.add_external_call(
        "DrGame".to_string(),
        |stack, _data, _sandbox| {
            let mut buffer = WIDGETBUFFER.write().unwrap();
            let mut update = UPDATE.write().unwrap();
            let tiledrawer = TILEDRAWER.read().unwrap();

            let mut zoom = 1.0;
            if let Some(TheValue::Float(v)) = stack.pop() {
                zoom = v;
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

                if zoom != 1.0 {
                    let scaled_width = (buffer.dim().width as f32 / zoom) as i32;
                    let scaled_height = (buffer.dim().height as f32 / zoom) as i32;
                    let mut zoom_buffer =
                        TheRGBABuffer::new(TheDim::new(0, 0, scaled_width, scaled_height));
                    tiledrawer.draw_region(&mut zoom_buffer, region, &mut update, &settings);
                    zoom_buffer.scaled_into(&mut buffer);
                } else {
                    tiledrawer.draw_region(&mut buffer, region, &mut update, &settings);
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
            if let Some(TheValue::ColorObject(c, _r)) = stack.pop() {
                color = c;
            }

            buffer.fill(color.to_u8_array());

            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    compiler.add_external_call(
        "DrText".to_string(),
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
}
