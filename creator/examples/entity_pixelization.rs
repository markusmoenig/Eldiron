//! Native GPU regression for entity-only pixelization, occlusion and live toggling.
//! cargo run -p eldiron-creator --example entity_pixelization -- /tmp/entity-pixels
use scenevm::{
    Atom, Camera3D, CameraKind, DynamicMeshVertex, DynamicObject, GeoId, RenderMode, SceneVM,
};
use theframework::prelude::*;

fn mesh(id: GeoId, tile: Uuid, points: &[[f32; 3]], indices: &[u32]) -> DynamicObject {
    DynamicObject::mesh(
        id,
        tile,
        points
            .iter()
            .map(|p| DynamicMeshVertex {
                position: Vec3::new(p[0], p[1], p[2]),
                uv: Vec2::new((p[0] + 2.) / 4., (p[1] + 2.) / 4.),
                normal: Vec3::unit_z(),
            })
            .collect(),
        indices.to_vec(),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or("/tmp/entity-pixels".into()),
    );
    std::fs::create_dir_all(&out)?;
    let mut vm = SceneVM::new(256, 256);
    vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
    vm.execute(Atom::SetCamera3D {
        camera: Camera3D {
            kind: CameraKind::OrthoIso,
            ortho_half_h: 2.,
            ..Default::default()
        },
    });
    vm.execute(Atom::SetGP0(Vec4::new(0.03, 0.03, 0.03, 0.)));
    vm.execute(Atom::SetGP2(Vec4::new(0., 1., 1., 0.)));
    vm.execute(Atom::SetGP3(Vec4::new(1., 1., 1., 1.)));
    vm.execute(Atom::SetGP8(Vec4::new(0., 0., 1., 1.)));
    vm.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 1.)));
    let tile = Uuid::new_v4();
    let world = Uuid::new_v4();
    vm.execute(Atom::AddSolid {
        id: tile,
        color: [40, 180, 230, 255],
    });
    vm.execute(Atom::AddSolid {
        id: world,
        color: [210, 130, 50, 255],
    });
    vm.execute(Atom::BuildAtlas);
    // A character triangle and an item triangle. Their sloping edges must get coarse.
    for (id, offset) in [(GeoId::Character(1), -0.85), (GeoId::Item(1), 0.85)] {
        vm.execute(Atom::AddDynamic {
            object: mesh(
                id,
                tile,
                &[
                    [offset - 0.65, -1., 0.],
                    [offset + 0.7, -0.85, 0.],
                    [offset - 0.18, 1.4, 0.],
                ],
                &[0, 1, 2],
            ),
        });
    }
    // A thin world rectangle in front. It must remain sharp and occlude either pass.
    vm.execute(Atom::AddDynamic {
        object: mesh(
            GeoId::Unknown(1),
            world,
            &[
                [-1.65, -0.6, 1.],
                [1.65, -0.6, 1.],
                [1.65, -0.37, 1.],
                [-1.65, -0.37, 1.],
            ],
            &[0, 1, 2, 0, 2, 3],
        ),
    });
    let mut native = vec![0; 256 * 256 * 4];
    vm.vm.set_entity_pixelization(0, 1);
    vm.render_frame(&mut native, 256, 256);
    let mut coarse = vec![0; native.len()];
    vm.vm.set_entity_pixelization(32, 8);
    vm.render_frame(&mut coarse, 256, 256);
    let diff = native
        .chunks_exact(4)
        .zip(coarse.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(diff > 100, "Pixelization did not affect geometry: {diff}");
    for y in 155..164 {
        for x in 26..230 {
            let p = (y * 256 + x) * 4;
            assert_eq!(
                &native[p..p + 4],
                &coarse[p..p + 4],
                "Foreground world geometry was changed or hidden at {x},{y}"
            );
        }
    }
    let mut again = vec![0; native.len()];
    vm.vm.set_entity_pixelization(0, 1);
    vm.render_frame(&mut again, 256, 256);
    assert_eq!(
        native, again,
        "Disabling pixelization must restore the native frame"
    );
    std::fs::write(
        out.join("native.png"),
        TheRGBABuffer::from(native, 256, 256).to_png()?,
    )?;
    std::fs::write(
        out.join("pixelized.png"),
        TheRGBABuffer::from(coarse, 256, 256).to_png()?,
    )?;
    // Post effects must not introduce subpixel noise, blur or colors outside the palette.
    let palette = [
        [16u8, 24, 32],
        [25, 93, 160],
        [40, 180, 230],
        [140, 210, 240],
        [210, 130, 50],
        [250, 240, 220],
    ];
    vm.execute(Atom::SetPalette(
        palette
            .iter()
            .map(|c| {
                Vec4::new(
                    (c[0] as f32 / 255.).powf(2.2),
                    (c[1] as f32 / 255.).powf(2.2),
                    (c[2] as f32 / 255.).powf(2.2),
                    1.,
                )
            })
            .collect(),
    ));
    vm.execute(Atom::SetGP9(Vec4::new(1., 0., 1., 2.2)));
    vm.vm
        .set_raster3d_post_style_params(Vec4::new(1., 0.5, 0.2, 0.), Vec4::zero());
    vm.vm.set_entity_pixelization(32, 8);
    let mut crisp = vec![0; 256 * 256 * 4];
    vm.render_frame(&mut crisp, 256, 256);
    let at = (96 * 256 + 64) * 4;
    let color = &crisp[at..at + 4];
    assert!(
        palette
            .iter()
            .any(|c| (0..3).all(|k| (c[k] as i32 - color[k] as i32).abs() <= 1)),
        "Shaded entity is outside the project palette: {color:?}"
    );
    for y in 96..104 {
        for x in 64..72 {
            let p = (y * 256 + x) * 4;
            assert_eq!(
                &crisp[p..p + 4],
                color,
                "A coarse pixel contains blur or fine grain at {x},{y}"
            );
        }
    }
    std::fs::write(
        out.join("crisp-with-post.png"),
        TheRGBABuffer::from(crisp, 256, 256).to_png()?,
    )?;
    vm.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 1.)));
    vm.vm
        .set_raster3d_post_style_params(Vec4::zero(), Vec4::zero());
    vm.execute(Atom::SetPalette(vec![]));
    // Rebuild with single-sample world rendering too (default above is 4x MSAA).
    vm.execute(Atom::SetRaster3DMsaaSamples(0));
    vm.vm.set_entity_pixelization(32, 4);
    vm.render_frame(&mut again, 256, 256);
    std::fs::write(
        out.join("pixelized-no-msaa.png"),
        TheRGBABuffer::from(again, 256, 256).to_png()?,
    )?;
    // Non-multiple dimensions exercise partial blocks and target reallocation.
    let mut odd = vec![0; 255 * 251 * 4];
    vm.render_frame(&mut odd, 255, 251);
    assert!(odd.chunks_exact(4).any(|p| p[2] > 100));

    vm.execute(Atom::ClearDynamics);
    vm.execute(Atom::SetRenderMode(RenderMode::Raster2D));
    vm.execute(Atom::SetTransform2D(Mat3::identity()));
    let image = Uuid::new_v4();
    let mut rgba = vec![0; 64 * 64 * 4];
    for y in 0..64 {
        for x in 0..64 {
            let p = (y * 64 + x) * 4;
            rgba[p..p + 4].copy_from_slice(&[
                (30 + x * 3) as u8,
                (30 + y * 3) as u8,
                190,
                if x + y > 28 { 255 } else { 0 },
            ]);
        }
    }
    vm.execute(Atom::AddTile {
        id: image,
        width: 64,
        height: 64,
        frames: vec![rgba.clone()],
        material_frames: None,
    });
    vm.execute(Atom::SetAvatarBillboardData {
        id: GeoId::Character(2),
        size: 64,
        rgba,
    });
    vm.execute(Atom::BuildAtlas);
    vm.execute(Atom::AddDynamic {
        object: DynamicObject::billboard_avatar_2d(
            GeoId::Character(2),
            Vec2::new(64., 64.),
            96.,
            96.,
        ),
    });
    vm.execute(Atom::AddDynamic {
        object: DynamicObject::billboard_tile_2d(
            GeoId::Item(2),
            image,
            Vec2::new(192., 64.),
            96.,
            96.,
        ),
    });
    vm.execute(Atom::AddDynamic {
        object: DynamicObject::billboard_tile_2d(
            GeoId::Unknown(2),
            image,
            Vec2::new(64., 192.),
            96.,
            96.,
        ),
    });
    let mut native = vec![0; 256 * 256 * 4];
    vm.vm.set_entity_pixelization(0, 1);
    vm.render_frame(&mut native, 256, 256);
    let mut coarse = vec![0; native.len()];
    vm.vm.set_entity_pixelization(8, 4);
    vm.render_frame(&mut coarse, 256, 256);
    let changed = |x0: usize, y0: usize| {
        (y0..y0 + 96)
            .flat_map(|y| (x0..x0 + 96).map(move |x| (y * 256 + x) * 4))
            .filter(|&p| native[p..p + 4] != coarse[p..p + 4])
            .count()
    };
    assert!(changed(16, 16) > 100, "2D character sprite was unaffected");
    assert!(changed(144, 16) > 100, "2D bitmap item was unaffected");
    assert_eq!(changed(16, 144), 0, "2D world tiles must keep their detail");
    vm.vm.set_entity_pixelization(0, 1);
    let mut again = vec![0; native.len()];
    vm.render_frame(&mut again, 256, 256);
    assert_eq!(
        native, again,
        "2D live disable did not restore source detail"
    );
    std::fs::write(
        out.join("2d-native.png"),
        TheRGBABuffer::from(native, 256, 256).to_png()?,
    )?;
    std::fs::write(
        out.join("2d-pixelized.png"),
        TheRGBABuffer::from(coarse, 256, 256).to_png()?,
    )?;
    println!(
        "GPU checks passed: 2D character/bitmap item sampling and world isolation; 3D outlines, world occlusion, live toggling, resizing, MSAA and non-MSAA."
    );
    Ok(())
}
