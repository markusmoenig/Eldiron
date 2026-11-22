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
            "body_build",
            "body_height",
            "face_style",
            "hair_style",
            "hair_color",
            "hair_index",
            "avatar_hair",
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
            "icon_template",
            "rig_template",
            "icon_avatar",
            "avatar_channels",
            "torso_index",
            "arms_index",
            "legs_index",
            "feet_index",
            "category",
            "ruleset_kind",
            "icon",
            "icon_color",
            "visual_template_width",
            "visual_template_height",
            "visual_template_pixels",
            "blade_color",
            "blade_color_index",
            "grip_color",
            "grip_color_index",
            "accent_color",
            "accent_color_index",
            "highlight_color",
            "highlight_color_index",
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
            item_attributes: visual.iter().copied()
                .chain(["station_capabilities"])
                .map(str::to_string).collect(),
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
            .map(|(key, value)| {
                (
                    key.clone(),
                    match value {
                        Value::Light(light) => {
                            Value::Light(shared::public_geometry::project_light(light))
                        }
                        _ => value.clone(),
                    },
                )
            })
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
        let mut attributes = policy.attributes(&item.attributes, private, true);
        // A deployed station's affordances are public inspection data. Keep
        // arbitrary item properties and container contents under existing policy.
        if !private
            && matches!(attributes.get("station_capabilities"), Some(Value::StrArray(_)))
        {
            if let Some(condition) = item.attributes.get("condition") {
                let condition = match condition {
                    Value::Int(_) | Value::UInt(_) | Value::Int64(_) => condition.clone(),
                    _ => Value::Int(-1),
                };
                attributes.insert("condition".into(), condition);
            }
        }
        Self {
            id: item.id,
            creator_id: item.creator_id,
            position: [item.position.x, item.position.y, item.position.z],
            item_type: item.item_type.clone(),
            max_capacity: item.max_capacity,
            is_container: item.is_container()
                || item.attributes.get_bool_default("container", false)
                || item.attributes.get_int_default("container_slots", 0) > 0,
            attributes,
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

#[cfg(test)]
mod visual_tests {
    use super::*;
    #[test]
    fn journal_replication_exposes_station_usability_without_disclosing_private_item_data() {
        let mut station = Item::new();
        station.set_attribute(
            "station_capabilities",
            Value::StrArray(vec!["bonework".into()]),
        );
        station.set_attribute("condition", Value::Int(73));
        station.set_attribute("secret_recipe_stock", Value::Int(99));
        station.container = Some(vec![Item::new()]);
        let public = ItemSnapshot::capture(&station, false, &ReplicationPolicy::default());
        assert_eq!(public.attributes.get("condition"), Some(&Value::Int(73)));
        assert!(public.attributes.contains_key("station_capabilities"));
        assert!(!public.attributes.contains_key("secret_recipe_stock"));
        assert!(public.contents.is_none());
        let mut hidden = ReplicationPolicy::default();
        hidden.item_attributes.remove("station_capabilities");
        let filtered = ItemSnapshot::capture(&station, false, &hidden);
        assert!(!filtered.attributes.contains_key("station_capabilities"));
        assert!(!filtered.attributes.contains_key("condition"));
        station.attributes.remove("station_capabilities");
        assert!(
            !ItemSnapshot::capture(&station, false, &ReplicationPolicy::default())
                .attributes
                .contains_key("condition")
        );
    }
    #[test]
    fn remote_equipment_keeps_render_inputs_and_hides_private_state() {
        let mut actor = Entity::default();
        actor.set_attribute("body_build", Value::Str("stout".into()));
        actor.set_attribute("body_height", Value::Float(1.73));
        actor.set_attribute("face_style", Value::Str("bearded".into()));
        actor.set_attribute("hair_style", Value::Str("braid".into()));
        actor.set_attribute("hair_color", Value::Str("#a95330".into()));
        let mut item = Item::default();
        item.id = 12;
        item.attributes
            .set("rig_template", Value::Str("armor".into()));
        item.attributes
            .set("avatar_channels", Value::StrArray(vec!["torso".into()]));
        item.attributes.set("blade_color_index", Value::Int(7));
        item.attributes
            .set("private_quest", Value::Str("HIDDEN".into()));
        let mut light = rusterix::Light::new(rusterix::LightType::Point).with_intensity(2.);
        light
            .properties
            .set("private_token", Value::Str("HIDDEN_LIGHT".into()));
        item.attributes.set("light", Value::Light(light));
        actor.equipped.insert("body".into(), item);
        let snapshot = EntitySnapshot::capture(&actor, None, &ReplicationPolicy::default());
        let replica = snapshot.materialize();
        assert_eq!(replica.attributes.get_str("body_build"), Some("stout"));
        assert_eq!(replica.attributes.get_float("body_height"), Some(1.73));
        assert_eq!(replica.attributes.get_str("face_style"), Some("bearded"));
        assert_eq!(replica.attributes.get_str("hair_style"), Some("braid"));
        assert_eq!(replica.attributes.get_str("hair_color"), Some("#a95330"));
        let item = &replica.equipped["body"];
        assert_eq!(item.attributes.get_str("rig_template"), Some("armor"));
        if let Some(Value::Light(light)) = item.attributes.get("light") {
            assert_eq!(light.get_intensity(), 2.);
            assert!(light.properties.get("private_token").is_none());
        } else {
            panic!("public equipment light missing");
        }
        assert_eq!(item.attributes.get_int("blade_color_index"), Some(7));
        assert!(item.attributes.get("avatar_channels").is_some());
        assert!(item.attributes.get("private_quest").is_none());
    }
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
