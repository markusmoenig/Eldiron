//! Declarative policies consumed by behavior nodes; graph parameters select IDs only.
use serde::Deserialize;
use toml::Table;

#[derive(Clone, Debug, Deserialize)]
pub struct LookoutPolicy {
    pub disposition: String,
    pub radius: f32,
    pub retry_seconds: f32,
}
#[derive(Clone, Debug, Deserialize)]
pub struct EngagePolicy {
    pub actions: Vec<String>,
    pub speed: f32,
    pub pursuit_distance: f32,
    pub blocked_seconds: f32,
}
fn policy<T: serde::de::DeserializeOwned>(
    rules: &Table,
    kind: &str,
    id: &str,
) -> Result<T, String> {
    rules
        .get("behavior")
        .and_then(|v| v.get(kind))
        .and_then(|v| v.get(id))
        .ok_or_else(|| format!("Missing ruleset behavior.{kind}.{id}"))?
        .clone()
        .try_into()
        .map_err(|e| format!("Invalid behavior.{kind}.{id}: {e}"))
}
/// Source-based boundary for clients using a different TOML crate version.
pub fn policy_ids_from_source(source: &str, kind: &str) -> Result<Vec<String>, String> {
    let rules = source.parse::<Table>().map_err(|error| error.to_string())?;
    Ok(policy_ids(&rules, kind))
}

pub fn policy_ids(rules: &Table, kind: &str) -> Vec<String> {
    rules
        .get("behavior")
        .and_then(|v| v.get(kind))
        .and_then(|v| v.as_table())
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}
pub fn lookout(rules: &Table, id: &str) -> Result<LookoutPolicy, String> {
    let p: LookoutPolicy = policy(rules, "lookout", id)?;
    if !p.retry_seconds.is_finite()
        || p.retry_seconds <= 0.0
        || !p.radius.is_finite()
        || p.radius <= 0.0
        || rules
            .get("dispositions")
            .and_then(|v| v.get(p.disposition.as_str()))
            .is_none()
    {
        return Err(format!("Invalid lookout policy '{id}'"));
    }
    Ok(p)
}
pub fn engage(rules: &Table, id: &str) -> Result<EngagePolicy, String> {
    let p: EngagePolicy = policy(rules, "engage", id)?;
    if p.actions.is_empty()
        || [p.speed, p.pursuit_distance, p.blocked_seconds]
            .iter()
            .any(|n| !n.is_finite() || *n <= 0.0)
    {
        return Err(format!("Invalid engage policy '{id}'"));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_policies_are_valid_and_unknown_profiles_fail() {
        let rules: Table = crate::latest_official_ruleset().parse().unwrap();
        for id in policy_ids(&rules, "lookout") {
            assert!(lookout(&rules, &id).is_ok());
        }
        assert_eq!(engage(&rules, "default").unwrap().actions, ["basic_attack"]);
        assert!(lookout(&rules, "missing").is_err());
        assert!(engage(&rules, "missing").is_err());
    }
}
