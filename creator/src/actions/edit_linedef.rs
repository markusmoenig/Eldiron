use crate::prelude::*;
use std::str::FromStr;

pub const EDIT_LINEDEF_ACTION_ID: &str = "284638fa-5769-442a-a55e-88121a37f193";

pub struct EditLinedef {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditLinedef {
    fn build_nodeui() -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::Text(
            "actionLinedefName".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));

        nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), "".into()));

        nodeui
    }
}

impl Action for EditLinedef {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_linedef"),
                Uuid::from_str(EDIT_LINEDEF_ACTION_ID).unwrap(),
            ),
            nodeui: Self::build_nodeui(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_linedef_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode == EditorViewMode::D2 && !map.selected_linedefs.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(linedef_id) = map.selected_linedefs.first()
            && let Some(linedef) = map.find_linedef(*linedef_id)
        {
            self.nodeui
                .set_text_value("actionLinedefName", linedef.name.clone());
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        let name = self
            .nodeui
            .get_text_value("actionLinedefName")
            .unwrap_or_default();

        for linedef_id in map.selected_linedefs.clone() {
            if let Some(linedef) = map.find_linedef_mut(linedef_id)
                && name != linedef.name
            {
                linedef.name = name.clone();
                changed = true;
            }
        }

        if changed {
            Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
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

    fn hud_material_slots(
        &self,
        map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        crate::actions::builder_hud_material_slots_for_selected_linedef(map)
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
