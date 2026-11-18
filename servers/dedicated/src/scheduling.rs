use super::*;
use rusterix::server::scheduling::RegionSchedulingPolicy;
pub(super) fn configuration(project: &Project) -> Result<RegionSchedulingPolicy, SaveError> {
    let config = project.config.parse::<toml::Table>().map_err(failure)?;
    let mut policy = RegionSchedulingPolicy::default();
    let Some(table) = config.get("multiplayer").and_then(|v| v.get("simulation")) else {
        return Ok(policy);
    };
    let table = table
        .as_table()
        .ok_or_else(|| failure("[multiplayer.simulation] must be a table"))?;
    if table
        .keys()
        .any(|k| !matches!(k.as_str(), "pause_unoccupied_regions" | "idle_after_ms"))
    {
        return Err(failure("Unknown multiplayer.simulation setting"));
    }
    if let Some(value) = table.get("idle_after_ms") {
        let ms = value
            .as_integer()
            .filter(|ms| (20..=600_000).contains(ms))
            .ok_or_else(|| failure("Simulation idle_after_ms must be 20..600000"))?;
        policy.idle_after = Duration::from_millis(ms as u64);
    }
    if let Some(value) = table.get("pause_unoccupied_regions") {
        let names = value.as_array().filter(|v| v.len() <= 1024).ok_or_else(|| failure("Simulation pause_unoccupied_regions must be a list of at most 1024 region names"))?;
        for name in names {
            let name = name
                .as_str()
                .ok_or_else(|| failure("Paused region names must be strings"))?;
            let regions: Vec<_> = project.regions.iter().filter(|r| r.name == name).collect();
            if regions.len() != 1 {
                return Err(failure(format!(
                    "Paused region name must identify one region: {name}"
                )));
            }
            if !policy.pause_unoccupied_regions.insert(regions[0].map.id) {
                return Err(failure("Duplicate paused region name"));
            }
        }
    }
    Ok(policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_project_policy_is_optional_and_resolves_exact_regions() {
        let mut project = Project::new();
        project.regions[0].name = "Dungeon".into();
        assert!(
            configuration(&project)
                .unwrap()
                .pause_unoccupied_regions
                .is_empty()
        );
        let good =
            "[multiplayer.simulation]\npause_unoccupied_regions=['Dungeon']\nidle_after_ms=5000";
        project.config = good.into();
        let p = configuration(&project).unwrap();
        assert_eq!(
            p.pause_unoccupied_regions,
            [project.regions[0].map.id].into()
        );
        for bad in [
            good.replace("Dungeon", "Missing"),
            good.replace("5000", "0"),
            good.replace("5000", "1.5"),
            good.replace("idle_after_ms", "idle_milis"),
            good.replace("['Dungeon']", "['Dungeon','Dungeon']"),
        ] {
            project.config = bad;
            assert!(configuration(&project).is_err());
        }
    }
}
