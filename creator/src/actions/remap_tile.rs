use crate::editor::{DOCKMANAGER, UNDOMANAGER};
use crate::prelude::*;

pub struct RemapTile {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for RemapTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let nodeui: TheNodeUI = TheNodeUI::default();

        Self {
            id: TheId::named(&fl!("action_remap_tile")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_remap_tile_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Tiles" && server_ctx.curr_tile_id.is_some()
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                let prev = tile.clone();

                for tex in &mut tile.textures {
                    for y in 0..tex.height {
                        for x in 0..tex.width {
                            let mut col = tex.get_pixel(x as u32, y as u32);
                            let t = col[3];
                            let color = TheColor::from(col);

                            if let Some(index) = project.palette.find_closest_color_index(&color) {
                                if let Some(c) = project.palette.colors.get(index) {
                                    if let Some(c) = c {
                                        col = c.to_u8_array();
                                        col[3] = t;
                                    }
                                }
                            }

                            tex.set_pixel(x as u32, y as u32, col);
                        }
                    }
                }

                for tex in &mut tile.textures {
                    tex.generate_normals(true);
                }

                let undo_atom = ProjectUndoAtom::TileEdit(prev, tile.clone());
                UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tiles"),
                    TheValue::Empty,
                ));
            }
        }
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
