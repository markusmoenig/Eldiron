use denrim_materials::{
    cpu::Evaluator,
    format,
    ir::{Channel, Program},
    metadata::Kernel,
};

#[test]
fn canonical_fast_tiles_match_the_interpreter_including_alpha_and_channel_mapping() {
    let source = r#"project 1
material "surface" called "Surface"
  mode pbr
  row "mask"
    use "Color" id "mask"
      set "color" to 0.4 0.7 0.2 0.6
  row "grain"
    use "Perlin Noise" id "grain"
      input from row "mask" xyz
      set "globalScale" to 0.7
      set "localScale" to 0.5 2
  row "color"
    use "Mix" id "mix"
      input from row "grain" yxz mapped -0.2 to 1.2
      set "color1" to 0.05 0.2 0.3 0.4
      set "color2" to 0.7 0.8 0.9 0.8
      set "factor" to 0.3 0.6 1.2
  output base color from row "color" xyz
  output opacity from row "color" w
"#;
    let parsed = format::parse(source).unwrap();
    let fast = Program::lower(&parsed).unwrap();
    let mut interpreted = parsed.clone();
    for tile in interpreted
        .materials
        .iter_mut()
        .flat_map(|m| &mut m.rows)
        .flat_map(|r| &mut r.tiles)
    {
        let kernel = Kernel::find(&tile.kernel).unwrap();
        tile.code = Some(format!("{}\n", denrim_materials::tilelang::source(kernel)));
    }
    let interpreted = Program::lower(&interpreted).unwrap();
    for x in -3..=3 {
        for y in -3..=3 {
            let uv = [x as f32 * 0.17, y as f32 * 0.19];
            for channel in [Channel::BaseColor, Channel::Opacity] {
                let a = Evaluator::new(&fast)
                    .sample("surface", channel, uv)
                    .unwrap();
                let b = Evaluator::new(&interpreted)
                    .sample("surface", channel, uv)
                    .unwrap();
                assert!(
                    a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 0.00001),
                    "{uv:?}: {a:?} vs {b:?}"
                );
            }
        }
    }
}
