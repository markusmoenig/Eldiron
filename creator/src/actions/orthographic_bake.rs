use crate::editor::RUSTERIX;
use crate::prelude::*;

const SAMPLE_COUNT_ID: &str = "actionBakeSamples";

fn bake_applicable(server_ctx: &ServerContext) -> bool {
    server_ctx.get_map_context() == MapContext::Region
        && !server_ctx.pc.is_prefab()
        && server_ctx.editor_view_mode == EditorViewMode::Iso
}

pub struct RenderOrthographicBake {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for RenderOrthographicBake {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            SAMPLE_COUNT_ID.into(),
            "".into(),
            "".into(),
            32,
            1..=256,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_bake_render_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_bake_render")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_bake_render_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        bake_applicable(server_ctx)
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let samples = self
            .nodeui
            .get_i32_value(SAMPLE_COUNT_ID)
            .unwrap_or(32)
            .clamp(1, 256) as u32;
        let progress = {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.request_orthographic_bake(map.id, samples);
            rusterix.orthographic_bake.progress_text()
        };
        server_ctx.background_progress = progress;
        ctx.ui.redraw_all = true;
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            fl!("status_bake_requested"),
        ));
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

pub struct ToggleOrthographicBakeVisibility {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ToggleOrthographicBakeVisibility {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_bake_toggle_visibility_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_bake_toggle_visibility")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_bake_toggle_visibility_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        bake_applicable(server_ctx)
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let status = match RUSTERIX
            .write()
            .unwrap()
            .toggle_orthographic_bake_visibility()
        {
            Some(true) => fl!("status_bake_visible"),
            Some(false) => fl!("status_bake_hidden"),
            None => fl!("status_bake_missing"),
        };
        ctx.ui.redraw_all = true;
        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
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

pub struct ClearOrthographicBake {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ClearOrthographicBake {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_bake_clear_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_bake_clear")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_bake_clear_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        bake_applicable(server_ctx)
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        RUSTERIX.write().unwrap().clear_orthographic_bake();
        map.orthographic_bake = None;
        map.changed = map.changed.wrapping_add(1);
        server_ctx.background_progress = None;
        ctx.ui.redraw_all = true;
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            fl!("status_bake_cleared"),
        ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bake_actions_are_available_only_in_the_orthographic_region_view() {
        let mut server_ctx = ServerContext::default();
        server_ctx.editor_view_mode = EditorViewMode::Iso;
        assert!(bake_applicable(&server_ctx));

        server_ctx.editor_view_mode = EditorViewMode::Orbit;
        assert!(!bake_applicable(&server_ctx));
    }
}
