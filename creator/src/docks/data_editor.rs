use crate::docks::data::DataDock;
use crate::prelude::*;

const AVATAR_PREVIEW_PLAY_BUTTON: &str = "Avatar Preview Play";

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

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut play = TheTraybarButton::new(TheId::named(AVATAR_PREVIEW_PLAY_BUTTON));
        play.set_text(fl!("menu_pause"));
        play.set_status_text(&fl!("status_avatar_preview_playback"));
        toolbar_hlayout.add_widget(Box::new(play));

        toolbar_canvas.set_layout(toolbar_hlayout);
        preview_canvas.set_top(toolbar_canvas);
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
        textedit.display_line_number(false);
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
        self.sync_preview_play_button(ui, ctx);
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

    fn mark_saved(&mut self) {
        self.inner.mark_saved();
    }

    fn reset_for_project_switch(&mut self) {
        self.inner.reset_for_project_switch();
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
        let mut redraw = match event {
            TheEvent::ValueChanged(id, value) if id.name == "DockDataEditorMax" => {
                ui.set_widget_value("DockDataEditor", ctx, value.clone());
                let forwarded =
                    TheEvent::ValueChanged(TheId::named("DockDataEditor"), value.clone());
                let redraw = self
                    .inner
                    .handle_event(&forwarded, ui, ctx, project, server_ctx);
                self.sync_preview_play_button(ui, ctx);
                redraw
            }
            TheEvent::StateChanged(id, state)
                if id.name == AVATAR_PREVIEW_PLAY_BUTTON && *state == TheWidgetState::Clicked =>
            {
                self.toggle_preview_playback(ui, ctx, project, server_ctx)
            }
            _ => self.inner.handle_event(event, ui, ctx, project, server_ctx),
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
    fn preview_source(ui: &mut TheUI, editor: &str) -> Option<String> {
        ui.get_text_area_edit(editor)
            .map(|edit| edit.get_state().rows.join("\n"))
    }

    fn preview_is_playing(source: &str) -> bool {
        source
            .parse::<toml::Table>()
            .ok()
            .and_then(|table| table.get("play").and_then(toml::Value::as_bool))
            .unwrap_or(true)
    }

    fn with_preview_playing(source: &str, playing: bool) -> String {
        let had_trailing_newline = source.ends_with('\n');
        let mut rows: Vec<String> = source.lines().map(ToString::to_string).collect();
        let first_table = rows
            .iter()
            .position(|row| row.trim_start().starts_with('['))
            .unwrap_or(rows.len());

        let mut found = false;
        for row in rows.iter_mut().take(first_table) {
            let trimmed = row.trim_start();
            if let Some((key, _)) = trimmed.split_once('=')
                && key.trim() == "play"
            {
                let indent_len = row.len().saturating_sub(trimmed.len());
                let indent = &row[..indent_len];
                *row = format!("{indent}play = {playing}");
                found = true;
                break;
            }
        }

        if !found {
            rows.insert(first_table, format!("play = {playing}"));
        }

        let mut updated = rows.join("\n");
        if had_trailing_newline {
            updated.push('\n');
        }
        updated
    }

    fn sync_preview_play_button(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        let playing = Self::preview_source(ui, "DockDataEditorMax")
            .is_none_or(|source| Self::preview_is_playing(&source));
        let label = if playing {
            fl!("menu_pause")
        } else {
            fl!("menu_play")
        };
        ui.set_widget_value(AVATAR_PREVIEW_PLAY_BUTTON, ctx, TheValue::Text(label));
    }

    fn toggle_preview_playback(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some(source) = Self::preview_source(ui, "DockDataEditorMax") else {
            return false;
        };
        let playing = !Self::preview_is_playing(&source);
        let updated = Self::with_preview_playing(&source, playing);

        ui.set_widget_value("DockDataEditor", ctx, TheValue::Text(updated.clone()));
        ui.set_widget_value("DockDataEditorMax", ctx, TheValue::Text(updated.clone()));
        let forwarded =
            TheEvent::ValueChanged(TheId::named("DockDataEditor"), TheValue::Text(updated));
        let redraw = self
            .inner
            .handle_event(&forwarded, ui, ctx, project, server_ctx);
        if playing {
            server_ctx.animation_counter = 0;
        }
        self.sync_preview_play_button(ui, ctx);
        redraw
    }

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
