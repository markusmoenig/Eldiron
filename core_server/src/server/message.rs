use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Message {
    Quit(),
    Status(String),
    CreatePlayerInstance(Uuid, Position),
    ExecutePlayerAction(Uuid, usize, PlayerAction),
}