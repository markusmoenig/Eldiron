use crate::Entity;
use theframework::prelude::*;

/// Messages to the Region
#[derive(Debug)]
pub enum Command {
    CreateEntity(Uuid, Entity),
}
