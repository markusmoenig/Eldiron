//! Project-owned CPU prefab sources, independent of which view is visible.
//! Views register a binding and dispatch completed core graph packets; only this
//! owner compiles/persists them. UI history and presentation consume its results.
use super::*;
use crate::project::Project;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrefabSourceKey {
    pub asset: Uuid,
    pub part: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrefabSourceError {
    MissingSource(PrefabSourceKey),
    SourceChanged(PrefabSourceKey),
    InvalidSource(PrefabSourceKey, String),
    UnknownBinding,
    WrongBinding,
    InvalidOrder,
}
impl std::fmt::Display for PrefabSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSource(_) => f.write_str("The prefab part is no longer available"),
            Self::SourceChanged(_) => f.write_str("The prefab source changed outside this draft"),
            Self::InvalidSource(_, message) => f.write_str(message),
            Self::UnknownBinding | Self::WrongBinding => {
                f.write_str("The prefab edit belongs to another source")
            }
            Self::InvalidOrder => f.write_str("Prefab edits arrived out of order"),
        }
    }
}
impl std::error::Error for PrefabSourceError {}

#[derive(Clone, Debug)]
pub struct PrefabAuthoringSource {
    document: GraphDocument,
    committed: GraphDocument,
    diagnostic: Option<PrefabSourceError>,
    revision: u64,
}
impl PrefabAuthoringSource {
    pub fn document(&self) -> &GraphDocument {
        &self.document
    }
    pub fn committed(&self) -> &GraphDocument {
        &self.committed
    }
    pub fn diagnostic(&self) -> Option<&PrefabSourceError> {
        self.diagnostic.as_ref()
    }
    /// Changes only when a source is explicitly loaded/rebased, not on ordinary
    /// adoption/validation. A host can replace that view's callback epoch then.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
struct AssetSource {
    graphs: PrefabGraphs,
    persisted: bool,
}
struct Binding {
    key: PrefabSourceKey,
    serial: u64,
}
#[derive(Debug)]
pub struct PrefabSourceApply {
    pub key: PrefabSourceKey,
    pub adopted: bool,
    pub change: Option<PrefabGraphChange>,
    pub diagnostic: Option<PrefabSourceError>,
}
#[derive(Default)]
pub struct PrefabGraphWorkspace {
    assets: HashMap<Uuid, AssetSource>,
    sources: HashMap<PrefabSourceKey, PrefabAuthoringSource>,
    bindings: HashMap<u64, Binding>,
    revision: u64,
    sequence: u64,
}
impl PrefabGraphWorkspace {
    fn graphs(
        &mut self,
        project: &Project,
        key: PrefabSourceKey,
    ) -> Result<PrefabGraphs, PrefabSourceError> {
        let asset = project
            .block_props
            .get(&key.asset)
            .filter(|asset| asset.find_part(key.part).is_some())
            .ok_or(PrefabSourceError::MissingSource(key))?;
        let stored = project.prefab_graphs.get(&key.asset);
        let cached = self.assets.entry(key.asset).or_insert_with(|| AssetSource {
            graphs: stored
                .cloned()
                .unwrap_or_else(|| PrefabGraphs::import(asset)),
            persisted: stored.is_some(),
        });
        if let Some(stored) = stored {
            cached.graphs = stored.clone();
            cached.persisted = true;
        } else if cached.persisted {
            // An explicit restore removed the authored graph again. New source
            // identities/epochs must replace the now-obsolete saved document.
            cached.graphs = PrefabGraphs::import(asset);
            cached.persisted = false;
        }
        cached.graphs.reconcile(asset);
        Ok(cached.graphs.clone())
    }
    pub fn source(&self, key: PrefabSourceKey) -> Option<&PrefabAuthoringSource> {
        self.sources.get(&key)
    }
    /// Loading a view never writes Project. Preserve true means the caller still
    /// owns unacknowledged edits; a conflicting source remains recoverable.
    pub fn load(
        &mut self,
        project: &Project,
        key: PrefabSourceKey,
        preserve: bool,
    ) -> Result<&PrefabAuthoringSource, PrefabSourceError> {
        let graphs = self.graphs(project, key)?;
        let canonical = graphs
            .branches
            .iter()
            .find(|branch| branch.part_id == key.part)
            .ok_or(PrefabSourceError::MissingSource(key))?
            .graph
            .clone();
        let changed = self
            .sources
            .get(&key)
            .is_none_or(|source| source.committed != canonical);
        if changed {
            if preserve && self.sources.contains_key(&key) {
                self.sources.get_mut(&key).unwrap().diagnostic =
                    Some(PrefabSourceError::SourceChanged(key));
            } else {
                self.revision = self
                    .revision
                    .checked_add(1)
                    .expect("prefab source revision exhausted");
                self.sources.insert(
                    key,
                    PrefabAuthoringSource {
                        document: canonical.clone(),
                        committed: canonical,
                        diagnostic: None,
                        revision: self.revision,
                    },
                );
                self.bindings.retain(|_, binding| binding.key != key);
            }
        }
        Ok(&self.sources[&key])
    }
    /// Supply a completed domain operation or initial private layout. This does
    /// not persist anything; ordinary input should dispatch core edit packets.
    pub fn set_working_document(
        &mut self,
        key: PrefabSourceKey,
        document: GraphDocument,
    ) -> Result<(), PrefabSourceError> {
        self.sources
            .get_mut(&key)
            .ok_or(PrefabSourceError::MissingSource(key))?
            .document = document;
        Ok(())
    }
    pub fn bind(&mut self, key: PrefabSourceKey, binding: u64) -> Result<(), PrefabSourceError> {
        if !self.sources.contains_key(&key) {
            return Err(PrefabSourceError::MissingSource(key));
        }
        if let Some(existing) = self.bindings.get(&binding) {
            return if existing.key == key {
                Ok(())
            } else {
                Err(PrefabSourceError::WrongBinding)
            };
        }
        self.bindings.insert(binding, Binding { key, serial: 0 });
        Ok(())
    }
    pub fn unbind(&mut self, binding: u64) {
        self.bindings.remove(&binding);
    }
    pub fn binding_source(&self, binding: u64) -> Option<PrefabSourceKey> {
        self.bindings.get(&binding).map(|binding| binding.key)
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = (PrefabSourceKey, &PrefabSourceError)> {
        self.sources
            .iter()
            .filter_map(|(key, source)| source.diagnostic.as_ref().map(|error| (*key, error)))
    }
    /// Merge pending views by GraphCommit.sequence before dispatch. Once adopted,
    /// even an invalid private draft is acknowledged; later correction advances
    /// from that source instead of replaying a rejected serial forever.
    pub fn apply_commit(
        &mut self,
        project: &mut Project,
        commit: &GraphCommit,
    ) -> Result<PrefabSourceApply, PrefabSourceError> {
        let binding = self
            .bindings
            .get(&commit.binding)
            .ok_or(PrefabSourceError::UnknownBinding)?;
        let key = binding.key;
        if commit.serial == 0 {
            return Err(PrefabSourceError::InvalidOrder);
        }
        if commit.serial <= binding.serial {
            return Ok(PrefabSourceApply {
                key,
                adopted: false,
                change: None,
                diagnostic: self
                    .sources
                    .get(&key)
                    .and_then(|source| source.diagnostic.clone()),
            });
        }
        if commit.serial != binding.serial + 1 || commit.sequence <= self.sequence {
            return Err(PrefabSourceError::InvalidOrder);
        }
        let graphs = self.graphs(project, key)?;
        let source = self
            .sources
            .get_mut(&key)
            .ok_or(PrefabSourceError::MissingSource(key))?;
        for edit in &commit.edits {
            edit.apply(&mut source.document, true);
        }
        // Adoption is distinct from successful compilation; no UI owner may
        // discard this raw draft merely because another source became visible.
        self.bindings.get_mut(&commit.binding).unwrap().serial = commit.serial;
        self.sequence = commit.sequence;
        let change = self.compile_source(project, key, graphs);
        Ok(PrefabSourceApply {
            key,
            adopted: true,
            change,
            diagnostic: self.sources[&key].diagnostic.clone(),
        })
    }
    /// Explicit completed toolbar/domain operation, not a snapshot of a working
    /// view mid-gesture. Retained input packets must be consumed before this call.
    pub fn commit_document(
        &mut self,
        project: &mut Project,
        key: PrefabSourceKey,
        document: GraphDocument,
    ) -> Result<PrefabSourceApply, PrefabSourceError> {
        let graphs = self.graphs(project, key)?;
        self.set_working_document(key, document)?;
        let change = self.compile_source(project, key, graphs);
        Ok(PrefabSourceApply {
            key,
            adopted: true,
            change,
            diagnostic: self.sources[&key].diagnostic.clone(),
        })
    }
    fn compile_source(
        &mut self,
        project: &mut Project,
        key: PrefabSourceKey,
        mut graphs: PrefabGraphs,
    ) -> Option<PrefabGraphChange> {
        let source = self.sources.get_mut(&key)?;
        let Some(branch) = graphs
            .branches
            .iter_mut()
            .find(|branch| branch.part_id == key.part)
        else {
            source.diagnostic = Some(PrefabSourceError::MissingSource(key));
            return None;
        };
        if branch.graph != source.committed {
            source.diagnostic = Some(PrefabSourceError::SourceChanged(key));
            return None;
        }
        if source.document == source.committed {
            source.diagnostic = None;
            return None;
        }
        branch.graph = source.document.clone();
        let Some(base) = project.block_props.get(&key.asset) else {
            source.diagnostic = Some(PrefabSourceError::MissingSource(key));
            return None;
        };
        match graphs.compile_for_authoring(base) {
            Ok(asset) => {
                let Some(change) =
                    PrefabGraphChange::capture(project, key.asset, graphs.clone(), asset)
                else {
                    source.diagnostic = Some(PrefabSourceError::MissingSource(key));
                    return None;
                };
                let changed = change.apply(project, true);
                source.committed = source.document.clone();
                source.diagnostic = None;
                self.assets.insert(
                    key.asset,
                    AssetSource {
                        graphs,
                        persisted: true,
                    },
                );
                changed.then_some(change)
            }
            Err(message) => {
                source.diagnostic = Some(PrefabSourceError::InvalidSource(key, message));
                None
            }
        }
    }
    pub fn remove(&mut self, key: PrefabSourceKey) {
        self.sources.remove(&key);
        self.bindings.retain(|_, binding| binding.key != key);
    }
    pub fn clear(&mut self) {
        self.sources.clear();
        self.assets.clear();
        self.bindings.clear();
        self.sequence = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        Project,
        PrefabGraphWorkspace,
        PrefabSourceKey,
        PrefabSourceKey,
    ) {
        let mut project = Project::default();
        let mut asset = BlockPropAsset::new("Workspace asset");
        asset
            .parts
            .push(rusterix::BlockPropPart::new_authored("First", Vec::new()));
        asset
            .parts
            .push(rusterix::BlockPropPart::new_authored("Second", Vec::new()));
        let first = PrefabSourceKey {
            asset: asset.id,
            part: asset.parts[0].id,
        };
        let second = PrefabSourceKey {
            asset: asset.id,
            part: asset.parts[1].id,
        };
        project.block_props.insert(asset.id, asset);
        let mut workspace = PrefabGraphWorkspace::default();
        workspace.load(&project, first, false).unwrap();
        workspace.load(&project, second, false).unwrap();
        workspace.bind(first, 11).unwrap();
        workspace.bind(second, 22).unwrap();
        (project, workspace, first, second)
    }
    fn edit(
        workspace: &PrefabGraphWorkspace,
        key: PrefabSourceKey,
        definition: &str,
        field: &str,
        text: &str,
    ) -> GraphEdit {
        let document = workspace.source(key).unwrap().document();
        let node = document
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some(definition))
            .unwrap();
        let row = node
            .rows
            .iter()
            .find(|row| row.key.as_deref() == Some(field))
            .unwrap();
        GraphEdit::SetValue {
            node: node.id,
            row: row.id,
            before: row.value.clone(),
            after: GraphControlValue::Text(text.into()),
        }
    }
    fn name(project: &Project, key: PrefabSourceKey) -> &str {
        &project.block_props[&key.asset]
            .find_part(key.part)
            .unwrap()
            .name
    }
    #[test]
    fn source_workspace_retains_import_ids_and_interleaved_history_without_touching_unrelated_data()
    {
        fn assert_cpu<T: Send + Sync>() {}
        assert_cpu::<PrefabGraphWorkspace>();
        let (mut project, mut workspace, a, b) = fixture();
        assert!(
            !project.prefab_graphs.contains_key(&a.asset),
            "opening a source is not a project edit"
        );
        let source_b = workspace.source(b).unwrap().document().clone();
        let revision = workspace.source(b).unwrap().revision();
        workspace.load(&project, b, false).unwrap();
        assert_eq!(workspace.source(b).unwrap().revision(), revision);
        assert_eq!(workspace.source(b).unwrap().document(), &source_b);
        let a1 = GraphCommit::new(
            11,
            1,
            vec![edit(&workspace, a, "prefab_part", "name", "A1 日本")],
        );
        let b1 = GraphCommit::new(
            22,
            1,
            vec![edit(&workspace, b, "prefab_part", "name", "B1")],
        );
        let original = project.block_props[&a.asset].clone();
        let mut changes = Vec::new();
        changes.push(
            workspace
                .apply_commit(&mut project, &a1)
                .unwrap()
                .change
                .unwrap(),
        );
        assert_eq!(
            project.prefab_graphs[&a.asset]
                .branches
                .iter()
                .find(|branch| branch.part_id == b.part)
                .unwrap()
                .graph,
            source_b
        );
        workspace.load(&project, b, true).unwrap();
        changes.push(
            workspace
                .apply_commit(&mut project, &b1)
                .unwrap()
                .change
                .unwrap(),
        );
        let a2 = GraphCommit::new(
            11,
            2,
            vec![edit(&workspace, a, "prefab_part", "name", "A2")],
        );
        changes.push(
            workspace
                .apply_commit(&mut project, &a2)
                .unwrap()
                .change
                .unwrap(),
        );
        assert_eq!(name(&project, a), "A2");
        assert_eq!(name(&project, b), "B1");
        let repeat = workspace.apply_commit(&mut project, &a1).unwrap();
        assert!(!repeat.adopted && repeat.change.is_none());
        project.name = "Independent project edit".into();
        for change in changes.iter().rev() {
            assert!(change.apply(&mut project, false));
        }
        assert_eq!(project.block_props[&a.asset], original);
        assert!(!project.prefab_graphs.contains_key(&a.asset));
        assert_eq!(project.name, "Independent project edit");
        for change in &changes {
            assert!(change.apply(&mut project, true));
        }
        let bytes = crate::project_io::encode_project(&project).unwrap();
        let restored = crate::project_io::decode_project(&bytes).unwrap();
        assert_eq!(name(&restored, a), "A2");
        assert_eq!(name(&restored, b), "B1");
    }
    #[test]
    fn source_workspace_retains_invalid_hidden_drafts_while_other_parts_compile_and_allows_correction()
     {
        let (mut project, mut workspace, a, b) = fixture();
        let original = project.block_props[&a.asset].clone();
        let invalid = GraphCommit::new(
            11,
            1,
            vec![edit(
                &workspace,
                a,
                "prefab_transform",
                "pivot_x",
                "invalid",
            )],
        );
        let outcome = workspace.apply_commit(&mut project, &invalid).unwrap();
        assert!(outcome.adopted && outcome.change.is_none());
        assert!(matches!(
            outcome.diagnostic,
            Some(PrefabSourceError::InvalidSource(_, _))
        ));
        assert_eq!(project.block_props[&a.asset], original);
        assert!(!project.prefab_graphs.contains_key(&a.asset));
        let b_edit = GraphCommit::new(
            22,
            1,
            vec![edit(
                &workspace,
                b,
                "prefab_part",
                "name",
                "Other source edited",
            )],
        );
        assert!(
            workspace
                .apply_commit(&mut project, &b_edit)
                .unwrap()
                .change
                .is_some()
        );
        workspace.load(&project, a, true).unwrap();
        assert!(workspace.source(a).unwrap().diagnostic().is_some());
        assert_eq!(workspace.diagnostics().count(), 1);
        let correction = GraphCommit::new(
            11,
            2,
            vec![edit(&workspace, a, "prefab_transform", "pivot_x", "1.25")],
        );
        let corrected = workspace.apply_commit(&mut project, &correction).unwrap();
        assert!(corrected.change.is_some() && corrected.diagnostic.is_none());
        assert_eq!(
            project.block_props[&a.asset]
                .find_part(a.part)
                .unwrap()
                .pivot[0],
            1.25
        );
        assert_eq!(name(&project, b), "Other source edited");
        assert_eq!(workspace.diagnostics().count(), 0);
    }
    #[test]
    fn source_workspace_rejects_gaps_wrong_owners_and_out_of_order_publication_without_mutation() {
        let (mut project, mut workspace, a, b) = fixture();
        assert_eq!(
            workspace.bind(b, 11).unwrap_err(),
            PrefabSourceError::WrongBinding
        );
        let first = GraphCommit::new(
            11,
            1,
            vec![edit(&workspace, a, "prefab_part", "name", "First packet")],
        );
        let second = GraphCommit::new(
            11,
            2,
            vec![edit(&workspace, a, "prefab_part", "name", "Second packet")],
        );
        let source = serde_json::to_value(&project).unwrap();
        assert_eq!(
            workspace.apply_commit(&mut project, &second).unwrap_err(),
            PrefabSourceError::InvalidOrder
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), source);
        assert_eq!(
            workspace.source(a).unwrap().document(),
            workspace.source(a).unwrap().committed()
        );
        assert!(
            workspace
                .apply_commit(&mut project, &first)
                .unwrap()
                .change
                .is_some()
        );
        assert!(
            workspace
                .apply_commit(&mut project, &second)
                .unwrap()
                .change
                .is_some()
        );
        let older_b = GraphCommit::new(
            22,
            1,
            vec![edit(&workspace, b, "prefab_part", "name", "Older B")],
        );
        let later_a = GraphCommit::new(
            11,
            3,
            vec![edit(&workspace, a, "prefab_part", "name", "Later A")],
        );
        assert!(
            workspace
                .apply_commit(&mut project, &later_a)
                .unwrap()
                .change
                .is_some()
        );
        assert_eq!(
            workspace.apply_commit(&mut project, &older_b).unwrap_err(),
            PrefabSourceError::InvalidOrder
        );
        assert_eq!(name(&project, b), "Second");
    }
    #[test]
    fn source_workspace_preserves_conflicts_then_rebases_a_clean_view_and_rejects_old_bindings() {
        let (mut project, mut workspace, a, _) = fixture();
        let draft = GraphCommit::new(
            11,
            1,
            vec![edit(&workspace, a, "prefab_part", "name", "Private draft")],
        );
        let mut graphs = workspace.assets[&a.asset].graphs.clone();
        let node = graphs
            .branches
            .iter_mut()
            .find(|branch| branch.part_id == a.part)
            .unwrap()
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("prefab_part"))
            .unwrap();
        node.rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("name"))
            .unwrap()
            .value = GraphControlValue::Text("External source".into());
        project.prefab_graphs.insert(a.asset, graphs);
        project
            .block_props
            .get_mut(&a.asset)
            .unwrap()
            .parts
            .iter_mut()
            .find(|part| part.id == a.part)
            .unwrap()
            .name = "External source".into();
        let external = serde_json::to_value(&project).unwrap();
        let outcome = workspace.apply_commit(&mut project, &draft).unwrap();
        assert!(outcome.adopted && outcome.change.is_none());
        assert_eq!(
            outcome.diagnostic,
            Some(PrefabSourceError::SourceChanged(a))
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), external);
        workspace.load(&project, a, true).unwrap();
        assert!(workspace.source(a).unwrap().diagnostic().is_some());
        let previous_revision = workspace.source(a).unwrap().revision();
        workspace.load(&project, a, false).unwrap();
        assert!(workspace.source(a).unwrap().revision() > previous_revision);
        assert!(workspace.source(a).unwrap().diagnostic().is_none());
        assert_eq!(
            workspace.apply_commit(&mut project, &draft).unwrap_err(),
            PrefabSourceError::UnknownBinding
        );
    }
    #[test]
    fn source_workspace_clear_and_deleted_targets_cannot_resurrect_prior_project_edits() {
        let (mut project, mut workspace, a, _) = fixture();
        let packet = GraphCommit::new(
            11,
            1,
            vec![edit(&workspace, a, "prefab_part", "name", "Late")],
        );
        let source = project.block_props.shift_remove(&a.asset).unwrap();
        assert_eq!(
            workspace.apply_commit(&mut project, &packet).unwrap_err(),
            PrefabSourceError::MissingSource(a)
        );
        assert!(!project.block_props.contains_key(&a.asset));
        project.block_props.insert(a.asset, source);
        workspace.clear();
        workspace.load(&project, a, false).unwrap();
        workspace.bind(a, 33).unwrap();
        assert_eq!(
            workspace.apply_commit(&mut project, &packet).unwrap_err(),
            PrefabSourceError::UnknownBinding
        );
        assert_eq!(name(&project, a), "First");
    }
}
