use rusterix::Map;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Character {
    pub id: Uuid,

    pub name: String,

    /// The character model.
    pub map: Map,

    /// The source code.
    pub source: String,

    /// The id of the character bundle.
    pub character_id: Uuid,
    /// The custom bundle to override the default behavior.
    pub instance: TheCodeBundle,
}

impl Default for Character {
    fn default() -> Self {
        Self::new()
    }
}

impl Character {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New Character".to_string(),

            map: Map::default(),
            source: String::new(),

            character_id: Uuid::new_v4(),
            instance: TheCodeBundle::new(),
        }
    }
}
