// --- Items / Inventory System

use crate::prelude::*;

/// An inventory item
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Spell {
    pub id                  : Uuid,
    pub name                : String,
    pub distance            : i32,
    pub tile                : Option<TileData>,
}

impl Spell {
    pub fn new(id: Uuid, name: String) -> Self {
        Self {
            id,
            name,
            distance        : 5,
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

    pub fn get_tile(&mut self) -> ScriptTile {
        if let Some(tile) = &self.tile {
            let tile_id = TileId::new(tile.tilemap, tile.x_off, tile.y_off);
            ScriptTile::new(tile_id)
        } else {
            let tile_id = TileId::new(Uuid::new_v4(), 0, 0);
            ScriptTile::new(tile_id)
        }
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

    /// Length of the spells array
    pub fn len(&mut self) -> i32 {
        self.spells.len() as i32
    }

    pub fn get_spell_at(&mut self, index: i32) -> Spell {
        if index >= 0 && index < self.spells.len() as i32 {
            return self.spells[index as usize].clone()
        }
        Spell::new(Uuid::new_v4(), "".to_string())
    }

    pub fn get_spell(&self, name: &String) -> Spell {
        for index in 0..self.spells.len() {
            if self.spells[index].name == *name {
                return self.spells[index].clone();
            }
        }
        Spell::new(Uuid::new_v4(), "".to_string())
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
        .register_get("tile", Spell::get_tile)
        .register_get("name", Spell::get_name);

    engine.register_type_with_name::<Spells>("Spells")
        .register_fn("len", Spells::len)
        .register_fn("spell_at", Spells::get_spell_at)
        .register_iterator::<Spells>();

}