use super::*;
use denrim_materials::geometry::group::Control;
use presentation::{number, property};
pub fn categories(controls: &[Control]) -> Vec<String> {
    let mut result = vec![];
    for c in controls {
        if !result.contains(&c.category) {
            result.push(c.category.clone());
        }
    }
    result
}
pub fn parameters(
    controls: &[Control],
    selection: &rigging::Selection,
) -> (Vec<MaterialNumberView>, Vec<MaterialPropertyView>) {
    let categories = categories(controls);
    let selected = selection.category.min(categories.len().saturating_sub(1));
    let mut n = vec![];
    let mut p = vec![];
    if categories.is_empty() {
        return (n, p);
    }
    n.push(number(
        "@group_category",
        "Controls",
        selected as f32,
        0.,
        categories.len().saturating_sub(1) as f32,
    ));
    p.push(property(
        "@group_category",
        "Controls",
        vec!["@group_category".into()],
        MaterialPropertyKind::Choice,
        categories.clone(),
    ));
    let mut visible = controls
        .iter()
        .filter(|c| c.category == categories[selected])
        .collect::<Vec<_>>();
    visible.sort_by_key(|c| c.order);
    for c in visible {
        let id = format!("control:{}", c.id);
        if c.is_palette() {
            let components = (0..3).map(|k| format!("{id}:{k}")).collect::<Vec<_>>();
            for key in &components {
                n.push(number(key, &c.label, 0., 0., 1.));
            }
            p.push(property(
                &id,
                &c.label,
                components,
                MaterialPropertyKind::Color,
                vec![],
            ));
        } else {
            n.push(number(&id, &c.label, c.value, c.min, c.max));
            p.push(property(
                &id,
                &c.label,
                vec![id.clone()],
                MaterialPropertyKind::Number,
                vec![],
            ));
        }
        if let Some(property) = p.last_mut() {
            property.tooltip = c.help.clone();
        }
    }
    n.push(number("group_reset", "Reset controls", 0., 0., 2.));
    p.push(property(
        "group_reset",
        "Reset controls",
        vec!["group_reset".into()],
        MaterialPropertyKind::Choice,
        vec![
            "Keep current values".into(),
            "Reset this section".into(),
            "Reset all controls".into(),
        ],
    ));
    (n, p)
}
pub fn colors(
    tile: &Tile,
    palette: &ThePalette,
    numbers: &mut [MaterialNumberView],
    bindings: &mut Vec<MaterialPaletteBindingView>,
) {
    if let Operation::Group { controls, .. } = &tile.op {
        for c in controls.iter().filter(|c| c.is_palette()) {
            let id = format!("control:{}", c.id);
            bindings.push(MaterialPaletteBindingView {
                operation: tile.id.clone(),
                parameter: id.clone(),
                index: c.value as u16,
            });
            if let Some(color) = palette
                .colors
                .get(c.value as usize)
                .and_then(Option::as_ref)
            {
                let rgb = color.to_u8_array();
                for k in 0..3 {
                    if let Some(n) = numbers.iter_mut().find(|n| n.id == format!("{id}:{k}")) {
                        let s = rgb[k] as f32 / 255.;
                        n.value = if s <= 0.04045 {
                            s / 12.92
                        } else {
                            ((s + 0.055) / 1.055).powf(2.4)
                        };
                    }
                }
            }
        }
    }
}
pub fn edit(
    controls: &mut [Control],
    key: &str,
    value: f32,
    selection: &rigging::Selection,
) -> Result<(), String> {
    if key == "group_reset" {
        let category = categories(controls).get(selection.category).cloned();
        for c in controls {
            if value.round() == 2.
                || (value.round() == 1. && category.as_ref() == Some(&c.category))
            {
                c.value = c.reference;
            }
        }
        return Ok(());
    }
    let id = key
        .strip_prefix("control:")
        .ok_or("Unknown group control")?;
    let c = controls
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or("Missing group control")?;
    if !value.is_finite() || value < c.min || value > c.max {
        return Err("Value is outside this control's range".into());
    }
    c.value = value;
    Ok(())
}
