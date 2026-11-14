//! Immutable shared CPU text payloads with the exact String wire shape. Large
//! encoded image channels can cross document/job ownership without copying.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SharedText(Arc<String>);
impl SharedText {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Replacing a derived/save projection must not copy an original payload
    /// merely to discard its contents. Other owners keep their original text.
    pub fn clear(&mut self) {
        self.0 = Arc::default();
    }
}
impl From<String> for SharedText {
    fn from(value: String) -> Self {
        Self(Arc::new(value))
    }
}
impl From<&str> for SharedText {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}
impl From<SharedText> for String {
    fn from(value: SharedText) -> Self {
        Arc::unwrap_or_clone(value.0)
    }
}
impl Deref for SharedText {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<str> for SharedText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<[u8]> for SharedText {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl std::fmt::Display for SharedText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl Serialize for SharedText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for SharedText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}
impl PartialEq<String> for SharedText {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}
impl PartialEq<str> for SharedText {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl PartialEq<&str> for SharedText {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn string_json_and_binary_wire_shapes_survive_shared_ownership_and_projection_clear() {
        let original = "PNG/depth Base64 payload 日本語".repeat(1000);
        let shared = SharedText::from(original.clone());
        let mut projection = shared.clone();
        assert_eq!(shared.as_ptr(), projection.as_ptr());
        assert_eq!(
            serde_json::to_vec(&shared).unwrap(),
            serde_json::to_vec(&original).unwrap()
        );
        assert_eq!(
            bincode::serialize(&shared).unwrap(),
            bincode::serialize(&original).unwrap()
        );
        let restored: SharedText =
            bincode::deserialize(&bincode::serialize(&original).unwrap()).unwrap();
        assert_eq!(restored.as_str(), original);
        projection.clear();
        assert!(projection.is_empty());
        assert_eq!(shared.as_str(), original);
        let wire = serde_json::to_vec(&shared).unwrap();
        let old: String = serde_json::from_slice(&wire).unwrap();
        assert_eq!(old, original);
    }
}
