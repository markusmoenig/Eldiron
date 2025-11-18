use crate::prelude::*;

pub struct DuplicateTile {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for DuplicateTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Duplicates the currently selected tile.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Duplicate Tile"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Duplicates the currently selected tile."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker
            && server_ctx.curr_tile_id.is_some()
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(mut tile) = project.tiles.get(&tile_id).cloned() {
                tile.id = Uuid::new_v4();
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
        }
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
