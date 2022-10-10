use crate::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum MessageType {
    Status,
    Say,
    Yell,
    Private,
    Debug,
    Error,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct MessageData {
    pub message_type        : MessageType,
    pub message             : String,
    pub from                : String,
    pub buffer              : Option<(usize, usize, Vec<u8>)>
}

/// Represents a multi choice item
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MultiChoiceData {
    pub id                      : Uuid,
    pub header                  : String,
    pub text                    : String,
    pub answer                  : String,
    pub pos                     : Option<(usize, usize)>,
    pub buffer                  : Option<(usize, usize, Vec<u8>)>
}