use crate::prelude::*;

pub struct EditSector {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EditSector {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "actionSectorName".into(),
            "Sector Name".into(),
            "Set the name of the sector.".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Edit the attributes of the selected sector.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Edit Sector"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Edit sector attributes."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        map.selected_sectors.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(sector_id) = map.selected_sectors.first() {
            if let Some(sector) = map.find_sector(*sector_id) {
                self.nodeui
                    .set_text_value("actionSectorName", sector.name.clone());
            }
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

        let name = self
            .nodeui
            .get_text_value("actionSectorName")
            .unwrap_or(String::new());

        if let Some(sector_id) = map.selected_sectors.first() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                if name != sector.name {
                    sector.name = name;
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
