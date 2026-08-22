use std::collections::BTreeMap;

use toml::{Table, Value};

use crate::{
    RulesetAttributeMap, RulesetCatalog, RulesetRollSummary, evaluate_formula, parse_ruleset_table,
    ruleset_catalog, ruleset_table_at_path, ruleset_xp_for_level, summarize_class,
    summarize_roll_path, summarize_spell_roll, summarize_weapon_damage,
};

/// A renderer-independent response from the interactive ruleset help engine.
///
/// Creator presents `output` in its Help dock. `suggestions` are deliberately
/// separate so another frontend can render them as completion candidates or
/// buttons without parsing prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetHelpCommand {
    pub command: String,
    pub description: String,
}

impl RulesetHelpCommand {
    fn new(command: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetHelpResponse {
    pub output: String,
    /// Structured valid commands for rich frontends. This keeps command
    /// layout out of monospaced prose and lets frontends make entries clickable.
    pub commands: Vec<RulesetHelpCommand>,
    pub commands_title: Option<String>,
    pub suggestions: Vec<String>,
    /// Ruleset item ids relevant to this response. Rich frontends may render
    /// these as an icon gallery while text-only frontends ignore them.
    pub item_ids: Vec<String>,
    pub clear: bool,
}

impl RulesetHelpResponse {
    fn new(output: impl Into<String>, suggestions: Vec<String>) -> Self {
        Self {
            output: output.into(),
            commands: Vec::new(),
            commands_title: None,
            suggestions,
            item_ids: Vec::new(),
            clear: false,
        }
    }

    fn with_items(mut self, item_ids: Vec<String>) -> Self {
        self.item_ids = item_ids;
        self
    }

    fn with_commands(mut self, commands: Vec<RulesetHelpCommand>) -> Self {
        self.commands = commands;
        self.commands_title = Some("Valid commands".to_string());
        self
    }

    fn with_command_list(
        mut self,
        title: impl Into<String>,
        commands: Vec<RulesetHelpCommand>,
    ) -> Self {
        self.commands = commands;
        self.commands_title = Some(title.into());
        self
    }

    fn clear() -> Self {
        Self {
            output: String::new(),
            commands: Vec::new(),
            commands_title: None,
            suggestions: Vec::new(),
            item_ids: Vec::new(),
            clear: true,
        }
    }
}

pub fn ruleset_help_intro(source: &str) -> Result<RulesetHelpResponse, String> {
    let root = parse_ruleset_table(source)?;
    Ok(HelpEngine::new(root).intro())
}

pub fn execute_ruleset_help(source: &str, command: &str) -> Result<RulesetHelpResponse, String> {
    let root = parse_ruleset_table(source)?;
    HelpEngine::new(root).execute(command)
}

struct HelpEngine {
    root: Table,
    catalog: RulesetCatalog,
}

impl HelpEngine {
    fn new(root: Table) -> Self {
        let catalog = ruleset_catalog(&root);
        Self { root, catalog }
    }

    fn ruleset_label(&self) -> String {
        let metadata = table_at_path(&self.root, &["ruleset"]);
        let name = metadata
            .and_then(|table| table_string(table, "name"))
            .or_else(|| self.catalog.id.clone())
            .unwrap_or_else(|| "Ruleset".to_string());
        let version = self
            .catalog
            .version
            .as_deref()
            .map(|version| format!(" {version}"))
            .unwrap_or_default();
        format!("{name}{version}")
    }

    fn intro(&self) -> RulesetHelpResponse {
        RulesetHelpResponse::new(
            format!(
                "Interactive Ruleset Help\n{}\n\nExplore the project's effective ruleset. Select a command below or type a query.",
                self.ruleset_label()
            ),
            self.root_suggestions(),
        )
        .with_commands(self.available_commands())
    }

    fn execute(&self, command: &str) -> Result<RulesetHelpResponse, String> {
        let args = split_command(command)?;
        let Some(head) = args.first().map(|arg| arg.to_ascii_lowercase()) else {
            return Ok(self.intro());
        };
        let tail = &args[1..];

        let response = match head.as_str() {
            "help" | "commands" | "?" => {
                let topic = tail.first().map(String::as_str);
                let response = RulesetHelpResponse::new(
                    if topic.is_some() {
                        self.command_help(topic)
                    } else {
                        format!(
                            "Ruleset Help\n{}\n\nAll queries use the project's effective ruleset.",
                            self.ruleset_label()
                        )
                    },
                    self.suggestions_for_help(topic),
                );
                if topic.is_none() {
                    response.with_commands(self.available_commands())
                } else {
                    response
                }
            }
            "overview" | "summary" => RulesetHelpResponse::new(
                self.overview(),
                vec!["topics".into(), "progression".into(), "examples".into()],
            ),
            "topics" => RulesetHelpResponse::new(self.topics(), self.topic_suggestions()),
            "list" | "ls" => self.list(tail)?,
            "paths" | "tree" => self.paths(tail)?,
            "show" | "get" => self.show(tail)?,
            "search" | "find" => self.search(tail)?,
            "progression" | "levels" => self.progression(tail)?,
            "unlocks" => self.unlocks(tail)?,
            "spells" => self.class_grants(tail, "spells")?,
            "abilities" => self.class_grants(tail, "abilities")?,
            "class" | "classes" => self.class(tail)?,
            "item" | "items" => self.item(tail)?,
            "combat" | "damage" => self.combat(tail)?,
            "attributes" | "attribute" | "stats" => self.attributes(tail)?,
            "equipment" => self.section_or_entry(&["equipment"], tail)?,
            "economy" => self.section_or_entry(&["economy"], tail)?,
            "invocations" | "invocation" => self.section_or_entry(&["invocation_schemes"], tail)?,
            "fx" | "effects" => self.section_or_entry(&["fx", "presets"], tail)?,
            "race" | "races" => self.section_or_entry(&["races"], tail)?,
            "profession" | "professions" => self.section_or_entry(&["professions"], tail)?,
            "skill" | "skills" => self.section_or_entry(&["skills"], tail)?,
            "resource" | "resources" => self.section_or_entry(&["resources"], tail)?,
            "action" | "actions" => self.section_or_entry(&["actions"], tail)?,
            "ability" => self.section_or_entry(&["abilities"], tail)?,
            "spell" => self.section_or_entry(&["spells"], tail)?,
            "condition" | "conditions" => self.section_or_entry(&["conditions"], tail)?,
            "recipe" | "recipes" => self.recipe(tail)?,
            "calc" | "calculate" => self.calculate(tail)?,
            "examples" | "example" => self.examples(tail)?,
            "clear" | "cls" => RulesetHelpResponse::clear(),
            _ => {
                let matches = self.search_values(&args.join(" "), 12);
                let hint = if matches.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n\nPossible ruleset matches:\n{}",
                        matches
                            .iter()
                            .map(|entry| format!("  {entry}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                };
                return Err(format!(
                    "Unknown help command `{}`. Type `help` for valid commands.{hint}",
                    args[0]
                ));
            }
        };
        Ok(response)
    }

    fn root_suggestions(&self) -> Vec<String> {
        [
            "overview",
            "topics",
            "progression",
            "spells",
            "abilities",
            "combat",
            "equipment",
            "recipes",
            "examples damage",
            "search <text>",
            "paths",
        ]
        .into_iter()
        .filter(|command| {
            let head = command.split_whitespace().next().unwrap_or(command);
            self.command_is_available(head)
        })
        .map(str::to_string)
        .collect()
    }

    fn topic_suggestions(&self) -> Vec<String> {
        [
            "progression",
            "spells",
            "abilities",
            "combat",
            "attributes",
            "equipment",
            "conditions",
            "recipes",
            "invocations",
            "economy",
        ]
        .into_iter()
        .filter(|name| self.command_is_available(name))
        .map(str::to_string)
        .collect()
    }

    fn command_is_available(&self, command: &str) -> bool {
        let path: &[&str] = match command {
            "progression" => &["progression"],
            "spells" => &["spells"],
            "abilities" => &["abilities"],
            "combat" => &["combat"],
            "attributes" => &["attributes"],
            "equipment" => &["equipment"],
            "conditions" => &["conditions"],
            "recipes" => &["recipes"],
            "invocations" => &["invocation_schemes"],
            "economy" => &["economy"],
            _ => return true,
        };
        value_at_path(&self.root, path).is_some()
    }

    fn available_commands(&self) -> Vec<RulesetHelpCommand> {
        let mut commands = vec![
            RulesetHelpCommand::new("overview", "Ruleset identity and content counts"),
            RulesetHelpCommand::new("topics", "Explorable ruleset domains"),
            RulesetHelpCommand::new("list [path]", "Valid children or ids at a path"),
            RulesetHelpCommand::new("paths [prefix]", "Every reachable ruleset table"),
            RulesetHelpCommand::new("show <path>", "Raw effective rule value"),
            RulesetHelpCommand::new("search <text>", "Names, descriptions, values, and paths"),
        ];

        if self.command_is_available("progression") {
            commands.push(RulesetHelpCommand::new(
                "progression [class]",
                "XP, per-level growth, and unlock timeline",
            ));
            commands.push(RulesetHelpCommand::new(
                "unlocks <class> [level]",
                "Abilities and spells granted by level",
            ));
        }
        if self.command_is_available("spells") {
            commands.push(RulesetHelpCommand::new(
                "spells [class]",
                "Spells and their acquisition levels",
            ));
        }
        if self.command_is_available("abilities") {
            commands.push(RulesetHelpCommand::new(
                "abilities [class]",
                "Abilities and their acquisition levels",
            ));
        }
        for (section, command, description) in [
            ("classes", "class <id>", "Class rules"),
            ("races", "race <id>", "Race rules"),
            ("professions", "profession <id>", "Profession rules"),
            ("actions", "action <id>", "Action definitions"),
            ("spells", "spell <id>", "Spell definitions"),
            ("abilities", "ability <id>", "Ability definitions"),
            ("items", "item <id>", "Items and equipment"),
            ("recipes", "recipe <id>", "Crafting recipes"),
            ("resources", "resource <id>", "Gathering resources"),
            ("conditions", "condition <id>", "Condition definitions"),
            ("skills", "skill <id>", "Skill definitions"),
        ] {
            if value_at_path(&self.root, &[section]).is_some() {
                commands.push(RulesetHelpCommand::new(command, description));
            }
        }
        if self.command_is_available("attributes") {
            commands.push(RulesetHelpCommand::new(
                "attributes [derived-stat]",
                "Attributes, roles, and formulas",
            ));
        }
        if self.command_is_available("combat") {
            commands.push(RulesetHelpCommand::new(
                "combat [damage-kind]",
                "Timing and damage/reduction policy",
            ));
        }
        for (command, description) in [
            ("equipment", "Equipment slots and restrictions"),
            ("economy", "Currency and economy rules"),
            ("invocations", "Invocation schemes"),
        ] {
            if self.command_is_available(command) {
                commands.push(RulesetHelpCommand::new(command, description));
            }
        }
        if self.command_is_available("fx") {
            commands.push(RulesetHelpCommand::new(
                "fx [preset]",
                "Procedural effect rules",
            ));
        }
        commands.extend([
            RulesetHelpCommand::new("calc <kind> ...", "Executable rule calculations"),
            RulesetHelpCommand::new("examples [topic]", "Generated examples from this ruleset"),
            RulesetHelpCommand::new("clear", "Clear the Help transcript"),
        ]);
        commands
    }

    fn command_help(&self, topic: Option<&str>) -> String {
        let Some(topic) = topic.map(str::to_ascii_lowercase) else {
            let mut lines = vec!["Valid commands".to_string()];
            lines.extend(
                self.available_commands()
                    .into_iter()
                    .map(|entry| format!("  {}\n    {}", entry.command, entry.description)),
            );
            lines.push(String::new());
            lines.push(
                "Type `help <command>` for syntax. All queries use the project's effective ruleset."
                    .to_string(),
            );
            return lines.join("\n");
        };

        match topic.as_str() {
            "calc" | "calculate" => [
                "Calculations",
                "  calc weapon <id> [ATTR=VALUE ...]",
                "  calc spell <id> [ATTR=VALUE ...]",
                "  calc ability <id> [ATTR=VALUE ...]",
                "  calc roll <path> [ATTR=VALUE ...]",
                "  calc derived <stat> [base=VALUE ATTR=VALUE ...]",
                "  calc damage <weapon|spell|ability|roll> <id|path> [values...]",
                "",
                "Full damage values accept attacker.ATTR, defender.ATTR, equipment.ATTR,",
                "and source.ATTR. Unprefixed attributes are treated as attacker values.",
                "Example: calc damage weapon hand_axe STR=14 defender.ARMOR=2 equipment.ARMOR=3",
            ]
            .join("\n"),
            "progression" | "levels" => [
                "Progression",
                "  progression             show the XP table and available classes",
                "  progression <class>     show growth and the complete unlock timeline",
                "  unlocks <class> [level] focus on grants up to or at one level",
                "  spells <class>           show when that class receives each spell",
                "  abilities <class>        show when that class receives each ability",
            ]
            .join("\n"),
            "list" | "paths" | "show" | "search" => [
                "Universal discovery",
                "  list                     list top-level sections",
                "  list <path>              list direct keys, for example `list combat.kinds`",
                "  paths [prefix]           recursively list every table path",
                "  show <path>              print any effective TOML value",
                "  search <text>            search paths, keys, descriptions, and scalar values",
                "",
                "These commands provide coverage for new rules before specialized views exist.",
            ]
            .join("\n"),
            "examples" | "example" => [
                "Generated examples",
                "  examples                 list available example families",
                "  examples damage          one executable roll for every damage kind",
                "  examples progression     progression queries for every class",
                "  examples crafting        recipe queries",
                "  examples all             all generated examples",
            ]
            .join("\n"),
            other if self.command_is_available(other) => format!(
                "`{other}` is available for this ruleset. Enter `{other}` without arguments to list valid ids, or use `show` and `paths` for raw rule paths."
            ),
            other => {
                format!("No specialized help is available for `{other}`. Try `search {other}`.")
            }
        }
    }

    fn suggestions_for_help(&self, topic: Option<&str>) -> Vec<String> {
        if topic.is_some() {
            self.root_suggestions()
        } else {
            self.topic_suggestions()
                .into_iter()
                .map(|topic| format!("help {topic}"))
                .collect()
        }
    }

    fn topics(&self) -> String {
        let mut lines = vec![format!("Topics in {}", self.ruleset_label())];
        for (command, description) in [
            ("progression", "XP, class growth, level unlocks"),
            ("spells", "spell definitions and class acquisition levels"),
            (
                "abilities",
                "ability definitions and class acquisition levels",
            ),
            ("combat", "damage kinds, rolls, mitigation, cooldowns"),
            ("attributes", "defaults, semantic roles, derived formulas"),
            ("equipment", "slots, categories, handedness, permissions"),
            (
                "conditions",
                "duration, stacking, modifiers, periodic effects",
            ),
            (
                "recipes",
                "requirements, ingredients, products, skill gains",
            ),
            ("resources", "gathering nodes and resource actions"),
            ("invocations", "input schemes, lexicons, spell phrases"),
            ("economy", "currencies and starting wealth"),
            ("fx", "procedural presentation presets"),
        ] {
            if self.command_is_available(command) {
                lines.push(format!("  {command:<13} {description}"));
            }
        }
        lines.push("".into());
        lines.push("Universal coverage: list, paths, show, and search.".into());
        lines.join("\n")
    }

    fn overview(&self) -> String {
        let mut lines = vec![self.ruleset_label()];
        if let Some(id) = self.catalog.id.as_deref() {
            lines.push(format!("id: {id}"));
        }
        if let Some(schema) = self.catalog.schema_version.as_deref() {
            lines.push(format!("schema: {schema}"));
        }
        lines.push(String::new());
        lines.push("Content".into());
        for (name, count) in [
            ("races", self.catalog.races.len()),
            ("classes", self.catalog.classes.len()),
            ("professions", self.catalog.professions.len()),
            ("skills", self.catalog.skills.len()),
            ("resources", self.catalog.resources.len()),
            ("recipes", self.catalog.recipes.len()),
            ("actions", self.catalog.actions.len()),
            ("conditions", self.catalog.conditions.len()),
            ("abilities", self.catalog.abilities.len()),
            ("spells", self.catalog.spells.len()),
            ("weapons", self.catalog.weapons.len()),
            ("armor", self.catalog.armor.len()),
            ("clothing", self.catalog.clothing.len()),
            ("item templates", self.catalog.item_templates.len()),
            ("FX presets", self.catalog.fx_presets.len()),
        ] {
            if count > 0 {
                lines.push(format!("  {name:<15} {count}"));
            }
        }
        lines.push(String::new());
        lines.push(format!(
            "Top-level rule sections: {}",
            sorted_keys(&self.root).join(", ")
        ));
        lines.join("\n")
    }

    fn list(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let path = args.first().map(|arg| path_parts(arg)).unwrap_or_default();
        let value = if path.is_empty() {
            None
        } else {
            Some(
                value_at_path_ci(&self.root, &path)
                    .ok_or_else(|| format!("Ruleset path `{}` was not found.", path.join(".")))?,
            )
        };
        let keys = match value {
            None => sorted_keys(&self.root),
            Some(Value::Table(table)) => sorted_keys(table),
            Some(Value::Array(values)) => values.iter().map(value_inline).collect(),
            Some(value) => vec![value_inline(value)],
        };
        let label = if path.is_empty() {
            "Top-level ruleset sections".to_string()
        } else {
            format!("Children of {}", path.join("."))
        };
        let output = if keys.is_empty() {
            format!("{label}\n  <none>")
        } else {
            format!("{label}\n{}", indent_lines(&keys))
        };
        let suggestions = keys
            .iter()
            .take(12)
            .map(|key| {
                if path.is_empty() {
                    format!("list {key}")
                } else {
                    format!("show {}.{key}", path.join("."))
                }
            })
            .collect();
        Ok(RulesetHelpResponse::new(output, suggestions))
    }

    fn paths(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let prefix = args.first().map(|arg| path_parts(arg)).unwrap_or_default();
        let (value, display_prefix) = if prefix.is_empty() {
            (None, String::new())
        } else {
            let value = value_at_path_ci(&self.root, &prefix)
                .ok_or_else(|| format!("Ruleset path `{}` was not found.", prefix.join(".")))?;
            (Some(value), prefix.join("."))
        };
        let mut paths = Vec::new();
        match value {
            None => collect_paths_from_table(&self.root, "", &mut paths),
            Some(Value::Table(table)) => {
                collect_paths_from_table(table, &display_prefix, &mut paths)
            }
            Some(_) => paths.push(display_prefix),
        }
        let output = format!("Ruleset paths ({})\n{}", paths.len(), indent_lines(&paths));
        let suggestions = paths
            .iter()
            .take(12)
            .map(|path| format!("show {path}"))
            .collect();
        Ok(RulesetHelpResponse::new(output, suggestions))
    }

    fn show(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let path = required_path(args, "Usage: show <ruleset.path>")?;
        let value = value_at_path_ci(&self.root, &path)
            .ok_or_else(|| format!("Ruleset path `{}` was not found.", path.join(".")))?;
        let rendered = match value {
            Value::Table(table) => toml::to_string(table)
                .map_err(|error| format!("Could not render ruleset value: {error}"))?
                .trim()
                .to_string(),
            _ => value_inline(value),
        };
        Ok(RulesetHelpResponse::new(
            format!("{}\n{}", path.join("."), rendered),
            vec![format!("list {}", path.join(".")), "paths".into()],
        ))
    }

    fn search(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        if args.is_empty() {
            return Err("Usage: search <text>".into());
        }
        let query = args.join(" ");
        let matches = self.search_values(&query, 100);
        let output = if matches.is_empty() {
            format!("No ruleset matches for `{query}`.")
        } else {
            format!(
                "Matches for `{query}` ({})\n{}",
                matches.len(),
                indent_lines(&matches)
            )
        };
        let suggestions = matches
            .iter()
            .take(10)
            .filter_map(|line| {
                line.split_once(" = ")
                    .map(|(path, _)| format!("show {path}"))
            })
            .collect();
        Ok(RulesetHelpResponse::new(output, suggestions))
    }

    fn search_values(&self, query: &str, limit: usize) -> Vec<String> {
        let mut matches = Vec::new();
        collect_search_matches(
            &self.root,
            "",
            &query.to_ascii_lowercase(),
            limit,
            &mut matches,
        );
        matches
    }

    fn section_or_entry(
        &self,
        path: &[&str],
        args: &[String],
    ) -> Result<RulesetHelpResponse, String> {
        let table = table_at_path(&self.root, path)
            .ok_or_else(|| format!("This ruleset has no `{}` section.", path.join(".")))?;
        let Some(requested) = args.first() else {
            let keys = sorted_keys(table);
            let command = section_command(path);
            return Ok(RulesetHelpResponse::new(
                format!(
                    "{} ({})\n{}",
                    title_case(path.last().unwrap_or(&"rules")),
                    keys.len(),
                    indent_lines(&keys)
                ),
                keys.iter()
                    .take(12)
                    .map(|id| format!("{command} {id}"))
                    .collect(),
            ));
        };
        let (id, value) = table_entry_ci(table, requested)
            .ok_or_else(|| unknown_id(path.last().unwrap_or(&"entry"), requested, table))?;
        let entry = value
            .as_table()
            .ok_or_else(|| format!("`{}` is not a rule table.", path.join(".")))?;
        let mut full_path = path
            .iter()
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>();
        full_path.push(id.clone());
        Ok(RulesetHelpResponse::new(
            describe_table(&full_path.join("."), entry),
            vec![
                format!("show {}", full_path.join(".")),
                format!("list {}", path.join(".")),
            ],
        ))
    }

    fn class(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let Some(requested) = args.first() else {
            return self.section_or_entry(&["classes"], args);
        };
        let classes = table_at_path(&self.root, &["classes"])
            .ok_or_else(|| "This ruleset has no classes.".to_string())?;
        let (class_id, _) = table_entry_ci(classes, requested)
            .ok_or_else(|| unknown_id("class", requested, classes))?;
        let summary = summarize_class(&self.root, &class_id)?;
        let mut lines = vec![format!("Class: {}", summary.id)];
        if let Some(description) = summary.description {
            lines.push(description);
        }
        if let Some(role) = summary.role {
            lines.push(format!("Role: {role}"));
        }
        lines.push(format!(
            "Primary attributes: {}",
            list_or_dash(&summary.primary_attributes)
        ));
        lines.push(format!(
            "Allowed weapons: {}",
            list_or_dash(&summary.allowed_weapons)
        ));
        lines.push(format!(
            "Allowed armor: {}",
            list_or_dash(&summary.allowed_armor)
        ));
        lines.push(format!("Abilities: {}", list_or_dash(&summary.abilities)));
        lines.push(format!("Spells: {}", list_or_dash(&summary.spells)));
        lines.push(String::new());
        lines.push(self.progression_for_class(&class_id)?);
        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            vec![
                format!("progression {class_id}"),
                format!("spells {class_id}"),
                format!("abilities {class_id}"),
                format!("show classes.{class_id}"),
            ],
        ))
    }

    fn progression(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        if let Some(class_id) = args.first() {
            let classes = table_at_path(&self.root, &["classes"])
                .ok_or_else(|| "This ruleset has no classes.".to_string())?;
            let (class_id, _) = table_entry_ci(classes, class_id)
                .ok_or_else(|| unknown_id("class", class_id, classes))?;
            return Ok(RulesetHelpResponse::new(
                self.progression_for_class(&class_id)?,
                vec![
                    format!("unlocks {class_id}"),
                    format!("spells {class_id}"),
                    format!("abilities {class_id}"),
                ],
            ));
        }

        let mut lines = vec!["Progression".into()];
        if let Some(table) = table_at_path(&self.root, &["progression", "xp_table"]) {
            lines.push("XP required by level:".into());
            let mut levels = table
                .keys()
                .filter_map(|key| level_number(key).map(|level| (level, key)))
                .collect::<Vec<_>>();
            levels.sort_by_key(|(level, _)| *level);
            for (level, _) in levels {
                if let Some(xp) = ruleset_xp_for_level(&self.root, level) {
                    lines.push(format!("  Level {level:<2} {xp} XP"));
                }
            }
        }
        let classes = table_at_path(&self.root, &["classes"]);
        let class_ids = classes.map(sorted_keys).unwrap_or_default();
        if !class_ids.is_empty() {
            lines.push(String::new());
            lines.push("Class timelines:".into());
            for class_id in &class_ids {
                let unlock_count = class_unlocks(&self.root, class_id).len();
                lines.push(format!("  {class_id:<16} {unlock_count} unlock levels"));
            }
        }
        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            class_ids
                .into_iter()
                .map(|class_id| format!("progression {class_id}"))
                .collect(),
        ))
    }

    fn progression_for_class(&self, class_id: &str) -> Result<String, String> {
        let classes = table_at_path(&self.root, &["classes"])
            .ok_or_else(|| "This ruleset has no classes.".to_string())?;
        let (_, class_value) = table_entry_ci(classes, class_id)
            .ok_or_else(|| unknown_id("class", class_id, classes))?;
        let class = class_value
            .as_table()
            .ok_or_else(|| format!("Class `{class_id}` is not a table."))?;
        let mut lines = vec![format!("{class_id} progression")];

        if let Some(level) = class
            .get("progression")
            .and_then(Value::as_table)
            .and_then(|progression| progression.get("level"))
            .and_then(Value::as_table)
        {
            if let Some(gain) = number_at(level, "primary_attribute_gain") {
                lines.push(format!("Primary attribute gain per level: {gain}"));
            }
            if let Some(gains) = level.get("resource_gains").and_then(Value::as_array) {
                lines.push("Resource growth per level:".into());
                for gain in gains.iter().filter_map(Value::as_table) {
                    let attribute = table_string(gain, "attribute").unwrap_or_else(|| "?".into());
                    let maximum = table_string(gain, "maximum_attribute")
                        .unwrap_or_else(|| attribute.clone());
                    let value = number_at(gain, "per_level").unwrap_or(0.0);
                    lines.push(format!("  {attribute}/{maximum}: +{value}"));
                }
            }
        }

        let unlocks = class_unlocks(&self.root, class_id);
        lines.push("Unlock timeline:".into());
        if unlocks.is_empty() {
            lines.push("  <none>".into());
        } else {
            for (level, grants) in unlocks {
                let xp = ruleset_xp_for_level(&self.root, level)
                    .map(|xp| format!(", {xp} XP"))
                    .unwrap_or_default();
                lines.push(format!("  Level {level}{xp}"));
                for (kind, ids) in grants {
                    lines.push(format!("    {}: {}", title_case(&kind), list_or_dash(&ids)));
                }
            }
        }
        Ok(lines.join("\n"))
    }

    fn unlocks(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let Some(requested) = args.first() else {
            return Err("Usage: unlocks <class> [level]".into());
        };
        let classes = table_at_path(&self.root, &["classes"])
            .ok_or_else(|| "This ruleset has no classes.".to_string())?;
        let (class_id, _) = table_entry_ci(classes, requested)
            .ok_or_else(|| unknown_id("class", requested, classes))?;
        let requested_level = args
            .get(1)
            .map(|value| {
                value
                    .parse::<u32>()
                    .map_err(|_| format!("Level `{value}` is not a positive integer."))
            })
            .transpose()?;
        let unlocks = class_unlocks(&self.root, &class_id);
        let mut lines = vec![format!("{class_id} unlocks")];
        for (level, grants) in unlocks {
            if requested_level.is_some_and(|requested| level != requested) {
                continue;
            }
            lines.push(format!("  Level {level}"));
            for (kind, ids) in grants {
                lines.push(format!("    {}: {}", title_case(&kind), list_or_dash(&ids)));
            }
        }
        if lines.len() == 1 {
            lines.push(match requested_level {
                Some(level) => format!("  No unlocks at level {level}."),
                None => "  <none>".into(),
            });
        }
        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            vec![format!("progression {class_id}")],
        ))
    }

    fn class_grants(&self, args: &[String], kind: &str) -> Result<RulesetHelpResponse, String> {
        let classes = table_at_path(&self.root, &["classes"]);
        let requested_class = args.first().map(String::as_str);
        let canonical_class = if let Some(requested) = requested_class {
            let table = classes.ok_or_else(|| "This ruleset has no classes.".to_string())?;
            Some(
                table_entry_ci(table, requested)
                    .map(|(id, _)| id)
                    .ok_or_else(|| unknown_id("class", requested, table))?,
            )
        } else {
            None
        };

        let definitions = table_at_path(&self.root, &[kind])
            .ok_or_else(|| format!("This ruleset has no {kind}."))?;
        let mut grants_by_id: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
        if let Some(classes) = classes {
            for class_id in sorted_keys(classes) {
                if canonical_class
                    .as_deref()
                    .is_some_and(|wanted| wanted != class_id)
                {
                    continue;
                }
                for (level, grants) in class_unlocks(&self.root, &class_id) {
                    for id in grants.get(kind).into_iter().flatten() {
                        grants_by_id
                            .entry(id.clone())
                            .or_default()
                            .push((class_id.clone(), level));
                    }
                }
            }
        }

        let title = canonical_class
            .as_deref()
            .map(|class| format!("{} for {class}", title_case(kind)))
            .unwrap_or_else(|| title_case(kind));
        let mut lines = vec![title];
        for id in sorted_keys(definitions) {
            let definition = definitions.get(&id).and_then(Value::as_table);
            let name = definition
                .and_then(|table| table_string(table, "name"))
                .unwrap_or_else(|| id.clone());
            let grants = grants_by_id.get(&id).cloned().unwrap_or_default();
            if canonical_class.is_some() && grants.is_empty() {
                continue;
            }
            let grant_text = if grants.is_empty() {
                "not granted by a class unlock".into()
            } else {
                grants
                    .iter()
                    .map(|(class, level)| format!("{class} level {level}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            lines.push(format!("  {name} ({id}) — {grant_text}"));
        }
        if lines.len() == 1 {
            lines.push("  <none>".into());
        }
        let singular = kind.trim_end_matches('s');
        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            sorted_keys(definitions)
                .into_iter()
                .take(12)
                .map(|id| format!("{singular} {id}"))
                .collect(),
        ))
    }

    fn item(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let items = table_at_path(&self.root, &["items"])
            .ok_or_else(|| "This ruleset has no items.".to_string())?;
        let Some(requested) = args.first() else {
            let mut lines = vec!["Items".into()];
            let mut suggestions = Vec::new();
            for group in sorted_keys(items) {
                let ids = items
                    .get(&group)
                    .and_then(Value::as_table)
                    .map(sorted_keys)
                    .unwrap_or_default();
                lines.push(format!("  {} ({})", title_case(&group), ids.len()));
                for id in ids.iter().take(8) {
                    lines.push(format!("    {id}"));
                    suggestions.push(format!("item {id}"));
                }
            }
            return Ok(
                RulesetHelpResponse::new(lines.join("\n"), suggestions).with_items(
                    items
                        .values()
                        .filter_map(Value::as_table)
                        .flat_map(sorted_keys)
                        .collect(),
                ),
            );
        };

        for group in sorted_keys(items) {
            let Some(group_table) = items.get(&group).and_then(Value::as_table) else {
                continue;
            };
            if let Some((id, value)) = table_entry_ci(group_table, requested) {
                let table = value
                    .as_table()
                    .ok_or_else(|| format!("Item `{id}` is not a table."))?;
                let path = format!("items.{group}.{id}");
                let mut output = describe_table(&path, table);
                if table.get("damage").and_then(Value::as_table).is_some() {
                    output.push_str(&format!(
                        "\n\nTry `calc weapon {id}` or `calc damage weapon {id}`."
                    ));
                }
                let mut suggestions = vec![format!("show {path}")];
                if group == "weapons" && table.get("damage").and_then(Value::as_table).is_some() {
                    suggestions.push(format!("calc weapon {id}"));
                    suggestions.push(format!("calc damage weapon {id}"));
                }
                return Ok(RulesetHelpResponse::new(output, suggestions).with_items(vec![id]));
            }
        }
        let ids = items
            .values()
            .filter_map(Value::as_table)
            .flat_map(sorted_keys)
            .collect::<Vec<_>>();
        Err(format!(
            "Unknown item `{requested}`. Valid item ids include: {}",
            ids.into_iter().take(30).collect::<Vec<_>>().join(", ")
        ))
    }

    fn recipe(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let recipes = table_at_path(&self.root, &["recipes"])
            .ok_or_else(|| "This ruleset has no recipes.".to_string())?;
        let Some(requested) = args.first() else {
            return self.section_or_entry(&["recipes"], args);
        };
        let (id, value) = table_entry_ci(recipes, requested)
            .ok_or_else(|| unknown_id("recipe", requested, recipes))?;
        let recipe = value
            .as_table()
            .ok_or_else(|| format!("Recipe `{id}` is not a table."))?;
        let mut lines = vec![format!(
            "Recipe: {} ({id})",
            table_string(recipe, "name").unwrap_or_else(|| id.clone())
        )];
        if let Some(description) = table_string(recipe, "description") {
            lines.push(description);
        }
        for key in ["skill", "required_skill", "duration"] {
            if let Some(value) = recipe.get(key) {
                lines.push(format!("{}: {}", title_case(key), value_inline(value)));
            }
        }
        for key in ["consumes", "produces"] {
            lines.push(format!("{}:", title_case(key)));
            let entries = recipe
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_table)
                .map(|entry| {
                    let item = table_string(entry, "item").unwrap_or_else(|| "?".into());
                    let quantity = entry
                        .get("quantity")
                        .map(value_inline)
                        .unwrap_or_else(|| "1".into());
                    format!("  {item} x{quantity}")
                })
                .collect::<Vec<_>>();
            if entries.is_empty() {
                lines.push("  <none>".into());
            } else {
                lines.extend(entries);
            }
        }
        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            vec![format!("show recipes.{id}"), "examples crafting".into()],
        ))
    }

    fn attributes(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        if let Some(stat) = args.first() {
            let derived = table_at_path(&self.root, &["derived_stats"])
                .ok_or_else(|| "This ruleset has no derived stats.".to_string())?;
            let (id, value) = table_entry_ci(derived, stat)
                .ok_or_else(|| unknown_id("derived stat", stat, derived))?;
            let table = value
                .as_table()
                .ok_or_else(|| format!("Derived stat `{id}` is not a table."))?;
            return Ok(RulesetHelpResponse::new(
                describe_table(&format!("derived_stats.{id}"), table),
                vec![
                    format!("calc derived {id}"),
                    format!("show derived_stats.{id}"),
                ],
            ));
        }
        let mut lines = vec!["Attributes".into()];
        if let Some(attributes) = table_at_path(&self.root, &["attributes"]) {
            for key in sorted_keys(attributes) {
                if let Some(value) = attributes.get(&key) {
                    lines.push(format!("  {}: {}", title_case(&key), value_inline(value)));
                }
            }
        }
        if let Some(derived) = table_at_path(&self.root, &["derived_stats"]) {
            lines.push(String::new());
            lines.push("Derived stats:".into());
            for id in sorted_keys(derived) {
                let formula = derived
                    .get(&id)
                    .and_then(Value::as_table)
                    .and_then(|table| table_string(table, "formula"))
                    .unwrap_or_else(|| "?".into());
                lines.push(format!("  {id} = {formula}"));
            }
        }
        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            vec![
                "list derived_stats".into(),
                "calc derived DMG STR=14".into(),
            ],
        ))
    }

    fn combat(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let combat = table_at_path(&self.root, &["combat"])
            .ok_or_else(|| "This ruleset has no combat section.".to_string())?;
        if let Some(kind) = args.first() {
            let kinds = combat
                .get("kinds")
                .and_then(Value::as_table)
                .ok_or_else(|| "This ruleset has no damage kinds.".to_string())?;
            let (id, value) = table_entry_ci(kinds, kind)
                .ok_or_else(|| unknown_id("damage kind", kind, kinds))?;
            let table = value
                .as_table()
                .ok_or_else(|| format!("Damage kind `{id}` is not a table."))?;
            return Ok(RulesetHelpResponse::new(
                describe_table(&format!("combat.kinds.{id}"), table),
                vec!["examples damage".into(), format!("show combat.kinds.{id}")],
            ));
        }
        let mut lines = vec!["Combat".into()];
        for (key, value) in combat.iter().filter(|(_, value)| !value.is_table()) {
            lines.push(format!("  {}: {}", title_case(key), value_inline(value)));
        }
        if let Some(kinds) = combat.get("kinds").and_then(Value::as_table) {
            lines.push("Damage kinds:".into());
            for id in sorted_keys(kinds) {
                let description = kinds
                    .get(&id)
                    .and_then(Value::as_table)
                    .and_then(|table| table_string(table, "description"))
                    .unwrap_or_default();
                lines.push(format!("  {id:<12} {description}"));
            }
        }
        if let Some(unarmed) = combat.get("unarmed_damage").and_then(Value::as_table) {
            lines.push("Unarmed damage:".into());
            lines.extend(
                scalar_lines(unarmed)
                    .into_iter()
                    .map(|line| format!("  {line}")),
            );
        }
        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            vec![
                "examples damage".into(),
                "help calc".into(),
                "list combat.kinds".into(),
            ],
        ))
    }

    fn calculate(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let Some(kind) = args.first().map(|arg| arg.to_ascii_lowercase()) else {
            return Err(self.command_help(Some("calc")));
        };
        match kind.as_str() {
            "weapon" => {
                let id = args
                    .get(1)
                    .ok_or_else(|| "Usage: calc weapon <id> [ATTR=VALUE ...]".to_string())?;
                let attributes = parse_attributes(&args[2..])?;
                let summary =
                    summarize_weapon_damage(&self.root, id, &roll_attributes(&attributes))?;
                Ok(RulesetHelpResponse::new(
                    format_roll(&format!("Weapon: {id}"), &summary),
                    vec![format!("calc damage weapon {id}"), format!("item {id}")],
                ))
            }
            "spell" => {
                let id = args
                    .get(1)
                    .ok_or_else(|| "Usage: calc spell <id> [ATTR=VALUE ...]".to_string())?;
                let attributes = parse_attributes(&args[2..])?;
                let (roll_kind, summary) =
                    summarize_spell_roll(&self.root, id, &roll_attributes(&attributes))?;
                Ok(RulesetHelpResponse::new(
                    format_roll(&format!("Spell: {id} ({})", roll_kind.label()), &summary),
                    vec![format!("calc damage spell {id}"), format!("spell {id}")],
                ))
            }
            "ability" => {
                let id = args
                    .get(1)
                    .ok_or_else(|| "Usage: calc ability <id> [ATTR=VALUE ...]".to_string())?;
                let attributes = parse_attributes(&args[2..])?;
                let (path, summary) = self.ability_roll(id, &roll_attributes(&attributes))?;
                Ok(RulesetHelpResponse::new(
                    format_roll(&format!("Ability: {id}"), &summary),
                    vec![format!("calc damage ability {id}"), format!("show {path}")],
                ))
            }
            "roll" => {
                let path = args
                    .get(1)
                    .ok_or_else(|| "Usage: calc roll <path> [ATTR=VALUE ...]".to_string())?;
                let attributes = parse_attributes(&args[2..])?;
                let parts = path_parts(path);
                let summary = summarize_roll_path(
                    &self.root,
                    &parts.iter().map(String::as_str).collect::<Vec<_>>(),
                    &roll_attributes(&attributes),
                )?;
                Ok(RulesetHelpResponse::new(
                    format_roll(path, &summary),
                    vec![format!("show {path}"), format!("calc damage roll {path}")],
                ))
            }
            "derived" => self.calculate_derived(&args[1..]),
            "damage" => self.calculate_damage(&args[1..]),
            _ => Err(format!(
                "Unknown calculation `{kind}`.\n\n{}",
                self.command_help(Some("calc"))
            )),
        }
    }

    fn ability_roll(
        &self,
        requested: &str,
        attributes: &RulesetAttributeMap,
    ) -> Result<(String, RulesetRollSummary), String> {
        let abilities = table_at_path(&self.root, &["abilities"])
            .ok_or_else(|| "This ruleset has no abilities.".to_string())?;
        let (id, ability) = table_entry_ci(abilities, requested)
            .ok_or_else(|| unknown_id("ability", requested, abilities))?;
        let ability = ability
            .as_table()
            .ok_or_else(|| format!("Ability `{id}` is not a table."))?;
        for roll_key in ["damage", "healing"] {
            if ability.get(roll_key).and_then(Value::as_table).is_some() {
                let path = format!("abilities.{id}.{roll_key}");
                let summary =
                    summarize_roll_path(&self.root, &["abilities", &id, roll_key], attributes)?;
                return Ok((path, summary));
            }
        }
        Err(format!("Ability `{id}` has no damage or healing roll."))
    }

    fn calculate_derived(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let requested = args
            .first()
            .ok_or_else(|| "Usage: calc derived <stat> [base=VALUE ATTR=VALUE ...]".to_string())?;
        let derived = table_at_path(&self.root, &["derived_stats"])
            .ok_or_else(|| "This ruleset has no derived stats.".to_string())?;
        let (id, value) = table_entry_ci(derived, requested)
            .ok_or_else(|| unknown_id("derived stat", requested, derived))?;
        let table = value
            .as_table()
            .ok_or_else(|| format!("Derived stat `{id}` is not a table."))?;
        let formula = table_string(table, "formula")
            .ok_or_else(|| format!("Derived stat `{id}` has no formula."))?;
        let supplied = parse_attributes(&args[1..])?;
        let defaults = table_at_path(&self.root, &["attributes", "defaults"]);
        let resolve = |name: &str| {
            supplied
                .get(name)
                .copied()
                .or_else(|| supplied.get(&name.to_ascii_uppercase()).copied())
                .or_else(|| defaults.and_then(|table| number_at(table, name)))
                .unwrap_or(0.0)
        };
        let mut result = evaluate_formula(&formula, resolve)
            .ok_or_else(|| format!("Could not evaluate formula `{formula}`."))?;
        if let Some(minimum) = number_at(table, "minimum") {
            result = result.max(minimum);
        }
        if let Some(maximum) = number_at(table, "maximum") {
            result = result.min(maximum);
        }
        let identifiers = crate::formula_identifiers(&formula);
        let values = identifiers
            .iter()
            .map(|name| format!("{name}={}", resolve(name)))
            .collect::<Vec<_>>();
        Ok(RulesetHelpResponse::new(
            format!(
                "Derived stat: {id}\nFormula: {formula}\nInputs: {}\nResult: {}",
                list_or_dash(&values),
                format_number(result)
            ),
            vec![
                format!("attributes {id}"),
                format!("show derived_stats.{id}"),
            ],
        ))
    }

    fn calculate_damage(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let source_kind = args
            .first()
            .map(|value| value.to_ascii_lowercase())
            .ok_or_else(|| {
                "Usage: calc damage <weapon|spell|ability|roll> <id|path> [values...]".to_string()
            })?;
        let source_id = args.get(1).ok_or_else(|| {
            "Usage: calc damage <weapon|spell|ability|roll> <id|path> [values...]".to_string()
        })?;
        let attributes = parse_attributes(&args[2..])?;
        let roll_attributes = roll_attributes(&attributes);
        let (label, mut summary, source_path) = match source_kind.as_str() {
            "weapon" => (
                format!("weapon {source_id}"),
                summarize_weapon_damage(&self.root, source_id, &roll_attributes)?,
                format!("items.weapons.{source_id}.damage"),
            ),
            "spell" => {
                let (_, summary) = summarize_spell_roll(&self.root, source_id, &roll_attributes)?;
                let key = if table_at_path(&self.root, &["spells", source_id, "damage"]).is_some() {
                    "damage"
                } else {
                    "healing"
                };
                (
                    format!("spell {source_id}"),
                    summary,
                    format!("spells.{source_id}.{key}"),
                )
            }
            "ability" => {
                let (path, summary) = self.ability_roll(source_id, &roll_attributes)?;
                (format!("ability {source_id}"), summary, path)
            }
            "roll" => {
                let parts = path_parts(source_id);
                let refs = parts.iter().map(String::as_str).collect::<Vec<_>>();
                (
                    format!("roll {source_id}"),
                    summarize_roll_path(&self.root, &refs, &roll_attributes)?,
                    source_id.clone(),
                )
            }
            _ => return Err("Damage source must be weapon, spell, ability, or roll.".into()),
        };

        let mut lines = vec![format!("Full damage projection: {label}")];
        lines.push(format!(
            "Source roll: {} + {} = {} to {}, average {:.2}",
            summary.spec.roll,
            format_number(summary.total_bonus),
            format_number(summary.minimum),
            format_number(summary.maximum),
            summary.average
        ));

        if source_kind == "weapon" {
            let quality = scoped_attribute(&attributes, "source", "quality").unwrap_or(100.0);
            let condition = scoped_attribute(&attributes, "source", "condition").unwrap_or(100.0);
            let multiplier = item_damage_multiplier(quality) * item_damage_multiplier(condition);
            summary.minimum = scale_item_damage(summary.minimum, multiplier);
            summary.maximum = scale_item_damage(summary.maximum, multiplier);
            summary.average = scale_item_damage(summary.average, multiplier);
            lines.push(format!(
                "Weapon quality/condition: {}% / {}% => x{multiplier:.4}",
                format_number(quality),
                format_number(condition)
            ));
        }

        let mut values = [
            summary.minimum.round(),
            summary.maximum.round(),
            summary.average.round(),
        ];
        let damage_kind = summary
            .spec
            .damage_kind
            .clone()
            .or_else(|| {
                value_at_path_ci(&self.root, &path_parts(&source_path))
                    .and_then(Value::as_table)
                    .and_then(|table| table_string(table, "damage_kind"))
            })
            .unwrap_or_else(|| {
                table_at_path(&self.root, &["combat"])
                    .and_then(|table| table_string(table, "default_damage_kind"))
                    .unwrap_or_else(|| "physical".into())
            });
        lines.push(format!("Damage kind: {damage_kind}"));

        if let Some(kind) = table_at_path(&self.root, &["combat", "kinds", &damage_kind]) {
            if let Some(outgoing) = kind.get("damage").and_then(Value::as_table) {
                let addition = structured_modifier(outgoing, &attributes, "attacker", "source");
                for value in &mut values {
                    *value = (*value + addition).max(0.0).round();
                }
                lines.push(format!("Outgoing modifiers: {:+}", format_number(addition)));
            }
            if let Some(reduction) = kind.get("reduction").and_then(Value::as_table) {
                let reduction_value = structured_reduction(reduction, &attributes);
                for value in &mut values {
                    *value = (*value - reduction_value).max(0.0).round();
                }
                lines.push(format!(
                    "Defender reduction: -{}",
                    format_number(reduction_value)
                ));
            }
        }
        lines.push(format!(
            "Final damage: {} to {}, projected average {}",
            format_number(values[0]),
            format_number(values[1]),
            format_number(values[2])
        ));
        lines.push(String::new());
        lines.push("This deterministic projection uses the effective structured combat tables. Live race/class overrides and transient runtime effects must be supplied through the effective attribute inputs.".into());

        Ok(RulesetHelpResponse::new(
            lines.join("\n"),
            vec![
                format!("show {source_path}"),
                format!("combat {damage_kind}"),
            ],
        ))
    }

    fn examples(&self, args: &[String]) -> Result<RulesetHelpResponse, String> {
        let topic = args.first().map(|arg| arg.to_ascii_lowercase());
        match topic.as_deref() {
            None => Ok(RulesetHelpResponse::new(
                "Generated example families\nChoose a formatted example family below.",
                vec![
                    "examples damage".into(),
                    "examples progression".into(),
                    "examples crafting".into(),
                    "examples discovery".into(),
                    "examples all".into(),
                ],
            )),
            Some("damage") => self.damage_examples(),
            Some("progression") => self.progression_examples(),
            Some("crafting") => self.crafting_examples(),
            Some("discovery") => Ok(RulesetHelpResponse::new(
                "Discovery examples\nBrowse and search the effective ruleset with these queries.",
                vec![
                    "list combat.kinds".into(),
                    "paths classes".into(),
                    "show attributes.roles".into(),
                    "search cooldown".into(),
                ],
            )),
            Some("all") => {
                let mut commands = self.damage_examples()?.commands;
                commands.extend(
                    self.progression_examples()?
                        .suggestions
                        .into_iter()
                        .map(|command| {
                            RulesetHelpCommand::new(command, "Class progression example")
                        }),
                );
                commands.extend(
                    self.crafting_examples()?
                        .suggestions
                        .into_iter()
                        .map(|command| RulesetHelpCommand::new(command, "Crafting example")),
                );
                Ok(RulesetHelpResponse::new(
                    "All generated examples\nDamage, progression, and crafting queries from the effective ruleset.",
                    Vec::new(),
                )
                .with_command_list("Generated queries", commands))
            }
            Some(other) => Err(format!("Unknown example family `{other}`. Try `examples`.")),
        }
    }

    fn damage_examples(&self) -> Result<RulesetHelpResponse, String> {
        let mut examples: BTreeMap<String, String> = BTreeMap::new();
        for weapon in &self.catalog.weapons {
            if let Some(table) = table_at_path(&self.root, &["items", "weapons", weapon, "damage"])
            {
                let kind = table_string(table, "damage_kind").unwrap_or_else(|| "physical".into());
                examples.entry(kind).or_insert_with(|| {
                    format!("calc damage weapon {weapon} STR=14 defender.ARMOR=2 equipment.ARMOR=1")
                });
            }
        }
        for spell in &self.catalog.spells {
            if let Ok((_, summary)) =
                summarize_spell_roll(&self.root, spell, &RulesetAttributeMap::new())
            {
                let kind = summary.spec.damage_kind.unwrap_or_else(|| "spell".into());
                examples.entry(kind).or_insert_with(|| {
                    format!(
                        "calc damage spell {spell} WIS=14 INT=12 attacker.POWER=2 defender.RESIST=1"
                    )
                });
            }
        }
        for ability in &self.catalog.abilities {
            if let Ok((_, summary)) = self.ability_roll(ability, &RulesetAttributeMap::new()) {
                let kind = summary
                    .spec
                    .damage_kind
                    .unwrap_or_else(|| "physical".into());
                examples.entry(kind).or_insert_with(|| {
                    format!("calc damage ability {ability} STR=14 DEX=14 defender.ARMOR=2")
                });
            }
        }
        let output = if examples.is_empty() {
            "Damage examples\nNo damage roll tables are present in this ruleset."
        } else {
            "Damage examples\nGenerated from the effective ruleset's present roll tables."
        };
        let commands = examples
            .into_iter()
            .map(|(kind, command)| {
                RulesetHelpCommand::new(command, format!("{kind} damage example"))
            })
            .collect();
        Ok(RulesetHelpResponse::new(output, Vec::new())
            .with_command_list("Damage calculations", commands))
    }

    fn progression_examples(&self) -> Result<RulesetHelpResponse, String> {
        let commands = self
            .catalog
            .classes
            .iter()
            .map(|class| format!("progression {class}"))
            .collect::<Vec<_>>();
        Ok(RulesetHelpResponse::new(
            "Progression examples\nOne formatted query for each class timeline.",
            commands,
        ))
    }

    fn crafting_examples(&self) -> Result<RulesetHelpResponse, String> {
        let commands = self
            .catalog
            .recipes
            .iter()
            .map(|recipe| format!("recipe {recipe}"))
            .collect::<Vec<_>>();
        Ok(RulesetHelpResponse::new(
            "Crafting examples\nOne formatted query for each ruleset recipe.",
            commands,
        ))
    }
}

fn section_command(path: &[&str]) -> &'static str {
    match path {
        ["races"] => "race",
        ["classes"] => "class",
        ["professions"] => "profession",
        ["skills"] => "skill",
        ["resources"] => "resource",
        ["actions"] => "action",
        ["abilities"] => "ability",
        ["spells"] => "spell",
        ["conditions"] => "condition",
        ["recipes"] => "recipe",
        ["invocation_schemes"] => "invocation",
        ["fx", "presets"] => "fx",
        _ => "show",
    }
}

fn split_command(source: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in source.trim().chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if quote.is_some() {
        return Err("Unclosed quote in help command.".into());
    }
    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

fn path_parts(path: &str) -> Vec<String> {
    path.split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn required_path(args: &[String], usage: &str) -> Result<Vec<String>, String> {
    args.first()
        .map(|path| path_parts(path))
        .filter(|path| !path.is_empty())
        .ok_or_else(|| usage.to_string())
}

fn table_at_path<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Table> {
    ruleset_table_at_path(root, path)
}

fn value_at_path<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Value> {
    let mut value = None;
    for (index, part) in path.iter().enumerate() {
        value = if index == 0 {
            root.get(*part)
        } else {
            value?.as_table()?.get(*part)
        };
    }
    value
}

fn value_at_path_ci<'a>(root: &'a Table, path: &[String]) -> Option<&'a Value> {
    let mut table = root;
    let mut value = None;
    for (index, requested) in path.iter().enumerate() {
        let (_, found) = table_entry_ci(table, requested)?;
        value = Some(found);
        if index + 1 < path.len() {
            table = found.as_table()?;
        }
    }
    value
}

fn table_entry_ci<'a>(table: &'a Table, requested: &str) -> Option<(String, &'a Value)> {
    table
        .get_key_value(requested)
        .map(|(key, value)| (key.clone(), value))
        .or_else(|| {
            table
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(requested))
                .map(|(key, value)| (key.clone(), value))
        })
}

fn sorted_keys(table: &Table) -> Vec<String> {
    let mut keys = table.keys().cloned().collect::<Vec<_>>();
    keys.sort_by_key(|key| key.to_ascii_lowercase());
    keys
}

fn table_string(table: &Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn number_at(table: &Table, key: &str) -> Option<f32> {
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
        Value::Float(value) => format_number(*value as f32),
        Value::Boolean(value) => value.to_string(),
        Value::Datetime(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(value_inline)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Table(table) => format!("{{ {} }}", sorted_keys(table).join(", ")),
    }
}

fn title_case(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn indent_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        "  <none>".into()
    } else {
        lines
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn list_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".into()
    } else {
        values.join(", ")
    }
}

fn scalar_lines(table: &Table) -> Vec<String> {
    let mut lines = table
        .iter()
        .filter(|(_, value)| !value.is_table())
        .map(|(key, value)| format!("{}: {}", title_case(key), value_inline(value)))
        .collect::<Vec<_>>();
    lines.sort_by_key(|line| line.to_ascii_lowercase());
    lines
}

fn describe_table(path: &str, table: &Table) -> String {
    let mut lines = Vec::new();
    let name = table_string(table, "name");
    lines.push(match name {
        Some(name) => format!("{name}\nPath: {path}"),
        None => path.to_string(),
    });
    if let Some(description) = table_string(table, "description") {
        lines.push(description);
    }
    let scalars = scalar_lines(table)
        .into_iter()
        .filter(|line| !line.starts_with("Name:") && !line.starts_with("Description:"))
        .collect::<Vec<_>>();
    if !scalars.is_empty() {
        lines.push(String::new());
        lines.extend(scalars);
    }
    let children = table
        .iter()
        .filter(|(_, value)| value.is_table())
        .map(|(key, value)| {
            let child = value.as_table().unwrap();
            let summary = scalar_lines(child);
            if summary.is_empty() {
                format!("  {key}")
            } else {
                format!("  {key}: {}", summary.join("; "))
            }
        })
        .collect::<Vec<_>>();
    if !children.is_empty() {
        lines.push(String::new());
        lines.push("Nested rules:".into());
        lines.extend(children);
    }
    lines.join("\n")
}

fn unknown_id(kind: &str, requested: &str, table: &Table) -> String {
    format!(
        "Unknown {kind} `{requested}`. Valid ids: {}",
        sorted_keys(table).join(", ")
    )
}

fn collect_paths_from_table(table: &Table, prefix: &str, output: &mut Vec<String>) {
    for key in sorted_keys(table) {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        output.push(path.clone());
        if let Some(child) = table.get(&key).and_then(Value::as_table) {
            collect_paths_from_table(child, &path, output);
        }
    }
}

fn collect_search_matches(
    table: &Table,
    prefix: &str,
    query: &str,
    limit: usize,
    output: &mut Vec<String>,
) {
    if output.len() >= limit {
        return;
    }
    for key in sorted_keys(table) {
        if output.len() >= limit {
            return;
        }
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        let Some(value) = table.get(&key) else {
            continue;
        };
        if let Some(child) = value.as_table() {
            if path.to_ascii_lowercase().contains(query) {
                output.push(format!("{path} = {{ {} }}", sorted_keys(child).join(", ")));
            }
            collect_search_matches(child, &path, query, limit, output);
        } else {
            let rendered = value_inline(value);
            if path.to_ascii_lowercase().contains(query)
                || rendered.to_ascii_lowercase().contains(query)
            {
                output.push(format!("{path} = {rendered}"));
            }
        }
    }
}

fn level_number(key: &str) -> Option<u32> {
    key.strip_prefix("level_")
        .unwrap_or(key)
        .parse::<u32>()
        .ok()
}

fn string_array(table: &Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn class_unlocks(root: &Table, class_id: &str) -> BTreeMap<u32, BTreeMap<String, Vec<String>>> {
    let Some(unlocks) = table_at_path(root, &["classes", class_id, "unlocks"]) else {
        return BTreeMap::new();
    };
    let mut output = BTreeMap::new();
    for (level_key, value) in unlocks {
        let Some(level) = level_number(level_key) else {
            continue;
        };
        let Some(grants) = value.as_table() else {
            continue;
        };
        let mut kinds = BTreeMap::new();
        for kind in ["abilities", "spells", "recipes", "actions"] {
            let values = string_array(grants, kind);
            if !values.is_empty() {
                kinds.insert(kind.to_string(), values);
            }
        }
        for (kind, value) in grants {
            if !kinds.contains_key(kind) {
                let values = value
                    .as_array()
                    .map(|values| values.iter().map(value_inline).collect::<Vec<_>>())
                    .unwrap_or_default();
                if !values.is_empty() {
                    kinds.insert(kind.clone(), values);
                }
            }
        }
        output.insert(level, kinds);
    }
    output
}

fn parse_attributes(args: &[String]) -> Result<RulesetAttributeMap, String> {
    let mut attributes = RulesetAttributeMap::new();
    for raw in args {
        let Some((key, value)) = raw.split_once('=') else {
            return Err(format!(
                "Calculation input `{raw}` must use NAME=VALUE syntax."
            ));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("Calculation input `{raw}` has an empty name."));
        }
        let value = value
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("Calculation input `{raw}` has a non-numeric value."))?;
        attributes.insert(key.to_string(), value);
    }
    Ok(attributes)
}

fn roll_attributes(attributes: &RulesetAttributeMap) -> RulesetAttributeMap {
    let mut output = attributes.clone();
    for (key, value) in attributes {
        if let Some(key) = key.strip_prefix("attacker.") {
            output.entry(key.to_string()).or_insert(*value);
        }
    }
    output
}

fn format_roll(label: &str, summary: &RulesetRollSummary) -> String {
    let attribute = summary
        .spec
        .bonus_attribute
        .as_deref()
        .map(|attribute| {
            format!(
                "{attribute}={} => +{} every {}",
                format_number(summary.attribute_value),
                format_number(summary.attribute_bonus),
                format_number(summary.spec.bonus_every)
            )
        })
        .unwrap_or_else(|| "none".into());
    let mut lines = vec![
        label.to_string(),
        format!("Roll: {}", summary.spec.roll),
        format!("Fixed bonus: {}", format_number(summary.spec.bonus)),
        format!("Attribute bonus: {attribute}"),
        format!("Total bonus: {}", format_number(summary.total_bonus)),
        format!("Minimum: {}", format_number(summary.minimum)),
        format!("Maximum: {}", format_number(summary.maximum)),
        format!("Average: {:.2}", summary.average),
    ];
    if let Some(kind) = summary.spec.damage_kind.as_deref() {
        lines.push(format!("Damage kind: {kind}"));
    }
    lines.join("\n")
}

fn format_number(value: f32) -> String {
    if (value - value.round()).abs() < f32::EPSILON {
        format!("{:.0}", value)
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn scoped_attribute(attributes: &RulesetAttributeMap, scope: &str, attribute: &str) -> Option<f32> {
    attributes
        .get(&format!("{scope}.{attribute}"))
        .copied()
        .or_else(|| {
            attributes
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(&format!("{scope}.{attribute}")))
                .map(|(_, value)| *value)
        })
        .or_else(|| {
            (scope == "attacker")
                .then(|| {
                    attributes.get(attribute).copied().or_else(|| {
                        attributes
                            .iter()
                            .find(|(key, _)| key.eq_ignore_ascii_case(attribute))
                            .map(|(_, value)| *value)
                    })
                })
                .flatten()
        })
}

fn structured_attribute_bonus(table: &Table, attributes: &RulesetAttributeMap, scope: &str) -> f32 {
    let Some(attribute) = table_string(table, "bonus_attribute") else {
        return 0.0;
    };
    let every = number_at(table, "bonus_every")
        .filter(|value| *value > 0.0)
        .unwrap_or(1.0);
    (scoped_attribute(attributes, scope, &attribute).unwrap_or(0.0) / every).floor()
}

fn structured_modifier(
    table: &Table,
    attributes: &RulesetAttributeMap,
    entity_scope: &str,
    source_scope: &str,
) -> f32 {
    let mut value = number_at(table, "bonus").unwrap_or(0.0);
    if let Some(attribute) = table_string(table, "source_bonus_attribute") {
        value += scoped_attribute(attributes, source_scope, &attribute).unwrap_or(0.0);
    }
    if let Some(attribute) = table_string(table, "attacker_bonus_attribute") {
        value += scoped_attribute(attributes, entity_scope, &attribute).unwrap_or(0.0);
    }
    value + structured_attribute_bonus(table, attributes, entity_scope)
}

fn structured_reduction(table: &Table, attributes: &RulesetAttributeMap) -> f32 {
    let mut value = number_at(table, "bonus").unwrap_or(0.0);
    if let Some(attribute) = table_string(table, "attribute") {
        value += scoped_attribute(attributes, "defender", &attribute).unwrap_or(0.0);
    }
    if let Some(attribute) = table_string(table, "equipped_armor_attribute") {
        value += scoped_attribute(attributes, "equipment", &attribute).unwrap_or(0.0);
    }
    value + structured_attribute_bonus(table, attributes, "defender")
}

fn item_damage_multiplier(percent: f32) -> f32 {
    0.75 + percent.round().clamp(1.0, 100.0) / 400.0
}

fn scale_item_damage(value: f32, multiplier: f32) -> f32 {
    if value > 0.0 {
        (value * multiplier).max(1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_RULES_OVERRIDE, latest_official_ruleset, merge_ruleset_sources};

    fn run(command: &str) -> RulesetHelpResponse {
        execute_ruleset_help(latest_official_ruleset(), command).unwrap()
    }

    #[test]
    fn intro_and_topics_are_derived_from_the_ruleset() {
        let intro = ruleset_help_intro(latest_official_ruleset()).unwrap();
        assert!(intro.output.contains("Eldiron Official Ruleset 1.0.0"));
        assert!(
            intro
                .commands
                .iter()
                .any(|entry| entry.command == "progression [class]")
        );

        let topics = run("topics");
        assert!(topics.output.contains("progression"));
        assert!(topics.output.contains("invocations"));
    }

    #[test]
    fn progression_reports_spell_unlock_levels() {
        let progression = run("progression Cleric");
        assert!(progression.output.contains("Level 1"));
        assert!(progression.output.contains("minor_heal"));
        assert!(progression.output.contains("Level 10"));
        assert!(progression.output.contains("sanctuary"));

        let spells = run("spells Cleric");
        assert!(
            spells
                .output
                .contains("Minor Heal (minor_heal) — Cleric level 1")
        );
        assert!(
            spells
                .output
                .contains("Sanctuary (sanctuary) — Cleric level 10")
        );
        assert!(!spells.output.contains("Fire Spark"));
    }

    #[test]
    fn generic_discovery_reaches_arbitrary_future_rules() {
        let source = merge_ruleset_sources(
            latest_official_ruleset(),
            r#"
[weather.moon_storm]
description = "A future ruleset domain."
intensity = 7
"#,
        )
        .unwrap();
        let paths = execute_ruleset_help(&source, "paths weather").unwrap();
        assert!(paths.output.contains("weather.moon_storm"));
        let search = execute_ruleset_help(&source, "search future ruleset").unwrap();
        assert!(search.output.contains("weather.moon_storm.description"));
        let show = execute_ruleset_help(&source, "show weather.moon_storm.intensity").unwrap();
        assert!(show.output.ends_with('7'));
    }

    #[test]
    fn command_discovery_only_offers_sections_present_now() {
        let source = r#"
[ruleset]
id = "minimal"
version = "1.0.0"
schema_version = "1"

[weather.clear]
description = "No progression system is present."
"#;
        let intro = ruleset_help_intro(source).unwrap();
        assert!(!intro.suggestions.iter().any(|entry| entry == "progression"));
        assert!(
            !intro
                .commands
                .iter()
                .any(|entry| entry.command == "progression [class]")
        );
        assert!(
            intro
                .commands
                .iter()
                .any(|entry| entry.command == "paths [prefix]")
        );
    }

    #[test]
    fn specialized_suggestions_use_public_command_aliases() {
        let abilities = run("ability");
        assert!(
            abilities
                .suggestions
                .iter()
                .all(|command| command.starts_with("ability "))
        );
        let invocations = run("invocation");
        assert!(
            invocations
                .suggestions
                .iter()
                .all(|command| command.starts_with("invocation "))
        );
    }

    #[test]
    fn calculations_use_effective_overrides() {
        let source = merge_ruleset_sources(
            latest_official_ruleset(),
            r#"
[items.weapons.hand_axe.damage]
roll = "2d8"
bonus = 5
bonus_attribute = "STR"
bonus_every = 4
damage_kind = "physical"
"#,
        )
        .unwrap();
        let response = execute_ruleset_help(&source, "calc weapon hand_axe STR=12").unwrap();
        assert!(response.output.contains("Roll: 2d8"));
        assert!(response.output.contains("Total bonus: 8"));
        assert!(response.output.contains("Minimum: 10"));
    }

    #[test]
    fn full_damage_projection_applies_quality_and_mitigation() {
        let response = run(
            "calc damage weapon hand_axe STR=14 source.quality=50 source.condition=50 defender.ARMOR=2 equipment.ARMOR=1",
        );
        assert!(
            response
                .output
                .contains("Weapon quality/condition: 50% / 50%")
        );
        assert!(response.output.contains("Defender reduction: -3"));
        assert!(response.output.contains("Final damage:"));
    }

    #[test]
    fn damage_examples_cover_present_damage_kinds() {
        let response = run("examples damage");
        assert!(!response.output.contains("calc damage"));
        assert_eq!(
            response.commands_title.as_deref(),
            Some("Damage calculations")
        );
        assert!(
            response
                .commands
                .iter()
                .any(|entry| entry.description == "physical damage example")
        );
        assert!(
            response
                .commands
                .iter()
                .any(|entry| entry.description == "arcane damage example")
        );
        assert!(
            response
                .commands
                .iter()
                .any(|entry| entry.description == "fire damage example")
        );
    }

    #[test]
    fn empty_override_constant_remains_valid_for_help() {
        let source =
            merge_ruleset_sources(latest_official_ruleset(), DEFAULT_RULES_OVERRIDE).unwrap();
        assert!(execute_ruleset_help(&source, "overview").is_ok());
    }

    #[test]
    fn clear_is_a_frontend_signal() {
        let response = run("clear");
        assert!(response.clear);
        assert!(response.output.is_empty());
    }
}
