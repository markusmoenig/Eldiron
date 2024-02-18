use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TileFX {
    None,
    ColorCorrection(TheCollection),
    LightEmitter(TheCollection),
}

impl TileFX {
    pub fn new_fx(name: &str) -> TileFX {
        match name {
            "Color Correction" => {
                let mut collection = TheCollection::named(str!("Color Correction"));
                collection.set("Brightness", TheValue::FloatRange(1.0, 0.0..=1.0));
                TileFX::ColorCorrection(collection)
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
            TileFX::ColorCorrection(_) => "Color Correction",
            TileFX::LightEmitter(_) => "Light Emitter",
        }
    }
}
