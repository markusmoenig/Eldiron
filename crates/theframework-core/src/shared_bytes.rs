//! Copy-on-write CPU asset bytes. Ownership changes do not change the serialized
//! Vec<u8> representation; renderer/job handles never enter persistent data.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
};

/// A shareable immutable source of individually addressable byte planes.
pub trait BytePlanes: Send + Sync {
    fn read(&self, plane: usize) -> Result<&Arc<Vec<u8>>, &str>;
}

#[derive(Clone)]
pub struct SharedBytes(Storage);

#[derive(Clone)]
enum Storage {
    Resident(Arc<Vec<u8>>),
    Deferred(Arc<Deferred>),
}

struct Deferred {
    source: Arc<dyn BytePlanes>,
    plane: usize,
    fallback_len: usize,
    fallback: OnceLock<Arc<Vec<u8>>>,
}

impl SharedBytes {
    pub fn is_deferred(&self) -> bool {
        matches!(self.0, Storage::Deferred(_))
    }

    pub fn deferred(source: Arc<dyn BytePlanes>, plane: usize, fallback_len: usize) -> Self {
        Self(Storage::Deferred(Arc::new(Deferred {
            source,
            plane,
            fallback_len,
            fallback: OnceLock::new(),
        })))
    }

    pub fn try_resolve(&self) -> Result<&Vec<u8>, &str> {
        self.read().map(AsRef::as_ref)
    }

    /// Immutable identity for derived in-memory caches. Archive clones with
    /// the same plane preserve the identity. Mutated bytes are hashed directly.
    pub fn hash_source(&self, hash: &mut impl Hasher) {
        match &self.0 {
            Storage::Resident(bytes) => {
                0u8.hash(hash);
                bytes.hash(hash);
            }
            Storage::Deferred(plane) => {
                1u8.hash(hash);
                Arc::as_ptr(&plane.source).cast::<()>().hash(hash);
                plane.plane.hash(hash);
            }
        }
    }

    fn read(&self) -> Result<&Arc<Vec<u8>>, &str> {
        match &self.0 {
            Storage::Resident(bytes) => Ok(bytes),
            Storage::Deferred(plane) => plane.source.read(plane.plane),
        }
    }

    fn bytes(&self) -> &Arc<Vec<u8>> {
        match &self.0 {
            Storage::Resident(bytes) => bytes,
            Storage::Deferred(plane) => plane.source.read(plane.plane).unwrap_or_else(|error| {
                // Existing texture readers are infallible. Keep indexing safe
                // for corrupt archives and surface the failure through save.
                plane.fallback.get_or_init(|| {
                    eprintln!("Unable to load baked asset data: {error}");
                    Arc::new(vec![0; plane.fallback_len])
                })
            }),
        }
    }
}

impl Default for SharedBytes {
    fn default() -> Self {
        Vec::new().into()
    }
}
impl fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Storage::Resident(bytes) => f.debug_tuple("SharedBytes").field(&bytes.len()).finish(),
            Storage::Deferred(plane) => f.debug_tuple("DeferredBytes").field(&plane.plane).finish(),
        }
    }
}
impl PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Storage::Resident(a), Storage::Resident(b)) if Arc::ptr_eq(a, b) => true,
            (Storage::Deferred(a), Storage::Deferred(b))
                if a.plane == b.plane && Arc::ptr_eq(&a.source, &b.source) => true,
            _ => self.deref() == other.deref(),
        }
    }
}
impl Eq for SharedBytes {}
impl PartialEq<Vec<u8>> for SharedBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl PartialEq<SharedBytes> for Vec<u8> {
    fn eq(&self, other: &SharedBytes) -> bool {
        other == self
    }
}
impl<const N: usize> PartialEq<[u8; N]> for SharedBytes {
    fn eq(&self, other: &[u8; N]) -> bool {
        self.as_slice() == other
    }
}
impl PartialOrd for SharedBytes {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SharedBytes {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other { Ordering::Equal } else { self.deref().cmp(other.deref()) }
    }
}
impl Hash for SharedBytes {
    fn hash<H: Hasher>(&self, hash: &mut H) {
        self.deref().hash(hash);
    }
}
impl From<Vec<u8>> for SharedBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(Storage::Resident(Arc::new(bytes)))
    }
}
impl FromIterator<u8> for SharedBytes {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}
impl From<SharedBytes> for Vec<u8> {
    fn from(bytes: SharedBytes) -> Self {
        match bytes.0 {
            Storage::Resident(bytes) => Arc::unwrap_or_clone(bytes),
            Storage::Deferred(_) => bytes.to_vec(),
        }
    }
}
impl Deref for SharedBytes {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        self.bytes()
    }
}
impl DerefMut for SharedBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if matches!(self.0, Storage::Deferred(_)) {
            self.0 = Storage::Resident(self.bytes().clone());
        }
        let Storage::Resident(bytes) = &mut self.0 else { unreachable!() };
        Arc::make_mut(bytes)
    }
}
impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
impl Serialize for SharedBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.try_resolve().map_err(serde::ser::Error::custom)?.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for SharedBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<u8>::deserialize(deserializer).map(Self::from)
    }
}
