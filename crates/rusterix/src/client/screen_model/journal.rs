//! Search, acquisition explanations and planner-backed inspection shared by HUDs.
use crate::server::region::crafting::preview::{
    RecipeInspector, RecipePreview, target_matches_recipe,
};
use crate::{Assets, Entity, Map, Rect, learning};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use theframework_core::prelude::Uuid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JournalTab {
    #[default]
    Recipes,
    Knowledge,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JournalTarget {
    pub item: u32,
    pub creator: Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    pub key: String,
    pub name: String,
    pub craft: String,
    pub action: Option<String>,
    pub lines: Vec<String>,
    search: String,
}
#[derive(Default)]
pub struct JournalModel {
    source: String,
    source_revision: u64,
    inspector: Option<RecipeInspector>,
    snapshot: Vec<u8>,
    actor: Option<Entity>,
    recipes: Vec<JournalEntry>,
    reports: BTreeMap<(String, Option<JournalTarget>), RecipePreview>,
    sources: BTreeMap<(u32, Uuid), String>,
    knowledge_snapshot: Vec<u8>,
    available: std::collections::BTreeSet<String>,
}
impl JournalModel {
    pub fn bind(
        &mut self,
        assets: &Assets,
        actor: &Entity,
        map: &Map,
        paused: bool,
    ) -> Result<(), String> {
        self.sources.clear();
        for entity in &map.entities {
            if let Some(name) = entity.attributes.get_str("name") {
                self.sources
                    .insert((entity.id, entity.creator_id), name.into());
            }
            for item in entity
                .iter_inventory()
                .map(|(_, item)| item)
                .chain(entity.equipped.values())
            {
                if let Some(name) = item.attributes.get_str("name") {
                    self.sources.insert((item.id, item.creator_id), name.into());
                }
            }
        }
        for item in &map.items {
            if let Some(name) = item.attributes.get_str("name") {
                self.sources.insert((item.id, item.creator_id), name.into());
            }
        }
        if self.inspector.is_none() || self.source != assets.rules {
            self.inspector = Some(RecipeInspector::new(
                assets
                    .rules
                    .parse::<toml::Table>()
                    .map_err(|e| e.to_string())?,
            )?);
            self.source = assets.rules.clone();
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            self.source.hash(&mut hash);
            self.source_revision = hash.finish();
            self.snapshot.clear();
            self.knowledge_snapshot.clear();
            self.recipes.clear();
            let root = self.inspector.as_ref().unwrap().root();
            let mut seen = std::collections::BTreeSet::new();
            if let Some(actions) = root.get("actions").and_then(toml::Value::as_table) {
                for id in actions.keys() {
                    let Some(action) = eldiron_ruleset::resolve_action(root, id)? else {
                        continue;
                    };
                    if action.kind != eldiron_ruleset::ResolvedActionKind::Craft {
                        continue;
                    }
                    let Some(recipe_id) = action.recipe else {
                        continue;
                    };
                    if !seen.insert(recipe_id.clone()) {
                        continue;
                    }
                    let Some(recipe) = root.get("recipes").and_then(|v| v.get(&recipe_id)) else {
                        continue;
                    };
                    let craft = recipe
                        .get("skill")
                        .and_then(toml::Value::as_str)
                        .unwrap_or(if recipe.get("repair").is_some() {
                            "maintenance"
                        } else {
                            "general"
                        })
                        .to_string();
                    let name = recipe
                        .get("name")
                        .and_then(toml::Value::as_str)
                        .unwrap_or(&action.name)
                        .to_string();
                    let ingredients = ["consumes", "produces"]
                        .into_iter()
                        .filter_map(|key| recipe.get(key).and_then(toml::Value::as_array))
                        .flatten()
                        .filter_map(|entry| entry.get("item").and_then(toml::Value::as_str))
                        .filter_map(|id| {
                            root.get("items")
                                .and_then(toml::Value::as_table)
                                .into_iter()
                                .flatten()
                                .find_map(|(_, items)| items.get(id))
                        })
                        .filter_map(|item| item.get("name").and_then(toml::Value::as_str))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let search = format!(
                        "{name} {craft} {recipe} {ingredients} {}",
                        action.description.as_deref().unwrap_or("")
                    )
                    .to_lowercase();
                    self.recipes.push(JournalEntry {
                        key: format!("rules.{id}"),
                        name,
                        craft,
                        action: Some(id.clone()),
                        lines: Vec::new(),
                        search,
                    });
                }
            }
            self.recipes
                .sort_by(|a, b| a.name.cmp(&b.name).then(a.key.cmp(&b.key)));
        }
        let stations = map
            .items
            .iter()
            .filter(|item| item.attributes.get("station_capabilities").is_some())
            .cloned()
            .collect::<Vec<_>>();
        let knowledge_snapshot =
            serde_json::to_vec(&(actor.attributes.get(learning::JOURNAL), &actor.equipped))
                .map_err(|e| e.to_string())?;
        if knowledge_snapshot != self.knowledge_snapshot {
            let rules = assets.value_rules().ok_or("Rules are unavailable")?;
            self.available = rules
                .learning
                .knowledge
                .keys()
                .filter(|id| learning::has_knowledge(&rules, actor, id).unwrap_or(false))
                .cloned()
                .collect();
            self.knowledge_snapshot = knowledge_snapshot;
        }
        let snapshot =
            serde_json::to_vec(&(actor, &stations, paused, map.id)).map_err(|e| e.to_string())?;
        if self.snapshot != snapshot {
            self.snapshot = snapshot;
            self.actor = Some(actor.clone());
            self.reports.clear();
            self.inspector
                .as_mut()
                .unwrap()
                .bind(actor, &stations, paused);
        }
        Ok(())
    }
    pub fn entries(
        &self,
        assets: &Assets,
        tab: JournalTab,
        search: &str,
        craft: &str,
    ) -> Result<Vec<JournalEntry>, String> {
        let Some(actor) = self.actor.as_ref() else {
            return Ok(Vec::new());
        };
        let rules = assets.value_rules().ok_or("Rules are unavailable")?;
        let mut entries = if tab == JournalTab::Recipes {
            self.recipes
                .iter()
                .filter(|entry| {
                    let id = entry
                        .action
                        .as_ref()
                        .and_then(|id| ruleset_recipe(self.inspector.as_ref().unwrap().root(), id));
                    id.is_some_and(|id| {
                        rules.learning.recipe_grants.get(id).is_none_or(|grants| {
                            grants
                                .iter()
                                .any(|knowledge| self.available.contains(knowledge))
                        })
                    })
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            let mut entries = Vec::new();
            let journal = learning::journal(actor)?;
            let mut known = std::collections::BTreeSet::new();
            for acquisition in journal.acquisitions {
                let definition = rules.learning.knowledge.get(&acquisition.knowledge);
                let name = definition
                    .map(|d| d.name.clone())
                    .unwrap_or(acquisition.knowledge.clone());
                let source = match acquisition.origin {
                    learning::Origin::Class { id } => format!("Starting class: {id}"),
                    learning::Origin::Character => "Starting choice".into(),
                    learning::Origin::Teacher {
                        entity,
                        lesson,
                        creator,
                    } => format!(
                        "Teacher: {} · {}",
                        self.sources
                            .get(&(entity, creator))
                            .map(String::as_str)
                            .unwrap_or("a teacher"),
                        rules
                            .learning
                            .teaching
                            .get(&lesson)
                            .map(|lesson| lesson.name.as_str())
                            .unwrap_or("learned lesson")
                    ),
                    learning::Origin::Book {
                        item,
                        lesson,
                        creator,
                    } => format!(
                        "Book: {} · {}",
                        self.sources
                            .get(&(item, creator))
                            .map(String::as_str)
                            .unwrap_or("a studied book"),
                        rules
                            .learning
                            .teaching
                            .get(&lesson)
                            .map(|lesson| lesson.name.as_str())
                            .unwrap_or("learned lesson")
                    ),
                };
                let mut lines = vec!["Permanently learned".into(), source];
                if let Some(definition) = definition {
                    if let Some(description) = &definition.description {
                        lines.push(description.clone());
                    }
                    lines.push(format!(
                        "Recipes: {}",
                        definition
                            .recipes
                            .iter()
                            .map(|id| recipe_name(self.inspector.as_ref().unwrap().root(), id))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                let search = format!("{name} {}", lines.join(" ")).to_lowercase();
                entries.push(JournalEntry {
                    key: format!("knowledge.{}", acquisition.knowledge),
                    name,
                    craft: "permanent".into(),
                    action: None,
                    lines,
                    search,
                });
                known.insert(acquisition.knowledge);
            }
            for (id, definition) in &rules.learning.knowledge {
                if known.contains(id) || !self.available.contains(id) {
                    continue;
                }
                let mut lines=vec!["Temporary equipment knowledge".into(),"Available while the granting equipment is worn and active. Learning this pattern independently makes it permanent.".into()];
                for item in actor.equipped.values() {
                    for instance in crate::effective_values::enchantments(item)? {
                        let Some(enchantment) = rules.enchantments.get(&instance.definition) else {
                            continue;
                        };
                        if enchantment.knowledge_grants.contains(id)
                            && (!enchantment.requires_positive_condition
                                || crate::equipment_lifecycle::condition(item)
                                    .is_some_and(|c| c > 0.))
                        {
                            lines.push(format!(
                                "{}: {}",
                                item.attributes.get_str("name").unwrap_or("Equipment"),
                                enchantment.name
                            ));
                        }
                    }
                }
                lines.push(format!(
                    "Recipes: {}",
                    definition
                        .recipes
                        .iter()
                        .map(|id| recipe_name(self.inspector.as_ref().unwrap().root(), id))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                entries.push(JournalEntry {
                    key: format!("knowledge.{id}"),
                    name: definition.name.clone(),
                    craft: "temporary".into(),
                    action: None,
                    search: format!("{} {}", definition.name, lines.join(" ")).to_lowercase(),
                    lines,
                });
            }
            entries
        };
        let terms = search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>();
        entries.retain(|entry| {
            (craft == "all" || entry.craft == craft)
                && terms.iter().all(|term| entry.search.contains(term))
        });
        entries.sort_by(|a, b| a.name.cmp(&b.name).then(a.key.cmp(&b.key)));
        Ok(entries)
    }
    pub fn crafts(&self) -> Vec<String> {
        let mut crafts = self
            .recipes
            .iter()
            .map(|e| e.craft.clone())
            .collect::<Vec<_>>();
        crafts.sort();
        crafts.dedup();
        crafts.insert(0, "all".into());
        crafts
    }
    pub fn craft_name(&self, id: &str) -> String {
        match id {
            "all" => "All".into(),
            "general" => "General".into(),
            "maintenance" => "Maintenance".into(),
            "permanent" => "Permanent".into(),
            "temporary" => "Equipment".into(),
            _ => self
                .inspector
                .as_ref()
                .and_then(|i| i.root().get("skills").and_then(|v| v.get(id)))
                .and_then(|v| v.get("name"))
                .and_then(toml::Value::as_str)
                .unwrap_or(id)
                .to_string(),
        }
    }
    pub fn targets(&self, action: &str) -> Vec<(JournalTarget, String)> {
        let Some(actor) = self.actor.as_ref() else {
            return Vec::new();
        };
        let Some(inspector) = self.inspector.as_ref() else {
            return Vec::new();
        };
        let Some(recipe) = ruleset_recipe(inspector.root(), action)
            .and_then(|id| inspector.root().get("recipes").and_then(|v| v.get(id)))
            .and_then(toml::Value::as_table)
        else {
            return Vec::new();
        };
        let mut targets = actor
            .iter_inventory()
            .map(|(_, i)| i)
            .chain(actor.equipped.values())
            .filter(|item| target_matches_recipe(inspector.root(), recipe, item).unwrap_or(false))
            .map(|item| {
                (
                    JournalTarget {
                        item: item.id,
                        creator: item.creator_id,
                    },
                    format!(
                        "{} #{} · {}% condition",
                        item.attributes.get_str("name").unwrap_or("Item"),
                        item.id,
                        item.attributes.get_int_default("condition", 100)
                    ),
                )
            })
            .collect::<Vec<_>>();
        targets.sort_by_key(|(target, _)| *target);
        targets
    }
    pub fn targeted(&self, action: &str) -> bool {
        self.inspector
            .as_ref()
            .and_then(|inspector| {
                ruleset_recipe(inspector.root(), action)
                    .and_then(|id| inspector.root().get("recipes").and_then(|v| v.get(id)))
            })
            .is_some_and(|recipe| recipe.get("repair").is_some() || recipe.get("enchant").is_some())
    }
    pub fn operation_name(&self, action: &str) -> &'static str {
        let recipe = self.inspector.as_ref().and_then(|inspector| {
            ruleset_recipe(inspector.root(), action)
                .and_then(|id| inspector.root().get("recipes").and_then(|v| v.get(id)))
        });
        if recipe.is_some_and(|recipe| recipe.get("repair").is_some()) {
            "Repair"
        } else if recipe.is_some_and(|recipe| recipe.get("enchant").is_some()) {
            "Enchant"
        } else {
            "Craft"
        }
    }
    pub fn rules_revision(&self) -> u64 {
        self.source_revision
    }
    pub fn inspect(&mut self, action: &str, target: Option<JournalTarget>) -> RecipePreview {
        let key = (action.to_string(), target);
        if let Some(report) = self.reports.get(&key) {
            return report.clone();
        }
        let valid = target.is_none_or(|target| {
            self.actor.as_ref().is_some_and(|actor| {
                actor
                    .iter_inventory()
                    .map(|(_, i)| i)
                    .chain(actor.equipped.values())
                    .any(|i| i.id == target.item && i.creator_id == target.creator)
            })
        });
        let mut report = self
            .inspector
            .as_mut()
            .map(|inspector| {
                inspector.inspect(action, target.filter(|_| valid).map(|target| target.item))
            })
            .unwrap_or_default();
        if !valid {
            report.ready = false;
            report.reason = Some("Selected item is no longer owned. Choose a target again.".into());
        }
        if self.reports.len() >= 128 {
            self.reports.clear();
        }
        self.reports.insert(key, report.clone());
        report
    }
}
fn ruleset_recipe<'a>(root: &'a toml::Table, action: &str) -> Option<&'a str> {
    root.get("actions")?.get(action)?.get("recipe")?.as_str()
}
fn recipe_name(root: &toml::Table, id: &str) -> String {
    root.get("recipes")
        .and_then(|v| v.get(id))
        .and_then(|v| v.get("name"))
        .and_then(toml::Value::as_str)
        .unwrap_or(id)
        .to_string()
}

/// Inspection keeps readiness and current stocks before background rule text.
pub fn journal_detail_rows(
    description: &crate::client::rules_ui::RulesDescription,
) -> Vec<super::CatalogDetailRow> {
    description
        .lines
        .iter()
        .map(|line| {
            let (label, text) = line
                .split_once(':')
                .map_or((None, line.clone()), |(label, text)| {
                    (Some(label.trim().to_string()), text.trim().to_string())
                });
            let kind = if label.as_deref() == Some("Unavailable") {
                super::CatalogDetailRowKind::Warning
            } else if label.is_some() {
                super::CatalogDetailRowKind::Fact
            } else {
                super::CatalogDetailRowKind::Summary
            };
            super::CatalogDetailRow { label, text, kind }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct JournalLayout {
    pub rect: Rect,
    pub search: Rect,
    pub filter: Rect,
    pub tabs: [Rect; 2],
    pub rows: Vec<Rect>,
    pub detail: Rect,
    pub target: Rect,
    pub craft: Rect,
    pub pin: Rect,
    pub previous: Rect,
    pub next: Rect,
    pub page: usize,
    pub page_count: usize,
    pub start: usize,
}
pub fn journal_geometry(
    size: theframework_core::prelude::Vec2<i32>,
    position: Option<theframework_core::prelude::Vec2<i32>>,
    count: usize,
    page: usize,
) -> JournalLayout {
    let w = 780f32.min((size.x - 4).max(0) as f32);
    let h = 480f32.min((size.y - 4).max(0) as f32);
    let x = position
        .map_or((size.x as f32 - w) / 2., |p| p.x as f32)
        .clamp(2., (size.x as f32 - w - 2.).max(2.));
    let y = position
        .map_or((size.y as f32 - h) / 2., |p| p.y as f32)
        .clamp(2., (size.y as f32 - h - 2.).max(2.));
    let left = (w * 0.42).max(0.);
    let top = y + 94.;
    let bottom = (y + h - 38.).max(top);
    let capacity = (((bottom - top) / 40.).floor() as usize).max(1);
    let page_count = count.div_ceil(capacity).max(1);
    let page = page.min(page_count - 1);
    let start = page * capacity;
    let rows = (start..(start + capacity).min(count))
        .enumerate()
        .map(|(i, _)| Rect::new(x + 8., top + i as f32 * 40., (left - 16.).max(0.), 36.))
        .collect();
    JournalLayout {
        rect: Rect::new(x, y, w, h),
        search: Rect::new(x + 8., y + 60., (w * 0.65 - 12.).max(0.), 26.),
        filter: Rect::new(x + w * 0.65, y + 60., (w * 0.35 - 8.).max(0.), 26.),
        tabs: [
            Rect::new(x + 8., y + 30., ((w - 16.) / 2.).max(0.), 24.),
            Rect::new(x + w / 2., y + 30., ((w - 16.) / 2.).max(0.), 24.),
        ],
        rows,
        detail: Rect::new(
            x + left,
            top,
            (w - left - 8.).max(0.),
            (bottom - top - 100.).max(0.),
        ),
        target: Rect::new(
            x + left,
            (bottom - 92.).max(top),
            (w - left - 8.).max(0.),
            30.,
        ),
        craft: Rect::new(
            x + left,
            (bottom - 54.).max(top),
            ((w - left - 16.) / 2.).max(0.),
            30.,
        ),
        pin: Rect::new(
            x + left + (w - left) / 2.,
            (bottom - 54.).max(top),
            ((w - left - 16.) / 2.).max(0.),
            30.,
        ),
        previous: Rect::new(x + 8., y + h - 30., 30., 22.),
        next: Rect::new(x + left - 38., y + h - 30., 30., 22.),
        page,
        page_count,
        start,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Item, Value};
    #[test]
    fn leather_family_journal_discovers_patterns_and_compatible_equipment_without_class_bar_edits()
    {
        let mut assets = Assets::new();
        assets.rules = eldiron_ruleset::latest_official_ruleset().into();
        let mut actor = Entity::new();
        actor.id = 1;
        actor.inventory = vec![None; 4];
        actor.set_attribute("class", Value::Str("Warrior".into()));
        let rules = assets.value_rules().unwrap();
        let mut model = JournalModel::default();
        model.bind(&assets, &actor, &Map::default(), false).unwrap();
        let entries = model
            .entries(&assets, JournalTab::Recipes, "", "all")
            .unwrap();
        for piece in ["hood", "vest", "leggings", "gloves", "boots"] {
            assert!(
                entries
                    .iter()
                    .any(|e| e.key == format!("rules.craft_make_leather_{piece}"))
            );
        }
        assert!(!entries.iter().any(|e| e.key == "rules.inscribe_fox"));
        learning::acquire(
            &mut actor,
            &rules,
            &["fox_inscription".into(), "leatherworking_patterns".into()],
            learning::Origin::Character,
        )
        .unwrap();
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(&assets.rules).unwrap();
        for (index, id) in ["leather_vest", "training_sword", "raw_bone"]
            .into_iter()
            .enumerate()
        {
            let mut item = Item::new();
            crate::server::data::apply_item_data(
                &mut item,
                &templates.iter().find(|t| t.id == id).unwrap().data,
            );
            item.id = index as u32 + 2;
            actor.add_item(item).unwrap();
        }
        model.bind(&assets, &actor, &Map::default(), false).unwrap();
        let entries = model
            .entries(&assets, JournalTab::Recipes, "", "all")
            .unwrap();
        assert!(entries.iter().any(|e| e.key == "rules.inscribe_fox"));
        assert!(
            entries
                .iter()
                .any(|e| e.key == "rules.craft_cut_straps_batch")
        );
        assert_eq!(
            model.targets("inscribe_fox").len(),
            2,
            "armor and weapons, not raw reagents"
        );
        assert_eq!(model.targets("repair_leather_armor").len(), 1);
    }
    #[test]
    fn journal_search_knowledge_lifetimes_and_exact_targets_follow_current_snapshots() {
        let mut assets = Assets::new();
        assets.rules = eldiron_ruleset::document::RulesState::default().current;
        let mut root = assets.rules.parse::<toml::Table>().unwrap();
        root["items"]["materials"]["raw_bone"]["name"] =
            toml::Value::String("Ancient Ivory".into());
        assets.rules = toml::to_string(&root).unwrap();
        let mut map = Map::default();
        let mut actor = Entity::new();
        actor.id = 1;
        actor.inventory = vec![None; 4];
        actor.set_attribute("skill_boneworking", Value::Int(10));
        let rules = assets.value_rules().unwrap();
        let mut model = JournalModel::default();
        model.bind(&assets, &actor, &map, false).unwrap();
        assert!(
            !model
                .entries(&assets, JournalTab::Recipes, "Ancient Ivory", "all")
                .unwrap()
                .is_empty(),
            "ingredient search uses display names as well as IDs"
        );
        assert!(
            model
                .entries(&assets, JournalTab::Recipes, "batch", "all")
                .unwrap()
                .is_empty()
        );
        let mut guide = Item::new();
        guide.id = 2;
        guide.set_attribute("category", Value::Str("bone_armor".into()));
        guide.set_attribute("name", Value::Str("Ancestral Helm".into()));
        crate::effective_values::attach(&mut guide, &rules, "boneworking_guidance").unwrap();
        actor.equipped.insert("head".into(), guide);
        model.bind(&assets, &actor, &map, false).unwrap();
        assert_eq!(
            model
                .entries(&assets, JournalTab::Recipes, "BATCH bone", "boneworking")
                .unwrap()
                .len(),
            1
        );
        let temporary = model
            .entries(&assets, JournalTab::Knowledge, "Ancestral", "temporary")
            .unwrap();
        assert_eq!(temporary.len(), 1);
        assert!(
            temporary[0]
                .lines
                .iter()
                .any(|line| line.contains("Temporary equipment"))
        );
        assert!(learning::journal(&actor).unwrap().acquisitions.is_empty());
        learning::acquire(
            &mut actor,
            &rules,
            &["boneworking_patterns".into()],
            learning::Origin::Character,
        )
        .unwrap();
        actor.equipped.clear();
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(&assets.rules).unwrap();
        let template = templates.iter().find(|t| t.id == "bone_cuirass").unwrap();
        for id in [3, 4] {
            let mut item = Item::new();
            item.id = id;
            crate::server::data::apply_item_data(&mut item, &template.data);
            actor.add_item(item).unwrap();
        }
        model.bind(&assets, &actor, &map, false).unwrap();
        assert_eq!(
            model
                .entries(&assets, JournalTab::Knowledge, "Advanced", "permanent")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            model
                .entries(&assets, JournalTab::Recipes, "batch", "all")
                .unwrap()
                .len(),
            1
        );
        let targets = model.targets("repair_bone_armor");
        assert_eq!(targets.len(), 2);
        let selected = targets[0].0;
        actor.remove_item_from_slot(actor.get_item_slot(selected.item).unwrap());
        map.entities = vec![actor.clone()];
        model.bind(&assets, &actor, &map, false).unwrap();
        let report = model.inspect("repair_bone_armor", Some(selected));
        assert!(!report.ready);
        assert!(report.reason.unwrap().contains("no longer owned"));
        assert_eq!(
            model.targets("repair_bone_armor").len(),
            1,
            "the remaining piece is never substituted for a selected item"
        );
        let stale = actor.clone();
        let before = serde_json::to_value(&actor).unwrap();
        for _ in 0..3 {
            model.inspect("repair_bone_armor", None);
            model
                .entries(&assets, JournalTab::Recipes, "", "all")
                .unwrap();
        }
        assert_eq!(serde_json::to_value(&actor).unwrap(), before);
        assert_eq!(
            learning::journal(&actor).unwrap(),
            learning::journal(&stale).unwrap()
        );
    }
    #[test]
    fn journal_pagination_and_filtering_remain_bounded_for_large_catalogues() {
        let first = journal_geometry(
            theframework_core::prelude::Vec2::new(960, 540),
            None,
            2000,
            0,
        );
        let last = journal_geometry(
            theframework_core::prelude::Vec2::new(960, 540),
            None,
            2000,
            usize::MAX,
        );
        assert!(first.rows.len() < 20);
        assert!(last.page < last.page_count);
        assert!(last.start < 2000);
        assert!(
            last.rows
                .iter()
                .all(|row| row.y + row.height <= last.rect.y + last.rect.height)
        );
        let small = journal_geometry(theframework_core::prelude::Vec2::new(200, 160), None, 0, 99);
        assert_eq!((small.page, small.page_count), (0, 1));
        assert!(small.rows.is_empty());
    }
}
