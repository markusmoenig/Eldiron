#![cfg(target_os = "macos")]
use denrim_materials::{cpu::Evaluator, ir::Channel, live::Document, metal};
use std::{fs, path::Path, process::Command};

fn reference(document: &Document, path: &Path) {
    let pixels = Evaluator::new(document.program())
        .render("demo", Channel::BaseColor, 73, 61)
        .unwrap();
    let bytes = pixels
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 1.])
        .flat_map(f32::to_le_bytes)
        .collect::<Vec<_>>();
    fs::write(path, bytes).unwrap();
}

#[test]
#[ignore = "requires a Metal device; run cargo test --test live_metal_runtime -- --ignored --nocapture"]
fn buffer_edits_render_through_one_compiled_pipeline() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/live-metal-validation");
    fs::create_dir_all(&dir).unwrap();
    let harness = dir.join("compare");
    let result = Command::new("xcrun")
        .args(["swiftc", "-module-cache-path"])
        .arg(dir.join("module-cache"))
        .arg(root.join("tests/metal_compare.swift"))
        .arg("-o")
        .arg(&harness)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut document = Document::parse(include_str!("../examples/warped-disc.material")).unwrap();
    let mut shader = metal::compile(document.program(), "demo", Channel::BaseColor).unwrap();
    fs::write(dir.join("live.metal"), &shader.source).unwrap();
    fs::write(dir.join("before.params"), shader.parameter_bytes()).unwrap();
    reference(&document, &dir.join("before.rgba32f"));
    document
        .set_constant("demo", "noise", 0, "globalScale", &[0.35])
        .unwrap();
    document
        .set_constant("demo", "shape", 0, "GRAD", &[1.])
        .unwrap();
    document
        .set_constant("demo", "shape", 0, "inStrengthGradient", &[0.8])
        .unwrap();
    document
        .set_constant("demo", "shape", 0, "localScale", &[0.8, 1.3])
        .unwrap();
    document
        .set_constant("demo", "shape", 1, "localValue", &[0.5, 0.7, 0.9])
        .unwrap();
    document
        .set_constant("demo", "surface", 0, "color2", &[0.9, 0.4, 0.1, 1.])
        .unwrap();
    shader.refresh(document.program()).unwrap();
    fs::write(dir.join("after.params"), shader.parameter_bytes()).unwrap();
    reference(&document, &dir.join("after.rgba32f"));
    assert_ne!(
        fs::read(dir.join("before.rgba32f")).unwrap(),
        fs::read(dir.join("after.rgba32f")).unwrap()
    );
    let result = Command::new(&harness)
        .arg(dir.join("live.metal"))
        .arg(dir.join("before.rgba32f"))
        .args(["73", "61"])
        .arg(dir.join("before.params"))
        .arg(dir.join("after.rgba32f"))
        .arg(dir.join("after.params"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    println!("{}", String::from_utf8_lossy(&result.stdout));
}
