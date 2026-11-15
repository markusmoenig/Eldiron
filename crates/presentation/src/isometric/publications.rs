//! Non-owning lookup of immutable sources already leased by prepared frames.
//! Eviction limits cache ownership; it must not duplicate a still-live source
//! for every triangle when one asset exceeds that cache's retention budget.
use rustc_hash::FxHashMap;
use std::{
    hash::Hash,
    sync::{Arc, Weak},
};

pub(super) struct Publications<K, V>(FxHashMap<K, Weak<V>>);
impl<K, V> Default for Publications<K, V> {
    fn default() -> Self {
        Self(FxHashMap::default())
    }
}
impl<K: Eq + Hash, V> Publications<K, V> {
    pub(super) fn get(&self, key: &K) -> Option<Arc<V>> {
        self.0.get(key).and_then(Weak::upgrade)
    }
    pub(super) fn publish(&mut self, key: K, source: &Arc<V>) {
        // Metadata tracks current caller-owned publications and cache entries.
        // It never owns a GPU buffer; retire dead keys on each new publication.
        self.0.retain(|_, source| source.strong_count() != 0);
        self.0.insert(key, Arc::downgrade(source));
    }
    pub(super) fn clear(&mut self) {
        self.0.clear();
    }
}
