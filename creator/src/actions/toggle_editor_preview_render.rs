use crate::editor::RUSTERIX;
use crate::prelude::*;

fn set_dirty_and_status(ctx: &mut TheContext, status: String) {
    RUSTERIX.write().unwrap().set_dirty();
    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
}

fn editor_preview_render_applicable(server_ctx: &ServerContext) -> bool {
    server_ctx.get_map_context() == MapContext::Region
        && !server_ctx.pc.is_prefab()
        && server_ctx.editor_view_mode != EditorViewMode::D2
}

pub struct ToggleEditorPreviewPost {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ToggleEditorPreviewPost {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_editor_preview_post_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_editor_preview_post")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_editor_preview_post_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        editor_preview_render_applicable(server_ctx)
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let enabled = {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.editor_preview_post_enabled = !rusterix.editor_preview_post_enabled;
            rusterix.editor_preview_post_enabled
        };
        set_dirty_and_status(
            ctx,
            if enabled {
                fl!("status_editor_preview_post_on")
            } else {
                fl!("status_editor_preview_post_off")
            },
        );
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

pub struct ToggleEditorPreviewLighting {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ToggleEditorPreviewLighting {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_editor_preview_lighting_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_editor_preview_lighting")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_editor_preview_lighting_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        editor_preview_render_applicable(server_ctx)
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let enabled = {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.editor_preview_lighting_enabled = !rusterix.editor_preview_lighting_enabled;
            rusterix.editor_preview_lighting_enabled
        };
        set_dirty_and_status(
            ctx,
            if enabled {
                fl!("status_editor_preview_lighting_on")
            } else {
                fl!("status_editor_preview_lighting_off")
            },
        );
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
    fn preview_post_and_lighting_are_region_only() {
        let mut server_ctx = ServerContext::default();
        server_ctx.editor_view_mode = EditorViewMode::Orbit;
        assert!(editor_preview_render_applicable(&server_ctx));

        server_ctx.pc = ProjectContext::Prefab(Uuid::new_v4());
        assert!(!editor_preview_render_applicable(&server_ctx));
    }
}
