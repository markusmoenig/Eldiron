//! Native render harness for the editable shield and sword icon experiments.
//! Run `shield_icon_experiment` or `sword_icon_experiment` with `--ignored`.
use super::*;
use crate::prelude::*;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

const SIZE: usize = 32;
#[derive(Deserialize)]
struct Variant {
    label: String,
    bindings: BTreeMap<String, BTreeMap<String, u16>>,
}
fn asset(name: &str) -> (EditableMaterialAsset, Variant) {
    asset_for("shield", name)
}
fn asset_for(kind: &str, name: &str) -> (EditableMaterialAsset, Variant) {
    let source = crate::Embedded::get(&format!("icon_experiments/{kind}/{kind}.material")).unwrap();
    let variant =
        crate::Embedded::get(&format!("icon_experiments/{kind}/{name}.palette.json")).unwrap();
    let variant: Variant = serde_json::from_slice(variant.data.as_ref()).unwrap();
    let mut asset = EditableMaterialAsset::parse(
        &format!("{kind}-icon-experiment"),
        std::str::from_utf8(source.data.as_ref()).unwrap(),
        kind,
    )
    .unwrap();
    let project = Project::default();
    super::super::palette::ensure_color_bindings(&mut asset, &project.palette).unwrap();
    for (operation, parameters) in &variant.bindings {
        for (parameter, index) in parameters {
            asset = super::super::palette::bind_parameter(
                &asset,
                operation,
                parameter,
                *index,
                &project.palette,
            )
            .unwrap();
        }
    }
    asset.validate().unwrap();
    assert!(
        asset
            .root
            .project
            .materials
            .iter()
            .flat_map(|m| &m.rows)
            .flat_map(|r| &r.tiles)
            .all(|tile| tile.code.is_none()),
        "Icon recipes must use stock tiles only"
    );
    (asset, variant)
}
fn png(path: &Path, width: usize, height: usize, pixels: &[u8]) {
    let mut encoder = png::Encoder::new(
        std::fs::File::create(path).unwrap(),
        width as u32,
        height as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}
#[test]
fn shield_recipe_uses_editable_palette_colors_and_radial_rivets() {
    let (asset, _) = asset("steel-blue");
    let bundle = asset.save().unwrap();
    let reopened = EditableMaterialAsset::reopen(&bundle).unwrap();
    assert_eq!(asset.root.project, reopened.root.project);
    let program = asset.program(&InstanceRecord::new("shield-cpu")).unwrap();
    let evaluator = denrim_materials::cpu::Evaluator::new(&program);
    // Grid Rings places 16 copies around the first ring; later rings lie outside
    // this self-contained icon. Check real coverage at every rivet centre.
    for i in 0..16 {
        let angle = ((i as f32 + 0.5) / 16. - 0.5) * std::f32::consts::TAU;
        let uv = [angle.cos() * 0.359375, angle.sin() * 0.359375];
        let sampled = evaluator
            .sample_tile("shield", "rivets", 0, uv, 1. / SIZE as f32)
            .unwrap();
        assert!(
            sampled.color[3] > 0.99,
            "Missing radial rivet {i}: {:?}",
            sampled.color
        );
    }
    let opacity = |uv| {
        evaluator
            .sample("shield", denrim_materials::ir::Channel::Opacity, uv)
            .unwrap()[0]
    };
    assert_eq!(opacity([0.49, 0.49]), 0.);
    assert_eq!(opacity([0., 0.]), 1.);
}
#[test]
fn sword_recipe_keeps_point_guard_and_grip_connected_after_pose_and_reopen() {
    let (asset, _) = asset_for("sword", "steel-blue");
    let saved = asset.save().unwrap();
    let reopened = EditableMaterialAsset::reopen(&saved).unwrap();
    assert_eq!(asset.root.project, reopened.root.project);
    let program = reopened.program(&InstanceRecord::new("sword-cpu")).unwrap();
    let evaluator = denrim_materials::cpu::Evaluator::new(&program);
    let alpha = |uv| {
        evaluator
            .sample("sword", denrim_materials::ir::Channel::Opacity, uv)
            .unwrap()[0]
    };
    let pose = |p: [f32; 2]| {
        let c = std::f32::consts::FRAC_1_SQRT_2;
        [1.14 * c * (p[0] - p[1]), 1.14 * c * (p[0] + p[1])]
    };
    for point in [
        [0., -0.44],
        [0., -0.2],
        [0., 0.15],
        [0.15625, 0.1171875],
        [0., 0.29],
        [0., 0.421875],
    ] {
        assert_eq!(alpha(pose(point)), 1., "Sword part is missing at {point:?}");
    }
    for step in 0..43 {
        assert_eq!(
            alpha(pose([0., -0.42 + step as f32 * 0.02])),
            1.,
            "The blade, guard, grip and pommel must form a continuous silhouette"
        );
    }
    assert_eq!(alpha([0.49, 0.49]), 0.);
    assert_eq!(alpha([-0.49, -0.49]), 0.);
}
#[test]
#[ignore = "renders 32px procedural shield variants on a native GPU"]
fn shield_icon_experiment() {
    render_experiment("shield", 450);
}
#[test]
#[ignore = "renders 32px procedural sword variants on a native GPU"]
fn sword_icon_experiment() {
    render_experiment("sword", 120);
}
fn render_experiment(kind: &str, minimum_opaque_pixels: usize) {
    let (device, queue) = pollster::block_on(async {
        let adapter = wgpu::Instance::default()
            .request_adapter(&Default::default())
            .await
            .unwrap();
        adapter.request_device(&Default::default()).await.unwrap()
    });
    let runtime = pollster::block_on(MaterialZ::new(&device, &queue)).unwrap();
    let output = std::env::var("ELDIRON_ICON_OUTPUT")
        .unwrap_or_else(|_| format!("/tmp/eldiron-{kind}-experiment"));
    let output = Path::new(&output);
    std::fs::create_dir_all(output).unwrap();
    let mut compiled: Option<MaterialInstance> = None;
    let mut pictures = Vec::new();
    for name in ["steel-blue", "bronze-red", "iron-green"] {
        let (asset, variant) = asset_for(kind, name);
        if let Some(instance) = &mut compiled {
            let generation = instance.pipeline_generation();
            instance.update_asset(&asset).unwrap();
            assert_eq!(
                generation,
                instance.pipeline_generation(),
                "Palette variants should reuse their shader pipeline"
            );
        } else {
            compiled = Some(
                pollster::block_on(
                    runtime.compile(&asset, InstanceRecord::new(&format!("{kind}-gpu"))),
                )
                .unwrap(),
            );
        }
        let maps = render_grid_time(
            &runtime,
            compiled.as_ref().unwrap(),
            &device,
            &queue,
            [SIZE; 2],
            [1.; 2],
            [-0.5; 2],
            0.,
        )
        .unwrap();
        let pixels = preview_images(&maps)[0].clone();
        assert_eq!(pixels.len(), SIZE * SIZE * 4);
        assert!(
            pixels.chunks_exact(4).all(|p| p[3] == 0 || p[3] == 255),
            "Pixel silhouettes must have clean alpha"
        );
        assert!(pixels.chunks_exact(4).filter(|p| p[3] == 255).count() > minimum_opaque_pixels);
        assert_eq!(&pixels[..4], &[0, 0, 0, 0]);
        png(&output.join(format!("{name}.png")), SIZE, SIZE, &pixels);
        std::fs::write(
            output.join(format!("{name}.materialz")),
            denrim_materials::format::write(&asset.root.project),
        )
        .unwrap();
        std::fs::write(
            output.join(format!("{name}.eldiron-material.json")),
            asset.save().unwrap(),
        )
        .unwrap();
        pictures.push((variant.label, pixels));
    }
    assert_ne!(pictures[0].1, pictures[1].1);
    assert_ne!(pictures[0].1, pictures[2].1);
    let theme = TheDarkTheme::new();
    let background = *theme.color(MaterialCanvas);
    let text = *theme.color(MaterialText);
    let checker = [
        *theme.color(MaterialCheckerDark),
        *theme.color(MaterialCheckerLight),
    ];
    let width = 576;
    let height = 240;
    let mut sheet = background.repeat(width * height);
    let draw = TheDraw2D::new();
    for (i, (label, pixels)) in pictures.iter().enumerate() {
        draw.text_rect(
            sheet.as_mut_slice(),
            &(i * 192, 5, 192, 25),
            width,
            label,
            TheFontSettings {
                size: 14.,
                ..Default::default()
            },
            &text,
            &background,
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );
        for (zoom, y0) in [(4, 36), (1, 182)] {
            let side = SIZE * zoom;
            let x0 = i * 192 + (192 - side) / 2;
            for y in 0..side {
                for x in 0..side {
                    let source = &pixels[((y / zoom) * SIZE + x / zoom) * 4..][..4];
                    let destination = ((y0 + y) * width + x0 + x) * 4;
                    let color = if source[3] == 0 {
                        checker[(x / (4 * zoom) + y / (4 * zoom)) % 2]
                    } else {
                        source.try_into().unwrap()
                    };
                    sheet[destination..destination + 4].copy_from_slice(&color);
                }
            }
        }
    }
    png(
        &output.join(format!("{kind}-variants.png")),
        width,
        height,
        &sheet,
    );
    println!("32x32 icons and preview: {}", output.display());
}
