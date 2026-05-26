use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};
use theframework::prelude::{TheColor, ThePalette};
use toml::{Table, Value};

pub const OFFICIAL_RULESET_ID: &str = "eldiron.official";
pub const OFFICIAL_RULESET_VERSION: &str = "1.0.0";
pub const OFFICIAL_RULESET_SCHEMA_VERSION: &str = "1";
pub const DEFAULT_RULESET_CONFIG: &str = r#"[ruleset]
id = "eldiron.official"
version = "1.0.0"
schema_version = "1"
source = "official"
update_policy = "compatible"
"#;
pub const DEFAULT_RULES_OVERRIDE: &str = r#"# Game / Rules is the project-level override layer for the official ruleset
# selected in Game / Settings.
#
# New projects use the bundled Eldiron Official Ruleset by default, so this file
# starts empty. Add changes here when this project intentionally changes the
# default rules.
"#;

const OFFICIAL_ELDIRON_V1_CORE: &str = include_str!("../../../rulesets/eldiron/v1/ruleset.toml");
const OFFICIAL_ELDIRON_V1_IDENTITY: &str =
    include_str!("../../../rulesets/eldiron/v1/identity.toml");
const OFFICIAL_ELDIRON_V1_ATTRIBUTES: &str =
    include_str!("../../../rulesets/eldiron/v1/attributes.toml");
const OFFICIAL_ELDIRON_V1_PROGRESSION: &str =
    include_str!("../../../rulesets/eldiron/v1/progression.toml");
const OFFICIAL_ELDIRON_V1_COMBAT: &str = include_str!("../../../rulesets/eldiron/v1/combat.toml");
const OFFICIAL_ELDIRON_V1_MESSAGES: &str =
    include_str!("../../../rulesets/eldiron/v1/messages.toml");
const OFFICIAL_ELDIRON_V1_EQUIPMENT: &str =
    include_str!("../../../rulesets/eldiron/v1/equipment.toml");
const OFFICIAL_ELDIRON_V1_FX: &str = include_str!("../../../rulesets/eldiron/v1/fx.toml");
const OFFICIAL_ELDIRON_V1_ACTIONS: &str = include_str!("../../../rulesets/eldiron/v1/actions.toml");
const OFFICIAL_ELDIRON_V1_RECIPES: &str = include_str!("../../../rulesets/eldiron/v1/recipes.toml");
const OFFICIAL_ELDIRON_V1_ABILITIES_SPELLS: &str =
    include_str!("../../../rulesets/eldiron/v1/abilities_spells.toml");
const OFFICIAL_ELDIRON_V1_RACES_CLASSES: &str =
    include_str!("../../../rulesets/eldiron/v1/races_classes.toml");
const OFFICIAL_ELDIRON_V1_LOCALES: &str = include_str!("../../../rulesets/eldiron/v1/locales.toml");
const OFFICIAL_ELDIRON_V1_HUMANOID_AVATAR: &str =
    include_str!("../../../rulesets/eldiron/v1/assets/humanoid.eldiron_avatar");

static OFFICIAL_ELDIRON_V1: LazyLock<String> = LazyLock::new(|| {
    [
        OFFICIAL_ELDIRON_V1_CORE,
        OFFICIAL_ELDIRON_V1_IDENTITY,
        OFFICIAL_ELDIRON_V1_ATTRIBUTES,
        OFFICIAL_ELDIRON_V1_PROGRESSION,
        OFFICIAL_ELDIRON_V1_COMBAT,
        OFFICIAL_ELDIRON_V1_MESSAGES,
        OFFICIAL_ELDIRON_V1_EQUIPMENT,
        OFFICIAL_ELDIRON_V1_FX,
        OFFICIAL_ELDIRON_V1_ACTIONS,
        OFFICIAL_ELDIRON_V1_RECIPES,
        OFFICIAL_ELDIRON_V1_ABILITIES_SPELLS,
        OFFICIAL_ELDIRON_V1_RACES_CLASSES,
    ]
    .join("\n\n")
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundledRuleset {
    pub id: &'static str,
    pub version: &'static str,
    pub schema_version: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundledAvatarAsset {
    pub id: &'static str,
    pub ruleset_id: &'static str,
    pub ruleset_version: &'static str,
    pub path: &'static str,
    pub source: &'static str,
}

pub fn bundled_rulesets() -> &'static [BundledRuleset] {
    &[BundledRuleset {
        id: OFFICIAL_RULESET_ID,
        version: OFFICIAL_RULESET_VERSION,
        schema_version: OFFICIAL_RULESET_SCHEMA_VERSION,
    }]
}

pub fn latest_official_ruleset() -> &'static str {
    OFFICIAL_ELDIRON_V1.as_str()
}

pub fn latest_official_ruleset_locales() -> &'static str {
    OFFICIAL_ELDIRON_V1_LOCALES
}

pub fn bundled_avatar_assets() -> &'static [BundledAvatarAsset] {
    &[BundledAvatarAsset {
        id: "humanoid",
        ruleset_id: OFFICIAL_RULESET_ID,
        ruleset_version: OFFICIAL_RULESET_VERSION,
        path: "assets/humanoid.eldiron_avatar",
        source: OFFICIAL_ELDIRON_V1_HUMANOID_AVATAR,
    }]
}

pub fn bundled_avatar_assets_for_ruleset(
    ruleset_id: &str,
    ruleset_version: &str,
) -> Vec<&'static BundledAvatarAsset> {
    bundled_avatar_assets()
        .iter()
        .filter(|asset| {
            asset.ruleset_id == ruleset_id
                && (asset.ruleset_version == ruleset_version
                    || ruleset_version == "1"
                    || ruleset_version == "1.0"
                    || ruleset_version == "v1")
        })
        .collect()
}

pub fn bundled_avatars_for_project(
    config_src: &str,
) -> Result<Vec<(&'static str, rusterix::Avatar)>, String> {
    let (id, version, source) = selected_ruleset(config_src);
    if source == "project" {
        return Ok(Vec::new());
    }

    bundled_avatar_assets_for_ruleset(&id, &version)
        .into_iter()
        .map(|asset| {
            serde_json::from_str::<rusterix::Avatar>(asset.source)
                .map(|avatar| (asset.id, avatar))
                .map_err(|err| {
                    format!(
                        "Bundled ruleset avatar '{}' at '{}' could not be parsed: {}",
                        asset.id, asset.path, err
                    )
                })
        })
        .collect()
}

pub fn official_ruleset(id: &str, version: &str) -> Option<&'static str> {
    bundled_rulesets()
        .iter()
        .find(|ruleset| {
            ruleset.id == id
                && (ruleset.version == version
                    || version == "1"
                    || version == "1.0"
                    || version == "v1")
        })
        .map(|_| latest_official_ruleset())
}

pub fn official_ruleset_locales(id: &str, version: &str) -> Option<&'static str> {
    bundled_rulesets()
        .iter()
        .find(|ruleset| {
            ruleset.id == id
                && (ruleset.version == version
                    || version == "1"
                    || version == "1.0"
                    || version == "v1")
        })
        .map(|_| latest_official_ruleset_locales())
}

pub fn resolve_project_rules(config_src: &str, override_src: &str) -> Result<String, String> {
    let (id, version, source) = selected_ruleset(config_src);
    if source == "project" {
        if override_src.trim().is_empty() {
            return Ok(String::new());
        }
        let rules = override_src
            .parse::<Table>()
            .map_err(|err| format!("Project ruleset TOML parse error: {}", err))?;
        return toml::to_string(&rules)
            .map_err(|err| format!("Project ruleset serialize error: {}", err));
    }

    let base_src = official_ruleset(&id, &version).ok_or_else(|| {
        let available = bundled_rulesets()
            .iter()
            .map(|ruleset| format!("{}@{}", ruleset.id, ruleset.version))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Project requires {}@{}, but this build includes: {}",
            id, version, available
        )
    })?;

    merge_ruleset_sources(base_src, override_src)
}

pub fn resolve_project_locales(
    config_src: &str,
    locale_override_src: &str,
) -> Result<String, String> {
    let (id, version, source) = selected_ruleset(config_src);
    if source == "project" {
        if locale_override_src.trim().is_empty() {
            return Ok(String::new());
        }
        let locales = locale_override_src
            .parse::<Table>()
            .map_err(|err| format!("Project locales TOML parse error: {}", err))?;
        return toml::to_string(&locales)
            .map_err(|err| format!("Project locales serialize error: {}", err));
    }

    let base_src = official_ruleset_locales(&id, &version).ok_or_else(|| {
        let available = bundled_rulesets()
            .iter()
            .map(|ruleset| format!("{}@{}", ruleset.id, ruleset.version))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Project requires locales for {}@{}, but this build includes: {}",
            id, version, available
        )
    })?;

    merge_locale_sources(base_src, locale_override_src)
}

pub fn selected_ruleset(config_src: &str) -> (String, String, String) {
    let Ok(config) = config_src.parse::<Table>() else {
        return (
            OFFICIAL_RULESET_ID.to_string(),
            OFFICIAL_RULESET_VERSION.to_string(),
            "official".to_string(),
        );
    };

    let ruleset = config.get("ruleset").and_then(Value::as_table).or_else(|| {
        config
            .get("game")
            .and_then(Value::as_table)
            .and_then(|game| game.get("ruleset"))
            .and_then(Value::as_table)
    });

    let id = ruleset
        .and_then(|ruleset| ruleset.get("id"))
        .and_then(Value::as_str)
        .unwrap_or(OFFICIAL_RULESET_ID)
        .to_string();
    let version = ruleset
        .and_then(|ruleset| ruleset.get("version"))
        .and_then(Value::as_str)
        .unwrap_or(OFFICIAL_RULESET_VERSION)
        .to_string();
    let source = ruleset
        .and_then(|ruleset| ruleset.get("source"))
        .and_then(Value::as_str)
        .unwrap_or("official")
        .to_string();

    (id, version, source)
}

pub fn has_top_level_ruleset(config_src: &str) -> bool {
    config_src
        .parse::<Table>()
        .ok()
        .and_then(|config| config.get("ruleset").and_then(Value::as_table).cloned())
        .is_some()
}

pub fn prefix_default_ruleset_config(config_src: &mut String) {
    let trimmed = config_src.trim_end();
    if trimmed.is_empty() {
        *config_src = DEFAULT_RULESET_CONFIG.to_string();
        return;
    }

    *config_src = format!("{}\n{}", DEFAULT_RULESET_CONFIG, trimmed);
}

pub fn merge_ruleset_sources(base_src: &str, override_src: &str) -> Result<String, String> {
    let mut base = base_src
        .parse::<Table>()
        .map_err(|err| format!("Official ruleset TOML parse error: {}", err))?;

    if !override_src.trim().is_empty() {
        let overrides = override_src
            .parse::<Table>()
            .map_err(|err| format!("Rules override TOML parse error: {}", err))?;
        merge_toml_tables(&mut base, overrides);
    }

    toml::to_string(&base).map_err(|err| format!("Effective ruleset serialize error: {}", err))
}

pub fn merge_locale_sources(base_src: &str, override_src: &str) -> Result<String, String> {
    let mut base = base_src
        .parse::<Table>()
        .map_err(|err| format!("Official ruleset locales TOML parse error: {}", err))?;

    if !override_src.trim().is_empty() {
        let overrides = override_src
            .parse::<Table>()
            .map_err(|err| format!("Project locales TOML parse error: {}", err))?;
        merge_toml_tables(&mut base, overrides);
    }

    toml::to_string(&base).map_err(|err| format!("Effective locales serialize error: {}", err))
}

fn merge_toml_tables(base: &mut Table, overlay: Table) {
    for (key, value) in overlay {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base_table)), Value::Table(overlay_table)) => {
                merge_toml_tables(base_table, overlay_table);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RulesetDice {
    pub count: u32,
    pub sides: u32,
}

impl RulesetDice {
    pub fn minimum(&self) -> f32 {
        self.count as f32
    }

    pub fn maximum(&self) -> f32 {
        (self.count * self.sides) as f32
    }

    pub fn average(&self) -> f32 {
        self.count as f32 * (self.sides as f32 + 1.0) / 2.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RulesetRollSpec {
    pub roll: String,
    pub dice: RulesetDice,
    pub bonus: f32,
    pub bonus_attribute: Option<String>,
    pub bonus_every: f32,
    pub damage_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RulesetRollSummary {
    pub spec: RulesetRollSpec,
    pub attribute_value: f32,
    pub attribute_bonus: f32,
    pub total_bonus: f32,
    pub minimum: f32,
    pub maximum: f32,
    pub average: f32,
}

pub type RulesetAttributeMap = BTreeMap<String, f32>;

pub fn parse_ruleset_dice(input: &str) -> Result<RulesetDice, String> {
    let value = input.trim().to_ascii_lowercase();
    let Some((count, sides)) = value.split_once('d') else {
        return Err(format!(
            "Dice roll '{}' must use NdM syntax, like 1d6",
            input
        ));
    };
    let count = if count.trim().is_empty() {
        1
    } else {
        count
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("Dice count in '{}' is not a positive integer", input))?
    };
    let sides = sides
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("Dice sides in '{}' is not a positive integer", input))?;
    if count == 0 || sides == 0 {
        return Err(format!("Dice roll '{}' must use positive values", input));
    }

    Ok(RulesetDice { count, sides })
}

fn table_number(table: &Table, key: &str, default: f32) -> f32 {
    match table.get(key) {
        Some(Value::Integer(value)) => *value as f32,
        Some(Value::Float(value)) => *value as f32,
        _ => default,
    }
}

fn table_string(table: &Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn roll_spec_from_table(table: &Table) -> Result<RulesetRollSpec, String> {
    let roll =
        table_string(table, "roll").ok_or_else(|| "Roll table is missing 'roll'".to_string())?;
    let dice = parse_ruleset_dice(&roll)?;
    let bonus_attribute = table_string(table, "bonus_attribute");
    let mut bonus_every = table_number(table, "bonus_every", 1.0);
    if bonus_every <= 0.0 || !bonus_every.is_finite() {
        bonus_every = 1.0;
    }

    Ok(RulesetRollSpec {
        roll,
        dice,
        bonus: table_number(table, "bonus", 0.0),
        bonus_attribute,
        bonus_every,
        damage_kind: table_string(table, "damage_kind"),
    })
}

pub fn summarize_roll_table(
    table: &Table,
    attributes: &RulesetAttributeMap,
) -> Result<RulesetRollSummary, String> {
    let spec = roll_spec_from_table(table)?;
    let attribute_value = spec
        .bonus_attribute
        .as_ref()
        .and_then(|attribute| attributes.get(attribute))
        .copied()
        .unwrap_or(0.0);
    let attribute_bonus = if spec.bonus_attribute.is_some() {
        (attribute_value / spec.bonus_every).floor()
    } else {
        0.0
    };
    let total_bonus = spec.bonus + attribute_bonus;

    Ok(RulesetRollSummary {
        minimum: spec.dice.minimum() + total_bonus,
        maximum: spec.dice.maximum() + total_bonus,
        average: spec.dice.average() + total_bonus,
        spec,
        attribute_value,
        attribute_bonus,
        total_bonus,
    })
}

pub fn ruleset_table_at_path<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Table> {
    let mut value: Option<&Value> = None;
    for (index, part) in path.iter().enumerate() {
        value = if index == 0 {
            root.get(*part)
        } else {
            value?.as_table().and_then(|table| table.get(*part))
        };
    }
    value?.as_table()
}

pub fn summarize_roll_path(
    root: &Table,
    path: &[&str],
    attributes: &RulesetAttributeMap,
) -> Result<RulesetRollSummary, String> {
    let table = ruleset_table_at_path(root, path)
        .ok_or_else(|| format!("Ruleset roll path '{}' was not found", path.join(".")))?;
    summarize_roll_table(table, attributes)
}

pub fn parse_ruleset_table(src: &str) -> Result<Table, String> {
    src.parse::<Table>()
        .map_err(|err| format!("Ruleset TOML parse error: {}", err))
}

pub fn summarize_roll_path_from_source(
    src: &str,
    path: &[&str],
    attributes: &RulesetAttributeMap,
) -> Result<RulesetRollSummary, String> {
    let root = parse_ruleset_table(src)?;
    summarize_roll_path(&root, path, attributes)
}

pub fn ruleset_xp_for_level(root: &Table, level: u32) -> Option<i64> {
    let xp_table = ruleset_table_at_path(root, &["progression", "xp_table"])?;
    let key = format!("level_{}", level);
    xp_table
        .get(&key)
        .or_else(|| xp_table.get(&level.to_string()))
        .and_then(Value::as_integer)
}

pub fn ruleset_xp_for_level_from_source(src: &str, level: u32) -> Result<Option<i64>, String> {
    let root = parse_ruleset_table(src)?;
    Ok(ruleset_xp_for_level(&root, level))
}

pub fn summarize_weapon_damage(
    root: &Table,
    weapon_id: &str,
    attributes: &RulesetAttributeMap,
) -> Result<RulesetRollSummary, String> {
    summarize_roll_path(root, &["items", "weapons", weapon_id, "damage"], attributes)
}

pub fn summarize_weapon_damage_from_source(
    src: &str,
    weapon_id: &str,
    attributes: &RulesetAttributeMap,
) -> Result<RulesetRollSummary, String> {
    let root = parse_ruleset_table(src)?;
    summarize_weapon_damage(&root, weapon_id, attributes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RulesetSpellRollKind {
    Damage,
    Healing,
}

impl RulesetSpellRollKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::Healing => "healing",
        }
    }
}

pub fn summarize_spell_roll(
    root: &Table,
    spell_id: &str,
    attributes: &RulesetAttributeMap,
) -> Result<(RulesetSpellRollKind, RulesetRollSummary), String> {
    if let Ok(summary) = summarize_roll_path(root, &["spells", spell_id, "damage"], attributes) {
        return Ok((RulesetSpellRollKind::Damage, summary));
    }
    if let Ok(summary) = summarize_roll_path(root, &["spells", spell_id, "healing"], attributes) {
        return Ok((RulesetSpellRollKind::Healing, summary));
    }
    Err(format!(
        "Spell '{}' has no damage or healing roll table.",
        spell_id
    ))
}

pub fn summarize_spell_roll_from_source(
    src: &str,
    spell_id: &str,
    attributes: &RulesetAttributeMap,
) -> Result<(RulesetSpellRollKind, RulesetRollSummary), String> {
    let root = parse_ruleset_table(src)?;
    summarize_spell_roll(&root, spell_id, attributes)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetClassSummary {
    pub id: String,
    pub description: Option<String>,
    pub role: Option<String>,
    pub primary_attributes: Vec<String>,
    pub allowed_weapons: Vec<String>,
    pub allowed_armor: Vec<String>,
    pub abilities: Vec<String>,
    pub spells: Vec<String>,
    pub attributes: BTreeMap<String, String>,
    pub level_unlocks: BTreeMap<String, Vec<String>>,
    pub starting_loadout: BTreeMap<String, Vec<String>>,
}

fn table_string_array(table: &Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn simple_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Integer(value) => Some(value.to_string()),
        Value::Float(value) => Some(value.to_string()),
        Value::Boolean(value) => Some(value.to_string()),
        _ => None,
    }
}

fn table_string_values(table: &Table) -> BTreeMap<String, String> {
    table
        .iter()
        .filter_map(|(key, value)| simple_value_to_string(value).map(|value| (key.clone(), value)))
        .collect()
}

fn table_string_array_values(table: &Table) -> BTreeMap<String, Vec<String>> {
    table
        .iter()
        .filter_map(|(key, value)| {
            value.as_array().map(|values| {
                (
                    key.clone(),
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                )
            })
        })
        .collect()
}

pub fn summarize_class(root: &Table, class_id: &str) -> Result<RulesetClassSummary, String> {
    let class = ruleset_table_at_path(root, &["classes", class_id])
        .ok_or_else(|| format!("Class '{}' was not found.", class_id))?;
    let attributes = class
        .get("attributes")
        .and_then(Value::as_table)
        .map(table_string_values)
        .unwrap_or_default();
    let level_unlocks = class
        .get("unlocks")
        .and_then(Value::as_table)
        .map(|unlocks| {
            unlocks
                .iter()
                .filter_map(|(level, value)| {
                    value.as_table().map(|table| {
                        let mut entries = Vec::new();
                        for (category, values) in table_string_array_values(table) {
                            for value in values {
                                entries.push(format!("{}:{}", category, value));
                            }
                        }
                        (level.clone(), entries)
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let starting_loadout = class
        .get("starting_loadout")
        .and_then(Value::as_table)
        .map(table_string_array_values)
        .unwrap_or_default();

    Ok(RulesetClassSummary {
        id: class_id.to_string(),
        description: table_string(class, "description"),
        role: table_string(class, "role"),
        primary_attributes: table_string_array(class, "primary_attributes"),
        allowed_weapons: table_string_array(class, "allowed_weapons"),
        allowed_armor: table_string_array(class, "allowed_armor"),
        abilities: table_string_array(class, "abilities"),
        spells: table_string_array(class, "spells"),
        attributes,
        level_unlocks,
        starting_loadout,
    })
}

pub fn summarize_class_from_source(
    src: &str,
    class_id: &str,
) -> Result<RulesetClassSummary, String> {
    let root = parse_ruleset_table(src)?;
    summarize_class(&root, class_id)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetCatalog {
    pub id: Option<String>,
    pub version: Option<String>,
    pub schema_version: Option<String>,
    pub source: Option<String>,
    pub races: Vec<String>,
    pub classes: Vec<String>,
    pub professions: Vec<String>,
    pub skills: Vec<String>,
    pub resources: Vec<String>,
    pub recipes: Vec<String>,
    pub weapons: Vec<String>,
    pub armor: Vec<String>,
    pub clothing: Vec<String>,
    pub spells: Vec<String>,
    pub abilities: Vec<String>,
    pub actions: Vec<String>,
    pub fx_presets: Vec<String>,
    pub item_templates: Vec<String>,
}

fn sorted_table_keys(root: &Table, path: &[&str]) -> Vec<String> {
    ruleset_table_at_path(root, path)
        .map(|table| {
            let mut keys = table.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys
        })
        .unwrap_or_default()
}

pub fn ruleset_catalog(root: &Table) -> RulesetCatalog {
    let metadata = root.get("ruleset").and_then(Value::as_table);
    let mut item_templates = ruleset_item_group_names(root)
        .into_iter()
        .flat_map(|group| {
            sorted_table_keys(root, &["items", &group])
                .into_iter()
                .map(move |id| format!("items.{}.{}", group, id))
        })
        .collect::<Vec<_>>();
    item_templates.sort();

    RulesetCatalog {
        id: metadata
            .and_then(|ruleset| ruleset.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        version: metadata
            .and_then(|ruleset| ruleset.get("version"))
            .and_then(Value::as_str)
            .map(str::to_string),
        schema_version: metadata
            .and_then(|ruleset| ruleset.get("schema_version"))
            .and_then(Value::as_str)
            .map(str::to_string),
        source: metadata
            .and_then(|ruleset| ruleset.get("source"))
            .and_then(Value::as_str)
            .map(str::to_string),
        races: sorted_table_keys(root, &["races"]),
        classes: sorted_table_keys(root, &["classes"]),
        professions: sorted_table_keys(root, &["professions"]),
        skills: sorted_table_keys(root, &["skills"]),
        resources: sorted_table_keys(root, &["resources"]),
        recipes: sorted_table_keys(root, &["recipes"]),
        weapons: sorted_table_keys(root, &["items", "weapons"]),
        armor: sorted_table_keys(root, &["items", "armor"]),
        clothing: sorted_table_keys(root, &["items", "clothing"]),
        spells: sorted_table_keys(root, &["spells"]),
        abilities: sorted_table_keys(root, &["abilities"]),
        actions: sorted_table_keys(root, &["actions"]),
        fx_presets: sorted_table_keys(root, &["fx", "presets"]),
        item_templates,
    }
}

pub fn ruleset_catalog_from_source(src: &str) -> Result<RulesetCatalog, String> {
    let root = parse_ruleset_table(src)?;
    Ok(ruleset_catalog(&root))
}

fn section_path(section: &str) -> Option<&'static [&'static str]> {
    match section.to_ascii_lowercase().as_str() {
        "race" | "races" => Some(&["races"]),
        "class" | "classes" => Some(&["classes"]),
        "profession" | "professions" => Some(&["professions"]),
        "skill" | "skills" => Some(&["skills"]),
        "resource" | "resources" => Some(&["resources"]),
        "recipe" | "recipes" => Some(&["recipes"]),
        "weapon" | "weapons" => Some(&["items", "weapons"]),
        "armor" | "armors" => Some(&["items", "armor"]),
        "spell" | "spells" => Some(&["spells"]),
        "ability" | "abilities" => Some(&["abilities"]),
        "action" | "actions" => Some(&["actions"]),
        "fx" | "effect" | "effects" | "fx_preset" | "fx_presets" => Some(&["fx", "presets"]),
        "condition" | "conditions" => Some(&["conditions"]),
        "item" | "items" => Some(&["items"]),
        _ => None,
    }
}

pub fn ruleset_section_ids_from_source(src: &str, section: &str) -> Result<Vec<String>, String> {
    let root = parse_ruleset_table(src)?;
    let Some(path) = section_path(section) else {
        return Err(format!(
            "Unknown ruleset section '{}'. Try races, classes, professions, skills, recipes, weapons, armor, spells, or abilities.",
            section
        ));
    };
    Ok(sorted_table_keys(&root, path))
}

fn ruleset_value_at_path<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Value> {
    let mut value: Option<&Value> = None;
    for (index, part) in path.iter().enumerate() {
        value = if index == 0 {
            root.get(*part)
        } else {
            value?.as_table().and_then(|table| table.get(*part))
        };
    }
    value
}

fn format_ruleset_value(value: &Value) -> Result<String, String> {
    match value {
        Value::Table(table) => toml::to_string(table)
            .map(|text| text.trim().to_string())
            .map_err(|err| format!("Ruleset value could not be serialized: {}", err)),
        Value::Array(values) => Ok(values
            .iter()
            .filter_map(simple_value_to_string)
            .collect::<Vec<_>>()
            .join(", ")),
        _ => Ok(simple_value_to_string(value).unwrap_or_else(|| value.to_string())),
    }
}

pub fn ruleset_show_path_from_source(src: &str, path: &[&str]) -> Result<Option<String>, String> {
    let root = parse_ruleset_table(src)?;
    let Some(value) = ruleset_value_at_path(&root, path) else {
        return Ok(None);
    };
    format_ruleset_value(value).map(Some)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RulesetValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetValidationIssue {
    pub severity: RulesetValidationSeverity,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RulesetValidationReport {
    pub issues: Vec<RulesetValidationIssue>,
}

impl RulesetValidationReport {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == RulesetValidationSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == RulesetValidationSeverity::Warning)
            .count()
    }

    pub fn is_ok(&self) -> bool {
        self.error_count() == 0
    }

    fn push(
        &mut self,
        severity: RulesetValidationSeverity,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(RulesetValidationIssue {
            severity,
            path: path.into(),
            message: message.into(),
        });
    }

    fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.push(RulesetValidationSeverity::Error, path, message);
    }

    fn warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.push(RulesetValidationSeverity::Warning, path, message);
    }
}

fn table_key_set(root: &Table, path: &[&str]) -> BTreeSet<String> {
    sorted_table_keys(root, path).into_iter().collect()
}

fn validate_string_reference(
    report: &mut RulesetValidationReport,
    path: &str,
    label: &str,
    value: Option<&str>,
    known: &BTreeSet<String>,
) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !known.contains(value) {
        report.error(
            path,
            format!("{} '{}' does not exist in the ruleset.", label, value),
        );
    }
}

fn validate_string_array_references(
    report: &mut RulesetValidationReport,
    table: &Table,
    key: &str,
    path: &str,
    label: &str,
    known: &BTreeSet<String>,
) {
    for value in table_string_array(table, key) {
        if !known.contains(&value) {
            report.error(
                format!("{}.{}", path, key),
                format!("{} '{}' does not exist in the ruleset.", label, value),
            );
        }
    }
}

fn validate_roll_table(report: &mut RulesetValidationReport, path: &str, table: &Table) {
    let Some(roll) = table_string(table, "roll") else {
        report.error(path, "Roll table is missing 'roll'.");
        return;
    };
    if let Err(err) = parse_ruleset_dice(&roll) {
        report.error(format!("{}.roll", path), err);
    }

    if table_number(table, "bonus_every", 1.0) <= 0.0 {
        report.error(
            format!("{}.bonus_every", path),
            "bonus_every must be greater than zero.",
        );
    }
}

fn validate_roll_path(report: &mut RulesetValidationReport, root: &Table, path: &[&str]) {
    if let Some(table) = ruleset_table_at_path(root, path) {
        validate_roll_table(report, &path.join("."), table);
    }
}

fn level_number_from_key(key: &str) -> Option<u32> {
    key.strip_prefix("level_")
        .or(Some(key))
        .and_then(|value| value.parse::<u32>().ok())
}

fn validate_xp_table(report: &mut RulesetValidationReport, root: &Table) {
    let Some(xp_table) = ruleset_table_at_path(root, &["progression", "xp_table"]) else {
        report.warning("progression.xp_table", "No XP table is defined.");
        return;
    };

    let mut levels = Vec::new();
    for (key, value) in xp_table {
        let Some(level) = level_number_from_key(key) else {
            report.error(
                format!("progression.xp_table.{}", key),
                "XP table keys must be level_N or numeric levels.",
            );
            continue;
        };
        let Some(xp) = value.as_integer() else {
            report.error(
                format!("progression.xp_table.{}", key),
                "XP table values must be integers.",
            );
            continue;
        };
        if level < 2 {
            report.error(
                format!("progression.xp_table.{}", key),
                "XP table levels must start at level 2 or higher.",
            );
        }
        if xp < 0 {
            report.error(
                format!("progression.xp_table.{}", key),
                "XP table values must be zero or greater.",
            );
        }
        levels.push((level, xp, key.clone()));
    }

    levels.sort_by_key(|(level, _, _)| *level);
    for window in levels.windows(2) {
        let (_, previous_xp, previous_key) = &window[0];
        let (_, xp, key) = &window[1];
        if xp <= previous_xp {
            report.error(
                format!("progression.xp_table.{}", key),
                format!(
                    "XP must increase after progression.xp_table.{}.",
                    previous_key
                ),
            );
        }
    }

    if let Some(max_level) = ruleset_table_at_path(root, &["progression", "level"])
        .and_then(|table| table.get("max_level"))
        .and_then(Value::as_integer)
    {
        for expected in 2..=max_level.max(1) as u32 {
            if !levels.iter().any(|(level, _, _)| *level == expected) {
                report.warning(
                    "progression.xp_table",
                    format!("No XP entry exists for level {}.", expected),
                );
            }
        }
    }
}

fn validate_item_rules(
    report: &mut RulesetValidationReport,
    root: &Table,
    damage_kinds: &BTreeSet<String>,
) {
    let weapon_categories = table_key_set(root, &["equipment", "weapon_categories"]);
    let armor_categories = table_key_set(root, &["equipment", "armor_categories"]);
    let mut item_templates = BTreeSet::new();
    for group in ruleset_item_group_names(root) {
        item_templates.extend(table_key_set(root, &["items", &group]));
    }
    let weapon_slots = ruleset_table_at_path(root, &["equipment"])
        .map(|table| table_string_array(table, "weapon_slots"))
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let armor_slots = ruleset_table_at_path(root, &["equipment"])
        .map(|table| table_string_array(table, "armor_slots"))
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();

    if let Some(weapons) = ruleset_table_at_path(root, &["items", "weapons"]) {
        for (id, value) in weapons {
            let Some(weapon) = value.as_table() else {
                report.error(
                    format!("items.weapons.{}", id),
                    "Weapon entry must be a table.",
                );
                continue;
            };
            let path = format!("items.weapons.{}", id);
            validate_string_reference(
                report,
                &format!("{}.category", path),
                "Weapon category",
                table_string(weapon, "category").as_deref(),
                &weapon_categories,
            );
            validate_string_reference(
                report,
                &format!("{}.slot", path),
                "Weapon slot",
                table_string(weapon, "slot").as_deref(),
                &weapon_slots,
            );
            validate_roll_path(report, root, &["items", "weapons", id, "damage"]);
            if let Some(damage) = weapon.get("damage").and_then(Value::as_table) {
                validate_string_reference(
                    report,
                    &format!("{}.damage.damage_kind", path),
                    "Damage kind",
                    table_string(damage, "damage_kind").as_deref(),
                    damage_kinds,
                );
            }
            if let Some(attributes) = weapon.get("attributes").and_then(Value::as_table) {
                validate_string_reference(
                    report,
                    &format!("{}.attributes.damage_kind", path),
                    "Damage kind",
                    table_string(attributes, "damage_kind").as_deref(),
                    damage_kinds,
                );
                validate_string_reference(
                    report,
                    &format!("{}.attributes.ammunition", path),
                    "Ammunition item",
                    table_string(attributes, "ammunition").as_deref(),
                    &item_templates,
                );
            }
        }
    }

    if let Some(armor) = ruleset_table_at_path(root, &["items", "armor"]) {
        for (id, value) in armor {
            let Some(item) = value.as_table() else {
                report.error(
                    format!("items.armor.{}", id),
                    "Armor entry must be a table.",
                );
                continue;
            };
            let path = format!("items.armor.{}", id);
            validate_string_reference(
                report,
                &format!("{}.category", path),
                "Armor category",
                table_string(item, "category").as_deref(),
                &armor_categories,
            );
            validate_string_reference(
                report,
                &format!("{}.slot", path),
                "Armor slot",
                table_string(item, "slot").as_deref(),
                &armor_slots,
            );
        }
    }
}

fn validate_ability_and_spell_rules(
    report: &mut RulesetValidationReport,
    root: &Table,
    damage_kinds: &BTreeSet<String>,
) {
    if let Some(abilities) = ruleset_table_at_path(root, &["abilities"]) {
        for (id, value) in abilities {
            let Some(ability) = value.as_table() else {
                report.error(
                    format!("abilities.{}", id),
                    "Ability entry must be a table.",
                );
                continue;
            };
            let path = format!("abilities.{}", id);
            validate_string_reference(
                report,
                &format!("{}.damage_kind", path),
                "Damage kind",
                table_string(ability, "damage_kind").as_deref(),
                damage_kinds,
            );
            if let Some(damage) = ability.get("damage").and_then(Value::as_table) {
                validate_roll_table(report, &format!("{}.damage", path), damage);
                validate_string_reference(
                    report,
                    &format!("{}.damage.damage_kind", path),
                    "Damage kind",
                    table_string(damage, "damage_kind").as_deref(),
                    damage_kinds,
                );
            }
        }
    }

    if let Some(spells) = ruleset_table_at_path(root, &["spells"]) {
        for (id, value) in spells {
            let Some(spell) = value.as_table() else {
                report.error(format!("spells.{}", id), "Spell entry must be a table.");
                continue;
            };
            let path = format!("spells.{}", id);
            validate_string_reference(
                report,
                &format!("{}.damage_kind", path),
                "Damage kind",
                table_string(spell, "damage_kind").as_deref(),
                damage_kinds,
            );
            if let Some(damage) = spell.get("damage").and_then(Value::as_table) {
                validate_roll_table(report, &format!("{}.damage", path), damage);
                validate_string_reference(
                    report,
                    &format!("{}.damage.damage_kind", path),
                    "Damage kind",
                    table_string(damage, "damage_kind").as_deref(),
                    damage_kinds,
                );
            }
            if let Some(healing) = spell.get("healing").and_then(Value::as_table) {
                validate_roll_table(report, &format!("{}.healing", path), healing);
            }
        }
    }

    let abilities = table_key_set(root, &["abilities"]);
    let spells = table_key_set(root, &["spells"]);
    let professions = table_key_set(root, &["professions"]);
    let skills = table_key_set(root, &["skills"]);
    let mut item_templates = BTreeSet::new();
    for group in ruleset_item_group_names(root) {
        item_templates.extend(table_key_set(root, &["items", &group]));
    }
    if let Some(actions) = ruleset_table_at_path(root, &["actions"]) {
        for (id, value) in actions {
            let Some(action) = value.as_table() else {
                report.error(format!("actions.{}", id), "Action entry must be a table.");
                continue;
            };
            let path = format!("actions.{}", id);
            validate_string_reference(
                report,
                &format!("{}.skill", path),
                "Skill",
                table_string(action, "skill").as_deref(),
                &skills,
            );
            if let Some(requires) = action.get("requires").and_then(Value::as_table) {
                validate_string_reference(
                    report,
                    &format!("{}.requires.ability", path),
                    "Ability",
                    table_string(requires, "ability").as_deref(),
                    &abilities,
                );
                validate_string_reference(
                    report,
                    &format!("{}.requires.spell", path),
                    "Spell",
                    table_string(requires, "spell").as_deref(),
                    &spells,
                );
                validate_string_reference(
                    report,
                    &format!("{}.requires.profession", path),
                    "Profession",
                    table_string(requires, "profession").as_deref(),
                    &professions,
                );
            }
            validate_item_quantity_list(report, &path, action, "consumes", &item_templates);
            if let Some(result) = action.get("result").and_then(Value::as_table) {
                validate_string_reference(
                    report,
                    &format!("{}.result.item", path),
                    "Item",
                    table_string(result, "item").as_deref(),
                    &item_templates,
                );
            }
        }
    }
}

fn validate_item_quantity_list(
    report: &mut RulesetValidationReport,
    path: &str,
    table: &Table,
    key: &str,
    item_templates: &BTreeSet<String>,
) {
    if let Some(entries) = table.get(key).and_then(Value::as_array) {
        for (index, value) in entries.iter().enumerate() {
            let Some(entry) = value.as_table() else {
                report.error(
                    format!("{}.{}.{}", path, key, index),
                    "Item quantity entry must be a table.",
                );
                continue;
            };
            validate_string_reference(
                report,
                &format!("{}.{}.{}.item", path, key, index),
                "Item",
                table_string(entry, "item").as_deref(),
                item_templates,
            );
        }
    }
}

fn validate_recipe_rules(report: &mut RulesetValidationReport, root: &Table) {
    let professions = table_key_set(root, &["professions"]);
    let classes = table_key_set(root, &["classes"]);
    let skills = table_key_set(root, &["skills"]);
    let spells = table_key_set(root, &["spells"]);
    let mut item_templates = BTreeSet::new();
    for group in ruleset_item_group_names(root) {
        item_templates.extend(table_key_set(root, &["items", &group]));
    }

    if let Some(recipes) = ruleset_table_at_path(root, &["recipes"]) {
        for (id, value) in recipes {
            let Some(recipe) = value.as_table() else {
                report.error(format!("recipes.{}", id), "Recipe entry must be a table.");
                continue;
            };
            let path = format!("recipes.{}", id);
            validate_string_reference(
                report,
                &format!("{}.skill", path),
                "Skill",
                table_string(recipe, "skill").as_deref(),
                &skills,
            );
            validate_string_reference(
                report,
                &format!("{}.profession_hint", path),
                "Profession",
                table_string(recipe, "profession_hint").as_deref(),
                &professions,
            );
            validate_string_reference(
                report,
                &format!("{}.class_hint", path),
                "Class",
                table_string(recipe, "class_hint").as_deref(),
                &classes,
            );
            if let Some(requires) = recipe.get("requires").and_then(Value::as_table) {
                validate_string_reference(
                    report,
                    &format!("{}.requires.spell", path),
                    "Spell",
                    table_string(requires, "spell").as_deref(),
                    &spells,
                );
            }
            validate_item_quantity_list(report, &path, recipe, "consumes", &item_templates);
            validate_item_quantity_list(report, &path, recipe, "produces", &item_templates);
        }
    }
}

fn validate_resource_rules(report: &mut RulesetValidationReport, root: &Table) {
    let actions = table_key_set(root, &["actions"]);
    let skills = table_key_set(root, &["skills"]);
    let mut item_templates = BTreeSet::new();
    for group in ruleset_item_group_names(root) {
        item_templates.extend(table_key_set(root, &["items", &group]));
    }

    if let Some(resources) = ruleset_table_at_path(root, &["resources"]) {
        for (id, value) in resources {
            let Some(resource) = value.as_table() else {
                report.error(
                    format!("resources.{}", id),
                    "Resource entry must be a table.",
                );
                continue;
            };
            let path = format!("resources.{}", id);
            validate_string_reference(
                report,
                &format!("{}.action", path),
                "Action",
                table_string(resource, "action").as_deref(),
                &actions,
            );
            validate_string_reference(
                report,
                &format!("{}.skill", path),
                "Skill",
                table_string(resource, "skill").as_deref(),
                &skills,
            );
            if let Some(produces) = resource.get("produces").and_then(Value::as_table) {
                validate_string_reference(
                    report,
                    &format!("{}.produces.item", path),
                    "Item",
                    table_string(produces, "item").as_deref(),
                    &item_templates,
                );
            }
        }
    }
}

fn validate_class_rules(report: &mut RulesetValidationReport, root: &Table) {
    let weapon_categories = table_key_set(root, &["equipment", "weapon_categories"]);
    let armor_categories = table_key_set(root, &["equipment", "armor_categories"]);
    let weapons = table_key_set(root, &["items", "weapons"]);
    let armor = table_key_set(root, &["items", "armor"]);
    let clothing = table_key_set(root, &["items", "clothing"]);
    let mut item_templates = BTreeSet::new();
    for group in ruleset_item_group_names(root) {
        item_templates.extend(table_key_set(root, &["items", &group]));
    }
    let abilities = table_key_set(root, &["abilities"]);
    let spells = table_key_set(root, &["spells"]);

    let Some(classes) = ruleset_table_at_path(root, &["classes"]) else {
        report.warning("classes", "No classes are defined.");
        return;
    };

    for (id, value) in classes {
        let Some(class) = value.as_table() else {
            report.error(format!("classes.{}", id), "Class entry must be a table.");
            continue;
        };
        let path = format!("classes.{}", id);
        validate_string_array_references(
            report,
            class,
            "allowed_weapons",
            &path,
            "Weapon category",
            &weapon_categories,
        );
        validate_string_array_references(
            report,
            class,
            "allowed_armor",
            &path,
            "Armor category",
            &armor_categories,
        );
        validate_string_array_references(report, class, "abilities", &path, "Ability", &abilities);
        validate_string_array_references(report, class, "spells", &path, "Spell", &spells);
        validate_string_array_references(report, class, "spell_lists", &path, "Spell", &spells);

        for table_key in ["starting_loadout", "unlocks"] {
            if let Some(table) = class.get(table_key).and_then(Value::as_table) {
                validate_class_reference_table(
                    report,
                    table,
                    &format!("{}.{}", path, table_key),
                    &weapons,
                    &armor,
                    &clothing,
                    &item_templates,
                    &abilities,
                    &spells,
                );
            }
        }
    }
}

fn validate_class_reference_table(
    report: &mut RulesetValidationReport,
    table: &Table,
    path: &str,
    weapons: &BTreeSet<String>,
    armor: &BTreeSet<String>,
    clothing: &BTreeSet<String>,
    item_templates: &BTreeSet<String>,
    abilities: &BTreeSet<String>,
    spells: &BTreeSet<String>,
) {
    for (key, value) in table {
        if key.starts_with("level_") {
            if let Some(level_table) = value.as_table() {
                validate_class_reference_table(
                    report,
                    level_table,
                    &format!("{}.{}", path, key),
                    weapons,
                    armor,
                    clothing,
                    item_templates,
                    abilities,
                    spells,
                );
            } else {
                report.error(
                    format!("{}.{}", path, key),
                    "Level unlock entry must be a table.",
                );
            }
            continue;
        }

        let Some(values) = value.as_array() else {
            report.error(
                format!("{}.{}", path, key),
                "Loadout and unlock values must be arrays.",
            );
            continue;
        };
        for entry in values.iter().filter_map(Value::as_str) {
            let known = match key.as_str() {
                "weapons" => weapons.contains(entry),
                "armor" => armor.contains(entry),
                "clothing" => clothing.contains(entry),
                "abilities" => abilities.contains(entry),
                "spells" | "spell_lists" => spells.contains(entry),
                "inventory" | "items" => item_templates.contains(entry),
                "equipment" => item_templates.contains(entry),
                _ => {
                    report.warning(
                        format!("{}.{}", path, key),
                        format!("Unknown reference category '{}'.", key),
                    );
                    true
                }
            };
            if !known {
                report.error(
                    format!("{}.{}", path, key),
                    format!("Referenced '{}' does not exist.", entry),
                );
            }
        }
    }
}

fn validate_visual_rules(report: &mut RulesetValidationReport, root: &Table) {
    let avatars = table_key_set(root, &["assets", "avatars"]);
    if let Some(defaults) = ruleset_table_at_path(root, &["visuals", "defaults"]) {
        validate_string_reference(
            report,
            "visuals.defaults.avatar",
            "Avatar",
            table_string(defaults, "avatar").as_deref(),
            &avatars,
        );
    }
    if let Some(races) = ruleset_table_at_path(root, &["races"]) {
        for (id, value) in races {
            if let Some(race) = value.as_table() {
                validate_string_reference(
                    report,
                    &format!("races.{}.default_avatar", id),
                    "Avatar",
                    table_string(race, "default_avatar").as_deref(),
                    &avatars,
                );
            }
        }
    }
}

fn validate_identity_rules(report: &mut RulesetValidationReport, root: &Table) {
    let races = table_key_set(root, &["races"]);
    let classes = table_key_set(root, &["classes"]);
    if let Some(defaults) = ruleset_table_at_path(root, &["identity", "defaults"]) {
        validate_string_reference(
            report,
            "identity.defaults.race",
            "Race",
            table_string(defaults, "race").as_deref(),
            &races,
        );
        validate_string_reference(
            report,
            "identity.defaults.class",
            "Class",
            table_string(defaults, "class").as_deref(),
            &classes,
        );
    }
}

fn validate_relation_and_intent_rules(report: &mut RulesetValidationReport, root: &Table) {
    let races = table_key_set(root, &["races"]);
    let dispositions = table_key_set(root, &["dispositions"]);

    if let Some(relations) = ruleset_table_at_path(root, &["race_relations"]) {
        for (actor_race, value) in relations {
            if !races.contains(actor_race) {
                report.error(
                    format!("race_relations.{}", actor_race),
                    format!("Actor race '{}' does not exist.", actor_race),
                );
            }
            let Some(table) = value.as_table() else {
                report.error(
                    format!("race_relations.{}", actor_race),
                    "Race relation entry must be a table.",
                );
                continue;
            };
            for (target_race, disposition) in table {
                if !races.contains(target_race) {
                    report.error(
                        format!("race_relations.{}.{}", actor_race, target_race),
                        format!("Target race '{}' does not exist.", target_race),
                    );
                }
                let Some(disposition) = disposition.as_str() else {
                    report.error(
                        format!("race_relations.{}.{}", actor_race, target_race),
                        "Race relation disposition must be a string.",
                    );
                    continue;
                };
                if !dispositions.contains(disposition.trim()) {
                    report.error(
                        format!("race_relations.{}.{}", actor_race, target_race),
                        format!("Disposition '{}' does not exist.", disposition),
                    );
                }
            }
        }
    }

    let allowed_target_kinds = BTreeSet::from(["entity".to_string(), "item".to_string()]);
    if let Some(intents) = ruleset_table_at_path(root, &["intents"]) {
        for (intent, value) in intents {
            let Some(table) = value.as_table() else {
                report.error(
                    format!("intents.{}", intent),
                    "Intent entry must be a table.",
                );
                continue;
            };
            validate_string_array_references(
                report,
                table,
                "allowed_dispositions",
                &format!("intents.{}", intent),
                "Disposition",
                &dispositions,
            );
            validate_string_array_references(
                report,
                table,
                "allowed_target_kinds",
                &format!("intents.{}", intent),
                "Target kind",
                &allowed_target_kinds,
            );
            if let Some(distance) = table.get("distance") {
                if distance.as_table().is_none()
                    && distance.as_float().is_none()
                    && distance.as_integer().is_none()
                {
                    report.error(
                        format!("intents.{}.distance", intent),
                        "Intent distance must be a number or table.",
                    );
                }
            }
        }
    }
}

pub fn validate_ruleset(root: &Table) -> RulesetValidationReport {
    let mut report = RulesetValidationReport::default();
    let damage_kinds = table_key_set(root, &["combat", "kinds"]);

    validate_identity_rules(&mut report, root);
    validate_relation_and_intent_rules(&mut report, root);
    validate_visual_rules(&mut report, root);
    validate_xp_table(&mut report, root);
    validate_roll_path(&mut report, root, &["combat", "unarmed_damage"]);
    validate_item_rules(&mut report, root, &damage_kinds);
    validate_ability_and_spell_rules(&mut report, root, &damage_kinds);
    validate_recipe_rules(&mut report, root);
    validate_resource_rules(&mut report, root);
    validate_class_rules(&mut report, root);

    if let Err(err) = ruleset_item_templates(root) {
        report.error("items", err);
    }

    report
}

pub fn validate_ruleset_from_source(src: &str) -> Result<RulesetValidationReport, String> {
    let root = parse_ruleset_table(src)?;
    Ok(validate_ruleset(&root))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetItemTemplate {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub ruleset_path: String,
    pub data: String,
}

fn insert_string(table: &mut Table, key: &str, value: impl Into<String>) {
    table.insert(key.to_string(), Value::String(value.into()));
}

fn insert_bool(table: &mut Table, key: &str, value: bool) {
    table.insert(key.to_string(), Value::Boolean(value));
}

fn insert_float(table: &mut Table, key: &str, value: f64) {
    table.insert(key.to_string(), Value::Float(value));
}

fn insert_integer(table: &mut Table, key: impl Into<String>, value: i64) {
    table.insert(key.into(), Value::Integer(value));
}

fn ruleset_item_group_names(root: &Table) -> Vec<String> {
    let Some(items) = ruleset_table_at_path(root, &["items"]) else {
        return Vec::new();
    };
    let mut names = items
        .iter()
        .filter_map(|(key, value)| value.as_table().map(|_| key.clone()))
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn ruleset_item_kind(table_name: &str) -> String {
    table_name
        .strip_suffix('s')
        .unwrap_or(table_name)
        .to_string()
}

fn ruleset_item_template_data(
    root_table: &Table,
    id: &str,
    kind: &str,
    table_name: &str,
    item: &Table,
) -> Result<RulesetItemTemplate, String> {
    let name = table_string(item, "name").unwrap_or_else(|| id.to_string());
    let ruleset_path = format!("items.{}.{}", table_name, id);

    let mut attributes = Table::new();
    insert_string(&mut attributes, "name", &name);
    if let Some(description) = table_string(item, "description") {
        insert_string(&mut attributes, "description", &description);
        insert_string(&mut attributes, "on_look", description);
    }
    insert_bool(&mut attributes, "visible", true);
    insert_bool(&mut attributes, "static", false);
    insert_bool(&mut attributes, "blocking", false);
    insert_float(&mut attributes, "radius", 0.5);
    insert_float(&mut attributes, "worth", 0.0);
    insert_bool(&mut attributes, "monetary", false);

    for key in ["category", "slot", "rarity"] {
        if let Some(value) = table_string(item, key) {
            insert_string(&mut attributes, key, value);
        }
    }

    for key in [
        "visual_template",
        "icon_template",
        "rig_template",
        "rig_scale",
        "rig_pivot",
        "rig_layer",
        "rig_flip_back",
        "blade_color",
        "blade_color_index",
        "grip_color",
        "grip_color_index",
        "accent_color",
        "accent_color_index",
        "highlight_color",
        "highlight_color_index",
        "max_stack",
        "ammunition_quantity",
    ] {
        if let Some(value) = item.get(key) {
            attributes.insert(key.to_string(), value.clone());
        }
    }

    if let Some(max_stack) = item.get("max_stack").and_then(Value::as_integer) {
        insert_integer(&mut attributes, "max_capacity", max_stack);
    }

    if let Some(template_name) = table_string(item, "visual_template")
        && let Some(templates) = root_table.get("visual_templates").and_then(Value::as_table)
        && let Some(template) = templates
            .get(template_name.trim())
            .and_then(Value::as_table)
    {
        if let Some(width) = template.get("width").and_then(Value::as_integer) {
            insert_integer(&mut attributes, "visual_template_width", width);
        }
        if let Some(height) = template.get("height").and_then(Value::as_integer) {
            insert_integer(&mut attributes, "visual_template_height", height);
        }
        if let Some(pixels) = template.get("pixels").and_then(Value::as_array) {
            let pixels = pixels
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .map(Value::String)
                .collect::<Vec<_>>();
            if !pixels.is_empty() {
                attributes.insert("visual_template_pixels".to_string(), Value::Array(pixels));
            }
        }
    }

    if let Some(color_index) = item.get("color").and_then(Value::as_integer) {
        insert_integer(&mut attributes, "color_index", color_index);
        if let Some(channels) = item.get("avatar_channels").and_then(Value::as_array) {
            let channels = channels
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|channel| !channel.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if !channels.is_empty() {
                attributes.insert(
                    "avatar_channels".to_string(),
                    Value::Array(channels.iter().cloned().map(Value::String).collect()),
                );
                for channel in channels {
                    insert_integer(&mut attributes, format!("{}_index", channel), color_index);
                }
            }
        }
    }

    if let Some(item_attributes) = item.get("attributes").and_then(Value::as_table) {
        for (key, value) in item_attributes {
            attributes.insert(key.clone(), value.clone());
        }
    }

    insert_string(&mut attributes, "ruleset_path", &ruleset_path);
    insert_string(&mut attributes, "ruleset_kind", kind);
    insert_string(&mut attributes, "ruleset_id", id);

    let mut root = Table::new();
    root.insert("attributes".to_string(), Value::Table(attributes));

    if let Some(damage) = item.get("damage").and_then(Value::as_table) {
        let mut ruleset = Table::new();
        ruleset.insert("damage".to_string(), Value::Table(damage.clone()));
        root.insert("ruleset".to_string(), Value::Table(ruleset));
    }

    let data = toml::to_string(&root)
        .map_err(|err| format!("Ruleset item '{}' could not be serialized: {}", id, err))?;

    Ok(RulesetItemTemplate {
        id: id.to_string(),
        name,
        kind: kind.to_string(),
        ruleset_path,
        data,
    })
}

pub fn ruleset_palette(root: &Table) -> Result<ThePalette, String> {
    let mut palette = ThePalette::empty_256();
    let Some(palette_table) = ruleset_table_at_path(root, &["palette"]) else {
        return Ok(palette);
    };

    for (key, value) in palette_table {
        let index = key
            .parse::<usize>()
            .map_err(|_| format!("Palette index '{}' must be a number", key))?;
        if index >= palette.colors.len() {
            return Err(format!(
                "Palette index '{}' is out of range; Eldiron palettes support 0..255",
                key
            ));
        }
        let Some(hex) = value
            .as_str()
            .map(str::trim)
            .filter(|value| value.starts_with('#') && (value.len() == 7 || value.len() == 9))
        else {
            return Err(format!(
                "Palette index '{}' must be a #RRGGBB or #RRGGBBAA string",
                key
            ));
        };
        palette.colors[index] = Some(TheColor::from_hex(hex));
    }

    Ok(palette)
}

pub fn ruleset_palette_from_source(src: &str) -> Result<ThePalette, String> {
    let root = parse_ruleset_table(src)?;
    ruleset_palette(&root)
}

fn collect_ruleset_item_templates(
    root: &Table,
    table_name: &str,
    kind: &str,
    templates: &mut Vec<RulesetItemTemplate>,
) -> Result<(), String> {
    let Some(items) = ruleset_table_at_path(root, &["items", table_name]) else {
        return Ok(());
    };

    for (id, value) in items {
        if let Some(item) = value.as_table() {
            templates.push(ruleset_item_template_data(
                root, id, kind, table_name, item,
            )?);
        }
    }

    Ok(())
}

pub fn ruleset_item_templates(root: &Table) -> Result<Vec<RulesetItemTemplate>, String> {
    let mut templates = Vec::new();
    for group in ruleset_item_group_names(root) {
        let kind = ruleset_item_kind(&group);
        collect_ruleset_item_templates(root, &group, &kind, &mut templates)?;
    }
    Ok(templates)
}

pub fn ruleset_item_templates_from_source(src: &str) -> Result<Vec<RulesetItemTemplate>, String> {
    let root = parse_ruleset_table(src)?;
    ruleset_item_templates(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_default_official_ruleset() {
        let rules = resolve_project_rules("", "").unwrap();
        let table = rules.parse::<Table>().unwrap();
        assert_eq!(
            table
                .get("ruleset")
                .and_then(Value::as_table)
                .and_then(|ruleset| ruleset.get("id"))
                .and_then(Value::as_str),
            Some(OFFICIAL_RULESET_ID)
        );
        assert!(table.get("combat").is_some());
        assert!(table.get("progression").is_some());
        assert!(table.get("classes").is_some());
        assert!(table.get("items").is_some());
        assert!(table.get("abilities").is_some());
    }

    #[test]
    fn resolves_default_official_ruleset_locales() {
        let locales = resolve_project_locales("", "").unwrap();
        let table = locales.parse::<Table>().unwrap();

        assert_eq!(
            table
                .get("en")
                .and_then(Value::as_table)
                .and_then(|en| en.get("spells"))
                .and_then(Value::as_table)
                .and_then(|spells| spells.get("missing_target"))
                .and_then(Value::as_str),
            Some("Cast at what?")
        );
        assert_eq!(
            table
                .get("en")
                .and_then(Value::as_table)
                .and_then(|en| en.get("actions"))
                .and_then(Value::as_table)
                .and_then(|actions| actions.get("not_ready"))
                .and_then(Value::as_str),
            Some("{action} is not ready yet")
        );
    }

    #[test]
    fn project_locales_override_official_ruleset_locales() {
        let locales = resolve_project_locales(
            "",
            r#"
            [en.spells]
            missing_target = "Choose a spell target"

            [de.spells]
            missing_target = "Zauber auf welches Ziel?"
            "#,
        )
        .unwrap();
        let table = locales.parse::<Table>().unwrap();

        assert_eq!(
            table
                .get("en")
                .and_then(Value::as_table)
                .and_then(|en| en.get("spells"))
                .and_then(Value::as_table)
                .and_then(|spells| spells.get("missing_target"))
                .and_then(Value::as_str),
            Some("Choose a spell target")
        );
        assert_eq!(
            table
                .get("en")
                .and_then(Value::as_table)
                .and_then(|en| en.get("spells"))
                .and_then(Value::as_table)
                .and_then(|spells| spells.get("not_ready"))
                .and_then(Value::as_str),
            Some("{spell} is not ready yet")
        );
        assert_eq!(
            table
                .get("de")
                .and_then(Value::as_table)
                .and_then(|de| de.get("spells"))
                .and_then(Value::as_table)
                .and_then(|spells| spells.get("missing_target"))
                .and_then(Value::as_str),
            Some("Zauber auf welches Ziel?")
        );
    }

    #[test]
    fn loads_bundled_humanoid_avatar() {
        let avatars = bundled_avatars_for_project("").unwrap();
        let (_, avatar) = avatars
            .iter()
            .find(|(id, _)| *id == "humanoid")
            .expect("humanoid avatar should be bundled");

        assert!(!avatar.animations.is_empty());
    }

    #[test]
    fn applies_project_overrides_after_official_base() {
        let rules = resolve_project_rules(
            "",
            r#"
            [combat]
            incoming_damage = "value - defender.armor.ARMOR"

            [spells.minor_heal]
            cost_mp = 3
            "#,
        )
        .unwrap();
        let table = rules.parse::<Table>().unwrap();

        assert!(
            table
                .get("combat")
                .and_then(Value::as_table)
                .and_then(|combat| combat.get("outgoing_damage"))
                .is_none()
        );
        assert_eq!(
            table
                .get("combat")
                .and_then(Value::as_table)
                .and_then(|combat| combat.get("incoming_damage"))
                .and_then(Value::as_str),
            Some("value - defender.armor.ARMOR")
        );
        assert_eq!(
            table
                .get("spells")
                .and_then(Value::as_table)
                .and_then(|spells| spells.get("minor_heal"))
                .and_then(Value::as_table)
                .and_then(|spell| spell.get("cost_mp"))
                .and_then(Value::as_integer),
            Some(3)
        );
    }

    #[test]
    fn official_ruleset_defines_human_warrior_baseline() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let warrior = table
            .get("classes")
            .and_then(Value::as_table)
            .and_then(|classes| classes.get("Warrior"))
            .and_then(Value::as_table)
            .expect("Warrior class should exist");

        assert_eq!(
            warrior
                .get("attributes")
                .and_then(Value::as_table)
                .and_then(|attributes| attributes.get("MAX_HP"))
                .and_then(Value::as_integer),
            Some(16)
        );
        assert!(
            warrior
                .get("abilities")
                .and_then(Value::as_array)
                .is_some_and(|abilities| abilities
                    .iter()
                    .any(|ability| ability.as_str() == Some("power_strike")))
        );

        assert!(
            table
                .get("combat")
                .and_then(Value::as_table)
                .and_then(|combat| combat.get("kinds"))
                .and_then(Value::as_table)
                .and_then(|kinds| kinds.get("physical"))
                .is_some()
        );
        assert!(
            table
                .get("items")
                .and_then(Value::as_table)
                .and_then(|items| items.get("weapons"))
                .and_then(Value::as_table)
                .and_then(|weapons| weapons.get("training_sword"))
                .is_some()
        );
        assert!(
            table
                .get("progression")
                .and_then(Value::as_table)
                .and_then(|progression| progression.get("xp_table"))
                .and_then(Value::as_table)
                .and_then(|xp_table| xp_table.get("level_2"))
                .and_then(Value::as_integer)
                .is_some()
        );
        assert!(
            table
                .get("items")
                .and_then(Value::as_table)
                .and_then(|items| items.get("weapons"))
                .and_then(Value::as_table)
                .and_then(|weapons| weapons.get("training_sword"))
                .and_then(Value::as_table)
                .and_then(|sword| sword.get("damage"))
                .and_then(Value::as_table)
                .and_then(|damage| damage.get("roll"))
                .and_then(Value::as_str)
                .is_some()
        );
        assert!(
            table
                .get("derived_stats")
                .and_then(Value::as_table)
                .and_then(|stats| stats.get("DMG"))
                .and_then(Value::as_table)
                .and_then(|dmg| dmg.get("formula"))
                .is_none()
        );
    }

    #[test]
    fn parses_ruleset_dice() {
        assert_eq!(
            parse_ruleset_dice("1d6").unwrap(),
            RulesetDice { count: 1, sides: 6 }
        );
        assert_eq!(
            parse_ruleset_dice("d8").unwrap(),
            RulesetDice { count: 1, sides: 8 }
        );
        assert!(parse_ruleset_dice("0d6").is_err());
        assert!(parse_ruleset_dice("6").is_err());
    }

    #[test]
    fn summarizes_official_training_sword_damage() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let attributes = RulesetAttributeMap::from([("STR".to_string(), 12.0)]);
        let summary = summarize_weapon_damage(&table, "training_sword", &attributes).unwrap();

        assert_eq!(summary.spec.roll, "1d6");
        assert_eq!(summary.spec.bonus, 1.0);
        assert_eq!(summary.attribute_value, 12.0);
        assert_eq!(summary.attribute_bonus, 3.0);
        assert_eq!(summary.total_bonus, 4.0);
        assert_eq!(summary.minimum, 5.0);
        assert_eq!(summary.maximum, 10.0);
        assert_eq!(summary.average, 7.5);
        assert_eq!(summary.spec.damage_kind.as_deref(), Some("physical"));
    }

    #[test]
    fn summarizes_official_hunting_bow_damage() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let attributes = RulesetAttributeMap::from([("DEX".to_string(), 12.0)]);
        let summary = summarize_weapon_damage(&table, "hunting_bow", &attributes).unwrap();

        assert_eq!(summary.spec.roll, "1d6");
        assert_eq!(summary.spec.bonus, 0.0);
        assert_eq!(summary.attribute_value, 12.0);
        assert_eq!(summary.attribute_bonus, 3.0);
        assert_eq!(summary.minimum, 4.0);
        assert_eq!(summary.maximum, 9.0);
        assert_eq!(summary.spec.damage_kind.as_deref(), Some("physical"));
    }

    #[test]
    fn reads_official_xp_table() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();

        assert_eq!(ruleset_xp_for_level(&table, 2), Some(100));
        assert_eq!(ruleset_xp_for_level(&table, 5), Some(700));
        assert_eq!(ruleset_xp_for_level(&table, 99), None);
    }

    #[test]
    fn summarizes_official_spell_rolls() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let fire_attrs = RulesetAttributeMap::from([("INT".to_string(), 12.0)]);
        let (kind, fire) = summarize_spell_roll(&table, "fire_spark", &fire_attrs).unwrap();

        assert_eq!(kind, RulesetSpellRollKind::Damage);
        assert_eq!(fire.spec.roll, "1d6");
        assert_eq!(fire.attribute_bonus, 3.0);
        assert_eq!(fire.spec.damage_kind.as_deref(), Some("fire"));

        let heal_attrs = RulesetAttributeMap::from([("WIS".to_string(), 12.0)]);
        let (kind, heal) = summarize_spell_roll(&table, "minor_heal", &heal_attrs).unwrap();

        assert_eq!(kind, RulesetSpellRollKind::Healing);
        assert_eq!(heal.spec.roll, "1d6");
        assert_eq!(heal.total_bonus, 4.0);

        let (kind, holy) = summarize_spell_roll(&table, "holy_light", &heal_attrs).unwrap();

        assert_eq!(kind, RulesetSpellRollKind::Damage);
        assert_eq!(holy.spec.roll, "1d6");
        assert_eq!(holy.spec.damage_kind.as_deref(), Some("arcane"));
    }

    #[test]
    fn summarizes_official_warrior_class() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let warrior = summarize_class(&table, "Warrior").unwrap();

        assert_eq!(warrior.role.as_deref(), Some("martial"));
        assert_eq!(
            warrior.attributes.get("STR").map(String::as_str),
            Some("12")
        );
        assert!(warrior.spells.is_empty());
        assert!(warrior.abilities.iter().any(|ability| ability == "guard"));
        assert!(warrior.level_unlocks.get("level_2").is_some_and(|unlocks| {
            unlocks
                .iter()
                .any(|entry| entry == "abilities:power_strike")
        }));
    }

    #[test]
    fn summarizes_official_citizen_class() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let citizen = summarize_class(&table, "Citizen").unwrap();

        assert_eq!(citizen.role.as_deref(), Some("civilian"));
        assert!(citizen.abilities.is_empty());
        assert!(citizen.spells.is_empty());
        assert_eq!(
            citizen
                .attributes
                .get("inventory_slots")
                .map(String::as_str),
            Some("8")
        );
        assert!(
            citizen
                .starting_loadout
                .get("clothing")
                .is_some_and(|items| items.iter().any(|item| item == "linen_shirt"))
        );
    }

    #[test]
    fn summarizes_official_cleric_class() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let cleric = summarize_class(&table, "Cleric").unwrap();

        assert_eq!(cleric.role.as_deref(), Some("divine"));
        assert_eq!(cleric.attributes.get("WIS").map(String::as_str), Some("12"));
        assert!(cleric.spells.iter().any(|spell| spell == "minor_heal"));
        assert!(cleric.allowed_weapons.iter().any(|weapon| weapon == "mace"));
        assert!(
            cleric.level_unlocks.get("level_2").is_some_and(|unlocks| {
                unlocks.iter().any(|entry| entry == "spells:holy_light")
            })
        );
        assert!(
            cleric
                .starting_loadout
                .get("inventory")
                .is_some_and(|items| items.iter().any(|item| item == "blessed_herb"))
        );
        assert!(
            cleric
                .starting_loadout
                .get("weapons")
                .is_some_and(|weapons| weapons.iter().any(|weapon| weapon == "novice_mace"))
        );
    }

    #[test]
    fn summarizes_official_ranger_class() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let ranger = summarize_class(&table, "Ranger").unwrap();

        assert_eq!(ranger.role.as_deref(), Some("ranged"));
        assert_eq!(ranger.attributes.get("DEX").map(String::as_str), Some("12"));
        assert!(ranger.allowed_weapons.iter().any(|weapon| weapon == "bow"));
        assert!(
            ranger
                .starting_loadout
                .get("weapons")
                .is_some_and(|weapons| weapons.iter().any(|weapon| weapon == "hunting_bow"))
        );
        assert!(
            ranger
                .starting_loadout
                .get("inventory")
                .is_some_and(|items| items.iter().any(|item| item == "wooden_arrows"))
        );
    }

    #[test]
    fn extracts_official_item_templates() {
        let templates = ruleset_item_templates_from_source(latest_official_ruleset()).unwrap();

        assert!(templates.iter().any(|template| {
            template.id == "training_sword"
                && template.kind == "weapon"
                && template.ruleset_path == "items.weapons.training_sword"
                && template
                    .data
                    .contains("ruleset_path = \"items.weapons.training_sword\"")
                && template
                    .data
                    .contains("visual_template = \"sword_diagonal\"")
                && template
                    .data
                    .contains("description = \"A blunt wooden practice sword")
                && template.data.contains("blade_color_index = 10")
                && template.data.contains("highlight_color_index = 14")
                && template.data.contains("visual_template_pixels")
                && template.data.contains("rig_scale = 0.85")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "hand_axe"
                && template.data.contains("visual_template = \"axe_diagonal\"")
                && template.data.contains("visual_template_pixels")
                && template.data.contains("on_look = \"A compact iron axe")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "novice_mace"
                && template
                    .data
                    .contains("visual_template = \"mace_diagonal\"")
                && template.data.contains("visual_template_pixels")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "hunting_bow"
                && template.data.contains("visual_template = \"bow_diagonal\"")
                && template.data.contains("visual_template_pixels")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "training_spear"
                && template
                    .data
                    .contains("visual_template = \"spear_diagonal\"")
                && template.data.contains("visual_template_pixels")
                && template
                    .data
                    .contains("on_look = \"A simple practice spear")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "wooden_arrows"
                && template.kind == "ammunition"
                && template.ruleset_path == "items.ammunition.wooden_arrows"
                && template
                    .data
                    .contains("visual_template = \"arrow_diagonal\"")
                && template.data.contains("visual_template_pixels")
                && template
                    .data
                    .contains("on_look = \"A bundle of plain wooden arrows for bows.\"")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "blessed_herb"
                && template.kind == "reagent"
                && template.ruleset_path == "items.reagents.blessed_herb"
                && template.data.contains("visual_template = \"herb_sprig\"")
                && template.data.contains("visual_template_pixels")
                && template
                    .data
                    .contains("on_look = \"A small bundle of blessed herbs")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "green_wood"
                && template.kind == "material"
                && template.ruleset_path == "items.materials.green_wood"
                && template
                    .data
                    .contains("visual_template = \"spear_diagonal\"")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "feather"
                && template.kind == "material"
                && template.ruleset_path == "items.materials.feather"
        }));
        assert!(templates.iter().any(|template| {
            template.id == "wild_herb"
                && template.kind == "material"
                && template.ruleset_path == "items.materials.wild_herb"
        }));
        assert!(
            templates
                .iter()
                .any(|template| template.id == "padded_armor")
        );
        assert!(
            templates
                .iter()
                .any(|template| template.id == "leather_vest")
        );
        assert!(
            templates
                .iter()
                .any(|template| template.id == "chain_shirt")
        );
        assert!(templates.iter().any(|template| {
            template.id == "round_shield"
                && template.data.contains("visual_template = \"shield\"")
                && template.data.contains("visual_template_pixels")
                && template.data.contains("on_look = \"A round wooden shield")
        }));
        assert!(templates.iter().any(|template| {
            template.id == "linen_shirt"
                && template.kind == "clothing"
                && template.ruleset_path == "items.clothing.linen_shirt"
                && template.data.contains("color_index = 2")
                && template.data.contains("torso_index = 2")
                && template.data.contains("arms_index = 2")
        }));
    }

    #[test]
    fn extracts_official_ruleset_palette() {
        let palette = ruleset_palette_from_source(latest_official_ruleset()).unwrap();

        assert_eq!(
            palette.colors[2].as_ref().map(TheColor::to_hex).as_deref(),
            Some("#BCAD9F")
        );
        assert_eq!(
            palette.colors[30].as_ref().map(TheColor::to_hex).as_deref(),
            Some("#14233A")
        );
    }

    #[test]
    fn catalogs_and_shows_official_ruleset_paths() {
        let catalog = ruleset_catalog_from_source(latest_official_ruleset()).unwrap();

        assert_eq!(catalog.id.as_deref(), Some(OFFICIAL_RULESET_ID));
        assert!(catalog.races.iter().any(|id| id == "Human"));
        assert!(catalog.races.iter().any(|id| id == "Orc"));
        assert!(catalog.classes.iter().any(|id| id == "Citizen"));
        assert!(catalog.classes.iter().any(|id| id == "Warrior"));
        assert!(catalog.classes.iter().any(|id| id == "Cleric"));
        assert!(catalog.classes.iter().any(|id| id == "Ranger"));
        assert!(catalog.professions.iter().any(|id| id == "Blacksmith"));
        assert!(catalog.professions.iter().any(|id| id == "Merchant"));
        assert!(catalog.skills.iter().any(|id| id == "fletching"));
        assert!(catalog.skills.iter().any(|id| id == "herbalism"));
        assert!(catalog.skills.iter().any(|id| id == "restoration"));
        assert!(catalog.resources.iter().any(|id| id == "wild_herb_node"));
        assert!(catalog.recipes.iter().any(|id| id == "wooden_arrows"));
        assert!(catalog.recipes.iter().any(|id| id == "blessed_herb"));
        assert!(catalog.actions.iter().any(|id| id == "basic_attack"));
        assert!(catalog.actions.iter().any(|id| id == "gather_herbs"));
        assert!(catalog.actions.iter().any(|id| id == "holy_light"));
        assert!(catalog.weapons.iter().any(|id| id == "training_sword"));
        assert!(catalog.weapons.iter().any(|id| id == "novice_mace"));
        assert!(catalog.weapons.iter().any(|id| id == "hunting_bow"));
        assert!(catalog.weapons.iter().any(|id| id == "training_spear"));
        assert!(catalog.armor.iter().any(|id| id == "chain_shirt"));
        assert!(catalog.clothing.iter().any(|id| id == "linen_shirt"));
        assert!(
            catalog
                .item_templates
                .iter()
                .any(|path| path == "items.weapons.training_sword")
        );
        assert!(
            catalog
                .item_templates
                .iter()
                .any(|path| path == "items.ammunition.wooden_arrows")
        );
        assert!(
            catalog
                .item_templates
                .iter()
                .any(|path| path == "items.reagents.blessed_herb")
        );
        assert!(
            catalog
                .item_templates
                .iter()
                .any(|path| path == "items.materials.green_wood")
        );
        assert!(
            catalog
                .item_templates
                .iter()
                .any(|path| path == "items.resources.wild_herb_node")
        );

        let classes =
            ruleset_section_ids_from_source(latest_official_ruleset(), "classes").unwrap();
        assert!(classes.iter().any(|id| id == "Warrior"));
        let professions =
            ruleset_section_ids_from_source(latest_official_ruleset(), "professions").unwrap();
        assert!(professions.iter().any(|id| id == "Herbalist"));
        let recipes =
            ruleset_section_ids_from_source(latest_official_ruleset(), "recipes").unwrap();
        assert!(recipes.iter().any(|id| id == "wooden_arrows"));
        let skills = ruleset_section_ids_from_source(latest_official_ruleset(), "skills").unwrap();
        assert!(skills.iter().any(|id| id == "fletching"));
        let actions =
            ruleset_section_ids_from_source(latest_official_ruleset(), "actions").unwrap();
        assert!(actions.iter().any(|id| id == "minor_heal"));

        let sword = ruleset_show_path_from_source(
            latest_official_ruleset(),
            &["items", "weapons", "training_sword"],
        )
        .unwrap()
        .unwrap();
        assert!(sword.contains("name = \"Training Sword\""));
        assert!(sword.contains("[attributes]"));
    }

    #[test]
    fn validates_official_ruleset_without_issues() {
        let report = validate_ruleset_from_source(latest_official_ruleset()).unwrap();

        assert_eq!(report.error_count(), 0, "{:?}", report.issues);
        assert_eq!(report.warning_count(), 0, "{:?}", report.issues);
        assert!(report.is_ok());
    }

    #[test]
    fn validation_reports_broken_references_and_rolls() {
        let report = validate_ruleset_from_source(
            r#"
            [assets.avatars.humanoid]
            path = "assets/humanoid.eldiron_avatar"

            [visuals.defaults]
            avatar = "missing_avatar"

            [identity.defaults]
            race = "Human"
            class = "Cleric"

            [progression.level]
            max_level = 3

            [progression.xp_table]
            level_2 = 100
            level_3 = 90

            [combat.kinds.physical]

            [equipment]
            weapon_slots = ["main_hand"]
            armor_slots = ["body"]

            [equipment.weapon_categories.sword]
            [equipment.armor_categories.cloth]

            [items.weapons.training_sword]
            name = "Training Sword"
            category = "missing_category"
            slot = "main_hand"

            [items.weapons.training_sword.damage]
            roll = "6"
            damage_kind = "shadow"

            [abilities.basic_attack]
            damage_kind = "physical"

            [classes.Warrior]
            allowed_weapons = ["axe"]
            allowed_armor = ["cloth"]
            abilities = ["missing_ability"]

            [classes.Warrior.starting_loadout]
            weapons = ["missing_sword"]

            [races.Human]
            default_avatar = "humanoid"
            "#,
        )
        .unwrap();

        assert!(report.error_count() >= 6, "{:?}", report.issues);
        assert!(report.issues.iter().any(|issue| {
            issue
                .path
                .contains("items.weapons.training_sword.damage.roll")
        }));
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "visuals.defaults.avatar")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "classes.Warrior.abilities")
        );
    }

    #[test]
    fn reads_top_level_ruleset_selection() {
        let (id, version, source) = selected_ruleset(
            r#"
            [ruleset]
            id = "eldiron.official"
            version = "1.0"
            source = "official"
            "#,
        );

        assert_eq!(id, OFFICIAL_RULESET_ID);
        assert_eq!(version, "1.0");
        assert_eq!(source, "official");
    }

    #[test]
    fn prefixes_default_ruleset_config() {
        let mut config = "[game]\nname = \"Old Project\"\n".to_string();
        prefix_default_ruleset_config(&mut config);

        assert!(has_top_level_ruleset(&config));
        assert!(config.starts_with("[ruleset]\n"));
        let (id, version, source) = selected_ruleset(&config);
        assert_eq!(id, OFFICIAL_RULESET_ID);
        assert_eq!(version, OFFICIAL_RULESET_VERSION);
        assert_eq!(source, "official");
    }
}
