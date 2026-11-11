//! Native graph widget binding. Applications own the authoring state; the view
//! routes TheUI scope/clipboard/IME to the same controller used for painting.
use super::*;
use crate::{
    paint::{PaintOp, Transform},
    platform::{InputEvent, PointerButton, TextInputState},
    prelude::*,
    text_edit::{EditorCommand, EditorResponse},
    theui::paint::{UiPaintResult, WidgetPainter, bounds},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
static NEXT_BINDING: AtomicU64 = AtomicU64::new(1);

/// Per-view CPU authoring state retained by the application across UI tree or
/// renderer replacement. Use a distinct controller for each visible graph view;
/// authoritative project data/undo still belongs to the application. Do not hold
/// this lock across UI callbacks. GraphCommitted carries completed edits, rather
/// than asking the application to persist another unfinished gesture's draft.
pub struct GraphViewState {
    document: GraphDocument,
    editor: GraphEditor,
    text: GraphTextSession,
    owner: String,
    binding: u64,
    revision: u64,
    serial: u64,
    acknowledged: u64,
    pending: std::collections::VecDeque<Arc<GraphCommit>>,
}
impl GraphViewState {
    pub fn new(owner: impl Into<String>, document: GraphDocument) -> Self {
        Self {
            document,
            editor: GraphEditor::default(),
            text: GraphTextSession::default(),
            owner: owner.into(),
            binding: NEXT_BINDING.fetch_add(1, Ordering::Relaxed),
            revision: 1,
            serial: 0,
            acknowledged: 0,
            pending: std::collections::VecDeque::new(),
        }
    }
    pub fn document(&self) -> &GraphDocument {
        &self.document
    }
    pub fn editor(&self) -> &GraphEditor {
        &self.editor
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn binding(&self) -> u64 {
        self.binding
    }
    /// Explicit external replacement starts a new callback/command epoch, even
    /// with identical authored IDs/content. Ordinary acknowledgements need not
    /// replace this working model; consume committed GraphEdits in order instead.
    pub fn replace_document(&mut self, owner: impl Into<String>, document: GraphDocument) {
        let owner = owner.into();
        let viewport = (self.owner == owner).then_some(self.editor.viewport);
        self.owner = owner;
        self.document = document;
        self.editor = GraphEditor::default();
        if let Some(viewport) = viewport {
            self.editor.viewport = viewport;
        }
        self.text.reset();
        self.binding = NEXT_BINDING.fetch_add(1, Ordering::Relaxed);
        self.serial = 0;
        self.acknowledged = 0;
        self.pending.clear();
        self.invalidate();
    }
    /// Application commands mutate the CPU graph/controller and invalidate its
    /// presentation. This never initializes a GPU or a raster font cache.
    pub fn edit<R>(&mut self, edit: impl FnOnce(&mut GraphDocument, &mut GraphEditor) -> R) -> R {
        let result = edit(&mut self.document, &mut self.editor);
        self.invalidate();
        result
    }
    pub fn finish_text(&mut self, commit: bool) {
        self.editor.finish_text(&mut self.document, commit);
        self.text.reset();
        self.invalidate();
    }
    /// Publish newly completed controller edits once. The returned value may be
    /// sent as a UI notification; pending copies remain until owner acknowledgement.
    pub fn publish_edits(&mut self) -> Option<Arc<GraphCommit>> {
        let edits = self.editor.take_edits();
        if edits.is_empty() {
            return None;
        }
        self.serial = self
            .serial
            .checked_add(1)
            .expect("graph commit serial exhausted");
        let commit = Arc::new(GraphCommit {
            binding: self.binding,
            serial: self.serial,
            edits,
        });
        self.pending.push_back(commit.clone());
        Some(commit)
    }
    pub fn pending_commits(&self) -> impl Iterator<Item = &GraphCommit> {
        self.pending.iter().map(Arc::as_ref)
    }
    /// Lease an ordered prefix for application outside this model's lock. UI
    /// wake-ups and the retained queue share payloads instead of cloning graphs.
    pub fn pending_through(&self, serial: u64) -> Vec<Arc<GraphCommit>> {
        self.pending
            .iter()
            .take_while(|commit| commit.serial <= serial)
            .cloned()
            .collect()
    }
    pub fn last_serial(&self) -> u64 {
        self.serial
    }
    /// Acknowledge an applied prefix. Old binding/future/regressing serials are
    /// rejected; an already acknowledged serial is an idempotent success.
    pub fn acknowledge(&mut self, binding: u64, serial: u64) -> bool {
        if binding != self.binding || serial > self.serial || serial < self.acknowledged {
            return false;
        }
        self.acknowledged = serial;
        while self
            .pending
            .front()
            .is_some_and(|commit| commit.serial <= serial)
        {
            self.pending.pop_front();
        }
        true
    }
    /// Capture committed field content for Save/Play without cancelling preedit,
    /// changing the native target, or clearing the text controller's undo history.
    pub fn checkpoint_text(&mut self) -> Option<Arc<GraphCommit>> {
        self.editor.checkpoint_text(&mut self.document);
        self.publish_edits()
    }

    pub fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Native graph control. Its CPU state is explicitly supplied by its application,
/// and its image handles are supplied by the frame's shared resource domain.
pub struct TheGraphView {
    id: TheId,
    dim: TheDim,
    limiter: TheSizeLimiter,
    state: Arc<Mutex<GraphViewState>>,
    context: Box<dyn GraphContext + Send + Sync>,
    controls: Box<dyn GraphControls + Send + Sync>,
    connections: Box<dyn GraphConnectionPolicy + Send + Sync>,
    images: Arc<dyn GraphImageHandles + Send + Sync>,
    theme: Option<GraphTheme>,
    active: bool,
    auto_focus: bool,
    dragging: bool,
    dirty: bool,
    painted_revision: u64,
}
impl TheGraphView {
    pub fn with_state(id: TheId, state: Arc<Mutex<GraphViewState>>) -> Self {
        let mut view = Self::new(id);
        view.state = state;
        view
    }
    pub fn model(&self) -> Arc<Mutex<GraphViewState>> {
        self.state.clone()
    }
    pub fn set_context(&mut self, context: impl GraphContext + Send + Sync + 'static) {
        self.context = Box::new(context);
        self.dirty = true;
    }
    pub fn set_controls(&mut self, controls: impl GraphControls + Send + Sync + 'static) {
        self.controls = Box::new(controls);
        self.dirty = true;
    }
    pub fn set_connection_policy(
        &mut self,
        policy: impl GraphConnectionPolicy + Send + Sync + 'static,
    ) {
        self.connections = Box::new(policy);
    }
    pub fn set_image_handles(&mut self, images: Arc<dyn GraphImageHandles + Send + Sync>) {
        if !Arc::ptr_eq(&self.images, &images) {
            self.images = images;
            self.dirty = true;
        }
    }
    pub fn set_theme(&mut self, theme: Option<GraphTheme>) {
        self.theme = theme;
        self.dirty = true;
    }
    pub fn set_auto_focus(&mut self, active: bool) {
        self.auto_focus = active;
    }
    fn focused(&self, ctx: &TheContext) -> bool {
        self.active && ctx.ui.has_focus(&self.id)
    }
    fn notify(&mut self, ctx: &mut TheContext) {
        let commit = self.state.lock().unwrap().publish_edits();
        if let Some(commit) = commit {
            ctx.ui
                .send(TheEvent::GraphCommitted(self.id.clone(), commit));
        }
    }
    fn response(&mut self, response: &EditorResponse, ctx: &mut TheContext) {
        self.dirty |= response.input.redraw || response.changed;
        if let Some(request) = &response.clipboard {
            ctx.ui.editor_clipboard.push_back(request.clone());
        }
        self.notify(ctx);
    }
    fn prepare_session(&mut self, ctx: &mut TheContext) {
        let active = self.focused(ctx);
        let mut state = self.state.lock().unwrap();
        let GraphViewState {
            document,
            editor,
            text,
            ..
        } = &mut *state;
        text.input(
            editor,
            document,
            &mut ctx.text,
            &InputEvent::Modifiers(ctx.ui.input_modifiers),
            active,
        );
        text.prepare(editor, document, &mut ctx.text, active);
    }
    fn edit_input(&mut self, event: &InputEvent, ctx: &mut TheContext) -> EditorResponse {
        let active = self.focused(ctx);
        let response = {
            let mut state = self.state.lock().unwrap();
            let GraphViewState {
                document,
                editor,
                text,
                ..
            } = &mut *state;
            text.input(
                editor,
                document,
                &mut ctx.text,
                &InputEvent::Modifiers(ctx.ui.input_modifiers),
                active,
            );
            text.input(editor, document, &mut ctx.text, event, active)
        };
        self.response(&response, ctx);
        response
    }
    fn local_event(&self, event: &InputEvent) -> InputEvent {
        let local = |position: [f32; 2]| {
            [
                position[0] - self.dim.x as f32,
                position[1] - self.dim.y as f32,
            ]
        };
        match event {
            InputEvent::PointerMoved { position } => InputEvent::PointerMoved {
                position: local(*position),
            },
            InputEvent::PointerButton {
                button,
                pressed,
                position,
            } => InputEvent::PointerButton {
                button: *button,
                pressed: *pressed,
                position: local(*position),
            },
            _ => event.clone(),
        }
    }
    fn scroll(&mut self, delta: [f32; 2], ctx: &TheContext) {
        let aspect = self.dim.width.max(1) as f32 / self.dim.height.max(1) as f32;
        let dx = delta[0] * -0.4 / aspect.max(1.);
        let dy = delta[1] * -0.4 * aspect.min(1.);
        let mut state = self.state.lock().unwrap();
        if ctx.ui.input_modifiers.control || ctx.ui.input_modifiers.super_key {
            let cursor = state.editor.cursor;
            state.editor.viewport.zoom_at(cursor, (dy * 0.01).exp());
        } else {
            state.editor.viewport.pan[0] -= dx;
            state.editor.viewport.pan[1] -= dy;
        }
        state.invalidate();
    }
}
impl TheWidget for TheGraphView {
    fn new(id: TheId) -> Self {
        Self {
            id,
            dim: TheDim::zero(),
            limiter: TheSizeLimiter::new(),
            state: Arc::new(Mutex::new(GraphViewState::new(
                "",
                GraphDocument::default(),
            ))),
            context: Box::new(()),
            controls: Box::new(BasicGraphControls),
            connections: Box::new(AllowGraphConnections),
            images: Arc::new(()),
            theme: None,
            active: false,
            auto_focus: false,
            dragging: false,
            dirty: true,
            painted_revision: 0,
        }
    }
    fn id(&self) -> &TheId {
        &self.id
    }
    fn dim(&self) -> &TheDim {
        &self.dim
    }
    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }
    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }
    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }
    fn set_dim(&mut self, dim: TheDim, ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.dirty = true;
            ctx.ui.send(TheEvent::WidgetResized(self.id.clone(), dim));
        }
    }
    fn needs_redraw(&mut self) -> bool {
        self.dirty || self.painted_revision != self.state.lock().unwrap().revision
    }
    fn set_needs_redraw(&mut self, dirty: bool) {
        self.dirty = dirty;
    }
    fn supports_hover(&mut self) -> bool {
        true
    }
    fn supports_text_input(&self) -> bool {
        self.state.lock().unwrap().editor.text_focus().is_some()
    }
    fn uses_native_editor(&self) -> bool {
        true
    }
    fn supports_editor_focus(&self) -> bool {
        true
    }
    fn supports_undo_redo(&mut self) -> bool {
        self.supports_text_input()
    }
    fn supports_clipboard(&mut self) -> bool {
        self.supports_text_input()
    }

    fn editor_set_active(&mut self, active: bool, ctx: &mut TheContext) {
        if self.active != active {
            if !active && self.dragging {
                let mut state = self.state.lock().unwrap();
                let GraphViewState {
                    document, editor, ..
                } = &mut *state;
                editor.cancel_gesture(document);
                self.dragging = false;
            }
            self.active = active;
            self.dirty = true;
        }
        self.prepare_session(ctx);
    }
    fn editor_input(&mut self, event: &InputEvent, ctx: &mut TheContext) -> Option<EditorResponse> {
        if let InputEvent::Modifiers(modifiers) = event {
            ctx.ui.input_modifiers = *modifiers;
        }
        if !self.supports_text_input() && matches!(event, InputEvent::Key { .. }) {
            // Diagram accelerators/deletion keep their application owner.
            return None;
        }
        let event = self.local_event(event);
        Some(self.edit_input(&event, ctx))
    }
    fn editor_command(
        &mut self,
        command: EditorCommand,
        ctx: &mut TheContext,
    ) -> Option<EditorResponse> {
        let active = self.focused(ctx);
        if !self.supports_text_input() {
            if active {
                let binding = self.state.lock().unwrap().binding;
                ctx.ui
                    .send(TheEvent::GraphCommand(self.id.clone(), binding, command));
            }
            return Some(EditorResponse {
                input: crate::platform::InputResponse {
                    consumed: true,
                    redraw: false,
                },
                ..Default::default()
            });
        }
        let response = {
            let mut state = self.state.lock().unwrap();
            let GraphViewState {
                document,
                editor,
                text,
                ..
            } = &mut *state;
            text.command(editor, document, &mut ctx.text, command, active)
        };
        self.response(&response, ctx);
        Some(response)
    }
    fn editor_input_state(&mut self, ctx: &mut TheContext) -> Option<TextInputState> {
        self.prepare_session(ctx);
        let active = self.focused(ctx);
        let mut state = self.state.lock().unwrap();
        let GraphViewState {
            document,
            editor,
            text,
            ..
        } = &mut *state;
        let mut input = text.input_state(editor, document, &mut ctx.text, active)?;
        input.area.x += self.dim.x as f32;
        input.area.y += self.dim.y as f32;
        Some(input)
    }
    fn prepare_paint(&mut self, ctx: &mut TheContext, _: &dyn TheTheme) {
        self.prepare_session(ctx);
    }
    fn paint(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let rect = bounds(&self.dim);
        let theme = self.theme.clone().unwrap_or_else(|| GraphTheme {
            background: *p.theme.color(MaterialCanvas),
            grid: *p.theme.color(MaterialGrid),
            body: *p.theme.color(MaterialPanel),
            control: *p.theme.color(MaterialControl),
            text: *p.theme.color(MaterialText),
            muted: *p.theme.color(MaterialMuted),
            wire: *p.theme.color(MaterialWire),
            active: *p.theme.color(SelectedTextEditBorder1),
            ..GraphTheme::default()
        });
        p.list.ops.push(PaintOp::PushClip(rect));
        p.list
            .ops
            .push(PaintOp::PushTransform(Transform::translate(rect.x, rect.y)));
        let active = self.active && p.ui.has_focus(&self.id);
        let mut state = self.state.lock().unwrap();
        let GraphViewState {
            document,
            editor,
            text,
            ..
        } = &mut *state;
        text.paint(
            editor,
            document,
            self.context.as_ref(),
            self.controls.as_ref(),
            PaintGraphPainter {
                paint: p.list,
                text: p.text,
                images: self.images.as_ref(),
                family: "Roboto",
            },
            [rect.width, rect.height],
            &theme,
            active,
        );
        self.painted_revision = state.revision;
        drop(state);
        p.list.ops.push(PaintOp::PopTransform);
        p.list.ops.push(PaintOp::PopClip);
        if active {
            p.outline(rect, *p.theme.color(DefaultSelection), 1.);
        }
        self.dirty = false;
        Ok(())
    }
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(point) => {
                ctx.ui.set_focus(&self.id);
                self.active = true;
                self.dragging = true;
                let binding = self.state.lock().unwrap().binding;
                ctx.ui
                    .send(TheEvent::GraphInteractionStarted(self.id.clone(), binding));
                let point = [point.x as f32, point.y as f32];
                let response = {
                    let mut state = self.state.lock().unwrap();
                    let GraphViewState {
                        document,
                        editor,
                        text,
                        ..
                    } = &mut *state;
                    text.input(
                        editor,
                        document,
                        &mut ctx.text,
                        &InputEvent::Modifiers(ctx.ui.input_modifiers),
                        true,
                    );
                    if ctx.ui.input_modifiers.shift
                        && !text.bounds().is_some_and(|r| r.contains(point))
                    {
                        editor.begin_cut(document, point);
                        EditorResponse::default()
                    } else {
                        text.pointer_down(
                            editor,
                            document,
                            &mut ctx.text,
                            point,
                            self.controls.as_ref(),
                        )
                        .unwrap_or_default()
                    }
                };
                self.response(&response, ctx);
                self.dirty = true;
            }
            TheEvent::MouseDragged(point) if self.dragging => {
                let point = [point.x as f32, point.y as f32];
                let response = self.edit_input(&InputEvent::PointerMoved { position: point }, ctx);
                if !response.input.consumed {
                    let mut state = self.state.lock().unwrap();
                    let GraphViewState {
                        document, editor, ..
                    } = &mut *state;
                    editor.pointer_move(document, point, self.controls.as_ref());
                    self.dirty = true;
                }
            }
            TheEvent::MouseUp(point) if self.dragging => {
                self.dragging = false;
                let point = [point.x as f32, point.y as f32];
                let response = self.edit_input(
                    &InputEvent::PointerButton {
                        button: PointerButton::Primary,
                        pressed: false,
                        position: point,
                    },
                    ctx,
                );
                if !response.input.consumed {
                    let mut state = self.state.lock().unwrap();
                    let GraphViewState {
                        document, editor, ..
                    } = &mut *state;
                    editor.pointer_up(document, point, self.connections.as_ref());
                }
                self.notify(ctx);
                self.dirty = true;
            }
            TheEvent::Hover(point) => {
                ctx.ui.set_hover(&self.id);
                if self.auto_focus {
                    ctx.ui.set_focus(&self.id);
                }
                let mut state = self.state.lock().unwrap();
                state.editor.cursor = [point.x as f32, point.y as f32];
                self.dirty |=
                    state.editor.pending_wire().is_some() || state.editor.cut_path.is_some();
            }
            TheEvent::MouseWheel(delta) => {
                self.scroll([delta.x as f32, delta.y as f32], ctx);
            }
            TheEvent::PreciseScroll(delta) => {
                self.scroll([delta.x, delta.y], ctx);
            }
            TheEvent::Pinch(delta) => {
                let mut state = self.state.lock().unwrap();
                let cursor = state.editor.cursor;
                state.editor.viewport.zoom_at(cursor, (1. + delta).max(0.1));
                state.invalidate();
            }
            TheEvent::LostFocus(_) => {
                self.active = false;
                self.dragging = false;
                let mut state = self.state.lock().unwrap();
                let GraphViewState {
                    document, editor, ..
                } = &mut *state;
                editor.cancel_gesture(document);
                state.finish_text(true);
                drop(state);
                self.notify(ctx);
                self.dirty = true;
            }
            TheEvent::Context(point) => {
                ctx.ui.set_focus(&self.id);
                ctx.ui
                    .send(TheEvent::RenderViewContext(self.id.clone(), *point));
            }
            TheEvent::Drop(point, drop) => {
                let (binding, position) = {
                    let state = self.state.lock().unwrap();
                    (
                        state.binding,
                        state
                            .editor
                            .viewport
                            .to_graph([point.x as f32, point.y as f32]),
                    )
                };
                ctx.ui.send(TheEvent::GraphDrop(
                    self.id.clone(),
                    binding,
                    position,
                    drop.clone(),
                ));
            }
            _ => return false,
        }
        self.needs_redraw()
    }
    fn draw(&mut self, _: &mut TheRGBABuffer, _: &mut Box<dyn TheStyle>, _: &mut TheContext) {
        panic!(
            "TheGraphView requires native command painting; use the explicit reference graph adapter for CPU output"
        );
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
