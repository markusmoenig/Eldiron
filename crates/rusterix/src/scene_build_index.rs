use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::{Chunk, GeoId, LineStrip2D, Poly2D, Poly3D};
use vek::Vec2;

/// Reverse index from scene owners to streamed chunk origins.
///
/// Scene geometry is still stored in SceneVM chunks. This index is derived when
/// chunks are streamed in and lets editor/runtime code ask which chunks contain
/// geometry owned by a sector, terrain tile, item, etc.
#[derive(Debug, Default, Clone)]
pub struct SceneBuildIndex {
    owner_to_chunks: FxHashMap<GeoId, FxHashSet<(i32, i32)>>,
    chunk_to_owners: FxHashMap<(i32, i32), FxHashSet<GeoId>>,
    chunk_cache: FxHashMap<(i32, i32), Chunk>,
    chunk_to_polys2d: FxHashMap<(i32, i32), FxHashMap<GeoId, Poly2D>>,
    chunk_to_lines2d_px: FxHashMap<(i32, i32), FxHashMap<GeoId, LineStrip2D>>,
    chunk_to_polys3d: FxHashMap<(i32, i32), FxHashMap<GeoId, Vec<Poly3D>>>,
}

#[derive(Debug, Default, Clone)]
pub struct SceneOwnerGeometry {
    pub chunks: FxHashSet<(i32, i32)>,
    pub polys2d: Vec<Poly2D>,
    pub lines2d_px: Vec<LineStrip2D>,
    pub polys3d: Vec<Poly3D>,
}

impl SceneBuildIndex {
    pub fn clear(&mut self) {
        self.owner_to_chunks.clear();
        self.chunk_to_owners.clear();
        self.chunk_cache.clear();
        self.chunk_to_polys2d.clear();
        self.chunk_to_lines2d_px.clear();
        self.chunk_to_polys3d.clear();
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
        self.chunk_cache.insert(origin, chunk.clone());
        self.chunk_to_polys2d
            .insert(origin, chunk.polys_map.clone());
        self.chunk_to_lines2d_px
            .insert(origin, chunk.lines2d_px.clone());
        self.chunk_to_polys3d
            .insert(origin, chunk.polys3d_map.clone());
    }

    pub fn remove_chunk_origin(&mut self, origin: (i32, i32)) {
        self.chunk_to_polys2d.remove(&origin);
        self.chunk_to_lines2d_px.remove(&origin);
        self.chunk_to_polys3d.remove(&origin);
        self.chunk_cache.remove(&origin);

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
        let mut chunk = self.chunk_cache.get(&origin)?.clone();
        for owner in excluded {
            chunk.polys_map.remove(owner);
            chunk.lines2d_px.remove(owner);
            chunk.polys3d_map.remove(owner);
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

    pub fn owner_count(&self) -> usize {
        self.owner_to_chunks.len()
    }

    pub fn owner_geometry(&self, owner: GeoId) -> SceneOwnerGeometry {
        SceneOwnerGeometry {
            chunks: self.chunks_for_owner(owner),
            polys2d: self.polys2d_for_owner(owner),
            lines2d_px: self.lines2d_px_for_owner(owner),
            polys3d: self.polys3d_for_owner(owner),
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
            }
        }

        replacement_chunks.into_values().collect()
    }
}
