use super::*;
pub(super) fn fixture(kind: usize) -> EditorDisplaySession {
    let points = vec![
        Vec2::new(0., -0.5),
        Vec2::new(1.25, -0.5),
        Vec2::new(0.7, 1.),
        Vec2::new(0., 2.5),
    ];
    let mut profile = EditorProfile2D::new("Revolve profile", points.clone());
    profile.presets = vec![
        EditorProfilePreset::new("Original", points),
        EditorProfilePreset::new(
            "Vase",
            vec![
                Vec2::new(0., -0.5),
                Vec2::new(0.8, 0.),
                Vec2::new(0.4, 1.5),
                Vec2::new(0.75, 2.5),
            ],
        ),
        EditorProfilePreset::new("Cone", vec![Vec2::new(1.25, -0.5), Vec2::new(0., 2.5)]),
    ];
    profile.mirror_preview = kind != 4;
    let mut session = EditorDisplaySession::new(
        Uuid::from_u128(501),
        ProjectContext::Prefab(Uuid::from_u128(502)),
        EditorDisplay::Profile2D(profile),
    );
    session.set_grid_subdivisions(if kind == 1 { 32. } else { 4. });
    if kind == 1 {
        session.selected_point = Some(2);
        session.hover_coord = Some(Vec2::new(112, 18));
    }
    if kind == 2 {
        session.set_profile_scale(1.6);
        session.dragging_profile_scale = true;
    }
    if kind == 3 {
        session.zoom = 1.35;
        session.pan = Vec2::new(18., -15.);
        session.hovered_point = Some(1);
    }
    if kind == 5 {
        session.apply_profile_preset(1);
        session.hover_coord = Some(Vec2::new(30, 132));
    }
    session
}
#[test]
fn reference_action_display_preserves_independent_pre_extraction_fixtures() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/editor_display/fixtures");
    for width in [320, 800] {
        for kind in 0..6 {
            let mut ctx = TheContext::new(width as usize, 360, 1.);
            let mut buffer = TheRGBABuffer::new(TheDim::sized(width, 360));
            fixture(kind).draw(&mut buffer, &mut ctx);
            let decoder = png::Decoder::new(std::io::BufReader::new(
                std::fs::File::open(dir.join(format!("profile-{kind}-{width}.png"))).unwrap(),
            ));
            let mut reader = decoder.read_info().unwrap();
            let mut expected = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut expected).unwrap();
            assert_eq!(info.color_type, png::ColorType::Rgba);
            assert_eq!(
                buffer.pixels(),
                &expected[..info.buffer_size()],
                "{kind}/{width}"
            );
        }
    }
}

#[test]
#[cfg(feature = "native-render")]
#[ignore = "actual Metal profile composition, density lifecycle and device reconstruction"]
fn native_profile_composes_independent_fixtures_and_recovers_without_display_bitmaps() {
    use crate::editor_overlay::{panel::PanelScope, test_support::*};
    use theframework::paint::*;
    let mut gpu = GpuPanel::new();
    let scope = PanelScope {
        document: Uuid::from_u128(701),
        tool: Uuid::from_u128(702),
        revision: 1,
    };
    let target = |session: &EditorDisplaySession, size| native::DisplayTarget {
        scope,
        owner: session.owner,
        context: session.project_context,
        size,
        view_name: "PrefabView",
        visible: true,
    };
    for width in [320, 800] {
        for kind in 0..6 {
            let size = (width, 360);
            let ctx = TheContext::new(width, 360, 1.);
            let mut session = fixture(kind);
            let binding = target(&session, size);
            let frame = session.prepare_frame(binding).unwrap();
            assert!(session.last_transform.is_none());
            let uploads = gpu.gpu.transfers().uploaded_bytes;
            let mut list = frame.paint(&mut gpu.text, &mut gpu.images, 1.).unwrap();
            list.ops.insert(
                0,
                PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
            );
            list.validate().unwrap();
            assert_eq!(gpu.gpu.transfers().uploaded_bytes, uploads);
            assert_eq!(
                list.image_handles().count(),
                0,
                "no CPU profile display bitmap"
            );
            assert!(
                list.ops
                    .iter()
                    .any(|op| matches!(op, PaintOp::PixelLine { .. }))
            );
            assert!(list.ops.iter().any(|op| matches!(
                op,
                PaintOp::GlyphRun { .. } | PaintOp::OpaqueGlyphRun { .. }
            )));
            assert!(!session.presented_paint(&frame, &[], binding));
            let pixels = gpu.render(&list, size, 1.);
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "src/editor_display/fixtures/profile-{kind}-{width}.png"
            ));
            gpu.compare_reference(
                frame.commands(),
                &list,
                &ctx,
                &pixels,
                &expected(&path),
                &format!("profile-{kind}-{width}"),
            );
            assert!(session.presented_paint(&frame, &list.receipts().collect::<Vec<_>>(), binding));
        }
    }
    // These densities verify resource, clip and receipt lifecycles; full Creator
    // DPI appearance still requires comparison with the old RenderView scaling.
    let size = (800, 360);
    let mut session = fixture(3);
    let binding = target(&session, size);
    let mut baseline = None;
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        session.invalidate_frame();
        let frame = session.prepare_frame(binding).unwrap();
        let mut list = frame
            .paint(&mut gpu.text, &mut gpu.images, density)
            .unwrap();
        list.ops.insert(
            0,
            PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
        );
        list.validate().unwrap();
        let pixels = gpu.render(&list, size, density);
        assert!(session.presented_paint(&frame, &list.receipts().collect::<Vec<_>>(), binding));
        if density == 1. {
            baseline = Some(pixels);
        }
    }
    let old = session.prepare_frame(binding).unwrap();
    session.invalidate_frame();
    let mut replacement = GpuPanel::new();
    assert!(!session.presented_paint(&old, &[old.receipt()], binding));
    let frame = session.prepare_frame(binding).unwrap();
    let mut list = frame
        .paint(&mut replacement.text, &mut replacement.images, 1.)
        .unwrap();
    list.ops.insert(
        0,
        PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
    );
    assert_eq!(
        replacement.render(&list, size, 1.),
        baseline.unwrap(),
        "all profile RGBA pixels including Swash text survive actual device replacement"
    );
    assert!(session.presented_paint(&frame, &list.receipts().collect::<Vec<_>>(), binding));
}
