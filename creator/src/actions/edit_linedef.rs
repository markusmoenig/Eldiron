use crate::prelude::*;

pub struct EditLinedef {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EditLinedef {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "actionLinedefName".into(),
            "Linedef Name".into(),
            "Set the name of the linedef.".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Edit the attributes of the selected linedef.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Edit Linedef"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Edit linedef attributes."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        map.selected_linedefs.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(linedef_id) = map.selected_linedefs.first() {
            if let Some(linedef) = map.find_linedef(*linedef_id) {
                self.nodeui
                    .set_text_value("actionLinedefName", linedef.name.clone());
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
            .get_text_value("actionLinedefName")
            .unwrap_or(String::new());

        if let Some(linedef_id) = map.selected_linedefs.first() {
            if let Some(linedef) = map.find_linedef_mut(*linedef_id) {
                if name != linedef.name {
                    linedef.name = name;
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
