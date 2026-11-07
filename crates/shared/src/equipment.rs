//! Project-aware equipment authoring uses the same policy and template lookup as gameplay.
use crate::{project::Project, rulesets::ResolvedEquipmentPolicy};
use rusterix::{Assets, Entity};
use std::collections::{BTreeMap, BTreeSet};
use theframework::prelude::Uuid;

#[derive(Clone, Debug)]
pub struct EquipmentItem {
    pub id: Uuid,
    pub reference: String,
    pub name: String,
    pub slot: String,
    kind: String,
    category: Option<String>,
    occupied: BTreeSet<String>,
}

pub struct EquipmentCatalog {
    resolver: Assets,
    pub items: BTreeMap<String, EquipmentItem>,
    policy: ResolvedEquipmentPolicy,
}
impl EquipmentCatalog {
    pub fn new(project: &Project) -> Result<Self, String> {
        let mut resolver = Assets::new();
        resolver.rules = project.rules_source()?;
        resolver.item_template_ids = project
            .items
            .values()
            .map(|i| (i.id, i.name.clone()))
            .collect();
        resolver.items = project
            .items
            .values()
            .map(|i| (i.name.clone(), (i.source.clone(), i.data.clone())))
            .collect();
        let rules = resolver.rules_table().ok_or("Invalid equipment rules")?;
        let policy = crate::rulesets::resolve_equipment_policy(&rules)?;
        let mut items = BTreeMap::new();
        for item in project.items.values() {
            let mut runtime = rusterix::Item::default();
            rusterix::server::data::apply_item_data(&mut runtime, &item.data);
            let slot = runtime
                .attributes
                .get_str("slot")
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            let kind = rusterix::server::region::equipment_item_kind(&policy, &runtime);
            let category = runtime.attributes.get_str("category").map(str::to_owned);
            let occupied = policy.occupied_slots(&kind, category.as_deref(), &slot);
            let reference = item.id.to_string();
            items.insert(
                item.name.clone(),
                EquipmentItem {
                    id: item.id,
                    reference,
                    name: item.name.clone(),
                    slot,
                    kind,
                    category,
                    occupied,
                },
            );
        }
        Ok(Self {
            resolver,
            items,
            policy,
        })
    }
    pub fn find(&self, reference: &str) -> Result<&EquipmentItem, String> {
        let name = self
            .resolver
            .resolve_item_class_name(reference)
            .ok_or_else(|| format!("Unknown equipment item: {reference}"))?;
        self.items
            .get(&name)
            .ok_or_else(|| format!("Missing equipment item: {reference}"))
    }
    pub fn item(&self, id: Uuid) -> Option<&EquipmentItem> {
        self.items.values().find(|i| i.id == id)
    }
    fn character(&self, data: &str) -> Entity {
        let mut entity = Entity::default();
        rusterix::server::data::apply_entity_data(&mut entity, data);
        if let Some(rules) = self.resolver.rules_table() {
            rusterix::server::region::apply_ruleset_character_defaults(&rules, &mut entity);
        }
        entity
    }
    pub fn validate_references(&self, data: &str, references: &[String]) -> Result<(), String> {
        let entity = self.character(data);
        let mut equipped: Vec<&EquipmentItem> = vec![];
        for reference in references.iter().filter(|s| !s.trim().is_empty()) {
            let item = self.find(reference)?;
            if item.slot.is_empty() {
                return Err(format!("{} has no equipment slot.", item.name));
            }
            self.policy
                .check_item(
                    entity.attributes.get_str("class"),
                    &item.kind,
                    item.category.as_deref(),
                    &item.slot,
                )
                .map_err(|error| {
                    format!(
                        "{} ({}): {error}",
                        item.name,
                        entity.attributes.get_str("class").unwrap_or("Character")
                    )
                })?;
            for other in &equipped {
                if other.id == item.id {
                    return Err(format!("{} is listed more than once.", item.name));
                }
                if let Some(slot) = item.occupied.intersection(&other.occupied).next() {
                    return Err(format!(
                        "{} conflicts with {} ({}).",
                        item.name,
                        other.name,
                        slot.replace('_', " ")
                    ));
                }
            }
            equipped.push(item);
        }
        Ok(())
    }
    /// Missing explicit equipment inherits the class outfit. Empty lists intentionally equip nothing.
    pub fn validate_explicit(&self, data: &str) -> Result<(), String> {
        let table = data.parse::<toml::Table>().map_err(|e| e.to_string())?;
        let Some(value) = table
            .get("attributes")
            .and_then(|a| a.get("start_equipped_items"))
            .or_else(|| table.get("start_equipped_items"))
        else {
            return Ok(());
        };
        let references = if let Some(array) = value.as_array() {
            array
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or("Equipment entries must be item names or IDs".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?
        } else if let Some(name) = value.as_str() {
            vec![name.into()]
        } else {
            return Err("Equipment must be a list of item names or IDs".into());
        };
        self.validate_references(data, &references)
    }
    /// A same-slot replacement is atomic; cross-slot conflicts require an explicit removal.
    pub fn replace_for_drop(
        &self,
        data: &str,
        existing: &[String],
        id: Uuid,
    ) -> Result<Vec<String>, String> {
        let incoming = self.item(id).ok_or("The dropped item was removed")?;
        let mut next = vec![];
        let mut inserted = false;
        for reference in existing {
            if reference.trim().is_empty() {
                continue;
            }
            let old = self.find(reference)?;
            if old.slot == incoming.slot {
                if !inserted {
                    next.push(if old.id == incoming.id {
                        reference.clone()
                    } else {
                        incoming.reference.clone()
                    });
                    inserted = true;
                }
            } else {
                next.push(reference.clone());
            }
        }
        if !inserted {
            next.push(incoming.reference.clone());
        }
        self.validate_references(data, &next)?;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Project {
        let mut p = Project::new();
        p.sync_ruleset_items().unwrap();
        p
    }
    fn id(p: &Project, name: &str) -> Uuid {
        p.items.values().find(|i| i.name == name).unwrap().id
    }
    #[test]
    fn authoring_replacements_and_conflicts_follow_runtime_policy() {
        let p = fixture();
        let catalog = EquipmentCatalog::new(&p).unwrap();
        let warrior = "[attributes]\nrace='Human'\nclass='Warrior'\n";
        let initial = vec![
            "Linen Shirt".into(),
            "Training Sword".into(),
            "Round Shield".into(),
        ];
        let shirt = id(&p, "Linen Shirt");
        assert_eq!(
            catalog.replace_for_drop(warrior, &initial, shirt).unwrap(),
            initial
        );
        let replacement = catalog
            .replace_for_drop(warrior, &initial, id(&p, "Chain Shirt"))
            .unwrap();
        assert_eq!(replacement.len(), 3);
        assert_eq!(catalog.find(&replacement[0]).unwrap().name, "Chain Shirt");
        assert_eq!(&replacement[1..], &initial[1..]);
        let error = catalog
            .replace_for_drop(warrior, &initial, id(&p, "Hunting Bow"))
            .unwrap_err();
        assert!(
            error.contains("Round Shield") && error.contains("off hand"),
            "{error}"
        );
        let cleric = "[attributes]\nclass='Cleric'\n";
        let error = catalog
            .replace_for_drop(cleric, &[], id(&p, "Training Sword"))
            .unwrap_err();
        assert!(
            error.contains("Training Sword") && error.to_lowercase().contains("cleric"),
            "{error}"
        );
    }
    #[test]
    fn custom_ids_survive_renames_and_invalid_drafts_keep_last_valid_setup() {
        let mut p = fixture();
        let mut shirt = crate::item::Item::new();
        shirt.name = "Custom shirt".into();
        shirt.data = "[attributes]\nslot='torso'\ncategory='cloth'\n".into();
        let item = shirt.id;
        p.items.insert(item, shirt);
        let mut character = crate::character::Character::new();
        character.data =
            format!("[attributes]\nclass='Warrior'\nstart_equipped_items=['{item}']\n");
        let owner = crate::entity_graph::character_key(character.id);
        let character_id = character.id;
        p.characters.insert(character_id, character);
        crate::entity_graph::synchronize(&mut p).unwrap();
        p.items.get_mut(&item).unwrap().name = "Renamed shirt".into();
        let catalog = EquipmentCatalog::new(&p).unwrap();
        assert_eq!(
            catalog.find(&item.to_string()).unwrap().name,
            "Renamed shirt"
        );
        let mut display = Entity::default();
        rusterix::server::data::apply_entity_data(&mut display, &p.characters[&character_id].data);
        let equipped = rusterix::server::data::character_for_display(&display, &catalog.resolver);
        assert_eq!(equipped.equipped["torso"].creator_id, item);
        let previous = p.characters[&character_id].data.clone();
        let mut doc: theframework::thegraph::GraphDocument =
            serde_json::from_value(p.node_graphs[&owner].clone()).unwrap();
        let node = doc
            .nodes
            .iter_mut()
            .find(|n| n.definition.as_deref() == Some("entity_equipment"))
            .unwrap();
        let row = node
            .rows
            .iter_mut()
            .find(|r| r.key.as_deref() == Some("start_equipped_items"))
            .unwrap();
        if let theframework::thegraph::GraphControlValue::List { rows, .. } = &mut row.value {
            rows[0][0] = theframework::thegraph::GraphControlValue::Text("missing item".into());
        }
        assert!(
            crate::entity_graph::update_owner(&mut p, &owner, &doc)
                .unwrap_err()
                .contains("missing item")
        );
        p.node_graphs
            .insert(owner.clone(), serde_json::to_value(doc).unwrap());
        assert!(
            crate::entity_graph::synchronize(&mut p)
                .unwrap_err()
                .contains("missing item")
        );
        assert_eq!(p.characters[&character_id].data, previous);
        assert!(
            p.node_graphs[&owner].to_string().contains("missing item"),
            "draft remains editable"
        );
    }
}
