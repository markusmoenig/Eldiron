//! Decode current projects without staging every texture/paint array as JSON
//! Values. Typed structs stream; maps retain borrowed fields for key sorting.
//! Map keys still use the previous Value decoder's lexical/last-key policy:
//! IndexMap iteration order can affect authored asset selection and draw order.
use super::Project;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, de::*};
use serde_json::{Error, Value, value::RawValue};
use std::{collections::BTreeMap, fmt};

pub(crate) fn decode(bytes: &[u8]) -> Result<Project, Error> {
    std::str::from_utf8(bytes).map_err(Error::custom)?;
    validate_depth(bytes)?;
    let mut legacy = false;
    let mut reader = serde_json::Deserializer::from_slice(bytes);
    // The remote derive is the original field decoder. Observe the retired
    // root group field while decoding, avoiding a second full-document probe.
    let project =
        Project::deserialize(ProjectReader(&mut reader, &mut legacy)).and_then(|project| {
            reader.end()?;
            Ok(project)
        });
    match project {
        Ok(project) if !legacy => Ok(project),
        Ok(project) => {
            drop(project);
            serde_json::from_slice(bytes)
        }
        // Retain original group flattening, errors and unusual previously
        // accepted inputs, including duplicate structs/invalid early values.
        Err(_) => serde_json::from_slice(bytes),
    }
}

fn validate_depth(bytes: &[u8]) -> Result<(), Error> {
    // RawValue skips nested values without Value's recursion accounting, and
    // each borrowed field starts a new JSON reader. Retain the original 127
    // container limit, including unknown fields, without building a value tree.
    // RawValue has already validated syntax/UTF-8; only strings hide delimiters.
    let (mut depth, mut quoted, mut escaped) = (0u32, false, false);
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth == 128 {
                        return Err(Error::custom("recursion limit exceeded"));
                    }
                }
                b'}' | b']' => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| Error::custom("unexpected closing delimiter"))?
                }
                _ => {}
            }
        }
    }
    Ok(())
}

struct ProjectReader<'a, D>(D, &'a mut bool);
impl<'de, D: Deserializer<'de>> Deserializer<'de> for ProjectReader<'_, D> {
    type Error = D::Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.0.deserialize_any(ProjectVisitor(visitor, self.1))
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }
}
struct ProjectVisitor<'a, V>(V, &'a mut bool);
impl<'de, V: Visitor<'de>> Visitor<'de> for ProjectVisitor<'_, V> {
    type Value = V::Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.expecting(f)
    }
    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
        self.0.visit_map(ProjectFields {
            map,
            legacy: self.1,
            group: false,
        })
    }
}
struct ProjectFields<'a, M> {
    map: M,
    legacy: &'a mut bool,
    group: bool,
}
impl<'de, M: MapAccess<'de>> MapAccess<'de> for ProjectFields<'_, M> {
    type Error = M::Error;
    fn next_key_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, M::Error> {
        let Some(key) = self.map.next_key::<String>()? else {
            return Ok(None);
        };
        self.group = key == "tile_groups";
        // Root Project keys are named struct identifiers, not numeric map keys.
        seed.deserialize(serde::de::value::StringDeserializer::<M::Error>::new(key))
            .map(Some)
    }
    fn next_value_seed<S: DeserializeSeed<'de>>(&mut self, seed: S) -> Result<S::Value, M::Error> {
        if self.group {
            let raw: &RawValue = self.map.next_value()?;
            *self.legacy = has_groups(raw).map_err(M::Error::custom)?;
            seed.deserialize(Sorted(raw)).map_err(M::Error::custom)
        } else {
            self.map.next_value_seed(Seed(seed))
        }
    }
    fn size_hint(&self) -> Option<usize> {
        self.map.size_hint()
    }
}
fn has_groups<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    struct Groups;
    impl<'de> Visitor<'de> for Groups {
        type Value = bool;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a legacy tile group map")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<bool, M::Error> {
            let mut any = false;
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {
                any = true;
            }
            Ok(any)
        }
    }
    d.deserialize_map(Groups)
}

struct Sorted<D>(D);
macro_rules! forward {
    ($method:ident $(, $arg:ident: $ty:ty)*) => {
        fn $method<V: Visitor<'de>>(self, $($arg: $ty,)* visitor: V) -> Result<V::Value, D::Error> {
            self.0.$method($($arg,)* Wrap(visitor))
        }
    };
}
impl<'de, D: Deserializer<'de>> Deserializer<'de> for Sorted<D> {
    type Error = D::Error;
    forward!(deserialize_any);
    forward!(deserialize_bool);
    forward!(deserialize_i8);
    forward!(deserialize_i16);
    forward!(deserialize_i32);
    forward!(deserialize_i64);
    forward!(deserialize_i128);
    forward!(deserialize_u8);
    forward!(deserialize_u16);
    forward!(deserialize_u32);
    forward!(deserialize_u64);
    forward!(deserialize_u128);
    forward!(deserialize_f32);
    forward!(deserialize_f64);
    forward!(deserialize_char);
    forward!(deserialize_str);
    forward!(deserialize_string);
    forward!(deserialize_bytes);
    forward!(deserialize_byte_buf);
    forward!(deserialize_option);
    forward!(deserialize_unit);
    forward!(deserialize_unit_struct, name: &'static str);
    forward!(deserialize_newtype_struct, name: &'static str);
    forward!(deserialize_seq);
    forward!(deserialize_tuple, len: usize);
    forward!(deserialize_tuple_struct, name: &'static str, len: usize);
    forward!(deserialize_map);
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.0.deserialize_struct(name, fields, Struct(visitor))
    }
    forward!(deserialize_enum, name: &'static str, variants: &'static [&'static str]);
    forward!(deserialize_identifier);
    forward!(deserialize_ignored_any);
}

struct Wrap<V>(V);
macro_rules! visit {
    ($method:ident, $ty:ty) => {
        fn $method<E: serde::de::Error>(self, value: $ty) -> Result<Self::Value, E> {
            self.0.$method(value)
        }
    };
}
impl<'de, V: Visitor<'de>> Visitor<'de> for Wrap<V> {
    type Value = V::Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.expecting(f)
    }
    visit!(visit_bool, bool);
    visit!(visit_i8, i8);
    visit!(visit_i16, i16);
    visit!(visit_i32, i32);
    visit!(visit_i64, i64);
    visit!(visit_i128, i128);
    visit!(visit_u8, u8);
    visit!(visit_u16, u16);
    visit!(visit_u32, u32);
    visit!(visit_u64, u64);
    visit!(visit_u128, u128);
    visit!(visit_f32, f32);
    visit!(visit_f64, f64);
    visit!(visit_char, char);
    visit!(visit_str, &str);
    visit!(visit_borrowed_str, &'de str);
    visit!(visit_string, String);
    visit!(visit_bytes, &[u8]);
    visit!(visit_borrowed_bytes, &'de [u8]);
    visit!(visit_byte_buf, Vec<u8>);
    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.0.visit_none()
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.0.visit_unit()
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        self.0.visit_some(Sorted(d))
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        self.0.visit_newtype_struct(Sorted(d))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, a: A) -> Result<Self::Value, A::Error> {
        self.0.visit_seq(Sequence(a))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
        let mut fields = BTreeMap::new();
        while let Some((key, raw)) = a.next_entry::<String, &RawValue>()? {
            fields.insert(key, raw);
        }
        self.0
            .visit_map(Fields {
                fields: fields.into_iter(),
                pending: None,
            })
            .map_err(A::Error::custom)
    }
    fn visit_enum<A: EnumAccess<'de>>(self, a: A) -> Result<Self::Value, A::Error> {
        self.0.visit_enum(Enum(a))
    }
}

struct Seed<S>(S);
impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for Seed<S> {
    type Value = S::Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        self.0.deserialize(Sorted(d))
    }
}

// Struct field order does not determine map iteration order. Decode directly,
// retaining the sorting wrapper only for each field's nested actual maps.
// This avoids rescanning large region/paint arrays at every struct boundary.
struct Struct<V>(V);
impl<'de, V: Visitor<'de>> Visitor<'de> for Struct<V> {
    type Value = V::Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.expecting(f)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, a: A) -> Result<Self::Value, A::Error> {
        self.0.visit_seq(Sequence(a))
    }
    fn visit_map<A: MapAccess<'de>>(self, a: A) -> Result<Self::Value, A::Error> {
        self.0.visit_map(StructFields(a))
    }
}
struct StructFields<A>(A);
impl<'de, A: MapAccess<'de>> MapAccess<'de> for StructFields<A> {
    type Error = A::Error;
    fn next_key_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, A::Error> {
        self.0.next_key_seed(seed)
    }
    fn next_value_seed<S: DeserializeSeed<'de>>(&mut self, seed: S) -> Result<S::Value, A::Error> {
        self.0.next_value_seed(Seed(seed))
    }
    fn size_hint(&self) -> Option<usize> {
        self.0.size_hint()
    }
}
struct Sequence<A>(A);
impl<'de, A: SeqAccess<'de>> SeqAccess<'de> for Sequence<A> {
    type Error = A::Error;
    fn next_element_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, A::Error> {
        self.0.next_element_seed(Seed(seed))
    }
    fn size_hint(&self) -> Option<usize> {
        self.0.size_hint()
    }
}

struct Fields<'de> {
    fields: std::collections::btree_map::IntoIter<String, &'de RawValue>,
    pending: Option<&'de RawValue>,
}
impl<'de> MapAccess<'de> for Fields<'de> {
    type Error = Error;
    fn next_key_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Error> {
        let Some((key, raw)) = self.fields.next() else {
            return Ok(None);
        };
        self.pending = Some(raw);
        // Replay only this key through serde_json's own map-key decoder. Plain
        // StringDeserializer rejects numeric/bool/enum keys accepted in JSON.
        // Owned Value keys also avoid borrowing from a temporary escaped key.
        let mut key_object = serde_json::Map::new();
        key_object.insert(key, Value::Null);
        Value::Object(key_object)
            .deserialize_map(Key(seed))
            .map(Some)
    }
    fn next_value_seed<S: DeserializeSeed<'de>>(&mut self, seed: S) -> Result<S::Value, Error> {
        seed.deserialize(Sorted(
            self.pending
                .take()
                .ok_or_else(|| Error::custom("missing raw field"))?,
        ))
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len())
    }
}
struct Key<S>(S);
impl<'de, S: DeserializeSeed<'de>> Visitor<'de> for Key<S> {
    type Value = S::Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("one JSON object key")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<Self::Value, M::Error> {
        let key = m
            .next_key_seed(self.0)?
            .ok_or_else(|| M::Error::custom("missing object key"))?;
        m.next_value::<IgnoredAny>()?;
        Ok(key)
    }
}
struct Enum<A>(A);
impl<'de, A: EnumAccess<'de>> EnumAccess<'de> for Enum<A> {
    type Error = A::Error;
    type Variant = Variant<A::Variant>;
    fn variant_seed<S: DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<(S::Value, Self::Variant), A::Error> {
        self.0
            .variant_seed(seed)
            .map(|(key, value)| (key, Variant(value)))
    }
}
struct Variant<A>(A);
impl<'de, A: VariantAccess<'de>> VariantAccess<'de> for Variant<A> {
    type Error = A::Error;
    fn unit_variant(self) -> Result<(), A::Error> {
        self.0.unit_variant()
    }
    fn newtype_variant_seed<S: DeserializeSeed<'de>>(self, seed: S) -> Result<S::Value, A::Error> {
        self.0.newtype_variant_seed(Seed(seed))
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, A::Error> {
        self.0.tuple_variant(len, Wrap(visitor))
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, A::Error> {
        self.0.struct_variant(fields, Struct(visitor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde::Serialize;

    fn project_json(path: &std::path::Path) -> Vec<u8> {
        let bytes = std::fs::read(path).unwrap();
        if bytes.starts_with(b"PK\x03\x04") {
            use std::io::Read;
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
            let mut output = Vec::new();
            archive
                .by_name("project.json")
                .unwrap()
                .read_to_end(&mut output)
                .unwrap();
            output
        } else {
            bytes
        }
    }

    #[test]
    fn shipped_current_sources_keep_serialized_content_and_indexmap_order() {
        fn normalize_entity_sets(map: &mut Value) {
            for field in ["entities", "items"] {
                if let Some(entries) = map.get_mut(field).and_then(Value::as_array_mut) {
                    for entry in entries {
                        // Entity/Item declare this field as FxHashSet<String>.
                        // Streaming sequences have no eager array size hint;
                        // set growth can change iteration order, never contents.
                        if let Some(set) = entry
                            .get_mut("dirty_attributes")
                            .and_then(Value::as_array_mut)
                        {
                            set.sort_by(|a, b| a.as_str().unwrap().cmp(b.as_str().unwrap()));
                        }
                    }
                }
            }
            if let Some(profiles) = map.get_mut("profiles").and_then(Value::as_object_mut) {
                for profile in profiles.values_mut() {
                    normalize_entity_sets(profile);
                }
            }
        }
        fn source(project: &Project) -> Value {
            let mut value = serde_json::to_value(project).unwrap();
            for region in value["regions"].as_array_mut().unwrap() {
                normalize_entity_sets(&mut region["map"]);
            }
            value
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_projects");
        for name in ["Hideout2D.eldiron", "3DStarter.eldiron"] {
            let bytes = project_json(&root.join(name));
            let old: Project = serde_json::from_slice(&bytes).unwrap();
            // Resolve historically absent generated-ID defaults once before
            // comparing the current serialized source in both typed decoders.
            let canonical = serde_json::to_vec(&old).unwrap();
            let expected: Project = serde_json::from_slice(&canonical).unwrap();
            let new = decode(&canonical).unwrap();
            assert!(
                source(&new) == source(&expected),
                "serialized source content differs: {name}"
            );
            macro_rules! ordered {
                ($($field:ident),* $(,)?) => { $(
                    assert_eq!(new.$field.keys().collect::<Vec<_>>(), expected.$field.keys().collect::<Vec<_>>(), "{} {} order", name, stringify!($field));
                )* };
            }
            ordered!(
                tiles,
                procedural_materials,
                procedural_appearances,
                isometric_bakes,
                material_compositions,
                map_material_bindings,
                procedural_material_cache_keys,
                block_props,
                prefab_graphs,
                block_prop_paint,
                tile_collections,
                tile_board_tiles,
                avatars
            );
        }
    }

    #[test]
    #[ignore = "opt-in isolated release decoder profiling; needs ELDIRON_PROJECT_JSON_PATH"]
    fn profile_project_json_decoder() {
        let path = std::env::var_os("ELDIRON_PROJECT_JSON_PATH").expect("project path");
        let bytes = project_json(std::path::Path::new(&path));
        let backend =
            std::env::var("ELDIRON_PROJECT_JSON_DECODER").unwrap_or_else(|_| "streamed".into());
        let start = std::time::Instant::now();
        let project: Project = match backend.as_str() {
            "streamed" => decode(&bytes).unwrap(),
            "reference" => serde_json::from_slice(&bytes).unwrap(),
            _ => panic!("unsupported project decoder {backend}"),
        };
        let elapsed = start.elapsed();
        let frames = project
            .tiles
            .values()
            .flat_map(|t| &t.textures)
            .collect::<Vec<_>>();
        eprintln!(
            "{}",
            serde_json::json!({"decoder":backend,"json_bytes":bytes.len(),"decode_ms":elapsed.as_secs_f64()*1000.,
            "project":project.name,"regions":project.regions.len(),"tiles":project.tiles.len(),"tile_frames":frames.len(),
            "tile_color_bytes":frames.iter().map(|t|t.data.len()).sum::<usize>(),"material_sources":project.procedural_materials.len()})
        );
        std::hint::black_box(project);
    }

    #[derive(Serialize, Deserialize, Debug)]
    enum Example {
        Unit,
        Newtype(IndexMap<String, Vec<u8>>),
        Tuple(u32, IndexMap<u32, String>),
        Struct {
            z: Option<Vec<IndexMap<String, u32>>>,
            a: String,
        },
    }
    #[test]
    fn borrowed_fields_preserve_value_order_keys_variants_and_duplicate_policy() {
        let bytes = br#"{"z":{"Struct":{"z":[{"b":2,"a":1}],"a":"escaped\\value"}},"b":{"Tuple":[7,{"20":"z","3":"a"}]},"a":{"Newtype":{"z":[255,2,0],"a":[]}},"b":{"Tuple":[9,{"20":"z","3":"a"}]},"u":"Unit"}"#;
        let value: Value = serde_json::from_slice(bytes).unwrap();
        let old: IndexMap<String, Example> = serde_json::from_value(value).unwrap();
        let raw: &RawValue = serde_json::from_slice(bytes).unwrap();
        let new = IndexMap::<String, Example>::deserialize(Sorted(raw)).unwrap();
        assert_eq!(
            serde_json::to_string(&new).unwrap(),
            serde_json::to_string(&old).unwrap()
        );
    }

    #[test]
    fn current_project_decoder_preserves_complete_serialization_and_rejects_invalid_groups() {
        let mut project = Project::new();
        project.tiles.insert(
            theframework_core::prelude::Uuid::from_u128(9),
            rusterix::Tile::from_texture(rusterix::Texture::from_color([9, 8, 7, 6])),
        );
        project.tiles.insert(
            theframework_core::prelude::Uuid::from_u128(1),
            rusterix::Tile::from_texture(rusterix::Texture::from_color([1, 2, 3, 4])),
        );
        let bytes = serde_json::to_vec(&project).unwrap();
        let expected: Project = serde_json::from_slice(&bytes).unwrap();
        let actual = decode(&bytes).unwrap();
        assert_eq!(
            serde_json::to_string(&actual).unwrap(),
            serde_json::to_string(&expected).unwrap()
        );
        let duplicated = format!(
            "{{\"name\":17,{}",
            &std::str::from_utf8(&bytes).unwrap()[1..]
        );
        let old: Project = serde_json::from_str(&duplicated).unwrap();
        assert_eq!(
            serde_json::to_string(&decode(duplicated.as_bytes()).unwrap()).unwrap(),
            serde_json::to_string(&old).unwrap()
        );
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["tile_groups"] = serde_json::json!({});
        value["tile_board_groups"] = serde_json::json!({"unused": {"x": 1, "y": 2}});
        let bytes = serde_json::to_vec(&value).unwrap();
        let expected: Project = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            serde_json::to_string(&decode(&bytes).unwrap()).unwrap(),
            serde_json::to_string(&expected).unwrap()
        );
        for invalid in [Value::Null, serde_json::json!([]), serde_json::json!(17)] {
            value["tile_groups"] = invalid;
            let bytes = serde_json::to_vec(&value).unwrap();
            assert!(serde_json::from_slice::<Project>(&bytes).is_err());
            assert!(decode(&bytes).is_err());
        }
    }

    #[test]
    fn raw_field_readers_preserve_json_recursion_limit_including_unknown_fields() {
        let mut value = serde_json::to_value(Project::new()).unwrap();
        value["unknown"] = serde_json::json!("escaped quote \\\" and {[ delimiters");
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(decode(&bytes).is_ok());
        for count in [126, 127, 128, 256] {
            // root object adds one container above the nested unknown array.
            let bytes = format!(
                "{{\"unknown\":{}0{}}}",
                "[".repeat(count),
                "]".repeat(count)
            );
            let old = serde_json::from_str::<Value>(&bytes);
            let new = validate_depth(bytes.as_bytes());
            assert_eq!(new.is_ok(), old.is_ok(), "{count}");
            if new.is_err() {
                assert!(decode(bytes.as_bytes()).is_err());
            }
        }
    }
}
