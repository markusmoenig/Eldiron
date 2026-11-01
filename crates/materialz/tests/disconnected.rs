use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel, metal};
#[test]
fn empty_documents_and_explicit_disconnected_processors_are_valid() {
    assert!(compile("project 1\n").unwrap().materials.is_empty());
    for name in ["Multiply", "Border", "fBm"] {
        let source = format!(
            "project 1\nmaterial \"m\" called \"M\"\n  row \"r\"\n    use \"{name}\"\n      input disconnected\n  output base color from row \"r\" xyz\n"
        );
        let ast = format::parse(&source).unwrap();
        assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
        let program = compile(&source).unwrap();
        assert!(
            Evaluator::new(&program)
                .sample("m", Channel::BaseColor, [0.1, 0.2])
                .unwrap()
                .iter()
                .all(|v| v.is_finite())
        );
        metal::compile(&program, "m", Channel::BaseColor).unwrap();
    }
}
