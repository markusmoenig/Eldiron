use crate::prelude::*;
use crate::procedural_material::map_composition;

pub struct ComposeMaterials {
    id: TheId,
}
impl Action for ComposeMaterials {
    fn new() -> Self {
        Self {
            id: TheId::named("Compose Materials"),
        }
    }
    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        "Edit a split/blend layer stack on the selected map surface.".into()
    }
    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }
    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server: &ServerContext) -> bool {
        server.pc.is_region() && map_composition::selection(map, server).is_some()
    }
    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if let Some(binding) = map_composition::selection(map, server) {
            ctx.ui.send(TheEvent::Custom(
                TheId::named(map_composition::OPEN),
                TheValue::Text(serde_json::to_string(&binding).unwrap()),
            ));
        }
        None
    }
    fn params(&self) -> TheNodeUI {
        TheNodeUI::default()
    }
    fn handle_event(
        &mut self,
        _event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server: &mut ServerContext,
    ) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_uses_map_selection_without_a_picker_destination() {
        let action = ComposeMaterials::new();
        let mut map = Map::default();
        map.sectors.push(rusterix::Sector::new(1, vec![]));
        map.selected_sectors = vec![1];
        let mut server = ServerContext::default();
        server.pc = ProjectContext::Region(Uuid::new_v4());
        server.curr_map_tool_type = MapToolType::Sector;
        server.selected_hud_icon_index = 0;
        let mut ctx = TheContext::new(64, 64, 1.);
        assert!(matches!(action.role(), ActionRole::Editor));
        assert!(action.is_applicable(&map, &mut ctx, &server));
        assert!(server.curr_tile_board_cell.is_none());
        map.sectors.push(rusterix::Sector::new(2, vec![]));
        map.selected_sectors.push(2);
        assert!(!action.is_applicable(&map, &mut ctx, &server));
    }
}
