use crate::prelude::*;
use core_shared::prelude::*;

/// Skill tree
#[allow(unused)]
pub fn skill_tree(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Bottom
}

/// Skill tree
#[allow(unused)]
pub fn skill_level(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Bottom
}