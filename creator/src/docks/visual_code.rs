use crate::docks::visual_code_undo::*;
use crate::prelude::*;
use codegridfx::Module;
use theframework::prelude::*;

/// Unique identifier for entities being edited
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    CharacterInstance(Uuid, Uuid), // (region_id, instance_id)
    Character(Uuid),
    Item(Uuid),
}

pub struct VisualCodeDock {
    module: Module,
    // Per-entity undo stacks
    entity_undos: FxHashMap<EntityKey, VisualCodeUndo>,
    current_entity: Option<EntityKey>,
    max_undo: usize,
    prev_module: Option<Module>,
}

impl Dock for VisualCodeDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            module: Module::default(),
            entity_undos: FxHashMap::default(),
            current_entity: None,
            max_undo: 30,
            prev_module: None,
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
                        // Switch to this entity's undo stack
                        self.switch_to_entity(EntityKey::CharacterInstance(id, instance_id), ctx);
                    }
                }
            } else if server_ctx.pc.is_character() {
                if let Some(character) = project.characters.get(&id) {
                    self.module = character.module.clone();
                    self.module
                        .set_module_type(codegridfx::ModuleType::CharacterTemplate);
                    self.module.view_name = "DockVisualScripting".into();
                    self.module.redraw(ui, ctx);
                    // Switch to this entity's undo stack
                    self.switch_to_entity(EntityKey::Character(id), ctx);
                }
            } else if server_ctx.pc.is_item() {
                if let Some(item) = project.items.get(&id) {
                    self.module = item.module.clone();
                    self.module
                        .set_module_type(codegridfx::ModuleType::ItemTemplate);
                    self.module.view_name = "DockVisualScripting".into();
                    self.module.redraw(ui, ctx);
                    // Switch to this entity's undo stack
                    self.switch_to_entity(EntityKey::Item(id), ctx);
                }
            }
        }
    }

    fn import(
        &mut self,
        content: String,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        self.module = Module::from_json(&content);
        if let Some(prev) = &self.prev_module {
            self.module.id = prev.id;
        }
        self.module.redraw(ui, ctx);
        self.handle_event(
            &TheEvent::Custom(TheId::named("ModuleChanged"), TheValue::Empty),
            ui,
            ctx,
            project,
            server_ctx,
        );
    }

    fn export(&self) -> Option<String> {
        Some(self.module.to_json())
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
                    // Add undo atom before applying the change
                    if let Some(prev) = &self.prev_module {
                        let atom =
                            VisualCodeUndoAtom::ModuleEdit(prev.clone(), self.module.clone());
                        self.add_undo(atom, ctx);
                    }

                    // Store current module as previous for next change
                    self.prev_module = Some(self.module.clone());

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

    fn supports_undo(&self) -> bool {
        true
    }

    fn has_changes(&self) -> bool {
        // Check if any entity has changes (index >= 0, meaning not fully undone)
        self.entity_undos.values().any(|undo| undo.has_changes())
    }

    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get_mut(&entity_key) {
                undo.undo(&mut self.module, ui, ctx);
                self.prev_module = Some(self.module.clone());
                self.set_undo_state_to_ui(ctx);

                // Update the project with the undone module
                self.update_project_module(project, server_ctx);
            }
        }
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get_mut(&entity_key) {
                undo.redo(&mut self.module, ui, ctx);
                self.prev_module = Some(self.module.clone());
                self.set_undo_state_to_ui(ctx);

                // Update the project with the redone module
                self.update_project_module(project, server_ctx);
            }
        }
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get(&entity_key) {
                if undo.has_undo() {
                    ctx.ui.set_enabled("Undo");
                } else {
                    ctx.ui.set_disabled("Undo");
                }

                if undo.has_redo() {
                    ctx.ui.set_enabled("Redo");
                } else {
                    ctx.ui.set_disabled("Redo");
                }
                return;
            }
        }

        // No entity selected or no undo stack
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
    }
}

impl VisualCodeDock {
    /// Switch to a different entity and update undo button states
    fn switch_to_entity(&mut self, entity_key: EntityKey, ctx: &mut TheContext) {
        self.current_entity = Some(entity_key);
        self.prev_module = Some(self.module.clone());
        self.set_undo_state_to_ui(ctx);
    }

    /// Add an undo atom to the current entity's undo stack
    fn add_undo(&mut self, atom: VisualCodeUndoAtom, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity {
            let undo = self
                .entity_undos
                .entry(entity_key)
                .or_insert_with(VisualCodeUndo::new);
            undo.add(atom);
            undo.truncate_to_limit(self.max_undo);
            self.set_undo_state_to_ui(ctx);
        }
    }

    /// Update the project with the current module state
    fn update_project_module(&mut self, project: &mut Project, server_ctx: &mut ServerContext) {
        if let Some(id) = server_ctx.pc.id() {
            let code = self.module.build(false);
            let debug_code = self.module.build(true);

            if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id() {
                if let Some(region) = project.get_region_mut(&id) {
                    if let Some(character_instance) = region.characters.get_mut(&instance_id) {
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
                    item.source = code;
                    item.source_debug = debug_code;
                }
            }
        }
    }
}
