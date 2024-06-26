use crate::editor::{MODELFXEDITOR, TILEDRAWER};
use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum MaterialFXUndoAtom {
    AddMaterial(MaterialFXObject),
    AddNode(Uuid, String, String),
    Edit(Uuid, String, String),
}

impl MaterialFXUndoAtom {
    pub fn undo(
        &self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        match self {
            MaterialFXUndoAtom::AddMaterial(material) => {
                project.materials.shift_remove(&material.id);
                if server_ctx.curr_material_object == Some(material.id) {
                    server_ctx.curr_material_object = None;
                }

                let mut editor = MODELFXEDITOR.lock().unwrap();
                editor.set_material_tiles(ui, ctx, project, None);
                editor.set_material_node_ui(server_ctx, project, ui, ctx);
            }
            MaterialFXUndoAtom::AddNode(id, prev, _) | MaterialFXUndoAtom::Edit(id, prev, _) => {
                if let Some(material) = project.materials.get_mut(id) {
                    *material = MaterialFXObject::from_json(prev);
                    material.render_preview(&project.palette, &TILEDRAWER.lock().unwrap().tiles);

                    let node_canvas = material.to_canvas(&project.palette);
                    ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);

                    let mut editor = MODELFXEDITOR.lock().unwrap();
                    editor.set_material_tiles(ui, ctx, project, None);
                    editor.set_material_node_ui(server_ctx, project, ui, ctx);
                    editor.set_selected_material_node_ui(server_ctx, project, ui, ctx);
                    editor.render_material_changes(*id, server_ctx, project);
                }
            }
        }
    }
    pub fn redo(
        &self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        match self {
            MaterialFXUndoAtom::AddMaterial(material) => {
                project.materials.insert(material.id, material.clone());
            }
            MaterialFXUndoAtom::AddNode(id, _, next) | MaterialFXUndoAtom::Edit(id, _, next) => {
                if let Some(material) = project.materials.get_mut(id) {
                    *material = MaterialFXObject::from_json(next);
                    material.render_preview(&project.palette, &TILEDRAWER.lock().unwrap().tiles);

                    let node_canvas = material.to_canvas(&project.palette);
                    ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);

                    let mut editor = MODELFXEDITOR.lock().unwrap();
                    editor.set_material_tiles(ui, ctx, project, None);
                    editor.set_material_node_ui(server_ctx, project, ui, ctx);
                    editor.set_selected_material_node_ui(server_ctx, project, ui, ctx);
                    editor.render_material_changes(*id, server_ctx, project);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct MaterialFXUndo {
    pub stack: Vec<MaterialFXUndoAtom>,
    pub index: isize,
}

impl Default for MaterialFXUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialFXUndo {
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

    pub fn add(&mut self, atom: MaterialFXUndoAtom) {
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
}
