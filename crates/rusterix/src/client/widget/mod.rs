pub mod avatar;
pub mod deco;
pub mod game;
pub mod game_backend;
pub mod messages;
pub mod screen;
pub mod text;

use crate::{
    Assets, Entity, Map, Pixel, PlayerCamera, Rect, Texture, Value, WHITE, client::draw2d,
};
use draw2d::Draw2D;
use theframework::prelude::*;

/// Used right now for button widgets
pub struct Widget {
    pub name: String,
    pub id: u32,
    pub rect: Rect,
    pub action: String,
    pub intent: Option<String>,
    pub spell: Option<String>,
    pub group: Option<String>,
    pub show: Option<Vec<String>>,
    pub hide: Option<Vec<String>>,
    pub deactivate: Vec<String>,
    pub camera: Option<PlayerCamera>,
    pub player_camera: Option<PlayerCamera>,
    pub camera_target: Option<String>,
    pub party: Option<String>,
    pub inventory_index: Option<usize>,
    pub equipped_slot: Option<String>,
    pub portrait: bool,
    pub drag_drop: bool,
    pub textures: Vec<Texture>,
    pub entity_cursor_id: Option<Uuid>,
    pub entity_clicked_cursor_id: Option<Uuid>,
    pub item_cursor_id: Option<Uuid>,
    pub item_clicked_cursor_id: Option<Uuid>,
    pub border_color: Pixel,
    pub border_size: i32,
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
            spell: None,
            group: None,
            show: None,
            hide: None,
            deactivate: vec![],
            camera: None,
            player_camera: None,
            camera_target: None,
            party: None,
            inventory_index: None,
            equipped_slot: None,
            portrait: false,
            drag_drop: false,
            textures: vec![],
            entity_cursor_id: None,
            entity_clicked_cursor_id: None,
            item_cursor_id: None,
            item_clicked_cursor_id: None,
            border_color: WHITE,
            border_size: 0,
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _map: &Map,
        assets: &Assets,
        entity: Option<&Entity>,
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

        let entity = entity;
        let item_to_draw = if let Some(inventory_index) = &self.inventory_index {
            entity.and_then(|entity| {
                entity
                .inventory
                .get(*inventory_index)
                .and_then(|item| item.as_ref())
            })
        } else if let Some(slot) = &self.equipped_slot {
            entity.and_then(|entity| entity.get_equipped_item(slot))
        } else {
            None
        };

        if self.portrait
            && let Some(entity) = entity
            && let Some(tile) = Self::portrait_tile_for_entity(entity, assets)
        {
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
        } else if let Some(item) = item_to_draw
            && let Some(Value::Source(source)) = item.attributes.get("source")
            && let Some(tile) = source.tile_from_tile_list(assets)
        {
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

        if self.border_size > 0 {
            draw2d.rect_outline_thickness(
                buffer.pixels_mut(),
                &(
                    self.rect.x as usize,
                    self.rect.y as usize,
                    self.rect.width as usize,
                    self.rect.height as usize,
                ),
                stride,
                &self.border_color,
                self.border_size as usize,
            );
        }
    }

    fn portrait_tile_for_entity(entity: &Entity, assets: &Assets) -> Option<crate::Tile> {
        if let Some(source) = entity.attributes.get_source("portrait_tile_id") {
            return source.tile_from_tile_list(assets);
        }
        if let Some(id) = entity.attributes.get_id("portrait_tile_id") {
            return assets.tiles.get(&id).cloned();
        }
        entity
            .attributes
            .get_str("portrait_tile_id")
            .and_then(|value| Uuid::parse_str(value.trim()).ok())
            .and_then(|id| assets.tiles.get(&id).cloned())
    }
}
