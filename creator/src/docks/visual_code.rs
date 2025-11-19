use crate::prelude::*;
use codegridfx::Module;
use theframework::prelude::*;

pub struct VisualCodeDock {
    module: Module,
}

impl Dock for VisualCodeDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            module: Module::default(),
        }
    }

    fn setup(&mut self, ctx: &mut TheContext) -> TheCanvas {
        self.module.build_canvas(ctx, "DockVisualScripting")
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(id) = server_ctx.pc.id() {
            if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id() {
                if let Some(region) = project.get_region(&id) {
                    if let Some(character_instance) = region.characters.get(&instance_id) {
                        self.module = character_instance.module.clone();
                        self.module
                            .set_module_type(codegridfx::ModuleType::CharacterInstance);
                        self.module.view_name = "DockVisualScripting".into();
                        self.module.redraw(ui, ctx);
                    }
                }
            } else if server_ctx.pc.is_character() {
                if let Some(character) = project.characters.get(&id) {
                    self.module = character.module.clone();
                    self.module
                        .set_module_type(codegridfx::ModuleType::CharacterTemplate);
                    self.module.view_name = "DockVisualScripting".into();
                    self.module.redraw(ui, ctx);
                }
            } else if server_ctx.pc.is_item() {
                if let Some(item) = project.items.get(&id) {
                    self.module = item.module.clone();
                    self.module
                        .set_module_type(codegridfx::ModuleType::ItemTemplate);
                    self.module.view_name = "DockVisualScripting".into();
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
                        let code = self.module.build(false);
                        let debug_code = self.module.build(true);

                        if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id()
                        {
                            if let Some(region) = project.get_region_mut(&id) {
                                if let Some(character_instance) =
                                    region.characters.get_mut(&instance_id)
                                {
                                    character_instance.module = self.module.clone();
                                    character_instance.source = code;
                                    character_instance.source_debug = debug_code;
                                }
                            }
                        } else if server_ctx.pc.is_character() {
                            if let Some(character) = project.characters.get_mut(&id) {
                                character.module = self.module.clone();
                                character.source = code;
                                character.source_debug = debug_code;
                            }
                        } else if server_ctx.pc.is_item() {
                            if let Some(item) = project.items.get_mut(&id) {
                                item.module = self.module.clone();
                                item.module = self.module.clone();
                                item.source = code;
                                item.source_debug = debug_code;
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
