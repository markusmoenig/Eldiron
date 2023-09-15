use crate::prelude::*;

pub mod area;
pub mod behavior;
pub mod game;
pub mod item;
pub mod player;
pub mod system;
pub mod utilities;

pub type NodeCall = fn(
    instance_index: usize,
    id: (Uuid, Uuid),
    data: &mut RegionInstance,
    behavior_type: BehaviorType,
) -> BehaviorNodeConnector;
pub type NodeDataCall =
    fn(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector;
