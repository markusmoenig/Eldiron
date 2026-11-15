//! Immutable ordered catalogue snapshots with copy-on-write authoring.
//! Serialization delegates to IndexMap, preserving the existing project shape.
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug)]
pub struct SharedIndexMap<K, V>(Arc<IndexMap<K, V>>);
impl<K, V> Clone for SharedIndexMap<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<K, V> Default for SharedIndexMap<K, V> {
    fn default() -> Self {
        Self(Arc::new(IndexMap::new()))
    }
}
impl<K, V> From<IndexMap<K, V>> for SharedIndexMap<K, V> {
    fn from(map: IndexMap<K, V>) -> Self {
        Self(Arc::new(map))
    }
}
impl<K, V> SharedIndexMap<K, V> {
    pub fn clear(&mut self) {
        if let Some(map) = Arc::get_mut(&mut self.0) {
            map.clear();
        } else {
            *self = Self::default();
        }
    }
    pub fn same_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<K, V> Deref for SharedIndexMap<K, V> {
    type Target = IndexMap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<K: Clone, V: Clone> DerefMut for SharedIndexMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}
impl<K: Eq + Hash, V: Eq> PartialEq for SharedIndexMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.same_storage(other) || self.0.as_ref() == other.0.as_ref()
    }
}
impl<K: Eq + Hash, V: Eq> Eq for SharedIndexMap<K, V> {}
impl<K: Serialize, V: Serialize> Serialize for SharedIndexMap<K, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}
impl<'de, K: Deserialize<'de> + Eq + Hash, V: Deserialize<'de>> Deserialize<'de>
    for SharedIndexMap<K, V>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        IndexMap::deserialize(deserializer).map(Self::from)
    }
}
impl<'a, K, V> IntoIterator for &'a SharedIndexMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = indexmap::map::Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a, K: Clone, V: Clone> IntoIterator for &'a mut SharedIndexMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = indexmap::map::IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        Arc::make_mut(&mut self.0).iter_mut()
    }
}
impl<K: Clone, V: Clone> IntoIterator for SharedIndexMap<K, V> {
    type Item = (K, V);
    type IntoIter = indexmap::map::IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        Arc::unwrap_or_clone(self.0).into_iter()
    }
}
impl<K: Eq + Hash, V> FromIterator<(K, V)> for SharedIndexMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        IndexMap::from_iter(iter).into()
    }
}
