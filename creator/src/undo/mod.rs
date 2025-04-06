pub mod material_undo;
pub mod palette_undo;
pub mod region_undo;
pub mod screen_undo;

use crate::prelude::*;
use material_undo::*;
use screen_undo::*;

#[derive(PartialEq, Clone, Debug)]
pub enum UndoManagerContext {
    None,
    Region,
    Material,
    Screen,
    CodeGridFX,
    Palette,
}

#[derive(Clone, Debug)]
pub struct UndoManager {
    pub context: UndoManagerContext,

    regions: FxHashMap<Uuid, RegionUndo>,
    material: MaterialUndo,
    screen: ScreenUndo,
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
            material: MaterialUndo::default(),
            screen: ScreenUndo::default(),
            palette: PaletteUndo::default(),
        }
    }

    pub fn set_context(mut self, context: UndoManagerContext, _ctx: &mut TheContext) {
        self.context = context;
        // match &self.context {
        //     UndoManagerContext::Region => {}
        // }
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

    pub fn add_material_undo(&mut self, atom: MaterialUndoAtom, ctx: &mut TheContext) {
        self.context = UndoManagerContext::Material;
        self.material.add(atom);
        ctx.ui.set_enabled("Undo");
        self.can_save(ctx);
    }

    pub fn add_screen_undo(&mut self, atom: ScreenUndoAtom, ctx: &mut TheContext) {
        self.context = UndoManagerContext::Screen;
        self.screen.add(atom);
        ctx.ui.set_enabled("Undo");
        self.can_save(ctx);
    }

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
                return true;
            }
        }
        if self.material.has_undo() {
            return false;
        }
        if self.screen.has_undo() {
            return false;
        }
        if self.palette.has_undo() {
            return false;
        }
        false
    }

    // Clears the ModelFX undo.
    // pub fn clear_materialfx(&mut self) {
    //     self.materialfx.clear();
    // }
}
