//! Matched offscreen release profiling; run each backend in a fresh process.
//! This measures input/update through GPU completion, not physical display
//! latency. Reference CPU painting uses a persistent RGBA upload and the same
//! compositor/device/output as native painting; this is not the old Pixels host.
use super::*;
use crate::native_application::NativeCreatorApplication;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, wgpu};
use serde_json::{Value as Json, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use theframework::{
    paint::{PaintList, Rect, Sampling},
    platform::{GpuApplication, HostContext, UiApplicationAdapter},
    render_wgpu::PaintRenderer,
};

struct Reference {
    editor: Editor,
    ui: TheUI,
    context: TheContext,
    pixels: Vec<u8>,
    upload: Arc<GpuImage>,
    paint: PaintList,
}
enum Application {
    Native(Box<UiApplicationAdapter<NativeCreatorApplication>>),
    Reference(Box<Reference>),
}
impl Application {
    fn parts(&mut self) -> (&mut Editor, &mut TheUI, &mut TheContext) {
        match self {
            Self::Native(app) => {
                let app = app.as_mut();
                (
                    app.application.release_benchmark_editor(),
                    &mut app.session.ui,
                    &mut app.session.context,
                )
            }
            Self::Reference(app) => (&mut app.editor, &mut app.ui, &mut app.context),
        }
    }
    fn ready(&mut self, path: &std::path::Path) -> bool {
        let native = matches!(self, Self::Native(_));
        let (editor, _, _) = self.parts();
        editor
            .document_info()
            .is_some_and(|info| info.path.as_deref() == Some(path))
            && editor.native_reference_source_ready_for_test()
            && (!native || editor.native_world_ready_for_test())
    }
    fn input(&mut self, mode: &str, frame: usize) {
        let (editor, ui, context) = self.parts();
        // Redraw only; do not replace the tracker and accelerate game ticks.
        editor.update_tracker.force_redraw();
        let view = ui.get_render_view("PolyView").unwrap();
        let id = view.id().clone();
        let dim = *view.dim();
        match mode {
            "hover" => context.ui.send(TheEvent::RenderViewHoverChanged(
                id,
                Vec2::new(dim.width / 2 + frame as i32 % 40, dim.height / 2),
            )),
            "pan" => context
                .ui
                .send(TheEvent::RenderViewPreciseScrollBy(id, Vec2::new(2, 0))),
            "zoom" => context.ui.send(TheEvent::RenderViewZoomBy(id, 0.005)),
            "idle" => (),
            _ => unreachable!(),
        }
    }
    fn frame(
        &mut self,
        gpu: &GpuContext,
        host: &mut HostContext,
        registry: &mut ImageRegistry,
        renderer: &mut PaintRenderer,
        output: &GpuImage,
    ) -> Sample {
        let start = Instant::now();
        let update_ms;
        let paint_ms;
        let submit_ms;
        let operations;
        match self {
            Self::Native(app) => {
                app.update(host, Duration::from_millis(16));
                update_ms = milliseconds(start.elapsed());
                let phase = Instant::now();
                let paint = app.frame(host, registry).unwrap();
                operations = paint.ops.len();
                paint_ms = milliseconds(phase.elapsed());
                let phase = Instant::now();
                renderer
                    .render(
                        &paint,
                        registry,
                        &output.view,
                        output.descriptor.format,
                        host.physical_size,
                        host.density,
                        [0., 0., 0., 1.],
                    )
                    .unwrap();
                submit_ms = milliseconds(phase.elapsed());
                assert!(app.session.ui.canvas.buffer.pixels().is_empty());
                assert!(
                    !app.application
                        .release_benchmark_editor()
                        .native_world_has_cpu_storage_for_test()
                );
            }
            Self::Reference(app) => {
                app.ui.update(&mut app.context);
                app.editor.update_ui(&mut app.ui, &mut app.context);
                app.editor.update(&mut app.context);
                update_ms = milliseconds(start.elapsed());
                let phase = Instant::now();
                app.ui.draw(&mut app.pixels, &mut app.context);
                paint_ms = milliseconds(phase.elapsed());
                let phase = Instant::now();
                gpu.record_upload(app.pixels.len() as u64);
                gpu.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &app.upload.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &app.pixels,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(host.physical_size.0 * 4),
                        rows_per_image: Some(host.physical_size.1),
                    },
                    app.upload.texture.size(),
                );
                renderer
                    .render(
                        &app.paint,
                        registry,
                        &output.view,
                        output.descriptor.format,
                        host.physical_size,
                        host.density,
                        [0., 0., 0., 1.],
                    )
                    .unwrap();
                submit_ms = milliseconds(phase.elapsed());
                operations = app.paint.ops.len();
            }
        }
        let phase = Instant::now();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let completion_wait_ms = milliseconds(phase.elapsed());
        if let Self::Native(app) = self {
            app.presented(host);
        }
        Sample {
            update_ms,
            paint_ms,
            submit_ms,
            completion_wait_ms,
            total_ms: milliseconds(start.elapsed()),
            operations,
            draw_calls: renderer.frame_stats().draw_calls,
            continuation_frames: 0,
        }
    }
}
#[derive(serde::Serialize)]
struct Sample {
    update_ms: f64,
    paint_ms: f64,
    submit_ms: f64,
    completion_wait_ms: f64,
    total_ms: f64,
    operations: usize,
    draw_calls: usize,
    continuation_frames: usize,
}
fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.
}
fn distribution(values: impl Iterator<Item = f64>) -> Json {
    let mut values = values.collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    let percentile = |fraction: f64| values[((values.len() - 1) as f64 * fraction).ceil() as usize];
    json!({"p50":percentile(0.50),"p95":percentile(0.95),"p99":percentile(0.99),
        "mean":values.iter().sum::<f64>() / values.len() as f64})
}
fn resources(gpu: &GpuContext) -> Json {
    let transfers = gpu.transfers();
    let buffers = gpu.immutable_buffer_cache_usage();
    let images = gpu.image_allocation_usage();
    let targets = gpu.target_pool_usage();
    json!({"transfers":{"uploaded_bytes":transfers.uploaded_bytes,
        "readback_bytes":transfers.readback_bytes,"capture_requests":transfers.capture_requests,
        "asset_bake_readback_bytes":transfers.asset_bake_readback_bytes},
        "immutable_buffers":{"live_bytes":buffers.live_bytes,"peak_live_bytes":buffers.peak_live_bytes,
            "cached_gpu_bytes":buffers.gpu_bytes,"cached_cpu_bytes":buffers.cpu_bytes,
            "allocations":buffers.allocations,"uploaded_bytes":buffers.uploaded_bytes},
        "unpooled_images":{"live_bytes":images.live_bytes,"peak_live_bytes":images.peak_live_bytes,
            "allocations":images.allocations},"target_pool":format!("{targets:?}"),
        "source_pool":format!("{:?}",gpu.source_pool_usage()),
        "working_buffers":format!("{:?}",gpu.working_buffer_pool_usage())})
}
fn binding_usage(renderer: &PaintRenderer) -> Json {
    let usage = renderer.binding_cache_usage();
    json!({"groups":usage.groups,"estimated_bytes":usage.estimated_bytes,
        "group_limit":usage.group_limit,"byte_limit":usage.byte_limit,
        "created":usage.created,"hits":usage.hits,"evictions":usage.evictions})
}

#[test]
#[ignore = "matched offscreen release profiling; requires ELDIRON_BENCH_PROJECT_PATH and a native GPU"]
fn benchmark_creator_release_paths() {
    assert!(
        !cfg!(debug_assertions),
        "Performance evidence requires --release"
    );
    let path = std::fs::canonicalize(std::env::var("ELDIRON_BENCH_PROJECT_PATH").unwrap()).unwrap();
    let backend = std::env::var("ELDIRON_BENCH_BACKEND").unwrap_or_else(|_| "native".into());
    assert!(matches!(backend.as_str(), "native" | "reference"));
    let frames = std::env::var("ELDIRON_BENCH_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120usize)
        .clamp(10, 2000);
    let density = std::env::var("ELDIRON_BENCH_UI_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1f32);
    assert!(density.is_finite() && (0.5..=4.).contains(&density));
    let size = (
        (1600. * density).round() as u32,
        (1000. * density).round() as u32,
    );
    let started = Instant::now();
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let device_ms = milliseconds(started.elapsed());
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
    let mut host = HostContext::new(size);
    host.resize(size, density);
    let args = vec![
        "release-benchmark".into(),
        path.to_string_lossy().into_owned(),
    ];
    let mut application = if backend == "native" {
        let mut app =
            UiApplicationAdapter::new(NativeCreatorApplication::for_release_benchmark(args));
        app.initialize(&mut host, &mut registry).unwrap();
        Application::Native(Box::new(app))
    } else {
        let mut editor = Editor::new();
        editor.bind_native_reference_context(gpu.clone());
        let mut context = TheContext::new(1600, 1000, density);
        context.ui_render_scale = density;
        context.framebuffer_width = size.0 as usize;
        context.framebuffer_height = size.1 as usize;
        let mut ui = TheUI::new();
        ui.init(&mut context);
        ui.canvas.root = true;
        ui.canvas.set_dim(TheDim::sized(1600, 1000), &mut context);
        editor.init_ui(&mut ui, &mut context);
        editor.set_cmd_line_args(args, &mut context);
        let upload = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
        let image = registry.insert(upload.clone()).unwrap();
        let mut paint = PaintList::default();
        paint.image(image, Rect::new(0., 0., 1600., 1000.), Sampling::Nearest);
        Application::Reference(Box::new(Reference {
            editor,
            ui,
            context,
            pixels: vec![0; size.0 as usize * size.1 as usize * 4],
            upload,
            paint,
        }))
    };
    let mut first_ready_ms = None;
    let mut configured = false;
    let mut warmup_frames = 0;
    loop {
        application.input("idle", 0);
        application.frame(&gpu, &mut host, &mut registry, &mut renderer, &output);
        warmup_frames += 1;
        let (editor, _, context) = application.parts();
        if !configured
            && editor
                .document_info()
                .is_some_and(|info| info.path.as_deref() == Some(&path))
        {
            let map = &editor.documents.project.regions[0].map;
            assert!(
                !map.vertices.is_empty() || !map.geometry_objects.is_empty(),
                "Fixture must contain authored polygon or mesh geometry"
            );
            if std::env::var("ELDIRON_BENCH_VIEW").is_ok_and(|v| v == "3d") {
                context.ui.send(TheEvent::IndexChanged(
                    TheId::named("Editor View Switch"),
                    3,
                ));
            }
            configured = true;
        } else if configured && application.ready(&path) {
            first_ready_ms.get_or_insert_with(|| milliseconds(started.elapsed()));
            if warmup_frames >= 30 {
                break;
            }
        }
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "Benchmark did not reach a complete authored viewport"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    // A requested comparison image is an explicit capture outside timed frames.
    if let Ok(destination) = std::env::var("ELDIRON_BENCH_SNAPSHOT") {
        let mut capture = renderkit::ReadbackTicket::capture(&gpu, output.clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let image = capture.try_finish(&gpu).unwrap().unwrap();
        std::fs::write(
            destination,
            TheRGBABuffer::from(image.pixels().to_vec(), image.width(), image.height())
                .to_png()
                .unwrap(),
        )
        .unwrap();
    }
    let before = resources(&gpu);
    let bindings_before = binding_usage(&renderer);
    let glyphs_before = renderer.glyphs.rasterizations;
    let (editor, _, _) = application.parts();
    let region = &editor.documents.project.regions[0];
    let initial_camera = json!({"map_offset":region.map.offset.into_array(),
        "grid_size":region.map.grid_size,"position":region.editing_position_3d.into_array(),
        "look_at":region.editing_look_at_3d.into_array(),"iso_scale":editor.server_ctx.camera.iso_camera.scale});
    let transfers = gpu.transfers();
    let mut modes = serde_json::Map::new();
    for mode in ["idle", "hover", "pan", "zoom"] {
        let mut samples = Vec::with_capacity(frames);
        for frame in 0..frames {
            let input = Instant::now();
            application.input(mode, frame);
            let mut sample =
                application.frame(&gpu, &mut host, &mut registry, &mut renderer, &output);
            let due = Instant::now() + Duration::from_secs(30);
            let mut pending = 0;
            while !application.ready(&path) {
                let next =
                    application.frame(&gpu, &mut host, &mut registry, &mut renderer, &output);
                sample.update_ms += next.update_ms;
                sample.paint_ms += next.paint_ms;
                sample.submit_ms += next.submit_ms;
                sample.completion_wait_ms += next.completion_wait_ms;
                sample.operations += next.operations;
                sample.draw_calls += next.draw_calls;
                pending += 1;
                assert!(
                    Instant::now() < due,
                    "Input did not reach a complete native frame"
                );
            }
            sample.total_ms = milliseconds(input.elapsed());
            sample.continuation_frames = pending;
            samples.push(sample);
        }
        let summary = json!({"total_ms":distribution(samples.iter().map(|s|s.total_ms)),
            "update_ms":distribution(samples.iter().map(|s|s.update_ms)),
            "paint_ms":distribution(samples.iter().map(|s|s.paint_ms)),
            "submit_ms":distribution(samples.iter().map(|s|s.submit_ms)),
            "completion_wait_ms":distribution(samples.iter().map(|s|s.completion_wait_ms)),
            "draw_calls":distribution(samples.iter().map(|s|s.draw_calls as f64)),
            "continuation_frames":distribution(samples.iter().map(|s|s.continuation_frames as f64))});
        eprintln!("Release {backend} {mode}: {summary}");
        modes.insert(mode.into(), json!({"summary":summary,"samples":samples}));
    }
    if backend == "native" {
        let after = gpu.transfers();
        assert_eq!(
            after.readback_bytes - after.asset_bake_readback_bytes,
            transfers.readback_bytes - transfers.asset_bake_readback_bytes
        );
        assert_eq!(
            after.capture_requests - after.asset_bake_readback_requests,
            transfers.capture_requests - transfers.asset_bake_readback_requests
        );
    }
    let (editor, ui, context) = application.parts();
    let viewport = *ui.get_render_view("PolyView").unwrap().dim();
    let result = json!({"backend":backend,"profile":"release","project":path,
        "logical_size":[1600,1000],"physical_size":[size.0,size.1],"density":density,
        "viewport":[viewport.x,viewport.y,viewport.width,viewport.height],
        "view_mode":format!("{:?}",editor.server_ctx.shared.editor_view_mode),
        "map_camera":format!("{:?}",editor.documents.project.regions[0].map.camera),
        "ui_render_scale":context.ui_render_scale,"frames_per_mode":frames,"initial_camera":initial_camera,
        "glyph_rasterizations_before":glyphs_before,"glyph_rasterizations_after":renderer.glyphs.rasterizations,
        "gpu_adapter":format!("{:?}",gpu.adapter.get_info()),
        "device_init_ms":device_ms,"first_complete_view_ms":first_ready_ms,
        "warmup_frames":warmup_frames,"resources_before":before,"resources_after":resources(&gpu),
        "bindings_before":bindings_before,"bindings_after":binding_usage(&renderer),
        "modes":modes,"scope":"Offscreen input through GPU completion. CPU reference uses persistent RGBA upload plus shared compositor; excludes native window/acquisition/display, physical input latency and driver-wide memory. Phase CPU timings are not GPU timestamps."});
    let destination = std::env::var("ELDIRON_BENCH_OUTPUT").unwrap();
    std::fs::write(&destination, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    eprintln!("Release benchmark evidence: {destination}");
}
