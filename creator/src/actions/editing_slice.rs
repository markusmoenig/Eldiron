use crate::prelude::*;

pub struct EditingSlice {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EditingSlice {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::IntEditSlider(
            "actionEditingSlicePos".into(),
            "".into(),
            "".into(),
            0,
            -100000..=100000,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::IntEditSlider(
            "actionEditingSliceHeight".into(),
            "".into(),
            "".into(),
            2,
            1..=10,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_editing_slice")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_editing_slice_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &CreatorContext) -> bool {
        server_ctx.shared.editing_surface.is_none() && server_ctx.shared.editor_view_mode == EditorViewMode::D2
    }

    fn apply_project(
        &self,
        _project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut CreatorContext,
    ) {
        let pos = self
            .nodeui
            .get_i32_value("actionEditingSlicePos")
            .unwrap_or(0);
        let height = self
            .nodeui
            .get_i32_value("actionEditingSliceHeight")
            .unwrap_or(2)
            .max(1);

        server_ctx.shared.editing_slice = pos as f32;
        server_ctx.shared.editing_slice_height = height as f32;

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Client Properties"),
            TheValue::Empty,
        ));
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
        _server_ctx: &mut CreatorContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
