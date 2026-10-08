//! Adapt resolved Eldiron ruleset tables to the standalone appearance catalog.
use crate::*;
use toml::{Table, Value};
/// Appearance declarations live in `[procedural_characters]`,
/// `[procedural_materials]` and `[items.<group>.<id>.appearance]`.
/// Gameplay fields and unrelated items are deliberately left to Eldiron.
pub fn catalog_from_table(root: &Table) -> Result<Catalog> {
    let characters = root
        .get("procedural_characters")
        .cloned()
        .ok_or("ruleset has no procedural_characters table")?;
    let materials = root
        .get("procedural_materials")
        .cloned()
        .unwrap_or_else(|| Value::Table(Table::new()));
    let mut items = Table::new();
    if let Some(groups) = root.get("items").and_then(Value::as_table) {
        for (group_name, group) in groups {
            let group = group
                .as_table()
                .ok_or_else(|| format!("items.{group_name} must be a table"))?;
            for (id, value) in group {
                if let Some(appearance) = value.get("appearance")
                    && items.insert(id.clone(), appearance.clone()).is_some()
                {
                    return Err(format!("ambiguous procedural item ID {id}").into());
                }
            }
        }
    }
    let mut table = Table::new();
    table.insert("version".into(), Value::Integer(1));
    table.insert("characters".into(), characters);
    table.insert("materials".into(), materials);
    table.insert("items".into(), Value::Table(items));
    Catalog::parse(&toml::to_string(&table)?)
}
#[cfg(feature = "eldiron")]
pub fn catalog_from_resolved(ruleset: &eldiron_ruleset::ResolvedRuleset) -> Result<Catalog> {
    let mut catalog = catalog_from_table(ruleset.table())?;
    let palette = eldiron_ruleset::ruleset_palette(ruleset.table())?;
    let colors: Vec<String> = palette
        .colors
        .iter()
        .flatten()
        .map(|c| {
            format!(
                "#{:02x}{:02x}{:02x}",
                (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
                (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
                (c.b.clamp(0.0, 1.0) * 255.0).round() as u8
            )
        })
        .collect();
    for material in catalog.materials.values_mut() {
        if material.palette.is_empty() {
            material.palette = colors.clone();
        }
    }
    Ok(catalog)
}
