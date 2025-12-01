use crate::{editor::DOCKMANAGER, prelude::*};

pub struct ImportVCode {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ImportVCode {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_import_vcode_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_import_vcode")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_import_vcode_desc")
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
        ctx.ui.open_file_requester(
            TheId::named_with_id("actionImportVisualCode", Uuid::new_v4()),
            "Import Visual Code".into(),
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
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::FileRequesterResult(id, paths) => {
                if id.name == "actionImportVisualCode" {
                    for p in paths {
                        if let Ok(contents) = std::fs::read_to_string(p) {
                            DOCKMANAGER
                                .write()
                                .unwrap()
                                .import(contents, ui, ctx, project, server_ctx);
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }
}
