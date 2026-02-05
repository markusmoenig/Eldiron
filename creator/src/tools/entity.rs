use crate::hud::{Hud, HudMode};
use crate::{
    editor::{RUSTERIX, UNDOMANAGER},
    prelude::*,
};
use MapEvent::*;
use rusterix::EntityAction;
use rusterix::prelude::*;
use theframework::prelude::*;

pub struct EntityTool {
    id: TheId,
    hud: Hud,

    drag_state: Option<DragState>,
    move_eps2: f32,
}

#[derive(Clone)]
struct DragState {
    target: DragTarget,
    start_pos: Vec2<f32>,
    changed: bool,
    grab_offset: Vec2<f32>,
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
            hud: Hud::new(HudMode::Entity),

            drag_state: None,
            move_eps2: 0.01, // squared distance in map units to consider as movement
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
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if self.handle_game_click(coord, map) {
                    return None;
                }

                if let Some(click_pos) = self.map_pos_unsnapped(ui, server_ctx, map, coord) {
                    if server_ctx.get_map_context() == MapContext::Region {
                        if let Some(hit) = self.pick_hit(map, click_pos) {
                            map.clear_selection();
                            map.selected_entity_item = Some(hit.id());

                            let grab_offset = hit.pos - click_pos;

                            self.drag_state = Some(DragState {
                                target: hit.target,
                                start_pos: hit.pos,
                                changed: false,
                                grab_offset,
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

                if let Some(state) = self.drag_state.take() {
                    if state.changed {
                        match state.target {
                            DragTarget::Entity(id) => {
                                if let Some(entity) =
                                    map.entities.iter_mut().find(|e| e.creator_id == id)
                                {
                                    // Snap based on final dragged position, not pointer
                                    let snapped = Self::snap_to_grid(
                                        Vec2::new(entity.position.x, entity.position.z),
                                        map.subdivisions,
                                    );
                                    entity.position.x = snapped.x;
                                    entity.position.z = snapped.y;
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
                                    let snapped = Self::snap_to_grid(
                                        Vec2::new(item.position.x, item.position.z),
                                        map.subdivisions,
                                    );
                                    item.position.x = snapped.x;
                                    item.position.z = snapped.y;
                                    server_ctx
                                        .moved_items
                                        .entry(id)
                                        .and_modify(|entry| entry.1 = item.position)
                                        .or_insert((item.position, item.position));
                                }
                            }
                        }
                    }
                }

                self.drag_state = None;
            }
            MapDragged(coord) => {
                if let Some(_render_view) = ui.get_render_view("PolyView") {
                    if let Some(mut state) = self.drag_state.take() {
                        // Keep drag freeform; no snapping while moving
                        let pointer_pos = self
                            .map_pos_unsnapped(ui, server_ctx, map, coord)
                            .unwrap_or(Vec2::new(0.0, 0.0));
                        let mut drag_pos = pointer_pos + state.grab_offset;

                        // Ignore tiny mouse jitter so a pure click doesn't register as a move
                        let delta = drag_pos - state.start_pos;
                        let moved = delta.x * delta.x + delta.y * delta.y > self.move_eps2;
                        if !moved {
                            drag_pos = state.start_pos;
                        }

                        match state.target {
                            DragTarget::Entity(id) => {
                                if let Some(entity) =
                                    map.entities.iter_mut().find(|e| e.creator_id == id)
                                {
                                    if moved {
                                        entity.position.x = drag_pos.x;
                                        entity.position.z = drag_pos.y;
                                        state.changed = true;
                                    }

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
                                    if moved {
                                        item.position.x = drag_pos.x;
                                        item.position.z = drag_pos.y;
                                        state.changed = true;
                                    }

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
                if let Some(hit_pos) = self.map_pos_unsnapped(ui, server_ctx, map, coord) {
                    if server_ctx.get_map_context() == MapContext::Region {
                        if let Some(hit) = self.pick_hit(map, hit_pos) {
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

                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    server_ctx.hover = (None, None, None);
                    let cp = server_ctx.local_to_map_cell(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );
                    server_ctx.hover_cursor = Some(cp);
                }
            }
            _ => {}
        }

        None
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        assets: &Assets,
    ) {
        let id = if !map.selected_linedefs.is_empty() {
            Some(map.selected_linedefs[0])
        } else {
            None
        };
        self.hud.draw(buffer, map, ctx, server_ctx, id, assets);
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
                    if let Some(render_view) = ui.get_render_view("PolyView") {
                        if ctx.ui.has_focus(render_view.id()) {
                            return self.delete_selected(ui, ctx, project, server_ctx);
                        }
                    }
                }
            }
            _ => {}
        }

        false
    }
}

impl EntityTool {
    /// Convert screen coords to map space without snapping so clicking doesn't move things
    fn map_pos_unsnapped(
        &self,
        ui: &mut TheUI,
        _server_ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
    ) -> Option<Vec2<f32>> {
        ui.get_render_view("PolyView").map(|render_view| {
            let dim = *render_view.dim();
            let grid_space_pos = Vec2::new(coord.x as f32, coord.y as f32)
                - Vec2::new(dim.width as f32, dim.height as f32) / 2.0
                - Vec2::new(map.offset.x, -map.offset.y);

            grid_space_pos / map.grid_size
        })
    }

    /// Snap a map position to the current grid/subdivision
    fn snap_to_grid(pos: Vec2<f32>, subdivisions: f32) -> Vec2<f32> {
        if subdivisions > 1.0 {
            Vec2::new(
                (pos.x * subdivisions).round() / subdivisions,
                (pos.y * subdivisions).round() / subdivisions,
            )
        } else {
            Vec2::new(pos.x.round(), pos.y.round())
        }
    }

    fn pick_hit(&self, map: &Map, pos: Vec2<f32>) -> Option<Hit> {
        // Allow picking even when subdivisions differ; use a small radius in map units
        let radius2 = 0.16; // about 0.4 cell diameter

        if let Some(entity) = map.entities.iter().find(|e| {
            let d = e.get_pos_xz() - pos;
            d.x * d.x + d.y * d.y < radius2
        }) {
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

        if let Some(item) = map.items.iter().find(|i| {
            let d = i.get_pos_xz() - pos;
            d.x * d.x + d.y * d.y < radius2
        }) {
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
