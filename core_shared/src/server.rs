use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerCmd {
    NoOp,
    LoginAnonymous,
    GameUpdate(GameUpdate),
    GameCmd(String)
}

impl ServerCmd {

    pub fn to_json(&self) -> Option<String> {
        if let Some(json) = serde_json::to_string(self).ok() {
            Some(json)
        } else {
            None
        }
    }

    pub fn to_bin(&self) -> Option<Vec<u8>> {
        if let Ok(bin) = bincode::serialize(&self) {
            Some(bin)
        } else {
            None
        }
    }

    pub fn from_bin(bin: &[u8]) -> Option<Self> {
        if let Ok(data) = bincode::deserialize(&bin) {
            Some(data)
        } else {
            None
        }
    }
}