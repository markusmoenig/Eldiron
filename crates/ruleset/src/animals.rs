//! Placeable fauna use ordinary character identities, behavior graphs and world stock.
use serde::Deserialize;
use std::collections::BTreeMap;
use toml::Table;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Animal {
    pub name: String,
    pub description: String,
    pub race: String,
    pub class: String,
    pub roam_distance: f32,
    pub roam_speed: f32,
    pub pause_minutes: u32,
}

pub fn resolve(root: &Table) -> Result<BTreeMap<String, Animal>, String> {
    let Some(definitions) = root.get("animals") else {
        return Ok(BTreeMap::new());
    };
    let mut animals = BTreeMap::new();
    for (id, value) in definitions.as_table().ok_or("animals must be a table")? {
        let animal: Animal = value
            .clone()
            .try_into()
            .map_err(|e| format!("animals.{id}: {e}"))?;
        if id.trim().is_empty()
            || id.len() > 128
            || animal.name.trim().is_empty()
            || !animal.roam_distance.is_finite()
            || !(0.1..=10.).contains(&animal.roam_distance)
            || !animal.roam_speed.is_finite()
            || !(0.1..=10.).contains(&animal.roam_speed)
            || animal.pause_minutes > 60
        {
            return Err(format!(
                "animals.{id}: invalid identity or roaming parameters"
            ));
        }
        let race = root
            .get("races")
            .and_then(|v| v.get(&animal.race))
            .ok_or_else(|| format!("animals.{id}: unknown race '{}'", animal.race))?;
        if root
            .get("classes")
            .and_then(|v| v.get(&animal.class))
            .is_none()
        {
            return Err(format!("animals.{id}: unknown class '{}'", animal.class));
        }
        let profile = race
            .get("attributes")
            .and_then(|v| v.get("remains_profile"))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("animals.{id}: missing remains profile"))?;
        let remains = crate::world_resources::resolve(
            root,
            &crate::world_resources::ResourceBinding {
                profile: profile.into(),
                ..Default::default()
            },
        )?;
        if !remains.spent_when_empty
            || remains
                .supplies
                .values()
                .any(|s| s.capacity.is_none() || s.recovery.is_some())
        {
            return Err(format!(
                "animals.{id}: remains must be finite and cannot recover"
            ));
        }
        animals.insert(id.clone(), animal);
    }
    Ok(animals)
}

pub fn from_source(source: &str) -> Result<BTreeMap<String, Animal>, String> {
    resolve(&source.parse::<Table>().map_err(|e| e.to_string())?)
}

pub(crate) fn validate(root: &Table, report: &mut crate::RulesetValidationReport) {
    if let Err(error) = resolve(root) {
        report.error("animals", error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_fauna_have_finite_world_sourced_remains() {
        let mut root: Table = crate::latest_official_ruleset().parse().unwrap();
        let animals = resolve(&root).unwrap();
        assert_eq!(animals.len(), 7);
        assert!(animals.contains_key("pig"));
        root["animals"]["pig"]["race"] = toml::Value::String("missing".into());
        assert!(resolve(&root).unwrap_err().contains("unknown race"));
        root["animals"]["pig"]["race"] = toml::Value::String("Pig".into());
        root["resource_profiles"]["pig_remains"]["spent_when_empty"] = toml::Value::Boolean(false);
        assert!(resolve(&root).unwrap_err().contains("finite"));
    }
}
