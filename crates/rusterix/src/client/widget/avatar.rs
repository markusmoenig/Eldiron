use crate::{
    Assets, AvatarDirection, AvatarShadingOptions, Entity, Rect,
    avatar_builder::AvatarRuntimeBuilder, client::draw2d,
};
use draw2d::Draw2D;
use theframework::prelude::*;

pub struct AvatarWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub party: Option<String>,

    pub avatar_name: Option<String>,
    pub animation: Option<String>,
    pub frame_index: usize,
    pub perspective: AvatarDirection,
    pub show_weapons: bool,
}

impl Default for AvatarWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl AvatarWidget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            party: None,
            avatar_name: None,
            animation: None,
            frame_index: 0,
            perspective: AvatarDirection::Front,
            show_weapons: true,
        }
    }

    pub fn init(&mut self) {
        if let Ok(value) = self.toml_str.parse::<toml::Table>()
            && let Some(ui) = value.get("ui").and_then(toml::Value::as_table)
        {
            if let Some(v) = ui.get("avatar").and_then(toml::Value::as_str) {
                let name = v.trim();
                if !name.is_empty() {
                    self.avatar_name = Some(name.to_string());
                }
            }

            if let Some(v) = ui.get("party").and_then(toml::Value::as_str) {
                let binding = v.trim();
                if !binding.is_empty() {
                    self.party = Some(binding.to_string());
                }
            }

            if let Some(v) = ui.get("animation").and_then(toml::Value::as_str) {
                let anim = v.trim();
                if !anim.is_empty() {
                    self.animation = Some(anim.to_string());
                }
            }

            if let Some(v) = ui.get("frame_index").and_then(toml::Value::as_integer)
                && v >= 0
            {
                self.frame_index = v as usize;
            }

            if let Some(v) = ui.get("perspective").and_then(toml::Value::as_str) {
                self.perspective = match v.to_ascii_lowercase().as_str() {
                    "back" => AvatarDirection::Back,
                    "left" => AvatarDirection::Left,
                    "right" => AvatarDirection::Right,
                    _ => AvatarDirection::Front,
                };
            }

            if let Some(v) = ui.get("show_weapons").and_then(toml::Value::as_bool) {
                self.show_weapons = v;
            }
        }
    }

    pub fn update_draw(
        &mut self,
        target: &mut TheRGBABuffer,
        assets: &Assets,
        entity: Option<&Entity>,
        draw2d: &Draw2D,
    ) {
        self.buffer.fill([0, 0, 0, 0]);

        let Some(entity) = entity else {
            target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
            return;
        };

        let Some(avatar) = self
            .avatar_name
            .as_ref()
            .and_then(|name| assets.avatars.get(name))
            .or_else(|| AvatarRuntimeBuilder::find_avatar_for_entity(entity, assets))
        else {
            target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
            return;
        };

        let Some(out) = AvatarRuntimeBuilder::build_preview_for_entity_with_weapons(
            entity,
            avatar,
            assets,
            self.animation.as_deref(),
            self.perspective,
            self.frame_index,
            AvatarShadingOptions::default(),
            self.show_weapons,
        ) else {
            target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
            return;
        };

        let stride = self.buffer.stride();
        let dst_w = self.buffer.dim().width as usize;
        let dst_h = self.buffer.dim().height as usize;
        draw2d.blend_scale_chunk(
            self.buffer.pixels_mut(),
            &(0, 0, dst_w, dst_h),
            stride,
            &out.rgba,
            &(out.size as usize, out.size as usize),
        );
        target.blend_into(self.rect.x as i32, self.rect.y as i32, &self.buffer);
    }
}
