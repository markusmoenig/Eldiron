use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Character {
    pub id: Uuid,

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
            character_id: Uuid::new_v4(),

            instance: TheCodeBundle::new(),
        }
    }
}
