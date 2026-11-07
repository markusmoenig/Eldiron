//! Editable recipes with an in-memory mutation stamp. Saved project data stays
//! an ordinary ordered map; accessing mutable data invalidates dependent caches,
//! including in-place String edits and undo/project replacement.
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU64, Ordering},
};
use theframework::prelude::Uuid;

fn stamp() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProceduralAssets {
    data: IndexMap<Uuid, String>,
    #[serde(skip, default = "stamp")]
    revision: u64,
}
impl ProceduralAssets {
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
impl Default for ProceduralAssets {
    fn default() -> Self {
        Self {
            data: IndexMap::new(),
            revision: stamp(),
        }
    }
}
impl Clone for ProceduralAssets {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            revision: stamp(),
        }
    }
}
impl PartialEq for ProceduralAssets {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}
impl Deref for ProceduralAssets {
    type Target = IndexMap<Uuid, String>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl DerefMut for ProceduralAssets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.revision = stamp();
        &mut self.data
    }
}
impl<'a> IntoIterator for &'a ProceduralAssets {
    type Item = (&'a Uuid, &'a String);
    type IntoIter = indexmap::map::Iter<'a, Uuid, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}
impl<'a> IntoIterator for &'a mut ProceduralAssets {
    type Item = (&'a Uuid, &'a mut String);
    type IntoIter = indexmap::map::IterMut<'a, Uuid, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.revision = stamp();
        self.data.iter_mut()
    }
}
impl IntoIterator for ProceduralAssets {
    type Item = (Uuid, String);
    type IntoIter = indexmap::map::IntoIter<Uuid, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
impl FromIterator<(Uuid, String)> for ProceduralAssets {
    fn from_iter<T: IntoIterator<Item = (Uuid, String)>>(iter: T) -> Self {
        Self {
            data: iter.into_iter().collect(),
            revision: stamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_clones_and_reloads_invalidate_but_reads_and_saves_do_not() {
        let mut a = ProceduralAssets::default();
        let id = Uuid::new_v4();
        a.insert(id, "abcd".into());
        let r = a.revision();
        assert_eq!(&a[&id], "abcd");
        assert_eq!(a.revision(), r);
        a.get_mut(&id).unwrap().replace_range(0..1, "z");
        assert_ne!(a.revision(), r);
        let r = a.revision();
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(a.revision(), r);
        assert!(!json.contains("revision"));
        let b: ProceduralAssets = serde_json::from_str(&json).unwrap();
        assert_eq!(a, b);
        assert_ne!(a.revision(), b.revision());
        let c = a.clone();
        assert_eq!(a, c);
        assert_ne!(a.revision(), c.revision());
        for (_, text) in &mut a {
            text.clear();
        }
        assert_ne!(a.revision(), r);
    }
}
