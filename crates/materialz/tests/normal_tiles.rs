use denrim_materials::{
    compile,
    cpu::{Evaluator, PreviewMode},
    format,
    ir::{Channel, Limits, Program},
    metadata::Kernel,
    metal,
};

const ICON: &str = include_str!("../examples/icon.material");
const RASTER: &str = include_str!("../examples/raster-border.material");
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

#[test]
fn color_is_an_ordinary_source_and_preserves_an_incoming_mask() {
    let source = "project 1\nmaterial \"m\" called \"M\"\n  row \"color\" called \"Color\"\n    use \"Color\"\n      set \"color\" to 0.1 0.2 0.3 0.5\n    use \"Color\"\n      set \"color\" to 0.4 0.5 0.6 0.5\n  output base color from row \"color\" xyz\n";
    let p = compile(source).unwrap();
    let e = Evaluator::new(&p);
    let first = e.sample_tile("m", "color", 0, [0., 0.], 0.001).unwrap();
    let second = e.sample_tile("m", "color", 1, [0., 0.], 0.001).unwrap();
    assert_eq!(first.color, [0.1, 0.2, 0.3, 0.5]);
    assert_eq!(second.color, [0.4, 0.5, 0.6, 0.25]);
    assert!(second.distance.is_none());
    let p = compile(&ICON.replace(
        "set \"color\" to 0.06 0.32 0.65 1",
        "set \"color\" to 0.06 0.32 0.65 0.5",
    ))
    .unwrap();
    let e = Evaluator::new(&p);
    near(
        e.sample_tile("icon", "styled", 0, [0.37, 0.], 0.01)
            .unwrap()
            .color[3],
        0.25,
    );
}

#[test]
fn noise_and_color_blend_use_shape_masks_without_special_styling_nodes() {
    let p = compile(ICON).unwrap();
    let e = Evaluator::new(&p);
    for uv in [[0., 0.], [0.16, 0.11], [0.37, 0.], [0.49, 0.49]] {
        let shape = e.sample_tile("icon", "silhouette", 1, uv, 0.01).unwrap();
        for tile in 0..3 {
            let sample = e.sample_tile("icon", "textured", tile, uv, 0.01).unwrap();
            assert_eq!(sample.distance, shape.distance);
            near(sample.color[3], shape.color[3]);
        }
    }
    let pixels = e
        .render_tile("icon", "textured", 2, PreviewMode::Image, 48, 48)
        .unwrap();
    assert!(pixels.windows(2).any(|p| p[0] != p[1]));
}

#[test]
fn border_uses_image_alpha_when_distance_is_absent() {
    let p = compile(RASTER).unwrap();
    let e = Evaluator::new(&p);
    let middle = e.sample_tile("image", "icon", 3, [0., 0.], 0.001).unwrap();
    assert!(middle.distance.is_none());
    assert_eq!(middle.color, [0.08, 0.45, 0.7, 1.]);
    let edge = e
        .sample_tile("image", "icon", 3, [0.34, 0.], 0.001)
        .unwrap();
    near(edge.color[0], 1.);
    near(edge.color[1], 0.7);
    near(edge.color[2], 0.15);
    for uv in [[0.351, 0.], [0.35, 0.], [0.49, 0.49]] {
        let before = e.sample_tile("image", "icon", 2, uv, 0.01).unwrap();
        let after = e.sample_tile("image", "icon", 3, uv, 0.01).unwrap();
        near(before.color[3], after.color[3]);
    }
    let shader = metal::compile_tile_preview(&p, "image", "icon", 3, PreviewMode::Image).unwrap();
    assert!(shader.source.contains("eroded = min"));
    let p = compile(ICON).unwrap();
    assert!(
        !metal::compile_tile_preview(&p, "icon", "styled", 1, PreviewMode::Image)
            .unwrap()
            .source
            .contains("eroded = min")
    );
}

#[test]
fn alpha_sampling_is_counted_in_the_complexity_budget() {
    let ast = format::parse(RASTER).unwrap();
    let p = Program::lower(&ast).unwrap();
    let id = p.tile("image", "icon", 3).unwrap();
    assert_eq!(p.costs[id.0].expanded_calls, 148);
    assert!(
        Program::lower_with_limits(
            &ast,
            Limits {
                expanded_calls: 100,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .to_string()
        .contains("Sampling complexity")
    );
    let nested = RASTER.replace(
        "  output base color",
        "    use \"Border\"\n  output base color",
    );
    assert!(
        compile(&nested)
            .unwrap_err()
            .to_string()
            .contains("Sampling complexity")
    );
    assert_eq!(Kernel::find("Color"), Some(Kernel::Color));
    assert_eq!(Kernel::find("SDF Fill"), None);
    assert_eq!(Kernel::find("SDF Border"), None);
    assert_eq!(Kernel::find("SDF Circle"), None);
    let p = compile(ICON).unwrap();
    assert!(
        Evaluator::new(&p)
            .sample("icon", Channel::Opacity, [0., 0.])
            .is_ok()
    );
}
