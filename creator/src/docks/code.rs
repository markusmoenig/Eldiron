use crate::prelude::*;
use codegridfx::Module;
use theframework::prelude::*;

pub struct CodeDock {
    module: Module,
}

impl Dock for CodeDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            module: Module::default(),
        }
    }

    fn setup(&mut self, ctx: &mut TheContext) -> TheCanvas {
        self.module.build_canvas(ctx, "DockCodeEditor")
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(id) = server_ctx.pc.id() {
            if server_ctx.pc.is_character() {
                if let Some(character) = project.characters.get(&id) {
                    self.module = character.module.clone();
                    self.module
                        .set_module_type(codegridfx::ModuleType::CharacterTemplate);
                    self.module.view_name = "DockCodeEditor".into();
                    self.module.redraw(ui, ctx);
                }
            } else if server_ctx.pc.is_item() {
                if let Some(item) = project.items.get(&id) {
                    self.module = item.module.clone();
                    self.module
                        .set_module_type(codegridfx::ModuleType::ItemTemplate);
                    self.module.view_name = "DockCodeEditor".into();
                    self.module.redraw(ui, ctx);
                }
            }
        }
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = self.module.handle_event(event, ui, ctx, &project.palette);

        match event {
            TheEvent::Custom(id, _) => {
                if id.name == "ModuleChanged" {
                    if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_character() {
                            if let Some(character) = project.characters.get_mut(&id) {
                                character.module = self.module.clone();
                            }
                        } else if server_ctx.pc.is_item() {
                            if let Some(item) = project.items.get_mut(&id) {
                                item.module = self.module.clone();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
