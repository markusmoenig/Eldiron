//! Item-owned clip/socket selection; all poses and weapon deformation remain MZ data.
use crate::Entity;
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub(super) struct Profile {
    pub idle: String,
    pub walk: String,
    pub attack: String,
    pub joint: String,
    pub two_handed: bool,
}
impl Profile {
    pub fn for_entity(entity: &Entity) -> Self {
        let primary = entity.equipped.get("main_hand");
        let secondary = entity
            .equipped
            .get("tool")
            .or_else(|| entity.equipped.get("off_hand"));
        let Some(item) = primary.or(secondary) else {
            return Self::default();
        };
        let text = |key| {
            primary
                .and_then(|i| i.attributes.get_str(key))
                .filter(|s| !s.trim().is_empty())
                .or_else(|| secondary.and_then(|i| i.attributes.get_str(key)))
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        };
        Self {
            idle: text("appearance_idle"),
            walk: text("appearance_walk"),
            attack: text("appearance_attack"),
            joint: item
                .attributes
                .get_str("appearance_hold_joint")
                .unwrap_or("")
                .trim()
                .to_owned(),
            two_handed: item
                .attributes
                .get_bool_default("appearance_two_handed", false),
        }
    }
}
/// The server visual timer and both renderers use the same authored duration.
pub fn attack_duration(entity: &Entity) -> f32 {
    let seconds = if entity.attributes.get("avatar_attack_time").is_some() {
        entity
            .attributes
            .get_float_default("avatar_attack_time", 0.35)
    } else {
        entity.equipped.get("main_hand").map_or(0.35, |item| {
            item.attributes
                .get_float_default("appearance_attack_time", 0.35)
        })
    };
    if seconds.is_finite() {
        seconds.clamp(0.1, 10.)
    } else {
        0.35
    }
}
