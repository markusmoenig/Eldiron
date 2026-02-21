use crate::docks::data::DataDock;
use crate::prelude::*;

pub struct DataEditorDock {
    inner: DataDock,
}

impl Dock for DataEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            inner: DataDock::new(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut preview_canvas = TheCanvas::new();
        let mut preview_layout = TheRGBALayout::new(TheId::named("Data Editor RGBA Layout"));
        if let Some(rgba_view) = preview_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            rgba_view.set_background([24, 24, 24, 255]);
            rgba_view.set_grid(None);
            rgba_view.set_supports_external_zoom(false);
        }
        preview_canvas.set_layout(preview_layout);
        center.set_top(preview_canvas);

        let mut textedit = TheTextAreaEdit::new(TheId::named("DockDataEditorMax"));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_syntax_from_string(source);
            textedit.set_code_type("TOML");
        }
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_theme_from_string(source);
            textedit.set_code_theme("Gruvbox Dark");
        }
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        textedit.set_supports_undo(false);
        center.set_widget(textedit);

        center
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.inner.activate(ui, ctx, project, server_ctx);
        self.sync_hidden_to_max_editor(ui, ctx);
        self.update_preview(ui, ctx, project, server_ctx);
    }

    fn minimized(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.inner.minimized(ui, ctx);
    }

    fn supports_actions(&self) -> bool {
        self.inner.supports_actions()
    }

    fn supports_undo(&self) -> bool {
        self.inner.supports_undo()
    }

    fn has_changes(&self) -> bool {
        self.inner.has_changes()
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        self.inner.set_undo_state_to_ui(ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = if let TheEvent::ValueChanged(id, value) = event
            && id.name == "DockDataEditorMax"
        {
            let forwarded = TheEvent::ValueChanged(TheId::named("DockDataEditor"), value.clone());
            self.inner
                .handle_event(&forwarded, ui, ctx, project, server_ctx)
        } else {
            self.inner.handle_event(event, ui, ctx, project, server_ctx)
        };
        if let TheEvent::WidgetResized(id, _) = event
            && id.name == "Data Editor RGBA Layout View"
        {
            redraw = true;
        }
        if let TheEvent::Custom(id, _) = event
            && id.name == "Soft Update Minimap"
        {
            redraw = true;
        }
        if redraw {
            self.sync_hidden_to_max_editor(ui, ctx);
            self.update_preview(ui, ctx, project, server_ctx);
        }
        redraw
    }

    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        self.inner.undo(ui, ctx, project, server_ctx);
        self.sync_hidden_to_max_editor(ui, ctx);
        self.update_preview(ui, ctx, project, server_ctx);
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        self.inner.redo(ui, ctx, project, server_ctx);
        self.sync_hidden_to_max_editor(ui, ctx);
        self.update_preview(ui, ctx, project, server_ctx);
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        self.inner.draw_minimap(buffer, project, ctx, server_ctx)
    }

    fn supports_minimap_animation(&self) -> bool {
        self.inner.supports_minimap_animation()
    }
}

impl DataEditorDock {
    fn sync_hidden_to_max_editor(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(hidden) = ui.get_text_area_edit("DockDataEditor") {
            let text = hidden.get_state().rows.join("\n");
            ui.set_widget_value("DockDataEditorMax", ctx, TheValue::Text(text));
        }
    }

    fn update_preview(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        let Some(layout) = ui.get_rgba_layout("Data Editor RGBA Layout") else {
            return;
        };
        let Some(view) = layout.rgba_view_mut().as_rgba_view() else {
            return;
        };
        let dim = *view.dim();
        let mut buffer = TheRGBABuffer::new(TheDim::sized(
            (dim.width - 16).max(1),
            (dim.height - 16).max(1),
        ));
        buffer.fill([18, 18, 18, 255]);
        let _ = self
            .inner
            .draw_minimap(&mut buffer, project, ctx, server_ctx);
        view.set_buffer(buffer);
    }
}
