use crate::{GameSession, Project};
use rusterix::{Assets, Map, ServerState, server::message::RuntimeRenderState};
use theframework_core::TheTime;

/// Borrowed source bytes for a graphical text owner.
pub struct FontSource<'a> {
    pub id: theframework_core::Uuid,
    pub name: &'a str,
    pub bytes: &'a [u8],
}

pub struct ScreenSource<'a> {
    pub id: theframework_core::Uuid,
    pub name: &'a str,
    pub map: &'a Map,
    pub responsive: bool,
}

/// CPU inspection of an addressed region. This does not borrow Assets or the
/// mutable server, so an authoring controller can inspect a session while its
/// unrelated asset/edit services remain available. No ticking or output drain.
#[derive(Clone, Copy)]
pub struct RegionInspection<'a> {
    pub session_id: theframework_core::Uuid,
    pub tick_sequence: u64,
    /// Authoring region identity, independent of a generated runtime map ID.
    pub region_id: theframework_core::Uuid,
    pub map: &'a Map,
    pub state: ServerState,
    pub time: TheTime,
}

/// A graphical/text consumer borrows this snapshot without owning a server,
/// advancing time or draining addressed outputs. Render settings are plain CPU data.
pub struct RuntimeView<'a> {
    /// Ephemeral view incarnation, stable across ordinary polling/pause/GPU
    /// recovery. Restart and a new baseline generation revoke old input scopes.
    pub session_id: theframework_core::Uuid,
    pub player: rusterix::party::PlayerView,
    project: &'a Project,
    pub assets: &'a Assets,
    pub map: Option<&'a Map>,
    pub time: TheTime,
    pub state: ServerState,
    pub render_state: RuntimeRenderState,
}
impl RuntimeView<'_> {
    pub(crate) fn from_replica<'a>(
        session_id: theframework_core::Uuid,
        controlled: Option<u32>,
        project: &'a Project,
        assets: &'a Assets,
        map: Option<&'a Map>,
        time: TheTime,
        state: ServerState,
        render_state: RuntimeRenderState,
    ) -> RuntimeView<'a> {
        RuntimeView {
            session_id,
            player: rusterix::party::PlayerView::controlled(controlled),
            project,
            assets,
            map,
            time,
            state,
            render_state,
        }
    }
    pub fn player_entity(&self) -> Option<&rusterix::Entity> {
        self.player.player(self.map?)
    }

    pub fn party_entity<'a>(
        &self,
        map: &'a Map,
        binding: Option<&str>,
    ) -> Option<&'a rusterix::Entity> {
        self.player.resolve(map, binding)
    }
    /// Creator may edit screen layouts/fonts/config while a session is running.
    /// Those presentation sources can stay live without borrowing authored maps
    /// as simulation storage. Map, time, state and incarnation stay unchanged.
    pub fn with_presentation_sources<'a>(self, sources: &'a Project) -> RuntimeView<'a>
    where
        Self: 'a,
    {
        RuntimeView {
            project: sources,
            ..self
        }
    }
    pub fn config(&self) -> &str {
        &self.project.config
    }
    pub fn screens(&self) -> impl Iterator<Item = ScreenSource<'_>> {
        self.project.screens.values().map(|screen| ScreenSource {
            id: screen.id,
            name: &screen.map.name,
            map: &screen.map,
            responsive: screen.is_responsive(),
        })
    }
    pub fn screen(&self, name: &str) -> Option<ScreenSource<'_>> {
        self.screens().find(|screen| screen.name == name)
    }
    /// Borrow raw authored fonts; a graphical text owner registers them with
    /// Parley/Swash. Merely reading the world never initializes a font rasterizer.
    pub fn fonts(&self) -> impl Iterator<Item = FontSource<'_>> {
        self.project
            .assets
            .values()
            .filter_map(|asset| match &asset.buffer {
                shared::asset::AssetBuffer::Font(bytes) => Some(FontSource {
                    id: asset.id,
                    name: &asset.name,
                    bytes,
                }),
                _ => None,
            })
    }
}
impl crate::SessionPump {
    pub fn inspect_region<'a>(
        &self,
        project: &'a Project,
        server: &rusterix::Server,
        id: &theframework_core::Uuid,
    ) -> Option<RegionInspection<'a>> {
        let region = project.regions.iter().find(|r| &r.map.id == id)?;
        let map = &region.map;
        Some(RegionInspection {
            session_id: self.session_id(),
            tick_sequence: self.tick_sequence(),
            region_id: region.id,
            map,
            state: server.state,
            time: server.get_time(id).unwrap_or(project.time),
        })
    }
    /// Addressed output consumers borrow the matching region's CPU state.
    /// Off-screen audio/dialog delivery must not resolve entity IDs in whichever
    /// map happens to be visible in the current game screen.
    pub fn view_region<'a>(
        &'a self,
        project: &'a Project,
        server: &rusterix::Server,
        assets: &'a Assets,
        id: &theframework_core::Uuid,
    ) -> Option<RuntimeView<'a>> {
        if let Some(client) = self.client_world(server.local_connection()) {
            return client.view(project, assets, Some(*id));
        }
        let map = &project.regions.iter().find(|r| &r.map.id == id)?.map;
        Some(RuntimeView {
            session_id: self.session_id(),
            player: server.local_player_id().map_or(
                rusterix::party::PlayerView::Legacy,
                rusterix::party::PlayerView::Controlled,
            ),
            project,
            assets,
            map: Some(map),
            time: server.get_time(id).unwrap_or(project.time),
            state: server.state,
            render_state: server.get_render_state(id),
        })
    }
    pub fn view<'a>(
        &'a self,
        project: &'a Project,
        server: &rusterix::Server,
        assets: &'a Assets,
        current_map: &str,
    ) -> RuntimeView<'a> {
        if let Some(client) = self.client_world(server.local_connection()) {
            return client
                .view(project, assets, None)
                .unwrap_or_else(|| client.empty_view(project, assets));
        }
        let map = project
            .regions
            .iter()
            .find(|r| r.map.name == current_map)
            .map(|r| &r.map);
        RuntimeView {
            session_id: self.session_id(),
            player: server.local_player_id().map_or(
                rusterix::party::PlayerView::Legacy,
                rusterix::party::PlayerView::Controlled,
            ),
            project,
            assets,
            map,
            time: map
                .and_then(|map| server.get_time(&map.id))
                .unwrap_or(project.time),
            state: server.state,
            render_state: map.map_or_else(
                || server.world_render.clone(),
                |map| server.get_render_state(&map.id),
            ),
        }
    }
}

impl GameSession {
    /// Independent CPU view of one connection's current region and character.
    /// Gameplay data is owned by the recipient's replica, not the simulation.
    pub fn view_for_connection(
        &self,
        connection: crate::PlayerConnectionId,
    ) -> Result<RuntimeView<'_>, String> {
        if let Some(client) = self.client_world(connection) {
            return Ok(client
                .view(&self.project, &self.assets, None)
                .unwrap_or_else(|| client.empty_view(&self.project, &self.assets)));
        }
        if self.started {
            return Err("Client replica is not ready".into());
        }
        let state = self
            .server
            .connection_state(connection)
            .ok_or("Unknown or expired player connection")?;
        let region_id = match state {
            crate::PlayerConnectionState::Active(control) => Some(control.region_id),
            crate::PlayerConnectionState::Spawning { region_id } => Some(region_id),
            crate::PlayerConnectionState::AwaitingCharacter => None,
        };
        let name = region_id
            .and_then(|id| {
                self.server
                    .region_name_id_map
                    .iter()
                    .find_map(|(name, region)| (*region == id).then_some(name.as_str()))
            })
            .unwrap_or("");
        let mut view = self
            .pump
            .view(&self.project, &self.server, &self.assets, name);
        view.session_id = connection.uuid();
        view.player = rusterix::party::PlayerView::controlled(
            state.control().map(|control| control.entity_id),
        );
        Ok(view)
    }
    pub fn inspect_region(&self, id: &theframework_core::Uuid) -> Option<RegionInspection<'_>> {
        self.pump.inspect_region(&self.project, &self.server, id)
    }
    pub fn view_region(&self, id: &theframework_core::Uuid) -> Option<RuntimeView<'_>> {
        self.pump
            .view_region(&self.project, &self.server, &self.assets, id)
    }
    pub fn view(&self) -> RuntimeView<'_> {
        let mut view = self
            .pump
            .view(&self.project, &self.server, &self.assets, &self.current_map);
        // GameSession owns runtime document time; borrowed authoring hosts use
        // the pump's server-time snapshot without changing authored time.
        if !self.started {
            view.time = self.project.time;
        }
        view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn addressed_region_view_does_not_use_the_current_maps_entity_ids_or_advance_time() {
        let mut project = Project::new();
        project.regions[0].map.name = "First".into();
        let mut second = project.regions[0].clone();
        second.map.id = theframework_core::Uuid::new_v4();
        second.map.name = "Second".into();
        project.regions.push(second);
        let first = project.regions[0].map.id;
        let second = project.regions[1].map.id;
        let mut session = GameSession::new(project);
        session.current_map = "Second".into();
        let before = session.project.time;
        assert_eq!(session.view().map.unwrap().id, second);
        let addressed = session.view_region(&first).unwrap();
        assert_eq!(addressed.map.unwrap().id, first);
        assert_eq!(addressed.time, before);
        let mut live_sources = Project::new();
        live_sources.config = "live screen configuration".into();
        live_sources.time = TheTime::new_time(23, 59).unwrap();
        let live = addressed.with_presentation_sources(&live_sources);
        assert_eq!(live.config(), "live screen configuration");
        assert_eq!(live.map.unwrap().id, first);
        assert_eq!(live.time, before);
        assert_eq!(live.session_id, session.view().session_id);
        let inspection = session
            .pump
            .inspect_region(&session.project, &session.server, &first)
            .unwrap();
        // The addressed map borrow does not retain server/assets. Editing an
        // unrelated asset or changing server state cannot rewrite that snapshot.
        session.assets.config = "unrelated authoring edit".into();
        session.server.state = ServerState::Paused;
        assert_eq!(inspection.map.id, first);
        assert_eq!(inspection.region_id, session.project.regions[0].id);
        assert!(inspection.state == ServerState::Off);
        assert_eq!(inspection.tick_sequence, 0);
        assert_eq!(inspection.time, before);
        assert!(
            session
                .view_region(&theframework_core::Uuid::new_v4())
                .is_none()
        );
        assert_eq!(session.project.time, before);
        assert_eq!(session.current_map, "Second");
    }
}
