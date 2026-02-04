pub mod deco;
pub mod game;
pub mod messages;
pub mod screen;
pub mod text;

use crate::{Assets, Entity, Map, Rect, Texture, Value, client::draw2d};
use draw2d::Draw2D;
use theframework::prelude::*;

/// Used right now for button widgets
pub struct Widget {
    pub name: String,
    pub id: u32,
    pub rect: Rect,
    pub action: String,
    pub intent: Option<String>,
    pub show: Option<Vec<String>>,
    pub hide: Option<Vec<String>>,
    pub deactivate: Vec<String>,
    pub inventory_index: Option<usize>,
    pub textures: Vec<Texture>,
    pub entity_cursor_id: Option<Uuid>,
    pub entity_clicked_cursor_id: Option<Uuid>,
    pub item_cursor_id: Option<Uuid>,
    pub item_clicked_cursor_id: Option<Uuid>,
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            id: 0,
            rect: Rect::default(),
            action: String::new(),
            intent: None,
            show: None,
            hide: None,
            deactivate: vec![],
            inventory_index: None,
            textures: vec![],
            entity_cursor_id: None,
            entity_clicked_cursor_id: None,
            item_cursor_id: None,
            item_clicked_cursor_id: None,
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _map: &Map,
        assets: &Assets,
        entity: &Entity,
        draw2d: &Draw2D,
        animation_frame: &usize,
        texture_index: usize,
    ) {
        let stride = buffer.stride();

        if !self.textures.is_empty() {
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    self.rect.x as usize,
                    self.rect.y as usize,
                    self.rect.width as usize,
                    self.rect.height as usize,
                ),
                stride,
                &self.textures[texture_index].data,
                &(
                    self.textures[texture_index].width as usize,
                    self.textures[texture_index].height as usize,
                ),
            );
        }

        if let Some(inventory_index) = &self.inventory_index {
            if let Some(item) = entity.inventory.get(*inventory_index) {
                if let Some(item) = item {
                    if let Some(Value::Source(source)) = item.attributes.get("source") {
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            let index = *animation_frame % tile.textures.len();
                            let rect = self.rect.with_border(4.0);
                            draw2d.blend_scale_chunk(
                                buffer.pixels_mut(),
                                &(
                                    rect.x as usize,
                                    rect.y as usize,
                                    rect.width as usize,
                                    rect.height as usize,
                                ),
                                stride,
                                &tile.textures[index].data,
                                &(
                                    tile.textures[index].width as usize,
                                    tile.textures[index].height as usize,
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}
