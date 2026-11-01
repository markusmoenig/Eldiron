use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel, metal};
const SOURCE: &str = "project 1\nmaterial \"m\" called \"Defaults\"\n  default roughness to 0.3\n  default metallic to 0.8\n  default base color to 0.1 0.2 0.3\n  row \"c\"\n    use \"Color\"\n";
#[test]
fn material_defaults_are_typed_persisted_and_sampled_without_output_connections() {
    let ast = format::parse(SOURCE).unwrap();
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
    let p = compile(SOURCE).unwrap();
    let e = Evaluator::new(&p);
    assert_eq!(
        e.sample("m", Channel::Roughness, [0., 0.]).unwrap(),
        vec![0.3]
    );
    assert_eq!(
        e.sample("m", Channel::Metallic, [0., 0.]).unwrap(),
        vec![0.8]
    );
    assert_eq!(
        e.sample("m", Channel::BaseColor, [0., 0.]).unwrap(),
        vec![0.1, 0.2, 0.3]
    );
    assert_eq!(e.sample("m", Channel::Ior, [0., 0.]).unwrap(), vec![1.5]);
    assert_eq!(p.materials[0].rows.len(), 1);
    assert!(p.materials[0].outputs.is_empty());
}
#[test]
fn wiring_overrides_but_preserves_material_fallback() {
    let connected = format!("{SOURCE}  output roughness from row \"c\" x\n");
    let p = compile(&connected).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample("m", Channel::Roughness, [0., 0.])
            .unwrap(),
        vec![0.2]
    );
    assert_eq!(
        p.materials[0].default_value(Channel::Roughness).unwrap(),
        [0.3]
    );
    assert_eq!(
        Evaluator::new(&compile(SOURCE).unwrap())
            .sample("m", Channel::Roughness, [0., 0.])
            .unwrap(),
        vec![0.3]
    );
}
#[test]
fn default_edits_refresh_gpu_buffer_without_recompiling() {
    let p = compile(SOURCE).unwrap();
    let mut shader = metal::compile(&p, "m", Channel::Roughness).unwrap();
    let original = shader.source.clone();
    assert_eq!(shader.parameters(), &[[0.3, 0., 0., 0.]]);
    let edited = compile(&SOURCE.replace("roughness to 0.3", "roughness to 0.7")).unwrap();
    shader.refresh(&edited).unwrap();
    assert_eq!(shader.source, original);
    assert_eq!(shader.parameters(), &[[0.7, 0., 0., 0.]]);
}
#[test]
fn invalid_defaults_do_not_pass_lowering() {
    for statement in [
        "default roughness to 1 2",
        "default base color to 1",
        "default nonexistent to 1",
        "default distance to 1",
        "default metallic to NaN",
        "default roughness to 0.2\n  default roughness to 0.3",
    ] {
        let text = format!(
            "project 1\nmaterial \"m\" called \"M\"\n  {statement}\n  row \"c\"\n    use \"Color\"\n"
        );
        assert!(compile(&text).is_err(), "{statement}");
    }
}

#[test]
fn pbr_mode_round_trips_and_evaluates_standard_channels() {
    let source = "project 1\nmaterial \"m\" called \"Eldiron PBR\"\n  mode pbr\n  default base color to 0.1 0.2 0.3\n  default roughness to 0.4\n  default metallic to 0.7\n  default emission to 0.2 0.1 0\n  default opacity to 0.25\n";
    let ast = format::parse(source).unwrap();
    assert_eq!(ast.materials[0].mode, "pbr");
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
    let program = compile(source).unwrap();
    let evaluator = Evaluator::new(&program);
    for (channel, expected) in [
        (Channel::BaseColor, vec![0.1, 0.2, 0.3]),
        (Channel::Roughness, vec![0.4]),
        (Channel::Metallic, vec![0.7]),
        (Channel::Emission, vec![0.2, 0.1, 0.]),
        (Channel::Opacity, vec![0.25]),
    ] {
        assert_eq!(evaluator.sample("m", channel, [0., 0.]).unwrap(), expected);
        assert!(metal::compile(&program, "m", channel).is_ok());
    }
}
