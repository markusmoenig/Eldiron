use crate::prelude::*;

pub struct ImportPalette {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ImportPalette {
    fn new() -> Self
    where
        Self: Sized,
    {
        let nodeui: TheNodeUI = TheNodeUI::default();

        Self {
            id: TheId::named(&fl!("action_import_palette")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_import_palette_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.palette_tool_active
    }

    fn apply_project(
        &self,
        _project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) {
        ctx.ui.open_file_requester(
            TheId::named_with_id("actionImportPalette", Uuid::new_v4()),
            "Switch Project Palette".into(),
            TheFileExtension::new(
                "Palette".into(),
                vec![
                    "txt".to_string(),
                    "TXT".to_string(),
                    "hex".to_string(),
                    "HEX".to_string(),
                    "gpl".into(),
                    "pal".into(),
                ],
            ),
        );
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        _event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }
}
