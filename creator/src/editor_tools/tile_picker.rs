use crate::editor::{PALETTE, RUSTERIX, UNDOMANAGER};
use crate::prelude::*;

pub struct TilePickerTool {
    id: TheId,
}

impl EditorTool for TilePickerTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id("Tile Picker Tool", Uuid::new_v4()),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Picker Tool (P). Click to sample a pixel color, then select it in the palette or add it if missing.".to_string()
    }

    fn icon_name(&self) -> String {
        "eyedropper-sample".to_string()
    }

    fn rgba_view_mode(&self) -> Option<TheRGBAViewMode> {
        Some(TheRGBAViewMode::TileEditor)
    }

    fn accel(&self) -> Option<char> {
        Some('P')
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
        {
            return self.pick_from(*coord, ui, ctx, project, server_ctx);
        }
        false
    }
}

impl TilePickerTool {
    fn pick_from(
        &mut self,
        pos: Vec2<i32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
            && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
            && rgba_view.has_paste_preview()
        {
            return false;
        }

        let editing_ctx = server_ctx.editing_ctx;
        let Some(texture) = project.get_editing_texture(&editing_ctx) else {
            return false;
        };

        let width = texture.width as i32;
        let height = texture.height as i32;
        if pos.x < 0 || pos.y < 0 || pos.x >= width || pos.y >= height {
            return false;
        }

        let sampled = texture.get_pixel(pos.x as u32, pos.y as u32);
        let sampled_color = TheColor::from(sampled);

        let prev_palette = project.palette.clone();

        let mut selected_index = project
            .palette
            .colors
            .iter()
            .position(|entry| entry.as_ref() == Some(&sampled_color));

        if selected_index.is_none() {
            project.palette.add_unique_color(sampled_color.clone());
            selected_index = project
                .palette
                .colors
                .iter()
                .position(|entry| entry.as_ref() == Some(&sampled_color));
        }

        if let Some(index) = selected_index {
            project.palette.current_index = index as u16;
        }

        if project.palette != prev_palette {
            let undo = ProjectUndoAtom::PaletteEdit(prev_palette, project.palette.clone());
            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
        }

        apply_palette(ui, ctx, server_ctx, project);

        if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
            palette_picker.set_palette(project.palette.clone());
            palette_picker.set_index(project.palette.current_index as usize);
        }
        if let Some(widget) = ui.get_widget("Palette Color Picker") {
            widget.set_value(TheValue::ColorObject(sampled_color.clone()));
        }
        if let Some(widget) = ui.get_widget("Palette Hex Edit") {
            widget.set_value(TheValue::Text(sampled_color.to_hex()));
        }

        *PALETTE.write().unwrap() = project.palette.clone();
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.assets.palette = project.palette.clone();
            rusterix.set_tiles(project.tiles.clone(), true);
        }

        true
    }
}
