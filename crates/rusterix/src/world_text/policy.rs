//! Shared CPU world-anchor and private-message policy. No raster drawing.
use super::{TextAnchor, WorldText};
use crate::{Assets, Map};
use theframework_core::TheTime;
use vek::Vec3;

/// Resolve only an explicitly addressed local-player description. Ordinary
/// broadcasts, empty text and another actor's private output cannot become HUD.
pub fn resolve_for_player(
    map: &Map,
    assets: &Assets,
    time: &TheTime,
    receiver: u32,
    message: WorldText,
) -> Option<WorldText> {
    resolve_for_player_view(
        map,
        assets,
        time,
        receiver,
        message,
        crate::party::PlayerView::Legacy,
    )
}

pub fn resolve_for_player_view(
    map: &Map,
    assets: &Assets,
    time: &TheTime,
    receiver: u32,
    mut message: WorldText,
    player: crate::party::PlayerView,
) -> Option<WorldText> {
    if player.entity_id(map) != Some(receiver) || message.text.trim().is_empty() {
        return None;
    }
    let (sender_entity, sender_item) = match message.anchor {
        TextAnchor::Entity(id) => (Some(id), None),
        TextAnchor::Item(id) => (None, Some(id)),
        _ => (None, None),
    };
    message.text = crate::message_text::resolver::MsgResolver::new().resolve_with_context(
        crate::message_text::parser::MsgParser::new().parse(&message.text),
        map,
        assets,
        crate::message_text::resolver::MessageContext {
            sender_entity,
            sender_item,
            receiver_entity: Some(receiver),
            world_time: Some(*time),
        },
    );
    Some(message)
}

pub fn anchor_position(anchor: &TextAnchor, map: &Map, assets: &Assets) -> Option<Vec3<f32>> {
    match anchor {
        TextAnchor::Position(p) => Some(*p),
        TextAnchor::Entity(id) => map.entities.iter().find(|e| e.id == *id).map(|e| {
            e.position
                + Vec3::new(
                    0.,
                    e.attributes.get_float_default("height", 1.8).max(0.1),
                    0.,
                )
        }),
        TextAnchor::Item(id) => map
            .items
            .iter()
            .find(|i| i.id == *id)
            .map(|item| {
                if let Some(a) = crate::procedural_avatar::Runtime::resolve_item(item, assets) {
                    let s = item.attributes.get_float_default("appearance_scale", 1.);
                    let yaw = item
                        .attributes
                        .get_float_default("appearance_yaw", 0.)
                        .to_radians();
                    let x = (a.bounds.0[0] + a.bounds.1[0]) * 0.5 * s;
                    let z = (a.bounds.0[2] + a.bounds.1[2]) * 0.5 * s;
                    item.position
                        + Vec3::new(
                            x * yaw.cos() + z * yaw.sin(),
                            a.bounds.1[1] * s + 0.12,
                            -x * yaw.sin() + z * yaw.cos(),
                        )
                } else {
                    item.position
                        + Vec3::new(
                            0.,
                            item.attributes.get_float_default("height", 1.).max(0.1),
                            0.,
                        )
                }
            })
            .or_else(|| carrier(anchor, map).map(|e| e.position + Vec3::new(0., 1.8, 0.))),
        TextAnchor::Prefab(id) => map
            .block_prop_instances
            .iter()
            .find(|i| i.id == *id)
            .and_then(|i| {
                let asset = assets.block_props.get(&i.asset_id)?;
                let part = asset.parts.first()?;
                crate::block_prop_part_world_anchor(asset, i, part.id)
                    .map(|p| p + Vec3::new(0., 1., 0.))
            }),
    }
}

pub fn carrier<'a>(anchor: &TextAnchor, map: &'a Map) -> Option<&'a crate::Entity> {
    let TextAnchor::Item(id) = anchor else {
        return None;
    };
    map.entities
        .iter()
        .find(|e| e.get_item(*id).is_some() || e.equipped.values().any(|i| i.id == *id))
}
