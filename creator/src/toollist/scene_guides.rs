//! Shared authoring guide producers. Every GPU mutation targets the scene
//! supplied by the caller; these helpers never acquire the global renderer.
use super::*;

impl ToolList {
    fn add_construction_grid_3d(
        &self,
        assets: &Assets,
        scene: &mut rusterix::SceneHandler,
        map: &Map,
        server_ctx: &ServerContext,
    ) {
        let block_grid_active = server_ctx.block_tool_active;
        let base_step = if block_grid_active {
            crate::blocks::block_tool_horizontal_grid_step(
                server_ctx.curr_block_asset_id,
                &assets.block_props,
                map,
                server_ctx,
            )
        } else {
            ServerContext::edit_grid_step(map.subdivisions).max(0.01)
        };
        let grid_bbox = map.bbox().expanded(Vec2::new(16.0, 16.0));
        let span = (grid_bbox.max.x - grid_bbox.min.x)
            .max(grid_bbox.max.y - grid_bbox.min.y)
            .max(16.0);
        let mut display_step = base_step;
        while span / display_step > 256.0 {
            display_step *= 2.0;
        }

        let min_x_step = (grid_bbox.min.x.min(-8.0) / display_step).floor() as i32;
        let max_x_step = (grid_bbox.max.x.max(8.0) / display_step).ceil() as i32;
        let min_z_step = (grid_bbox.min.y.min(-8.0) / display_step).floor() as i32;
        let max_z_step = (grid_bbox.max.y.max(8.0) / display_step).ceil() as i32;
        let min_x = min_x_step as f32 * display_step;
        let max_x = max_x_step as f32 * display_step;
        let min_z = min_z_step as f32 * display_step;
        let max_z = max_z_step as f32 * display_step;
        let grid_base_y = if block_grid_active {
            server_ctx.block_grid_level as f32 * server_ctx.block_grid_cell_size.max(0.05)
        } else {
            0.0
        };
        let grid_y = grid_base_y + 0.012;
        let minor = Self::overlay_color(self.prefab_grid_rgb, 0.48);
        let whole = Self::overlay_color(self.prefab_grid_rgb, 0.66);
        let major = Self::overlay_color(self.prefab_preview_wire_rgb, 0.78);
        let x_axis = [0.88, 0.28, 0.24, 0.96];
        let z_axis = [0.25, 0.52, 0.94, 0.96];
        let mut grid_index = 0u32;

        for x_step in min_x_step..=max_x_step {
            let x = x_step as f32 * display_step;
            if x.abs() <= 0.0001 {
                continue;
            }
            let base_units = x / base_step;
            let is_whole = (base_units - base_units.round()).abs() <= 0.0001;
            let is_major = is_whole && (base_units.round() as i32).rem_euclid(4) == 0;
            scene.overlay_3d.add_hardware_line_3d(
                GeoId::Unknown(0xE300_0000u32.wrapping_add(grid_index)),
                Vec3::new(x, grid_y, min_z),
                Vec3::new(x, grid_y, max_z),
                if is_major {
                    major
                } else if is_whole {
                    whole
                } else {
                    minor
                },
                8,
            );
            grid_index = grid_index.wrapping_add(1);
        }

        for z_step in min_z_step..=max_z_step {
            let z = z_step as f32 * display_step;
            if z.abs() <= 0.0001 {
                continue;
            }
            let base_units = z / base_step;
            let is_whole = (base_units - base_units.round()).abs() <= 0.0001;
            let is_major = is_whole && (base_units.round() as i32).rem_euclid(4) == 0;
            scene.overlay_3d.add_hardware_line_3d(
                GeoId::Unknown(0xE301_0000u32.wrapping_add(grid_index)),
                Vec3::new(min_x, grid_y, z),
                Vec3::new(max_x, grid_y, z),
                if is_major {
                    major
                } else if is_whole {
                    whole
                } else {
                    minor
                },
                8,
            );
            grid_index = grid_index.wrapping_add(1);
        }

        scene.overlay_3d.add_hardware_line_3d(
            GeoId::Unknown(0xE302_0000),
            Vec3::new(min_x, grid_y + 0.004, 0.0),
            Vec3::new(max_x, grid_y + 0.004, 0.0),
            x_axis,
            9,
        );
        scene.overlay_3d.add_hardware_line_3d(
            GeoId::Unknown(0xE302_0001),
            Vec3::new(0.0, grid_y + 0.004, min_z),
            Vec3::new(0.0, grid_y + 0.004, max_z),
            z_axis,
            9,
        );
    }

    pub(crate) fn prepare_geometry_overlay_3d(
        &mut self,
        project: &Project,
        server_ctx: &ServerContext,
        assets: &Assets,
        camera: &dyn D3Camera,
        scene: &mut rusterix::SceneHandler,
    ) {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return;
        }

        scene.clear_overlay();
        if server_ctx.show_editor_grid
            && let Some(map) = project.get_map(server_ctx)
        {
            self.add_construction_grid_3d(assets, scene, map, server_ctx);
        }
        if server_ctx.curr_map_tool_type == MapToolType::Terrain {
            if let Some(map) = project.get_map(server_ctx)
                && let Some(center) = server_ctx.hover_cursor_3d
            {
                let points = TerrainTool::outline(
                    map,
                    Vec2::new(center.x, center.z),
                    server_ctx.terrain_brush_radius,
                );
                let tile = scene.selected;
                let normal = camera.basis_vectors().0;
                for (i, pair) in points.windows(2).enumerate() {
                    scene.overlay_3d.add_line_3d(
                        GeoId::Unknown(0x7E00_0000 + i as u32),
                        tile,
                        pair[0] + Vec3::new(0., 0.02, 0.),
                        pair[1] + Vec3::new(0., 0.02, 0.),
                        0.06,
                        normal,
                        100,
                    );
                }
            }
            scene.set_overlay();
            return;
        }
        // scene.vm.set_layer_activity_logging(true);

        // basis_vectors returns (forward, right, up)
        let (cam_forward, cam_right, cam_up) = camera.basis_vectors();
        let view_right = cam_right;
        let view_up = cam_up;
        let view_nudge = cam_forward * -0.002; // small toward-camera nudge to avoid z-fighting
        let camera_position = camera.position();
        let camera_scale = camera.scale();
        let camera_fov = camera.fov().to_radians();
        let overlay_world_size = |point: Vec3<f32>| -> f32 {
            if server_ctx.editor_view_mode == EditorViewMode::Iso {
                (camera_scale * 0.035).clamp(0.055, 0.28)
            } else {
                let distance = (point - camera_position).magnitude().max(0.1);
                (distance * (camera_fov * 0.5).tan() * 0.045).clamp(0.055, 0.28)
            }
        };
        let thickness = 0.15;

        if let Some(source_map) = project.get_map(server_ctx) {
            let filtered;
            let map = if source_map.sectors.iter().any(|s| s.layer.is_some()) {
                let mut map = source_map.clone();
                crate::utils::filter_iso_placements_for_3d(&mut map, &project.tiles);
                filtered = map;
                &filtered
            } else {
                source_map
            };
            let mut visible_sector_ids: FxHashSet<u32> = FxHashSet::default();
            let mut visible_linedef_ids: FxHashSet<u32> = FxHashSet::default();
            let mut visible_vertex_ids: FxHashSet<u32> = FxHashSet::default();

            for sector in &map.sectors {
                visible_sector_ids.insert(sector.id);
                for linedef_id in &sector.linedefs {
                    visible_linedef_ids.insert(*linedef_id);
                    if let Some(linedef) = map.find_linedef(*linedef_id) {
                        visible_vertex_ids.insert(linedef.start_vertex);
                        visible_vertex_ids.insert(linedef.end_vertex);
                    }
                }
            }

            // Helper to draw a single world-space line into the overlay
            let push_line = |id: GeoId,
                             scene: &mut rusterix::SceneHandler,
                             mut a: Vec3<f32>,
                             mut b: Vec3<f32>,
                             normal: Vec3<f32>,
                             selected: bool,
                             hovered: bool| {
                // Z-fight mitigation: nudge along CAMERA FORWARD, not the line normal
                if selected {
                    let extra_nudge = cam_forward * -0.004; // toward camera
                    a += extra_nudge;
                    b += extra_nudge;
                }

                let tile_id = if selected || hovered {
                    scene.selected
                } else {
                    scene.white
                };

                scene
                    .overlay_3d
                    .add_line_3d(id, tile_id, a, b, thickness, normal, 100);
            };

            let block_grid_active = server_ctx.block_tool_active;
            let prefab_preview_color = Self::overlay_color(self.prefab_preview_wire_rgb, 0.92);
            let prefab_preview_footprint_color =
                Self::overlay_color(self.prefab_preview_wire_rgb, 0.95);
            let grid_step = if block_grid_active {
                crate::blocks::block_tool_horizontal_grid_step(
                    server_ctx.curr_block_asset_id,
                    &assets.block_props,
                    map,
                    server_ctx,
                )
            } else {
                ServerContext::edit_grid_step(map.subdivisions)
            };
            let grid_base_y = if block_grid_active {
                server_ctx.block_grid_level as f32 * server_ctx.block_grid_cell_size.max(0.05)
            } else {
                0.0
            };
            let grid_y = if block_grid_active {
                grid_base_y + 0.012
            } else {
                0.012
            };
            if block_grid_active
                && let Some(template) = server_ctx.block_recipe_template.as_ref()
                && let Some(hit) = crate::blocks::block_grid_plane_hit(server_ctx)
                    .or(server_ctx.hover_cursor_3d)
                    .or(server_ctx.hover_surface_hit_pos)
            {
                let cell = Vec3::new(
                    (hit.x / grid_step).floor() as i32,
                    server_ctx.block_grid_level,
                    (hit.z / grid_step).floor() as i32,
                );
                let cells = server_ctx.block_drag_start_cell.map_or_else(
                    || vec![cell],
                    |start| {
                        crate::blocks::block_stroke_cells(
                            start,
                            server_ctx.block_drag_end_cell.unwrap_or(start),
                            server_ctx.block_stroke_mode,
                        )
                    },
                );
                let base = server_ctx
                    .block_drag_base_y
                    .or_else(|| crate::blocks::block_surface_base_y(server_ctx, grid_base_y))
                    .unwrap_or(grid_base_y);
                let mut ghost = template.clone();
                for (cell_index, cell) in cells
                    .iter()
                    .take((50_000 / template.vertices.len().max(1)).clamp(1, 128))
                    .enumerate()
                {
                    ghost.transform = crate::tools::props_recipe::placement_transform(
                        *cell,
                        base,
                        grid_step,
                        server_ctx.block_rotation_quarters,
                    );
                    let vertices = ghost
                        .vertices
                        .iter()
                        .map(|v| ghost.transform_point(*v))
                        .collect::<Vec<_>>();
                    let mut edges = std::collections::HashSet::new();
                    for face in &ghost.faces {
                        for (a, b) in face
                            .indices
                            .iter()
                            .zip(face.indices.iter().cycle().skip(1))
                            .take(face.indices.len())
                        {
                            let edge = ((*a).min(*b), (*a).max(*b));
                            if edges.insert(edge)
                                && let Some((a, b)) = vertices.get(*a).zip(vertices.get(*b))
                            {
                                scene.overlay_3d.add_hardware_line_3d(
                                    GeoId::Unknown(
                                        0xE320_0000u32
                                            .wrapping_add((cell_index as u32) << 16)
                                            .wrapping_add(edges.len() as u32),
                                    ),
                                    *a + cam_forward * -0.006,
                                    *b + cam_forward * -0.006,
                                    prefab_preview_color,
                                    15,
                                );
                            }
                        }
                    }
                }
            }

            if block_grid_active
                && let Some(asset_id) = server_ctx.curr_block_asset_id
                && let Some(asset) = crate::blocks::block_asset(asset_id)
                && let Some(hit) = crate::blocks::block_grid_plane_hit(server_ctx)
                    .or(server_ctx.hover_cursor_3d)
                    .or(server_ctx.hover_surface_hit_pos)
                    .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))
            {
                let rotation = server_ctx.block_rotation_quarters.rem_euclid(4);
                let block_sizing = crate::blocks::block_sizing_from_context(server_ctx);
                let width_extra = if crate::blocks::asset_supports_width(asset) {
                    block_sizing.span_extra_cells
                } else {
                    0.0
                };
                let depth_extra = if crate::blocks::asset_supports_depth(asset) {
                    block_sizing.depth_extra_cells
                } else {
                    0.0
                };
                let (footprint_x, footprint_z, offset_x, offset_z) = if rotation % 2 == 0 {
                    (
                        asset.footprint.x as f32 + width_extra * 2.0,
                        asset.footprint.z as f32 + depth_extra * 2.0,
                        -width_extra,
                        -depth_extra,
                    )
                } else {
                    (
                        asset.footprint.z as f32 + depth_extra * 2.0,
                        asset.footprint.x as f32 + width_extra * 2.0,
                        -depth_extra,
                        -width_extra,
                    )
                };
                let size_x = footprint_x * grid_step;
                let size_z = footprint_z * grid_step;
                let outline_offset_x = offset_x * grid_step;
                let outline_offset_z = offset_z * grid_step;
                let hover_cell = Vec3::new(
                    (hit.x / grid_step).floor() as i32,
                    server_ctx.block_grid_level,
                    (hit.z / grid_step).floor() as i32,
                );
                let cells = if let Some(start) = server_ctx.block_drag_start_cell {
                    crate::blocks::block_stroke_cells(
                        start,
                        server_ctx.block_drag_end_cell.unwrap_or(start),
                        server_ctx.block_stroke_mode,
                    )
                } else {
                    vec![hover_cell]
                };
                let preview_base_y = server_ctx
                    .block_drag_base_y
                    .or_else(|| crate::blocks::block_surface_base_y(server_ctx, grid_base_y))
                    .unwrap_or(grid_base_y);
                let y = preview_base_y + 0.030;
                let preview_color = prefab_preview_color;
                let preview_fill = scene.selected;
                let erase_preview =
                    server_ctx.block_operation == crate::blocks::BLOCK_OPERATION_ERASE;
                let fill_preview_boxes = !erase_preview && asset.name != "Stairs";

                for (cell_index, cell) in cells.iter().enumerate() {
                    let x0 = cell.x as f32 * grid_step;
                    let z0 = cell.z as f32 * grid_step;
                    let base = Vec3::new(x0, preview_base_y, z0);
                    let outline_x0 = x0 + if erase_preview { 0.0 } else { outline_offset_x };
                    let outline_z0 = z0 + if erase_preview { 0.0 } else { outline_offset_z };
                    let corners = [
                        Vec3::new(outline_x0, y, outline_z0),
                        Vec3::new(
                            outline_x0 + if erase_preview { grid_step } else { size_x },
                            y,
                            outline_z0,
                        ),
                        Vec3::new(
                            outline_x0 + if erase_preview { grid_step } else { size_x },
                            y,
                            outline_z0 + if erase_preview { grid_step } else { size_z },
                        ),
                        Vec3::new(
                            outline_x0,
                            y,
                            outline_z0 + if erase_preview { grid_step } else { size_z },
                        ),
                    ];
                    for (edge_index, (a, b)) in
                        [(0, 1), (1, 2), (2, 3), (3, 0)].into_iter().enumerate()
                    {
                        scene.overlay_3d.add_hardware_line_3d(
                            GeoId::Unknown(
                                0xE306_0000u32
                                    .wrapping_add((cell_index as u32) << 4)
                                    .wrapping_add(edge_index as u32),
                            ),
                            corners[a],
                            corners[b],
                            if erase_preview {
                                [0.90, 0.22, 0.14, 0.95]
                            } else {
                                prefab_preview_footprint_color
                            },
                            14,
                        );
                    }

                    if erase_preview {
                        continue;
                    }

                    for box_index in 0..asset.boxes.len() {
                        let Some((min, max)) = crate::blocks::adjusted_rotated_bounds(
                            asset,
                            box_index,
                            block_sizing,
                            rotation,
                        ) else {
                            continue;
                        };
                        let min = base + min * grid_step;
                        let max = base + max * grid_step;
                        let corners = [
                            Vec3::new(min.x, min.y, min.z),
                            Vec3::new(max.x, min.y, min.z),
                            Vec3::new(max.x, max.y, min.z),
                            Vec3::new(min.x, max.y, min.z),
                            Vec3::new(min.x, min.y, max.z),
                            Vec3::new(max.x, min.y, max.z),
                            Vec3::new(max.x, max.y, max.z),
                            Vec3::new(min.x, max.y, max.z),
                        ];
                        let vertices = corners
                            .iter()
                            .map(|point| [point.x, point.y, point.z, 1.0])
                            .collect::<Vec<_>>();
                        let uvs = vec![
                            [0.0, 0.0],
                            [1.0, 0.0],
                            [1.0, 1.0],
                            [0.0, 1.0],
                            [0.0, 0.0],
                            [1.0, 0.0],
                            [1.0, 1.0],
                            [0.0, 1.0],
                        ];
                        let indices = vec![
                            (0, 1, 2),
                            (0, 2, 3),
                            (5, 4, 7),
                            (5, 7, 6),
                            (4, 0, 3),
                            (4, 3, 7),
                            (1, 5, 6),
                            (1, 6, 2),
                            (3, 2, 6),
                            (3, 6, 7),
                            (4, 5, 1),
                            (4, 1, 0),
                        ];
                        if fill_preview_boxes {
                            let mut fill = scenevm::Poly3D::poly(
                                GeoId::Unknown(
                                    0xE307_0000u32
                                        .wrapping_add((cell_index as u32) << 8)
                                        .wrapping_add(box_index as u32),
                                ),
                                preview_fill,
                                vertices,
                                uvs,
                                indices,
                            )
                            .with_opacity(0.20);
                            fill.layer = 13;
                            scene.overlay_3d.add_3d(fill);
                        }

                        for (edge_index, (a, b)) in [
                            (0, 1),
                            (1, 2),
                            (2, 3),
                            (3, 0),
                            (4, 5),
                            (5, 6),
                            (6, 7),
                            (7, 4),
                            (0, 4),
                            (1, 5),
                            (2, 6),
                            (3, 7),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            scene.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE308_0000u32
                                        .wrapping_add((cell_index as u32) << 8)
                                        .wrapping_add((box_index as u32) << 5)
                                        .wrapping_add(edge_index as u32),
                                ),
                                corners[a] + cam_forward * -0.006,
                                corners[b] + cam_forward * -0.006,
                                preview_color,
                                15,
                            );
                        }
                    }
                }
            }

            if block_grid_active
                && let Some(asset_id) = server_ctx.curr_block_asset_id
                && crate::blocks::block_asset(asset_id).is_none()
                && assets.block_props.get(&asset_id).is_some_and(|asset| {
                    asset.placement.mode == rusterix::BlockPropPlacementMode::Ground
                })
                && let Some(hit) = crate::blocks::block_grid_plane_hit(server_ctx)
                    .or(server_ctx.hover_cursor_3d)
                    .or(server_ctx.hover_surface_hit_pos)
                    .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))
            {
                let hover_cell = Vec3::new(
                    (hit.x / grid_step).floor() as i32,
                    server_ctx.block_grid_level,
                    (hit.z / grid_step).floor() as i32,
                );
                let cells = if let Some(start) = server_ctx.block_drag_start_cell {
                    crate::blocks::block_stroke_cells(
                        start,
                        server_ctx.block_drag_end_cell.unwrap_or(start),
                        server_ctx.block_stroke_mode,
                    )
                } else {
                    vec![hover_cell]
                };
                let preview_base_y = server_ctx
                    .block_drag_base_y
                    .or_else(|| crate::blocks::block_surface_base_y(server_ctx, grid_base_y))
                    .unwrap_or(grid_base_y);
                let erase_preview =
                    server_ctx.block_operation == crate::blocks::BLOCK_OPERATION_ERASE;
                let rotation = server_ctx.block_rotation_quarters.rem_euclid(4);
                let angle = rotation as f32 * std::f32::consts::FRAC_PI_2;
                let (sin, cos) = angle.sin_cos();

                for (cell_index, cell) in cells.iter().enumerate() {
                    if erase_preview {
                        let x0 = cell.x as f32 * grid_step;
                        let z0 = cell.z as f32 * grid_step;
                        let y = preview_base_y + 0.030;
                        let corners = [
                            Vec3::new(x0, y, z0),
                            Vec3::new(x0 + grid_step, y, z0),
                            Vec3::new(x0 + grid_step, y, z0 + grid_step),
                            Vec3::new(x0, y, z0 + grid_step),
                        ];
                        for (edge_index, (a, b)) in
                            [(0, 1), (1, 2), (2, 3), (3, 0)].into_iter().enumerate()
                        {
                            scene.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE309_0000u32
                                        .wrapping_add((cell_index as u32) << 4)
                                        .wrapping_add(edge_index as u32),
                                ),
                                corners[a],
                                corners[b],
                                [0.90, 0.22, 0.14, 0.95],
                                15,
                            );
                        }
                        continue;
                    }

                    let mut instance = rusterix::BlockPropInstance::new(asset_id);
                    instance.world_transform[0][0] = cos;
                    instance.world_transform[0][2] = -sin;
                    instance.world_transform[2][0] = sin;
                    instance.world_transform[2][2] = cos;
                    instance.world_transform[3][0] = cell.x as f32 * grid_step;
                    instance.world_transform[3][1] = preview_base_y;
                    instance.world_transform[3][2] = cell.z as f32 * grid_step;
                    if let Some(asset) = assets.block_props.get(&asset_id) {
                        crate::blocks::apply_prefab_auto_sizing(
                            asset,
                            &mut instance,
                            crate::blocks::block_sizing_from_context(server_ctx),
                        );
                    }
                    let resolved =
                        rusterix::resolve_block_prop_geometry(&[instance], &assets.block_props);
                    for (object_index, object) in resolved.geometry_objects.iter().enumerate() {
                        for (face_index, face) in object.faces.iter().enumerate() {
                            if face.indices.len() < 2 {
                                continue;
                            }
                            for edge_index in 0..face.indices.len() {
                                let a_index = face.indices[edge_index];
                                let b_index = face.indices[(edge_index + 1) % face.indices.len()];
                                let (Some(a), Some(b)) =
                                    (object.vertices.get(a_index), object.vertices.get(b_index))
                                else {
                                    continue;
                                };
                                scene.overlay_3d.add_hardware_line_3d(
                                    GeoId::Unknown(
                                        0xE30A_0000u32
                                            .wrapping_add((cell_index as u32) << 12)
                                            .wrapping_add((object_index as u32) << 8)
                                            .wrapping_add((face_index as u32) << 4)
                                            .wrapping_add(edge_index as u32),
                                    ),
                                    object.transform_point(*a) + cam_forward * -0.006,
                                    object.transform_point(*b) + cam_forward * -0.006,
                                    prefab_preview_footprint_color,
                                    15,
                                );
                            }
                        }
                    }
                }
            }

            if block_grid_active
                && server_ctx.block_operation != crate::blocks::BLOCK_OPERATION_ERASE
                && let Some(asset_id) = server_ctx.curr_block_asset_id
                && let Some(asset) = assets.block_props.get(&asset_id).cloned()
                && asset.placement.mode != rusterix::BlockPropPlacementMode::Ground
            {
                let valid = crate::blocks::prefab_surface_placement_valid(&asset, map, server_ctx);
                let placement_color = if valid {
                    [0.26, 0.92, 0.42, 0.98]
                } else {
                    [0.96, 0.22, 0.18, 0.98]
                };
                if let Some(instance) =
                    crate::blocks::surface_prefab_preview_instance(&asset, map, server_ctx)
                {
                    let resolved =
                        rusterix::resolve_block_prop_geometry(&[instance], &assets.block_props);
                    let mut line_index = 0u32;
                    for object in &resolved.geometry_objects {
                        for face in &object.faces {
                            if face.indices.len() < 2 {
                                continue;
                            }
                            for edge_index in 0..face.indices.len() {
                                let Some(a) = object.vertices.get(face.indices[edge_index]) else {
                                    continue;
                                };
                                let Some(b) = object
                                    .vertices
                                    .get(face.indices[(edge_index + 1) % face.indices.len()])
                                else {
                                    continue;
                                };
                                scene.overlay_3d.add_hardware_line_3d(
                                    GeoId::Unknown(0xE30C_0000u32.wrapping_add(line_index)),
                                    object.transform_point(*a) + cam_forward * -0.006,
                                    object.transform_point(*b) + cam_forward * -0.006,
                                    placement_color,
                                    15,
                                );
                                line_index = line_index.wrapping_add(1);
                            }
                        }
                    }
                } else if let Some(hit) = server_ctx
                    .hover_surface_hit_pos
                    .or(server_ctx.hover_cursor_3d)
                    .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))
                {
                    let size = overlay_world_size(hit).clamp(0.08, 0.28);
                    for (index, axis) in [Vec3::unit_x(), Vec3::unit_y(), Vec3::unit_z()]
                        .into_iter()
                        .enumerate()
                    {
                        scene.overlay_3d.add_hardware_line_3d(
                            GeoId::Unknown(0xE30D_0000u32.wrapping_add(index as u32)),
                            hit - axis * size,
                            hit + axis * size,
                            placement_color,
                            17,
                        );
                    }
                }
            }

            if !map.linedefs.is_empty() {
                let reference_y = grid_y + 0.010;
                let mut reference_index = 0u32;
                for linedef in &map.linedefs {
                    let Some(a) = map.find_vertex(linedef.start_vertex) else {
                        continue;
                    };
                    let Some(b) = map.find_vertex(linedef.end_vertex) else {
                        continue;
                    };
                    let is_selected = map.selected_linedefs.contains(&linedef.id)
                        || map.sectors.iter().any(|sector| {
                            map.selected_sectors.contains(&sector.id)
                                && sector.linedefs.contains(&linedef.id)
                        });
                    scene.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE303_0000u32.wrapping_add(reference_index)),
                        Vec3::new(a.x, reference_y, a.y),
                        Vec3::new(b.x, reference_y, b.y),
                        if is_selected {
                            [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 0.88]
                        } else {
                            [0.74, 0.74, 0.74, 0.44]
                        },
                        12,
                    );
                    reference_index = reference_index.wrapping_add(1);
                }

                for (vertex_index, vertex) in map.vertices.iter().enumerate() {
                    if !map.selected_vertices.contains(&vertex.id) {
                        continue;
                    }
                    let center = Vec3::new(vertex.x, reference_y + 0.004, vertex.y);
                    let half = 0.08;
                    scene.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE304_0000u32.wrapping_add(vertex_index as u32)),
                        center - Vec3::new(half, 0.0, 0.0),
                        center + Vec3::new(half, 0.0, 0.0),
                        [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 0.88],
                        13,
                    );
                    scene.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE305_0000u32.wrapping_add(vertex_index as u32)),
                        center - Vec3::new(0.0, 0.0, half),
                        center + Vec3::new(0.0, 0.0, half),
                        [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 0.88],
                        13,
                    );
                }
            }

            if server_ctx.curr_map_tool_type == MapToolType::Selection
                && !map.selected_block_prop_instances.is_empty()
            {
                let selected_ids = map
                    .selected_block_prop_instances
                    .iter()
                    .copied()
                    .collect::<FxHashSet<_>>();
                let selected_instances = map
                    .block_prop_instances
                    .iter()
                    .filter(|instance| selected_ids.contains(&instance.id))
                    .cloned()
                    .collect::<Vec<_>>();
                let resolved =
                    rusterix::resolve_block_prop_geometry(&selected_instances, &assets.block_props);
                let mut line_index = 0u32;
                for object in &resolved.geometry_objects {
                    for face in &object.faces {
                        for edge_index in 0..face.indices.len() {
                            let Some(a) = object.vertices.get(face.indices[edge_index]) else {
                                continue;
                            };
                            let Some(b) = object
                                .vertices
                                .get(face.indices[(edge_index + 1) % face.indices.len()])
                            else {
                                continue;
                            };
                            scene.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(0xE30B_0000u32.wrapping_add(line_index)),
                                object.transform_point(*a) + view_nudge,
                                object.transform_point(*b) + view_nudge,
                                [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0],
                                21,
                            );
                            line_index = line_index.wrapping_add(1);
                        }
                    }
                }
            }

            let direct_geometry_tool = matches!(
                server_ctx.curr_map_tool_type,
                MapToolType::Selection
                    | MapToolType::Vertex
                    | MapToolType::Linedef
                    | MapToolType::Sector
            );
            let selected_tile = scene.selected;
            let white_tile = scene.white;
            let push_cube_marker = |scene: &mut rusterix::SceneHandler,
                                    marker_id: GeoId,
                                    selected: bool,
                                    center: Vec3<f32>,
                                    size: f32,
                                    opacity: f32,
                                    layer: i32| {
                let mut cube = scenevm::Poly3D::cube(
                    marker_id,
                    if selected { selected_tile } else { white_tile },
                    center,
                    size,
                )
                .with_opacity(opacity);
                cube.layer = layer;
                scene.overlay_3d.add_3d(cube);
            };
            let push_handle_rect = |scene: &mut rusterix::SceneHandler,
                                    outline_id_base: u32,
                                    hit_id: GeoId,
                                    selected: bool,
                                    center: Vec3<f32>,
                                    size: f32,
                                    color: [f32; 4],
                                    opacity: f32| {
                let half = size * 0.5;
                let p0 = center - view_right * half - view_up * half;
                let p1 = center + view_right * half - view_up * half;
                let p2 = center + view_right * half + view_up * half;
                let p3 = center - view_right * half + view_up * half;
                let mut fill = scenevm::Poly3D::poly(
                    hit_id,
                    if selected { selected_tile } else { white_tile },
                    vec![
                        [p0.x, p0.y, p0.z, 1.0],
                        [p1.x, p1.y, p1.z, 1.0],
                        [p2.x, p2.y, p2.z, 1.0],
                        [p3.x, p3.y, p3.z, 1.0],
                    ],
                    vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    vec![(0, 1, 2), (0, 2, 3)],
                )
                .with_opacity(opacity);
                fill.layer = 39;
                scene.overlay_3d.add_3d(fill);

                for (index, (a, b)) in [(p0, p1), (p1, p2), (p2, p3), (p3, p0)]
                    .into_iter()
                    .enumerate()
                {
                    scene.overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(outline_id_base.wrapping_add(index as u32)),
                        a,
                        b,
                        color,
                        40,
                    );
                }
            };
            if server_ctx.curr_map_tool_type == MapToolType::Effects
                && let Some(asset_id) = server_ctx.curr_block_asset_id
                && let Some(asset) = assets.block_props.get(&asset_id).cloned()
            {
                let instance = rusterix::BlockPropInstance::new(asset_id);
                // Disabled effects still need editor handles so they can be
                // selected, repositioned, and switched back on.
                let mut preview_asset = asset.clone();
                for effect in &mut preview_asset.particle_effects {
                    effect.enabled = true;
                }
                for effect in &mut preview_asset.light_effects {
                    effect.enabled = true;
                }
                let mut preview_catalog = assets.block_props.clone();
                preview_catalog.insert(asset_id, preview_asset);
                let resolved = rusterix::resolve_block_prop_effects(&[instance], &preview_catalog);
                let mut handles = resolved
                    .particles
                    .iter()
                    .map(|effect| (effect.effect_id, effect.origin, effect.direction))
                    .collect::<Vec<_>>();
                for light in &resolved.lights {
                    let direction = asset
                        .light_effects
                        .iter()
                        .find(|effect| effect.id == light.effect_id)
                        .and_then(|effect| {
                            asset.find_part(effect.part_id).and_then(|part| {
                                part.attachments
                                    .iter()
                                    .find(|attachment| attachment.id == effect.attachment_id)
                            })
                        })
                        .map(|attachment| Vec3::from(attachment.direction))
                        .and_then(|direction| direction.try_normalized())
                        .unwrap_or(Vec3::unit_y());
                    handles.push((light.effect_id, light.position, direction));
                }
                for (index, (effect_id, origin, direction)) in handles.into_iter().enumerate() {
                    let selected = server_ctx.selected_prefab_effect_id == Some(effect_id);
                    let size = overlay_world_size(origin);
                    let marker_size = (size * 1.25).clamp(0.08, 0.24);
                    let direction_length = (size * 5.5).clamp(0.35, 1.35);
                    let direction_end = origin + direction * direction_length;
                    let move_id = 0xEFFE_0000u32.wrapping_add(index as u32);
                    let direction_id = 0xEFFF_0000u32.wrapping_add(index as u32);
                    push_cube_marker(
                        scene,
                        GeoId::Gizmo(move_id),
                        selected,
                        origin + view_nudge,
                        marker_size,
                        if selected { 0.95 } else { 0.72 },
                        47,
                    );
                    scene.overlay_3d.add_hardware_line_3d(
                        GeoId::Gizmo(direction_id),
                        origin + view_nudge,
                        direction_end + view_nudge,
                        if selected {
                            [1.0, 0.68, 0.20, 1.0]
                        } else {
                            [0.44, 0.78, 1.0, 0.92]
                        },
                        48,
                    );
                    push_cube_marker(
                        scene,
                        GeoId::Gizmo(direction_id),
                        selected,
                        direction_end + view_nudge,
                        marker_size * 0.72,
                        0.90,
                        49,
                    );

                    if selected
                        && let Some(effect) = asset
                            .particle_effects
                            .iter()
                            .find(|effect| effect.id == effect_id)
                    {
                        let half: Vec3<f32> = Vec3::from(effect.emitter.spawn_area);
                        let corners = [
                            origin + Vec3::new(-half.x, -half.y, -half.z),
                            origin + Vec3::new(half.x, -half.y, -half.z),
                            origin + Vec3::new(half.x, half.y, -half.z),
                            origin + Vec3::new(-half.x, half.y, -half.z),
                            origin + Vec3::new(-half.x, -half.y, half.z),
                            origin + Vec3::new(half.x, -half.y, half.z),
                            origin + Vec3::new(half.x, half.y, half.z),
                            origin + Vec3::new(-half.x, half.y, half.z),
                        ];
                        for (edge_index, (a, b)) in [
                            (0, 1),
                            (1, 2),
                            (2, 3),
                            (3, 0),
                            (4, 5),
                            (5, 6),
                            (6, 7),
                            (7, 4),
                            (0, 4),
                            (1, 5),
                            (2, 6),
                            (3, 7),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            scene.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xEFFC_0000u32
                                        .wrapping_add((index as u32) << 5)
                                        .wrapping_add(edge_index as u32),
                                ),
                                corners[a] + view_nudge,
                                corners[b] + view_nudge,
                                [0.40, 0.92, 1.0, 0.86],
                                46,
                            );
                        }
                    } else if selected
                        && let Some(light) = asset
                            .light_effects
                            .iter()
                            .find(|light| light.id == effect_id)
                    {
                        let radius = light.range.max(0.01);
                        let segments = 32u32;
                        for segment in 0..segments {
                            let a = segment as f32 / segments as f32 * std::f32::consts::TAU;
                            let b = (segment + 1) as f32 / segments as f32 * std::f32::consts::TAU;
                            for (plane, (from, to)) in [
                                (
                                    0u32,
                                    (
                                        origin + Vec3::new(a.cos() * radius, 0.0, a.sin() * radius),
                                        origin + Vec3::new(b.cos() * radius, 0.0, b.sin() * radius),
                                    ),
                                ),
                                (
                                    1u32,
                                    (
                                        origin + Vec3::new(a.cos() * radius, a.sin() * radius, 0.0),
                                        origin + Vec3::new(b.cos() * radius, b.sin() * radius, 0.0),
                                    ),
                                ),
                            ] {
                                scene.overlay_3d.add_hardware_line_3d(
                                    GeoId::Unknown(
                                        0xEFFD_0000u32
                                            .wrapping_add((index as u32) << 7)
                                            .wrapping_add(plane << 6)
                                            .wrapping_add(segment),
                                    ),
                                    from + view_nudge,
                                    to + view_nudge,
                                    [1.0, 0.72, 0.24, 0.72],
                                    46,
                                );
                            }
                        }
                    }
                }
            }
            if server_ctx.curr_map_tool_type == MapToolType::Selection
                && server_ctx.geometry_gizmo_op == GeometryGizmoOp::Move
                && !map.selected_block_prop_instances.is_empty()
            {
                let selected_ids = map
                    .selected_block_prop_instances
                    .iter()
                    .copied()
                    .collect::<FxHashSet<_>>();
                let selected_instances = map
                    .block_prop_instances
                    .iter()
                    .filter(|instance| selected_ids.contains(&instance.id))
                    .cloned()
                    .collect::<Vec<_>>();
                let resolved =
                    rusterix::resolve_block_prop_geometry(&selected_instances, &assets.block_props);
                let mut min = Vec3::broadcast(f32::INFINITY);
                let mut max = Vec3::broadcast(f32::NEG_INFINITY);
                let mut found = false;
                for object in &resolved.geometry_objects {
                    for vertex in &object.vertices {
                        let world = object.transform_point(*vertex);
                        if !world.x.is_finite() || !world.y.is_finite() || !world.z.is_finite() {
                            continue;
                        }
                        min.x = min.x.min(world.x);
                        min.y = min.y.min(world.y);
                        min.z = min.z.min(world.z);
                        max.x = max.x.max(world.x);
                        max.y = max.y.max(world.y);
                        max.z = max.z.max(world.z);
                        found = true;
                    }
                }
                if found {
                    let center = (min + max) * 0.5;
                    let base_overlay_size = overlay_world_size(center);
                    let axis_len = (base_overlay_size * 7.0).clamp(0.38, 1.75);
                    let handle_size = (base_overlay_size * 1.15).clamp(0.08, 0.22);
                    let id_salt = map
                        .selected_block_prop_instances
                        .first()
                        .map(|id| (id.as_u128() as u32) & 0x0000_F000)
                        .unwrap_or(0);
                    let local_axes = selected_instances.first().map(|instance| {
                        std::array::from_fn::<_, 3, _>(|i| {
                            Vec3::new(
                                instance.world_transform[i][0],
                                instance.world_transform[i][1],
                                instance.world_transform[i][2],
                            )
                            .normalized()
                        })
                    });
                    let axes = if let Some(local_axes) = local_axes {
                        [
                            (201, local_axes[0], [0.86, 0.22, 0.22, 1.0]),
                            (202, local_axes[1], [0.28, 0.78, 0.32, 1.0]),
                            (203, local_axes[2], [0.32, 0.48, 0.94, 1.0]),
                        ]
                    } else {
                        [
                            (1, Vec3::unit_x(), [0.86, 0.22, 0.22, 1.0]),
                            (2, Vec3::unit_y(), [0.28, 0.78, 0.32, 1.0]),
                            (3, Vec3::unit_z(), [0.32, 0.48, 0.94, 1.0]),
                        ]
                    };
                    for (axis_id, axis, color) in axes {
                        let handle_center = center + axis * axis_len + view_nudge;
                        scene.overlay_3d.add_hardware_line_3d(
                            GeoId::Gizmo(axis_id),
                            center + view_nudge,
                            handle_center,
                            color,
                            42 + axis_id as i32,
                        );
                        push_handle_rect(
                            scene,
                            0xE3C0_0000u32
                                .wrapping_add(id_salt)
                                .wrapping_add(axis_id << 4),
                            GeoId::Gizmo(axis_id),
                            false,
                            handle_center + cam_forward * -0.012,
                            handle_size,
                            color,
                            0.88,
                        );
                    }
                }
            }
            for object in &map.geometry_objects {
                let selected = map.selected_geometry_objects.contains(&object.id);
                let hovered = server_ctx.geo_hit == Some(GeoId::GeometryObject(object.id));
                if !selected && !hovered {
                    continue;
                }
                let id_salt = (object.id.as_u128() as u32) & 0x0000_F000;

                let mut min = Vec3::broadcast(f32::INFINITY);
                let mut max = Vec3::broadcast(f32::NEG_INFINITY);
                let mut found = false;
                for vertex in &object.vertices {
                    let world = object.transform_point(*vertex) + view_nudge;
                    if !world.x.is_finite() || !world.y.is_finite() || !world.z.is_finite() {
                        continue;
                    }
                    min.x = min.x.min(world.x);
                    min.y = min.y.min(world.y);
                    min.z = min.z.min(world.z);
                    max.x = max.x.max(world.x);
                    max.y = max.y.max(world.y);
                    max.z = max.z.max(world.z);
                    found = true;
                }
                if !found {
                    continue;
                }

                if direct_geometry_tool {
                    let mut gizmo_min = Vec3::broadcast(f32::INFINITY);
                    let mut gizmo_max = Vec3::broadcast(f32::NEG_INFINITY);
                    let mut gizmo_found = false;
                    let mut has_mode_selection =
                        server_ctx.curr_map_tool_type == MapToolType::Selection;
                    if server_ctx.curr_map_tool_type != MapToolType::Selection {
                        for (_, vertex_index) in map
                            .selected_geometry_vertices
                            .iter()
                            .filter(|(id, _)| *id == object.id)
                        {
                            let Some(vertex) = object.vertices.get(*vertex_index) else {
                                continue;
                            };
                            let world = object.transform_point(*vertex) + view_nudge;
                            gizmo_min.x = gizmo_min.x.min(world.x);
                            gizmo_min.y = gizmo_min.y.min(world.y);
                            gizmo_min.z = gizmo_min.z.min(world.z);
                            gizmo_max.x = gizmo_max.x.max(world.x);
                            gizmo_max.y = gizmo_max.y.max(world.y);
                            gizmo_max.z = gizmo_max.z.max(world.z);
                            gizmo_found = true;
                            if matches!(
                                server_ctx.curr_map_tool_type,
                                MapToolType::Vertex | MapToolType::Linedef
                            ) {
                                has_mode_selection = true;
                            }
                        }
                        for (_, face_index) in map
                            .selected_geometry_faces
                            .iter()
                            .filter(|(id, _)| *id == object.id)
                        {
                            let Some(face) = object.faces.get(*face_index) else {
                                continue;
                            };
                            for vertex_index in &face.indices {
                                let Some(vertex) = object.vertices.get(*vertex_index) else {
                                    continue;
                                };
                                let world = object.transform_point(*vertex) + view_nudge;
                                gizmo_min.x = gizmo_min.x.min(world.x);
                                gizmo_min.y = gizmo_min.y.min(world.y);
                                gizmo_min.z = gizmo_min.z.min(world.z);
                                gizmo_max.x = gizmo_max.x.max(world.x);
                                gizmo_max.y = gizmo_max.y.max(world.y);
                                gizmo_max.z = gizmo_max.z.max(world.z);
                                gizmo_found = true;
                            }
                            if server_ctx.curr_map_tool_type == MapToolType::Sector {
                                has_mode_selection = true;
                            }
                        }
                    }
                    let gizmo_min = if gizmo_found { gizmo_min } else { min };
                    let gizmo_max = if gizmo_found { gizmo_max } else { max };
                    let center = (gizmo_min + gizmo_max) * 0.5;
                    let base_overlay_size = overlay_world_size(center);
                    let axis_len = (base_overlay_size * 7.0).clamp(0.38, 1.75);
                    let move_handle_size = (base_overlay_size * 1.15).clamp(0.08, 0.22);
                    let resize_handle_size = (base_overlay_size * 0.95).clamp(0.07, 0.18);
                    let draw_selection_gizmo = server_ctx.curr_map_tool_type
                        != MapToolType::Selection
                        || map.selected_geometry_objects.len() <= 1
                        || map
                            .selected_geometry_objects
                            .first()
                            .is_some_and(|selected_id| *selected_id == object.id);
                    if selected && has_mode_selection && draw_selection_gizmo {
                        let show_move_gizmo = server_ctx.curr_map_tool_type
                            != MapToolType::Selection
                            || server_ctx.geometry_gizmo_op == GeometryGizmoOp::Move;
                        let show_resize_gizmo = server_ctx.curr_map_tool_type
                            == MapToolType::Selection
                            && server_ctx.geometry_gizmo_op == GeometryGizmoOp::Resize;
                        if show_move_gizmo {
                            for (axis_id, delta, color) in [
                                (1, Vec3::new(axis_len, 0.0, 0.0), [0.86, 0.22, 0.22, 1.0]),
                                (2, Vec3::new(0.0, axis_len, 0.0), [0.28, 0.78, 0.32, 1.0]),
                                (3, Vec3::new(0.0, 0.0, axis_len), [0.32, 0.48, 0.94, 1.0]),
                            ]
                            .into_iter()
                            {
                                let handle_center = center + delta + view_nudge;
                                scene.overlay_3d.add_hardware_line_3d(
                                    GeoId::Gizmo(axis_id),
                                    center + view_nudge,
                                    handle_center,
                                    color,
                                    42 + axis_id as i32,
                                );
                                push_handle_rect(
                                    scene,
                                    0xE3A0_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add(axis_id << 4),
                                    GeoId::Gizmo(axis_id),
                                    false,
                                    handle_center + cam_forward * -0.012,
                                    move_handle_size,
                                    color,
                                    0.88,
                                );
                            }
                        }
                        if show_resize_gizmo {
                            for (axis_id, handle_center, color) in [
                                (
                                    101,
                                    Vec3::new(gizmo_min.x, center.y, center.z),
                                    [0.86, 0.22, 0.22, 1.0],
                                ),
                                (
                                    102,
                                    Vec3::new(gizmo_max.x, center.y, center.z),
                                    [0.86, 0.22, 0.22, 1.0],
                                ),
                                (
                                    103,
                                    Vec3::new(center.x, gizmo_min.y, center.z),
                                    [0.28, 0.78, 0.32, 1.0],
                                ),
                                (
                                    104,
                                    Vec3::new(center.x, gizmo_max.y, center.z),
                                    [0.28, 0.78, 0.32, 1.0],
                                ),
                                (
                                    105,
                                    Vec3::new(center.x, center.y, gizmo_min.z),
                                    [0.32, 0.48, 0.94, 1.0],
                                ),
                                (
                                    106,
                                    Vec3::new(center.x, center.y, gizmo_max.z),
                                    [0.32, 0.48, 0.94, 1.0],
                                ),
                            ] {
                                push_handle_rect(
                                    scene,
                                    0xE3B0_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add(axis_id << 4),
                                    GeoId::Gizmo(axis_id),
                                    false,
                                    handle_center + view_nudge + cam_forward * -0.014,
                                    resize_handle_size,
                                    color,
                                    0.72,
                                );
                            }
                        }
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Selection {
                    let corners = [
                        Vec3::new(min.x, min.y, min.z),
                        Vec3::new(max.x, min.y, min.z),
                        Vec3::new(max.x, max.y, min.z),
                        Vec3::new(min.x, max.y, min.z),
                        Vec3::new(min.x, min.y, max.z),
                        Vec3::new(max.x, min.y, max.z),
                        Vec3::new(max.x, max.y, max.z),
                        Vec3::new(min.x, max.y, max.z),
                    ];
                    let edges = [
                        (0, 1),
                        (1, 2),
                        (2, 3),
                        (3, 0),
                        (4, 5),
                        (5, 6),
                        (6, 7),
                        (7, 4),
                        (0, 4),
                        (1, 5),
                        (2, 6),
                        (3, 7),
                    ];
                    let color = if selected {
                        [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                    } else {
                        [1.0, 1.0, 1.0, 0.78]
                    };
                    for (edge_index, (a, b)) in edges.iter().enumerate() {
                        scene.overlay_3d.add_hardware_line_3d(
                            GeoId::Unknown(
                                0xE340_0000u32
                                    .wrapping_add(id_salt)
                                    .wrapping_add(edge_index as u32),
                            ),
                            corners[*a],
                            corners[*b],
                            color,
                            20,
                        );
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Vertex {
                    for (vertex_index, vertex) in object.vertices.iter().enumerate() {
                        let is_selected = map
                            .selected_geometry_vertices
                            .contains(&(object.id, vertex_index));
                        push_cube_marker(
                            scene,
                            GeoId::GeometryObject(object.id),
                            is_selected,
                            object.transform_point(*vertex) + view_nudge + cam_forward * -0.01,
                            overlay_world_size(object.transform_point(*vertex))
                                * if is_selected { 0.88 } else { 0.62 },
                            if is_selected { 0.94 } else { 0.66 },
                            39,
                        );
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Linedef {
                    let mut edge_index = 0u32;
                    let mut surface_line_index = 0u32;
                    let mut drawn_geometry_vertices = FxHashSet::default();
                    for (face_index, face) in object.faces.iter().enumerate() {
                        for index in 0..face.indices.len() {
                            let Some(a) = object.vertices.get(face.indices[index]) else {
                                continue;
                            };
                            let Some(b) = object
                                .vertices
                                .get(face.indices[(index + 1) % face.indices.len()])
                            else {
                                continue;
                            };
                            let a_selected = map
                                .selected_geometry_vertices
                                .contains(&(object.id, face.indices[index]));
                            let b_selected = map.selected_geometry_vertices.contains(&(
                                object.id,
                                face.indices[(index + 1) % face.indices.len()],
                            ));
                            let edge_selected = a_selected && b_selected;
                            scene.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE370_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add(edge_index),
                                ),
                                object.transform_point(*a) + view_nudge,
                                object.transform_point(*b) + view_nudge,
                                if edge_selected {
                                    [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                                } else {
                                    [1.0, 1.0, 1.0, 0.82]
                                },
                                20,
                            );
                            for (vertex_index, vertex, is_selected) in [
                                (face.indices[index], a, a_selected),
                                (
                                    face.indices[(index + 1) % face.indices.len()],
                                    b,
                                    b_selected,
                                ),
                            ] {
                                if is_selected && drawn_geometry_vertices.insert(vertex_index) {
                                    push_cube_marker(
                                        scene,
                                        GeoId::GeometryObject(object.id),
                                        true,
                                        object.transform_point(*vertex)
                                            + view_nudge
                                            + cam_forward * -0.01,
                                        overlay_world_size(object.transform_point(*vertex)) * 0.72,
                                        0.92,
                                        39,
                                    );
                                }
                            }
                            edge_index = edge_index.wrapping_add(1);
                        }

                        let normal = if face.indices.len() >= 3 {
                            let first = object.transform_point(
                                *object
                                    .vertices
                                    .get(face.indices[0])
                                    .unwrap_or(&Vec3::zero()),
                            );
                            let mut normal = Vec3::<f32>::zero();
                            for index in 1..face.indices.len().saturating_sub(1) {
                                let Some(a) = object.vertices.get(face.indices[index]) else {
                                    continue;
                                };
                                let Some(b) = object.vertices.get(face.indices[index + 1]) else {
                                    continue;
                                };
                                normal += (object.transform_point(*a) - first)
                                    .cross(object.transform_point(*b) - first);
                            }
                            normal.try_normalized().unwrap_or(cam_forward)
                        } else {
                            cam_forward
                        };
                        let mut drawn_surface_points = FxHashSet::default();
                        for (segment_index, segment) in face.surface_segments.iter().enumerate() {
                            let Some(points) = surface_segment_points(face, segment, normal, 8)
                            else {
                                continue;
                            };
                            let segment_selected = map
                                .selected_geometry_surface_segments
                                .contains(&(object.id, face_index, segment_index));
                            let world_points = points
                                .iter()
                                .map(|point| {
                                    object.transform_point(*point) + view_nudge + normal * 0.014
                                })
                                .collect::<Vec<_>>();
                            let side = world_points
                                .windows(2)
                                .find_map(|points| {
                                    let dir = (points[1] - points[0]).try_normalized()?;
                                    normal.cross(dir).try_normalized()
                                })
                                .unwrap_or_else(|| {
                                    cam_right.try_normalized().unwrap_or(Vec3::unit_x())
                                });
                            let line_offset = 0.018;
                            let base_id = 0xE390_0000u32
                                .wrapping_add(id_salt)
                                .wrapping_add((face_index as u32) << 11)
                                .wrapping_add(surface_line_index.wrapping_mul(128));
                            let strokes = if segment_selected {
                                [
                                    (-line_offset, [0.30, 0.12, 0.43, 0.88], 23),
                                    (0.0, [0.98, 0.72, 1.0, 1.0], 24),
                                    (line_offset, [1.0, 0.92, 1.0, 0.86], 25),
                                ]
                            } else {
                                [
                                    (-line_offset, [0.03, 0.03, 0.03, 0.62], 23),
                                    (0.0, [1.0, 1.0, 1.0, 0.98], 24),
                                    (line_offset, [0.44, 0.78, 1.0, 0.72], 25),
                                ]
                            };
                            for (stroke_index, (offset, color, layer)) in
                                strokes.into_iter().enumerate()
                            {
                                let delta = side * offset;
                                for (piece_index, piece) in world_points.windows(2).enumerate() {
                                    let offset_id = stroke_index as u32 * 32 + piece_index as u32;
                                    scene.overlay_3d.add_hardware_line_3d(
                                        GeoId::Unknown(base_id.wrapping_add(offset_id)),
                                        piece[0] + delta,
                                        piece[1] + delta,
                                        color,
                                        layer,
                                    );
                                }
                            }
                            for (point_index, point) in [
                                (segment.start, world_points[0]),
                                (
                                    segment.end,
                                    *world_points.last().unwrap_or(&world_points[0]),
                                ),
                            ] {
                                drawn_surface_points.insert(point_index);
                                let point_selected = map
                                    .selected_geometry_surface_points
                                    .contains(&(object.id, face_index, point_index))
                                    || segment_selected;
                                push_cube_marker(
                                    scene,
                                    GeoId::GeometryObject(object.id),
                                    point_selected,
                                    point,
                                    overlay_world_size(point)
                                        * if point_selected { 0.66 } else { 0.50 },
                                    if point_selected { 0.9 } else { 0.48 },
                                    39,
                                );
                            }
                            surface_line_index = surface_line_index.wrapping_add(1);
                        }
                        for (point_index, surface_point) in face.surface_points.iter().enumerate() {
                            if drawn_surface_points.contains(&point_index) {
                                continue;
                            }
                            let point = object.transform_point(surface_point.position)
                                + view_nudge
                                + normal * 0.014;
                            let point_selected = map.selected_geometry_surface_points.contains(&(
                                object.id,
                                face_index,
                                point_index,
                            ));
                            push_cube_marker(
                                scene,
                                GeoId::GeometryObject(object.id),
                                point_selected,
                                point,
                                overlay_world_size(point)
                                    * if point_selected { 0.66 } else { 0.50 },
                                if point_selected { 0.9 } else { 0.48 },
                                39,
                            );
                        }
                    }
                }

                if server_ctx.curr_map_tool_type == MapToolType::Sector {
                    for (face_index, face) in object.faces.iter().enumerate() {
                        let is_selected = map
                            .selected_geometry_faces
                            .contains(&(object.id, face_index));
                        if is_selected && face.indices.len() >= 3 {
                            let vertices = face
                                .indices
                                .iter()
                                .filter_map(|vertex_index| object.vertices.get(*vertex_index))
                                .map(|vertex| {
                                    let point = object.transform_point(*vertex)
                                        + view_nudge
                                        + cam_forward * -0.012;
                                    [point.x, point.y, point.z, 1.0]
                                })
                                .collect::<Vec<_>>();
                            if vertices.len() >= 3 {
                                let uv_len = vertices.len();
                                let mut indices = Vec::with_capacity(vertices.len() - 2);
                                for index in 1..vertices.len() - 1 {
                                    indices.push((0, index, index + 1));
                                }
                                let mut fill = scenevm::Poly3D::poly(
                                    GeoId::GeometryObject(object.id),
                                    selected_tile,
                                    vertices,
                                    vec![[0.0, 0.0]; uv_len],
                                    indices,
                                )
                                .with_opacity(0.32);
                                fill.layer = 38;
                                scene.overlay_3d.add_3d(fill);
                            }
                        }
                        for edge_index in 0..face.indices.len() {
                            let Some(a) = object.vertices.get(face.indices[edge_index]) else {
                                continue;
                            };
                            let Some(b) = object
                                .vertices
                                .get(face.indices[(edge_index + 1) % face.indices.len()])
                            else {
                                continue;
                            };
                            scene.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE380_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add((face_index as u32) << 8)
                                        .wrapping_add(edge_index as u32),
                                ),
                                object.transform_point(*a) + view_nudge,
                                object.transform_point(*b) + view_nudge,
                                if is_selected {
                                    [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0]
                                } else {
                                    [1.0, 1.0, 1.0, 0.82]
                                },
                                20,
                            );
                        }
                    }
                }

                if selected {
                    for (selection_index, (_, face_index)) in map
                        .selected_geometry_faces
                        .iter()
                        .filter(|(id, _)| *id == object.id)
                        .enumerate()
                    {
                        let Some(face) = object.faces.get(*face_index) else {
                            continue;
                        };
                        if face.indices.len() < 2 {
                            continue;
                        }
                        for edge_index in 0..face.indices.len() {
                            let Some(a) = object.vertices.get(face.indices[edge_index]) else {
                                continue;
                            };
                            let Some(b) = object
                                .vertices
                                .get(face.indices[(edge_index + 1) % face.indices.len()])
                            else {
                                continue;
                            };
                            scene.overlay_3d.add_hardware_line_3d(
                                GeoId::Unknown(
                                    0xE350_0000u32
                                        .wrapping_add(id_salt)
                                        .wrapping_add((selection_index as u32) << 8)
                                        .wrapping_add(edge_index as u32),
                                ),
                                object.transform_point(*a) + view_nudge,
                                object.transform_point(*b) + view_nudge,
                                [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0],
                                21,
                            );
                        }
                    }

                    for (_, vertex_index) in map
                        .selected_geometry_vertices
                        .iter()
                        .filter(|(id, _)| *id == object.id)
                    {
                        let Some(vertex) = object.vertices.get(*vertex_index) else {
                            continue;
                        };
                        push_cube_marker(
                            scene,
                            GeoId::GeometryObject(object.id),
                            true,
                            object.transform_point(*vertex) + view_nudge + cam_forward * -0.012,
                            overlay_world_size(object.transform_point(*vertex)) * 0.88,
                            0.94,
                            39,
                        );
                    }
                }
            }

            if direct_geometry_tool && !map.geometry_objects.is_empty() {
                self.prepare_tool_preview_overlay_3d(project, server_ctx, assets, camera, scene);
                scene.set_overlay();
                return;
            }

            // Helper to draw a single vertex as a camera-facing billboard in the overlay
            let vertex_size_world = 0.24_f32; // slightly larger for visibility
            let push_vertex =
                |id: GeoId, p: Vec3<f32>, selected: bool, scene: &mut rusterix::SceneHandler| {
                    let tile_id = if selected {
                        scene.selected
                    } else {
                        scene.white
                    };
                    scene.overlay_3d.add_billboard_3d(
                        id,
                        tile_id,
                        p,
                        view_right,
                        view_up,
                        vertex_size_world,
                        true,
                    );
                };

            if server_ctx.curr_map_tool_type == MapToolType::Vertex {
                let detail_surface = server_ctx
                    .active_detail_surface
                    .as_ref()
                    .or(server_ctx.hover_surface.as_ref())
                    .or(server_ctx.editing_surface.as_ref())
                    .cloned();
                let detail_vertex_mode = server_ctx.curr_map_tool_type == MapToolType::Vertex
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && detail_surface.is_some();

                if detail_vertex_mode {
                    if let Some(surface) = detail_surface
                        && let Some(profile_id) = surface.profile
                        && let Some(profile_map) = map.profiles.get(&profile_id)
                    {
                        for v in &profile_map.vertices {
                            let mut pos = surface.uv_to_world(Vec2::new(v.x, -v.y)) + view_nudge;
                            pos += Vec3::zero();
                            let selected = profile_map.selected_vertices.contains(&v.id)
                                || server_ctx.hover.0 == Some(v.id);
                            push_vertex(GeoId::Vertex(v.id), pos, selected, scene);
                        }
                    }
                } else {
                    for v in map.vertices.iter() {
                        let Some(world_pos) = map.get_vertex_3d(v.id) else {
                            continue;
                        };
                        let mut pos = Vec3::new(world_pos.x, world_pos.y, world_pos.z);
                        pos += view_nudge;
                        let selected = map.selected_vertices.contains(&v.id)
                            || server_ctx.hover.0 == Some(v.id);

                        push_vertex(GeoId::Vertex(v.id), pos, selected, scene);
                    }
                }
            } else {
                let detail_surface = server_ctx
                    .active_detail_surface
                    .as_ref()
                    .or(server_ctx.hover_surface.as_ref())
                    .or(server_ctx.editing_surface.as_ref())
                    .cloned();
                let skip_world_linedef_overlay = server_ctx.curr_map_tool_type
                    == MapToolType::Linedef
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && detail_surface.is_some();
                // Linedefs
                if server_ctx.curr_map_tool_type == MapToolType::Linedef {
                    if server_ctx.curr_map_tool_type == MapToolType::Linedef
                        && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    {
                        if let Some(surface) = detail_surface.clone() {
                            if let Some(profile_id) = surface.profile
                                && let Some(profile_map) = map.profiles.get(&profile_id)
                            {
                                for linedef in &profile_map.linedefs {
                                    let Some(start_vertex) = profile_map
                                        .vertices
                                        .iter()
                                        .find(|vertex| vertex.id == linedef.start_vertex)
                                    else {
                                        continue;
                                    };
                                    let Some(end_vertex) = profile_map
                                        .vertices
                                        .iter()
                                        .find(|vertex| vertex.id == linedef.end_vertex)
                                    else {
                                        continue;
                                    };

                                    let a = surface
                                        .uv_to_world(Vec2::new(start_vertex.x, -start_vertex.y))
                                        + view_nudge;
                                    let b = surface
                                        .uv_to_world(Vec2::new(end_vertex.x, -end_vertex.y))
                                        + view_nudge;

                                    let is_selected =
                                        profile_map.selected_linedefs.contains(&linedef.id);
                                    let is_hovered = server_ctx.hover.1 == Some(linedef.id);

                                    push_line(
                                        GeoId::Linedef(linedef.id),
                                        scene,
                                        a,
                                        b,
                                        cam_forward,
                                        is_selected,
                                        is_hovered,
                                    );
                                }
                            }
                        }
                    }

                    if !skip_world_linedef_overlay {
                        for linedef in &map.linedefs {
                            let is_selected = map.selected_linedefs.contains(&linedef.id);
                            let is_hovered = server_ctx.hover.1 == Some(linedef.id);
                            if !linedef.sector_ids.is_empty() {
                                continue;
                            }

                            if let (Some(vs), Some(ve)) = (
                                map.get_vertex_3d(linedef.start_vertex),
                                map.get_vertex_3d(linedef.end_vertex),
                            ) {
                                let a = Vec3::new(vs.x, vs.y, vs.z) + view_nudge;
                                let b = Vec3::new(ve.x, ve.y, ve.z) + view_nudge;
                                let normal = cam_forward;

                                push_line(
                                    GeoId::Linedef(linedef.id),
                                    scene,
                                    a,
                                    b,
                                    normal,
                                    is_selected,
                                    is_hovered,
                                );
                            }
                        }
                    }
                }

                // Sectors
                use std::collections::HashMap;
                #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
                struct EdgeKey {
                    v0: u32,
                    v1: u32,
                }
                impl EdgeKey {
                    fn from_vertices(a: u32, b: u32) -> Self {
                        if a < b {
                            EdgeKey { v0: a, v1: b }
                        } else {
                            EdgeKey { v0: b, v1: a }
                        }
                    }
                }
                #[derive(Clone)]
                struct EdgeInfo {
                    a: Vec3<f32>,
                    b: Vec3<f32>,
                    selected: bool,
                    hovered: bool,
                    rep_ld_id: u32, // representative linedef id for picking/hit-testing
                }
                let mut edge_accum: HashMap<EdgeKey, EdgeInfo> = HashMap::new();

                if server_ctx.curr_map_tool_type == MapToolType::Sector
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && let Some(surface) = server_ctx
                        .active_detail_surface
                        .as_ref()
                        .or(server_ctx.hover_surface.as_ref())
                        .or(server_ctx.editing_surface.as_ref())
                        .cloned()
                    && let Some(profile_id) = surface.profile
                    && let Some(profile_map) = map.profiles.get(&profile_id)
                {
                    for sector in &profile_map.sectors {
                        let sector_is_selected = profile_map.selected_sectors.contains(&sector.id);
                        let sector_is_hovered = server_ctx.hover.2 == Some(sector.id);
                        for &ld_id in &sector.linedefs {
                            let Some(linedef) = profile_map.find_linedef(ld_id) else {
                                continue;
                            };
                            let Some(start_vertex) = profile_map.find_vertex(linedef.start_vertex)
                            else {
                                continue;
                            };
                            let Some(end_vertex) = profile_map.find_vertex(linedef.end_vertex)
                            else {
                                continue;
                            };
                            let a = surface.uv_to_world(Vec2::new(start_vertex.x, -start_vertex.y))
                                + view_nudge;
                            let b = surface.uv_to_world(Vec2::new(end_vertex.x, -end_vertex.y))
                                + view_nudge;
                            push_line(
                                GeoId::Sector(sector.id),
                                scene,
                                a,
                                b,
                                cam_forward,
                                sector_is_selected,
                                sector_is_hovered,
                            );
                        }
                    }
                }

                let skip_world_sector_overlay = server_ctx.curr_map_tool_type
                    == MapToolType::Sector
                    && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    && server_ctx.active_detail_surface.is_some();

                for surface in map.surfaces.values() {
                    if skip_world_sector_overlay {
                        continue;
                    }
                    let sector_id = surface.sector_id;
                    let Some(sector) = map.find_sector(sector_id) else {
                        continue;
                    };
                    let sector_is_selected = map.selected_sectors.contains(&sector_id);
                    let sector_is_hovered = server_ctx.hover.2 == Some(sector_id);

                    if sector.properties.contains("rect") && server_ctx.no_rect_geo_on_map {
                        continue;
                    }

                    let nudge = view_nudge; // consistent camera-side nudge avoids opposite-face z-fight

                    if let Some(points3) = sector.vertices_world(map) {
                        let n_pts = points3.len();
                        let n_ld = sector.linedefs.len();
                        let n = n_pts.min(n_ld);
                        if n >= 2 {
                            for i in 0..n {
                                let a = points3[i] + nudge;
                                let b = points3[(i + 1) % n_pts] + nudge;
                                let ld_id = sector.linedefs[i];

                                let mut line_is_selected = false;

                                if server_ctx.curr_map_tool_type == MapToolType::Linedef
                                    || server_ctx.curr_map_tool_type == MapToolType::Selection
                                {
                                    line_is_selected = map.selected_linedefs.contains(&ld_id)
                                        || server_ctx.hover.1 == Some(ld_id);
                                } else if server_ctx.curr_map_tool_type == MapToolType::Sector {
                                    line_is_selected = sector_is_selected || sector_is_hovered;
                                };

                                // Build unordered edge key from linedef vertices, fallback if not found
                                let key = if let Some(ld_ref) = map.find_linedef(ld_id) {
                                    EdgeKey::from_vertices(ld_ref.start_vertex, ld_ref.end_vertex)
                                } else {
                                    // Fallback: build a key from the nearest map vertices to a/b (should be rare)
                                    continue;
                                };

                                edge_accum
                                    .entry(key)
                                    .and_modify(|e| {
                                        e.selected |= line_is_selected;
                                        e.hovered |= server_ctx.hover.1 == Some(ld_id);
                                        e.a = a;
                                        e.b = b; // keep latest endpoints
                                    })
                                    .or_insert(EdgeInfo {
                                        a,
                                        b,
                                        selected: line_is_selected,
                                        hovered: server_ctx.hover.1 == Some(ld_id),
                                        rep_ld_id: ld_id,
                                    });
                            }
                        }
                    }
                }

                // Emit deduplicated edges
                for (_key, e) in edge_accum.into_iter() {
                    push_line(
                        // &mut overlay_batches,
                        // GeometrySource::Linedef(e.rep_ld_id),
                        GeoId::Linedef(e.rep_ld_id),
                        scene,
                        e.a,
                        e.b,
                        cam_forward,
                        e.selected,
                        e.hovered,
                    );
                }
            }

            // Flush final overlay batches: draw normal overlays first, then highlighted front overlays last
            // for batch in overlay_batches.drain(..) {
            //     rusterix.client.scene.d3_overlay.push(batch);
            // }
            // for batch in overlay_batches_front.drain(..) {
            //     rusterix.client.scene.d3_overlay.push(batch);
            // }
        }

        self.prepare_tool_preview_overlay_3d(project, server_ctx, assets, camera, scene);
        scene.set_overlay();
    }

    pub(crate) fn prepare_tool_preview_overlay_3d(
        &mut self,
        project: &Project,
        server_ctx: &ServerContext,
        _assets: &Assets,
        camera: &dyn D3Camera,
        scene: &mut rusterix::SceneHandler,
    ) {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return;
        }

        let map = if server_ctx.pc.is_prefab() {
            project.prefab_editor_map.as_ref()
        } else {
            project.get_region_ctx(server_ctx).map(|region| &region.map)
        };
        let Some(map) = map else { return; };

        if scene.vm.vm_layer_count() < 4 {
            scene.clear_overlay();
        }

        scene.tool_overlay_3d = scenevm::Chunk::default();
        scene.tool_overlay_3d.priority = 1;

        let (cam_forward, cam_right, cam_up) = camera.basis_vectors();
        let view_nudge = cam_forward * -0.002;
        let thickness = 0.15;
        let white = scene.white;

        if server_ctx.curr_map_tool_type == MapToolType::Rect {
            if let Some(terrain_id) = server_ctx.rect_terrain_id {
                let config = TerrainConfig::for_map(map);
                let corners = TerrainGenerator::tile_outline_world(map, terrain_id, &config);
                let n = TerrainGenerator::tile_normal(map, terrain_id, &config);
                for i in 0..4 {
                    scene.tool_overlay_3d.add_line_3d(
                        GeoId::Unknown(i as u32),
                        white,
                        corners[i] + view_nudge,
                        corners[(i + 1) % 4] + view_nudge,
                        thickness,
                        n,
                        100,
                    );
                }
            } else if let Some(sector_id) = server_ctx.rect_sector_id_3d {
                let mut index = 0;
                for surface in map.surfaces.values() {
                    if surface.sector_id == sector_id {
                        let corners =
                            surface.tile_outline_world_local(server_ctx.rect_tile_id_3d, map);
                        let n = surface.plane.normal;
                        for i in 0..4 {
                            scene.tool_overlay_3d.add_line_3d(
                                GeoId::Unknown(index),
                                white,
                                corners[i] + view_nudge,
                                corners[(i + 1) % 4] + view_nudge,
                                thickness,
                                n,
                                100,
                            );
                            index += 1;
                        }
                    }
                }
            } else if let Some((object_id, face_index)) = server_ctx.rect_geometry_face_3d
                && let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == object_id)
                && let Some(face) = object.faces.get(face_index)
                && face.indices.len() >= 3
            {
                let points = face
                    .indices
                    .iter()
                    .filter_map(|vertex_index| object.vertices.get(*vertex_index).copied())
                    .collect::<Vec<_>>();
                if points.len() == face.indices.len() {
                    let first = points[0];
                    let mut normal = Vec3::<f32>::zero();
                    for index in 1..points.len().saturating_sub(1) {
                        normal += (points[index] - first).cross(points[index + 1] - first);
                    }
                    if let Some(local_normal) = normal.try_normalized() {
                        let abs = Vec3::new(
                            local_normal.x.abs(),
                            local_normal.y.abs(),
                            local_normal.z.abs(),
                        );
                        let to_uv = |point: Vec3<f32>| {
                            if abs.y >= abs.x && abs.y >= abs.z {
                                Vec2::new(point.x, point.z)
                            } else if abs.x >= abs.z {
                                Vec2::new(point.z, point.y)
                            } else {
                                Vec2::new(point.x, point.y)
                            }
                        };
                        let from_uv = |uv: Vec2<f32>| {
                            if abs.y >= abs.x && abs.y >= abs.z {
                                Vec3::new(uv.x, first.y, uv.y)
                            } else if abs.x >= abs.z {
                                Vec3::new(first.x, uv.y, uv.x)
                            } else {
                                Vec3::new(uv.x, uv.y, first.z)
                            }
                        };
                        let min_uv = points
                            .iter()
                            .map(|point| to_uv(*point))
                            .fold(Vec2::broadcast(f32::INFINITY), |acc, uv| {
                                Vec2::new(acc.x.min(uv.x), acc.y.min(uv.y))
                            });
                        let max_uv = points
                            .iter()
                            .map(|point| to_uv(*point))
                            .fold(Vec2::broadcast(f32::NEG_INFINITY), |acc, uv| {
                                Vec2::new(acc.x.max(uv.x), acc.y.max(uv.y))
                            });
                        let tile = server_ctx.rect_tile_id_3d;
                        let tile_min = min_uv + Vec2::new(tile.0 as f32, tile.1 as f32);
                        let tile_max = Vec2::new(
                            (tile_min.x + 1.0).min(max_uv.x),
                            (tile_min.y + 1.0).min(max_uv.y),
                        );
                        let local_corners = [
                            from_uv(tile_min),
                            from_uv(Vec2::new(tile_max.x, tile_min.y)),
                            from_uv(tile_max),
                            from_uv(Vec2::new(tile_min.x, tile_max.y)),
                        ];
                        let world_corners =
                            local_corners.map(|point| object.transform_point(point) + view_nudge);
                        let world_normal = {
                            let origin = object.transform_point(Vec3::zero());
                            (object.transform_point(local_normal) - origin)
                                .try_normalized()
                                .unwrap_or(local_normal)
                        };
                        for i in 0..4 {
                            scene.tool_overlay_3d.add_line_3d(
                                GeoId::Unknown(0xE3D0_0000u32.wrapping_add(i as u32)),
                                white,
                                world_corners[i],
                                world_corners[(i + 1) % 4],
                                thickness,
                                world_normal,
                                100,
                            );
                        }
                    }
                }
            }
        } else if matches!(server_ctx.curr_map_tool_type, MapToolType::Linedef) {
            if let Some(start) = map.curr_grid_pos_3d {
                let target = server_ctx.hover_cursor_3d.or_else(|| {
                    matches!(server_ctx.geo_hit, Some(GeoId::GeometryObject(_)))
                        .then_some(server_ctx.geo_hit_pos)
                });
                if let Some(target) = target {
                    let color = [187.0 / 255.0, 122.0 / 255.0, 208.0 / 255.0, 1.0];
                    scene.tool_overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE3C0_0000),
                        start + view_nudge + cam_forward * -0.012,
                        target + view_nudge + cam_forward * -0.012,
                        color,
                        100,
                    );
                }
            }
        } else if server_ctx.curr_map_tool_type == MapToolType::CollisionProbe {
            let green = [74.0 / 255.0, 222.0 / 255.0, 128.0 / 255.0, 1.0];
            let yellow = [250.0 / 255.0, 204.0 / 255.0, 21.0 / 255.0, 1.0];
            let orange = [251.0 / 255.0, 146.0 / 255.0, 60.0 / 255.0, 1.0];
            let red = [248.0 / 255.0, 113.0 / 255.0, 113.0 / 255.0, 1.0];
            let magenta = [244.0 / 255.0, 114.0 / 255.0, 255.0 / 255.0, 1.0];
            let mut index = 0u32;
            macro_rules! push_hardware {
                ($scene:expr, $a:expr, $b:expr, $color:expr, $priority:expr $(,)?) => {{
                    $scene.tool_overlay_3d.add_hardware_line_3d(
                        GeoId::Unknown(0xE3F0_0000u32.wrapping_add(index)),
                        $a + view_nudge + cam_forward * -0.012,
                        $b + view_nudge + cam_forward * -0.012,
                        $color,
                        $priority,
                    );
                    index = index.wrapping_add(1);
                }};
            }
            macro_rules! push_contrast {
                ($scene:expr, $a:expr, $b:expr, $color:expr, $priority:expr $(,)?) => {{
                    let shadow = [0.02, 0.025, 0.035, $color[3].min(0.75)];
                    let right = cam_right * 0.018;
                    let up = cam_up * 0.018;
                    let offsets = [-right, right, -up, up, Vec3::zero()];
                    for offset in offsets {
                        push_hardware!($scene, $a + offset, $b + offset, shadow, $priority - 1);
                    }
                    push_hardware!($scene, $a, $b, $color, $priority);
                    push_hardware!(
                        $scene,
                        $a + right * 0.5,
                        $b + right * 0.5,
                        $color,
                        $priority
                    );
                    push_hardware!($scene, $a + up * 0.5, $b + up * 0.5, $color, $priority);
                }};
            }

            let mut draw_result = |scene: &mut rusterix::SceneHandler,
                                   result: &rusterix::CollisionProbeResult,
                                   faded: bool| {
                let fade = |mut color: [f32; 4]| {
                    if faded {
                        color[3] *= 0.45;
                    }
                    color
                };
                let green = fade(green);
                let yellow = fade(yellow);
                let orange = fade(orange);
                let red = fade(red);
                let magenta = fade(magenta);

                let goto_path = if result.goto_path.len() >= 2 {
                    result.goto_path.clone()
                } else if result.goto_path_found || result.arrived {
                    let fallback_end =
                        result.steps.last().map(|step| step.to).unwrap_or_else(|| {
                            Vec3::new(result.target.x, result.start.y, result.target.y)
                        });
                    vec![result.start, fallback_end]
                } else {
                    Vec::new()
                };
                for segment in goto_path.windows(2) {
                    push_contrast!(
                        scene,
                        segment[0] + Vec3::new(0.0, 0.18, 0.0),
                        segment[1] + Vec3::new(0.0, 0.18, 0.0),
                        magenta,
                        104,
                    );
                }

                for step in &result.steps {
                    let color = match step.kind {
                        rusterix::CollisionProbeStepKind::Walk => green,
                        rusterix::CollisionProbeStepKind::StepUp
                        | rusterix::CollisionProbeStepKind::StepDown => yellow,
                        rusterix::CollisionProbeStepKind::Contact => orange,
                        rusterix::CollisionProbeStepKind::Blocked
                        | rusterix::CollisionProbeStepKind::NoFloor => red,
                    };
                    push_contrast!(scene, step.from, step.to, color, 100);

                    if let Some(blocker) = step.blocker {
                        let y = step.from.y + 0.08;
                        let blocker_color =
                            if step.kind == rusterix::CollisionProbeStepKind::Contact {
                                orange
                            } else {
                                red
                            };
                        push_contrast!(
                            scene,
                            Vec3::new(blocker.start.x, y, blocker.start.y),
                            Vec3::new(blocker.end.x, y, blocker.end.y),
                            blocker_color,
                            101,
                        );
                    }
                }

                let marker = 0.1;
                let start_color = green;
                let target_color = if result.arrived { green } else { red };
                let start = result.start + Vec3::new(0.0, 0.06, 0.0);
                let target_y = result
                    .steps
                    .last()
                    .map(|step| step.to.y)
                    .unwrap_or(result.start.y);
                let target = Vec3::new(result.target.x, target_y + 0.06, result.target.y);
                push_hardware!(
                    scene,
                    start + Vec3::new(-marker, 0.0, 0.0),
                    start + Vec3::new(marker, 0.0, 0.0),
                    start_color,
                    102,
                );
                push_hardware!(
                    scene,
                    start + Vec3::new(0.0, 0.0, -marker),
                    start + Vec3::new(0.0, 0.0, marker),
                    start_color,
                    102,
                );
                push_hardware!(
                    scene,
                    target + Vec3::new(-marker, 0.0, 0.0),
                    target + Vec3::new(marker, 0.0, 0.0),
                    target_color,
                    102,
                );
                push_hardware!(
                    scene,
                    target + Vec3::new(0.0, 0.0, -marker),
                    target + Vec3::new(0.0, 0.0, marker),
                    target_color,
                    102,
                );
            };

            if let Some(result) = server_ctx
                .collision_probe_result
                .as_ref()
                .or_else(|| server_ctx.collision_probe_results.last())
            {
                draw_result(scene, result, false);
            }
        }

        let id = scene.tool_overlay_3d_id;
        let chunk = scene.tool_overlay_3d.clone();
        scene.vm.set_active_vm(3);
        scene.vm.execute(scenevm::Atom::AddChunk { id, chunk });
        scene.vm.set_active_vm(0);
        // The editor tool overlay is isolated on VM layer 3. Updating it does
        // not change base entities, items, lights, billboards, or particles.
        // Invalidating dynamics here forced the 2,257-object Stonefall scene
        // to rebuild on every Vertex-tool redraw after leaving Game mode.
    }
}
