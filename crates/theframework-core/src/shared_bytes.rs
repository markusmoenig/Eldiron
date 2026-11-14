//! Copy-on-write CPU asset bytes. Ownership changes do not change the serialized
//! Vec<u8> representation; renderer/job handles never enter persistent data.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SharedBytes(Arc<Vec<u8>>);
impl From<Vec<u8>> for SharedBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(Arc::new(bytes))
    }
}
impl From<SharedBytes> for Vec<u8> {
    fn from(bytes: SharedBytes) -> Self {
        Arc::unwrap_or_clone(bytes.0)
    }
}
impl Deref for SharedBytes {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for SharedBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}
impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl Serialize for SharedBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for SharedBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<u8>::deserialize(deserializer).map(Self::from)
    }
}
