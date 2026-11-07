use num_traits::zero;
use rusterix::Map;
use theframework::prelude::*;

/// An item instance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Item {
    pub id: Uuid,
    pub name: String,

    /// The item map model.
    pub map: Map,

    /// The module source
    #[serde(default)]
    pub module: serde_json::Value,

    /// The instance initialization or template code.
    pub source: String,

    /// The instance initialization or template debug code.
    #[serde(default)]
    pub source_debug: String,

    /// The attributes toml data.
    #[serde(default)]
    pub data: String,

    /// Authoring metadata used for look/description style presentation.
    #[serde(default)]
    pub authoring: String,

    /// Project-owned editable icon animation frames for the default/on state.
    /// An empty list inherits the icon resolved from a mapped tile, the active
    /// ruleset, or the item's ordinary visual fallback.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_frames: Vec<rusterix::Texture>,

    /// Archive paths for project-owned On frames. The archive loader hydrates
    /// these PNGs back into `icon_frames`; legacy JSON keeps using inline data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_frame_paths: Vec<String>,

    /// Project-owned editable icon animation frames for the optional off state.
    /// An empty list inherits an available off-state tile or ruleset artwork;
    /// otherwise the item has no off-state icon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_off_frames: Vec<rusterix::Texture>,

    /// Archive paths for project-owned Off frames. The archive loader hydrates
    /// these PNGs back into `icon_off_frames`; legacy JSON keeps using inline data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_off_frame_paths: Vec<String>,

    /// The initial position.
    pub position: Vec3<f32>,

    /// The id of the character template.
    pub item_id: Uuid,
}

impl Default for Item {
    fn default() -> Self {
        Self::new()
    }
}

impl Item {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "NewItem".to_string(),

            module: serde_json::Value::Null,
            map: Map::default(),
            source: String::new(),
            source_debug: String::new(),
            data: String::new(),
            authoring: String::new(),
            icon_frames: Vec::new(),
            icon_frame_paths: Vec::new(),
            icon_off_frames: Vec::new(),
            icon_off_frame_paths: Vec::new(),
            position: zero(),

            item_id: Uuid::new_v4(),
        }
    }

    pub fn icon_frames_for_state(&self, on: bool) -> &Vec<rusterix::Texture> {
        if on {
            &self.icon_frames
        } else {
            &self.icon_off_frames
        }
    }

    pub fn icon_frames_for_state_mut(&mut self, on: bool) -> &mut Vec<rusterix::Texture> {
        if on {
            &mut self.icon_frames
        } else {
            &mut self.icon_off_frames
        }
    }
}

/// Rules refresh gameplay templates; appearance links remain project-owned edits.
pub fn merge_appearance_data(existing: &str, template: &str) -> String {
    let (Ok(old), Ok(mut next)) = (
        existing.parse::<toml::Table>(),
        template.parse::<toml::Table>(),
    ) else {
        return template.into();
    };
    let keys = [
        "appearance_icon",
        "appearance_icon_off",
        "appearance_geometry",
        "appearance_geometry_off",
        "appearance_ground_flat",
        "appearance_materials",
        "appearance_padding",
        "appearance_worn_geometry",
        "appearance_idle",
        "appearance_walk",
        "appearance_attack",
        "appearance_attack_time",
        "appearance_hold_joint",
        "appearance_two_handed",
    ];
    let old_attrs = old.get("attributes").and_then(toml::Value::as_table);
    let overrides = keys
        .into_iter()
        .filter_map(|key| {
            old_attrs
                .and_then(|attrs| attrs.get(key))
                .or_else(|| old.get(key))
                .map(|value| (key, value.clone()))
        })
        .collect::<Vec<_>>();
    if overrides.is_empty() {
        return template.into();
    }
    let Some(attrs) = next
        .entry("attributes")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
    else {
        return template.into();
    };
    let mut changed = false;
    for (key, value) in overrides {
        if attrs.get(key) != Some(&value) {
            attrs.insert(key.into(), value);
            changed = true;
        }
    }
    if changed {
        toml::to_string_pretty(&next).unwrap_or_else(|_| template.into())
    } else {
        template.into()
    }
}

#[cfg(test)]
mod appearance_tests {
    #[test]
    fn rules_refresh_preserves_appearance_links_and_updates_gameplay() {
        let data = super::merge_appearance_data(
            "[attributes]\nappearance_icon = 'custom-icon'\nappearance_geometry = 'custom-model'\nDMG = 999\n",
            "[attributes]\nname = 'Updated'\nDMG = 2\n",
        );
        let root = data.parse::<toml::Table>().unwrap();
        let attrs = root["attributes"].as_table().unwrap();
        assert_eq!(attrs["appearance_icon"].as_str(), Some("custom-icon"));
        assert_eq!(attrs["appearance_geometry"].as_str(), Some("custom-model"));
        assert_eq!(attrs["DMG"].as_integer(), Some(2));
    }

    #[test]
    fn appearance_links_are_canonical_and_rules_refresh_is_idempotent() {
        let template = "[attributes]\nname = 'Item'\nappearance_geometry = 'model'\n";
        assert_eq!(super::merge_appearance_data(template, template), template);
        let changed = super::merge_appearance_data(
            "appearance_icon = 'icon'\n[attributes]\nappearance_geometry = 'custom'\n",
            "name = 'Item'\n",
        );
        let root = changed.parse::<toml::Table>().unwrap();
        assert_eq!(root["attributes"]["appearance_icon"].as_str(), Some("icon"));
        assert_eq!(
            root["attributes"]["appearance_geometry"].as_str(),
            Some("custom")
        );
        assert_eq!(
            super::merge_appearance_data(template, "attributes = 1\n"),
            "attributes = 1\n"
        );
    }
}
