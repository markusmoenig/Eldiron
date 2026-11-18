//! Immutable verified region package caching.
use crate::*;
use std::collections::VecDeque;
pub use shared::public_geometry::RegionGeometry;

/// Four whole-region packages and 16 MiB of encoded payloads. Prefetch entries
/// never install live maps or pin memory outside this cache.
pub struct ContentCache {
    entries: VecDeque<(ContentHint, PhysicsContent, usize)>,
    bytes: usize,
}
impl Default for ContentCache {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            bytes: 0,
        }
    }
}
impl ContentCache {
    pub fn get(&mut self, hint: &ContentHint) -> Option<PhysicsContent> {
        let index = self.entries.iter().position(|(h, _, _)| {
            h.reference == hint.reference
                && h.physics_revision == hint.physics_revision
                && h.key == hint.key
        })?;
        let entry = self.entries.remove(index)?;
        let content = entry.1.clone();
        self.entries.push_back(entry);
        Some(content)
    }
    /// Call after fetch/verify_content has checked the original transport bytes;
    /// reencoding hash maps is not a canonical-byte integrity check.
    pub fn insert(&mut self, hint: ContentHint, content: PhysicsContent) -> Result<(), String> {
        if content.reference != hint.reference || content.physics_revision != hint.physics_revision
        {
            return Err("Cached content reference mismatch".into());
        }
        if self.entries.iter().any(|(h, _, _)| {
            h.reference.map == hint.reference.map
                && (h.reference.runtime_revision, h.physics_revision)
                    > (hint.reference.runtime_revision, hint.physics_revision)
        }) {
            return Ok(());
        }
        let bytes = codec::encode_content(&content).map_err(|e| e.to_string())?;
        if let Some(index) = self
            .entries
            .iter()
            .position(|(h, _, _)| h.reference.map == hint.reference.map)
        {
            self.bytes -= self.entries.remove(index).unwrap().2;
        }
        while self.entries.len() >= 4 || self.bytes + bytes.len() > 16 * 1024 * 1024 {
            let Some((_, _, size)) = self.entries.pop_front() else {
                break;
            };
            self.bytes -= size;
        }
        self.bytes += bytes.len();
        self.entries.push_back((hint, content, bytes.len()));
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn encoded_bytes(&self) -> usize {
        self.bytes
    }
}

/// Optional downloads run separately from the ordered gameplay reader. A slow
/// preload cannot stall movement acknowledgments, commands or heartbeats.
pub struct ContentLoader {
    cache: ContentCache,
    pending: Vec<(ContentHint, crate::content_task::ContentTask)>,
}
impl Default for ContentLoader {
    fn default() -> Self {
        Self {
            cache: Default::default(),
            pending: vec![],
        }
    }
}
impl ContentLoader {
    pub fn poll_ready(&mut self) {
        self.poll();
    }
    fn poll(&mut self) {
        let mut index = 0;
        while index < self.pending.len() {
            if let Some(result) = self.pending[index].1.poll() {
                let (hint, _) = self.pending.swap_remove(index);
                if let Ok(package) = result {
                    let _ = self.cache.insert(hint, package);
                }
            } else {
                index += 1;
            }
        }
    }
    pub fn get(&mut self, hint: &ContentHint) -> Option<PhysicsContent> {
        self.poll();
        self.cache.get(hint)
    }
    pub fn insert(&mut self, hint: ContentHint, package: PhysicsContent) -> Result<(), String> {
        self.poll();
        self.cache.insert(hint, package)
    }
    pub fn prefetch(&mut self, endpoint: &str, hints: &[ContentHint]) {
        self.poll();
        for hint in hints.iter().take(4) {
            if self.pending.len() >= 4
                || self.cache.get(hint).is_some()
                || self.pending.iter().any(|(h, _)| {
                    h.reference == hint.reference
                        && h.physics_revision == hint.physics_revision
                        && h.key == hint.key
                })
            {
                continue;
            }
            if let Ok(task) = crate::content_task::ContentTask::start(endpoint, hint) {
                self.pending.push((hint.clone(), task));
            }
        }
    }
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    pub fn encoded_bytes(&self) -> usize {
        self.cache.encoded_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusterix::{Map,GeometryObject,Value};
    fn package() -> (ContentHint, PhysicsContent) {
        let mut map = Map::default();
        map.name = "Public region".into();
        let id = map.add_vertex_at(0., 0.);
        map.vertices
            .iter_mut()
            .find(|v| v.id == id)
            .unwrap()
            .properties
            .set("secret", Value::Str("VERTEX_SECRET".into()));
        map.properties
            .set("script", Value::Str("SCRIPT_SECRET".into()));
        map.geometry_objects.push(GeometryObject::box_from_bounds(
            "OBJECT_SECRET",
            [0., 0., 0.].into(),
            [1., 1., 1.].into(),
        ));
        map.geometry_objects[0]
            .properties
            .set("quest", Value::Str("QUEST_SECRET".into()));
        map.geometry_objects[0]
            .properties
            .set("visible", Value::Bool(true));
        let mut entity = rusterix::Entity::default();
        entity.id = 10;
        entity.set_attribute("name", Value::Str("ACTOR_SECRET".into()));
        map.entities.push(entity);
        let mut nested = Map::default();
        nested
            .properties
            .set("private", Value::Str("PROFILE_SECRET".into()));
        map.profiles.insert(nested.id, nested);
        let reference = eldiron_runtime::replication::RegionContentRef {
            map: map.id,
            map_revision: map.changed,
            runtime_revision: 1,
        };
        let content = PhysicsContent {
            reference,
            physics_revision: 0,
            name: map.name.clone(),
            mini: Default::default(),
            collision: Default::default(),
            geometry: RegionGeometry::capture(&map).unwrap(),
        };
        use sha2::{Digest, Sha256};
        let bytes = codec::encode_content(&content).unwrap();
        (
            ContentHint {
                reference,
                physics_revision: 0,
                key: format!("{:x}", Sha256::digest(&bytes)),
            },
            content,
        )
    }
    #[test]
    fn region_geometry_is_explicit_private_free_verified_and_applied_atomically_with_collision() {
        let (hint, content) = package();
        let bytes = codec::encode_content(&content).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        for secret in [
            "VERTEX_SECRET",
            "SCRIPT_SECRET",
            "OBJECT_SECRET",
            "QUEST_SECRET",
            "ACTOR_SECRET",
            "PROFILE_SECRET",
        ] {
            assert!(!text.contains(secret), "leaked {secret}");
        }
        let verified = crate::remote::verify_content(&hint, &bytes).unwrap();
        let mut target = Map::default();
        verified.geometry.apply(&mut target);
        assert_eq!(target.id, hint.reference.map);
        assert_eq!(target.geometry_objects.len(), 1);
        assert!(target.entities.is_empty());
        target.geometry_objects[0].tags = vec!["release_render_hint".into()];
        verified.geometry.apply(&mut target);
        assert_eq!(target.geometry_objects[0].tags, vec!["release_render_hint"]);
        let mut bad = hint.clone();
        bad.key = "0".repeat(64);
        assert!(crate::remote::verify_content(&bad, &bytes).is_err());
        let mut bad = content.geometry.clone();
        bad.objects[0].faces[0].indices.push(10000);
        assert!(bad.validate().is_err());
        bad = content.geometry;
        bad.vertices[0].x = f32::NAN;
        assert!(bad.validate().is_err());
    }
    #[test]
    fn cache_is_bounded_reuses_preloads_and_replaces_stale_revisions() {
        let mut cache = ContentCache::default();
        let (first, content) = package();
        cache.insert(first.clone(), content).unwrap();
        assert!(cache.get(&first).is_some());
        for _ in 0..5 {
            let (hint, content) = package();
            cache.insert(hint, content).unwrap();
        }
        assert_eq!(cache.len(), 4);
        assert!(cache.encoded_bytes() <= 16 * 1024 * 1024);
        assert!(cache.get(&first).is_none());
        let (old, mut content) = package();
        cache.insert(old.clone(), content.clone()).unwrap();
        content.reference.runtime_revision += 1;
        let new = ContentHint {
            reference: content.reference,
            physics_revision: content.physics_revision,
            key: "a".repeat(64),
        };
        cache.insert(new.clone(), content).unwrap();
        assert!(cache.get(&old).is_none());
        assert!(cache.get(&new).is_some());
    }
}
