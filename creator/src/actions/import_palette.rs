use crate::{editor::UNDOMANAGER, prelude::*};

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
            "Load Palette".into(),
            TheFileExtension::new(
                "Palette".into(),
                vec![
                    "txt".to_string(),
                    "TXT".to_string(),
                    "hex".to_string(),
                    "HEX".to_string(),
                ],
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
                if id.name == "actionImportPalette" {
                    for p in paths {
                        if let Ok(contents) = std::fs::read_to_string(p) {
                            let prev = project.art_palette.clone();
                            let prev_materials = project.art_palette_materials.clone();

                            project.load_art_palette_from_text(contents);
                            apply_palette(ui, ctx, server_ctx, project);
                            crate::undo::project_helper::refresh_palette_runtime(project);

                            let undo_atom = ProjectUndoAtom::PaletteEdit(
                                prev,
                                prev_materials,
                                project.art_palette.clone(),
                                project.art_palette_materials.clone(),
                            );
                            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
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
