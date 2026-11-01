use denrim_materials::{cpu::Evaluator, ir::Channel, live::Document, metal};

const DEMO: &str = include_str!("../examples/warped-disc.material");

#[test]
fn constant_edits_refresh_buffers_without_changing_source() {
    let mut document = Document::parse(DEMO).unwrap();
    let mut shader = metal::compile(document.program(), "demo", Channel::BaseColor).unwrap();
    let source = shader.source.clone();
    let layout = shader.slots.clone();
    let old = shader.parameter_bytes();
    let before = Evaluator::new(document.program())
        .sample("demo", Channel::BaseColor, [0., 0.])
        .unwrap();
    document
        .set_constant("demo", "shape", 1, "globalValue", &[0.2])
        .unwrap();
    document
        .set_constant("demo", "shape", 0, "GRAD", &[1.])
        .unwrap();
    document
        .set_constant("demo", "surface", 0, "color2", &[0.9, 0.2, 0.1, 1.])
        .unwrap();
    shader.refresh(document.program()).unwrap();
    assert_eq!(source, shader.source);
    assert_eq!(layout, shader.slots);
    assert_ne!(old, shader.parameter_bytes());
    assert_ne!(
        before,
        Evaluator::new(document.program())
            .sample("demo", Channel::BaseColor, [0., 0.])
            .unwrap()
    );
    let slot = shader
        .slots
        .iter()
        .find(|s| s.row == "shape" && s.tile == 1 && s.parameter == "globalValue")
        .unwrap();
    assert_eq!(shader.parameters()[slot.index], [0.2, 0., 0., 0.]);
    assert_eq!(shader.parameter_bytes().len(), shader.slots.len() * 16);
    let reloaded = Document::parse(&document.text()).unwrap();
    assert_eq!(
        metal::compile(reloaded.program(), "demo", Channel::BaseColor)
            .unwrap()
            .parameter_bytes(),
        shader.parameter_bytes()
    );
}

#[test]
fn rejected_edits_are_atomic_and_connected_parameters_are_preserved() {
    let mut document = Document::parse(DEMO).unwrap();
    let original = document.text();
    for (row, tile, parameter, values) in [
        ("shape", 0, "inBEVEL", vec![0.]),
        ("shape", 0, "localScale", vec![1.]),
        ("shape", 0, "GRAD", vec![0.5]),
        ("shape", 0, "typo", vec![1.]),
        ("shape", 0, "globalScale", vec![f32::NAN]),
        ("warped", 0, "rot", vec![30.]),
        ("shape", 99, "globalScale", vec![1.]),
    ] {
        assert!(
            document
                .set_constant("demo", row, tile, parameter, &values)
                .is_err()
        );
        assert_eq!(document.text(), original);
    }
    assert!(
        document
            .set_constant("absent", "shape", 0, "globalScale", &[1.])
            .is_err()
    );
    assert_eq!(document.text(), original);
}

#[test]
fn stale_shader_rejects_structural_changes_without_overwriting_buffer() {
    let document = Document::parse(DEMO).unwrap();
    let mut shader = metal::compile(document.program(), "demo", Channel::BaseColor).unwrap();
    let old = shader.parameter_bytes();
    // Changing a mapping changes computation, not an ordinary parameter value.
    let next = Document::parse(&DEMO.replace("mapped -80 to 80", "mapped -40 to 40")).unwrap();
    assert!(
        shader
            .refresh(next.program())
            .unwrap_err()
            .to_string()
            .contains("structure changed")
    );
    assert_eq!(shader.parameter_bytes(), old);
    let next = Document::parse(&DEMO.replace("row \"shape\"", "row \"renamed-id\"")).unwrap();
    assert!(shader.refresh(next.program()).is_err());
}

#[test]
fn parameter_layout_uses_only_reachable_constants() {
    let document = Document::parse(DEMO).unwrap();
    let shader = metal::compile(document.program(), "demo", Channel::Roughness).unwrap();
    // Unified Perlin retains the two scale controls and adds dimensions, slice and Z stretch.
    assert_eq!(
        shader
            .slots
            .iter()
            .map(|s| s.parameter.as_str())
            .collect::<Vec<_>>(),
        [
            "globalScale",
            "localScale",
            "dimensions",
            "slice",
            "depthScale"
        ]
    );
    assert!(shader.slots.iter().all(|s| s.row == "noise"));
    assert!(
        shader
            .source
            .contains("constant float4* params [[buffer(2)]]")
    );
    assert_eq!(
        shader.parameters(),
        &[
            [0.8, 0., 0., 0.],
            [1., 1.3, 0., 0.],
            [0., 0., 0., 0.],
            [0., 0., 0., 0.],
            [1., 0., 0., 0.]
        ]
    );
}
