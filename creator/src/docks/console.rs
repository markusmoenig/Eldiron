use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::{Entity, Item, Value, server::ServerState};
use theframework::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsoleFocus {
    Root,
    Entity(u32),
    Item(u32),
}

pub struct ConsoleDock {
    transcript: String,
    focus: ConsoleFocus,
}

#[derive(Clone)]
struct RuntimeEntity {
    entity: Entity,
}

#[derive(Clone)]
struct RuntimeItem {
    item: Item,
}

impl ConsoleDock {
    fn console_input_id(ui: &mut TheUI) -> Option<TheId> {
        ui.get_widget("Console Input")
            .map(|widget| widget.id().clone())
    }

    fn set_output(&mut self, text: String, ui: &mut TheUI, ctx: &mut TheContext) {
        self.transcript = text;
        self.sync_output(ui, ctx);
    }

    fn sync_output(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        ui.set_widget_value(
            "Console Output",
            ctx,
            TheValue::Text(self.transcript.clone()),
        );
    }

    fn set_input(&self, ui: &mut TheUI, ctx: &mut TheContext, text: &str) {
        ui.set_widget_value("Console Input", ctx, TheValue::Text(text.to_string()));
    }

    fn clear_input(&self, ui: &mut TheUI) {
        if let Some(widget) = ui.get_widget("Console Input")
            && let Some(edit) = widget.as_text_line_edit()
        {
            edit.set_text(String::new());
        }
    }

    fn prompt(&self, project: &Project, server_ctx: &ServerContext) -> String {
        let region_name = project
            .get_region_ctx(server_ctx)
            .map(|region| region.name.clone())
            .unwrap_or_else(|| "Region".to_string());
        match self.focus {
            ConsoleFocus::Root => region_name,
            ConsoleFocus::Entity(id) => {
                let name = Self::runtime_snapshot(project, server_ctx)
                    .ok()
                    .and_then(|(entities, _)| {
                        entities
                            .iter()
                            .find(|entity| entity.entity.id == id)
                            .map(|entity| Self::entity_name(&entity.entity))
                    })
                    .unwrap_or_else(|| "Character".to_string());
                format!("{} / {}", region_name, name)
            }
            ConsoleFocus::Item(id) => {
                let name = Self::runtime_snapshot(project, server_ctx)
                    .ok()
                    .and_then(|(_, items)| {
                        items
                            .iter()
                            .find(|item| item.item.id == id)
                            .map(|item| Self::item_name(&item.item))
                    })
                    .unwrap_or_else(|| "Item".to_string());
                format!("{} / {}", region_name, name)
            }
        }
    }

    fn entity_name(entity: &Entity) -> String {
        entity
            .get_attr_string("name")
            .unwrap_or_else(|| format!("Entity {}", entity.id))
    }

    fn item_name(item: &Item) -> String {
        item.get_attr_string("name")
            .unwrap_or_else(|| format!("Item {}", item.id))
    }

    fn quoted(text: &str) -> String {
        format!("\"{}\"", text.replace('"', "'"))
    }

    fn format_value(value: &Value) -> String {
        value.to_string()
    }

    fn intro() -> String {
        [
            "Console ready.",
            "Commands: help, list, focus <name|id>, show, get <key>, rules <query>, pwd, up, clear",
            "When the game is running, `list` shows live characters and items for the current editor region.",
        ]
        .join("\n")
    }

    fn parse_id(text: &str) -> Option<u32> {
        text.trim().parse::<u32>().ok()
    }

    fn entity_matches(entity: &Entity, needle: &str) -> bool {
        entity.id.to_string() == needle || Self::entity_name(entity).eq_ignore_ascii_case(needle)
    }

    fn item_matches(item: &Item, needle: &str) -> bool {
        item.id.to_string() == needle || Self::item_name(item).eq_ignore_ascii_case(needle)
    }

    fn collect_nested_items(items: &[Item], out: &mut Vec<RuntimeItem>) {
        for item in items {
            out.push(RuntimeItem { item: item.clone() });
            if let Some(container) = &item.container {
                Self::collect_nested_items(container, out);
            }
        }
    }

    fn collect_nested_items_from_entity(entity: &Entity, out: &mut Vec<RuntimeItem>) {
        for item in entity.inventory.iter().flatten() {
            out.push(RuntimeItem { item: item.clone() });
            if let Some(container) = &item.container {
                Self::collect_nested_items(container, out);
            }
        }
        for item in entity.equipped.values() {
            out.push(RuntimeItem { item: item.clone() });
            if let Some(container) = &item.container {
                Self::collect_nested_items(container, out);
            }
        }
    }

    fn collect_focusable_items(
        entities: &[RuntimeEntity],
        items: &[RuntimeItem],
    ) -> Vec<RuntimeItem> {
        let mut collected = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for item in items {
            if seen.insert(item.item.id) {
                collected.push(item.clone());
            }
            if let Some(container) = &item.item.container {
                let mut nested = Vec::new();
                Self::collect_nested_items(container, &mut nested);
                for child in nested {
                    if seen.insert(child.item.id) {
                        collected.push(child);
                    }
                }
            }
        }

        for entity in entities {
            let mut nested = Vec::new();
            Self::collect_nested_items_from_entity(&entity.entity, &mut nested);
            for child in nested {
                if seen.insert(child.item.id) {
                    collected.push(child);
                }
            }
        }

        collected
    }

    fn focused_entity<'a>(&self, entities: &'a [RuntimeEntity]) -> Option<&'a RuntimeEntity> {
        match self.focus {
            ConsoleFocus::Entity(id) => entities.iter().find(|entity| entity.entity.id == id),
            _ => None,
        }
    }

    fn focused_item<'a>(&self, items: &'a [RuntimeItem]) -> Option<&'a RuntimeItem> {
        match self.focus {
            ConsoleFocus::Item(id) => items.iter().find(|item| item.item.id == id),
            _ => None,
        }
    }

    fn pad(value: &str, width: usize) -> String {
        let mut out = String::new();
        let mut count = 0usize;
        for ch in value.chars() {
            if count >= width {
                break;
            }
            out.push(ch);
            count += 1;
        }
        while count < width {
            out.push(' ');
            count += 1;
        }
        out
    }

    fn entry_cell(name: &str, id: u32, width: usize) -> String {
        let label = Self::quoted(name);
        let left = Self::pad(&label, width.saturating_sub(8));
        format!("{} {:>6}", left, id)
    }

    fn push_item_tree(lines: &mut Vec<String>, item: &Item, depth: usize) {
        let indent = "\t".repeat(depth);
        lines.push(format!(
            "{}{} {}",
            indent,
            Self::quoted(&Self::item_name(item)),
            item.id
        ));

        if let Some(container) = &item.container {
            for child in container {
                Self::push_item_tree(lines, child, depth + 1);
            }
        }
    }

    fn push_equipped_tree(lines: &mut Vec<String>, slot: &str, item: &Item) {
        lines.push(format!(
            "{} = {} {}",
            slot,
            Self::quoted(&Self::item_name(item)),
            item.id
        ));
        if let Some(container) = &item.container {
            for child in container {
                Self::push_item_tree(lines, child, 1);
            }
        }
    }

    fn pair_row(
        left: Option<String>,
        right: Option<String>,
        width: usize,
        separator: &str,
    ) -> String {
        format!(
            "{}{}{}",
            Self::pad(left.as_deref().unwrap_or(""), width),
            separator,
            right.unwrap_or_default()
        )
    }

    fn triple_row(
        left: Option<String>,
        middle: Option<String>,
        right: Option<String>,
        width: usize,
        separator: &str,
    ) -> String {
        format!(
            "{}{}{}{}{}",
            Self::pad(left.as_deref().unwrap_or(""), width),
            separator,
            Self::pad(middle.as_deref().unwrap_or(""), width),
            separator,
            right.unwrap_or_default()
        )
    }

    fn list_root(&self, entities: &[RuntimeEntity], items: &[RuntimeItem]) -> String {
        let column_width = 38usize;
        let separator = " | ";
        let mut lines = vec![
            Self::pair_row(
                Some(format!("Characters ({})", entities.len())),
                Some(format!("Items ({})", items.len())),
                column_width,
                separator,
            ),
            Self::pair_row(
                Some("Name                               Id".to_string()),
                Some("Name                               Id".to_string()),
                column_width,
                separator,
            ),
        ];

        let row_count = entities.len().max(items.len()).max(1);
        for index in 0..row_count {
            let left = entities.get(index).map(|entity| {
                Self::entry_cell(
                    &Self::entity_name(&entity.entity),
                    entity.entity.id,
                    column_width,
                )
            });
            let right = items.get(index).map(|item| {
                Self::entry_cell(&Self::item_name(&item.item), item.item.id, column_width)
            });
            lines.push(Self::pair_row(left, right, column_width, separator));
        }
        lines.join("\n")
    }

    fn list_entity(&self, entity: &RuntimeEntity) -> String {
        let mut lines = vec![
            format!(
                "Character {} {}",
                Self::quoted(&Self::entity_name(&entity.entity)),
                entity.entity.id
            ),
            format!(
                "position = [{:.2}, {:.2}, {:.2}]",
                entity.entity.position.x, entity.entity.position.y, entity.entity.position.z
            ),
            format!(
                "orientation = [{:.2}, {:.2}]",
                entity.entity.orientation.x, entity.entity.orientation.y
            ),
        ];

        let mut attr_lines = Vec::new();
        let keys = entity.entity.attributes.keys_sorted();
        if keys.is_empty() {
            attr_lines.push("<none>".to_string());
        } else {
            for key in keys {
                if key == "setup" || key == "_source_seq" {
                    continue;
                }
                if let Some(value) = entity.entity.attributes.get(key) {
                    attr_lines.push(format!("{} = {}", key, Self::format_value(value)));
                }
            }
        }
        if attr_lines.is_empty() {
            attr_lines.push("<none>".to_string());
        }

        let mut inventory_lines = Vec::new();
        for item in entity.entity.inventory.iter().flatten() {
            Self::push_item_tree(&mut inventory_lines, item, 1);
        }
        if inventory_lines.is_empty() {
            inventory_lines.push("<empty>".to_string());
        }

        let mut equipped_lines = Vec::new();
        for (slot, item) in &entity.entity.equipped {
            Self::push_equipped_tree(&mut equipped_lines, slot, item);
        }
        if equipped_lines.is_empty() {
            equipped_lines.push("<empty>".to_string());
        }

        let column_width = 28;
        let separator = " | ";
        lines.push(Self::triple_row(
            Some("attributes".to_string()),
            Some("inventory".to_string()),
            Some("equipped".to_string()),
            column_width,
            separator,
        ));
        let row_count = attr_lines
            .len()
            .max(inventory_lines.len())
            .max(equipped_lines.len());
        for index in 0..row_count {
            lines.push(Self::triple_row(
                attr_lines.get(index).cloned(),
                inventory_lines.get(index).cloned(),
                equipped_lines.get(index).cloned(),
                column_width,
                separator,
            ));
        }

        lines.join("\n")
    }

    fn list_item(&self, item: &RuntimeItem) -> String {
        let mut lines = vec![
            format!(
                "Item {} {}",
                Self::quoted(&Self::item_name(&item.item)),
                item.item.id
            ),
            format!(
                "position = [{:.2}, {:.2}, {:.2}]",
                item.item.position.x, item.item.position.y, item.item.position.z
            ),
            "attributes".to_string(),
        ];

        let keys = item.item.attributes.keys_sorted();
        if keys.is_empty() {
            lines.push("<none>".to_string());
        } else {
            for key in keys {
                if key == "setup" || key == "_source_seq" {
                    continue;
                }
                if let Some(value) = item.item.attributes.get(key) {
                    lines.push(format!("{} = {}", key, Self::format_value(value)));
                }
            }
        }

        lines.push("container".to_string());
        if let Some(container) = &item.item.container {
            if container.is_empty() {
                lines.push("<empty>".to_string());
            } else {
                for child in container {
                    Self::push_item_tree(&mut lines, child, 1);
                }
            }
        } else {
            lines.push("<none>".to_string());
        }

        lines.join("\n")
    }

    fn runtime_snapshot(
        project: &Project,
        server_ctx: &ServerContext,
    ) -> Result<(Vec<RuntimeEntity>, Vec<RuntimeItem>), String> {
        let rusterix = RUSTERIX.read().unwrap();
        if rusterix.server.state != ServerState::Running {
            return Err("Game is not running.".to_string());
        }

        let mut runtime_entities = Vec::new();
        let mut runtime_items = Vec::new();

        let (entities, items) = rusterix.server.get_entities_items(&server_ctx.curr_region);
        if let Some(entities) = entities {
            for entity in entities {
                runtime_entities.push(RuntimeEntity {
                    entity: entity.clone(),
                });
            }
        }
        if let Some(items) = items {
            for item in items {
                runtime_items.push(RuntimeItem { item: item.clone() });
            }
        }

        if runtime_entities.is_empty()
            && runtime_items.is_empty()
            && let Some(region) = project.get_region_ctx(server_ctx)
        {
            for entity in &region.map.entities {
                runtime_entities.push(RuntimeEntity {
                    entity: entity.clone(),
                });
            }
            for item in &region.map.items {
                runtime_items.push(RuntimeItem { item: item.clone() });
            }
        }

        Ok((runtime_entities, runtime_items))
    }

    fn rules_usage() -> &'static str {
        "Usage:\n\
rules overview\n\
rules validate\n\
rules list [races|classes|weapons|armor|spells|abilities|actions]\n\
rules show <ruleset.path>\n\
rules class <class_id>\n\
rules xp <level>\n\
rules weapon <weapon_id> [ATTR=VALUE ...]\n\
rules spell <spell_id> [ATTR=VALUE ...]\n\
rules roll <ruleset.path.to.roll> [ATTR=VALUE ...]"
    }

    fn parse_rules_attributes(
        args: &[&str],
    ) -> Result<shared::rulesets::RulesetAttributeMap, String> {
        let mut attributes = shared::rulesets::RulesetAttributeMap::new();
        for raw in args {
            let Some((key, value)) = raw.split_once('=') else {
                return Err(format!("Attribute `{}` must use ATTR=VALUE syntax.", raw));
            };
            let key = key.trim();
            if key.is_empty() {
                return Err(format!("Attribute `{}` has an empty name.", raw));
            }
            let value = value
                .trim()
                .parse::<f32>()
                .map_err(|_| format!("Attribute `{}` has a non-numeric value.", raw))?;
            attributes.insert(key.to_string(), value);
        }
        Ok(attributes)
    }

    fn effective_rules_source(project: &Project) -> Result<String, String> {
        shared::rulesets::resolve_project_rules(&project.config, &project.rules)
    }

    fn format_roll_summary(label: &str, summary: &shared::rulesets::RulesetRollSummary) -> String {
        let attr_line = if let Some(attribute) = summary.spec.bonus_attribute.as_deref() {
            format!(
                "{}={} => +{} every {}",
                attribute,
                summary.attribute_value,
                summary.attribute_bonus,
                summary.spec.bonus_every
            )
        } else {
            "none".into()
        };
        let kind_line = summary
            .spec
            .damage_kind
            .as_deref()
            .map(|kind| format!("\ndamage kind: {}", kind))
            .unwrap_or_default();

        format!(
            "{}\nroll: {}\nbonus: {}\nattribute bonus: {}\ntotal bonus: {}\nmin: {}\nmax: {}\naverage: {:.2}{}",
            label,
            summary.spec.roll,
            summary.spec.bonus,
            attr_line,
            summary.total_bonus,
            summary.minimum,
            summary.maximum,
            summary.average,
            kind_line
        )
    }

    fn join_or_dash(values: &[String]) -> String {
        if values.is_empty() {
            "-".into()
        } else {
            values.join(", ")
        }
    }

    fn format_ruleset_catalog(catalog: &shared::rulesets::RulesetCatalog) -> String {
        format!(
            "ruleset: {}@{}\nschema: {}\nsource: {}\nraces: {}\nclasses: {}\nweapons: {}\narmor: {}\nclothing: {}\nspells: {}\nabilities: {}\nactions: {}\nfx presets: {}\nitem templates: {}",
            catalog.id.as_deref().unwrap_or("-"),
            catalog.version.as_deref().unwrap_or("-"),
            catalog.schema_version.as_deref().unwrap_or("-"),
            catalog.source.as_deref().unwrap_or("-"),
            catalog.races.len(),
            catalog.classes.len(),
            catalog.weapons.len(),
            catalog.armor.len(),
            catalog.clothing.len(),
            catalog.spells.len(),
            catalog.abilities.len(),
            catalog.actions.len(),
            catalog.fx_presets.len(),
            catalog.item_templates.len(),
        )
    }

    fn format_ruleset_list(label: &str, values: &[String]) -> String {
        if values.is_empty() {
            return format!("{}:\n-", label);
        }

        format!("{}:\n{}", label, values.join("\n"))
    }

    fn format_ruleset_validation(report: &shared::rulesets::RulesetValidationReport) -> String {
        let mut lines = vec![format!(
            "ruleset validation: {} error(s), {} warning(s)",
            report.error_count(),
            report.warning_count()
        )];

        if report.issues.is_empty() {
            lines.push("OK".into());
            return lines.join("\n");
        }

        for issue in &report.issues {
            let severity = match issue.severity {
                shared::rulesets::RulesetValidationSeverity::Error => "ERROR",
                shared::rulesets::RulesetValidationSeverity::Warning => "WARN",
            };
            lines.push(format!("{} {}: {}", severity, issue.path, issue.message));
        }

        lines.join("\n")
    }

    fn execute_rules_command(&self, args: &[&str], project: &Project) -> String {
        let Some(command) = args.first().copied() else {
            return Self::rules_usage().to_string();
        };

        match command {
            "overview" | "summary" | "catalog" | "info" => {
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                match shared::rulesets::ruleset_catalog_from_source(&rules) {
                    Ok(catalog) => Self::format_ruleset_catalog(&catalog),
                    Err(err) => err,
                }
            }
            "validate" | "check" => {
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                match shared::rulesets::validate_ruleset_from_source(&rules) {
                    Ok(report) => Self::format_ruleset_validation(&report),
                    Err(err) => err,
                }
            }
            "list" => {
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                if let Some(section) = args.get(1).copied() {
                    return match shared::rulesets::ruleset_section_ids_from_source(&rules, section)
                    {
                        Ok(values) => Self::format_ruleset_list(section, &values),
                        Err(err) => err,
                    };
                }

                match shared::rulesets::ruleset_catalog_from_source(&rules) {
                    Ok(catalog) => [
                        Self::format_ruleset_list("races", &catalog.races),
                        Self::format_ruleset_list("classes", &catalog.classes),
                        Self::format_ruleset_list("weapons", &catalog.weapons),
                        Self::format_ruleset_list("armor", &catalog.armor),
                        Self::format_ruleset_list("clothing", &catalog.clothing),
                        Self::format_ruleset_list("spells", &catalog.spells),
                        Self::format_ruleset_list("abilities", &catalog.abilities),
                        Self::format_ruleset_list("actions", &catalog.actions),
                        Self::format_ruleset_list("fx presets", &catalog.fx_presets),
                    ]
                    .join("\n\n"),
                    Err(err) => err,
                }
            }
            "show" => {
                let Some(path) = args.get(1).copied() else {
                    return Self::rules_usage().to_string();
                };
                let path_parts = path
                    .split('.')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                if path_parts.is_empty() {
                    return Self::rules_usage().to_string();
                }
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                match shared::rulesets::ruleset_show_path_from_source(&rules, &path_parts) {
                    Ok(Some(value)) => format!("{}:\n{}", path, value),
                    Ok(None) => format!("Ruleset path '{}' was not found.", path),
                    Err(err) => err,
                }
            }
            "xp" => {
                let Some(level) = args.get(1) else {
                    return Self::rules_usage().to_string();
                };
                let Ok(level) = level.parse::<u32>() else {
                    return format!("Level `{}` is not a positive integer.", level);
                };
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                match shared::rulesets::ruleset_xp_for_level_from_source(&rules, level) {
                    Ok(Some(xp)) => format!("level: {}\nrequired xp: {}", level, xp),
                    Ok(None) => format!("No XP entry for level {}.", level),
                    Err(err) => err,
                }
            }
            "weapon" => {
                let Some(weapon_id) = args.get(1).copied() else {
                    return Self::rules_usage().to_string();
                };
                let attributes = match Self::parse_rules_attributes(&args[2..]) {
                    Ok(attributes) => attributes,
                    Err(err) => return err,
                };
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                match shared::rulesets::summarize_weapon_damage_from_source(
                    &rules,
                    weapon_id,
                    &attributes,
                ) {
                    Ok(summary) => {
                        Self::format_roll_summary(&format!("weapon: {}", weapon_id), &summary)
                    }
                    Err(err) => err,
                }
            }
            "spell" => {
                let Some(spell_id) = args.get(1).copied() else {
                    return Self::rules_usage().to_string();
                };
                let attributes = match Self::parse_rules_attributes(&args[2..]) {
                    Ok(attributes) => attributes,
                    Err(err) => return err,
                };
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                match shared::rulesets::summarize_spell_roll_from_source(
                    &rules,
                    spell_id,
                    &attributes,
                ) {
                    Ok((kind, summary)) => Self::format_roll_summary(
                        &format!("spell: {} ({})", spell_id, kind.label()),
                        &summary,
                    ),
                    Err(err) => err,
                }
            }
            "roll" => {
                let Some(path) = args.get(1).copied() else {
                    return Self::rules_usage().to_string();
                };
                let path_parts = path
                    .split('.')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                if path_parts.is_empty() {
                    return Self::rules_usage().to_string();
                }
                let attributes = match Self::parse_rules_attributes(&args[2..]) {
                    Ok(attributes) => attributes,
                    Err(err) => return err,
                };
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                match shared::rulesets::summarize_roll_path_from_source(
                    &rules,
                    &path_parts,
                    &attributes,
                ) {
                    Ok(summary) => Self::format_roll_summary(path, &summary),
                    Err(err) => err,
                }
            }
            "class" => {
                let Some(class_id) = args.get(1).copied() else {
                    return Self::rules_usage().to_string();
                };
                let rules = match Self::effective_rules_source(project) {
                    Ok(rules) => rules,
                    Err(err) => return err,
                };
                let summary = match shared::rulesets::summarize_class_from_source(&rules, class_id)
                {
                    Ok(summary) => summary,
                    Err(err) => return err,
                };
                let mut attributes = summary
                    .attributes
                    .iter()
                    .map(|(key, value)| format!("{}={}", key, value))
                    .collect::<Vec<_>>();
                attributes.sort();
                let unlocks = summary
                    .level_unlocks
                    .iter()
                    .map(|(level, values)| format!("{}: {}", level, Self::join_or_dash(values)))
                    .collect::<Vec<_>>()
                    .join("\n");
                let loadout = summary
                    .starting_loadout
                    .iter()
                    .map(|(category, values)| {
                        format!("{}: {}", category, Self::join_or_dash(values))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                format!(
                    "class: {}\nrole: {}\ndescription: {}\nprimary attributes: {}\nallowed weapons: {}\nallowed armor: {}\nabilities: {}\nspells: {}\nattributes: {}\nunlocks:\n{}\nstarting loadout:\n{}",
                    summary.id,
                    summary.role.as_deref().unwrap_or("-"),
                    summary.description.as_deref().unwrap_or("-"),
                    Self::join_or_dash(&summary.primary_attributes),
                    Self::join_or_dash(&summary.allowed_weapons),
                    Self::join_or_dash(&summary.allowed_armor),
                    Self::join_or_dash(&summary.abilities),
                    Self::join_or_dash(&summary.spells),
                    Self::join_or_dash(&attributes),
                    if unlocks.is_empty() {
                        "-".into()
                    } else {
                        unlocks
                    },
                    if loadout.is_empty() {
                        "-".into()
                    } else {
                        loadout
                    },
                )
            }
            _ => Self::rules_usage().to_string(),
        }
    }

    fn focus_label(&self, entities: &[RuntimeEntity], items: &[RuntimeItem]) -> String {
        match self.focus {
            ConsoleFocus::Root => "root".to_string(),
            ConsoleFocus::Entity(id) => entities
                .iter()
                .find(|entity| entity.entity.id == id)
                .map(|entity| {
                    format!(
                        "character {} {}",
                        Self::quoted(&Self::entity_name(&entity.entity)),
                        id
                    )
                })
                .unwrap_or_else(|| format!("character {}", id)),
            ConsoleFocus::Item(id) => items
                .iter()
                .find(|item| item.item.id == id)
                .map(|item| format!("item {} {}", Self::quoted(&Self::item_name(&item.item)), id))
                .unwrap_or_else(|| format!("item {}", id)),
        }
    }

    fn execute_command(
        &mut self,
        command: &str,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> String {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        if trimmed.eq_ignore_ascii_case("help") {
            return [
                "help  show available commands",
                "list  list the current scope",
                "focus <name|id>  focus a character or item from root",
                "show  show the current character or item details",
                "get <key>  show one attribute from the current character or item",
                "rules overview  show active ruleset counts and metadata",
                "rules validate  validate the effective ruleset",
                "rules list [section]  list ruleset races, classes, weapons, armor, spells, abilities, or actions",
                "rules show <path>  show a TOML value from the effective ruleset",
                "rules class <class_id>  inspect a ruleset class",
                "rules xp <level>  show required XP for a level",
                "rules weapon <weapon_id> [ATTR=VALUE ...]  calculate weapon damage",
                "rules spell <spell_id> [ATTR=VALUE ...]  calculate spell damage or healing",
                "pwd  show the current console focus",
                "up  go back to root",
                "clear  clear the console output",
            ]
            .join("\n");
        }

        if trimmed.eq_ignore_ascii_case("clear") {
            self.transcript.clear();
            return String::new();
        }

        if let Some((head, tail)) = trimmed.split_once(' ')
            && head.eq_ignore_ascii_case("rules")
        {
            let args = tail.split_whitespace().collect::<Vec<_>>();
            return self.execute_rules_command(&args, project);
        }
        if trimmed.eq_ignore_ascii_case("rules") {
            return Self::rules_usage().to_string();
        }

        let (entities, items) = match Self::runtime_snapshot(project, server_ctx) {
            Ok(snapshot) => snapshot,
            Err(err) => return err,
        };
        let focusable_items = Self::collect_focusable_items(&entities, &items);

        match trimmed.split_once(' ') {
            Some((head, tail))
                if head.eq_ignore_ascii_case("focus") || head.eq_ignore_ascii_case("cd") =>
            {
                let needle = tail.trim();
                if needle.is_empty() {
                    return format!("Usage: {} <name|id>", head);
                }
                if needle == ".." || needle.eq_ignore_ascii_case("root") || needle == "/" {
                    self.focus = ConsoleFocus::Root;
                    return self.list_root(&entities, &items);
                }

                if let Some(id) = Self::parse_id(needle) {
                    if entities.iter().any(|entity| entity.entity.id == id) {
                        self.focus = ConsoleFocus::Entity(id);
                        if let Some(entity) = entities.iter().find(|entity| entity.entity.id == id)
                        {
                            return self.list_entity(entity);
                        }
                    }
                    if focusable_items.iter().any(|item| item.item.id == id) {
                        self.focus = ConsoleFocus::Item(id);
                        if let Some(item) = focusable_items.iter().find(|item| item.item.id == id) {
                            return self.list_item(item);
                        }
                    }
                }

                let matching_entities: Vec<&RuntimeEntity> = entities
                    .iter()
                    .filter(|entity| Self::entity_matches(&entity.entity, needle))
                    .collect();
                let matching_items: Vec<&RuntimeItem> = focusable_items
                    .iter()
                    .filter(|item| Self::item_matches(&item.item, needle))
                    .collect();

                if matching_entities.len() + matching_items.len() > 1 {
                    let mut lines = vec!["Multiple matches".to_string()];
                    for entity in matching_entities {
                        lines.push(format!(
                            "character  {}  {}",
                            Self::quoted(&Self::entity_name(&entity.entity)),
                            entity.entity.id
                        ));
                    }
                    for item in matching_items {
                        lines.push(format!(
                            "item       {}  {}",
                            Self::quoted(&Self::item_name(&item.item)),
                            item.item.id
                        ));
                    }
                    return lines.join("\n");
                }

                if let Some(entity) = matching_entities.first() {
                    self.focus = ConsoleFocus::Entity(entity.entity.id);
                    return self.list_entity(entity);
                }
                if let Some(item) = matching_items.first() {
                    self.focus = ConsoleFocus::Item(item.item.id);
                    return self.list_item(item);
                }

                format!("No runtime character or item matched `{}`.", needle)
            }
            Some((head, tail)) if head.eq_ignore_ascii_case("get") => {
                let key = tail.trim();
                if key.is_empty() {
                    return "Usage: get <key>".to_string();
                }
                match self.focus {
                    ConsoleFocus::Entity(_) => {
                        if let Some(entity) = self.focused_entity(&entities) {
                            if let Some(value) = entity.entity.attributes.get(key) {
                                format!("{} = {}", key, Self::format_value(value))
                            } else {
                                format!("Attribute `{}` not found.", key)
                            }
                        } else {
                            self.focus = ConsoleFocus::Root;
                            "Focused character no longer exists.".to_string()
                        }
                    }
                    ConsoleFocus::Item(_) => {
                        if let Some(item) = self.focused_item(&items) {
                            if let Some(item) = focusable_items
                                .iter()
                                .find(|candidate| candidate.item.id == item.item.id)
                            {
                                if let Some(value) = item.item.attributes.get(key) {
                                    format!("{} = {}", key, Self::format_value(value))
                                } else {
                                    format!("Attribute `{}` not found.", key)
                                }
                            } else {
                                self.focus = ConsoleFocus::Root;
                                "Focused item no longer exists.".to_string()
                            }
                        } else if let Some(item) = focusable_items
                            .iter()
                            .find(|candidate| matches!(self.focus, ConsoleFocus::Item(id) if candidate.item.id == id))
                        {
                            if let Some(value) = item.item.attributes.get(key) {
                                format!("{} = {}", key, Self::format_value(value))
                            } else {
                                format!("Attribute `{}` not found.", key)
                            }
                        } else {
                            self.focus = ConsoleFocus::Root;
                            "Focused item no longer exists.".to_string()
                        }
                    }
                    ConsoleFocus::Root => "Focus a character or item first.".to_string(),
                }
            }
            _ => match trimmed.to_ascii_lowercase().as_str() {
                "ls" => "Use `list`.".to_string(),
                "cd .." => {
                    self.focus = ConsoleFocus::Root;
                    self.list_root(&entities, &items)
                }
                "list" => match self.focus {
                    ConsoleFocus::Root => self.list_root(&entities, &items),
                    ConsoleFocus::Entity(_) => {
                        if let Some(entity) = self.focused_entity(&entities) {
                            self.list_entity(entity)
                        } else {
                            self.focus = ConsoleFocus::Root;
                            "Focused character no longer exists.".to_string()
                        }
                    }
                    ConsoleFocus::Item(_) => {
                        if let Some(item) = focusable_items
                            .iter()
                            .find(|candidate| matches!(self.focus, ConsoleFocus::Item(id) if candidate.item.id == id))
                        {
                            self.list_item(item)
                        } else {
                            self.focus = ConsoleFocus::Root;
                            "Focused item no longer exists.".to_string()
                        }
                    }
                },
                "show" | "info" => match self.focus {
                    ConsoleFocus::Root => self.list_root(&entities, &items),
                    ConsoleFocus::Entity(_) => {
                        if let Some(entity) = self.focused_entity(&entities) {
                            self.list_entity(entity)
                        } else {
                            self.focus = ConsoleFocus::Root;
                            "Focused character no longer exists.".to_string()
                        }
                    }
                    ConsoleFocus::Item(_) => {
                        if let Some(item) = focusable_items
                            .iter()
                            .find(|candidate| matches!(self.focus, ConsoleFocus::Item(id) if candidate.item.id == id))
                        {
                            self.list_item(item)
                        } else {
                            self.focus = ConsoleFocus::Root;
                            "Focused item no longer exists.".to_string()
                        }
                    }
                },
                "pwd" => self.focus_label(&entities, &items),
                "up" => {
                    self.focus = ConsoleFocus::Root;
                    self.list_root(&entities, &items)
                }
                _ => format!("Unknown command `{}`. Type `help`.", trimmed),
            },
        }
    }
}

impl Dock for ConsoleDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            transcript: Self::intro(),
            focus: ConsoleFocus::Root,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut output = TheTextAreaEdit::new(TheId::named("Console Output"));
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            output.add_theme_from_string(source);
            output.set_code_theme("Gruvbox Dark");
        }
        if let Some(bytes) = crate::Embedded::get("parser/console.sublime-syntax")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            output.add_syntax_from_string(source);
            output.set_code_type("Eldiron Console");
        }
        output.set_font_size(13.0);
        output.set_continuous(true);
        output.display_line_number(false);
        output.use_global_statusbar(true);
        output.readonly(true);
        output.set_supports_undo(false);
        canvas.set_widget(output);

        let mut input_canvas = TheCanvas::default();
        let mut input = TheTextLineEdit::new(TheId::named("Console Input"));
        input.set_status_text("Enter a console command and press Return.");
        input.set_font_size(12.5);
        input.limiter_mut().set_max_height(24);
        input_canvas.set_widget(input);
        canvas.set_bottom(input_canvas);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        if self.transcript.is_empty() {
            self.transcript = Self::intro();
        }
        self.sync_output(ui, ctx);
        self.set_input(ui, ctx, "");
        if let Some(id) = Self::console_input_id(ui) {
            ctx.ui.set_focus(&id);
        }
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::ValueChanged(id, value) = event
            && id.name == "Console Input"
        {
            let command = value.to_string().unwrap_or_default();
            let command = command.trim().to_string();
            if command.is_empty() {
                self.set_input(ui, ctx, "");
                return false;
            }

            let mut output = format!("{} > {}", self.prompt(project, server_ctx), command);
            let result = self.execute_command(&command, project, server_ctx);
            if !result.is_empty() {
                output.push('\n');
                output.push_str(&result);
            }
            self.set_output(output, ui, ctx);
            self.clear_input(ui);
            if let Some(focus_id) = Self::console_input_id(ui) {
                ctx.ui.focus = Some(focus_id.clone());
                ctx.ui.keyboard_focus = Some(focus_id.clone());
                ctx.ui.send(TheEvent::GainedFocus(focus_id));
                ui.process_events(ctx);
            }
            return true;
        }

        false
    }

    fn supports_actions(&self) -> bool {
        false
    }
}
