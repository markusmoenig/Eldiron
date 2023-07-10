use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Message {
    Quit(),
    Status(String),
    CreatePlayer(Uuid, Option<String>, CharacterInstanceData),
    CreatePlayerInstance(Uuid, Position),
    DestroyPlayerInstance(Uuid),
    ExecutePlayerAction(Uuid, Uuid, PlayerAction),
    PlayerUpdate(Uuid, GameUpdate),
    TransferCharacter(Uuid, BehaviorInstance, Sheet),
    SaveCharacter(Uuid, String, Sheet),
    CharacterHasBeenTransferredInsidePool(Uuid, Uuid),
    SetDebugBehaviorId(Uuid),
    DebugData(BehaviorDebugData),

    // User Management
    AddUserToLobby(User),
    RemoveUserFromLobby(Uuid),
    SetUserName(Uuid, String),
    SetUserScreenName(Uuid, String),
}