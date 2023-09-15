use crate::prelude::*;
use core_shared::prelude::*;

/// Skill tree
#[allow(unused)]
pub fn node_skill_tree(
    _id: (Uuid, Uuid),
    _nodes: &mut FxHashMap<Uuid, GameBehaviorData>,
) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Bottom
}

/// Skill tree
#[allow(unused)]
pub fn node_skill_level(
    _id: (Uuid, Uuid),
    _nodes: &mut FxHashMap<Uuid, GameBehaviorData>,
) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Bottom
}
