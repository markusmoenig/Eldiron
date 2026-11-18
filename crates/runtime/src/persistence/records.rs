use super::*;
use crate::Project;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use theframework_core::Uuid;

pub const SAVE_VERSION: u32 = 1;
pub const MAX_SAVE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_DURABLE_REQUESTS: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavePurpose {
    SinglePlayer,
    CreatorTest(String),
    Dedicated,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterRecord {
    pub id: Uuid,
    /// Trusted host association; clients cannot assign this owner.
    pub account: Option<Uuid>,
    pub entity: u32,
    pub region: Uuid,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CommittedRequest {
    pub fingerprint: String,
    pub outcome: serde_json::Value,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct WorldSave {
    pub version: u32,
    pub world: Uuid,
    pub purpose: SavePurpose,
    pub content: String,
    pub revision: u64,
    pub characters: Vec<CharacterRecord>,
    pub local_character: Option<Uuid>,
    pub requests: BTreeMap<Uuid, CommittedRequest>,
    /// Private host metadata, never included in replication or content exports.
    #[serde(default)]
    pub host_data: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub action_streams: BTreeMap<Uuid, ActionJournal>,
    pub world_state: WorldRecord,
    pub clock: rusterix::simulation::SavedClock,
    pub tick: u64,
    pub movement_tick: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    Invalid(String),
    Incompatible(String),
    Store(String),
    Conflict,
    Busy,
    NeedsReload,
    AlreadyCommitted,
    ActionSequence { next: u64 },
}
impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SaveError {}
pub(super) fn invalid(error: impl std::fmt::Display) -> SaveError {
    SaveError::Invalid(error.to_string())
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn canonical(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::to_value(
            map.into_iter()
                .map(|(k, v)| (k, canonical(v)))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap(),
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(canonical).collect())
        }
        value => value,
    }
}
pub fn content_revision(project: &Project) -> Result<String, SaveError> {
    let definition = serde_json::to_value(project).map_err(invalid)?;
    let rules = project.rules_source().map_err(invalid)?;
    Ok(digest(
        &serde_json::to_vec(&canonical(serde_json::json!({
            "project":definition,"resolved_rules":rules,"runtime":env!("CARGO_PKG_VERSION"),
            "save_schema":SAVE_VERSION,
        })))
        .map_err(invalid)?,
    ))
}
fn legacy_container() -> u32 {
    1
}
#[derive(Deserialize)]
struct Envelope<'a> {
    #[serde(default = "legacy_container")]
    format: u32,
    checksum: String,
    #[serde(borrow)]
    payload: &'a serde_json::value::RawValue,
}
#[derive(Serialize)]
struct RawEnvelope<'a> {
    format: u32,
    checksum: String,
    payload: &'a serde_json::value::RawValue,
}
impl WorldSave {
    pub fn validate(&self) -> Result<(), SaveError> {
        if self.version != SAVE_VERSION {
            return Err(SaveError::Incompatible(format!(
                "Save schema {} requires migration (supported {SAVE_VERSION})",
                self.version
            )));
        }
        if self.world.is_nil()
            || self.content.len() != 64
            || self.revision == 0
            || self.requests.len() > MAX_DURABLE_REQUESTS
        {
            return Err(invalid("Invalid save metadata"));
        }
        self.world_state.validate().map_err(invalid)?;
        rusterix::simulation::SimulationClock::restore(&self.clock).map_err(invalid)?;
        let mut ids = HashSet::new();
        let mut bodies = HashSet::new();
        for character in &self.characters {
            if character.id.is_nil()
                || !ids.insert(character.id)
                || !bodies.insert(character.entity)
                || !self.world_state.regions.iter().any(|r| {
                    r.geometry.id == character.region
                        && r.entities.iter().chain(&r.stashed).any(|e| {
                            e.id == character.entity
                                && e.attributes.get_bool_default("player", false)
                        })
                })
            {
                return Err(invalid("Invalid/duplicate durable character or location"));
            }
        }
        if self.action_streams.len() > 100_000
            || self
                .action_streams
                .iter()
                .any(|(id, journal)| id.is_nil() || !journal.valid())
        {
            return Err(invalid("Invalid character action journal"));
        }
        if self.local_character.is_some_and(|id| !ids.contains(&id)) {
            return Err(invalid("Local character missing"));
        }
        let players: HashSet<_> = self
            .world_state
            .regions
            .iter()
            .flat_map(|r| r.entities.iter().chain(&r.stashed))
            .filter(|e| e.attributes.get_bool_default("player", false))
            .map(|e| e.id)
            .collect();
        if players != bodies {
            return Err(invalid("A saved player has no durable character identity"));
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>, SaveError> {
        self.validate()?;
        // Container 2 embeds the exact JSON object instead of escaping a
        // second complete JSON document inside a string. World schema stays 1.
        let payload = serde_json::value::to_raw_value(self).map_err(invalid)?;
        let bytes = serde_json::to_vec(&RawEnvelope {
            format: 2,
            checksum: digest(payload.get().as_bytes()),
            payload: &payload,
        })
        .map_err(invalid)?;
        if bytes.len() > MAX_SAVE_BYTES {
            return Err(invalid("Save exceeds size limit"));
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SaveError> {
        if bytes.len() > MAX_SAVE_BYTES {
            return Err(invalid("Save exceeds size limit"));
        }
        let envelope: Envelope<'_> = serde_json::from_slice(bytes).map_err(invalid)?;
        let payload: std::borrow::Cow<'_, str> = match envelope.format {
            1 => std::borrow::Cow::Owned(
                serde_json::from_str::<String>(envelope.payload.get()).map_err(invalid)?,
            ),
            2 => std::borrow::Cow::Borrowed(envelope.payload.get()),
            _ => {
                return Err(SaveError::Incompatible(
                    "Save container requires explicit migration".into(),
                ));
            }
        };
        if digest(payload.as_bytes()) != envelope.checksum {
            return Err(invalid("Save checksum mismatch"));
        }
        // Decode directly into typed records. Building a complete JSON Value
        // tree here multiplied allocation/parse costs for every durable click.
        let saved: Self = match serde_json::from_str(&payload) {
            Ok(saved) => saved,
            Err(error) => {
                #[derive(Deserialize)]
                struct Header {
                    version: u32,
                }
                if serde_json::from_str::<Header>(&payload).is_ok_and(|h| h.version != SAVE_VERSION)
                {
                    return Err(SaveError::Incompatible(
                        "Save schema requires explicit migration".into(),
                    ));
                }
                return Err(invalid(error));
            }
        };
        saved.validate()?;
        Ok(saved)
    }
}

/// A checked image and its exact encoded bytes. Private fields prevent callers
/// from pairing one world's receipt/header with another world's payload.
/// Store adapters may reuse the encoding without weakening validation.
pub struct PreparedSave {
    image: WorldSave,
    bytes: Vec<u8>,
}
impl PreparedSave {
    pub fn new(image: &WorldSave) -> Result<Self, SaveError> {
        let bytes = image.encode()?;
        let image = WorldSave::decode(&bytes)?;
        Ok(Self { image, bytes })
    }
    pub fn image(&self) -> &WorldSave {
        &self.image
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_image(self) -> WorldSave {
        self.image
    }
}
