//! Versioned remote boundary. Only explicit public DTOs cross this crate.
//! Runtime/Creator sessions keep their existing in-process path.
#[cfg(target_arch = "wasm32")]
pub mod browser;
pub mod codec;
pub mod content;
mod content_stream;
mod content_task;
pub use content::{ContentCache, ContentLoader, RegionGeometry};
pub use content_stream::{ContentDelivery, ContentStream, ContentStreamDiagnostics};
pub mod gameplay;
mod input_cadence;
mod text_requests;
pub use input_cadence::InputCadence;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod public_asset_gate;
pub mod public_assets;
pub mod public_client;
pub mod remote;
pub use gameplay::*;

use eldiron_runtime::{ClientFrame, MovementCommand, replication::RegionContentRef};
use serde::{Deserialize, Serialize};
use theframework_core::Uuid;

pub const PROTOCOL_VERSION: u16 = 4;
pub const SUBPROTOCOL: &str = "eldiron.v4";
pub const MAX_CLIENT_BYTES: usize = 64 * 1024;
pub const MAX_SERVER_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CONTENT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SOCKET_QUEUE: usize = 64;

/// No Debug implementation: accidental request logging must not reveal secrets.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(pub String);
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ClientMessage {
    Hello {
        version: u16,
    },
    Guest {
        name: String,
    },
    Register {
        username: String,
        password: Secret,
    },
    Login {
        username: String,
        password: Secret,
    },
    Resume {
        token: Secret,
    },
    Characters,
    CreateCharacter {
        request: Uuid,
        name: String,
    },
    JoinCharacter {
        character: Uuid,
    },
    Inputs {
        frames: Vec<MovementCommand>,
    },
    Resync,
    Action(GameAction),
    Logout,
    LogoutAll,
    ChangePassword {
        current_password: Secret,
        new_password: Secret,
    },
    ExternalLogin {
        provider: String,
        current_password: Option<Secret>,
    },
    CancelExternalLogin,
    UnlinkExternal {
        provider: String,
        current_password: Secret,
    },
    Ping {
        nonce: u32,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostMode {
    Demo,
    Persistent,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub id: Uuid,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityProvider {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentHint {
    pub reference: RegionContentRef,
    pub physics_revision: u64,
    pub key: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        version: u16,
        mode: HostMode,
        content_revision: String,
        #[serde(default)]
        account_management: bool,
        #[serde(default)]
        gameplay_extensions: bool,
        #[serde(default)]
        identity_providers: Vec<IdentityProvider>,
    },
    Authorized {
        account: Uuid,
        token: Secret,
        characters: Vec<CharacterSummary>,
    },
    AccountUpdated(AccountUpdate),
    ExternalChallenge {
        provider: String,
        verification_uri: String,
        user_code: Secret,
        expires_in: u32,
    },
    Characters(Vec<CharacterSummary>),
    CharacterCreated {
        request: Uuid,
        character: Uuid,
    },
    Frame {
        frame: ClientFrame,
        content: Vec<ContentHint>,
    },
    /// Optional immutable packages; no live objects or region subscription.
    Preload {
        content: Vec<ContentHint>,
    },
    Rejected {
        code: RejectCode,
    },
    ActionStream {
        stream: Uuid,
        next_sequence: u64,
    },
    ActionReply(ActionReply),
    Offers {
        world: Uuid,
        generation: u64,
        region: Option<Uuid>,
        offers: Vec<OfferedChoice>,
    },
    Pong {
        nonce: u32,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountUpdate {
    PasswordChanged,
    LoggedOutEverywhere,
    IdentityLinked,
    IdentityUnlinked,
    Rejected(RejectCode),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectCode {
    WrongVersion,
    WrongMode,
    AuthenticationRequired,
    InvalidCredentials,
    InvalidRequest,
    Unavailable,
    CharacterUnavailable,
    AlreadyPlaying,
    InputResync,
    RateLimited,
    ServerBusy,
}

/// Public compiled region geometry/physics. No authored Project, scripts, VM state,
/// templates, credentials, inventory or arbitrary properties are exported.
#[derive(Clone, Serialize, Deserialize)]
pub struct PhysicsContent {
    pub reference: RegionContentRef,
    pub physics_revision: u64,
    pub name: String,
    pub mini: rusterix::map::mini::MapMini,
    pub collision: rusterix::collision_world::CollisionWorld,
    pub geometry: RegionGeometry,
}

/// Connection metadata can differ between a self-hosted/managed client export
/// and its world host. Match game content after the same ruleset migration,
/// without coupling the gameplay fingerprint to its deployment address. Durable
/// saves retain their separate, stricter project/schema identity.
pub fn project_content_revision(project: &eldiron_runtime::Project) -> Result<String, String> {
    let mut canonical = project.clone();
    canonical.migrate_default_ruleset();
    let mut config = canonical
        .config
        .parse::<toml::Table>()
        .map_err(|e| e.to_string())?;
    if let Some(multiplayer) = config.remove("multiplayer") {
        let mut gameplay = toml::Table::new();
        for key in ["gameplay", "logout"] {
            if let Some(value) = multiplayer.get(key) {
                gameplay.insert(key.into(), value.clone());
            }
        }
        if !gameplay.is_empty() {
            config.insert("multiplayer".into(), gameplay.into());
        }
    }
    if let Some(game) = config.get_mut("game").and_then(toml::Value::as_table_mut) {
        game.remove("mode");
    }
    canonical.config = toml::to_string(&config).map_err(|e| e.to_string())?;
    eldiron_runtime::persistence::content_revision(&canonical).map_err(|e| e.to_string())
}
#[cfg(test)]
mod project_revision_tests {
    #[test]
    fn deployment_addresses_do_not_change_game_content_identity() {
        let mut project = eldiron_runtime::Project::default();
        project.config = "[game]\nmode='single_player'\ngame_tick_ms=250".into();
        let original = super::project_content_revision(&project).unwrap();
        project.config="[game]\nmode='multiplayer'\ngame_tick_ms=250\n[multiplayer]\nendpoint='wss://managed.example/play'\ndeployment_id='world-a'".into();
        assert_eq!(super::project_content_revision(&project).unwrap(), original);
        project.config = project.config.replace("250", "200");
        assert_ne!(super::project_content_revision(&project).unwrap(), original);
        project
            .config
            .push_str("\n[multiplayer.gameplay]\nallowed_cameras=['2d']");
        let policy = super::project_content_revision(&project).unwrap();
        project.config = project
            .config
            .replace("allowed_cameras=['2d']", "allowed_cameras=['firstp']");
        assert_ne!(
            super::project_content_revision(&project).unwrap(),
            policy,
            "gameplay permissions belong to content identity"
        );
    }
}
