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

    fn apply_project(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // If the update sector is a screen widget, refresh the screen list to show potential new names
        if let ProjectContext::ScreenWidget(id, widget_id) = server_ctx.pc {
            if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                if let Some(node) = tree_layout.get_node_by_id_mut(&id) {
                    if let Some(screen) = project.screens.get(&id) {
                        gen_screen_tree_items(node, screen);
                    }
                    node.new_item_selected(&TheId::named_with_id_and_reference(
                        "Screen Content List Item",
                        widget_id,
                        id,
                    ));
                }
            }
        }
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
