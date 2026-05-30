use crate::editor::{EDITCAMERA, RUSTERIX, SIDEBARMODE, UNDOMANAGER};
use crate::prelude::*;
use rusterix::Value;
use vek::{Vec2, Vec3};

fn geometry_selection_status_text(map: &Map, server_ctx: &ServerContext) -> Option<String> {
    if server_ctx.editor_view_mode == EditorViewMode::D2 {
        return None;
    }

    if !map.selected_geometry_surface_points.is_empty()
        || !map.selected_geometry_surface_segments.is_empty()
    {
        if selected_surface_detail_is_closed_loop(map) {
            return Some(fl!("status_geometry_surface_loop_selection"));
        }
        return Some(fl!("status_geometry_surface_selection"));
    }

    if !map.selected_geometry_faces.is_empty() {
        return Some(fl!("status_geometry_face_selection"));
    }

    if !map.selected_geometry_vertices.is_empty() {
        if map.geometry_selection_mode == 3 {
            return Some(fl!("status_geometry_edge_selection"));
        }
        return Some(fl!("status_geometry_vertex_selection"));
    }

    if !map.selected_geometry_objects.is_empty() {
        return Some(fl!("status_geometry_object_selection"));
    }

    match server_ctx.curr_map_tool_type {
        MapToolType::Selection
        | MapToolType::Sector
        | MapToolType::Vertex
        | MapToolType::Linedef => Some(fl!("status_geometry_empty_selection")),
        _ => None,
    }
}

fn selected_surface_detail_is_closed_loop(map: &Map) -> bool {
    if map.selected_geometry_surface_segments.len() < 3 {
        return false;
    }

    let (object_id, face_index, _) = match map.selected_geometry_surface_segments.first() {
        Some(selection) => *selection,
        None => return false,
    };
    if !map
        .selected_geometry_surface_segments
        .iter()
        .all(|(id, face, _)| *id == object_id && *face == face_index)
    {
        return false;
    }

    let Some(object) = map
        .geometry_objects
        .iter()
        .find(|object| object.id == object_id)
    else {
        return false;
    };
    let Some(face) = object.faces.get(face_index) else {
        return false;
    };

    let mut degrees: FxHashMap<usize, usize> = FxHashMap::default();
    for (_, _, segment_index) in &map.selected_geometry_surface_segments {
        let Some(segment) = face.surface_segments.get(*segment_index) else {
            return false;
        };
        *degrees.entry(segment.start).or_insert(0) += 1;
        *degrees.entry(segment.end).or_insert(0) += 1;
    }

    degrees.len() >= 3 && degrees.values().all(|degree| *degree == 2)
}

pub struct MapEditor {
    curr_tile_uuid: Option<Uuid>,

    icon_normal_border_color: RGBA,
    icon_selected_border_color: RGBA,
}

#[allow(clippy::new_without_default)]
impl MapEditor {
    fn geometry_face_auto_uvs(points: &[Vec3<f32>]) -> Vec<Vec2<f32>> {
        if points.len() < 3 {
            return vec![Vec2::zero(); points.len()];
        }

        let mut normal = Vec3::<f32>::zero();
        for index in 1..points.len() - 1 {
            normal += (points[index] - points[0]).cross(points[index + 1] - points[0]);
        }

        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        points
            .iter()
            .map(|point| {
                if abs.y >= abs.x && abs.y >= abs.z {
                    Vec2::new(point.x, point.z)
                } else if abs.x >= abs.z {
                    Vec2::new(point.z, point.y)
                } else {
                    Vec2::new(point.x, point.y)
                }
            })
            .collect()
    }

    fn geometry_face_texture_shortcut_signs(
        object: &rusterix::GeometryObject,
        face: &rusterix::GeometryFace,
    ) -> (Vec2<f32>, f32) {
        let points = face
            .indices
            .iter()
            .filter_map(|vertex_index| object.vertices.get(*vertex_index).copied())
            .collect::<Vec<_>>();
        if points.len() != face.indices.len() || points.len() < 3 {
            return (Vec2::broadcast(1.0), 1.0);
        }

        let uvs = if face.auto_uv || face.uvs.len() != face.indices.len() {
            Self::geometry_face_auto_uvs(&points)
        } else {
            face.uvs.clone()
        };

        let mut normal = Vec3::<f32>::zero();
        for index in 1..points.len() - 1 {
            normal += (points[index] - points[0]).cross(points[index + 1] - points[0]);
        }
        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        let (canonical_u, canonical_v) = if abs.y >= abs.x && abs.y >= abs.z {
            (Vec3::unit_x(), Vec3::unit_z())
        } else if abs.x >= abs.z {
            (Vec3::unit_z(), Vec3::unit_y())
        } else {
            (Vec3::unit_x(), Vec3::unit_y())
        };

        let mut best_u = (0.0f32, Vec3::zero());
        let mut best_v = (0.0f32, Vec3::zero());
        for index in 0..points.len() {
            let next = (index + 1) % points.len();
            let delta_world = points[next] - points[index];
            let delta_uv = uvs[next] - uvs[index];
            let abs_u = delta_uv.x.abs();
            let abs_v = delta_uv.y.abs();
            if abs_u > best_u.0 && abs_u > 1e-5 {
                best_u = (abs_u, delta_world * delta_uv.x.signum());
            }
            if abs_v > best_v.0 && abs_v > 1e-5 {
                best_v = (abs_v, delta_world * delta_uv.y.signum());
            }
        }

        let u_sign = if best_u.1.dot(canonical_u) < 0.0 {
            -1.0
        } else {
            1.0
        };
        let v_sign = if best_v.1.dot(canonical_v) < 0.0 {
            -1.0
        } else {
            1.0
        };
        (Vec2::new(u_sign, v_sign), u_sign * v_sign)
    }

    fn selected_textured_geometry_faces(map: &Map) -> Vec<(usize, usize)> {
        map.selected_geometry_faces
            .iter()
            .filter_map(|(object_id, face_index)| {
                let object_index = map
                    .geometry_objects
                    .iter()
                    .position(|object| object.id == *object_id)?;
                let face = map.geometry_objects[object_index].faces.get(*face_index)?;
                if face.tile.is_some() || !face.tiles.is_empty() {
                    Some((object_index, *face_index))
                } else {
                    None
                }
            })
            .collect()
    }

    fn nudge_selected_face_texture(
        map: &mut Map,
        key: TheKeyCode,
        shift: bool,
        scale: bool,
    ) -> bool {
        let selected_faces = Self::selected_textured_geometry_faces(map);
        if selected_faces.is_empty() {
            return false;
        }

        let mut changed = false;
        for (object_index, face_index) in selected_faces {
            let Some((offset_sign, rotation_sign)) =
                map.geometry_objects.get(object_index).and_then(|object| {
                    let face = object.faces.get(face_index)?;
                    Some(Self::geometry_face_texture_shortcut_signs(object, face))
                })
            else {
                continue;
            };
            let Some(face) = map
                .geometry_objects
                .get_mut(object_index)
                .and_then(|object| object.faces.get_mut(face_index))
            else {
                continue;
            };

            if scale {
                let step = 0.1;
                match key {
                    TheKeyCode::Left => {
                        face.texture_scale.x = (face.texture_scale.x - step).max(0.05)
                    }
                    TheKeyCode::Right => face.texture_scale.x += step,
                    TheKeyCode::Down => {
                        face.texture_scale.y = (face.texture_scale.y - step).max(0.05)
                    }
                    TheKeyCode::Up => face.texture_scale.y += step,
                    _ => continue,
                }
                changed = true;
            } else if shift {
                let step = 5.0;
                match key {
                    TheKeyCode::Left => face.texture_rotation -= step * rotation_sign,
                    TheKeyCode::Right => face.texture_rotation += step * rotation_sign,
                    _ => continue,
                }
                changed = true;
            } else {
                let step = 0.1;
                match key {
                    TheKeyCode::Left => face.texture_offset.x -= step * offset_sign.x,
                    TheKeyCode::Right => face.texture_offset.x += step * offset_sign.x,
                    TheKeyCode::Down => face.texture_offset.y -= step * offset_sign.y,
                    TheKeyCode::Up => face.texture_offset.y += step * offset_sign.y,
                    _ => continue,
                }
                changed = true;
            }
        }

        if changed {
            map.update_surfaces();
            map.changed += 1;
        }
        changed
    }

    fn handle_face_texture_arrow_shortcut(
        &mut self,
        key: TheKeyCode,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2
            || server_ctx.get_map_context() != MapContext::Region
        {
            return false;
        }

        let Some(map) = project.get_map_mut(server_ctx) else {
            return false;
        };
        let prev = map.clone();
        if !Self::nudge_selected_face_texture(map, key, ui.shift, ui.ctrl || ui.logo) {
            return false;
        }

        let undo_atom =
            ProjectUndoAtom::MapEdit(server_ctx.pc, Box::new(prev), Box::new(map.clone()));
        crate::utils::editor_scene_apply_map_edit_atom(project, server_ctx, &undo_atom);
        UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
        RUSTERIX.write().unwrap().set_overlay_dirty();
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action Parameters"),
            TheValue::Empty,
        ));
        ctx.ui.redraw_all = true;
        true
    }

    pub fn new() -> Self {
        Self {
            curr_tile_uuid: None,

            icon_normal_border_color: [100, 100, 100, 255],
            icon_selected_border_color: [255, 255, 255, 255],
        }
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
    ) -> TheCanvas {
        let mut center = TheCanvas::new();

        // let mut shared_layout = TheSharedHLayout::new(TheId::named("Editor Shared"));

        // let mut region_editor = TheRGBALayout::new(TheId::named("Region Editor"));
        // if let Some(rgba_view) = region_editor.rgba_view_mut().as_rgba_view() {
        //     rgba_view.set_mode(TheRGBAViewMode::Display);

        //     if let Some(buffer) = ctx.ui.icon("eldiron_map") {
        //         rgba_view.set_buffer(buffer.clone());
        //     }

        //     rgba_view.set_grid_color([255, 255, 255, 5]);
        //     rgba_view.set_hover_color(Some([255, 255, 255, 100]));
        //     rgba_view.set_wheel_scale(-0.2);
        // }

        // let mut region_editor_canvas = TheCanvas::new();
        // region_editor_canvas.set_layout(region_editor);
        // shared_layout.add_canvas(region_editor_canvas);

        // let mut render_canvas: TheCanvas = TheCanvas::new();
        // let render_view = TheRenderView::new(TheId::named("RenderView"));
        // render_canvas.set_widget(render_view);
        // shared_layout.add_canvas(render_canvas);

        //center.set_layout(shared_layout);

        let mut output_stack = TheStackLayout::new(TheId::named("Game Output Stack"));

        let mut poly_canvas: TheCanvas = TheCanvas::new();
        let mut render_view = TheRenderView::new(TheId::named("PolyView"));
        render_view.set_auto_focus(true);
        poly_canvas.set_widget(render_view);
        output_stack.add_canvas(poly_canvas);

        let text_canvas = crate::textplay::TextGameState::setup_canvas();
        output_stack.add_canvas(text_canvas);

        let mut output_canvas = TheCanvas::new();
        output_canvas.set_layout(output_stack);
        center.set_center(output_canvas);

        // Picker

        let mut tile_picker = TheCanvas::new();
        let mut vlayout = TheVLayout::new(TheId::named("Editor Icon Layout"));
        vlayout.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        vlayout.limiter_mut().set_max_width(90);
        vlayout.set_margin(Vec4::new(0, 10, 0, 5));

        let mut icon_preview = TheIconView::new(TheId::named("Icon Preview"));
        icon_preview.set_alpha_mode(false);
        icon_preview.limiter_mut().set_max_size(Vec2::new(65, 65));
        icon_preview.set_border_color(Some([100, 100, 100, 255]));
        vlayout.add_widget(Box::new(icon_preview));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(2);
        vlayout.add_widget(Box::new(spacer));

        let mut view_mode_gb = TheGroupButton::new(TheId::named("Map Editor Camera"));
        view_mode_gb.add_text_status_icon(
            "".to_string(),
            "2D Camera".to_string(),
            "square".to_string(),
        );
        view_mode_gb.add_text_status_icon(
            "".to_string(),
            "3D Camera: Iso".to_string(),
            "cube".to_string(),
        );
        view_mode_gb.add_text_status_icon(
            "".to_string(),
            "3D Camera: First person".to_string(),
            "camera".to_string(),
        );
        view_mode_gb.set_item_width(26);
        vlayout.add_widget(Box::new(view_mode_gb));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(0);
        vlayout.add_widget(Box::new(spacer));

        let mut grid_sub_div = TheTextLineEdit::new(TheId::named("Grid Subdiv"));
        grid_sub_div.set_value(TheValue::Float(1.0));
        // opacity.set_default_value(TheValue::Float(1.0));
        grid_sub_div.set_info_text(Some("Subdiv".to_string()));
        grid_sub_div.set_range(TheValue::RangeI32(1..=32));
        grid_sub_div.set_continuous(true);
        grid_sub_div.limiter_mut().set_max_width(150);
        grid_sub_div.set_status_text(&fl!("status_map_editor_grid_sub_div"));
        grid_sub_div.limiter_mut().set_max_width(75);
        vlayout.add_widget(Box::new(grid_sub_div));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(2);
        vlayout.add_widget(Box::new(spacer));

        let mut ground_icon = TheIconView::new(TheId::named("Ground Icon"));
        ground_icon.set_text(Some("FLOOR".to_string()));
        ground_icon.set_text_size(10.0);
        ground_icon.set_text_color([200, 200, 200, 255]);
        ground_icon.limiter_mut().set_max_size(Vec2::new(48, 48));
        ground_icon.set_border_color(Some(self.icon_selected_border_color));

        let mut wall_icon = TheIconView::new(TheId::named("Wall Icon"));
        wall_icon.set_text(Some("WALL".to_string()));
        wall_icon.set_text_size(10.0);
        wall_icon.set_text_color([200, 200, 200, 255]);
        wall_icon.limiter_mut().set_max_size(Vec2::new(48, 48));
        wall_icon.set_border_color(Some(self.icon_normal_border_color));

        let mut ceiling_icon = TheIconView::new(TheId::named("Ceiling Icon"));
        ceiling_icon.set_text(Some("CEILING".to_string()));
        ceiling_icon.set_text_size(10.0);
        ceiling_icon.set_text_color([200, 200, 200, 255]);
        ceiling_icon.limiter_mut().set_max_size(Vec2::new(48, 48));
        ceiling_icon.set_border_color(Some(self.icon_normal_border_color));

        // let mut cc_icon = TheIconView::new(TheId::named("Tile FX Icon"));
        // cc_icon.set_text(Some("FX".to_string()));
        // cc_icon.set_text_size(10.0);
        // cc_icon.set_text_color([200, 200, 200, 255]);
        // cc_icon.limiter_mut().set_max_size(vec2i(48, 48));
        // cc_icon.set_border_color(Some(self.icon_normal_border_color));

        vlayout.add_widget(Box::new(ground_icon));
        vlayout.add_widget(Box::new(wall_icon));
        vlayout.add_widget(Box::new(ceiling_icon));
        //vlayout.add_widget(Box::new(cc_icon));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(2);
        vlayout.add_widget(Box::new(spacer));

        let mut text = TheText::new(TheId::named("Cursor Position"));
        text.set_text("()".to_string());
        text.set_text_color([200, 200, 200, 255]);
        vlayout.add_widget(Box::new(text));

        // let mut text = TheText::new(TheId::named("Cursor Height"));
        // text.set_text("H: -".to_string());
        // text.set_text_color([200, 200, 200, 255]);
        // vlayout.add_widget(Box::new(text));

        tile_picker.set_layout(vlayout);
        //center.set_left(tile_picker);

        // Tool Params
        // let mut toolbar_hlayout = TheHLayout::new(TheId::named("Game Tool Params"));
        // toolbar_hlayout.set_background_color(None);
        // toolbar_hlayout.set_margin(Vec4::new(10, 2, 5, 2));

        // let mut toolbar_canvas = TheCanvas::default();
        // toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        // toolbar_canvas.set_layout(toolbar_hlayout);

        center.bottom_is_expanding = true;
        // center.set_bottom(toolbar_canvas);

        center
    }

    pub fn load_from_project(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext, project: &Project) {
        RUSTERIX
            .write()
            .unwrap()
            .set_tiles(project.tiles.clone(), true);
    }

    #[allow(clippy::suspicious_else_formatting)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::KeyCodeDown(TheValue::KeyCode(key)) => {
                if !ui.focus_widget_supports_text_input(ctx) {
                    match key {
                        TheKeyCode::Up
                        | TheKeyCode::Down
                        | TheKeyCode::Left
                        | TheKeyCode::Right => {
                            redraw |= self.handle_face_texture_arrow_shortcut(
                                key.clone(),
                                ui,
                                ctx,
                                project,
                                server_ctx,
                            );
                        }
                        _ => {}
                    }
                }
            }
            TheEvent::KeyCodeUp(TheValue::KeyCode(_)) => {}

            TheEvent::Copy => {
                if !server_ctx.polyview_has_focus(ctx) {
                    return false;
                }
                if let Some(map) = project.get_map_mut(server_ctx) {
                    if map.has_selection() {
                        server_ctx.clipboard = map.copy_selected(false);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Geometry copied into clipboard.".to_string(),
                        ));
                    } else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "No geometry selected!".to_string(),
                        ));
                    }
                }
            }
            TheEvent::Cut => {
                if !server_ctx.polyview_has_focus(ctx) {
                    return false;
                }
                if let Some(map) = project.get_map_mut(server_ctx) {
                    if map.has_selection() {
                        let _prev = map.clone();
                        server_ctx.clipboard = map.copy_selected(true);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Geometry copied into clipboard.".to_string(),
                        ));
                    } else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "No geometry selected!".to_string(),
                        ));
                    }
                }
            }
            TheEvent::Paste(_, _) => {
                // TODO use focus_widget_supports_clipboard here
                if !server_ctx.clipboard.is_empty() && server_ctx.polyview_has_focus(ctx) {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Geometry pasted. Click to insert, Escape to cancel.".to_string(),
                    ));
                    server_ctx.paste_clipboard = Some(server_ctx.clipboard.clone());
                }
            }
            TheEvent::Custom(id, value) => {
                if id.name == "Base State Selected" {
                    if let Some(layout) = ui.get_text_layout("Node Settings") {
                        layout.clear();
                    }
                } else if id.name == "SoftRig Selected" {
                    if let TheValue::Id(id) = value {
                        let mut nodeui: TheNodeUI = TheNodeUI::default();

                        if let Some(map) = project.get_map(server_ctx) {
                            let name = if let Some(softrig) = map.softrigs.get(id) {
                                softrig.name.clone()
                            } else {
                                "???".to_string()
                            };

                            let item = TheNodeUIItem::Text(
                                "softRigName".into(),
                                "Rig Name".into(),
                                "Set the name of the soft rig keyframe.".into(),
                                name,
                                None,
                                false,
                            );
                            nodeui.add_item(item);
                        }

                        if let Some(layout) = ui.get_text_layout("Node Settings") {
                            nodeui.apply_to_text_layout(layout);
                            ctx.ui.relayout = true;

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Show Node Settings"),
                                TheValue::Text("Soft Rig Settings".to_string()),
                            ));
                        }
                    }
                } else if id.name == "Map Selection Changed" {
                    set_code(ui, ctx, project, server_ctx);
                    self.apply_map_settings(ui, ctx, project, server_ctx);

                    if let Some(map) = project.get_map(server_ctx)
                        && let Some(status) = geometry_selection_status_text(map, server_ctx)
                    {
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    }

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Action List"),
                        TheValue::Empty,
                    ));

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Action Parameters"),
                        TheValue::Empty,
                    ));
                }
                // else if id.name == "Cursor Pos Changed" {
                //     if let Some(text) = ui.get_text("Cursor Position") {
                //         if let Some(v) = value.to_vec2f() {
                //             text.set_text(format!("{}, {}", v.x, v.y));
                //         }
                //         redraw = true;
                //     }

                //     if let Some(layout) = ui.get_layout("Editor Icon Layout") {
                //         layout.relayout(ctx);
                //     }
                // }
                //crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == "PolyView" {
                    let is_running = crate::editor::RUSTERIX.read().unwrap().server.state
                        == rusterix::ServerState::Running;
                    if is_running && server_ctx.game_mode {
                        let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                        if rusterix.client.scroll_messages(coord.y as isize) {
                            return true;
                        }
                        rusterix.client.target_offset -= *coord;
                    } else if !server_ctx.world_mode {
                        if server_ctx.editor_view_mode == EditorViewMode::D2
                            && let Some(map) = project.get_map_mut(server_ctx)
                        {
                            if ui.ctrl || ui.logo {
                                let old_grid_size = map.grid_size;

                                map.grid_size += coord.y as f32;
                                map.grid_size = map.grid_size.clamp(5.0, 100.0);

                                let scale = map.grid_size / old_grid_size;
                                map.offset *= scale;
                            } else {
                                map.offset += Vec2::new(-coord.x as f32, coord.y as f32);
                            }
                            map.curr_rectangle = None;
                        }

                        if server_ctx.get_map_context() == MapContext::Region {
                            if server_ctx.editor_view_mode == EditorViewMode::D2
                                && server_ctx.profile_view.is_some()
                            {
                            } else {
                                if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                    if ui.shift {
                                        if let Some(region) =
                                            project.get_region_mut(&server_ctx.curr_region)
                                        {
                                            if let Some(render_view) =
                                                ui.get_render_view("PolyView")
                                            {
                                                let dim = *render_view.dim();
                                                EDITCAMERA.read().unwrap().pan_3d_by_delta(
                                                    region,
                                                    server_ctx,
                                                    *coord,
                                                    Vec2::new(dim.x, dim.y),
                                                );
                                            }
                                        }
                                    } else if ui.alt {
                                        EDITCAMERA
                                            .write()
                                            .unwrap()
                                            .rotate(coord.map(|v| v as f32), server_ctx);
                                    } else {
                                        EDITCAMERA
                                            .write()
                                            .unwrap()
                                            .scroll_by(coord.y as f32, server_ctx);
                                    }
                                    redraw = true;
                                }
                            }
                            if server_ctx.editor_view_mode != EditorViewMode::D2 {
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Soft Update Minimap"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Map Editor Camera" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if *index == 0 {
                            region.map.camera = MapCamera::TwoD;
                        } else if *index == 1 {
                            region.map.camera = MapCamera::ThreeDIso;
                        } else if *index == 2 {
                            region.map.camera = MapCamera::ThreeDFirstPerson;
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Grid Subdiv" {
                    if let Some(value) = value.to_i32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.map.subdivisions =
                                (1.0 / ServerContext::edit_grid_step(value as f32)).round();
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "linedefAddMidpoint" && *state == TheWidgetState::Clicked {
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        let prev = map.clone();
                        let mut changed = false;
                        for linedef_id in &map.selected_linedefs.clone() {
                            map.add_midpoint(*linedef_id);
                            changed = true;
                        }
                        if changed {
                            self.add_map_undo(map, prev, ctx, server_ctx);
                        }
                    }
                }
                /*
                if id.name == "linedefDeleteWall" && *state == TheWidgetState::Clicked {
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        let prev = map.clone();
                        let mut changed = false;
                        for linedef in &map.selected_linedefs.clone() {
                            if let Some(linedef) = map.find_linedef_mut(*linedef) {
                                if !linedef.profile.is_empty() {
                                    linedef.profile = Map::default();
                                    changed = true;
                                }
                            }
                        }
                        if changed {
                            self.add_map_undo(map, prev, ctx, server_ctx);
                        }
                    }
                    crate::utils::scenemanager_render_map(project, server_ctx);
                }*/
                // Region Content List Selection
                if id.name == "Screen Content List Item" {
                    if let Some(screen) = project.screens.get_mut(&id.references) {
                        for sector in screen.map.sectors.iter_mut() {
                            if sector.creator_id == id.uuid {
                                screen.map.selected_sectors = vec![sector.id];
                                RUSTERIX.write().unwrap().set_dirty();
                                server_ctx.cc = ContentContext::Sector(id.uuid);
                                // if !sector.properties.contains("source") {
                                //     // Create default code item
                                //     if let Some(bytes) = crate::Embedded::get("python/widget.py") {
                                //         if let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
                                //         {
                                //             sector
                                //                 .properties
                                //                 .set("source", Value::Str(source.into()));
                                //         }
                                //     }
                                // }
                                if !sector.properties.contains("data") {
                                    // Create default data item
                                    if let Some(bytes) = crate::Embedded::get("toml/widget.toml") {
                                        if let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
                                        {
                                            sector
                                                .properties
                                                .set("data", Value::Str(source.into()));
                                        }
                                    }
                                }
                            }
                        }

                        let mut center = None;
                        for sector in screen.map.sectors.iter() {
                            if sector.creator_id == id.uuid {
                                center = sector.center(&screen.map.clone());
                                break;
                            }
                        }

                        if let Some(center) = center {
                            if let Some(render_view) = ui.get_render_view("PolyView") {
                                let dim = *render_view.dim();
                                server_ctx.center_map_at_grid_pos(
                                    Vec2::new(dim.width as f32, dim.height as f32),
                                    Vec2::new(center.x, center.y),
                                    &mut screen.map,
                                );
                            }
                        }
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ScreenWidget(id.references, id.uuid),
                        );
                    }
                } else
                // Region Content List Selection
                if id.name == "Region Content List Item" {
                    // If this is a character instance, update its name from the template

                    let mut temp_id = None;
                    let mut temp_name = "".to_string();
                    if let Some(region) = project.get_region(&server_ctx.curr_region) {
                        if let Some(character) = region.characters.get(&id.uuid) {
                            temp_id = Some(character.character_id);
                        }
                    }

                    if let Some(temp_id) = temp_id {
                        if let Some(char_temp) = project.characters.get(&temp_id) {
                            temp_name = char_temp.name.clone();
                        }
                    }

                    if !temp_name.is_empty() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(character) = region.characters.get_mut(&id.uuid) {
                                character.name = temp_name.clone();
                            }
                        }
                    }

                    // ---

                    let mut character_template_id: Option<Uuid> = None;
                    let mut found = false;
                    let mut is_character_instance = false;
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.map.clear_selection();

                        if let Some(character) = region.characters.get(&id.uuid) {
                            found = true;
                            is_character_instance = true;

                            if *SIDEBARMODE.write().unwrap() == SidebarMode::Region {
                                /*
                                ui.set_widget_value(
                                    "CodeEdit",
                                    ctx,
                                    TheValue::Text(character.source.clone()),
                                );*/
                            } else if *SIDEBARMODE.write().unwrap() == SidebarMode::Character {
                                character_template_id = Some(character.character_id);
                            }
                            region.map.selected_entity_item = Some(id.uuid);
                            server_ctx.curr_region_content =
                                ContentContext::CharacterInstance(id.uuid);
                            server_ctx.cc = ContentContext::CharacterInstance(id.uuid);
                            if let Some(render_view) = ui.get_render_view("PolyView") {
                                let dim = *render_view.dim();
                                let mut position =
                                    Vec2::new(character.position.x, character.position.z);

                                // If server is running, get the instance position
                                for entity in region.map.entities.iter() {
                                    if entity.creator_id == character.id {
                                        position = entity.get_pos_xz();

                                        break;
                                    }
                                }

                                if !server_ctx.content_click_from_map {
                                    server_ctx.center_map_at_grid_pos(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        position,
                                        &mut region.map,
                                    );
                                }
                            }
                        } else if let Some(item) = region.items.get(&id.uuid) {
                            found = true;
                            // if *SIDEBARMODE.write().unwrap() == SidebarMode::Region {
                            //     ui.set_widget_value(
                            //         "CodeEdit",
                            //         ctx,
                            //         TheValue::Text(item.source.clone()),
                            //     );
                            // }

                            region.map.selected_entity_item = Some(id.uuid);
                            server_ctx.curr_region_content = ContentContext::ItemInstance(id.uuid);
                            server_ctx.cc = ContentContext::ItemInstance(id.uuid);
                            if let Some(render_view) = ui.get_render_view("PolyView") {
                                let dim = *render_view.dim();

                                if !server_ctx.content_click_from_map {
                                    server_ctx.center_map_at_grid_pos(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        Vec2::new(item.position.x, item.position.z),
                                        &mut region.map,
                                    );
                                }
                            }
                        }

                        if !found {
                            // Test sectors
                            for sector in &region.map.sectors.clone() {
                                if sector.creator_id == id.uuid {
                                    ui.set_widget_value(
                                        "CodeEdit",
                                        ctx,
                                        TheValue::Text(String::new()),
                                    );

                                    server_ctx.curr_region_content =
                                        ContentContext::Sector(id.uuid);
                                    server_ctx.cc = ContentContext::Sector(id.uuid);
                                    if let Some(center) = sector.center(&region.map) {
                                        if let Some(render_view) = ui.get_render_view("PolyView") {
                                            let dim = *render_view.dim();

                                            server_ctx.center_map_at_grid_pos(
                                                Vec2::new(dim.width as f32, dim.height as f32),
                                                center,
                                                &mut region.map,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        server_ctx.content_click_from_map = false;
                        RUSTERIX.write().unwrap().set_dirty();
                    }

                    if found {
                        if is_character_instance {
                            set_project_context(
                                ctx,
                                ui,
                                project,
                                server_ctx,
                                ProjectContext::RegionCharacterInstance(
                                    server_ctx.curr_region,
                                    id.uuid,
                                ),
                            );
                        } else {
                            set_project_context(
                                ctx,
                                ui,
                                project,
                                server_ctx,
                                ProjectContext::RegionItemInstance(server_ctx.curr_region, id.uuid),
                            );
                        }
                    }

                    // If in character sidebar mode, set the code aand data
                    if let Some(character_template_id) = character_template_id {
                        server_ctx.curr_region_content =
                            ContentContext::CharacterTemplate(character_template_id);
                        server_ctx.cc = ContentContext::CharacterTemplate(character_template_id);
                        // set_code(ui, ctx, project, server_ctx);
                    }
                }
                // Region Selection
                else if id.name == "Region Item" {
                    for r in &project.regions {
                        if r.id == id.uuid {
                            server_ctx.curr_region = r.id;
                            redraw = true;
                        }
                    }
                }
                // An item in the tile list was selected
                else if id.name == "Tilemap Tile" {
                    self.curr_tile_uuid = Some(id.uuid);
                } else if id.name == "Tilemap Editor Add Anim"
                    || id.name == "Tilemap Editor Add Multi"
                {
                }
            }
            _ => {}
        }
        redraw
    }

    /// Sets the settings for the map selection.
    pub fn apply_map_settings(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(id) = server_ctx.pc.id()
            && server_ctx.pc.is_screen()
        {
            let mut sector_id = None;
            if let Some(map) = project.get_map_mut(server_ctx) {
                if !map.selected_sectors.is_empty() {
                    if let Some(sector) = map.find_sector_mut(map.selected_sectors[0]) {
                        if sector.name.is_empty() {
                            // sector.name = "Unnamed".to_string();
                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named_with_id_and_reference(
                                    "Screen Content List Item",
                                    sector.creator_id,
                                    sector.creator_id,
                                ),
                                TheWidgetState::Clicked,
                            ));
                        }

                        sector_id = Some(sector.creator_id);
                    }
                }
            }

            if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                if let Some(node) = tree_layout.get_node_by_id_mut(&id) {
                    if let Some(screen) = project.screens.get(&id) {
                        gen_screen_tree_items(node, screen);
                    }
                    if let Some(sector_id) = sector_id {
                        node.new_item_selected(&TheId::named_with_id_and_reference(
                            "Screen Content List Item",
                            sector_id,
                            id,
                        ));
                    }
                }
            }
        }

        /*

        // Create Node Settings if necessary
        if let Some(layout) = ui.get_text_layout("Node Settings") {
            layout.clear();
        }

        if server_ctx.curr_map_tool_type != MapToolType::Sector {
            CODEEDITOR.write().unwrap().clear_shader(ui, ctx);
        }

        if let Some(map) = project.get_map_mut(server_ctx) {
            if server_ctx.curr_map_tool_type == MapToolType::Linedef
                && !map.selected_linedefs.is_empty()
            {
                self.create_linedef_settings(map, map.selected_linedefs[0], ui, ctx, server_ctx);
            } else if (server_ctx.curr_map_tool_type == MapToolType::Sector
                || server_ctx.curr_map_tool_type == MapToolType::Rect)
                && !map.selected_sectors.is_empty()
            {
                if server_ctx.get_map_context() == MapContext::Screen {
                    // In Screen Context make sure we create the default code and data items
                    if let Some(layout) = ui.get_list_layout("Screen Content List") {
                        if let Some(sector) = map.find_sector_mut(map.selected_sectors[0]) {
                            if sector.name.is_empty() {
                                sector.name = "Unnamed".to_string();
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Content List"),
                                    TheValue::Empty,
                                ));
                                ctx.ui.send(TheEvent::StateChanged(
                                    TheId::named_with_id_and_reference(
                                        "Screen Content List Item",
                                        sector.creator_id,
                                        sector.creator_id,
                                    ),
                                    TheWidgetState::Clicked,
                                ));
                            }
                            layout.select_item(sector.creator_id, ctx, true);
                        }
                    }
                }
                self.create_sector_settings(map, map.selected_sectors[0], ui, ctx, server_ctx);
            } else if server_ctx.curr_map_tool_type == MapToolType::Vertex
                && !map.selected_vertices.is_empty()
            {
                self.create_vertex_settings(map, map.selected_vertices[0], ui, ctx, server_ctx);
            }
        }*/
    }

    /// Adds an undo step for the given map change.
    fn add_map_undo(&self, map: &Map, prev: Map, ctx: &mut TheContext, server_ctx: &ServerContext) {
        let undo_atom =
            ProjectUndoAtom::MapEdit(server_ctx.pc, Box::new(prev), Box::new(map.clone()));
        UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
        RUSTERIX.write().unwrap().set_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusterix::PixelSource;
    use vek::Vec3;

    fn selected_box_with_textured_face() -> (Map, Uuid) {
        let mut map = Map::default();
        let mut object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        object.faces[2].tile = Some(PixelSource::PaletteIndex(3));
        map.geometry_objects.push(object);
        map.selected_geometry_faces.push((object_id, 2));
        (map, object_id)
    }

    #[test]
    fn arrow_shortcuts_adjust_selected_textured_face_uvs() {
        let (mut map, _) = selected_box_with_textured_face();

        assert!(MapEditor::nudge_selected_face_texture(
            &mut map,
            TheKeyCode::Right,
            false,
            false,
        ));
        assert_eq!(map.geometry_objects[0].faces[2].texture_offset.x, 0.1);

        assert!(MapEditor::nudge_selected_face_texture(
            &mut map,
            TheKeyCode::Left,
            true,
            false,
        ));
        assert_eq!(map.geometry_objects[0].faces[2].texture_rotation, -5.0);

        assert!(MapEditor::nudge_selected_face_texture(
            &mut map,
            TheKeyCode::Up,
            false,
            true,
        ));
        assert_eq!(map.geometry_objects[0].faces[2].texture_scale.y, 1.1);
    }

    #[test]
    fn arrow_shortcuts_follow_mirrored_face_uv_winding() {
        let (mut map, _) = selected_box_with_textured_face();
        let face = &mut map.geometry_objects[0].faces[2];
        face.auto_uv = false;
        face.uvs = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];

        assert!(MapEditor::nudge_selected_face_texture(
            &mut map,
            TheKeyCode::Right,
            false,
            false,
        ));
        assert_eq!(map.geometry_objects[0].faces[2].texture_offset.x, -0.1);

        assert!(MapEditor::nudge_selected_face_texture(
            &mut map,
            TheKeyCode::Right,
            true,
            false,
        ));
        assert_eq!(map.geometry_objects[0].faces[2].texture_rotation, -5.0);
    }

    #[test]
    fn arrow_shortcuts_ignore_untextured_faces() {
        let (mut map, object_id) = selected_box_with_textured_face();
        map.selected_geometry_faces = vec![(object_id, 1)];

        assert!(!MapEditor::nudge_selected_face_texture(
            &mut map,
            TheKeyCode::Right,
            false,
            false,
        ));
        assert_eq!(map.geometry_objects[0].faces[1].texture_offset.x, 0.0);
    }
}
