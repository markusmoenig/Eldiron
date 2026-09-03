use crate::prelude::*;

/// Prefab-only presentation tool. The Prefab editor owns effect authoring and
/// uses this tool selection to expose its particle/light dock.
pub struct EffectsTool {
    id: TheId,
}

impl Tool for EffectsTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Prefab Effects Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
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
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            ToolEvent::Activate => {
                server_ctx.curr_map_tool_type = MapToolType::Effects;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            ToolEvent::DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.selected_prefab_effect_id = None;
                true
            }
            _ => false,
        }
    }
}
