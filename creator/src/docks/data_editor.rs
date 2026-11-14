use crate::docks::data::DataDock;
use crate::prelude::*;

#[cfg(feature = "native-ui")]
type PreviewSource =
    std::sync::Arc<dyn theframework::theui::thewidget::therenderview::RenderViewPaintSource>;

const AVATAR_PREVIEW_PLAY_BUTTON: &str = "Avatar Preview Play";

pub struct DataEditorDock {
    inner: DataDock,
    #[cfg(feature = "native-ui")]
    preview_sources: crate::minimap::native::Sources,
    #[cfg(feature = "native-ui")]
    preview_frame: Option<(Option<PreviewSource>, PreviewSource)>,
}

impl Dock for DataEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            inner: DataDock::new(),
            #[cfg(feature = "native-ui")]
            preview_sources: Default::default(),
            #[cfg(feature = "native-ui")]
            preview_frame: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut preview_canvas = TheCanvas::new();
        #[cfg(feature = "native-ui")]
        {
            let mut preview = TheRenderView::new(TheId::named("Data Editor RGBA Layout View"));
            // Match the old RGBA layout's expanding bounds; RenderView itself
            // defaults to a small 20-pixel limiter intended for toolbar views.
            *preview.limiter_mut() = TheSizeLimiter::new();
            preview_canvas.set_widget(preview);
        }
        #[cfg(not(feature = "native-ui"))]
        {
            let mut preview_layout = TheRGBALayout::new(TheId::named("Data Editor RGBA Layout"));
            if let Some(rgba_view) = preview_layout.rgba_view_mut().as_rgba_view() {
                rgba_view.set_mode(TheRGBAViewMode::TilePicker);
                rgba_view.set_background([24, 24, 24, 255]);
                rgba_view.set_grid(None);
                rgba_view.set_supports_external_zoom(false);
            }
            preview_canvas.set_layout(preview_layout);
        }

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
        server_ctx: &mut CreatorContext,
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
        #[cfg(feature = "native-ui")]
        {
            self.preview_sources = Default::default();
            self.preview_frame = None;
        }
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
        server_ctx: &mut CreatorContext,
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
        server_ctx: &mut CreatorContext,
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
        server_ctx: &mut CreatorContext,
    ) {
        self.inner.redo(ui, ctx, project, server_ctx);
        self.sync_hidden_to_max_editor(ui, ctx);
        self.update_preview(ui, ctx, project, server_ctx);
    }

    #[cfg(feature = "native-ui")]
    fn native_minimap_source(
        &self,
        size: Vec2<i32>,
        project: &Project,
        server_ctx: &CreatorContext,
        sources: &mut crate::minimap::native::Sources,
    ) -> Option<
        std::sync::Arc<dyn theframework::theui::thewidget::therenderview::RenderViewPaintSource>,
    > {
        self.inner
            .native_minimap_source(size, project, server_ctx, sources)
    }

    #[cfg(any(not(feature = "native-ui"), test))]
    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &CreatorContext,
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
        server_ctx: &mut CreatorContext,
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
            server_ctx.shared.animation_counter = 0;
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
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &CreatorContext,
    ) {
        #[cfg(feature = "native-ui")]
        {
            let Some(view) = ui.get_render_view("Data Editor RGBA Layout View") else {
                return;
            };
            let dim = *view.dim();
            let size = Vec2::new((dim.width - 16).max(1), (dim.height - 16).max(1));
            let source = self.inner.native_minimap_source(
                size,
                project,
                server_ctx,
                &mut self.preview_sources,
            );
            let same = self
                .preview_frame
                .as_ref()
                .is_some_and(|(old, _)| match (old, &source) {
                    (None, None) => true,
                    (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
                    _ => false,
                });
            if !same {
                let frame = std::sync::Arc::new(RigPreview {
                    source: source.clone(),
                });
                self.preview_frame = Some((source, frame));
                ctx.ui.redraw_all = true;
            }
            view.set_paint_source(Some(self.preview_frame.as_ref().unwrap().1.clone()));
            return;
        }
        #[cfg(not(feature = "native-ui"))]
        {
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
}

#[cfg(feature = "native-ui")]
struct RigPreview {
    source: Option<PreviewSource>,
}
#[cfg(feature = "native-ui")]
impl theframework::theui::thewidget::therenderview::RenderViewPaintSource for RigPreview {
    fn paint(
        &self,
        p: &mut theframework::theui::paint::WidgetPainter<'_>,
        size: [f32; 2],
    ) -> theframework::theui::paint::UiPaintResult {
        use theframework::paint::{PaintOp, Rect, Transform};
        p.list.ops.push(PaintOp::PixelRects {
            rects: vec![Rect::new(0., 0., size[0], size[1])].into(),
            color: [24. / 255., 24. / 255., 24. / 255., 1.],
        });
        let inner = [(size[0] - 16.).max(1.), (size[1] - 16.).max(1.)];
        p.list
            .ops
            .push(PaintOp::PushTransform(Transform::translate(8., 8.)));
        p.list.ops.push(PaintOp::PixelRects {
            rects: vec![Rect::new(0., 0., inner[0], inner[1])].into(),
            color: [18. / 255., 18. / 255., 18. / 255., 1.],
        });
        if let Some(source) = &self.source {
            source.paint(p, inner)?;
        }
        p.list.ops.push(PaintOp::PopTransform);
        Ok(())
    }
}

#[cfg(all(test, feature = "native-ui"))]
impl DataEditorDock {
    pub(crate) fn native_editor_preview_reference_case(
        project: &Project,
        server: &CreatorContext,
    ) -> (
        std::sync::Arc<dyn theframework::theui::thewidget::therenderview::RenderViewPaintSource>,
        TheRGBABuffer,
    ) {
        let size = Vec2::new(143, 84);
        let mut ctx = TheContext::new(size.x as usize, size.y as usize, 1.);
        ctx.ui.command_paint = true;
        let mut ui = TheUI::new();
        let mut view = TheRenderView::new(TheId::named("Data Editor RGBA Layout View"));
        view.set_dim(TheDim::sized(size.x, size.y), &mut ctx);
        view.render_buffer_mut().resize(size.x, size.y);
        ui.canvas.set_widget(view);
        let mut dock = Self::new();
        let mut canvas = dock.setup(&mut ctx);
        canvas.set_dim(TheDim::sized(640, 480), &mut ctx);
        let preview = canvas
            .get_widget(Some(&"Data Editor RGBA Layout View".into()), None)
            .unwrap();
        assert_eq!(preview.dim().width, 640);
        assert!(
            preview.dim().height > 200,
            "the native rig preview keeps the former expanding layout"
        );
        dock.update_preview(&mut ui, &mut ctx, project, server);
        let frame = dock.preview_frame.as_ref().unwrap().1.clone();
        dock.update_preview(&mut ui, &mut ctx, project, server);
        assert!(std::sync::Arc::ptr_eq(
            &frame,
            &dock.preview_frame.as_ref().unwrap().1
        ));
        assert!(
            !ui.get_render_view("Data Editor RGBA Layout View")
                .unwrap()
                .has_cpu_display_storage()
        );
        let mut reference = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
        reference.fill([24, 24, 24, 255]);
        let mut inner = TheRGBABuffer::new(TheDim::sized(size.x - 16, size.y - 16));
        inner.fill([18, 18, 18, 255]);
        dock.inner
            .draw_minimap(&mut inner, project, &mut ctx, server);
        reference.copy_into(8, 8, &inner);
        // RGBAView copied stored RGB into the application's opaque display.
        // The blend helper's stored alpha is not the final surface alpha.
        for pixel in reference.pixels_mut().chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        (frame, reference)
    }
}
