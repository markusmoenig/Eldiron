pub mod asset;
pub mod actions;
pub mod regiondata;
pub mod characterdata;
pub mod update;
pub mod message;
pub mod property;
pub mod lightdata;
pub mod undo;
pub mod scope_buffer;
pub mod value;
pub mod structs;
pub mod script;
pub mod inventory;

pub mod prelude {
    pub use crate::asset::*;
    pub use crate::asset::tilemap::*;
    pub use crate::asset::tileset::*;
    pub use crate::asset::image::*;

    pub use crate::actions::*;
    pub use crate::regiondata::*;
    pub use crate::characterdata::*;
    pub use crate::update::*;
    pub use crate::message::*;
    pub use crate::property::*;
    pub use crate::lightdata::*;
    pub use crate::undo::*;
    pub use crate::scope_buffer::*;
    pub use crate::structs::*;
    pub use crate::script::*;
    pub use crate::inventory::*;

    pub use crate::value::Value;
    pub use rustc_hash::FxHashMap;

    pub use uuid::Uuid;
    pub use std::collections::HashMap;
    pub use serde::{Deserialize, Serialize};
}
