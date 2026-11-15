//! Shared authored properties for immutable scene publications. Serialization
//! delegates to the original map; allocation and equality metadata stay private.
use super::Value;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
};
use theframework_core::prelude::FxHashMap;

#[derive(Clone, Debug, Default)]
struct Storage {
    values: FxHashMap<String, Value>,
    reflexive: OnceLock<bool>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct SharedValues(Arc<Storage>);

impl SharedValues {
    pub(super) fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for SharedValues {
    type Target = FxHashMap<String, Value>;
    fn deref(&self) -> &Self::Target {
        &self.0.values
    }
}
impl DerefMut for SharedValues {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let storage = Arc::make_mut(&mut self.0);
        storage.reflexive.take();
        &mut storage.values
    }
}
impl PartialEq for SharedValues {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.0, &other.0) {
            // Value contains floats (also nested inside textures/materials).
            // Preserve the original map's NaN/nonreflexive equality behavior.
            *self
                .0
                .reflexive
                .get_or_init(|| self.0.values.values().all(|value| value == value))
        } else {
            self.0.values == other.0.values
        }
    }
}
impl Serialize for SharedValues {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.values.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for SharedValues {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(Arc::new(Storage {
            values: FxHashMap::deserialize(deserializer)?,
            reflexive: OnceLock::new(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValueContainer;

    #[test]
    fn property_publications_share_storage_and_detach_every_mutation() {
        let mut authored = ValueContainer::new();
        authored.set("name", Value::Str("authored".into()));
        authored.set("enabled", Value::Bool(true));
        let frozen = authored.clone();
        assert!(Arc::ptr_eq(&authored.values.0, &frozen.values.0));
        assert_eq!(authored, frozen);
        assert_eq!(frozen.cached_fingerprint(|| 41), 41);
        // Cached semantic fingerprints remain valid in the immutable owner.
        for mutation in 0..4 {
            let mut edited = frozen.clone();
            match mutation {
                0 => edited.set("name", Value::Str("edited".into())),
                1 => *edited.get_mut("name").unwrap() = Value::Str("edited".into()),
                2 => edited.toggle("enabled"),
                _ => {
                    edited.remove("name");
                }
            }
            assert!(!Arc::ptr_eq(&edited.values.0, &frozen.values.0));
            assert_ne!(edited, frozen);
            assert_eq!(edited.cached_fingerprint(|| 42), 42);
            assert_eq!(
                frozen.cached_fingerprint(|| panic!("frozen fingerprint invalidated")),
                41
            );
            assert_eq!(frozen.get_str("name"), Some("authored"));
            assert_eq!(frozen.get_bool("enabled"), Some(true));
        }
    }

    #[test]
    fn shared_equality_preserves_nonreflexive_values_and_invalidates_on_edit() {
        let mut original = ValueContainer::new();
        original.set("number", Value::Float(1.));
        let frozen = original.clone();
        assert_eq!(original, frozen); // primes the shared reflexivity metadata
        *original.get_mut("number").unwrap() = Value::Float(f32::NAN);
        let nan = original.clone();
        assert_ne!(original, nan);
        assert_ne!(original, original);
        assert_eq!(frozen.get_float("number"), Some(1.));
        original.set("number", Value::Vec3([0., f32::NAN, 1.]));
        assert_ne!(original, original.clone());
        original.set("number", Value::Float(1.));
        assert_eq!(original, original.clone());
        assert_eq!(original, frozen); // independently allocated but equal maps
    }

    #[test]
    fn property_storage_preserves_original_json_and_binary_map_shape() {
        #[derive(Serialize, Deserialize)]
        struct OriginalContainer {
            values: FxHashMap<String, Value>,
        }
        let mut values = ValueContainer::new();
        values.set("name", Value::Str("unchanged format".into()));
        values.set("height", Value::Float(2.5));
        values.set("point", Value::Vec3([1., 2., 3.]));
        let original = OriginalContainer {
            values: (*values.values).clone(),
        };
        let json = serde_json::to_vec(&original).unwrap();
        let binary = bincode::serialize(&original).unwrap();
        assert_eq!(serde_json::to_vec(&values).unwrap(), json);
        assert_eq!(bincode::serialize(&values).unwrap(), binary);
        assert_eq!(
            serde_json::from_slice::<ValueContainer>(&json).unwrap(),
            values
        );
        assert_eq!(
            bincode::deserialize::<ValueContainer>(&binary).unwrap(),
            values
        );
        let reopened: OriginalContainer =
            bincode::deserialize(&bincode::serialize(&values).unwrap()).unwrap();
        assert_eq!(reopened.values, original.values);
    }

    #[test]
    fn property_source_identity_retains_serialized_fields_ignored_by_value_equality() {
        use theframework_core::prelude::TheColor;
        let mut authored = ValueContainer::new();
        authored.set("color", Value::Color(TheColor::new(0.5, 0.2, 0.1, 1.)));
        let frozen = authored.clone();
        assert!(authored.same_serialized_source(&frozen));
        if let Some(Value::Color(color)) = authored.get_mut("color") {
            color.name = "different serialized name".into();
            color.r += 0.00001; // existing color equality deliberately tolerates this
        }
        assert_eq!(authored, frozen);
        assert!(!authored.same_serialized_source(&frozen));
        assert!(authored.same_serialized_source(&authored.clone()));
    }
}
