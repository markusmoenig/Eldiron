use crate::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum MessageType {
    Status,
    Say,
    Yell,
    Tell,
    Debug,
    Error,
    Vendor,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct MessageData {
    pub message_type        : MessageType,
    pub message             : String,
    pub from                : String,
    #[serde(skip)]
    pub right               : Option<String>,
    #[serde(skip)]
    pub center              : Option<String>,
    #[serde(skip)]
    pub buffer              : Option<(usize, usize, Vec<u8>)>
}

impl MessageData {
    pub fn new(message_type: MessageType, message: String, from: String) -> Self {
        Self {
            message_type,
            message,
            from,
            right           : None,
            center          : None,
            buffer          : None
        }
    }
}

/// Represents a multi choice item
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MultiChoiceData {
    pub id                      : Uuid,
    pub header                  : String,
    pub text                    : String,
    pub answer                  : String,
    pub pos                     : Option<(usize, usize)>,
    pub buffer                  : Option<(usize, usize, Vec<u8>)>,
    // For inventory items
    pub item_behavior_id        : Option<Uuid>,
    pub item_amount             : Option<usize>,
    pub item_price              : Option<Currency>,
}

/// Represents communication between a player and an npc
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerCommunication {
    pub player_index            : usize,
    pub npc_index               : usize,
    pub npc_behavior_id         : (Uuid, Uuid),
    pub player_answer           : Option<String>,

    pub start_time              : Date,
    pub end_time                : Date,
}