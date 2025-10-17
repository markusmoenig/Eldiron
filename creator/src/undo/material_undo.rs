use crate::editor::{SHADEGRIDFX, SHADERBUFFER};
use crate::prelude::*;
use codegridfx::Module;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MaterialUndoAtom {
    ShaderEdit(Module, Module),
}

impl MaterialUndoAtom {
    pub fn undo(&self, _project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            MaterialUndoAtom::ShaderEdit(prev, _) => {
                let mut shadergridfx = SHADEGRIDFX.write().unwrap();
                *shadergridfx = prev.clone();
                shadergridfx.redraw(ui, ctx);
                shadergridfx.show_settings(ui, ctx);

                crate::utils::draw_shader_into(&shadergridfx, &mut SHADERBUFFER.write().unwrap());
                shadergridfx.set_shader_background(SHADERBUFFER.read().unwrap().clone(), ui, ctx);
            }
        }
    }
    pub fn redo(&self, _project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            MaterialUndoAtom::ShaderEdit(_, next) => {
                let mut shadergridfx = SHADEGRIDFX.write().unwrap();
                *shadergridfx = next.clone();
                shadergridfx.redraw(ui, ctx);
                shadergridfx.show_settings(ui, ctx);

                crate::utils::draw_shader_into(&shadergridfx, &mut SHADERBUFFER.write().unwrap());
                shadergridfx.set_shader_background(SHADERBUFFER.read().unwrap().clone(), ui, ctx);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaterialUndo {
    pub stack: Vec<MaterialUndoAtom>,
    pub index: isize,
}

impl Default for MaterialUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialUndo {
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

    pub fn add(&mut self, atom: MaterialUndoAtom) {
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
