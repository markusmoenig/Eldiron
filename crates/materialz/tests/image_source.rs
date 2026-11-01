use denrim_materials::{
    cpu::Evaluator,
    format,
    image_source::ImageSource,
    ir::{Channel, Program},
};
fn project() -> format::Project {
    format::parse("project 1\nmaterial \"m\" called \"Image\"\n  mode pbr\n  tiling raw\n  row \"r\"\n    use \"Image / Tile Source\" id \"image\"\n  output base color from row \"r\" xyz\n  output opacity from row \"r\" w\n").unwrap()
}
fn source() -> ImageSource {
    ImageSource::rgba8(
        2,
        1,
        &[
            vec![255, 0, 0, 255, 0, 255, 0, 0],
            vec![0, 0, 255, 255, 255, 255, 255, 255],
        ],
    )
    .unwrap()
}
#[test]
fn portable_image_roundtrip_animation_alpha_and_address_modes() {
    let mut p = project();
    let mut image = source();
    image.reference = Some("eldiron:tile:example".into());
    p.materials[0].rows[0].tiles[0].image = Some(image);
    let text = format::write(&p);
    assert!(!text.contains("source\n"));
    assert_eq!(format::parse(&text).unwrap(), p);
    let program = Program::lower(&p).unwrap();
    let e = Evaluator::new(&program);
    assert_eq!(
        e.sample_at_time("m", Channel::BaseColor, [-0.25, 0.], 0.)
            .unwrap(),
        vec![1., 0., 0.]
    );
    assert_eq!(
        e.sample_at_time("m", Channel::BaseColor, [-0.25, 0.], 0.5)
            .unwrap(),
        vec![0., 0., 1.]
    );
    assert_eq!(
        e.sample_at_time("m", Channel::BaseColor, [-0.25, 0.], 1.5)
            .unwrap(),
        vec![0., 0., 1.]
    );
    assert_eq!(
        e.sample("m", Channel::Opacity, [0.25, 0.]).unwrap(),
        vec![0.]
    );
    assert_eq!(
        e.sample("m", Channel::Opacity, [0.75, 0.]).unwrap(),
        vec![0.]
    );
    for (address, expected) in [(1, vec![0., 1., 0.]), (2, vec![1., 0., 0.])] {
        p.materials[0].rows[0].tiles[0].parameters = vec![(
            "address".into(),
            format::Binding::Constant(vec![address as f32]),
        )];
        let program = Program::lower(&p).unwrap();
        assert_eq!(
            Evaluator::new(&program)
                .sample("m", Channel::BaseColor, [0.75, 0.])
                .unwrap(),
            expected
        );
        #[cfg(feature = "wgpu")]
        denrim_materials::wgsl::compile(&program, "m", Channel::BaseColor).unwrap();
    }
}
#[test]
fn linear_filtering_preserves_straight_alpha_and_pbr_planes() {
    let mut p = project();
    let mut image = source();
    image
        .set_plane(
            "roughness",
            "linear",
            &[vec![128, 128, 128, 255, 64, 64, 64, 255]],
        )
        .unwrap();
    p.materials[0].rows[0].tiles[0].image = Some(image);
    p.materials[0].rows[0].tiles[0].parameters =
        vec![("filter".into(), format::Binding::Constant(vec![1.]))];
    let program = Program::lower(&p).unwrap();
    let e = Evaluator::new(&program);
    assert_eq!(
        e.sample("m", Channel::BaseColor, [0., 0.]).unwrap(),
        vec![1., 0., 0.]
    );
    assert_eq!(
        e.sample("m", Channel::Opacity, [0., 0.]).unwrap(),
        vec![0.5]
    );
    #[cfg(feature = "wgpu")]
    denrim_materials::wgsl::compile(&program, "m", Channel::BaseColor).unwrap();
    p.materials[0].rows[0].tiles[0].parameters =
        vec![("channel".into(), format::Binding::Constant(vec![1.]))];
    let program = Program::lower(&p).unwrap();
    let v = Evaluator::new(&program)
        .sample("m", Channel::BaseColor, [-0.25, 0.])
        .unwrap();
    assert!((v[0] - 128. / 255.).abs() < 0.00001);
}
#[test]
fn images_validate_and_pixel_edits_reuse_gpu_shader_layout() {
    assert!(ImageSource::rgba8(0, 1, &[vec![]]).is_err());
    assert!(ImageSource::rgba8(2, 1, &[vec![0; 4]]).is_err());
    let mut p = project();
    p.materials[0].rows[0].tiles[0].image = Some(source());
    let program = Program::lower(&p).unwrap();
    let mut shader = denrim_materials::metal::compile(&program, "m", Channel::BaseColor).unwrap();
    let before = shader.parameter_bytes();
    p.materials[0].rows[0].tiles[0].image =
        Some(ImageSource::rgba8(2, 1, &[vec![255; 8], vec![0; 8]]).unwrap());
    shader.refresh(&Program::lower(&p).unwrap()).unwrap();
    assert_ne!(before, shader.parameter_bytes());
    let mut empty = project();
    empty.materials[0].rows[0].tiles[0].image = None;
    assert_eq!(
        Evaluator::new(&Program::lower(&empty).unwrap())
            .sample("m", Channel::Opacity, [0.; 2])
            .unwrap(),
        vec![0.]
    );
}
