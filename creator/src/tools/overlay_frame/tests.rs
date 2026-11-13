use super::*;

fn tool(kind: usize) -> Box<dyn Tool> {
    match kind {
        0 => Box::new(vertex::VertexTool::new()),
        1 => Box::new(linedef::LinedefTool::new()),
        2 => Box::new(sector::SectorTool::new()),
        3 => Box::new(entity::EntityTool::new()),
        4 => Box::new(geometry::GeometryTool::new()),
        5 => Box::new(blocks::BlockTool::new()),
        _ => unreachable!(),
    }
}
fn fixture(kind: usize, state: usize) -> (Map, ServerContext) {
    let mut map = Map::new();
    map.id = Uuid::from_u128(1001);
    map.vertices = vec![
        rusterix::Vertex::new(11, 0., 0.),
        rusterix::Vertex::new(12, 2., 0.),
        rusterix::Vertex::new(13, 2., 2.),
        rusterix::Vertex::new(14, 0., 2.),
    ];
    map.linedefs = vec![
        rusterix::Linedef::new(21, 11, 12),
        rusterix::Linedef::new(22, 12, 13),
        rusterix::Linedef::new(23, 13, 14),
        rusterix::Linedef::new(24, 14, 11),
    ];
    map.sectors = vec![rusterix::Sector::new(31, vec![21, 22, 23, 24])];
    let mut object = rusterix::GeometryObject::box_("Fixture box", Vec3::zero(), Vec3::one());
    object.id = Uuid::from_u128(40);
    map.geometry_objects.push(object);
    if state > 0 {
        map.selected_vertices = vec![11];
        map.selected_linedefs = vec![21];
        map.selected_sectors = vec![31];
        map.selected_geometry_objects = vec![Uuid::from_u128(40)];
        map.curr_rectangle = Some((Vec2::new(54.25, 68.5), Vec2::new(170.75, 187.5)));
    }
    let mut server = ServerContext::default();
    server.pc = ProjectContext::Region(map.id);
    server.editor_view_mode = if state == 0 {
        EditorViewMode::D2
    } else {
        EditorViewMode::Orbit
    };
    server.curr_map_tool_type = match kind {
        0 => MapToolType::Vertex,
        1 => MapToolType::Linedef,
        2 => MapToolType::Sector,
        4 => MapToolType::Selection,
        _ => MapToolType::General,
    };
    if state == 2 {
        let mut profile = Map::new();
        profile.id = Uuid::from_u128(1002);
        profile.vertices = vec![
            rusterix::Vertex::new(101, 0., 0.),
            rusterix::Vertex::new(102, 1., 0.),
            rusterix::Vertex::new(103, 1., 1.),
        ];
        profile.linedefs = vec![
            rusterix::Linedef::new(201, 101, 102),
            rusterix::Linedef::new(202, 102, 103),
            rusterix::Linedef::new(203, 103, 101),
        ];
        profile.sectors = vec![rusterix::Sector::new(301, vec![201, 202, 203])];
        profile.selected_vertices = vec![101];
        profile.selected_linedefs = vec![201];
        profile.selected_sectors = vec![301];
        let id = profile.id;
        map.profiles.insert(id, profile);
        let mut surface = rusterix::Surface::new(31);
        surface.profile = Some(id);
        server.geometry_edit_mode = GeometryEditMode::Detail;
        server.active_detail_surface = Some(surface);
    }
    (map, server)
}

#[test]
fn original_standard_tool_overlays_preserve_independent_fixtures() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools/overlay_frame/fixtures");
    let assets = rusterix::Assets::default();
    for width in [320, 800] {
        for kind in 0..6 {
            for state in 0..3 {
                let (mut map, mut server) = fixture(kind, state);
                let mut tool = tool(kind);
                let mut ctx = TheContext::new(width as usize, 360, 1.);
                let mut image = TheRGBABuffer::new(TheDim::sized(width, 360));
                image.fill([12, 24, 36, 255]);
                tool.draw_hud(&mut image, &mut map, &mut ctx, &mut server, &assets);
                let expected = crate::editor_overlay::test_support::expected(
                    &root.join(format!("tool-{kind}-{state}-{width}.png")),
                );
                assert_eq!(image.pixels(), expected, "{kind}/{state}/{width}");
            }
        }
    }
}

fn target(
    tool: &dyn Tool,
    _map: &Map,
    server: &ServerContext,
    size: (usize, usize),
) -> ToolOverlayTarget {
    ToolOverlayTarget {
        document: Uuid::from_u128(1101),
        tool: tool.id().uuid,
        view: Uuid::from_u128(1102),
        revision: 1,
        context: server.pc,
        size,
        visible: true,
    }
}
fn prepare(
    tool: &mut dyn Tool,
    target: ToolOverlayTarget,
    map: &Map,
    server: &ServerContext,
    ctx: &TheContext,
    assets: &Assets,
) -> ToolOverlayFrame {
    let slots = crate::actions::named_geometry_hud_material_slots_for_selection(map)
        .or_else(|| crate::actions::prefab_hud_material_slots_for_selected_instances(map));
    let light = ctx
        .ui
        .icon("light_small")
        .map(|icon| crate::hud::HudImageSource {
            width: icon.pixel_width(),
            height: icon.pixel_height(),
            pixels: icon.pixels(),
        });
    let style = crate::hud::HudStyle::current();
    tool.prepare_overlay(ToolOverlayInputs {
        target,
        hud: crate::hud::HudInputs {
            map,
            server,
            assets,
            selected_id: None,
            material_slots: slots.as_deref(),
            light,
        },
        style: &style,
    })
    .expect("converted standard tool")
}
#[test]
fn aggregate_preparation_preserves_original_draw_order_without_activating_input_or_mutating_sources()
 {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools/overlay_frame/fixtures");
    let assets = Assets::default();
    for width in [320, 800] {
        for kind in 0..6 {
            for state in 0..3 {
                let (map, server) = fixture(kind, state);
                let original = serde_json::to_value(&map).unwrap();
                let mut tool = tool(kind);
                let ctx = TheContext::new(width, 360, 1.);
                let target = target(tool.as_ref(), &map, &server, (width, 360));
                let frame = prepare(tool.as_mut(), target, &map, &server, &ctx, &assets);
                assert!(!frame.input_current(tool.as_mut(), target, &map, &server));
                let mut image = TheRGBABuffer::new(TheDim::sized(width as i32, 360));
                image.fill([12, 24, 36, 255]);
                frame.draw_reference(&mut image, &ctx);
                let expected = crate::editor_overlay::test_support::expected(
                    &root.join(format!("tool-{kind}-{state}-{width}.png")),
                );
                assert_eq!(image.pixels(), expected, "{kind}/{state}/{width}");
                assert_eq!(serde_json::to_value(&map).unwrap(), original);
                assert_eq!(server.selected_hud_icon_index, 0);
            }
        }
    }
}
#[test]
fn standard_tool_selection_rules_keep_detail_profile_overrides_and_geometry_identity() {
    for kind in 0..6 {
        for state in 0..3 {
            let (map, server) = fixture(kind, state);
            let tool = tool(kind);
            let expected = if state == 0 {
                None
            } else if state == 1 {
                [Some(11), Some(21), Some(31), Some(21), Some(40), None][kind]
            } else {
                [Some(101), Some(201), Some(301), Some(21), Some(40), None][kind]
            };
            assert_eq!(
                tool.hud_selected_id(&map, &server),
                expected,
                "{kind}/{state}"
            );
        }
    }
}
#[cfg(feature = "native-ui")]
fn markers(frame: &ToolOverlayFrame) -> Vec<theframework::paint::FrameReceipt> {
    frame
        .parts
        .iter()
        .filter_map(|part| match part {
            Part::Commands(_) => None,
            Part::Hud(frame) => Some(frame.token.receipt()),
            Part::Props(frame) => Some(frame.receipt()),
            Part::Terrain(frame) => Some(frame.receipt()),
            Part::Rect(frame) => Some(frame.receipt()),
            Part::Paint(frame) => Some(frame.receipt()),
        })
        .collect()
}
#[test]
#[cfg(feature = "native-ui")]
fn aggregate_presented_receipts_and_each_tool_source_are_required_before_input() {
    let assets = Assets::default();
    let ctx = TheContext::new(800, 360, 1.);
    for kind in 0..6 {
        let (map, mut server) = fixture(kind, 1);
        let mut tool = tool(kind);
        let target = target(tool.as_ref(), &map, &server, (800, 360));
        let frame = prepare(tool.as_mut(), target, &map, &server, &ctx, &assets);
        assert!(!frame.input_current(tool.as_mut(), target, &map, &server));
        assert!(!tool.presented_overlay(&frame, &[], target, &map, &mut server));
        assert!(!tool.presented_overlay(&frame, &markers(&frame), target, &map, &mut server));
        let fresh = prepare(tool.as_mut(), target, &map, &server, &ctx, &assets);
        assert!(tool.presented_overlay(&fresh, &markers(&fresh), target, &map, &mut server));
        assert!(fresh.input_current(tool.as_mut(), target, &map, &server));
        // Replaying a root cannot re-promote transient geometry.
        assert!(!tool.presented_overlay(&fresh, &markers(&fresh), target, &map, &mut server));
        assert!(!fresh.input_current(tool.as_mut(), target, &map, &server));
    }
}
#[test]
#[cfg(feature = "native-ui")]
fn aggregate_document_widget_tool_source_layout_and_profile_changes_never_rebind_old_input() {
    let assets = Assets::default();
    let ctx = TheContext::new(800, 360, 1.);
    let (mut map, mut server) = fixture(0, 2);
    let mut tool = tool(0);
    let binding = target(tool.as_ref(), &map, &server, (800, 360));
    let changed = [
        ToolOverlayTarget {
            document: Uuid::new_v4(),
            ..binding
        },
        ToolOverlayTarget {
            view: Uuid::new_v4(),
            ..binding
        },
        ToolOverlayTarget {
            tool: Uuid::new_v4(),
            ..binding
        },
        ToolOverlayTarget {
            revision: 2,
            ..binding
        },
        ToolOverlayTarget {
            size: (801, 360),
            ..binding
        },
        ToolOverlayTarget {
            visible: false,
            ..binding
        },
        ToolOverlayTarget {
            context: ProjectContext::Prefab(Uuid::new_v4()),
            ..binding
        },
    ];
    for current in changed {
        let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
        assert!(tool.presented_overlay(&frame, &markers(&frame), binding, &map, &mut server));
        assert!(!frame.input_current(tool.as_mut(), current, &map, &server));
        assert!(!tool.presented_overlay(&frame, &markers(&frame), current, &map, &mut server));
    }
    // Base map change is also checked by the common HUD owner.
    let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    assert!(tool.presented_overlay(&frame, &markers(&frame), binding, &map, &mut server));
    map.changed = map.changed.wrapping_add(1);
    assert!(!frame.input_current(tool.as_mut(), binding, &map, &server));
    // Independent profile selection must be guarded even when the first selected
    // ID and base map revision are unchanged.
    let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    assert!(tool.presented_overlay(&frame, &markers(&frame), binding, &map, &mut server));
    map.profiles
        .get_mut(&Uuid::from_u128(1002))
        .unwrap()
        .selected_vertices
        .push(102);
    assert!(!frame.input_current(tool.as_mut(), binding, &map, &server));
    let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    assert!(tool.presented_overlay(&frame, &markers(&frame), binding, &map, &mut server));
    server.active_detail_surface.as_mut().unwrap().id = Uuid::new_v4();
    assert!(!frame.input_current(tool.as_mut(), binding, &map, &server));
    // Zero/hidden preparation actively revokes pending and presented geometry.
    let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    let style = crate::hud::HudStyle::current();
    assert!(
        tool.prepare_overlay(ToolOverlayInputs {
            target: ToolOverlayTarget {
                size: (0, 360),
                ..binding
            },
            hud: crate::hud::HudInputs {
                map: &map,
                server: &server,
                assets: &assets,
                selected_id: None,
                material_slots: None,
                light: None
            },
            style: &style
        })
        .is_none()
    );
    assert!(!tool.presented_overlay(&frame, &markers(&frame), binding, &map, &mut server));
}
#[test]
#[cfg(feature = "native-ui")]
fn aggregate_props_requires_all_visible_parts_and_keeps_hidden_panels_inactive() {
    let assets = Assets::default();
    let ctx = TheContext::new(800, 360, 1.);
    for state in [0, 1] {
        let (map, mut server) = fixture(5, state);
        let mut tool = tool(5);
        let binding = target(tool.as_ref(), &map, &server, (800, 360));
        let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
        let receipts = markers(&frame);
        assert_eq!(receipts.len(), if state == 0 { 1 } else { 2 });
        if state == 1 {
            assert!(!tool.presented_overlay(&frame, &receipts[..1], binding, &map, &mut server));
            assert!(!tool.overlay_hud().unwrap().frame_input_current(
                binding.hud_scope(),
                binding.size,
                &map,
                &server
            ));
        }
        let fresh = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
        assert!(tool.presented_overlay(&fresh, &markers(&fresh), binding, &map, &mut server));
        assert!(fresh.input_current(tool.as_mut(), binding, &map, &server));
        if state == 1 {
            // Props configuration can change without changing the HUD selection.
            server.curr_block_asset_name = Some("Concurrent source change".into());
            assert!(!fresh.input_current(tool.as_mut(), binding, &map, &server));
        }
    }
}

#[test]
#[cfg(feature = "native-ui")]
fn aggregate_same_source_preparation_never_borrows_another_frames_presentation_authority() {
    let assets = Assets::default();
    let ctx = TheContext::new(800, 360, 1.);
    for kind in 0..6 {
        let (map, mut server) = fixture(kind, 1);
        let mut tool = tool(kind);
        let target = target(tool.as_ref(), &map, &server, (800, 360));
        let old = prepare(tool.as_mut(), target, &map, &server, &ctx, &assets);
        assert!(tool.presented_overlay(&old, &markers(&old), target, &map, &mut server));
        let pending = prepare(tool.as_mut(), target, &map, &server, &ctx, &assets);
        assert!(
            !pending.input_current(tool.as_mut(), target, &map, &server),
            "pending frame has never appeared in a successful root"
        );
        assert!(
            old.input_current(tool.as_mut(), target, &map, &server),
            "unchanged visible geometry stays active until replacement"
        );
        assert!(tool.presented_overlay(&pending, &markers(&pending), target, &map, &mut server));
        assert!(!old.input_current(tool.as_mut(), target, &map, &server));
        assert!(pending.input_current(tool.as_mut(), target, &map, &server));
    }
}

#[test]
#[cfg(feature = "native-ui")]
fn aggregate_unpresented_profile_source_changes_and_foreign_part_owners_are_rejected() {
    let assets = Assets::default();
    let ctx = TheContext::new(800, 360, 1.);
    let (mut map, mut server) = fixture(0, 2);
    let mut tool = tool(0);
    let binding = target(tool.as_ref(), &map, &server, (800, 360));
    let pending = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    map.profiles
        .get_mut(&Uuid::from_u128(1002))
        .unwrap()
        .changed += 1;
    assert!(!tool.presented_overlay(&pending, &markers(&pending), binding, &map, &mut server));
    let mut foreign_tool = super::tests::tool(0);
    let foreign_binding = target(foreign_tool.as_ref(), &map, &server, (800, 360));
    let mut foreign = prepare(
        foreign_tool.as_mut(),
        foreign_binding,
        &map,
        &server,
        &ctx,
        &assets,
    );
    foreign.target = binding; // Even an accidentally rewritten outer binding cannot retarget nested owners.
    assert!(!tool.presented_overlay(&foreign, &markers(&foreign), binding, &map, &mut server));
}

#[test]
#[cfg(feature = "native-render")]
#[ignore = "actual GPU standard tool aggregation, original fixtures, density and device reconstruction"]
fn aggregate_native_tools_match_independent_reference_fixtures_and_rebuild_on_a_new_device() {
    use crate::editor_overlay::{procedural_gpu::GpuIcons, test_support::*};
    use theframework::{paint::*, render_wgpu::UiImageCache};
    let mut gpu = GpuPanel::new();
    let mut icons = GpuIcons::new(gpu.gpu.clone(), 1024 * 1024, 128).unwrap();
    let mut cache = UiImageCache::default();
    let assets = Assets::default();
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools/overlay_frame/fixtures");
    for width in [320, 800] {
        for kind in 0..6 {
            for state in 0..3 {
                let (map, mut server) = fixture(kind, state);
                let mut tool = tool(kind);
                let size = (width, 360);
                let binding = target(tool.as_ref(), &map, &server, size);
                let ctx = TheContext::new(width, 360, 1.);
                let uploads = gpu.gpu.transfers().uploaded_bytes;
                let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
                assert_eq!(
                    gpu.gpu.transfers().uploaded_bytes,
                    uploads,
                    "tool preparation performs no GPU work"
                );
                assert!(!frame.input_current(tool.as_mut(), binding, &map, &server));
                let mut resources = icons.frame(&mut cache, &mut gpu.images.registry);
                let mut list = frame.paint(&mut gpu.text, &mut resources, 1.).unwrap();
                list.ops.insert(
                    0,
                    PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
                );
                resources.finish_frame(&list).unwrap();
                drop(resources);
                let pixels = gpu.render(&list, size, 1.);
                gpu.compare_reference_many(
                    &frame.commands().collect::<Vec<_>>(),
                    &list,
                    &ctx,
                    &pixels,
                    &expected(&root.join(format!("tool-{kind}-{state}-{width}.png"))),
                    &format!("tool-{kind}-{state}-{width}"),
                );
                assert!(tool.presented_overlay(
                    &frame,
                    &list.receipts().collect::<Vec<_>>(),
                    binding,
                    &map,
                    &mut server
                ));
                assert!(frame.input_current(tool.as_mut(), binding, &map, &server));
            }
        }
    }
    let (map, mut server) = fixture(5, 2);
    let mut tool = tool(5);
    let size = (800, 360);
    let binding = target(tool.as_ref(), &map, &server, size);
    let ctx = TheContext::new(800, 360, 1.);
    let mut baseline = None;
    // These runs establish source, receipt and resource lifetimes; original
    // Creator RenderView scaling at non-unit densities still needs visual parity.
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        tool.invalidate_overlay();
        let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
        let mut resources = icons.frame(&mut cache, &mut gpu.images.registry);
        let mut list = frame.paint(&mut gpu.text, &mut resources, density).unwrap();
        list.ops.insert(
            0,
            PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
        );
        resources.finish_frame(&list).unwrap();
        drop(resources);
        let pixels = gpu.render(&list, size, density);
        assert!(tool.presented_overlay(
            &frame,
            &list.receipts().collect::<Vec<_>>(),
            binding,
            &map,
            &mut server
        ));
        if density == 1. {
            baseline = Some(pixels);
            let before = gpu.gpu.transfers().uploaded_bytes;
            let glyphs = gpu.renderer.glyphs.rasterizations;
            let next = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
            let mut resources = icons.frame(&mut cache, &mut gpu.images.registry);
            let mut list = next.paint(&mut gpu.text, &mut resources, density).unwrap();
            list.ops.insert(
                0,
                PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
            );
            resources.finish_frame(&list).unwrap();
            drop(resources);
            assert_eq!(
                gpu.render(&list, size, density),
                *baseline.as_ref().unwrap()
            );
            assert_eq!(gpu.gpu.transfers().uploaded_bytes, before);
            assert_eq!(gpu.renderer.glyphs.rasterizations, glyphs);
            assert!(tool.presented_overlay(
                &next,
                &list.receipts().collect::<Vec<_>>(),
                binding,
                &map,
                &mut server
            ));
        }
    }
    let old = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    tool.invalidate_overlay();
    let mut replacement = GpuPanel::new();
    let mut replacement_icons = GpuIcons::new(replacement.gpu.clone(), 1024 * 1024, 128).unwrap();
    let mut replacement_cache = UiImageCache::default();
    assert!(!tool.presented_overlay(&old, &markers(&old), binding, &map, &mut server));
    let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    let mut resources =
        replacement_icons.frame(&mut replacement_cache, &mut replacement.images.registry);
    let mut list = frame
        .paint(&mut replacement.text, &mut resources, 1.)
        .unwrap();
    list.ops.insert(
        0,
        PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
    );
    resources.finish_frame(&list).unwrap();
    drop(resources);
    assert_eq!(
        replacement.render(&list, size, 1.),
        baseline.unwrap(),
        "all tool RGBA pixels including Swash labels survive a real device replacement"
    );
    assert!(tool.presented_overlay(
        &frame,
        &list.receipts().collect::<Vec<_>>(),
        binding,
        &map,
        &mut server
    ));
}

#[test]
#[cfg(feature = "native-ui")]
fn reference_glyph_bounds_include_actual_legacy_row_wrapping_without_masking_nontext_rows() {
    let (map, server) = fixture(0, 1);
    let mut tool = tool(0);
    let ctx = TheContext::new(320, 360, 1.);
    let assets = Assets::default();
    let binding = target(tool.as_ref(), &map, &server, (320, 360));
    let frame = prepare(tool.as_mut(), binding, &map, &server, &ctx, &assets);
    let boxes = frame
        .commands()
        .flat_map(|commands| commands.reference_text_bounds(&ctx))
        .collect::<Vec<_>>();
    assert!(
        boxes.iter().any(|rect| rect.contains([3., 6.])),
        "actual legacy glyph 6 spills from X>320 into the next row"
    );
    assert!(
        !boxes.iter().any(|rect| rect.contains([3., 100.])),
        "only actual old glyph boxes are excluded"
    );
}

#[cfg(feature = "native-ui")]
mod routing {
    use super::*;
    use std::sync::Arc;

    struct RecreatedTool {
        id: TheId,
        inner: Box<dyn Tool>,
    }
    impl Tool for RecreatedTool {
        fn new() -> Self {
            Self {
                id: TheId::empty(),
                inner: tool(0),
            }
        }
        fn id(&self) -> TheId {
            self.id.clone()
        }
        fn info(&self) -> String {
            self.inner.info()
        }
        fn icon_name(&self) -> String {
            self.inner.icon_name()
        }
        fn overlay_hud(&mut self) -> Option<&mut crate::hud::Hud> {
            self.inner.overlay_hud()
        }
        fn hud_selected_id(&self, map: &Map, server: &ServerContext) -> Option<u32> {
            self.inner.hud_selected_id(map, server)
        }
        fn overlay_screen_rectangle(
            &self,
            map: &Map,
            server: &ServerContext,
        ) -> Option<(Vec2<f32>, Vec2<f32>)> {
            self.inner.overlay_screen_rectangle(map, server)
        }
    }
    fn ready(
        kind: usize,
        state: usize,
    ) -> (
        Box<dyn Tool>,
        Map,
        ServerContext,
        TheContext,
        Arc<ToolOverlayFrame>,
        ToolOverlayTarget,
    ) {
        let (map, mut server) = fixture(kind, state);
        let mut tool = tool(kind);
        let ctx = TheContext::new(800, 360, 1.);
        let target = target(tool.as_ref(), &map, &server, (800, 360));
        let frame = Arc::new(prepare(
            tool.as_mut(),
            target,
            &map,
            &server,
            &ctx,
            &Assets::default(),
        ));
        assert!(tool.presented_overlay(&frame, &markers(&frame), target, &map, &mut server));
        (tool, map, server, ctx, frame, target)
    }
    fn command(route: ToolOverlayRoute) -> ToolOverlayCommand {
        match route {
            ToolOverlayRoute::Consumed {
                command: Some(command),
                ..
            } => command,
            _ => panic!("expected an accepted overlay command"),
        }
    }
    fn hud_point(
        tool: &mut dyn Tool,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
        wanted: crate::hud::HudAction,
    ) -> Vec2<i32> {
        let hud = tool.overlay_hud().unwrap();
        for y in 0..target.size.1 as i32 {
            for x in 0..target.size.0 as i32 {
                let at = Vec2::new(x, y);
                if hud.hit_test(target.hud_scope(), target.size, at, map, server)
                    == crate::hud::HudHit::Action(wanted)
                {
                    return at;
                }
            }
        }
        panic!("missing painted action {wanted:?}")
    }
    #[test]
    fn native_overlay_routing_requires_presented_source_and_focus_before_accepting_input() {
        let (mut tool, map, mut server, ctx, frame, target) = ready(0, 0);
        let mut input = ToolOverlayInputState::default();
        let pending = Arc::new(prepare(
            tool.as_mut(),
            target,
            &map,
            &server,
            &ctx,
            &Assets::default(),
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&pending),
                target,
                &MapEvent::MapKey('4'),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Unavailable
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                None,
                target,
                &MapEvent::MapClicked(Vec2::new(12, 12)),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Unavailable
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapKey('4'),
                false,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapKey('x'),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapClicked(Vec2::new(700, 300)),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapClicked(Vec2::new(-1, 4)),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
        assert!(tool.presented_overlay(&pending, &markers(&pending), target, &map, &mut server));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapKey('4'),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Unavailable
        ));
        command(input.route(
            tool.as_mut(),
            Some(&pending),
            target,
            &MapEvent::MapKey('4'),
            true,
            &map,
            &server,
        ));
    }
    #[test]
    fn native_overlay_commands_deliver_after_same_source_repaint_and_return_scoped_effects() {
        let (mut tool, mut map, mut server, mut ctx, frame, target) = ready(0, 0);
        let mut input = ToolOverlayInputState::default();
        let accepted = command(input.route(
            tool.as_mut(),
            Some(&frame),
            target,
            &MapEvent::MapKey('4'),
            true,
            &map,
            &server,
        ));
        let replacement = prepare(
            tool.as_mut(),
            target,
            &map,
            &server,
            &ctx,
            &Assets::default(),
        );
        assert!(tool.presented_overlay(
            &replacement,
            &markers(&replacement),
            target,
            &map,
            &mut server
        ));
        let effects = accepted
            .apply(
                tool.as_mut(),
                target,
                &mut map,
                &mut server,
                &mut TheUI::new(),
                &mut ctx,
            )
            .unwrap();
        assert_eq!(map.subdivisions, 8.);
        assert!(effects.scene_dirty && effects.overlay_dirty && effects.redraw);
        assert_eq!(effects.events.len(), 1);
        // The notification survives its own model mutation without authorizing
        // any further action from the stale frame.
        let event = effects
            .events
            .into_iter()
            .next()
            .unwrap()
            .take_if_current(tool.as_mut(), target, &map, &server)
            .unwrap();
        assert!(matches!(event, TheEvent::Custom(id, _) if id.name == "Tool Changed"));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapKey('2'),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Unavailable
        ));
    }
    #[test]
    fn native_overlay_accepted_commands_reject_changed_documents_sources_and_owners() {
        for change in 0..8 {
            let (mut tool, mut map, mut server, mut ctx, frame, mut target) = ready(0, 1);
            let accepted = command(ToolOverlayInputState::default().route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapKey('6'),
                true,
                &map,
                &server,
            ));
            match change {
                0 => target.document = Uuid::new_v4(),
                1 => target.revision += 1,
                2 => target.view = Uuid::new_v4(),
                3 => target.visible = false,
                4 => map.changed += 1,
                5 => map.selected_vertices.clear(),
                6 => tool.invalidate_overlay(),
                7 => {
                    tool = Box::new(RecreatedTool {
                        id: tool.id(),
                        inner: Box::new(vertex::VertexTool::new()),
                    });
                }
                _ => unreachable!(),
            }
            let before = serde_json::to_value(&map).unwrap();
            assert!(
                accepted
                    .apply(
                        tool.as_mut(),
                        target,
                        &mut map,
                        &mut server,
                        &mut TheUI::new(),
                        &mut ctx
                    )
                    .is_none(),
                "change {change}"
            );
            assert_eq!(serde_json::to_value(&map).unwrap(), before);
        }
    }
    #[test]
    fn native_overlay_drag_capture_survives_own_edits_and_ends_without_world_fallthrough() {
        let (mut tool, mut map, mut server, mut ctx, frame, mut target) = ready(0, 0);
        let mut input = ToolOverlayInputState::default();
        let at = hud_point(
            tool.as_mut(),
            target,
            &map,
            &server,
            crate::hud::HudAction::Subdivision(8),
        );
        let accepted = command(input.route(
            tool.as_mut(),
            Some(&frame),
            target,
            &MapEvent::MapClicked(at),
            true,
            &map,
            &server,
        ));
        accepted
            .apply(
                tool.as_mut(),
                target,
                &mut map,
                &mut server,
                &mut TheUI::new(),
                &mut ctx,
            )
            .unwrap();
        for event in [
            MapEvent::MapDragged(Vec2::new(700, 300)),
            MapEvent::MapUp(Vec2::new(700, 300)),
        ] {
            assert!(matches!(
                input.route(tool.as_mut(), None, target, &event, true, &map, &server),
                ToolOverlayRoute::Consumed { command: None, .. }
            ));
        }
        assert!(matches!(
            input.route(
                tool.as_mut(),
                None,
                target,
                &MapEvent::MapUp(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
        let frame = Arc::new(prepare(
            tool.as_mut(),
            target,
            &map,
            &server,
            &ctx,
            &Assets::default(),
        ));
        assert!(tool.presented_overlay(&frame, &markers(&frame), target, &map, &mut server));
        command(input.route(
            tool.as_mut(),
            Some(&frame),
            target,
            &MapEvent::MapClicked(at),
            true,
            &map,
            &server,
        ));
        target.document = Uuid::new_v4();
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapDragged(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Unavailable
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                None,
                target,
                &MapEvent::MapUp(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
    }
    #[test]
    fn native_overlay_hud_icon_and_gizmo_commands_preserve_semantics_without_renderer_access() {
        for (kind, wanted) in [
            (
                1,
                crate::hud::HudAction::SelectIcon {
                    index: 1,
                    wall_row: true,
                },
            ),
            (4, crate::hud::HudAction::GeometryMode { resize: true }),
        ] {
            let (mut tool, mut map, mut server, mut ctx, mut frame, target) = ready(kind, 1);
            if kind == 1 {
                // Wall-row icons are supplied by the active action's material
                // slots; the minimal geometry fixture has none by default.
                let slots = ["Base", "Upper"].map(|label| crate::actions::ActionMaterialSlot {
                    label: label.into(),
                    source: None,
                });
                frame = Arc::new(
                    tool.prepare_overlay(ToolOverlayInputs {
                        target,
                        hud: crate::hud::HudInputs {
                            map: &map,
                            server: &server,
                            assets: &Assets::default(),
                            selected_id: None,
                            material_slots: Some(&slots),
                            light: None,
                        },
                        style: &crate::hud::HudStyle::current(),
                    })
                    .unwrap(),
                );
                assert!(tool.presented_overlay(
                    &frame,
                    &markers(&frame),
                    target,
                    &map,
                    &mut server
                ));
            }
            let at = hud_point(tool.as_mut(), target, &map, &server, wanted);
            let accepted = command(ToolOverlayInputState::default().route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapClicked(at),
                true,
                &map,
                &server,
            ));
            let effects = accepted
                .apply(
                    tool.as_mut(),
                    target,
                    &mut map,
                    &mut server,
                    &mut TheUI::new(),
                    &mut ctx,
                )
                .unwrap();
            if kind == 1 {
                assert_eq!(server.selected_hud_icon_index, 1);
                assert_eq!(server.selected_wall_row, Some(1));
                assert_eq!(effects.events.len(), 2);
            } else {
                assert_eq!(server.geometry_gizmo_op, GeometryGizmoOp::Resize);
                assert!(effects.overlay_dirty);
            }
        }
    }
    #[test]
    fn native_overlay_props_commands_keep_authoring_events_scoped_until_delivery() {
        for stale in [false, true] {
            let (mut tool, mut map, mut server, mut ctx, frame, target) = ready(5, 1);
            let mut input = ToolOverlayInputState::default();
            let accepted = command(input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapClicked(Vec2::new(160, 75)),
                true,
                &map,
                &server,
            ));
            let effects = accepted
                .apply(
                    tool.as_mut(),
                    target,
                    &mut map,
                    &mut server,
                    &mut TheUI::new(),
                    &mut ctx,
                )
                .unwrap();
            assert!(!effects.scene_dirty && !effects.overlay_dirty);
            assert_eq!(
                server.block_operation, 0,
                "authoring waits for validated event delivery"
            );
            assert_eq!(effects.events.len(), 1);
            if stale {
                server.curr_block_asset_id = Some(Uuid::new_v4());
            }
            let event = effects.events.into_iter().next().unwrap().take_if_current(
                tool.as_mut(),
                target,
                &map,
                &server,
            );
            if stale {
                assert!(event.is_none());
            } else {
                assert!(
                    matches!(event, Some(TheEvent::IndexChanged(id, 1)) if id.name == "Blocks Dock Operation")
                );
            }
        }
    }
    #[test]
    fn native_overlay_props_popups_use_the_accepted_button_and_reject_stale_delivery() {
        for stale in [false, true] {
            let (mut tool, mut map, mut server, mut ctx, frame, target) = ready(5, 1);
            let rect = frame
                .parts
                .iter()
                .find_map(|part| {
                    if let Part::Props(props) = part {
                        props.buttons.iter().find_map(|(rect, action)| {
                            matches!(action, props_overlay::Action::Placement).then_some(*rect)
                        })
                    } else {
                        None
                    }
                })
                .unwrap();
            let at = Vec2::new(rect.x + rect.width / 2, rect.y + rect.height / 2);
            let accepted = command(ToolOverlayInputState::default().route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapClicked(at),
                true,
                &map,
                &server,
            ));
            let mut ui = TheUI::new();
            let mut view = TheRenderView::new(TheId::named_with_id("PolyView", target.view));
            view.set_dim(TheDim::rect(23, 35, 800, 360), &mut ctx);
            ui.canvas.set_widget(view);
            if stale {
                server.block_grid_level += 1;
            }
            let effects = accepted.apply(
                tool.as_mut(),
                target,
                &mut map,
                &mut server,
                &mut ui,
                &mut ctx,
            );
            if stale {
                assert!(effects.is_none());
                assert!(ui.popover.is_none());
            } else {
                let effects = effects.unwrap();
                assert!(effects.redraw && effects.events.is_empty());
                let popup = ui.popover.as_ref().expect("placement controls opened");
                assert_eq!(popup.id.name, "Props Overlay Settings");
                assert_eq!(
                    popup.anchor,
                    TheDim::rect(23 + rect.x, 35 + rect.y, rect.width, rect.height)
                );
                assert!(ui.get_widget("Props Grid").is_some());
                assert!(ui.get_widget("Props Level").is_some());
            }
        }
    }
    #[test]
    fn native_overlay_hover_capture_cancellation_and_blank_props_regions_are_explicit() {
        let (mut tool, map, server, _ctx, frame, target) = ready(5, 1);
        let mut input = ToolOverlayInputState::default();
        let at = Vec2::new(10, 27);
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapHover(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Consumed {
                command: None,
                redraw: true
            }
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapHover(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Consumed {
                command: None,
                redraw: false
            }
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapClicked(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Consumed { command: None, .. }
        ));
        input.invalidate();
        assert!(matches!(
            input.route(
                tool.as_mut(),
                None,
                target,
                &MapEvent::MapDragged(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
        assert!(matches!(
            input.route(
                tool.as_mut(),
                Some(&frame),
                target,
                &MapEvent::MapClicked(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Consumed { command: None, .. }
        ));
        input.route(
            tool.as_mut(),
            None,
            target,
            &MapEvent::MapEscape,
            true,
            &map,
            &server,
        );
        assert!(matches!(
            input.route(
                tool.as_mut(),
                None,
                target,
                &MapEvent::MapUp(at),
                true,
                &map,
                &server
            ),
            ToolOverlayRoute::Outside { .. }
        ));
    }
}
