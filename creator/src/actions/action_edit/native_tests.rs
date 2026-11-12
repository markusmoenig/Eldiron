use super::*;
fn fixture(kind: usize) -> EditorDisplaySession {
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
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/actions/action_edit/fixtures");
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
