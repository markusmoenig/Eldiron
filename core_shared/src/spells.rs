// --- Items / Inventory System

use crate::prelude::*;

/// An inventory item
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Spell {
    pub id                  : Uuid,
    pub name                : String,
    pub tile                : Option<TileData>,
}

impl Spell {
    pub fn new(id: Uuid, name: String) -> Self {
        Self {
            id,
            name,
            tile            : None,
        }
    }

    /// Reads the Spell properties from a PropertySink.
    pub fn read_from_sink(&mut self, _sink: &PropertySink) {

    }

    // Getter

    pub fn get_name(&mut self) -> String {
        self.name.clone()
    }
}

/// Spells
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Spells {
    pub spells                  : Vec<Spell>,
    pub spells_to_execute       : Vec<String>,
}

impl Spells {
    pub fn new() -> Self {
        Self {
            spells              : vec![],
            spells_to_execute   : vec![],
        }
    }

    /// Queues a spell to be executed
    pub fn execute(&mut self, name: &str) {
        self.spells_to_execute.push(name.to_string());
    }
}

// Implement 'IntoIterator' trait
impl IntoIterator for Spells {
    type Item = Spell;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.spells.into_iter()
    }
}

pub fn script_register_spells_api(engine: &mut rhai::Engine) {

    engine.register_type_with_name::<Spell>("Spell")
        .register_get("name", Spell::get_name);

    engine.register_type_with_name::<Spells>("Spells")
        .register_iterator::<Spells>();

}