use denrim_materials::{compile, cpu::Evaluator, ir::Channel, metadata::Kernel, tilelang};
fn recipe(width: usize, height: usize, hex: &str) -> String {
    let source = tilelang::source(Kernel::Color);
    let code = format!("{source}\n# raster-rgba8 {width} {height} srgb {hex}");
    let mut text="project 1\nmaterial \"m\" called \"Raster\"\n  mode pbr\n  row \"image\"\n    use \"Color\" id \"pixels\"\n      source\n".to_owned();
    for line in code.lines() {
        text.push_str("        ");
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("      end source\n  output base color from row \"image\" xyz\n  output opacity from row \"image\" w\n");
    text
}
#[test]
fn raster_sampling_wraps_negative_uv_and_preserves_srgb_and_alpha() {
    let p = compile(&recipe(2, 1, "8000008000ff00ff")).unwrap();
    let e = Evaluator::new(&p);
    let red = e.sample("m", Channel::BaseColor, [0.1, 0.5]).unwrap();
    assert!((red[0] - 0.21586053).abs() < 0.0001);
    assert!((e.sample("m", Channel::Opacity, [0.1, 0.5]).unwrap()[0] - 128. / 255.).abs() < 0.0001);
    assert_eq!(
        e.sample("m", Channel::BaseColor, [-0.25, 1.5]).unwrap(),
        vec![0., 1., 0.]
    );
}
#[test]
fn raster_payload_is_bounded_and_not_counted_as_executable_source() {
    assert!(compile(&recipe(0, 1, "ff0000ff")).is_err());
    assert!(compile(&recipe(usize::MAX, 2, "ff0000ff")).is_err());
    assert!(compile(&recipe(2, 1, "ff0000ff")).is_err());
    assert!(compile(&recipe(1, 1, "xx0000ff")).is_err());
    let p = compile(&recipe(64, 64, &"ff0000ff".repeat(64 * 64))).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample("m", Channel::BaseColor, [0.9, 0.1])
            .unwrap(),
        vec![1., 0., 0.]
    );
}
#[cfg(feature = "wgpu")]
#[test]
fn raster_gpu_data_uses_storage_and_pixel_updates_keep_the_shader_layout() {
    let p = compile(&recipe(64, 64, &"ff0000ff".repeat(64 * 64))).unwrap();
    let mut shader = denrim_materials::wgsl::compile(&p, "m", Channel::BaseColor).unwrap();
    assert!(shader.parameters().len() >= 64 * 64);
    let source = shader.source.clone();
    let q = compile(&recipe(64, 64, &"00ff00ff".repeat(64 * 64))).unwrap();
    shader.refresh(&q).unwrap();
    assert_eq!(shader.source, source);
    assert!(shader.parameters().iter().any(|v| *v == [0., 1., 0., 1.]));
}
