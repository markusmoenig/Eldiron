//! App-retained graph panels. Widgets route native input and paint; completed
//! edits stay in GraphViewState until the CPU source owner acknowledges them.
use super::*;
use std::sync::{Arc, Mutex};
use theframework::{
    paint::{PaintOp, Rect, Transform},
    platform::{InputEvent, InputResponse, PointerButton, TextInputState, TextInputTarget},
    text::{TextOptions, TextSystem},
    text_edit::{EditorAction, EditorColors, EditorCommand, EditorResponse, EditorView},
    theui::paint::{UiPaintResult, WidgetPainter, bounds, rgba},
};

struct ConversationDraft {
    binding: u64,
    row: GraphId,
    expected: GraphControlValue,
    committed: Conversation,
    editor: ConversationEditor,
    field: Option<(ConversationField, EditorView, String)>,
}
enum Panel {
    Text(GraphTextDraft),
    Conversation(ConversationDraft),
    Picker(super::native_picker::NativePicker),
}
/// Kept by the application with the source, independently of the visible widget
/// and GPU context. Lock panels before graph state, and release both before UI
/// callbacks or project/undo effects.
#[derive(Default)]
pub(super) struct NativeNodePanels {
    panel: Option<Panel>,
    error: Option<String>,
}
impl NativeNodePanels {
    pub(super) fn is_open(&self) -> bool {
        self.panel.is_some()
    }
    pub(super) fn has_changes(&self) -> bool {
        match &self.panel {
            Some(Panel::Text(draft)) => draft.has_changes(),
            Some(Panel::Conversation(draft)) => {
                draft.editor.conversation() != &draft.committed
                    || draft
                        .field
                        .as_ref()
                        .is_some_and(|(_, view, baseline)| view.editor.text() != baseline)
            }
            Some(Panel::Picker(_)) | None => false,
        }
    }
    /// Full project replacement may have no text/font context. Dropping the old
    /// controllers revokes their unique targets without accepting more input.
    pub(super) fn revoke(&mut self) {
        self.panel = None;
        self.error = None;
    }
    fn text(&mut self) -> Option<&mut EditorView> {
        match self.panel.as_mut()? {
            Panel::Text(draft) => Some(&mut draft.view),
            Panel::Conversation(draft) => draft.field.as_mut().map(|(_, view, _)| view),
            Panel::Picker(draft) => Some(&mut draft.search),
        }
    }
    fn valid(&self, model: &GraphViewState) -> Result<(), GraphDraftError> {
        match &self.panel {
            Some(Panel::Text(draft)) => draft.validate(model.binding(), model.document()),
            Some(Panel::Conversation(draft)) => {
                if draft.binding != model.binding() {
                    return Err(GraphDraftError::WrongBinding);
                }
                let value = model
                    .document()
                    .nodes
                    .iter()
                    .find(|node| node.id == draft.editor.node)
                    .and_then(|node| node.rows.iter().find(|row| row.id == draft.row))
                    .ok_or(GraphDraftError::MissingTarget)?;
                if value.value != draft.expected {
                    return Err(GraphDraftError::SourceChanged);
                }
                Ok(())
            }
            Some(Panel::Picker(draft)) => draft.validate(model),
            None => Ok(()),
        }
    }
    pub(super) fn open_picker(
        &mut self,
        model: &mut GraphViewState,
        target: &TextTarget,
        picker: GraphPicker,
        fonts: &mut TextSystem,
    ) -> Result<(), GraphDraftError> {
        self.checkpoint(model)?;
        self.panel = Some(Panel::Picker(super::native_picker::NativePicker::new(
            model, target, picker, fonts,
        )?));
        self.error = None;
        Ok(())
    }
    pub(super) fn open_text(
        &mut self,
        model: &mut GraphViewState,
        focus: GraphTextFocus,
        fonts: &mut TextSystem,
    ) -> Result<(), GraphDraftError> {
        self.checkpoint(model)?;
        model.edit(|document, editor| editor.cancel_gesture(document));
        model.finish_text(true);
        model.publish_edits();
        let draft = GraphTextDraft::new(model.binding(), focus, model.document(), fonts)?;
        self.panel = Some(Panel::Text(draft));
        self.error = None;
        Ok(())
    }
    pub(super) fn open_conversation(
        &mut self,
        model: &mut GraphViewState,
        node: GraphId,
        view: [f32; 2],
    ) -> Result<(), GraphDraftError> {
        self.checkpoint(model)?;
        let (row, expected, conversation) = model
            .document()
            .nodes
            .iter()
            .find(|candidate| candidate.id == node)
            .and_then(|node| {
                node.rows
                    .iter()
                    .find(|row| row.key.as_deref() == Some("conversation"))
            })
            .and_then(|row| {
                let value = serde_json::to_value(&row.value).ok()?;
                Some((
                    row.id,
                    row.value.clone(),
                    Conversation::from_control(&value)?,
                ))
            })
            .ok_or(GraphDraftError::MissingTarget)?;
        model.edit(|document, editor| editor.cancel_gesture(document));
        model.finish_text(true);
        model.publish_edits();
        self.panel = Some(Panel::Conversation(ConversationDraft {
            binding: model.binding(),
            row,
            expected,
            committed: conversation.clone(),
            editor: ConversationEditor::new(node, conversation, view),
            field: None,
        }));
        self.error = None;
        Ok(())
    }
    fn flush_conversation_field(draft: &mut ConversationDraft) {
        if let Some((field, view, baseline)) = &mut draft.field
            && view.editor.text() != baseline
        {
            // Keep the existing conversation field normalization policy. The
            // EditorView itself retains exact input/history, including CRLF.
            draft.editor.set_text(*field, view.editor.text().to_owned());
            *baseline = view.editor.text().to_owned();
        }
    }
    /// Save/Play checkpoint: exclude preedit and retain field/controller history.
    /// No Project, history manager, renderer or window is accessed here.
    pub(super) fn checkpoint(
        &mut self,
        model: &mut GraphViewState,
    ) -> Result<Option<Arc<GraphCommit>>, GraphDraftError> {
        let result = (|| {
            self.valid(model)?;
            match &mut self.panel {
                Some(Panel::Text(draft)) => model.checkpoint_draft(draft),
                Some(Panel::Conversation(draft)) => {
                    Self::flush_conversation_field(draft);
                    // Opening a field or selecting an already active entry is
                    // not an authored edit. Keep exact raw JSON/unknown fields
                    // until the conversation's actual data changes.
                    if draft.editor.conversation() == &draft.committed {
                        return Ok(model.publish_edits());
                    }
                    let value = GraphControlValue::Custom {
                        kind: "conversation".into(),
                        data: serde_json::to_value(draft.editor.conversation()).unwrap(),
                    };
                    let commit = model.commit_control(
                        draft.binding,
                        draft.editor.node,
                        draft.row,
                        &draft.expected,
                        value.clone(),
                    )?;
                    draft.expected = value;
                    draft.committed = draft.editor.conversation().clone();
                    Ok(commit)
                }
                Some(Panel::Picker(_)) => Ok(model.publish_edits()),
                None => Ok(model.checkpoint_text()),
            }
        })();
        self.error = result.as_ref().err().map(ToString::to_string);
        result
    }
    /// Revoke transient input without discarding a recoverable private draft.
    pub(super) fn suspend(&mut self, fonts: &mut TextSystem) {
        if let Some(view) = self.text() {
            view.set_focused(fonts, false);
        }
    }
    pub(super) fn clear(&mut self, fonts: &mut TextSystem) {
        self.suspend(fonts);
        self.panel = None;
        self.error = None;
    }
    fn close(
        &mut self,
        model: &mut GraphViewState,
        fonts: &mut TextSystem,
        commit: bool,
    ) -> Result<Option<Arc<GraphCommit>>, GraphDraftError> {
        let result = if commit {
            self.checkpoint(model)?
        } else {
            None
        };
        self.clear(fonts);
        Ok(result)
    }
    fn open_field(&mut self, field: ConversationField, fonts: &mut TextSystem) {
        let Some(Panel::Conversation(draft)) = &mut self.panel else {
            return;
        };
        Self::flush_conversation_field(draft);
        if let Some((_, old, _)) = &mut draft.field {
            old.set_focused(fonts, false);
        }
        let Some(text) = draft.editor.text(field) else {
            return;
        };
        let mut view = EditorView::new(true);
        view.set_selection_drag_move(true);
        view.editor.set_text(fonts, &text);
        draft.field = Some((field, view, text));
    }
    fn prepare(&mut self, fonts: &mut TextSystem, outer: Rect, active: bool) {
        let mut rect = Rect::new(
            outer.x + outer.width * 0.15,
            outer.y + outer.height * 0.12,
            outer.width * 0.7,
            outer.height * 0.76,
        );
        if let Some(Panel::Conversation(draft)) = &mut self.panel {
            draft.editor.fit([outer.width, outer.height]);
            if let Some((field, _, _)) = &draft.field {
                let r = draft
                    .editor
                    .editor_rect(*field, [outer.width, outer.height]);
                rect = Rect::new(
                    outer.x + r.origin[0],
                    outer.y + r.origin[1],
                    r.size[0],
                    r.size[1],
                );
            }
        }
        let picker = if let Some(Panel::Picker(draft)) = &mut self.panel {
            draft.sync_query();
            draft.picker.fit([outer.width, outer.height]);
            if let Some(r) = draft.picker.search_rect() {
                rect = Rect::new(
                    outer.x + r.origin[0],
                    outer.y + r.origin[1],
                    r.size[0],
                    r.size[1],
                );
            }
            true
        } else {
            false
        };
        if let Some(view) = self.text() {
            let inner = if picker { rect } else { rect.inset(12.) };
            view.prepare(
                fonts,
                TextOptions {
                    family: "Roboto",
                    size: if picker { 13. } else { 14. },
                    width: Some(inner.width),
                    line_height: Some(if picker { 18. } else { 21. }),
                },
                inner,
                false,
            );
            view.set_focused(fonts, active);
        }
    }
}

/// Composition of the ordinary native graph and app-owned panels. The same
/// controller owns paint, keyboard/IME, menus and candidate geometry.
pub(super) struct NativeNodesView {
    graph: TheGraphView,
    assets: tiles::TileControls,
    definitions: Arc<GraphDefinitions>,
    live: Arc<live::LiveEvents>,
    allow_empty_tile: bool,
    previous_images: HashMap<String, Arc<TheRGBABuffer>>,
    panels: Arc<Mutex<NativeNodePanels>>,
    active: bool,
    dirty: bool,
    last_click: Option<(Instant, [f32; 2])>,
    pointer: [f32; 2],
    editor_click: Option<(Instant, [f32; 2], TextInputTarget, u8)>,
}
impl NativeNodesView {
    pub(super) fn with_state(
        id: TheId,
        model: Arc<Mutex<GraphViewState>>,
        panels: Arc<Mutex<NativeNodePanels>>,
    ) -> Self {
        let mut graph = TheGraphView::with_state(id, model);
        graph.set_auto_focus(true);
        graph.set_scroll_behavior(-0.6, false);
        graph.set_connection_policy(NodeConnectionPolicy);
        Self {
            graph,
            assets: tiles::TileControls::default(),
            definitions: Arc::new(GraphDefinitions::default()),
            live: Arc::new(live::LiveEvents::default()),
            allow_empty_tile: false,
            previous_images: HashMap::new(),
            panels,
            active: false,
            dirty: true,
            last_click: None,
            pointer: [0.; 2],
            editor_click: None,
        }
    }
    pub(super) fn expect_clipboard(&mut self, binding: u64, request: u64) -> bool {
        self.graph.expect_clipboard(binding, request)
    }
    pub(super) fn model(&self) -> Arc<Mutex<GraphViewState>> {
        self.graph.model()
    }
    pub(super) fn set_assets(&mut self, assets: tiles::TileControls) {
        self.graph.set_controls(assets.clone());
        self.assets = assets;
        self.refresh_context();
    }
    pub(super) fn set_snapshot(
        &mut self,
        definitions: Arc<GraphDefinitions>,
        live: Arc<live::LiveEvents>,
        allow_empty_tile: bool,
    ) {
        self.definitions = definitions;
        self.live = live;
        self.allow_empty_tile = allow_empty_tile;
        self.refresh_context();
    }
    fn refresh_context(&mut self) {
        self.graph.set_context(live::NativeContext {
            definitions: self.definitions.clone(),
            live: self.live.clone(),
            equipment_names: self.assets.equipment_names.clone(),
        });
        self.dirty = true;
    }
    fn is_panel(&self) -> bool {
        self.panels.lock().unwrap().panel.is_some()
    }
    fn is_focused(&self, ctx: &TheContext) -> bool {
        self.active && ctx.ui.has_focus(self.id())
    }
    fn prepare_panel(&mut self, ctx: &mut TheContext) {
        let active = self.is_focused(ctx);
        if self.is_panel() {
            self.graph.cancel_clipboard();
        }
        self.graph
            .editor_set_active(active && !self.is_panel(), ctx);
        let mut panels = self.panels.lock().unwrap();
        let model = self.graph.model();
        if let Err(error) = panels.valid(&model.lock().unwrap()) {
            panels.error = Some(error.to_string());
            panels.suspend(&mut ctx.text);
            return;
        }
        panels.error = None;
        panels.prepare(&mut ctx.text, bounds(self.dim()), active);
    }
    fn notify(&mut self, commit: Option<Arc<GraphCommit>>, ctx: &mut TheContext) {
        if let Some(commit) = commit {
            ctx.ui
                .send(TheEvent::GraphCommitted(self.id().clone(), commit));
        }
        self.dirty = true;
    }
    fn response(&mut self, response: &EditorResponse, ctx: &mut TheContext) {
        if let Some(request) = &response.clipboard {
            ctx.ui.editor_clipboard.push_back(request.clone());
        }
        self.dirty |= response.input.redraw || response.changed;
    }
    fn panel_input(&mut self, event: &InputEvent, ctx: &mut TheContext) -> EditorResponse {
        self.prepare_panel(ctx);
        let mut panels = self.panels.lock().unwrap();
        if panels.error.is_some() || !self.is_focused(ctx) {
            return EditorResponse::default();
        }
        if let Some(Panel::Picker(draft)) = &mut panels.panel
            && !draft.search.editor.is_composing()
            && let InputEvent::Key {
                physical,
                pressed: true,
                ..
            } = event
        {
            use theframework::platform::PhysicalKey;
            match physical {
                PhysicalKey::Up | PhysicalKey::Down => {
                    draft
                        .picker
                        .navigate_rows(if *physical == PhysicalKey::Up { -1 } else { 1 });
                    self.dirty = true;
                    return EditorResponse {
                        input: InputResponse {
                            consumed: true,
                            redraw: true,
                        },
                        ..Default::default()
                    };
                }
                PhysicalKey::Enter => {
                    let item = draft.picker.selected_id().map(str::to_owned);
                    if let Some(item) = item {
                        match draft.select(&mut self.graph.model().lock().unwrap(), &item) {
                            Ok(commit) => {
                                panels.clear(&mut ctx.text);
                                drop(panels);
                                self.notify(commit, ctx);
                                return EditorResponse {
                                    input: InputResponse {
                                        consumed: true,
                                        redraw: true,
                                    },
                                    ..Default::default()
                                };
                            }
                            Err(error) => panels.error = Some(error.to_string()),
                        }
                    }
                    return EditorResponse {
                        input: InputResponse {
                            consumed: true,
                            redraw: true,
                        },
                        ..Default::default()
                    };
                }
                _ => {}
            }
        }
        if panels.text().is_none()
            && matches!(
                event,
                InputEvent::Key {
                    physical: theframework::platform::PhysicalKey::Escape,
                    pressed: true,
                    ..
                }
            )
        {
            let commit = panels
                .close(&mut self.graph.model().lock().unwrap(), &mut ctx.text, true)
                .ok()
                .flatten();
            drop(panels);
            self.notify(commit, ctx);
            return EditorResponse {
                input: InputResponse {
                    consumed: true,
                    redraw: true,
                },
                ..Default::default()
            };
        }
        let response = panels
            .text()
            .map(|view| view.input(&mut ctx.text, event))
            .unwrap_or_default();
        if let Some(Panel::Picker(draft)) = &mut panels.panel {
            draft.sync_query();
        }
        let commit = if response.action == Some(EditorAction::Cancel) {
            if let Some(Panel::Conversation(draft)) = &mut panels.panel {
                if let Some((_, view, _)) = &mut draft.field {
                    view.set_focused(&mut ctx.text, false);
                }
                draft.field = None;
                None
            } else {
                panels
                    .close(
                        &mut self.graph.model().lock().unwrap(),
                        &mut ctx.text,
                        false,
                    )
                    .ok()
                    .flatten()
            }
        } else {
            None
        };
        drop(panels);
        self.response(&response, ctx);
        if response.action.is_some() {
            self.notify(commit, ctx);
        }
        response
    }
}
impl TheWidget for NativeNodesView {
    fn new(id: TheId) -> Self {
        Self::with_state(
            id,
            Arc::new(Mutex::new(GraphViewState::new(
                "",
                GraphDocument::default(),
            ))),
            Arc::new(Mutex::new(NativeNodePanels::default())),
        )
    }
    fn id(&self) -> &TheId {
        self.graph.id()
    }
    fn dim(&self) -> &TheDim {
        self.graph.dim()
    }
    fn dim_mut(&mut self) -> &mut TheDim {
        self.graph.dim_mut()
    }
    fn limiter(&self) -> &TheSizeLimiter {
        self.graph.limiter()
    }
    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        self.graph.limiter_mut()
    }
    fn set_dim(&mut self, dim: TheDim, ctx: &mut TheContext) {
        self.graph.set_dim(dim, ctx);
        self.dirty = true;
    }
    fn needs_redraw(&mut self) -> bool {
        self.dirty || self.graph.needs_redraw()
    }
    fn set_needs_redraw(&mut self, dirty: bool) {
        self.dirty = dirty;
        self.graph.set_needs_redraw(dirty);
    }
    fn supports_hover(&mut self) -> bool {
        true
    }
    fn supports_precise_scroll(&self) -> bool {
        true
    }
    fn supports_pinch(&self) -> bool {
        !self.is_panel()
    }
    fn uses_native_editor(&self) -> bool {
        true
    }
    fn supports_editor_focus(&self) -> bool {
        true
    }
    fn supports_text_input(&self) -> bool {
        if self.is_panel() {
            self.panels.lock().unwrap().text().is_some()
        } else {
            self.graph.supports_text_input()
        }
    }
    fn supports_undo_redo(&mut self) -> bool {
        !self.is_panel() || self.supports_text_input()
    }
    fn supports_clipboard(&mut self) -> bool {
        !self.is_panel() || self.supports_text_input()
    }
    fn editor_set_active(&mut self, active: bool, ctx: &mut TheContext) {
        self.active = active;
        self.graph
            .editor_set_active(active && !self.is_panel(), ctx);
        self.prepare_panel(ctx);
    }
    fn editor_input(&mut self, event: &InputEvent, ctx: &mut TheContext) -> Option<EditorResponse> {
        if self.is_panel() {
            Some(self.panel_input(event, ctx))
        } else {
            self.graph.editor_input(event, ctx)
        }
    }
    fn editor_command(
        &mut self,
        command: EditorCommand,
        ctx: &mut TheContext,
    ) -> Option<EditorResponse> {
        if !self.is_panel() {
            return self.graph.editor_command(command, ctx);
        }
        self.prepare_panel(ctx);
        let mut panels = self.panels.lock().unwrap();
        let response = if panels.error.is_none() && self.is_focused(ctx) {
            panels
                .text()
                .map(|view| {
                    let response = view.editor.execute(&mut ctx.text, command);
                    view.reveal_caret(&mut ctx.text);
                    response
                })
                .unwrap_or_default()
        } else {
            EditorResponse::default()
        };
        drop(panels);
        self.response(&response, ctx);
        Some(response)
    }
    fn editor_input_state(&mut self, ctx: &mut TheContext) -> Option<TextInputState> {
        if !self.is_panel() {
            return self.graph.editor_input_state(ctx);
        }
        self.prepare_panel(ctx);
        let mut panels = self.panels.lock().unwrap();
        if panels.error.is_some() {
            return None;
        }
        panels.text()?.text_input_state(&mut ctx.text)
    }
    fn prepare_paint(&mut self, ctx: &mut TheContext, theme: &dyn TheTheme) {
        self.graph.prepare_paint(ctx, theme);
        self.prepare_panel(ctx);
    }
    fn paint(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let outer = bounds(self.dim());
        let theme = GraphTheme {
            background: *p.theme.color(MaterialCanvas),
            grid: *p.theme.color(MaterialGrid),
            body: *p.theme.color(MaterialPanel),
            control: *p.theme.color(MaterialControl),
            text: *p.theme.color(MaterialText),
            muted: *p.theme.color(MaterialMuted),
            wire: *p.theme.color(MaterialWire),
            active: *p.theme.color(SelectedTextEditBorder1),
            ..GraphTheme::default()
        };
        let active = self.active && p.ui.has_focus(self.id()) && !self.is_panel();
        let prefix = format!("nodes:{}", self.id().uuid);
        let images = super::native_images::NativeImageFrame::new(
            prefix,
            &self.assets,
            p.images,
            &mut self.previous_images,
        );
        self.graph
            .paint_commands(p.list, p.text, &images, &theme, active);
        let mut panels = self.panels.lock().unwrap();
        if let Some(Panel::Conversation(draft)) = &mut panels.panel {
            p.list.ops.push(PaintOp::PushClip(outer));
            p.list.ops.push(PaintOp::PushTransform(Transform::translate(
                outer.x, outer.y,
            )));
            draft.editor.paint(
                &mut PaintGraphPainter {
                    paint: p.list,
                    text: p.text,
                    images: &images,
                    family: "Roboto",
                },
                &theme,
            );
            p.list.ops.push(PaintOp::PopTransform);
            p.list.ops.push(PaintOp::PopClip);
        }
        if let Some(Panel::Picker(draft)) = &mut panels.panel {
            p.list.ops.push(PaintOp::PushClip(outer));
            p.list.ops.push(PaintOp::PushTransform(Transform::translate(
                outer.x, outer.y,
            )));
            draft.picker.paint_without_query(&mut PaintGraphPainter {
                paint: p.list,
                text: p.text,
                images: &images,
                family: "Roboto",
            });
            p.list.ops.push(PaintOp::PopTransform);
            p.list.ops.push(PaintOp::PopClip);
        }
        let picker = matches!(panels.panel, Some(Panel::Picker(_)));
        if let Some(view) = panels.text() {
            let rect = view.bounds();
            if !picker {
                p.list.rect(
                    Rect::new(
                        rect.x - 12.,
                        rect.y - 12.,
                        rect.width + 24.,
                        rect.height + 24.,
                    ),
                    8.,
                    rgba(theme.body),
                );
            }
            view.paint(
                p.text,
                p.list,
                EditorColors {
                    text: rgba(theme.text),
                    selection: [0.16, 0.44, 0.53, 0.8],
                    caret: rgba(theme.text),
                    composition: rgba(theme.active),
                },
                true,
            );
        }
        let error = panels.error.clone();
        drop(panels);
        images.finish()?;
        if active {
            p.outline(outer, *p.theme.color(DefaultSelection), 1.);
        }
        if let Some(error) = error {
            p.label(
                outer.inset(16.),
                &error,
                13.,
                theme.active,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Top,
            );
        }
        self.dirty = false;
        Ok(())
    }
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if let TheEvent::MouseDown(point)
        | TheEvent::MouseUp(point)
        | TheEvent::MouseDragged(point)
        | TheEvent::Hover(point) = event
        {
            self.pointer = [point.x as f32, point.y as f32];
        }
        if self.is_panel() {
            match event {
                TheEvent::MouseDown(point) => {
                    ctx.ui.set_focus(self.id());
                    self.active = true;
                    self.prepare_panel(ctx);
                    let local = [point.x as f32, point.y as f32];
                    let position = [
                        local[0] + self.dim().x as f32,
                        local[1] + self.dim().y as f32,
                    ];
                    let mut panels = self.panels.lock().unwrap();
                    if panels
                        .text()
                        .is_some_and(|view| view.bounds().contains(position))
                    {
                        let view = panels.text().unwrap();
                        let target = view.editor.input_target();
                        let clicks = self
                            .editor_click
                            .filter(|(time, old, old_target, _)| {
                                *old_target == target
                                    && time.elapsed() < Duration::from_millis(500)
                                    && (old[0] - position[0]).powi(2)
                                        + (old[1] - position[1]).powi(2)
                                        <= 16.
                            })
                            .map_or(1, |(_, _, _, clicks)| clicks % 3 + 1);
                        self.editor_click = Some((Instant::now(), position, target, clicks));
                        view.pointer_down(&mut ctx.text, position, clicks);
                    } else {
                        if let Some(Panel::Picker(draft)) = &mut panels.panel {
                            if let Some(item) = draft.picker.pick(local) {
                                let result =
                                    draft.select(&mut self.graph.model().lock().unwrap(), &item);
                                match result {
                                    Ok(commit) => {
                                        panels.clear(&mut ctx.text);
                                        drop(panels);
                                        self.notify(commit, ctx);
                                        return true;
                                    }
                                    Err(error) => panels.error = Some(error.to_string()),
                                }
                            } else if !draft.picker.contains(local) {
                                panels.clear(&mut ctx.text);
                            }
                            self.dirty = true;
                            return true;
                        }
                        if let Some(Panel::Conversation(draft)) = &mut panels.panel {
                            Self::flush_field_for_click(draft, &mut ctx.text);
                        }
                        let click = match &mut panels.panel {
                            Some(Panel::Conversation(draft)) => draft.editor.pointer_down(local),
                            _ => ConversationClick::Outside,
                        };
                        let commit = match click {
                            ConversationClick::Edit(field) => {
                                panels.open_field(field, &mut ctx.text);
                                None
                            }
                            ConversationClick::Handled => None,
                            ConversationClick::Outside => panels
                                .close(&mut self.graph.model().lock().unwrap(), &mut ctx.text, true)
                                .ok()
                                .flatten(),
                        };
                        drop(panels);
                        self.notify(commit, ctx);
                        self.prepare_panel(ctx);
                    }
                    self.dirty = true;
                    return true;
                }
                TheEvent::MouseDragged(point) | TheEvent::MouseUp(point) => {
                    let position = [
                        point.x as f32 + self.dim().x as f32,
                        point.y as f32 + self.dim().y as f32,
                    ];
                    let event = if matches!(event, TheEvent::MouseUp(_)) {
                        InputEvent::PointerButton {
                            button: PointerButton::Primary,
                            pressed: false,
                            position,
                        }
                    } else {
                        InputEvent::PointerMoved { position }
                    };
                    self.panel_input(&event, ctx);
                    return true;
                }
                TheEvent::LostFocus(_) => {
                    self.active = false;
                    self.panels.lock().unwrap().suspend(&mut ctx.text);
                    self.dirty = true;
                    return true;
                }
                TheEvent::MouseWheel(delta) => {
                    self.scroll_panel([delta.x as f32, delta.y as f32], ctx);
                    return true;
                }
                TheEvent::PreciseScroll(delta) => {
                    self.scroll_panel([delta.x, delta.y], ctx);
                    return true;
                }
                TheEvent::Hover(point) => {
                    if let Some(Panel::Picker(draft)) = &mut self.panels.lock().unwrap().panel {
                        self.dirty |= draft.picker.hover(Some([point.x as f32, point.y as f32]));
                    }
                    return self.dirty;
                }
                _ => return false,
            }
        }
        if let TheEvent::MouseDown(point) = event {
            let position = [point.x as f32, point.y as f32];
            let model = self.graph.model();
            let mut panels = self.panels.lock().unwrap();
            let mut model = model.lock().unwrap();
            if let Some((target, picker)) = super::native_picker::picker_at(
                &model,
                position,
                [self.dim().width as f32, self.dim().height as f32],
                &self.definitions,
                &self.assets,
                self.allow_empty_tile,
            ) {
                let result = panels.open_picker(&mut model, &target, picker, &mut ctx.text);
                let commit = model.pending_through(u64::MAX).last().cloned();
                if let Err(error) = result {
                    panels.error = Some(error.to_string());
                }
                drop(model);
                drop(panels);
                self.graph.editor_set_active(false, ctx);
                self.active = true;
                ctx.ui.set_focus(self.id());
                self.last_click = None;
                self.prepare_panel(ctx);
                self.notify(commit, ctx);
                return true;
            }
            drop(model);
            drop(panels);
            let double = self.last_click.is_some_and(|(time, previous)| {
                time.elapsed() < Duration::from_millis(400)
                    && (position[0] - previous[0]).abs() < 5.
                    && (position[1] - previous[1]).abs() < 5.
            });
            self.last_click = Some((Instant::now(), position));
            if double {
                let model = self.graph.model();
                let mut panels = self.panels.lock().unwrap();
                let mut model = model.lock().unwrap();
                let focus = model.editor().text_focus().cloned();
                let graph_point = model.editor().viewport.to_graph(position);
                let node = model
                    .document()
                    .nodes
                    .iter()
                    .rev()
                    .find(|node| {
                        node.definition.as_deref() == Some("talk")
                            && node.rect(&model.document().metrics()).contains(graph_point)
                    })
                    .map(|node| node.id);
                let result = if let Some(node) = node {
                    panels.open_conversation(
                        &mut model,
                        node,
                        [self.dim().width as f32, self.dim().height as f32],
                    )
                } else if let Some(focus) = focus {
                    panels.open_text(&mut model, focus, &mut ctx.text)
                } else {
                    drop(model);
                    drop(panels);
                    return self.graph.on_event(event, ctx);
                };
                if result.is_ok() {
                    let commit = model.pending_through(u64::MAX).last().cloned();
                    drop(model);
                    drop(panels);
                    self.graph.editor_set_active(false, ctx);
                    self.active = true;
                    ctx.ui.set_focus(self.id());
                    self.prepare_panel(ctx);
                    self.notify(commit, ctx);
                    return true;
                }
            }
        }
        let changed = self.graph.on_event(event, ctx);
        match event {
            TheEvent::MouseDown(_) => self.active = ctx.ui.has_focus(self.id()),
            TheEvent::LostFocus(_) => self.active = false,
            _ => {}
        }
        changed
    }
    fn draw(&mut self, _: &mut TheRGBABuffer, _: &mut Box<dyn TheStyle>, _: &mut TheContext) {
        panic!("NativeNodesView requires command painting");
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
impl NativeNodesView {
    fn scroll_panel(&mut self, delta: [f32; 2], ctx: &mut TheContext) {
        let mut panels = self.panels.lock().unwrap();
        if let Some(Panel::Picker(draft)) = &mut panels.panel {
            draft.picker.scroll_pixels(-delta[1]);
        } else if let Some(view) = panels.text() {
            // TheUI's legacy event envelope already supplies logical pixels.
            view.scroll_by(&mut ctx.text, delta, true);
        } else if let Some(Panel::Conversation(draft)) = &mut panels.panel {
            draft.editor.scroll_at(self.pointer, -delta[1]);
        }
        self.dirty = true;
    }
    fn flush_field_for_click(draft: &mut ConversationDraft, fonts: &mut TextSystem) {
        NativeNodePanels::flush_conversation_field(draft);
        if let Some((_, view, _)) = &mut draft.field {
            view.set_focused(fonts, false);
        }
        draft.field = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use theframework::{
        paint::{ImageHandle, PaintList},
        platform::{LogicalKey, PhysicalKey},
        text_edit::EditorClipboard,
        theui::paint::{UiImageResources, UiPaintError},
    };
    struct NoImages;
    impl UiImageResources for NoImages {
        fn image(
            &mut self,
            _: &str,
            _: &TheRGBABuffer,
            _: bool,
        ) -> Result<ImageHandle, UiPaintError> {
            panic!("native graph/panels must not upload a CPU framebuffer");
        }
    }
    struct Fixture {
        ui: TheUI,
        ctx: TheContext,
        id: TheId,
        model: Arc<Mutex<GraphViewState>>,
        panels: Arc<Mutex<NativeNodePanels>>,
        node: GraphId,
    }
    impl Fixture {
        fn new() -> Self {
            let node = catalog::definitions()
                .node("talk")
                .unwrap()
                .instantiate([40., 40.]);
            let node_id = node.id;
            let model = Arc::new(Mutex::new(GraphViewState::new(
                "behavior/test",
                GraphDocument {
                    nodes: vec![node],
                    ..Default::default()
                },
            )));
            let panels = Arc::new(Mutex::new(NativeNodePanels::default()));
            let mut ctx = TheContext::new(1000, 700, 1.5);
            ctx.ui.command_paint = true;
            let mut ui = TheUI::new();
            ui.init(&mut ctx);
            let id = TheId::named("Native Nodes");
            let mut side = TheCanvas::new();
            let mut spacer = TheSpacer::new(TheId::named("left"));
            spacer.limiter_mut().set_max_width(80);
            side.set_widget(spacer);
            ui.canvas.set_left(side);
            let mut center = TheCanvas::new();
            center.set_widget(NativeNodesView::with_state(
                id.clone(),
                model.clone(),
                panels.clone(),
            ));
            ui.canvas.set_center(center);
            ui.prepare_paint_layout(&mut ctx);
            panels
                .lock()
                .unwrap()
                .open_conversation(&mut model.lock().unwrap(), node_id, [920., 700.])
                .unwrap();
            panels
                .lock()
                .unwrap()
                .open_field(ConversationField::StepLine(0), &mut ctx.text);
            ctx.ui.set_focus(&id);
            ui.get_widget("Native Nodes")
                .unwrap()
                .editor_set_active(true, &mut ctx);
            let mut f = Self {
                ui,
                ctx,
                id,
                model,
                panels,
                node: node_id,
            };
            f.paint();
            f
        }
        fn paint(&mut self) -> PaintList {
            self.ui
                .paint_commands(&mut self.ctx, &mut NoImages)
                .unwrap()
        }
        fn state(&mut self) -> TextInputState {
            self.ui
                .get_widget("Native Nodes")
                .unwrap()
                .editor_input_state(&mut self.ctx)
                .unwrap()
        }
        fn send(&mut self, event: InputEvent) -> InputResponse {
            self.ui.prepare_paint_layout(&mut self.ctx);
            self.ui.dispatch_input(&event, &mut self.ctx)
        }
        fn command(&mut self, command: EditorCommand) {
            self.ui
                .get_widget("Native Nodes")
                .unwrap()
                .editor_command(command, &mut self.ctx);
            self.ui.process_events(&mut self.ctx);
        }
        fn checkpoint(&mut self) -> Result<Option<Arc<GraphCommit>>, GraphDraftError> {
            self.panels
                .lock()
                .unwrap()
                .checkpoint(&mut self.model.lock().unwrap())
        }
        fn conversation(&self) -> Conversation {
            NodesDock::conversation_of(&self.model.lock().unwrap().document().nodes[0]).unwrap()
        }
    }
    #[test]
    fn native_node_conversation_save_keeps_ime_history_and_source_owned_transactions() {
        let mut f = Fixture::new();
        let before = f.model.lock().unwrap().document().clone();
        let state = f.state();
        let dim = *f.ui.get_widget("Native Nodes").unwrap().dim();
        assert!(dim.x > 0);
        assert!(state.area.x >= dim.x as f32);
        f.command(EditorCommand::SelectAll);
        assert!(f.send(InputEvent::Text("Line\r\n東京".into())).consumed);
        assert!(
            f.send(InputEvent::CompositionFor {
                target: state.target,
                text: "日本".into(),
                cursor: Some((0, 6))
            })
            .consumed
        );
        assert_eq!(
            f.conversation().steps[0].text,
            "Hello.",
            "private panel is not Project source"
        );
        let commit = f.checkpoint().unwrap().unwrap();
        assert_eq!(commit.edits.len(), 1);
        assert_eq!(commit.serial, 1);
        assert_eq!(f.conversation().steps[0].text, "Line\r\n東京");
        assert_eq!(f.state().target, state.target);
        assert!(
            f.panels
                .lock()
                .unwrap()
                .text()
                .unwrap()
                .editor
                .is_composing()
        );
        assert!(f.checkpoint().unwrap().is_none());
        f.command(EditorCommand::Undo);
        assert_eq!(
            f.panels.lock().unwrap().text().unwrap().editor.text(),
            "Hello."
        );
        let undo = f.checkpoint().unwrap().unwrap();
        assert_eq!(undo.serial, 2);
        assert_eq!(f.conversation().steps[0].text, "Hello.");
        let mut source = before.clone();
        for edit in commit.edits.iter().chain(undo.edits.iter()) {
            edit.apply(&mut source, true);
        }
        assert_eq!(source, before);
        assert_eq!(f.model.lock().unwrap().pending_commits().count(), 2);
        f.command(EditorCommand::SelectAll);
        f.command(EditorCommand::Copy);
        assert!(
            matches!(f.ctx.ui.editor_clipboard.pop_front(), Some(EditorClipboard::Write { text, .. }) if text == "Hello.")
        );
        f.paint();
    }
    #[test]
    fn native_node_conversation_replacement_revokes_callbacks_and_preserves_conflicting_private_text()
     {
        let mut f = Fixture::new();
        let target = f.state().target;
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Private draft".into()));
        f.command(EditorCommand::Paste);
        let Some(EditorClipboard::Read { request }) = f.ctx.ui.editor_clipboard.pop_front() else {
            panic!("paste request");
        };
        let doc = f.model.lock().unwrap().document().clone();
        f.model.lock().unwrap().replace_document("other", doc);
        assert!(matches!(f.checkpoint(), Err(GraphDraftError::WrongBinding)));
        assert!(
            !f.send(InputEvent::TextFor {
                target,
                text: "stale".into()
            })
            .consumed
        );
        assert!(
            !f.send(InputEvent::Clipboard {
                request,
                result: theframework::platform::ClipboardResult::Text(Some(
                    "stale clipboard".into()
                ))
            })
            .consumed
        );
        assert_eq!(f.conversation().steps[0].text, "Hello.");
        assert_eq!(
            f.panels.lock().unwrap().text().unwrap().editor.text(),
            "Private draft"
        );
        assert!(f.model.lock().unwrap().pending_commits().next().is_none());
        assert!(
            f.ui.get_widget("Native Nodes")
                .unwrap()
                .editor_input_state(&mut f.ctx)
                .is_none()
        );
        f.panels.lock().unwrap().clear(&mut f.ctx.text);
        assert!(
            !f.ui
                .get_widget("Native Nodes")
                .unwrap()
                .supports_text_input()
        );
    }
    #[test]
    fn native_node_conversation_field_switch_revokes_old_ime_and_resize_keeps_the_new_editor() {
        let mut f = Fixture::new();
        let old = f.state().target;
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Authored line".into()));
        f.panels
            .lock()
            .unwrap()
            .open_field(ConversationField::ChoiceLabel(0, 0), &mut f.ctx.text);
        f.paint();
        let next = f.state().target;
        assert_ne!(next, old);
        f.send(InputEvent::TextFor {
            target: old,
            text: "stale".into(),
        });
        assert_eq!(
            f.panels.lock().unwrap().text().unwrap().editor.text(),
            "Leave"
        );
        f.ui.get_widget("Native Nodes")
            .unwrap()
            .set_dim(TheDim::new(120, 60, 640, 420), &mut f.ctx);
        f.ui.get_widget("Native Nodes")
            .unwrap()
            .prepare_paint(&mut f.ctx, &TheBlackBlueTheme::new());
        let resized = f.state();
        assert_eq!(resized.target, next);
        assert!(resized.area.x >= 120. && resized.area.x < 760.);
        assert!(resized.area.y >= 60. && resized.area.y < 480.);
        f.checkpoint().unwrap().unwrap();
        assert_eq!(f.conversation().steps[0].text, "Authored line");
        assert_eq!(f.conversation().steps[0].choices[0].label, "Leave");
        assert_eq!(f.model.lock().unwrap().document().nodes[0].id, f.node);
        assert_eq!(f.ctx.ui.focus, Some(f.id.clone()));
    }
    fn key(physical: PhysicalKey) -> InputEvent {
        InputEvent::Key {
            physical,
            logical: LogicalKey::Other,
            pressed: true,
            repeat: false,
        }
    }
    fn click(f: &mut Fixture, position: [f32; 2]) {
        for pressed in [true, false] {
            f.send(InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed,
                position,
            });
        }
    }
    #[test]
    fn native_node_picker_search_uses_ime_clipboard_and_keyboard_selection_without_authored_query_text()
     {
        let mut f = Fixture::new();
        f.panels.lock().unwrap().clear(&mut f.ctx.text);
        let row = GraphRow::new(
            "Choice",
            GraphControlValue::Choice {
                options: (0..50).map(|i| format!("Choice 日本 {i}")).collect(),
                selected: 0,
            },
        );
        let target = TextTarget {
            node: f.node,
            row: row.id,
            cell: None,
        };
        f.model
            .lock()
            .unwrap()
            .edit(|doc, _| doc.nodes[0].rows.push(row));
        let items = (0..50)
            .map(|i| GraphPickerItem {
                id: i.to_string(),
                label: format!("Choice 日本 {i}"),
            })
            .collect();
        let picker = GraphPicker::compact(
            GraphRect {
                origin: [100., 80.],
                size: [230., 26.],
            },
            [920., 700.],
            items,
        );
        f.panels
            .lock()
            .unwrap()
            .open_picker(
                &mut f.model.lock().unwrap(),
                &target,
                picker,
                &mut f.ctx.text,
            )
            .unwrap();
        f.paint();
        let state = f.state();
        f.send(InputEvent::PointerMoved {
            position: [state.area.x + 5., state.area.y + 45.],
        });
        f.send(InputEvent::Scroll {
            delta: [0., -48.],
            precise: true,
        });
        {
            let panels = f.panels.lock().unwrap();
            let Some(Panel::Picker(draft)) = &panels.panel else {
                panic!("picker");
            };
            assert_eq!(draft.picker.scroll, 2);
        }
        f.send(InputEvent::PointerMoved {
            position: [state.area.x + 5., state.area.y + 5.],
        });

        f.send(InputEvent::CompositionFor {
            target: state.target,
            text: "日本".into(),
            cursor: Some((0, 6)),
        });
        assert!(f.checkpoint().unwrap().is_none());
        {
            let panels = f.panels.lock().unwrap();
            let Some(Panel::Picker(draft)) = &panels.panel else {
                panic!("picker");
            };
            assert!(draft.search.editor.is_composing());
            assert!(draft.picker.query.is_empty());
        }
        f.send(InputEvent::TextFor {
            target: state.target,
            text: "Choice 日本 2".into(),
        });
        f.paint();
        f.send(key(PhysicalKey::Down));
        f.send(key(PhysicalKey::Down));
        f.paint();
        {
            let panels = f.panels.lock().unwrap();
            let Some(Panel::Picker(draft)) = &panels.panel else {
                panic!("picker");
            };
            assert_eq!(draft.picker.query, "Choice 日本 2");
            assert_eq!(draft.picker.selected_id(), Some("20"));
        }
        f.command(EditorCommand::SelectAll);
        f.command(EditorCommand::Copy);
        assert!(
            matches!(f.ctx.ui.editor_clipboard.pop_front(),Some(EditorClipboard::Write { text,.. }) if text == "Choice 日本 2")
        );
        assert!(f.model.lock().unwrap().pending_commits().next().is_none());
        assert!(f.send(key(PhysicalKey::Enter)).consumed);
        assert!(f.panels.lock().unwrap().panel.is_none());
        assert!(matches!(
            f.model.lock().unwrap().document().nodes[0]
                .rows
                .last()
                .unwrap()
                .value,
            GraphControlValue::Choice { selected: 20, .. }
        ));
        assert_eq!(f.model.lock().unwrap().pending_commits().count(), 1);
        assert!(
            !f.send(InputEvent::TextFor {
                target: state.target,
                text: "stale".into()
            })
            .consumed
        );
    }
    #[test]
    fn native_node_multiline_double_click_checkpoints_exact_text_and_closes_without_replaying_old_callbacks()
     {
        let mut f = Fixture::new();
        f.panels.lock().unwrap().clear(&mut f.ctx.text);
        let mut node = GraphNode::new("Text", [40., 40.], [60, 80, 100, 255]);
        node.rows.push(GraphRow::new(
            "Text",
            GraphControlValue::Text("Original 東京".into()),
        ));
        f.model.lock().unwrap().replace_document(
            "text",
            GraphDocument {
                nodes: vec![node],
                ..Default::default()
            },
        );
        f.paint();
        let dim = *f.ui.get_widget("Native Nodes").unwrap().dim();
        let model = f.model.lock().unwrap();
        let row = model.document().nodes[0].row_rect(0, &model.document().metrics());
        let p = model
            .editor()
            .viewport
            .to_screen([row.origin[0] + 15., row.origin[1] + 10.]);
        drop(model);
        let p = [p[0] + dim.x as f32, p[1] + dim.y as f32];
        click(&mut f, p);
        click(&mut f, p);
        assert!(matches!(
            f.panels.lock().unwrap().panel,
            Some(Panel::Text(_))
        ));
        f.paint();
        let state = f.state();
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("A\r\n東京\nB".into()));
        let commit = f.checkpoint().unwrap().unwrap();
        assert_eq!(commit.edits.len(), 1);
        assert_eq!(f.state().target, state.target);
        f.command(EditorCommand::Undo);
        let undo = f.checkpoint().unwrap().unwrap();
        assert_eq!(undo.serial, 2);
        assert_eq!(
            f.model.lock().unwrap().document().nodes[0].rows[0].value,
            GraphControlValue::Text("Original 東京".into())
        );
        let point = {
            let mut panels = f.panels.lock().unwrap();
            let r = panels.text().unwrap().bounds();
            [r.x + 4., r.y + 8.]
        };
        click(&mut f, point);
        click(&mut f, point);
        f.command(EditorCommand::Copy);
        assert!(
            matches!(f.ctx.ui.editor_clipboard.pop_front(),Some(EditorClipboard::Write { text,.. }) if text == "Original")
        );
        f.send(key(PhysicalKey::Escape));
        assert!(f.panels.lock().unwrap().panel.is_none());
        assert!(
            !f.send(InputEvent::TextFor {
                target: state.target,
                text: "stale".into()
            })
            .consumed
        );
        // Repeated blank clicks must not reenter TheGraphView holding its model lock.
        click(&mut f, [900., 650.]);
        click(&mut f, [900., 650.]);
        f.paint();
    }
    #[test]
    fn native_node_images_resolve_visible_sources_once_and_detect_immutable_snapshot_replacement() {
        struct Images {
            calls: Vec<(String, bool)>,
        }
        impl UiImageResources for Images {
            fn image(
                &mut self,
                key: &str,
                image: &TheRGBABuffer,
                changed: bool,
            ) -> Result<ImageHandle, UiPaintError> {
                assert_eq!((image.pixel_width(), image.pixel_height()), (4, 4));
                self.calls.push((key.into(), changed));
                Ok(ImageHandle {
                    owner: 1,
                    slot: 0,
                    generation: 1,
                })
            }
        }
        let mut f = Fixture::new();
        f.panels.lock().unwrap().clear(&mut f.ctx.text);
        let make_node = |position, key: &str| {
            let mut node = GraphNode::new("Images", position, [50, 80, 100, 255]);
            for _ in 0..2 {
                node.rows.push(GraphRow::new(
                    "Tile",
                    GraphControlValue::Custom {
                        kind: "tile".into(),
                        data: key.into(),
                    },
                ));
            }
            node
        };
        f.model.lock().unwrap().replace_document(
            "images",
            GraphDocument {
                nodes: vec![
                    make_node([40., 40.], "visible"),
                    make_node([15000., 40.], "hidden"),
                ],
                ..Default::default()
            },
        );
        let mut assets = tiles::TileControls::default();
        for key in ["visible", "hidden"] {
            let mut image = TheRGBABuffer::new(TheDim::sized(4, 4));
            image.fill([30, 180, 70, 255]);
            assets.set_preview(key, Some(image));
        }
        f.ui.get_widget("Native Nodes")
            .unwrap()
            .as_any()
            .downcast_mut::<NativeNodesView>()
            .unwrap()
            .set_assets(assets.clone());
        let mut images = Images { calls: vec![] };
        let list = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        assert_eq!(list.image_handles().count(), 2);
        assert_eq!(images.calls.len(), 1);
        assert!(images.calls[0].0.ends_with(":visible") && images.calls[0].1);
        f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        assert_eq!(images.calls.len(), 2);
        assert!(!images.calls[1].1);
        let mut replacement = TheRGBABuffer::new(TheDim::sized(4, 4));
        replacement.fill([210, 40, 20, 255]);
        assets.set_preview("visible", Some(replacement));
        f.ui.get_widget("Native Nodes")
            .unwrap()
            .as_any()
            .downcast_mut::<NativeNodesView>()
            .unwrap()
            .set_assets(assets);
        f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        assert_eq!(images.calls.len(), 3);
        assert!(images.calls[2].1);
        let emitter = rusterix::ParticleEmitter::new(vek::Vec3::zero(), vek::Vec3::unit_y());
        let frame = crate::docks::particle_preview::prepare_particle_emitter_preview(
            &emitter, 280, 160, 0.68,
        );
        let mut assets = tiles::TileControls::default();
        assets.set_radial_preview("visible", &frame);
        f.ui.get_widget("Native Nodes")
            .unwrap()
            .as_any()
            .downcast_mut::<NativeNodesView>()
            .unwrap()
            .set_assets(assets);
        let list = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        assert_eq!(
            list.ops
                .iter()
                .filter(|op| matches!(op, PaintOp::PixelRadialLayer { .. }))
                .count(),
            2
        );
        assert_eq!(list.image_handles().count(), 0);
        assert_eq!(
            images.calls.len(),
            3,
            "procedural graph previews cannot upload a CPU display bitmap"
        );
        f.model
            .lock()
            .unwrap()
            .edit(|_, editor| editor.viewport.pan = [-5000., -5000.]);
        f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        assert_eq!(images.calls.len(), 3);
        assert!(
            f.ui.get_widget("Native Nodes")
                .unwrap()
                .as_any()
                .downcast_mut::<NativeNodesView>()
                .unwrap()
                .previous_images
                .is_empty()
        );
    }

    #[cfg(feature = "native-render")]
    #[test]
    #[ignore = "requires a native GPU"]
    fn native_node_panels_gpu_paint_keeps_japanese_preedit_without_cpu_panel_images() {
        use renderkit::*;
        use theframework::render_wgpu::{PaintRenderer, UiImages};
        let mut f = Fixture::new();
        f.ui.set_theme(Box::new(TheBlackBlueTheme::new()), &mut f.ctx);
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text(
            "Line one\r\n東京 and e\u{301}\nمرحبا".into(),
        ));
        let target = f.state().target;
        f.send(InputEvent::CompositionFor {
            target,
            text: "日本".into(),
            cursor: Some((0, 6)),
        });
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut images = UiImages::new(gpu.clone());
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1500, 1050)).unwrap();
        let list = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        assert_eq!(
            f.panels
                .lock()
                .unwrap()
                .text()
                .unwrap()
                .editor
                .layout(&mut f.ctx.text)
                .lines()
                .len(),
            3
        );
        assert_eq!(list.image_handles().count(), 0);
        images.finish_frame(&list).unwrap();
        renderer
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (1500, 1050),
                1.5,
                [0.02, 0.02, 0.02, 1.],
            )
            .unwrap();
        assert!(renderer.glyphs.rasterizations > 0);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        assert!(f.ui.canvas.buffer.pixels().is_empty());
        let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let capture = ticket.try_finish(&gpu).unwrap().unwrap();
        let file = std::fs::File::create("/private/tmp/eldiron-native-node-panels.png").unwrap();
        let mut encoder = png::Encoder::new(file, 1500, 1050);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(capture.pixels())
            .unwrap();
        assert_eq!(f.state().target, target);
        assert!(
            f.panels
                .lock()
                .unwrap()
                .text()
                .unwrap()
                .editor
                .is_composing()
        );
        f.checkpoint().unwrap().unwrap();
        assert_eq!(
            f.conversation().steps[0].text,
            "Line one\r\n東京 and e\u{301}\nمرحبا"
        );
    }
    #[test]
    fn native_node_panel_replacement_checkpoints_prior_text_and_rejects_its_late_input() {
        let mut f = Fixture::new();
        let old = f.state().target;
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Saved on next panel".into()));
        let row = GraphRow::new(
            "Choice",
            GraphControlValue::Choice {
                options: vec!["One".into(), "Two".into()],
                selected: 0,
            },
        );
        let target = TextTarget {
            node: f.node,
            row: row.id,
            cell: None,
        };
        f.model
            .lock()
            .unwrap()
            .edit(|doc, _| doc.nodes[0].rows.push(row));
        let picker = GraphPicker::new(
            "Choice",
            [30., 30.],
            vec![
                GraphPickerItem {
                    id: "0".into(),
                    label: "One".into(),
                },
                GraphPickerItem {
                    id: "1".into(),
                    label: "Two".into(),
                },
            ],
        );
        f.panels
            .lock()
            .unwrap()
            .open_picker(
                &mut f.model.lock().unwrap(),
                &target,
                picker,
                &mut f.ctx.text,
            )
            .unwrap();
        f.paint();
        assert_eq!(f.conversation().steps[0].text, "Saved on next panel");
        assert_eq!(f.model.lock().unwrap().last_serial(), 1);
        assert_ne!(f.state().target, old);
        assert!(
            !f.send(InputEvent::TextFor {
                target: old,
                text: "stale".into()
            })
            .consumed
        );
        assert_eq!(f.panels.lock().unwrap().text().unwrap().editor.text(), "");
        f.send(key(PhysicalKey::Down));
        f.send(key(PhysicalKey::Down));
        f.send(key(PhysicalKey::Enter));
        assert_eq!(f.model.lock().unwrap().last_serial(), 2);
        assert_eq!(f.conversation().steps[0].text, "Saved on next panel");
    }
    #[test]
    fn native_node_open_and_save_preserves_unedited_conversation_json_including_unknown_fields() {
        let mut f = Fixture::new();
        f.panels.lock().unwrap().clear(&mut f.ctx.text);
        f.model.lock().unwrap().edit(|doc, _| {
            let GraphControlValue::Custom { data, .. } = &mut doc.nodes[0].rows[0].value else {
                unreachable!()
            };
            data["author_notes"] = serde_json::json!({"future":"Keep this metadata"});
            data["steps"][0]["voice"] = "speaker-a".into();
            data["steps"][0]["text"] = "  Raw\r\n東京  ".into();
        });
        let before = f.model.lock().unwrap().document().clone();
        f.panels
            .lock()
            .unwrap()
            .open_conversation(&mut f.model.lock().unwrap(), f.node, [920., 700.])
            .unwrap();
        f.panels
            .lock()
            .unwrap()
            .open_field(ConversationField::StepLine(0), &mut f.ctx.text);
        f.paint();
        assert!(
            f.checkpoint().unwrap().is_none(),
            "opening/saving must not rewrite authored JSON"
        );
        assert_eq!(f.model.lock().unwrap().document(), &before);
        assert!(f.model.lock().unwrap().pending_commits().next().is_none());
    }
    #[test]
    fn native_node_pointer_opens_row_and_list_choices_with_one_source_owned_transaction() {
        for table in [false, true] {
            let mut f = Fixture::new();
            f.panels.lock().unwrap().clear(&mut f.ctx.text);
            let choice = GraphControlValue::Choice {
                options: vec!["One".into(), "Two 日本".into()],
                selected: 0,
            };
            let mut node = GraphNode::new("Selection", [40., 40.], [50, 80, 110, 255]);
            node.rows.push(GraphRow::new(
                "Choose",
                if table {
                    GraphControlValue::List {
                        columns: vec![GraphListColumn {
                            id: "choice".into(),
                            label: "Choice".into(),
                            control: choice.clone(),
                        }],
                        row_ids: vec![Uuid::new_v4()],
                        rows: vec![vec![choice]],
                    }
                } else {
                    choice
                },
            ));
            f.node = node.id;
            f.model.lock().unwrap().replace_document(
                "choice-owner",
                GraphDocument {
                    nodes: vec![node],
                    ..Default::default()
                },
            );
            let before = f.model.lock().unwrap().document().clone();
            {
                let view =
                    f.ui.get_widget("Native Nodes")
                        .unwrap()
                        .as_any()
                        .downcast_mut::<NativeNodesView>()
                        .unwrap();
                view.set_snapshot(
                    Arc::new(catalog::definitions()),
                    Arc::new(live::LiveEvents::default()),
                    false,
                );
            }
            f.paint();
            let point = {
                let model = f.model.lock().unwrap();
                let metrics = model.document().metrics();
                let rect = model.document().nodes[0].row_rect(0, &metrics);
                let local = model.editor().viewport.to_screen([
                    rect.origin[0] + 12.,
                    rect.origin[1] + if table { metrics.list_header + 8. } else { 8. },
                ]);
                let dim = f.ui.get_widget("Native Nodes").unwrap().dim();
                [local[0] + dim.x as f32, local[1] + dim.y as f32]
            };
            click(&mut f, point);
            f.paint();
            let state = f.state();
            assert!(matches!(
                f.panels.lock().unwrap().panel,
                Some(Panel::Picker(_))
            ));
            assert_eq!(f.model.lock().unwrap().document(), &before);
            assert!(f.model.lock().unwrap().pending_commits().next().is_none());
            f.send(InputEvent::TextFor {
                target: state.target,
                text: "Two 日本".into(),
            });
            f.paint();
            f.send(key(PhysicalKey::Down));
            f.send(key(PhysicalKey::Enter));
            assert!(f.panels.lock().unwrap().panel.is_none());
            let after = f.model.lock().unwrap().document().clone();
            let value = &after.nodes[0].rows[0].value;
            let value = if table {
                let GraphControlValue::List { rows, .. } = value else {
                    panic!("table")
                };
                &rows[0][0]
            } else {
                value
            };
            assert!(matches!(
                value,
                GraphControlValue::Choice { selected: 1, .. }
            ));
            let packet = f.model.lock().unwrap().pending_through(u64::MAX);
            assert_eq!(packet.len(), 1);
            assert_eq!(packet[0].edits.len(), 1);
            let mut restored = after.clone();
            packet[0].edits[0].apply(&mut restored, false);
            assert_eq!(restored, before);
            assert!(
                !f.send(InputEvent::TextFor {
                    target: state.target,
                    text: "stale query".into()
                })
                .consumed
            );
            assert_eq!(f.model.lock().unwrap().document(), &after);
        }
    }
    #[test]
    fn native_node_focus_loss_releases_active_paint_even_when_the_host_keeps_logical_focus() {
        let mut f = Fixture::new();
        f.panels.lock().unwrap().clear(&mut f.ctx.text);
        let widget = f.ui.get_widget("Native Nodes").unwrap();
        let view = widget.as_any().downcast_mut::<NativeNodesView>().unwrap();
        view.on_event(&TheEvent::MouseDown(Vec2::new(800, 600)), &mut f.ctx);
        assert!(view.active);
        assert!(f.ctx.ui.has_focus(&f.id));
        view.on_event(&TheEvent::LostFocus(f.id.clone()), &mut f.ctx);
        assert!(
            !view.active,
            "window/host focus loss must end active native painting"
        );
        assert!(view.editor_input_state(&mut f.ctx).is_none());
    }
}
