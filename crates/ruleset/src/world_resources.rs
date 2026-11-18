//! Persistent world sources. Definitions specify outputs; map bindings can only
//! override fields explicitly exposed by the profile. All durations use the
//! host's monotonic simulation clock in milliseconds, never wall-clock time.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use toml::{Table, Value};

mod state;
pub use state::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub item: String,
    pub quantity: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockUnit {
    Count,
    Volume,
    Opportunity,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub amount: u32,
    pub interval_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    pub unit: StockUnit,
    /// Omitted capacity explicitly means unlimited; initial must then be absent.
    pub capacity: Option<u32>,
    pub initial: Option<u32>,
    pub recovery: Option<Recovery>,
    pub outputs: Vec<Output>,
    pub healthy_at: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityPolicy {
    Whole,
    Clamped,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interaction {
    pub supply: String,
    pub quantity: u32,
    pub policy: QuantityPolicy,
    pub range: f32,
    pub tool: Option<String>,
    #[serde(default)]
    pub tool_wear: u32,
    pub skill: Option<String>,
    #[serde(default)]
    pub minimum_skill: u32,
    #[serde(default)]
    pub skill_gain: u32,
    pub transition: Option<String>,
    #[serde(default)]
    pub discard_supplies: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exploitation {
    pub supply: String,
    pub empty_count: u32,
    pub window_ms: u64,
    pub suspension_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceProfile {
    pub name: String,
    pub default_interaction: String,
    #[serde(default = "active")]
    pub initial_phase: String,
    pub expires_after_ms: Option<u64>,
    #[serde(default)]
    pub spent_when_empty: bool,
    #[serde(default)]
    pub allow_overrides: Vec<String>,
    pub exploitation: Option<Exploitation>,
    pub supplies: BTreeMap<String, Supply>,
    pub interactions: BTreeMap<String, Interaction>,
}
fn active() -> String {
    "active".into()
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyOverride {
    pub initial: Option<u32>,
    pub capacity: Option<u32>,
    pub recovery_interval_ms: Option<u64>,
    pub recovery_amount: Option<u32>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceBinding {
    pub profile: String,
    /// Named profile interaction used by this access point's Use action.
    pub interaction: Option<String>,
    /// Optional explicit logical placement UUID. Omit for independent copies.
    pub instance: Option<String>,
    /// A ruleset pool owns stock across all its access points and regions.
    pub pool: Option<String>,
    #[serde(default)]
    pub visual_tiles: BTreeMap<String, String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub supplies: BTreeMap<String, SupplyOverride>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcePool {
    pub profile: String,
    #[serde(default)]
    pub supplies: BTreeMap<String, SupplyOverride>,
}

pub fn profiles(root: &Table) -> Result<BTreeMap<String, ResourceProfile>, String> {
    root.get("resource_profiles")
        .map(|v| {
            v.clone()
                .try_into()
                .map_err(|e: toml::de::Error| e.to_string())
        })
        .unwrap_or_else(|| Ok(BTreeMap::new()))
}
pub fn binding(source: &str) -> Result<Option<ResourceBinding>, String> {
    let table: Table = match source.parse() {
        Ok(table) => table,
        Err(error) => {
            let declares_resource = source.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("[resource")
                    || line.starts_with("resource =")
                    || line.starts_with("\"resource\" =")
            });
            if declares_resource {
                return Err(format!("Invalid resource TOML: {error}"));
            }
            return Ok(None);
        }
    };
    table
        .get("resource")
        .map(|v| {
            v.clone()
                .try_into()
                .map_err(|e: toml::de::Error| e.to_string())
        })
        .transpose()
}
pub fn resolve(root: &Table, binding: &ResourceBinding) -> Result<ResourceProfile, String> {
    let mut profile = profiles(root)?
        .remove(&binding.profile)
        .ok_or_else(|| format!("Unknown resource profile '{}'", binding.profile))?;
    if binding.interaction.as_ref().is_some_and(|id| !profile.interactions.contains_key(id)) {
        return Err("Resource access interaction is not declared by its profile".into());
    }
    if let Some(instance) = &binding.instance {
        let id = theframework_core::Uuid::parse_str(instance)
            .map_err(|_| "Resource instance must be a UUID")?;
        if id.is_nil() {
            return Err("Resource instance cannot be nil".into());
        }
    }
    if !binding.visual_tiles.is_empty() && !binding.visual_tiles.contains_key("healthy") {
        return Err("Resource visual tiles require a healthy fallback".into());
    }
    for tile in binding.visual_tiles.values() {
        if theframework_core::Uuid::parse_str(tile).is_err() {
            return Err("Resource visual tiles must reference tile UUIDs".into());
        }
    }
    if let Some(pool) = &binding.pool {
        if binding.instance.is_some() || !binding.supplies.is_empty() {
            return Err(
                "A pool access point cannot override stock or declare a separate instance".into(),
            );
        }
        let pool: ResourcePool = root
            .get("resource_pools")
            .and_then(|v| v.get(pool))
            .ok_or("Unknown resource pool")?
            .clone()
            .try_into()
            .map_err(|e: toml::de::Error| e.to_string())?;
        if pool.profile != binding.profile {
            return Err("Pool and access point profiles disagree".into());
        }
        apply_overrides(&mut profile, &pool.supplies)?;
    } else {
        apply_overrides(&mut profile, &binding.supplies)?;
    }
    validate_profile(root, &profile)?;
    Ok(profile)
}
fn apply_overrides(
    profile: &mut ResourceProfile,
    overrides: &BTreeMap<String, SupplyOverride>,
) -> Result<(), String> {
    for (name, values) in overrides {
        for (field, present) in [
            ("initial", values.initial.is_some()),
            ("capacity", values.capacity.is_some()),
            (
                "recovery_interval_ms",
                values.recovery_interval_ms.is_some(),
            ),
            ("recovery_amount", values.recovery_amount.is_some()),
        ] {
            if present && !profile.allow_overrides.contains(&format!("{name}.{field}")) {
                return Err(format!("Profile does not allow overriding {name}.{field}"));
            }
        }
        let supply = profile
            .supplies
            .get_mut(name)
            .ok_or_else(|| format!("Unknown supply '{name}'"))?;
        if let Some(value) = values.initial {
            supply.initial = Some(value);
        }
        if let Some(value) = values.capacity {
            supply.capacity = Some(value);
        }
        if let Some(value) = values.recovery_interval_ms {
            supply
                .recovery
                .as_mut()
                .ok_or("Supply has no recovery")?
                .interval_ms = value;
        }
        if let Some(value) = values.recovery_amount {
            supply
                .recovery
                .as_mut()
                .ok_or("Supply has no recovery")?
                .amount = value;
        }
    }
    Ok(())
}
fn validate_profile(root: &Table, profile: &ResourceProfile) -> Result<(), String> {
    let catalogue = crate::catalogue::Catalogue::from_table(root);
    if profile.name.trim().is_empty()
        || profile.initial_phase.trim().is_empty()
        || profile.supplies.is_empty()
        || profile.interactions.is_empty()
    {
        return Err("Resource needs a name, initial phase, supplies and interactions".into());
    }
    if !profile
        .interactions
        .contains_key(&profile.default_interaction)
    {
        return Err("Unknown default resource interaction".into());
    }
    if matches!(
        profile.initial_phase.as_str(),
        "expired" | "spent" | "damaged"
    ) {
        return Err("Initial resource phase cannot be a reserved terminal/suspended phase".into());
    }
    if profile.expires_after_ms == Some(0) {
        return Err("Resource expiry must be positive".into());
    }
    for (name, supply) in &profile.supplies {
        match supply.capacity {
            Some(0) => return Err(format!("{name}: capacity must be positive")),
            Some(capacity) if supply.initial.unwrap_or(capacity) > capacity => {
                return Err(format!("{name}: initial exceeds capacity"));
            }
            None if supply.initial.is_some() || supply.recovery.is_some() => {
                return Err(format!(
                    "{name}: unlimited supply cannot declare initial stock or recovery"
                ));
            }
            _ => {}
        }
        if supply
            .recovery
            .as_ref()
            .is_some_and(|r| r.amount == 0 || r.interval_ms == 0)
        {
            return Err(format!("{name}: recovery values must be positive"));
        }
        if supply
            .healthy_at
            .is_some_and(|n| n == 0 || supply.capacity.is_none_or(|c| n > c))
        {
            return Err(format!("{name}: invalid appearance threshold"));
        }
        if supply.outputs.is_empty() {
            return Err(format!("{name}: outputs are required"));
        }
        for output in &supply.outputs {
            if output.quantity == 0
                || catalogue
                    .resolve(&crate::catalogue::DefinitionKey {
                        domain: "items".into(),
                        id: output.item.clone(),
                    })?
                    .is_none()
            {
                return Err(format!(
                    "{name}: invalid registered output '{}'",
                    output.item
                ));
            }
            let item = root
                .get("items")
                .and_then(Value::as_table)
                .and_then(|groups| groups.values().find_map(|g| g.get(&output.item)));
            let liquid = item
                .and_then(|i| i.get("quantity_unit"))
                .and_then(Value::as_str)
                == Some("volume");
            if (supply.unit == StockUnit::Volume) != liquid {
                return Err(format!(
                    "{name}: volume output must reference a volume item; count/opportunity outputs must be solid items"
                ));
            }
        }
    }
    for interaction in profile.interactions.values() {
        let supply = profile
            .supplies
            .get(&interaction.supply)
            .ok_or("Interaction references an unknown supply")?;
        if interaction.quantity > 1_000_000
            || interaction.tool_wear > i32::MAX as u32
            || interaction.minimum_skill > i32::MAX as u32
            || interaction.skill_gain > i32::MAX as u32
        {
            return Err("Interaction values exceed the supported operation bounds".into());
        }
        if interaction.quantity == 0 || !interaction.range.is_finite() || interaction.range <= 0.0 {
            return Err("Interaction needs a positive bounded quantity and finite range".into());
        }
        if supply.unit == StockUnit::Volume
            && (interaction.policy != QuantityPolicy::Clamped
                || supply.outputs.len() != 1
                || supply.outputs[0].quantity != 1)
        {
            return Err("Volume collection must use clamped 1:1 transfer into a vessel".into());
        }
        if interaction.tool_wear > 0 && interaction.tool.is_none() {
            return Err("Tool wear requires an explicit tool capability".into());
        }
        if interaction.skill_gain > 0 && interaction.skill.is_none() {
            return Err("Advancement requires an explicit skill".into());
        }
        if interaction
            .skill
            .as_ref()
            .is_some_and(|s| !catalogue.ids("skills").contains(s))
        {
            return Err("Interaction references an unknown skill".into());
        }
        if interaction.discard_supplies
            && interaction.transition.as_deref() == Some(&profile.initial_phase)
        {
            return Err("Discarding supplies requires a distinct lifecycle phase".into());
        }
        if interaction.discard_supplies && interaction.transition.is_none() {
            return Err("Discarding supplies requires a lifecycle transition".into());
        }
    }
    if let Some(e) = &profile.exploitation {
        if !profile.supplies.contains_key(&e.supply)
            || e.empty_count == 0
            || e.window_ms == 0
            || e.suspension_ms == 0
        {
            return Err("Invalid exploitation policy".into());
        }
    }
    Ok(())
}
pub(crate) fn validate(root: &Table, report: &mut crate::RulesetValidationReport) {
    match profiles(root) {
        Err(error) => report.error("resource_profiles", error),
        Ok(profiles) => {
            for (id, profile) in profiles {
                if let Err(error) = validate_profile(root, &profile) {
                    report.error(format!("resource_profiles.{id}"), error);
                }
            }
        }
    }
    if let Some(pools) = root.get("resource_pools") {
        let Some(pools) = pools.as_table() else {
            report.error("resource_pools", "Pools must be a table");
            return;
        };
        for (id, value) in pools {
            let result = value
                .clone()
                .try_into::<ResourcePool>()
                .map_err(|e| e.to_string())
                .and_then(|pool| {
                    resolve(
                        root,
                        &ResourceBinding {
                            profile: pool.profile,
                            pool: Some(id.clone()),
                            ..Default::default()
                        },
                    )
                    .map(|_| ())
                });
            if let Err(error) = result {
                report.error(format!("resource_pools.{id}"), error);
            }
        }
    }
}

/// Bind through the shared validator while preserving unrelated authoring data.
/// Initial values are only defaults for new world instances, never a refill.
pub fn bind_source(root: &Table, source: &str, binding: ResourceBinding) -> Result<String, String> {
    resolve(root, &binding)?;
    let mut table: Table = source.parse().map_err(|e: toml::de::Error| e.to_string())?;
    if table.contains_key("resource") {
        return Err(
            "This placement already has a resource binding; edit its TOML explicitly".into(),
        );
    }
    table.insert(
        "resource".into(),
        Value::try_from(binding).map_err(|e| e.to_string())?,
    );
    toml::to_string(&table).map_err(|e| e.to_string())
}
