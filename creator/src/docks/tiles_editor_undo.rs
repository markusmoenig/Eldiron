use crate::prelude::*;
use theframework::prelude::*;

/// Undo atoms for tile editor operations
#[derive(Clone, Debug)]
pub enum TileEditorUndoAtom {
    /// Tile texture edit: (tile_id, before_texture, after_texture)
    TileEdit(Uuid, rusterix::Tile, rusterix::Tile),
}

impl TileEditorUndoAtom {
    pub fn undo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            TileEditorUndoAtom::TileEdit(tile_id, prev, _) => {
                if let Some(tile) = project.tiles.get_mut(tile_id) {
                    if !tile.textures.is_empty() {
                        *tile = prev.clone();

                        // Notify tile editor to refresh
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tile Editor"),
                            TheValue::Empty,
                        ));

                        // Update tile picker if visible
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tilepicker"),
                            TheValue::Empty,
                        ));
                    }
                }
            }
        }
    }

    pub fn redo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            TileEditorUndoAtom::TileEdit(tile_id, _, next) => {
                if let Some(tile) = project.tiles.get_mut(tile_id) {
                    if !tile.textures.is_empty() {
                        *tile = next.clone();

                        // Notify tile editor to refresh
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tile Editor"),
                            TheValue::Empty,
                        ));

                        // Update tile picker if visible
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tilepicker"),
                            TheValue::Empty,
                        ));
                    }
                }
            }
        }
    }
}

/// Undo stack for tile editor
#[derive(Clone, Debug)]
pub struct TileEditorUndo {
    pub stack: Vec<TileEditorUndoAtom>,
    pub index: isize,
}

impl Default for TileEditorUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl TileEditorUndo {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            index: -1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.stack = vec![];
        self.index = -1;
    }

    pub fn has_undo(&self) -> bool {
        self.index >= 0
    }

    pub fn has_redo(&self) -> bool {
        self.index >= -1 && self.index < self.stack.len() as isize - 1
    }

    pub fn add(&mut self, atom: TileEditorUndoAtom) {
        // Remove any redo history
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(&mut self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(project, ui, ctx);
            self.index -= 1;
        }
    }

    pub fn redo(&mut self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(project, ui, ctx);
        }
    }

    pub fn truncate_to_limit(&mut self, limit: usize) {
        if self.stack.len() > limit {
            let excess = self.stack.len() - limit;
            self.stack.drain(0..excess);
            self.index -= excess as isize;
            if self.index < -1 {
                self.index = -1;
            }
        }
    }
}
