//! Transitional Creator application on the common GPU UI/host contract.
//! Geometry and appearance outputs use the injected device. Legacy Editor owns the
//! unconverted world/tool/history orchestration; this is not M5/M7 retirement.
use crate::editor::{Editor, native_views::NativeViewTokens};
use renderkit::{RenderError, Result};
use web_time::{Duration, Instant};
use theframework::{paint::PaintList, platform::*, prelude::*, render_wgpu::UiFrameImages};

#[path = "native_documents.rs"]
mod documents;
#[cfg(test)]
#[path = "native_visibility_check.rs"]
mod visibility_check;

pub struct NativeCreatorApplication {
    editor: Editor,
    args: Vec<String>,
    pending: Option<NativeViewTokens>,
    smoke: bool,
    deadline: Option<Instant>,
    presentations: u64,
    start_services: bool,
    startup: Option<Instant>,
    idle_probe: Option<IdleProbe>,
}
struct IdleProbe {
    duration: Duration,
    started: Option<(Instant, u64)>,
}
impl NativeCreatorApplication {
    #[cfg(test)]
    pub(crate) fn for_release_benchmark(args: Vec<String>) -> Self {
        let mut application = Self::new(args, false);
        application.start_services = false;
        application
    }
    #[cfg(test)]
    pub(crate) fn release_benchmark_editor(&mut self) -> &mut Editor {
        &mut self.editor
    }
    pub fn new(args: Vec<String>, smoke: bool) -> Self {
        let startup = std::env::var_os("ELDIRON_PROFILE_STARTUP")
            .is_some()
            .then(Instant::now);
        let mut profile = crate::misc::FrameProfile::new();
        let mut editor = Editor::new();
        editor.enable_native_views();
        profile.lap("native_editor_construct");
        let idle_probe = smoke
            .then(|| std::env::var("ELDIRON_NATIVE_IDLE_PROBE_MS").ok())
            .flatten()
            .and_then(|value| value.parse::<u64>().ok())
            .map(|ms| IdleProbe {
                duration: Duration::from_millis(ms.clamp(1000, 60_000)),
                started: None,
            });
        Self {
            editor,
            args,
            pending: None,
            smoke,
            deadline: None,
            presentations: 0,
            start_services: true,
            startup,
            idle_probe,
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
    fn initialize(&mut self, ui: &mut UiSession, images: &mut UiFrameImages<'_>) -> Result<()> {
        let mut profile = crate::misc::FrameProfile::new();
        self.editor.bind_native_wake(ui.wake.clone());
        self.editor
            .bind_native_reference_context(images.registry.context().clone());
        // Preserve Creator's existing root/layout bootstrap before docks are
        // constructed; some source-backed docks inspect their initial bounds.
        ui.ui.canvas.root = true;
        ui.ui.canvas.set_dim(
            TheDim::sized(ui.context.width as i32, ui.context.height as i32),
            &mut ui.context,
        );
        self.editor.init_ui(&mut ui.ui, &mut ui.context);
        profile.lap("native_init_ui");
        ui.ui.prepare_paint_layout(&mut ui.context);
        profile.lap("native_initial_layout");
        // Fontique resolves native fallback for Parley/Swash. The raster
        // system-font bootstrap belongs to the reference host, not GPU startup.
        if self.start_services {
            self.editor.init(&mut ui.context);
        }
        profile.lap("native_services");
        self.editor
            .set_cmd_line_args(std::mem::take(&mut self.args), &mut ui.context);
        profile.lap("native_arguments");
        if self.smoke {
            self.deadline = Some(Instant::now() + Duration::from_secs(10));
        }
        Ok(())
    }

    fn raw_input(&mut self, event: &InputEvent, ui: &mut UiSession) -> Option<InputResponse> {
        if let InputEvent::ClipboardImage { request, result } = event {
            return Some(self.editor.native_image_clipboard(*request, result, ui));
        }
        if matches!(event, InputEvent::PointerButton { pressed: true, .. } | InputEvent::Key { pressed: true, .. }) { self.editor.native_runtime_audio_gesture(); }
        ui.ui.prepare_paint_layout(&mut ui.context);
        self.editor.track_native_world_navigation(event, ui);
        let response = self.editor.native_game_input_raw(event, ui)
            .or_else(|| self.editor.native_play_raw_input(event, ui));
        if response.is_some() && matches!(event, InputEvent::Modifiers(_) | InputEvent::Focus(_)) {
            ui.input(event);
        }
        // Declined application shortcuts still reach Creator's document menu.
        if response.as_ref().is_some_and(|response| !response.consumed)
            && (ui.ui.logo || ui.ui.ctrl)
            && matches!(
                event,
                InputEvent::Key {
                    pressed: true,
                    repeat: false,
                    ..
                }
            )
        {
            return None;
        }
        response
    }
    fn application_text_input(&mut self, ui: &mut UiSession) -> Option<TextInputState> {
        self.editor.native_play_text_input(&mut ui.ui, &ui.context)
    }
    fn application_text_snapshot(
        &mut self,
        ui: &mut UiSession,
    ) -> Option<theframework::text_edit::NativeTextSnapshot> {
        self.editor
            .native_play_text_snapshot(&mut ui.ui, &ui.context)
    }
    fn application_text_query(
        &mut self,
        ui: &mut UiSession,
        request: &theframework::text_edit::NativeTextRequest,
    ) -> Option<theframework::text_edit::NativeTextAnswer> {
        self.editor
            .native_play_text_query(&mut ui.ui, &ui.context, request)
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
                let handled = self.editor.native_world_relative_motion(*delta, ui);
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
    fn update(&mut self, ui: &mut UiSession, elapsed: Duration) -> FrameRequest {
        if let Some(probe) = &self.idle_probe
            && let Some((start, presentations)) = probe.started
            && start.elapsed() >= probe.duration
        {
            eprintln!(
                "Creator native idle probe: {} additional presentations in {:.2}s",
                self.presentations - presentations,
                start.elapsed().as_secs_f64()
            );
            ui.requests.push(HostRequest::Exit);
            return FrameRequest::idle();
        }
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
        if let Err(error) = self.editor.advance_native_runtime(elapsed) {
            ui.context.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
        }
        let mut redraw = ui.ui.update(&mut ui.context);
        redraw |= self.editor.advance_native_world_camera(&mut ui.context);
        redraw |= self.editor.update_ui(&mut ui.ui, &mut ui.context);
        redraw |= self.editor.update(&mut ui.context);
        self.editor.publish_native_world_paint_sources();
        redraw |= self.editor.advance_native_world_source();
        redraw |= self.editor.advance_native_world_paint();
        redraw |= self.editor.advance_native_world_stamps();
        redraw |= self.editor.advance_native_world_paste();
        redraw |= self.editor.advance_native_minimap_source();
        self.editor.sync_native_game_input(&mut ui.ui, &ui.context);
        if let Err(error) = self.editor.sync_native_play(&mut ui.ui, &mut ui.context, elapsed) {
            ui.context.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
        }
        redraw |= self.editor.native_views_waiting();
        // A smoke run deliberately needs three presentation acknowledgements;
        // a normal static editor must not redraw merely to satisfy this probe.
        redraw |= self.smoke && self.presentations < 3;
        let wake_after = self.editor.native_update_wake_after(redraw)
            .into_iter()
            .chain(self.deadline.filter(|_| self.presentations < 3)
                .map(|due| due.saturating_duration_since(Instant::now())))
            .chain(self.idle_probe.as_ref().and_then(|probe| probe.started.map(|(start, _)| {
                probe.duration.saturating_sub(start.elapsed())
            })))
            .min();
        FrameRequest {
            redraw,
            wake_after,
        }
    }
    fn frame(&mut self, ui: &mut UiSession, images: &mut UiFrameImages<'_>) -> Result<PaintList> {
        let mut profile = crate::misc::FrameProfile::new();
        self.editor.pre_ui(&mut ui.context);
        profile.lap("native_pre_ui");
        ui.ui.prepare_paint_layout(&mut ui.context);
        profile.lap("native_frame_layout");
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
        profile.lap("native_views");
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
        profile.lap("native_ui_paint");
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
        if self.presentations == 1
            && let Some(start) = self.startup.take()
        {
            eprintln!(
                "Creator native first surface presentation: {:.2}ms",
                start.elapsed().as_secs_f64() * 1000.
            );
        }
        if self.smoke && self.presentations >= 3 {
            if let Some(probe) = &mut self.idle_probe {
                if probe.started.is_none() {
                    probe.started = Some((Instant::now(), self.presentations));
                    eprintln!("Creator native idle probe started after 3 surface presentations");
                    eprintln!(
                        "Native idle owners: empty={} world={} display={} play={}",
                        self.editor.native_empty_viewport(),
                        self.editor.native_world_owns_viewport(),
                        self.editor.native_display_owns_viewport(),
                        self.editor.native_play_owns_viewport()
                    );
                }
                return;
            }
            eprintln!(
                "Creator native smoke passed: {} successful surface presentations",
                self.presentations
            );
            ui.requests.push(HostRequest::Exit);
        }
    }

    fn gpu_recreated(&mut self, ui: &mut UiSession, images: &mut UiFrameImages<'_>) -> Result<()> {
        self.editor
            .bind_native_reference_context(images.registry.context().clone());
        self.pending = None;
        self.editor.cancel_native_play_input(&mut ui.ui);
        self.editor.reset_native_play_gpu(images.registry).map_err(RenderError)?;
        self.editor
            .cancel_native_world_gesture(&mut ui.ui, &mut ui.context);
        self.editor.invalidate_native_views(&mut ui.ui);
        Ok(())
    }
    fn suspended(&mut self, ui: &mut UiSession) {
        self.editor.native_runtime_suspended(true);
        self.editor.cancel_native_play_input(&mut ui.ui);
        self.pending = None;
        ui.input(&InputEvent::Focus(false));
        self.editor
            .cancel_native_world_gesture(&mut ui.ui, &mut ui.context);
        self.editor.mouse_leave(&mut ui.context);
        self.editor.invalidate_native_views(&mut ui.ui);
    }
    fn resumed(&mut self, ui: &mut UiSession) {
        self.editor.native_runtime_suspended(false);
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
    use super::visibility_check::check_native_visibility;
    use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu};
    use rusterix::D3Camera;
    use std::sync::Arc;
    use theframework::render_wgpu::PaintRenderer;
    // Return the first complete native candidate without acknowledging it. This
    // preserves tests of input against an unpresented frame while allowing the
    // same bounded source preparation that production hosts drive over frames.
    fn ready_world_frame(
        app: &mut UiApplicationAdapter<NativeCreatorApplication>,
        host: &mut HostContext,
        images: &mut ImageRegistry,
        gpu: &GpuContext,
        renderer: &mut PaintRenderer,
        output: &GpuImage,
    ) -> theframework::paint::PaintList {
        let before = gpu.transfers();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            app.update(host, Duration::from_millis(16));
            let paint = app.frame(host, images).unwrap();
            let tokens = app.application.pending.as_ref().unwrap();
            if tokens.world_ready_for_test() { return paint; }
            assert!(tokens.world_for_test() || app.application.editor.native_views_waiting(),
                "an incomplete native world must retain its continuation");
            assert!(app.session.ui.canvas.buffer.pixels().is_empty());
            assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
            renderer.render(&paint, images, &output.view, output.descriptor.format,
                host.physical_size, host.density, [0.;4]).unwrap();
            app.presented(host);
            assert_no_viewport_readback(before, gpu);
            assert!(Instant::now() < deadline,
                "native world did not settle: {}", app.application.editor.native_world_debug_for_test());
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    #[ignore = "actual native large-plan generation, progressive display and saved channels"]
    fn native_large_bake_plan_generates_without_a_viewport_adapter() {
        let gpu=pollster::block_on(GpuContext::headless()).unwrap();
        let mut images=ImageRegistry::new(gpu.clone());
        let mut application=NativeCreatorApplication::new(vec!["eldiron-creator-native".into()],false);
        application.start_services=false;
        let mut app=UiApplicationAdapter::new(application);
        app.application.editor.check_context_ownership_for_test(&gpu);
        let mut host=HostContext::new((1200,800));
        app.initialize(&mut host,&mut images).unwrap();
        let output=GpuImage::new(&gpu,ImageDescriptor::rgba(1200,800)).unwrap();
        let mut renderer=PaintRenderer::new(gpu.clone()).unwrap();
        app.update(&mut host,Duration::from_millis(16));
        let paint=app.frame(&mut host,&mut images).unwrap();
        renderer.render(&paint,&images,&output.view,output.descriptor.format,(1200,800),1.,[0.;4]).unwrap();
        app.presented(&mut host);
        let fixture=app.application.editor.begin_native_large_generation_for_test(&mut app.session.ui,&mut app.session.context);
        let settled=Instant::now()+Duration::from_secs(30);
        loop {
            app.update(&mut host,Duration::from_millis(16));
            if app.application.editor.native_world_source_for_test().is_some_and(|(_,ready)|ready)
                && app.application.editor.native_reference_source_ready_for_test() { break; }
            assert!(Instant::now()<settled,"large fixture sources did not settle");
            std::thread::sleep(Duration::from_millis(2));
        }
        let (authored,old,revision)=app.application.editor.request_native_progressive_bake_for_test(&mut app.session.ui,&mut app.session.context);
        let before=gpu.transfers();
        let deadline=Instant::now()+Duration::from_secs(240);
        let mut largest_plan=0;let mut preview=false;let mut frames=0;
        loop {
            app.update(&mut host,Duration::from_millis(16));
            let paint=app.frame(&mut host,&mut images).unwrap();
            renderer.render(&paint,&images,&output.view,output.descriptor.format,(1200,800),1.,[0.;4]).unwrap();
            app.presented(&mut host);
            assert_no_viewport_readback(before,&gpu);
            assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
            let (plan,seen)=app.application.editor.native_large_generation_progress_for_test();
            largest_plan=largest_plan.max(plan);preview|=seen;frames+=1;
            assert!(gpu.source_pool_usage().bytes<=gpu.source_pool_usage().byte_limit);
            if app.application.editor.native_progressive_bake_complete_for_test(&authored,&old,revision) { break; }
            assert!(Instant::now()<deadline,"large native generation did not complete: plan={largest_plan}, frames={frames}");
        }
        assert!(largest_plan>64*1024*1024,"fixture must actually exceed the previous plan cutoff: {largest_plan}");
        assert!(preview,"large generation must publish progressive native feedback");
        assert!(gpu.transfers().asset_bake_readback_requests>before.asset_bake_readback_requests);
        app.application.editor.check_native_generated_bake_history_for_test(&old,&mut app.session.ui,&mut app.session.context);
        std::fs::copy("/private/tmp/eldiron-native-generated-bake.eldiron","/private/tmp/eldiron-native-large-generated-bake.eldiron").unwrap();
        eprintln!("Large native generation: plan={largest_plan} bytes, {frames} root frames, captures={}, bytes={}",gpu.transfers().asset_bake_readback_requests-before.asset_bake_readback_requests,gpu.transfers().asset_bake_readback_bytes-before.asset_bake_readback_bytes);
        app.application.editor.finish_native_bake_for_test(fixture,&mut app.session.ui,&mut app.session.context);
    }
    #[test]
    #[ignore = "requires actual native GPU; exercises the assembled Creator UI"]
    fn native_creator_application_paints_the_root_ui_without_a_root_framebuffer() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        if std::env::var_os("ELDIRON_NATIVE_TILE_BOARD_ONLY").is_some() {
            let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
            let mut context = TheContext::new(230, 140, 1.);
            check_asset_sources(&gpu, &mut renderer, &mut context);
            return;
        }
        let mut registry = ImageRegistry::new(gpu.clone());
        let mut application =
            NativeCreatorApplication::new(vec!["eldiron-creator-native".into()], false);
        // External services are unrelated to offscreen UI verification.
        application.start_services = false;
        let mut app = UiApplicationAdapter::new(application);
        app.application.editor.check_context_ownership_for_test(&gpu);
        let mut host = HostContext::new((1200, 800));
        app.initialize(&mut host, &mut registry).unwrap();
        assert_eq!(
            app.application.editor.reference_for_test().read().scene_handler.vm.device_generation(),
            Some(gpu.generation()),
        );
        let authored =
            serde_json::to_value(&app.application.editor.project_for_native_test()).unwrap();
        app.update(&mut host, Duration::from_millis(16));
        let list = app.frame(&mut host, &mut registry).unwrap();
        assert!(
            !scenevm::SceneVM::compatibility_gpu_initialized(),
            "canonical Creator startup must not initialize the compatibility device",
        );
        list.validate().unwrap();
        assert!(
            app.session.ui.canvas.buffer.pixels().is_empty(),
            "the Creator root cannot allocate a legacy display framebuffer"
        );
        assert!(
            !app.session
                .ui
                .get_render_view("PolyView")
                .unwrap()
                .has_cpu_display_storage(),
            "an unselected native map cannot allocate and upload a blank viewport bitmap"
        );
        assert!(
            !app.session
                .ui
                .get_render_view("Tiles Dock View 0")
                .unwrap()
                .has_cpu_display_storage(),
            "the native tile picker must retain source assets instead of a viewport bitmap"
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
        let mut dirty_widgets = Vec::new();
        app.session.ui.canvas.visit_visible_widgets(&mut |widget| {
            if widget.needs_redraw() {
                dirty_widgets.push(widget.id().name.clone());
            }
        });
        assert!(
            !app.session.ui.needs_paint(&app.session.context),
            "the assembled native root must acknowledge a successful static paint; dirty widgets={dirty_widgets:?}, relayout={}, redraw_all={}",
            app.session.context.ui.needs_layout(), app.session.context.ui.needs_full_paint()
        );
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
            if !app.application.editor.reference_jobs_for_test().scene()
                .has_pending_geometry()
            {
                break;
            }
        }
        assert!(
            !app.application.editor.reference_jobs_for_test().scene()
                .has_pending_geometry()
        );
        assert_eq!(
            app.application.editor.controllers_for_test().tools.read()
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
        assert!(app.application.editor.native_world_character_preview_for_test(),
            "stopped authoring uses class/setup preview defaults on its owned scene");
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
        check_native_paste(
            &mut app, &mut host, &mut replacement, &gpu, &mut renderer, &output,
        );
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
                    let builds = app.application.editor.reference_jobs_for_test().scene();
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
            let pose = app.application.editor.authoring_camera_for_test()
                .orbit_camera
                .as_scenevm_camera();
            if camera_index == 2 {
                assert!(app.application.editor.handle_native_world_input(
                    &cube_click,
                    &mut app.session.ui,
                    &mut app.session.context
                ));
                assert_eq!(
                    app.application.editor.authoring_camera_for_test()
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
            if matches!(camera_index, 2 | 3 | 4) {
                check_native_paste(
                    &mut app, &mut host, &mut replacement, &gpu, &mut renderer, &output,
                );
            }
            if camera_index == 2 {
                check_native_surface_detail(
                    &mut app, &mut host, &mut replacement, &gpu, &mut renderer, &output,
                );
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
                    app.application.editor.authoring_camera_for_test()
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
                let (image, guides, draws) = app.application.editor
                    .native_world_logical_guides_for_test().expect("isometric logical GPU layer");
                assert!(guides > 0 && draws > 3, "iso guides execute as real commands in the logical GPU layer");
                assert!(
                    world
                        .ops
                        .iter()
                        .any(|op| matches!(op, theframework::paint::PaintOp::PixelImage { image: id, .. } if *id == image)),
                    "root presents the GPU layer containing the actual iso guides"
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
        check_native_bake(&mut app, &mut host, &mut replacement, &gpu, &mut renderer, &output);
        check_native_custom_contexts(&mut app, &mut host, &mut replacement, &gpu, &mut renderer, &output);
        check_native_region_contexts(&mut app, &mut host, &mut replacement, &gpu, &mut renderer, &output);
        check_native_visibility(&mut app, &mut host, &mut replacement, &gpu, &mut renderer);
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
        app.update(&mut host, Duration::from_millis(16));
        check_native_paste(
            &mut app, &mut host, &mut replacement, &gpu, &mut renderer, &output,
        );
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
                let builds = app.application.editor.reference_jobs_for_test().scene();
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
        let paint = ready_world_frame(&mut app, &mut host, &mut replacement,
            &gpu, &mut renderer, &output);
        assert!(
            app.application.pending.as_ref().unwrap().world_ready_for_test(),
            "3D Paint uses its completed native scene owner"
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
        let paint = ready_world_frame(&mut app, &mut host, &mut replacement,
            &gpu, &mut renderer, &output);
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
        app.application.editor.authoring_camera_for_test()
            .iso_camera
            .zoom(1.);
        let paint = ready_world_frame(&mut app, &mut host, &mut replacement,
            &gpu, &mut renderer, &output);
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
            let frame = ready_world_frame(app, host, registry, &gpu, &mut renderer, &output);
            if let Some(picker) = app.session.ui.get_render_view("Prefab Editor Tiles Dock View 0") {
                assert!(
                    !picker.has_cpu_display_storage(),
                    "Prefab's normal tile picker cannot restore the old bitmap Paint dock"
                );
            }
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
        // The first down on a presented door preview closes it, then must
        // resume against the acknowledged closed scene, including an early up.
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.geometry".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.update(&mut host, Duration::from_millis(16));
        let part = app.application.editor.project_for_native_test().block_props[&prefab].parts[0].id;
        app.session.context.ui.send(TheEvent::NewListItemSelected(
            TheId::named_with_id("Prefab Branch", part),
            TheId::named("Prefab Graph Branches"),
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Prefab Editor Configure Door"),
            TheWidgetState::Clicked,
        ));
        app.update(&mut host, Duration::from_millis(16));
        assert!(
            !app.application.editor.project_for_native_test().block_props[&prefab]
                .components
                .is_empty()
        );
        let closed = serde_json::to_value(
            &app.application
                .editor
                .project_for_native_test()
                .prefab_editor_map
                .as_ref()
                .unwrap()
                .geometry_objects,
        )
        .unwrap();
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Prefab Editor Preview Door"),
            TheWidgetState::Clicked,
        ));
        settle_prefab(&mut app, &mut host, &mut replacement);
        assert_ne!(
            serde_json::to_value(
                &app.application
                    .editor
                    .project_for_native_test()
                    .prefab_editor_map
                    .as_ref()
                    .unwrap()
                    .geometry_objects
            )
            .unwrap(),
            closed
        );
        let dim = *app.session.ui.get_render_view("PrefabView").unwrap().dim();
        let blank = Vec2::new(dim.width - 40, dim.height - 40);
        app.session.context.ui.set_focus(&prefab_view);
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(prefab_view.clone(), blank));
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewUp(prefab_view.clone(), blank));
        app.update(&mut host, Duration::from_millis(16));
        assert_eq!(
            app.application.editor.native_retry_state_for_test(),
            (true, false)
        );
        assert_eq!(
            serde_json::to_value(
                &app.application
                    .editor
                    .project_for_native_test()
                    .prefab_editor_map
                    .as_ref()
                    .unwrap()
                    .geometry_objects
            )
            .unwrap(),
            closed
        );
        assert!(
            !app.application
                .editor
                .project_for_native_test()
                .prefab_editor_map
                .as_ref()
                .unwrap()
                .selected_geometry_objects
                .is_empty()
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.application.editor.native_retry_state_for_test() != (false, false) {
            settle_prefab(&mut app, &mut host, &mut replacement);
            assert!(
                Instant::now() < deadline,
                "accepted door click did not resume and release"
            );
        }
        assert!(
            app.application
                .editor
                .project_for_native_test()
                .prefab_editor_map
                .as_ref()
                .unwrap()
                .selected_geometry_objects
                .is_empty(),
            "the accepted background click clears selection after the closed scene is presented"
        );
        // Focus loss cancels the waiting physical sequence without inventing
        // another authoring click when the replacement finally becomes ready.
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Prefab Editor Preview Door"),
            TheWidgetState::Clicked,
        ));
        settle_prefab(&mut app, &mut host, &mut replacement);
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewClicked(prefab_view.clone(), blank));
        app.session
            .context
            .ui
            .send(TheEvent::RenderViewUp(prefab_view.clone(), blank));
        app.update(&mut host, Duration::from_millis(16));
        assert!(app.application.editor.native_retry_state_for_test().0);
        app.input(&InputEvent::Focus(false), &mut host);
        assert_eq!(
            app.application.editor.native_retry_state_for_test(),
            (false, false)
        );
        settle_prefab(&mut app, &mut host, &mut replacement);
        assert!(
            !app.application
                .editor
                .project_for_native_test()
                .prefab_editor_map
                .as_ref()
                .unwrap()
                .selected_geometry_objects
                .is_empty()
        );
        app.input(&InputEvent::Focus(true), &mut host);
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.iso_paint".into()),
        ));
        app.session.context.ui.send(TheEvent::IndexChanged(
            TheId::named(crate::tools::paint_picker::MODE),
            0,
        ));
        // Tool activation queues the dock-mode notification for the next update.
        app.update(&mut host, Duration::from_millis(16));
        settle_prefab(&mut app, &mut host, &mut replacement);
        assert_eq!(
            app.session.ui.get_stack_layout("Prefab Editor Mode Stack").unwrap().index(),
            1,
            "Paint shares the normal embedded tile picker"
        );
        let picker = app.session.ui.get_layout("Prefab Editor Tiles Dock Tabs").unwrap();
        assert!(picker.dim().width > 0 && picker.dim().height > 0);
        for prefix in ["", "Prefab "] {
            for obsolete in ["3D Paint Preset Strip", "3D Paint Material Strip", "3D Paint Brush Editor"] {
                assert!(app.session.ui.get_widget(&format!("{prefix}{obsolete}")).is_none(),
                    "the retired Paint dock must not be constructed");
            }
        }
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
        // Continue through actual toolbar Play controls using the same CPU
        // driver as the common player. The live authoring world also retains
        // its own GPU scene; authored game-screen mode is migrated separately.
        let play_document = app
            .application
            .editor
            .begin_native_play_for_test(&mut app.session.ui, &mut app.session.context);
        app.update(&mut host, Duration::from_millis(16));
        let authored =
            serde_json::to_value(&app.application.editor.project_for_native_test().regions)
                .unwrap();
        let authored_session = app.application.editor.native_play_authoring_checkpoint_for_test();
        let authored_time = app.application.editor.project_for_native_test().time;
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Play"),
            TheWidgetState::Clicked,
        ));
        app.update(&mut host, Duration::from_millis(16));
        let initial = app
            .application
            .editor
            .native_play_stats_for_test()
            .expect("toolbar Play owns the shared runtime driver");
        let owned = app.application.editor.native_owned_runtime_for_test().unwrap();
        assert!(owned.0 && owned.1, "native Play owns the sole authoritative server; facade remains Off");
        // Advance CPU time without preparing/presenting any scene. Visual
        // consumers cannot determine simulation cadence or create tick debt.
        app.update(&mut host, owned.2 * 3);
        assert_eq!(app.application.editor.native_play_stats_for_test().unwrap().0, initial.0 + 3);
        app.application.editor.publish_native_runtime_sources_for_test();
        assert!(!app.application.editor.native_runtime_sources_received_for_test());
        app.update(&mut host, Duration::ZERO);
        assert!(app.application.editor.native_runtime_sources_received_for_test());
        assert_eq!(
            app.application.editor.native_play_authoring_checkpoint_for_test(),
            authored_session,
            "startup/warmup cannot modify the saved document or its undo revision"
        );
        app.application.editor.inject_native_play_projection_for_test();
        let runtime_time = TheTime::new_time(21, 37).unwrap();
        app.session.context.ui.send(TheEvent::ValueChanged(
            TheId::named("Server Time Slider"), TheValue::Time(runtime_time),
        ));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.update(&mut host, Duration::from_millis(16));
            let before = gpu.transfers();
            let frame = app.frame(&mut host, &mut replacement).unwrap();
            assert!(
                app.application.pending.as_ref().unwrap().world_for_test(),
                "live Play authoring uses the owning native scene"
            );
            renderer
                .render(
                    &frame,
                    &replacement,
                    &output.view,
                    output.descriptor.format,
                    (1600, 1000),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(&mut host);
            assert_no_viewport_readback(before, &gpu);
            assert!(
                !app.application
                    .editor
                    .native_world_has_cpu_storage_for_test()
            );
            if app.application.editor.native_world_ready_for_test() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "live Play source never became ready"
            );
        }
        assert!(app.application.editor.native_play_projection_received_for_test());
        assert!(!app.application.editor.native_world_character_preview_for_test(),
            "owned Play renders runtime character state without authoring defaults");
        assert_eq!(
            app.application.editor.native_play_authoring_checkpoint_for_test(),
            authored_session,
            "server-generated geometry belongs only to the runtime projection"
        );
        assert_eq!(app.application.editor.native_world_speech_for_test(), (0, 0, 0, false),
            "a world with no speech does not create a font registry");
        app.application.editor.queue_native_world_speech_for_test();
        app.application.editor.queue_native_play_message_for_test();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            std::thread::sleep(Duration::from_millis(50));
            app.update(&mut host, Duration::from_millis(50));
            let state = app.application.editor.native_play_stats_for_test().unwrap();
            if state.0 > initial.0 && state.2 == 1 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Creator shared clock/output did not advance: {state:?}"
            );
        }
        let audio_before = app.application.editor.native_audio_commands_for_test();
        app.application.editor.queue_native_unseen_audio_for_test();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.application.editor.native_audio_commands_for_test() == audio_before {
            std::thread::sleep(Duration::from_millis(10));
            app.update(&mut host, Duration::ZERO);
            assert!(Instant::now() < deadline, "unseen-region audio was tied to the authoring view");
        }
        assert_eq!(app.application.editor.native_audio_commands_for_test(), audio_before + 1);
        std::thread::sleep(Duration::from_millis(40));
        app.update(&mut host, Duration::ZERO);
        assert_eq!(app.application.editor.native_audio_commands_for_test(), audio_before + 1, "audio cannot replay on another poll");
        app.application.editor.tag_native_console_actor_for_test();
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Console Feedback Action"), TheValue::Text("list chars".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        assert!(app.application.editor.native_console_text_for_test().contains("Console session actor"));
        let old_console_link = app.application.editor.native_console_runtime_link_for_test().unwrap();
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Pause"),
            TheWidgetState::Clicked,
        ));
        app.update(&mut host, Duration::from_millis(16));
        let paused = app.application.editor.native_play_stats_for_test().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        app.update(&mut host, Duration::from_millis(300));
        assert_eq!(
            app.application.editor.native_play_stats_for_test(),
            Some(paused),
            "paused Play cannot tick or drain a message twice"
        );
        let mut original_projection = None;
        let mut speech_layouts = None;
        for (mode, camera) in [
            (crate::prelude::EditorViewMode::D2, rusterix::MapCamera::TwoD),
            (crate::prelude::EditorViewMode::D2, rusterix::MapCamera::TwoDIso),
            (crate::prelude::EditorViewMode::Iso, rusterix::MapCamera::ThreeDIso),
        ] {
            let old = app.application.editor.native_world_speech_projection_for_test(mode, camera, crate::prelude::Vec3::zero());
            original_projection.get_or_insert(old);
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                app.update(&mut host, Duration::ZERO);
                let before = gpu.transfers();
                let frame = app.frame(&mut host, &mut replacement).unwrap();
                assert!(app.application.pending.as_ref().unwrap().world_for_test());
                let speech = app.application.editor.native_world_speech_for_test();
                assert_eq!((speech.0, speech.1, speech.3), (1, 1, true), "speech in {camera:?}");
                renderer.render(&frame, &replacement, &output.view, output.descriptor.format,
                    (1600, 1000), 1., [0.; 4]).unwrap();
                app.presented(&mut host);
                assert_no_viewport_readback(before, &gpu);
                assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
                if app.application.editor.native_world_ready_for_test() {
                    break;
                }
                assert!(Instant::now() < deadline, "speech view source was not published");
            }
            let built = app.application.editor.native_world_speech_for_test().2;
            if let Some(before) = speech_layouts {
                assert_eq!(built, before, "camera changes reuse speech shaping");
            }
            speech_layouts = Some(built);
        }
        let speech_before_recovery = app.application.editor.native_world_speech_for_test();
        app.gpu_recreated(&mut host, &mut replacement).unwrap();
        assert_eq!(app.application.editor.native_world_speech_for_test().0, speech_before_recovery.0,
            "GPU recovery retains CPU speech");
        let frame = app.frame(&mut host, &mut replacement).unwrap();
        renderer.render(&frame, &replacement, &output.view, output.descriptor.format,
            (1600, 1000), 1., [0.; 4]).unwrap();
        app.presented(&mut host);
        assert_eq!(app.application.editor.native_world_speech_for_test(), speech_before_recovery,
            "replacement targets rebuild placement and reuse the retained speech layout");
        let (mode, camera, center) = original_projection.unwrap();
        app.application.editor.native_world_speech_projection_for_test(mode, camera, center);
        assert_eq!(app.application.editor.native_play_stats_for_test(), Some(paused),
            "speech projection never ticks or drains the shared runtime");
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Play"),
            TheWidgetState::Clicked,
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.update(&mut host, Duration::from_millis(16));
        let resumed = app.application.editor.native_play_stats_for_test().unwrap();
        assert_eq!(
            (resumed.0, resumed.1),
            (paused.0, paused.1),
            "resume preserves runtime identity and discards paused elapsed time"
        );
        assert_eq!(
            resumed.2, 0,
            "the existing Resume policy clears pending presentation messages"
        );
        // Real authoring GameInput stays on its acknowledged GPU scene and
        // borrows the same CPU control owner as the full Game tool.
        let toggle = *app.session.ui.get_widget("GameInput").unwrap().dim();
        let toggle_at = [
            (toggle.x + toggle.width / 2) as f32,
            (toggle.y + toggle.height / 2) as f32,
        ];
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position: toggle_at,
            },
            &mut host,
        );
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: toggle_at,
            },
            &mut host,
        );
        app.update(&mut host, Duration::from_millis(16));
        let world_view = app
            .session
            .ui
            .get_render_view("PolyView")
            .unwrap()
            .id()
            .clone();
        app.session.context.ui.set_focus(&world_view);
        let key = |physical, logical, pressed| InputEvent::Key {
            physical,
            logical,
            pressed,
            repeat: false,
        };
        let w = |pressed| {
            key(
                PhysicalKey::Letter(b'W'),
                LogicalKey::Character("w".into()),
                pressed,
            )
        };
        let up = |pressed| key(PhysicalKey::Up, LogicalKey::Named(TheKeyCode::Up), pressed);
        let before_input = app
            .application
            .editor
            .native_runtime_input_for_test()
            .unwrap();
        app.input(&w(true), &mut host);
        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap(),
            before_input,
            "GameInput cannot start movement before presentation"
        );
        let before = gpu.transfers();
        let frame = app.frame(&mut host, &mut replacement).unwrap();
        assert!(app.application.pending.as_ref().unwrap().world_for_test());
        assert!(!app.application.pending.as_ref().unwrap().play_for_test());
        assert!(
            !app.application
                .editor
                .native_world_has_cpu_storage_for_test()
        );
        assert!(
            !app.session
                .ui
                .get_render_view("PolyView")
                .unwrap()
                .has_cpu_display_storage()
        );
        renderer
            .render(
                &frame,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.input(&w(true), &mut host);
        app.input(&up(true), &mut host);
        let accepted = app
            .application
            .editor
            .native_runtime_input_for_test()
            .unwrap();
        assert_eq!(accepted.0, initial.1);
        assert_eq!(accepted.1.held_controls, 2);
        assert_eq!(
            accepted.1.submitted_actions,
            before_input.1.submitted_actions + 1,
            "aliased press sends one changed movement state"
        );
        app.input(&w(false), &mut host);
        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap()
                .1
                .held_controls,
            1
        );
        app.input(&up(false), &mut host);
        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap()
                .1
                .held_controls,
            0
        );
        app.input(&w(true), &mut host);
        app.gpu_recreated(&mut host, &mut replacement).unwrap();
        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap()
                .1
                .held_controls,
            0,
            "GPU recovery cancels accepted controls without replacing their CPU owner"
        );

        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap()
                .0,
            initial.1
        );
        let frame = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &frame,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.input(&w(true), &mut host);
        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap()
                .1
                .held_controls,
            1
        );
        app.input(&InputEvent::Focus(false), &mut host);
        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap()
                .1
                .held_controls,
            0
        );
        // Focus belongs to the host, not whichever presentation mode was open
        // when the event arrived. A background mode switch cannot accept keys.
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.game".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.update(&mut host, Duration::from_millis(16));
        let background = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &background,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.session.context.ui.set_focus(&world_view);
        app.input(&w(true), &mut host);
        assert_eq!(
            app.application
                .editor
                .native_runtime_input_for_test()
                .unwrap()
                .1
                .held_controls,
            0,
            "a background Game tool cannot acquire movement"
        );
        app.input(&InputEvent::Focus(true), &mut host);
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.geometry".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.update(&mut host, Duration::from_millis(16));
        let foreground = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &foreground,
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
            "authoring GameInput performs no color readback"
        );
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position: toggle_at,
            },
            &mut host,
        );
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: toggle_at,
            },
            &mut host,
        );
        app.update(&mut host, Duration::from_millis(16));
        assert!(!app.application.editor.native_game_input_claim_for_test());
        // The real Game tool must use the same authored-screen controller as
        // the common player, with independent GPU scenes and text ownership.
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.game".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.update(&mut host, Duration::from_millis(16));
        let before = gpu.transfers();
        let game = app.frame(&mut host, &mut replacement).unwrap();
        assert!(app.application.pending.as_ref().unwrap().play_for_test());
        let stats = app.application.editor.native_game_stats_for_test().unwrap();
        assert_eq!(stats.0, initial.1);
        assert!(stats.2 > 0);
        assert!(!stats.3);
        assert!(
            !app.session
                .ui
                .get_render_view("PolyView")
                .unwrap()
                .has_cpu_display_storage()
        );
        let field = app.application.editor.native_game_input_for_test().unwrap();
        let click = InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position: field,
        };
        app.input(&click, &mut host);
        assert!(
            host.text_input.is_none(),
            "unpresented screen cannot focus an authored field"
        );
        renderer
            .render(
                &game,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.input(&click, &mut host);
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: field,
            },
            &mut host,
        );
        let target = host
            .text_input
            .expect("authored field publishes its own native input target")
            .target;
        let snapshot = app.native_text_snapshot(&mut host).unwrap();
        assert_eq!(snapshot.target, target);
        assert_eq!(&*snapshot.text, "Éowyn");
        app.input(
            &InputEvent::TextFor {
                target,
                text: "東京".into(),
            },
            &mut host,
        );
        let edited = app.native_text_snapshot(&mut host).unwrap();
        assert!(edited.text.contains("東京"));
        assert_eq!(
            app.application
                .editor
                .native_game_stats_for_test()
                .unwrap()
                .1,
            0,
            "text does not become movement"
        );
        let frame = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &frame,
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
            "authored Game screens render without color readback"
        );
        // Renderer recreation and hiding/reopening the Game tool retain the
        // CPU session/text, but require new acknowledged GPU scene outputs.
        app.gpu_recreated(&mut host, &mut replacement).unwrap();
        assert!(host.text_input.is_none());
        let rebuilt = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &rebuilt,
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
            app.application
                .editor
                .native_game_stats_for_test()
                .unwrap()
                .0,
            initial.1
        );
        assert_eq!(app.native_text_snapshot(&mut host).unwrap().target, target);
        assert!(
            app.native_text_snapshot(&mut host)
                .unwrap()
                .text
                .contains("東京")
        );
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.geometry".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.update(&mut host, Duration::from_millis(16));
        let hidden = app.frame(&mut host, &mut replacement).unwrap();
        assert!(!app.application.pending.as_ref().unwrap().play_for_test());
        renderer
            .render(
                &hidden,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.game".into()),
        ));
        app.update(&mut host, Duration::from_millis(16));
        app.update(&mut host, Duration::from_millis(16));
        let restored = app.frame(&mut host, &mut replacement).unwrap();
        assert!(app.application.pending.as_ref().unwrap().play_for_test());
        renderer
            .render(
                &restored,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        let stats = app.application.editor.native_game_stats_for_test().unwrap();
        assert_eq!(stats.0, initial.1);
        assert!(stats.2 > 0);
        assert!(!stats.3);
        let field = app.application.editor.native_game_input_for_test().unwrap();
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position: field,
            },
            &mut host,
        );
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: field,
            },
            &mut host,
        );
        assert!(
            app.native_text_snapshot(&mut host)
                .unwrap()
                .text
                .contains("東京")
        );
        assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
        let mut capture = ReadbackTicket::capture(&gpu, output.clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let image = capture.try_finish(&gpu).unwrap().unwrap();
        let mut encoder = png::Encoder::new(
            std::fs::File::create("/private/tmp/eldiron-native-creator-play.png").unwrap(),
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
        app.input(&InputEvent::Focus(false), &mut host);
        assert!(host.text_input.is_none());
        app.input(&InputEvent::Focus(true), &mut host);
        let frame = app.frame(&mut host, &mut replacement).unwrap();
        renderer
            .render(
                &frame,
                &replacement,
                &output.view,
                output.descriptor.format,
                (1600, 1000),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(&mut host);
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position: [800., 500.],
            },
            &mut host,
        );
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: [800., 500.],
            },
            &mut host,
        );
        assert!(host.text_input.is_none());
        let key = |physical, logical, pressed| InputEvent::Key {
            physical,
            logical,
            pressed,
            repeat: false,
        };
        app.input(
            &key(
                PhysicalKey::Letter(b'W'),
                LogicalKey::Character("w".into()),
                true,
            ),
            &mut host,
        );
        app.input(
            &key(PhysicalKey::Up, LogicalKey::Named(TheKeyCode::Up), true),
            &mut host,
        );
        assert_eq!(
            app.application
                .editor
                .native_game_stats_for_test()
                .unwrap()
                .1,
            2
        );
        app.input(
            &key(
                PhysicalKey::Letter(b'W'),
                LogicalKey::Character("w".into()),
                false,
            ),
            &mut host,
        );
        assert_eq!(
            app.application
                .editor
                .native_game_stats_for_test()
                .unwrap()
                .1,
            1
        );
        app.input(
            &key(PhysicalKey::Up, LogicalKey::Named(TheKeyCode::Up), false),
            &mut host,
        );
        assert_eq!(
            app.application
                .editor
                .native_game_stats_for_test()
                .unwrap()
                .1,
            0
        );
        // Text Play shares the same input queue/CPU incarnation and emits
        // native widget paint without drawing the hidden game framebuffer.
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Text Play"),
            TheWidgetState::Clicked,
        ));
        for _ in 0..3 {
            app.update(&mut host, Duration::from_millis(16));
        }
        assert_eq!(
            app.session
                .context
                .ui
                .focus
                .as_ref()
                .map(|id| id.name.as_str()),
            Some(crate::textplay::TextGameState::GAME_INPUT_ID),
            "the application services the text tool activation request"
        );
        let input = app
            .session
            .ui
            .get_widget(crate::textplay::TextGameState::GAME_INPUT_ID)
            .unwrap()
            .id()
            .clone();
        app.session.context.ui.send(TheEvent::Custom(
            TheId::named_with_id_and_reference(
                crate::textplay::TextGameState::ACTIVATE_EVENT,
                input.uuid,
                Uuid::new_v4(),
            ),
            TheValue::Text(crate::textplay::TextGameState::GAME_INPUT_ID.into()),
        ));
        app.session.context.ui.set_focus(&world_view);
        for _ in 0..2 {
            app.update(&mut host, Duration::from_millis(16));
        }
        assert_eq!(
            app.session.context.ui.focus.as_ref().unwrap().uuid,
            world_view.uuid,
            "an old document cannot activate a persistent text widget in the new document"
        );
        crate::textplay::TextGameState::request_activation(
            crate::textplay::TextGameState::GAME_INPUT_ID,
            app.application.editor.native_document_instance_for_test(),
            &mut app.session.ui,
            &mut app.session.context,
        );
        for _ in 0..2 {
            app.update(&mut host, Duration::from_millis(16));
        }
        assert_eq!(
            app.session.context.ui.focus.as_ref().unwrap().uuid,
            input.uuid
        );
        let before_text = app
            .application
            .editor
            .native_runtime_input_for_test()
            .unwrap();
        app.session.context.ui.send(TheEvent::ValueChanged(
            TheId::named(crate::textplay::TextGameState::GAME_INPUT_ID),
            TheValue::Text("intent look".into()),
        ));
        for _ in 0..3 {
            app.update(&mut host, Duration::from_millis(16));
        }
        let after_text = app
            .application
            .editor
            .native_runtime_input_for_test()
            .unwrap();
        assert_eq!(after_text.0, before_text.0);
        assert_eq!(
            after_text.1.submitted_actions,
            before_text.1.submitted_actions + 1
        );
        assert!(
            app.application
                .editor
                .native_text_output_for_test("> intent look")
        );
        let before_text_paint = gpu.transfers();
        let frame = app.frame(&mut host, &mut replacement).unwrap();
        // Preparing this root fails explicitly if a widget lacks native paint.
        renderer
            .render(
                &frame,
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
            before_text_paint.readback_bytes
        );
        assert!(
            !app.application
                .editor
                .native_world_has_cpu_storage_for_test()
        );
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Text Play"),
            TheWidgetState::Clicked,
        ));
        for _ in 0..2 {
            app.update(&mut host, Duration::from_millis(16));
        }
        assert_eq!(
            app.application.editor.project_for_native_test().time,
            authored_time,
            "text commands and clock updates cannot advance authored time"
        );
        assert_eq!(
            serde_json::to_value(&app.application.editor.project_for_native_test().regions).unwrap(),
            authored,
            "running text/graphical Play cannot alter authored maps"
        );
        app.application.editor.edit_native_play_authoring_for_test(&mut app.session.context);
        let edited_session = app.application.editor.native_play_authoring_checkpoint_for_test();
        assert_ne!(edited_session.1, authored_session.1);
        let edited_authored = serde_json::to_value(
            &app.application.editor.project_for_native_test().regions,
        ).unwrap();
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Stop"),
            TheWidgetState::Clicked,
        ));
        app.update(&mut host, Duration::from_millis(16));
        assert!(app.application.editor.native_play_stats_for_test().is_none());
        assert_eq!(app.application.editor.native_world_speech_for_test().0, 0,
            "Stop retires speech even if its world view is hidden");
        assert!(!app.application.editor.native_text_output_for_test("> intent look"));
        assert_eq!(
            serde_json::to_value(&app.application.editor.project_for_native_test().regions).unwrap(),
            edited_authored,
            "Stop preserves edits made during Play instead of restoring an old snapshot"
        );
        assert_eq!(app.application.editor.project_for_native_test().time, authored_time);
        app.session.context.ui.send(TheEvent::StateChanged(TheId::named("Undo"), TheWidgetState::Clicked));
        app.update(&mut host, Duration::from_millis(16));
        let mut undone = authored.clone();
        undone[0]["map"]["changed"] = serde_json::json!(
            edited_authored[0]["map"]["changed"].as_u64().unwrap() + 1
        );
        assert_eq!(serde_json::to_value(&app.application.editor.project_for_native_test().regions).unwrap(), undone);
        app.session.context.ui.send(TheEvent::StateChanged(TheId::named("Redo"), TheWidgetState::Clicked));
        app.update(&mut host, Duration::from_millis(16));
        let mut authored = edited_authored;
        authored[0]["map"]["changed"] = serde_json::json!(
            undone[0]["map"]["changed"].as_u64().unwrap() + 1
        );
        assert_eq!(serde_json::to_value(&app.application.editor.project_for_native_test().regions).unwrap(), authored);
        app.session.context.ui.send(TheEvent::StateChanged(
            TheId::named("Play"),
            TheWidgetState::Clicked,
        ));
        app.update(&mut host, Duration::from_millis(16));
        let restarted = app.application.editor.native_play_stats_for_test().unwrap();
        assert_ne!(restarted.1, initial.1, "restarting revokes the prior runtime identity");
        let before_stale_console = app.application.editor.native_console_text_for_test();
        app.session.context.ui.send(TheEvent::Custom(TheId::named("Console Feedback Action"), old_console_link));
        app.update(&mut host, Duration::from_millis(16));
        assert_eq!(app.application.editor.native_console_text_for_test(), before_stale_console, "old runtime actor links cannot target a restarted session");
        // Project replacement stops the original runtime before projecting or
        // saving the new document, and preserves current authored edits.
        app.application
            .editor
            .begin_native_play_for_test(&mut app.session.ui, &mut app.session.context);
        assert!(
            app.application
                .editor
                .native_play_stats_for_test()
                .is_none()
        );
        assert!(app.application.editor.reference_for_test().read().server.state == rusterix::ServerState::Off);
        assert_eq!(
            serde_json::to_value(
                app.application
                    .editor
                    .native_play_saved_map_for_test(play_document)
            )
            .unwrap(),
            authored,
            "project switching cannot save transient Play data"
        );
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
        check_asset_sources(&gpu, &mut renderer, &mut app.session.context);
        check_host_documents(&mut app, &mut host, &gpu);
    }
    fn check_host_documents(
        app: &mut UiApplicationAdapter<NativeCreatorApplication>,
        host: &mut HostContext,
        gpu: &Arc<GpuContext>,
    ) {
        app.application.editor.native_pixel_dock_for_test(&mut app.session.ui, &mut app.session.context, true);
        for _ in 0..2 { app.update(host, Duration::from_millis(16)); }
        assert!(!app.session.ui.get_render_view("PolyView").unwrap().has_cpu_display_storage(), "a visible pixel editor must not allocate/read back its hidden world view");
        app.application.editor.native_pixel_dock_for_test(&mut app.session.ui, &mut app.session.context, false);
        for _ in 0..2 { app.update(host, Duration::from_millis(16)); }
        let before = gpu.transfers().readback_bytes;
        let mode = app.application.editor.native_camera_mode_for_test(crate::prelude::EditorViewMode::Iso);
        let idle_source = serde_json::to_value(app.application.editor.project_for_native_test()).unwrap();
        app.input(&InputEvent::RelativeMotion { delta: [40., -25.] }, host);
        assert_eq!(serde_json::to_value(app.application.editor.project_for_native_test()).unwrap(), idle_source, "unowned device motion must not pan the authored camera over toolbars or other controls");
        app.application.editor.native_camera_mode_for_test(mode);
        let original = app.application.host_document_info().unwrap();
        let source = serde_json::to_vec(app.application.editor.project_for_native_test()).unwrap();
        assert!(app.application.host_load_document(&mut app.session, original.document, b"invalid project", None, false).is_err());
        assert_eq!(app.application.host_document_info().unwrap().document, original.document);
        assert!(source == serde_json::to_vec(app.application.editor.project_for_native_test()).unwrap(), "failed byte loads must preserve the authoring document");
        let save = app.application.host_prepare_document_save(&mut app.session, original.document).unwrap();
        assert!(app.application.host_prepare_document_save(&mut app.session, original.document).is_err(), "only one host write may own a snapshot");
        let bytes = app.application.host_document_save_bytes(save.ticket).unwrap().to_vec();
        assert_eq!(bytes.len(), save.length);
        let decoded = shared::project_io::decode_project(&bytes).unwrap();
        assert_eq!(decoded.regions.len(), app.application.editor.project_for_native_test().regions.len());
        assert!(app.application.host_complete_document_save(&mut app.session, save.ticket + 1, true, None).is_err());
        assert_eq!(app.application.host_document_save_bytes(save.ticket).unwrap(), bytes.as_slice());
        let canceled = app.application.host_complete_document_save(&mut app.session, save.ticket, false, None).unwrap();
        assert!(!canceled.saved);
        assert!(app.application.host_document_save_bytes(save.ticket).is_none());
        assert!(app.application.host_complete_document_save(&mut app.session, save.ticket, true, None).is_err());

        let save = app.application.host_prepare_document_save(&mut app.session, original.document).unwrap();
        let bytes = app.application.host_document_save_bytes(save.ticket).unwrap().to_vec();
        let path = std::path::PathBuf::from("/private/tmp/eldiron-native-host-document.eldiron");
        std::fs::write(&path, &bytes).unwrap();
        let completed = app.application.host_complete_document_save(&mut app.session, save.ticket, true, Some(path.clone())).unwrap();
        assert!(completed.saved && completed.clean, "unchanged successful snapshot must mark its document saved");
        assert!(!app.application.host_document_info().unwrap().dirty);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);

        // A native save may finish after tab activation. Its source remains owned
        // by the old document, and completion cannot clean the new active tab.
        let save = app.application.host_prepare_document_save(&mut app.session, original.document).unwrap();
        let loaded = app.application.host_load_document(&mut app.session, original.document, &bytes, Some(path.clone()), false).unwrap();
        assert_ne!(loaded.document, original.document, "loaded files require fresh callback identities even with identical map IDs");
        assert!(app.application.host_prepare_document_save(&mut app.session, original.document).is_err());
        let completed = app.application.host_complete_document_save(&mut app.session, save.ticket, true, Some(path)).unwrap();
        assert_eq!(completed.document, original.document);
        assert!(completed.saved && completed.clean);
        assert_eq!(app.application.host_document_info().unwrap().document, loaded.document);
        assert!(app.application.host_load_document(&mut app.session, original.document, &bytes, None, false).is_err(), "stale open completion must not replace another active document");
        app.application.editor.check_host_save_conflict_for_test(&mut app.session.ui, &mut app.session.context);
        app.update(host, Duration::from_millis(16));
        assert_eq!(gpu.transfers().readback_bytes, before, "document transfers use CPU source bytes, never scene color capture");
    }

    fn check_native_region_contexts(
        app: &mut UiApplicationAdapter<NativeCreatorApplication>,
        host: &mut HostContext,
        images: &mut ImageRegistry,
        gpu: &Arc<GpuContext>,
        renderer: &mut PaintRenderer,
        output: &Arc<GpuImage>,
    ) {
        for (item, two_d) in [(false, false), (true, false), (false, true), (true, true)] {
            let fixture = app.application.editor.begin_native_region_context_for_test(
                item,
                two_d,
                &mut app.session.ui,
                &mut app.session.context,
            );
            let before = gpu.transfers().readback_bytes;
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                app.update(host, Duration::from_millis(16));
                let list = app.frame(host, images).unwrap();
                assert!(
                    app.application.pending.as_ref().unwrap().world_for_test(),
                    "actor selection must retain its native region canvas"
                );
                renderer
                    .render(
                        &list,
                        images,
                        &output.view,
                        output.descriptor.format,
                        (1200, 800),
                        1.,
                        [0.; 4],
                    )
                    .unwrap();
                app.presented(host);
                if app
                    .application
                    .editor
                    .native_region_context_ready_for_test(&fixture)
                {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "actor's region canvas never became ready"
                );
            }
            assert_eq!(gpu.transfers().readback_bytes, before);
            assert!(
                !app.application
                    .editor
                    .native_world_has_cpu_storage_for_test()
            );
            app.application
                .editor
                .finish_native_region_context_for_test(
                    fixture,
                    &mut app.session.ui,
                    &mut app.session.context,
                );
            app.update(host, Duration::from_millis(16));
        }
    }

    fn assert_no_viewport_readback(before: renderkit::TransferCounters, gpu: &GpuContext) {
        let after = gpu.transfers();
        assert_eq!(
            (after.readback_bytes - after.asset_bake_readback_bytes,
                after.capture_requests - after.asset_bake_readback_requests),
            (before.readback_bytes - before.asset_bake_readback_bytes,
                before.capture_requests - before.asset_bake_readback_requests),
            "normal viewport transfer changed: before={before:?}, after={after:?}",
        );
        if before.readback_bytes != after.readback_bytes {
            eprintln!("Concurrent explicit asset transfers: before={before:?}, after={after:?}");
        }
    }

    fn check_native_bake(
        app: &mut UiApplicationAdapter<NativeCreatorApplication>,
        host: &mut HostContext,
        images: &mut ImageRegistry,
        gpu: &Arc<GpuContext>,
        renderer: &mut PaintRenderer,
        output: &Arc<GpuImage>,
    ) {
        crate::actions::orthographic_bake::tests::check_parent_bake_commands_and_history(gpu.clone());
        let mut fixture = app
            .application
            .editor
            .begin_native_bake_for_test(&mut app.session.ui, &mut app.session.context);
        let mut previous = None;
        for phase in 0..5 {
            let mut phase_pending = true;
            let before = gpu.transfers().readback_bytes;
            let deadline = Instant::now() + Duration::from_secs(10);
            let (image, size) = loop {
                app.update(host, Duration::from_millis(16));
                if phase_pending {
                    app.application.editor.set_native_bake_phase_for_test(
                        &mut fixture,
                        phase,
                        &mut app.session.ui,
                        &mut app.session.context,
                    );
                    phase_pending = false;
                }
                let paint = app.frame(host, images).unwrap();
                let native_receipt = app.application.pending.as_ref().unwrap().world_for_test();
                assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
                assert_eq!(gpu.transfers().readback_bytes, before);

                renderer
                    .render(
                        &paint,
                        images,
                        &output.view,
                        output.descriptor.format,
                        (1200, 800),
                        1.,
                        [0.; 4],
                    )
                    .unwrap();
                app.presented(host);
                if let Some(image) = app.application.editor.native_bake_image_for_test(&fixture) {
                    assert!(native_receipt, "completed persisted bake must have a native receipt");
                    break image;
                }
                assert!(
                    Instant::now() < deadline,
                    "persisted bake never became ready"
                );
            };
            assert_eq!(gpu.transfers().readback_bytes, before);
            assert!(
                !app.application
                    .editor
                    .native_world_has_cpu_storage_for_test()
            );
            app.application.editor.check_native_bake_pick_for_test(&fixture);
            let mut capture = ReadbackTicket::capture(gpu, images.get(image).unwrap()).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .unwrap();
            let pixels = capture.try_finish(gpu).unwrap().unwrap();
            let reference = app
                .application
                .editor
                .native_bake_reference_for_test(gpu, size);
            let delta = pixels
                .pixels()
                .chunks_exact(4)
                .zip(reference.chunks_exact(4))
                .flat_map(|(a, b)| (0..3).map(move |i| a[i].abs_diff(b[i])))
                .max()
                .unwrap();
            eprintln!("Persisted native bake phase{phase}: max reference delta={delta}");
            for (suffix, data) in [
                ("native", pixels.pixels()),
                ("reference", reference.as_slice()),
            ] {
                let file = std::fs::File::create(format!(
                    "/private/tmp/eldiron-native-bake-{phase}-{suffix}.png"
                ))
                .unwrap();
                let mut encoder = png::Encoder::new(file, size.0, size.1);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(data)
                    .unwrap();
            }
            assert!(
                delta <= 1,
                "persisted bake reference RGB phase{phase}: {delta}"
            );
            if phase == 1 {
                assert!(pixels.pixels() != previous.as_ref().unwrap());
            }
            previous = Some(pixels.pixels().to_vec());
        }
        let (authored, old, revision) = app.application.editor.request_native_progressive_bake_for_test(
            &mut app.session.ui, &mut app.session.context);
        let transfers = gpu.transfers();
        let deadline = Instant::now() + Duration::from_secs(90);
        let mut progressive_frames = 0;
        loop {
            app.update(host, Duration::from_millis(16));
            let paint = app.frame(host, images).unwrap();
            assert!(app.application.pending.as_ref().unwrap().world_for_test(), "progressive bake must keep the viewport native");
            renderer.render(&paint, images, &output.view, output.descriptor.format, (1200,800), 1., [0.;4]).unwrap();
            app.presented(host);
            assert_no_viewport_readback(transfers, gpu);
            assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
            progressive_frames += 1;
            if app.application.editor.native_progressive_bake_complete_for_test(&authored, &old, revision) { break; }
            assert!(Instant::now() < deadline, "native progressive bake did not publish");
        }
        assert!(gpu.transfers().asset_bake_readback_requests > transfers.asset_bake_readback_requests);
        eprintln!("Native generated bake published after {progressive_frames} GPU frames");
        app.application.editor.check_native_bake_pick_for_test(&fixture);
        let (baked_image, baked_size) = app.application.editor.native_generated_bake_image_for_test();
        let mut capture = ReadbackTicket::capture(gpu, images.get(baked_image).unwrap()).unwrap();
        gpu.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(Duration::from_secs(30)) }).unwrap();
        let captured = capture.try_finish(gpu).unwrap().unwrap();
        let mut encoder = png::Encoder::new(std::fs::File::create("/private/tmp/eldiron-native-generated-bake.png").unwrap(), baked_size.0, baked_size.1);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(captured.pixels()).unwrap();
        app.application.editor.check_native_generated_bake_history_for_test(&old, &mut app.session.ui, &mut app.session.context);
        check_native_paste(app, host, images, gpu, renderer, output);
        let authored_paint = app.application.editor.stage_native_bake_paint_for_test();
        let (authored, old, revision) = app.application.editor.request_native_progressive_bake_for_test(
            &mut app.session.ui, &mut app.session.context);
        let transfers = gpu.transfers();
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            app.update(host, Duration::from_millis(16));
            let paint = app.frame(host, images).unwrap();
            assert!(app.application.pending.as_ref().unwrap().world_for_test(), "painted bake must keep the viewport native");
            renderer.render(&paint, images, &output.view, output.descriptor.format, (1200,800), 1., [0.;4]).unwrap();
            app.presented(host);
            assert_no_viewport_readback(transfers, gpu);
            assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
            if app.application.editor.native_progressive_bake_complete_for_test(&authored, &old, revision) { break; }
            assert!(Instant::now() < deadline, "painted progressive bake did not publish");
        }
        app.application.editor.check_native_bake_paint_for_test(&authored_paint, gpu);
        app.application.editor.check_native_generated_bake_history_for_test(&old, &mut app.session.ui, &mut app.session.context);
        std::fs::copy("/private/tmp/eldiron-native-generated-bake.eldiron", "/private/tmp/eldiron-native-painted-bake.eldiron").unwrap();
        // Undo/redo queues legitimate source changes. Settle them before
        // isolating a subsequent paint-only invalidation of a fresh request.
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            app.update(host, Duration::from_millis(16));
            if app.application.editor.native_world_source_for_test().is_some_and(|(_, ready)| ready)
                && app.application.editor.native_reference_source_ready_for_test() { break; }
            assert!(Instant::now() < deadline, "history sources must settle before pending-paint isolation");
            std::thread::sleep(Duration::from_millis(2));
        }
        let (_, retained, _) = app.application.editor.request_native_progressive_bake_for_test(
            &mut app.session.ui, &mut app.session.context);
        app.application.editor.log_native_bake_paint_state_for_test();
        let transfers = gpu.transfers();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            app.update(host, Duration::from_millis(16));
            let paint = app.frame(host, images).unwrap();
            renderer.render(&paint, images, &output.view, output.descriptor.format, (1200,800), 1., [0.;4]).unwrap();
            app.presented(host);
            assert_no_viewport_readback(transfers, gpu);
            if app.application.editor.native_bake_paint_pending_for_test() { break; }
            if Instant::now() >= deadline {
                app.application.editor.log_native_bake_paint_state_for_test();
                panic!("paint work never became pending");
            }
        }
        app.application.editor.edit_native_bake_paint_for_test(&mut app.session.context);
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            app.update(host, Duration::from_millis(16));
            let paint = app.frame(host, images).unwrap();
            assert!(app.application.pending.as_ref().unwrap().world_for_test()
                || app.application.editor.native_views_waiting(), "bake cancellation must remain native or pending");
            assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
            renderer.render(&paint, images, &output.view, output.descriptor.format, (1200,800), 1., [0.;4]).unwrap();
            app.presented(host);
            assert_no_viewport_readback(transfers, gpu);
            if app.application.editor.native_bake_paint_retired_for_test(&retained) { break; }
            assert!(Instant::now() < deadline, "paint-only edit must retire the frozen job");
        }
        app.application.editor.stage_native_large_bake_for_test();
        let transfers = gpu.transfers();
        let deadline = Instant::now() + Duration::from_secs(45);
        let mut pending_pages = 0;
        loop {
            app.update(host,Duration::from_millis(16));
            let paint = app.frame(host,images).unwrap();
            if !app.application.pending.as_ref().unwrap().world_for_test() { pending_pages += 1; }
            renderer.render(&paint,images,&output.view,output.descriptor.format,(1200,800),1.,[0.;4]).unwrap();
            app.presented(host);
            assert_no_viewport_readback(transfers,gpu);
            assert!(!app.application.editor.native_world_has_cpu_storage_for_test());
            assert!(gpu.source_pool_usage().bytes <= gpu.source_pool_usage().byte_limit);
            if app.application.editor.native_large_bake_ready_for_test() { break; }
            assert!(Instant::now()<deadline,"large persisted bake pages never completed");
        }
        assert!(pending_pages>0,"large composition must wait for its private source pages");
        eprintln!("Native 64-tile bake published after {pending_pages} pending root frames; source bytes={}",gpu.source_pool_usage().bytes);
        let (image,size)=app.application.editor.native_generated_bake_image_for_test();
        let mut capture=ReadbackTicket::capture(gpu,images.get(image).unwrap()).unwrap();
        gpu.device.poll(wgpu::PollType::Wait { submission_index:None,timeout:Some(Duration::from_secs(30)) }).unwrap();
        let pixels=capture.try_finish(gpu).unwrap().unwrap();
        let reference=app.application.editor.native_bake_reference_for_test(gpu,size);
        let delta=pixels.pixels().chunks_exact(4).zip(reference.chunks_exact(4))
            .flat_map(|(a,b)|(0..3).map(move|i|a[i].abs_diff(b[i]))).max().unwrap();
        assert!(delta<=1,"large native bake reference delta={delta}");
        app.application.editor.finish_native_bake_for_test(
            fixture,
            &mut app.session.ui,
            &mut app.session.context,
        );
        app.update(host, Duration::from_millis(16));
    }

    fn check_native_custom_contexts(
        app: &mut UiApplicationAdapter<NativeCreatorApplication>,
        host: &mut HostContext,
        images: &mut ImageRegistry,
        gpu: &Arc<GpuContext>,
        renderer: &mut PaintRenderer,
        output: &Arc<GpuImage>,
    ) {
        for (kind, name) in [
            (0, "screen"),
            (1, "surface"),
            (2, "screen-stored-3d"),
            (3, "screen-iso"),
            (4, "surface-iso"),
            (5, "surface-stored-3d"),
        ] {
            let fixture = app.application.editor.begin_native_custom_for_test(
                kind,
                &mut app.session.ui,
                &mut app.session.context,
            );
            let mut previous_pixels = None;
            for phase in 0..match kind {
                1 => 4,
                3 | 4 => 2,
                _ => 3,
            } {
                let mut phase_pending = true;
                let before = gpu.transfers().readback_bytes;
                let deadline = Instant::now() + Duration::from_secs(10);
                let (plan, size) = loop {
                    app.update(host, Duration::from_millis(16));
                    if phase_pending {
                        // Set the controlled clock after the real update pump;
                        // wall time during reference capture may advance ticks.
                        app.application
                            .editor
                            .set_native_custom_phase_for_test(phase);
                        phase_pending = false;
                    }
                    let paint = app.frame(host, images).unwrap();
                    assert!(
                        app.application.pending.as_ref().unwrap().world_for_test(),
                        "{name} must use native publication"
                    );
                    renderer
                        .render(
                            &paint,
                            images,
                            &output.view,
                            output.descriptor.format,
                            (1200, 800),
                            1.,
                            [0.; 4],
                        )
                        .unwrap();
                    app.presented(host);
                    if let Some(plan) = app.application.editor.native_custom_plan_for_test(&fixture)
                    {
                        break plan;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "custom {name} never became ready"
                    );
                };
                assert_eq!(gpu.transfers().readback_bytes, before);
                assert!(
                    !app.application
                        .editor
                        .native_world_has_cpu_storage_for_test()
                );
                let target = GpuImage::new(gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
                renderer
                    .render(
                        &plan,
                        images,
                        &target.view,
                        target.descriptor.format,
                        size,
                        1.,
                        [0.; 4],
                    )
                    .unwrap();
                let mut capture = ReadbackTicket::capture(gpu, target).unwrap();
                gpu.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(30)),
                    })
                    .unwrap();
                let pixels = capture.try_finish(gpu).unwrap().unwrap();
                let reference = app
                    .application
                    .editor
                    .native_custom_reference_for_test(gpu, size);
                let delta = pixels
                    .pixels()
                    .chunks_exact(4)
                    .zip(reference.chunks_exact(4))
                    .flat_map(|(a, b)| (0..3).map(move |i| a[i].abs_diff(b[i])))
                    .max()
                    .unwrap();
                eprintln!("Custom {name} phase {phase}: max reference delta={delta}");
                for (suffix, data) in [
                    ("native", pixels.pixels()),
                    ("reference", reference.as_slice()),
                ] {
                    let file = std::fs::File::create(format!(
                        "/private/tmp/eldiron-native-custom-{name}-{suffix}.png"
                    ))
                    .unwrap();
                    let mut encoder = png::Encoder::new(file, size.0, size.1);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(data)
                        .unwrap();
                }
                assert!(delta <= 1, "custom {name} must preserve reference RGB");
                if let Some(previous) = previous_pixels {
                    assert!(
                        pixels.pixels() != previous,
                        "custom visual inputs must invalidate their GPU output without a map edit"
                    );
                }
                previous_pixels = Some(pixels.pixels().to_vec());
            }
            check_native_paste(app, host, images, gpu, renderer, output);
            app.application.editor.finish_native_custom_for_test(
                fixture,
                &mut app.session.ui,
                &mut app.session.context,
            );
            app.update(host, Duration::from_millis(16));
        }
    }

    fn check_native_surface_detail(
        app: &mut UiApplicationAdapter<NativeCreatorApplication>,
        host: &mut HostContext,
        images: &mut ImageRegistry,
        gpu: &Arc<GpuContext>,
        renderer: &mut PaintRenderer,
        output: &Arc<GpuImage>,
    ) {
        let fixture = app
            .application
            .editor
            .begin_native_surface_detail_for_test();
        let transfers = gpu.transfers();
        let deadline = Instant::now() + Duration::from_secs(10);
        let (view, point, image) = loop {
            app.update(host, Duration::from_millis(16));
            let paint = app.frame(host, images).unwrap();
            assert!(
                app.application.pending.as_ref().unwrap().world_for_test(),
                "surface detail stays native"
            );
            renderer
                .render(
                    &paint,
                    images,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(host);
            if let Some(frame) = app
                .application
                .editor
                .native_surface_detail_frame_for_test(&fixture)
            {
                break frame;
            }
            assert!(
                Instant::now() < deadline,
                "surface detail source never became ready"
            );
        };
        assert_eq!(gpu.transfers().readback_bytes, transfers.readback_bytes);
        assert!(
            !app.application
                .editor
                .native_world_has_cpu_storage_for_test()
        );
        let mut capture = ReadbackTicket::capture(gpu, images.get(image).unwrap().clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let pixels = capture.try_finish(gpu).unwrap().unwrap();
        let file =
            std::fs::File::create("/private/tmp/eldiron-native-surface-detail-parent.png").unwrap();
        let mut encoder = png::Encoder::new(file, pixels.width(), pixels.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels.pixels())
            .unwrap();
        for event in [
            TheEvent::RenderViewHoverChanged(view.clone(), point),
            TheEvent::RenderViewClicked(view.clone(), point),
            TheEvent::RenderViewUp(view, point),
        ] {
            app.session.context.ui.send(event);
            app.update(host, Duration::from_millis(16));
        }
        app.application
            .editor
            .check_native_surface_detail_selection_for_test(&fixture);
        // The selected child can disappear while the parent document remains.
        // Empty-view handling must not erase its newly painted parent world.
        let expected = app
            .application
            .editor
            .remove_native_surface_detail_child_for_test(&fixture);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            app.update(host, Duration::from_millis(16));
            let paint = app.frame(host, images).unwrap();
            assert!(
                app.application.pending.as_ref().unwrap().world_for_test(),
                "removing a profile must not clear the displayed parent world"
            );
            renderer
                .render(
                    &paint,
                    images,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(host);
            if app
                .application
                .editor
                .native_missing_detail_child_ready_for_test(&fixture, &expected)
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "parent world did not recover from a missing profile"
            );
        }
        app.application
            .editor
            .finish_native_surface_detail_for_test(fixture, &mut app.session.ui);
        app.update(host, Duration::from_millis(16));
        // Restore the ordinary world/cube acknowledgement before the enclosing
        // camera workflow continues using its actual navigation controls.
        let paint = app.frame(host, images).unwrap();
        renderer
            .render(
                &paint,
                images,
                &output.view,
                output.descriptor.format,
                (1200, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        app.presented(host);
    }

    fn check_native_paste(
        app: &mut UiApplicationAdapter<NativeCreatorApplication>,
        host: &mut HostContext,
        images: &mut ImageRegistry,
        gpu: &Arc<GpuContext>,
        renderer: &mut PaintRenderer,
        output: &Arc<GpuImage>,
    ) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            app.update(host, Duration::from_millis(16));
            if app.application.editor.native_world_source_for_test().is_some_and(|(_, ready)| ready)
                && app.application.editor.native_reference_source_ready_for_test() {
                break;
            }
            assert!(Instant::now() < deadline, "preceding authoring jobs must settle before isolating paste");
            // The retained reference worker advances on the editor's wall-clock
            // redraw cadence. A tight update-only loop can finish before its
            // next tick, even though the owned source has already completed.
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(app.application.editor.native_reference_source_ready_for_test(), "preceding authoring jobs must settle before isolating paste");
        let source = app.application.editor.reference_for_test().read().scene_handler.build_index.snapshot();
        let (owned_source, ready) = app.application.editor.native_world_source_for_test().unwrap();
        assert!(ready, "the application must publish its own completed authoring source");
        assert!(!Arc::ptr_eq(&source, &owned_source), "native geometry must not borrow the reference worker's publication");
        let before_transfers = gpu.transfers();
        let (view, vertices, authored) = app
            .application
            .editor
            .stage_native_paste_for_test(&mut app.session.ui, &mut app.session.context);
        app.update(host, Duration::from_millis(16));
        assert!(app.application.editor.native_paste_active_for_test());
        let size = *app.session.ui.get_render_view(&view.name).unwrap().dim();
        let point = Vec2::new(size.width / 2 + 55, size.height / 2 + 45);
        assert!(app.application.editor.handle_native_world_input(
            &TheEvent::RenderViewHoverChanged(view.clone(), point),
            &mut app.session.ui,
            &mut app.session.context
        ));
        let click = TheEvent::RenderViewClicked(view.clone(), point);
        assert!(
            app.application.editor.handle_native_world_input(
                &click,
                &mut app.session.ui,
                &mut app.session.context
            ),
            "unpresented paste must not commit"
        );
        for _ in 0..128 {
            app.update(host, Duration::from_millis(16));
            let list = app.frame(host, images).unwrap();
            assert!(app.application.pending.as_ref().unwrap().world_for_test());
            renderer
                .render(
                    &list,
                    images,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(host);
            // A 3D target requires an acknowledged camera/ray; the first hover
            // before any paste frame deliberately cannot produce one.
            assert!(app.application.editor.handle_native_world_input(
                &TheEvent::RenderViewHoverChanged(view.clone(), point),
                &mut app.session.ui, &mut app.session.context,
            ));
            if app.application.editor.native_paste_ready_for_test() {
                break;
            }
        }
        assert!(
            app.application.editor.native_paste_ready_for_test(),
            "paste must publish its own ready source"
        );
        assert_eq!(
            app.application.editor.native_paste_authored_for_test(),
            authored,
            "preview cannot enter authored data"
        );
        assert!(
            Arc::ptr_eq(
                &source,
                &app.application.editor.reference_for_test().read()
                    .scene_handler
                    .build_index
                    .snapshot()
            ),
            "paste must not replace the reference source"
        );
        assert!(Arc::ptr_eq(
            &owned_source,
            &app.application.editor.native_world_source_for_test().unwrap().0,
        ), "clipboard previews must retain the application's authoring source");
        assert_eq!(
            gpu.transfers().readback_bytes,
            before_transfers.readback_bytes
        );
        assert!(
            !app.session
                .ui
                .get_render_view(&view.name)
                .unwrap()
                .has_cpu_display_storage()
        );
        let preview = app
            .application
            .editor
            .native_paste_preview_for_test()
            .unwrap();
        assert_eq!(preview.0, vertices + 4);
        assert!(
            images.get(preview.1).is_some(),
            "paste output is a native GPU resource"
        );
        if app.application.editor.native_paste_mode_for_test() != crate::prelude::EditorViewMode::D2 {
            assert!(app.application.editor.native_paste_has_ghost_guides_for_test(),
                "the displayed 3D guides must include the clipboard projection");
        }
        let original_projection = app
            .application
            .editor
            .native_paste_projection_for_test()
            .unwrap();
        app.update(host, Duration::ZERO);
        assert!(
            Arc::ptr_eq(
                &original_projection,
                &app.application
                    .editor
                    .native_paste_projection_for_test()
                    .unwrap()
            ),
            "an unchanged paste must reuse its CPU projection"
        );
        let point = point + Vec2::new(38, 14);
        assert!(app.application.editor.handle_native_world_input(
            &TheEvent::RenderViewHoverChanged(view.clone(), point),
            &mut app.session.ui,
            &mut app.session.context
        ));
        let click = TheEvent::RenderViewClicked(view.clone(), point);
        assert!(
            app.application.editor.handle_native_world_input(
                &click,
                &mut app.session.ui,
                &mut app.session.context
            ),
            "a moved preview must be acknowledged again"
        );
        for _ in 0..128 {
            app.update(host, Duration::from_millis(16));
            let list = app.frame(host, images).unwrap();
            renderer
                .render(
                    &list,
                    images,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(host);
            // A 3D target requires an acknowledged camera/ray; the first hover
            // before any paste frame deliberately cannot produce one.
            assert!(app.application.editor.handle_native_world_input(
                &TheEvent::RenderViewHoverChanged(view.clone(), point),
                &mut app.session.ui, &mut app.session.context,
            ));
            if app.application.editor.native_paste_ready_for_test() {
                break;
            }
        }
        assert!(app.application.editor.native_paste_ready_for_test());
        let moved = app
            .application
            .editor
            .native_paste_projection_for_test()
            .unwrap();
        assert!(!Arc::ptr_eq(&moved, &original_projection));
        assert_ne!(
            [moved.vertices.last().unwrap().x, moved.vertices.last().unwrap().y],
            [original_projection.vertices.last().unwrap().x, original_projection.vertices.last().unwrap().y],
            "moving the preview must move its geometry on at least one map axis",
        );
        assert_eq!(
            app.application.editor.native_paste_authored_for_test(),
            authored
        );
        assert_eq!(
            gpu.transfers().readback_bytes,
            before_transfers.readback_bytes
        );
        // Requested test evidence only; normal preview display above had zero
        // color readback and no destination pixel storage.
        let preview = app
            .application
            .editor
            .native_paste_preview_for_test()
            .unwrap();
        let mut capture =
            ReadbackTicket::capture(gpu, images.get(preview.1).unwrap().clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let pixels = capture.try_finish(gpu).unwrap().unwrap();
        let mode = match app.application.editor.native_paste_mode_for_test() {
            crate::prelude::EditorViewMode::Orbit => "orbit",
            crate::prelude::EditorViewMode::Iso => "3d-iso",
            crate::prelude::EditorViewMode::FirstP => "first-person",
            _ if moved.camera == rusterix::MapCamera::TwoDIso => "isometric",
            _ => "topdown",
        };
        let path = format!("/private/tmp/eldiron-native-paste-{mode}.png");
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = png::Encoder::new(file, pixels.width(), pixels.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels.pixels())
            .unwrap();
        app.session.context.ui.send(click);
        app.update(host, Duration::from_millis(16));
        assert!(!app.application.editor.native_paste_active_for_test());
        assert_eq!(
            app.application.editor.native_world_vertices_for_test(),
            vertices + 4,
            "real paste dispatcher commits once"
        );
        assert!(!Arc::ptr_eq(
            &owned_source,
            &app.application.editor.native_world_source_for_test().unwrap().0,
        ), "an authored paste must replace the owned static geometry publication");
        for (command, expected) in [
            ("Undo", vertices),
            ("Redo", vertices + 4),
            ("Undo", vertices),
        ] {
            app.session.context.ui.send(TheEvent::StateChanged(
                TheId::named(command),
                TheWidgetState::Clicked,
            ));
            app.update(host, Duration::from_millis(16));
            assert_eq!(
                app.application.editor.native_world_vertices_for_test(),
                expected
            );
        }
        // Escape also revokes an unpresented projection without any authored edit.
        let (_, _, authored) = app
            .application
            .editor
            .stage_native_paste_for_test(&mut app.session.ui, &mut app.session.context);
        app.update(host, Duration::from_millis(16));
        app.session
            .context
            .ui
            .send(TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape)));
        app.update(host, Duration::from_millis(16));
        assert!(!app.application.editor.native_paste_active_for_test());
        assert_eq!(
            app.application.editor.native_paste_authored_for_test(),
            authored
        );
        for _ in 0..128 {
            app.update(host, Duration::from_millis(16));
            let list = app.frame(host, images).unwrap();
            renderer
                .render(
                    &list,
                    images,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(host);
            if app.application.editor.native_world_ready_for_test() {
                break;
            }
        }
        assert!(app.application.editor.native_world_ready_for_test());
        let retained = app.application.editor.native_world_source_for_test().unwrap().0;
        app.gpu_recreated(host, images).unwrap();
        assert!(Arc::ptr_eq(
            &retained,
            &app.application.editor.native_world_source_for_test().unwrap().0,
        ), "GPU reconstruction must preserve completed CPU authoring geometry");
        let list = app.frame(host, images).unwrap();
        renderer.render(&list, images, &output.view, output.descriptor.format, (1200, 800), 1., [0.; 4]).unwrap();
        app.presented(host);
        assert!(app.application.editor.native_world_ready_for_test());
    }
    fn check_asset_sources(
        gpu: &std::sync::Arc<GpuContext>,
        renderer: &mut PaintRenderer,
        context: &mut TheContext,
    ) {
        use theframework::paint::{DisplayCompositing, PaintOp};
        crate::minimap::native_scene::check_for_test(gpu, renderer);
        // Check populated picker assets through the actual GPU compositor in
        // this same assembled checkpoint, including crop/alpha/drag semantics.
        for (label, source, reference) in crate::docks::tiles::native_reference_cases_for_test()
            .into_iter()
            .chain(crate::minimap::native::reference_cases())
        {
            let size = (reference.dim().width as u32, reference.dim().height as u32);
            let mut images = theframework::render_wgpu::UiImages::new(gpu.clone());
            let mut paint = PaintList::default();
            paint.ops.push(PaintOp::DisplayCompositing(
                DisplayCompositing::LegacyOpaque,
            ));
            let mut style = TheClassicStyle::new();
            let mut painter = theframework::theui::paint::WidgetPainter {
                list: &mut paint,
                text: &mut context.text,
                images: &mut images,
                ui: &context.ui,
                theme: style.theme().as_mut(),
            };
            source
                .paint(&mut painter, [size.0 as f32, size.1 as f32])
                .unwrap();
            images.finish_frame(&paint).unwrap();
            let target = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
            renderer
                .render(
                    &paint,
                    &images.registry,
                    &target.view,
                    target.descriptor.format,
                    size,
                    1.,
                    [0.; 4],
                )
                .unwrap();
            let mut capture = ReadbackTicket::capture(&gpu, target).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .unwrap();
            let pixels = capture.try_finish(&gpu).unwrap().unwrap();
            let delta = pixels
                .pixels()
                .iter()
                .zip(reference.pixels())
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            // Procedural source AA can differ by one byte before a second
            // byte-rounding boundary in composition. The reviewed budget is
            // two for this generated-alpha case; hard edges/assets stay at one.
            let tolerance = if label == "particle source alpha" { 2 } else { 1 };
            if delta > tolerance {
                std::fs::write(
                    "/private/tmp/eldiron-source-mismatch-gpu.rgba",
                    pixels.pixels(),
                )
                .unwrap();
                std::fs::write(
                    "/private/tmp/eldiron-source-mismatch-reference.rgba",
                    reference.pixels(),
                )
                .unwrap();
                std::fs::write(
                    "/private/tmp/eldiron-source-mismatch.json",
                    serde_json::to_vec(
                        &serde_json::json!({"label": label,"size":size,"delta":delta}),
                    )
                    .unwrap(),
                )
                .unwrap();
                let differences = pixels.pixels().chunks_exact(4).zip(reference.pixels().chunks_exact(4))
                    .enumerate().filter(|(_, (actual, expected))| actual.iter().zip(expected.iter()).any(|(a,b)| a.abs_diff(*b) > 1))
                    .take(12).map(|(index, (actual, expected))| serde_json::json!({
                        "at": [index as u32 % size.0, index as u32 / size.0], "actual": actual, "expected": expected,
                    })).collect::<Vec<_>>();
                eprintln!(
                    "Source mismatch details: {}",
                    serde_json::json!({"label":label,"size":size,"differences":differences})
                );
            }
            assert!(
                delta <= tolerance,
                "{label} source preview differs from the CPU reference: max channel delta={delta}"
            );
            eprintln!("Native asset {label}: max reference delta={delta}");
            check_source_dpi(gpu, renderer, &paint, &images.registry, &reference, tolerance);
            images.clear();
        }
    }

    fn check_source_dpi(
        gpu: &std::sync::Arc<GpuContext>,
        renderer: &mut PaintRenderer,
        paint: &PaintList,
        images: &ImageRegistry,
        reference: &TheRGBABuffer,
        tolerance: u8,
    ) {
        for density in [1.25, 1.5, 1.75, 2.] {
            let size = (
                (reference.dim().width as f32 * density).round() as u32,
                (reference.dim().height as f32 * density).round() as u32,
            );
            let target = GpuImage::new(gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
            renderer
                .render(
                    paint,
                    images,
                    &target.view,
                    target.descriptor.format,
                    size,
                    density,
                    [0.; 4],
                )
                .unwrap();
            let mut capture = ReadbackTicket::capture(gpu, target).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .unwrap();
            let pixels = capture.try_finish(gpu).unwrap().unwrap();
            let mut delta = 0;
            let mut differences = Vec::new();
            for y in 0..size.1 {
                for x in 0..size.0 {
                    let sx = ((x as f32 + 0.5) / density).floor() as i32;
                    let sy = ((y as f32 + 0.5) / density).floor() as i32;
                    let expected = if sx < reference.dim().width && sy < reference.dim().height {
                        let offset =
                            (sx as usize + sy as usize * reference.dim().width as usize) * 4;
                        &reference.pixels()[offset..offset + 4]
                    } else {
                        &[0, 0, 0, 255]
                    };
                    let offset = (x as usize + y as usize * size.0 as usize) * 4;
                    if differences.len() < 8 && pixels.pixels()[offset..offset+4].iter().zip(expected.iter()).any(|(a,b)|a.abs_diff(*b)>tolerance) {
                        differences.push(serde_json::json!({"at":[x,y],"source":[sx,sy],"actual":&pixels.pixels()[offset..offset+4],"expected":expected}));
                    }
                    for channel in 0..4 {
                        delta = delta
                            .max(pixels.pixels()[offset + channel].abs_diff(expected[channel]));
                    }
                }
            }
            if delta > tolerance { eprintln!("Source DPI mismatch: {}",serde_json::json!({"size":size,"density":density,"differences":differences})); }
            assert!(
                delta <= tolerance,
                "source DPI={density}: max reference delta={delta}"
            );
            eprintln!("Native source DPI={density}: max reference delta={delta}");
        }
    }

}
