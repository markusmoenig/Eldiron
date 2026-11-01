use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel};
const RECIPE: &str = r#"project 1
material "water" called "Water"
  mode pbr
  animation 16 2
  row "ripples"
    use "Perlin Noise" id "noise"
    use "Loop Flow" id "flow"
      set "velocity" to 0 0.35
  output base color from row "ripples" xyz
"#;
#[test]
fn normalized_loop_moves_and_closes_and_metadata_roundtrips() {
    let document = format::parse(RECIPE).unwrap();
    assert_eq!(document.materials[0].animation.frames, 16);
    assert_eq!(format::parse(&format::write(&document)).unwrap(), document);
    let program = compile(RECIPE).unwrap();
    let eval = Evaluator::new(&program);
    let mut changed = false;
    for i in 0..16 {
        let uv = [i as f32 / 9., 0.173];
        let a = eval
            .sample_at_time("water", Channel::BaseColor, uv, 0.)
            .unwrap();
        let b = eval
            .sample_at_time("water", Channel::BaseColor, uv, 0.5)
            .unwrap();
        let c = eval
            .sample_at_time("water", Channel::BaseColor, uv, 1.)
            .unwrap();
        assert!(a.iter().zip(c).all(|(x, y)| (x - y).abs() < 0.00001));
        changed |= a.iter().zip(b).any(|(x, y)| (x - y).abs() > 0.01);
    }
    assert!(changed);
    assert!(
        eval.sample_at_time("water", Channel::BaseColor, [0.; 2], f32::NAN)
            .is_err()
    );
    assert!(format::parse(&RECIPE.replace("animation 16 2", "animation 0 2")).is_err());
}

#[test]
fn continuous_bricks_have_multiple_clay_tones_and_do_not_repeat_cell_keys() {
    let recipe = include_str!("../examples/continuous-bricks.material");
    let program = compile(recipe).unwrap();
    let eval = Evaluator::new(&program);
    let mut bands = std::collections::HashSet::new();
    let mut different = false;
    let band = |color: Vec<f32>| {
        if color[0] < 0.29 {
            0
        } else if color[0] < 0.38 {
            1
        } else {
            2
        }
    };
    for y in 0..8 {
        for x in 0..4 {
            let uv = [
                (x as f32 + 0.5 + (y % 2) as f32 * 0.5 + 0.0625) / 2. - 0.5,
                (y as f32 + 0.625) / 4. - 0.5,
            ];
            let color = eval.sample("surface", Channel::BaseColor, uv).unwrap();
            assert!(
                color[0] > 0.2,
                "Brick centers must stay in the clay palette: {color:?}"
            );
            let a = band(color);
            bands.insert(a);
            let b = band(
                eval.sample("surface", Channel::BaseColor, [uv[0] + 1., uv[1]])
                    .unwrap(),
            );
            different |= a != b;
        }
    }
    assert_eq!(
        bands.len(),
        3,
        "Bricks should show three restrained clay tones"
    );
    assert!(
        different,
        "World-space bricks must not repeat one fixed colored brick"
    );
}
#[test]
fn static_frame_count_retains_authored_playback_duration() {
    let doc = format::parse(&RECIPE.replace("animation 16 2", "animation 1 4")).unwrap();
    assert_eq!(format::parse(&format::write(&doc)).unwrap(), doc);
}
