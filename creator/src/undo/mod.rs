// pub mod materialfx_undo;
pub mod palette_undo;
pub mod region_undo;

use crate::prelude::*;

//use crate::editor::MODELFXEDITOR;

// use self::materialfx_undo::MaterialFXUndo;

#[derive(PartialEq, Clone, Debug)]
pub enum UndoManagerContext {
    None,
    Region,
    MaterialFX,
    CodeGridFX,
    Palette,
}

#[derive(Clone, Debug)]
pub struct UndoManager {
    pub context: UndoManagerContext,

    regions: FxHashMap<Uuid, RegionUndo>,
    // materialfx: MaterialFXUndo,
    palette: PaletteUndo,
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
            // materialfx: MaterialFXUndo::default(),
            palette: PaletteUndo::default(),
        }
    }

    pub fn add_region_undo(&mut self, region: &Uuid, atom: RegionUndoAtom, ctx: &mut TheContext) {
        self.context = UndoManagerContext::Region;
        let region_undo = self.regions.entry(*region).or_default();
        region_undo.add(atom);
        ctx.ui.set_enabled("Undo");
        self.can_save(ctx);
    }

    // pub fn add_materialfx_undo(&mut self, atom: MaterialFXUndoAtom, ctx: &mut TheContext) {
    //     self.context = UndoManagerContext::MaterialFX;
    //     self.materialfx.add(atom);
    //     ctx.ui.set_enabled("Undo");
    //     self.can_save(ctx);
    // }

    pub fn add_palette_undo(&mut self, atom: PaletteUndoAtom, ctx: &mut TheContext) {
        self.context = UndoManagerContext::Palette;
        self.palette.add(atom);
        ctx.ui.set_enabled("Undo");
        self.can_save(ctx);
    }

    pub fn undo(
        &mut self,
        context_id: Uuid,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
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
            // UndoManagerContext::MaterialFX => {
            //     self.materialfx.undo(server_ctx, project, ui, ctx);

            //     if !self.materialfx.has_undo() {
            //         ctx.ui.set_disabled("Undo");
            //     } else {
            //         ctx.ui.set_enabled("Undo");
            //     }

            //     if !self.materialfx.has_redo() {
            //         ctx.ui.set_disabled("Redo");
            //     } else {
            //         ctx.ui.set_enabled("Redo");
            //     }
            // }
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
            _ => {}
        }
        self.can_save(ctx);
    }

    pub fn redo(
        &mut self,
        context_id: Uuid,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
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
            // UndoManagerContext::MaterialFX => {
            //     self.materialfx.redo(server_ctx, project, ui, ctx);

            //     if !self.materialfx.has_undo() {
            //         ctx.ui.set_disabled("Undo");
            //     } else {
            //         ctx.ui.set_enabled("Undo");
            //     }

            //     if !self.materialfx.has_redo() {
            //         ctx.ui.set_disabled("Redo");
            //     } else {
            //         ctx.ui.set_enabled("Redo");
            //     }
            // }
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
            _ => {}
        }
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
        for region_undo in self.regions.values() {
            if region_undo.has_undo() {
                return false;
            }
        }
        true
    }

    // Clears the ModelFX undo.
    // pub fn clear_materialfx(&mut self) {
    //     self.materialfx.clear();
    // }
}
