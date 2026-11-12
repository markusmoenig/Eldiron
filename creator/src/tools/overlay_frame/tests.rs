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
