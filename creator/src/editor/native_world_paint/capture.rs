//! Retain unchanged original paint catalogues; expand linked owners/pages in
//! bounded slices. No incomplete merge is exposed to UV/stamp publication.
use super::*;
use shared::iso_paint::{ISO_PAINT_BAKE_VERSION, IsoPaintBakedChunk, IsoPaintOwner};
use shared::shared_index_map::SharedIndexMap;
#[cfg(test)]
#[path = "capture/tests.rs"]
mod tests;

struct PrefabSource {
    id: Uuid,
    present: bool,
    visible: bool,
    version: u8,
    pages: SharedIndexMap<String, IsoPaintBakedChunk>,
}
impl PrefabSource {
    fn capture(id: Uuid, paint: Option<&IsoPaintLayer>) -> Self {
        Self {
            id,
            present: paint.is_some(),
            visible: paint.is_some_and(|paint| paint.visible),
            version: paint.map_or(0, |paint| paint.baked_version),
            pages: paint
                .map(|paint| paint.baked_chunks.clone())
                .unwrap_or_default(),
        }
    }
    fn matches(&self, paint: Option<&IsoPaintLayer>) -> bool {
        self.present == paint.is_some()
            && paint.is_none_or(|paint| {
                self.visible == paint.visible
                    && self.version == paint.baked_version
                    && (self.pages.same_storage(&paint.baked_chunks)
                        || self.pages.is_empty() && paint.baked_chunks.is_empty())
            })
    }
}
struct Source {
    scope: Scope,
    base: Option<IsoPaintLayer>,
    selection: bool,
    region: bool,
    assets_revision: u64,
    instances: Vec<(Uuid, Uuid)>,
    prefabs: Vec<PrefabSource>,
    by_id: FxHashMap<Uuid, usize>,
}
impl Source {
    fn presentation(&self) -> IsoPaintLayer {
        let mut output = self.base.clone().unwrap_or_default();
        output.selection_visible = self.selection;
        if self.region {
            output.surface_instance_owners.clear();
            if !output.visible {
                output.chunks.clear();
                output.baked_chunks.clear();
                output.surface_commit_strokes.clear();
            }
        }
        output
    }
    fn matches(
        &self,
        scope: Scope,
        base: Option<&IsoPaintLayer>,
        selection: bool,
        map: Option<&Map>,
        assets: &rusterix::Assets,
        catalog: &indexmap::IndexMap<Uuid, IsoPaintLayer>,
    ) -> bool {
        self.scope == scope
            && self.selection == selection
            && self.region == map.is_some()
            && match (&self.base, base) {
                (None, None) => true,
                (Some(snapshot), Some(base)) => base.same_presentation_source(snapshot, selection),
                _ => false,
            }
            && map.is_none_or(|map| {
                self.assets_revision == assets.block_props_revision
                    && self.instances.len() == map.block_prop_instances.len()
                    && self.instances.iter().zip(&map.block_prop_instances).all(
                        |((id, asset), instance)| *id == instance.id && *asset == instance.asset_id,
                    )
                    && self
                        .prefabs
                        .iter()
                        .all(|source| source.matches(catalog.get(&source.id)))
            })
    }
    fn capture(
        scope: Scope,
        base: Option<&IsoPaintLayer>,
        selection: bool,
        map: Option<&Map>,
        assets: &rusterix::Assets,
        catalog: &indexmap::IndexMap<Uuid, IsoPaintLayer>,
    ) -> Self {
        let instances: Vec<_> = map
            .into_iter()
            .flat_map(|map| &map.block_prop_instances)
            .map(|instance| (instance.id, instance.asset_id))
            .collect();
        // Same hash-set insertion and traversal as the original merge helper:
        // overlapping Prefab page keys retain its last-insert-wins policy.
        let mut ids = FxHashSet::default();
        for (_, asset) in &instances {
            ids.insert(*asset);
        }
        let prefabs: Vec<_> = ids
            .into_iter()
            .map(|id| PrefabSource::capture(id, catalog.get(&id)))
            .collect();
        let by_id = prefabs
            .iter()
            .enumerate()
            .map(|(index, source)| (source.id, index))
            .collect();
        Self {
            scope,
            base: base.map(|paint| paint.presentation_source(selection)),
            selection,
            region: map.is_some(),
            assets_revision: assets.block_props_revision,
            instances,
            prefabs,
            by_id,
        }
    }
}
enum Phase {
    Region,
    Owners,
    Prefabs,
    Done,
}
struct Merge {
    output: IsoPaintLayer,
    original: SharedIndexMap<String, IsoPaintBakedChunk>,
    phase: Phase,
    page: usize,
    instance: usize,
    part: usize,
    object: usize,
    prefab: usize,
    owners: FxHashSet<Uuid>,
}
impl Merge {
    fn new(source: &Source) -> Self {
        let mut output = source.presentation();
        let original = std::mem::take(&mut output.baked_chunks);
        Self {
            output,
            original,
            phase: Phase::Region,
            page: 0,
            instance: 0,
            part: 0,
            object: 0,
            prefab: 0,
            owners: FxHashSet::default(),
        }
    }
    fn next_instance(&mut self) {
        self.instance += 1;
        self.part = 0;
        self.object = 0;
    }
    fn advance(&mut self, source: &Source, assets: &rusterix::Assets, items: usize) -> bool {
        let started = web_time::Instant::now();
        for _ in 0..items.min(4096) {
            if started.elapsed() >= std::time::Duration::from_millis(2) {
                break;
            }
            match self.phase {
                Phase::Region => {
                    if let Some((key, page)) = self.original.get_index(self.page) {
                        self.output.baked_chunks.insert(key.clone(), page.clone());
                        self.page += 1;
                    } else {
                        self.phase = Phase::Owners;
                        self.page = 0;
                    }
                }
                Phase::Owners => {
                    let Some((instance, asset_id)) = source.instances.get(self.instance) else {
                        self.phase = Phase::Prefabs;
                        continue;
                    };
                    if !source.prefabs[source.by_id[asset_id]].visible {
                        self.next_instance();
                        continue;
                    }
                    let Some(asset) = assets.block_props.get(asset_id) else {
                        self.next_instance();
                        continue;
                    };
                    let Some(part) = asset.parts.get(self.part) else {
                        self.next_instance();
                        continue;
                    };
                    let Some(object) = part.geometry_source.geometry_objects().get(self.object)
                    else {
                        self.part += 1;
                        self.object = 0;
                        continue;
                    };
                    let id = rusterix::block_prop_instance_object_id(*instance, part.id, object.id);
                    if self.owners.insert(id) {
                        self.output
                            .surface_instance_owners
                            .push(IsoPaintOwner::GeometryObject(id));
                    }
                    self.object += 1;
                }
                Phase::Prefabs => {
                    let Some(prefab) = source.prefabs.get(self.prefab) else {
                        self.phase = Phase::Done;
                        continue;
                    };
                    if !prefab.visible || prefab.version != ISO_PAINT_BAKE_VERSION {
                        self.prefab += 1;
                        self.page = 0;
                        continue;
                    }
                    self.output.visible = true;
                    if let Some((key, page)) = prefab.pages.get_index(self.page) {
                        self.output.baked_chunks.insert(key.clone(), page.clone());
                        self.page += 1;
                    } else {
                        self.prefab += 1;
                        self.page = 0;
                    }
                }
                Phase::Done => return true,
            }
        }
        matches!(self.phase, Phase::Done)
    }
}
struct State {
    source: Source,
    merge: Option<Merge>,
    ready: Option<Arc<IsoPaintLayer>>,
}
#[derive(Default)]
pub struct Capture {
    state: Option<State>,
}
impl Capture {
    pub fn clear(&mut self) {
        self.state = None;
    }
    pub fn pending(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| state.merge.is_some())
    }
    pub fn request(
        &mut self,
        scope: Scope,
        project: &Project,
        server: &CreatorContext,
        map: &Map,
        assets: &rusterix::Assets,
    ) -> Result<Option<Arc<IsoPaintLayer>>, String> {
        let selection = server.shared.curr_map_tool_type == MapToolType::IsoPaint;
        let (base, region) = if let ProjectContext::Prefab(id) = server.shared.pc {
            (project.block_prop_paint.get(&id), None)
        } else {
            let region = project
                .get_region_ctx(&server.shared)
                .ok_or("native 3D region removed")?;
            (Some(&region.iso_paint), Some(map))
        };
        Ok(self.request_source(
            scope,
            base,
            selection,
            region,
            assets,
            &project.block_prop_paint,
            4096,
        ))
    }
    fn request_source(
        &mut self,
        scope: Scope,
        base: Option<&IsoPaintLayer>,
        selection: bool,
        map: Option<&Map>,
        assets: &rusterix::Assets,
        catalog: &indexmap::IndexMap<Uuid, IsoPaintLayer>,
        items: usize,
    ) -> Option<Arc<IsoPaintLayer>> {
        if self.state.as_ref().is_none_or(|state| {
            !state
                .source
                .matches(scope, base, selection, map, assets, catalog)
        }) {
            let source = Source::capture(scope, base, selection, map, assets, catalog);
            let (merge, ready) = if source.region && !source.instances.is_empty() {
                (Some(Merge::new(&source)), None)
            } else {
                (None, Some(Arc::new(source.presentation())))
            };
            self.state = Some(State {
                source,
                merge,
                ready,
            });
        }
        self.advance_snapshot(assets, items)
    }
    /// Finish the immutable generation already admitted to the display pipeline.
    /// New authored changes are captured after that generation is presented.
    pub fn advance_pinned(&mut self, assets: &rusterix::Assets) -> Option<Arc<IsoPaintLayer>> {
        self.advance_snapshot(assets, 4096)
    }
    fn advance_snapshot(&mut self, assets: &rusterix::Assets, items: usize) -> Option<Arc<IsoPaintLayer>> {
        let state = self.state.as_mut()?;
        if let Some(merge) = &mut state.merge
            && merge.advance(&state.source, assets, items)
        {
            state.ready = Some(Arc::new(state.merge.take().unwrap().output));
        }
        state.ready.clone()
    }
}
