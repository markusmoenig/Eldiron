use crate::prelude::*;

pub struct SetEditingSurface {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for SetEditingSurface {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_set_edit_surface_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_set_edit_surface")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_set_edit_surface_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'u'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        map.selected_sectors.len() == 1 && server_ctx.editor_view_mode != EditorViewMode::D2
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if let Some(sector_id) = map.selected_sectors.first().cloned() {
            let mut profile_to_add = None;

            if let Some(surface) = map.get_surface_for_sector_id_mut(sector_id) {
                // If there is no profile yet for the surface we add one
                if surface.profile.is_none() {
                    let profile = Map::default();
                    surface.profile = Some(profile.id);
                    profile_to_add = Some(profile);
                }

                let mut surface = surface.clone();
                if let Some(sector) = map.find_sector(sector_id) {
                    if let Some(vertices) = sector.vertices_world(map) {
                        surface.world_vertices = vertices;
                    }
                }

                server_ctx.editing_surface = Some(surface.clone());
                server_ctx.editor_view_mode = EditorViewMode::D2;
            }

            if let Some(profile_to_add) = profile_to_add {
                map.profiles.insert(profile_to_add.id, profile_to_add);
            }

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render SceneManager Map"),
                TheValue::Empty,
            ));

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Backup Editing Position"),
                TheValue::Empty,
            ));
        }

        None
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
