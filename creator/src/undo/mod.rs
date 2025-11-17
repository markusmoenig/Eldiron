pub mod character_undo;
pub mod item_undo;
pub mod material_undo;
pub mod palette_undo;
pub mod project_atoms;
pub mod project_helper;
pub mod project_undo;
pub mod region_undo;
pub mod screen_undo;

use crate::prelude::*;
use character_undo::*;
use item_undo::*;
use material_undo::*;
use project_undo::*;
use screen_undo::*;

#[derive(PartialEq, Clone, Debug)]
pub enum UndoManagerContext {
    None,
    Region,
    Material,
    Screen,
    Character,
    Item,
    Palette,
}

#[derive(Clone, Debug)]
pub struct UndoManager {
    pub context: UndoManagerContext,
    pub max_undo: usize,

    project: ProjectUndo,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            context: UndoManagerContext::None,
            max_undo: 30,

            project: ProjectUndo::default(),
        }
    }

    pub fn set_context(mut self, context: UndoManagerContext, _ctx: &mut TheContext) {
        self.context = context;
    }

    pub fn add_undo(&mut self, atom: ProjectUndoAtom, ctx: &mut TheContext) {
        println!("Undo: {:?}", atom.to_string());
        self.project.add(atom);
        self.project.truncate_to_limit(self.max_undo);
        ctx.ui.set_enabled("Undo");
        self.can_save(ctx);
    }

    #[allow(unused_variables)]
    pub fn add_region_undo(&mut self, region: &Uuid, atom: RegionUndoAtom, ctx: &mut TheContext) {
        // self.context = UndoManagerContext::Region;
        // let region_undo = self.regions.entry(*region).or_default();
        // region_undo.add(atom);
        // region_undo.truncate_to_limit(self.max_undo);
        // ctx.ui.set_enabled("Undo");
        // self.can_save(ctx);
    }

    #[allow(unused_variables)]
    pub fn add_material_undo(&mut self, atom: MaterialUndoAtom, ctx: &mut TheContext) {
        // self.context = UndoManagerContext::Material;
        // self.material.add(atom);
        // self.material.truncate_to_limit(self.max_undo);
        // ctx.ui.set_enabled("Undo");
        // self.can_save(ctx);
    }

    #[allow(unused_variables)]
    pub fn add_character_undo(&mut self, atom: CharacterUndoAtom, ctx: &mut TheContext) {
        // self.context = UndoManagerContext::Character;
        // self.character.add(atom);
        // self.character.truncate_to_limit(self.max_undo);
        // ctx.ui.set_enabled("Undo");
        // self.can_save(ctx);
    }

    #[allow(unused_variables)]
    pub fn add_item_undo(&mut self, atom: ItemUndoAtom, ctx: &mut TheContext) {
        // self.context = UndoManagerContext::Item;
        // self.item.add(atom);
        // self.item.truncate_to_limit(self.max_undo);
        // ctx.ui.set_enabled("Undo");
        // self.can_save(ctx);
    }

    #[allow(unused_variables)]
    pub fn add_screen_undo(&mut self, atom: ScreenUndoAtom, ctx: &mut TheContext) {
        // self.context = UndoManagerContext::Screen;
        // self.screen.add(atom);
        // self.screen.truncate_to_limit(self.max_undo);
        // ctx.ui.set_enabled("Undo");
        // self.can_save(ctx);
    }

    #[allow(unused_variables)]
    pub fn add_palette_undo(&mut self, atom: PaletteUndoAtom, ctx: &mut TheContext) {
        // self.context = UndoManagerContext::Palette;
        // self.palette.add(atom);
        // self.palette.truncate_to_limit(self.max_undo);
        // ctx.ui.set_enabled("Undo");
        // self.can_save(ctx);
    }

    pub fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if !self.project.has_undo() {
            ctx.ui.set_disabled("Undo");
        } else {
            ctx.ui.set_enabled("Undo");
        }

        if !self.project.has_redo() {
            ctx.ui.set_disabled("Redo");
        } else {
            ctx.ui.set_enabled("Redo");
        }
    }

    pub fn undo(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if self.project.has_undo() {
            self.project.undo(project, ui, ctx, server_ctx);
        }

        self.set_undo_state_to_ui(ctx);
        /*
        match &self.context {
            UndoManagerContext::None => {}
            UndoManagerContext::Region => {
                if let Some(region_undo) = self.regions.get_mut(&context_id) {
                    if let Some(region) = project.get_region_mut(&context_id) {
                        if region_undo.has_undo() {
                            region_undo.undo(region, ui, ctx);
                        }

                        if !region_undo.has_undo() {
                            ctx.ui.set_disabled("Undo");
                        } else {
                            ctx.ui.set_enabled("Undo");
                        }

                        if !region_undo.has_redo() {
                            ctx.ui.set_disabled("Redo");
                        } else {
                            ctx.ui.set_enabled("Redo");
                        }
                    }
                }
            }
            UndoManagerContext::Material => {
                self.material.undo(project, ui, ctx);

                if !self.material.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.material.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Character => {
                self.character.undo(project, ui, ctx);

                if !self.character.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.character.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Item => {
                self.item.undo(project, ui, ctx);

                if !self.item.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.item.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Screen => {
                self.screen.undo(project, ui, ctx);

                if !self.screen.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.screen.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Palette => {
                self.palette.undo(server_ctx, project, ui, ctx);

                if !self.palette.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.palette.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
        }*/
        self.can_save(ctx);
    }

    pub fn redo(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if self.project.has_redo() {
            self.project.redo(project, ui, ctx, server_ctx);
        }

        self.set_undo_state_to_ui(ctx);
        /*
        match &self.context {
            UndoManagerContext::None => {}
            UndoManagerContext::Region => {
                if let Some(region_undo) = self.regions.get_mut(&context_id) {
                    if let Some(region) = project.get_region_mut(&context_id) {
                        if region_undo.has_redo() {
                            region_undo.redo(region, ui, ctx);
                        }

                        if !region_undo.has_undo() {
                            ctx.ui.set_disabled("Undo");
                        } else {
                            ctx.ui.set_enabled("Undo");
                        }

                        if !region_undo.has_redo() {
                            ctx.ui.set_disabled("Redo");
                        } else {
                            ctx.ui.set_enabled("Redo");
                        }
                    }
                }
            }
            UndoManagerContext::Material => {
                self.material.redo(project, ui, ctx);

                if !self.material.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.material.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Character => {
                self.character.redo(project, ui, ctx);

                if !self.character.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.character.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Item => {
                self.item.redo(project, ui, ctx);

                if !self.item.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.item.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Screen => {
                self.screen.redo(project, ui, ctx);

                if !self.screen.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.screen.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
            UndoManagerContext::Palette => {
                self.palette.redo(server_ctx, project, ui, ctx);

                if !self.palette.has_undo() {
                    ctx.ui.set_disabled("Undo");
                } else {
                    ctx.ui.set_enabled("Undo");
                }

                if !self.palette.has_redo() {
                    ctx.ui.set_disabled("Redo");
                } else {
                    ctx.ui.set_enabled("Redo");
                }
            }
        }*/
        self.can_save(ctx);
    }

    /// Checks if the undo manager is empty and disables the save buttons if it is.
    pub fn can_save(&self, ctx: &mut TheContext) {
        if self.has_undo() {
            // ctx.ui.set_disabled("Save");
            // ctx.ui.set_disabled("Save As");
        } else {
            ctx.ui.set_enabled("Save");
            ctx.ui.set_enabled("Save As");
        }
    }

    /// Checks if the undo manager has any undoable actions.
    pub fn has_undo(&self) -> bool {
        if self.project.has_undo() {
            return true;
        }
        false
    }
}
