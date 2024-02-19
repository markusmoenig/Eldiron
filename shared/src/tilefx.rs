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
                let mut collection = TheCollection::named(str!("Brightness"));
                collection.set("Brightness", TheValue::FloatRange(1.0, 0.0..=2.0));
                TileFX::Brightness(collection)
            }
            "Light Emitter" => {
                let mut collection = TheCollection::named(str!("Light Emitter"));
                collection.set("Strength", TheValue::FloatRange(1.0, 0.1..=3.0));
                TileFX::LightEmitter(collection)
            }
            _ => TileFX::None,
        }
    }

    pub fn to_string(self) -> &'static str {
        match self {
            TileFX::None => "None",
            TileFX::Brightness(_) => "Brightness",
            TileFX::LightEmitter(_) => "Light Emitter",
        }
    }

    pub fn collection(&self) -> Option<&TheCollection> {
        match self {
            TileFX::None => None,
            TileFX::Brightness(collection) => Some(collection),
            TileFX::LightEmitter(collection) => Some(collection),
        }
    }
}
