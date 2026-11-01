use denrim_materials::{compile, cpu::Evaluator, ir::Channel, metadata::Kernel, metal};
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00001, "{a} != {b}");
}
fn over(background_alpha: f32, foreground_alpha: f32, opacity: f32) -> String {
    format!(
        r#"project 1
material "m" called "Composite"
  row "front"
    use "Color"
      set "color" to 0.9 0.1 0.3 {foreground_alpha}
  row "back"
    use "Color"
      set "color" to 0.2 0.4 0.8 {background_alpha}
  row "out"
    use "Alpha Over"
      input from row "back" xyz
      set "foreground" from row "front" xyzw
      set "opacity" to {opacity}
  output base color from row "out" xyz
  output opacity from row "out" w
"#
    )
}
#[test]
fn alpha_over_handles_partial_opaque_and_transparent_layers_without_premultiplying_twice() {
    for (back, front, opacity, expected) in [
        (0.5, 0.25, 0.8, [0.43333334, 0.3, 0.6333333, 0.6]),
        (0.5, 1., 1., [0.9, 0.1, 0.3, 1.]),
        (0.5, 0., 1., [0.2, 0.4, 0.8, 0.5]),
        (0., 0., 1., [0.; 4]),
        (1., 1., 0., [0.2, 0.4, 0.8, 1.]),
    ] {
        let program = compile(&over(back, front, opacity)).unwrap();
        let value = Evaluator::new(&program)
            .sample_tile("m", "out", 0, [0., 0.], 0.01)
            .unwrap();
        for (a, b) in value.color.into_iter().zip(expected) {
            close(a, b);
        }
        assert!(value.distance.is_none());
        metal::compile(&program, "m", Channel::BaseColor).unwrap();
        #[cfg(feature = "wgpu")]
        denrim_materials::wgsl::compile(&program, "m", Channel::BaseColor).unwrap();
    }
}
fn gradient(angle: f32, scale: f32, offset: f32) -> String {
    format!(
        r#"project 1
material "m" called "Gradient"
  row "r"
    use "Circle"
      set "radius" to 0.4
    use "Directional Gradient"
      set "angle" to {angle}
      set "scale" to {scale}
      set "offset" to {offset}
  output base color from row "r" xyz
  output opacity from row "r" w
"#
    )
}
#[test]
fn directional_gradient_has_stable_axes_clamps_and_preserves_the_shape_mask_and_distance() {
    for (angle, point, expected) in [
        (0., [0.2, 0.1], 0.7),
        (90., [0.2, 0.1], 0.6),
        (180., [0., 0.2], 0.5),
        (270., [0.2, 0.1], 0.4),
    ] {
        let program = compile(&gradient(angle, 1., 0.5)).unwrap();
        let evaluator = Evaluator::new(&program);
        let shape = evaluator.sample_tile("m", "r", 0, point, 0.01).unwrap();
        let value = evaluator.sample_tile("m", "r", 1, point, 0.01).unwrap();
        close(value.color[0], expected);
        close(value.color[3], shape.color[3]);
        assert_eq!(value.distance, shape.distance);
        let outside = evaluator.sample_tile("m", "r", 1, [2., 2.], 0.01).unwrap();
        assert_eq!(outside.color[3], 0.);
        assert!((0. ..=1.).contains(&outside.color[0]));
        metal::compile(&program, "m", Channel::BaseColor).unwrap();
        #[cfg(feature = "wgpu")]
        denrim_materials::wgsl::compile(&program, "m", Channel::BaseColor).unwrap();
    }
    assert!(
        Kernel::find("Directional Gradient")
            .unwrap()
            .requires_input()
    );
    assert!(
        Kernel::find("Alpha Over")
            .unwrap()
            .metadata()
            .clears_distance
    );
}
