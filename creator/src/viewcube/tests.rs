use super::*;

pub(super) fn fixture(kind: usize) -> D3OrbitCamera {
    let mut camera = D3OrbitCamera::new();
    match kind {
        0 => {}
        1..=4 => {
            camera.azimuth = [0.4, 2.1, 3.6, 5.2][kind - 1];
            camera.elevation = [0.5, -0.6, 0.7, -0.5][kind - 1];
        }
        5..=10 => ViewCube::snap_camera(
            &mut camera,
            [
                ViewCubeFace::PositiveX,
                ViewCubeFace::NegativeX,
                ViewCubeFace::PositiveY,
                ViewCubeFace::NegativeY,
                ViewCubeFace::PositiveZ,
                ViewCubeFace::NegativeZ,
            ][kind - 5],
        ),
        11..=12 => {
            camera.azimuth = if kind == 11 { 0.0005 } else { 0.002 };
            camera.elevation = 0.;
        }
        _ => unreachable!(),
    }
    camera
}
fn backdrop(width: i32, height: i32) -> TheRGBABuffer {
    let mut image = TheRGBABuffer::new(TheDim::sized(width, height));
    for y in 0..height {
        for x in 0..width {
            image.set_pixel(
                x,
                y,
                &[
                    20 + (x % 23) as u8,
                    27 + (y % 19) as u8,
                    38 + ((x + y) % 31) as u8,
                    255,
                ],
            );
        }
    }
    image
}

#[test]
fn reference_viewcube_preserves_independent_original_fixtures() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/viewcube/fixtures");
    for (width, height) in [(100, 150), (800, 360)] {
        for kind in 0..13 {
            let mut ctx = TheContext::new(width as usize, height as usize, 1.);
            let mut image = backdrop(width, height);
            ViewCube::draw(&mut image, &mut ctx, &fixture(kind));
            let expected = crate::editor_overlay::test_support::expected(
                &path.join(format!("cube-{kind}-{width}.png")),
            );
            assert_eq!(image.pixels(), expected, "{kind}/{width}");
        }
    }
}

#[test]
#[cfg(feature = "native-render")]
#[ignore = "actual Metal ViewCube triangle, alpha, line and Swash output"]
fn native_viewcube_matches_independent_original_fixtures() {
    use crate::editor_overlay::test_support::*;
    use theframework::{paint::*, theui::paint::UiImageResources};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/viewcube/fixtures");
    let mut gpu = GpuPanel::new();
    let mut owner = ViewCubeFrames::default();
    let scope = crate::editor_overlay::panel::PanelScope {
        document: Uuid::from_u128(901),
        tool: Uuid::from_u128(902),
        revision: 1,
    };
    let target = |size| CubeTarget {
        scope,
        size,
        visible: true,
        view: Uuid::from_u128(903),
    };
    for (width, height) in [(100, 150), (800, 360)] {
        let size = (width as usize, height as usize);
        let ctx = TheContext::new(size.0, size.1, 1.);
        // Independent immutable world-placeholder source, never a composited cube.
        let source = theframework_core::ImageAsset::rgba(
            width as u32,
            height as u32,
            backdrop(width, height).pixels().to_vec(),
        )
        .unwrap();
        let background = gpu
            .images
            .asset(&format!("cube-test-background-{width}"), &source, false)
            .unwrap();
        for kind in 0..13 {
            let camera = fixture(kind);
            let binding = target(size);
            let frame = owner.prepare(binding, &camera).unwrap();
            let uploads = gpu.gpu.transfers().uploaded_bytes;
            let overlay = frame.paint(&mut gpu.text, &mut gpu.images, 1.).unwrap();
            assert!(!owner.presented(&frame, &[], binding, &camera));
            assert_eq!(overlay.image_handles().count(), 0);
            assert_eq!(gpu.gpu.transfers().uploaded_bytes, uploads);
            let mut list = PaintList::default();
            list.ops.push(PaintOp::DisplayCompositing(
                DisplayCompositing::LegacyOpaque,
            ));
            list.image(
                background,
                Rect::new(0., 0., width as f32, height as f32),
                Sampling::Nearest,
            );
            list.ops.extend(overlay.ops);
            let pixels = gpu.render(&list, size, 1.);
            gpu.compare_reference(
                frame.commands(),
                &list,
                &ctx,
                &pixels,
                &expected(&path.join(format!("cube-{kind}-{width}.png"))),
                &format!("cube-{kind}-{width}"),
            );
            assert!(owner.presented(
                &frame,
                &list.receipts().collect::<Vec<_>>(),
                binding,
                &camera
            ));
        }
    }

    let size = (800, 360);
    let binding = target(size);
    let camera = fixture(3);
    let source =
        theframework_core::ImageAsset::rgba(800, 360, backdrop(800, 360).pixels().to_vec())
            .unwrap();
    let background = gpu
        .images
        .asset("cube-test-background-800", &source, false)
        .unwrap();
    let mut baseline = None;
    // Resource and hit lifecycle coverage, not final old Creator DPI visual parity.
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        owner.invalidate();
        let frame = owner.prepare(binding, &camera).unwrap();
        let mut list = PaintList::default();
        list.ops.push(PaintOp::DisplayCompositing(
            DisplayCompositing::LegacyOpaque,
        ));
        list.image(background, Rect::new(0., 0., 800., 360.), Sampling::Nearest);
        list.ops.extend(
            frame
                .paint(&mut gpu.text, &mut gpu.images, density)
                .unwrap()
                .ops,
        );
        let pixels = gpu.render(&list, size, density);
        assert!(owner.presented(
            &frame,
            &list.receipts().collect::<Vec<_>>(),
            binding,
            &camera
        ));
        if density == 1. {
            baseline = Some(pixels);
        }
    }
    let old = owner.prepare(binding, &camera).unwrap();
    owner.invalidate();
    let mut replacement = GpuPanel::new();
    assert!(!owner.presented(&old, &[old.receipt()], binding, &camera));
    let background = replacement
        .images
        .asset("cube-test-background-800", &source, false)
        .unwrap();
    let frame = owner.prepare(binding, &camera).unwrap();
    let mut list = PaintList::default();
    list.ops.push(PaintOp::DisplayCompositing(
        DisplayCompositing::LegacyOpaque,
    ));
    list.image(background, Rect::new(0., 0., 800., 360.), Sampling::Nearest);
    list.ops.extend(
        frame
            .paint(&mut replacement.text, &mut replacement.images, 1.)
            .unwrap()
            .ops,
    );
    assert_eq!(
        replacement.render(&list, size, 1.),
        baseline.unwrap(),
        "every cube RGBA pixel including Swash labels survives actual device replacement"
    );
    assert!(owner.presented(
        &frame,
        &list.receipts().collect::<Vec<_>>(),
        binding,
        &camera
    ));
}
