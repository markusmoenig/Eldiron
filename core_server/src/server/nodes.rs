use crate::prelude::*;

pub mod utilities;
pub mod behavior;
pub mod area;
pub mod game;
pub mod player;
pub mod item;
pub mod system;

pub type NodeCall = fn(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector;
