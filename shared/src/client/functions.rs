//use crate::prelude::*;
use super::{CHARACTER, REGIONS, TILEDRAWER, UPDATE, WIDGETBUFFER, FONTS};
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
                let mut offset = Vec2i::zero();
                if let Some(character) = update.characters.get(&CHARACTER.read().unwrap()) {
                    offset.x = character.position.x as i32 * region.grid_size;
                    offset.y = character.position.y as i32 * region.grid_size;
                }
                tiledrawer.draw_region(&mut buffer, region, &0, &mut update, &16.0, &0, offset);
            }

            // if let Some(key) = KEY_DOWN.read().unwrap().clone() {
            //     stack.push(TheValue::Text(key));
            // } else {
            //     stack.push(TheValue::Empty);
            // }

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
