use crate::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterData {
    pub name                    : String,

    #[serde(skip)]
    pub id                      : Uuid,
    #[serde(skip)]
    pub index                   : usize,

    pub position                : Position,
    pub old_position            : Option<Position>,
    pub max_transition_time     : usize,
    pub curr_transition_time    : usize,

    pub tile                    : TileId,
    pub effects                 : Vec<TileId>
}

impl CharacterData {

    pub fn get_name(&mut self) -> String {
        self.name.clone()
    }

    pub fn register(engine: &mut rhai::Engine) {
        engine.register_type_with_name::<CharacterData>("CharacterData")
            .register_get("name", CharacterData::get_name);
    }
}

// Struct to be able to iterate them on the client

#[derive(Clone)]
pub struct CharacterList {
    pub characters      : Vec<CharacterData>
}

impl CharacterList {
    pub fn new(characters: Vec<CharacterData>) -> Self {
        Self {
            characters
        }
    }

    pub fn register(engine: &mut rhai::Engine) {
        engine.register_type_with_name::<CharacterList>("CharacterList")
        .register_iterator::<CharacterList>();
    }
}

// Implement 'IntoIterator' trait
impl IntoIterator for CharacterList {
    type Item = CharacterData;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.characters.into_iter()
    }
}