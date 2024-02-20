use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TileFX {
    None,
    Brightness(TheCollection),
    LightEmitter(TheCollection),
}

impl TileFX {
    pub fn new_fx(name: &str) -> TileFX {
        match name {
            "Brightness" => {
                let mut collection = TheCollection::named(str!(name));
                collection.set("Brightness", TheValue::FloatRange(1.0, 0.0..=2.0));
                collection.set("Mask", TheValue::TileMask(TheTileMask::default()));
                TileFX::Brightness(collection)
            }
            "Light Emitter" => {
                let mut collection = TheCollection::named(str!(name));
                collection.set("Emission Strength", TheValue::FloatRange(1.0, 0.1..=3.0));
                collection.set("Mask", TheValue::TileMask(TheTileMask::default()));
                TileFX::LightEmitter(collection)
            }
            _ => TileFX::None,
        }
    }

    /// Convert to kind.
    pub fn to_kind(&self) -> String {
        match self {
            TileFX::None => str!("None"),
            TileFX::Brightness(_) => str!("Brightness"),
            TileFX::LightEmitter(_) => str!("Light Emitter"),
        }
    }

    /// Convert to collection.
    pub fn collection(&self) -> Option<&TheCollection> {
        match self {
            TileFX::None => None,
            TileFX::Brightness(collection) => Some(collection),
            TileFX::LightEmitter(collection) => Some(collection),
        }
    }

    /// Convert to cloned collection.
    pub fn collection_cloned(&self) -> TheCollection {
        match self {
            TileFX::None => TheCollection::default(),
            TileFX::Brightness(collection) => collection.clone(),
            TileFX::LightEmitter(collection) => collection.clone(),
        }
    }
}
