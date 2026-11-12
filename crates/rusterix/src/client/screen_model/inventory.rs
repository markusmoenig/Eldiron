//! Borrowed inventory selection shared by reference and native HUDs.
//! Sorting changes presentation order; absolute inventory indices stay intact.
use crate::{Entity, Item};

pub fn inventory_item_category(item: &Item) -> &'static str {
    let slot = item
        .attributes
        .get_str("slot")
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if matches!(
        slot.as_str(),
        "head"
            | "neck"
            | "torso"
            | "chest"
            | "hands"
            | "belt"
            | "legs"
            | "feet"
            | "boots"
            | "cloak"
            | "main_hand"
            | "off_hand"
            | "shield"
            | "focus"
            | "ammunition"
    ) {
        return "equipment";
    }
    if matches!(slot.as_str(), "material" | "reagent" | "resource") {
        return "materials";
    }

    let category = item
        .attributes
        .get_str("category")
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if ["potion", "food", "drink", "scroll"]
        .iter()
        .any(|value| category.contains(value))
        || item.attributes.get_bool_default("consumable", false)
    {
        return "consumables";
    }
    if [
        "material", "wood", "ore", "dust", "resin", "salt", "shard", "feather", "herb", "liquid",
        "oil", "mineral",
    ]
    .iter()
    .any(|value| category.contains(value))
        || item.attributes.get_str("reagent_for").is_some()
        || item.attributes.get_bool_default("stackable", false)
    {
        return "materials";
    }
    "misc"
}

pub fn inventory_items<'a>(
    actor: &'a Entity,
    category: &str,
    sort: &str,
) -> Vec<(usize, &'a Item)> {
    let mut items = actor
        .inventory
        .iter()
        .enumerate()
        .filter_map(|(index, item)| item.as_ref().map(|item| (index, item)))
        .filter(|(_, item)| {
            category.eq_ignore_ascii_case("all")
                || inventory_item_category(item).eq_ignore_ascii_case(category)
        })
        .collect::<Vec<_>>();

    match sort {
        "name" => items.sort_by(|left, right| {
            let left = left
                .1
                .attributes
                .get_str("name")
                .unwrap_or(&left.1.item_type);
            let right = right
                .1
                .attributes
                .get_str("name")
                .unwrap_or(&right.1.item_type);
            left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
        }),
        "value" => items.sort_by(|left, right| {
            right
                .1
                .attributes
                .get_float_default("worth", 0.0)
                .total_cmp(&left.1.attributes.get_float_default("worth", 0.0))
        }),
        "quantity" => {
            items.sort_by(|left, right| right.1.stack_quantity().cmp(&left.1.stack_quantity()))
        }
        _ => items.sort_by_key(|(index, _)| *index),
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    #[test]
    fn inventory_categories_keep_slot_precedence_and_authored_attribute_fallbacks() {
        let mut item = Item::new();
        item.attributes
            .set("slot", Value::Str(" MAIN_HAND ".into()));
        item.attributes
            .set("category", Value::Str("Healing Potion".into()));
        assert_eq!(inventory_item_category(&item), "equipment");
        item.attributes.set("slot", Value::Str("resource".into()));
        assert_eq!(inventory_item_category(&item), "materials");
        item.attributes.set("slot", Value::Str("".into()));
        assert_eq!(inventory_item_category(&item), "consumables");
        item.attributes.set("category", Value::Str("".into()));
        item.attributes
            .set("reagent_for", Value::Str("ward".into()));
        assert_eq!(inventory_item_category(&item), "materials");
        assert_eq!(inventory_item_category(&Item::new()), "misc");
    }
    #[test]
    fn inventory_order_keeps_sparse_source_indices_stable_ties_and_borrowed_items_without_mutation()
    {
        let mut actor = Entity::new();
        actor.inventory = vec![None; 5];
        for (index, name, worth, quantity, slot) in [
            (0, "Brass", 3., 1, "material"),
            (2, "alpha", 10., 4, "main_hand"),
            (3, "Alpha", 10., 4, "off_hand"),
            (4, "Zinc", 1., 8, "resource"),
        ] {
            let mut item = Item::new();
            item.id = index as u32 + 10;
            item.item_type = name.into();
            item.attributes.set("slot", Value::Str(slot.into()));
            item.attributes.set("worth", Value::Float(worth));
            item.set_stack_quantity(quantity);
            actor.inventory[index] = Some(item);
        }
        let before = serde_json::to_value(&actor).unwrap();
        for (sort, indices) in [
            ("newest", vec![0, 2, 3, 4]),
            ("unrecognized", vec![0, 2, 3, 4]),
            ("name", vec![2, 3, 0, 4]),
            ("value", vec![2, 3, 0, 4]),
            ("quantity", vec![4, 2, 3, 0]),
        ] {
            let items = inventory_items(&actor, "ALL", sort);
            assert_eq!(
                items.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
                indices
            );
            for (index, item) in items {
                assert!(std::ptr::eq(item, actor.inventory[index].as_ref().unwrap()));
            }
        }
        assert_eq!(
            inventory_items(&actor, "MaTeRiAlS", "name")
                .iter()
                .map(|(index, _)| *index)
                .collect::<Vec<_>>(),
            vec![0, 4]
        );
        assert!(inventory_items(&actor, "unknown category", "name").is_empty());
        assert_eq!(serde_json::to_value(&actor).unwrap(), before);
    }
}
