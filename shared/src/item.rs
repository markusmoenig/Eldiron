use theframework::prelude::*;

/// An item instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Item {
    pub id: Uuid,

    /// The id of the item bundle.
    pub item_id: Uuid,

    /// The custom bundle to override the default behavior.
    pub instance: TheCodeBundle,
}

impl Default for Item {
    fn default() -> Self {
        Self::new()
    }
}

impl Item {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            item_id: Uuid::new_v4(),

            instance: TheCodeBundle::new(),
        }
    }
}
