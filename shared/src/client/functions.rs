//use crate::prelude::*;
use super::{CHARACTER, REGIONS, TILEDRAWER, UPDATE, WIDGETBUFFER, FONTS, DRAWSETTINGS};
use theframework::prelude::*;

pub fn add_compiler_client_functions(compiler: &mut TheCompiler) {
    //
    compiler.add_external_call(
        "DrGame".to_string(),
        |_stack, _data, _sandbox| {

            let mut buffer = WIDGETBUFFER.write().unwrap();
            let mut update = UPDATE.write().unwrap();
            let tiledrawer = TILEDRAWER.read().unwrap();

            if let Some(region) = REGIONS.read().unwrap().get(&update.id) {
                let mut settings = DRAWSETTINGS.write().unwrap();
                let character_id = CHARACTER.read().unwrap();
                if let Some(_character) = update.characters.get(&character_id) {
                    settings.center_on_character = Some(*character_id);
                } else {
                    settings.center_on_character = None;
                }
                tiledrawer.draw_region(&mut buffer, region, &mut update, &settings);
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
