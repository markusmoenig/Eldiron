use crate::editor::DOCKMANAGER;
use crate::prelude::*;

pub struct CopyTileID {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for CopyTileID {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_copy_tile_id_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_copy_tile_id")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_copy_tile_id_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Tiles"
            && (server_ctx.curr_tile_source.is_some() || server_ctx.curr_tile_id.is_some())
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let id = match server_ctx.curr_tile_source {
            Some(rusterix::TileSource::TileGroup(group_id)) => Some(group_id),
            Some(rusterix::TileSource::SingleTile(tile_id)) => Some(tile_id),
            Some(rusterix::TileSource::TileGroupMember { .. }) => server_ctx.curr_tile_id,
            Some(rusterix::TileSource::Procedural(id)) => Some(id),
            None => server_ctx.curr_tile_id,
        };

        if let Some(id) = id {
            let txt = format!("\"{id}\"");
            ctx.ui.clipboard = Some(TheValue::Text(txt.clone()));
            let mut clipboard = arboard::Clipboard::new().unwrap();
            clipboard.set_text(txt.clone()).unwrap();
        }

        None
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
