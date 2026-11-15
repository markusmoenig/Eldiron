//! Focused release shader/pipeline compile check, without a window or project.
//! Broad workflow, visual, residency and performance acceptance run separately.
use renderkit::{AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, wgpu};
use scenevm::{
    Atom, Raster3DSurfacePaintEntry, RenderMode, SceneVM, SharedAtlas,
    paint_pages::PaintPageBinding, render_output::SceneRenderTarget,
};
use std::sync::Arc;

fn main() {
    let context = pollster::block_on(GpuContext::headless()).expect("headless device");
    let mut scene =
        SceneVM::new_with_context_and_atlas(context.clone(), 4, 4, SharedAtlas::new(64, 64));
    scene.execute(Atom::SetRenderMode(RenderMode::Raster3D));
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
    let mut target = SceneRenderTarget::new(context.clone(), 4, 4).unwrap();
    for (revision, missing) in [false, true, false].into_iter().enumerate() {
        let empty = Raster3DSurfacePaintEntry {
            geo: [0; 4],
            uv_origin: [0; 2],
            uv_size: [0; 2],
            atlas_rect: [0; 4],
        };
        let entries: Arc<[_]> = vec![
            Raster3DSurfacePaintEntry {
                geo: [1; 4],
                uv_origin: [0; 2],
                uv_size: [64; 2],
                atlas_rect: if missing { [0; 4] } else { [0, 0, 64, 64] },
            },
            empty,
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
                Some(scenevm::paint_feedback::PaintPageStatus::Ready)
            );
        }
    }
    assert!(!target.has_cpu_storage() && !target.has_readback_storage());
    assert!(!SceneVM::compatibility_gpu_initialized());
    println!(
        "Raster resident/paged/resident pipelines compiled; per-view four-byte receipt verified (empty geometry)."
    );
}
