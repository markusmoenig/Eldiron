use renderkit::{GpuContext, ReadbackTicket};
use scenevm::{Atom, RenderMode, SceneVM, prelude::Vec4, render_output::SceneRenderTarget};
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn injected_scene_outputs_remain_gpu_resident_and_independent() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut a = SceneVM::new_with_context(context.clone(), 17, 9);
    let mut b = SceneVM::new_with_context(context.clone(), 11, 7);
    a.execute(Atom::SetRenderMode(RenderMode::Raster2D));
    b.execute(Atom::SetRenderMode(RenderMode::Raster2D));
    a.execute(Atom::SetBackground(Vec4::new(1., 0., 0., 1.)));
    b.execute(Atom::SetBackground(Vec4::new(0., 1., 0., 1.)));
    let mut ta = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    let mut tb = SceneRenderTarget::new(context.clone(), 11, 7).unwrap();
    let oa = a.render_gpu(&mut ta, 1).unwrap();
    let ob = b.render_gpu(&mut tb, 2).unwrap();
    assert!(!ta.has_cpu_storage() && !ta.has_readback_storage());
    assert!(!tb.has_cpu_storage() && !tb.has_readback_storage());
    assert_eq!(
        (oa.image.descriptor.width, ob.image.descriptor.width),
        (17, 11)
    );
    assert_eq!(context.transfers().readback_bytes, 0);
    let mut ca = ReadbackTicket::capture(&context, oa.image).unwrap();
    let mut cb = ReadbackTicket::capture(&context, ob.image).unwrap();
    context
        .device
        .poll(renderkit::wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let aa = ca.try_finish(&context).unwrap().unwrap();
    let bb = cb.try_finish(&context).unwrap().unwrap();
    assert_eq!(&aa.pixels()[..4], &[255, 0, 0, 255]);
    assert_eq!(&bb.pixels()[..4], &[0, 255, 0, 255]);
}
