use denrim_materials::{
    compile,
    cpu::{Evaluator, PreviewMode, combine_distance},
    format::{self, Combine},
    ir::Channel,
    live::Document,
    metal,
};
const ICON: &str = include_str!("../examples/icon.material");
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

#[test]
fn shape_rows_carry_images_and_distance_through_styles() {
    let p = compile(ICON).unwrap();
    let e = Evaluator::new(&p);
    let shape = e
        .sample_tile("icon", "silhouette", 1, [0., 0.], 0.001)
        .unwrap();
    let fill = e.sample_tile("icon", "styled", 0, [0., 0.], 0.001).unwrap();
    let border = e.sample_tile("icon", "styled", 1, [0., 0.], 0.001).unwrap();
    assert!(shape.distance.unwrap() < 0.);
    assert_eq!(shape.distance, fill.distance);
    assert_eq!(fill.distance, border.distance);
    assert_eq!(border.color, [0.06, 0.32, 0.65, 1.]);
    let hole = e
        .sample_tile("icon", "styled", 1, [0.16, 0.11], 0.001)
        .unwrap();
    assert!(hole.distance.unwrap() > 0.);
    assert_eq!(hole.color[3], 0.);
    let outside = e
        .sample_tile("icon", "styled", 1, [0.49, 0.49], 0.001)
        .unwrap();
    assert_eq!(outside.color[3], 0.);
    let edge = e
        .sample_tile("icon", "styled", 1, [0.35, 0.], 0.001)
        .unwrap();
    close(edge.color[0], 1.);
    close(edge.color[1], 0.72);
    close(edge.color[2], 0.15);
    close(
        e.sample("icon", Channel::Distance, [0., 0.]).unwrap()[0],
        shape.distance.unwrap(),
    );
}
#[test]
fn border_does_not_double_antialias_coverage() {
    let p = compile(ICON).unwrap();
    let e = Evaluator::new(&p);
    for x in [0.367, 0.37, 0.373] {
        let a = e.sample_tile("icon", "styled", 0, [x, 0.], 0.01).unwrap();
        let b = e.sample_tile("icon", "styled", 1, [x, 0.], 0.01).unwrap();
        close(a.color[3], b.color[3]);
    }
    close(
        e.sample_tile("icon", "styled", 1, [0.37, 0.], 0.01)
            .unwrap()
            .color[3],
        0.5,
    );
}
#[test]
fn boolean_shape_combinations_have_defined_signs() {
    close(combine_distance(-0.2, 0.1, Combine::Union, 0.1), -0.2);
    close(combine_distance(-0.2, -0.1, Combine::Subtract, 0.1), 0.1);
    close(combine_distance(-0.2, 0.1, Combine::Intersect, 0.1), 0.1);
    close(combine_distance(0., 0., Combine::SmoothUnion, 0.2), -0.05);
    close(combine_distance(-0.2, 0.1, Combine::Replace, 0.1), 0.1);
    let ast = format::parse(ICON).unwrap();
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
}
#[test]
fn every_tile_has_an_image_thumbnail_and_sdf_tiles_have_extra_modes() {
    let p = compile(ICON).unwrap();
    let e = Evaluator::new(&p);
    for mode in [
        PreviewMode::Image,
        PreviewMode::Shape,
        PreviewMode::Distance,
    ] {
        let pixels = e.render_tile("icon", "styled", 1, mode, 32, 32).unwrap();
        assert_eq!(pixels.len(), 1024);
        assert!(pixels.iter().flatten().all(|v| v.is_finite()));
        assert!(
            metal::compile_tile_preview(&p, "icon", "styled", 1, mode)
                .unwrap()
                .source
                .contains("MzSample")
        );
    }
    let p = compile(include_str!("../examples/warped-disc.material")).unwrap();
    assert!(
        Evaluator::new(&p)
            .render_tile("demo", "noise", 0, PreviewMode::Image, 16, 16)
            .is_ok()
    );
    assert!(
        Evaluator::new(&p)
            .render_tile("demo", "noise", 0, PreviewMode::Shape, 16, 16)
            .is_err()
    );
    assert_eq!(
        Evaluator::new(&p)
            .sample_tile("demo", "noise", 0, [0., 0.], 0.01)
            .unwrap()
            .distance,
        None
    );
}
#[test]
fn uniform_transform_scales_distance_nonuniform_or_warped_clears_it() {
    let base = "project 1\nmaterial \"m\" called \"M\"\n  row \"s\" called \"S\"\n    use \"Circle\"\n      set \"radius\" to 0.2\n    use \"Transform\"\n      set \"globalScale\" to 2\n  output base color from row \"s\" xyz\n";
    let p = compile(base).unwrap();
    let e = Evaluator::new(&p);
    close(
        e.sample_tile("m", "s", 1, [0.3, 0.], 0.001)
            .unwrap()
            .distance
            .unwrap(),
        -0.1,
    );
    let nonuniform = base.replace("set \"globalScale\" to 2", "set \"localScale\" to 1 2");
    let p = compile(&nonuniform).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample_tile("m", "s", 1, [0., 0.], 0.001)
            .unwrap()
            .distance,
        None
    );
    let with_border = nonuniform.replace(
        "  output base color",
        "    use \"Border\"\n  output base color",
    );
    let p = compile(&with_border).unwrap();
    let raster = Evaluator::new(&p)
        .sample_tile("m", "s", 2, [0.19, 0.], 0.001)
        .unwrap();
    assert_eq!(raster.distance, None);
    close(raster.color[0], 1.);
    close(raster.color[1], 0.8);
    let mut d = Document::parse(base).unwrap();
    let mut shader =
        metal::compile_tile_preview(d.program(), "m", "s", 1, PreviewMode::Shape).unwrap();
    d.set_constant("m", "s", 1, "globalScale", &[0.5]).unwrap();
    shader.refresh(d.program()).unwrap();
}
#[test]
fn image_operators_preserve_the_distance_and_noise_can_use_it_as_a_parameter() {
    let source=ICON.replace("    use \"Border\"", "    use \"Multiply\"\n      set \"globalValue\" from row \"silhouette\" distance\n    use \"Perlin Noise\"\n    use \"Border\"");
    let p = compile(&source).unwrap();
    let e = Evaluator::new(&p);
    let before = e.sample_tile("icon", "styled", 0, [0., 0.], 0.001).unwrap();
    let after = e.sample_tile("icon", "styled", 3, [0., 0.], 0.001).unwrap();
    assert_eq!(before.distance, after.distance);
}
#[test]
fn invalid_distance_inputs_and_dimensions_are_rejected() {
    let no_shape = ICON
        .replace("use \"Rounded Rectangle\"", "use \"Perlin Noise\"")
        .replace("      set \"halfSize\" to 0.37 0.32\n", "")
        .replace("      set \"radius\" to 0.09\n", "");
    assert!(
        compile(&no_shape)
            .unwrap_err()
            .to_string()
            .contains("incoming distance")
    );
    assert!(compile(&ICON.replace("combine subtract", "combine imaginary")).is_err());
    assert!(
        compile(&ICON.replace(
            "combine subtract",
            "combine replace\n      combine subtract"
        ))
        .is_err()
    );
    assert!(compile(&ICON.replace("set \"radius\" to 0.115", "set \"radius\" to -1")).is_err());
    assert!(compile(&ICON.replace("set \"width\" to 0.025", "set \"width\" to -1")).is_err());
}

#[test]
fn spatially_varying_shapes_and_transforms_do_not_claim_a_metric_distance() {
    let prefix = "project 1\nmaterial \"m\" called \"M\"\n  row \"noise\" called \"Noise\"\n    use \"Perlin Noise\"\n  row \"s\" called \"Shape\"\n    use \"Circle\"\n";
    for body in [
        "      set \"radius\" from row \"noise\" x\n",
        "    use \"Transform\"\n      set \"offset\" from row \"noise\" xy\n",
        "    use \"Transform\"\n      set \"rot\" from row \"noise\" x\n",
    ] {
        let source = format!("{prefix}{body}  output base color from row \"s\" xyz\n");
        let p = compile(&source).unwrap();
        let e = Evaluator::new(&p);
        let tile = p.nodes[p.materials[0].rows[1].1.0].tile_index;
        assert_eq!(
            e.sample_tile("m", "s", tile, [0., 0.], 0.001)
                .unwrap()
                .distance,
            None
        );
        assert!(
            e.render_tile("m", "s", tile, PreviewMode::Image, 16, 16)
                .is_ok()
        );
        assert!(
            compile(&source.replace(
                "output base color from row \"s\" xyz",
                "output distance from row \"s\" distance"
            ))
            .is_err()
        );
    }
}
