use crate::editor::{EDITCAMERA, NODEEDITOR, RUSTERIX, SIDEBARMODE, UNDOMANAGER};
use crate::prelude::*;
use rusterix::{D3Camera, PixelSource, Value};
use vek::Vec2;

pub struct MapEditor {
    curr_tile_uuid: Option<Uuid>,

    icon_normal_border_color: RGBA,
    icon_selected_border_color: RGBA,
}

#[allow(clippy::new_without_default)]
impl MapEditor {
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

        let mut poly_canvas: TheCanvas = TheCanvas::new();
        let mut render_view = TheRenderView::new(TheId::named("PolyView"));
        render_view.set_auto_focus(true);
        poly_canvas.set_widget(render_view);

        center.set_center(poly_canvas);

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
        grid_sub_div.set_range(TheValue::RangeI32(1..=10));
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
                        TheKeyCode::Up => {
                            if server_ctx.editor_view_mode == EditorViewMode::FirstP {
                                EDITCAMERA.write().unwrap().move_action =
                                    Some(CustomMoveAction::Forward);
                            }
                        }
                        TheKeyCode::Down => {
                            if server_ctx.editor_view_mode == EditorViewMode::FirstP {
                                EDITCAMERA.write().unwrap().move_action =
                                    Some(CustomMoveAction::Backward);
                            }
                        }
                        TheKeyCode::Left => {
                            if server_ctx.editor_view_mode == EditorViewMode::FirstP {
                                EDITCAMERA.write().unwrap().move_action =
                                    Some(CustomMoveAction::Left);
                            }
                        }
                        TheKeyCode::Right => {
                            if server_ctx.editor_view_mode == EditorViewMode::FirstP {
                                EDITCAMERA.write().unwrap().move_action =
                                    Some(CustomMoveAction::Right);
                            }
                        }
                        _ => {}
                    }
                }
            }
            TheEvent::KeyCodeUp(TheValue::KeyCode(_)) => {
                if server_ctx.editor_view_mode == EditorViewMode::FirstP {
                    EDITCAMERA.write().unwrap().move_action = None;
                }
            }

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
                        let prev = map.clone();
                        server_ctx.clipboard = map.copy_selected(true);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Geometry copied into clipboard.".to_string(),
                        ));
                        let undo_atom =
                            RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));
                        UNDOMANAGER.write().unwrap().add_region_undo(
                            &server_ctx.curr_region,
                            undo_atom,
                            ctx,
                        );
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

                        let rusterix = RUSTERIX.read().unwrap();
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            NODEEDITOR
                                .write()
                                .unwrap()
                                .create_shape_preview(map, &rusterix.assets);
                        }
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

                            let rusterix = RUSTERIX.read().unwrap();
                            if let Some(map) = project.get_map_mut(server_ctx) {
                                NODEEDITOR
                                    .write()
                                    .unwrap()
                                    .create_shape_preview(map, &rusterix.assets);
                            }
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
                        rusterix.client.target_offset -= *coord;
                    } else if !server_ctx.world_mode {
                        if let Some(map) = project.get_map_mut(server_ctx) {
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
                                if ui.logo || ui.ctrl {
                                    EDITCAMERA
                                        .write()
                                        .unwrap()
                                        .scroll_by(coord.y as f32, server_ctx);
                                } else if ui.alt {
                                    EDITCAMERA
                                        .write()
                                        .unwrap()
                                        .rotate(coord.map(|v| v as f32), server_ctx);
                                } else if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    // Move camera
                                    if server_ctx.editor_view_mode == EditorViewMode::Orbit {
                                        // Orbit move — screen-space pan using ray/plane intersections
                                        if let Some(render_view) = ui.get_render_view("PolyView") {
                                            let dim = *render_view.dim();
                                            let viewport_w = dim.x as f32;
                                            let viewport_h = dim.y as f32;
                                            if viewport_w > 0.0 && viewport_h > 0.0 {
                                                let orbit =
                                                    &EDITCAMERA.read().unwrap().orbit_camera;

                                                // Camera basis and parameters
                                                let (fwd, right, up) = orbit.basis_vectors();
                                                let distance = orbit.distance();
                                                let fov = orbit.fov; // vertical fov (radians)
                                                let aspect = viewport_w / viewport_h;
                                                let tan_half_fov = (fov * 0.5).tan();

                                                // Orbit target (pivot) is the editing position
                                                let target = region.editing_position_3d;
                                                // Reconstruct camera position from target, forward and distance
                                                let cam_pos = target - fwd * distance;

                                                // Ground plane at target.y (y-up world)
                                                let plane_y = target.y;
                                                let eps = 1e-6f32;

                                                // Helper: build world-space ray dir from pixel offset relative to screen center
                                                let ray_dir = |dx_pixels: f32, dy_pixels: f32| -> vek::Vec3<f32> {
                                                    // Convert to NDC offsets (center = 0,0). Note: screen y grows downward
                                                    let x_ndc = (dx_pixels) / (viewport_w * 0.5);
                                                    let y_ndc = (-dy_pixels) / (viewport_h * 0.5);
                                                    // Camera-space scale along right/up for given pixel offset
                                                    let sx = x_ndc * tan_half_fov * aspect;
                                                    let sy = y_ndc * tan_half_fov;
                                                    (fwd + right * sx + up * sy).normalized()
                                                };

                                                let intersect_plane = |orig: vek::Vec3<f32>, dir: vek::Vec3<f32>| -> Option<vek::Vec3<f32>> {
                                                    if dir.y.abs() < eps { return None; }
                                                    let t = (plane_y - orig.y) / dir.y;
                                                    if t.is_finite() { Some(orig + dir * t) } else { None }
                                                };

                                                let dir0 = ray_dir(0.0, 0.0);
                                                let dir1 = ray_dir(coord.x as f32, coord.y as f32);

                                                if let (Some(p0), Some(p1)) = (
                                                    intersect_plane(cam_pos, dir0),
                                                    intersect_plane(cam_pos, dir1),
                                                ) {
                                                    let delta = p0 - p1;
                                                    if delta.x.is_finite()
                                                        && delta.y.is_finite()
                                                        && delta.z.is_finite()
                                                    {
                                                        region.editing_position_3d += delta;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if server_ctx.editor_view_mode == EditorViewMode::Iso {
                                        // Iso move
                                        if let Some(render_view) = ui.get_render_view("PolyView") {
                                            let dim = *render_view.dim();
                                            let viewport_h = dim.y as f32;

                                            let (_fwd, right, up) = EDITCAMERA
                                                .read()
                                                .unwrap()
                                                .iso_camera
                                                .basis_vectors();
                                            let ortho_h =
                                                EDITCAMERA.read().unwrap().iso_camera.scale();
                                            let world_per_pixel = (ortho_h) / viewport_h;

                                            let right_xz: Vec3<f32> =
                                                vek::Vec3::new(right.x, 0.0, right.z).normalized();
                                            let up_xz =
                                                vek::Vec3::new(up.x, 0.0, up.z).normalized();

                                            let delta = right_xz * coord.x as f32 * world_per_pixel
                                                - up_xz * coord.y as f32 * world_per_pixel;

                                            region.editing_position_3d += delta;
                                        }
                                    } else {
                                        // Firstp move
                                        let firstp = &EDITCAMERA.read().unwrap().firstp_camera;
                                        let (forward, right, _up) = firstp.basis_vectors();

                                        let speed = 0.1;
                                        let dx = coord.x as f32;
                                        let dy = coord.y as f32;
                                        let delta = forward * (dy) * speed + right * (dx) * speed;

                                        region.editing_position_3d += delta;
                                    }
                                    redraw = true;
                                }
                            }
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Soft Update Minimap"),
                                TheValue::Empty,
                            ));
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
            TheEvent::GainedFocus(id) => {
                if id.name == "Region Editor View"
                    || id.name == "RenderView"
                    || id.name == "PolyView"
                {
                    if server_ctx.get_map_context() == MapContext::Shader {
                        UNDOMANAGER.write().unwrap().context = UndoManagerContext::Material;
                    } else if server_ctx.get_map_context() == MapContext::Screen {
                        UNDOMANAGER.write().unwrap().context = UndoManagerContext::Screen;
                    } else {
                        UNDOMANAGER.write().unwrap().context = UndoManagerContext::Region;
                    }
                } else if id.name == "ModelFX RGBA Layout View" {
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Material;
                } else if id.name == "Palette Picker" {
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Palette;
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Grid Subdiv" {
                    if let Some(value) = value.to_i32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.map.subdivisions = value as f32;
                        }
                    }
                } else if id.name == "lightColor" {
                    if let Some(value) = value.to_color() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let prev = map.clone();
                            let mut changed = false;
                            for linedef_id in map.selected_linedefs.clone() {
                                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                    if let Some(row) = server_ctx.selected_wall_row {
                                        let i = row + 1;
                                        let light_name = format!("row{i}_light");
                                        if let Some(Value::Light(light)) =
                                            linedef.properties.get_mut(&light_name)
                                        {
                                            light.set_color(value.to_array_3());
                                            changed = true;
                                        }
                                    }
                                }
                            }
                            for sector_id in map.selected_sectors.clone() {
                                if let Some(sector) = map.find_sector_mut(sector_id) {
                                    if let Some(Value::Light(light)) =
                                        sector.properties.get_mut("floor_light")
                                    {
                                        light.set_color(value.to_array_3());
                                        changed = true;
                                    }
                                    if let Some(Value::Light(light)) =
                                        sector.properties.get_mut("ceiling_light")
                                    {
                                        light.set_color(value.to_array_3());
                                        changed = true;
                                    }
                                }
                            }
                            if changed {
                                self.add_map_undo(map, prev, ctx, server_ctx);
                                if server_ctx.get_map_context() == MapContext::Region {
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Render SceneManager Map"),
                                        TheValue::Empty,
                                    ));
                                }
                            }
                        }
                    }
                } else if id.name == "lightIntensity"
                    || id.name == "lightStartDistance"
                    || id.name == "lightEndDistance"
                {
                    if let Some(value) = value.to_f32() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let prev = map.clone();
                            let mut changed = false;
                            for linedef_id in map.selected_linedefs.clone() {
                                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                                    if let Some(row) = server_ctx.selected_wall_row {
                                        let i = row + 1;
                                        let light_name = format!("row{i}_light");
                                        if let Some(Value::Light(light)) =
                                            linedef.properties.get_mut(&light_name)
                                        {
                                            if id.name == "lightIntensity" {
                                                light.set_intensity(value);
                                                changed = true;
                                            } else if id.name == "lightStartDistance" {
                                                light.set_start_distance(value);
                                                changed = true;
                                            } else if id.name == "lightEndDistance" {
                                                light.set_end_distance(value);
                                                changed = true;
                                            }
                                        }
                                    }
                                }
                            }
                            for sector_id in map.selected_sectors.clone() {
                                if let Some(sector) = map.find_sector_mut(sector_id) {
                                    if let Some(Value::Light(light)) =
                                        sector.properties.get_mut("floor_light")
                                    {
                                        if id.name == "lightIntensity" {
                                            light.set_intensity(value);
                                            changed = true;
                                        } else if id.name == "lightStartDistance" {
                                            light.set_start_distance(value);
                                            changed = true;
                                        } else if id.name == "lightEndDistance" {
                                            light.set_end_distance(value);
                                            changed = true;
                                        }
                                    }
                                    if let Some(Value::Light(light)) =
                                        sector.properties.get_mut("ceiling_light")
                                    {
                                        if id.name == "lightIntensity" {
                                            light.set_intensity(value);
                                            changed = true;
                                        } else if id.name == "lightStartDistance" {
                                            light.set_start_distance(value);
                                            changed = true;
                                        } else if id.name == "lightEndDistance" {
                                            light.set_end_distance(value);
                                            changed = true;
                                        }
                                    }
                                }
                            }
                            if changed {
                                self.add_map_undo(map, prev, ctx, server_ctx);
                                if server_ctx.get_map_context() == MapContext::Region {
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Render SceneManager Map"),
                                        TheValue::Empty,
                                    ));
                                }
                            }
                        }
                    }
                } else if id.name == "vertexName" {
                    if let Some(value) = value.to_string() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            for vertex_id in &region.map.selected_vertices.clone() {
                                let prev = region.map.clone();
                                if let Some(vertex) = region.map.find_vertex_mut(*vertex_id) {
                                    vertex.name = value.to_string();
                                    let undo_atom = RegionUndoAtom::MapEdit(
                                        Box::new(prev),
                                        Box::new(region.map.clone()),
                                    );
                                    UNDOMANAGER.write().unwrap().add_region_undo(
                                        &server_ctx.curr_region,
                                        undo_atom,
                                        ctx,
                                    );
                                }
                            }
                        }
                    }
                } else if id.name == "vertexHeight" {
                    if let Some(value) = value.to_f32() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            for vertex_id in &map.selected_vertices.clone() {
                                let prev = map.clone();
                                if let Some(vertex) = map.find_vertex_mut(*vertex_id) {
                                    vertex.properties.set(
                                        &self.transform_to_snake_case(&id.name, "vertex"),
                                        Value::Float(value),
                                    );
                                    self.add_map_undo(map, prev, ctx, server_ctx);
                                    if server_ctx.get_map_context() == MapContext::Region {
                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Render SceneManager Map"),
                                            TheValue::Empty,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "linedefWallHeight" || id.name == "linedefWallWidth" {
                    if let Some(value) = value.to_f32() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            for linedef_id in &map.selected_linedefs.clone() {
                                let prev = map.clone();
                                if let Some(linedef) = map.find_linedef_mut(*linedef_id) {
                                    // if linedef.properties.get_float_default(
                                    //     &self.transform_to_snake_case(&id.name, "linedef"),
                                    //     0.0,
                                    // ) != value
                                    // {
                                    linedef.properties.set(
                                        &self.transform_to_snake_case(&id.name, "linedef"),
                                        Value::Float(value),
                                    );
                                    self.add_map_undo(map, prev, ctx, server_ctx);
                                    if server_ctx.get_map_context() == MapContext::Region {
                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Render SceneManager Map"),
                                            TheValue::Empty,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "linedefSourceRepeat" || id.name == "linedefCastsShadows" {
                    if let Some(value) = value.to_i32() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            for linedef_id in &map.selected_linedefs.clone() {
                                let prev = map.clone();
                                if let Some(linedef) = map.find_linedef_mut(*linedef_id) {
                                    linedef.properties.set(
                                        &self.transform_to_snake_case(&id.name, "linedef"),
                                        Value::Int(value),
                                    );
                                    self.add_map_undo(map, prev, ctx, server_ctx);
                                }
                            }
                        }
                    }
                    redraw = true;
                } else if id.name == "linedefName" {
                    if let Some(value) = value.to_string() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            for linedef_id in &map.selected_linedefs.clone() {
                                let prev = map.clone();
                                if let Some(linedef) = map.find_linedef_mut(*linedef_id) {
                                    linedef.name = value.to_string();
                                    let undo_atom = RegionUndoAtom::MapEdit(
                                        Box::new(prev),
                                        Box::new(map.clone()),
                                    );
                                    UNDOMANAGER.write().unwrap().add_region_undo(
                                        &server_ctx.curr_region,
                                        undo_atom,
                                        ctx,
                                    );
                                }
                            }
                        }
                    }
                } else if id.name == "sectorName" {
                    if let Some(value) = value.to_string() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            for sector_id in &map.selected_sectors.clone() {
                                let prev = map.clone();
                                if let Some(sector) = map.find_sector_mut(*sector_id) {
                                    if sector.name != value.clone() {
                                        sector.name = value.clone();
                                        let undo_atom = RegionUndoAtom::MapEdit(
                                            Box::new(prev),
                                            Box::new(map.clone()),
                                        );
                                        UNDOMANAGER.write().unwrap().add_region_undo(
                                            &server_ctx.curr_region,
                                            undo_atom,
                                            ctx,
                                        );
                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Update Content List"),
                                            TheValue::Empty,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    redraw = true;
                } else if id.name == "sectorFloorHeight"
                    || id.name == "sectorCeilingHeight"
                    || id.name == "sectorOcclusion"
                {
                    if let Some(value) = value.to_f32() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            for sector_id in &map.selected_sectors.clone() {
                                let prev = map.clone();
                                map.changed += 1;
                                if let Some(sector) = map.find_sector_mut(*sector_id) {
                                    sector.properties.set(
                                        &self.transform_to_snake_case(&id.name, "sector"),
                                        Value::Float(value),
                                    );
                                    self.add_map_undo(map, prev, ctx, server_ctx);
                                    if server_ctx.get_map_context() == MapContext::Region {
                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Render SceneManager Map"),
                                            TheValue::Empty,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    redraw = true;
                } else if id.name == "sectorCeilingInIso"
                    || id.name == "sectorRectRendering"
                    || id.name == "sectorTileMode"
                {
                    if let Some(value) = value.to_i32() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            for sector_id in &map.selected_sectors.clone() {
                                let prev = map.clone();
                                if let Some(sector) = map.find_sector_mut(*sector_id) {
                                    sector.properties.set(
                                        &self.transform_to_snake_case(&id.name, "sector"),
                                        Value::Int(value),
                                    );
                                    self.add_map_undo(map, prev, ctx, server_ctx);
                                    if server_ctx.get_map_context() == MapContext::Region {
                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Render SceneManager Map"),
                                            TheValue::Empty,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    redraw = true;
                } else if id.name == "softRigName" {
                    if let Some(value) = value.to_string() {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            if let Some(id) = map.editing_rig {
                                let prev = map.clone();
                                if let Some(rig) = map.softrigs.get_mut(&id) {
                                    rig.name = value;
                                    let undo_atom = RegionUndoAtom::MapEdit(
                                        Box::new(prev),
                                        Box::new(map.clone()),
                                    );
                                    UNDOMANAGER.write().unwrap().add_region_undo(
                                        &server_ctx.curr_region,
                                        undo_atom,
                                        ctx,
                                    );
                                }
                            }
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

                        /*
                        let undo_atom =
                            RegionUndoAtom::MapEdit(Box::new(prev), Box::new(region.map.clone()));
                        UNDOMANAGER.write().unwrap().add_region_undo(
                            &server_ctx.curr_region,
                            undo_atom,
                            ctx,
                        );
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));*/

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
                    // TILEDRAWER.lock().unwrap().tiles = project.extract_tiles();
                    // server.update_tiles(project.extract_tiles());
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
                            sector.name = "Unnamed".to_string();
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
        if server_ctx.get_map_context() == MapContext::Region {
            let undo_atom = RegionUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));
            UNDOMANAGER
                .write()
                .unwrap()
                .add_region_undo(&server_ctx.curr_region, undo_atom, ctx);
        } else if server_ctx.get_map_context() == MapContext::Model {
            /*
            let undo_atom = MaterialUndoAtom::MapEdit(Box::new(prev), Box::new(map.clone()));
            UNDOMANAGER
                .write()
                .unwrap()
                .add_material_undo(undo_atom, ctx);
            */
        }
        RUSTERIX.write().unwrap().set_dirty();
    }

    /*
    fn create_light_settings(
        &self,
        map: &Map,
        index: u32,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) {
        let nodeui = EffectWrapper::create_light_ui(&map.lights[index as usize]);
        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Show Node Settings"),
                TheValue::Text("Light Settings".to_string()),
            ));
        }
    }*/

    fn _create_vertex_settings(
        &self,
        map: &Map,
        vertex_id: u32,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) {
        /*
        // Check if we need to apply the shape graph to the node editor
        if server_ctx.curr_map_context == MapContext::Character
            || server_ctx.curr_map_context == MapContext::Item
        {
            if server_ctx.curr_map_tool_helper != MapToolHelper::NodeEditor {
                ctx.ui
                    .send(TheEvent::IndexChanged(TheId::named("Map Helper Switch"), 2));
                if let Some(widget) = ui.get_group_button("Map Helper Switch") {
                    widget.set_index(2);
                }
            }
            if let Some(vertex) = map.find_vertex(vertex_id) {
                if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                    vertex.properties.get("shape_graph")
                {
                    if let Some(graph) = map.shapefx_graphs.get(id) {
                        NODEEDITOR
                            .write()
                            .unwrap()
                            .apply_graph(NodeContext::Shape, graph, ui, ctx);
                    }
                }
            }
        }*/

        let mut nodeui = TheNodeUI::default();

        if let Some(vertex) = map.find_vertex(vertex_id) {
            let item = TheNodeUIItem::Text(
                "vertexName".into(),
                "Name".into(),
                "Set the name of the vertex".into(),
                vertex.name.clone(),
                None,
                false,
            );
            nodeui.add_item(item);
        }

        if let Some(vertex) = map.find_vertex(vertex_id) {
            let item = TheNodeUIItem::FloatEditSlider(
                "vertexHeight".into(),
                "Height".into(),
                "Specifies the height at this vertex, used by region graph nodes (e.g. paths) to shape the terrain or lights etc.".into(),
                vertex.properties.get_float_default("height", 0.0),
                0.0..=100.0,
                false,
            );
            nodeui.add_item(item);
        }

        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            // layout.relayout(ctx);
            ctx.ui.relayout = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Show Node Settings"),
                TheValue::Text("Vertex Settings".to_string()),
            ));
        }
    }

    fn _create_linedef_settings(
        &self,
        map: &Map,
        linedef_id: u32,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // Check if we need to apply the shape graph to the node editor
        if server_ctx.get_map_context() == MapContext::Character
            || server_ctx.get_map_context() == MapContext::Item
        {
            if server_ctx.curr_map_tool_helper != MapToolHelper::NodeEditor {
                ctx.ui
                    .send(TheEvent::IndexChanged(TheId::named("Map Helper Switch"), 2));
                if let Some(widget) = ui.get_group_button("Map Helper Switch") {
                    widget.set_index(2);
                }
            }
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                    linedef.properties.get("shape_graph")
                {
                    if let Some(graph) = map.shapefx_graphs.get(id) {
                        NODEEDITOR
                            .write()
                            .unwrap()
                            .apply_graph(NodeContext::Shape, graph, ui, ctx);
                    }
                }
            }
        } else
        // Check if we need to apply the material graph to the node editor
        if server_ctx.get_map_context() == MapContext::Shader {
            if server_ctx.curr_map_tool_helper != MapToolHelper::NodeEditor {
                ctx.ui
                    .send(TheEvent::IndexChanged(TheId::named("Map Helper Switch"), 2));
                if let Some(widget) = ui.get_group_button("Map Helper Switch") {
                    widget.set_index(2);
                }
            }
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(PixelSource::ShapeFXGraphId(id)) =
                    linedef.properties.get_default_source()
                {
                    if let Some(graph) = map.shapefx_graphs.get(id) {
                        NODEEDITOR.write().unwrap().apply_graph(
                            NodeContext::Material,
                            graph,
                            ui,
                            ctx,
                        );
                    }
                }
            }
        } else
        // Check if we need to apply the node graph to the node editor
        // if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
        if let Some(linedef) = map.find_linedef(linedef_id) {
            if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                linedef.properties.get("region_graph")
            {
                if let Some(graph) = map.shapefx_graphs.get(id) {
                    NODEEDITOR
                        .write()
                        .unwrap()
                        .apply_graph(NodeContext::Region, graph, ui, ctx);
                }
            }
        }

        let mut nodeui = TheNodeUI::default();
        if let Some(linedef) = map.find_linedef(linedef_id) {
            let item = TheNodeUIItem::Text(
                "linedefName".into(),
                "Name".into(),
                "Set the name of the linedef".into(),
                linedef.name.clone(),
                None,
                false,
            );
            nodeui.add_item(item);

            if server_ctx.get_map_context() == MapContext::Region {
                // let item = TheNodeUIItem::Text(
                //     "linedefInstanceOf".into(),
                //     "Name".into(),
                //     "Set the item instance of the linedef".into(),
                //     linedef.name.clone(),
                //     None,
                //     false,
                // );
                // nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "linedefWallWidth".into(),
                    "Wall Width".into(),
                    "Set the width of the wall in 2D.".into(),
                    linedef.properties.get_float_default("wall_width", 0.0),
                    0.0..=2.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "linedefWallHeight".into(),
                    "Wall Height".into(),
                    "Set the height of the wall.".into(),
                    linedef.properties.get_float_default("wall_height", 0.0),
                    0.0..=4.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::Selector(
                    "linedefCastsShadows".into(),
                    "Casts Shadows".into(),
                    "Defines if the linedef casts shadows.".into(),
                    vec!["Yes".to_string(), "No".to_string()],
                    linedef.properties.get_int_default("casts_shadows", 0),
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::Selector(
                    "linedefSourceRepeat".into(),
                    "Repeat Source".into(),
                    "Set if the source textures should be repeated or applied individually for each row (allowing gaps).".into(),
                    vec!["Yes".to_string(), "No".to_string()],
                    linedef.properties.get_int_default("source_repeat", 0),
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::Button(
                    "linedefAddMidpoint".into(),
                    "Add Midpoint".into(),
                    "Adds a new midpoint vertex to the line, splitting it.".into(),
                    "Split Line".into(),
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::Button(
                    "linedefCreateWall".into(),
                    "Create Wall".into(),
                    "Create a wall profile for the linedef.".into(),
                    "Wall Profile".into(),
                );
                nodeui.add_item(item);
                /*
                } else {
                    let item = TheNodeUIItem::Button(
                        "linedefDeleteWall".into(),
                        "Delete Wall".into(),
                        "Delete the wall profile for the linedef.".into(),
                        "Wall Profile".into(),
                    );
                    nodeui.add_item(item);
                }*/
            }
        }

        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            // layout.relayout(ctx);
            ctx.ui.relayout = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Show Node Settings"),
                TheValue::Text("Linedef Settings".to_string()),
            ));
        }
    }

    fn _create_sector_settings(
        &self,
        map: &Map,
        sector_id: u32,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // Check if we need to apply the shape graph to the node editor
        if server_ctx.get_map_context() == MapContext::Character
            || server_ctx.get_map_context() == MapContext::Item
        {
            if server_ctx.curr_map_tool_helper != MapToolHelper::NodeEditor {
                ctx.ui
                    .send(TheEvent::IndexChanged(TheId::named("Map Helper Switch"), 2));
                if let Some(widget) = ui.get_group_button("Map Helper Switch") {
                    widget.set_index(2);
                }
            }
            if let Some(sector) = map.find_sector(sector_id) {
                if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                    sector.properties.get("shape_graph")
                {
                    if let Some(graph) = map.shapefx_graphs.get(id) {
                        NODEEDITOR
                            .write()
                            .unwrap()
                            .apply_graph(NodeContext::Shape, graph, ui, ctx);
                    }
                }
            }
        } else
        // Check if we need to apply the screen graph to the node editor
        if server_ctx.get_map_context() == MapContext::Screen {
            if server_ctx.curr_map_tool_helper != MapToolHelper::NodeEditor {
                ctx.ui
                    .send(TheEvent::IndexChanged(TheId::named("Map Helper Switch"), 2));
                if let Some(widget) = ui.get_group_button("Map Helper Switch") {
                    widget.set_index(2);
                }
            }
            if let Some(sector) = map.find_sector(sector_id) {
                if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                    sector.properties.get("screen_graph")
                {
                    if let Some(graph) = map.shapefx_graphs.get(id) {
                        NODEEDITOR.write().unwrap().apply_graph(
                            NodeContext::Screen,
                            graph,
                            ui,
                            ctx,
                        );
                    }
                }
            }
        } else
        // Check if we need to apply the material graph to the node editor
        if server_ctx.get_map_context() == MapContext::Shader {
            if server_ctx.curr_map_tool_helper != MapToolHelper::NodeEditor {
                ctx.ui
                    .send(TheEvent::IndexChanged(TheId::named("Map Helper Switch"), 2));
                if let Some(widget) = ui.get_group_button("Map Helper Switch") {
                    widget.set_index(2);
                }
            }
            if let Some(sector) = map.find_sector(sector_id) {
                if let Some(PixelSource::ShapeFXGraphId(id)) =
                    sector.properties.get_default_source()
                {
                    if let Some(graph) = map.shapefx_graphs.get(id) {
                        NODEEDITOR.write().unwrap().apply_graph(
                            NodeContext::Material,
                            graph,
                            ui,
                            ctx,
                        );
                    }
                }
            }
        } else
        //if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
        if let Some(sector) = map.find_sector(sector_id) {
            if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                sector.properties.get("region_graph")
            {
                if let Some(graph) = map.shapefx_graphs.get(id) {
                    NODEEDITOR
                        .write()
                        .unwrap()
                        .apply_graph(NodeContext::Region, graph, ui, ctx);
                }
            }
        }

        let mut nodeui = TheNodeUI::default();

        if let Some(sector) = map.find_sector(sector_id) {
            /*
                CODEEDITOR
                    .write()
                    .unwrap()
                    .set_shader_sector(ui, ctx, sector);
            */

            let item = TheNodeUIItem::Text(
                "sectorName".into(),
                "Name".into(),
                "Set the name of the sector".into(),
                sector.name.clone(),
                None,
                false,
            );
            nodeui.add_item(item);

            if server_ctx.get_map_context() == MapContext::Region {
                let item = TheNodeUIItem::FloatEditSlider(
                    "sectorFloorHeight".into(),
                    "Floor Height".into(),
                    "Set the elevation of of the sector's floor.".into(),
                    sector.properties.get_float_default("floor_height", 0.0),
                    0.0..=4.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "sectorCeilingHeight".into(),
                    "Ceiling Height".into(),
                    "Set the elevation of the sector's ceiling.".into(),
                    sector.properties.get_float_default("ceiling_height", 0.0),
                    0.0..=4.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::Selector(
                    "sectorCeilingInIso".into(),
                    "Ceiling in ISO".into(),
                    "Set if the ceiling should be displayed in an ISO camera.".into(),
                    vec!["Yes".to_string(), "No".to_string()],
                    sector.properties.get_int_default("ceiling_in_iso", 0),
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "sectorOcclusion".into(),
                    "Occlusion".into(),
                    "Specifies the occlusion of daylight for the sector. A value of 1.0 means full daylight, 0.0 means no daylight.".into(),
                    sector.properties.get_float_default("occlusion", 1.0),
                    0.0..=1.0,
                    false,
                );
                nodeui.add_item(item);
            }

            let item = TheNodeUIItem::Selector(
                "sectorTileMode".into(),
                "Tile Mode".into(),
                "Set the tile mode of the sector.".into(),
                vec!["Scale".to_string(), "Repeat".to_string()],
                sector.properties.get_int_default("tile_mode", 1),
            );
            nodeui.add_item(item);

            /*
            if server_ctx.get_map_context() == MapContext::Material
                || server_ctx.get_map_context() == MapContext::Character
                || server_ctx.get_map_context() == MapContext::Item
            {
                let item = TheNodeUIItem::FloatEditSlider(
                    "sectorMaterialAA".into(),
                    "Anti-Aliasing".into(),
                    "Amount of Anti-Aliasing.".into(),
                    sector.properties.get_float_default("material_a_a", 1.0),
                    0.0..=2.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "sectorMaterialRounding".into(),
                    "Rounding".into(),
                    "Amount of rounding.".into(),
                    sector
                        .properties
                        .get_float_default("material_rounding", 0.0),
                    0.0..=10.0,
                    false,
                );
                nodeui.add_item(item);
            }*/

            if sector.layer.is_some() {
                nodeui.add_item(TheNodeUIItem::Separator("Rect Tool".into()));

                let item = TheNodeUIItem::Selector(
                    "sectorRectRendering".into(),
                    "3D Render".into(),
                    "Set the 3D render mode of the Rect.".into(),
                    vec![
                        "Billboard".to_string(),
                        "Box".to_string(),
                        "Floor".to_string(),
                    ],
                    sector.properties.get_int_default("rect_rendering", 0),
                );
                nodeui.add_item(item);
            }

            // Show the floor light
            if let Some(Value::Light(light)) = sector.properties.get("floor_light") {
                let light_ui = EffectWrapper::create_light_ui(light);
                let item = TheNodeUIItem::Separator("Floor Light".to_string());
                nodeui.add_item(item);
                for (_, item) in light_ui.list_items() {
                    nodeui.add_item(item.clone());
                }
            }
            // Show the ceiling light
            if let Some(Value::Light(light)) = sector.properties.get("ceiling_light") {
                let light_ui = EffectWrapper::create_light_ui(light);
                let item = TheNodeUIItem::Separator("Ceiling Light".to_string());
                nodeui.add_item(item);
                for (_, item) in light_ui.list_items() {
                    nodeui.add_item(item.clone());
                }
            }

            // Screen
            if server_ctx.get_map_context() == MapContext::Screen {
                let item = TheNodeUIItem::Text(
                    "sectorName".into(),
                    "Name".into(),
                    "Set the name of the sector".into(),
                    sector.name.clone(),
                    None,
                    false,
                );
                nodeui.add_item(item);
            }
        }

        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            // layout.relayout(ctx);
            ctx.ui.relayout = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Show Node Settings"),
                TheValue::Text("Sector Settings".to_string()),
            ));
        }
    }

    fn transform_to_snake_case(&self, input: &str, strip_prefix: &str) -> String {
        // Strip the prefix if it exists
        let stripped = if let Some(remainder) = input.strip_prefix(strip_prefix) {
            remainder
        } else {
            input
        };

        // Convert to snake_case
        let mut snake_case = String::new();
        for (i, c) in stripped.chars().enumerate() {
            if c.is_uppercase() {
                // Add an underscore before uppercase letters (except for the first character)
                if i > 0 {
                    snake_case.push('_');
                }
                snake_case.push(c.to_ascii_lowercase());
            } else {
                snake_case.push(c);
            }
        }

        snake_case
    }
}
