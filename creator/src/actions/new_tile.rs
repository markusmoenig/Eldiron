use rusterix::Texture;

use crate::editor::DOCKMANAGER;
use crate::prelude::*;

pub struct NewTile {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for NewTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::IntEditSlider(
            "actionNewTileSize".into(),
            "".into(),
            "".into(),
            16,
            8..=64,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::IntEditSlider(
            "actionNewTileFrames".into(),
            "".into(),
            "".into(),
            1,
            1..=8,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_new_tile")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_new_tile_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(
        &self,
        _map: &Map,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Tiles"
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) {
        let size = self
            .nodeui
            .get_i32_value("actionNewTileSize")
            .unwrap_or(16)
            .clamp(8, 64) as u32;

        let frames = self
            .nodeui
            .get_i32_value("actionNewTileFrames")
            .unwrap_or(1)
            .clamp(1, 8) as u32;

        let mut textures = vec![];
        let mut c = [128, 128, 128, 255];
        if let Some(col) = project.palette.get_current_color() {
            c = col.to_u8_array();
        }

        for _ in 0..frames {
            let mut texture = Texture::alloc(size as usize, size as usize);
            texture.fill(c);
            textures.push(texture);
        }

        let mut tile = rusterix::Tile::from_textures(textures);
        tile.set_default_materials();
        project.tiles.insert(tile.id, tile);

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tiles"),
            TheValue::Empty,
        ));
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
