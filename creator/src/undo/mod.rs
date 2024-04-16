pub mod region_undo;

use crate::prelude::*;

#[derive(PartialEq, Clone, Debug)]
pub enum UndoManagerContext {
    None,
    Region,
    CodeGridFX,
}

#[derive(PartialEq, Clone, Debug)]
pub struct UndoManager {
    pub context: UndoManagerContext,
    regions: FxHashMap<Uuid, RegionUndo>,
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
            regions: FxHashMap::default(),
        }
    }

    pub fn add_region_undo(&mut self, region: &Uuid, atom: RegionUndoAtom, ctx: &mut TheContext) {
        self.context = UndoManagerContext::Region;
        let region_undo = self.regions.entry(*region).or_default();
        region_undo.add(atom);
        ctx.ui.set_enabled("Undo");
        self.can_save(ctx);
    }

    pub fn undo(&mut self, context_id: Uuid, project: &mut Project, ctx: &mut TheContext) {
        match &self.context {
            UndoManagerContext::None => {}
            UndoManagerContext::Region => {
                if let Some(region_undo) = self.regions.get_mut(&context_id) {
                    if let Some(region) = project.get_region_mut(&context_id) {
                        if region_undo.has_undo() {
                            region_undo.undo(region);
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
            _ => {}
        }
        self.can_save(ctx);
    }

    pub fn redo(&mut self, context_id: Uuid, project: &mut Project, ctx: &mut TheContext) {
        match &self.context {
            UndoManagerContext::None => {}
            UndoManagerContext::Region => {
                if let Some(region_undo) = self.regions.get_mut(&context_id) {
                    if let Some(region) = project.get_region_mut(&context_id) {
                        if region_undo.has_redo() {
                            region_undo.redo(region);
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
            _ => {}
        }
        self.can_save(ctx);
    }

    /// Checks if the undo manager is empty and disables the save buttons if it is.
    pub fn can_save(&self, ctx: &mut TheContext) {
        if self.has_undo() {
            ctx.ui.set_disabled("Save");
            ctx.ui.set_disabled("Save As");
        } else {
            ctx.ui.set_enabled("Save");
            ctx.ui.set_enabled("Save As");
        }
    }

    /// Checks if the undo manager has any undoable actions.
    pub fn has_undo(&self) -> bool {
        for region_undo in self.regions.values() {
            if region_undo.has_undo() {
                return false;
            }
        }
        true
    }
}
