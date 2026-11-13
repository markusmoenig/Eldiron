//! CPU session ownership for Creator. Native Play owns GameSession; the explicit
//! reference display adapter still lends its legacy facade's CPU storage.
use eldiron_runtime::{
    EventInterests, GameSession, PlayerInput, SessionClock, SessionParts, SessionPump,
    SessionUpdate,
};
use rusterix::{Assets, EntityAction, Rusterix, Server, ServerState};
use shared::project::Project;
use theframework_core::Uuid;
use web_time::{Duration, Instant};

struct ReferenceSession {
    project: Project,
    pump: SessionPump,
    clock: SessionClock,
}
enum Storage {
    Owned(GameSession),
    Reference(ReferenceSession),
}
pub(super) struct CreatorPlaySession {
    pub document: Uuid,
    storage: Storage,
    pub input: PlayerInput,
    pub requested: bool,
    last: Instant,
    visual_key: u64,
    audio: Option<rusterix::audio::output::AudioOutput>,
    appearances: rusterix::procedural_avatar::AppearancePreparation,
}
impl CreatorPlaySession {
    pub fn new(
        document: Uuid,
        project: Project,
        tick_ms: u64,
        controls: rusterix::input_action::ClientAction,
        requested: bool,
    ) -> Self {
        Self {
            document,
            storage: Storage::Reference(ReferenceSession {
                project,
                pump: SessionPump::new(EventInterests::default()),
                clock: SessionClock::new(Duration::from_millis(tick_ms.max(1))),
            }),
            input: PlayerInput::from_controls(controls),
            requested,
            last: Instant::now(),
            visual_key: 0,
            audio: None,
            appearances: Default::default(),
        }
    }
    pub fn start_owned(
        document: Uuid,
        project: Project,
        tick_ms: u64,
        quick_start: bool,
        bridge: &mut Rusterix,
    ) -> Result<Self, String> {
        let mut game = GameSession::new(project);
        game.start(eldiron_runtime::SessionOptions {
            debug: true,
            create_local_player: false,
            ..Default::default()
        })?;
        game.clock = SessionClock::new(Duration::from_millis(tick_ms.max(1)));
        // CPU client setup remains an explicit presentation/start-menu adapter.
        // It never receives the authoritative Server and cannot advance it.
        bridge.assets = game.assets.clone();
        bridge.set_block_props(game.project.block_props.clone());
        let mut commands = shared::rusterix_utils::setup_client_sources(bridge, &mut game.project);
        if quick_start {
            commands.extend(bridge.start_with_entity_defaults());
        }
        game.assets = bridge.assets.clone();
        game.current_map = bridge.client.current_map.clone();
        game.submit_startup_commands(commands);
        for _ in 0..3 {
            game.tick_once();
        }
        bridge.client.set_server_time(game.project.time);
        let mut appearances = rusterix::procedural_avatar::AppearancePreparation::default();
        appearances.prepare(game.server.entities.values().flatten(), &game.assets);
        let requested = game.has_started_player();
        let visual_key = Self::visual_key(&bridge.assets);
        Ok(Self {
            document,
            storage: Storage::Owned(game),
            input: PlayerInput::from_controls(bridge.client.player_input_snapshot()),
            requested,
            last: Instant::now(),
            visual_key,
            audio: Some(Default::default()),
            appearances,
        })
    }
    pub fn wake_after(&self) -> Option<Duration> {
        let tick = match &self.storage {
            Storage::Owned(game) if game.server.state == ServerState::Running => {
                Some(game.clock.until_tick())
            }
            _ => None,
        };
        tick.into_iter()
            .chain(self.audio.as_ref().and_then(|audio| audio.wake_after()))
            .min()
    }
    #[cfg(test)]
    pub fn clock_step(&self) -> Duration {
        match &self.storage {
            Storage::Owned(game) => game.clock.step(),
            Storage::Reference(game) => game.clock.step(),
        }
    }
    pub fn audio_suspended(&mut self, suspended: bool) {
        if let Some(audio) = &mut self.audio {
            audio.set_suspended(suspended);
        }
    }
    pub fn audio_user_gesture(&mut self) {
        if let Storage::Owned(game) = &self.storage
            && let Some(audio) = &mut self.audio
        {
            audio.user_gesture(&game.assets, self.visual_key);
        }
    }
    pub fn dispatch_audio(&mut self, commands: &[rusterix::server::message::AudioCommand]) -> bool {
        if let Storage::Owned(game) = &self.storage
            && let Some(audio) = &mut self.audio
        {
            audio.dispatch(&game.assets, self.visual_key, commands);
            true
        } else {
            false
        }
    }
    #[cfg(test)]
    pub fn audio_stats(&self) -> Option<rusterix::audio::output::AudioOutputStats> {
        self.audio.as_ref().map(|audio| audio.stats())
    }
    pub fn owned(&self) -> bool {
        matches!(self.storage, Storage::Owned(_))
    }
    pub fn project(&self) -> &Project {
        match &self.storage {
            Storage::Owned(g) => &g.project,
            Storage::Reference(g) => &g.project,
        }
    }
    pub fn project_mut(&mut self) -> &mut Project {
        match &mut self.storage {
            Storage::Owned(g) => &mut g.project,
            Storage::Reference(g) => &mut g.project,
        }
    }
    pub fn pump(&self) -> &SessionPump {
        match &self.storage {
            Storage::Owned(g) => g.driver(),
            Storage::Reference(g) => &g.pump,
        }
    }
    pub fn pump_mut(&mut self) -> &mut SessionPump {
        match &mut self.storage {
            Storage::Owned(g) => g.parts().pump,
            Storage::Reference(g) => &mut g.pump,
        }
    }
    pub fn assets<'a>(&'a self, fallback: &'a Assets) -> &'a Assets {
        match &self.storage {
            Storage::Owned(g) => &g.assets,
            _ => fallback,
        }
    }
    pub fn parts<'a>(
        &'a mut self,
        server: &'a mut Server,
        assets: &'a mut Assets,
    ) -> SessionParts<'a> {
        match &mut self.storage {
            Storage::Owned(g) => g.parts(),
            Storage::Reference(g) => SessionParts {
                project: &mut g.project,
                pump: &mut g.pump,
                server,
                assets,
            },
        }
    }
    pub fn server<'a>(&'a self, fallback: &'a Server) -> &'a Server {
        match &self.storage {
            Storage::Owned(g) => &g.server,
            _ => fallback,
        }
    }
    pub fn server_mut<'a>(&'a mut self, fallback: &'a mut Server) -> &'a mut Server {
        match &mut self.storage {
            Storage::Owned(g) => &mut g.server,
            _ => fallback,
        }
    }
    pub fn state(&self, fallback: &Server) -> ServerState {
        self.server(fallback).state
    }
    pub fn view<'a>(
        &'a self,
        bridge: &'a Rusterix,
        sources: &'a Project,
    ) -> eldiron_runtime::RuntimeView<'a> {
        match &self.storage {
            Storage::Owned(g) => g.view(),
            Storage::Reference(g) => g.pump.view(
                &g.project,
                &bridge.server,
                &bridge.assets,
                &bridge.client.current_map,
            ),
        }
        .with_presentation_sources(sources)
    }
    /// The authoring viewport can inspect a different region from the player.
    /// Keep its speech anchors/time addressed to the displayed runtime map.
    pub fn view_region<'a>(
        &'a self,
        bridge: &'a Rusterix,
        sources: &'a Project,
        map: &Uuid,
    ) -> Option<eldiron_runtime::RuntimeView<'a>> {
        match &self.storage {
            Storage::Owned(game) => game.view_region(map),
            Storage::Reference(game) => game.pump.view_region(
                &game.project, &bridge.server, &bridge.assets, map,
            ),
        }
        .map(|world| world.with_presentation_sources(sources))
    }
    pub fn inspect_region<'a>(
        &'a self,
        server: &Server,
        region: Uuid,
    ) -> Option<eldiron_runtime::RegionInspection<'a>> {
        let map = &self.project().get_region(&region)?.map;
        match &self.storage {
            Storage::Owned(g) => g.inspect_region(&map.id),
            Storage::Reference(g) => g.pump.inspect_region(&g.project, server, &map.id),
        }
    }
    pub fn suspend_clock(&mut self) {
        match &mut self.storage {
            Storage::Owned(g) => g.clock.reset_elapsed(),
            Storage::Reference(g) => g.clock.reset_elapsed(),
        }
        self.last = Instant::now();
    }
    fn visual_key(assets: &Assets) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut key = std::collections::hash_map::DefaultHasher::new();
        assets.visual_source_revision.hash(&mut key);
        assets.avatars.revision().hash(&mut key);
        assets.locales_src.hash(&mut key);
        assets.audio_fx_src.hash(&mut key);
        for (name, source) in &assets.procedural_avatars {
            name.hash(&mut key);
            (std::sync::Arc::as_ptr(source) as usize).hash(&mut key);
        }
        assets
            .entities
            .iter()
            .for_each(|entry| entry.hash(&mut key));
        assets.items.iter().for_each(|entry| entry.hash(&mut key));
        assets.tiles_revision.hash(&mut key);
        assets.block_props_revision.hash(&mut key);
        assets.config.hash(&mut key);
        assets.rules.hash(&mut key);
        assets.authoring_src.hash(&mut key);
        serde_json::to_vec(&assets.palette)
            .expect("serializable palette")
            .hash(&mut key);
        key.finish()
    }
    pub fn advance_elapsed(
        &mut self,
        elapsed: Duration,
        authored: &Project,
        bridge: &mut Rusterix,
    ) -> Result<SessionUpdate, String> {
        let update = match &mut self.storage {
            Storage::Owned(g) => {
                // Explicit live authoring publication. Runtime actors/maps/time
                // remain owned here; CPU visual source caches may be refreshed.
                if g.assets.rules != bridge.assets.rules {
                    let rules = bridge
                        .assets
                        .rules
                        .parse()
                        .map_err(|error| format!("Rules publication: {error}"))?;
                    g.server.publish_rules(rules);
                }
                let key = Self::visual_key(&bridge.assets);
                if self.visual_key != key {
                    g.assets.publish_visual_sources_from(&bridge.assets);
                    g.assets.visual_source_revision = key;
                    g.project
                        .rules_document
                        .clone_from(&authored.rules_document);
                    g.project.authoring.clone_from(&authored.authoring);
                    g.project.config.clone_from(&authored.config);
                    g.project.locales.clone_from(&authored.locales);
                    self.visual_key = key;
                }
                let update = g.advance(elapsed);
                bridge.client.current_map.clone_from(&g.current_map);
                bridge.client.set_server_time(g.project.time);
                self.appearances
                    .prepare(g.server.entities.values().flatten(), &g.assets);
                update
            }
            Storage::Reference(g) => {
                let update = g.pump.advance(
                    &mut g.clock,
                    elapsed,
                    &mut g.project,
                    &mut bridge.server,
                    &mut bridge.assets,
                    &mut bridge.client.current_map,
                );
                if let Some(time) = update.time {
                    g.project.time = time;
                }
                bridge.prepare_runtime_appearances();
                update
            }
        };
        if let Storage::Owned(game) = &self.storage
            && let Some(audio) = &mut self.audio
        {
            audio.update(&game.assets, self.visual_key, elapsed);
        }
        self.finish_input(bridge)?;
        Ok(update)
    }
    fn finish_input(&mut self, bridge: &mut Rusterix) -> Result<(), String> {
        let isometric = self.project().regions.iter().find(|r| r.map.name == bridge.client.current_map)
            .and_then(|r|r.map.entities.iter().find(|e|e.is_player()))
            .is_some_and(|e|matches!(e.attributes.get("player_camera"),Some(rusterix::Value::PlayerCamera(camera)) if camera.is_isometric_2d()));
        if let Some(action) = self.input.controls.set_isometric_controls(isometric) {
            self.submit_action(&mut bridge.server, action)?;
        }
        let server = match &mut self.storage {
            Storage::Owned(g) => &mut g.server,
            _ => &mut bridge.server,
        };
        if server.state == ServerState::Running {
            self.input.flush(server);
        }
        Ok(())
    }
    pub fn advance(&mut self, bridge: &mut Rusterix) -> SessionUpdate {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let Storage::Reference(g) = &mut self.storage else {
            unreachable!("owned sessions advance from host time");
        };
        let update = g.pump.advance(
            &mut g.clock,
            elapsed,
            &mut g.project,
            &mut bridge.server,
            &mut bridge.assets,
            &mut bridge.client.current_map,
        );
        if let Some(time) = update.time {
            g.project.time = time;
        }
        bridge.prepare_runtime_appearances();
        let _ = self.finish_input(bridge);
        update
    }
    pub fn submit_action(
        &mut self,
        fallback: &mut Server,
        action: EntityAction,
    ) -> Result<(), String> {
        let server = match &mut self.storage {
            Storage::Owned(g) => &mut g.server,
            _ => fallback,
        };
        if server.state != ServerState::Running && action != EntityAction::Off {
            return Ok(());
        }
        if let Err(error) = self.input.submit_action(server, self.requested, action) {
            self.input.controls.cancel_controls();
            let _ = self
                .input
                .submit_action(server, self.requested, EntityAction::Off);
            return Err(error);
        }
        Ok(())
    }
    pub fn cancel_input(&mut self, server: &mut Server) {
        let stop = self.input.controls.cancel_controls();
        let _ = self.submit_action(server, stop);
    }
    pub fn start_player(
        &mut self,
        bridge: &mut Rusterix,
        options: eldiron_runtime::PlayerStartOptions<'_>,
    ) -> Result<(), String> {
        match &mut self.storage {
            Storage::Owned(g) => {
                if let Some(info) = g.start_player(options)? {
                    self.input.controls.configure(&info, &g.assets);
                    self.requested = true;
                }
            }
            Storage::Reference(_) => {
                let map = bridge
                    .assets
                    .maps
                    .get(&bridge.client.current_map)
                    .ok_or("Player start map is unavailable")?;
                let prepared =
                    rusterix::player_start::prepare_player_start(&bridge.assets, map, options)?;
                self.input
                    .controls
                    .configure(&prepared.info, &bridge.assets);
                bridge
                    .server
                    .process_client_commands(vec![prepared.command]);
                self.requested = true;
            }
        }
        Ok(())
    }
    pub fn flush_command(&mut self, bridge: &mut Rusterix) {
        match &mut self.storage {
            Storage::Owned(g) => {
                g.tick_once();
                bridge.client.current_map.clone_from(&g.current_map);
                bridge.client.set_server_time(g.project.time);
                self.appearances
                    .prepare(g.server.entities.values().flatten(), &g.assets);
            }
            Storage::Reference(g) => {
                let update = g.pump.update(
                    1,
                    true,
                    &mut g.project,
                    &mut bridge.server,
                    &mut bridge.assets,
                    &mut bridge.client.current_map,
                );
                if let Some(time) = update.time {
                    g.project.time = time;
                }
                bridge.prepare_runtime_appearances();
            }
        }
    }
    pub fn text_runtime<'a>(
        play: Option<&'a mut Self>,
        authored: &'a Project,
        bridge: &'a mut Rusterix,
    ) -> (&'a Project, crate::textplay::TextGameRuntime<'a>) {
        if let Some(play) = play {
            let (project, server, assets, current_map) = match &mut play.storage {
                Storage::Owned(g) => (&mut g.project, &mut g.server, &g.assets, &g.current_map),
                Storage::Reference(g) => (
                    &mut g.project,
                    &mut bridge.server,
                    &bridge.assets,
                    &bridge.client.current_map,
                ),
            };
            for region in &mut project.regions {
                server.apply_entities_items(&mut region.map);
                if region.map.name == *current_map
                    && let Some(time) = server.get_time(&region.map.id)
                {
                    project.time = time;
                }
            }
            project.authoring.clone_from(&authored.authoring);
            project.config.clone_from(&authored.config);
            (
                &*project,
                crate::textplay::TextGameRuntime {
                    server,
                    assets,
                    time: project.time,
                    input: Some((&mut play.input, play.requested)),
                },
            )
        } else {
            (
                authored,
                crate::textplay::TextGameRuntime {
                    server: &mut bridge.server,
                    assets: &bridge.assets,
                    time: bridge.client.server_time,
                    input: None,
                },
            )
        }
    }
    pub fn authoring_map<'a>(
        &self,
        authored: &'a rusterix::Map,
    ) -> std::borrow::Cow<'a, rusterix::Map> {
        let Some(region) = self
            .project()
            .regions
            .iter()
            .find(|r| r.map.id == authored.id)
        else {
            return std::borrow::Cow::Borrowed(authored);
        };
        let mut map = authored.clone();
        map.entities.clone_from(&region.map.entities);
        map.items.clone_from(&region.map.items);
        for prop in &mut map.block_prop_instances {
            if let Some(runtime) = region
                .map
                .block_prop_instances
                .iter()
                .find(|p| p.id == prop.id)
            {
                prop.runtime_state.clone_from(&runtime.runtime_state);
            }
        }
        std::borrow::Cow::Owned(map)
    }
}

impl Drop for CreatorPlaySession {
    fn drop(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.reset();
        }
    }
}
