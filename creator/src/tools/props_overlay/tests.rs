use super::*;
use crate::hud::HudColors;
fn fixture(kind: usize) -> (PropsOverlay, CreatorContext) {
    let mut server = CreatorContext::default();
    server.hud_colors = HudColors::from_theme(&TheBlackBlueTheme::new());
    server.shared.editor_view_mode = EditorViewMode::Orbit;
    server.shared.curr_block_asset_id = Some(if kind == 0 {
        crate::blocks::block_assets()[0].id
    } else {
        Uuid::from_u128(91)
    });
    server.shared.curr_block_asset_name = Some("Retained Props Source".into());
    server.shared.block_operation = kind as i32 % 3;
    server.shared.block_stroke_mode = kind as i32 % 2;
    server.shared.block_rotation_quarters = 3;
    server.shared.block_damage_enabled = true;
    let mut overlay = PropsOverlay {
        builtin: kind == 0,
        deletable: kind == 1,
        resizable: true,
        subdivisions: 4.,
        ..Default::default()
    };
    if kind >= 2 {
        let controls = if kind == 3 {
            let doc = denrim_materials::geometry::Document::parse(include_str!(
                "../../../embedded/meshes/recipes/fence.mzgeometry.json"
            ))
            .unwrap();
            props_recipe::published(&doc)
        } else {
            vec![]
        };
        overlay.recipe = Some((Uuid::from_u128(91), controls));
    }
    (overlay, server)
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
fn reference_props_panel_preserves_independent_pre_extraction_fixtures() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools/props_overlay/fixtures");
    for width in [360, 640] {
        for kind in 0..4 {
            let (mut ctx, mut buffer) = setup((width, 360));
            let (mut overlay, server) = fixture(kind);
            overlay.draw(&mut buffer, &mut ctx, &server);
            let decoder = png::Decoder::new(std::io::BufReader::new(
                std::fs::File::open(dir.join(format!("props-{kind}-{width}.png"))).unwrap(),
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
        document: Uuid::from_u128(201),
        tool: Uuid::from_u128(202),
        revision: 9,
    }
}
fn acknowledge(
    overlay: &mut PropsOverlay,
    frame: &PanelFrame<PropsSource, Action>,
    scope: PanelScope,
    size: (usize, usize),
    server: &CreatorContext,
) -> bool {
    #[cfg(feature = "native-ui")]
    {
        overlay.presented_paint(frame, &[frame.receipt()], scope, size, server)
    }
    #[cfg(not(feature = "native-ui"))]
    {
        overlay
            .frames
            .acknowledge_reference(frame, scope, size, &overlay.source(server))
    }
}
#[test]
fn props_panel_preparation_stages_actions_and_requires_current_document_asset_and_configuration() {
    let (mut overlay, mut server) = fixture(3);
    let size = (640, 360);
    let scope = scope();
    let point = Vec2::new(30, 70);
    // CPU-only preparation borrows configuration, not a UI context or renderer.
    let frame = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    assert!(matches!(
        overlay.hit_test(scope, size, point, &server),
        PanelHit::Unavailable
    ));
    assert!(acknowledge(&mut overlay, &frame, scope, size, &server));
    assert!(
        matches!(overlay.hit_test(scope, size, point, &server), PanelHit::Inside(Some((_, Action::Event(TheEvent::IndexChanged(id,0))))) if id.name == "Blocks Dock Operation")
    );
    let old = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    server.shared.curr_block_asset_id = Some(Uuid::new_v4());
    assert!(matches!(
        overlay.hit_test(scope, size, point, &server),
        PanelHit::Unavailable
    ));
    assert!(!acknowledge(&mut overlay, &old, scope, size, &server));
    let selected = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    assert!(acknowledge(&mut overlay, &selected, scope, size, &server));
    server.shared.block_rotation_quarters += 1;
    assert!(matches!(
        overlay.hit_test(scope, size, point, &server),
        PanelHit::Unavailable
    ));
    server.shared.block_rotation_quarters -= 1;
    overlay.recipe.as_mut().unwrap().1[0].control.value += 1.;
    assert!(matches!(
        overlay.hit_test(scope, size, point, &server),
        PanelHit::Unavailable
    ));
    let changed = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    assert!(acknowledge(&mut overlay, &changed, scope, size, &server));
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
            overlay.hit_test(other, size, point, &server),
            PanelHit::Unavailable
        ));
    }
    assert!(matches!(
        overlay.hit_test(scope, (360, 360), point, &server),
        PanelHit::Unavailable
    ));
    let tiny = overlay.prepare_frame(scope, (300, 360), crate::hud::colors(&server), &server);
    assert!(tiny.commands.is_empty());
    assert!(!acknowledge(&mut overlay, &changed, scope, size, &server));
    assert!(!acknowledge(
        &mut overlay,
        &tiny,
        scope,
        (300, 360),
        &server
    ));
    server.shared.editor_view_mode = EditorViewMode::D2;
    let hidden = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    assert!(hidden.commands.is_empty());
    assert!(matches!(
        overlay.hit_test(scope, size, point, &server),
        PanelHit::Unavailable
    ));
    server.shared.editor_view_mode = EditorViewMode::Orbit;
    let resumed = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    overlay.invalidate_frame();
    assert!(!acknowledge(&mut overlay, &resumed, scope, size, &server));
    let resumed = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    assert!(acknowledge(&mut overlay, &resumed, scope, size, &server));
}
#[test]
fn reference_props_panel_dispatches_existing_actions_and_consumes_stale_captured_gestures() {
    let (mut ctx, mut buffer) = setup((640, 360));
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.ui.state_events_sender = Some(tx);
    let mut ui = TheUI::new();
    let (mut overlay, mut server) = fixture(1);
    overlay.draw(&mut buffer, &mut ctx, &server);
    assert!(overlay.map_event(
        &MapEvent::MapClicked(Vec2::new(30, 70)),
        &mut ui,
        &mut ctx,
        &server
    ));
    assert!(
        matches!(rx.try_recv(), Ok(TheEvent::IndexChanged(id,0)) if id.name == "Blocks Dock Operation")
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
    server.shared.curr_block_asset_id = Some(Uuid::new_v4());
    assert!(overlay.map_event(
        &MapEvent::MapClicked(Vec2::new(30, 70)),
        &mut ui,
        &mut ctx,
        &server
    ));
    assert!(rx.try_recv().is_err());
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
    server.shared.editor_view_mode = EditorViewMode::D2;
    overlay.draw(&mut buffer, &mut ctx, &server);
    assert!(!overlay.bounds.is_valid());
}
#[test]
#[cfg(feature = "native-render")]
#[ignore = "actual Metal composition and device reconstruction"]
fn native_props_panel_composes_builtins_prefabs_and_recipes_without_display_bitmaps() {
    use crate::editor_overlay::test_support::*;
    use theframework::paint::*;
    let mut gpu = GpuPanel::new();
    let scope = scope();
    let mut baseline = None;
    for width in [360, 640] {
        for kind in 0..4 {
            let size = (width, 360);
            let (mut overlay, server) = fixture(kind);
            let (ctx, _) = setup((width as i32, 360));
            let frame = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
            let uploads = gpu.gpu.transfers().uploaded_bytes;
            let mut list = frame.paint(&mut gpu.text, &mut gpu.images, 1.).unwrap();
            list.ops.insert(
                0,
                PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
            );
            list.validate().unwrap();
            assert_eq!(gpu.gpu.transfers().uploaded_bytes, uploads);
            assert_eq!(list.image_handles().count(), 0);
            assert!(!overlay.presented_paint(&frame, &[], scope, size, &server));
            let pixels = gpu.render(&list, size, 1.);
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "src/tools/props_overlay/fixtures/props-{kind}-{width}.png"
            ));
            gpu.compare_reference(
                &frame.commands,
                &list,
                &ctx,
                &pixels,
                &expected(&path),
                &format!("props-{kind}-{width}"),
            );
            assert!(overlay.presented_paint(
                &frame,
                &list.receipts().collect::<Vec<_>>(),
                scope,
                size,
                &server
            ));
            if width == 640 && kind == 3 {
                baseline = Some(pixels);
            }
        }
    }
    let (mut overlay, server) = fixture(3);
    let size = (640, 360);
    for density in [1.25, 1.5, 2.] {
        let frame = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
        let mut list = frame
            .paint(&mut gpu.text, &mut gpu.images, density)
            .unwrap();
        list.ops.insert(
            0,
            PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
        );
        gpu.render(&list, size, density);
        assert!(overlay.presented_paint(
            &frame,
            &list.receipts().collect::<Vec<_>>(),
            scope,
            size,
            &server
        ));
        overlay.invalidate_frame();
    }
    let mut replacement = GpuPanel::new();
    let frame = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    let mut list = frame
        .paint(&mut replacement.text, &mut replacement.images, 1.)
        .unwrap();
    list.ops.insert(
        0,
        PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
    );
    assert_eq!(replacement.render(&list, size, 1.), baseline.unwrap());
    assert!(overlay.presented_paint(
        &frame,
        &list.receipts().collect::<Vec<_>>(),
        scope,
        size,
        &server
    ));
}

#[test]
fn props_panel_native_routing_preserves_events_and_blocks_unpresented_or_changed_sources() {
    let (mut ctx, _) = setup((640, 360));
    let mut ui = TheUI::new();
    let (mut overlay, mut server) = fixture(1);
    let (tx, rx) = std::sync::mpsc::channel();
    ctx.ui.state_events_sender = Some(tx);
    let scope = scope();
    let size = (640, 360);
    let frame = overlay.prepare_frame(scope, size, crate::hud::colors(&server), &server);
    let click = MapEvent::MapClicked(Vec2::new(30, 70));
    assert_eq!(
        overlay.route_map_event(scope, size, &click, &mut ui, &mut ctx, &server),
        PanelEvent::Unavailable
    );
    assert!(acknowledge(&mut overlay, &frame, scope, size, &server));
    assert_eq!(
        overlay.route_map_event(scope, size, &click, &mut ui, &mut ctx, &server),
        PanelEvent::Consumed
    );
    assert!(
        matches!(rx.try_recv(), Ok(TheEvent::IndexChanged(id,0)) if id.name == "Blocks Dock Operation")
    );
    server.shared.curr_block_asset_id = Some(Uuid::new_v4());
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
}
