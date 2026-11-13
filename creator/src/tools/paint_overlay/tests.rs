use super::*;
use crate::hud::HudColors;
fn colors() -> HudColors { HudColors::from_theme(&TheBlackBlueTheme::new()) }
fn fixture(kind: usize) -> PaintOverlay {
    let mut spec = Spec::new(if kind == 5 { "brick" } else { "moss" }, kind == 1);
    spec.scale = 1.3;
    spec.mortar = 0.12;
    spec.colors = vec![
        [50, 90, 130, 255],
        [170, 100, 50, 120],
        [210, 220, 230, 0],
        [70, 150, 80, 255],
    ];
    let controls = Controls {
        title: "Retained Paint Source".into(),
        source: Some(Uuid::from_u128(55)),
        operation: match kind {
            2 => "erase",
            3 => "pick",
            4 => "select",
            _ => "paint",
        }
        .into(),
        mode: if kind == 1 { "stamp" } else { "coat" }.into(),
        shape: "soft".into(),
        clip: "surface".into(),
        rotation: 0.4,
        size: 1.25,
        opacity: 0.7,
        density: 0.65,
        jitter: 0.2,
        material: 17,
        colors: spec.colors.clone(),
        color: [80,120,180,255],
        spec: Some(spec),
        palette: ThePalette::default(),
        visible: kind != 3,
    };
    PaintOverlay {
        controls: Some(controls),
        ..Default::default()
    }
}
fn setup(size: (i32, i32)) -> (TheContext, TheRGBABuffer) {
    let ctx = TheContext::new(size.0 as usize, size.1 as usize, 1.);
    let mut buffer = TheRGBABuffer::new(TheDim::sized(size.0, size.1));
    for p in buffer.pixels_mut().chunks_exact_mut(4) {
        p.copy_from_slice(&[12, 24, 36, 0]);
    }
    (ctx, buffer)
}
#[test]
fn reference_paint_panel_preserves_independent_pre_extraction_fixtures() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools/paint_overlay/fixtures");
    for width in [360, 640] {
        for kind in 0..6 {
            let (mut ctx, mut buffer) = setup((width, 360));
            fixture(kind).draw(&mut buffer, &mut ctx, colors());
            let decoder = png::Decoder::new(std::io::BufReader::new(
                std::fs::File::open(dir.join(format!("paint-{kind}-{width}.png"))).unwrap(),
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

fn scope() -> PanelScope {
    PanelScope {
        document: Uuid::from_u128(101),
        tool: Uuid::from_u128(102),
        revision: 7,
    }
}
fn acknowledge(
    overlay: &mut PaintOverlay,
    frame: &PanelFrame<Option<Controls>, Action>,
    scope: PanelScope,
    size: (usize, usize),
) -> bool {
    #[cfg(feature = "native-ui")]
    {
        overlay.presented_paint(frame, &[frame.receipt()], scope, size)
    }
    #[cfg(not(feature = "native-ui"))]
    {
        overlay
            .frames
            .acknowledge_reference(frame, scope, size, &overlay.controls)
    }
}
#[test]
fn paint_panel_preparation_stages_frozen_actions_and_revokes_stale_or_hidden_input() {
    // Preparation is CPU-only: no TheContext, fonts, image buffer or GPU.
    let mut overlay = fixture(0);
    let size = (640, 360);
    let scope = scope();
    let point = Vec2::new(30, 78);
    let frame = overlay.prepare_frame(scope, size, colors());
    assert!(matches!(
        overlay.hit_test(scope, size, point),
        PanelHit::Unavailable
    ));
    assert!(acknowledge(&mut overlay, &frame, scope, size));
    assert!(matches!(
        overlay.hit_test(scope, size, point),
        PanelHit::Inside(Some((_, Action::Mode(0))))
    ));
    assert!(matches!(
        overlay.hit_test(scope, size, Vec2::new(0, 0)),
        PanelHit::Outside
    ));
    let equivalent = overlay.prepare_frame(scope, size, colors());
    assert!(
        matches!(overlay.hit_test(scope, size, point), PanelHit::Inside(_)),
        "preparing identical input leaves acknowledged controls valid"
    );
    assert!(acknowledge(&mut overlay, &equivalent, scope, size));
    assert!(!acknowledge(&mut overlay, &frame, scope, size));
    overlay.controls.as_mut().unwrap().size = 3.;
    assert!(matches!(
        overlay.hit_test(scope, size, point),
        PanelHit::Unavailable
    ));
    assert!(!acknowledge(&mut overlay, &equivalent, scope, size));
    let changed = overlay.prepare_frame(scope, size, colors());
    assert!(acknowledge(&mut overlay, &changed, scope, size));
    for other in [
        PanelScope {
            document: Uuid::new_v4(),
            ..scope
        },
        PanelScope {
            tool: Uuid::new_v4(),
            ..scope
        },
        PanelScope {
            revision: scope.revision + 1,
            ..scope
        },
    ] {
        assert!(matches!(
            overlay.hit_test(other, size, point),
            PanelHit::Unavailable
        ));
    }
    assert!(matches!(
        overlay.hit_test(scope, (641, 360), point),
        PanelHit::Unavailable
    ));
    let tiny = overlay.prepare_frame(scope, (300, 360), colors());
    assert!(tiny.commands.is_empty());
    assert!(matches!(
        overlay.hit_test(scope, size, point),
        PanelHit::Unavailable
    ));
    assert!(!acknowledge(&mut overlay, &changed, scope, size));
    assert!(!acknowledge(&mut overlay, &tiny, scope, (300, 360)));
    let resumed = overlay.prepare_frame(scope, size, colors());
    overlay.invalidate_frame();
    assert!(!acknowledge(&mut overlay, &resumed, scope, size));
    let resumed = overlay.prepare_frame(scope, size, colors());
    assert!(acknowledge(&mut overlay, &resumed, scope, size));
    overlay.controls = None;
    let hidden = overlay.prepare_frame(scope, size, colors());
    assert!(hidden.commands.is_empty());
    assert!(matches!(
        overlay.hit_test(scope, size, point),
        PanelHit::Unavailable
    ));
    assert!(!acknowledge(&mut overlay, &resumed, scope, size));
}
#[test]
fn reference_paint_panel_actions_and_pointer_capture_never_retarget_stale_controls() {
    let (mut ctx, mut buffer) = setup((640, 360));
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.ui.state_events_sender = Some(tx);
    let mut ui = TheUI::new();
    let server = ServerContext::default();
    let mut overlay = fixture(0);
    overlay.draw(&mut buffer, &mut ctx, colors());
    assert!(overlay.map_event(
        &MapEvent::MapClicked(Vec2::new(30, 78)),
        &mut ui,
        &mut ctx,
        &server
    ));
    assert!(
        matches!(rx.try_recv(), Ok(TheEvent::IndexChanged(id,0)) if id.name == super::super::paint_picker::MODE)
    );
    assert!(overlay.map_event(
        &MapEvent::MapDragged(Vec2::new(600, 330)),
        &mut ui,
        &mut ctx,
        &server
    ));
    assert!(overlay.map_event(
        &MapEvent::MapUp(Vec2::new(600, 330)),
        &mut ui,
        &mut ctx,
        &server
    ));
    overlay.controls.as_mut().unwrap().source = Some(Uuid::new_v4());
    assert!(overlay.map_event(
        &MapEvent::MapClicked(Vec2::new(30, 78)),
        &mut ui,
        &mut ctx,
        &server
    ));
    assert!(
        rx.try_recv().is_err(),
        "stale source cannot receive an old action"
    );
    assert!(overlay.map_event(
        &MapEvent::MapUp(Vec2::new(600, 330)),
        &mut ui,
        &mut ctx,
        &server
    ));
    assert!(!overlay.map_event(
        &MapEvent::MapClicked(Vec2::new(600, 330)),
        &mut ui,
        &mut ctx,
        &server
    ));
    overlay.controls = None;
    overlay.draw(&mut buffer, &mut ctx, colors());
    assert!(!overlay.bounds.is_valid());
}
#[test]
#[cfg(feature = "native-render")]
#[ignore = "actual Metal composition, fractional density and device reconstruction"]
fn native_paint_panel_composes_all_reference_modes_and_reconstructs_without_display_bitmaps() {
    use crate::editor_overlay::test_support::*;
    use theframework::paint::*;
    let mut gpu = GpuPanel::new();
    let scope = scope();
    for width in [360, 640] {
        for kind in 0..6 {
            let size = (width, 360);
            let mut overlay = fixture(kind);
            let (ctx, _) = setup((width as i32, 360));
            let frame = overlay.prepare_frame(scope, size, colors());
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
                "no panel-sized or thumbnail bitmap"
            );
            assert!(!overlay.presented_paint(&frame, &[], scope, size));
            let pixels = gpu.render(&list, size, 1.);
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "src/tools/paint_overlay/fixtures/paint-{kind}-{width}.png"
            ));
            gpu.compare_reference(
                &frame.commands,
                &list,
                &ctx,
                &pixels,
                &expected(&path),
                &format!("paint-{kind}-{width}"),
            );
            assert!(overlay.presented_paint(
                &frame,
                &list.receipts().collect::<Vec<_>>(),
                scope,
                size
            ));
        }
    }
    let size = (640, 360);
    let mut overlay = fixture(5);
    let mut baseline = None;
    for density in [1., 1.25, 1.5, 2.] {
        overlay.invalidate_frame();
        let frame = overlay.prepare_frame(scope, size, colors());
        let mut list = frame
            .paint(&mut gpu.text, &mut gpu.images, density)
            .unwrap();
        list.ops.insert(
            0,
            PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
        );
        let pixels = gpu.render(&list, size, density);
        assert!(overlay.presented_paint(&frame, &list.receipts().collect::<Vec<_>>(), scope, size));
        if density == 1. {
            baseline = Some(pixels);
        }
    }
    let old = overlay.prepare_frame(scope, size, colors());
    overlay.invalidate_frame();
    let mut replacement = GpuPanel::new();
    assert!(!overlay.presented_paint(&old, &[old.receipt()], scope, size));
    let frame = overlay.prepare_frame(scope, size, colors());
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
        "every RGBA pixel including Swash text survives a real device replacement"
    );
    assert!(overlay.presented_paint(&frame, &list.receipts().collect::<Vec<_>>(), scope, size));
}

#[test]
fn paint_panel_native_routing_keeps_actions_and_distinguishes_unavailable_from_outside() {
    let (mut ctx, _) = setup((640, 360));
    let mut ui = TheUI::new();
    let server = ServerContext::default();
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.ui.state_events_sender = Some(tx);
    let mut overlay = fixture(0);
    let scope = scope();
    let size = (640, 360);
    let frame = overlay.prepare_frame(scope, size, colors());
    let click = MapEvent::MapClicked(Vec2::new(30, 78));
    assert_eq!(
        overlay.route_map_event(scope, size, &click, &mut ui, &mut ctx, &server),
        PanelEvent::Unavailable
    );
    assert!(acknowledge(&mut overlay, &frame, scope, size));
    assert_eq!(
        overlay.route_map_event(scope, size, &click, &mut ui, &mut ctx, &server),
        PanelEvent::Consumed
    );
    assert!(
        matches!(rx.try_recv(), Ok(TheEvent::IndexChanged(id,0)) if id.name == super::super::paint_picker::MODE)
    );
    overlay.controls.as_mut().unwrap().source = Some(Uuid::new_v4());
    assert_eq!(
        overlay.route_map_event(
            scope,
            size,
            &MapEvent::MapDragged(Vec2::new(600, 330)),
            &mut ui,
            &mut ctx,
            &server
        ),
        PanelEvent::Consumed
    );
    assert_eq!(
        overlay.route_map_event(
            scope,
            size,
            &MapEvent::MapUp(Vec2::new(600, 330)),
            &mut ui,
            &mut ctx,
            &server
        ),
        PanelEvent::Consumed
    );
    assert_eq!(
        overlay.route_map_event(scope, size, &click, &mut ui, &mut ctx, &server),
        PanelEvent::Unavailable
    );
    assert!(rx.try_recv().is_err());
    let frame = overlay.prepare_frame(scope, size, colors());
    assert!(acknowledge(&mut overlay, &frame, scope, size));
    assert_eq!(
        overlay.route_map_event(
            scope,
            size,
            &MapEvent::MapClicked(Vec2::new(600, 330)),
            &mut ui,
            &mut ctx,
            &server
        ),
        PanelEvent::Outside
    );
}
#[test]
fn reference_paint_panel_clips_its_original_bounds_at_small_viewports() {
    for size in [(340, 260), (360, 260), (360, 275), (360, 299)] {
        let (mut ctx, mut buffer) = setup(size);
        let mut overlay = fixture(5);
        overlay.draw(&mut buffer, &mut ctx, colors());
        assert_eq!(buffer.pixels().len(), size.0 as usize * size.1 as usize * 4);
        assert!(matches!(
            overlay.hit_test(
                overlay.reference_scope(),
                (size.0 as usize, size.1 as usize),
                Vec2::new(20, size.1)
            ),
            PanelHit::Outside
        ));
    }
}
