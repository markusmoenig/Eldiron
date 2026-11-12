use super::*;
const SIZE: (i32, i32) = (900, 160);
const MODES: [HudMode; 9] = [
    HudMode::Selection,
    HudMode::Prefab,
    HudMode::Vertex,
    HudMode::Linedef,
    HudMode::Sector,
    HudMode::Effects,
    HudMode::Rect,
    HudMode::Terrain,
    HudMode::Entity,
];
fn fixture(
    mode: HudMode,
    orbit: bool,
) -> (Hud, Map, ServerContext, TheContext, Assets, TheRGBABuffer) {
    let mut assets = Assets::default();
    let mut texture = rusterix::Texture::alloc(63, 29);
    for (y, row) in texture.data.chunks_exact_mut(63 * 4).enumerate() {
        for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&[
                (x * 4) as u8,
                (y * 8) as u8,
                120,
                if (x + y) % 3 == 0 { 100 } else { 255 },
            ]);
        }
    }
    let tile = rusterix::Tile::from_texture(texture);
    let tile_id = tile.id;
    assets.tiles.insert(tile_id, tile);
    let mut map = Map::default();
    map.subdivisions = 8.;
    let mut sector = rusterix::Sector::new(42, vec![]);
    sector
        .properties
        .set("source", Value::Source(PixelSource::TileId(tile_id)));
    sector.properties.set("floor_light", Value::Bool(true));
    map.sectors.push(sector);
    let mut server = ServerContext::default();
    server.pc = ProjectContext::Region(map.id);
    server.editor_view_mode = if orbit {
        EditorViewMode::Orbit
    } else {
        EditorViewMode::D2
    };
    server.hover_cursor = Some(Vec2::new(1.25, -2.5));
    server.hover_cursor_3d = Some(Vec3::new(1.1, 2.2, -3.3));
    server.hover_height = Some(3.5);
    server.background_progress = Some("Preparing fixture".into());
    server.selected_hud_icon_index = 999;
    let mut ctx = TheContext::new(SIZE.0 as usize, SIZE.1 as usize, 1.);
    let mut light = TheRGBABuffer::new(TheDim::sized(3, 3));
    for (i, p) in light.pixels_mut().chunks_exact_mut(4).enumerate() {
        p.copy_from_slice(&[240, 220, 80, if i % 2 == 0 { 160 } else { 0 }]);
    }
    ctx.ui.add_icon("light_small".into(), light);
    let mut buffer = TheRGBABuffer::new(TheDim::sized(SIZE.0, SIZE.1));
    for p in buffer.pixels_mut().chunks_exact_mut(4) {
        p.copy_from_slice(&[12, 24, 36, 0]);
    }
    (Hud::new(mode), map, server, ctx, assets, buffer)
}
fn style() -> HudStyle {
    HudStyle {
        colors: HudColors {
            background: [48, 48, 50, 235],
            panel: [34, 34, 36, 240],
            text: [168, 168, 172, 255],
            selected_text: [240, 240, 242, 255],
            accent: [210, 176, 92, 255],
            border: [76, 78, 84, 255],
            hover: [54, 54, 58, 255],
            selected: [68, 68, 72, 255],
        },
        geometry_labels: ["MOVE".into(), "SIZE".into()],
    }
}
fn size() -> (usize, usize) {
    (SIZE.0 as usize, SIZE.1 as usize)
}
fn scope() -> HudScope {
    HudScope {
        document: Uuid::new_v4(),
        tool: Uuid::new_v4(),
        revision: 31,
    }
}
#[allow(clippy::too_many_arguments)]
fn prepare(
    hud: &mut Hud,
    scope: HudScope,
    size: (usize, usize),
    map: &Map,
    ctx: &TheContext,
    server: &ServerContext,
    id: Option<u32>,
    assets: &Assets,
    style: &HudStyle,
) -> HudFrame {
    let light = ctx.ui.icon("light_small").map(|icon| HudImageSource {
        width: icon.pixel_width(),
        height: icon.pixel_height(),
        pixels: icon.pixels(),
    });
    hud.prepare_frame(
        scope,
        size,
        HudInputs {
            map,
            server,
            assets,
            selected_id: id,
            material_slots: None,
            light,
        },
        style,
    )
}
fn expected(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/hud/fixtures")
        .join(format!("{name}.png"));
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width, info.height), (SIZE.0 as u32, SIZE.1 as u32));
    assert_eq!(info.color_type, png::ColorType::Rgba);
    pixels.truncate(info.buffer_size());
    pixels
}
fn acknowledge(
    hud: &mut Hud,
    frame: &HudFrame,
    scope: HudScope,
    size: (usize, usize),
    map: &Map,
    server: &mut ServerContext,
) -> bool {
    #[cfg(feature = "native-ui")]
    {
        hud.presented_paint(frame, &[frame.token.receipt()], scope, size, map, server)
    }
    #[cfg(not(feature = "native-ui"))]
    {
        hud.presented_frame(frame, scope, size, map, server)
    }
}
#[test]
fn reference_hud_commands_preserve_all_eighteen_pre_extraction_rgba_fixtures() {
    for mode in MODES {
        for orbit in [false, true] {
            let (mut hud, map, mut server, mut ctx, assets, mut buffer) = fixture(mode, orbit);
            let authored = serde_json::to_vec(&map).unwrap();
            let scope = scope();
            let frame = prepare(
                &mut hud,
                scope,
                size(),
                &map,
                &mut ctx,
                &mut server,
                Some(42),
                &assets,
                &style(),
            );
            assert_eq!(
                server.selected_hud_icon_index, 999,
                "preparation cannot clamp live input selection"
            );
            assert!(
                hud.icon_rects.is_empty()
                    && hud.subdiv_rects.is_empty()
                    && hud.edit_mode_rects.is_empty()
            );
            assert!(!hud.frame_input_current(scope, size(), &map, &server));
            frame.commands.draw_reference(&mut buffer, &ctx);
            let reference = expected(&format!("{mode:?}-{orbit}"));
            let differing = buffer
                .pixels()
                .iter()
                .zip(&reference)
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(
                differing, 0,
                "{mode:?}-{orbit}: original RGBA pixels must remain exact"
            );
            assert!(acknowledge(
                &mut hud,
                &frame,
                scope,
                size(),
                &map,
                &mut server
            ));
            assert_eq!(serde_json::to_vec(&map).unwrap(), authored);
        }
    }
}
#[test]
fn hud_frames_require_painted_source_scope_and_never_promote_prepared_hit_geometry() {
    let (mut hud, mut map, mut server, mut ctx, assets, _) = fixture(HudMode::Selection, true);
    let scope = scope();
    let frame = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(500, 5), &map, &server),
        HudHit::Unavailable
    );
    #[cfg(feature = "native-ui")]
    assert!(!hud.presented_paint(&frame, &[], scope, size(), &map, &mut server));
    for invalid in [
        HudScope {
            document: Uuid::new_v4(),
            ..scope
        },
        HudScope {
            tool: Uuid::new_v4(),
            ..scope
        },
        HudScope {
            revision: scope.revision + 1,
            ..scope
        },
    ] {
        assert!(!acknowledge(
            &mut hud,
            &frame,
            invalid,
            size(),
            &map,
            &mut server
        ));
    }
    assert!(!acknowledge(
        &mut hud,
        &frame,
        scope,
        (899, 160),
        &map,
        &mut server
    ));
    map.selected_vertices.push(77);
    assert!(!acknowledge(
        &mut hud,
        &frame,
        scope,
        size(),
        &map,
        &mut server
    ));
    map.selected_vertices.clear();
    assert!(acknowledge(
        &mut hud,
        &frame,
        scope,
        size(),
        &map,
        &mut server
    ));
    assert_eq!(server.selected_hud_icon_index, 0);
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(500, 5), &map, &server),
        HudHit::Action(HudAction::GeometryMode { resize: true })
    );
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(880, 35), &map, &server),
        HudHit::Action(HudAction::SelectIcon {
            index: 0,
            wall_row: false
        })
    );
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(240, 5), &map, &server),
        HudHit::Action(HudAction::Subdivision(2))
    );
    for point in [Vec2::new(-1, 5), Vec2::new(900, 5), Vec2::new(5, 160)] {
        assert_eq!(
            hud.hit_test(scope, size(), point, &map, &server),
            HudHit::Outside
        );
    }
    let effect = hud.apply_action(HudAction::Subdivision(4), &mut map, &mut server);
    assert_eq!(map.subdivisions, 4.);
    assert!(effect.scene_dirty && effect.overlay_dirty && effect.redraw);
    assert!(
        effect
            .events
            .iter()
            .any(|event| matches!(event,TheEvent::Custom(id,_) if id.name=="Tool Changed"))
    );
    assert!(!hud.frame_input_current(scope, size(), &map, &server));
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(240, 5), &map, &server),
        HudHit::Unavailable
    );
    let next = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    assert!(hud.icon_rects.is_empty() && hud.subdiv_rects.is_empty());
    assert!(!acknowledge(
        &mut hud,
        &frame,
        scope,
        size(),
        &map,
        &mut server
    ));
    assert!(acknowledge(
        &mut hud,
        &next,
        scope,
        size(),
        &map,
        &mut server
    ));
    let effects = hud.apply_action(
        HudAction::GeometryMode { resize: true },
        &mut map,
        &mut server,
    );
    assert_eq!(server.geometry_gizmo_op, GeometryGizmoOp::Resize);
    assert!(effects.overlay_dirty && !effects.scene_dirty);
    assert!(!hud.frame_input_current(scope, size(), &map, &server));
}
#[test]
fn hud_acknowledgements_reject_old_owners_hidden_resized_and_replaced_documents() {
    let (mut hud, map, mut server, mut ctx, assets, _) = fixture(HudMode::Sector, false);
    let scope = scope();
    let a = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    let b = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    // Backpressure can present the older of two preparations with identical source.
    assert!(acknowledge(&mut hud, &a, scope, size(), &map, &mut server));
    assert!(!acknowledge(&mut hud, &a, scope, size(), &map, &mut server));
    assert!(acknowledge(&mut hud, &b, scope, size(), &map, &mut server));
    let mut other = Hud::new(HudMode::Sector);
    assert!(!acknowledge(
        &mut other,
        &b,
        scope,
        size(),
        &map,
        &mut server
    ));
    hud.invalidate_frame();
    assert!(!hud.frame_input_current(scope, size(), &map, &server));
    assert!(!acknowledge(&mut hud, &b, scope, size(), &map, &mut server));
    let hidden = prepare(
        &mut hud,
        scope,
        (0, 0),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    assert!(!acknowledge(
        &mut hud,
        &hidden,
        scope,
        (0, 0),
        &map,
        &mut server
    ));
    assert_eq!(
        hud.hit_test(scope, (0, 0), Vec2::zero(), &map, &server),
        HudHit::Unavailable
    );
    let resumed = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    assert!(acknowledge(
        &mut hud,
        &resumed,
        scope,
        size(),
        &map,
        &mut server
    ));
    let replacement = HudScope {
        document: Uuid::new_v4(),
        revision: 0,
        ..scope
    };
    let next = prepare(
        &mut hud,
        replacement,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    assert!(!hud.frame_input_current(scope, size(), &map, &server));
    assert!(!acknowledge(
        &mut hud,
        &resumed,
        scope,
        size(),
        &map,
        &mut server
    ));
    assert!(acknowledge(
        &mut hud,
        &next,
        replacement,
        size(),
        &map,
        &mut server
    ));
}

#[cfg(feature = "native-render")]
fn capture(gpu: &renderkit::GpuContext, mut ticket: renderkit::ReadbackTicket) -> Vec<u8> {
    gpu.device
        .poll(renderkit::wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
#[cfg(feature = "native-render")]
fn text_mask(
    frame: &HudFrame,
    list: &theframework::paint::PaintList,
    ctx: &TheContext,
    renderer: &mut theframework::render_wgpu::PaintRenderer,
) -> Vec<bool> {
    use theframework::paint::*;
    let mut mask = vec![false; size().0 * size().1];
    let mut mark = |r: Rect| {
        let x0 = (r.x - 1.).floor().max(0.) as usize;
        let x1 = (r.x + r.width + 1.).ceil().max(0.).min(SIZE.0 as f32) as usize;
        let y0 = (r.y - 1.).floor().max(0.) as usize;
        let y1 = (r.y + r.height + 1.).ceil().max(0.).min(SIZE.1 as f32) as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                mask[y * size().0 + x] = true;
            }
        }
    };
    for r in frame.commands.reference_text_bounds(ctx) {
        mark(r);
    }
    let mut clips = vec![Rect::new(0., 0., SIZE.0 as f32, SIZE.1 as f32)];
    for op in &list.ops {
        match op {
            PaintOp::PushClip(r) => clips.push(clips.last().unwrap().intersection(*r)),
            PaintOp::PopClip => {
                clips.pop();
            }
            PaintOp::GlyphRun { run, origin, .. } | PaintOp::OpaqueGlyphRun { run, origin, .. } => {
                for glyph in run.glyphs.iter() {
                    let position = [origin[0] + glyph.position[0], origin[1] + glyph.position[1]];
                    if let Some((_, bitmap)) = renderer.glyphs.glyph(run, glyph.id, 1., position) {
                        let r = Rect::new(
                            position[0].floor() + bitmap.left as f32,
                            position[1].floor() - bitmap.top as f32,
                            bitmap.width as f32,
                            bitmap.height as f32,
                        );
                        mark(r.intersection(*clips.last().unwrap()));
                    }
                }
            }
            _ => {}
        }
    }
    mask
}
#[cfg(feature = "native-render")]
fn write_native(name: &str, pixels: &[u8], size: (u32, u32)) {
    let mut encoder = png::Encoder::new(
        std::fs::File::create(format!("/private/tmp/eldiron-hud-native-{name}.png")).unwrap(),
        size.0,
        size.1,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}
#[test]
#[cfg(feature = "native-render")]
#[ignore = "requires actual native GPU; remaining text metrics/AA are measured separately"]
fn native_hud_frames_compose_raw_sources_and_swash_commands_without_display_pixels() {
    use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket};
    use theframework::{
        paint::*,
        render_wgpu::{PaintRenderer, UiImages},
    };
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(SIZE.0 as u32, SIZE.1 as u32)).unwrap();
    for mode in MODES {
        for orbit in [false, true] {
            let (mut hud, map, mut server, mut ctx, assets, reference_buffer) =
                fixture(mode, orbit);
            drop(reference_buffer);
            let scope = scope();
            let authored = serde_json::to_vec(&map).unwrap();
            let frame = prepare(
                &mut hud,
                scope,
                size(),
                &map,
                &mut ctx,
                &mut server,
                Some(42),
                &assets,
                &style(),
            );
            let sources = frame.commands.images().cloned().collect::<Vec<_>>();
            assert!(
                sources
                    .iter()
                    .all(|source| (source.width(), source.height())
                        != (SIZE.0 as u32, SIZE.1 as u32))
            );
            let upload = gpu.transfers().uploaded_bytes;
            let mut list = frame.paint(&mut ctx.text, &mut images, 1.).unwrap();
            assert_eq!(
                gpu.transfers().uploaded_bytes - upload,
                sources
                    .iter()
                    .map(|s| s.width() as u64 * s.height() as u64 * 8)
                    .sum::<u64>(),
                "only raw source assets are imported before glyph blitting"
            );
            list.ops.insert(
                0,
                PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
            );
            list.validate().unwrap();
            images.finish_frame(&list).unwrap();
            assert!(
                list.ops
                    .iter()
                    .any(|op| matches!(op, PaintOp::OpaqueGlyphRun { .. }))
            );
            assert!(!hud.presented_paint(&frame, &[], scope, size(), &map, &mut server));
            renderer
                .render(
                    &list,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    (SIZE.0 as u32, SIZE.1 as u32),
                    1.,
                    [12. / 255., 24. / 255., 36. / 255., 1.],
                )
                .unwrap();
            let mask = text_mask(&frame, &list, &ctx, &mut renderer);
            let pixels = capture(&gpu, ReadbackTicket::capture(&gpu, output.clone()).unwrap());
            let reference = expected(&format!("{mode:?}-{orbit}"));
            let mut compared = 0;
            let mut exact = 0;
            for (i, (actual, expected)) in pixels
                .chunks_exact(4)
                .zip(reference.chunks_exact(4))
                .enumerate()
            {
                if actual[..3] == expected[..3] {
                    exact += 1;
                }
                if mask[i] {
                    continue;
                }
                compared += 1;
                assert!(
                    actual[..3]
                        .iter()
                        .zip(&expected[..3])
                        .all(|(a, b)| a.abs_diff(*b) <= 1),
                    "{mode:?}-{orbit} pixel ({},{}): {actual:?} vs {expected:?}",
                    i % size().0,
                    i / size().0
                );
            }
            assert!(
                compared > size().0 * size().1 * 9 / 10,
                "text exclusion cannot mask general panel/image regressions"
            );
            println!(
                "HUD {mode:?}-{orbit}: {compared} non-text RGB pixels within one unit; full exact {exact}/{} (text parity remains open)",
                size().0 * size().1
            );
            write_native(
                &format!("{mode:?}-{orbit}"),
                &pixels,
                (SIZE.0 as u32, SIZE.1 as u32),
            );
            assert_eq!(serde_json::to_vec(&map).unwrap(), authored);
            assert_eq!(server.selected_hud_icon_index, 999);
            let receipts = list.receipts().collect::<Vec<_>>();
            assert!(hud.presented_paint(&frame, &receipts, scope, size(), &map, &mut server));
            let warm = gpu.transfers();
            let glyphs = renderer.glyphs.rasterizations;
            let next = prepare(
                &mut hud,
                scope,
                size(),
                &map,
                &mut ctx,
                &mut server,
                Some(42),
                &assets,
                &style(),
            );
            let next_sources = next.commands.images().cloned().collect::<Vec<_>>();
            assert_eq!(sources.len(), next_sources.len());
            for (a, b) in sources.iter().zip(&next_sources) {
                assert!(std::sync::Arc::ptr_eq(a, b));
            }
            let mut repeated = next.paint(&mut ctx.text, &mut images, 1.).unwrap();
            repeated.ops.insert(
                0,
                PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
            );
            images.finish_frame(&repeated).unwrap();
            renderer
                .render(
                    &repeated,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    (SIZE.0 as u32, SIZE.1 as u32),
                    1.,
                    [12. / 255., 24. / 255., 36. / 255., 1.],
                )
                .unwrap();
            assert_eq!(gpu.transfers(), warm);
            assert_eq!(renderer.glyphs.rasterizations, glyphs);
        }
    }
}
#[test]
#[cfg(feature = "native-render")]
#[ignore = "requires actual native GPU and replacement device"]
fn native_hud_source_and_input_lifetimes_survive_fractional_scale_and_gpu_reconstruction() {
    use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket};
    use theframework::{
        paint::*,
        render_wgpu::{PaintRenderer, UiImages},
    };
    let (mut hud, map, mut server, mut ctx, assets, reference_buffer) =
        fixture(HudMode::Sector, false);
    drop(reference_buffer);
    let scope = scope();
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let frame = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    let sources = frame.commands.images().cloned().collect::<Vec<_>>();
    assert!(
        sources.iter().any(|s| (s.width(), s.height()) == (63, 29)),
        "native preparation retains raw source pixels"
    );
    let mut first_output = None;
    for density in [1., 1.25, 1.5, 2.] {
        let physical = (
            (SIZE.0 as f32 * density) as u32,
            (SIZE.1 as f32 * density) as u32,
        );
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(physical.0, physical.1)).unwrap();
        let mut list = frame.paint(&mut ctx.text, &mut images, density).unwrap();
        list.ops.insert(
            0,
            PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
        );
        images.finish_frame(&list).unwrap();
        renderer
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                physical,
                density,
                [12. / 255., 24. / 255., 36. / 255., 1.],
            )
            .unwrap();
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let warm = gpu.transfers();
        let glyphs = renderer.glyphs.rasterizations;
        renderer
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                physical,
                density,
                [12. / 255., 24. / 255., 36. / 255., 1.],
            )
            .unwrap();
        assert_eq!(gpu.transfers(), warm);
        assert_eq!(renderer.glyphs.rasterizations, glyphs);
        if density == 1. {
            first_output = Some(output);
        }
    }
    let before = capture(
        &gpu,
        ReadbackTicket::capture(&gpu, first_output.unwrap()).unwrap(),
    );
    let list = frame.paint(&mut ctx.text, &mut images, 1.).unwrap();
    let receipts = list.receipts().collect::<Vec<_>>();
    assert!(hud.presented_paint(&frame, &receipts, scope, size(), &map, &mut server));
    hud.invalidate_frame();
    assert!(!hud.presented_paint(&frame, &receipts, scope, size(), &map, &mut server));
    let replacement = pollster::block_on(GpuContext::headless()).unwrap();
    let mut new_images = UiImages::new(replacement.clone());
    let mut rebuilt_renderer = PaintRenderer::new(replacement.clone()).unwrap();
    let rebuilt = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    for (a, b) in sources.iter().zip(rebuilt.commands.images()) {
        assert!(std::sync::Arc::ptr_eq(a, b));
    }
    let mut next = rebuilt.paint(&mut ctx.text, &mut new_images, 1.).unwrap();
    next.ops.insert(
        0,
        PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
    );
    new_images.finish_frame(&next).unwrap();
    let output = GpuImage::new(
        &replacement,
        ImageDescriptor::rgba(SIZE.0 as u32, SIZE.1 as u32),
    )
    .unwrap();
    rebuilt_renderer
        .render(
            &next,
            &new_images.registry,
            &output.view,
            output.descriptor.format,
            (SIZE.0 as u32, SIZE.1 as u32),
            1.,
            [12. / 255., 24. / 255., 36. / 255., 1.],
        )
        .unwrap();
    assert_eq!(replacement.transfers().readback_bytes, 0);
    assert!(hud.presented_paint(
        &rebuilt,
        &next.receipts().collect::<Vec<_>>(),
        scope,
        size(),
        &map,
        &mut server
    ));
    let pixels = capture(
        &replacement,
        ReadbackTicket::capture(&replacement, output).unwrap(),
    );
    assert_eq!(
        pixels, before,
        "replacement device must reconstruct every original native RGBA pixel, including text"
    );
}

#[test]
fn unpainted_and_stale_reference_hud_regions_never_retarget_authoring_clicks() {
    let (mut hud, mut map, mut server, mut ctx, assets, _) = fixture(HudMode::Sector, false);
    let mut ui = TheUI::new();
    let scope = scope();
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(10, 5), &map, &server),
        HudHit::Unavailable
    );
    assert!(
        hud.clicked(10, 5, &mut map, &mut ui, &mut ctx, &mut server),
        "reference header keeps its existing click consumption before first draw"
    );
    let frame = prepare(
        &mut hud,
        scope,
        size(),
        &map,
        &mut ctx,
        &mut server,
        Some(42),
        &assets,
        &style(),
    );
    assert!(acknowledge(
        &mut hud,
        &frame,
        scope,
        size(),
        &map,
        &mut server
    ));
    map.changed = map.changed.wrapping_add(1);
    server.selected_hud_icon_index = 2;
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(880, 35), &map, &server),
        HudHit::Unavailable
    );
    assert!(hud.clicked(880, 35, &mut map, &mut ui, &mut ctx, &mut server));
    assert_eq!(
        server.selected_hud_icon_index, 2,
        "stale input cannot apply the old selection"
    );
    assert!(!hud.clicked(100, 100, &mut map, &mut ui, &mut ctx, &mut server));
}

#[test]
fn shortcut_guidance_commands_keep_reference_blending_and_bottom_geometry() {
    let (_, _, _, ctx, _, mut actual) = fixture(HudMode::Selection, true);
    let mut reference = actual.clone();
    let palette = style().colors;
    let help = "Move gizmo - R rotate - Delete removes";
    let bar = Hud::shortcut_guidance_rect(size().0, size().1, help).unwrap();
    let stride = reference.stride();
    // Independent previous four-call reference behavior.
    ctx.draw
        .rect(reference.pixels_mut(), &bar, stride, &palette.panel);
    ctx.draw.rect(
        reference.pixels_mut(),
        &(bar.0, bar.1, bar.2, 1),
        stride,
        &palette.border,
    );
    ctx.draw.text_rect_blend(
        reference.pixels_mut(),
        &(bar.0 + 9, bar.1 + 1, bar.2 - 18, bar.3 - 2),
        stride,
        help,
        TheFontSettings {
            size: 12.5,
            ..Default::default()
        },
        &palette.selected_text,
        TheHorizontalAlign::Left,
        TheVerticalAlign::Center,
    );
    let commands = Hud::prepare_guidance(size(), Some(help), palette);
    commands.draw_reference(&mut actual, &ctx);
    assert_eq!(actual.pixels(), reference.pixels());
    assert!(Hud::prepare_guidance(size(), None, palette).is_empty());
    assert!(Hud::prepare_guidance((12, 10), Some(help), palette).is_empty());
}

#[test]
fn hud_preparation_accepts_resolved_slots_without_context_or_action_callbacks() {
    let mut hud = Hud::new(HudMode::Selection);
    let map = Map::default();
    let mut server = ServerContext::default();
    server.pc = ProjectContext::Region(map.id);
    server.curr_action_id = Some(Uuid::new_v4());
    server.selected_hud_icon_index = 7;
    let assets = Assets::default();
    let scope = scope();
    let slots = [
        ActionMaterialSlot {
            label: "First".into(),
            source: None,
        },
        ActionMaterialSlot {
            label: "Second".into(),
            source: None,
        },
    ];
    // No context, font rasterizer, action registry, renderer, window or GPU is
    // constructed merely to prepare a native HUD description.
    let frame = hud.prepare_frame(
        scope,
        size(),
        HudInputs {
            map: &map,
            server: &server,
            assets: &assets,
            selected_id: None,
            material_slots: Some(&slots),
            light: None,
        },
        &style(),
    );
    assert_eq!(server.selected_hud_icon_index, 7);
    assert!(hud.icon_rects.is_empty());
    assert!(acknowledge(
        &mut hud,
        &frame,
        scope,
        size(),
        &map,
        &mut server
    ));
    assert_eq!(server.selected_hud_icon_index, 1);
    assert_eq!(hud.icon_rects.len(), 2);
    assert_eq!(
        hud.hit_test(scope, size(), Vec2::new(830, 30), &map, &server),
        HudHit::Action(HudAction::SelectIcon {
            index: 0,
            wall_row: false
        })
    );
    let changed = HudScope {
        revision: scope.revision + 1,
        ..scope
    };
    let next = hud.prepare_frame(
        changed,
        size(),
        HudInputs {
            map: &map,
            server: &server,
            assets: &assets,
            selected_id: None,
            material_slots: Some(&slots[..1]),
            light: None,
        },
        &style(),
    );
    assert!(!hud.frame_input_current(scope, size(), &map, &server));
    assert!(!acknowledge(
        &mut hud,
        &frame,
        scope,
        size(),
        &map,
        &mut server
    ));
    assert!(acknowledge(
        &mut hud,
        &next,
        changed,
        size(),
        &map,
        &mut server
    ));
    assert_eq!(server.selected_hud_icon_index, 0);
}
