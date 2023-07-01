use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Message {
    Quit(),
    Status(String),
    CreatePlayer(Uuid, CharacterInstanceData),
    CreatePlayerInstance(Uuid, Position),
    DestroyPlayerInstance(Uuid),
    ExecutePlayerAction(Uuid, Uuid, PlayerAction),
    PlayerUpdate(Uuid, GameUpdate),
    TransferCharacter(Uuid, BehaviorInstance, Sheet),
    CharacterHasBeenTransferredInsidePool(Uuid, Uuid),
    SetDebugBehaviorId(Uuid),
    DebugData(BehaviorDebugData),

    // User Management
    AddUserToLobby(User),
    RemoveUserFromLobby(Uuid),
}