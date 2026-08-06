use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};
use theframework::prelude::{TheColor, ThePalette};
use toml::{Table, Value};

pub mod cli;
mod formula;
pub use formula::{evaluate_formula, formula_identifiers, formula_is_valid};

pub const OFFICIAL_RULESET_ID: &str = "eldiron.official";
pub const OFFICIAL_RULESET_VERSION: &str = "1.0.0";
pub const OFFICIAL_RULESET_SCHEMA_VERSION: &str = "1";
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SUPPORTED_RULESET_SCHEMA_VERSIONS: &[&str] = &["1"];
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

const OFFICIAL_ELDIRON_V1_CORE: &str = include_str!("../rulesets/eldiron/v1/ruleset.toml");
const OFFICIAL_ELDIRON_V1_IDENTITY: &str = include_str!("../rulesets/eldiron/v1/identity.toml");
const OFFICIAL_ELDIRON_V1_ATTRIBUTES: &str = include_str!("../rulesets/eldiron/v1/attributes.toml");
const OFFICIAL_ELDIRON_V1_PROGRESSION: &str =
    include_str!("../rulesets/eldiron/v1/progression.toml");
const OFFICIAL_ELDIRON_V1_COMBAT: &str = include_str!("../rulesets/eldiron/v1/combat.toml");
const OFFICIAL_ELDIRON_V1_ECONOMY: &str = include_str!("../rulesets/eldiron/v1/economy.toml");
const OFFICIAL_ELDIRON_V1_MESSAGES: &str = include_str!("../rulesets/eldiron/v1/messages.toml");
const OFFICIAL_ELDIRON_V1_EQUIPMENT: &str = include_str!("../rulesets/eldiron/v1/equipment.toml");
const OFFICIAL_ELDIRON_V1_FX: &str = include_str!("../rulesets/eldiron/v1/fx.toml");
const OFFICIAL_ELDIRON_V1_ICONS: &str = include_str!("../rulesets/eldiron/v1/icons.toml");
const OFFICIAL_ELDIRON_V1_INVOCATIONS: &str =
    include_str!("../rulesets/eldiron/v1/invocations.toml");
const OFFICIAL_ELDIRON_V1_CONDITIONS: &str = include_str!("../rulesets/eldiron/v1/conditions.toml");
const OFFICIAL_ELDIRON_V1_ACTIONS: &str = include_str!("../rulesets/eldiron/v1/actions.toml");
const OFFICIAL_ELDIRON_V1_RECIPES: &str = include_str!("../rulesets/eldiron/v1/recipes.toml");
const OFFICIAL_ELDIRON_V1_ABILITIES_SPELLS: &str =
    include_str!("../rulesets/eldiron/v1/abilities_spells.toml");
const OFFICIAL_ELDIRON_V1_RACES_CLASSES: &str =
    include_str!("../rulesets/eldiron/v1/races_classes.toml");
const OFFICIAL_ELDIRON_V1_LOCALES: &str = include_str!("../rulesets/eldiron/v1/locales.toml");
const OFFICIAL_ELDIRON_V1_HUMANOID_AVATAR: &str =
    include_str!("../rulesets/eldiron/v1/assets/humanoid.eldiron_avatar");
const OFFICIAL_ELDIRON_V1_ORC_AVATAR: &str =
    include_str!("../rulesets/eldiron/v1/assets/orc.eldiron_avatar");
const OFFICIAL_ELDIRON_V1_SKELETON_AVATAR: &str =
    include_str!("../rulesets/eldiron/v1/assets/skeleton.eldiron_avatar");

static OFFICIAL_ELDIRON_V1: LazyLock<String> = LazyLock::new(|| {
    [
        OFFICIAL_ELDIRON_V1_CORE,
        OFFICIAL_ELDIRON_V1_IDENTITY,
        OFFICIAL_ELDIRON_V1_ATTRIBUTES,
        OFFICIAL_ELDIRON_V1_PROGRESSION,
        OFFICIAL_ELDIRON_V1_COMBAT,
        OFFICIAL_ELDIRON_V1_ECONOMY,
        OFFICIAL_ELDIRON_V1_MESSAGES,
        OFFICIAL_ELDIRON_V1_EQUIPMENT,
        OFFICIAL_ELDIRON_V1_FX,
        OFFICIAL_ELDIRON_V1_ICONS,
        OFFICIAL_ELDIRON_V1_INVOCATIONS,
        OFFICIAL_ELDIRON_V1_CONDITIONS,
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

/// Ruleset selection stored in Game / Settings.
///
/// This is intentionally independent from the bundled official ruleset. A
/// project-owned ruleset uses the same selection and resolution path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetSelection {
    pub id: String,
    pub version: String,
    pub schema_version: String,
    pub source: String,
    pub update_policy: String,
}

impl Default for RulesetSelection {
    fn default() -> Self {
        Self {
            id: OFFICIAL_RULESET_ID.to_string(),
            version: OFFICIAL_RULESET_VERSION.to_string(),
            schema_version: OFFICIAL_RULESET_SCHEMA_VERSION.to_string(),
            source: "official".to_string(),
            update_policy: "compatible".to_string(),
        }
    }
}

/// Typed metadata resolved from a ruleset's `[ruleset]` table, with project
/// selection values used as migration fallbacks for schema-1 project rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesetMetadata {
    pub id: String,
    pub name: Option<String>,
    pub version: String,
    pub schema_version: String,
    pub min_engine_version: Option<String>,
    pub status: Option<String>,
}

/// Parsed author-facing rules before selection, inheritance, and project
/// overrides have been resolved.
///
/// Schema 1 is still represented by a TOML table internally. This boundary is
/// where authoring constructs are migrated into typed domain definitions.
#[derive(Clone, Debug, PartialEq)]
pub struct RawRuleset {
    table: Table,
}

impl RawRuleset {
    pub fn parse(src: &str, source_label: &str) -> Result<Self, String> {
        let table = src
            .parse::<Table>()
            .map_err(|err| format!("{} TOML parse error: {}", source_label, err))?;
        Ok(Self { table })
    }

    pub fn from_table(table: Table) -> Self {
        Self { table }
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn into_table(self) -> Table {
        self.table
    }

    pub fn merge_overlay(&mut self, overlay: RawRuleset) {
        merge_toml_tables(&mut self.table, overlay.table);
    }
}

/// Effective schema-1 rules after official/project selection and overrides.
///
/// Runtime consumers temporarily continue to receive serialized TOML while
/// they migrate to this type. That adapter is not a promise that older project
/// representations remain loadable.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedRuleset {
    selection: RulesetSelection,
    metadata: RulesetMetadata,
    table: Table,
    validation: RulesetValidationReport,
}

impl ResolvedRuleset {
    fn from_raw(raw: RawRuleset, selection: RulesetSelection) -> Result<Self, String> {
        ensure_supported_schema_version(&selection.schema_version, "Project ruleset selection")?;

        let table = raw.into_table();
        let metadata_table = table.get("ruleset").and_then(Value::as_table);
        let declared_schema_version = metadata_table
            .and_then(|ruleset| ruleset.get("schema_version"))
            .and_then(Value::as_str);
        if let Some(declared) = declared_schema_version {
            ensure_supported_schema_version(declared, "Resolved ruleset")?;
            if declared != selection.schema_version {
                return Err(format!(
                    "Project selects ruleset schema {}, but the resolved ruleset declares schema {}.",
                    selection.schema_version, declared
                ));
            }
        }

        let metadata = RulesetMetadata {
            id: metadata_table
                .and_then(|ruleset| ruleset.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(&selection.id)
                .to_string(),
            name: metadata_table
                .and_then(|ruleset| ruleset.get("name"))
                .and_then(Value::as_str)
                .map(str::to_string),
            version: metadata_table
                .and_then(|ruleset| ruleset.get("version"))
                .and_then(Value::as_str)
                .unwrap_or(&selection.version)
                .to_string(),
            schema_version: declared_schema_version
                .unwrap_or(&selection.schema_version)
                .to_string(),
            min_engine_version: metadata_table
                .and_then(|ruleset| ruleset.get("min_engine_version"))
                .and_then(Value::as_str)
                .map(str::to_string),
            status: metadata_table
                .and_then(|ruleset| ruleset.get("status"))
                .and_then(Value::as_str)
                .map(str::to_string),
        };
        if let Some(min_engine_version) = metadata.min_engine_version.as_deref() {
            ensure_minimum_engine_version(min_engine_version)?;
        }
        let validation = validate_ruleset(&table);

        Ok(Self {
            selection,
            metadata,
            table,
            validation,
        })
    }

    pub fn selection(&self) -> &RulesetSelection {
        &self.selection
    }

    pub fn metadata(&self) -> &RulesetMetadata {
        &self.metadata
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn validation(&self) -> &RulesetValidationReport {
        &self.validation
    }

    pub fn action(&self, action_id: &str) -> Result<Option<ResolvedAction>, String> {
        resolve_action(&self.table, action_id)
    }

    pub fn actions(&self) -> Result<BTreeMap<String, ResolvedAction>, String> {
        resolve_actions(&self.table)
    }

    pub fn invocation_schemes(&self) -> Result<BTreeMap<String, ResolvedInvocationScheme>, String> {
        resolve_invocation_schemes(&self.table)
    }

    pub fn conditions(&self) -> Result<BTreeMap<String, ResolvedCondition>, String> {
        resolve_conditions(&self.table)
    }

    pub fn derived_stats(&self) -> Result<BTreeMap<String, ResolvedDerivedStat>, String> {
        resolve_derived_stats(&self.table)
    }

    pub fn identity_defaults(&self) -> Result<ResolvedIdentityDefaults, String> {
        resolve_identity_defaults(&self.table)
    }

    pub fn attribute_roles(&self) -> Result<ResolvedAttributeRoles, String> {
        resolve_attribute_roles(&self.table)
    }

    pub fn equipment_policy(&self) -> Result<ResolvedEquipmentPolicy, String> {
        resolve_equipment_policy(&self.table)
    }

    pub fn action_for_invocation(
        &self,
        scheme_id: &str,
        phrase: &str,
    ) -> Result<Option<ResolvedAction>, String> {
        resolve_action_invocation(&self.table, scheme_id, phrase)
    }

    pub fn into_table(self) -> Table {
        self.table
    }

    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string(&self.table)
            .map_err(|err| format!("Effective ruleset serialize error: {}", err))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundledAvatarAsset {
    pub id: &'static str,
    pub ruleset_id: &'static str,
    pub ruleset_version: &'static str,
    pub path: &'static str,
    pub source: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundledTextureAsset {
    pub id: &'static str,
    pub ruleset_id: &'static str,
    pub ruleset_version: &'static str,
    pub path: &'static str,
    pub source: &'static [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundledTileAsset {
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
    &[
        BundledAvatarAsset {
            id: "humanoid",
            ruleset_id: OFFICIAL_RULESET_ID,
            ruleset_version: OFFICIAL_RULESET_VERSION,
            path: "assets/humanoid.eldiron_avatar",
            source: OFFICIAL_ELDIRON_V1_HUMANOID_AVATAR,
        },
        BundledAvatarAsset {
            id: "orc",
            ruleset_id: OFFICIAL_RULESET_ID,
            ruleset_version: OFFICIAL_RULESET_VERSION,
            path: "assets/orc.eldiron_avatar",
            source: OFFICIAL_ELDIRON_V1_ORC_AVATAR,
        },
        BundledAvatarAsset {
            id: "skeleton",
            ruleset_id: OFFICIAL_RULESET_ID,
            ruleset_version: OFFICIAL_RULESET_VERSION,
            path: "assets/skeleton.eldiron_avatar",
            source: OFFICIAL_ELDIRON_V1_SKELETON_AVATAR,
        },
    ]
}

pub fn bundled_texture_assets() -> &'static [BundledTextureAsset] {
    macro_rules! official_icon {
        ($id:literal) => {
            BundledTextureAsset {
                id: $id,
                ruleset_id: OFFICIAL_RULESET_ID,
                ruleset_version: OFFICIAL_RULESET_VERSION,
                path: concat!("assets/icons/", $id, ".png"),
                source: include_bytes!(concat!(
                    "../rulesets/eldiron/v1/assets/icons/",
                    $id,
                    ".png"
                )),
            }
        };
    }

    &[
        official_icon!("walk"),
        official_icon!("look"),
        official_icon!("use"),
        official_icon!("take"),
        official_icon!("drop"),
        official_icon!("basic_attack"),
        official_icon!("power_strike"),
        official_icon!("minor_heal"),
        official_icon!("holy_light"),
        official_icon!("gather_herbs"),
        official_icon!("gather_wood"),
        official_icon!("gather_feathers"),
        official_icon!("training_sword"),
        official_icon!("hand_axe"),
        official_icon!("novice_mace"),
        official_icon!("hunting_bow"),
        official_icon!("training_spear"),
        official_icon!("padded_armor"),
        official_icon!("leather_vest"),
        official_icon!("chain_shirt"),
        official_icon!("round_shield"),
        official_icon!("linen_shirt"),
        official_icon!("wool_trousers"),
        official_icon!("leather_shoes"),
        official_icon!("small_bag"),
        official_icon!("loot_corpse"),
        official_icon!("wooden_arrows"),
        official_icon!("blessed_herb"),
        official_icon!("green_wood"),
        official_icon!("feather"),
        official_icon!("wild_herb"),
        official_icon!("wild_herb_node"),
        official_icon!("green_wood_node"),
        official_icon!("bird_nest_node"),
        official_icon!("torch"),
    ]
}

pub fn bundled_tile_assets() -> &'static [BundledTileAsset] {
    macro_rules! official_tile {
        ($id:literal) => {
            BundledTileAsset {
                id: $id,
                ruleset_id: OFFICIAL_RULESET_ID,
                ruleset_version: OFFICIAL_RULESET_VERSION,
                path: concat!("assets/tiles/", $id, ".eldiron_tile"),
                source: include_str!(concat!(
                    "../rulesets/eldiron/v1/assets/tiles/",
                    $id,
                    ".eldiron_tile"
                )),
            }
        };
    }

    &[official_tile!("torch_off"), official_tile!("torch_on")]
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

pub fn bundled_texture_assets_for_ruleset(
    ruleset_id: &str,
    ruleset_version: &str,
) -> Vec<&'static BundledTextureAsset> {
    bundled_texture_assets()
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

pub fn bundled_tile_assets_for_ruleset(
    ruleset_id: &str,
    ruleset_version: &str,
) -> Vec<&'static BundledTileAsset> {
    bundled_tile_assets()
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

fn ensure_supported_schema_version(schema_version: &str, context: &str) -> Result<(), String> {
    if SUPPORTED_RULESET_SCHEMA_VERSIONS.contains(&schema_version) {
        return Ok(());
    }
    Err(format!(
        "{} requires unsupported schema version '{}'. This build supports: {}.",
        context,
        schema_version,
        SUPPORTED_RULESET_SCHEMA_VERSIONS.join(", ")
    ))
}

fn parse_release_version(version: &str, context: &str) -> Result<(u64, u64, u64), String> {
    let release = version
        .split_once(['-', '+'])
        .map(|(release, _)| release)
        .unwrap_or(version);
    let parts = release.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!(
            "{} '{}' is not a valid three-part version.",
            context, version
        ));
    }

    let parse_part = |part: &str| {
        part.parse::<u64>().map_err(|_| {
            format!(
                "{} '{}' is not a valid three-part version.",
                context, version
            )
        })
    };
    Ok((
        parse_part(parts[0])?,
        parse_part(parts[1])?,
        parse_part(parts[2])?,
    ))
}

fn ensure_minimum_engine_version(min_engine_version: &str) -> Result<(), String> {
    let required = parse_release_version(min_engine_version, "Minimum engine version")?;
    let current = parse_release_version(ENGINE_VERSION, "Current engine version")?;
    if current >= required {
        return Ok(());
    }

    Err(format!(
        "Ruleset requires Eldiron engine {}, but this build is {}.",
        min_engine_version, ENGINE_VERSION
    ))
}

/// Resolve the selected schema-1 ruleset while retaining typed selection,
/// metadata, validation, and effective-table access.
pub fn resolve_project_ruleset(
    config_src: &str,
    override_src: &str,
) -> Result<ResolvedRuleset, String> {
    let selection = selected_ruleset_config(config_src);
    ensure_supported_schema_version(&selection.schema_version, "Project ruleset selection")?;

    if selection.source == "project" {
        let raw = RawRuleset::parse(override_src, "Project ruleset")?;
        return ResolvedRuleset::from_raw(raw, selection);
    }

    let base_src = official_ruleset(&selection.id, &selection.version).ok_or_else(|| {
        let available = bundled_rulesets()
            .iter()
            .map(|ruleset| format!("{}@{}", ruleset.id, ruleset.version))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Project requires {}@{}, but this build includes: {}",
            selection.id, selection.version, available
        )
    })?;
    let mut raw = RawRuleset::parse(base_src, "Official ruleset")?;
    if !override_src.trim().is_empty() {
        raw.merge_overlay(RawRuleset::parse(override_src, "Rules override")?);
    }
    ResolvedRuleset::from_raw(raw, selection)
}

pub fn resolve_project_rules(config_src: &str, override_src: &str) -> Result<String, String> {
    resolve_project_ruleset(config_src, override_src)?.to_toml_string()
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

pub fn selected_ruleset_config(config_src: &str) -> RulesetSelection {
    let Ok(config) = config_src.parse::<Table>() else {
        return RulesetSelection::default();
    };

    let ruleset = config.get("ruleset").and_then(Value::as_table).or_else(|| {
        config
            .get("game")
            .and_then(Value::as_table)
            .and_then(|game| game.get("ruleset"))
            .and_then(Value::as_table)
    });

    let defaults = RulesetSelection::default();
    RulesetSelection {
        id: ruleset
            .and_then(|ruleset| ruleset.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(&defaults.id)
            .to_string(),
        version: ruleset
            .and_then(|ruleset| ruleset.get("version"))
            .and_then(Value::as_str)
            .unwrap_or(&defaults.version)
            .to_string(),
        schema_version: ruleset
            .and_then(|ruleset| ruleset.get("schema_version"))
            .and_then(Value::as_str)
            .unwrap_or(&defaults.schema_version)
            .to_string(),
        source: ruleset
            .and_then(|ruleset| ruleset.get("source"))
            .and_then(Value::as_str)
            .unwrap_or(&defaults.source)
            .to_string(),
        update_policy: ruleset
            .and_then(|ruleset| ruleset.get("update_policy"))
            .and_then(Value::as_str)
            .unwrap_or(&defaults.update_policy)
            .to_string(),
    }
}

pub fn selected_ruleset(config_src: &str) -> (String, String, String) {
    let selection = selected_ruleset_config(config_src);
    (selection.id, selection.version, selection.source)
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
    let mut base = RawRuleset::parse(base_src, "Official ruleset")?;

    if !override_src.trim().is_empty() {
        base.merge_overlay(RawRuleset::parse(override_src, "Rules override")?);
    }

    toml::to_string(base.table())
        .map_err(|err| format!("Effective ruleset serialize error: {}", err))
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedActionKind {
    Attack,
    Spell,
    Interaction,
    Gather,
    Craft,
    Custom(String),
}

impl ResolvedActionKind {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "attack" => Self::Attack,
            "spell" => Self::Spell,
            "interaction" => Self::Interaction,
            "gather" => Self::Gather,
            "craft" => Self::Craft,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::Attack => "attack",
            Self::Spell => "spell",
            Self::Interaction => "interaction",
            Self::Gather => "gather",
            Self::Craft => "craft",
            Self::Custom(value) => value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedActionTarget {
    SelfTarget,
    HostileEntity,
    HostileOrNeutralEntity,
    FriendlyEntity,
    FriendlyOrSelf,
    AnyEntity,
    GroundItem,
    InventoryItem,
    AnyItem,
    ResourceNode,
    WorldPosition,
    Custom(String),
}

impl ResolvedActionTarget {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "self" => Self::SelfTarget,
            "hostile_entity" => Self::HostileEntity,
            "hostile_or_neutral_entity" => Self::HostileOrNeutralEntity,
            "friendly_entity" => Self::FriendlyEntity,
            "friendly_or_self" => Self::FriendlyOrSelf,
            "any_entity" => Self::AnyEntity,
            "ground_item" => Self::GroundItem,
            "inventory_item" => Self::InventoryItem,
            "item" | "any_item" => Self::AnyItem,
            "resource_node" | "resource" => Self::ResourceNode,
            "position" | "world_position" | "terrain" | "ground" => Self::WorldPosition,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::SelfTarget => "self",
            Self::HostileEntity => "hostile_entity",
            Self::HostileOrNeutralEntity => "hostile_or_neutral_entity",
            Self::FriendlyEntity => "friendly_entity",
            Self::FriendlyOrSelf => "friendly_or_self",
            Self::AnyEntity => "any_entity",
            Self::GroundItem => "ground_item",
            Self::InventoryItem => "inventory_item",
            Self::AnyItem => "any_item",
            Self::ResourceNode => "resource_node",
            Self::WorldPosition => "world_position",
            Self::Custom(value) => value,
        }
    }

    fn is_entity_target(&self) -> bool {
        matches!(
            self,
            Self::SelfTarget
                | Self::HostileEntity
                | Self::HostileOrNeutralEntity
                | Self::FriendlyEntity
                | Self::FriendlyOrSelf
                | Self::AnyEntity
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedActionRange {
    Default,
    Fixed(f32),
    Weapon { fallback: f32 },
    Source { source: String, fallback: f32 },
}

impl ResolvedActionRange {
    fn parse(value: Option<&Value>, path: &str) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self::Default);
        };
        if let Some(number) = value
            .as_float()
            .or_else(|| value.as_integer().map(|value| value as f64))
        {
            let number = number as f32;
            if number.is_finite() && number >= 0.0 {
                return Ok(Self::Fixed(number));
            }
            return Err(format!(
                "{}.range must be a finite number at least zero.",
                path
            ));
        }
        if let Some(source) = value.as_str() {
            return match source.trim().to_ascii_lowercase().as_str() {
                "weapon" | "weapon_range" => Ok(Self::Weapon { fallback: 1.5 }),
                source if !source.is_empty() => Ok(Self::Source {
                    source: source.to_string(),
                    fallback: 1.5,
                }),
                _ => Err(format!("{}.range must not be empty.", path)),
            };
        }
        if let Some(table) = value.as_table() {
            let source = table_string(table, "source")
                .ok_or_else(|| format!("{}.range.source is required.", path))?;
            let fallback = table_number(table, "fallback", 1.5);
            if !fallback.is_finite() || fallback < 0.0 {
                return Err(format!(
                    "{}.range.fallback must be a finite number at least zero.",
                    path
                ));
            }
            if matches!(
                source.trim().to_ascii_lowercase().as_str(),
                "weapon" | "weapon_range"
            ) {
                return Ok(Self::Weapon { fallback });
            }
            return Ok(Self::Source { source, fallback });
        }
        Err(format!(
            "{}.range must be a number, source name, or source table.",
            path
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedActionPredicateValue {
    Bool(bool),
    Number(f32),
    String(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedActionAttributePredicate {
    Equals(ResolvedActionPredicateValue),
    NotEquals(ResolvedActionPredicateValue),
    AtLeast(f32),
    AtMost(f32),
    Contains(String),
    NotContains(String),
}

impl ResolvedActionAttributePredicate {
    pub fn matches_bool(&self, actual: bool) -> bool {
        match self {
            Self::Equals(ResolvedActionPredicateValue::Bool(expected)) => actual == *expected,
            Self::NotEquals(ResolvedActionPredicateValue::Bool(expected)) => actual != *expected,
            _ => false,
        }
    }

    pub fn matches_number(&self, actual: f32) -> bool {
        match self {
            Self::Equals(ResolvedActionPredicateValue::Number(expected)) => actual == *expected,
            Self::NotEquals(ResolvedActionPredicateValue::Number(expected)) => actual != *expected,
            Self::AtLeast(expected) => actual >= *expected,
            Self::AtMost(expected) => actual <= *expected,
            _ => false,
        }
    }

    pub fn matches_string(&self, actual: &str) -> bool {
        match self {
            Self::Equals(ResolvedActionPredicateValue::String(expected)) => actual == expected,
            Self::NotEquals(ResolvedActionPredicateValue::String(expected)) => actual != expected,
            Self::Contains(expected) => actual
                .split(',')
                .map(str::trim)
                .any(|value| value == expected),
            Self::NotContains(expected) => !actual
                .split(',')
                .map(str::trim)
                .any(|value| value == expected),
            _ => false,
        }
    }

    pub fn matches_strings(&self, actual: &[String]) -> bool {
        match self {
            Self::Contains(expected) => actual.iter().any(|value| value.trim() == expected),
            Self::NotContains(expected) => !actual.iter().any(|value| value.trim() == expected),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedActionRequirement {
    Ability(String),
    Spell(String),
    Profession(String),
    Skill {
        id: String,
        minimum: i32,
    },
    Attribute {
        id: String,
        predicate: ResolvedActionAttributePredicate,
    },
    TargetAttribute {
        id: String,
        predicate: ResolvedActionAttributePredicate,
    },
}

fn parse_action_predicate_value(
    value: &Value,
    path: &str,
) -> Result<ResolvedActionPredicateValue, String> {
    if let Some(value) = value.as_bool() {
        return Ok(ResolvedActionPredicateValue::Bool(value));
    }
    if let Some(value) = value
        .as_float()
        .or_else(|| value.as_integer().map(|value| value as f64))
    {
        let value = value as f32;
        if value.is_finite() {
            return Ok(ResolvedActionPredicateValue::Number(value));
        }
        return Err(format!("{} must be a finite number.", path));
    }
    if let Some(value) = value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(ResolvedActionPredicateValue::String(value.to_string()));
    }
    Err(format!(
        "{} must be a boolean, finite number, or non-empty string.",
        path
    ))
}

fn parse_action_attribute_requirement(
    value: &Value,
    path: &str,
    target: bool,
) -> Result<ResolvedActionRequirement, String> {
    let table = value
        .as_table()
        .ok_or_else(|| format!("{} must be an attribute predicate table.", path))?;
    let id = table_string(table, "id").ok_or_else(|| format!("{}.id is required.", path))?;
    let operators = [
        "equals",
        "not_equals",
        "at_least",
        "at_most",
        "contains",
        "not_contains",
    ]
    .into_iter()
    .filter(|key| table.contains_key(*key))
    .collect::<Vec<_>>();
    if operators.len() != 1 {
        return Err(format!(
            "{} must define exactly one of equals, not_equals, at_least, at_most, contains, or not_contains.",
            path
        ));
    }
    let operator = operators[0];
    let predicate = match operator {
        "equals" => ResolvedActionAttributePredicate::Equals(parse_action_predicate_value(
            &table[operator],
            &format!("{}.{}", path, operator),
        )?),
        "not_equals" => ResolvedActionAttributePredicate::NotEquals(parse_action_predicate_value(
            &table[operator],
            &format!("{}.{}", path, operator),
        )?),
        "at_least" | "at_most" => {
            let number = table[operator]
                .as_float()
                .or_else(|| table[operator].as_integer().map(|value| value as f64))
                .map(|value| value as f32)
                .ok_or_else(|| format!("{}.{} must be a number.", path, operator))?;
            if !number.is_finite() {
                return Err(format!("{}.{} must be a finite number.", path, operator));
            }
            if operator == "at_least" {
                ResolvedActionAttributePredicate::AtLeast(number)
            } else {
                ResolvedActionAttributePredicate::AtMost(number)
            }
        }
        "contains" | "not_contains" => {
            let member = table[operator]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{}.{} must be a non-empty string.", path, operator))?
                .to_string();
            if operator == "contains" {
                ResolvedActionAttributePredicate::Contains(member)
            } else {
                ResolvedActionAttributePredicate::NotContains(member)
            }
        }
        _ => unreachable!("operator list is exhaustive"),
    };
    if target {
        Ok(ResolvedActionRequirement::TargetAttribute { id, predicate })
    } else {
        Ok(ResolvedActionRequirement::Attribute { id, predicate })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedConditionStacking {
    Replace,
    Refresh,
    Stack,
    Ignore,
}

impl ResolvedConditionStacking {
    fn parse(value: &str, path: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "replace" => Ok(Self::Replace),
            "refresh" => Ok(Self::Refresh),
            "stack" => Ok(Self::Stack),
            "ignore" => Ok(Self::Ignore),
            other => Err(format!(
                "{}.stacking '{}' is unsupported; use replace, refresh, stack, or ignore.",
                path, other
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedConditionModifier {
    pub attribute: String,
    pub add: f32,
    pub multiply: f32,
    pub minimum: Option<f32>,
    pub maximum: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedConditionPeriodicEffect {
    Damage {
        amount: f32,
        damage_kind: String,
    },
    Healing {
        amount: f32,
    },
    Modify {
        field: ResolvedActionModificationField,
        add: f32,
        minimum: Option<f32>,
        maximum: Option<f32>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedConditionPeriodic {
    pub interval_seconds: f32,
    pub initial_delay_seconds: f32,
    pub effects: Vec<ResolvedConditionPeriodicEffect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedCondition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub duration_seconds: f32,
    pub stacking: ResolvedConditionStacking,
    pub max_stacks: usize,
    pub tags: Vec<String>,
    pub immune_traits: Vec<String>,
    pub modifiers: Vec<ResolvedConditionModifier>,
    pub periodic: Option<ResolvedConditionPeriodic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDerivedStat {
    pub id: String,
    pub formula: String,
    pub minimum: Option<f32>,
    pub maximum: Option<f32>,
    pub dependencies: BTreeSet<String>,
}

impl ResolvedDerivedStat {
    fn from_table(stat_id: &str, table: &Table) -> Result<Self, String> {
        let path = format!("derived_stats.{}", stat_id);
        let formula = table_string(table, "formula")
            .ok_or_else(|| format!("{}.formula is required.", path))?;
        if !formula_is_valid(&formula) {
            return Err(format!("{}.formula is invalid.", path));
        }
        let minimum = optional_action_effect_number(table, "minimum", &path)?;
        let maximum = optional_action_effect_number(table, "maximum", &path)?;
        if minimum
            .zip(maximum)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            return Err(format!("{}.minimum must not exceed maximum.", path));
        }
        Ok(Self {
            id: stat_id.to_string(),
            dependencies: formula_identifiers(&formula),
            formula,
            minimum,
            maximum,
        })
    }
}

impl ResolvedCondition {
    fn from_table(condition_id: &str, table: &Table) -> Result<Self, String> {
        let condition_id = condition_id.trim();
        if condition_id.is_empty() {
            return Err("Condition id must not be empty.".to_string());
        }
        let path = format!("conditions.{}", condition_id);
        let duration_seconds = table_number(table, "duration", 0.0);
        if !duration_seconds.is_finite() || duration_seconds < 0.0 {
            return Err(format!(
                "{}.duration must be a finite number at least zero.",
                path
            ));
        }
        let stacking = ResolvedConditionStacking::parse(
            table_string(table, "stacking")
                .as_deref()
                .unwrap_or("refresh"),
            &path,
        )?;
        let max_stacks = table
            .get("max_stacks")
            .and_then(Value::as_integer)
            .unwrap_or(if stacking == ResolvedConditionStacking::Stack {
                99
            } else {
                1
            });
        if max_stacks <= 0 {
            return Err(format!("{}.max_stacks must be at least one.", path));
        }
        if stacking != ResolvedConditionStacking::Stack && max_stacks != 1 {
            return Err(format!(
                "{}.max_stacks must be one unless stacking = 'stack'.",
                path
            ));
        }
        let mut modifiers = Vec::new();
        if let Some(values) = table.get("modifiers") {
            let values = values
                .as_array()
                .ok_or_else(|| format!("{}.modifiers must be an array.", path))?;
            let mut attributes = BTreeSet::new();
            for (index, value) in values.iter().enumerate() {
                let modifier = value.as_table().ok_or_else(|| {
                    format!("{}.modifiers.{} must be a modifier table.", path, index)
                })?;
                let attribute = table_string(modifier, "attribute").ok_or_else(|| {
                    format!("{}.modifiers.{}.attribute is required.", path, index)
                })?;
                if !attributes.insert(attribute.clone()) {
                    return Err(format!(
                        "{}.modifiers must not modify '{}' more than once.",
                        path, attribute
                    ));
                }
                let modifier_path = format!("{}.modifiers.{}", path, index);
                if !["add", "multiply", "minimum", "maximum"]
                    .iter()
                    .any(|key| modifier.contains_key(*key))
                {
                    return Err(format!(
                        "{} must define at least one of add, multiply, minimum, or maximum.",
                        modifier_path
                    ));
                }
                let add =
                    optional_action_effect_number(modifier, "add", &modifier_path)?.unwrap_or(0.0);
                let multiply = optional_action_effect_number(modifier, "multiply", &modifier_path)?
                    .unwrap_or(1.0);
                if multiply < 0.0 {
                    return Err(format!("{}.multiply must be at least zero.", modifier_path));
                }
                let minimum = optional_action_effect_number(modifier, "minimum", &modifier_path)?;
                let maximum = optional_action_effect_number(modifier, "maximum", &modifier_path)?;
                if minimum
                    .zip(maximum)
                    .is_some_and(|(minimum, maximum)| minimum > maximum)
                {
                    return Err(format!(
                        "{}.minimum must not exceed maximum.",
                        modifier_path
                    ));
                }
                modifiers.push(ResolvedConditionModifier {
                    attribute,
                    add,
                    multiply,
                    minimum,
                    maximum,
                });
            }
        }
        let periodic = table
            .get("periodic")
            .map(|value| {
                let periodic = value
                    .as_table()
                    .ok_or_else(|| format!("{}.periodic must be a table.", path))?;
                let interval_seconds = periodic
                    .get("interval")
                    .and_then(|value| {
                        value
                            .as_float()
                            .or_else(|| value.as_integer().map(|value| value as f64))
                    })
                    .map(|value| value as f32)
                    .ok_or_else(|| format!("{}.periodic.interval is required.", path))?;
                if !interval_seconds.is_finite() || interval_seconds <= 0.0 {
                    return Err(format!(
                        "{}.periodic.interval must be a finite number above zero.",
                        path
                    ));
                }
                let initial_delay_seconds =
                    table_number(periodic, "initial_delay", interval_seconds);
                if !initial_delay_seconds.is_finite() || initial_delay_seconds < 0.0 {
                    return Err(format!(
                        "{}.periodic.initial_delay must be a finite number at least zero.",
                        path
                    ));
                }
                let effects = periodic
                    .get("effects")
                    .and_then(Value::as_array)
                    .ok_or_else(|| format!("{}.periodic.effects must be a non-empty array.", path))?;
                if effects.is_empty() {
                    return Err(format!(
                        "{}.periodic.effects must be a non-empty array.",
                        path
                    ));
                }
                let mut resolved_effects = Vec::new();
                for (index, value) in effects.iter().enumerate() {
                    let effect = value.as_table().ok_or_else(|| {
                        format!("{}.periodic.effects.{} must be a table.", path, index)
                    })?;
                    let effect_path = format!("{}.periodic.effects.{}", path, index);
                    let has_damage = effect.contains_key("damage");
                    let has_healing = effect.contains_key("healing");
                    let has_field =
                        effect.contains_key("attribute") || effect.contains_key("resource");
                    if [has_damage, has_healing, has_field]
                        .into_iter()
                        .filter(|present| *present)
                        .count()
                        != 1
                    {
                        return Err(format!(
                            "{} must define exactly one of damage, healing, attribute, or resource.",
                            effect_path
                        ));
                    }
                    if has_damage {
                        let amount = table_number(effect, "damage", 0.0);
                        if !amount.is_finite() || amount <= 0.0 {
                            return Err(format!(
                                "{}.damage must be a finite number above zero.",
                                effect_path
                            ));
                        }
                        let damage_kind = table_string(effect, "damage_kind")
                            .unwrap_or_else(|| "physical".to_string());
                        resolved_effects.push(ResolvedConditionPeriodicEffect::Damage {
                            amount,
                            damage_kind,
                        });
                    } else if has_healing {
                        let amount = table_number(effect, "healing", 0.0);
                        if !amount.is_finite() || amount <= 0.0 {
                            return Err(format!(
                                "{}.healing must be a finite number above zero.",
                                effect_path
                            ));
                        }
                        resolved_effects
                            .push(ResolvedConditionPeriodicEffect::Healing { amount });
                    } else {
                        let field = match (
                            table_string(effect, "attribute"),
                            table_string(effect, "resource"),
                        ) {
                            (Some(attribute), None) => {
                                ResolvedActionModificationField::Attribute(attribute)
                            }
                            (None, Some(resource)) => {
                                ResolvedActionModificationField::Resource(resource)
                            }
                            _ => {
                                return Err(format!(
                                    "{} must define exactly one of attribute or resource.",
                                    effect_path
                                ));
                            }
                        };
                        let add = effect
                            .get("add")
                            .and_then(|value| {
                                value
                                    .as_float()
                                    .or_else(|| value.as_integer().map(|value| value as f64))
                            })
                            .map(|value| value as f32)
                            .ok_or_else(|| format!("{}.add is required.", effect_path))?;
                        if !add.is_finite() {
                            return Err(format!("{}.add must be finite.", effect_path));
                        }
                        let minimum =
                            optional_action_effect_number(effect, "minimum", &effect_path)?;
                        let maximum =
                            optional_action_effect_number(effect, "maximum", &effect_path)?;
                        if minimum
                            .zip(maximum)
                            .is_some_and(|(minimum, maximum)| minimum > maximum)
                        {
                            return Err(format!(
                                "{}.minimum must not exceed maximum.",
                                effect_path
                            ));
                        }
                        resolved_effects.push(ResolvedConditionPeriodicEffect::Modify {
                            field,
                            add,
                            minimum,
                            maximum,
                        });
                    }
                }
                Ok(ResolvedConditionPeriodic {
                    interval_seconds,
                    initial_delay_seconds,
                    effects: resolved_effects,
                })
            })
            .transpose()?;

        Ok(Self {
            id: condition_id.to_string(),
            name: table_string(table, "name").unwrap_or_else(|| condition_id.replace('_', " ")),
            description: table_string(table, "description"),
            duration_seconds,
            stacking,
            max_stacks: max_stacks as usize,
            tags: table_string_array(table, "tags"),
            immune_traits: table_string_array(table, "immune_traits"),
            modifiers,
            periodic,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedActionResourceCost {
    pub resource: String,
    pub amount: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedActionItemCost {
    pub item: String,
    pub quantity: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedActionItemSource {
    pub item: String,
    pub condition_cost: f32,
    pub destroy_on_empty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedActionValueSource {
    Weapon,
    RulesetPath(String),
}

impl ResolvedActionValueSource {
    fn parse(value: &str, path: &str) -> Result<Self, String> {
        let value = value.trim();
        if value.is_empty() {
            return Err(format!("{} must not be empty.", path));
        }
        if value.eq_ignore_ascii_case("weapon") {
            Ok(Self::Weapon)
        } else {
            Ok(Self::RulesetPath(value.to_string()))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolvedActionEffectRecipient {
    Actor,
    Target,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedActionModificationField {
    Attribute(String),
    Resource(String),
}

impl ResolvedActionModificationField {
    pub fn id(&self) -> &str {
        match self {
            Self::Attribute(id) | Self::Resource(id) => id,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedActionEffectValue {
    Bool(bool),
    Integer(i64),
    Float(f32),
    String(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedActionModification {
    Add(f32),
    Set(ResolvedActionEffectValue),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedActionEffect {
    Damage {
        source: ResolvedActionValueSource,
    },
    Healing {
        source: ResolvedActionValueSource,
    },
    GiveItem {
        item: String,
        quantity: usize,
    },
    Take,
    Script {
        event: String,
    },
    ApplyCondition {
        condition: String,
        recipient: ResolvedActionEffectRecipient,
    },
    RemoveCondition {
        condition: String,
        recipient: ResolvedActionEffectRecipient,
    },
    Modify {
        recipient: ResolvedActionEffectRecipient,
        field: ResolvedActionModificationField,
        operation: ResolvedActionModification,
        minimum: Option<f32>,
        maximum: Option<f32>,
        maximum_attribute: Option<String>,
    },
}

fn parse_action_condition_effect(
    value: &Value,
    path: &str,
    apply: bool,
) -> Result<ResolvedActionEffect, String> {
    let (condition, recipient) = if let Some(condition) = value
        .as_str()
        .map(str::trim)
        .filter(|condition| !condition.is_empty())
    {
        (condition.to_string(), ResolvedActionEffectRecipient::Target)
    } else {
        let table = value
            .as_table()
            .ok_or_else(|| format!("{} must be a condition id or table.", path))?;
        let condition =
            table_string(table, "id").ok_or_else(|| format!("{}.id is required.", path))?;
        let recipient = match table_string(table, "recipient")
            .unwrap_or_else(|| "target".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "actor" | "self" => ResolvedActionEffectRecipient::Actor,
            "target" | "action_target" => ResolvedActionEffectRecipient::Target,
            _ => {
                return Err(format!("{}.recipient must be 'actor' or 'target'.", path));
            }
        };
        (condition, recipient)
    };
    if apply {
        Ok(ResolvedActionEffect::ApplyCondition {
            condition,
            recipient,
        })
    } else {
        Ok(ResolvedActionEffect::RemoveCondition {
            condition,
            recipient,
        })
    }
}

fn parse_action_effect_value(
    value: &Value,
    path: &str,
) -> Result<ResolvedActionEffectValue, String> {
    if let Some(value) = value.as_bool() {
        return Ok(ResolvedActionEffectValue::Bool(value));
    }
    if let Some(value) = value.as_integer() {
        return Ok(ResolvedActionEffectValue::Integer(value));
    }
    if let Some(value) = value.as_float() {
        let value = value as f32;
        if value.is_finite() {
            return Ok(ResolvedActionEffectValue::Float(value));
        }
        return Err(format!("{} must be finite.", path));
    }
    if let Some(value) = value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(ResolvedActionEffectValue::String(value.to_string()));
    }
    Err(format!(
        "{} must be a boolean, number, or non-empty string.",
        path
    ))
}

fn optional_action_effect_number(
    table: &Table,
    key: &str,
    path: &str,
) -> Result<Option<f32>, String> {
    let Some(value) = table.get(key) else {
        return Ok(None);
    };
    let number = value
        .as_float()
        .or_else(|| value.as_integer().map(|value| value as f64))
        .map(|value| value as f32)
        .ok_or_else(|| format!("{}.{} must be a number.", path, key))?;
    if !number.is_finite() {
        return Err(format!("{}.{} must be finite.", path, key));
    }
    Ok(Some(number))
}

fn parse_action_modification(value: &Value, path: &str) -> Result<ResolvedActionEffect, String> {
    let table = value
        .as_table()
        .ok_or_else(|| format!("{} must be a modification table.", path))?;
    let attribute = table_string(table, "attribute");
    let resource = table_string(table, "resource");
    let field = match (attribute, resource) {
        (Some(attribute), None) => ResolvedActionModificationField::Attribute(attribute),
        (None, Some(resource)) => ResolvedActionModificationField::Resource(resource),
        _ => {
            return Err(format!(
                "{} must define exactly one of attribute or resource.",
                path
            ));
        }
    };
    let recipient = match table_string(table, "recipient")
        .unwrap_or_else(|| "target".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "actor" | "self" => ResolvedActionEffectRecipient::Actor,
        "target" | "action_target" => ResolvedActionEffectRecipient::Target,
        _ => {
            return Err(format!("{}.recipient must be 'actor' or 'target'.", path));
        }
    };
    let has_add = table.contains_key("add");
    let has_set = table.contains_key("set");
    if has_add == has_set {
        return Err(format!("{} must define exactly one of add or set.", path));
    }
    let operation = if has_add {
        let amount = optional_action_effect_number(table, "add", path)?
            .expect("add exists when parsing an add operation");
        ResolvedActionModification::Add(amount)
    } else {
        ResolvedActionModification::Set(parse_action_effect_value(
            &table["set"],
            &format!("{}.set", path),
        )?)
    };
    if matches!(field, ResolvedActionModificationField::Resource(_))
        && matches!(
            operation,
            ResolvedActionModification::Set(
                ResolvedActionEffectValue::Bool(_) | ResolvedActionEffectValue::String(_)
            )
        )
    {
        return Err(format!("{}.resource modifications must be numeric.", path));
    }

    let minimum = optional_action_effect_number(table, "minimum", path)?;
    let maximum = optional_action_effect_number(table, "maximum", path)?;
    let maximum_attribute = table_string(table, "maximum_attribute");
    if maximum.is_some() && maximum_attribute.is_some() {
        return Err(format!(
            "{} must not define both maximum and maximum_attribute.",
            path
        ));
    }
    if minimum.zip(maximum).is_some_and(|(min, max)| min > max) {
        return Err(format!(
            "{}.minimum must not be greater than maximum.",
            path
        ));
    }
    let numeric_operation = matches!(operation, ResolvedActionModification::Add(_))
        || matches!(
            operation,
            ResolvedActionModification::Set(
                ResolvedActionEffectValue::Integer(_) | ResolvedActionEffectValue::Float(_)
            )
        );
    if (minimum.is_some() || maximum.is_some() || maximum_attribute.is_some()) && !numeric_operation
    {
        return Err(format!(
            "{} clamps require a numeric add or set operation.",
            path
        ));
    }

    Ok(ResolvedActionEffect::Modify {
        recipient,
        field,
        operation,
        minimum,
        maximum,
        maximum_attribute,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ResolvedActionPresentation {
    pub icon: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedInvocationSchemeKind {
    TokenSequence,
}

impl ResolvedInvocationSchemeKind {
    fn parse(value: &str, path: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "token_sequence" | "sequence" => Ok(Self::TokenSequence),
            other => Err(format!(
                "{}.kind '{}' is unsupported; use token_sequence.",
                path, other
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedInvocationScheme {
    pub id: String,
    pub name: String,
    pub kind: ResolvedInvocationSchemeKind,
    pub tokens: Vec<String>,
    pub separator: String,
    pub max_tokens: usize,
    pub case_sensitive: bool,
}

impl ResolvedInvocationScheme {
    fn from_table(scheme_id: &str, table: &Table) -> Result<Self, String> {
        let scheme_id = scheme_id.trim();
        if scheme_id.is_empty() {
            return Err("Invocation scheme id must not be empty.".to_string());
        }
        let path = format!("invocation_schemes.{}", scheme_id);
        let kind = ResolvedInvocationSchemeKind::parse(
            table_string(table, "kind")
                .as_deref()
                .unwrap_or("token_sequence"),
            &path,
        )?;
        let tokens = table
            .get("tokens")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{}.tokens must be a non-empty string array.", path))?
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| format!("{}.tokens.{} must be a non-empty string.", path, index))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if tokens.is_empty() {
            return Err(format!("{}.tokens must not be empty.", path));
        }
        let case_sensitive = table
            .get("case_sensitive")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let separator = table
            .get("separator")
            .and_then(Value::as_str)
            .unwrap_or(" ")
            .to_string();
        let max_tokens = table
            .get("max_tokens")
            .and_then(Value::as_integer)
            .unwrap_or(4);
        if max_tokens <= 0 {
            return Err(format!("{}.max_tokens must be at least one.", path));
        }
        let mut normalized_tokens = BTreeSet::new();
        for token in &tokens {
            let normalized = if case_sensitive {
                token.clone()
            } else {
                token.to_lowercase()
            };
            if !normalized_tokens.insert(normalized) {
                return Err(format!(
                    "{}.tokens contains the duplicate token '{}'.",
                    path, token
                ));
            }
        }

        Ok(Self {
            id: scheme_id.to_string(),
            name: table_string(table, "name").unwrap_or_else(|| scheme_id.replace('_', " ")),
            kind,
            tokens,
            separator,
            max_tokens: max_tokens as usize,
            case_sensitive,
        })
    }

    fn normalize_token(&self, token: &str) -> String {
        let token = token.trim();
        if self.case_sensitive {
            token.to_string()
        } else {
            token.to_lowercase()
        }
    }

    pub fn phrase_for_sequence(&self, sequence: &[String]) -> String {
        sequence.join(&self.separator)
    }

    pub fn normalize_phrase(&self, phrase: &str) -> String {
        let normalized = if self.separator.chars().all(char::is_whitespace) {
            phrase.split_whitespace().collect::<Vec<_>>().join(" ")
        } else if self.separator.is_empty() {
            phrase.trim().to_string()
        } else {
            phrase
                .trim()
                .split(&self.separator)
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(&self.separator)
        };
        if self.case_sensitive {
            normalized
        } else {
            normalized.to_lowercase()
        }
    }

    fn binding_key(&self, sequence: &[String]) -> String {
        self.normalize_phrase(&self.phrase_for_sequence(sequence))
    }

    fn contains_token(&self, token: &str) -> bool {
        let token = self.normalize_token(token);
        self.tokens
            .iter()
            .any(|candidate| self.normalize_token(candidate) == token)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedActionInvocation {
    pub scheme: String,
    pub sequence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedAction {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: ResolvedActionKind,
    pub intent: Option<String>,
    pub target: ResolvedActionTarget,
    pub range: ResolvedActionRange,
    pub cooldown_seconds: f32,
    pub damage_kind: Option<String>,
    pub requirements: Vec<ResolvedActionRequirement>,
    pub resource_costs: Vec<ResolvedActionResourceCost>,
    pub item_costs: Vec<ResolvedActionItemCost>,
    pub item_source: Option<ResolvedActionItemSource>,
    pub effects: Vec<ResolvedActionEffect>,
    pub recipe: Option<String>,
    pub invocations: Vec<ResolvedActionInvocation>,
    pub presentation: ResolvedActionPresentation,
}

impl ResolvedAction {
    pub fn from_table(action_id: &str, table: &Table) -> Result<Self, String> {
        let action_id = action_id.trim();
        if action_id.is_empty() {
            return Err("Action id must not be empty.".to_string());
        }
        let path = format!("actions.{}", action_id);
        let name = table_string(table, "name").unwrap_or_else(|| action_id.replace('_', " "));
        let kind = ResolvedActionKind::parse(
            table_string(table, "kind")
                .as_deref()
                .unwrap_or("interaction"),
        );
        let target = ResolvedActionTarget::parse(
            table_string(table, "target")
                .as_deref()
                .unwrap_or("any_entity"),
        );
        let range = ResolvedActionRange::parse(table.get("range"), &path)?;
        let cooldown_seconds = table_number(table, "cooldown", 0.0);
        if !cooldown_seconds.is_finite() || cooldown_seconds < 0.0 {
            return Err(format!(
                "{}.cooldown must be a finite number at least zero.",
                path
            ));
        }

        let mut requirements = Vec::new();
        if let Some(requires_value) = table.get("requires") {
            let requires = requires_value
                .as_table()
                .ok_or_else(|| format!("{}.requires must be a table.", path))?;
            if let Some(ability) = table_string(requires, "ability") {
                requirements.push(ResolvedActionRequirement::Ability(ability));
            }
            if let Some(spell) = table_string(requires, "spell") {
                requirements.push(ResolvedActionRequirement::Spell(spell));
            }
            if let Some(profession) = table_string(requires, "profession") {
                requirements.push(ResolvedActionRequirement::Profession(profession));
            }
            if let Some(attributes) = requires.get("attributes") {
                let attributes = attributes
                    .as_array()
                    .ok_or_else(|| format!("{}.requires.attributes must be an array.", path))?;
                for (index, value) in attributes.iter().enumerate() {
                    requirements.push(parse_action_attribute_requirement(
                        value,
                        &format!("{}.requires.attributes.{}", path, index),
                        false,
                    )?);
                }
            }
            if let Some(attributes) = requires.get("target_attributes") {
                if !target.is_entity_target() {
                    return Err(format!(
                        "{}.requires.target_attributes requires an entity action target.",
                        path
                    ));
                }
                let attributes = attributes.as_array().ok_or_else(|| {
                    format!("{}.requires.target_attributes must be an array.", path)
                })?;
                for (index, value) in attributes.iter().enumerate() {
                    requirements.push(parse_action_attribute_requirement(
                        value,
                        &format!("{}.requires.target_attributes.{}", path, index),
                        true,
                    )?);
                }
            }
        }
        if let Some(skill) = table_string(table, "skill") {
            let minimum = table_number(table, "required_skill", 0.0).round();
            if !minimum.is_finite() || minimum < 0.0 {
                return Err(format!(
                    "{}.required_skill must be a finite number at least zero.",
                    path
                ));
            }
            requirements.push(ResolvedActionRequirement::Skill {
                id: skill,
                minimum: minimum as i32,
            });
        }

        let mut resource_costs = Vec::new();
        if let Some(costs) = table.get("cost").and_then(Value::as_table) {
            for (resource, value) in costs {
                let amount = value
                    .as_float()
                    .or_else(|| value.as_integer().map(|value| value as f64))
                    .map(|value| value as f32)
                    .ok_or_else(|| format!("{}.cost.{} must be a number.", path, resource))?;
                if !amount.is_finite() || amount < 0.0 {
                    return Err(format!(
                        "{}.cost.{} must be a finite number at least zero.",
                        path, resource
                    ));
                }
                resource_costs.push(ResolvedActionResourceCost {
                    resource: resource.clone(),
                    amount,
                });
            }
            resource_costs.sort_by(|a, b| a.resource.cmp(&b.resource));
        }

        let mut item_costs = Vec::new();
        if let Some(consumes) = table.get("consumes").and_then(Value::as_array) {
            for (index, value) in consumes.iter().enumerate() {
                let entry = value.as_table().ok_or_else(|| {
                    format!(
                        "{}.consumes.{} must be an item quantity table.",
                        path, index
                    )
                })?;
                let item = table_string(entry, "item")
                    .ok_or_else(|| format!("{}.consumes.{}.item is required.", path, index))?;
                let quantity = entry
                    .get("quantity")
                    .and_then(Value::as_integer)
                    .unwrap_or(1);
                if quantity <= 0 {
                    return Err(format!(
                        "{}.consumes.{}.quantity must be greater than zero.",
                        path, index
                    ));
                }
                item_costs.push(ResolvedActionItemCost {
                    item,
                    quantity: quantity as usize,
                });
            }
        }

        let item_source = table
            .get("source")
            .map(|value| {
                let source = value
                    .as_table()
                    .ok_or_else(|| format!("{}.source must be an item source table.", path))?;
                let item = table_string(source, "item")
                    .ok_or_else(|| format!("{}.source.item is required.", path))?;
                let condition_cost = table_number(source, "condition_cost", 0.0);
                if !condition_cost.is_finite() || condition_cost < 0.0 {
                    return Err(format!(
                        "{}.source.condition_cost must be a finite number at least zero.",
                        path
                    ));
                }
                let destroy_on_empty = source
                    .get("destroy_on_empty")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Ok(ResolvedActionItemSource {
                    item,
                    condition_cost,
                    destroy_on_empty,
                })
            })
            .transpose()?;
        if let Some(source) = item_source.as_ref()
            && item_costs
                .iter()
                .any(|cost| cost.item.eq_ignore_ascii_case(&source.item))
        {
            return Err(format!(
                "{}.source.item must not also appear in consumes; source wear and consumed costs are separate.",
                path
            ));
        }

        let mut effects = Vec::new();
        if let Some(result) = table.get("result").and_then(Value::as_table) {
            if let Some(damage) = table_string(result, "damage") {
                effects.push(ResolvedActionEffect::Damage {
                    source: ResolvedActionValueSource::parse(
                        &damage,
                        &format!("{}.result.damage", path),
                    )?,
                });
            }
            if let Some(healing) = table_string(result, "healing") {
                effects.push(ResolvedActionEffect::Healing {
                    source: ResolvedActionValueSource::parse(
                        &healing,
                        &format!("{}.result.healing", path),
                    )?,
                });
            }
            if let Some(item) = table_string(result, "item") {
                let quantity = result
                    .get("quantity")
                    .and_then(Value::as_integer)
                    .unwrap_or(1);
                if quantity <= 0 {
                    return Err(format!(
                        "{}.result.quantity must be greater than zero.",
                        path
                    ));
                }
                effects.push(ResolvedActionEffect::GiveItem {
                    item,
                    quantity: quantity as usize,
                });
            }
            if result.get("take").and_then(Value::as_bool) == Some(true) {
                effects.push(ResolvedActionEffect::Take);
            }
            if let Some(script) = result.get("script") {
                let event = script
                    .as_str()
                    .map(str::trim)
                    .filter(|event| !event.is_empty())
                    .ok_or_else(|| {
                        format!("{}.result.script must be a non-empty event name.", path)
                    })?;
                effects.push(ResolvedActionEffect::Script {
                    event: event.to_string(),
                });
            }
            if let Some(condition) = result.get("apply_condition") {
                effects.push(parse_action_condition_effect(
                    condition,
                    &format!("{}.result.apply_condition", path),
                    true,
                )?);
            }
            if let Some(condition) = result.get("remove_condition") {
                effects.push(parse_action_condition_effect(
                    condition,
                    &format!("{}.result.remove_condition", path),
                    false,
                )?);
            }
            if let Some(modifications) = result.get("modify") {
                let modifications = modifications
                    .as_array()
                    .ok_or_else(|| format!("{}.result.modify must be an array.", path))?;
                for (index, value) in modifications.iter().enumerate() {
                    effects.push(parse_action_modification(
                        value,
                        &format!("{}.result.modify.{}", path, index),
                    )?);
                }
            }
        }
        let modifications = effects
            .iter()
            .filter_map(|effect| {
                if let ResolvedActionEffect::Modify {
                    recipient, field, ..
                } = effect
                {
                    Some((recipient, field))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut modified_fields = BTreeSet::new();
        for (recipient, field) in &modifications {
            let recipient = match recipient {
                ResolvedActionEffectRecipient::Actor => "actor",
                ResolvedActionEffectRecipient::Target => "target",
            };
            if !modified_fields.insert(format!("{}:{}", recipient, field.id())) {
                return Err(format!(
                    "{}.result.modify must not modify {}.{} more than once.",
                    path,
                    recipient,
                    field.id()
                ));
            }
            if recipient == "target" && !target.is_entity_target() {
                return Err(format!(
                    "{}.result.modify target recipients require an entity action target.",
                    path
                ));
            }
        }
        let mut condition_effects = BTreeSet::new();
        for effect in &effects {
            let (condition, recipient) = match effect {
                ResolvedActionEffect::ApplyCondition {
                    condition,
                    recipient,
                }
                | ResolvedActionEffect::RemoveCondition {
                    condition,
                    recipient,
                } => (condition, recipient),
                _ => continue,
            };
            if *recipient == ResolvedActionEffectRecipient::Target && !target.is_entity_target() {
                return Err(format!(
                    "{}.result condition target recipients require an entity action target.",
                    path
                ));
            }
            if !condition_effects.insert((condition.clone(), recipient.clone())) {
                return Err(format!(
                    "{}.result must not apply or remove condition '{}' more than once for the same recipient.",
                    path, condition
                ));
            }
        }
        if !modifications.is_empty()
            && effects.iter().any(|effect| {
                matches!(
                    effect,
                    ResolvedActionEffect::Damage { .. }
                        | ResolvedActionEffect::Healing { .. }
                        | ResolvedActionEffect::GiveItem { .. }
                        | ResolvedActionEffect::Take
                )
            })
        {
            return Err(format!(
                "{}.result.modify currently combines only with result.script.",
                path
            ));
        }
        if !modifications.is_empty() && table_string(table, "recipe").is_some() {
            return Err(format!(
                "{}.result.modify does not combine with a recipe.",
                path
            ));
        }
        if item_source.is_some()
            && !effects
                .iter()
                .any(|effect| matches!(effect, ResolvedActionEffect::Script { .. }))
        {
            return Err(format!(
                "{}.source currently requires a result.script effect.",
                path
            ));
        }
        if item_source.is_some() && target == ResolvedActionTarget::WorldPosition {
            return Err(format!(
                "{}.source does not yet support world_position targets.",
                path
            ));
        }

        let presentation = table
            .get("ui")
            .and_then(Value::as_table)
            .map(|ui| ResolvedActionPresentation {
                icon: table_string(ui, "icon"),
            })
            .unwrap_or_default();
        let invocations = table
            .get("invocations")
            .map(|value| {
                let values = value
                    .as_array()
                    .ok_or_else(|| format!("{}.invocations must be an array.", path))?;
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let invocation = value.as_table().ok_or_else(|| {
                            format!(
                                "{}.invocations.{} must be an invocation table.",
                                path, index
                            )
                        })?;
                        let scheme = table_string(invocation, "scheme").ok_or_else(|| {
                            format!("{}.invocations.{}.scheme is required.", path, index)
                        })?;
                        let sequence = invocation
                            .get("sequence")
                            .and_then(Value::as_array)
                            .ok_or_else(|| {
                                format!(
                                    "{}.invocations.{}.sequence must be a non-empty string array.",
                                    path, index
                                )
                            })?
                            .iter()
                            .enumerate()
                            .map(|(token_index, token)| {
                                token
                                    .as_str()
                                    .map(str::trim)
                                    .filter(|token| !token.is_empty())
                                    .map(str::to_string)
                                    .ok_or_else(|| {
                                        format!(
                                            "{}.invocations.{}.sequence.{} must be a non-empty string.",
                                            path, index, token_index
                                        )
                                    })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        if sequence.is_empty() {
                            return Err(format!(
                                "{}.invocations.{}.sequence must not be empty.",
                                path, index
                            ));
                        }
                        Ok(ResolvedActionInvocation { scheme, sequence })
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            id: action_id.to_string(),
            name,
            description: table_string(table, "description"),
            kind,
            intent: table_string(table, "intent"),
            target,
            range,
            cooldown_seconds,
            damage_kind: table_string(table, "damage_kind"),
            requirements,
            resource_costs,
            item_costs,
            item_source,
            effects,
            recipe: table_string(table, "recipe"),
            invocations,
            presentation,
        })
    }

    pub fn required_ability(&self) -> Option<&str> {
        self.requirements.iter().find_map(|requirement| {
            if let ResolvedActionRequirement::Ability(ability) = requirement {
                Some(ability.as_str())
            } else {
                None
            }
        })
    }

    pub fn required_spell(&self) -> Option<&str> {
        self.requirements.iter().find_map(|requirement| {
            if let ResolvedActionRequirement::Spell(spell) = requirement {
                Some(spell.as_str())
            } else {
                None
            }
        })
    }

    pub fn required_profession(&self) -> Option<&str> {
        self.requirements.iter().find_map(|requirement| {
            if let ResolvedActionRequirement::Profession(profession) = requirement {
                Some(profession.as_str())
            } else {
                None
            }
        })
    }

    pub fn skill_requirement(&self) -> Option<(&str, i32)> {
        self.requirements.iter().find_map(|requirement| {
            if let ResolvedActionRequirement::Skill { id, minimum } = requirement {
                Some((id.as_str(), *minimum))
            } else {
                None
            }
        })
    }

    pub fn damage_source(&self) -> Option<&ResolvedActionValueSource> {
        self.effects.iter().find_map(|effect| {
            if let ResolvedActionEffect::Damage { source } = effect {
                Some(source)
            } else {
                None
            }
        })
    }

    pub fn healing_source(&self) -> Option<&ResolvedActionValueSource> {
        self.effects.iter().find_map(|effect| {
            if let ResolvedActionEffect::Healing { source } = effect {
                Some(source)
            } else {
                None
            }
        })
    }

    pub fn resource_cost(&self, resource: &str) -> Option<f32> {
        self.resource_costs
            .iter()
            .find(|cost| cost.resource.eq_ignore_ascii_case(resource.trim()))
            .map(|cost| cost.amount)
    }

    pub fn takes_target_item(&self) -> bool {
        self.effects
            .iter()
            .any(|effect| matches!(effect, ResolvedActionEffect::Take))
    }

    pub fn script_event(&self) -> Option<&str> {
        self.effects.iter().find_map(|effect| {
            if let ResolvedActionEffect::Script { event } = effect {
                Some(event.as_str())
            } else {
                None
            }
        })
    }

    pub fn has_state_modifications(&self) -> bool {
        self.effects
            .iter()
            .any(|effect| matches!(effect, ResolvedActionEffect::Modify { .. }))
    }

    pub fn has_condition_effects(&self) -> bool {
        self.effects.iter().any(|effect| {
            matches!(
                effect,
                ResolvedActionEffect::ApplyCondition { .. }
                    | ResolvedActionEffect::RemoveCondition { .. }
            )
        })
    }
}

pub fn resolve_action_icon(root: &Table, action: &ResolvedAction) -> Option<String> {
    if let Some(icon) = action.presentation.icon.as_ref() {
        return Some(icon.clone());
    }

    let icons = ruleset_table_at_path(root, &["icons"]);
    for definition_id in [
        action.required_ability(),
        action.required_spell(),
        Some(action.id.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        if icons.is_some_and(|icons| icons.contains_key(definition_id)) {
            return Some(definition_id.to_string());
        }
    }

    let fallbacks = ruleset_table_at_path(root, &["ui", "action_icon_fallbacks"])?;
    let mut roles = Vec::new();
    if action.healing_source().is_some() {
        roles.push("healing");
    }
    if action.has_condition_effects() {
        roles.push("condition");
    }
    roles.push(action.kind.id());
    roles.push("default");
    roles
        .into_iter()
        .find_map(|role| table_string(fallbacks, role))
}

pub fn resolve_item_icon(
    root: &Table,
    kind: Option<&str>,
    explicit: Option<&str>,
) -> Option<String> {
    if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return Some(explicit.to_string());
    }
    let fallbacks = ruleset_table_at_path(root, &["ui", "item_icon_fallbacks"])?;
    kind.map(str::trim)
        .filter(|kind| !kind.is_empty())
        .and_then(|kind| table_string(fallbacks, kind))
        .or_else(|| table_string(fallbacks, "default"))
}

/// Resolve a ruleset-authored semantic FX fallback for an action stage.
///
/// Explicit `[actions.<id>.fx.<stage>]` entries remain authoritative at
/// runtime. This only supplies a preset when that explicit stage is absent.
pub fn resolve_action_fx_fallback(
    root: &Table,
    action: &ResolvedAction,
    stage: &str,
) -> Option<String> {
    let fallbacks = ruleset_table_at_path(root, &["fx", "action_fallbacks"])?;
    let mut roles = Vec::new();
    if action.healing_source().is_some() {
        roles.push("healing");
    }
    if action.has_condition_effects() {
        roles.push("condition");
    }
    roles.push(action.kind.id());
    roles.push("default");
    roles.into_iter().find_map(|role| {
        fallbacks
            .get(role)
            .and_then(Value::as_table)
            .and_then(|fallback| table_string(fallback, stage))
    })
}

/// Resolve a ruleset-authored semantic FX fallback for a condition stage.
pub fn resolve_condition_fx_fallback(root: &Table, stage: &str) -> Option<String> {
    ruleset_table_at_path(root, &["fx", "condition_fallbacks"])
        .and_then(|fallbacks| table_string(fallbacks, stage))
}

pub fn resolve_action(root: &Table, action_id: &str) -> Result<Option<ResolvedAction>, String> {
    let Some(actions) = ruleset_table_at_path(root, &["actions"]) else {
        return Ok(None);
    };
    let Some(value) = actions.get(action_id.trim()) else {
        return Ok(None);
    };
    let table = value
        .as_table()
        .ok_or_else(|| format!("actions.{} must be a table.", action_id.trim()))?;
    ResolvedAction::from_table(action_id, table).map(Some)
}

pub fn resolve_condition(
    root: &Table,
    condition_id: &str,
) -> Result<Option<ResolvedCondition>, String> {
    let Some(conditions) = ruleset_table_at_path(root, &["conditions"]) else {
        return Ok(None);
    };
    let Some(value) = conditions.get(condition_id.trim()) else {
        return Ok(None);
    };
    let table = value
        .as_table()
        .ok_or_else(|| format!("conditions.{} must be a table.", condition_id.trim()))?;
    ResolvedCondition::from_table(condition_id, table).map(Some)
}

pub fn resolve_conditions(root: &Table) -> Result<BTreeMap<String, ResolvedCondition>, String> {
    let Some(conditions) = ruleset_table_at_path(root, &["conditions"]) else {
        return Ok(BTreeMap::new());
    };
    conditions
        .iter()
        .map(|(condition_id, value)| {
            let table = value
                .as_table()
                .ok_or_else(|| format!("conditions.{} must be a table.", condition_id))?;
            ResolvedCondition::from_table(condition_id, table)
                .map(|condition| (condition_id.clone(), condition))
        })
        .collect()
}

pub fn resolve_derived_stat(
    root: &Table,
    stat_id: &str,
) -> Result<Option<ResolvedDerivedStat>, String> {
    let Some(stats) = ruleset_table_at_path(root, &["derived_stats"]) else {
        return Ok(None);
    };
    let Some(value) = stats.get(stat_id.trim()) else {
        return Ok(None);
    };
    let table = value
        .as_table()
        .ok_or_else(|| format!("derived_stats.{} must be a table.", stat_id.trim()))?;
    ResolvedDerivedStat::from_table(stat_id.trim(), table).map(Some)
}

pub fn resolve_derived_stats(
    root: &Table,
) -> Result<BTreeMap<String, ResolvedDerivedStat>, String> {
    let Some(stats) = ruleset_table_at_path(root, &["derived_stats"]) else {
        return Ok(BTreeMap::new());
    };
    stats
        .iter()
        .map(|(stat_id, value)| {
            let table = value
                .as_table()
                .ok_or_else(|| format!("derived_stats.{} must be a table.", stat_id))?;
            ResolvedDerivedStat::from_table(stat_id, table).map(|stat| (stat_id.clone(), stat))
        })
        .collect()
}

fn parse_actions(root: &Table) -> Result<BTreeMap<String, ResolvedAction>, String> {
    let Some(actions) = ruleset_table_at_path(root, &["actions"]) else {
        return Ok(BTreeMap::new());
    };
    actions
        .iter()
        .map(|(action_id, value)| {
            let table = value
                .as_table()
                .ok_or_else(|| format!("actions.{} must be a table.", action_id))?;
            ResolvedAction::from_table(action_id, table).map(|action| (action_id.clone(), action))
        })
        .collect()
}

pub fn resolve_invocation_schemes(
    root: &Table,
) -> Result<BTreeMap<String, ResolvedInvocationScheme>, String> {
    let Some(schemes) = ruleset_table_at_path(root, &["invocation_schemes"]) else {
        return Ok(BTreeMap::new());
    };
    schemes
        .iter()
        .map(|(scheme_id, value)| {
            let table = value
                .as_table()
                .ok_or_else(|| format!("invocation_schemes.{} must be a table.", scheme_id))?;
            ResolvedInvocationScheme::from_table(scheme_id, table)
                .map(|scheme| (scheme_id.clone(), scheme))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResolvedActionCatalogue {
    pub actions: BTreeMap<String, ResolvedAction>,
    pub invocation_schemes: BTreeMap<String, ResolvedInvocationScheme>,
    intent_actions: BTreeMap<String, String>,
    invocation_actions: BTreeMap<(String, String), String>,
}

impl ResolvedActionCatalogue {
    pub fn action(&self, action_id: &str) -> Option<&ResolvedAction> {
        self.actions.get(action_id.trim())
    }

    pub fn action_for_intent(&self, intent: &str) -> Option<&ResolvedAction> {
        self.intent_actions
            .get(&intent.trim().to_ascii_lowercase())
            .and_then(|action_id| self.actions.get(action_id))
    }

    pub fn action_for_invocation(&self, scheme_id: &str, phrase: &str) -> Option<&ResolvedAction> {
        let scheme_id = scheme_id.trim();
        let scheme = self.invocation_schemes.get(scheme_id)?;
        let key = (scheme_id.to_string(), scheme.normalize_phrase(phrase));
        self.invocation_actions
            .get(&key)
            .and_then(|action_id| self.actions.get(action_id))
    }
}

pub fn resolve_action_catalogue(root: &Table) -> Result<ResolvedActionCatalogue, String> {
    let actions = parse_actions(root)?;
    let invocation_schemes = resolve_invocation_schemes(root)?;
    let mut intent_actions = BTreeMap::new();
    let mut invocation_actions = BTreeMap::new();

    for (action_id, action) in &actions {
        if let Some(intent) = action.intent.as_deref() {
            let intent = intent.trim().to_ascii_lowercase();
            if let Some(existing_action) = intent_actions.insert(intent.clone(), action_id.clone())
            {
                return Err(format!(
                    "actions.{}.intent '{}' is already bound to action '{}'.",
                    action_id, intent, existing_action
                ));
            }
        }
        for (index, invocation) in action.invocations.iter().enumerate() {
            let path = format!("actions.{}.invocations.{}", action_id, index);
            let Some(scheme) = invocation_schemes.get(&invocation.scheme) else {
                return Err(format!(
                    "{}.scheme references unknown invocation scheme '{}'.",
                    path, invocation.scheme
                ));
            };
            if invocation.sequence.len() > scheme.max_tokens {
                return Err(format!(
                    "{}.sequence has {} tokens but invocation_schemes.{}.max_tokens is {}.",
                    path,
                    invocation.sequence.len(),
                    invocation.scheme,
                    scheme.max_tokens
                ));
            }
            for (token_index, token) in invocation.sequence.iter().enumerate() {
                if !scheme.contains_token(token) {
                    return Err(format!(
                        "{}.sequence.{} references unknown token '{}' in invocation scheme '{}'.",
                        path, token_index, token, invocation.scheme
                    ));
                }
            }
            let key = (
                invocation.scheme.clone(),
                scheme.binding_key(&invocation.sequence),
            );
            if let Some(existing_action) = invocation_actions.insert(key, action_id.clone()) {
                return Err(format!(
                    "{} duplicates an invocation already bound to action '{}'.",
                    path, existing_action
                ));
            }
        }
    }

    Ok(ResolvedActionCatalogue {
        actions,
        invocation_schemes,
        intent_actions,
        invocation_actions,
    })
}

pub fn resolve_actions(root: &Table) -> Result<BTreeMap<String, ResolvedAction>, String> {
    Ok(resolve_action_catalogue(root)?.actions)
}

pub fn resolve_action_invocation(
    root: &Table,
    scheme_id: &str,
    phrase: &str,
) -> Result<Option<ResolvedAction>, String> {
    Ok(resolve_action_catalogue(root)?
        .action_for_invocation(scheme_id, phrase)
        .cloned())
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
    RawRuleset::parse(src, "Ruleset").map(RawRuleset::into_table)
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedIdentityDefaults {
    pub race: Option<String>,
    pub class: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedAttributeRoles {
    pub attributes: BTreeMap<String, String>,
}

impl ResolvedAttributeRoles {
    pub fn get(&self, role: &str) -> Option<&str> {
        self.attributes
            .get(&role.trim().to_ascii_lowercase())
            .map(String::as_str)
    }
}

pub fn resolve_attribute_roles(root: &Table) -> Result<ResolvedAttributeRoles, String> {
    let Some(attributes) = root.get("attributes") else {
        return Ok(ResolvedAttributeRoles::default());
    };
    let attributes = attributes
        .as_table()
        .ok_or_else(|| "attributes must be a table.".to_string())?;
    let Some(roles) = attributes.get("roles") else {
        return Ok(ResolvedAttributeRoles::default());
    };
    let roles = roles
        .as_table()
        .ok_or_else(|| "attributes.roles must be a table.".to_string())?;
    let mut resolved = BTreeMap::new();
    for (role, value) in roles {
        let normalized_role = role.trim().to_ascii_lowercase();
        if normalized_role.is_empty() {
            return Err("attributes.roles contains an empty role id.".to_string());
        }
        let attribute = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "attributes.roles.{} must be a non-empty attribute id.",
                    role
                )
            })?;
        if resolved
            .insert(normalized_role.clone(), attribute.to_string())
            .is_some()
        {
            return Err(format!(
                "attributes.roles contains duplicate role '{}'.",
                normalized_role
            ));
        }
    }
    Ok(ResolvedAttributeRoles {
        attributes: resolved,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedClassResourceGain {
    pub attribute: String,
    pub maximum_attribute: String,
    pub per_level: i32,
}

pub fn resolve_class_resource_gains(
    root: &Table,
    class_id: &str,
) -> Result<Vec<ResolvedClassResourceGain>, String> {
    let Some(class) = ruleset_table_at_path(root, &["classes", class_id]) else {
        return Ok(Vec::new());
    };
    let Some(level) = class
        .get("progression")
        .and_then(Value::as_table)
        .and_then(|progression| progression.get("level"))
        .and_then(Value::as_table)
    else {
        return Ok(Vec::new());
    };
    for legacy in ["hp_per_level", "mp_per_level"] {
        if level.contains_key(legacy) {
            return Err(format!(
                "classes.{}.progression.level.{} is not supported; use resource_gains.",
                class_id, legacy
            ));
        }
    }
    let Some(gains) = level.get("resource_gains") else {
        return Ok(Vec::new());
    };
    let gains = gains.as_array().ok_or_else(|| {
        format!(
            "classes.{}.progression.level.resource_gains must be an array.",
            class_id
        )
    })?;
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, value) in gains.iter().enumerate() {
        let path = format!(
            "classes.{}.progression.level.resource_gains.{}",
            class_id, index
        );
        let gain = value
            .as_table()
            .ok_or_else(|| format!("{} must be a table.", path))?;
        let attribute = table_string(gain, "attribute")
            .ok_or_else(|| format!("{}.attribute must be a non-empty string.", path))?;
        let maximum_attribute = table_string(gain, "maximum_attribute")
            .ok_or_else(|| format!("{}.maximum_attribute must be a non-empty string.", path))?;
        let per_level = gain
            .get("per_level")
            .and_then(Value::as_integer)
            .ok_or_else(|| format!("{}.per_level must be a positive integer.", path))?;
        if per_level <= 0 || per_level > i32::MAX as i64 {
            return Err(format!("{}.per_level must be a positive integer.", path));
        }
        if !seen.insert(attribute.to_ascii_lowercase()) {
            return Err(format!(
                "{} duplicates resource attribute '{}'.",
                path, attribute
            ));
        }
        resolved.push(ResolvedClassResourceGain {
            attribute,
            maximum_attribute,
            per_level: per_level as i32,
        });
    }
    Ok(resolved)
}

fn optional_identity_id(defaults: &Table, key: &str) -> Result<Option<String>, String> {
    let Some(value) = defaults.get(key) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| format!("identity.defaults.{} must be a string.", key))?
        .trim();
    if value.is_empty() {
        return Err(format!(
            "identity.defaults.{} must not be an empty string.",
            key
        ));
    }
    Ok(Some(value.to_string()))
}

pub fn resolve_identity_defaults(root: &Table) -> Result<ResolvedIdentityDefaults, String> {
    let Some(identity) = root.get("identity") else {
        return Ok(ResolvedIdentityDefaults::default());
    };
    let identity = identity
        .as_table()
        .ok_or_else(|| "identity must be a table.".to_string())?;
    let Some(defaults) = identity.get("defaults") else {
        return Ok(ResolvedIdentityDefaults::default());
    };
    let defaults = defaults
        .as_table()
        .ok_or_else(|| "identity.defaults must be a table.".to_string())?;
    Ok(ResolvedIdentityDefaults {
        race: optional_identity_id(defaults, "race")?,
        class: optional_identity_id(defaults, "class")?,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedWeaponHandedness {
    OneHanded,
    TwoHanded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedEquipmentAvatarAnchor {
    MainHand,
    OffHand,
}

impl ResolvedEquipmentAvatarAnchor {
    fn parse(value: &str, path: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "main_hand" => Ok(Self::MainHand),
            "off_hand" => Ok(Self::OffHand),
            other => Err(format!(
                "{} has invalid avatar anchor '{}'; use main_hand or off_hand.",
                path, other
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedEquipmentCategory {
    pub id: String,
    pub handedness: Option<ResolvedWeaponHandedness>,
    pub occupies_slots: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedClassEquipmentPermissions {
    pub weapon_categories: Option<BTreeSet<String>>,
    pub armor_categories: Option<BTreeSet<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedEquipmentPolicy {
    pub weapon_slots: Vec<String>,
    pub armor_slots: Vec<String>,
    pub weapon_categories: BTreeMap<String, ResolvedEquipmentCategory>,
    pub armor_categories: BTreeMap<String, ResolvedEquipmentCategory>,
    pub class_permissions: BTreeMap<String, ResolvedClassEquipmentPermissions>,
    pub avatar_anchors: BTreeMap<String, ResolvedEquipmentAvatarAnchor>,
}

fn normalized_string_list(
    value: Option<&Value>,
    path: &str,
) -> Result<Option<Vec<String>>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .ok_or_else(|| format!("{} must be an array of strings.", path))?;
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let value = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("{}.{} must be a non-empty string.", path, index))?
            .to_ascii_lowercase();
        if !seen.insert(value.clone()) {
            return Err(format!("{} contains duplicate value '{}'.", path, value));
        }
        result.push(value);
    }
    Ok(Some(result))
}

fn normalized_string_set(
    value: Option<&Value>,
    path: &str,
) -> Result<Option<BTreeSet<String>>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .ok_or_else(|| format!("{} must be an array of strings.", path))?;
    let mut result = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let value = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("{}.{} must be a non-empty string.", path, index))?;
        result.insert(value.to_ascii_lowercase());
    }
    Ok(Some(result))
}

fn resolve_equipment_categories(
    root: &Table,
    section: &str,
    weapon: bool,
) -> Result<BTreeMap<String, ResolvedEquipmentCategory>, String> {
    let Some(categories) = ruleset_table_at_path(root, &["equipment", section]) else {
        return Ok(BTreeMap::new());
    };
    let mut result = BTreeMap::new();
    for (id, value) in categories {
        let path = format!("equipment.{}.{}", section, id);
        let table = value
            .as_table()
            .ok_or_else(|| format!("{} must be a table.", path))?;
        let handedness = if weapon {
            match table_string(table, "handed")
                .unwrap_or_else(|| "one_handed".to_string())
                .to_ascii_lowercase()
                .as_str()
            {
                "one_handed" => Some(ResolvedWeaponHandedness::OneHanded),
                "two_handed" => Some(ResolvedWeaponHandedness::TwoHanded),
                other => {
                    return Err(format!(
                        "{}.handed '{}' is invalid; use one_handed or two_handed.",
                        path, other
                    ));
                }
            }
        } else {
            None
        };
        let occupies_slots = normalized_string_set(
            table.get("occupies_slots"),
            &format!("{}.occupies_slots", path),
        )?
        .unwrap_or_default();
        let normalized_id = id.trim().to_ascii_lowercase();
        result.insert(
            normalized_id.clone(),
            ResolvedEquipmentCategory {
                id: normalized_id,
                handedness,
                occupies_slots,
            },
        );
    }
    Ok(result)
}

pub fn resolve_equipment_policy(root: &Table) -> Result<ResolvedEquipmentPolicy, String> {
    let equipment = ruleset_table_at_path(root, &["equipment"]);
    let weapon_slots = normalized_string_list(
        equipment.and_then(|table| table.get("weapon_slots")),
        "equipment.weapon_slots",
    )?
    .unwrap_or_default();
    let armor_slots = normalized_string_list(
        equipment.and_then(|table| table.get("armor_slots")),
        "equipment.armor_slots",
    )?
    .unwrap_or_default();
    let weapon_categories = resolve_equipment_categories(root, "weapon_categories", true)?;
    let armor_categories = resolve_equipment_categories(root, "armor_categories", false)?;

    let known_slots = weapon_slots
        .iter()
        .chain(armor_slots.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    for category in weapon_categories.values().chain(armor_categories.values()) {
        for slot in &category.occupies_slots {
            if !known_slots.contains(slot) {
                return Err(format!(
                    "Equipment category '{}'.occupies_slots references unknown slot '{}'.",
                    category.id, slot
                ));
            }
        }
    }

    let mut avatar_anchors = BTreeMap::new();
    if let Some(anchors) = equipment.and_then(|table| table.get("avatar_anchors")) {
        let anchors = anchors
            .as_table()
            .ok_or_else(|| "equipment.avatar_anchors must be a table.".to_string())?;
        for (slot, value) in anchors {
            let normalized_slot = slot.trim().to_ascii_lowercase();
            if !known_slots.contains(&normalized_slot) {
                return Err(format!(
                    "equipment.avatar_anchors.{} references unknown slot '{}'.",
                    slot, slot
                ));
            }
            let value = value.as_str().ok_or_else(|| {
                format!(
                    "equipment.avatar_anchors.{} must be main_hand or off_hand.",
                    slot
                )
            })?;
            let anchor = ResolvedEquipmentAvatarAnchor::parse(
                value,
                &format!("equipment.avatar_anchors.{}", slot),
            )?;
            if avatar_anchors
                .insert(normalized_slot.clone(), anchor)
                .is_some()
            {
                return Err(format!(
                    "equipment.avatar_anchors contains duplicate slot '{}'.",
                    normalized_slot
                ));
            }
        }
    }

    let mut class_permissions = BTreeMap::new();
    if let Some(classes) = ruleset_table_at_path(root, &["classes"]) {
        for (class_id, value) in classes {
            let Some(class) = value.as_table() else {
                continue;
            };
            class_permissions.insert(
                class_id.trim().to_ascii_lowercase(),
                ResolvedClassEquipmentPermissions {
                    weapon_categories: normalized_string_set(
                        class.get("allowed_weapons"),
                        &format!("classes.{}.allowed_weapons", class_id),
                    )?,
                    armor_categories: normalized_string_set(
                        class.get("allowed_armor"),
                        &format!("classes.{}.allowed_armor", class_id),
                    )?,
                },
            );
        }
    }

    Ok(ResolvedEquipmentPolicy {
        weapon_slots,
        armor_slots,
        weapon_categories,
        armor_categories,
        class_permissions,
        avatar_anchors,
    })
}

impl ResolvedEquipmentPolicy {
    pub fn slots_for_avatar_anchor(&self, anchor: ResolvedEquipmentAvatarAnchor) -> Vec<&str> {
        self.weapon_slots
            .iter()
            .chain(self.armor_slots.iter())
            .filter(|slot| self.avatar_anchors.get(*slot) == Some(&anchor))
            .map(String::as_str)
            .collect()
    }

    pub fn occupied_slots(
        &self,
        kind: &str,
        category: Option<&str>,
        slot: &str,
    ) -> BTreeSet<String> {
        let kind = kind.trim().to_ascii_lowercase();
        let category = category.map(|value| value.trim().to_ascii_lowercase());
        let mut occupied = BTreeSet::from([slot.trim().to_ascii_lowercase()]);
        let definition = match kind.as_str() {
            "weapon" | "weapons" => category
                .as_ref()
                .and_then(|id| self.weapon_categories.get(id)),
            "armor" | "armour" | "clothing" => category
                .as_ref()
                .and_then(|id| self.armor_categories.get(id)),
            _ => None,
        };
        if let Some(definition) = definition {
            occupied.extend(definition.occupies_slots.iter().cloned());
            if definition.handedness == Some(ResolvedWeaponHandedness::TwoHanded) {
                occupied.extend(self.weapon_slots.iter().cloned());
            }
        }
        occupied
    }

    pub fn check_item(
        &self,
        class_id: Option<&str>,
        kind: &str,
        category: Option<&str>,
        slot: &str,
    ) -> Result<(), String> {
        let kind = kind.trim().to_ascii_lowercase();
        let slot = slot.trim().to_ascii_lowercase();
        let category = category
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        let (known_categories, known_slots, permission) = match kind.as_str() {
            "weapon" | "weapons" => (
                Some(&self.weapon_categories),
                Some(&self.weapon_slots),
                class_id
                    .and_then(|class| {
                        self.class_permissions
                            .get(&class.trim().to_ascii_lowercase())
                    })
                    .and_then(|class| class.weapon_categories.as_ref()),
            ),
            "armor" | "armour" | "clothing" => (
                Some(&self.armor_categories),
                Some(&self.armor_slots),
                class_id
                    .and_then(|class| {
                        self.class_permissions
                            .get(&class.trim().to_ascii_lowercase())
                    })
                    .and_then(|class| class.armor_categories.as_ref()),
            ),
            _ => (None, None, None),
        };
        let Some(known_categories) = known_categories else {
            return Ok(());
        };
        let category =
            category.ok_or_else(|| format!("{} item has no equipment category.", kind))?;
        if !known_categories.is_empty() && !known_categories.contains_key(&category) {
            return Err(format!("Unknown {} category '{}'.", kind, category));
        }
        if let Some(known_slots) = known_slots
            && !known_slots.is_empty()
            && !known_slots.contains(&slot)
        {
            return Err(format!("Slot '{}' is not valid for a {} item.", slot, kind));
        }
        if let Some(permission) = permission
            && !permission.contains(&category)
        {
            return Err(format!(
                "Class does not allow {} category '{}'.",
                kind, category
            ));
        }
        Ok(())
    }
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
    let level_unlocks: BTreeMap<String, Vec<String>> = class
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
    let mut abilities = Vec::new();
    let mut spells = Vec::new();
    for entries in level_unlocks.values() {
        for entry in entries {
            if let Some(ability) = entry.strip_prefix("abilities:")
                && !abilities.contains(&ability.to_string())
            {
                abilities.push(ability.to_string());
            }
            if let Some(spell) = entry.strip_prefix("spells:")
                && !spells.contains(&spell.to_string())
            {
                spells.push(spell.to_string());
            }
        }
    }

    Ok(RulesetClassSummary {
        id: class_id.to_string(),
        description: table_string(class, "description"),
        role: table_string(class, "role"),
        primary_attributes: table_string_array(class, "primary_attributes"),
        allowed_weapons: table_string_array(class, "allowed_weapons"),
        allowed_armor: table_string_array(class, "allowed_armor"),
        abilities,
        spells,
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
    pub conditions: Vec<String>,
    pub invocation_schemes: Vec<String>,
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
        conditions: sorted_table_keys(root, &["conditions"]),
        invocation_schemes: sorted_table_keys(root, &["invocation_schemes"]),
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
        "invocation" | "invocations" | "invocation_scheme" | "invocation_schemes" => {
            Some(&["invocation_schemes"])
        }
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
            "Unknown ruleset section '{}'. Try races, classes, professions, skills, recipes, weapons, armor, spells, abilities, actions, or invocation_schemes.",
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
    let Some(progression) = ruleset_table_at_path(root, &["progression"]) else {
        return;
    };
    let level_table = progression.get("level").and_then(Value::as_table);
    let max_level = match level_table.and_then(|level| level.get("max_level")) {
        Some(Value::Integer(value)) if *value >= 1 => Some(*value as u32),
        Some(_) => {
            report.error(
                "progression.level.max_level",
                "Maximum level must be an integer of at least 1.",
            );
            None
        }
        None => None,
    };
    if let Some(expr) = level_table
        .and_then(|level| level.get("xp_for_level"))
        .and_then(Value::as_str)
        && !formula_is_valid(expr)
    {
        report.error(
            "progression.level.xp_for_level",
            "XP-for-level formula is invalid.",
        );
    }

    let Some(xp_table) = ruleset_table_at_path(root, &["progression", "xp_table"]) else {
        if level_table
            .and_then(|level| level.get("xp_for_level"))
            .and_then(Value::as_str)
            .is_none()
        {
            report.warning(
                "progression.xp_table",
                "Progression defines neither an XP table nor an xp_for_level formula.",
            );
        }
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
        if max_level.is_some_and(|max_level| level > max_level) {
            report.error(
                format!("progression.xp_table.{}", key),
                "XP table level exceeds progression.level.max_level.",
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

    if let Some(max_level) = max_level {
        for expected in 2..=max_level {
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
    let icons = table_key_set(root, &["icons"]);
    let mut item_templates = BTreeSet::new();
    for group in ruleset_item_group_names(root) {
        item_templates.extend(table_key_set(root, &["items", &group]));
        if let Some(items) = ruleset_table_at_path(root, &["items", &group]) {
            for (id, value) in items {
                if let Some(item) = value.as_table() {
                    validate_string_reference(
                        report,
                        &format!("items.{}.{}.icon", group, id),
                        "Icon",
                        table_string(item, "icon").as_deref(),
                        &icons,
                    );
                }
            }
        }
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
    if let Some(clothing) = ruleset_table_at_path(root, &["items", "clothing"]) {
        for (id, value) in clothing {
            let Some(item) = value.as_table() else {
                report.error(
                    format!("items.clothing.{}", id),
                    "Clothing entry must be a table.",
                );
                continue;
            };
            let path = format!("items.clothing.{}", id);
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
    let conditions = table_key_set(root, &["conditions"]);
    let icons = table_key_set(root, &["icons"]);
    let fx_presets = table_key_set(root, &["fx", "presets"]);
    let declared_attributes = declared_attribute_ids(root);
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
            match ResolvedAction::from_table(id, action) {
                Ok(resolved) => {
                    for cost in &resolved.resource_costs {
                        if !declared_attributes.contains(&cost.resource) {
                            report.error(
                                format!("{}.cost.{}", path, cost.resource),
                                format!("Attribute '{}' does not exist.", cost.resource),
                            );
                        }
                    }
                    for effect in &resolved.effects {
                        if let ResolvedActionEffect::ApplyCondition { condition, .. }
                        | ResolvedActionEffect::RemoveCondition { condition, .. } = effect
                        {
                            validate_string_reference(
                                report,
                                &format!("{}.result.condition", path),
                                "Condition",
                                Some(condition),
                                &conditions,
                            );
                        }
                        let source = match effect {
                            ResolvedActionEffect::Damage {
                                source: ResolvedActionValueSource::RulesetPath(source),
                            }
                            | ResolvedActionEffect::Healing {
                                source: ResolvedActionValueSource::RulesetPath(source),
                            } => Some(source),
                            _ => None,
                        };
                        if let Some(source) = source {
                            let parts = source
                                .split('.')
                                .map(str::trim)
                                .filter(|part| !part.is_empty())
                                .collect::<Vec<_>>();
                            if parts.is_empty()
                                || ruleset_value_at_path(root, &parts)
                                    .and_then(Value::as_table)
                                    .is_none()
                            {
                                report.error(
                                    format!("{}.result", path),
                                    format!(
                                        "Action effect source '{}' does not resolve to a ruleset table.",
                                        source
                                    ),
                                );
                            }
                        }
                    }
                    validate_string_reference(
                        report,
                        &format!("{}.source.item", path),
                        "Item",
                        resolved
                            .item_source
                            .as_ref()
                            .map(|source| source.item.as_str()),
                        &item_templates,
                    );
                    validate_string_reference(
                        report,
                        &format!("{}.ui.icon", path),
                        "Icon",
                        resolved.presentation.icon.as_deref(),
                        &icons,
                    );
                }
                Err(err) => report.error(&path, err),
            }
            if let Some(fx) = action.get("fx").and_then(Value::as_table) {
                for (stage, value) in fx {
                    let preset = value.as_str().map(str::to_string).or_else(|| {
                        value
                            .as_table()
                            .and_then(|stage| table_string(stage, "preset"))
                    });
                    if preset.is_none() {
                        report.error(
                            format!("{}.fx.{}", path, stage),
                            "Action FX stage must be a preset id or a table with preset.",
                        );
                    } else {
                        validate_string_reference(
                            report,
                            &format!("{}.fx.{}.preset", path, stage),
                            "FX preset",
                            preset.as_deref(),
                            &fx_presets,
                        );
                    }
                }
            }
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

fn validate_condition_rules(
    report: &mut RulesetValidationReport,
    root: &Table,
    damage_kinds: &BTreeSet<String>,
) {
    let conditions = match resolve_conditions(root) {
        Ok(conditions) => conditions,
        Err(err) => {
            report.error("conditions", err);
            return;
        }
    };
    for condition in conditions.values() {
        if let Some(periodic) = &condition.periodic {
            for (index, effect) in periodic.effects.iter().enumerate() {
                if let ResolvedConditionPeriodicEffect::Damage { damage_kind, .. } = effect {
                    validate_string_reference(
                        report,
                        &format!(
                            "conditions.{}.periodic.effects.{}.damage_kind",
                            condition.id, index
                        ),
                        "Damage kind",
                        Some(damage_kind),
                        damage_kinds,
                    );
                }
            }
        }
    }

    let presets = table_key_set(root, &["fx", "presets"]);
    if let Some(raw_conditions) = ruleset_table_at_path(root, &["conditions"]) {
        for (condition_id, value) in raw_conditions {
            let Some(condition) = value.as_table() else {
                continue;
            };
            let Some(fx_value) = condition.get("fx") else {
                continue;
            };
            let Some(fx) = fx_value.as_table() else {
                report.error(
                    format!("conditions.{}.fx", condition_id),
                    "Condition FX must be a table.",
                );
                continue;
            };
            for stage in fx.keys() {
                if !matches!(stage.as_str(), "apply" | "active" | "tick" | "remove") {
                    report.error(
                        format!("conditions.{}.fx.{}", condition_id, stage),
                        "Unsupported condition FX stage; use apply, active, tick, or remove.",
                    );
                }
            }
            for stage in ["apply", "active", "tick", "remove"] {
                let Some(value) = fx.get(stage) else {
                    continue;
                };
                let preset = value.as_str().map(str::to_string).or_else(|| {
                    value
                        .as_table()
                        .and_then(|stage| table_string(stage, "preset"))
                });
                if preset.is_none() {
                    report.error(
                        format!("conditions.{}.fx.{}", condition_id, stage),
                        "Condition FX stage must be a preset id or a table with preset.",
                    );
                    continue;
                }
                validate_string_reference(
                    report,
                    &format!("conditions.{}.fx.{}.preset", condition_id, stage),
                    "FX preset",
                    preset.as_deref(),
                    &presets,
                );
            }
        }
    }
}

fn validate_derived_stat_rules(report: &mut RulesetValidationReport, root: &Table) {
    let stats = match resolve_derived_stats(root) {
        Ok(stats) => stats,
        Err(err) => {
            report.error("derived_stats", err);
            return;
        }
    };

    fn visit(
        stat_id: &str,
        stats: &BTreeMap<String, ResolvedDerivedStat>,
        visiting: &mut Vec<String>,
        visited: &mut BTreeSet<String>,
        report: &mut RulesetValidationReport,
    ) {
        if visited.contains(stat_id) {
            return;
        }
        if let Some(index) = visiting.iter().position(|entry| entry == stat_id) {
            let mut cycle = visiting[index..].to_vec();
            cycle.push(stat_id.to_string());
            report.error(
                format!("derived_stats.{}.formula", stat_id),
                format!("Derived-stat dependency cycle: {}.", cycle.join(" -> ")),
            );
            return;
        }
        let Some(stat) = stats.get(stat_id) else {
            return;
        };
        visiting.push(stat_id.to_string());
        for dependency in &stat.dependencies {
            if let Some(dependency_id) = stats
                .keys()
                .find(|candidate| candidate.eq_ignore_ascii_case(dependency))
            {
                visit(dependency_id, stats, visiting, visited, report);
            }
        }
        visiting.pop();
        visited.insert(stat_id.to_string());
    }

    let mut visiting = Vec::new();
    let mut visited = BTreeSet::new();
    for stat_id in stats.keys() {
        visit(stat_id, &stats, &mut visiting, &mut visited, report);
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
    if let Some(crafting_value) = root.get("crafting") {
        let Some(crafting) = crafting_value.as_table() else {
            report.error("crafting", "Crafting configuration must be a table.");
            return;
        };
        if let Some(skill_gain_value) = crafting.get("skill_gain") {
            let Some(skill_gain) = skill_gain_value.as_table() else {
                report.error(
                    "crafting.skill_gain",
                    "Crafting skill gain must be a table.",
                );
                return;
            };
            if skill_gain
                .get("enabled")
                .is_some_and(|value| value.as_bool().is_none())
            {
                report.error(
                    "crafting.skill_gain.enabled",
                    "Crafting skill gain enabled must be a boolean.",
                );
            }
            for key in ["per_success", "below_recommended_bonus", "mastery_margin"] {
                let Some(value) = skill_gain.get(key) else {
                    continue;
                };
                let number = value
                    .as_float()
                    .or_else(|| value.as_integer().map(|value| value as f64));
                if !number.is_some_and(|number| number.is_finite() && number >= 0.0) {
                    report.error(
                        format!("crafting.skill_gain.{}", key),
                        "Crafting skill gain values must be finite numbers at least zero.",
                    );
                }
            }
        }
    }

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
    let actions = table_key_set(root, &["actions"]);
    let declared_attributes = declared_attribute_ids(root);
    let mut action_abilities = BTreeSet::new();
    let mut action_spells = BTreeSet::new();
    if let Some(action_tables) = ruleset_table_at_path(root, &["actions"]) {
        for value in action_tables.values() {
            let Some(action) = value.as_table() else {
                continue;
            };
            let Some(requires) = action.get("requires").and_then(Value::as_table) else {
                continue;
            };
            if let Some(ability) = table_string(requires, "ability") {
                action_abilities.insert(ability);
            }
            if let Some(spell) = table_string(requires, "spell") {
                action_spells.insert(spell);
            }
        }
    }
    let max_level = ruleset_table_at_path(root, &["progression", "level"])
        .and_then(|level| level.get("max_level"))
        .and_then(Value::as_integer)
        .map(|level| level.max(1) as u32);
    let equipment_policy = resolve_equipment_policy(root).ok();

    let Some(classes) = ruleset_table_at_path(root, &["classes"]) else {
        return;
    };

    for (id, value) in classes {
        let Some(class) = value.as_table() else {
            report.error(format!("classes.{}", id), "Class entry must be a table.");
            continue;
        };
        let path = format!("classes.{}", id);
        match resolve_class_resource_gains(root, id) {
            Ok(gains) => {
                for (index, gain) in gains.iter().enumerate() {
                    for (key, attribute) in [
                        ("attribute", &gain.attribute),
                        ("maximum_attribute", &gain.maximum_attribute),
                    ] {
                        if !declared_attributes.contains(attribute) {
                            report.error(
                                format!(
                                    "{}.progression.level.resource_gains.{}.{}",
                                    path, index, key
                                ),
                                format!("Attribute '{}' does not exist.", attribute),
                            );
                        }
                    }
                }
            }
            Err(err) => report.error(format!("{}.progression.level", path), err),
        }
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
        for key in ["abilities", "spells", "spell_lists"] {
            if class.contains_key(key) {
                report.error(
                    format!("{}.{}", path, key),
                    "Class abilities and spells must be owned by unlocks.level_N.",
                );
            }
        }
        if let Some(loadout) = class.get("starting_loadout").and_then(Value::as_table) {
            for key in ["abilities", "spells", "spell_lists"] {
                if loadout.contains_key(key) {
                    report.error(
                        format!("{}.starting_loadout.{}", path, key),
                        "Starting loadout owns items only; use unlocks.level_1 for abilities and spells.",
                    );
                }
            }
            if let Some(policy) = equipment_policy.as_ref() {
                let mut occupied_by = BTreeMap::<String, String>::new();
                for (list, kind, group) in [
                    ("weapons", "weapon", "weapons"),
                    ("armor", "armor", "armor"),
                    ("clothing", "clothing", "clothing"),
                ] {
                    for item_id in table_string_array(loadout, list) {
                        let Some(item) = ruleset_table_at_path(root, &["items", group, &item_id])
                        else {
                            continue;
                        };
                        let category = table_string(item, "category");
                        let slot = table_string(item, "slot");
                        let Some(slot) = slot else {
                            continue;
                        };
                        if let Err(err) =
                            policy.check_item(Some(id), kind, category.as_deref(), &slot)
                        {
                            report.error(
                                format!("{}.starting_loadout.{}", path, list),
                                format!("Item '{}': {}", item_id, err),
                            );
                        }
                        for occupied in policy.occupied_slots(kind, category.as_deref(), &slot) {
                            if let Some(other) = occupied_by.get(&occupied) {
                                report.error(
                                    format!("{}.starting_loadout", path),
                                    format!(
                                        "Items '{}' and '{}' both occupy slot '{}'.",
                                        other, item_id, occupied
                                    ),
                                );
                            } else {
                                occupied_by.insert(occupied, item_id.clone());
                            }
                        }
                    }
                }
            }
        }
        if let Some(unlocks) = class.get("unlocks").and_then(Value::as_table) {
            for (level_key, value) in unlocks {
                let Some(level) = level_key
                    .strip_prefix("level_")
                    .and_then(|value| value.parse::<u32>().ok())
                else {
                    report.error(
                        format!("{}.unlocks.{}", path, level_key),
                        "Unlock keys must use level_N.",
                    );
                    continue;
                };
                if level == 0 || max_level.is_some_and(|max_level| level > max_level) {
                    report.error(
                        format!("{}.unlocks.{}", path, level_key),
                        "Unlock level must be between 1 and progression.level.max_level.",
                    );
                }
                let Some(unlock) = value.as_table() else {
                    continue;
                };
                for ability in table_string_array(unlock, "abilities") {
                    if !action_abilities.contains(&ability) {
                        report.error(
                            format!("{}.unlocks.{}.abilities", path, level_key),
                            format!("Ability '{}' has no requiring action.", ability),
                        );
                    }
                }
                for spell in table_string_array(unlock, "spells") {
                    if !action_spells.contains(&spell) {
                        report.error(
                            format!("{}.unlocks.{}.spells", path, level_key),
                            format!("Spell '{}' has no requiring action.", spell),
                        );
                    }
                }
            }
        }
        if let Some(action_bar) = class.get("action_bar").and_then(Value::as_table) {
            for (bar_id, value) in action_bar {
                let Some(commands) = value.as_array() else {
                    report.error(
                        format!("{}.action_bar.{}", path, bar_id),
                        "Action bar entries must be arrays.",
                    );
                    continue;
                };
                for (index, command) in commands.iter().filter_map(Value::as_str).enumerate() {
                    if let Some(action_id) = command.trim().strip_prefix("rules.")
                        && !actions.contains(action_id)
                    {
                        report.error(
                            format!("{}.action_bar.{}.{}", path, bar_id, index),
                            format!("Action '{}' does not exist.", action_id),
                        );
                    }
                }
            }
        }

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
    let icons = table_key_set(root, &["icons"]);
    let fx_presets = table_key_set(root, &["fx", "presets"]);
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
    for fallback_table in ["action_icon_fallbacks", "item_icon_fallbacks"] {
        if let Some(fallbacks) = ruleset_table_at_path(root, &["ui", fallback_table]) {
            for (role, value) in fallbacks {
                if value.as_str().is_none() {
                    report.error(
                        format!("ui.{}.{}", fallback_table, role),
                        "Icon fallback must be an icon id string.",
                    );
                    continue;
                }
                validate_string_reference(
                    report,
                    &format!("ui.{}.{}", fallback_table, role),
                    "Icon",
                    value.as_str(),
                    &icons,
                );
            }
        }
    }
    if let Some(fallbacks) = ruleset_table_at_path(root, &["fx", "action_fallbacks"]) {
        for (role, value) in fallbacks {
            let Some(stages) = value.as_table() else {
                report.error(
                    format!("fx.action_fallbacks.{}", role),
                    "Action FX fallback role must be a table of stage = preset mappings.",
                );
                continue;
            };
            for (stage, value) in stages {
                if value.as_str().is_none() {
                    report.error(
                        format!("fx.action_fallbacks.{}.{}", role, stage),
                        "Action FX fallback must be a preset id string.",
                    );
                    continue;
                }
                validate_string_reference(
                    report,
                    &format!("fx.action_fallbacks.{}.{}", role, stage),
                    "FX preset",
                    value.as_str(),
                    &fx_presets,
                );
            }
        }
    }
    if let Some(fallbacks) = ruleset_table_at_path(root, &["fx", "condition_fallbacks"]) {
        for (stage, value) in fallbacks {
            if value.as_str().is_none() {
                report.error(
                    format!("fx.condition_fallbacks.{}", stage),
                    "Condition FX fallback must be a preset id string.",
                );
                continue;
            }
            validate_string_reference(
                report,
                &format!("fx.condition_fallbacks.{}", stage),
                "FX preset",
                value.as_str(),
                &fx_presets,
            );
        }
    }
}

pub fn declared_attribute_ids(root: &Table) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if let Some(attributes) = ruleset_table_at_path(root, &["attributes"]) {
        for list in ["primary", "resources", "combat", "progression"] {
            ids.extend(table_string_array(attributes, list));
        }
        if let Some(defaults) = attributes.get("defaults").and_then(Value::as_table) {
            ids.extend(defaults.keys().cloned());
        }
    }
    ids.extend(table_key_set(root, &["derived_stats"]));
    ids
}

fn validate_attribute_roles(report: &mut RulesetValidationReport, root: &Table) {
    let roles = match resolve_attribute_roles(root) {
        Ok(roles) => roles,
        Err(err) => {
            report.error("attributes.roles", err);
            return;
        }
    };
    let attributes = declared_attribute_ids(root);
    for (role, attribute) in roles.attributes {
        if !attributes.contains(&attribute) {
            report.error(
                format!("attributes.roles.{}", role),
                format!("Attribute '{}' does not exist.", attribute),
            );
        }
    }
}

fn validate_identity_rules(report: &mut RulesetValidationReport, root: &Table) {
    let races = table_key_set(root, &["races"]);
    let classes = table_key_set(root, &["classes"]);
    if let Err(err) = resolve_identity_defaults(root) {
        report.error("identity.defaults", err);
        return;
    }
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

fn validate_invocation_rules(report: &mut RulesetValidationReport, root: &Table) {
    if let Err(err) = resolve_action_catalogue(root) {
        report.error("invocations", err);
    }
}

pub fn validate_ruleset(root: &Table) -> RulesetValidationReport {
    let mut report = RulesetValidationReport::default();
    let damage_kinds = table_key_set(root, &["combat", "kinds"]);

    validate_attribute_roles(&mut report, root);
    validate_identity_rules(&mut report, root);
    validate_relation_and_intent_rules(&mut report, root);
    validate_visual_rules(&mut report, root);
    if let Err(err) = resolve_equipment_policy(root) {
        report.error("equipment", err);
    }
    validate_xp_table(&mut report, root);
    validate_roll_path(&mut report, root, &["combat", "unarmed_damage"]);
    validate_item_rules(&mut report, root, &damage_kinds);
    validate_condition_rules(&mut report, root, &damage_kinds);
    validate_derived_stat_rules(&mut report, root);
    validate_ability_and_spell_rules(&mut report, root, &damage_kinds);
    validate_recipe_rules(&mut report, root);
    validate_resource_rules(&mut report, root);
    validate_class_rules(&mut report, root);
    validate_invocation_rules(&mut report, root);

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
    pub source: String,
    pub data: String,
    pub authoring: String,
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
    let source = table_string(item, "script")
        .or_else(|| table_string(item, "source"))
        .unwrap_or_default();
    let authoring = item
        .get("authoring")
        .and_then(Value::as_table)
        .map(|authoring| toml::to_string(authoring).unwrap_or_default())
        .unwrap_or_default();

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
    insert_integer(&mut attributes, "quality", 100);
    insert_integer(&mut attributes, "condition", 100);

    for key in ["category", "slot", "rarity"] {
        if let Some(value) = table_string(item, key) {
            insert_string(&mut attributes, key, value);
        }
    }

    for key in [
        "icon",
        "visual_template",
        "container_template",
        "icon_template",
        "rig_template",
        "icon_color",
        "rig_scale",
        "rig_pivot",
        "rig_layer",
        "rig_flip_back",
        "appearance_recipe",
        "headgear_recipe",
        "appearance_space",
        "appearance_tiling",
        "appearance_seed",
        "worth",
        "monetary",
        "currency",
        "amount",
        "quality",
        "condition",
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
    if !attributes.contains_key("icon")
        && let Some(icon) = resolve_item_icon(root_table, Some(kind), None)
    {
        insert_string(&mut attributes, "icon", icon);
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

    if let Some(damage) = item.get("damage").and_then(Value::as_table) {
        if let Some(roll) = table_string(damage, "roll") {
            insert_string(&mut attributes, "damage_roll", roll);
        }
        if let Some(value) = damage.get("bonus") {
            attributes.insert("damage_bonus".to_string(), value.clone());
        }
        if let Some(attribute) = table_string(damage, "bonus_attribute") {
            insert_string(&mut attributes, "damage_bonus_attribute", attribute);
        }
        if let Some(value) = damage.get("bonus_every") {
            attributes.insert("damage_bonus_every".to_string(), value.clone());
        }
        if let Some(kind) = table_string(damage, "damage_kind") {
            insert_string(&mut attributes, "damage_kind", kind);
        }
    }

    if let Some(durability) = item.get("durability").and_then(Value::as_table) {
        for (key, value) in durability {
            attributes.insert(format!("durability_{}", key), value.clone());
        }
    }

    insert_string(&mut attributes, "ruleset_path", &ruleset_path);
    insert_string(&mut attributes, "ruleset_kind", kind);
    insert_string(&mut attributes, "ruleset_id", id);

    let mut root = Table::new();
    for key in [
        "tile_id",
        "tile_id_front",
        "tile_id_back",
        "tile_id_left",
        "tile_id_right",
        "rig_tile_id",
        "rig_tile_id_front",
        "rig_tile_id_back",
        "rig_tile_id_left",
        "rig_tile_id_right",
    ] {
        if let Some(value) = item.get(key) {
            root.insert(key.to_string(), value.clone());
        }
    }
    if let Some(light) = item.get("light").and_then(Value::as_table) {
        root.insert("light".to_string(), Value::Table(light.clone()));
    }
    root.insert("attributes".to_string(), Value::Table(attributes));

    let mut ruleset = Table::new();
    if let Some(damage) = item.get("damage").and_then(Value::as_table) {
        ruleset.insert("damage".to_string(), Value::Table(damage.clone()));
    }
    if let Some(durability) = item.get("durability").and_then(Value::as_table) {
        ruleset.insert("durability".to_string(), Value::Table(durability.clone()));
    }
    if !ruleset.is_empty() {
        root.insert("ruleset".to_string(), Value::Table(ruleset));
    }

    let data = toml::to_string(&root)
        .map_err(|err| format!("Ruleset item '{}' could not be serialized: {}", id, err))?;

    Ok(RulesetItemTemplate {
        id: id.to_string(),
        name,
        kind: kind.to_string(),
        ruleset_path,
        source,
        data,
        authoring,
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
    fn reads_typed_ruleset_selection() {
        let selection = selected_ruleset_config(
            r#"
            [ruleset]
            id = "example.rules"
            version = "2.3.4"
            schema_version = "1"
            source = "project"
            update_policy = "pinned"
            "#,
        );

        assert_eq!(
            selection,
            RulesetSelection {
                id: "example.rules".into(),
                version: "2.3.4".into(),
                schema_version: "1".into(),
                source: "project".into(),
                update_policy: "pinned".into(),
            }
        );
    }

    #[test]
    fn resolves_official_ruleset_through_typed_model() {
        let resolved =
            resolve_project_ruleset(DEFAULT_RULESET_CONFIG, DEFAULT_RULES_OVERRIDE).unwrap();

        assert_eq!(resolved.selection().id, OFFICIAL_RULESET_ID);
        assert_eq!(resolved.selection().source, "official");
        assert_eq!(resolved.metadata().id, OFFICIAL_RULESET_ID);
        assert_eq!(resolved.metadata().version, OFFICIAL_RULESET_VERSION);
        assert_eq!(
            resolved.metadata().schema_version,
            OFFICIAL_RULESET_SCHEMA_VERSION
        );
        assert_eq!(
            resolved.metadata().min_engine_version.as_deref(),
            Some("0.91.0")
        );
        assert_eq!(resolved.metadata().status.as_deref(), Some("draft"));
        assert!(resolved.validation().is_ok());
        assert!(resolved.table().get("actions").is_some());
        assert!(resolved.table().get("items").is_some());

        let actions = resolved.actions().unwrap();
        assert_eq!(actions.len(), 37);
        let basic_attack = actions.get("basic_attack").unwrap();
        assert_eq!(basic_attack.kind, ResolvedActionKind::Attack);
        assert_eq!(
            basic_attack.target,
            ResolvedActionTarget::HostileOrNeutralEntity
        );
        assert_eq!(
            basic_attack.range,
            ResolvedActionRange::Weapon { fallback: 1.5 }
        );
        assert_eq!(basic_attack.cooldown_seconds, 1.0);
        assert_eq!(basic_attack.required_ability(), Some("basic_attack"));
        assert_eq!(
            basic_attack.damage_source(),
            Some(&ResolvedActionValueSource::Weapon)
        );
        assert_eq!(
            basic_attack.presentation.icon.as_deref(),
            Some("basic_attack")
        );
        let guard = actions.get("guard").unwrap();
        assert_eq!(guard.required_ability(), Some("guard"));
        assert!(guard.has_condition_effects());
        assert!(matches!(
            guard.effects.as_slice(),
            [ResolvedActionEffect::ApplyCondition {
                condition,
                recipient: ResolvedActionEffectRecipient::Target,
            }] if condition == "guarded"
        ));
        let guarded = resolved.conditions().unwrap().remove("guarded").unwrap();
        assert_eq!(guarded.duration_seconds, 2.0);
        assert_eq!(guarded.stacking, ResolvedConditionStacking::Refresh);
        assert_eq!(
            guarded.modifiers,
            [ResolvedConditionModifier {
                attribute: "ARMOR".into(),
                add: 2.0,
                multiply: 1.0,
                minimum: None,
                maximum: None,
            }]
        );
        let words = resolved
            .invocation_schemes()
            .unwrap()
            .remove("words_of_power")
            .unwrap();
        assert_eq!(words.kind, ResolvedInvocationSchemeKind::TokenSequence);
        assert_eq!(words.tokens, ["LO", "VI", "FUL", "YA", "IR", "SAR"]);
        assert_eq!(
            resolved
                .action_for_invocation("words_of_power", "  lo   vi ")
                .unwrap()
                .map(|action| action.id),
            Some("minor_heal".into())
        );
    }

    #[test]
    fn validates_action_invocation_schemes_tokens_lengths_and_collisions() {
        let valid = r#"
            [invocation_schemes.gestures]
            kind = "token_sequence"
            tokens = ["UP", "LEFT", "RIGHT"]
            max_tokens = 3

            [actions.open_way]
            target = "self"
            invocations = [
                { scheme = "gestures", sequence = ["UP", "LEFT"] },
            ]
            result = { script = "open_way" }
            "#
        .parse::<Table>()
        .unwrap();
        assert_eq!(
            resolve_action_invocation(&valid, "gestures", "up left")
                .unwrap()
                .map(|action| action.id),
            Some("open_way".into())
        );

        let unknown_scheme = r#"
            [actions.broken]
            invocations = [{ scheme = "missing", sequence = ["LO"] }]
            result = { script = "never" }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_actions(&unknown_scheme)
                .unwrap_err()
                .contains("unknown invocation scheme")
        );

        let unknown_token = r#"
            [invocation_schemes.words]
            tokens = ["LO"]

            [actions.broken]
            invocations = [{ scheme = "words", sequence = ["VI"] }]
            result = { script = "never" }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_actions(&unknown_token)
                .unwrap_err()
                .contains("unknown token")
        );

        let too_long = r#"
            [invocation_schemes.words]
            tokens = ["LO", "VI"]
            max_tokens = 1

            [actions.broken]
            invocations = [{ scheme = "words", sequence = ["LO", "VI"] }]
            result = { script = "never" }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_actions(&too_long)
                .unwrap_err()
                .contains("max_tokens")
        );

        let collision = r#"
            [invocation_schemes.words]
            tokens = ["LO", "VI"]

            [actions.first]
            invocations = [{ scheme = "words", sequence = ["LO", "VI"] }]
            result = { script = "first" }

            [actions.second]
            invocations = [{ scheme = "words", sequence = ["lo", "vi"] }]
            result = { script = "second" }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_actions(&collision)
                .unwrap_err()
                .contains("already bound")
        );
    }

    #[test]
    fn action_intents_are_generic_unique_bindings() {
        let root = parse_ruleset_table(
            r#"
            [actions.strike]
            kind = "attack"
            intent = "attack"
            target = "hostile_entity"
            result = { damage = "weapon" }
            "#,
        )
        .unwrap();
        let catalogue = resolve_action_catalogue(&root).unwrap();
        assert_eq!(
            catalogue
                .action_for_intent("ATTACK")
                .map(|action| action.id.as_str()),
            Some("strike")
        );

        let duplicate = parse_ruleset_table(
            r#"
            [actions.strike]
            intent = "attack"

            [actions.kick]
            intent = "ATTACK"
            "#,
        )
        .unwrap();
        let error = resolve_action_catalogue(&duplicate).unwrap_err();
        assert!(error.contains("already bound"));
        assert!(validate_ruleset(&duplicate).issues.iter().any(|issue| {
            issue.path == "invocations" && issue.message.contains("already bound")
        }));
    }

    #[test]
    fn typed_action_validation_rejects_invalid_execution_values() {
        let root = r#"
            [actions.broken]
            name = "Broken"
            kind = "attack"
            target = "hostile_entity"
            cooldown = -1
            consumes = [{ item = "arrow", quantity = 0 }]
            result = { damage = "weapon" }
            "#
        .parse::<Table>()
        .unwrap();

        let error = resolve_action(&root, "broken").unwrap_err();
        assert!(error.contains("cooldown"));

        let report = validate_ruleset(&root);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "actions.broken" && issue.message.contains("cooldown"))
        );

        let invalid_script = r#"
            [actions.broken_script]
            target = "any_item"
            result = { script = "" }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_action(&invalid_script, "broken_script")
                .unwrap_err()
                .contains("non-empty event name")
        );

        let invalid_source = r#"
            [actions.broken_source]
            target = "any_item"
            source = { item = "lockpick", condition_cost = 1 }
            result = { take = true }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_action(&invalid_source, "broken_source")
                .unwrap_err()
                .contains("currently requires a result.script effect")
        );

        let ambiguous_predicate = r#"
            [actions.broken_predicate]
            requires = { attributes = [
                { id = "karma", at_least = 10, at_most = 20 },
            ] }
            result = { script = "never" }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_action(&ambiguous_predicate, "broken_predicate")
                .unwrap_err()
                .contains("must define exactly one")
        );

        let invalid_modification_target = r#"
            [actions.broken_modify]
            target = "world_position"
            result = { modify = [
                { attribute = "mode", set = "marked" },
            ] }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_action(&invalid_modification_target, "broken_modify")
                .unwrap_err()
                .contains("require an entity action target")
        );

        let ambiguous_modification = r#"
            [actions.broken_modify]
            target = "self"
            result = { modify = [
                { resource = "stamina", add = 1, set = 2 },
            ] }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_action(&ambiguous_modification, "broken_modify")
                .unwrap_err()
                .contains("exactly one of add or set")
        );

        let invalid_target_predicate = r#"
            [actions.broken_target_requirement]
            target = "world_position"
            requires = { target_attributes = [
                { id = "traits", contains = "undead" },
            ] }
            result = { script = "never" }
            "#
        .parse::<Table>()
        .unwrap();
        assert!(
            resolve_action(&invalid_target_predicate, "broken_target_requirement")
                .unwrap_err()
                .contains("requires an entity action target")
        );
    }

    #[test]
    fn typed_attribute_predicates_match_supported_value_kinds() {
        assert!(
            ResolvedActionAttributePredicate::Equals(ResolvedActionPredicateValue::Bool(true))
                .matches_bool(true)
        );
        assert!(
            ResolvedActionAttributePredicate::NotEquals(ResolvedActionPredicateValue::String(
                "combat".into()
            ))
            .matches_string("stealth")
        );
        assert!(ResolvedActionAttributePredicate::AtLeast(10.0).matches_number(10.0));
        assert!(!ResolvedActionAttributePredicate::AtMost(9.0).matches_number(10.0));
        assert!(
            ResolvedActionAttributePredicate::Contains("undead".into())
                .matches_string("skeletal, undead")
        );
        assert!(
            ResolvedActionAttributePredicate::NotContains("living".into())
                .matches_strings(&["undead".into(), "skeletal".into()])
        );
    }

    #[test]
    fn resolves_project_owned_standalone_schema_1_ruleset() {
        let config = r#"
            [ruleset]
            id = "example.standalone"
            version = "1.0.0"
            schema_version = "1"
            source = "project"
            update_policy = "pinned"
            "#;
        let source = r#"
            [attributes]
            primary = ["BODY"]

            [attributes.defaults]
            BODY = 5

            [skills.mining]
            name = "Mining"

            [invocation_schemes.gestures]
            name = "Mining Gestures"
            tokens = ["TAP", "TURN"]
            max_tokens = 2

            [actions.mine_ore]
            name = "Mine Ore"
            kind = "gather"
            skill = "mining"
            required_skill = 25
            target = "resource_node"
            range = 2
            cooldown = 1
            consumes = [{ item = "mining_charge", quantity = 1 }]
            result = { item = "iron_ore", quantity = 2 }

            [actions.inspect_item]
            name = "Inspect Item"
            kind = "interaction"
            target = "any_item"
            range = 3
            requires = { attributes = [
                { id = "mode", equals = "active" },
                { id = "karma", at_least = 10 },
                { id = "traits", contains = "undead" },
            ] }
            result = { script = "inspect_target" }

            [actions.unlock_chest]
            name = "Unlock Chest"
            kind = "interaction"
            target = "any_item"
            range = 2
            source = { item = "lockpick", condition_cost = 5, destroy_on_empty = true }
            result = { script = "unlock_target" }

            [actions.mark_location]
            name = "Mark Location"
            kind = "interaction"
            target = "world_position"
            range = 6
            result = { script = "mark_location" }

            [actions.intimidate]
            name = "Intimidate"
            kind = "interaction"
            target = "any_entity"
            range = 2
            requires = { attributes = [
                { id = "stamina", at_least = 2 },
            ] }
            result = { script = "intimidated", modify = [
                { recipient = "actor", resource = "stamina", add = -2, minimum = 0 },
                { attribute = "karma", add = -5, minimum = 0 },
                { attribute = "mode", set = "frightened" },
            ] }

            [actions.rest]
            name = "Rest"
            kind = "interaction"
            target = "self"
            invocations = [{ scheme = "gestures", sequence = ["TAP", "TURN"] }]
            result = { modify = [
                { resource = "stamina", add = 10, maximum_attribute = "max_stamina" },
            ] }

            [actions.turn_undead]
            name = "Turn Undead"
            kind = "interaction"
            target = "any_entity"
            range = 3
            requires = { target_attributes = [
                { id = "traits", contains = "undead" },
            ] }
            result = { modify = [
                { attribute = "mode", set = "turned" },
            ] }

            [items.materials.iron_ore]
            name = "Iron Ore"
            slot = "material"
            stackable = true
            max_stack = 100

            [items.tools.mining_charge]
            name = "Mining Charge"
            slot = "tool"
            stackable = true
            max_stack = 20

            [items.tools.lockpick]
            name = "Lockpick"
            slot = "tool"
            stackable = false

            [resources.iron_vein]
            name = "Iron Vein"
            action = "mine_ore"
            skill = "mining"
            "#;

        let resolved = resolve_project_ruleset(config, source).unwrap();

        assert_eq!(resolved.metadata().id, "example.standalone");
        assert_eq!(resolved.metadata().version, "1.0.0");
        assert_eq!(resolved.metadata().schema_version, "1");
        assert_eq!(resolved.selection().source, "project");
        assert_eq!(
            resolved
                .table()
                .get("attributes")
                .and_then(Value::as_table)
                .and_then(|attributes| attributes.get("defaults"))
                .and_then(Value::as_table)
                .and_then(|defaults| defaults.get("BODY"))
                .and_then(Value::as_integer),
            Some(5)
        );
        assert!(resolved.table().get("classes").is_none());
        assert!(resolved.table().get("progression").is_none());
        assert!(resolved.validation().is_ok());
        let mine_ore = resolved
            .action("mine_ore")
            .unwrap()
            .expect("standalone sandbox action resolves");
        assert_eq!(mine_ore.kind, ResolvedActionKind::Gather);
        assert_eq!(mine_ore.target, ResolvedActionTarget::ResourceNode);
        assert_eq!(mine_ore.skill_requirement(), Some(("mining", 25)));
        assert_eq!(
            mine_ore.item_costs,
            vec![ResolvedActionItemCost {
                item: "mining_charge".into(),
                quantity: 1,
            }]
        );
        assert!(matches!(
            mine_ore.effects.as_slice(),
            [ResolvedActionEffect::GiveItem { item, quantity }]
                if item == "iron_ore" && *quantity == 2
        ));
        let inspect_item = resolved
            .action("inspect_item")
            .unwrap()
            .expect("standalone item action resolves");
        assert_eq!(inspect_item.target, ResolvedActionTarget::AnyItem);
        assert_eq!(inspect_item.script_event(), Some("inspect_target"));
        assert_eq!(
            inspect_item.requirements,
            vec![
                ResolvedActionRequirement::Attribute {
                    id: "mode".into(),
                    predicate: ResolvedActionAttributePredicate::Equals(
                        ResolvedActionPredicateValue::String("active".into())
                    ),
                },
                ResolvedActionRequirement::Attribute {
                    id: "karma".into(),
                    predicate: ResolvedActionAttributePredicate::AtLeast(10.0),
                },
                ResolvedActionRequirement::Attribute {
                    id: "traits".into(),
                    predicate: ResolvedActionAttributePredicate::Contains("undead".into()),
                },
            ]
        );
        let unlock_chest = resolved
            .action("unlock_chest")
            .unwrap()
            .expect("standalone source-item action resolves");
        assert_eq!(
            unlock_chest.item_source,
            Some(ResolvedActionItemSource {
                item: "lockpick".into(),
                condition_cost: 5.0,
                destroy_on_empty: true,
            })
        );
        assert_eq!(unlock_chest.script_event(), Some("unlock_target"));
        let mark_location = resolved
            .action("mark_location")
            .unwrap()
            .expect("standalone position action resolves");
        assert_eq!(mark_location.target, ResolvedActionTarget::WorldPosition);
        assert_eq!(mark_location.script_event(), Some("mark_location"));
        let intimidate = resolved
            .action("intimidate")
            .unwrap()
            .expect("standalone state action resolves");
        assert!(intimidate.has_state_modifications());
        assert_eq!(intimidate.script_event(), Some("intimidated"));
        assert!(matches!(
            intimidate.effects.as_slice(),
            [
                ResolvedActionEffect::Script { event },
                ResolvedActionEffect::Modify {
                    recipient: ResolvedActionEffectRecipient::Actor,
                    field: ResolvedActionModificationField::Resource(resource),
                    operation: ResolvedActionModification::Add(-2.0),
                    minimum: Some(0.0),
                    ..
                },
                ResolvedActionEffect::Modify {
                    recipient: ResolvedActionEffectRecipient::Target,
                    field: ResolvedActionModificationField::Attribute(attribute),
                    operation: ResolvedActionModification::Add(-5.0),
                    minimum: Some(0.0),
                    ..
                },
                ResolvedActionEffect::Modify {
                    recipient: ResolvedActionEffectRecipient::Target,
                    field: ResolvedActionModificationField::Attribute(mode),
                    operation: ResolvedActionModification::Set(
                        ResolvedActionEffectValue::String(value)
                    ),
                    ..
                },
            ] if event == "intimidated"
                && resource == "stamina"
                && attribute == "karma"
                && mode == "mode"
                && value == "frightened"
        ));
        let turn_undead = resolved
            .action("turn_undead")
            .unwrap()
            .expect("target-gated action resolves");
        assert!(matches!(
            turn_undead.requirements.as_slice(),
            [ResolvedActionRequirement::TargetAttribute {
                id,
                predicate: ResolvedActionAttributePredicate::Contains(trait_id),
            }] if id == "traits" && trait_id == "undead"
        ));
        assert_eq!(
            resolved
                .action_for_invocation("gestures", "tap turn")
                .unwrap()
                .map(|action| action.id),
            Some("rest".into())
        );
    }

    #[test]
    fn rejects_unsupported_selected_schema_before_runtime_resolution() {
        let error = resolve_project_ruleset(
            r#"
            [ruleset]
            id = "example.future"
            version = "1.0.0"
            schema_version = "99"
            source = "project"
            "#,
            "",
        )
        .unwrap_err();

        assert!(error.contains("unsupported schema version '99'"));
        assert!(error.contains("supports: 1"));
    }

    #[test]
    fn rejects_schema_mismatch_between_selection_and_project_rules() {
        let error = resolve_project_ruleset(
            r#"
            [ruleset]
            id = "example.mismatch"
            version = "1.0.0"
            schema_version = "1"
            source = "project"
            "#,
            r#"
            [ruleset]
            id = "example.mismatch"
            version = "1.0.0"
            schema_version = "2"
            "#,
        )
        .unwrap_err();

        assert!(error.contains("unsupported schema version '2'"));
    }

    #[test]
    fn rejects_rulesets_that_require_a_newer_engine() {
        let error = resolve_project_ruleset(
            r#"
            [ruleset]
            id = "example.future-engine"
            version = "1.0.0"
            schema_version = "1"
            min_engine_version = "99.0.0"
            source = "project"
            "#,
            r#"
            [ruleset]
            id = "example.future-engine"
            version = "1.0.0"
            schema_version = "1"
            min_engine_version = "99.0.0"
            "#,
        )
        .unwrap_err();

        assert!(error.contains("requires Eldiron engine 99.0.0"));
        assert!(error.contains(ENGINE_VERSION));
    }

    #[test]
    fn rejects_invalid_minimum_engine_versions() {
        let error = resolve_project_ruleset(
            r#"
            [ruleset]
            id = "example.bad-engine-version"
            version = "1.0.0"
            schema_version = "1"
            source = "project"
            "#,
            r#"
            [ruleset]
            id = "example.bad-engine-version"
            version = "1.0.0"
            schema_version = "1"
            min_engine_version = "future"
            "#,
        )
        .unwrap_err();

        assert!(error.contains("not a valid three-part version"));
    }

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
        let avatars =
            bundled_avatar_assets_for_ruleset(OFFICIAL_RULESET_ID, OFFICIAL_RULESET_VERSION);
        let avatar = avatars
            .iter()
            .find(|asset| asset.id == "humanoid")
            .expect("humanoid avatar should be bundled");

        assert!(avatar.source.contains("animations"));
    }

    #[test]
    fn loads_bundled_orc_avatar() {
        let avatars =
            bundled_avatar_assets_for_ruleset(OFFICIAL_RULESET_ID, OFFICIAL_RULESET_VERSION);
        let avatar = avatars
            .iter()
            .find(|asset| asset.id == "orc")
            .expect("orc avatar should be bundled");

        assert_eq!(avatar.path, "assets/orc.eldiron_avatar");
        assert!(avatar.source.contains("\"name\":\"orc\""));
    }

    #[test]
    fn loads_bundled_skeleton_avatar_copy() {
        let avatars =
            bundled_avatar_assets_for_ruleset(OFFICIAL_RULESET_ID, OFFICIAL_RULESET_VERSION);
        let avatar = avatars
            .iter()
            .find(|asset| asset.id == "skeleton")
            .expect("skeleton avatar should be bundled");

        assert_eq!(avatar.path, "assets/skeleton.eldiron_avatar");
        assert!(avatar.source.contains("\"name\":\"skeleton\""));
        assert!(avatar.source.contains("\"animations\""));
        assert_ne!(avatar.source, OFFICIAL_ELDIRON_V1_HUMANOID_AVATAR);
    }

    #[test]
    fn loads_bundled_ruleset_icons() {
        let textures =
            bundled_texture_assets_for_ruleset(OFFICIAL_RULESET_ID, OFFICIAL_RULESET_VERSION);

        for id in [
            "walk",
            "look",
            "basic_attack",
            "training_sword",
            "small_bag",
        ] {
            let texture = textures
                .iter()
                .find(|asset| asset.id == id)
                .unwrap_or_else(|| panic!("{id} icon should be bundled"));
            assert!(!texture.source.is_empty());
        }
    }

    #[test]
    fn loads_bundled_ruleset_tiles() {
        let tiles = bundled_tile_assets_for_ruleset(OFFICIAL_RULESET_ID, OFFICIAL_RULESET_VERSION);
        let off_tile = tiles
            .iter()
            .find(|asset| asset.id == "torch_off")
            .expect("torch off tile should be bundled");
        let on_tile = tiles
            .iter()
            .find(|asset| asset.id == "torch_on")
            .expect("torch on tile should be bundled");

        assert!(
            off_tile
                .source
                .contains("05ab6adc-1631-4ed2-9857-f85820a7f1ad")
        );
        assert!(
            on_tile
                .source
                .contains("f76473d1-70f6-4649-8b0d-cbac627f93d8")
        );
    }

    #[test]
    fn applies_project_overrides_after_official_base() {
        let rules = resolve_project_rules(
            "",
            r#"
            [combat]
            incoming_damage = "value - defender.armor.ARMOR"

            [actions.minor_heal]
            cost = { FOCUS = 3 }
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
            resolve_action(&table, "minor_heal")
                .unwrap()
                .and_then(|action| action.resource_cost("FOCUS")),
            Some(3.0)
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
                .get("unlocks")
                .and_then(Value::as_table)
                .and_then(|unlocks| unlocks.get("level_2"))
                .and_then(Value::as_table)
                .and_then(|unlock| unlock.get("abilities"))
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
                .and_then(Value::as_str)
                .is_some()
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
    fn progression_validation_supports_formula_tables_but_enforces_the_level_cap() {
        let formula = parse_ruleset_table(
            r#"
            [progression.level]
            max_level = 30
            xp_for_level = "level * level * 100"
            "#,
        )
        .unwrap();
        let formula_report = validate_ruleset(&formula);
        assert!(formula_report.is_ok());
        assert_eq!(
            formula_report.warning_count(),
            0,
            "{:?}",
            formula_report.issues
        );

        let above_cap = parse_ruleset_table(
            r#"
            [progression.level]
            max_level = 2

            [progression.xp_table]
            level_2 = 100
            level_3 = 250
            "#,
        )
        .unwrap();
        assert!(validate_ruleset(&above_cap).issues.iter().any(|issue| {
            issue.path == "progression.xp_table.level_3" && issue.message.contains("exceeds")
        }));
    }

    #[test]
    fn identity_defaults_are_optional_typed_and_reference_validated() {
        let root = parse_ruleset_table(
            r#"
            [identity.defaults]
            race = "Automaton"
            class = "Artificer"

            [races.Automaton]
            [classes.Artificer]
            "#,
        )
        .unwrap();
        let identity = resolve_identity_defaults(&root).unwrap();
        assert_eq!(identity.race.as_deref(), Some("Automaton"));
        assert_eq!(identity.class.as_deref(), Some("Artificer"));
        assert!(validate_ruleset(&root).is_ok());

        let classless = parse_ruleset_table("[actions.inspect]\nkind = \"interaction\"").unwrap();
        assert_eq!(
            resolve_identity_defaults(&classless).unwrap(),
            ResolvedIdentityDefaults::default()
        );

        let malformed = parse_ruleset_table("[identity.defaults]\nclass = []").expect("valid TOML");
        let report = validate_ruleset(&malformed);
        assert!(report.issues.iter().any(|issue| {
            issue.path == "identity.defaults" && issue.message.contains("must be a string")
        }));
    }

    #[test]
    fn attribute_roles_and_class_resource_gains_are_typed_and_generic() {
        let root = parse_ruleset_table(
            r#"
            [attributes]
            resources = ["VITAL", "VITAL_CAP", "FOCUS", "FOCUS_CAP"]
            progression = ["RANK", "RENOWN"]

            [attributes.roles]
            health = "VITAL"
            max_health = "VITAL_CAP"
            level = "RANK"
            experience = "RENOWN"

            [attributes.defaults]
            VITAL = 8
            VITAL_CAP = 8
            FOCUS = 3
            FOCUS_CAP = 3
            RANK = 1
            RENOWN = 0

            [classes.Seer.progression.level]
            resource_gains = [
                { attribute = "VITAL", maximum_attribute = "VITAL_CAP", per_level = 2 },
                { attribute = "FOCUS", maximum_attribute = "FOCUS_CAP", per_level = 3 },
            ]
            "#,
        )
        .unwrap();

        let roles = resolve_attribute_roles(&root).unwrap();
        assert_eq!(roles.get("HEALTH"), Some("VITAL"));
        assert_eq!(roles.get("max_health"), Some("VITAL_CAP"));
        assert_eq!(roles.get("level"), Some("RANK"));
        assert_eq!(roles.get("experience"), Some("RENOWN"));
        assert_eq!(
            resolve_class_resource_gains(&root, "Seer").unwrap(),
            vec![
                ResolvedClassResourceGain {
                    attribute: "VITAL".into(),
                    maximum_attribute: "VITAL_CAP".into(),
                    per_level: 2,
                },
                ResolvedClassResourceGain {
                    attribute: "FOCUS".into(),
                    maximum_attribute: "FOCUS_CAP".into(),
                    per_level: 3,
                },
            ]
        );
        assert!(validate_ruleset(&root).is_ok());

        let classless = parse_ruleset_table("[actions.inspect]\nkind = \"interaction\"").unwrap();
        assert_eq!(
            resolve_attribute_roles(&classless).unwrap(),
            ResolvedAttributeRoles::default()
        );
        assert!(
            resolve_class_resource_gains(&classless, "anything")
                .unwrap()
                .is_empty()
        );

        let unknown_role = parse_ruleset_table(
            r#"
            [attributes]
            resources = ["VITAL"]
            [attributes.roles]
            health = "MISSING"
            "#,
        )
        .unwrap();
        assert!(validate_ruleset(&unknown_role).issues.iter().any(|issue| {
            issue.path == "attributes.roles.health" && issue.message.contains("does not exist")
        }));

        let legacy_gain = parse_ruleset_table(
            r#"
            [attributes]
            resources = ["HP", "MAX_HP"]
            [classes.Old.progression.level]
            hp_per_level = 4
            "#,
        )
        .unwrap();
        assert!(validate_ruleset(&legacy_gain).issues.iter().any(|issue| {
            issue.path == "classes.Old.progression.level"
                && issue.message.contains("resource_gains")
        }));

        let unknown_gain = parse_ruleset_table(
            r#"
            [attributes]
            resources = ["VITAL"]
            [classes.Broken.progression.level]
            resource_gains = [
                { attribute = "VITAL", maximum_attribute = "MISSING", per_level = 2 },
            ]
            "#,
        )
        .unwrap();
        assert!(validate_ruleset(&unknown_gain).issues.iter().any(|issue| {
            issue.path == "classes.Broken.progression.level.resource_gains.0.maximum_attribute"
                && issue.message.contains("does not exist")
        }));

        let unknown_cost = parse_ruleset_table(
            r#"
            [attributes]
            resources = ["FOCUS"]
            [actions.broken]
            cost = { MISSING = 2 }
            result = { script = "never" }
            "#,
        )
        .unwrap();
        assert!(validate_ruleset(&unknown_cost).issues.iter().any(|issue| {
            issue.path == "actions.broken.cost.MISSING" && issue.message.contains("does not exist")
        }));
    }

    #[test]
    fn equipment_policy_resolves_permissions_slots_and_handedness_generically() {
        let root = parse_ruleset_table(
            r#"
            [equipment]
            weapon_slots = ["main_hand", "off_hand"]
            armor_slots = ["torso", "shield"]

            [equipment.avatar_anchors]
            main_hand = "main_hand"
            off_hand = "off_hand"
            shield = "off_hand"

            [equipment.weapon_categories.staff]
            handed = "two_handed"

            [equipment.weapon_categories.dagger]
            handed = "one_handed"

            [equipment.armor_categories.cloth]

            [equipment.armor_categories.shield]
            occupies_slots = ["off_hand"]

            [classes.Mage]
            allowed_weapons = ["staff"]
            allowed_armor = ["cloth"]
            "#,
        )
        .unwrap();
        let policy = resolve_equipment_policy(&root).unwrap();
        assert_eq!(policy.weapon_slots, vec!["main_hand", "off_hand"]);
        assert_eq!(policy.armor_slots, vec!["torso", "shield"]);
        assert_eq!(
            policy.slots_for_avatar_anchor(ResolvedEquipmentAvatarAnchor::MainHand),
            vec!["main_hand"]
        );
        assert_eq!(
            policy.slots_for_avatar_anchor(ResolvedEquipmentAvatarAnchor::OffHand),
            vec!["off_hand", "shield"]
        );
        assert!(
            policy
                .check_item(Some("Mage"), "weapon", Some("staff"), "main_hand")
                .is_ok()
        );
        assert!(
            policy
                .check_item(Some("Mage"), "weapon", Some("dagger"), "main_hand")
                .is_err()
        );
        assert!(
            policy
                .check_item(None, "weapon", Some("dagger"), "main_hand")
                .is_ok()
        );
        assert_eq!(
            policy.occupied_slots("weapon", Some("staff"), "main_hand"),
            BTreeSet::from(["main_hand".to_string(), "off_hand".to_string()])
        );
        assert_eq!(
            policy.occupied_slots("armor", Some("shield"), "shield"),
            BTreeSet::from(["off_hand".to_string(), "shield".to_string()])
        );

        let invalid = parse_ruleset_table(
            r#"
            [equipment]
            armor_slots = ["shield"]
            [equipment.armor_categories.shield]
            occupies_slots = ["missing_hand"]
            "#,
        )
        .unwrap();
        assert!(resolve_equipment_policy(&invalid).is_err());
        assert!(
            validate_ruleset(&invalid)
                .issues
                .iter()
                .any(|issue| issue.path == "equipment")
        );

        let duplicate_slots = parse_ruleset_table(
            r#"
            [equipment]
            weapon_slots = ["grip", "GRIP"]
            "#,
        )
        .unwrap();
        assert!(
            resolve_equipment_policy(&duplicate_slots)
                .unwrap_err()
                .contains("duplicate")
        );

        let invalid_anchor = parse_ruleset_table(
            r#"
            [equipment]
            weapon_slots = ["grip"]
            [equipment.avatar_anchors]
            grip = "tentacle"
            "#,
        )
        .unwrap();
        assert!(
            resolve_equipment_policy(&invalid_anchor)
                .unwrap_err()
                .contains("invalid avatar anchor")
        );

        let conflicting_loadout = parse_ruleset_table(
            r#"
            [equipment]
            weapon_slots = ["main_hand", "off_hand"]
            armor_slots = ["shield"]

            [equipment.weapon_categories.bow]
            handed = "two_handed"

            [equipment.armor_categories.shield]
            occupies_slots = ["off_hand"]

            [items.weapons.bow]
            category = "bow"
            slot = "main_hand"
            [items.weapons.bow.damage]
            roll = "1d1"

            [items.armor.shield]
            category = "shield"
            slot = "shield"

            [classes.Warrior]
            allowed_weapons = ["bow"]
            allowed_armor = ["shield"]
            [classes.Warrior.starting_loadout]
            weapons = ["bow"]
            armor = ["shield"]
            "#,
        )
        .unwrap();
        assert!(
            validate_ruleset(&conflicting_loadout)
                .issues
                .iter()
                .any(|issue| {
                    issue.path == "classes.Warrior.starting_loadout"
                        && issue.message.contains("off_hand")
                })
        );
    }

    #[test]
    fn reads_official_economy_table() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let economy = table
            .get("economy")
            .and_then(Value::as_table)
            .expect("economy table");
        assert_eq!(economy.get("base").and_then(Value::as_str), Some("copper"));
        let currencies = economy
            .get("currencies")
            .and_then(Value::as_table)
            .expect("currency table");
        assert_eq!(
            economy
                .get("starting_wealth")
                .and_then(Value::as_table)
                .and_then(|wealth| wealth.get("player"))
                .and_then(Value::as_integer),
            Some(50)
        );
        assert_eq!(
            currencies
                .get("gold")
                .and_then(Value::as_table)
                .and_then(|gold| gold.get("value"))
                .and_then(Value::as_integer),
            Some(100)
        );
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
            cleric.level_unlocks.get("level_4").is_some_and(|unlocks| {
                unlocks.iter().any(|entry| entry == "spells:turn_undead")
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
        assert!(
            cleric
                .starting_loadout
                .get("armor")
                .is_some_and(|armor| armor.iter().any(|item| item == "cleric_vestments"))
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
    fn official_adventuring_classes_have_complete_level_ten_cadence() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let max_level = ruleset_table_at_path(&table, &["progression", "level"])
            .and_then(|level| level.get("max_level"))
            .and_then(Value::as_integer);
        assert_eq!(max_level, Some(10));
        for level in 2..=10 {
            assert!(
                ruleset_xp_for_level(&table, level).is_some(),
                "missing XP for level {level}"
            );
        }

        let expected_levels = BTreeSet::from([
            "level_1", "level_2", "level_4", "level_6", "level_8", "level_10",
        ]);
        let actions = resolve_actions(&table).unwrap();
        for class_id in ["Warrior", "Cleric", "Ranger"] {
            let class = ruleset_table_at_path(&table, &["classes", class_id]).unwrap();
            assert!(!class.contains_key("abilities"));
            assert!(!class.contains_key("spells"));
            let loadout = class
                .get("starting_loadout")
                .and_then(Value::as_table)
                .unwrap();
            assert!(!loadout.contains_key("abilities"));
            assert!(!loadout.contains_key("spells"));

            let unlocks = class.get("unlocks").and_then(Value::as_table).unwrap();
            assert_eq!(
                unlocks.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                expected_levels
            );
            for unlock in unlocks.values().filter_map(Value::as_table) {
                for ability in table_string_array(unlock, "abilities") {
                    assert!(
                        actions
                            .values()
                            .any(|action| action.required_ability() == Some(ability.as_str()))
                    );
                }
                for spell in table_string_array(unlock, "spells") {
                    assert!(
                        actions
                            .values()
                            .any(|action| action.required_spell() == Some(spell.as_str()))
                    );
                }
            }
        }
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
                && template.data.contains("icon = \"training_sword\"")
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
                && template.data.contains("quality = 100")
                && template.data.contains("condition = 100")
                && template.data.contains("damage_roll = \"1d6\"")
                && template.data.contains("damage_bonus_attribute = \"DEX\"")
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
                && template.data.contains("icon = \"gather_wood\"")
                && template.data.contains("icon_color = \"#9a6a3a\"")
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
        assert!(templates.iter().any(|template| {
            template.id == "torch"
                && template.kind == "tool"
                && template.ruleset_path == "items.tools.torch"
                && template.source.contains("set_emit_light(value)")
                && template.source.contains("set_tile(\"f76473d1")
                && template.data.contains("tile_id = \"05ab6adc")
                && template.data.contains("[light]")
                && template.data.contains("range = 3.8")
                && template.data.contains("on_look_on = \"A lit torch")
                && template.authoring.contains("[state.on]")
        }));
        assert!(
            templates
                .iter()
                .any(|template| template.id == "padded_armor")
        );
        assert!(templates.iter().any(|template| {
            template.id == "cleric_vestments"
                && template.kind == "armor"
                && template.ruleset_path == "items.armor.cleric_vestments"
                && template.data.contains("color_index = 25")
                && template.data.contains("torso_index = 25")
                && template.data.contains("arms_index = 25")
        }));
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
        assert!(templates.iter().any(|template| {
            template.id == "small_bag"
                && template.kind == "container"
                && template.ruleset_path == "items.containers.small_bag"
                && template.data.contains("icon = \"small_bag\"")
                && template.data.contains("container = true")
                && template.data.contains("container_slots = 6")
                && template.data.contains("container_template = \"bag_small\"")
                && template.data.contains("visual_template = \"bag_pouch\"")
                && template.data.contains("visual_template_pixels")
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
    fn official_orc_skin_uses_ruleset_palette_indices() {
        let rules = parse_ruleset_table(latest_official_ruleset()).unwrap();
        let attributes = ruleset_table_at_path(&rules, &["races", "Orc", "attributes"])
            .expect("official Orc attributes");

        assert_eq!(
            attributes
                .get("light_skin_index")
                .and_then(Value::as_integer),
            Some(16)
        );
        assert_eq!(
            attributes
                .get("dark_skin_index")
                .and_then(Value::as_integer),
            Some(18)
        );
        assert_eq!(
            attributes.get("hands_index").and_then(Value::as_integer),
            Some(16)
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
        assert!(catalog.resources.iter().any(|id| id == "bird_nest_node"));
        assert!(catalog.resources.iter().any(|id| id == "wild_herb_node"));
        assert!(catalog.resources.iter().any(|id| id == "green_wood_node"));
        assert!(catalog.recipes.iter().any(|id| id == "wooden_arrows"));
        assert!(catalog.recipes.iter().any(|id| id == "blessed_herb"));
        assert!(catalog.actions.iter().any(|id| id == "basic_attack"));
        assert!(catalog.actions.iter().any(|id| id == "gather_feathers"));
        assert!(catalog.actions.iter().any(|id| id == "gather_herbs"));
        assert!(catalog.actions.iter().any(|id| id == "gather_wood"));
        assert!(catalog.actions.iter().any(|id| id == "guard"));
        assert!(catalog.conditions.iter().any(|id| id == "guarded"));
        assert!(catalog.actions.iter().any(|id| id == "holy_light"));
        assert!(
            catalog
                .invocation_schemes
                .iter()
                .any(|id| id == "words_of_power")
        );
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
        assert!(
            catalog
                .item_templates
                .iter()
                .any(|path| path == "items.resources.green_wood_node")
        );
        assert!(
            catalog
                .item_templates
                .iter()
                .any(|path| path == "items.resources.bird_nest_node")
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
    fn official_spells_have_words_and_a_connected_ritual_economy() {
        let table = latest_official_ruleset().parse::<Table>().unwrap();
        let actions = resolve_action_catalogue(&table).unwrap();
        let expected = [
            ("minor_heal", "LO VI", &["blessed_herb", "moonwater"][..]),
            ("holy_light", "YA FUL", &["consecrated_oil"][..]),
            ("blessing", "YA", &["consecrated_oil"][..]),
            ("turn_undead", "SAR IR", &["warding_salt"][..]),
            ("greater_heal", "LO LO", &["blessed_herb", "moonwater"][..]),
            ("smite", "FUL YA", &["consecrated_oil", "ember_bead"][..]),
            ("sanctuary", "SAR VI", &["warding_salt", "moonwater"][..]),
            ("fire_spark", "FUL", &["ember_bead"][..]),
        ];

        for (action_id, phrase, reagent_ids) in expected {
            let action = actions.action(action_id).unwrap();
            assert_eq!(
                actions
                    .action_for_invocation("words_of_power", phrase)
                    .map(|bound| bound.id.as_str()),
                Some(action_id)
            );
            for reagent_id in reagent_ids {
                assert!(
                    action
                        .item_costs
                        .iter()
                        .any(|cost| cost.item == *reagent_id),
                    "{action_id} should consume {reagent_id}"
                );
            }
        }

        for item_path in [
            &["items", "materials", "moonleaf"][..],
            &["items", "materials", "sun_shard"][..],
            &["items", "materials", "grave_dust"][..],
            &["items", "materials", "ember_resin"][..],
            &["items", "reagents", "moonwater"][..],
            &["items", "reagents", "consecrated_oil"][..],
            &["items", "reagents", "warding_salt"][..],
            &["items", "reagents", "ember_bead"][..],
            &["items", "armor", "ritual_censer"][..],
            &["items", "armor", "sunward_charm"][..],
        ] {
            assert!(
                ruleset_value_at_path(&table, item_path).is_some(),
                "{} should exist",
                item_path.join(".")
            );
        }
        for recipe_id in [
            "moonwater",
            "consecrated_oil",
            "warding_salt",
            "ember_beads",
            "ritual_censer",
            "sunward_charm",
        ] {
            assert!(
                ruleset_value_at_path(&table, &["recipes", recipe_id]).is_some(),
                "recipe {recipe_id} should exist"
            );
        }
    }

    #[test]
    fn crafting_skill_gain_policy_rejects_invalid_values() {
        let table = r#"
            [crafting.skill_gain]
            enabled = "sometimes"
            per_success = -1
            below_recommended_bonus = -2
            mastery_margin = -3
        "#
        .parse::<Table>()
        .unwrap();
        let report = validate_ruleset(&table);

        for path in [
            "crafting.skill_gain.enabled",
            "crafting.skill_gain.per_success",
            "crafting.skill_gain.below_recommended_bonus",
            "crafting.skill_gain.mastery_margin",
        ] {
            assert!(
                report.issues.iter().any(|issue| issue.path == path),
                "{path} should be rejected"
            );
        }
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
    fn resolves_typed_conditions_and_condition_action_effects() {
        let root = parse_ruleset_table(
            r#"
            [conditions.poisoned]
            name = "Poisoned"
            duration = 6
            stacking = "stack"
            max_stacks = 3
            tags = ["harmful", "poison"]
            immune_traits = ["undead"]
            modifiers = [
                { attribute = "SPEED", add = -1, multiply = 0.9, minimum = 1 },
            ]

            [conditions.poisoned.periodic]
            interval = 2
            initial_delay = 1
            effects = [
                { damage = 2, damage_kind = "poison" },
                { resource = "STAMINA", add = -1, minimum = 0 },
            ]

            [conditions.poisoned.fx.active]
            preset = "poison_motes"

            [conditions.poisoned.fx.tick]
            preset = "poison_burst"

            [combat.kinds.poison]
            description = "Toxic damage."

            [fx.presets.poison_motes]
            kind = "particles"

            [fx.presets.poison_burst]
            kind = "particles"

            [actions.poison_dart]
            kind = "attack"
            target = "hostile_entity"
            result = { apply_condition = { id = "poisoned", recipient = "target" } }

            [actions.antidote]
            kind = "interaction"
            target = "self"
            result = { remove_condition = "poisoned" }
            "#,
        )
        .unwrap();

        let condition = resolve_condition(&root, "poisoned").unwrap().unwrap();
        assert_eq!(condition.duration_seconds, 6.0);
        assert_eq!(condition.stacking, ResolvedConditionStacking::Stack);
        assert_eq!(condition.max_stacks, 3);
        assert_eq!(condition.immune_traits, ["undead"]);
        assert_eq!(condition.modifiers[0].attribute, "SPEED");
        assert_eq!(condition.modifiers[0].add, -1.0);
        assert_eq!(condition.modifiers[0].multiply, 0.9);
        assert_eq!(condition.modifiers[0].minimum, Some(1.0));
        assert_eq!(condition.modifiers[0].maximum, None);
        let periodic = condition.periodic.as_ref().unwrap();
        assert_eq!(periodic.interval_seconds, 2.0);
        assert_eq!(periodic.initial_delay_seconds, 1.0);
        assert!(matches!(
            periodic.effects.as_slice(),
            [
                ResolvedConditionPeriodicEffect::Damage {
                    amount: 2.0,
                    damage_kind,
                },
                ResolvedConditionPeriodicEffect::Modify {
                    field: ResolvedActionModificationField::Resource(resource),
                    add: -1.0,
                    minimum: Some(0.0),
                    maximum: None,
                },
            ] if damage_kind == "poison" && resource == "STAMINA"
        ));

        let poison_dart = resolve_action(&root, "poison_dart").unwrap().unwrap();
        assert!(poison_dart.has_condition_effects());
        assert!(matches!(
            poison_dart.effects.as_slice(),
            [ResolvedActionEffect::ApplyCondition {
                condition,
                recipient: ResolvedActionEffectRecipient::Target,
            }] if condition == "poisoned"
        ));
        assert!(validate_ruleset(&root).is_ok());
    }

    #[test]
    fn condition_validation_rejects_invalid_stacking_and_unknown_references() {
        let invalid = parse_ruleset_table(
            r#"
            [conditions.invalid]
            duration = -1
            stacking = "refresh"
            max_stacks = 2

            [actions.use_missing]
            kind = "interaction"
            target = "self"
            result = { apply_condition = "missing" }
            "#,
        )
        .unwrap();
        let report = validate_ruleset(&invalid);
        assert!(!report.is_ok());
        assert!(report.issues.iter().any(|issue| issue.path == "conditions"));
        assert!(report.issues.iter().any(|issue| {
            issue.path == "actions.use_missing.result.condition"
                && issue.message.contains("missing")
        }));
    }

    #[test]
    fn condition_validation_rejects_invalid_periodic_and_fx_references() {
        let invalid_periodic = parse_ruleset_table(
            r#"
            [conditions.poisoned]
            duration = 2

            [conditions.poisoned.periodic]
            interval = 0
            effects = [{ damage = 1, healing = 1 }]
            "#,
        )
        .unwrap();
        assert!(resolve_condition(&invalid_periodic, "poisoned").is_err());

        let invalid_references = parse_ruleset_table(
            r#"
            [conditions.poisoned]
            duration = 2

            [conditions.poisoned.periodic]
            interval = 1
            effects = [{ damage = 1, damage_kind = "missing" }]

            [conditions.poisoned.fx.tick]
            preset = "missing"
            "#,
        )
        .unwrap();
        let report = validate_ruleset(&invalid_references);
        assert!(!report.is_ok());
        assert!(report.issues.iter().any(|issue| {
            issue.path == "conditions.poisoned.periodic.effects.0.damage_kind"
                && issue.message.contains("missing")
        }));
        assert!(report.issues.iter().any(|issue| {
            issue.path == "conditions.poisoned.fx.tick.preset" && issue.message.contains("missing")
        }));
    }

    #[test]
    fn condition_validation_rejects_invalid_modifier_operations() {
        for modifier in [
            r#"{ attribute = "SPEED" }"#,
            r#"{ attribute = "SPEED", multiply = -1 }"#,
            r#"{ attribute = "SPEED", minimum = 10, maximum = 2 }"#,
        ] {
            let root = parse_ruleset_table(&format!(
                r#"
                [conditions.invalid]
                modifiers = [{}]
                "#,
                modifier
            ))
            .unwrap();
            assert!(resolve_condition(&root, "invalid").is_err());
        }
    }

    #[test]
    fn resolves_and_validates_derived_stat_dependencies() {
        let root = parse_ruleset_table(
            r#"
            [derived_stats.POWER]
            formula = "base + floor(max(0, WIS - 10) / 4)"
            minimum = 0

            [derived_stats.SPELL_DC]
            formula = "10 + POWER + floor(level / 2)"
            maximum = 30
            "#,
        )
        .unwrap();
        let stats = resolve_derived_stats(&root).unwrap();
        assert_eq!(stats["POWER"].minimum, Some(0.0));
        assert_eq!(
            stats["SPELL_DC"].dependencies,
            BTreeSet::from(["POWER".into(), "level".into()])
        );
        assert!(validate_ruleset(&root).is_ok());

        let cycle = parse_ruleset_table(
            r#"
            [derived_stats.A]
            formula = "b + 1"
            [derived_stats.B]
            formula = "A + 1"
            "#,
        )
        .unwrap();
        let report = validate_ruleset(&cycle);
        assert!(report.issues.iter().any(|issue| {
            issue.path.starts_with("derived_stats.") && issue.message.contains("dependency cycle")
        }));
    }

    #[test]
    fn semantic_icon_fallbacks_remove_per_definition_asset_requirements() {
        let root = parse_ruleset_table(
            r#"
            [icons.sword]
            [icons.hand]

            [ui.action_icon_fallbacks]
            attack = "sword"
            default = "hand"

            [ui.item_icon_fallbacks]
            material = "hand"
            default = "hand"

            [actions.strike]
            kind = "attack"
            target = "any_entity"
            result = { damage = "weapon" }
            "#,
        )
        .unwrap();
        let action = resolve_action(&root, "strike").unwrap().unwrap();
        assert_eq!(
            resolve_action_icon(&root, &action).as_deref(),
            Some("sword")
        );
        assert_eq!(
            resolve_item_icon(&root, Some("material"), None).as_deref(),
            Some("hand")
        );
        assert!(validate_ruleset(&root).is_ok());

        let invalid = parse_ruleset_table(
            r#"
            [icons.hand]
            [ui.action_icon_fallbacks]
            default = "missing"
            "#,
        )
        .unwrap();
        assert!(validate_ruleset(&invalid).issues.iter().any(|issue| {
            issue.path == "ui.action_icon_fallbacks.default" && issue.message.contains("missing")
        }));
    }

    #[test]
    fn semantic_fx_fallbacks_are_data_driven_and_validated() {
        let root = parse_ruleset_table(
            r#"
            [fx.presets.spark]
            [fx.presets.aura]

            [fx.action_fallbacks.attack]
            impact = "spark"

            [fx.action_fallbacks.condition]
            cast = "aura"

            [fx.condition_fallbacks]
            apply = "spark"
            active = "aura"

            [actions.strike]
            kind = "attack"
            target = "any_entity"
            result = { damage = "weapon" }

            [actions.guard]
            kind = "stance"
            target = "self"
            result = { apply_condition = "guarded" }

            [conditions.guarded]
            duration = 2
            "#,
        )
        .unwrap();
        let strike = resolve_action(&root, "strike").unwrap().unwrap();
        let guard = resolve_action(&root, "guard").unwrap().unwrap();
        assert_eq!(
            resolve_action_fx_fallback(&root, &strike, "impact").as_deref(),
            Some("spark")
        );
        assert_eq!(
            resolve_action_fx_fallback(&root, &guard, "cast").as_deref(),
            Some("aura")
        );
        assert_eq!(
            resolve_condition_fx_fallback(&root, "active").as_deref(),
            Some("aura")
        );
        assert!(validate_ruleset(&root).is_ok());

        let invalid = parse_ruleset_table(
            r#"
            [fx.presets.spark]
            [fx.action_fallbacks.default]
            impact = "missing"
            "#,
        )
        .unwrap();
        assert!(validate_ruleset(&invalid).issues.iter().any(|issue| {
            issue.path == "fx.action_fallbacks.default.impact" && issue.message.contains("missing")
        }));
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
