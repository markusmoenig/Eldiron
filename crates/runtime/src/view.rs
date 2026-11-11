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
    project: &'a Project,
    pub assets: &'a Assets,
    pub map: Option<&'a Map>,
    pub time: TheTime,
    pub state: ServerState,
    pub render_state: RuntimeRenderState,
}
impl RuntimeView<'_> {
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
impl GameSession {
    pub fn view(&self) -> RuntimeView<'_> {
        let map = self
            .project
            .regions
            .iter()
            .find(|r| r.map.name == self.current_map)
            .map(|r| &r.map);
        RuntimeView {
            project: &self.project,
            assets: &self.assets,
            map,
            time: self.project.time,
            state: self.server.state,
            render_state: map.map_or_else(
                || self.server.world_render.clone(),
                |map| self.server.get_render_state(&map.id),
            ),
        }
    }
}
