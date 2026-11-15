//! One immutable, revisioned CPU publication per overview owner.
use super::*;
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct Snapshot {
    pub(super) document: Uuid,
    pub(super) revision: u32,
    pub(super) map: Arc<Map>,
}
#[derive(Default)]
pub(super) struct SourceCache {
    revision: u32,
    current: Option<Snapshot>,
}
impl SourceCache {
    pub(super) fn current(&self, document: Uuid, map: &Map) -> Option<Snapshot> {
        self.current
            .as_ref()
            .filter(|source| source.document == document && source.map.same_overview_source(map))
            .cloned()
    }
    pub(super) fn publish(&mut self, document: Uuid, original: &Map) -> Snapshot {
        if let Some(current) = self.current(document, original) {
            return current;
        }
        self.revision = self
            .revision
            .checked_add(1)
            .expect("overview source revision exhausted");
        let mut map = original.clone();
        map.offset = Vec2::zero();
        map.grid_size = 1.;
        map.changed = self.revision;
        map.entities.clear();
        map.items.clear();
        map.clear_selection();
        map.clear_temp();
        map.curr_mouse_pos = None;
        let snapshot = Snapshot {
            document,
            revision: self.revision,
            map: Arc::new(map),
        };
        self.current = Some(snapshot.clone());
        snapshot
    }
    pub(super) fn clear(&mut self) {
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overview_publications_share_navigation_changes_but_reject_reused_revisions_and_documents() {
        let document = Uuid::new_v4();
        let mut map = Map::default();
        map.add_vertex_at(0., 0.);
        map.add_vertex_at(1., 1.);
        map.create_linedef_manual(0, 1);
        let mut cache = SourceCache::default();
        let first = cache.publish(document, &map);
        let frozen = serde_json::to_value(first.map.as_ref()).unwrap();
        map.offset = Vec2::new(12., -7.);
        map.grid_size = 64.;
        map.changed = 42;
        map.selected_vertices.push(0);
        map.curr_mouse_pos = Some(Vec2::new(3., 4.));
        let moved = cache.publish(document, &map);
        assert_eq!(first.revision, moved.revision);
        assert!(Arc::ptr_eq(&first.map, &moved.map));
        // Public data/undo may change content without advancing Map.changed.
        map.linedefs[0]
            .properties
            .set("overview edit", rusterix::Value::Int(7));
        assert!(cache.current(document, &map).is_none());
        let edited = cache.publish(document, &map);
        assert!(edited.revision > first.revision);
        assert_eq!(frozen, serde_json::to_value(first.map.as_ref()).unwrap());
        assert!(!Arc::ptr_eq(&first.map, &edited.map));
        assert!(cache.current(Uuid::new_v4(), &map).is_none());
        let reopened = cache.publish(Uuid::new_v4(), &map);
        assert!(reopened.revision > edited.revision);
        cache.clear();
        assert!(cache.current(document, &map).is_none());
        assert!(cache.publish(document, &map).revision > reopened.revision);
    }
}
