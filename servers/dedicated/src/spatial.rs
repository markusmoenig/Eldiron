use super::*;
use eldiron_runtime::replication::SpatialSubscriptions;

pub(super) fn configuration(project: &Project) -> Result<SpatialSubscriptions, SaveError> {
    let config = project.config.parse::<toml::Table>().map_err(failure)?;
    let mut subscriptions = SpatialSubscriptions::default();
    if let Some(table) = config.get("multiplayer").and_then(|v| v.get("spatial")) {
        let table = table
            .as_table()
            .ok_or_else(|| failure("[multiplayer.spatial] must be a table"))?;
        for (key, target) in [
            ("radius", &mut subscriptions.policy.radius),
            ("margin", &mut subscriptions.policy.margin),
        ] {
            if let Some(value) = table.get(key) {
                *target = value
                    .as_float()
                    .or_else(|| value.as_integer().map(|v| v as f64))
                    .ok_or_else(|| failure(format!("Spatial {key} must be a number")))?
                    as f32;
            }
        }
        if let Some(value) = table.get("prefetch_regions") {
            let names = value
                .as_array()
                .filter(|a| a.len() <= 4)
                .ok_or_else(|| failure("At most four prefetch region names"))?;
            for name in names {
                let name = name
                    .as_str()
                    .ok_or_else(|| failure("Prefetch region name must be a string"))?;
                let region = project
                    .regions
                    .iter()
                    .find(|r| r.name == name)
                    .ok_or_else(|| failure(format!("Unknown prefetch region: {name}")))?;
                subscriptions.prefetch.insert(region.map.id);
            }
        }
    }
    if !subscriptions.policy.valid() {
        return Err(failure(
            "Spatial radius must be 4..512 and margin 0..128 world units",
        ));
    }
    Ok(subscriptions)
}
