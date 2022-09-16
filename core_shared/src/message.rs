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
