use std::{env, fs, path::Path};

use toml::{Table, Value};

use crate::{
    RulesetAttributeMap, RulesetCatalog, RulesetRollSummary, RulesetValidationReport,
    RulesetValidationSeverity, latest_official_ruleset, parse_ruleset_table, ruleset_catalog,
    ruleset_section_ids_from_source, ruleset_show_path_from_source, ruleset_table_at_path,
    ruleset_xp_for_level, summarize_class, summarize_roll_path, summarize_spell_roll,
    summarize_weapon_damage, validate_ruleset,
};

pub fn run_from_env() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    run(&args)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let (rules_src, source, command_args) = rules_source_from_args(args)?;
    let Some(command) = command_args.first().map(String::as_str) else {
        return Err(usage().into());
    };
    let tail = &command_args[1..];

    match command {
        "check" => run_check(&rules_src, &source),
        "summary" => run_summary(&rules_src, &source),
        "list" => run_list(&rules_src, tail),
        "show" => run_show(&rules_src, tail),
        "xp" => run_xp(&rules_src, tail),
        "weapon" => run_weapon(&rules_src, tail),
        "spell" => run_spell(&rules_src, tail),
        "roll" => run_roll(&rules_src, tail),
        "class" => run_class(&rules_src, tail),
        "item" => run_item(&rules_src, tail),
        "recipe" => run_recipe(&rules_src, tail),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(usage().into()),
    }
}

fn usage() -> &'static str {
    "Usage:\n\
       eldiron-ruleset [--rules rules.toml] check\n\
       eldiron-ruleset [--rules rules.toml] summary\n\
       eldiron-ruleset [--rules rules.toml] list <section>\n\
       eldiron-ruleset [--rules rules.toml] show <ruleset.path>\n\
       eldiron-ruleset [--rules rules.toml] class <class_id>\n\
       eldiron-ruleset [--rules rules.toml] item <item_id> [ATTR=VALUE ...]\n\
       eldiron-ruleset [--rules rules.toml] recipe <recipe_id>\n\
       eldiron-ruleset [--rules rules.toml] xp <level>\n\
       eldiron-ruleset [--rules rules.toml] weapon <weapon_id> [ATTR=VALUE ...]\n\
       eldiron-ruleset [--rules rules.toml] spell <spell_id> [ATTR=VALUE ...]\n\
       eldiron-ruleset [--rules rules.toml] roll <ruleset.path.to.roll> [ATTR=VALUE ...]"
}

fn rules_source_from_args(args: &[String]) -> Result<(String, String, &[String]), String> {
    if args.first().is_some_and(|arg| arg == "--rules") {
        let Some(path) = args.get(1) else {
            return Err("--rules requires a path.".into());
        };
        let source = fs::read_to_string(path)
            .map_err(|err| format!("Could not read ruleset '{}': {}", path, err))?;
        return Ok((source, path.clone(), &args[2..]));
    }

    Ok((
        latest_official_ruleset().to_string(),
        "bundled official ruleset".into(),
        args,
    ))
}

fn rules_table(src: &str) -> Result<Table, String> {
    parse_ruleset_table(src)
}

fn format_list(values: &[String]) -> String {
    if values.is_empty() {
        "-".into()
    } else {
        values.join(", ")
    }
}

fn format_catalog_summary(source: &str, catalog: &RulesetCatalog) -> String {
    format!(
        "source: {}\n\
         id: {}\n\
         version: {}\n\
         schema: {}\n\
         races: {} ({})\n\
         classes: {} ({})\n\
         professions: {} ({})\n\
         skills: {} ({})\n\
         resources: {} ({})\n\
         recipes: {} ({})\n\
         actions: {} ({})\n\
         abilities: {} ({})\n\
         spells: {} ({})\n\
         weapons: {} ({})\n\
         armor: {} ({})\n\
         clothing: {} ({})\n\
         item templates: {}\n\
         fx presets: {} ({})",
        source,
        catalog.id.as_deref().unwrap_or("-"),
        catalog.version.as_deref().unwrap_or("-"),
        catalog.schema_version.as_deref().unwrap_or("-"),
        catalog.races.len(),
        format_list(&catalog.races),
        catalog.classes.len(),
        format_list(&catalog.classes),
        catalog.professions.len(),
        format_list(&catalog.professions),
        catalog.skills.len(),
        format_list(&catalog.skills),
        catalog.resources.len(),
        format_list(&catalog.resources),
        catalog.recipes.len(),
        format_list(&catalog.recipes),
        catalog.actions.len(),
        format_list(&catalog.actions),
        catalog.abilities.len(),
        format_list(&catalog.abilities),
        catalog.spells.len(),
        format_list(&catalog.spells),
        catalog.weapons.len(),
        format_list(&catalog.weapons),
        catalog.armor.len(),
        format_list(&catalog.armor),
        catalog.clothing.len(),
        format_list(&catalog.clothing),
        catalog.item_templates.len(),
        catalog.fx_presets.len(),
        format_list(&catalog.fx_presets),
    )
}

fn format_validation(source: &str, report: &RulesetValidationReport) -> String {
    let mut lines = vec![
        format!(
            "ruleset check: {} ({} errors, {} warnings)",
            if report.is_ok() { "ok" } else { "failed" },
            report.error_count(),
            report.warning_count()
        ),
        format!("source: {}", source),
    ];

    for issue in &report.issues {
        let severity = match issue.severity {
            RulesetValidationSeverity::Error => "error",
            RulesetValidationSeverity::Warning => "warning",
        };
        lines.push(format!("{} {}: {}", severity, issue.path, issue.message));
    }

    lines.join("\n")
}

fn run_check(src: &str, source: &str) -> Result<(), String> {
    let rules = rules_table(src)?;
    let report = validate_ruleset(&rules);
    println!("{}", format_validation(source, &report));
    if report.error_count() > 0 {
        return Err("Ruleset check failed.".into());
    }
    Ok(())
}

fn run_summary(src: &str, source: &str) -> Result<(), String> {
    let rules = rules_table(src)?;
    println!(
        "{}",
        format_catalog_summary(source, &ruleset_catalog(&rules))
    );
    Ok(())
}

fn run_list(src: &str, args: &[String]) -> Result<(), String> {
    let Some(section) = args.first() else {
        return Err(usage().into());
    };
    println!(
        "{}",
        format_list(&ruleset_section_ids_from_source(src, section)?)
    );
    Ok(())
}

fn run_show(src: &str, args: &[String]) -> Result<(), String> {
    let Some(path) = args.first() else {
        return Err(usage().into());
    };
    let path = path_parts(path);
    let Some(value) = ruleset_show_path_from_source(src, &path)? else {
        return Err(format!("Ruleset path '{}' was not found.", path.join(".")));
    };
    println!("{value}");
    Ok(())
}

fn run_xp(src: &str, args: &[String]) -> Result<(), String> {
    let Some(level) = args.first() else {
        return Err(usage().into());
    };
    let level = level
        .parse::<u32>()
        .map_err(|_| format!("Level '{}' is not a positive integer.", level))?;
    let rules = rules_table(src)?;
    let Some(xp) = ruleset_xp_for_level(&rules, level) else {
        return Err(format!("No XP entry for level {}.", level));
    };
    println!("level: {}\nrequired xp: {}", level, xp);
    Ok(())
}

fn parse_attributes(args: &[String]) -> Result<RulesetAttributeMap, String> {
    let mut attributes = RulesetAttributeMap::new();
    for raw in args {
        let Some((key, value)) = raw.split_once('=') else {
            return Err(format!("Attribute '{}' must use ATTR=VALUE syntax.", raw));
        };
        let value = value
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("Attribute '{}' has a non-numeric value.", raw))?;
        attributes.insert(key.trim().to_string(), value);
    }
    Ok(attributes)
}

fn path_parts(path: &str) -> Vec<&str> {
    path.split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn format_roll(label: &str, summary: &RulesetRollSummary) -> String {
    let attr_line = if let Some(attribute) = summary.spec.bonus_attribute.as_deref() {
        format!(
            "{}={} => +{} every {}",
            attribute, summary.attribute_value, summary.attribute_bonus, summary.spec.bonus_every
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

fn run_roll(src: &str, args: &[String]) -> Result<(), String> {
    let Some(path) = args.first() else {
        return Err(usage().into());
    };
    let path_parts = path_parts(path);
    if path_parts.is_empty() {
        return Err(usage().into());
    }
    let attributes = parse_attributes(&args[1..])?;
    let rules = rules_table(src)?;
    let summary = summarize_roll_path(&rules, &path_parts, &attributes)?;
    println!("{}", format_roll(path, &summary));
    Ok(())
}

fn run_weapon(src: &str, args: &[String]) -> Result<(), String> {
    let Some(weapon_id) = args.first() else {
        return Err(usage().into());
    };
    let attributes = parse_attributes(&args[1..])?;
    let rules = rules_table(src)?;
    let summary = summarize_weapon_damage(&rules, weapon_id, &attributes)?;
    println!(
        "{}",
        format_roll(&format!("weapon: {}", weapon_id), &summary)
    );
    Ok(())
}

fn run_spell(src: &str, args: &[String]) -> Result<(), String> {
    let Some(spell_id) = args.first() else {
        return Err(usage().into());
    };
    let attributes = parse_attributes(&args[1..])?;
    let rules = rules_table(src)?;
    let (kind, summary) = summarize_spell_roll(&rules, spell_id, &attributes)?;
    println!(
        "{}",
        format_roll(&format!("spell: {} ({})", spell_id, kind.label()), &summary)
    );
    Ok(())
}

fn run_class(src: &str, args: &[String]) -> Result<(), String> {
    let Some(class_id) = args.first() else {
        return Err(usage().into());
    };
    let rules = rules_table(src)?;
    let summary = summarize_class(&rules, class_id)?;
    let attrs = summary
        .attributes
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>();
    println!(
        "class: {}\nrole: {}\nprimary attributes: {}\nallowed weapons: {}\nallowed armor: {}\nabilities: {}\nspells: {}\nattributes: {}",
        summary.id,
        summary.role.as_deref().unwrap_or("-"),
        format_list(&summary.primary_attributes),
        format_list(&summary.allowed_weapons),
        format_list(&summary.allowed_armor),
        format_list(&summary.abilities),
        format_list(&summary.spells),
        format_list(&attrs),
    );
    Ok(())
}

fn item_table<'a>(rules: &'a Table, item_id: &str) -> Option<(&'static str, &'a Table)> {
    for group in [
        "weapons",
        "armor",
        "clothing",
        "ammunition",
        "reagents",
        "materials",
        "resources",
    ] {
        if let Some(table) = ruleset_table_at_path(rules, &["items", group])
            .and_then(|items| items.get(item_id))
            .and_then(Value::as_table)
        {
            return Some((group, table));
        }
    }
    None
}

fn table_string(table: &Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn table_number(table: &Table, key: &str) -> Option<f32> {
    table.get(key).and_then(|value| {
        value
            .as_float()
            .map(|value| value as f32)
            .or_else(|| value.as_integer().map(|value| value as f32))
    })
}

fn value_inline(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Boolean(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(value_inline)
            .collect::<Vec<_>>()
            .join(", "),
        _ => value.to_string(),
    }
}

fn table_lines(table: &Table) -> Vec<String> {
    let mut lines = table
        .iter()
        .filter(|(_, value)| !value.is_table())
        .map(|(key, value)| format!("{}: {}", key, value_inline(value)))
        .collect::<Vec<_>>();
    lines.sort();
    lines
}

fn run_item(src: &str, args: &[String]) -> Result<(), String> {
    let Some(item_id) = args.first() else {
        return Err(usage().into());
    };
    let attributes = parse_attributes(&args[1..])?;
    let rules = rules_table(src)?;
    let Some((group, item)) = item_table(&rules, item_id) else {
        return Err(format!("Item '{}' was not found.", item_id));
    };
    let name = table_string(item, "name").unwrap_or_else(|| item_id.to_string());
    let mut out = vec![
        format!("item: {}", name),
        format!("id: {}", item_id),
        format!("kind: {}", group.strip_suffix('s').unwrap_or(group)),
        format!(
            "category: {}",
            table_string(item, "category").unwrap_or_else(|| "-".into())
        ),
        format!(
            "slot: {}",
            table_string(item, "slot").unwrap_or_else(|| "-".into())
        ),
    ];
    if item.get("damage").and_then(Value::as_table).is_some() {
        let summary =
            summarize_roll_path(&rules, &["items", group, item_id, "damage"], &attributes)?;
        out.push(String::new());
        out.push(format_roll("damage", &summary));
    }
    if let Some(attrs) = item.get("attributes").and_then(Value::as_table) {
        out.push(String::new());
        out.push("attributes:".into());
        out.extend(table_lines(attrs));
    }
    println!("{}", out.join("\n"));
    Ok(())
}

fn item_label(rules: &Table, item_id: &str) -> String {
    if let Some((group, item)) = item_table(rules, item_id) {
        let name = table_string(item, "name").unwrap_or_else(|| item_id.to_string());
        format!("{} ({})", name, group)
    } else {
        item_id.to_string()
    }
}

fn item_quantity_lines(rules: &Table, recipe: &Table, key: &str) -> Vec<String> {
    recipe
        .get(key)
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_table)
                .filter_map(|entry| {
                    let item_id = entry.get("item")?.as_str()?.trim();
                    let quantity = entry
                        .get("quantity")
                        .and_then(Value::as_integer)
                        .unwrap_or(1)
                        .max(1);
                    Some(format!("{} x{}", item_label(rules, item_id), quantity))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn run_recipe(src: &str, args: &[String]) -> Result<(), String> {
    let Some(recipe_id) = args.first() else {
        return Err(usage().into());
    };
    let rules = rules_table(src)?;
    let Some(recipe) = ruleset_table_at_path(&rules, &["recipes"])
        .and_then(|recipes| recipes.get(recipe_id))
        .and_then(Value::as_table)
    else {
        return Err(format!("Recipe '{}' was not found.", recipe_id));
    };
    let mut out = vec![
        format!(
            "recipe: {}",
            table_string(recipe, "name").unwrap_or_else(|| recipe_id.to_string())
        ),
        format!("id: {}", recipe_id),
        format!(
            "skill: {}",
            table_string(recipe, "skill").unwrap_or_else(|| "-".into())
        ),
        format!(
            "required_skill: {}",
            table_number(recipe, "required_skill")
                .map(|value| format!("{:.0}", value))
                .unwrap_or_else(|| "-".into())
        ),
    ];
    out.push(String::new());
    out.push("consumes:".into());
    out.extend_or_dash(item_quantity_lines(&rules, recipe, "consumes"));
    out.push(String::new());
    out.push("produces:".into());
    out.extend_or_dash(item_quantity_lines(&rules, recipe, "produces"));
    println!("{}", out.join("\n"));
    Ok(())
}

trait ExtendOrDash {
    fn extend_or_dash(&mut self, values: Vec<String>);
}

impl ExtendOrDash for Vec<String> {
    fn extend_or_dash(&mut self, values: Vec<String>) {
        if values.is_empty() {
            self.push("-".into());
        } else {
            self.extend(values);
        }
    }
}

#[allow(dead_code)]
fn _assert_rules_path_is_file(path: &Path) -> bool {
    path.is_file()
}
