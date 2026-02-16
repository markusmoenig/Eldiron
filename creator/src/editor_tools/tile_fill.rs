use crate::docks::tiles_editor_undo::TileEditorUndoAtom;
use crate::prelude::*;

pub struct TileFillTool {
    id: TheId,
    before_tile: Option<rusterix::Tile>,
    before_snapshot: Option<(PixelEditingContext, rusterix::Texture)>,
}

impl EditorTool for TileFillTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id("Tile Fill Tool", Uuid::new_v4()),
            before_tile: None,
            before_snapshot: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Fill Tool (F). Click to flood fill connected pixels; respects active selection."
            .to_string()
    }

    fn icon_name(&self) -> String {
        "paint-bucket".to_string()
    }

    fn rgba_view_mode(&self) -> Option<TheRGBAViewMode> {
        Some(TheRGBAViewMode::TileEditor)
    }

    fn accel(&self) -> Option<char> {
        Some('F')
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::TileEditorClicked(id, coord) = event
            && id.name == "Tile Editor Dock RGBA Layout View"
            && self.fill_from(*coord, ui, ctx, project, server_ctx)
        {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Tile Editor Undo Available"),
                TheValue::Empty,
            ));
            return true;
        }
        false
    }

    fn get_undo_atom(&mut self, project: &Project) -> Option<Box<dyn std::any::Any>> {
        if let Some(before) = self.before_tile.take() {
            if let Some(tile) = project.tiles.get(&before.id) {
                if !tile.textures.is_empty() {
                    let after = tile.clone();
                    let atom = TileEditorUndoAtom::TileEdit(before.id, before, after);
                    return Some(Box::new(atom));
                }
            }
            return None;
        }

        if let Some((editing_ctx, before)) = self.before_snapshot.take() {
            if let Some(after) = project.get_editing_texture(&editing_ctx) {
                let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after.clone());
                return Some(Box::new(atom));
            }
        }
        None
    }
}

impl TileFillTool {
    fn fill_from(
        &mut self,
        pos: Vec2<i32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let editing_ctx = server_ctx.editing_ctx;

        if matches!(editing_ctx, PixelEditingContext::AvatarFrame(..))
            && server_ctx.avatar_anchor_slot != AvatarAnchorEditSlot::None
        {
            return false;
        }

        let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") else {
            return false;
        };
        let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() else {
            return false;
        };

        if rgba_view.has_paste_preview() {
            return false;
        }

        let selection = rgba_view.selection();
        if !selection.is_empty() && !selection.contains(&(pos.x, pos.y)) {
            return false;
        }

        let Some(fill_color) = (if ui.shift {
            Some([0, 0, 0, 0])
        } else {
            editing_ctx.get_draw_color(
                &project.palette,
                server_ctx.palette_opacity,
                server_ctx.body_marker_color,
            )
        }) else {
            return false;
        };

        self.before_tile = None;
        self.before_snapshot = None;
        match editing_ctx {
            PixelEditingContext::Tile(tile_id, _) => {
                if let Some(tile) = project.tiles.get(&tile_id) {
                    self.before_tile = Some(tile.clone());
                }
            }
            _ => {
                if let Some(texture) = project.get_editing_texture(&editing_ctx) {
                    self.before_snapshot = Some((editing_ctx, texture.clone()));
                }
            }
        }

        let Some(texture) = project.get_editing_texture_mut(&editing_ctx) else {
            return false;
        };
        let width = texture.width as i32;
        let height = texture.height as i32;
        if pos.x < 0 || pos.y < 0 || pos.x >= width || pos.y >= height {
            return false;
        }

        let target = texture.get_pixel(pos.x as u32, pos.y as u32);
        if target == fill_color {
            self.before_tile = None;
            self.before_snapshot = None;
            return false;
        }

        let mut stack = vec![(pos.x, pos.y)];
        let mut visited = vec![false; texture.width * texture.height];
        let mut changed = false;

        while let Some((x, y)) = stack.pop() {
            if x < 0 || y < 0 || x >= width || y >= height {
                continue;
            }
            if !selection.is_empty() && !selection.contains(&(x, y)) {
                continue;
            }

            let idx = y as usize * texture.width + x as usize;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            if texture.get_pixel(x as u32, y as u32) != target {
                continue;
            }

            texture.set_pixel(x as u32, y as u32, fill_color);
            changed = true;

            stack.push((x + 1, y));
            stack.push((x - 1, y));
            stack.push((x, y + 1));
            stack.push((x, y - 1));
        }

        if !changed {
            self.before_tile = None;
            self.before_snapshot = None;
            return false;
        }

        texture.generate_normals(true);

        match editing_ctx {
            PixelEditingContext::Tile(tile_id, _) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Tile Updated"),
                    TheValue::Id(tile_id),
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
            PixelEditingContext::None => {}
        }

        true
    }
}
