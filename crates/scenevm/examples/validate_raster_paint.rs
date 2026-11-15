//! Focused release paint receipt/regional replay check, without a window or project.
//! Broad workflow, visual, residency and performance acceptance run separately.
use renderkit::{AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, wgpu};
use scenevm::{
    Atom, Camera3D, CameraKind, Chunk, DynamicObject, GeoId, Poly3D, Raster3DSurfacePaintEntry,
    RenderMode, SceneVM, SharedAtlas,
    paint_pages::{PaintDispatchRegion, PaintPageBinding},
    prelude::{Vec2, Vec3},
    render_output::SceneRenderTarget,
};
use std::sync::Arc;

fn main() {
    let context = pollster::block_on(GpuContext::headless()).expect("headless device");
    let size = [31, 23];
    let atlas = SharedAtlas::new(64, 64);
    let tile = uuid::Uuid::new_v4();
    atlas.add_tile(tile, 1, 1, vec![vec![210, 80, 40, 255]], vec![]);
    let mut scene = SceneVM::new_with_context_and_atlas(context.clone(), size[0], size[1], atlas);
    scene.execute(Atom::SetRenderMode(RenderMode::Raster3D));
    scene.set_background_lighting(false);
    scene.execute(Atom::SetCamera3D {
        camera: Camera3D::builder()
            .with_kind(CameraKind::OrthoIso)
            .with_pos(Vec3::new(0., 0., 4.))
            .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
            .with_ortho_half_h(1.2),
    });
    let mut cube = Poly3D::cube(GeoId::Sector(1), tile, Vec3::zero(), 1.4);
    cube.paint_surface_id = Some([1; 4]);
    cube.paint_uvs = vec![[0.5; 2]; cube.vertices.len()];
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(cube);
    scene.execute(Atom::AddChunk {
        id: uuid::Uuid::new_v4(),
        chunk,
    });
    scene.vm.set_entity_pixelization(1, 3);
    scene.execute(Atom::AddDynamic {
        object: DynamicObject::billboard_tile(
            GeoId::Character(9),
            tile,
            Vec3::new(0.4, 0.2, 1.),
            Vec3::unit_x(),
            Vec3::unit_y(),
            0.5,
            0.8,
        ),
    });
    let image = |format| {
        GpuImage::new(
            &context,
            ImageDescriptor {
                width: 64,
                height: 64,
                format,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Straight,
            },
        )
        .unwrap()
    };
    let color = image(wgpu::TextureFormat::Rgba8Unorm);
    let material = image(wgpu::TextureFormat::Rgba8Uint);
    let mut target = SceneRenderTarget::new(context.clone(), size[0], size[1]).unwrap();
    for (revision, missing) in [false, true, false].into_iter().enumerate() {
        let empty = Raster3DSurfacePaintEntry {
            geo: [0; 4],
            uv_origin: [0; 2],
            uv_size: [0; 2],
            atlas_rect: [0; 4],
        };
        // FNV hash([1;4], [0;2]) selects slot 1 in this two-slot table.
        let entries: Arc<[_]> = vec![
            empty,
            Raster3DSurfacePaintEntry {
                geo: [1; 4],
                uv_origin: [0; 2],
                uv_size: [64; 2],
                atlas_rect: if missing { [0; 4] } else { [0, 0, 64, 64] },
            },
        ]
        .into();
        let binding = PaintPageBinding::new(
            &context,
            color.clone(),
            material.clone(),
            entries,
            Vec::new(),
        )
        .unwrap();
        scene.set_paint_page_binding(&context, binding).unwrap();
        scene.render_gpu(&mut target, revision as u64).unwrap();
        let receipt = scene.vm.capture_paint_page_status(&context).unwrap();
        assert_eq!(
            receipt.is_some(),
            missing,
            "only missing-key tables request metadata receipts"
        );
        if let Some(mut receipt) = receipt {
            context
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(20)),
                })
                .unwrap();
            assert_eq!(
                receipt.try_finish(&context).unwrap(),
                Some(scenevm::paint_feedback::PaintPageStatus::MissingEntry(1))
            );
        }
    }
    for samples in [1, 4] {
        scene.execute(Atom::SetRaster3DMsaaSamples(samples));
        scene.render_gpu(&mut target, 10).unwrap();
        let reference = renderkit::ReadbackTicket::capture(&context, target.image()).unwrap();
        let reference = finish(&context, reference);
        let [left, right] = PaintDispatchRegion::full(size).split().unwrap();
        for (index, region) in [left, left, right].into_iter().enumerate() {
            assert!(scene.set_layer_paint_dispatch(0, Some(region)));
            scene.render_gpu(&mut target, 11 + index as u64).unwrap();
        }
        let result = renderkit::ReadbackTicket::capture(&context, target.image()).unwrap();
        assert_eq!(
            reference,
            finish(&context, result),
            "regional color/depth/entity replay, samples={samples}"
        );
        scene.set_layer_paint_dispatch(0, None);
    }
    assert!(!target.has_cpu_storage() && !target.has_readback_storage());
    assert!(!SceneVM::compatibility_gpu_initialized());
    println!(
        "Raster fragment fault receipt verified; regional retries match full geometry/entity rendering at 1x/4x samples."
    );
}

fn finish(context: &GpuContext, mut ticket: renderkit::ReadbackTicket) -> Vec<u8> {
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(20)),
        })
        .unwrap();
    ticket
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}
