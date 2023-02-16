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
}