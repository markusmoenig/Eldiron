use crate::{editor::DOCKMANAGER, prelude::*};

pub struct ExportVCode {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ExportVCode {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_export_vcode_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_export_vcode")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_export_vcode_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(
        &self,
        _map: &Map,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Visual Code"
    }

    fn apply_project(
        &self,
        _project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) {
        ctx.ui.save_file_requester(
            TheId::named_with_id("actionExportVisualCode", Uuid::new_v4()),
            "Export Visual Code".into(),
            TheFileExtension::new(
                "Eldiron Visual Code".into(),
                vec!["eldiron_vcode".to_string()],
            ),
        );
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
        match event {
            TheEvent::FileRequesterResult(id, paths) => {
                if id.name == "actionExportVisualCode" {
                    if let Some(json) = DOCKMANAGER.read().unwrap().export() {
                        for p in paths {
                            let _ = std::fs::write(p.clone(), json.clone());
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }
}
