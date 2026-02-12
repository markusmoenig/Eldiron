use crate::prelude::*;
use theframework::prelude::*;

/// Undo atoms for tile editor operations
#[derive(Clone, Debug)]
pub enum TileEditorUndoAtom {
    /// Tile texture edit: (tile_id, before_tile, after_tile)
    TileEdit(Uuid, rusterix::Tile, rusterix::Tile),
    /// Generic texture edit via PixelEditingContext: (context, before_texture, after_texture)
    TextureEdit(PixelEditingContext, rusterix::Texture, rusterix::Texture),
    /// Avatar weapon attachment anchor edit for the currently edited frame.
    AvatarAnchorEdit(
        PixelEditingContext,
        Option<(i16, i16)>,
        Option<(i16, i16)>,
        Option<(i16, i16)>,
        Option<(i16, i16)>,
    ),
}

impl TileEditorUndoAtom {
    pub fn undo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            TileEditorUndoAtom::TileEdit(tile_id, prev, _) => {
                if let Some(tile) = project.tiles.get_mut(tile_id) {
                    *tile = prev.clone();

                    // Notify tile editor to refresh tile
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tile Picked"),
                        TheValue::Id(tile.id),
                    ));

                    // Update tile picker if visible
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tilepicker"),
                        TheValue::Empty,
                    ));
                }
            }
            TileEditorUndoAtom::TextureEdit(editing_ctx, prev, _) => {
                if let Some(texture) = project.get_editing_texture_mut(editing_ctx) {
                    *texture = prev.clone();
                }
                Self::send_editing_context_update(editing_ctx, ctx);
            }
            TileEditorUndoAtom::AvatarAnchorEdit(editing_ctx, prev_main, prev_off, _, _) => {
                if let Some(frame) = project.get_editing_avatar_frame_mut(editing_ctx) {
                    frame.weapon_main_anchor = *prev_main;
                    frame.weapon_off_anchor = *prev_off;
                }
                Self::send_editing_context_update(editing_ctx, ctx);
            }
        }
    }

    pub fn redo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            TileEditorUndoAtom::TileEdit(tile_id, _, next) => {
                if let Some(tile) = project.tiles.get_mut(tile_id) {
                    if !tile.textures.is_empty() {
                        *tile = next.clone();

                        // Notify tile editor to refresh tile
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Tile Picked"),
                            TheValue::Id(tile.id),
                        ));

                        // Update tile picker if visible
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tilepicker"),
                            TheValue::Empty,
                        ));
                    }
                }
            }
            TileEditorUndoAtom::TextureEdit(editing_ctx, _, next) => {
                if let Some(texture) = project.get_editing_texture_mut(editing_ctx) {
                    *texture = next.clone();
                }
                Self::send_editing_context_update(editing_ctx, ctx);
            }
            TileEditorUndoAtom::AvatarAnchorEdit(editing_ctx, _, _, next_main, next_off) => {
                if let Some(frame) = project.get_editing_avatar_frame_mut(editing_ctx) {
                    frame.weapon_main_anchor = *next_main;
                    frame.weapon_off_anchor = *next_off;
                }
                Self::send_editing_context_update(editing_ctx, ctx);
            }
        }
    }

    /// Sends the appropriate UI update events after an editing context undo/redo.
    fn send_editing_context_update(editing_ctx: &PixelEditingContext, ctx: &mut TheContext) {
        match editing_ctx {
            PixelEditingContext::None => {}
            PixelEditingContext::Tile(tile_id, _) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Tile Updated"),
                    TheValue::Id(*tile_id),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tilepicker"),
                    TheValue::Empty,
                ));
            }
            PixelEditingContext::AvatarFrame(..) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Editing Texture Updated"),
                    TheValue::Empty,
                ));
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

    pub fn has_changes(&self) -> bool {
        // Has changes if the index is not at the beginning (i.e., not fully undone)
        self.index >= 0
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
