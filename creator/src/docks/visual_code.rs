use crate::docks::visual_code_undo::*;
use crate::prelude::*;
use codegridfx::DebugModule;
use codegridfx::Module;
use theframework::prelude::*;

/// Unique identifier for entities being edited
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    World,
    Region(Uuid),
    CharacterInstance(Uuid, Uuid), // (region_id, instance_id)
    CharacterTemplate(Uuid),
    ItemInstance(Uuid, Uuid), // (region_id, instance_id)
    ItemTemplate(Uuid),
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
        if server_ctx.pc.is_world_visual_code() {
            self.module = project.world_module.clone();
            self.module.module_type = codegridfx::ModuleType::World;
            self.module.view_name = "DockVisualScripting".into();
            self.module.redraw(ui, ctx);
            self.switch_to_entity(EntityKey::World, ctx);
        } else if let Some(id) = server_ctx.pc.id() {
            if server_ctx.pc.is_region_visual_code() {
                if let Some(region) = project.get_region(&id) {
                    self.module = region.module.clone();
                    self.module.module_type = codegridfx::ModuleType::Region;
                    self.module.view_name = "DockVisualScripting".into();
                    self.module.redraw(ui, ctx);
                    self.switch_to_entity(EntityKey::Region(id), ctx);
                }
            } else if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id() {
                if let Some(region) = project.get_region(&id) {
                    if let Some(character_instance) = region.characters.get(&instance_id) {
                        self.module = character_instance.module.clone();
                        self.module
                            .set_module_type(codegridfx::ModuleType::CharacterInstance);

                        self.module.view_name = "DockVisualScripting".into();
                        self.module.redraw(ui, ctx);
                        self.switch_to_entity(EntityKey::CharacterInstance(id, instance_id), ctx);
                    }
                }
            } else if let Some(instance_id) = server_ctx.pc.get_region_item_instance_id() {
                if let Some(region) = project.get_region(&id) {
                    if let Some(item_instance) = region.items.get(&instance_id) {
                        self.module = item_instance.module.clone();
                        self.module
                            .set_module_type(codegridfx::ModuleType::ItemInstance);

                        self.module.view_name = "DockVisualScripting".into();
                        self.module.redraw(ui, ctx);
                        self.switch_to_entity(EntityKey::ItemInstance(id, instance_id), ctx);
                    }
                }
            } else if server_ctx.pc.is_character() {
                if let Some(character) = project.characters.get(&id) {
                    self.module = character.module.clone();
                    // Keep stored routines as-is; avoid re-running default-routine insertion
                    // on each dock activation.
                    self.module.module_type = codegridfx::ModuleType::CharacterTemplate;
                    self.module.view_name = "DockVisualScripting".into();
                    self.module.redraw(ui, ctx);
                    self.switch_to_entity(EntityKey::CharacterTemplate(id), ctx);
                }
            } else if server_ctx.pc.is_item() {
                if let Some(item) = project.items.get(&id) {
                    self.module = item.module.clone();
                    // Keep stored routines as-is; avoid re-running default-routine insertion
                    // on each dock activation.
                    self.module.module_type = codegridfx::ModuleType::ItemTemplate;
                    self.module.view_name = "DockVisualScripting".into();
                    self.module.redraw(ui, ctx);
                    self.switch_to_entity(EntityKey::ItemTemplate(id), ctx);
                }
            }
        }
    }

    fn apply_debug_data(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
        debug: &DebugModule,
    ) {
        if let Some(runtime_id) = self.runtime_debug_id(project, server_ctx) {
            self.module.redraw_debug(ui, ctx, runtime_id, debug);
        } else {
            self.module.redraw(ui, ctx);
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
                        // Guard against duplicate "ModuleChanged" emissions with unchanged state.
                        if prev.to_json() != self.module.to_json() {
                            let atom =
                                VisualCodeUndoAtom::ModuleEdit(prev.clone(), self.module.clone());
                            self.add_undo(atom, ctx);
                        }
                    }

                    // Store current module as previous for next change
                    self.prev_module = Some(self.module.clone());

                    self.update_project_module(project, server_ctx);
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

    fn mark_saved(&mut self) {
        for undo in self.entity_undos.values_mut() {
            undo.index = -1;
        }
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
        let code = self.module.build(false);
        let debug_code = self.module.build(true);

        match self.current_entity {
            Some(EntityKey::World) => {
                project.world_module = self.module.clone();
                project.world_source = code;
                project.world_source_debug = debug_code;
            }
            Some(EntityKey::Region(id)) => {
                if let Some(region) = project.get_region_mut(&id) {
                    region.module = self.module.clone();
                    region.source = code;
                    region.source_debug = debug_code;
                }
            }
            Some(EntityKey::CharacterInstance(region_id, instance_id)) => {
                if let Some(region) = project.get_region_mut(&region_id) {
                    if let Some(character_instance) = region.characters.get_mut(&instance_id) {
                        character_instance.module = self.module.clone();
                        character_instance.source = code;
                        character_instance.source_debug = debug_code;
                    }
                }
            }
            Some(EntityKey::CharacterTemplate(id)) => {
                if let Some(character) = project.characters.get_mut(&id) {
                    character.module = self.module.clone();
                    character.source = code;
                    character.source_debug = debug_code;
                }
            }
            Some(EntityKey::ItemInstance(region_id, instance_id)) => {
                if let Some(region) = project.get_region_mut(&region_id) {
                    if let Some(item_instance) = region.items.get_mut(&instance_id) {
                        item_instance.module = self.module.clone();
                        item_instance.source = code;
                        item_instance.source_debug = debug_code;
                    }
                }
            }
            Some(EntityKey::ItemTemplate(id)) => {
                if let Some(item) = project.items.get_mut(&id) {
                    item.module = self.module.clone();
                    item.source = code;
                    item.source_debug = debug_code;
                }
            }
            None => {
                let _ = server_ctx;
            }
        }
    }

    fn runtime_debug_id(&self, project: &Project, server_ctx: &ServerContext) -> Option<u32> {
        let region = project.get_region(&server_ctx.curr_region)?;

        match server_ctx.cc {
            ContentContext::CharacterInstance(instance_id) => region
                .map
                .entities
                .iter()
                .find(|entity| entity.creator_id == instance_id)
                .map(|entity| entity.id),
            ContentContext::CharacterTemplate(template_id) => region
                .characters
                .values()
                .find(|instance| instance.character_id == template_id)
                .and_then(|instance| {
                    region
                        .map
                        .entities
                        .iter()
                        .find(|entity| entity.creator_id == instance.id)
                        .map(|entity| entity.id)
                }),
            ContentContext::ItemInstance(instance_id) => region
                .map
                .items
                .iter()
                .find(|item| item.creator_id == instance_id)
                .map(|item| item.id),
            ContentContext::ItemTemplate(template_id) => region
                .items
                .values()
                .find(|instance| instance.item_id == template_id)
                .and_then(|instance| {
                    region
                        .map
                        .items
                        .iter()
                        .find(|item| item.creator_id == instance.id)
                        .map(|item| item.id)
                }),
            _ => None,
        }
    }
}
