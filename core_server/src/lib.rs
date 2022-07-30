pub mod gamedata;
pub mod server;

pub mod prelude {
    pub use crate::gamedata::GameData;
    pub use crate::gamedata::behavior::GameBehavior;

    pub use crate::server::Server;
    pub use crate::server::region_instance::RegionInstance;
    pub use crate::server::region_pool::RegionPool;

    pub use std::collections::HashMap;

    pub use rand::prelude::*;

    pub use std::fs;
    pub use std::path;
    pub use std::path::PathBuf;
}
