//! A small monochrome intent mark accompanies the unchanged normal pointer.
use super::*;
impl Client {
    fn targeting_cursor_command(&self, map: &Map) -> Option<String> {
        if self.curr_cursor.is_some() && self.curr_cursor != self.default_cursor {
            return None;
        }
        let point = self.cursor_pos.map(|v| v as f32);
        if !self.game_widgets.values().any(|g| g.rect.contains(point)) {
            return None;
        }
        if self.button_widgets.values().any(|b| {
            !Self::widget_name_is_hidden(&b.name, &self.widgets_to_hide) && b.rect.contains(point)
        }) {
            return None;
        }
        if [
            self.inventory_panel_rect,
            self.equipment_panel_rect,
            self.preferences_panel_rect,
            self.actions_panel_rect,
            self.open_container_panel_rect,
        ]
        .iter()
        .any(|r| r.is_some_and(|r| r.contains(point)))
        {
            return None;
        }
        let intent = self.get_current_intent()?;
        let command = if let Some(action) = intent.strip_prefix("action:") {
            format!("rules.{action}")
        } else {
            format!("intent.{intent}")
        };
        Self::contextual_open_close_command(Some(command), self.hovered_item_id, map)
    }
    pub(super) fn draw_intent_cursor(&mut self, map: &Map, assets: &Assets, overlay: bool) {
        let Some(command) = self.targeting_cursor_command(map) else {
            return;
        };
        let Some(texture) = Widget::command_icon_texture(
            assets,
            Some(&command),
            widget::ButtonVisualState::Normal,
            self.animation_frame,
        ) else {
            return;
        };
        let buffer = if overlay {
            &mut self.overlay
        } else {
            &mut self.target
        };
        let size = 16.;
        let x =
            (self.cursor_pos.x as f32 + 9.).clamp(0., (buffer.dim().width as f32 - size).max(0.));
        let y =
            (self.cursor_pos.y as f32 + 9.).clamp(0., (buffer.dim().height as f32 - size).max(0.));
        let stride = buffer.stride();
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        let pixels = buffer.pixels_mut();
        for py in 0..size as usize {
            for px in 0..size as usize {
                let dx = x as usize + px;
                let dy = y as usize + py;
                if dx >= width || dy >= height {
                    continue;
                }
                let sx = px * texture.width / size as usize;
                let sy = py * texture.height / size as usize;
                let src = (sy * texture.width + sx) * 4;
                let alpha = texture.data[src + 3];
                // Keep internal details (eye pupil, door frame) readable without hue.
                let luma = (texture.data[src] as u32 * 54
                    + texture.data[src + 1] as u32 * 183
                    + texture.data[src + 2] as u32 * 19)
                    / 256;
                let tone = if luma >= 100 { 230 } else { 35 };
                if alpha == 0 {
                    continue;
                }
                let dst = (dy * stride + dx) * 4;
                let background = [
                    pixels[dst],
                    pixels[dst + 1],
                    pixels[dst + 2],
                    pixels[dst + 3],
                ];
                pixels[dst..dst + 4].copy_from_slice(&self.draw2d.mix_color(
                    &background,
                    &[tone, tone, tone, alpha],
                    alpha as f32 / 255.,
                ));
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_intents_draw_their_mz_icons_and_respect_custom_cursors() {
        let mut c = Client::new();
        c.target = TheRGBABuffer::new(TheDim::sized(128, 128));
        c.cursor_pos = Vec2::new(32, 32);
        let mut g = GameWidget::new();
        g.rect = Rect::new(0., 0., 128., 128.);
        c.game_widgets.insert(Uuid::new_v4(), g);
        let mut a = Assets::default();
        a.rules="[intents.use.ui]\nicon='use'\n[intents.look.ui]\nicon='look'\n[intents.open.ui]\nicon='open'\n[intents.close.ui]\nicon='close'\n".into();
        for name in ["use", "look", "open", "close"] {
            a.textures.insert(
                name.into(),
                Texture::new([240, 100, 30, 255].repeat(16), 4, 4),
            );
        }
        let mut map = Map::default();
        let mut door = Item::new();
        door.id = 5;
        door.set_attribute("openable", Value::Bool(true));
        map.items.push(door);
        for intent in ["use", "look", "open"] {
            c.intent = intent.into();
            c.target.fill([0, 0, 0, 0]);
            c.draw_intent_cursor(&map, &a, false);
            assert!(c.target.pixels().chunks_exact(4).any(|p| p[3] > 0));
            assert!(
                c.target
                    .pixels()
                    .chunks_exact(4)
                    .filter(|p| p[3] > 0)
                    .all(|p| p[0] == p[1] && p[1] == p[2])
            );
        }
        c.intent = "open_close".into();
        c.hovered_item_id = Some(5);
        assert_eq!(
            c.targeting_cursor_command(&map).as_deref(),
            Some("intent.open")
        );
        map.items[0].set_attribute("active", Value::Bool(true));
        assert_eq!(
            c.targeting_cursor_command(&map).as_deref(),
            Some("intent.close")
        );
        c.curr_cursor = Some(Uuid::new_v4());
        assert!(c.targeting_cursor_command(&map).is_none());
    }
}
