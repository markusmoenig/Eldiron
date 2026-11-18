//! Versioned, recipient-projected snapshots shared by typed local delivery and
//! future WSS delivery. Static region/art content has a separate revision path.
mod snapshot;
pub use snapshot::{EntityPose, EntitySnapshot, ItemSnapshot, ReplicationPolicy};
mod client;
pub use client::{ApplyResult, ClientWorld, ReplicationError};
pub mod spatial;
pub use spatial::{SpatialPolicy, SpatialSubscriptions};
mod hub;
pub(crate) use hub::ReplicationHub;

pub use rusterix::server::region::resources::ResourceView;
use crate::RegionEvents;
use rusterix::{BlockPropInstance, ServerState, server::message::RuntimeRenderState};
use serde::{Deserialize, Serialize};
use theframework_core::{TheTime, Uuid};

pub const REPLICATION_VERSION: u16 = 3;
pub const MAX_QUEUED_FRAMES: usize = 64;
pub const MAX_REGIONS: usize = 1024;
pub const MAX_OBJECTS: usize = 100_000;
pub const MAX_PENDING_EVENTS: usize = 16_384;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RegionContentRef {
    pub map: Uuid,
    pub map_revision: u32,
    pub runtime_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegionSnapshot {
    pub content: RegionContentRef,
    pub entities: Vec<EntitySnapshot>,
    pub items: Vec<ItemSnapshot>,
    pub props: Vec<BlockPropInstance>,
    #[serde(default)]
    pub resource_views: Vec<ResourceView>,
    pub time: TheTime,
    pub render: RuntimeRenderState,
}

/// Changed objects are complete replacements, including attribute deletions.
/// This avoids a fresh observer depending on global dirty flags or item deltas.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegionDelta {
    pub content: RegionContentRef,
    pub poses: Vec<EntityPose>,
    pub entities: Vec<EntitySnapshot>,
    pub removed_entities: Vec<u32>,
    pub items: Vec<ItemSnapshot>,
    pub removed_items: Vec<u32>,
    pub props: Option<Vec<BlockPropInstance>>,
    #[serde(default)]
    pub resource_views: Option<Vec<ResourceView>>,
    pub time: TheTime,
    pub render: RuntimeRenderState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum WorldUpdate {
    Baseline(Vec<RegionSnapshot>),
    Delta(Vec<RegionDelta>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegionDelivery {
    pub region: Uuid,
    pub events: RegionEvents,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientFrame {
    pub version: u16,
    pub world: Uuid,
    pub connection: Uuid,
    pub generation: u64,
    pub sequence: u64,
    pub tick: u64,
    pub movement_tick: u64,
    pub motion: Option<rusterix::movement::MotionSnapshot>,
    pub controlled_entity: Option<u32>,
    pub current_region: Option<Uuid>,
    pub state: ServerState,
    pub update: WorldUpdate,
    pub events: Vec<RegionDelivery>,
}
