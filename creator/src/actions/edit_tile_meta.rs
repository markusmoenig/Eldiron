use crate::prelude::*;

pub struct EditTileMeta {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EditTileMeta {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let mut roles = vec![];
        for dir in TileRole::iterator() {
            roles.push(dir.to_string().to_string());
        }
        let item = TheNodeUIItem::Selector(
            "actionTileRole".into(),
            "Role".into(),
            "Edit the role of the tile.".into(),
            roles,
            1,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Selector(
            "actionTileBlocking".into(),
            "Blocking".into(),
            "Edit if the tile is blocking (for 2D games only).".into(),
            vec!["No".to_string(), "Yes".to_string()],
            0,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Text(
            "actionTileTags".into(),
            "Tags".into(),
            "Edit the tags of the tile.".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Edit the meta data of the currently selected tile.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Edit Tile Meta Data"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Edit the meta data of the selected tile."
    }

    fn role(&self) -> ActionRole {
        ActionRole::UI
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker
            && server_ctx.curr_tile_id.is_some()
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.get_tile(&tile_id) {
                self.nodeui
                    .set_i32_value("actionTileRole", tile.role as i32);
                self.nodeui
                    .set_i32_value("actionTileBlocking", if tile.blocking { 1 } else { 0 });
                self.nodeui
                    .set_text_value("actionTileTags", tile.name.clone());
            }
        }
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let role = self.nodeui.get_i32_value("actionTileRole").unwrap_or(0);
        let blocking = self.nodeui.get_i32_value("actionTileBlocking").unwrap_or(0);
        let name = self
            .nodeui
            .get_text_value("actionTileTags")
            .unwrap_or(String::new());

        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.get_tile_mut(&tile_id) {
                tile.role = TileRole::from_index(role as u8).unwrap_or(TileRole::ManMade);
                tile.blocking = blocking == 1;
                tile.name = name;
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
