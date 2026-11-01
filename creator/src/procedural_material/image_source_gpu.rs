//! Native verification of image uploads, animation and compositing on the shared GPU.
use super::*;
#[test]
#[ignore = "requires a native GPU"]
fn image_source_gpu_matches_cpu_and_reuses_pixel_upload_pipeline() {
    pollster::block_on(async {
        let adapter = wgpu::Instance::default()
            .request_adapter(&Default::default())
            .await
            .unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let runtime = MaterialZ::new(&device, &queue).await.unwrap();
        let mut asset=EditableMaterialAsset::parse("image","project 1\nmaterial \"m\" called \"Image\"\n  mode pbr\n  tiling raw\n  animation 2 2\n  row \"back\"\n    use \"Color\"\n      set \"color\" to 0.1 0.2 0.3 1\n  row \"image\"\n    use \"Image / Tile Source\" id \"image\"\n  row \"rough\"\n    use \"Image / Tile Source\" id \"rough\"\n      set \"channel\" to 1\n  row \"out\"\n    use \"Alpha Over\"\n      input from row \"back\" xyz\n      set \"foreground\" from row \"image\" xyzw\n  output base color from row \"out\" xyz\n  output opacity from row \"image\" w\n  output roughness from row \"rough\" x\n","m").unwrap();
        let mut image = denrim_materials::image_source::ImageSource::rgba8(
            2,
            2,
            &[
                vec![
                    255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 255, 255, 255, 255,
                ],
                vec![0, 0, 255, 255, 255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0, 128],
            ],
        )
        .unwrap();
        image
            .set_plane("roughness", "linear", &[vec![128; 16]])
            .unwrap();
        for row in &mut asset.root.project.materials[0].rows {
            for tile in &mut row.tiles {
                if tile.kernel == "Image / Tile Source" {
                    tile.image = Some(image.clone());
                }
            }
        }
        let mut instance = runtime
            .compile(&asset, InstanceRecord::new("image-test"))
            .await
            .unwrap();
        let generation = instance.pipeline_generation();
        for filtering in [0., 1.] {
            for row in &mut asset.root.project.materials[0].rows {
                for tile in &mut row.tiles {
                    if tile.kernel == "Image / Tile Source" {
                        tile.parameters.retain(|(id, _)| id != "filter");
                        tile.parameters.push((
                            "filter".into(),
                            denrim_materials::format::Binding::Constant(vec![filtering]),
                        ));
                    }
                }
            }
            if instance.update_asset(&asset).is_err() {
                instance = runtime
                    .compile(&asset, InstanceRecord::new("image-test"))
                    .await
                    .unwrap();
            }
            let program = asset.program(&InstanceRecord::new("cpu")).unwrap();
            let cpu = denrim_materials::cpu::Evaluator::new(&program);
            for time in [0., 0.5] {
                let maps = render_grid_time(
                    &runtime, &instance, &device, &queue, [4; 2], [1.; 2], [-0.5; 2], time,
                )
                .unwrap();
                for y in 0..4 {
                    for x in 0..4 {
                        let uv = [-0.5 + (x as f32 + 0.5) / 4., -0.5 + (y as f32 + 0.5) / 4.];
                        for (map, channel, width) in [
                            (0, denrim_materials::ir::Channel::BaseColor, 3),
                            (1, denrim_materials::ir::Channel::Opacity, 1),
                            (2, denrim_materials::ir::Channel::Roughness, 1),
                        ] {
                            let expected = cpu.sample_at_time("m", channel, uv, time).unwrap();
                            for c in 0..width {
                                assert!(
                                    (maps[map][y * 4 + x][c] - expected[c]).abs() < 0.002,
                                    "filter {filtering} time {time} pixel {x},{y} map {map} component {c}"
                                );
                            }
                        }
                    }
                }
            }
        }
        // Pixel replacement with matching dimensions and sampling keeps pipelines.
        let generation = instance.pipeline_generation().max(generation);
        asset.root.project.materials[0]
            .rows
            .iter_mut()
            .flat_map(|r| &mut r.tiles)
            .find(|t| t.id == "image")
            .unwrap()
            .image
            .as_mut()
            .unwrap()
            .planes[0]
            .frames[0] = "ff00ffff".repeat(4);
        instance.update_asset(&asset).unwrap();
        assert_eq!(generation, instance.pipeline_generation());
    });
}
