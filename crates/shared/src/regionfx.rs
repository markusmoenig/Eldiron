use indexmap::IndexMap;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RegionFX {
    None,
    Camera(TheCollection, RegionFXMetaData),
    RenderDistanceFog(TheCollection, RegionFXMetaData),
    Saturation(TheCollection, RegionFXMetaData),
}

impl RegionFX {
    pub fn new_fx(name: &str, collection: Option<TheCollection>) -> RegionFX {
        let mut coll = TheCollection::named(name.into());
        match name {
            "Renderer" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set(
                        "Shading",
                        TheValue::TextList(0, vec![str!("Pixel Art"), str!("PBR")]),
                    );
                    coll.set("Upscale", TheValue::FloatRange(1.5, 1.0..=5.0));
                }
                let mut meta = RegionFXMetaData::new();
                meta.set_description("Shading", str!("The shading model. Pixel Art does not shade the pixels (only for shadows). PBR does physical based shading."));
                meta.set_description("Upscale", str!("Upscale reduces the resolution of the game and then upscales it. This can be used to create a pixel art look."));
                RegionFX::Saturation(coll, meta)
            }
            "Camera" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("First Person Height", TheValue::FloatRange(0.5, 0.0..=1.0));
                    coll.set("First Person FoV", TheValue::FloatRange(70.0, 1.0..=140.0));

                    coll.set("Top Down Height", TheValue::FloatRange(14.0, 0.0..=20.0));
                    coll.set("Top Down X Offset", TheValue::FloatRange(-1.0, -5.0..=5.0));
                    coll.set("Top Down Z Offset", TheValue::FloatRange(1.0, -5.0..=5.0));

                    coll.set("", TheValue::Empty);

                    coll.set(
                        "Tilted Iso Alignment",
                        TheValue::TextList(0, vec![str!("Right"), str!("Left")]),
                    );
                }
                let mut meta = RegionFXMetaData::new();
                meta.set_description("Emission Strength", str!("The strength of the light."));
                meta.set_description(
                    "Maximum Distance",
                    str!("The maximum distance light is travelling (in tiles)."),
                );
                meta.set_description(
                    "Samples #",
                    str!("The number of light samples to take. More samples mean a softer light."),
                );
                meta.set_description(
                    "Sample Offset",
                    str!("The offset of the samples from the origin."),
                );
                meta.set_description(
                    "Limit Direction",
                    str!("Limits the light distribution to the given direction. Useful for example for windows."),
                );
                meta.second_column_ids = vec![str!("Top Down")];
                RegionFX::Camera(coll, meta)
            }
            "Distance / Fog" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Maximum Render Distance", TheValue::IntRange(20, 1..=100));
                }
                let mut meta = RegionFXMetaData::new();
                meta.set_description(
                    "Maximum Render Distance",
                    str!("The maximum render distance."),
                );
                RegionFX::Saturation(coll, meta)
            }
            "Saturation" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Saturation", TheValue::FloatRange(1.0, 0.0..=2.0));
                }
                let mut meta = RegionFXMetaData::new();
                meta.set_description("Saturation", str!("Adjusts the saturation."));
                RegionFX::Saturation(coll, meta)
            }
            _ => RegionFX::None,
        }
    }

    /// Convert to kind.
    pub fn to_kind(&self) -> String {
        match self {
            RegionFX::None => str!("None"),
            RegionFX::Camera(_, _) => str!("Camera"),
            RegionFX::RenderDistanceFog(_, _) => str!("Saturation"),
            RegionFX::Saturation(_, _) => str!("Saturation"),
        }
    }

    /// Reference to the collection.
    pub fn collection(&self) -> Option<&TheCollection> {
        match self {
            RegionFX::None => None,
            RegionFX::Camera(collection, _) => Some(collection),
            RegionFX::RenderDistanceFog(collection, _) => Some(collection),
            RegionFX::Saturation(collection, _) => Some(collection),
        }
    }

    /// Convert to cloned collection.
    pub fn collection_cloned(&self) -> TheCollection {
        match self {
            RegionFX::None => TheCollection::default(),
            RegionFX::Camera(collection, _) => collection.clone(),
            RegionFX::RenderDistanceFog(collection, _) => collection.clone(),
            RegionFX::Saturation(collection, _) => collection.clone(),
        }
    }

    /// Get a reference to the meta data.
    pub fn meta_data(&self) -> Option<&RegionFXMetaData> {
        match self {
            RegionFX::None => None,
            RegionFX::Camera(_, meta) => Some(meta),
            RegionFX::RenderDistanceFog(_, meta) => Some(meta),
            RegionFX::Saturation(_, meta) => Some(meta),
        }
    }

    /// Get the description of a key.
    pub fn get_description(&self, name: &str) -> String {
        if let Some(meta) = self.meta_data() {
            if let Some(description) = meta.description.get(name) {
                return description.clone();
            }
        }
        str!("")
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionFXMetaData {
    description: IndexMap<String, String>,
    second_column_ids: Vec<String>,
}

impl Default for RegionFXMetaData {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionFXMetaData {
    pub fn new() -> Self {
        Self {
            description: IndexMap::default(),
            second_column_ids: vec![],
        }
    }

    pub fn set_description(&mut self, key: &str, description: String) {
        self.description.insert(str!(key), description);
    }

    pub fn is_second_column(&self, key: &str) -> bool {
        for id in &self.second_column_ids {
            if key.contains(id) {
                return true;
            }
        }
        false
    }
}
