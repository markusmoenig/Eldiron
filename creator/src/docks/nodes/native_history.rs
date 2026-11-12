//! Application-owned history lives with an editing session, not a widget epoch
//! or an authored root UUID. A draft may become a branch and return through undo.
use super::*;
use shared::graph_authoring::NodeSourceTransaction;

#[derive(Default)]
pub(super) struct SourceHistory {
    undo: Vec<NodeSourceTransaction>,
    redo: Vec<NodeSourceTransaction>,
}
impl SourceHistory {
    pub(super) fn record(&mut self, transaction: NodeSourceTransaction) {
        if !transaction.is_noop() {
            self.undo.push(transaction);
            self.redo.clear();
        }
    }
}
impl NativeNodesWorkspace {
    pub fn can_undo(&self, key: &NodeSourceKey) -> bool {
        self.views
            .get(key)
            .is_some_and(|view| !view.history.undo.is_empty())
    }
    pub fn can_redo(&self, key: &NodeSourceKey) -> bool {
        self.views
            .get(key)
            .is_some_and(|view| !view.history.redo.is_empty())
    }
    /// End only the command's source input/gesture. Independent invalid drafts
    /// are reported by Save without preventing this valid source's command.
    fn flush_command(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        fonts: &mut TextSystem,
    ) -> Result<(), String> {
        if !self.views.contains_key(key) {
            return Err(NodeSourceError::MissingBranch.to_string());
        }
        self.suspend(key, fonts)?;
        let _ = self.collect_through(project, u64::MAX);
        let source = self
            .cpu
            .source(key)
            .ok_or_else(|| NodeSourceError::MissingOwner.to_string())?;
        if let Some(error) = source.error() {
            return Err(error.to_string());
        }
        Ok(())
    }
    fn check_destination(&self, from: &NodeSourceKey, to: &NodeSourceKey) -> Result<(), String> {
        // An otherwise empty destination can still belong to a live editor with
        // private text, pending callbacks or redo. Never steal that session.
        if from != to && self.views.contains_key(to) {
            return Err(NodeSourceError::SourceChanged.to_string());
        }
        Ok(())
    }
    fn rebind_session(
        &mut self,
        from: &NodeSourceKey,
        to: &NodeSourceKey,
        project: &Project,
        fonts: &mut TextSystem,
    ) -> Result<(), String> {
        let source = self
            .cpu
            .source(to)
            .ok_or_else(|| NodeSourceError::MissingBranch.to_string())?;
        let document = source.document().clone();
        let revision = source.revision();
        let mut view = self
            .views
            .remove(from)
            .ok_or_else(|| NodeSourceError::MissingBranch.to_string())?;
        view.panels.lock().unwrap().clear(fonts);
        let definitions = source_definitions(project, to);
        let mut retired = view.model.lock().unwrap();
        let viewport = retired.editor().viewport;
        let selected = retired.editor().selected;
        retired.replace_document("retired-source-controller", GraphDocument::default());
        drop(retired);
        // A stale widget may still hold the old Arcs until the next mount. It
        // must not be able to generate new input against the replacement source.
        let mut model = GraphViewState::new(model_owner(to), document);
        let branches = graph_branches(model.document(), &definitions);
        view.shown = if to.kind() == NodeGraphKind::Configuration {
            view.shown
                .filter(|root| branches.branch(*root).is_some())
                .or_else(|| branches.branches.first().map(|branch| branch.root))
        } else {
            to.branch
        };
        let visible = if to.kind() == NodeGraphKind::Configuration {
            view.shown
                .and_then(|root| branches.branch(root).map(|branch| branch.nodes.clone()))
        } else {
            None
        };
        model.edit(|doc, editor| {
            editor.viewport = viewport;
            editor.selected = selected.filter(|id| doc.nodes.iter().any(|node| node.id == *id));
            editor.set_visible(visible);
        });
        self.cpu
            .bind(to, model.binding())
            .map_err(|error| error.to_string())?;
        view.model = Arc::new(Mutex::new(model));
        view.panels = Arc::new(Mutex::new(NativeNodePanels::default()));
        view.revision = revision;
        view.id = TheId::named(VIEW);
        self.views.insert(to.clone(), view);
        if self.active.as_ref() == Some(from) {
            self.active = Some(to.clone());
            self.selected.insert(to.owner.clone(), to.branch);
        }
        self.refresh_errors();
        Ok(())
    }
    /// Run a completed domain operation against acknowledged source, then move
    /// its existing session/history if the source address changed. `edit` returns
    /// the new source key (usually unchanged), not an implicit current selection.
    pub fn edit_source<F>(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        fonts: &mut TextSystem,
        edit: F,
    ) -> Result<bool, String>
    where
        F: FnOnce(&mut GraphDocument, &GraphDefinitions) -> Result<NodeSourceKey, String>,
    {
        self.flush_command(project, key, fonts)?;
        let definitions = source_definitions(project, key);
        let before = self.cpu.source(key).unwrap().document().clone();
        let mut document = before.clone();
        let to = edit(&mut document, &definitions)?;
        if document == before && &to == key {
            return Ok(false);
        }
        self.check_destination(key, &to)?;
        let (transaction, outcome) = self
            .cpu
            .transition_document(
                project,
                key,
                &to,
                document,
                &definitions,
                |key, doc, base, project| {
                    project_source_graph(doc, base, project, key, &definitions)
                },
            )
            .map_err(|error| error.to_string())?;
        self.views.get_mut(key).unwrap().history.record(transaction);
        self.changes.extend(outcome.changes);
        self.rebind_session(key, &to, project, fonts)?;
        Ok(true)
    }
    pub fn undo_source(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        fonts: &mut TextSystem,
    ) -> Result<bool, String> {
        self.history_step(project, key, fonts, false)
    }
    pub fn redo_source(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        fonts: &mut TextSystem,
    ) -> Result<bool, String> {
        self.history_step(project, key, fonts, true)
    }
    fn history_step(
        &mut self,
        project: &mut Project,
        key: &NodeSourceKey,
        fonts: &mut TextSystem,
        forward: bool,
    ) -> Result<bool, String> {
        self.flush_command(project, key, fonts)?;
        let history = &self.views[key].history;
        let Some(mut transaction) = (if forward {
            history.redo.last()
        } else {
            history.undo.last()
        })
        .cloned() else {
            return Ok(false);
        };
        let from = if forward {
            &transaction.focus_before
        } else {
            &transaction.focus_after
        };
        let to = if forward {
            transaction.focus_after.clone()
        } else {
            transaction.focus_before.clone()
        };
        if from != key {
            return Err(NodeSourceError::WrongBinding.to_string());
        }
        self.check_destination(key, &to)?;
        let definitions = source_definitions(project, key);
        let outcome = self
            .cpu
            .replay_transaction(
                project,
                &mut transaction,
                forward,
                &definitions,
                |key, doc, base, project| {
                    project_source_graph(doc, base, project, key, &definitions)
                },
            )
            .map_err(|error| error.to_string())?;
        let history = &mut self.views.get_mut(key).unwrap().history;
        if forward {
            history.redo.pop();
            history.undo.push(transaction);
        } else {
            history.undo.pop();
            history.redo.push(transaction);
        }
        self.changes.extend(outcome.changes);
        self.rebind_session(key, &to, project, fonts)?;
        Ok(true)
    }
}
