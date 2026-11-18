use super::*;
use eldiron_network::{
    CharacterSummary, ClientMessage, HostMode, PhysicsContent, RejectCode, Secret, ServerMessage,
};
use eldiron_runtime::{
    MotionClient, MotionSettings,
    replication::{RegionContentRef, WorldUpdate},
};
use shared::project_play::{LaunchPlan, LaunchPurpose, MultiplayerSettings, ProjectPlaySettings};
use std::collections::HashMap;
pub(super) struct OnlineSession {
    settings: MultiplayerSettings,
    link: Option<online::Link>,
    started: bool,
    content_revision: String,
    pub(super) mode: Option<HostMode>,
    pub(super) account: bool,
    account_management: bool,
    pub(super) characters: Vec<CharacterSummary>,
    selected: usize,
    content: HashMap<RegionContentRef, PhysicsContent>,
    resume: Option<Secret>,
    request: Option<Uuid>,
    pending_baseline: bool,
    pub(super) playing: bool,
    play_screen: String,
    pub(super) status: String,
    actions: eldiron_network::ActionClient,
    input_cadence: eldiron_network::InputCadence,
}
impl OnlineSession {
    pub fn for_project(project: &Project) -> std::result::Result<Option<Self>, String> {
        let LaunchPlan::Remote(settings) =
            ProjectPlaySettings::parse(&project.config)?.launch(LaunchPurpose::Player)?
        else {
            return Ok(None);
        };
        let player_settings = eldiron_runtime::PlayerSettings::parse(&project.config)?;
        let play_screen = if !player_settings.play_screen.is_empty() {
            player_settings.play_screen
        } else if project.screens.values().any(|s| {
            s.map.name == player_settings.start_screen
                && rusterix::client::screen_model::has_widget_role(&s.map, "game")
        }) {
            player_settings.start_screen
        } else {
            String::new()
        };
        Ok(Some(Self {
            link: None,
            started: false,
            settings,
            content_revision: if let Some(release) = &project.client_release {
                release.manifest.content_revision.clone()
            } else {
                eldiron_network::project_content_revision(project)?
            },
            mode: None,
            account: false,
            account_management: false,
            characters: vec![],
            selected: 0,
            content: HashMap::new(),
            resume: None,
            request: None,
            pending_baseline: false,
            playing: false,
            play_screen,
            status: "Connecting…".into(),
            actions: Default::default(),
            input_cadence: Default::default(),
        }))
    }
    fn send(&self, message: ClientMessage) -> std::result::Result<(), String> {
        self.link
            .as_ref()
            .ok_or("Disconnected; reconnect to continue")?
            .send(message)
    }
    pub(super) fn supports_account_management(&self) -> bool {
        self.account_management
    }
    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }
    pub fn request_baseline(&mut self) {
        if !self.pending_baseline {
            let _ = self.send(ClientMessage::Resync);
            self.pending_baseline = true;
        }
    }
}
impl PlayerApplication {
    /// Bounded content/gameplay queue diagnostics for online loading and support.
    pub fn online_content_diagnostics(&self) -> Option<eldiron_network::ContentStreamDiagnostics> {
        self.online
            .as_ref()?
            .link
            .as_ref()
            .map(|link| link.diagnostics())
    }

    pub(super) fn online_command(&mut self, command: &str) -> std::result::Result<(), String> {
        let Some(mut online) = self.online.take() else {
            return Ok(());
        };
        let username = self
            .screen
            .elements
            .ui_state
            .get("online.username")
            .cloned()
            .unwrap_or_default();
        let password = self
            .screen
            .elements
            .ui_state
            .get("online.password")
            .cloned()
            .unwrap_or_default();
        let chosen_name = self
            .screen
            .elements
            .ui_state
            .get("start.name")
            .cloned()
            .unwrap_or_else(|| "Adventurer".into());
        let result = (|| -> std::result::Result<(), String> {
            match command {
                "connect" => {
                    online.link.take();
                    online.actions.disconnect();
                    online.input_cadence = Default::default();
                    online.link = Some(online::Link::open(
                        online.settings.clone(),
                        eldiron_network::public_assets::RegionAssets::for_project(
                            &self.player.runtime.project,
                        )?,
                    ));
                    online.started = true;
                    self.player.reset_remote();
                    online.content.clear();
                    online.mode = None;
                    online.account = false;
                    online.playing = false;
                    online.pending_baseline = false;
                    online.status = "Connecting…".into();
                }
                "login" | "register" => {
                    if online.mode != Some(HostMode::Persistent) {
                        return Err("Wait for an account-enabled server".into());
                    }
                    let password = Secret(password);
                    if command == "login" {
                        online.send(ClientMessage::Login { username, password })?;
                    } else {
                        online.send(ClientMessage::Register { username, password })?;
                    }
                    self.screen
                        .clear_secret_input("online.password", &mut self.fonts);
                    self.pending_clipboard = None;
                    online.status = "Signing in…".into();
                }
                "account" => {
                    if !online.account || !online.account_management {
                        return Err("Log in to a server with account management enabled".into());
                    }
                    online.status = "Password changes sign out every session.".into();
                }
                "account_back" => {}
                "change_password" => {
                    if !online.account || !online.account_management {
                        return Err("Account management is unavailable".into());
                    }
                    let current = self
                        .screen
                        .elements
                        .ui_state
                        .get("online.current_password")
                        .cloned()
                        .unwrap_or_default();
                    let new = self
                        .screen
                        .elements
                        .ui_state
                        .get("online.new_password")
                        .cloned()
                        .unwrap_or_default();
                    let confirm = self
                        .screen
                        .elements
                        .ui_state
                        .get("online.confirm_password")
                        .cloned()
                        .unwrap_or_default();
                    self.clear_account_secrets();
                    if !(12..=256).contains(&new.len()) || !(12..=256).contains(&current.len()) {
                        return Err("Passwords must contain 12–256 UTF-8 bytes.".into());
                    }
                    if new != confirm {
                        return Err("The new passwords do not match. Enter them again.".into());
                    }
                    online.send(ClientMessage::ChangePassword {
                        current_password: Secret(current),
                        new_password: Secret(new),
                    })?;
                    online.status = "Changing password…".into();
                }
                "logout_all" => {
                    self.clear_account_secrets();
                    if !online.account || !online.account_management {
                        return Err("Account management is unavailable".into());
                    }
                    online.send(ClientMessage::LogoutAll)?;
                    online.status = "Signing out all sessions…".into();
                }
                "start" | "create_character" => {
                    let name = chosen_name;
                    match online.mode {
                        Some(HostMode::Demo) => {
                            online.send(ClientMessage::Guest { name })?;
                            online.status = "Joining demo…".into();
                        }
                        Some(HostMode::Persistent) if online.account => {
                            if command == "start"
                                && let Some(character) = online.characters.get(online.selected)
                            {
                                online.send(ClientMessage::JoinCharacter {
                                    character: character.id,
                                })?;
                            } else {
                                let request = *online.request.get_or_insert_with(Uuid::new_v4);
                                online.send(ClientMessage::CreateCharacter { request, name })?;
                            }
                            online.status = "Joining world…".into();
                        }
                        _ => return Err("Log in before selecting a character".into()),
                    }
                }
                "next_character" => {
                    if !online.characters.is_empty() {
                        online.selected = (online.selected + 1) % online.characters.len();
                        online.status = format!(
                            "Selected: {} — press Play",
                            online.characters[online.selected].name
                        );
                    }
                }
                "logout" => {
                    self.clear_account_secrets();
                    let _ = online.send(ClientMessage::Logout);
                    online.link.take();
                    online.resume = None;
                    online.actions = Default::default();
                    online.account = false;
                    online.playing = false;
                    online.status = "Signed out".into();
                }
                _ => return Err(format!("Unsupported online command '{command}'")),
            }
            Ok(())
        })();
        if let Err(error) = &result {
            online.status = error.clone();
        }
        let account_navigation = if result.is_ok() && command == "account" {
            Some(lobby::ACCOUNT.to_string())
        } else if command == "account_back" {
            Some(if online.playing {
                online.play_screen.clone()
            } else {
                lobby::NAME.into()
            })
        } else {
            None
        };
        self.online = Some(online);
        if let Some(name) = account_navigation {
            self.clear_account_secrets();
            self.navigate(&name)?;
        }
        if matches!(command, "logout" | "connect") {
            self.player.reset_remote();
            self.navigate(lobby::NAME)?;
        }
        self.refresh_online_status();
        Ok(())
    }
    pub(super) fn send_online_inputs(
        &mut self,
        elapsed: Duration,
    ) -> std::result::Result<(), String> {
        let queued = self.player.take_remote_actions();
        let mut notification = None;
        let Some(online) = &mut self.online else {
            return Ok(());
        };
        let Some(client) = self.player.remote() else {
            return Ok(());
        };
        if let (Some(map), Some(actor)) = (
            client.authority().current_map(),
            client.authority().controlled_entity(),
        ) {
            for action in queued {
                if let Err(error) =
                    online
                        .actions
                        .enqueue_action(&action, map, actor, Some(&self.screen.intent))
                {
                    notification = Some(error);
                }
            }
        }
        if let Some(message) = online.actions.advance(elapsed) {
            if let Err(error) = online.send(message) {
                online.status = error;
            }
        }
        if client.needs_baseline() {
            online.request_baseline();
        } else if let Some(message) = online
            .input_cadence
            .advance(elapsed, client.pending_inputs())
        {
            if let Err(error) = online.send(message) {
                online.status = error;
            }
        }
        if let Some(error) = notification {
            self.online_action_warning(error);
        }
        Ok(())
    }
    pub(super) fn poll_online(&mut self) -> std::result::Result<(), String> {
        let Some(mut online) = self.online.take() else {
            return Ok(());
        };
        if !online.started {
            online.link = Some(online::Link::open(
                online.settings.clone(),
                eldiron_network::public_assets::RegionAssets::for_project(
                    &self.player.runtime.project,
                )?,
            ));
            online.started = true;
        }
        let mut navigate = None;
        for _ in 0..64 {
            let Some(event) = online.link.as_ref().and_then(|link| link.poll()) else {
                break;
            };
            let result = (|| -> std::result::Result<(), String> {
                match event {
                    online::Event::Failed(error) => {
                        self.clear_account_secrets();
                        online.link.take();
                        online.actions.disconnect();
                        online.input_cadence = Default::default();
                        self.player.reset_remote();
                        online.playing = false;
                        online.account = false;
                        online.status = error;
                        navigate = Some(lobby::NAME.into());
                    }
                    online::Event::Region(region) => {
                        shared::public_client::streaming::install_region(
                            &mut self.player.runtime.project,
                            &mut self.player.runtime.assets,
                            region,
                        )?;
                    }
                    online::Event::Content(hint, content) => {
                        online.content.retain(|key, _| {
                            key.map != hint.reference.map || *key == hint.reference
                        });
                        online.content.insert(hint.reference, content);
                    }
                    online::Event::Packet(message) => match message {
                        ServerMessage::AccountUpdated(update) => {
                            self.clear_account_secrets();
                            match update {
                                eldiron_network::AccountUpdate::Rejected(code) => {
                                    online.status=match code{RejectCode::InvalidCredentials=>"Current password was incorrect or authorization changed. Enter it again.".into(),_=>format!("Account update rejected: {code:?}")};
                                }
                                success => {
                                    online.resume = None;
                                    online.account = false;
                                    online.characters.clear();
                                    online.link.take();
                                    online.actions.disconnect();
                                    online.playing = false;
                                    online.request = None;
                                    online.input_cadence = Default::default();
                                    self.player.reset_remote();
                                    online.status = if success
                                        == eldiron_network::AccountUpdate::PasswordChanged
                                    {
                                        "Password changed. Log in with your new password.".into()
                                    } else {
                                        "All sessions signed out. Log in again to continue.".into()
                                    };
                                    navigate = Some(lobby::NAME.into());
                                }
                            }
                        }

                        ServerMessage::ActionStream {
                            stream,
                            next_sequence,
                        } => online.actions.stream_ready(stream, next_sequence),
                        ServerMessage::Offers {
                            world,
                            generation,
                            region,
                            offers,
                        } => online.actions.offers(world, generation, region, offers),
                        ServerMessage::ActionReply(reply) => match online.actions.reply(&reply) {
                            Some(eldiron_network::ActionResult::Outcome(
                                eldiron_network::RequestOutcome::Denied(error),
                            )) => self.online_action_warning(error),
                            Some(
                                eldiron_network::ActionResult::Conflict
                                | eldiron_network::ActionResult::OutOfOrder
                                | eldiron_network::ActionResult::AlreadyCommitted,
                            ) => {
                                self.online_action_warning(
                                    eldiron_network::RequestError::StaleContext,
                                );
                                if let Some(link) = &online.link {
                                    link.send(ClientMessage::Resync)
                                        .map_err(|e| e.to_string())?;
                                }
                            }
                            _ => {}
                        },
                        ServerMessage::Welcome {
                            version,
                            mode,
                            content_revision,
                            account_management,
                        } => {
                            if version != eldiron_network::PROTOCOL_VERSION
                                || content_revision != online.content_revision
                            {
                                return Err("Server and project content differ; install the matching game release".into());
                            }
                            online.account_management = account_management;
                            online.mode = Some(mode);
                            online.status = if mode == HostMode::Demo {
                                "Online demo — choose a name and press Play".into()
                            } else {
                                "Enter username and password to log in or create an account".into()
                            };
                            if let Some(token) = online.resume.take()
                                && mode == HostMode::Persistent
                            {
                                online.send(ClientMessage::Resume { token })?;
                            }
                        }
                        ServerMessage::Authorized {
                            token, characters, ..
                        } => {
                            online.account = true;
                            online.resume = Some(token);
                            online.characters = characters;
                            online.selected = 0;
                            self.screen
                                .clear_secret_input("online.password", &mut self.fonts);
                            self.pending_clipboard = None;
                            online.status = online
                                .characters
                                .first()
                                .map(|c| format!("Selected: {} — press Play", c.name))
                                .unwrap_or_else(|| "Choose a character name and press Play".into());
                        }
                        ServerMessage::Characters(characters) => {
                            online.characters = characters;
                            online.selected = 0;
                        }
                        ServerMessage::CharacterCreated { character, .. } => {
                            online.request = None;
                            if !online.characters.iter().any(|c| c.id == character) {
                                online.characters.push(CharacterSummary {
                                    id: character,
                                    name: self
                                        .screen
                                        .elements
                                        .ui_state
                                        .get("start.name")
                                        .cloned()
                                        .unwrap_or_else(|| "Character".into()),
                                });
                            }
                        }
                        ServerMessage::Frame { frame, .. } => {
                            if self.player.remote().is_none() {
                                self.player.attach_remote(MotionClient::new(
                                    frame.world,
                                    frame.connection,
                                    MotionSettings::default(),
                                ))?;
                            }
                            // Geometry and physics for one immutable reference
                            // install before its baseline becomes visible.
                            for (reference, package) in &online.content {
                                let mut map = self
                                    .player
                                    .runtime
                                    .project
                                    .regions
                                    .iter()
                                    .find(|r| r.map.id == reference.map)
                                    .ok_or("Missing packaged client region")?
                                    .map
                                    .clone();
                                package.geometry.apply(&mut map);
                                self.player
                                    .remote_mut()
                                    .unwrap()
                                    .install_content(*reference, &map)
                                    .map_err(|e| e.to_string())?;
                            }
                            if let WorldUpdate::Baseline(regions) = &frame.update {
                                for region in regions {
                                    self.player.install_remote_content(region.content)?;
                                }
                            }
                            for (reference, content) in online.content.drain() {
                                self.player
                                    .remote_mut()
                                    .unwrap()
                                    .install_authoritative_collision(
                                        reference,
                                        content.mini,
                                        content.collision,
                                    );
                            }
                            self.player.receive_remote_frame(&frame)?;
                            online.actions.observe_frame(&frame);
                            if matches!(frame.update, WorldUpdate::Baseline(_)) {
                                online.pending_baseline = false;
                            }
                            if frame.controlled_entity.is_some() && !online.playing {
                                online.playing = true;
                                navigate = Some(online.play_screen.clone());
                            }
                        }
                        ServerMessage::Rejected {
                            code: RejectCode::InputResync,
                        } => {
                            if let Some(client) = self.player.remote_mut() {
                                client.resume();
                            }
                            online.request_baseline();
                        }
                        ServerMessage::Rejected { code } => {
                            online.status = format!("Server: {code:?}");
                            if code == RejectCode::InvalidCredentials {
                                online.resume = None;
                            }
                        }
                        _ => {}
                    },
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.clear_account_secrets();
                online.link.take();
                online.actions.disconnect();
                self.player.reset_remote();
                online.playing = false;
                online.account = false;
                online.status = error;
                navigate = Some(lobby::NAME.into());
                break;
            }
        }
        self.online = Some(online);
        if let Some(name) = navigate {
            self.navigate(&name)?;
        }
        self.refresh_online_status();
        Ok(())
    }
    fn online_action_warning(&mut self, error: eldiron_network::RequestError) {
        use eldiron_network::RequestError as E;
        let message = match error {
            E::OutOfRange => "You are too far away.",
            E::ExpiredOffer | E::StaleContext => "That interaction is no longer available.",
            E::NotControlled => "Wait until your character has joined.",
            E::Unsupported => "That action is not available online.",
            E::NotFound => "That target is no longer available.",
            _ => "You cannot do that.",
        };
        let world = self.player.view();
        let events = eldiron_runtime::RegionEvents {
            messages: vec![(
                None,
                world.player_entity().map(|e| e.id),
                0,
                message.into(),
                "warning".into(),
            )],
            ..Default::default()
        };
        self.screen.receive(&world, &events);
    }
    pub(super) fn clear_account_secrets(&mut self) {
        for name in [
            "online.password",
            "online.current_password",
            "online.new_password",
            "online.confirm_password",
        ] {
            self.screen.clear_secret_input(name, &mut self.fonts);
        }
        self.pending_clipboard = None;
    }
    fn refresh_online_status(&mut self) {
        let Some(online) = &self.online else {
            return;
        };
        for widget in self.screen.elements.text_widgets.values_mut() {
            if widget.name == "OnlineStatus" {
                widget.text = online.status.clone();
            }
        }
        if online.account && online.account_management {
            self.screen
                .elements
                .widgets_to_hide
                .retain(|n| n != "OnlineAccount");
        } else if !self
            .screen
            .elements
            .widgets_to_hide
            .iter()
            .any(|n| n == "OnlineAccount")
        {
            self.screen
                .elements
                .widgets_to_hide
                .push("OnlineAccount".into());
        }
        let demo = online.mode == Some(HostMode::Demo);
        for name in [
            "OnlineUsername",
            "OnlinePassword",
            "OnlineUsernameLabel",
            "OnlinePasswordLabel",
            "OnlineLogin",
            "OnlineRegister",
            "OnlineNext",
            "OnlineCreate",
        ] {
            if demo {
                if !self
                    .screen
                    .elements
                    .widgets_to_hide
                    .iter()
                    .any(|s| s == name)
                {
                    self.screen.elements.widgets_to_hide.push(name.into());
                }
            } else {
                self.screen.elements.widgets_to_hide.retain(|s| s != name);
            }
        }
    }
}
