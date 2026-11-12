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
        self.editor.invalidate_native_views(&mut ui.ui);
        Ok(())
    }
    fn suspended(&mut self, ui: &mut UiSession) {
        self.pending = None;
        ui.input(&InputEvent::Focus(false));
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
        for camera_index in [2, 3, 4] {
            app.session.context.ui.send(TheEvent::IndexChanged(
                TheId::named("Editor View Switch"),
                camera_index,
            ));
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
                "an acknowledged native 3D query reaches the existing tool dispatcher"
            );
            assert!(app.application.editor.native_world_has_ray_for_test());
        }
    }
}
