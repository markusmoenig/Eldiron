use crate::prelude::*;

/// Prefab-only presentation tool. The Prefab editor owns effect authoring and
/// uses this tool selection to expose its particle/light dock.
pub struct EffectsTool {
    id: TheId,
    view_input: Option<ToolViewInput>,
}

impl Tool for EffectsTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Prefab Effects Tool"),
            view_input: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn set_view_input(&mut self, input: Option<ToolViewInput>) {
        self.view_input = input;
    }
    fn view_input_camera(&self) -> Option<scenevm::Camera3D> {
        self.view_input.as_ref()?.camera
    }
    fn prepare_overlay(&mut self, input: ToolOverlayInputs<'_>) -> Option<ToolOverlayFrame> {
        ToolOverlayFrame::passive(
            self,
            input,
            crate::editor_overlay::EditorOverlay::new(input.target.size),
        )
    }
    fn overlay_input_current(
        &mut self,
        _frame: &ToolOverlayFrame,
        _target: ToolOverlayTarget,
        _map: &Map,
        _server: &CreatorContext,
    ) -> bool {
        true
    }
    fn overlay_command_source_current(
        &mut self,
        _frame: &ToolOverlayFrame,
        _target: ToolOverlayTarget,
        _map: &Map,
        _server: &CreatorContext,
    ) -> bool {
        true
    }
    #[cfg(feature = "native-ui")]
    fn presented_overlay(
        &mut self,
        _frame: &ToolOverlayFrame,
        _receipts: &[theframework::paint::FrameReceipt],
        _target: ToolOverlayTarget,
        _map: &Map,
        _server: &mut CreatorContext,
    ) -> bool {
        true
    }
    fn info(&self) -> String {
        "Prefab particles and lights".to_string()
    }

    fn icon_name(&self) -> String {
        "light_small".to_string()
    }

    fn tool_event(
        &mut self,
        event: ToolEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut CreatorContext,
    ) -> bool {
        match event {
            ToolEvent::Activate => {
                server_ctx.shared.curr_map_tool_type = MapToolType::Effects;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            ToolEvent::DeActivate => {
                server_ctx.shared.curr_map_tool_type = MapToolType::General;
                server_ctx.shared.selected_prefab_effect_id = None;
                true
            }
            _ => false,
        }
    }
}
