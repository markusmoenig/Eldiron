#![cfg(feature = "gpu")]
use renderkit::{GpuContext, GpuImage, ReadbackTicket, wgpu};
use scenevm::{Atom, RenderMode, SceneVM, prelude::Vec4, render_output::SceneRenderTarget};
use std::sync::Arc;
fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    ticket
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn disabling_all_scene_layers_clears_previous_output() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut scene = SceneVM::new_with_context(context.clone(), 17, 9);
    scene.execute(Atom::SetRenderMode(RenderMode::Raster2D));
    scene.execute(Atom::SetBackground(Vec4::new(1., 0., 0., 1.)));
    let mut target = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    let first = scene.render_gpu(&mut target, 1).unwrap();
    assert!(
        capture(&context, first.image)
            .chunks_exact(4)
            .all(|p| p == [255, 0, 0, 255])
    );
    scene.vm.set_enabled(false);
    let empty = scene.render_gpu(&mut target, 2).unwrap();
    assert!(
        capture(&context, empty.image).iter().all(|v| *v == 0),
        "a scene with no enabled layers must publish transparent pixels"
    );
}
