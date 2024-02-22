use indexmap::IndexMap;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TileFX {
    None,
    Brightness(TheCollection, TileFXMetaData),
    LightEmitter(TheCollection, TileFXMetaData),
}

impl TileFX {
    pub fn new_fx(name: &str, collection: Option<TheCollection>) -> TileFX {
        let mut coll = TheCollection::named(name.into());
        match name {
            "Brightness" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Brightness", TheValue::FloatRange(1.0, 0.0..=2.0));
                    coll.set("Mask", TheValue::TileMask(TheTileMask::default()));
                }
                let mut meta = TileFXMetaData::new();
                meta.set_description("Brightness", str!("The brighntess of the tile."));
                TileFX::Brightness(coll, meta)
            }
            "Daylight" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Attenuation", TheValue::FloatRange(1.0, 0.0..=2.0));
                    coll.set("Mask", TheValue::TileMask(TheTileMask::default()));
                }
                let mut meta = TileFXMetaData::new();
                meta.set_description(
                    "Attenuation",
                    str!("Replaces the pixels in the mask with the attenuated daylight. This allows for daylight effects (windows etc)."),
                );
                TileFX::Brightness(coll, meta)
            }
            "Light Emitter" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Emission Strength", TheValue::FloatRange(1.0, 0.1..=3.0));
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
                    coll.set("Mask", TheValue::TileMask(TheTileMask::default()));
                }
                let mut meta = TileFXMetaData::new();
                meta.set_description("Emission Strength", str!("The strength of the light."));
                TileFX::LightEmitter(coll, meta)
            }
            _ => TileFX::None,
        }
    }

    /// Convert to kind.
    pub fn to_kind(&self) -> String {
        match self {
            TileFX::None => str!("None"),
            TileFX::Brightness(_, _) => str!("Brightness"),
            TileFX::LightEmitter(_, _) => str!("Light Emitter"),
        }
    }

    /// Reference to the collection.
    pub fn collection(&self) -> Option<&TheCollection> {
        match self {
            TileFX::None => None,
            TileFX::Brightness(collection, _) => Some(collection),
            TileFX::LightEmitter(collection, _) => Some(collection),
        }
    }

    /// Convert to cloned collection.
    pub fn collection_cloned(&self) -> TheCollection {
        match self {
            TileFX::None => TheCollection::default(),
            TileFX::Brightness(collection, _) => collection.clone(),
            TileFX::LightEmitter(collection, _) => collection.clone(),
        }
    }

    /// Get a reference to the meta data.
    pub fn meta_data(&self) -> Option<&TileFXMetaData> {
        match self {
            TileFX::None => None,
            TileFX::Brightness(_, meta) => Some(meta),
            TileFX::LightEmitter(_, meta) => Some(meta),
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
pub struct TileFXMetaData {
    description: IndexMap<String, String>,
}

impl Default for TileFXMetaData {
    fn default() -> Self {
        Self::new()
    }
}

impl TileFXMetaData {
    pub fn new() -> Self {
        Self {
            description: IndexMap::default(),
        }
    }

    pub fn set_description(&mut self, key: &str, description: String) {
        self.description.insert(str!(key), description);
    }
}
