//! Transport data never contains a simulation Entity, Map, script, or Assets.
//! Inventory is explicit: Entity's persistence serde skips inventory on decode.
use rusterix::{Entity, Item, Value, ValueContainer};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use theframework_core::Uuid;

#[derive(Clone, Debug)]
pub struct ReplicationPolicy {
    pub entity_attributes: BTreeSet<String>,
    pub item_attributes: BTreeSet<String>,
    pub prop_state: BTreeSet<String>,
}
impl Default for ReplicationPolicy {
    fn default() -> Self {
        let visual = [
            "name",
            "visible",
            "source",
            "tile_id",
            "portrait_tile_id",
            "mode",
            "active",
            "class_name",
            "player",
            "player_camera",
            "class",
            "race",
            "level",
            "sector",
            "sector_id",
            "avatar",
            "avatar_id",
            "avatar_animation",
            "avatar_attack_serial",
            "avatar_pose_only",
            "avatar_stride",
            "avatar_move_speed",
            "avatar_blend_time",
            "avatar_walk_speed",
            "appearance_geometry",
            "appearance_geometry_off",
            "appearance_scale",
            "appearance_yaw",
            "appearance_ground_flat",
            "appearance_idle",
            "appearance_walk",
            "appearance_attack",
            "appearance_attack_time",
            "appearance_hold_joint",
            "appearance_materials",
            "appearance_padding",
            "appearance_two_handed",
            "appearance_worn_geometry",
            "avatar_attack_time",
            "visual_template",
            "slot",
            "creator_template_id",
            "ruleset_id",
            "ruleset_path",
            "resource_access_id",
            "size",
            "radius",
            "height",
            "entity_height",
            "collision_height",
            "light",
            "light_color",
            "color",
            "rig_color",
            "color_index",
            "rig_color_index",
            "party_leader_id",
            "party_index",
            "party_member",
            "party_role",
            "_source_seq",
            "HP",
            "MAX_HP",
            "health",
            "max_health",
            "is_spell",
            "stackable",
            "quantity",
            "amount",
            "container",
            "is_container",
            "container_slots",
            "animation_clock",
            "animation_duration",
            "avatar_walk_rate",
            "billboard_alignment",
            "billboard_animation",
            "blocking",
            "static",
            "takeable",
            "door_group_id",
            "fx_lifetime",
            "fx_persistent",
            "fx_size_scale",
            "geometry_object_id",
            "ground_icon_size",
            "is_ruleset_fx",
            "iso_cutaway_area",
            "particle_emitter",
            "profile_host_sector_id",
            "profile_sector_id",
            "size_2d",
            "spell_impacting",
            "sector_feature",
            "campfire_flame_height",
            "campfire_flame_width",
            "campfire_flame_source",
            "source_size",
            "LEVEL",
        ];
        Self {
            entity_attributes: visual.iter().map(|s| s.to_string()).collect(),
            item_attributes: visual.iter().map(|s| s.to_string()).collect(),
            prop_state: ["open", "visible", "state"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }
}
impl ReplicationPolicy {
    pub(crate) fn attributes(
        &self,
        source: &ValueContainer,
        private: bool,
        item: bool,
    ) -> BTreeMap<String, Value> {
        let public = if item {
            &self.item_attributes
        } else {
            &self.entity_attributes
        };
        source
            .iter()
            .filter(|(key, _)| public.contains(*key) || (private && !key.starts_with('_')))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ItemSnapshot {
    pub id: u32,
    pub creator_id: Uuid,
    pub position: [f32; 3],
    pub item_type: String,
    pub max_capacity: u32,
    pub is_container: bool,
    pub attributes: BTreeMap<String, Value>,
    /// None means contents are undisclosed (or this is not a container).
    pub contents: Option<Vec<ItemSnapshot>>,
}
impl ItemSnapshot {
    pub(crate) fn capture(item: &Item, private: bool, policy: &ReplicationPolicy) -> Self {
        Self {
            id: item.id,
            creator_id: item.creator_id,
            position: [item.position.x, item.position.y, item.position.z],
            item_type: item.item_type.clone(),
            max_capacity: item.max_capacity,
            is_container: item.is_container()
                || item.attributes.get_bool_default("container", false)
                || item.attributes.get_int_default("container_slots", 0) > 0,
            attributes: policy.attributes(&item.attributes, private, true),
            contents: private
                .then(|| {
                    item.container.as_ref().map(|items| {
                        items
                            .iter()
                            .map(|item| Self::capture(item, true, policy))
                            .collect()
                    })
                })
                .flatten(),
        }
    }
    pub(crate) fn materialize(&self) -> Item {
        Item {
            id: self.id,
            creator_id: self.creator_id,
            position: self.position.into(),
            item_type: self.item_type.clone(),
            max_capacity: self.max_capacity,
            attributes: attributes(&self.attributes),
            container: self
                .contents
                .as_ref()
                .map(|items| items.iter().map(Self::materialize).collect())
                .or_else(|| self.is_container.then(Vec::new)),
            ..Item::default()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntitySnapshot {
    pub id: u32,
    pub creator_id: Uuid,
    pub position: [f32; 3],
    pub orientation: [f32; 2],
    pub tilt: f32,
    pub motion_revision: u32,
    pub attributes: BTreeMap<String, Value>,
    pub equipped: BTreeMap<String, ItemSnapshot>,
    pub inventory: Option<Vec<Option<ItemSnapshot>>>,
    pub wallet: Option<BTreeMap<String, i64>>,
}
impl EntitySnapshot {
    pub(crate) fn capture(entity: &Entity, owner: Option<u32>, policy: &ReplicationPolicy) -> Self {
        let private = owner.is_some_and(|id| {
            entity.id == id || entity.attributes.get_uint("party_leader_id") == Some(id)
        });
        Self {
            id: entity.id,
            creator_id: entity.creator_id,
            position: [entity.position.x, entity.position.y, entity.position.z],
            orientation: [entity.orientation.x, entity.orientation.y],
            tilt: entity.tilt,
            motion_revision: rusterix::movement::motion_revision(entity),
            attributes: policy.attributes(&entity.attributes, private, false),
            equipped: entity
                .equipped
                .iter()
                .map(|(slot, item)| (slot.clone(), ItemSnapshot::capture(item, private, policy)))
                .collect(),
            inventory: private.then(|| {
                entity
                    .inventory
                    .iter()
                    .map(|item| {
                        item.as_ref()
                            .map(|item| ItemSnapshot::capture(item, true, policy))
                    })
                    .collect()
            }),
            wallet: private.then(|| {
                entity
                    .wallet
                    .balances
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect()
            }),
        }
    }
    pub(crate) fn same_appearance(&self, other: &Self) -> bool {
        self.id == other.id
            && self.creator_id == other.creator_id
            && self.attributes == other.attributes
            && self.equipped == other.equipped
            && self.inventory == other.inventory
            && self.wallet == other.wallet
    }
    pub(crate) fn materialize(&self) -> Entity {
        let mut entity = Entity {
            id: self.id,
            creator_id: self.creator_id,
            position: self.position.into(),
            orientation: self.orientation.into(),
            tilt: self.tilt,
            attributes: attributes(&self.attributes),
            equipped: self
                .equipped
                .iter()
                .map(|(slot, item)| (slot.clone(), item.materialize()))
                .collect(),
            inventory: self
                .inventory
                .as_ref()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| item.as_ref().map(ItemSnapshot::materialize))
                        .collect()
                })
                .unwrap_or_default(),
            ..Entity::default()
        };
        entity.wallet.balances = self
            .wallet
            .as_ref()
            .map(|wallet| wallet.iter().map(|(k, v)| (k.clone(), *v)).collect())
            .unwrap_or_default();
        entity
            .attributes
            .set("__motion_revision", Value::UInt(self.motion_revision));
        entity.clear_dirty();
        entity
    }
}

pub(crate) fn attributes(values: &BTreeMap<String, Value>) -> ValueContainer {
    let mut result = ValueContainer::default();
    for (key, value) in values {
        result.set(key, value.clone());
    }
    result
}

/// Motion changes preserve the last fully validated appearance/private state.
/// Enters and permission/attribute/inventory changes still use replacements.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityPose {
    pub id: u32,
    pub position: [f32; 3],
    pub orientation: [f32; 2],
    pub tilt: f32,
    pub motion_revision: u32,
}
impl EntityPose {
    pub(crate) fn capture(entity: &EntitySnapshot) -> Self {
        Self {
            id: entity.id,
            position: entity.position,
            orientation: entity.orientation,
            tilt: entity.tilt,
            motion_revision: entity.motion_revision,
        }
    }
    pub(crate) fn apply(&self, entity: &mut Entity) {
        entity.position = self.position.into();
        entity.orientation = self.orientation.into();
        entity.tilt = self.tilt;
        if entity.attributes.get_uint("__motion_revision") != Some(self.motion_revision) {
            entity
                .attributes
                .set("__motion_revision", Value::UInt(self.motion_revision));
        }
    }
}
