//! Two-phase diagram clipboard commands. Source mutation stays in the CPU
//! workspace; widgets only route host completions carrying opaque request IDs.
use super::*;
use theframework::{platform::ClipboardResult, text_edit::EditorClipboard};
const CLIPBOARD_LIMIT: usize = 8 * 1024 * 1024;
pub struct Pending {
    key: NodeSourceKey,
    widget: TheId,
    request: u64,
    intent: Arc<GraphClipboardIntent>,
}
fn kind(key: &NodeSourceKey) -> &'static str {
    match key.kind() {
        NodeGraphKind::Configuration => "eldiron.configuration",
        _ => "eldiron.behavior",
    }
}
impl NodesDock {
    fn clipboard_scope(&self, id: &TheId, intent: &GraphClipboardIntent) -> Option<NodeSourceKey> {
        let key = self.native.cpu.binding_source(intent.binding)?.clone();
        let view = self.native.views.get(&key)?;
        (&view.id == id && view.model.lock().unwrap().binding() == intent.binding).then_some(key)
    }
    fn clipboard_unchanged(&self, key: &NodeSourceKey, intent: &GraphClipboardIntent) -> bool {
        let Some(view) = self.native.views.get(key) else {
            return false;
        };
        if view.panels.lock().unwrap().is_open() {
            return false;
        }
        let model = view.model.lock().unwrap();
        model.binding() == intent.binding
            && model.last_serial() == intent.serial
            && model.document() == intent.document.as_ref()
            && model.editor().selected == intent.selected
            && model.editor().selected_connection == intent.connection
            && model.editor().text_focus().is_none()
            && !model.editor().is_gesturing()
    }
    fn begin_graph_clipboard(
        &mut self,
        id: &TheId,
        intent: Arc<GraphClipboardIntent>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> Result<bool, String> {
        let Some(key) = self.clipboard_scope(id, &intent) else {
            return Ok(false);
        };
        self.native_clipboard = None;
        if intent.action != GraphClipboardAction::Copy && !self.clipboard_unchanged(&key, &intent) {
            return Ok(false);
        }
        // Copy honors its immutable input-time selection. Cut/paste additionally
        // require the source/selection to remain unchanged until completion.
        let selection = intent.selection();
        if intent.action != GraphClipboardAction::Paste && selection.nodes.is_empty() {
            return Ok(false);
        }
        if intent.action == GraphClipboardAction::Copy {
            let text = GraphClipboard::new(kind(&key), selection)
                .encode(CLIPBOARD_LIMIT)
                .map_err(|error| error.to_string())?;
            ctx.ui
                .editor_clipboard
                .push_back(EditorClipboard::Write { text, cut: None });
            return Ok(true);
        }
        if self.native.active.as_ref() != Some(&key) || !ctx.ui.has_focus(id) {
            return Ok(false);
        }
        self.native
            .prepare_source_command(project, &key, &mut ctx.text)?;
        if !self.clipboard_unchanged(&key, &intent) {
            return Ok(false);
        }
        let request = theframework::platform::new_clipboard_request();
        let operation = if intent.action == GraphClipboardAction::Cut {
            EditorClipboard::Write {
                text: GraphClipboard::new(kind(&key), selection)
                    .encode(CLIPBOARD_LIMIT)
                    .map_err(|error| error.to_string())?,
                cut: Some(request),
            }
        } else {
            EditorClipboard::Read { request }
        };
        let registered = ui
            .get_widget(VIEW)
            .filter(|widget| widget.id() == id)
            .and_then(|widget| widget.as_any().downcast_mut::<NativeNodesView>())
            .is_some_and(|widget| widget.expect_clipboard(intent.binding, request));
        if !registered {
            return Ok(false);
        }
        self.native_clipboard = Some(Pending {
            key,
            widget: id.clone(),
            request,
            intent,
        });
        ctx.ui.editor_clipboard.push_back(operation);
        Ok(true)
    }
    fn complete_graph_clipboard(
        &mut self,
        id: &TheId,
        binding: u64,
        request: u64,
        result: &ClipboardResult,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> Result<bool, String> {
        if self.native_clipboard.as_ref().is_none_or(|pending| {
            pending.request != request || &pending.widget != id || pending.intent.binding != binding
        }) {
            return Ok(false);
        }
        let pending = self.native_clipboard.take().unwrap();
        let key = &pending.key;
        if self.native.active.as_ref() != Some(key)
            || !ctx.ui.has_focus(id)
            || ui.get_widget(VIEW).is_none_or(|widget| widget.id() != id)
            || self.clipboard_scope(id, &pending.intent).as_ref() != Some(key)
            || !self.clipboard_unchanged(key, &pending.intent)
        {
            return Ok(false);
        }
        match (pending.intent.action, result) {
            (GraphClipboardAction::Cut, ClipboardResult::Written(true)) => {
                let nodes = pending.intent.nodes.iter().copied().collect();
                self.native
                    .edit_source(project, key, &mut ctx.text, |doc, definitions| {
                        remove_native_nodes(doc, &nodes);
                        let to = if key.kind() == NodeGraphKind::Behavior
                            && !doc.nodes.is_empty()
                            && !doc
                                .nodes
                                .iter()
                                .any(|node| is_branch_trigger(node, definitions))
                        {
                            NodeSourceKey {
                                owner: key.owner.clone(),
                                branch: None,
                            }
                        } else {
                            key.clone()
                        };
                        Ok(to)
                    })
            }
            (GraphClipboardAction::Paste, ClipboardResult::Text(Some(text))) => {
                let mut clipboard = GraphClipboard::decode(text, kind(key), CLIPBOARD_LIMIT)
                    .map_err(|error| error.to_string())?
                    .document;
                if clipboard.nodes.is_empty() {
                    return Ok(false);
                }
                let origin = clipboard
                    .nodes
                    .iter()
                    .fold([f32::INFINITY; 2], |origin, node| {
                        [
                            origin[0].min(node.position[0]),
                            origin[1].min(node.position[1]),
                        ]
                    });
                for node in &mut clipboard.nodes {
                    node.position[0] += pending.intent.position[0] - origin[0] - PASTE_OFFSET;
                    node.position[1] += pending.intent.position[1] - origin[1] - PASTE_OFFSET;
                }
                self.native
                    .paste_branch_document(project, key, &clipboard, &mut ctx.text)
            }
            _ => Ok(false),
        }
    }
    pub(super) fn native_clipboard_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        reference: &CreatorReference,
    ) -> Option<bool> {
        let result = match event {
            TheEvent::GraphClipboardIntent(id, intent) if id.name == VIEW => {
                self.begin_graph_clipboard(id, intent.clone(), ui, ctx, project)
            }
            TheEvent::GraphClipboardResult(id, binding, request, result) if id.name == VIEW => {
                self.complete_graph_clipboard(id, *binding, *request, result, ui, ctx, project)
            }
            _ => return None,
        };
        let handled = self.native_status(ctx, result);
        self.native_effects(project, ctx, reference);
        self.sync_native(project);
        self.prepare_native_layout();
        self.render_native(ui, ctx);
        Some(handled)
    }
}
