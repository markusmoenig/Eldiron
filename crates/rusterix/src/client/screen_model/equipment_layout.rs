//! Ruleset-owned equipment slots and backend-independent panel geometry.
use super::EquipmentPanelConfig;
use crate::{Assets, Entity, Rect};
use vek::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub struct EquipmentPanelSlotLayout {
    pub slot: String,
    pub rect: Rect,
    pub label_rect: Rect,
    pub item_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EquipmentPanelLayout {
    pub rect: Rect,
    pub title_rect: Rect,
    pub close_rect: Rect,
    pub avatar_rect: Rect,
    pub slots: Vec<EquipmentPanelSlotLayout>,
}

pub fn equipment_panel_slot_columns(
    config: &EquipmentPanelConfig,
    assets: &Assets,
) -> (Vec<String>, Vec<String>) {
    let all_slots = assets
        .rules_table()
        .and_then(|root| eldiron_ruleset::resolve_equipment_policy(&root).ok())
        .map(|policy| {
            policy
                .weapon_slots
                .into_iter()
                .chain(policy.armor_slots)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut used = rustc_hash::FxHashSet::default();
    let mut resolve = |configured: &[String]| {
        configured
            .iter()
            .filter_map(|slot| {
                let canonical = all_slots
                    .iter()
                    .find(|known| known.eq_ignore_ascii_case(slot.trim()))?;
                used.insert(canonical.clone()).then(|| canonical.clone())
            })
            .collect::<Vec<_>>()
    };
    let mut left = resolve(&config.left_slots);
    let mut right = resolve(&config.right_slots);
    for slot in all_slots {
        if left
            .iter()
            .chain(right.iter())
            .any(|existing| existing.eq_ignore_ascii_case(&slot))
        {
            continue;
        }
        if left.len() <= right.len() {
            left.push(slot);
        } else {
            right.push(slot);
        }
    }
    (left, right)
}

pub fn equipment_slot_label(slot: &str) -> String {
    slot.split(['_', '-', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_ascii_lowercase()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn equipment_panel_geometry(
    actor: &Entity,
    config: &EquipmentPanelConfig,
    assets: &Assets,
    position: Option<Vec2<i32>>,
    viewport: Vec2<i32>,
) -> EquipmentPanelLayout {
    let (left, right) = equipment_panel_slot_columns(config, assets);
    let row_count = left.len().max(right.len()).max(1);
    let slots_height =
        config.slot_size * row_count as f32 + config.spacing * row_count.saturating_sub(1) as f32;
    let avatar_width = config.avatar_width * config.avatar_scale;
    let avatar_height = config.avatar_height * config.avatar_scale;
    let content_height = slots_height.max(avatar_height);
    let computed_width = config.padding * 2.0
        + config.slot_size * 2.0
        + config.label_width * 2.0
        + config.column_gap * 2.0
        + avatar_width;
    let computed_height = config.title_height + config.padding * 2.0 + content_height;
    let (panel_width, panel_height) = config
        .authored_rect
        .map(|rect| (rect.width.max(1.0), rect.height.max(1.0)))
        .unwrap_or((computed_width, computed_height));
    let viewport_width = viewport.x.max(1) as f32;
    let viewport_height = viewport.y.max(1) as f32;
    let default_x = config
        .authored_rect
        .map(|rect| rect.x)
        .unwrap_or_else(|| ((viewport_width - panel_width) * 0.5).max(2.0));
    let default_y = config
        .authored_rect
        .map(|rect| rect.y)
        .unwrap_or_else(|| ((viewport_height - panel_height) * 0.5).max(2.0));
    let position =
        position.unwrap_or_else(|| Vec2::new(default_x.round() as i32, default_y.round() as i32));
    let x = (position.x as f32).clamp(2.0, (viewport_width - panel_width - 2.0).max(2.0));
    let y = (position.y as f32).clamp(2.0, (viewport_height - panel_height - 2.0).max(2.0));
    let rect = Rect::new(x, y, panel_width, panel_height);
    let title_rect = Rect::new(x, y, panel_width, config.title_height);
    let close_rect = Rect::new(x + panel_width - 30.0, y + 6.0, 22.0, 22.0);
    let content_y = y + config.title_height + config.padding;
    let left_x = x + config.padding;
    let avatar_x = left_x + config.slot_size + config.label_width + config.column_gap;
    let right_label_x = avatar_x + avatar_width + config.column_gap;
    let right_x = right_label_x + config.label_width;
    let avatar_rect = Rect::new(
        avatar_x,
        content_y + (content_height - avatar_height) * 0.5,
        avatar_width,
        avatar_height,
    );
    let mut slots = Vec::with_capacity(left.len() + right.len());
    for (index, slot) in left.iter().enumerate() {
        let row_y = content_y + index as f32 * (config.slot_size + config.spacing);
        slots.push(EquipmentPanelSlotLayout {
            slot: slot.clone(),
            rect: Rect::new(left_x, row_y, config.slot_size, config.slot_size),
            label_rect: Rect::new(
                left_x + config.slot_size + 6.0,
                row_y,
                (config.label_width - 6.0).max(1.0),
                config.slot_size,
            ),
            item_id: actor.get_equipped_item(slot).map(|item| item.id),
        });
    }
    for (index, slot) in right.iter().enumerate() {
        let row_y = content_y + index as f32 * (config.slot_size + config.spacing);
        slots.push(EquipmentPanelSlotLayout {
            slot: slot.clone(),
            rect: Rect::new(right_x, row_y, config.slot_size, config.slot_size),
            label_rect: Rect::new(
                right_label_x,
                row_y,
                (config.label_width - 6.0).max(1.0),
                config.slot_size,
            ),
            item_id: actor.get_equipped_item(slot).map(|item| item.id),
        });
    }
    EquipmentPanelLayout {
        rect,
        title_rect,
        close_rect,
        avatar_rect,
        slots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Item;
    #[test]
    fn equipment_uses_canonical_project_slots_and_deduplicates_configured_aliases() {
        let mut assets = Assets::default();
        assets.rules =
            "[equipment]\nweapon_slots=['main_hand','off_hand']\narmor_slots=['head','body']"
                .into();
        let config = EquipmentPanelConfig {
            left_slots: vec!["MAIN_HAND".into(), "main_hand".into(), "unknown".into()],
            right_slots: vec!["HEAD".into(), "Main_Hand".into()],
            ..Default::default()
        };
        let (left, right) = equipment_panel_slot_columns(&config, &assets);
        assert_eq!(left, vec!["main_hand", "off_hand"]);
        assert_eq!(right, vec!["head", "body"]);
        let mut actor = Entity::new();
        let mut item = Item::new();
        item.id = 7;
        actor.equipped.insert("main_hand".into(), item);
        let config = EquipmentPanelConfig {
            padding: 4.,
            title_height: 20.,
            slot_size: 16.,
            spacing: 2.,
            column_gap: 3.,
            label_width: 24.,
            avatar_width: 30.,
            avatar_height: 50.,
            ..config
        };
        let geometry =
            equipment_panel_geometry(&actor, &config, &assets, None, Vec2::new(200, 140));
        assert_eq!(geometry.rect, Rect::new(38., 31., 124., 78.));
        assert_eq!(geometry.avatar_rect, Rect::new(85., 55., 30., 50.));
        assert_eq!(geometry.slots[0].rect, Rect::new(42., 55., 16., 16.));
        assert_eq!(geometry.slots[0].item_id, Some(7));
        assert_eq!(
            equipment_slot_label("project_DWARF.trinket"),
            "Project Dwarf Trinket"
        );
    }
}
