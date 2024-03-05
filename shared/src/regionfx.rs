use indexmap::IndexMap;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RegionFX {
    None,
    Camera(TheCollection, RegionFXMetaData),
    Saturation(TheCollection, RegionFXMetaData),
}

impl RegionFX {
    pub fn new_fx(name: &str, collection: Option<TheCollection>) -> RegionFX {
        let mut coll = TheCollection::named(name.into());
        match name {
            "Camera" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set(
                        "Camera Type",
                        TheValue::TextList(0, vec![str!("Pinhole"), str!("Orthogonal")]),
                    );

                    coll.set("Origin X", TheValue::FloatRange(0.0, -1.0..=5.0));
                    coll.set("Origin Y", TheValue::FloatRange(0.0, -1.0..=5.0));
                    coll.set("Origin Z", TheValue::FloatRange(0.0, -1.0..=5.0));

                    coll.set("Center X", TheValue::FloatRange(0.0, -1.0..=5.0));
                    coll.set("Center Y", TheValue::FloatRange(0.0, -1.0..=5.0));
                    coll.set("Center Z", TheValue::FloatRange(0.0, -1.0..=5.0));

                    coll.set("FoV", TheValue::FloatRange(70.0, 1.0..=140.0));

                    /*
                    coll.set("Emission Strength", TheValue::FloatRange(1.0, 0.1..=3.0));
                    coll.set("Maximum Distance", TheValue::IntRange(10, 1..=20));
                    coll.set("Samples #", TheValue::IntRange(5, 1..=7));
                    coll.set("Sample Offset", TheValue::FloatRange(0.5, 0.01..=0.5));
                    coll.set(
                        "Limit Direction",
                        TheValue::TextList(
                            0,
                            vec![
                                str!("No"),
                                str!("Only North"),
                                str!("Only East"),
                                str!("Only South"),
                                str!("Only West"),
                            ],
                        ),
                    );
                    coll.set(
                        "Light Color",
                        TheValue::TextList(0, vec![str!("Color"), str!("Daylight")]),
                    );
                    coll.set("Color", TheValue::ColorObject(TheColor::white(), 0.0));
                    coll.set("Mask", TheValue::TileMask(TheTileMask::default()));
                    */
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
                RegionFX::Camera(coll, meta)
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
            RegionFX::Saturation(_, _) => str!("Saturation"),
        }
    }

    /// Reference to the collection.
    pub fn collection(&self) -> Option<&TheCollection> {
        match self {
            RegionFX::None => None,
            RegionFX::Camera(collection, _) => Some(collection),
            RegionFX::Saturation(collection, _) => Some(collection),
        }
    }

    /// Convert to cloned collection.
    pub fn collection_cloned(&self) -> TheCollection {
        match self {
            RegionFX::None => TheCollection::default(),
            RegionFX::Camera(collection, _) => collection.clone(),
            RegionFX::Saturation(collection, _) => collection.clone(),
        }
    }

    /// Get a reference to the meta data.
    pub fn meta_data(&self) -> Option<&RegionFXMetaData> {
        match self {
            RegionFX::None => None,
            RegionFX::Camera(_, meta) => Some(meta),
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
        }
    }

    pub fn set_description(&mut self, key: &str, description: String) {
        self.description.insert(str!(key), description);
    }
}
