//! Read-only public inspection runs the real planner in a private scratch world.
//! No scratch actor, output identity or attachment is returned or persisted.
use super::*;

pub fn target_matches_recipe(
    root: &toml::Table,
    recipe: &toml::Table,
    item: &Item,
) -> Result<bool, String> {
    let requirements = eldiron_ruleset::crafting::requirements(recipe)?;
    let Some(id) = item.attributes.get_str("ruleset_id") else {
        return Ok(false);
    };
    if let Some(enchant) = requirements.enchant {
        let registered = ruleset_item_table_by_id(root, id).is_some();
        let definition = root
            .get("enchantments")
            .and_then(|v| v.get(&enchant.definition))
            .ok_or("Unknown enchantment")?;
        let compatible = definition
            .get("allowed_categories")
            .and_then(toml::Value::as_array)
            .is_none_or(|categories| {
                categories.is_empty()
                    || categories
                        .iter()
                        .any(|c| c.as_str() == item.attributes.get_str("category"))
            });
        return Ok(registered
            && (enchant.items.is_empty() || enchant.items.iter().any(|member| member == id))
            && compatible);
    }
    if let Some(repair) = requirements.repair {
        let family = root
            .get("equipment_families")
            .and_then(|v| v.get(&repair.family))
            .ok_or("Unknown repair family")?;
        return Ok(["required_members", "variants"]
            .into_iter()
            .filter_map(|key| family.get(key).and_then(toml::Value::as_array))
            .flatten()
            .any(|member| member.as_str() == Some(id)));
    }
    Ok(false)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ingredient {
    pub id: String,
    pub name: String,
    pub required: usize,
    pub available: usize,
    pub liquid: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecipePreview {
    pub ready: bool,
    pub reason: Option<String>,
    pub ingredients: Vec<Ingredient>,
    pub requirements: Vec<String>,
    pub results: Vec<String>,
}
/// Reuse this object for one rules document; bind a new snapshot when the actor,
/// deployed stations, pause state or baseline changes. The private ID allocator
/// never touches a Server's SimulationWorld and preview state never feeds saves.
pub struct RecipeInspector {
    ctx: RegionCtx,
    actor: Option<Entity>,
}
impl RecipeInspector {
    pub fn new(root: toml::Table) -> Result<Self, String> {
        let mut ctx = RegionCtx::default();
        ctx.set_rules(root)?;
        Ok(Self { ctx, actor: None })
    }
    pub fn bind(&mut self, actor: &Entity, stations: &[Item], paused: bool) {
        self.actor = Some(actor.clone());
        self.ctx.map.entities = vec![actor.clone()];
        self.ctx.map.items = stations.to_vec();
        self.ctx.paused = paused;
        self.ctx.entity_state_data.clear();
        // Replicas expose remaining seconds and condition stacks as actor
        // attributes. Reconstruct only the planner's read-only private state.
        let mut state = crate::ValueContainer::default();
        if let Some(actions) = self
            .ctx
            .rules
            .get("actions")
            .and_then(toml::Value::as_table)
        {
            for id in actions.keys() {
                let suffix = cooldown_attr_suffix("rules", id);
                if let Some(left) = actor
                    .attributes
                    .get_float(&format!("cooldown_left_{suffix}"))
                {
                    state.set(&action_cooldown_key(id), Value::Float(left));
                }
            }
        }
        if let Some(conditions) = self
            .ctx
            .rules
            .get("conditions")
            .and_then(toml::Value::as_table)
        {
            for id in conditions.keys() {
                let suffix = cooldown_attr_suffix("condition", id);
                if let Some(stacks) = actor.attributes.get(&format!("{suffix}_stacks")) {
                    state.set(&condition_stacks_key(id), stacks.clone());
                }
            }
        }
        self.ctx.entity_state_data.insert(actor.id, state);
        fn high(item: &Item) -> u32 {
            item.container
                .iter()
                .flatten()
                .map(high)
                .fold(item.id, u32::max)
        }
        let high = owned(actor)
            .chain(stations.iter())
            .map(high)
            .fold(actor.id, u32::max);
        self.ctx
            .world
            .restore(high.saturating_add(1), Default::default());
    }
    pub fn inspect(&mut self, action_id: &str, target: Option<u32>) -> RecipePreview {
        let mut report = RecipePreview::default();
        let result = (|| {
            let actor = self.actor.as_ref().ok_or("No character selected")?.clone();
            self.ctx.map.entities = vec![actor.clone()];
            let action = self
                .ctx
                .resolved_action(action_id)?
                .ok_or("Unknown crafting action")?;
            if action.kind != ResolvedActionKind::Craft {
                return Err("This is not a crafting action".into());
            }
            let recipe_id = action.recipe.as_deref().ok_or("Action has no recipe")?;
            let recipe = ruleset_recipe_table(&self.ctx, recipe_id).ok_or("Unknown recipe")?;
            let output_count = item_quantity_entries(recipe, "produces")
                .iter()
                .try_fold(0u32, |total, (_, count)| {
                    total.checked_add(u32::try_from(*count).ok()?)
                });
            if output_count
                .and_then(|count| self.ctx.world.high_water().checked_add(count))
                .is_none()
            {
                return Err("Cannot preview additional item identities".into());
            }
            let mut costs = BTreeMap::<String, usize>::new();
            for (id, count) in item_quantity_entries(recipe, "consumes").into_iter().chain(
                action
                    .item_costs
                    .iter()
                    .map(|c| (c.item.clone(), c.quantity)),
            ) {
                let total = costs.entry(id).or_default();
                *total = total
                    .checked_add(count)
                    .ok_or("Input quantity exceeds bounds")?;
            }
            for (id, required) in costs {
                let (_, definition) = ruleset_item_table_by_id(&self.ctx.rules, &id)
                    .ok_or("Missing ingredient definition")?;
                let liquid = rule_string(definition, "quantity_unit") == Some("volume");
                let available = if liquid {
                    owned(&actor)
                        .filter(|item| {
                            let cap = item_number(item, "liquid_capacity", 0);
                            let count = item_number(item, "liquid_quantity", 0);
                            cap > 0
                                && count > 0
                                && count <= cap
                                && item.attributes.get_str("liquid_item") == Some(id.as_str())
                        })
                        .map(|item| item_number(item, "liquid_quantity", 0) as usize)
                        .sum()
                } else {
                    entity_inventory_item_count(&actor, &id)
                };
                report.ingredients.push(Ingredient {
                    id: id.clone(),
                    name: rule_string(definition, "name").unwrap_or(&id).into(),
                    required,
                    available,
                    liquid,
                });
            }
            let requirements = eldiron_ruleset::crafting::requirements(recipe)?;
            if let Some(skill) = rule_string(recipe, "skill") {
                let name = self
                    .ctx
                    .rules
                    .get("skills")
                    .and_then(|skills| skills.get(skill))
                    .and_then(|skill| skill.get("name"))
                    .and_then(toml::Value::as_str)
                    .unwrap_or(skill);
                let current = entity_skill_points(&actor, skill);
                let minimum = rule_number(recipe, "required_skill", 0.).round().max(0.) as i32;
                report.requirements.push(if minimum > 0 {
                    format!("{name}: {current} / {minimum} required")
                } else {
                    format!("{name}: {current} (no minimum)")
                });
            }
            if let Some(spell) = recipe_required_spell(recipe) {
                let name = self
                    .ctx
                    .rules
                    .get("spells")
                    .and_then(|spells| spells.get(spell))
                    .and_then(|spell| spell.get("name"))
                    .and_then(toml::Value::as_str)
                    .unwrap_or(spell);
                report.requirements.push(format!("Spell: {name}"));
            }
            if let Some(repair) = requirements.repair {
                if let Some(item) = target.and_then(|id| owned(&actor).find(|i| i.id == id)) {
                    let condition = item_number(item, "condition", 100);
                    report.results.push(format!(
                        "Condition: {condition} → {}",
                        (condition + repair.amount as i32).min(repair.maximum as i32)
                    ));
                    report.results.push(format!(
                        "Quality stays {}%; item identity and attachments are retained.",
                        item_number(item, "quality", 100)
                    ));
                }
            }
            if let Some(enchant) = requirements.enchant {
                let name = self
                    .ctx
                    .rules
                    .get("enchantments")
                    .and_then(|v| v.get(&enchant.definition))
                    .and_then(|v| v.get("name"))
                    .and_then(toml::Value::as_str)
                    .unwrap_or(&enchant.definition);
                report.results.push(format!("Enchantment: {name}"));
                report.results.extend(self.ctx.with_value_rules(|rules| {
                    let Some(definition) = rules.enchantments.get(&enchant.definition) else {
                        return Vec::new();
                    };
                    let mut lines = Vec::new();
                    for modifier in &definition.modifiers {
                        let mut effects = Vec::new();
                        if modifier.add != 0. {
                            effects.push(format!("{:+}", modifier.add));
                        }
                        if modifier.multiply != 1. {
                            effects.push(format!("×{}", modifier.multiply));
                        }
                        if let Some(min) = modifier.minimum {
                            effects.push(format!("minimum {min}"));
                        }
                        if let Some(max) = modifier.maximum {
                            effects.push(format!("maximum {max}"));
                        }
                        lines.push(format!(
                            "{}: {} while equipped{}",
                            modifier.attribute,
                            effects.join(", "),
                            if definition.requires_positive_condition {
                                " and usable"
                            } else {
                                ""
                            }
                        ));
                    }
                    for knowledge in &definition.knowledge_grants {
                        if let Some(knowledge) = rules.learning.knowledge.get(knowledge) {
                            lines.push(format!(
                                "Temporary knowledge: {} while equipped{}",
                                knowledge.name,
                                if definition.requires_positive_condition {
                                    " and usable"
                                } else {
                                    ""
                                }
                            ));
                        }
                    }
                    lines
                })?);
                if let Some(name) = enchant.name {
                    report.results.push(format!("Result: {name}"));
                } else if let Some(item) = target.and_then(|id| owned(&actor).find(|i| i.id == id))
                {
                    if let Some(name) = self.ctx.with_value_rules(|rules| {
                        crate::effective_values::enchanted_name(item, rules, &enchant.definition)
                    })?? {
                        report.results.push(format!("Result: {name}"));
                    }
                }
                report
                    .results
                    .push("Keeps quality, condition and item identity.".into());
            }
            for (id, count) in item_quantity_entries(recipe, "produces") {
                let name = ruleset_item_table_by_id(&self.ctx.rules, &id)
                    .and_then(|(_, item)| rule_string(item, "name"))
                    .unwrap_or(&id);
                report.results.push(format!(
                    "{count} {name} (quality {}%)",
                    recipe_output_quality(&actor, recipe)
                ));
            }
            let target = target.map(|item_id| RulesetActionTarget::Item {
                item_id,
                owner_entity_id: Some(actor.id),
            });
            let costs = action
                .item_costs
                .iter()
                .map(|cost| (cost.item.clone(), cost.quantity))
                .collect::<Vec<_>>();
            execute_action(
                &mut self.ctx,
                actor.id,
                recipe_id,
                &costs,
                &action,
                target.as_ref(),
            )
        })();
        match result {
            Ok(()) => report.ready = true,
            Err(mut reason) => {
                if reason == "Recipe materials are missing" {
                    if let Some(ingredient) = report
                        .ingredients
                        .iter()
                        .find(|ingredient| ingredient.available < ingredient.required)
                    {
                        reason = format!(
                            "Need {} more {}",
                            ingredient.required - ingredient.available,
                            ingredient.name
                        );
                    }
                } else {
                    for ingredient in &report.ingredients {
                        reason = reason.replace(&ingredient.id, &ingredient.name);
                    }
                }
                report.reason = Some(reason);
            }
        }
        report
    }
    pub fn root(&self) -> &toml::Table {
        &self.ctx.rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_pins_known_recipes_without_a_class_bar_and_keeps_other_commands_restricted() {
        let mut ctx = RegionCtx::default();
        ctx.set_rules(
            eldiron_ruleset::document::RulesState::default()
                .document()
                .unwrap()
                .table()
                .clone(),
        )
        .unwrap();
        let mut actor = Entity::new();
        actor.id = 1;
        actor.set_attribute("player", Value::Bool(true));
        actor.set_attribute("class", Value::Str("Custom sandbox identity".into()));
        ctx.map.entities.push(actor);
        assert!(!set_command_slot_for_entity(
            &mut ctx,
            1,
            "main.0",
            Some("rules.inscribe_bear_cuirass")
        ));
        let values = ctx.with_value_rules(Clone::clone).unwrap();
        crate::learning::acquire(
            &mut ctx.map.entities[0],
            &values,
            &["bear_inscription".into()],
            crate::learning::Origin::Character,
        )
        .unwrap();
        assert!(set_command_slot_for_entity(
            &mut ctx,
            1,
            "main.0",
            Some("rules.inscribe_bear_cuirass")
        ));
        assert_eq!(
            ctx.map.entities[0]
                .attributes
                .get_str("command_slot_main_0"),
            Some("rules.inscribe_bear_cuirass")
        );
        assert!(!set_command_slot_for_entity(
            &mut ctx,
            1,
            "main.0",
            Some("rules.basic_attack")
        ));
        assert!(!set_command_slot_for_entity(
            &mut ctx,
            1,
            "main.0",
            Some("rules.missing")
        ));
    }
    #[test]
    fn journal_inspection_matches_real_crafting_without_touching_the_source_snapshot_or_id_allocator()
     {
        for failure in [
            "ready",
            "knowledge",
            "skill",
            "water",
            "materials",
            "tool",
            "station",
            "space",
            "paused",
            "cooldown",
        ] {
            let mut ctx = RegionCtx::default();
            ctx.set_rules(
                eldiron_ruleset::document::RulesState::default()
                    .document()
                    .unwrap()
                    .table()
                    .clone(),
            )
            .unwrap();
            let mut actor = Entity::new();
            actor.id = ctx.world.next_id();
            actor.inventory = vec![None; 8];
            actor.set_attribute("skill_boneworking", Value::Int(10));
            let values = ctx.with_value_rules(Clone::clone).unwrap();
            crate::learning::acquire(
                &mut actor,
                &values,
                &["boneworking_patterns".into()],
                crate::learning::Origin::Character,
            )
            .unwrap();
            for (id, count) in [("raw_bone", 3), ("wooden_bowl", 1), ("stone_knife", 1)] {
                let mut item = ruleset_item_from_table(&ctx.rules, id, count, &ctx.world).unwrap();
                if id == "wooden_bowl" {
                    item.set_attribute("liquid_item", Value::Str("fresh_water".into()));
                    item.set_attribute("liquid_quantity", Value::Int(3));
                }
                actor.add_item(item).unwrap();
            }
            let mut bench =
                ruleset_item_from_table(&ctx.rules, "bone_workbench", 1, &ctx.world).unwrap();
            bench.position = actor.position;
            ctx.map.items.push(bench);
            match failure {
                "knowledge" => {
                    actor.attributes.remove(crate::learning::JOURNAL);
                }
                "skill" => actor.set_attribute("skill_boneworking", Value::Int(0)),
                "water" => actor.inventory[1]
                    .as_mut()
                    .unwrap()
                    .set_attribute("liquid_quantity", Value::Int(2)),
                "materials" => actor.inventory[0].as_mut().unwrap().set_stack_quantity(2),
                "tool" => actor.inventory[2]
                    .as_mut()
                    .unwrap()
                    .set_attribute("condition", Value::Int(0)),
                "station" => ctx.map.items[0].position.x += 10.,
                "space" => {
                    actor.inventory[0].as_mut().unwrap().set_stack_quantity(4);
                    for slot in 3..8 {
                        actor.inventory[slot] = Some(
                            ruleset_item_from_table(&ctx.rules, "stone_knife", 1, &ctx.world)
                                .unwrap(),
                        );
                    }
                }
                "paused" => ctx.paused = true,
                "cooldown" => {
                    actor.set_attribute(
                        "cooldown_left_rules_craft_shape_bone_batch",
                        Value::Float(2.),
                    );
                    let mut state = crate::ValueContainer::default();
                    state.set(
                        &action_cooldown_key("craft_shape_bone_batch"),
                        Value::Float(2.),
                    );
                    ctx.entity_state_data.insert(actor.id, state);
                }
                _ => {}
            }
            let id = actor.id;
            ctx.map.entities.push(actor.clone());
            let before = serde_json::to_value((&ctx.map.entities, &ctx.map.items)).unwrap();
            let high = ctx.world.high_water();
            let mut inspector = RecipeInspector::new(ctx.rules.clone()).unwrap();
            inspector.bind(&actor, &ctx.map.items, ctx.paused);
            let first = inspector.inspect("craft_shape_bone_batch", None);
            assert_eq!(
                first.ready,
                failure == "ready",
                "{failure}: {:?}",
                first.reason
            );
            if failure == "skill" {
                assert!(first.reason.as_deref().unwrap().contains("10"));
                assert!(
                    first
                        .requirements
                        .iter()
                        .any(|line| line.contains("0 / 10"))
                );
            }
            assert_eq!(inspector.inspect("craft_shape_bone_batch", None), first);
            assert_eq!(
                serde_json::to_value((&ctx.map.entities, &ctx.map.items)).unwrap(),
                before
            );
            assert_eq!(
                ctx.world.high_water(),
                high,
                "preview must never allocate authoritative IDs"
            );
            let action = ctx
                .resolved_action("craft_shape_bone_batch")
                .unwrap()
                .unwrap();
            assert_eq!(
                execute_action(&mut ctx, id, "shape_bone_batch", &[], &action, None).is_ok(),
                first.ready,
                "{failure}"
            );
        }
    }
}
