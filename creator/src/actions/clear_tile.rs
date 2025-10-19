use crate::prelude::*;

pub struct ClearTile {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ClearTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Clears the tiles from the selected sectors.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Clear Tile"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Clears the tiles of the selected sectors."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Property
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if map.selected_sectors.is_empty() {
            return false;
        }
        match server_ctx.editor_view_mode {
            EditorViewMode::D2 => server_ctx.editing_surface.is_none(),
            _ => true,
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                if sector.properties.contains("source") {
                    sector.properties.remove("source");
                    changed = true;
                }
            }
        }

        if changed {
            Some(RegionUndoAtom::MapEdit(
                Box::new(prev),
                Box::new(map.clone()),
            ))
        } else {
            None
        }
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
