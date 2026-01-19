use crate::{
    editor::{RUSTERIX, UNDOMANAGER},
    prelude::*,
};
use MapEvent::*;
use rusterix::EntityAction;
use theframework::prelude::*;

pub struct EntityTool {
    id: TheId,

    drag_state: Option<DragState>,
}

#[derive(Clone)]
struct DragState {
    target: DragTarget,
    start_pos: Vec2<f32>,
    changed: bool,
}

#[derive(Clone, Copy)]
enum DragTarget {
    Entity(Uuid),
    Item(Uuid),
}

impl Tool for EntityTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Entity Tool"),

            drag_state: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        fl!("tool_entity")
    }
    fn icon_name(&self) -> String {
        str!("treasure-chest")
    }
    fn accel(&self) -> Option<char> {
        Some('Y')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/entity".to_string())
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            ToolEvent::Activate => {
                server_ctx.curr_map_tool_type = MapToolType::General;

                true
            }
            ToolEvent::DeActivate => true,
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(coord) => {
                if self.handle_game_click(coord, map) {
                    return None;
                }

                if let Some(grid_pos) = self.map_cell(ui, server_ctx, map, coord, 1.0) {
                    if server_ctx.get_map_context() == MapContext::Region {
                        if let Some(hit) = self.pick_hit(map, grid_pos) {
                            map.clear_selection();
                            map.selected_entity_item = Some(hit.id());

                            self.drag_state = Some(DragState {
                                target: hit.target,
                                start_pos: hit.pos,
                                changed: false,
                            });

                            // Record original positions for movement tracking
                            match hit.target {
                                DragTarget::Entity(id) => {
                                    if let Some(entity) =
                                        map.entities.iter().find(|e| e.creator_id == id)
                                    {
                                        server_ctx
                                            .moved_entities
                                            .entry(id)
                                            .or_insert((entity.position, entity.position));
                                    }
                                }
                                DragTarget::Item(id) => {
                                    if let Some(item) =
                                        map.items.iter().find(|i| i.creator_id == id)
                                    {
                                        server_ctx
                                            .moved_items
                                            .entry(id)
                                            .or_insert((item.position, item.position));
                                    }
                                }
                            }

                            self.select_in_tree(ui, server_ctx, hit.id());
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                            RUSTERIX.write().unwrap().set_dirty();
                        }
                    }
                }
            }
            MapUp(coord) => {
                if self.handle_game_up(coord, map) {
                    return None;
                }

                self.drag_state = None;
            }
            MapDragged(coord) => {
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    if let Some(mut state) = self.drag_state.take() {
                        let dim = *render_view.dim();
                        let mut drag_pos = server_ctx.local_to_map_cell(
                            Vec2::new(dim.width as f32, dim.height as f32),
                            Vec2::new(coord.x as f32, coord.y as f32),
                            map,
                            map.subdivisions,
                        );
                        drag_pos += map.subdivisions * 0.5;

                        match state.target {
                            DragTarget::Entity(id) => {
                                if let Some(entity) =
                                    map.entities.iter_mut().find(|e| e.creator_id == id)
                                {
                                    entity.position.x = drag_pos.x;
                                    entity.position.z = drag_pos.y;
                                    state.changed = state.changed
                                        || state.start_pos.x != drag_pos.x
                                        || state.start_pos.y != drag_pos.y;

                                    server_ctx
                                        .moved_entities
                                        .entry(id)
                                        .and_modify(|entry| entry.1 = entity.position)
                                        .or_insert((entity.position, entity.position));
                                }
                            }
                            DragTarget::Item(id) => {
                                if let Some(item) =
                                    map.items.iter_mut().find(|i| i.creator_id == id)
                                {
                                    item.position.x = drag_pos.x;
                                    item.position.z = drag_pos.y;
                                    state.changed = state.changed
                                        || state.start_pos.x != drag_pos.x
                                        || state.start_pos.y != drag_pos.y;

                                    server_ctx
                                        .moved_items
                                        .entry(id)
                                        .and_modify(|entry| entry.1 = item.position)
                                        .or_insert((item.position, item.position));
                                }
                            }
                        }

                        self.drag_state = Some(state);
                    }
                }
            }
            MapHover(coord) => {
                if let Some(grid_pos) = self.map_cell(ui, server_ctx, map, coord, 1.0) {
                    if server_ctx.get_map_context() == MapContext::Region {
                        if let Some(hit) = self.pick_hit(map, grid_pos) {
                            ctx.ui
                                .send(TheEvent::SetStatusText(TheId::empty(), hit.status_text()));
                        } else {
                            ctx.ui
                                .send(TheEvent::SetStatusText(TheId::empty(), "".into()));
                        }
                    } else {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), "".into()));
                    }
                }
            }
            _ => {}
        }

        None
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        #[allow(clippy::single_match)]
        match event {
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                if *code == TheKeyCode::Delete {
                    return self.delete_selected(ui, ctx, project, server_ctx);
                }
            }
            _ => {}
        }

        false
    }
}

impl EntityTool {
    fn map_cell(
        &self,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
        subdivisions: f32,
    ) -> Option<Vec2<f32>> {
        ui.get_render_view("PolyView").map(|render_view| {
            let dim = *render_view.dim();
            server_ctx.local_to_map_cell(
                Vec2::new(dim.width as f32, dim.height as f32),
                Vec2::new(coord.x as f32, coord.y as f32),
                map,
                subdivisions,
            )
        })
    }

    fn pick_hit(&self, map: &Map, grid_pos: Vec2<f32>) -> Option<Hit> {
        if let Some(entity) = map
            .entities
            .iter()
            .find(|e| e.get_pos_xz().floor() == grid_pos)
        {
            return Some(Hit {
                target: DragTarget::Entity(entity.creator_id),
                name: entity
                    .attributes
                    .get_str("name")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Entity".into()),
                pos: Vec2::new(entity.position.x, entity.position.z),
            });
        }

        if let Some(item) = map
            .items
            .iter()
            .find(|i| i.get_pos_xz().floor() == grid_pos)
        {
            return Some(Hit {
                target: DragTarget::Item(item.creator_id),
                name: item
                    .attributes
                    .get_str("name")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Item".into()),
                pos: Vec2::new(item.position.x, item.position.z),
            });
        }

        None
    }

    fn delete_selected(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some(selected) = project
            .get_map_mut(server_ctx)
            .and_then(|map| map.selected_entity_item)
        else {
            return false;
        };

        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
            if let Some(index) = region.characters.get_index_of(&selected) {
                if let Some(character) = region.characters.get(&selected).cloned() {
                    let atom = ProjectUndoAtom::RemoveRegionCharacterInstance(
                        index,
                        server_ctx.curr_region,
                        character,
                    );
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    return true;
                }
            }

            if let Some(index) = region.items.get_index_of(&selected) {
                if let Some(item) = region.items.get(&selected).cloned() {
                    let atom = ProjectUndoAtom::RemoveRegionItemInstance(
                        index,
                        server_ctx.curr_region,
                        item,
                    );
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    return true;
                }
            }
        }

        false
    }

    fn select_in_tree(&self, ui: &mut TheUI, server_ctx: &ServerContext, id: Uuid) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(node) = tree_layout.get_node_by_id_mut(&server_ctx.curr_region) {
                node.new_item_selected(&TheId::named_with_id("Region Content List Item", id));
            }
        }
    }

    fn handle_game_click(&self, coord: Vec2<i32>, map: &mut Map) -> bool {
        let mut rusterix = RUSTERIX.write().unwrap();
        let is_running = rusterix.server.state == rusterix::ServerState::Running;

        if is_running {
            if let Some(action) = rusterix.client.touch_down(coord, map) {
                rusterix.server.local_player_action(action);
            }
            return true;
        }
        false
    }

    fn handle_game_up(&self, coord: Vec2<i32>, map: &mut Map) -> bool {
        let mut rusterix = RUSTERIX.write().unwrap();
        let is_running = rusterix.server.state == rusterix::ServerState::Running;

        if is_running {
            rusterix.client.touch_up(coord, map);
            rusterix.server.local_player_action(EntityAction::Off);
            return true;
        }
        false
    }
}

struct Hit {
    target: DragTarget,
    name: String,
    pos: Vec2<f32>,
}

impl Hit {
    fn id(&self) -> Uuid {
        match self.target {
            DragTarget::Entity(id) | DragTarget::Item(id) => id,
        }
    }

    fn status_text(&self) -> String {
        let prefix = match self.target {
            DragTarget::Entity(_) => "Entity",
            DragTarget::Item(_) => "Item",
        };
        format!("{prefix}: {}", self.name)
    }
}
