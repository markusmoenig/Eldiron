use crate::blocks::{
    BLOCK_COLUMN_SEGMENTS, BLOCK_OPERATION_ERASE, BLOCK_OPERATION_PLACE, BLOCK_OPERATION_REPLACE,
    adjusted_rotated_bounds, block_asset, block_component_kind, block_sizing_from_context,
    block_stroke_cells, block_surface_base_y, component_uses_cylinder, cylinder_vertices_and_faces,
    default_block_asset_id, localized_block_asset_name,
};
use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;

pub struct BlockTool {
    id: TheId,
    previous_dock: Option<String>,
    drag_start_cell: Option<Vec3<i32>>,
    drag_end_cell: Option<Vec3<i32>>,
    drag_base_y: Option<f32>,
    drag_changed: bool,
}

#[derive(Clone, Copy)]
struct BlockPlacement {
    cell: Vec3<i32>,
    base_y: f32,
}

impl BlockTool {
    fn selected_asset(
        server_ctx: &mut ServerContext,
    ) -> Option<&'static crate::blocks::BlockAsset> {
        let id = server_ctx
            .curr_block_asset_id
            .unwrap_or_else(default_block_asset_id);
        let asset = block_asset(id).or_else(|| block_asset(default_block_asset_id()))?;
        server_ctx.curr_block_asset_id = Some(asset.id);
        server_ctx.curr_block_asset_name = Some(asset.name.to_string());
        Some(asset)
    }

    fn selected_asset_id(server_ctx: &mut ServerContext) -> Uuid {
        *server_ctx
            .curr_block_asset_id
            .get_or_insert_with(default_block_asset_id)
    }

    fn placement_hit(server_ctx: &ServerContext) -> Option<Vec3<f32>> {
        crate::blocks::block_grid_plane_hit(server_ctx).or_else(|| {
            server_ctx
                .hover_cursor_3d
                .or(server_ctx.hover_surface_hit_pos)
                .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))
        })
    }

    fn snapped_grid_cell(point: Vec3<f32>, cell_size: f32, level: i32) -> Vec3<i32> {
        Vec3::new(
            (point.x / cell_size).floor() as i32,
            level,
            (point.z / cell_size).floor() as i32,
        )
    }

    fn current_placement(server_ctx: &ServerContext) -> Option<BlockPlacement> {
        let cell_size = server_ctx.block_grid_cell_size.max(0.05);
        let grid_y = server_ctx.block_grid_level as f32 * cell_size;
        let point = Self::placement_hit(server_ctx)?;
        Some(BlockPlacement {
            cell: Self::snapped_grid_cell(point, cell_size, server_ctx.block_grid_level),
            base_y: block_surface_base_y(server_ctx, grid_y).unwrap_or(grid_y),
        })
    }

    fn make_objects(
        asset: &crate::blocks::BlockAsset,
        grid_cell: Vec3<i32>,
        base_y: f32,
        cell_size: f32,
        quarter_turns: i32,
        sizing: crate::blocks::BlockSizing,
        damaged: bool,
    ) -> Vec<rusterix::GeometryObject> {
        let base = Vec3::new(
            grid_cell.x as f32 * cell_size,
            base_y,
            grid_cell.z as f32 * cell_size,
        );
        let instance_id = Uuid::new_v4();
        let group = format!("block:{instance_id}");
        let rotation = quarter_turns.rem_euclid(4);

        asset
            .boxes
            .iter()
            .enumerate()
            .filter_map(|(index, _block_box)| {
                let (min, max) = adjusted_rotated_bounds(asset, index, sizing, rotation)?;
                let name = if asset.boxes.len() == 1 {
                    asset.name.to_string()
                } else {
                    format!("{} {}", asset.name, index + 1)
                };
                let object_min = base + min * cell_size;
                let object_max = base + max * cell_size;
                let mut object = if component_uses_cylinder(block_component_kind(asset, index)) {
                    Self::cylinder_object(name, object_min, object_max)
                } else {
                    rusterix::GeometryObject::box_from_bounds(name, object_min, object_max)
                };
                if damaged {
                    let seed = Self::damage_seed(asset.id, grid_cell, index, rotation);
                    Self::apply_damage(&mut object, seed);
                    object.properties.set("block_damaged", Value::Bool(true));
                    object
                        .properties
                        .set("block_damage_seed", Value::UInt(seed));
                }
                object.kind = rusterix::GeometryObjectKind::Brush;
                object.group = group.clone();
                object.tags.push("block".to_string());
                object.properties.set("block_asset_id", Value::Id(asset.id));
                object
                    .properties
                    .set("block_asset_name", Value::Str(asset.name.to_string()));
                object
                    .properties
                    .set("block_instance_id", Value::Id(instance_id));
                object
                    .properties
                    .set("block_grid_x", Value::Int(grid_cell.x));
                object
                    .properties
                    .set("block_grid_y", Value::Int(grid_cell.y));
                object
                    .properties
                    .set("block_grid_z", Value::Int(grid_cell.z));
                object.properties.set("block_base_y", Value::Float(base_y));
                object
                    .properties
                    .set("block_rotation", Value::Int(rotation));
                object
                    .properties
                    .set("block_cell_size", Value::Float(cell_size));
                object
                    .properties
                    .set("block_height_cells", Value::Int(sizing.height_cells));
                object.properties.set(
                    "block_span_extra_cells",
                    Value::Int(sizing.span_extra_cells),
                );
                Some(object)
            })
            .collect()
    }

    fn geometry_face(indices: Vec<usize>) -> rusterix::GeometryFace {
        rusterix::GeometryFace {
            id: Uuid::new_v4(),
            paint_surface_id: None,
            uvs: indices
                .iter()
                .map(|_| Vec2::new(0.0, 0.0))
                .collect::<Vec<_>>(),
            indices,
            paint_uvs: Vec::new(),
            auto_uv: true,
            texture_offset: Vec2::zero(),
            texture_scale: Vec2::broadcast(1.0),
            texture_rotation: 0.0,
            tile: None,
            tiles: FxHashMap::default(),
            surface_points: Vec::new(),
            surface_segments: Vec::new(),
        }
    }

    fn cylinder_object(name: String, min: Vec3<f32>, max: Vec3<f32>) -> rusterix::GeometryObject {
        let (vertices, faces) = cylinder_vertices_and_faces(min, max, BLOCK_COLUMN_SEGMENTS);
        let mut object = rusterix::GeometryObject::new(name);
        object.vertices = vertices;
        object.faces = faces.into_iter().map(Self::geometry_face).collect();
        object
    }

    fn damage_seed(
        asset_id: Uuid,
        grid_cell: Vec3<i32>,
        component_index: usize,
        rotation: i32,
    ) -> u32 {
        let mut hash = 0x811c_9dc5_u32;
        for byte in asset_id.as_bytes() {
            hash ^= *byte as u32;
            hash = hash.wrapping_mul(0x0100_0193);
        }
        for value in [
            grid_cell.x,
            grid_cell.y,
            grid_cell.z,
            component_index as i32,
            rotation,
        ] {
            hash ^= value as u32;
            hash = hash.wrapping_mul(0x0100_0193);
        }
        hash
    }

    fn damage_rand(seed: &mut u32) -> f32 {
        *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((*seed >> 8) as f32) / ((u32::MAX >> 8) as f32)
    }

    fn apply_damage(object: &mut rusterix::GeometryObject, seed: u32) {
        if object.vertices.is_empty() {
            return;
        }

        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);
        for vertex in &object.vertices {
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);
            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }

        let size = max - min;
        if size.x <= 0.01 || size.y <= 0.01 || size.z <= 0.01 {
            return;
        }

        let center = (min + max) * 0.5;
        let mut rng = seed;
        let chip_x = if Self::damage_rand(&mut rng) < 0.5 {
            -1.0
        } else {
            1.0
        };
        let chip_z = if Self::damage_rand(&mut rng) < 0.5 {
            -1.0
        } else {
            1.0
        };
        let corner_cut = 0.18 + Self::damage_rand(&mut rng) * 0.14;
        let top_drop = size.y * (0.08 + Self::damage_rand(&mut rng) * 0.08);
        let jitter = size.x.min(size.z) * 0.035;

        for vertex in &mut object.vertices {
            let side_x = if vertex.x < center.x { -1.0 } else { 1.0 };
            let side_z = if vertex.z < center.z { -1.0 } else { 1.0 };
            let near_chip_corner = side_x == chip_x && side_z == chip_z;
            let high = vertex.y > min.y + size.y * 0.55;

            if near_chip_corner {
                vertex.x += (center.x - vertex.x) * corner_cut;
                vertex.z += (center.z - vertex.z) * corner_cut;
                if high {
                    vertex.y = (vertex.y - top_drop).max(min.y);
                }
            }

            if high {
                let noise = Self::damage_rand(&mut rng);
                if noise < 0.45 {
                    vertex.y = (vertex.y - top_drop * noise).max(min.y);
                }
            }

            let jx = (Self::damage_rand(&mut rng) - 0.5) * jitter;
            let jz = (Self::damage_rand(&mut rng) - 0.5) * jitter;
            vertex.x += jx;
            vertex.z += jz;
        }
    }

    fn block_object_grid_cell(object: &rusterix::GeometryObject) -> Option<Vec3<i32>> {
        Some(Vec3::new(
            object.properties.get_int("block_grid_x")?,
            object.properties.get_int("block_grid_y")?,
            object.properties.get_int("block_grid_z")?,
        ))
    }

    fn block_object_instance_id(object: &rusterix::GeometryObject) -> Option<Uuid> {
        object.properties.get_id("block_instance_id")
    }

    fn remove_block_instances_at_cells(map: &mut Map, cells: &[Vec3<i32>]) -> FxHashSet<Uuid> {
        let mut instances = FxHashSet::default();
        for object in &map.geometry_objects {
            let Some(cell) = Self::block_object_grid_cell(object) else {
                continue;
            };
            if !cells.contains(&cell) {
                continue;
            }
            if let Some(instance_id) = Self::block_object_instance_id(object) {
                instances.insert(instance_id);
            }
        }

        if instances.is_empty() {
            return instances;
        }

        map.geometry_objects.retain(|object| {
            Self::block_object_instance_id(object)
                .map(|instance_id| !instances.contains(&instance_id))
                .unwrap_or(true)
        });
        map.selected_geometry_objects
            .retain(|id| map.geometry_objects.iter().any(|object| object.id == *id));
        instances
    }

    fn prop_instance_grid_cell(instance: &rusterix::BlockPropInstance) -> Option<Vec3<i32>> {
        Some(Vec3::new(
            instance.overrides.get_int("block_grid_x")?,
            instance.overrides.get_int("block_grid_y")?,
            instance.overrides.get_int("block_grid_z")?,
        ))
    }

    fn remove_prop_instances_at_cells(map: &mut Map, cells: &[Vec3<i32>]) -> FxHashSet<Uuid> {
        let removed = map
            .block_prop_instances
            .iter()
            .filter_map(|instance| {
                Self::prop_instance_grid_cell(instance)
                    .filter(|cell| cells.contains(cell))
                    .map(|_| instance.id)
            })
            .collect::<FxHashSet<_>>();
        if removed.is_empty() {
            return removed;
        }
        map.block_prop_instances
            .retain(|instance| !removed.contains(&instance.id));
        map.block_prop_surface_placements
            .retain(|placement| !removed.contains(&placement.prop_instance_id));
        removed
    }

    fn make_prop_instance(
        asset_id: Uuid,
        grid_cell: Vec3<i32>,
        base_y: f32,
        cell_size: f32,
        quarter_turns: i32,
    ) -> rusterix::BlockPropInstance {
        let mut instance = rusterix::BlockPropInstance::new(asset_id);
        let angle = quarter_turns.rem_euclid(4) as f32 * std::f32::consts::FRAC_PI_2;
        let (sin, cos) = angle.sin_cos();
        instance.world_transform[0][0] = cos;
        instance.world_transform[0][2] = -sin;
        instance.world_transform[2][0] = sin;
        instance.world_transform[2][2] = cos;
        instance.world_transform[3][0] = grid_cell.x as f32 * cell_size;
        instance.world_transform[3][1] = base_y;
        instance.world_transform[3][2] = grid_cell.z as f32 * cell_size;
        instance
            .overrides
            .set("block_grid_x", Value::Int(grid_cell.x));
        instance
            .overrides
            .set("block_grid_y", Value::Int(grid_cell.y));
        instance
            .overrides
            .set("block_grid_z", Value::Int(grid_cell.z));
        instance.overrides.set("block_base_y", Value::Float(base_y));
        instance
            .overrides
            .set("block_rotation", Value::Int(quarter_turns.rem_euclid(4)));
        instance
    }

    fn clear_drag(&mut self, server_ctx: &mut ServerContext) {
        self.drag_start_cell = None;
        self.drag_end_cell = None;
        self.drag_base_y = None;
        self.drag_changed = false;
        server_ctx.block_drag_base_y = None;
        server_ctx.block_drag_start_cell = None;
        server_ctx.block_drag_end_cell = None;
    }

    fn begin_drag(&mut self, server_ctx: &mut ServerContext) -> bool {
        let Some(placement) = Self::current_placement(server_ctx) else {
            self.clear_drag(server_ctx);
            return false;
        };
        let cell = placement.cell;
        self.drag_start_cell = Some(cell);
        self.drag_end_cell = Some(cell);
        self.drag_base_y = Some(placement.base_y);
        self.drag_changed = false;
        server_ctx.block_drag_base_y = Some(placement.base_y);
        server_ctx.block_drag_start_cell = Some(cell);
        server_ctx.block_drag_end_cell = Some(cell);
        true
    }

    fn update_drag(&mut self, server_ctx: &mut ServerContext) -> bool {
        let Some(placement) = Self::current_placement(server_ctx) else {
            return false;
        };
        let cell = placement.cell;
        if self.drag_start_cell.is_none() {
            self.drag_start_cell = Some(cell);
            self.drag_base_y = Some(placement.base_y);
            server_ctx.block_drag_base_y = Some(placement.base_y);
            server_ctx.block_drag_start_cell = Some(cell);
        }
        if self.drag_end_cell != Some(cell) {
            self.drag_end_cell = Some(cell);
            self.drag_changed = true;
            server_ctx.block_drag_end_cell = Some(cell);
            return true;
        }
        false
    }

    fn operation_name(operation: i32) -> String {
        match operation {
            BLOCK_OPERATION_REPLACE => fl!("block_commit_replaced"),
            BLOCK_OPERATION_ERASE => fl!("block_commit_erased"),
            _ => fl!("block_commit_placed"),
        }
    }

    fn commit_stroke(
        &mut self,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let start = self.drag_start_cell?;
        let end = self.drag_end_cell.unwrap_or(start);
        let prev = map.clone();
        let cell_size = server_ctx.block_grid_cell_size.max(0.05);
        let base_y = self
            .drag_base_y
            .or(server_ctx.block_drag_base_y)
            .unwrap_or(start.y as f32 * cell_size);
        let operation = server_ctx
            .block_operation
            .clamp(BLOCK_OPERATION_PLACE, BLOCK_OPERATION_ERASE);
        let cells = block_stroke_cells(start, end, server_ctx.block_stroke_mode);
        if cells.is_empty() {
            return None;
        }

        let (removed, removed_props) =
            if operation == BLOCK_OPERATION_REPLACE || operation == BLOCK_OPERATION_ERASE {
                (
                    Self::remove_block_instances_at_cells(map, &cells),
                    Self::remove_prop_instances_at_cells(map, &cells),
                )
            } else {
                (FxHashSet::default(), FxHashSet::default())
            };

        let mut created = Vec::new();
        let mut created_props = Vec::new();
        let mut asset_name = fl!("block_asset_instances");
        if operation != BLOCK_OPERATION_ERASE {
            let asset_id = Self::selected_asset_id(server_ctx);
            if let Some(asset) = block_asset(asset_id) {
                asset_name = localized_block_asset_name(asset);
                for cell in &cells {
                    created.extend(Self::make_objects(
                        asset,
                        *cell,
                        base_y,
                        cell_size,
                        server_ctx.block_rotation_quarters,
                        block_sizing_from_context(server_ctx),
                        server_ctx.block_damage_enabled,
                    ));
                }
            } else {
                asset_name = server_ctx
                    .curr_block_asset_name
                    .clone()
                    .unwrap_or_else(|| "Project Prop".to_string());
                for cell in &cells {
                    created_props.push(Self::make_prop_instance(
                        asset_id,
                        *cell,
                        base_y,
                        cell_size,
                        server_ctx.block_rotation_quarters,
                    ));
                }
            }
        }

        if created.is_empty()
            && created_props.is_empty()
            && removed.is_empty()
            && removed_props.is_empty()
        {
            return None;
        }

        if !created.is_empty() {
            map.selected_vertices.clear();
            map.selected_linedefs.clear();
            map.selected_sectors.clear();
            map.selected_geometry_vertices.clear();
            map.selected_geometry_faces.clear();
            map.selected_geometry_objects = created.iter().map(|object| object.id).collect();
            map.geometry_selection_mode = 0;
            map.geometry_objects.extend(created);
        } else if !created_props.is_empty() {
            map.clear_selection();
            map.block_prop_instances.extend(created_props);
        } else {
            map.selected_geometry_objects.clear();
        }

        let cell_count = cells.len() as i64;
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            format!(
                "{}",
                fl!(
                    "status_block_commit",
                    operation = Self::operation_name(operation),
                    asset_name = asset_name,
                    cell_count = cell_count
                )
            ),
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Geometry Overlay 3D"),
            TheValue::Empty,
        ));
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
    }
}

impl Tool for BlockTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Prefab Tool"),
            previous_dock: None,
            drag_start_cell: None,
            drag_end_cell: None,
            drag_base_y: None,
            drag_changed: false,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_blocks")
    }

    fn icon_name(&self) -> String {
        "lego".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('B')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                server_ctx.block_tool_active = true;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx
                    .curr_block_asset_id
                    .get_or_insert_with(default_block_asset_id);
                if server_ctx.curr_block_asset_name.is_none() {
                    server_ctx.curr_block_asset_name =
                        Self::selected_asset(server_ctx).map(|asset| asset.name.to_string());
                }
                self.clear_drag(server_ctx);

                let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Prefabs" {
                    self.previous_dock = if current_dock.is_empty() {
                        None
                    } else {
                        Some(current_dock)
                    };
                }
                DOCKMANAGER.write().unwrap().set_dock(
                    "Prefabs".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_block_tool_active"),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            DeActivate => {
                server_ctx.block_tool_active = false;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                self.clear_drag(server_ctx);
                let prefab_editor_open = {
                    let manager = DOCKMANAGER.read().unwrap();
                    manager.dock == "Prefabs" && manager.state == DockManagerState::Editor
                };
                if DOCKMANAGER.read().unwrap().dock == "Prefabs" && !prefab_editor_open {
                    let mut dockmanager = DOCKMANAGER.write().unwrap();
                    dockmanager.minimize_for_tool_switch(ui, ctx, project, server_ctx);
                    if let Some(prev) = self.previous_dock.take() {
                        dockmanager.set_dock(prev, ui, ctx, project, server_ctx);
                    }
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return None;
        }

        match map_event {
            MapClicked(_) => {
                self.begin_drag(server_ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            MapDragged(_) => {
                if self.update_drag(server_ctx) {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Geometry Overlay 3D"),
                        TheValue::Empty,
                    ));
                }
                None
            }
            MapUp(_) => {
                if self.drag_start_cell.is_none() {
                    self.begin_drag(server_ctx);
                }
                let undo = self.commit_stroke(ctx, map, server_ctx);
                self.clear_drag(server_ctx);
                undo
            }
            MapKey('r') | MapKey('R') => {
                server_ctx.block_rotation_quarters =
                    (server_ctx.block_rotation_quarters + 1).rem_euclid(4);
                let degrees = (server_ctx.block_rotation_quarters.rem_euclid(4) * 90) as i64;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("{}", fl!("status_block_rotation", degrees = degrees)),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                None
            }
            MapKey(']') | MapKey('}') => {
                server_ctx.block_grid_level += 1;
                let level = server_ctx.block_grid_level as i64;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("{}", fl!("status_block_grid_level", level = level)),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                None
            }
            MapKey('[') | MapKey('{') => {
                server_ctx.block_grid_level -= 1;
                let level = server_ctx.block_grid_level as i64;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("{}", fl!("status_block_grid_level", level = level)),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                None
            }
            MapKey('h') => {
                server_ctx.block_height_cells = (server_ctx.block_height_cells + 1).clamp(1, 16);
                let height = server_ctx.block_height_cells as i64;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("{}", fl!("status_block_height", height = height)),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            MapKey('H') => {
                server_ctx.block_height_cells = (server_ctx.block_height_cells - 1).clamp(1, 16);
                let height = server_ctx.block_height_cells as i64;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("{}", fl!("status_block_height", height = height)),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            MapKey('w') => {
                server_ctx.block_span_extra_cells =
                    (server_ctx.block_span_extra_cells + 1).clamp(0, 16);
                let width = server_ctx.block_span_extra_cells as i64;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("{}", fl!("status_block_width_extra", width = width)),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            MapKey('W') => {
                server_ctx.block_span_extra_cells =
                    (server_ctx.block_span_extra_cells - 1).clamp(0, 16);
                let width = server_ctx.block_span_extra_cells as i64;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("{}", fl!("status_block_width_extra", width = width)),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            MapKey('e') | MapKey('E') => {
                server_ctx.block_operation = if server_ctx.block_operation == BLOCK_OPERATION_ERASE
                {
                    BLOCK_OPERATION_PLACE
                } else {
                    BLOCK_OPERATION_ERASE
                };
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    if server_ctx.block_operation == BLOCK_OPERATION_ERASE {
                        fl!("status_block_mode_erase")
                    } else {
                        fl!("status_block_mode_place")
                    },
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            MapKey('d') | MapKey('D') => {
                server_ctx.block_damage_enabled = !server_ctx.block_damage_enabled;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    if server_ctx.block_damage_enabled {
                        fl!("status_block_damage_on")
                    } else {
                        fl!("status_block_damage_off")
                    },
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            MapEscape => {
                self.clear_drag(server_ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_prop_instance_uses_grid_transform_and_metadata() {
        let asset_id = Uuid::new_v4();
        let instance = BlockTool::make_prop_instance(asset_id, Vec3::new(3, 2, -4), 2.5, 0.5, 1);
        assert_eq!(instance.asset_id, asset_id);
        assert!((instance.world_transform[0][0]).abs() < 1e-5);
        assert_eq!(instance.world_transform[0][2], -1.0);
        assert_eq!(instance.world_transform[2][0], 1.0);
        assert!((instance.world_transform[2][2]).abs() < 1e-5);
        assert_eq!(instance.world_transform[3][0], 1.5);
        assert_eq!(instance.world_transform[3][1], 2.5);
        assert_eq!(instance.world_transform[3][2], -2.0);
        assert_eq!(
            BlockTool::prop_instance_grid_cell(&instance),
            Some(Vec3::new(3, 2, -4))
        );
    }

    #[test]
    fn erasing_project_prop_also_removes_surface_occupants() {
        let mut map = Map::default();
        let instance =
            BlockTool::make_prop_instance(Uuid::new_v4(), Vec3::new(1, 0, 2), 0.0, 1.0, 0);
        let instance_id = instance.id;
        map.block_prop_instances.push(instance);
        map.block_prop_surface_placements
            .push(rusterix::BlockPropSurfacePlacement {
                id: Uuid::new_v4(),
                prop_instance_id: instance_id,
                surface_id: Uuid::new_v4(),
                occupant: rusterix::BlockPropOccupant::PropInstance(Uuid::new_v4()),
                local_transform: rusterix::identity_block_prop_transform(),
            });

        let removed = BlockTool::remove_prop_instances_at_cells(&mut map, &[Vec3::new(1, 0, 2)]);
        assert_eq!(removed, FxHashSet::from_iter([instance_id]));
        assert!(map.block_prop_instances.is_empty());
        assert!(map.block_prop_surface_placements.is_empty());
    }
}
