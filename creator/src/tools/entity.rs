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
    last_reference_y: Option<f32>,
    last_floor_y: Option<f32>,
    last_support_surface: Option<SupportSurfaceSample>,
}

#[derive(Clone, Copy)]
enum DragTarget {
    Entity(Uuid),
    Item(Uuid),
}

#[derive(Clone, Copy)]
struct PlacementSample {
    pos: Vec2<f32>,
    reference_y: Option<f32>,
    floor_y: Option<f32>,
    support_surface: Option<SupportSurfaceSample>,
}

#[derive(Clone, Copy)]
pub(crate) struct SupportSurfaceSample {
    pub instance_id: Uuid,
    pub surface_id: Uuid,
    pub local_position: Vec3<f32>,
    pub world_position: Vec3<f32>,
}

impl DragTarget {
    fn placement_clearance(self) -> f32 {
        match self {
            DragTarget::Entity(_) => 2.0,
            DragTarget::Item(_) => 1.0,
        }
    }
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
        str!("shapes")
    }
    fn accel(&self) -> Option<char> {
        Some('Y')
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
            MapKey(c) => {
                let dir = match c {
                    'q' | 'Q' => Some(-1.0_f32),
                    'e' | 'E' => Some(1.0_f32),
                    _ => None,
                };
                if let Some(step) = dir
                    && let Some(selected_id) = map.selected_entity_item
                    && let Some(entity) = map
                        .entities
                        .iter_mut()
                        .find(|e| e.creator_id == selected_id)
                {
                    let from = Self::snap_cardinal(entity.orientation);
                    let to = if step < 0.0 {
                        Vec2::new(-from.y, from.x)
                    } else {
                        Vec2::new(from.y, -from.x)
                    };
                    entity.orientation = to;
                    server_ctx
                        .rotated_entities
                        .entry(selected_id)
                        .and_modify(|entry| entry.1 = to)
                        .or_insert((from, to));
                    RUSTERIX.write().unwrap().set_dirty();
                }
            }
            MapClicked(coord) => {
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                if self.handle_game_click(coord, map) {
                    return None;
                }

                if server_ctx.get_map_context() == MapContext::Region
                    && let Some(hit) = self.pick_hit_for_coord(ui, server_ctx, map, coord)
                {
                    let placement = self.placement_sample_at_coord(
                        ui,
                        server_ctx,
                        map,
                        coord,
                        hit.target.placement_clearance(),
                    );
                    let click_pos = placement.map(|sample| sample.pos).unwrap_or(hit.pos);

                    map.clear_selection();
                    map.selected_entity_item = Some(hit.id());

                    let grab_offset = hit.pos - click_pos;

                    self.drag_state = Some(DragState {
                        target: hit.target,
                        start_pos: hit.pos,
                        changed: false,
                        grab_offset,
                        last_reference_y: placement.and_then(|sample| sample.reference_y),
                        last_floor_y: placement.and_then(|sample| sample.floor_y),
                        last_support_surface: placement.and_then(|sample| sample.support_surface),
                    });

                    match hit.target {
                        DragTarget::Entity(id) => {
                            if let Some(entity) = map.entities.iter().find(|e| e.creator_id == id) {
                                server_ctx
                                    .moved_entities
                                    .entry(id)
                                    .or_insert((entity.position, entity.position));
                            }
                        }
                        DragTarget::Item(id) => {
                            if let Some(item) = map.items.iter().find(|i| i.creator_id == id) {
                                server_ctx
                                    .moved_items
                                    .entry(id)
                                    .or_insert((item.position, item.position));
                            }
                            let occupant = rusterix::BlockPropOccupant::ItemInstance(id);
                            let placement = map
                                .block_prop_surface_placements
                                .iter()
                                .find(|placement| placement.occupant == occupant)
                                .cloned();
                            server_ctx
                                .moved_item_surface_placements
                                .entry(id)
                                .or_insert((placement.clone(), placement));
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
            MapUp(coord) => {
                if self.handle_game_up(coord, map) {
                    return None;
                }

                if let Some(state) = self.drag_state.take() {
                    if state.changed {
                        match state.target {
                            DragTarget::Entity(id) => {
                                let snapped = map.entities.iter().find(|e| e.creator_id == id).map(
                                    |entity| {
                                        Self::snap_to_grid(
                                            Vec2::new(entity.position.x, entity.position.z),
                                            map.subdivisions,
                                        )
                                    },
                                );
                                let floor_height = snapped.and_then(|pos| {
                                    if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                        Self::placement_floor_height(
                                            map,
                                            pos,
                                            state.last_reference_y,
                                            state.last_floor_y,
                                            state.target.placement_clearance(),
                                        )
                                    } else {
                                        None
                                    }
                                });
                                if let Some(entity) =
                                    map.entities.iter_mut().find(|e| e.creator_id == id)
                                {
                                    if let Some(snapped) = snapped {
                                        entity.position.x = snapped.x;
                                        entity.position.z = snapped.y;
                                    }
                                    if let Some(height) = floor_height {
                                        entity.position.y = height;
                                    }
                                    server_ctx
                                        .moved_entities
                                        .entry(id)
                                        .and_modify(|entry| entry.1 = entity.position)
                                        .or_insert((entity.position, entity.position));
                                }
                            }
                            DragTarget::Item(id) => {
                                let snapped = state
                                    .last_support_surface
                                    .is_none()
                                    .then(|| {
                                        map.items.iter().find(|i| i.creator_id == id).map(|item| {
                                            Self::snap_to_grid(
                                                Vec2::new(item.position.x, item.position.z),
                                                map.subdivisions,
                                            )
                                        })
                                    })
                                    .flatten();
                                let floor_height = state
                                    .last_support_surface
                                    .map(|surface| surface.world_position.y)
                                    .or_else(|| {
                                        snapped.and_then(|pos| {
                                            if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                                Self::placement_floor_height(
                                                    map,
                                                    pos,
                                                    state.last_reference_y,
                                                    state.last_floor_y,
                                                    state.target.placement_clearance(),
                                                )
                                            } else {
                                                None
                                            }
                                        })
                                    });
                                if let Some(item) =
                                    map.items.iter_mut().find(|i| i.creator_id == id)
                                {
                                    if let Some(snapped) = snapped {
                                        item.position.x = snapped.x;
                                        item.position.z = snapped.y;
                                    }
                                    if let Some(height) = floor_height {
                                        item.position.y = height;
                                    }
                                    server_ctx
                                        .moved_items
                                        .entry(id)
                                        .and_modify(|entry| entry.1 = item.position)
                                        .or_insert((item.position, item.position));
                                }
                                if let Err(message) = Self::commit_item_surface_placement(
                                    map,
                                    id,
                                    state.last_support_surface,
                                ) {
                                    if let Some(start) =
                                        server_ctx.moved_items.get(&id).map(|(from, _)| *from)
                                    {
                                        if let Some(item) =
                                            map.items.iter_mut().find(|item| item.creator_id == id)
                                        {
                                            item.position = start;
                                        }
                                        if let Some((_, to)) = server_ctx.moved_items.get_mut(&id) {
                                            *to = start;
                                        }
                                    }
                                    ctx.ui
                                        .send(TheEvent::SetStatusText(TheId::empty(), message));
                                } else if state.last_support_surface.is_some() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        fl!("status_item_placed_on_prefab_surface"),
                                    ));
                                }
                                let occupant = rusterix::BlockPropOccupant::ItemInstance(id);
                                let placement = map
                                    .block_prop_surface_placements
                                    .iter()
                                    .find(|placement| placement.occupant == occupant)
                                    .cloned();
                                if let Some((_, to)) =
                                    server_ctx.moved_item_surface_placements.get_mut(&id)
                                {
                                    *to = placement;
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
                        let placement = self.placement_sample_at_coord(
                            ui,
                            server_ctx,
                            map,
                            coord,
                            state.target.placement_clearance(),
                        );
                        let pointer_pos = placement
                            .map(|sample| sample.pos)
                            .unwrap_or(Vec2::new(0.0, 0.0));
                        let mut drag_pos = pointer_pos + state.grab_offset;
                        let mut placement_reference_y =
                            placement.and_then(|sample| sample.reference_y);
                        let mut placement_floor_y = placement.and_then(|sample| sample.floor_y);
                        let support_surface = placement
                            .and_then(|sample| sample.support_surface)
                            .and_then(|sample| {
                                Self::support_surface_sample_at_world_xz(map, sample, drag_pos)
                            });
                        if let Some(surface) = support_surface {
                            drag_pos =
                                Vec2::new(surface.world_position.x, surface.world_position.z);
                            placement_reference_y = Some(surface.world_position.y);
                            placement_floor_y = Some(surface.world_position.y);
                        }
                        if placement_reference_y.is_some() {
                            state.last_reference_y = placement_reference_y;
                        }
                        if placement_floor_y.is_some() {
                            state.last_floor_y = placement_floor_y;
                        }
                        state.last_support_surface = support_surface;

                        // Ignore tiny mouse jitter so a pure click doesn't register as a move
                        let delta = drag_pos - state.start_pos;
                        let moved = delta.x * delta.x + delta.y * delta.y > self.move_eps2;
                        if !moved {
                            drag_pos = state.start_pos;
                        }

                        match state.target {
                            DragTarget::Entity(id) => {
                                let floor_height =
                                    if moved && server_ctx.editor_view_mode != EditorViewMode::D2 {
                                        Self::placement_floor_height(
                                            map,
                                            drag_pos,
                                            placement_reference_y,
                                            placement_floor_y,
                                            state.target.placement_clearance(),
                                        )
                                    } else {
                                        None
                                    };
                                if let Some(entity) =
                                    map.entities.iter_mut().find(|e| e.creator_id == id)
                                {
                                    if moved {
                                        entity.position.x = drag_pos.x;
                                        entity.position.z = drag_pos.y;
                                        if let Some(height) = floor_height {
                                            entity.position.y = height;
                                        }
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
                                let floor_height =
                                    if moved && server_ctx.editor_view_mode != EditorViewMode::D2 {
                                        state
                                            .last_support_surface
                                            .map(|surface| surface.world_position.y)
                                            .or_else(|| {
                                                Self::placement_floor_height(
                                                    map,
                                                    drag_pos,
                                                    placement_reference_y,
                                                    placement_floor_y,
                                                    state.target.placement_clearance(),
                                                )
                                            })
                                    } else {
                                        None
                                    };
                                if let Some(item) =
                                    map.items.iter_mut().find(|i| i.creator_id == id)
                                {
                                    if moved {
                                        item.position.x = drag_pos.x;
                                        item.position.z = drag_pos.y;
                                        if let Some(height) = floor_height {
                                            item.position.y = height;
                                        }
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
                if server_ctx.get_map_context() == MapContext::Region {
                    if let Some(hit) = self.pick_hit_for_coord(ui, server_ctx, map, coord) {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), hit.status_text()));
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
    fn remove_selected_character_instance(
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        character_id: Uuid,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(region_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id) {
                region_node.remove_widget_by_uuid(&character_id);
            }
        }

        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
            region.characters.shift_remove(&character_id);
            region.map.entities.retain(|e| e.creator_id != character_id);
            region.map.selected_entity_item = None;
        }

        if let Some(region) = project.get_region(&server_ctx.curr_region)
            && let Some(tree_layout) = ui.get_tree_layout("Project Tree")
            && let Some(region_node) = tree_layout.get_node_by_id_mut(&region.id)
        {
            region_node.set_open(true);
        }

        shared::rusterix_utils::insert_content_into_maps(project);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        update_region(ctx);
    }

    fn remove_selected_item_instance(
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        item_id: Uuid,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(region_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id) {
                region_node.remove_widget_by_uuid(&item_id);
            }
        }

        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
            region.items.shift_remove(&item_id);
            region.map.items.retain(|i| i.creator_id != item_id);
            region.map.selected_entity_item = None;
        }

        if let Some(region) = project.get_region(&server_ctx.curr_region)
            && let Some(tree_layout) = ui.get_tree_layout("Project Tree")
            && let Some(region_node) = tree_layout.get_node_by_id_mut(&region.id)
        {
            region_node.set_open(true);
        }

        shared::rusterix_utils::insert_content_into_maps(project);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        update_region(ctx);
    }

    /// Convert screen coords to map space without snapping so clicking doesn't move things
    fn map_pos_unsnapped(
        &self,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
    ) -> Option<Vec2<f32>> {
        self.placement_sample_at_coord(ui, server_ctx, map, coord, 1.0)
            .map(|sample| sample.pos)
    }

    fn placement_sample_at_coord(
        &self,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
        clearance: f32,
    ) -> Option<PlacementSample> {
        if server_ctx.editor_view_mode != EditorViewMode::D2
            && let Some(render_view) = ui.get_render_view("PolyView")
        {
            let dim = *render_view.dim();
            let screen_uv = [
                coord.x as f32 / dim.width as f32,
                coord.y as f32 / dim.height as f32,
            ];
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.scene_handler.vm.set_active_vm(0);
            if let Some((geo_id, hit, _)) = rusterix.scene_handler.vm.pick_geo_id_at_uv(
                dim.width as u32,
                dim.height as u32,
                screen_uv,
                false,
                false,
            ) {
                let rendered_object_id = match geo_id {
                    scenevm::GeoId::GeometryObject(object_id) => Some(object_id),
                    _ => None,
                };
                let paint_surface_id = rusterix
                    .scene_handler
                    .vm
                    .pick_paint_surface_at_uv(dim.width as u32, dim.height as u32, screen_uv)
                    .filter(|surface| surface.valid)
                    .map(|surface| surface.paint_geo);
                let surface_hit = rendered_object_id
                    .and_then(|object_id| {
                        paint_surface_id.and_then(|paint_surface_id| {
                            rusterix::resolve_block_prop_support_surface_hit(
                                &map.block_prop_instances,
                                &rusterix.assets.block_props,
                                object_id,
                                paint_surface_id,
                            )
                        })
                    })
                    .or_else(|| {
                        rendered_object_id.and_then(|object_id| {
                            rusterix::resolve_block_prop_support_surface_hit_at_point(
                                &map.block_prop_instances,
                                &rusterix.assets.block_props,
                                object_id,
                                hit,
                            )
                        })
                    })
                    .or_else(|| {
                        rusterix::resolve_block_prop_support_surface_hit_at_world_point(
                            &map.block_prop_instances,
                            &rusterix.assets.block_props,
                            hit,
                        )
                    });
                if let Some(surface_hit) = surface_hit
                    && let Some(instance) = map
                        .block_prop_instances
                        .iter()
                        .find(|instance| instance.id == surface_hit.instance_id)
                    && let Some(asset) = rusterix.assets.block_props.get(&surface_hit.asset_id)
                    && let Some(surface) = asset.find_support_surface(surface_hit.surface_id)
                    && let Some(mut local_position) =
                        rusterix::block_prop_support_surface_local_point(
                            asset, instance, surface.id, hit,
                        )
                {
                    local_position.y = 0.0;
                    if surface.snap_spacing > 0.0 {
                        local_position.x = (local_position.x / surface.snap_spacing).round()
                            * surface.snap_spacing;
                        local_position.z = (local_position.z / surface.snap_spacing).round()
                            * surface.snap_spacing;
                    }
                    if let Some(world_position) = rusterix::block_prop_support_surface_world_point(
                        asset,
                        instance,
                        surface.id,
                        [local_position.x, local_position.y, local_position.z],
                    ) {
                        return Some(PlacementSample {
                            pos: Vec2::new(world_position.x, world_position.z),
                            reference_y: Some(world_position.y),
                            floor_y: Some(world_position.y),
                            support_surface: Some(SupportSurfaceSample {
                                instance_id: instance.id,
                                surface_id: surface.id,
                                local_position,
                                world_position,
                            }),
                        });
                    }
                }
                if let Some((ray_origin, ray_dir)) = server_ctx
                    .hover_ray_origin_3d
                    .zip(server_ctx.hover_ray_dir_3d)
                    && let Some((floor_hit, reference_y)) = map
                        .geometry_floor_hit_from_ray_for_placement(
                            ray_origin, ray_dir, hit, clearance,
                        )
                {
                    return Some(PlacementSample {
                        pos: Vec2::new(floor_hit.x, floor_hit.z),
                        reference_y: Some(reference_y),
                        floor_y: Some(floor_hit.y),
                        support_surface: None,
                    });
                }
                return Some(PlacementSample {
                    pos: Vec2::new(hit.x, hit.z),
                    reference_y: Some(hit.y),
                    floor_y: Some(hit.y),
                    support_surface: None,
                });
            }
            if let Some(hit) = server_ctx.hover_cursor_3d {
                return Some(PlacementSample {
                    pos: Vec2::new(hit.x, hit.z),
                    reference_y: Some(hit.y),
                    floor_y: Some(hit.y),
                    support_surface: None,
                });
            }
        }

        ui.get_render_view("PolyView").map(|render_view| {
            let dim = *render_view.dim();
            let grid_space_pos = Vec2::new(coord.x as f32, coord.y as f32)
                - Vec2::new(dim.width as f32, dim.height as f32) / 2.0
                - Vec2::new(map.offset.x, -map.offset.y);

            PlacementSample {
                pos: grid_space_pos / map.grid_size,
                reference_y: None,
                floor_y: None,
                support_surface: None,
            }
        })
    }

    fn support_surface_sample_at_world_xz(
        map: &Map,
        sample: SupportSurfaceSample,
        world_xz: Vec2<f32>,
    ) -> Option<SupportSurfaceSample> {
        let rusterix = RUSTERIX.read().unwrap();
        let instance = map
            .block_prop_instances
            .iter()
            .find(|instance| instance.id == sample.instance_id)?;
        let asset = rusterix.assets.block_props.get(&instance.asset_id)?;
        let surface = asset.find_support_surface(sample.surface_id)?;
        let transform =
            rusterix::block_prop_support_surface_world_transform(asset, instance, surface.id)?;
        let origin = Vec3::new(transform[3][0], transform[3][1], transform[3][2]);
        let normal = Vec3::new(transform[1][0], transform[1][1], transform[1][2]);
        if normal.y.abs() <= 1e-5 {
            return None;
        }
        let world_point = Vec3::new(
            world_xz.x,
            origin.y
                - (normal.x * (world_xz.x - origin.x) + normal.z * (world_xz.y - origin.z))
                    / normal.y,
            world_xz.y,
        );
        let resolved = rusterix::resolve_block_prop_support_surface_hit_at_world_point(
            &map.block_prop_instances,
            &rusterix.assets.block_props,
            world_point,
        )?;
        if resolved.instance_id != sample.instance_id || resolved.surface_id != sample.surface_id {
            return None;
        }
        let mut local_position = rusterix::block_prop_support_surface_local_point(
            asset,
            instance,
            surface.id,
            world_point,
        )?;
        local_position.y = 0.0;
        if surface.snap_spacing > 0.0 {
            local_position.x =
                (local_position.x / surface.snap_spacing).round() * surface.snap_spacing;
            local_position.z =
                (local_position.z / surface.snap_spacing).round() * surface.snap_spacing;
        }
        let world_position = rusterix::block_prop_support_surface_world_point(
            asset,
            instance,
            surface.id,
            [local_position.x, local_position.y, local_position.z],
        )?;
        Some(SupportSurfaceSample {
            instance_id: sample.instance_id,
            surface_id: sample.surface_id,
            local_position,
            world_position,
        })
    }

    pub(crate) fn commit_item_surface_placement(
        map: &mut Map,
        item_creator_id: Uuid,
        sample: Option<SupportSurfaceSample>,
    ) -> Result<(), String> {
        let occupant = rusterix::BlockPropOccupant::ItemInstance(item_creator_id);
        let Some(sample) = sample else {
            map.block_prop_surface_placements
                .retain(|placement| placement.occupant != occupant);
            return Ok(());
        };

        let rusterix = RUSTERIX.read().unwrap();
        let instance = map
            .block_prop_instances
            .iter()
            .find(|instance| instance.id == sample.instance_id)
            .ok_or_else(|| fl!("status_prefab_surface_missing"))?;
        let asset = rusterix
            .assets
            .block_props
            .get(&instance.asset_id)
            .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?;
        let surface = asset
            .find_support_surface(sample.surface_id)
            .ok_or_else(|| fl!("status_prefab_surface_missing"))?;
        let item = map
            .items
            .iter()
            .find(|item| item.creator_id == item_creator_id)
            .ok_or_else(|| fl!("status_prefab_surface_item_missing"))?;

        let mut item_tags = vec!["placeable".to_string(), item.item_type.clone()];
        if let Some(tags) = item.attributes.get_str("tags") {
            item_tags.extend(
                tags.split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_string),
            );
        }
        if !surface.allowed_item_tags.is_empty()
            && !surface.allowed_item_tags.iter().any(|allowed| {
                item_tags
                    .iter()
                    .any(|tag| tag.eq_ignore_ascii_case(allowed))
            })
        {
            return Err(fl!("status_prefab_surface_item_not_allowed"));
        }

        let occupied = map
            .block_prop_surface_placements
            .iter()
            .filter(|placement| {
                placement.prop_instance_id == sample.instance_id
                    && placement.surface_id == sample.surface_id
                    && placement.occupant != occupant
            })
            .collect::<Vec<_>>();
        if surface
            .capacity
            .is_some_and(|capacity| occupied.len() >= capacity as usize)
            || matches!(
                &surface.occupancy_policy,
                rusterix::BlockPropOccupancyPolicy::SingleOccupant
            ) && !occupied.is_empty()
        {
            return Err(fl!("status_prefab_surface_capacity_reached"));
        }
        if matches!(
            &surface.occupancy_policy,
            rusterix::BlockPropOccupancyPolicy::RejectOverlap
        ) {
            let threshold = (surface.snap_spacing * 0.5).max(0.05);
            if occupied.iter().any(|placement| {
                (placement.local_transform[3][0] - sample.local_position.x).abs() < threshold
                    && (placement.local_transform[3][2] - sample.local_position.z).abs() < threshold
            }) {
                return Err(fl!("status_prefab_surface_position_occupied"));
            }
        }
        drop(rusterix);

        map.block_prop_surface_placements
            .retain(|placement| placement.occupant != occupant);
        let mut local_transform = rusterix::identity_block_prop_transform();
        local_transform[3][0] = sample.local_position.x;
        local_transform[3][1] = sample.local_position.y;
        local_transform[3][2] = sample.local_position.z;
        map.block_prop_surface_placements
            .push(rusterix::BlockPropSurfacePlacement {
                id: Uuid::new_v4(),
                prop_instance_id: sample.instance_id,
                surface_id: sample.surface_id,
                occupant,
                local_transform,
            });
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.scene_handler.mark_dynamics_dirty();
            rusterix.set_dirty();
        }
        Ok(())
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

    fn placement_floor_height(
        map: &Map,
        pos: Vec2<f32>,
        reference_y: Option<f32>,
        pointer_floor_y: Option<f32>,
        clearance: f32,
    ) -> Option<f32> {
        const FLOOR_EPS: f32 = 0.08;
        const CLEARANCE_EPS: f32 = 0.05;

        if let Some(reference_y) = reference_y {
            let nearest = map.geometry_floor_height_nearest(pos, reference_y);
            let floor_candidates = map.geometry_floor_candidates_at(pos);
            let raw_floor = floor_candidates
                .iter()
                .find(|floor| (floor.height - reference_y).abs() <= FLOOR_EPS)
                .map(|floor| floor.height)
                .or_else(|| {
                    floor_candidates
                        .iter()
                        .filter(|floor| floor.height <= reference_y + FLOOR_EPS)
                        .min_by(|a, b| {
                            (reference_y - a.height)
                                .abs()
                                .partial_cmp(&(reference_y - b.height).abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|floor| floor.height)
                });

            let mut resolved = if let (Some(nearest), Some(raw_floor)) = (nearest, raw_floor) {
                if raw_floor - nearest < clearance.max(0.0) + CLEARANCE_EPS {
                    Some(raw_floor)
                } else {
                    Some(nearest)
                }
            } else {
                nearest.or(raw_floor)
            };
            if let (Some(pointer_floor_y), Some(current)) = (pointer_floor_y, resolved)
                && pointer_floor_y > current
                && pointer_floor_y - current < clearance.max(0.0) + CLEARANCE_EPS
            {
                resolved = Some(pointer_floor_y);
            }

            resolved
        } else {
            map.geometry_floor_height_at(pos)
        }
    }

    /// Snap a direction to the nearest cardinal axis.
    fn snap_cardinal(dir: Vec2<f32>) -> Vec2<f32> {
        if dir.x.abs() >= dir.y.abs() {
            if dir.x >= 0.0 {
                Vec2::new(1.0, 0.0)
            } else {
                Vec2::new(-1.0, 0.0)
            }
        } else if dir.y >= 0.0 {
            Vec2::new(0.0, 1.0)
        } else {
            Vec2::new(0.0, -1.0)
        }
    }

    fn pick_hit(&self, map: &Map, pos: Vec2<f32>, radius2: f32) -> Option<Hit> {
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

    fn pick_rendered_hit_for_coord(
        &self,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
    ) -> Option<Hit> {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return None;
        }

        let render_view = ui.get_render_view("PolyView")?;
        let dim = *render_view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return None;
        }
        let screen_uv = [
            coord.x as f32 / dim.width as f32,
            coord.y as f32 / dim.height as f32,
        ];
        let geo_id = {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.scene_handler.vm.set_active_vm(0);
            rusterix
                .scene_handler
                .vm
                .pick_geo_id_at_uv(dim.width as u32, dim.height as u32, screen_uv, false, true)
                .map(|(geo_id, _, _)| geo_id)?
        };

        match geo_id {
            scenevm::GeoId::Character(_) => map
                .entities
                .iter()
                .enumerate()
                .find(|(index, entity)| {
                    rusterix::SceneHandler::entity_render_geo_id(entity, *index) == geo_id
                })
                .map(|(_, entity)| Hit {
                    target: DragTarget::Entity(entity.creator_id),
                    name: entity
                        .attributes
                        .get_str("name")
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| "Entity".into()),
                    pos: entity.get_pos_xz(),
                }),
            scenevm::GeoId::Item(_) => map
                .items
                .iter()
                .enumerate()
                .find(|(index, item)| {
                    rusterix::SceneHandler::item_render_geo_id(item, *index) == geo_id
                })
                .map(|(_, item)| Hit {
                    target: DragTarget::Item(item.creator_id),
                    name: item
                        .attributes
                        .get_str("name")
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| "Item".into()),
                    pos: item.get_pos_xz(),
                }),
            _ => None,
        }
    }

    fn pick_hit_for_coord(
        &self,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        map: &Map,
        coord: Vec2<i32>,
    ) -> Option<Hit> {
        if let Some(hit) = self.pick_rendered_hit_for_coord(ui, server_ctx, map, coord) {
            return Some(hit);
        }
        let pos = self.map_pos_unsnapped(ui, server_ctx, map, coord)?;
        let radius2 = if server_ctx.editor_view_mode == EditorViewMode::D2 {
            0.16
        } else {
            1.44
        };
        self.pick_hit(map, pos, radius2)
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

        let mut character_to_remove: Option<(usize, Character)> = None;
        let mut item_to_remove: Option<(usize, shared::prelude::Item)> = None;

        if let Some(region) = project.get_region_ctx(server_ctx) {
            if let Some(index) = region.characters.get_index_of(&selected)
                && let Some(character) = region.characters.get(&selected).cloned()
            {
                character_to_remove = Some((index, character));
            }

            if let Some(index) = region.items.get_index_of(&selected)
                && let Some(item) = region.items.get(&selected).cloned()
            {
                item_to_remove = Some((index, item));
            }
        }

        if let Some((index, character)) = character_to_remove {
            let atom = ProjectUndoAtom::RemoveRegionCharacterInstance(
                index,
                server_ctx.curr_region,
                character.clone(),
            );
            Self::remove_selected_character_instance(project, ui, ctx, server_ctx, character.id);
            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
            return true;
        }

        if let Some((index, item)) = item_to_remove {
            let atom = ProjectUndoAtom::RemoveRegionItemInstance(
                index,
                server_ctx.curr_region,
                item.clone(),
            );
            Self::remove_selected_item_instance(project, ui, ctx, server_ctx, item.id);
            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
            return true;
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
            if let Some(action) = rusterix.client_touch_down(coord, map) {
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
            if let Some(action) = rusterix.client_touch_up(coord, map) {
                rusterix.server.local_player_action(action);
            }
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
