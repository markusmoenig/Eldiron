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

/// A graphical/text consumer borrows this snapshot without owning a server,
/// advancing time or draining addressed outputs. Render settings are plain CPU data.
pub struct RuntimeView<'a> {
    /// Ephemeral incarnation, unchanged by polling/pause/GPU recovery. Restart
    /// and document replacement revoke input even when serialized IDs match.
    pub session_id: theframework_core::Uuid,
    project: &'a Project,
    pub assets: &'a Assets,
    pub map: Option<&'a Map>,
    pub time: TheTime,
    pub state: ServerState,
    pub render_state: RuntimeRenderState,
}
impl RuntimeView<'_> {
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
    /// Addressed output consumers borrow the matching region's CPU state.
    /// Off-screen audio/dialog delivery must not resolve entity IDs in whichever
    /// map happens to be visible in the current game screen.
    pub fn view_region<'a>(
        &self,
        project: &'a Project,
        server: &rusterix::Server,
        assets: &'a Assets,
        id: &theframework_core::Uuid,
    ) -> Option<RuntimeView<'a>> {
        let map = &project.regions.iter().find(|r| &r.map.id == id)?.map;
        Some(RuntimeView {
            session_id: self.session_id(),
            project,
            assets,
            map: Some(map),
            time: server.get_time(id).unwrap_or(project.time),
            state: server.state,
            render_state: server.get_render_state(id),
        })
    }
    pub fn view<'a>(
        &self,
        project: &'a Project,
        server: &rusterix::Server,
        assets: &'a Assets,
        current_map: &str,
    ) -> RuntimeView<'a> {
        let map = project
            .regions
            .iter()
            .find(|r| r.map.name == current_map)
            .map(|r| &r.map);
        RuntimeView {
            session_id: self.session_id(),
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
        view.time = self.project.time;
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
        assert!(
            session
                .view_region(&theframework_core::Uuid::new_v4())
                .is_none()
        );
        assert_eq!(session.project.time, before);
        assert_eq!(session.current_map, "Second");
    }
}
