use crate::docks::tiles_editor_undo::TileEditorUndoAtom;
use crate::prelude::*;

pub struct TileDrawTool {
    id: TheId,
    /// For tile editing: snapshot of the entire tile before the stroke.
    before_tile: Option<rusterix::Tile>,
    /// For non-tile editing (avatar frames, etc.): snapshot of the texture + context.
    before_snapshot: Option<(PixelEditingContext, rusterix::Texture)>,
    changed: bool,
}

impl EditorTool for TileDrawTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id("Tile Draw Tool", Uuid::new_v4()),
            before_tile: None,
            before_snapshot: None,
            changed: false,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Draw Tool (D). Click and drag to draw pixels with the selected palette color.".to_string()
    }

    fn icon_name(&self) -> String {
        "draw".to_string()
    }

    fn rgba_view_mode(&self) -> Option<TheRGBAViewMode> {
        Some(TheRGBAViewMode::TileEditor)
    }

    fn accel(&self) -> Option<char> {
        Some('D')
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    // Snapshot before we start drawing
                    match server_ctx.editing_ctx {
                        PixelEditingContext::Tile(tile_id, _) => {
                            if let Some(tile) = project.tiles.get(&tile_id) {
                                self.before_tile = Some(tile.clone());
                            }
                        }
                        _ => {
                            if let Some(texture) =
                                project.get_editing_texture(&server_ctx.editing_ctx)
                            {
                                self.before_snapshot =
                                    Some((server_ctx.editing_ctx, texture.clone()));
                            }
                        }
                    }

                    self.draw_pixel(*coord, ui, ctx, project, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorDragged(id, coord) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    self.draw_pixel(*coord, ui, ctx, project, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorUp(_) => {
                if self.changed {
                    // For tiles, update the tile picker when the stroke finishes
                    if matches!(server_ctx.editing_ctx, PixelEditingContext::Tile(..)) {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tilepicker"),
                            TheValue::Empty,
                        ));
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tile Editor Undo Available"),
                        TheValue::Empty,
                    ));
                    self.changed = false;
                }
            }
            _ => {}
        }

        redraw
    }

    fn get_undo_atom(&mut self, project: &Project) -> Option<Box<dyn std::any::Any>> {
        // For tiles, use the TileEdit atom (works with tile picker and map even when editor closed)
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

        // For non-tile contexts, use the generic TextureEdit atom
        if let Some((editing_ctx, before)) = self.before_snapshot.take() {
            if let Some(after) = project.get_editing_texture(&editing_ctx) {
                let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after.clone());
                return Some(Box::new(atom));
            }
        }
        None
    }
}

impl TileDrawTool {
    fn draw_pixel(
        &mut self,
        pos: Vec2<i32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        let editing_ctx = server_ctx.editing_ctx;

        // Shift clears the pixel, otherwise use the context draw color
        let color_array = if ui.shift {
            Some([0, 0, 0, 0])
        } else {
            editing_ctx.get_draw_color(
                &project.palette,
                server_ctx.palette_opacity,
                server_ctx.body_marker_color,
            )
        };

        if let Some(color_array) = color_array {
            if matches!(editing_ctx, PixelEditingContext::AvatarFrame(..))
                && server_ctx.avatar_anchor_slot != AvatarAnchorEditSlot::None
            {
                return;
            }
            if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
                && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
            {
                if rgba_view.has_paste_preview() {
                    return;
                }
                let selection = rgba_view.selection();
                if !selection.is_empty() && !selection.contains(&(pos.x, pos.y)) {
                    return;
                }
            }

            if let Some(texture) = project.get_editing_texture_mut(&editing_ctx) {
                let width = texture.width as i32;
                let height = texture.height as i32;

                if pos.x >= 0 && pos.x < width && pos.y >= 0 && pos.y < height {
                    texture.set_pixel(pos.x as u32, pos.y as u32, color_array);
                    texture.generate_normals(true);

                    // Send context-appropriate update event
                    match editing_ctx {
                        PixelEditingContext::Tile(tile_id, _) => {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Tile Updated"),
                                TheValue::Id(tile_id),
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

                    self.changed = true;
                }
            }
        }
    }
}
