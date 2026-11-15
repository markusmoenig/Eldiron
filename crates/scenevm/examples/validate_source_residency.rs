//! Focused release contract for cooperative instance packing and source retention.
//! Explicit captures below are validation only, never normal viewport storage.
use renderkit::{GpuContext, ReadbackTicket, wgpu};
use scenevm::{
    Atom, Camera3D, CameraKind, OrganicBillboardInstance, OrganicBillboardSprite, RenderMode,
    SceneVM, SharedAtlas, prelude::Vec3, render_output::SceneRenderTarget,
    source_upload::SourceUploadBudget,
};
use std::sync::Arc;

fn main() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    for mode in [RenderMode::Raster3D, RenderMode::Compute3D] {
        let mut scene =
            SceneVM::new_with_context_and_atlas(context.clone(), 32, 32, SharedAtlas::new(64, 64));
        scene.execute(Atom::SetRenderMode(mode));
        scene.set_background_lighting(false);
        scene.execute(Atom::SetCamera3D {
            camera: Camera3D::builder()
                .with_kind(CameraKind::OrthoIso)
                .with_pos(Vec3::new(0., 0., 4.))
                .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
                .with_ortho_half_h(1.2),
        });
        let source = OrganicBillboardSprite {
            width: 256,
            height: 256,
            rgba: vec![190, 70, 25, 255].repeat(256 * 256).into(),
        };
        let instance = OrganicBillboardInstance {
            center: [0.; 3],
            width: 1.,
            height: 1.,
            sprite_index: 0,
            flags: 0,
        };
        let mut target = SceneRenderTarget::new(context.clone(), 32, 32).unwrap();
        let mut baseline = None;
        for count in [10_000usize, 10_001] {
            let mut instances = vec![instance; count];
            for hidden in &mut instances[1..] {
                hidden.center[0] = 100.;
            }
            scene.execute(Atom::SetOrganicBillboards {
                sprites: vec![source.clone()],
                instances,
            });
            let expected = 32 + count * 32 + 16 + 256 * 256 * 4;
            assert_eq!(scene.vm.compute_organic_source_bytes(), expected as u64);
            let mut budget = SourceUploadBudget::default();
            let mut packed = 0;
            let mut uploaded = 0;
            let mut copied = 0;
            let mut attempts = 0;
            loop {
                attempts += 1;
                assert!(attempts < 1000, "source preparation starved");
                budget.reset();
                let ready = scene.prepare_source_uploads(&mut budget).unwrap();
                assert!(budget.packed_cpu_bytes() <= 512 * 1024);
                assert!(budget.uploaded_bytes() <= 512 * 1024);
                packed += budget.packed_cpu_bytes();
                uploaded += budget.uploaded_bytes();
                copied += budget.gpu_copied_bytes();
                if ready {
                    break;
                }
            }
            if count == 10_000 {
                assert!(attempts > 1, "oversized CPU packet must yield");
                assert_eq!(packed, expected);
                assert!(uploaded >= expected);
            } else {
                assert_eq!(packed, 32 + count * 32, "retain packed source pixels");
                assert_eq!(
                    copied,
                    16 + 256 * 256 * 4,
                    "copy unchanged source tail on GPU"
                );
                assert!(uploaded <= 32 + count * 32 + 1024);
            }
            assert_eq!(scene.render_target_cpu_bytes(), 0);
            assert!(!target.has_cpu_storage() && !target.has_readback_storage());
            scene.render_gpu(&mut target, count as u64).unwrap();
            let pixels = capture(&context, target.image());
            if let Some(expected) = &baseline {
                assert_eq!(expected, &pixels, "instance count preserves visible source");
            } else {
                assert!(
                    pixels.chunks_exact(4).any(|pixel| pixel[0] > pixel[1] + 20),
                    "billboard must be visible"
                );
                baseline = Some(pixels);
            }
            println!(
                "{mode:?} count={count}: attempts={attempts} packed={packed} uploaded={uploaded} gpu_copied={copied}"
            );
        }
    }
}
fn capture(context: &GpuContext, image: Arc<renderkit::GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(context, image).unwrap();
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
