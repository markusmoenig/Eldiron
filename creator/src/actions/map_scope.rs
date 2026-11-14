//! Action targets are independent of the selected child editor context.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionMapScope {
    Selected,
    RegionWorld,
}
impl ActionMapScope {
    pub fn map<'a>(self, project: &'a Project, server: &CreatorContext) -> Option<&'a Map> {
        match self {
            Self::Selected => project.get_map(&server.shared),
            Self::RegionWorld => project
                .get_region(&server.shared.curr_region)
                .map(|region| &region.map),
        }
    }
    pub fn map_mut<'a>(
        self,
        project: &'a mut Project,
        server: &CreatorContext,
    ) -> Option<&'a mut Map> {
        match self {
            Self::Selected => project.get_map_mut(&server.shared),
            Self::RegionWorld => project
                .get_region_mut(&server.shared.curr_region)
                .map(|region| &mut region.map),
        }
    }
    pub fn undo_context(self, server: &CreatorContext) -> ProjectContext {
        match self {
            Self::Selected => server.shared.pc,
            Self::RegionWorld => ProjectContext::Region(server.shared.curr_region),
        }
    }
}
