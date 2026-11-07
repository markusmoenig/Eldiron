//! Data-defined worn surfaces. Mesh part IDs and MZ sources belong to the recipe.
use crate::{Item, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Wearable {
    pub geometry: Option<String>,
    pub materials: BTreeMap<String, String>,
    pub padding: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bindings_are_explicit_and_invalid_padding_or_duplicate_parts_are_rejected() {
        let mut item = Item::default();
        crate::server::data::apply_item_data(
            &mut item,
            "[attributes]\nappearance_materials=['custom-body=ruleset:material/linen-shirt']\nappearance_padding=0.005\n",
        );
        let worn = Wearable::from_item(&item).unwrap().unwrap();
        assert_eq!(
            worn.materials["custom-body"],
            "ruleset:material/linen-shirt"
        );
        item.set_attribute("appearance_padding", Value::Float(-0.1));
        assert!(Wearable::from_item(&item).is_err());
        item.set_attribute("appearance_padding", Value::Float(0.));
        item.set_attribute(
            "appearance_materials",
            Value::StrArray(vec!["part=a".into(), "part=b".into()]),
        );
        assert!(Wearable::from_item(&item).is_err());
    }
}
impl Wearable {
    pub fn from_item(item: &Item) -> Result<Option<Self>, String> {
        let entries = match item.attributes.get("appearance_materials") {
            Some(Value::StrArray(entries)) => entries.as_slice(),
            _ => &[],
        };
        let geometry = item
            .attributes
            .get_str("appearance_worn_geometry")
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned);
        if geometry.as_ref().is_some_and(|s| s.len() > 256) {
            return Err("Invalid worn geometry source".into());
        }
        if entries.len() > 64 {
            return Err("Worn appearance supports at most 64 part bindings".into());
        }
        let mut materials = BTreeMap::new();
        for entry in entries {
            let (part, source) = entry
                .split_once('=')
                .ok_or("Use mesh-part=material-source for worn materials")?;
            let (part, source) = (part.trim(), source.trim());
            if part.is_empty() || source.is_empty() || part.len() > 128 || source.len() > 256 {
                return Err("Invalid worn material binding".into());
            }
            if materials.insert(part.into(), source.into()).is_some() {
                return Err(format!("Worn material repeats mesh part {part}"));
            }
        }
        if materials.is_empty() && geometry.is_none() {
            return Ok(None);
        }
        let padding = item
            .attributes
            .get_float("appearance_padding")
            .or_else(|| {
                item.attributes
                    .get_int("appearance_padding")
                    .map(|v| v as f32)
            })
            .unwrap_or(0.);
        if !padding.is_finite() || !(0. ..=0.03).contains(&padding) {
            return Err("Garment padding must be 0–0.03 metres".into());
        }
        Ok(Some(Self {
            materials,
            padding,
            geometry,
        }))
    }
    pub fn signature(&self) -> u64 {
        super::fingerprint(&(&self.materials, &self.geometry, self.padding.to_bits()))
    }
}
