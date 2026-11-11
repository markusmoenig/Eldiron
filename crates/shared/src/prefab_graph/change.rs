//! One prefab authoring transaction. No UI, runtime, renderer or whole-project
//! snapshot is retained. Same-asset edits are ordered by the document owner.
use super::PrefabGraphs;
use crate::project::Project;
use rusterix::BlockPropAsset;
use theframework::prelude::Uuid;

#[derive(Clone, Debug, PartialEq)]
struct State {
    asset: BlockPropAsset,
    graphs: Option<PrefabGraphs>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrefabGraphChange {
    asset_id: Uuid,
    before: State,
    after: State,
}
impl PrefabGraphChange {
    /// Capture only this asset and its source. The caller supplies an already
    /// compiled candidate; unrelated project/asset edits remain outside the atom.
    pub fn capture(
        project: &Project,
        asset_id: Uuid,
        graphs: PrefabGraphs,
        asset: BlockPropAsset,
    ) -> Option<Self> {
        if asset.id != asset_id {
            return None;
        }
        Some(Self {
            asset_id,
            before: State {
                asset: project.block_props.get(&asset_id)?.clone(),
                graphs: project.prefab_graphs.get(&asset_id).cloned(),
            },
            after: State {
                asset,
                graphs: Some(graphs),
            },
        })
    }
    pub fn asset_id(&self) -> Uuid {
        self.asset_id
    }
    /// Restore the chosen state and rederive attachment positions from current
    /// placements, preserving unrelated authored changes. Removed targets are
    /// not resurrected by stale commands. Reapplying the same state is a no-op.
    pub fn apply(&self, project: &mut Project, forward: bool) -> bool {
        let state = if forward { &self.after } else { &self.before };
        let Some(current) = project.block_props.get(&self.asset_id) else {
            return false;
        };
        if current == &state.asset
            && project.prefab_graphs.get(&self.asset_id) == state.graphs.as_ref()
        {
            return false;
        }
        project
            .block_props
            .insert(self.asset_id, state.asset.clone());
        if let Some(graphs) = &state.graphs {
            project.prefab_graphs.insert(self.asset_id, graphs.clone());
        } else {
            project.prefab_graphs.shift_remove(&self.asset_id);
        }
        sync_prefab_placements(project);
        true
    }
}

/// CPU authoring constraints, also used by non-graph prefab tools. Updating a
/// saved asset must not construct a presentation owner to position attachments.
pub fn sync_prefab_placements(project: &mut Project) {
    let block_props = &project.block_props;
    for region in &mut project.regions {
        rusterix::sync_block_prop_surface_prop_transforms(
            &mut region.map.block_prop_instances,
            &region.map.block_prop_surface_placements,
            block_props,
        );
        rusterix::sync_block_prop_surface_item_positions(
            &region.map.block_prop_instances,
            &region.map.block_prop_surface_placements,
            &mut region.map.items,
            block_props,
        );
        for item in region.items.values_mut() {
            if let Some(runtime_item) = region
                .map
                .items
                .iter()
                .find(|runtime_item| runtime_item.creator_id == item.id)
            {
                item.position = runtime_item.position;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use theframework::thegraph::GraphControlValue;

    fn fixture(existing_graph: bool) -> (Project, PrefabGraphChange) {
        let mut project = Project::default();
        let mut asset = BlockPropAsset::new("Scoped asset");
        asset
            .parts
            .push(rusterix::BlockPropPart::new_authored("Before", Vec::new()));
        let id = asset.id;
        let mut graphs = PrefabGraphs::import(&asset);
        if existing_graph {
            project.prefab_graphs.insert(id, graphs.clone());
        }
        project.block_props.insert(id, asset.clone());
        let row = graphs.branches[0]
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("prefab_part"))
            .unwrap()
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("name"))
            .unwrap();
        row.value = GraphControlValue::Text("After 日本".into());
        let compiled = graphs.compile(&asset).unwrap();
        let change = PrefabGraphChange::capture(&project, id, graphs, compiled).unwrap();
        (project, change)
    }

    #[test]
    fn scoped_prefab_change_preserves_unrelated_edits_and_exact_source_presence() {
        for existing_graph in [false, true] {
            let (mut project, change) = fixture(existing_graph);
            let id = change.asset_id();
            let before_asset = project.block_props[&id].clone();
            let before_source = project.prefab_graphs.get(&id).cloned();
            assert!(change.apply(&mut project, true));
            assert!(!change.apply(&mut project, true));
            let after_asset = project.block_props[&id].clone();
            let after_source = project.prefab_graphs[&id].clone();
            assert_eq!(after_asset.parts[0].name, "After 日本");
            project.name = "Concurrent project edit".into();
            project.regions[0].map.name = "Concurrent map edit".into();
            let unrelated = BlockPropAsset::new("Concurrent asset");
            let other_id = unrelated.id;
            project.block_props.insert(other_id, unrelated.clone());
            for forward in [false, true, false, true] {
                assert!(change.apply(&mut project, forward));
                assert_eq!(project.name, "Concurrent project edit");
                assert_eq!(project.regions[0].map.name, "Concurrent map edit");
                assert_eq!(project.block_props[&other_id], unrelated);
                if forward {
                    assert_eq!(project.block_props[&id], after_asset);
                    assert_eq!(project.prefab_graphs[&id], after_source);
                } else {
                    assert_eq!(project.block_props[&id], before_asset);
                    assert_eq!(project.prefab_graphs.get(&id), before_source.as_ref());
                }
                let bytes = crate::project_io::encode_project(&project).unwrap();
                let restored = crate::project_io::decode_project(&bytes).unwrap();
                assert_eq!(restored.block_props[&id], project.block_props[&id]);
                assert_eq!(
                    restored.prefab_graphs.get(&id),
                    project.prefab_graphs.get(&id)
                );
            }
        }
    }

    #[test]
    fn scoped_prefab_change_rederives_items_and_props_from_current_placements() {
        use rusterix::{
            BlockPropFaceRef, BlockPropOccupancyPolicy, BlockPropOccupant, BlockPropSemanticShape,
            BlockPropSupportSurface, BlockPropSurfacePlacement,
        };
        use theframework::prelude::Vec3;
        let mut project = Project::default();
        let object =
            rusterix::GeometryObject::box_("Top", Vec3::new(0., 0.5, 0.), Vec3::new(2., 1., 2.));
        let face = BlockPropFaceRef {
            object_id: object.id,
            face_id: object.faces[4].id,
        };
        let mut asset = BlockPropAsset::new_authored("Table", vec![object]);
        let id = asset.id;
        let surface = Uuid::new_v4();
        asset.support_surfaces.push(BlockPropSupportSurface {
            id: surface,
            name: "Top".into(),
            part_id: asset.parts[0].id,
            shape: BlockPropSemanticShape::Faces(vec![face]),
            snap_spacing: 0.25,
            allowed_item_tags: Vec::new(),
            capacity: None,
            occupancy_policy: BlockPropOccupancyPolicy::RejectOverlap,
        });
        let graphs = PrefabGraphs::import(&asset);
        project.block_props.insert(id, asset.clone());
        // The imported source is itself the completed authoring change here.
        let change = PrefabGraphChange::capture(&project, id, graphs, asset).unwrap();
        let mut parent = rusterix::BlockPropInstance::new(id);
        parent.world_transform[3][0] = 5.;
        let parent_id = parent.id;
        let child = rusterix::BlockPropInstance::new(id);
        let child_id = child.id;
        let authored_item = crate::item::Item::new();
        let item_id = authored_item.id;
        let mut runtime_item = rusterix::Item::new();
        runtime_item.creator_id = item_id;
        let region = &mut project.regions[0];
        region.items.insert(item_id, authored_item);
        region.map.items.push(runtime_item);
        region.map.block_prop_instances.extend([parent, child]);
        region.map.block_prop_surface_placements = [
            BlockPropOccupant::ItemInstance(item_id),
            BlockPropOccupant::PropInstance(child_id),
        ]
        .into_iter()
        .map(|occupant| BlockPropSurfacePlacement {
            id: Uuid::new_v4(),
            prop_instance_id: parent_id,
            surface_id: surface,
            occupant,
            local_transform: rusterix::identity_block_prop_transform(),
        })
        .collect();
        assert!(change.apply(&mut project, true));
        let first = project.regions[0].items[&item_id].position;
        // A placement/camera-independent world edit occurs after the graph atom.
        project.regions[0].map.block_prop_instances[0].world_transform[3][0] += 7.;
        for forward in [false, true] {
            assert!(change.apply(&mut project, forward));
            let region = &project.regions[0];
            let position = region.items[&item_id].position;
            assert!((position.x - first.x - 7.).abs() < 1e-5);
            assert_eq!(position, region.map.items[0].position);
            let child = region
                .map
                .block_prop_instances
                .iter()
                .find(|instance| instance.id == child_id)
                .unwrap();
            assert!((child.world_transform[3][0] - position.x).abs() < 1e-5);
            assert!((child.world_transform[3][1] - position.y).abs() < 1e-5);
        }
    }

    #[test]
    fn scoped_prefab_change_rejects_wrong_asset_and_does_not_resurrect_removed_targets() {
        let (mut project, change) = fixture(false);
        let wrong = BlockPropAsset::new("Wrong candidate");
        assert!(
            PrefabGraphChange::capture(&project, change.asset_id(), PrefabGraphs::default(), wrong)
                .is_none()
        );
        project.block_props.shift_remove(&change.asset_id());
        assert!(!change.apply(&mut project, true));
        assert!(!change.apply(&mut project, false));
        assert!(!project.block_props.contains_key(&change.asset_id()));
        assert!(!project.prefab_graphs.contains_key(&change.asset_id()));
    }
}
