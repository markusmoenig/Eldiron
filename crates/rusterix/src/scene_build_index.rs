use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::{Chunk, GeoId, Line3D, LineStrip2D, Poly2D, Poly3D};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};
use vek::Vec2;

/// Reverse index from scene owners to streamed chunk origins.
///
/// Retains immutable CPU geometry independently of any renderer. This index is
/// derived when chunks stream in and lets editor/runtime code find geometry
/// owned by a sector, terrain tile, item, etc. Native views consume a cheap
/// retained snapshot, including after their GPU/device has been reconstructed.
#[derive(Debug, Default, Clone)]
pub struct SceneBuildIndex {
    owner_to_chunks: FxHashMap<GeoId, FxHashSet<(i32, i32)>>,
    chunk_to_owners: FxHashMap<(i32, i32), FxHashSet<GeoId>>,
    chunk_cache: FxHashMap<(i32, i32), Arc<Chunk>>,
    snapshot: OnceLock<Arc<SceneBuildSnapshot>>,
    chunk_to_polys2d: FxHashMap<(i32, i32), FxHashMap<GeoId, Poly2D>>,
    chunk_to_lines2d_px: FxHashMap<(i32, i32), FxHashMap<GeoId, LineStrip2D>>,
    chunk_to_polys3d: FxHashMap<(i32, i32), FxHashMap<GeoId, Vec<Poly3D>>>,
    chunk_to_lines3d: FxHashMap<(i32, i32), FxHashMap<GeoId, Vec<Line3D>>>,
}

/// One immutable publication of CPU world geometry. Chunk Arc identity survives
/// unrelated edits, allowing each view to upload only changed chunks. The outer
/// Arc is stable across repeated `snapshot()` calls until a source mutation.
#[derive(Debug)]
pub struct SceneBuildSnapshot {
    chunks: BTreeMap<(i32, i32), Arc<Chunk>>,
}
impl SceneBuildSnapshot {
    pub fn chunks(&self) -> &BTreeMap<(i32, i32), Arc<Chunk>> {
        &self.chunks
    }
}

#[derive(Debug, Default, Clone)]
pub struct SceneOwnerGeometry {
    pub chunks: FxHashSet<(i32, i32)>,
    pub polys2d: Vec<Poly2D>,
    pub lines2d_px: Vec<LineStrip2D>,
    pub polys3d: Vec<Poly3D>,
    pub lines3d: Vec<Line3D>,
}

impl SceneBuildIndex {
    pub fn snapshot(&self) -> Arc<SceneBuildSnapshot> {
        self.snapshot
            .get_or_init(|| {
                Arc::new(SceneBuildSnapshot {
                    chunks: self
                        .chunk_cache
                        .iter()
                        .map(|(origin, chunk)| (*origin, chunk.clone()))
                        .collect(),
                })
            })
            .clone()
    }
    pub fn clear(&mut self) {
        if !self.chunk_cache.is_empty() {
            self.snapshot.take();
        }
        self.owner_to_chunks.clear();
        self.chunk_to_owners.clear();
        self.chunk_cache.clear();
        self.chunk_to_polys2d.clear();
        self.chunk_to_lines2d_px.clear();
        self.chunk_to_polys3d.clear();
        self.chunk_to_lines3d.clear();
    }

    pub fn index_chunk(&mut self, chunk: &Chunk) {
        let origin = (chunk.origin.x, chunk.origin.y);
        self.remove_chunk_origin(origin);

        let owners: FxHashSet<GeoId> = chunk.owner_geo_ids().collect();
        for owner in &owners {
            self.owner_to_chunks
                .entry(*owner)
                .or_default()
                .insert(origin);
        }
        self.chunk_to_owners.insert(origin, owners);
        self.chunk_cache.insert(origin, Arc::new(chunk.clone()));
        self.snapshot.take();
        self.chunk_to_polys2d
            .insert(origin, chunk.polys_map.clone());
        self.chunk_to_lines2d_px
            .insert(origin, chunk.lines2d_px.clone());
        self.chunk_to_polys3d
            .insert(origin, chunk.polys3d_map.clone());
        self.chunk_to_lines3d.insert(origin, chunk.lines3d.clone());
    }

    pub fn remove_chunk_origin(&mut self, origin: (i32, i32)) {
        self.chunk_to_polys2d.remove(&origin);
        self.chunk_to_lines2d_px.remove(&origin);
        self.chunk_to_polys3d.remove(&origin);
        self.chunk_to_lines3d.remove(&origin);
        if self.chunk_cache.remove(&origin).is_some() {
            self.snapshot.take();
        }

        let Some(owners) = self.chunk_to_owners.remove(&origin) else {
            return;
        };
        for owner in owners {
            if let Some(chunks) = self.owner_to_chunks.get_mut(&owner) {
                chunks.remove(&origin);
                if chunks.is_empty() {
                    self.owner_to_chunks.remove(&owner);
                }
            }
        }
    }

    pub fn chunks_for_owner(&self, owner: GeoId) -> FxHashSet<(i32, i32)> {
        self.owner_to_chunks
            .get(&owner)
            .cloned()
            .unwrap_or_default()
    }

    pub fn chunks_for_owners(
        &self,
        owners: impl IntoIterator<Item = GeoId>,
    ) -> FxHashSet<(i32, i32)> {
        let mut chunks = FxHashSet::default();
        for owner in owners {
            if let Some(found) = self.owner_to_chunks.get(&owner) {
                chunks.extend(found.iter().copied());
            }
        }
        chunks
    }

    pub fn owners_for_chunk(&self, origin: Vec2<i32>) -> FxHashSet<GeoId> {
        self.chunk_to_owners
            .get(&(origin.x, origin.y))
            .cloned()
            .unwrap_or_default()
    }

    pub fn chunk_without_owners(
        &self,
        origin: (i32, i32),
        excluded: &FxHashSet<GeoId>,
    ) -> Option<Chunk> {
        let mut chunk = self.chunk_cache.get(&origin)?.as_ref().clone();
        for owner in excluded {
            chunk.polys_map.remove(owner);
            chunk.lines2d_px.remove(owner);
            chunk.polys3d_map.remove(owner);
            chunk.lines3d.remove(owner);
        }
        Some(chunk)
    }

    pub fn polys2d_for_owner(&self, owner: GeoId) -> Vec<Poly2D> {
        self.owner_to_chunks
            .get(&owner)
            .into_iter()
            .flat_map(|chunks| chunks.iter())
            .filter_map(|origin| self.chunk_to_polys2d.get(origin))
            .filter_map(|polys| polys.get(&owner))
            .cloned()
            .collect()
    }

    pub fn lines2d_px_for_owner(&self, owner: GeoId) -> Vec<LineStrip2D> {
        self.owner_to_chunks
            .get(&owner)
            .into_iter()
            .flat_map(|chunks| chunks.iter())
            .filter_map(|origin| self.chunk_to_lines2d_px.get(origin))
            .filter_map(|lines| lines.get(&owner))
            .cloned()
            .collect()
    }

    pub fn polys3d_for_owner(&self, owner: GeoId) -> Vec<Poly3D> {
        self.owner_to_chunks
            .get(&owner)
            .into_iter()
            .flat_map(|chunks| chunks.iter())
            .filter_map(|origin| self.chunk_to_polys3d.get(origin))
            .filter_map(|polys| polys.get(&owner))
            .flat_map(|polys| polys.iter())
            .cloned()
            .collect()
    }

    pub fn polys3d_for_owners(&self, owners: impl IntoIterator<Item = GeoId>) -> Vec<Poly3D> {
        owners
            .into_iter()
            .flat_map(|owner| self.polys3d_for_owner(owner))
            .collect()
    }

    pub fn lines3d_for_owner(&self, owner: GeoId) -> Vec<Line3D> {
        self.owner_to_chunks
            .get(&owner)
            .into_iter()
            .flat_map(|chunks| chunks.iter())
            .filter_map(|origin| self.chunk_to_lines3d.get(origin))
            .filter_map(|lines| lines.get(&owner))
            .flat_map(|lines| lines.iter())
            .cloned()
            .collect()
    }

    pub fn owner_count(&self) -> usize {
        self.owner_to_chunks.len()
    }

    pub fn owner_geometry(&self, owner: GeoId) -> SceneOwnerGeometry {
        SceneOwnerGeometry {
            chunks: self.chunks_for_owner(owner),
            polys2d: self.polys2d_for_owner(owner),
            lines2d_px: self.lines2d_px_for_owner(owner),
            polys3d: self.polys3d_for_owner(owner),
            lines3d: self.lines3d_for_owner(owner),
        }
    }

    pub fn build_owner_replacement_chunks(
        &self,
        owners: &FxHashSet<GeoId>,
        generated_chunks: impl IntoIterator<Item = Chunk>,
    ) -> Vec<Chunk> {
        let mut replacement_chunks: FxHashMap<(i32, i32), Chunk> = FxHashMap::default();

        for origin in self.chunks_for_owners(owners.iter().copied()) {
            if let Some(chunk) = self.chunk_without_owners(origin, owners) {
                replacement_chunks.insert(origin, chunk);
            }
        }

        for generated in generated_chunks {
            let origin = (generated.origin.x, generated.origin.y);
            let target = replacement_chunks.entry(origin).or_insert_with(|| {
                let mut chunk = Chunk::new(generated.origin, generated.size);
                chunk.bbox = generated.bbox.clone();
                chunk.priority = generated.priority;
                chunk
            });

            for owner in owners {
                if let Some(poly) = generated.polys_map.get(owner) {
                    target.polys_map.insert(*owner, poly.clone());
                }
                if let Some(line) = generated.lines2d_px.get(owner) {
                    target.lines2d_px.insert(*owner, line.clone());
                }
                if let Some(polys) = generated.polys3d_map.get(owner) {
                    target.polys3d_map.insert(*owner, polys.clone());
                }
                if let Some(lines) = generated.lines3d.get(owner) {
                    target.lines3d.insert(*owner, lines.clone());
                }
            }
        }

        replacement_chunks.into_values().collect()
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    #[test]
    fn retained_chunk_publications_survive_edits_removals_and_index_forks() {
        let mut index = SceneBuildIndex::default();
        let empty = index.snapshot();
        assert!(empty.chunks().is_empty());
        assert!(Arc::ptr_eq(&empty, &index.snapshot()));
        let mut first = Chunk::new(Vec2::zero(), 8);
        first.add(Poly2D::quad(
            GeoId::Sector(1),
            uuid::Uuid::new_v4(),
            [0., 0.],
            1.,
            0,
            true,
        ));
        index.index_chunk(&first);
        let a = index.snapshot();
        let a_chunk = a.chunks().get(&(0, 0)).unwrap();
        assert!(!Arc::ptr_eq(&a, &empty));
        assert!(empty.chunks().is_empty());
        let mut other = Chunk::new(Vec2::new(8, 0), 8);
        other.add(Poly2D::quad(
            GeoId::Sector(2),
            uuid::Uuid::new_v4(),
            [8., 0.],
            1.,
            0,
            true,
        ));
        index.index_chunk(&other);
        let b = index.snapshot();
        assert_eq!(b.chunks().len(), 2);
        assert_eq!(a.chunks().len(), 1);
        assert!(
            Arc::ptr_eq(a_chunk, b.chunks().get(&(0, 0)).unwrap()),
            "unrelated edits retain chunk identity"
        );
        let mut fork = index.clone();
        assert!(Arc::ptr_eq(&b, &fork.snapshot()));
        fork.remove_chunk_origin((0, 0));
        assert_eq!(fork.snapshot().chunks().len(), 1);
        assert_eq!(index.snapshot().chunks().len(), 2);
        assert_eq!(fork.chunks_for_owner(GeoId::Sector(1)).len(), 0);
        assert_eq!(index.chunks_for_owner(GeoId::Sector(1)).len(), 1);
        // Owner replacement clones an editable chunk; it cannot mutate snapshots.
        let mut excluded = FxHashSet::default();
        excluded.insert(GeoId::Sector(1));
        let edited = index.chunk_without_owners((0, 0), &excluded).unwrap();
        assert!(edited.polys_map.is_empty());
        assert_eq!(a_chunk.polys_map.len(), 1);
        index.index_chunk(&edited);
        let c = index.snapshot();
        assert!(!Arc::ptr_eq(a_chunk, c.chunks().get(&(0, 0)).unwrap()));
        assert_eq!(a_chunk.polys_map.len(), 1);
        let unchanged = index.snapshot();
        index.remove_chunk_origin((99, 99));
        assert!(Arc::ptr_eq(&unchanged, &index.snapshot()));
        index.clear();
        assert!(index.snapshot().chunks().is_empty());
        assert_eq!(b.chunks().len(), 2);
        let empty = index.snapshot();
        index.clear();
        assert!(Arc::ptr_eq(&empty, &index.snapshot()));
    }
}
