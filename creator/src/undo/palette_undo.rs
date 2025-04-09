use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum PaletteUndoAtom {
    Edit(ThePalette, ThePalette),
}

impl PaletteUndoAtom {
    pub fn undo(
        &self,
        _server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
    ) {
        match self {
            PaletteUndoAtom::Edit(prev, _) => {
                project.palette.clone_from(prev);
                if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                    let index = palette_picker.index();

                    palette_picker.set_palette(project.palette.clone());
                    if let Some(widget) = ui.get_widget("Palette Color Picker") {
                        if let Some(color) = &project.palette[index] {
                            widget.set_value(TheValue::ColorObject(color.clone()));
                        }
                    }
                    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                        if let Some(color) = &project.palette[index] {
                            widget.set_value(TheValue::Text(color.to_hex()));
                        }
                    }
                }
            }
        }
    }
    pub fn redo(
        &self,
        _server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
    ) {
        match self {
            PaletteUndoAtom::Edit(_, next) => {
                project.palette.clone_from(next);
                if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                    let index = palette_picker.index();

                    palette_picker.set_palette(project.palette.clone());
                    if let Some(widget) = ui.get_widget("Palette Color Picker") {
                        if let Some(color) = &project.palette[index] {
                            widget.set_value(TheValue::ColorObject(color.clone()));
                        }
                    }
                    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                        if let Some(color) = &project.palette[index] {
                            widget.set_value(TheValue::Text(color.to_hex()));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PaletteUndo {
    pub stack: Vec<PaletteUndoAtom>,
    pub index: isize,
}

impl Default for PaletteUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl PaletteUndo {
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
        if self.index >= -1 && self.index < self.stack.len() as isize - 1 {
            return true;
        }
        false
    }

    pub fn add(&mut self, atom: PaletteUndoAtom) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(server_ctx, project, ui, ctx);
            self.index -= 1;
        }
    }

    pub fn redo(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(server_ctx, project, ui, ctx);
        }
    }

    pub fn truncate_to_limit(&mut self, limit: usize) {
        if self.stack.len() > limit {
            let excess = self.stack.len() - limit;

            // Remove the oldest `excess` entries from the front
            self.stack.drain(0..excess);

            // Adjust the index accordingly
            self.index -= excess as isize;

            // Clamp to -1 minimum in case we truncated everything
            if self.index < -1 {
                self.index = -1;
            }
        }
    }
}
