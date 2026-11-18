//! Incremental, storage-only records. Capture every persisted field through
//! serde; do not depend on renderer revisions or best-effort dirty flags.
use super::*;
use serde::{
    Deserialize, Serialize,
    de::DeserializeOwned,
    ser::{self, SerializeMap, SerializeStruct},
};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use theframework_core::Uuid;
pub const MAX_GROUP_ACTIONS: usize = 8;
pub(crate) const MAX_PARTS: usize = 512_000;
pub(crate) const MAX_UPDATE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Key {
    Root(String),
    World(String),
    Region(Uuid, String),
    Geometry(Uuid),
    Entity(Uuid, u32),
    Item(Uuid, u32),
    Stashed(Uuid, u32),
    Respawn(Uuid, u32),
    Character(Uuid),
    Request(Uuid),
    Action(Uuid),
    Host(String),
    Regions,
    Characters,
    Entities(Uuid),
    Items(Uuid),
    Stashes(Uuid),
    Respawns(Uuid),
}
type Raw = Arc<RawValue>;
#[derive(Clone, Default)]
pub(crate) struct ImageIndex {
    parts: BTreeMap<Key, Raw>,
    geometry: BTreeMap<Uuid, [u8; 32]>,
}
#[derive(Deserialize)]
struct Update {
    version: u32,
    world: Uuid,
    from: u64,
    to: u64,
    changes: Vec<(Key, Option<Box<RawValue>>)>,
}
#[derive(Serialize)]
struct UpdateRef<'a> {
    version: u32,
    world: Uuid,
    from: u64,
    to: u64,
    changes: Vec<(&'a Key, Option<&'a RawValue>)>,
}
/// Checked changes and their resulting complete image. Adapters may journal
/// `bytes()` or use the snapshot fallback; both commit the same world/receipts.
pub struct PreparedUpdate {
    image: WorldSave,
    index: ImageIndex,
    bytes: Vec<u8>,
    from: u64,
}
impl PreparedUpdate {
    pub fn image(&self) -> &WorldSave {
        &self.image
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn previous_revision(&self) -> u64 {
        self.from
    }
    pub fn snapshot(&self) -> Result<PreparedSave, SaveError> {
        PreparedSave::new(&self.image)
    }
    pub(crate) fn into_head(self) -> (WorldSave, ImageIndex) {
        (self.image, self.index)
    }
    pub(crate) fn new(
        base: &WorldSave,
        old: &ImageIndex,
        next: WorldSave,
    ) -> Result<Self, SaveError> {
        if next.world != base.world
            || next.content != base.content
            || next.purpose != base.purpose
            || next.version != base.version
            || Some(next.revision) != base.revision.checked_add(1)
        {
            return Err(SaveError::Conflict);
        }
        let (image, index) = ImageIndex::capture(next, Some(old))?;
        let mut changes = vec![];
        for (key, value) in &index.parts {
            if old
                .parts
                .get(key)
                .is_none_or(|old| !Arc::ptr_eq(old, value) && old.get() != value.get())
            {
                changes.push((key, Some(value.as_ref())));
            }
        }
        for key in old.parts.keys() {
            if !index.parts.contains_key(key) {
                changes.push((key, None));
            }
        }
        let bytes = serde_json::to_vec(&UpdateRef {
            version: 1,
            world: image.world,
            from: base.revision,
            to: image.revision,
            changes,
        })
        .map_err(invalid)?;
        if bytes.len() > MAX_SAVE_BYTES {
            return Err(invalid("Incremental transaction exceeds size limit"));
        }
        Ok(Self {
            image,
            index,
            bytes,
            from: base.revision,
        })
    }
}
fn raw<T: Serialize + ?Sized>(value: &T) -> Result<Raw, SaveError> {
    serde_json::value::to_raw_value(value)
        .map(Arc::from)
        .map_err(invalid)
}
fn checked<T: Serialize + DeserializeOwned>(value: &mut T) -> Result<Raw, SaveError> {
    let encoded = raw(value)?;
    *value = serde_json::from_str(encoded.get()).map_err(invalid)?;
    Ok(encoded)
}
struct DigestWriter(Sha256);
impl std::io::Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn fingerprint<T: Serialize>(value: &T) -> Result<[u8; 32], SaveError> {
    let mut writer = DigestWriter(Sha256::new());
    value
        .serialize(
            &mut rmp_serde::Serializer::new(&mut writer)
                .with_struct_map()
                .with_human_readable(),
        )
        .map_err(invalid)?;
    Ok(writer.0.finalize().into())
}
fn object(fields: &BTreeMap<String, Raw>) -> Result<Vec<u8>, SaveError> {
    serde_json::to_vec(&RawMap(fields)).map_err(invalid)
}
struct RawMap<'a>(&'a BTreeMap<String, Raw>);
impl Serialize for RawMap<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0 {
            map.serialize_entry(k, v.as_ref())?;
        }
        map.end()
    }
}
fn empty_world() -> WorldRecord {
    WorldRecord {
        next_id: 1,
        values: Default::default(),
        regions: vec![],
        render: Default::default(),
        region_render: vec![],
    }
}

fn geometry_stub() -> Raw {
    static STUB: std::sync::OnceLock<Raw> = std::sync::OnceLock::new();
    STUB.get_or_init(|| {
        let mut map = rusterix::Map::default();
        map.id = Uuid::nil();
        raw(&rusterix::server::persistence::SavedGeometry::capture(&map)).unwrap()
    })
    .clone()
}
const ROOT_SKIP: &[&str] = &[
    "world_state",
    "characters",
    "requests",
    "host_data",
    "action_streams",
];
const REGION_SKIP: &[&str] = &["geometry", "entities", "items", "stashed", "respawns"];
impl ImageIndex {
    pub(crate) fn from_image(image: &WorldSave) -> Result<Self, SaveError> {
        Self::capture(image.clone(), None).map(|(_, index)| index)
    }
    pub(crate) fn capture(
        mut image: WorldSave,
        old: Option<&Self>,
    ) -> Result<(WorldSave, Self), SaveError> {
        image.validate()?;
        let mut index = Self::default();
        // All remaining metadata fields are discovered from the actual DTO,
        // so new behavior/VM/timer fields cannot be accidentally omitted.
        let root = fields(&image, ROOT_SKIP)?;
        let mut skeleton = root.clone();
        skeleton.insert("world_state".into(), raw(&empty_world())?);
        skeleton.insert("characters".into(), raw(&Vec::<u8>::new())?);
        for name in ["requests", "host_data", "action_streams"] {
            skeleton.insert(name.into(), raw(&BTreeMap::<String, u8>::new())?);
        }
        let mut head: WorldSave = serde_json::from_slice(&object(&skeleton)?).map_err(invalid)?;
        head.world_state = image.world_state;
        head.characters = image.characters;
        head.requests = image.requests;
        head.host_data = image.host_data;
        head.action_streams = image.action_streams;
        image = head;
        for (key, value) in root {
            index.parts.insert(Key::Root(key), value);
        }
        let world = fields(&image.world_state, &["regions"])?;
        let mut skeleton = world.clone();
        skeleton.insert("regions".into(), raw(&Vec::<u8>::new())?);
        let mut meta: WorldRecord = serde_json::from_slice(&object(&skeleton)?).map_err(invalid)?;
        meta.regions = image.world_state.regions;
        image.world_state = meta;
        for (key, value) in world {
            index.parts.insert(Key::World(key), value);
        }
        index.parts.insert(
            Key::Regions,
            raw(&image
                .world_state
                .regions
                .iter()
                .map(|r| r.geometry.id)
                .collect::<Vec<_>>())?,
        );
        for region in &mut image.world_state.regions {
            let id = region.geometry.id;
            let tail = fields(region, REGION_SKIP)?;
            let mut skeleton = tail.clone();
            skeleton.insert("geometry".into(), geometry_stub());
            for name in ["entities", "items", "stashed", "respawns"] {
                skeleton.insert(name.into(), raw(&Vec::<u8>::new())?);
            }
            let mut meta: RegionRecord =
                serde_json::from_slice(&object(&skeleton)?).map_err(invalid)?;
            std::mem::swap(&mut meta.geometry, &mut region.geometry);
            meta.entities = std::mem::take(&mut region.entities);
            meta.items = std::mem::take(&mut region.items);
            meta.stashed = std::mem::take(&mut region.stashed);
            meta.respawns = std::mem::take(&mut region.respawns);
            *region = meta;
            for (key, value) in tail {
                index.parts.insert(Key::Region(id, key), value);
            }
            // Binary serde preserves float bits and uses the same human-readable
            // field choices as JSON. Hashing avoids float formatting and parsing
            // unchanged geometry; hashes are scoped to this storage owner.
            let mut digest = fingerprint(&region.geometry)?;
            let geometry = if let Some(old) = old.filter(|o| o.geometry.get(&id) == Some(&digest)) {
                old.parts.get(&Key::Geometry(id)).unwrap().clone()
            } else {
                let value = checked(&mut region.geometry)?;
                digest = fingerprint(&region.geometry)?;
                value
            };
            index.geometry.insert(id, digest);
            index.parts.insert(Key::Geometry(id), geometry);
            index.parts.insert(
                Key::Entities(id),
                raw(&region.entities.iter().map(|e| e.id).collect::<Vec<_>>())?,
            );
            index.parts.insert(
                Key::Items(id),
                raw(&region.items.iter().map(|e| e.id).collect::<Vec<_>>())?,
            );
            index.parts.insert(
                Key::Stashes(id),
                raw(&region.stashed.iter().map(|e| e.id).collect::<Vec<_>>())?,
            );
            index.parts.insert(
                Key::Respawns(id),
                raw(&region.respawns.iter().map(|e| e.0).collect::<Vec<_>>())?,
            );
            for entity in &mut region.entities {
                index
                    .parts
                    .insert(Key::Entity(id, entity.id), checked(entity)?);
            }
            for item in &mut region.items {
                index.parts.insert(Key::Item(id, item.id), checked(item)?);
            }
            for entity in &mut region.stashed {
                index
                    .parts
                    .insert(Key::Stashed(id, entity.id), checked(entity)?);
            }
            let mut respawns = BTreeSet::new();
            for (key, entity) in &mut region.respawns {
                if !respawns.insert(*key) {
                    return Err(invalid("Duplicate respawn record"));
                }
                index.parts.insert(Key::Respawn(id, *key), checked(entity)?);
            }
        }
        index.parts.insert(
            Key::Characters,
            raw(&image.characters.iter().map(|c| c.id).collect::<Vec<_>>())?,
        );
        for c in &mut image.characters {
            index.parts.insert(Key::Character(c.id), checked(c)?);
        }
        for (k, v) in &mut image.requests {
            index.parts.insert(Key::Request(*k), checked(v)?);
        }
        for (k, v) in &mut image.action_streams {
            index.parts.insert(Key::Action(*k), checked(v)?);
        }
        for (k, v) in &mut image.host_data {
            index.parts.insert(Key::Host(k.clone()), checked(v)?);
        }
        image.validate()?;
        index.validate_size()?;
        // Reuse identical immutable records, including large item trees.
        if let Some(old) = old {
            for (key, value) in &mut index.parts {
                if let Some(prior) = old
                    .parts
                    .get(key)
                    .filter(|v| Arc::ptr_eq(v, value) || v.get() == value.get())
                {
                    *value = prior.clone();
                }
            }
        }
        Ok((image, index))
    }
    fn root(&self) -> Result<BTreeMap<String, Raw>, SaveError> {
        let mut root = BTreeMap::new();
        let mut world = BTreeMap::new();
        let mut regions = vec![];
        let mut chars = BTreeMap::new();
        let mut requests = BTreeMap::new();
        let mut actions = BTreeMap::new();
        let mut host = BTreeMap::new();
        for (key, value) in &self.parts {
            match key {
                Key::Root(key) => {
                    root.insert(key.clone(), value.clone());
                }
                Key::World(key) => {
                    world.insert(key.clone(), value.clone());
                }
                Key::Character(id) => {
                    chars.insert(*id, value.clone());
                }
                Key::Request(id) => {
                    requests.insert(id.to_string(), value.clone());
                }
                Key::Action(id) => {
                    actions.insert(id.to_string(), value.clone());
                }
                Key::Host(key) => {
                    host.insert(key.clone(), value.clone());
                }
                _ => {}
            }
        }
        let order: Vec<Uuid> = self.get(&Key::Regions)?;
        if order.iter().copied().collect::<BTreeSet<_>>().len() != order.len() {
            return Err(invalid("Duplicate incremental region"));
        }
        for id in order {
            let mut region = BTreeMap::new();
            for (key, value) in &self.parts {
                if let Key::Region(owner, key) = key
                    && *owner == id
                {
                    region.insert(key.clone(), value.clone());
                }
            }
            region.insert("geometry".into(), self.part(&Key::Geometry(id))?.clone());
            for (name, order, kind) in [
                ("entities", Key::Entities(id), 0),
                ("items", Key::Items(id), 1),
                ("stashed", Key::Stashes(id), 2),
                ("respawns", Key::Respawns(id), 3),
            ] {
                let order: Vec<u32> = self.get(&order)?;
                if order.iter().copied().collect::<BTreeSet<_>>().len() != order.len() {
                    return Err(invalid("Duplicate incremental object"));
                }
                let mut values = vec![];
                for key in order {
                    let part = self.part(&match kind {
                        0 => Key::Entity(id, key),
                        1 => Key::Item(id, key),
                        2 => Key::Stashed(id, key),
                        _ => Key::Respawn(id, key),
                    })?;
                    values.push(if kind == 3 {
                        raw(&(key, part.as_ref()))?
                    } else {
                        part.clone()
                    });
                }
                region.insert(name.into(), raw(&RawArray(&values))?);
            }
            regions.push(raw(&RawMap(&region))?);
        }
        world.insert("regions".into(), raw(&RawArray(&regions))?);
        root.insert("world_state".into(), raw(&RawMap(&world))?);
        let order: Vec<Uuid> = self.get(&Key::Characters)?;
        let mut values = vec![];
        let mut seen = BTreeSet::new();
        for id in order {
            if !seen.insert(id) {
                return Err(invalid("Duplicate incremental character"));
            }
            values.push(
                chars
                    .remove(&id)
                    .ok_or_else(|| invalid("Missing incremental character"))?,
            );
        }
        if !chars.is_empty() {
            return Err(invalid("Unreferenced incremental character"));
        }
        root.insert("characters".into(), raw(&RawArray(&values))?);
        for (name, values) in [
            ("requests", requests),
            ("host_data", host),
            ("action_streams", actions),
        ] {
            root.insert(name.into(), raw(&RawMap(&values))?);
        }
        Ok(root)
    }
    fn part(&self, key: &Key) -> Result<&Raw, SaveError> {
        self.parts
            .get(key)
            .ok_or_else(|| invalid("Missing incremental record"))
    }
    fn get<T: DeserializeOwned>(&self, key: &Key) -> Result<T, SaveError> {
        serde_json::from_str(self.part(key)?.get()).map_err(invalid)
    }
    fn validate_size(&self) -> Result<(), SaveError> {
        if self.parts.len() > MAX_PARTS {
            return Err(invalid("Too many incremental records"));
        }
        // Bound the complete portable image, not just this transaction's bytes.
        // Container and array/key overhead are conservatively reserved. No need
        // to parse or copy unchanged geometry to measure its serialized size.
        let size: usize = self
            .parts
            .iter()
            .map(|(k, v)| {
                v.get().len()
                    + match k {
                        Key::Root(k) | Key::World(k) | Key::Region(_, k) | Key::Host(k) => {
                            k.len() * 6 + 128
                        }
                        _ => 128,
                    }
            })
            .sum();
        if size > MAX_SAVE_BYTES {
            return Err(invalid("Incremental world exceeds size limit"));
        }
        Ok(())
    }
    pub(crate) fn materialize(&self) -> Result<WorldSave, SaveError> {
        let bytes = object(&self.root()?)?;
        if bytes.len() + 128 > MAX_SAVE_BYTES {
            return Err(invalid("Replayed world exceeds size limit"));
        }
        let image: WorldSave = serde_json::from_slice(&bytes).map_err(invalid)?;
        image.validate()?;
        Ok(image)
    }
    pub(crate) fn apply(
        &mut self,
        bytes: &[u8],
        world: Uuid,
        revision: u64,
    ) -> Result<u64, SaveError> {
        if bytes.len() > MAX_UPDATE_BYTES {
            return Err(invalid("Journal transaction exceeds limit"));
        }
        let update: Update = serde_json::from_slice(bytes).map_err(invalid)?;
        if update.version != 1
            || update.world != world
            || update.from != revision
            || Some(update.to) != revision.checked_add(1)
            || update.changes.len() > MAX_PARTS
        {
            return Err(invalid("Invalid journal transaction identity"));
        }
        let mut seen = BTreeSet::new();
        for (key, value) in update.changes {
            if !seen.insert(key.clone()) {
                return Err(invalid("Duplicate journal change"));
            }
            if let Some(value) = value {
                self.parts.insert(key, Arc::from(value));
            } else if self.parts.remove(&key).is_none() {
                return Err(invalid("Unknown journal removal"));
            }
        }
        self.validate_size()?;
        if self.get::<u64>(&Key::Root("revision".into()))? != update.to
            || self.get::<Uuid>(&Key::Root("world".into()))? != world
        {
            return Err(invalid("Journal header differs from transaction"));
        }
        Ok(update.to)
    }
}
struct RawArray<'a>(&'a [Raw]);
impl Serialize for RawArray<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for value in self.0 {
            seq.serialize_element(value.as_ref())?;
        }
        seq.end()
    }
}

// A serializer for the root fields only. Child fields use ordinary JSON serde.
// This automatically includes future persisted VM/behavior metadata fields.
struct Fields<'a> {
    skip: &'a [&'a str],
    values: BTreeMap<String, Raw>,
}
fn fields<T: Serialize>(value: &T, skip: &[&str]) -> Result<BTreeMap<String, Raw>, SaveError> {
    value
        .serialize(Fields {
            skip,
            values: BTreeMap::new(),
        })
        .map_err(invalid)
}
impl SerializeStruct for Fields<'_> {
    type Ok = BTreeMap<String, Raw>;
    type Error = serde_json::Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        if !self.skip.contains(&key) {
            self.values
                .insert(key.into(), serde_json::value::to_raw_value(value)?.into());
        }
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.values)
    }
}
macro_rules! unsupported {($($name:ident($($arg:ident:$ty:ty),*);)*)=>{$(fn $name(self,$($arg:$ty),*)->Result<Self::Ok,Self::Error>{$(let _=$arg;)*Err(ser::Error::custom("Expected a storage record struct"))})*};}
impl serde::Serializer for Fields<'_> {
    type Ok = BTreeMap<String, Raw>;
    type Error = serde_json::Error;
    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Self::Error> {
        Ok(self)
    }
    unsupported! {serialize_bool(v:bool);serialize_i8(v:i8);serialize_i16(v:i16);serialize_i32(v:i32);serialize_i64(v:i64);serialize_i128(v:i128);serialize_u8(v:u8);serialize_u16(v:u16);serialize_u32(v:u32);serialize_u64(v:u64);serialize_u128(v:u128);serialize_f32(v:f32);serialize_f64(v:f64);serialize_char(v:char);serialize_str(v:&str);serialize_bytes(v:&[u8]);serialize_unit();serialize_unit_struct(v:&'static str);serialize_none();serialize_unit_variant(n:&'static str,i:u32,v:&'static str);}
    fn serialize_some<T: Serialize + ?Sized>(self, _: &T) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom("Expected struct"))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use rusterix::{Entity, Item, Value};
    pub(crate) fn fixture() -> WorldSave {
        let mut map = rusterix::Map::default();
        map.id = Uuid::new_v4();
        let region = RegionRecord {
            geometry: rusterix::server::persistence::SavedGeometry::capture(&map),
            entities: vec![],
            items: vec![],
            stashed: vec![],
            respawns: vec![],
            values: Default::default(),
            ticks: 0,
            time: Default::default(),
            entity_classes: vec![],
            item_classes: vec![],
            entity_state: vec![],
            item_state: vec![],
            entity_alerts: vec![],
            item_alerts: vec![],
            entity_notifications: vec![],
            item_notifications: vec![],
            talk_steps: vec![],
            talk_resumes: vec![],
            dismiss_dialogues: vec![],
            entity_events: vec![],
            item_events: vec![],
            world_events: vec![],
            behaviors: Default::default(),
            globals: vec![],
            simulation_time: Default::default(),
            last_advance: Default::default(),
            last_request: None,
            pending_system: 0,
            pending_redraw: 0,
            step_pending: false,
        };
        let mut image = WorldSave {
            version: SAVE_VERSION,
            world: Uuid::new_v4(),
            purpose: SavePurpose::Dedicated,
            content: "a".repeat(64),
            revision: 1,
            characters: vec![],
            local_character: None,
            requests: Default::default(),
            host_data: Default::default(),
            action_streams: Default::default(),
            world_state: WorldRecord {
                regions: vec![region],
                ..empty_world()
            },
            clock: rusterix::simulation::SimulationClock::new(std::time::Duration::from_millis(
                250,
            ))
            .save(),
            tick: 0,
            movement_tick: 0,
        };
        let mut actor = Entity::default();
        actor.id = 1;
        actor.set_attribute("player", Value::Bool(true));
        let mut item = Item::default();
        item.id = 2;
        actor.inventory = vec![Some(item)];
        image.world_state.regions[0]
            .entities
            .push(SavedEntity::capture(&actor));
        image.world_state.next_id = 3;
        image.characters.push(CharacterRecord {
            id: Uuid::new_v4(),
            account: Some(Uuid::new_v4()),
            entity: 1,
            region: map.id,
        });
        image.validate().unwrap();
        image
    }
    fn assert_same(a: &WorldSave, b: &WorldSave) {
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
    }
    #[test]
    fn incremental_records_roundtrip_all_metadata_and_atomic_moves() {
        let image = fixture();
        let index = ImageIndex::from_image(&image).unwrap();
        assert_same(&image, &index.materialize().unwrap());
        let mut next = image.clone();
        next.revision += 1;
        next.tick = 10;
        next.movement_tick = 20;
        next.host_data.insert(
            "private".into(),
            serde_json::json!({"credential":"not replicated"}),
        );
        next.requests.insert(
            Uuid::new_v4(),
            CommittedRequest {
                fingerprint: "creation".into(),
                outcome: serde_json::json!(1),
            },
        );
        next.action_streams
            .entry(next.characters[0].id)
            .or_default()
            .append(1, "a".repeat(64), serde_json::json!("moved"))
            .unwrap();
        let region = &mut next.world_state.regions[0];
        region.geometry.name = "changed without a revision".into();
        region.geometry.offset.x = -0.;
        region.ticks = 10;
        region.values.set("flag", Value::Bool(true));
        region.entity_state.push((1, Default::default()));
        region.entity_notifications.push((1, 5, "wake".into()));
        region.item_notifications.push((2, 6, "expire".into()));
        region.globals.push(rusterix::vm::VMValue::from_bool(true));
        region
            .world_events
            .push(("event".into(), rusterix::vm::VMValue::from_bool(true)));
        region.last_request = Some(std::time::Duration::from_millis(10));
        region.pending_system = 2;
        region.pending_redraw = 1;
        region.step_pending = true;
        let mut actor = region.entities.remove(0);
        region.items.push(actor.inventory[0].take().unwrap());
        region.stashed.push(actor);
        let update = PreparedUpdate::new(&image, &index, next).unwrap();
        let mut replay = index.clone();
        assert_eq!(
            replay
                .apply(update.bytes(), image.world, image.revision)
                .unwrap(),
            2
        );
        assert_same(update.image(), &replay.materialize().unwrap());
        assert_eq!(
            update.image().world_state.regions[0]
                .geometry
                .offset
                .x
                .to_bits(),
            (-0f32).to_bits()
        );
        assert!(replay.apply(update.bytes(), image.world, 2).is_err());
        let mut wrong = index.clone();
        assert!(wrong.apply(update.bytes(), Uuid::new_v4(), 1).is_err());
        let mut malformed = serde_json::from_slice::<serde_json::Value>(update.bytes()).unwrap();
        let duplicate = malformed["changes"][0].clone();
        malformed["changes"].as_array_mut().unwrap().push(duplicate);
        assert!(
            index
                .clone()
                .apply(&serde_json::to_vec(&malformed).unwrap(), image.world, 1)
                .is_err()
        );
    }
    #[test]
    fn unchanged_geometry_is_shared_and_nonfinite_rejected() {
        let image = fixture();
        let index = ImageIndex::from_image(&image).unwrap();
        let id = image.world_state.regions[0].geometry.id;
        let mut invalid = image.clone();
        invalid.revision += 1;
        invalid.world_state.regions[0].respawns = vec![
            (1, invalid.world_state.regions[0].entities[0].clone()),
            (1, invalid.world_state.regions[0].entities[0].clone()),
        ];
        assert!(PreparedUpdate::new(&image, &index, invalid).is_err());
        let mut next = image.clone();
        next.revision += 1;
        let update = PreparedUpdate::new(&image, &index, next).unwrap();
        assert!(Arc::ptr_eq(
            index.part(&Key::Geometry(id)).unwrap(),
            update.index.part(&Key::Geometry(id)).unwrap()
        ));
        assert!(update.bytes().len() < 1024);
        let mut bad = image.clone();
        bad.revision += 1;
        bad.world_state.regions[0].geometry.grid_size = f32::NAN;
        assert!(PreparedUpdate::new(&image, &index, bad).is_err());
    }
}
