//! Existing assembled Creator visibility comparison; no independent fixture.
use super::*;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu};
use std::sync::Arc;
use theframework::render_wgpu::PaintRenderer;

pub(super) fn check_native_visibility(
    app: &mut UiApplicationAdapter<NativeCreatorApplication>,
    host: &mut HostContext,
    images: &mut ImageRegistry,
    gpu: &Arc<GpuContext>,
    renderer: &mut PaintRenderer,
) {
    let before_source =
        serde_json::to_value(app.application.editor.project_for_native_test()).unwrap();
    let original = app.application.editor.set_native_visibility_for_test();
    let configured =
        serde_json::to_value(app.application.editor.project_for_native_test()).unwrap();
    let before = gpu.transfers().readback_bytes;
    let deadline = Instant::now() + Duration::from_secs(5);
    let root = loop {
        app.update(host, Duration::from_millis(16));
        let root = app.frame(host, images).unwrap();
        if app
            .application
            .editor
            .native_world_source_for_test()
            .is_some_and(|(_, ready)| ready)
        {
            break root;
        }
        assert!(
            Instant::now() < deadline,
            "visibility source did not become ready"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(
        app.application.pending.as_ref().unwrap().world_for_test(),
        "active visibility keeps the actual native world owner"
    );
    let root_output = GpuImage::new(gpu, ImageDescriptor::rgba(1200, 800)).unwrap();
    renderer
        .render(
            &root,
            images,
            &root_output.view,
            root_output.descriptor.format,
            (1200, 800),
            1.,
            [0.; 4],
        )
        .unwrap();
    app.presented(host);
    assert!(
        !app.application
            .editor
            .native_world_has_cpu_storage_for_test()
    );
    assert_eq!(
        gpu.transfers().readback_bytes,
        before,
        "visibility preparation does not capture scene color"
    );
    let (base, masked, size) = app.application.editor.native_visibility_plans_for_test();
    let capture = |list: &PaintList, density: f32, renderer: &mut PaintRenderer| {
        let physical = (
            (size.0 as f32 * density).ceil() as u32,
            (size.1 as f32 * density).ceil() as u32,
        );
        let output = GpuImage::new(gpu, ImageDescriptor::rgba(physical.0, physical.1)).unwrap();
        renderer
            .render(
                list,
                images,
                &output.view,
                output.descriptor.format,
                physical,
                density,
                [0.; 4],
            )
            .unwrap();
        let mut ticket = ReadbackTicket::capture(gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        ticket.try_finish(gpu).unwrap().unwrap()
    };
    let unmasked = capture(&base, 1., renderer);
    let mut reference = unmasked.pixels().to_vec();
    app.application
        .editor
        .native_visibility_reference_for_test(&mut reference, size);
    assert_ne!(
        reference,
        unmasked.pixels(),
        "fixture must exercise hidden cells"
    );
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        let painted = capture(&masked, density, renderer);
        let mut worst = 0;
        for y in 0..painted.height() {
            for x in 0..painted.width() {
                let sx = ((x as f32 + 0.5) / density).floor() as u32;
                let sy = ((y as f32 + 0.5) / density).floor() as u32;
                if sx >= size.0 || sy >= size.1 {
                    continue;
                }
                let src = ((sy * size.0 + sx) * 4) as usize;
                let dst = ((y * painted.width() + x) * 4) as usize;
                for channel in 0..4 {
                    worst = worst
                        .max(reference[src + channel].abs_diff(painted.pixels()[dst + channel]));
                }
            }
        }
        if worst > 1 {
            let physical_base = capture(&base, density, renderer);
            let mut expected = vec![0; painted.pixels().len()];
            let mut reported = 0;
            for y in 0..painted.height() {
                for x in 0..painted.width() {
                    let sx = ((x as f32 + 0.5) / density).floor() as u32;
                    let sy = ((y as f32 + 0.5) / density).floor() as u32;
                    if sx >= size.0 || sy >= size.1 {
                        continue;
                    }
                    let src = ((sy * size.0 + sx) * 4) as usize;
                    let dst = ((y * painted.width() + x) * 4) as usize;
                    expected[dst..dst + 4].copy_from_slice(&reference[src..src + 4]);
                    if reported < 12
                        && (0..4)
                            .any(|c| reference[src + c].abs_diff(painted.pixels()[dst + c]) > 1)
                    {
                        eprintln!(
                            "Visibility mismatch physical=({x},{y}) logical=({sx},{sy}) expected={:?} actual={:?} baseLogical={:?} basePhysical={:?}",
                            &reference[src..src + 4],
                            &painted.pixels()[dst..dst + 4],
                            &unmasked.pixels()[src..src + 4],
                            &physical_base.pixels()[dst..dst + 4]
                        );
                        reported += 1;
                    }
                }
            }
            for (name, pixels, width, height) in [
                ("base-logical", unmasked.pixels(), size.0, size.1),
                (
                    "base-physical",
                    physical_base.pixels(),
                    painted.width(),
                    painted.height(),
                ),
                (
                    "masked-physical",
                    painted.pixels(),
                    painted.width(),
                    painted.height(),
                ),
                (
                    "expected-physical",
                    expected.as_slice(),
                    painted.width(),
                    painted.height(),
                ),
            ] {
                std::fs::write(
                    format!("/private/tmp/eldiron-native-visibility-{name}.png"),
                    TheRGBABuffer::from(pixels.to_vec(), width, height)
                        .to_png()
                        .unwrap(),
                )
                .unwrap();
            }
        }
        eprintln!("Native 2D visibility DPI={density}: max reference delta={worst}");
        assert!(
            worst <= 1,
            "visibility must preserve CPU cell/blend semantics at density {density}"
        );
    }
    assert_eq!(
        serde_json::to_value(app.application.editor.project_for_native_test()).unwrap(),
        configured
    );
    app.application
        .editor
        .restore_native_visibility_for_test(original);
    assert_eq!(
        serde_json::to_value(app.application.editor.project_for_native_test()).unwrap(),
        before_source
    );
    // Keep the real root frame path exercised as well as isolated source pixels.
    assert!(root.validate().is_ok());
}
