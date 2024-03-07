use indexmap::IndexMap;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFX {
    Cube,
    WallHorizontal(TheCollection, ModelFXMetaData),
}

impl ModelFX {
    pub fn new_fx(name: &str, collection: Option<TheCollection>) -> ModelFX {
        let mut coll = TheCollection::named(name.into());
        match name {
            "WallHorizontal" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Depth", TheValue::FloatRange(0.25, 0.0..=1.0));
                }
                let mut meta = ModelFXMetaData::new();
                meta.set_description("Depth", str!("The depth of the wall."));
                ModelFX::WallHorizontal(coll, meta)
            }
            _ => ModelFX::Cube,
        }
    }

    /// Convert to kind.
    pub fn to_kind(&self) -> String {
        match self {
            ModelFX::Cube => str!("Cube"),
            ModelFX::WallHorizontal(_, _) => str!("WallHorizontal"),
        }
    }

    /// Reference to the collection.
    pub fn collection(&self) -> Option<&TheCollection> {
        match self {
            ModelFX::Cube => None,
            ModelFX::WallHorizontal(collection, _) => Some(collection),
        }
    }

    /// Convert to cloned collection.
    pub fn collection_cloned(&self) -> TheCollection {
        match self {
            ModelFX::Cube => TheCollection::default(),
            ModelFX::WallHorizontal(collection, _) => collection.clone(),
        }
    }

    /// Get a reference to the meta data.
    pub fn meta_data(&self) -> Option<&ModelFXMetaData> {
        match self {
            ModelFX::Cube => None,
            ModelFX::WallHorizontal(_, meta) => Some(meta),
        }
    }

    /// Get the description of a key.
    pub fn get_description(&self, name: &str) -> String {
        if let Some(meta) = self.meta_data() {
            if let Some(description) = meta.description.get(name) {
                return description.clone();
            }
        }
        str!("")
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ModelFXMetaData {
    description: IndexMap<String, String>,
}

impl Default for ModelFXMetaData {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelFXMetaData {
    pub fn new() -> Self {
        Self {
            description: IndexMap::default(),
        }
    }

    pub fn set_description(&mut self, key: &str, description: String) {
        self.description.insert(str!(key), description);
    }
}
