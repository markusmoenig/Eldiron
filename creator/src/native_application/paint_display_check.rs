//! First-replacement and settled-frame comparison using the reported project.
use super::*;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu};
use theframework::render_wgpu::PaintRenderer;

fn pixels(gpu: &GpuContext, image: std::sync::Arc<GpuImage>) -> Vec<u8> {
    let mut read = ReadbackTicket::capture(gpu, image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    read.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}

#[test]
#[ignore = "native Metal Gate replacement-frame diagnostics"]
fn gate_3d_replacement_has_no_full_view_flash() {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let project =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_projects/Gate.eldiron");
    let mut application = NativeCreatorApplication::new(
        vec!["creator".into(), project.to_string_lossy().into()],
        false,
    );
    application.start_services = false;
    let mut app = UiApplicationAdapter::new(application);
    let mut host = HostContext::new((1200, 800));
    app.initialize(&mut host, &mut images).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1200, 800)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    app.update(&mut host, Duration::from_millis(16));
    assert!(
        !app.application
            .editor
            .project_for_native_test()
            .regions
            .is_empty()
    );
    app.session.context.ui.send(TheEvent::IndexChanged(
        TheId::named("Editor View Switch"),
        3,
    ));
    let paint = super::tests::ready_world_frame(
        &mut app,
        &mut host,
        &mut images,
        &gpu,
        &mut renderer,
        &output,
    );
    renderer
        .render(
            &paint,
            &images,
            &output.view,
            output.descriptor.format,
            (1200, 800),
            1.,
            [0.; 4],
        )
        .unwrap();
    app.presented(&mut host);

    for action in [
        "tool.iso_paint",
        "tool.terrain",
        "geometry-refresh",
        "atlas-refresh",
    ] {
        match action {
            "geometry-refresh" => {
                app.application
                    .editor
                    .native_suspend_world_source_for_test();
            }
            "atlas-refresh" => {
                app.application.editor.native_force_atlas_refresh_for_test();
            }
            _ => app.session.context.ui.send(TheEvent::Custom(
                TheId::named("Set Tool"),
                TheValue::Text(action.into()),
            )),
        }
        let mut frames = Vec::new();
        let mut roots = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            app.update(&mut host, Duration::from_millis(16));
            let paint = app.frame(&mut host, &mut images).unwrap();
            let ready = app
                .application
                .pending
                .as_ref()
                .unwrap()
                .world_ready_for_test();
            renderer
                .render(
                    &paint,
                    &images,
                    &output.view,
                    output.descriptor.format,
                    (1200, 800),
                    1.,
                    [0.; 4],
                )
                .unwrap();
            app.presented(&mut host);
            if ready {
                let image = app.application.editor.native_display_image_for_test();
                let image = images.get(image).unwrap();
                let capture = pixels(&gpu, image.clone());
                frames.push(capture);
                roots.push(pixels(&gpu, output.clone()));
                if frames.len() == 5 {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "Gate {action} never settled");
        }
        for (kind, frames) in [("scene", frames), ("root", roots)] {
            let last = frames.last().unwrap();
            let mut flashes = false;
            for (frame, image) in frames.iter().enumerate() {
                let changed = image
                    .chunks_exact(4)
                    .zip(last.chunks_exact(4))
                    .filter(|(a, b)| a.iter().zip(b.iter()).any(|(a, b)| a.abs_diff(*b) > 4))
                    .count();
                eprintln!(
                    "Gate {action} {kind} frame {frame}: {changed}/{} changed pixels",
                    image.len() / 4
                );
                flashes |= changed * 100 >= image.len() / 4;
            }
            assert!(!flashes, "Gate {action} {kind} flashes on replacement");
        }
    }
}
