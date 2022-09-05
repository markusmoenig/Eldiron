use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Message {
    Quit(),
    Status(String),
    CreatePlayerInstance(Uuid, Position),
    ExecutePlayerAction(Uuid, Uuid, PlayerAction),
    PlayerUpdate(Uuid, GameUpdate),
    TransferCharacter(Uuid, BehaviorInstance),
    CharacterHasBeenTransferredInsidePool(Uuid, Uuid),
    SetDebugBehaviorId(Uuid),
    DebugData(BehaviorDebugData),
}