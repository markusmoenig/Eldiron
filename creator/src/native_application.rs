//! Transitional Creator application on the common GPU UI/host contract.
//! Geometry and appearance outputs use the injected device. Legacy Editor owns the
//! unconverted world/tool/history orchestration; this is not M5/M7 retirement.
use crate::editor::{Editor, native_views::NativeViewTokens};
use renderkit::{RenderError, Result};
use std::time::{Duration, Instant};
use theframework::{paint::PaintList, platform::*, prelude::*, render_wgpu::UiFrameImages};

pub struct NativeCreatorApplication {
    editor: Editor,
    args: Vec<String>,
    pending: Option<NativeViewTokens>,
    smoke: bool,
    deadline: Option<Instant>,
    presentations: u64,
    start_services: bool,
}
impl NativeCreatorApplication {
    pub fn new(args: Vec<String>, smoke: bool) -> Self {
        let mut editor = Editor::new();
        editor.enable_native_views();
        Self {
            editor,
            args,
            pending: None,
            smoke,
            deadline: None,
            presentations: 0,
            start_services: true,
        }
    }
}
impl UiApplication for NativeCreatorApplication {
    fn title(&self) -> String {
        self.editor.window_title()
    }
    fn initial_size(&self) -> (u32, u32) {
        let size = self.editor.default_window_size();
        (size.0 as u32, size.1 as u32)
    }
    fn window_configuration(&self) -> WindowConfiguration {
        let minimum = self.editor.min_window_size();
        WindowConfiguration {
            minimum_size: Some((minimum.0 as u32, minimum.1 as u32)),
            position: self.editor.default_window_position(),
            icon: self.editor.window_icon(),
        }
    }
    fn document_edited(&self) -> bool {
        self.editor.has_changes()
    }
    fn window_moved(&mut self, position: (i32, i32), _: &mut UiSession) {
        self.editor.window_moved(position.0, position.1);
    }
    fn window_resized(&mut self, ui: &mut UiSession) {
        self.editor
            .window_resized(ui.context.width, ui.context.height);
    }
    fn initialize(&mut self, ui: &mut UiSession, _: &mut UiFrameImages<'_>) -> Result<()> {
        // Preserve Creator's existing root/layout bootstrap before docks are
        // constructed; some source-backed docks inspect their initial bounds.
        ui.ui.canvas.root = true;
        ui.ui.canvas.set_dim(
            TheDim::sized(ui.context.width as i32, ui.context.height as i32),
            &mut ui.context,
        );
        self.editor.init_ui(&mut ui.ui, &mut ui.context);
        ui.ui.prepare_paint_layout(&mut ui.context);
        #[cfg(not(target_arch = "wasm32"))]
        ui.context.load_system_fonts(self.editor.fonts_to_load());
        if self.start_services {
            self.editor.init(&mut ui.context);
        }
        self.editor
            .set_cmd_line_args(std::mem::take(&mut self.args), &mut ui.context);
        if self.smoke {
            self.deadline = Some(Instant::now() + Duration::from_secs(10));
        }
        Ok(())
    }
    fn input(&mut self, event: &InputEvent, ui: &mut UiSession) -> InputResponse {
        match event {
            InputEvent::Focus(false) => {
                let response = ui.input(event);
                self.editor
                    .cancel_native_world_gesture(&mut ui.ui, &mut ui.context);
                self.editor.invalidate_native_display_viewport(&mut ui.ui);
                self.editor.invalidate_native_world_viewport(&mut ui.ui);
                InputResponse {
                    redraw: true,
                    ..response
                }
            }
            InputEvent::RelativeMotion { delta } => {
                let handled = self
                    .editor
                    .mouse_motion(delta[0], delta[1], &mut ui.context);
                InputResponse {
                    consumed: handled,
                    redraw: handled,
                }
            }
            InputEvent::PointerLeft => {
                let response = ui.input(event);
                let changed = self.editor.mouse_leave(&mut ui.context);
                InputResponse {
                    consumed: response.consumed,
                    redraw: response.redraw || changed,
                }
            }
            _ => ui.input(event),
        }
    }
    fn update(&mut self, ui: &mut UiSession, _: Duration) -> FrameRequest {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
            && self.presentations < 3
        {
            ui.requests.push(HostRequest::Fail(format!(
                "Creator native smoke timed out: {} successful surface presentations",
                self.presentations
            )));
            return FrameRequest::idle();
        }
        self.editor.synchronize_native_document();
        let mut redraw = ui.ui.update(&mut ui.context);
        redraw |= self.editor.advance_native_world_camera(&mut ui.context);
        redraw |= self.editor.update_ui(&mut ui.ui, &mut ui.context);
        redraw |= self.editor.update(&mut ui.context);
        self.editor.publish_native_world_paint_sources();
        redraw |= self.editor.native_views_waiting();
        FrameRequest {
            redraw,
            wake_after: Some(Duration::from_secs_f64(
                1. / self.editor.target_fps().clamp(1., 240.),
            )),
        }
    }
    fn frame(&mut self, ui: &mut UiSession, images: &mut UiFrameImages<'_>) -> Result<PaintList> {
        self.editor.pre_ui(&mut ui.context);
        ui.ui.prepare_paint_layout(&mut ui.context);
        self.pending = match self
            .editor
            .prepare_native_views(&mut ui.ui, &mut ui.context, images)
        {
            Ok(tokens) => tokens,
            Err(error) => {
                self.pending = None;
                let _ = self
                    .editor
                    .finish_native_viewport_frame(None, images.registry);
                return Err(RenderError(error));
            }
        };
        let mut list = match ui.paint(images) {
            Ok(list) => list,
            Err(error) => {
                self.pending = None;
                let _ = self
                    .editor
                    .finish_native_viewport_frame(None, images.registry);
                return Err(RenderError(error.to_string()));
            }
        };
        list.ops.insert(
            0,
            theframework::paint::PaintOp::DisplayCompositing(
                theframework::paint::DisplayCompositing::LegacyOpaque,
            ),
        );
        if let Some(tokens) = &mut self.pending {
            tokens.retain_painted(&list);
        }
        self.editor
            .finish_native_viewport_frame(Some(&list), images.registry)
            .map_err(RenderError)?;
        Ok(list)
    }
    fn presented(&mut self, ui: &mut UiSession) {
        if let Some(token) = self.pending.take() {
            self.editor
                .presented_native_views(token, &mut ui.ui, &mut ui.context);
        }
        self.editor.post_ui(&mut ui.context);
        self.presentations += 1;
        if self.smoke && self.presentations >= 3 {
            ui.requests.push(HostRequest::Exit);
        }
    }
    fn gpu_recreated(&mut self, ui: &mut UiSession, _: &mut UiFrameImages<'_>) -> Result<()> {
        self.pending = None;
        self.editor
            .cancel_native_world_gesture(&mut ui.ui, &mut ui.context);
        self.editor.invalidate_native_views(&mut ui.ui);
        Ok(())
    }
    fn suspended(&mut self, ui: &mut UiSession) {
        self.pending = None;
        ui.input(&InputEvent::Focus(false));
        self.editor
            .cancel_native_world_gesture(&mut ui.ui, &mut ui.context);
        self.editor.mouse_leave(&mut ui.context);
        self.editor.invalidate_native_views(&mut ui.ui);
    }
    fn resumed(&mut self, ui: &mut UiSession) {
        ui.input(&InputEvent::Focus(true));
    }
    fn close_requested(&mut self, _: &mut UiSession) -> bool {
        !self.editor.closing()
            && (!self.editor.has_changes()
                || theframework::platform_services::confirm_unsaved_exit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu};
    use rusterix::D3Camera;
    use theframework::render_wgpu::PaintRenderer;
    #[test]
    #[ignore = "requires actual native GPU; exercises the assembled Creator UI"]
    fn native_creator_application_paints_the_root_ui_without_a_root_framebuffer() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut registry = ImageRegistry::new(gpu.clone());
        let mut application =
            NativeCreatorApplication::new(vec!["eldiron-creator-native".into()], false);
        // External services are unrelated to offscreen UI verification.
        application.start_services = false;
        let mut app = UiApplicationAdapter::new(application);
        let mut host = HostContext::new((1200, 800));
        app.initialize(&mut host, &mut registry).unwrap();
        let authored =
            serde_json::to_value(&app.application.editor.project_for_native_test()).unwrap();
        app.update(&mut host, Duration::from_millis(16));
        let list = app.frame(&mut host, &mut registry).unwrap();
        list.validate().unwrap();
        assert!(
            app.session.ui.canvas.buffer.pixels().is_empty(),
            "the Creator root cannot allocate a legacy display framebuffer"
        );
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1200, 800)).unwrap();
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        renderer
            .render(
                &list,
                &registry,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert!(renderer.frame_stats().glyph_quads > 20);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let before = gpu.transfers();
        let glyphs = renderer.glyphs.rasterizations;
        let repeated = app.frame(&mut host, &mut registry).unwrap();
        renderer
            .render(
                &repeated,
                &registry,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(gpu.transfers(), before);
        assert_eq!(renderer.glyphs.rasterizations, glyphs);
        assert_eq!(
            serde_json::to_value(&app.application.editor.project_for_native_test()).unwrap(),
            authored
        );
        let mut capture = ReadbackTicket::capture(&gpu, output.clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let image = capture.try_finish(&gpu).unwrap().unwrap();
        let mut encoder = png::Encoder::new(
            std::fs::File::create("/private/tmp/eldiron-native-creator-startup.png").unwrap(),
            1200,
            800,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(image.pixels())
            .unwrap();
        // Surface reconstruction preserves the assembled application and native
        // UI source owners. The generic host calls this only after replacement.
        let mut replacement = ImageRegistry::new(gpu.clone());
        app.gpu_recreated(&mut host, &mut replacement).unwrap();
        let rebuilt = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &rebuilt,
                &replacement,
                &GpuImage::new(&gpu, ImageDescriptor::rgba(1200, 800))
                    .unwrap()
                    .view,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(&app.application.editor.project_for_native_test()).unwrap(),
            authored
        );
        // Exercise the real viewport handoff in the same assembled application,
        // rather than another isolated profile renderer/fixture suite.
        let view = app
            .application
            .editor
            .begin_native_profile_for_test(&mut app.session.ui, &mut app.session.context);
        let initial_grid = app.application.editor.native_profile_grid_for_test();
        app.update(&mut host, Duration::from_millis(16));
        let profile = app.frame(&mut host, &mut replacement).unwrap();
        assert!(app.application.pending.as_ref().unwrap().profile_for_test());
        assert!(profile.receipts().next().is_some());
        let click = TheEvent::RenderViewClicked(view.clone(), Vec2::new(115, 14));
        assert!(app.application.editor.handle_native_display_input(
            &click,
            &mut app.session.ui,
            &mut app.session.context,
        ));
        assert_eq!(
            app.application.editor.native_profile_grid_for_test(),
            initial_grid,
            "preparing a root cannot activate profile input"
        );
        let before_profile = gpu.transfers().readback_bytes;
        renderer
            .render(
                &profile,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert_eq!(gpu.transfers().readback_bytes, before_profile);
        assert!(app.application.editor.handle_native_display_input(
            &click,
            &mut app.session.ui,
            &mut app.session.context,
        ));
        assert_eq!(app.application.editor.native_profile_grid_for_test(), 2.);
        assert_eq!(
            app.session.context.ui.focus.as_ref().unwrap().uuid,
            view.uuid
        );
        // The root acknowledges the current edited frame before keyboard cancel.
        let edited = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &edited,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert!(app.application.editor.handle_native_display_input(
            &TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape)),
            &mut app.session.ui,
            &mut app.session.context,
        ));
        assert!(!app.application.editor.has_native_profile_for_test());
        app.application
            .editor
            .begin_native_profile_for_test(&mut app.session.ui, &mut app.session.context);
        let commit_frame = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &commit_frame,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        let objects = app
            .application
            .editor
            .native_profile_geometry_count_for_test();
        assert!(app.application.editor.handle_native_display_input(
            &TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Return)),
            &mut app.session.ui,
            &mut app.session.context,
        ));
        assert_eq!(
            app.application
                .editor
                .native_profile_geometry_count_for_test(),
            objects + 1,
            "Return applies the current profile in the same document transaction"
        );
        assert!(!app.application.editor.has_native_profile_for_test());
        let world_view = app
            .application
            .editor
            .begin_native_world_for_test(&mut app.session.ui, &mut app.session.context);
        // Consume the real build results produced by Return before accepting
        // a new world pick. This is the application's normal update path.
        for _ in 0..64 {
            app.update(&mut host, Duration::from_millis(16));
            if !crate::editor::SCENEMANAGER
                .read()
                .unwrap()
                .has_pending_geometry()
            {
                break;
            }
        }
        assert!(
            !crate::editor::SCENEMANAGER
                .read()
                .unwrap()
                .has_pending_geometry()
        );
        assert_eq!(
            crate::editor::TOOLLIST
                .read()
                .unwrap()
                .current_game_tool_command_id(),
            Some("tool.vertex")
        );
        let world = app.frame(&mut host, &mut replacement).unwrap();
        assert!(app.application.pending.as_ref().unwrap().world_for_test());
        assert!(
            !app.application
                .editor
                .native_world_has_cpu_storage_for_test()
        );
        let before_world = gpu.transfers().readback_bytes;
        let grid = app.application.editor.native_profile_grid_for_test();
        let header = TheEvent::RenderViewClicked(world_view.clone(), Vec2::new(235, 5));
        assert!(app.application.editor.handle_native_world_input(
            &header,
            &mut app.session.ui,
            &mut app.session.context,
        ));
        assert_eq!(app.application.editor.native_profile_grid_for_test(), grid);
        renderer
            .render(
                &world,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert_eq!(gpu.transfers().readback_bytes, before_world);
        assert!(app.application.editor.handle_native_world_input(
            &header,
            &mut app.session.ui,
            &mut app.session.context,
        ));
        assert_eq!(app.application.editor.native_profile_grid_for_test(), 4.);
        app.update(&mut host, Duration::from_millis(16));
        let updated_world = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &updated_world,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        let vertices = app.application.editor.native_world_vertices_for_test();
        app.session.ui.shift = true;
        app.session.context.ui.send(TheEvent::RenderViewClicked(
            world_view.clone(),
            Vec2::new(250, 240),
        ));
        app.update(&mut host, Duration::from_millis(16));
        assert_eq!(
            app.application.editor.native_world_vertices_for_test(),
            vertices + 1,
            "an acknowledged outside HUD hit reaches the actual Vertex tool once"
        );
        app.session.ui.shift = false;
        app.session.context.ui.send(TheEvent::RenderViewUp(
            world_view.clone(),
            Vec2::new(250, 240),
        ));
        // Extend this existing assembled check through the real camera toolbar.
        // This is one integration checkpoint, with no separate fixture matrix.
        for camera_index in [2, 3, 4, 1, 0] {
            app.session.context.ui.send(TheEvent::IndexChanged(
                TheId::named("Editor View Switch"),
                camera_index,
            ));
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                app.update(&mut host, Duration::from_millis(16));
                let expected = match camera_index {
                    0 => rusterix::MapCamera::TwoD,
                    1 => rusterix::MapCamera::TwoDIso,
                    4 => rusterix::MapCamera::ThreeDFirstPerson,
                    _ => rusterix::MapCamera::ThreeDIso,
                };
                let ready = camera_index == 1 || {
                    let builds = crate::editor::SCENEMANAGER.read().unwrap();
                    !builds.has_pending_geometry()
                        && builds.source_matches(
                            app.application.editor.project_for_native_test().regions[0]
                                .map
                                .id,
                            expected,
                        )
                };
                if ready {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "camera {camera_index} source never became ready"
                );
                // Editor work is scheduled from elapsed host time, not the
                // synthetic Duration argument. Yield while that real job runs.
                std::thread::sleep(Duration::from_millis(2));
            }
            let world = app.frame(&mut host, &mut replacement).unwrap();
            assert!(
                app.application.pending.as_ref().unwrap().world_for_test(),
                "camera {camera_index} must use the native world"
            );
            assert!(
                !app.application
                    .editor
                    .native_world_has_cpu_storage_for_test()
            );
            let dim = *app
                .session
                .ui
                .get_render_view(&world_view.name)
                .unwrap()
                .dim();
            let cube_point = Vec2::new(dim.width - 52, 102);
            let cube_click = TheEvent::RenderViewClicked(world_view.clone(), cube_point);
            let pose = crate::editor::EDITCAMERA
                .read()
                .unwrap()
                .orbit_camera
                .as_scenevm_camera();
            if camera_index == 2 {
                assert!(app.application.editor.handle_native_world_input(
                    &cube_click,
                    &mut app.session.ui,
                    &mut app.session.context
                ));
                assert_eq!(
                    crate::editor::EDITCAMERA
                        .read()
                        .unwrap()
                        .orbit_camera
                        .as_scenevm_camera()
                        .forward,
                    pose.forward,
                    "a prepared cube cannot change the camera before root acknowledgement"
                );
            }
            let before = gpu.transfers().readback_bytes;
            renderer
                .render(
                    &world,
                    &replacement,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(&mut host);
            assert_eq!(gpu.transfers().readback_bytes, before);
            if camera_index == 2 {
                assert!(app.application.editor.handle_native_world_input(
                    &cube_click,
                    &mut app.session.ui,
                    &mut app.session.context
                ));
                assert!(app.application.editor.handle_native_world_input(
                    &TheEvent::RenderViewUp(world_view.clone(), cube_point),
                    &mut app.session.ui,
                    &mut app.session.context
                ));
                assert_ne!(
                    crate::editor::EDITCAMERA
                        .read()
                        .unwrap()
                        .orbit_camera
                        .as_scenevm_camera()
                        .forward,
                    pose.forward,
                    "an acknowledged cube face snaps the actual authoring camera"
                );
            }
            // A camera change must be rendered and acknowledged before picking.
            app.update(&mut host, Duration::from_millis(16));
            let world = app.frame(&mut host, &mut replacement).unwrap();
            renderer
                .render(
                    &world,
                    &replacement,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(&mut host);
            let hover = TheEvent::RenderViewHoverChanged(
                world_view.clone(),
                Vec2::new(dim.width / 2, dim.height / 2),
            );
            assert!(
                !app.application.editor.handle_native_world_input(
                    &hover,
                    &mut app.session.ui,
                    &mut app.session.context
                ),
                "an acknowledged native camera {camera_index} query reaches the existing tool dispatcher: {}",
                app.application.editor.native_world_debug_for_test()
            );
            if matches!(camera_index, 2 | 3 | 4) {
                assert!(app.application.editor.native_world_has_ray_for_test());
            }
            if camera_index == 1 {
                assert!(
                    world
                        .ops
                        .iter()
                        .any(|op| matches!(op, theframework::paint::PaintOp::SampledLine { .. })),
                    "iso guides are real GPU commands in the root"
                );
                let vertices = app.application.editor.native_world_vertices_for_test();
                app.session.ui.shift = true;
                let at = Vec2::new(dim.width / 2 + 80, dim.height / 2 + 40);
                app.session
                    .context
                    .ui
                    .send(TheEvent::RenderViewClicked(world_view.clone(), at));
                app.update(&mut host, Duration::from_millis(16));
                assert_eq!(
                    app.application.editor.native_world_vertices_for_test(),
                    vertices + 1,
                    "the native isometric pick reaches the actual Vertex tool once"
                );
                app.session.ui.shift = false;
                app.session
                    .context
                    .ui
                    .send(TheEvent::RenderViewUp(world_view.clone(), at));
            }
        }
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.terrain".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        let terrain = app.frame(&mut host, &mut replacement).unwrap();
        assert!(app.application.pending.as_ref().unwrap().world_for_test());
        let radius = app.application.editor.native_terrain_radius_for_test();
        let increase = TheEvent::RenderViewClicked(world_view.clone(), Vec2::new(215, 80));
        assert!(app.application.editor.handle_native_world_input(
            &increase,
            &mut app.session.ui,
            &mut app.session.context
        ));
        assert_eq!(
            app.application.editor.native_terrain_radius_for_test(),
            radius,
            "preparing terrain controls cannot activate them"
        );
        renderer
            .render(
                &terrain,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert!(app.application.editor.handle_native_world_input(
            &increase,
            &mut app.session.ui,
            &mut app.session.context
        ));
        assert_eq!(
            app.application.editor.native_terrain_radius_for_test(),
            radius + 0.5
        );
        app.session.context.ui.send(TheEvent::RenderViewUp(
            world_view.clone(),
            Vec2::new(215, 80),
        ));
        app.update(&mut host, Duration::from_millis(16));
        let terrain = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &terrain,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        let original = app.application.editor.native_terrain_points_for_test();
        let dim = *app
            .session
            .ui
            .get_render_view(&world_view.name)
            .unwrap()
            .dim();
        let sculpt = Vec2::new(dim.width / 2 + 80, dim.height / 2 + 40);
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(world_view.clone(), sculpt));
        app.update(&mut host, Duration::from_millis(16));
        assert_ne!(
            app.application.editor.native_terrain_points_for_test(),
            original,
            "the actual terrain brush changes authored sculpt data"
        );
        // Material publication on an initially unpainted map changes asset
        // revisions during the stroke; the draft must survive that next frame.
        let sculpt_frame = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &sculpt_frame,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert_ne!(
            app.application.editor.native_terrain_points_for_test(),
            original,
            "asset refresh cannot abandon the owned sculpt draft"
        );
        assert!(app.application.editor.handle_native_world_input(
            &TheEvent::KeyDown(TheValue::Char(']')),
            &mut app.session.ui,
            &mut app.session.context
        ));
        assert_eq!(
            app.application.editor.native_terrain_radius_for_test(),
            radius + 1.,
            "acknowledged terrain shortcuts remain usable during streamed source work"
        );
        app.input(&InputEvent::Focus(false), &mut host);
        assert_eq!(
            app.application.editor.native_terrain_points_for_test(),
            original,
            "focus loss cancels the owned terrain draft"
        );
        app.update(&mut host, Duration::from_millis(100));
        assert_eq!(
            app.application.editor.native_terrain_points_for_test(),
            original,
            "a cancelled terrain stroke cannot continue sculpting"
        );
        app.input(&InputEvent::Focus(true), &mut host);
        app.application.editor.native_rect_source_for_test();
        app.session.context.ui.send(TheEvent::IndexChanged(
            TheId::named("Editor View Switch"),
            1,
        ));
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.rect".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        let rect = app.frame(&mut host, &mut replacement).unwrap();
        assert!(app.application.pending.as_ref().unwrap().world_for_test());
        let compose = TheEvent::RenderViewClicked(world_view.clone(), Vec2::new(120, 35));
        assert!(app.application.editor.handle_native_world_input(
            &compose,
            &mut app.session.ui,
            &mut app.session.context
        ));
        assert!(
            !app.application.editor.native_rect_state_for_test().0,
            "unpresented Rect controls cannot author state"
        );
        renderer
            .render(
                &rect,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        let before = app.application.editor.native_rect_state_for_test();
        let width = app
            .session
            .ui
            .get_render_view(&world_view.name)
            .unwrap()
            .dim()
            .width;
        let rotate = if width >= 490 {
            Vec2::new(360, 35)
        } else {
            Vec2::new(180, 60)
        };
        assert!(app.application.editor.handle_native_world_input(
            &TheEvent::RenderViewClicked(world_view.clone(), rotate),
            &mut app.session.ui,
            &mut app.session.context,
        ));
        app.session.context.ui.send(TheEvent::RenderViewDragged(
            world_view.clone(),
            Vec2::new(480, 240),
        ));
        app.session.context.ui.send(TheEvent::RenderViewUp(
            world_view.clone(),
            Vec2::new(480, 240),
        ));
        app.update(&mut host, Duration::from_millis(16));
        let rotated = app.application.editor.native_rect_state_for_test();
        assert_eq!(
            rotated.1,
            (before.1 + 90.) % 360.,
            "the acknowledged Rotate control changes the real brush"
        );
        assert_eq!(
            rotated.2, before.2,
            "a control drag cannot become a paint stroke"
        );
        let rect = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &rect,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert!(app.application.editor.handle_native_world_input(
            &compose,
            &mut app.session.ui,
            &mut app.session.context
        ));
        assert!(app.application.editor.native_rect_state_for_test().0);
        app.session.context.ui.send(TheEvent::RenderViewUp(
            world_view.clone(),
            Vec2::new(120, 35),
        ));
        app.update(&mut host, Duration::from_millis(16));
        let rect = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &rect,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.session.context.ui.send(TheEvent::RenderViewClicked(
            world_view.clone(),
            Vec2::new(40, 35),
        ));
        app.session.context.ui.send(TheEvent::RenderViewUp(
            world_view.clone(),
            Vec2::new(40, 35),
        ));
        app.update(&mut host, Duration::from_millis(16));
        let rect = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &rect,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        let dim = *app
            .session
            .ui
            .get_render_view(&world_view.name)
            .unwrap()
            .dim();
        let at = Vec2::new(dim.width / 2 + 100, dim.height / 2 + 90);
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewHoverChanged(world_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        let rect = app.frame(&mut host, &mut replacement).unwrap();
        assert!(
            rect.ops
                .iter()
                .any(|op| matches!(op, theframework::paint::PaintOp::SpriteImage { .. })),
            "actual Rect hover emits a source sprite GPU blit"
        );
        renderer
            .render(
                &rect,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(world_view.clone(), at));
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewUp(world_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        assert!(
            app.application.editor.native_rect_state_for_test().2 > before.2,
            "the actual Rect controller commits authored placement"
        );
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.iso_paint".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        // Deliver activation's queued source/picker refresh before acknowledging
        // its controls, then exercise input through the actual editor event loop.
        app.update(&mut host, Duration::from_millis(16));
        let paint = app.frame(&mut host, &mut replacement).unwrap();
        assert!(
            app.application.pending.as_ref().unwrap().world_for_test(),
            "isometric Paint uses the native world owner"
        );
        let dim = *app
            .session
            .ui
            .get_render_view(&world_view.name)
            .unwrap()
            .dim();
        assert!(
            dim.width >= 340 && dim.height >= 260,
            "Paint panel check requires its visible layout: {:?}",
            dim
        );
        let inner = 504.min(dim.width - 16) - 16;
        let minus =
            TheEvent::RenderViewClicked(world_view.clone(), Vec2::new(16 + inner - 58 + 12, 118));
        let original = app.application.editor.native_paint_state_for_test();
        assert!(app.application.editor.handle_native_world_input(
            &minus,
            &mut app.session.ui,
            &mut app.session.context
        ));
        assert_eq!(
            app.application.editor.native_paint_state_for_test().0,
            original.0
        );
        renderer
            .render(
                &paint,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.session.context.ui.send(minus.clone());
        app.session.context.ui.send(TheEvent::RenderViewUp(
            world_view.clone(),
            Vec2::new(16 + inner - 58 + 12, 118),
        ));
        app.update(&mut host, Duration::from_millis(16));
        assert!(
            (app.application.editor.native_paint_state_for_test().0
                - (original.0 - 0.05).clamp(0., 1.))
            .abs()
                < 0.00001,
            "Paint controls reach the full editor dispatcher"
        );
        let paint = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &paint,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        let at = app
            .application
            .editor
            .native_paint_point_for_test((dim.width, dim.height));
        assert!(
            at.y > 260,
            "the authored ground point is outside the Paint panel"
        );
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewHoverChanged(world_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        let paint = app.frame(&mut host, &mut replacement).unwrap();
        assert!(
            paint
                .ops
                .iter()
                .any(|op| matches!(op, theframework::paint::PaintOp::PixelRects { .. })),
            "Paint feedback emits GPU logical-pixel spans"
        );
        renderer
            .render(
                &paint,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(world_view.clone(), at));
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewUp(world_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        let authored = app.application.editor.native_paint_state_for_test();
        assert!(
            authored.1 > original.1,
            "native input commits an actual Paint stroke"
        );
        assert_ne!(
            authored.2, original.2,
            "the editable paint source is published before frame preparation"
        );
        let paint = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &paint,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        // Continue the same assembled checkpoint through actual 3D Paint.
        // Give the actual object space outside Paint's original overlay panel.
        host.resize((1600, 1000), 1.);
        app.resize(&mut host);
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1600, 1000)).unwrap();
        app.session.context.ui.send(TheEvent::IndexChanged(
            TheId::named("Editor View Switch"),
            3,
        ));
        app.session.context.ui.send(TheEvent::IndexChanged(
            TheId::named(crate::tools::paint_picker::MODE),
            0,
        ));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.update(&mut host, Duration::from_millis(16));
            let ready = {
                let builds = crate::editor::SCENEMANAGER.read().unwrap();
                !builds.has_pending_geometry()
                    && builds.source_matches(
                        app.application.editor.project_for_native_test().regions[0]
                            .map
                            .id,
                        rusterix::MapCamera::ThreeDIso,
                    )
            };
            if ready {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "3D Paint source never became ready"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        app.update(&mut host, Duration::from_millis(16));
        let before = gpu.transfers();
        let paint = app.frame(&mut host, &mut replacement).unwrap();
        assert!(
            app.application.pending.as_ref().unwrap().world_for_test(),
            "3D Paint uses its native scene owner"
        );
        renderer
            .render(
                &paint,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert_eq!(
            gpu.transfers().readback_bytes,
            before.readback_bytes,
            "3D Paint presentation cannot capture surface data"
        );
        let target = app.application.editor.native_paint_3d_target_for_test();
        if target.is_none() {
            let mut capture = ReadbackTicket::capture(&gpu, output.clone()).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .unwrap();
            let pixels = capture.try_finish(&gpu).unwrap().unwrap();
            let mut encoder = png::Encoder::new(
                std::fs::File::create("/private/tmp/eldiron-native-paint3d-diagnostic.png")
                    .unwrap(),
                1600,
                1000,
            );
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(pixels.pixels())
                .unwrap();
        }
        let at = target.unwrap_or_else(|| {
            panic!(
                "real 3D paint surface: {}",
                app.application.editor.native_paint_3d_debug_for_test()
            )
        });
        let original = app.application.editor.native_paint_state_for_test();
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewHoverChanged(world_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        assert_eq!(
            gpu.transfers().readback_bytes,
            before.readback_bytes,
            "hover is a read-only CPU query"
        );
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(world_view.clone(), at));
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewUp(world_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        assert!(
            app.application.editor.native_paint_state_for_test().1 > original.1,
            "the actual 3D Paint controller commits a stroke"
        );
        assert!(
            app.application.editor.native_paint_has_3d_data_for_test(),
            "authored points retain the owning scene's paint UV, face and normal"
        );
        let captured = gpu.transfers();
        assert_eq!(
            captured.capture_requests,
            before.capture_requests + 1,
            "one explicit down-request captures authoring surface data"
        );
        assert!(captured.readback_bytes > before.readback_bytes);
        let paint = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &paint,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        assert_eq!(
            gpu.transfers().readback_bytes,
            captured.readback_bytes,
            "rendering committed paint never recaptures its surface"
        );
        // A moving camera revokes the frozen down-camera's draft rather than
        // reinterpreting its captured screen-space coordinates.
        let at = app
            .application
            .editor
            .native_paint_3d_target_for_test()
            .unwrap();
        let original = app.application.editor.native_paint_state_for_test().1;
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(world_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        assert!(app.application.editor.native_paint_state_for_test().1 > original);
        crate::editor::EDITCAMERA
            .write()
            .unwrap()
            .iso_camera
            .zoom(1.);
        let paint = app.frame(&mut host, &mut replacement).unwrap();
        assert_eq!(
            app.application.editor.native_paint_state_for_test().1,
            original,
            "camera motion cancels the original Paint draft"
        );
        renderer
            .render(
                &paint,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        // Adopt the actual Prefabs dock, viewport and its dock-owned effects
        // transaction in this same assembled Creator checkpoint.
        let regions =
            serde_json::to_value(&app.application.editor.project_for_native_test().regions)
                .unwrap();
        let (prefab_view, prefab) = app
            .application
            .editor
            .begin_native_prefab_for_test(&mut app.session.ui, &mut app.session.context);
        let mut present_prefab = |app: &mut UiApplicationAdapter<NativeCreatorApplication>,
                                  host: &mut HostContext,
                                  registry: &mut ImageRegistry| {
            app.update(host, Duration::from_millis(16));
            let frame = app.frame(host, registry).unwrap();
            assert!(
                app.application.pending.as_ref().unwrap().world_for_test(),
                "Prefab uses its own native SceneView"
            );
            renderer
                .render(
                    &frame,
                    registry,
                    &output.view,
                    output.descriptor.format,
                    (1600, 1000),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(host);
        };
        let mut settle_prefab = |app: &mut UiApplicationAdapter<NativeCreatorApplication>,
                                 host: &mut HostContext,
                                 registry: &mut ImageRegistry| {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                present_prefab(app, host, registry);
                if app.application.editor.native_world_ready_for_test() {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "Prefab builder: {}",
                    app.application.editor.native_world_debug_for_test()
                );
                std::thread::sleep(Duration::from_millis(2));
            }
        };
        settle_prefab(&mut app, &mut host, &mut replacement);
        let at = app
            .application
            .editor
            .native_prefab_handle_for_test()
            .expect("actual Prefab effect move handle");
        let original = app.application.editor.native_prefab_effect_for_test(prefab);
        let origin = app
            .application
            .editor
            .native_prefab_projection_for_test()
            .unwrap();
        let before = gpu.transfers();
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(prefab_view.clone(), at));
        app.session.context.ui.send(TheEvent::RenderViewDragged(
            prefab_view.clone(),
            at + Vec2::new(24, 12),
        ));
        app.update(&mut host, Duration::from_millis(16));
        assert_ne!(
            app.application
                .editor
                .native_prefab_effect_for_test(prefab)
                .0,
            original.0,
            "actual effect drag changes its attachment"
        );
        let moved = app
            .application
            .editor
            .native_prefab_projection_for_test()
            .unwrap();
        assert!(
            (moved - origin - Vec2::new(24., 12.)).magnitude() < 0.1,
            "hierarchy/scaled effect movement must follow the exact pointer delta"
        );
        app.application
            .editor
            .rename_native_prefab_for_test(prefab, "Concurrent Prefab name");
        settle_prefab(&mut app, &mut host, &mut replacement);
        app.input(&InputEvent::Focus(false), &mut host);
        assert_eq!(
            app.application.editor.native_prefab_effect_for_test(prefab),
            (original.0, "Concurrent Prefab name".into()),
            "focus cancellation restores only the owned position"
        );
        app.input(&InputEvent::Focus(true), &mut host);
        settle_prefab(&mut app, &mut host, &mut replacement);
        let at = app
            .application
            .editor
            .native_prefab_handle_for_test()
            .unwrap();
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(prefab_view.clone(), at));
        app.session.context.ui.send(TheEvent::RenderViewDragged(
            prefab_view.clone(),
            at + Vec2::new(20, 8),
        ));
        app.session.context.ui.send(TheEvent::RenderViewUp(
            prefab_view.clone(),
            at + Vec2::new(20, 8),
        ));
        app.update(&mut host, Duration::from_millis(16));
        let committed = app.application.editor.native_prefab_effect_for_test(prefab);
        assert_ne!(
            committed.0, original.0,
            "release commits the effect gesture"
        );
        app.input(&InputEvent::Focus(false), &mut host);
        assert_eq!(
            app.application.editor.native_prefab_effect_for_test(prefab),
            committed,
            "released effects cannot be cancelled again"
        );
        app.input(&InputEvent::Focus(true), &mut host);
        settle_prefab(&mut app, &mut host, &mut replacement);
        let deadline = Instant::now() + Duration::from_secs(2);
        while app.application.editor.native_prefab_particles_for_test() == 0 {
            assert!(
                Instant::now() < deadline,
                "the owning Prefab scene must advance its particles"
            );
            std::thread::sleep(Duration::from_millis(50));
            settle_prefab(&mut app, &mut host, &mut replacement);
        }
        for projection in [0, 1, 2] {
            app.session.context.ui.send(TheEvent::IndexChanged(
                TheId::named("Editor View Switch"),
                projection,
            ));
            settle_prefab(&mut app, &mut host, &mut replacement);
            assert!(
                !app.application
                    .editor
                    .native_world_has_cpu_storage_for_test(),
                "native Prefab scene targets have no CPU display storage"
            );
        }
        assert_eq!(
            serde_json::to_value(&app.application.editor.project_for_native_test().regions)
                .unwrap(),
            regions,
            "Prefab authoring cannot change region content"
        );
        assert_eq!(
            gpu.transfers().readback_bytes,
            before.readback_bytes,
            "Prefab rendering and effect queries perform no color readback"
        );
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.iso_paint".into()),
        ));
        app.session.context.ui.send(TheEvent::IndexChanged(
            TheId::named(crate::tools::paint_picker::MODE),
            0,
        ));
        settle_prefab(&mut app, &mut host, &mut replacement);
        let at = app
            .application
            .editor
            .native_paint_3d_target_for_test()
            .expect("real visible Prefab paint surface");
        let original = app.application.editor.native_paint_state_for_test();
        let before = gpu.transfers();
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(prefab_view.clone(), at));
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewUp(prefab_view.clone(), at));
        app.update(&mut host, Duration::from_millis(16));
        assert!(
            app.application.editor.native_paint_state_for_test().1 > original.1,
            "Prefab Paint commits into the owning asset layer"
        );
        assert!(app.application.editor.native_paint_has_3d_data_for_test());
        let authored = gpu.transfers();
        assert_eq!(
            authored.capture_requests,
            before.capture_requests + 1,
            "Prefab projected authoring explicitly requests one surface capture"
        );
        settle_prefab(&mut app, &mut host, &mut replacement);
        assert_eq!(
            gpu.transfers().readback_bytes,
            authored.readback_bytes,
            "Prefab paint display never recaptures its surface"
        );
        assert_eq!(
            serde_json::to_value(&app.application.editor.project_for_native_test().regions)
                .unwrap(),
            regions,
            "Prefab Paint leaves region data untouched"
        );
        drop(settle_prefab);
        drop(present_prefab);
        let mut capture = ReadbackTicket::capture(&gpu, output.clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let image = capture.try_finish(&gpu).unwrap().unwrap();
        let mut encoder = png::Encoder::new(
            std::fs::File::create("/private/tmp/eldiron-native-creator-prefab.png").unwrap(),
            1600,
            1000,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(image.pixels())
            .unwrap();
        // Check the new sampler within this assembled checkpoint. The reference
        // uses the existing independent CPU DDA, including duplicate blending.
        use theframework::paint::{DisplayCompositing, PaintOp};
        let mut reference = TheRGBABuffer::new(TheDim::sized(8, 8));
        for pixel in reference.pixels_mut().chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        let mut guide = PaintList::default();
        guide.ops.push(PaintOp::DisplayCompositing(
            DisplayCompositing::LegacyOpaque,
        ));
        for (a, b, color) in [
            ([1.1, 1.1], [1.3, 1.3], [255, 255, 255, 128]),
            ([7.6, 1.2], [-1.2, 7.6], [80, 170, 220, 150]),
        ] {
            rusterix::isometric::line(
                &mut reference,
                Vec2::new(a[0], a[1]),
                Vec2::new(b[0], b[1]),
                color,
            );
            guide.ops.push(PaintOp::SampledLine {
                from: a,
                to: b,
                color: color.map(|v| v as f32 / 255.),
            });
        }
        let mut tile = rusterix::Tile::default();
        let data = (0..12)
            .flat_map(|i| {
                [
                    (i * 17) as u8,
                    (220 - i * 11) as u8,
                    (30 + i * 13) as u8,
                    [0, 80, 160, 255][i % 4],
                ]
            })
            .collect::<Vec<_>>();
        tile.textures
            .push(rusterix::Texture::new(data.clone(), 4, 3));
        tile.isometric = Some(rusterix::isometric::SpriteLayout {
            pixels_per_unit: 1.,
            directions: 1,
            frames_per_direction: 1,
            ..Default::default()
        });
        let source = theframework_core::ImageAsset::rgba(4, 3, data).unwrap();
        let image = replacement.upload_linear(&source).unwrap();
        // Fractional downsampling differs from resizing to the rounded extent.
        // The second case clips a partially transparent upscaled sprite.
        for (origin, scale) in [([1.4, 1.4], 0.6), ([-0.6, 2.4], 1.6)] {
            rusterix::isometric::draw_tile_preview(
                &mut reference,
                &tile,
                rusterix::isometric::Projection {
                    origin: Vec2::new(origin[0], origin[1]),
                    unit: scale,
                },
                Vec2::zero(),
                0.,
            );
            guide.ops.push(PaintOp::SpriteImage {
                image,
                origin,
                scale: [scale; 2],
                tint: [1., 1., 1., 0.65],
            });
        }
        guide.validate().unwrap();
        assert!(guide.image_handles().any(|handle| handle == image));
        for density in [1., 1.5, 2.] {
            let extent = (8. * density) as u32;
            let pixels = GpuImage::new(&gpu, ImageDescriptor::rgba(extent, extent)).unwrap();
            renderer
                .render(
                    &guide,
                    &replacement,
                    &pixels.view,
                    pixels.descriptor.format,
                    (extent, extent),
                    density,
                    [0., 0., 0., 1.],
                )
                .unwrap();
            let mut capture = ReadbackTicket::capture(&gpu, pixels).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .unwrap();
            let pixels = capture.try_finish(&gpu).unwrap().unwrap();
            for y in 0..extent {
                for x in 0..extent {
                    let expected = (((y as f32 + 0.5) / density) as usize * 8
                        + ((x as f32 + 0.5) / density) as usize)
                        * 4;
                    let actual = (y as usize * extent as usize + x as usize) * 4;
                    for channel in 0..4 {
                        assert!(
                            (pixels.pixels()[actual + channel] as i16
                                - reference.pixels()[expected + channel] as i16)
                                .abs()
                                <= 1,
                            "DDA/nearest logical pixel mismatch at density={density}, ({x},{y}), channel={channel}: got={} expected={}",
                            pixels.pixels()[actual + channel],
                            reference.pixels()[expected + channel]
                        );
                    }
                }
            }
        }
    }
}
