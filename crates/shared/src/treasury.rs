use crate::prelude::*;
use theframework::prelude::*;

fn default_treasury_package_id() -> Uuid {
    Uuid::new_v4()
}

#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct TreasuryPackageMetadata {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct TreasuryPackageManifest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: String,
}

impl TreasuryPackageManifest {
    pub fn tile_collection(metadata: &TreasuryPackageMetadata) -> Self {
        Self {
            name: metadata.name.clone(),
            author: metadata.author.clone(),
            version: metadata.version.clone(),
            description: metadata.description.clone(),
            kind: "tile_collection".to_string(),
        }
    }
}

impl TreasuryPackageMetadata {
    pub fn from_collection(collection: &TileCollectionAsset) -> Self {
        Self {
            name: collection.name.clone(),
            author: collection.author.clone(),
            version: collection.version.clone(),
            description: collection.description.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct TreasuryPackageSummary {
    #[serde(default = "default_treasury_package_id")]
    pub id: Uuid,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
}

impl TreasuryPackageSummary {
    pub fn display_name(&self) -> String {
        if self.name.is_empty() {
            self.slug.clone()
        } else {
            self.name.clone()
        }
    }

    pub fn metadata(&self) -> TreasuryPackageMetadata {
        TreasuryPackageMetadata {
            name: self.name.clone(),
            author: self.author.clone(),
            version: self.version.clone(),
            description: self.description.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct TreasuryIndexCategories {
    #[serde(default)]
    pub tiles: Vec<TreasuryPackageSummary>,
}

#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct TreasuryIndex {
    #[serde(default)]
    pub tiles: Vec<TreasuryPackageSummary>,
    #[serde(default)]
    pub categories: TreasuryIndexCategories,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TreasuryTileCollectionPackage {
    #[serde(default)]
    pub metadata: TreasuryPackageMetadata,
    pub collection: TileCollectionAsset,
    #[serde(default)]
    pub tiles: IndexMap<Uuid, rusterix::Tile>,
    #[serde(default)]
    pub tile_groups: IndexMap<Uuid, rusterix::TileGroup>,
    #[serde(default)]
    pub tile_node_groups: IndexMap<Uuid, NodeGroupAsset>,
}
