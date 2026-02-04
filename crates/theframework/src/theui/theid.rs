pub use crate::prelude::*;
use std::hash::{Hash, Hasher};

/// Defines the identifier for a widget, its name and Uuid.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TheId {
    pub name: String,
    pub uuid: Uuid,
    pub references: Uuid,
}

impl PartialEq for TheId {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for TheId {}

impl Hash for TheId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state); // Hash based only on `uuid`
    }
}

impl TheId {
    /// Creates an Id based on a given name.
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_string(),
            uuid: Uuid::new_v4(),
            references: Uuid::nil(),
        }
    }

    /// Creates an Id based on a given name and uuid.
    pub fn named_with_id(name: &str, uuid: Uuid) -> Self {
        Self {
            name: name.to_string(),
            uuid,
            references: Uuid::nil(),
        }
    }

    /// Creates an Id based on a given name and reference uuid.
    pub fn named_with_reference(name: &str, references: Uuid) -> Self {
        Self {
            name: name.to_string(),
            uuid: Uuid::new_v4(),
            references,
        }
    }

    /// Creates an Id based on a given name, id and reference uuid.
    pub fn named_with_id_and_reference(name: &str, uuid: Uuid, references: Uuid) -> Self {
        Self {
            name: name.to_string(),
            uuid,
            references,
        }
    }

    /// Creates an empty id (an id wth an empty name).
    pub fn empty() -> Self {
        Self {
            name: "".to_string(),
            uuid: Uuid::new_v4(),
            references: Uuid::nil(),
        }
    }

    /// Matches the id against optional names and uuids.
    pub fn matches(&self, name: Option<&String>, uuid: Option<&Uuid>) -> bool {
        if name.is_none() && uuid.is_none() {
            return false;
        }

        if uuid.is_some() {
            return uuid == Some(&self.uuid);
        }

        name == Some(&self.name) || uuid == Some(&self.uuid)
    }

    /// Checks if the ids are equal (reference the same widget).
    pub fn equals(&self, other: &Option<TheId>) -> bool {
        if let Some(other) = other {
            if self.uuid == other.uuid {
                return true;
            }
        }
        false
    }
}
