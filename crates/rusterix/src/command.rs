use crate::Entity;
use theframework::prelude::*;

/// Messages to the Region / server runtime.
#[derive(Debug)]
pub enum Command {
    CreateEntity(Uuid, Entity),
}
