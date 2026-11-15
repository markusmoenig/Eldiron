#![cfg(feature = "gpu")]
use renderkit::{GpuContext, ReadbackTicket};
use scenevm::{Atom, RenderMode, SceneVM, prelude::Vec4, render_output::SceneRenderTarget};
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn injected_scene_outputs_remain_gpu_resident_and_independent() {
    assert!(!SceneVM::compatibility_gpu_initialized());
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut a = SceneVM::new_with_context(context.clone(), 17, 9);
    let mut b = SceneVM::new_with_context(context.clone(), 11, 7);
    assert!(!a.has_compatibility_target_storage());
    assert!(!b.has_compatibility_target_storage());
    a.execute(Atom::SetRenderMode(RenderMode::Raster2D));
    b.execute(Atom::SetRenderMode(RenderMode::Raster2D));
    a.execute(Atom::SetBackground(Vec4::new(1., 0., 0., 1.)));
    b.execute(Atom::SetBackground(Vec4::new(0., 1., 0., 1.)));
    let mut ta = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    let mut tb = SceneRenderTarget::new(context.clone(), 11, 7).unwrap();
    let oa = a.render_gpu(&mut ta, 1).unwrap();
    let ob = b.render_gpu(&mut tb, 2).unwrap();
    assert!(!a.has_compatibility_target_storage());
    assert!(!b.has_compatibility_target_storage());
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
    // A host replacement retains CPU scene policy and retires all derived
    // resources. Old leased outputs remain separate from the new domain.
    let replacement = pollster::block_on(GpuContext::headless()).unwrap();
    let old_source = a.vm.source2d.clone();
    let old_background = a.vm.background;
    a.rebind_context(replacement.clone());
    assert!(!a.has_compatibility_target_storage());
    assert_eq!(a.device_generation(), Some(replacement.generation()));
    assert_eq!(a.vm.source2d, old_source);
    assert_eq!(a.vm.background, old_background);
    assert!(a.vm.gpu.is_none());
    assert!(
        a.render_gpu(&mut ta, 3).is_err(),
        "old target cannot enter the new device domain"
    );
    let mut recovered = SceneRenderTarget::new(replacement.clone(), 17, 9).unwrap();
    let output = a.render_gpu(&mut recovered, 4).unwrap();
    assert!(!a.has_compatibility_target_storage());
    assert_eq!(replacement.transfers().readback_bytes, 0);
    assert!(!recovered.has_cpu_storage() && !recovered.has_readback_storage());
    let mut capture = ReadbackTicket::capture(&replacement, output.image).unwrap();
    replacement
        .device
        .poll(renderkit::wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let pixels = capture.try_finish(&replacement).unwrap().unwrap();
    assert_eq!(&pixels.pixels()[..4], &[255, 0, 0, 255]);
    assert_eq!(
        b.device_generation(),
        Some(context.generation()),
        "the other view retains its original owner"
    );
    let mut requested = vec![0; 17 * 9 * 4];
    a.render_frame(&mut requested, 17, 9);
    assert_eq!(&requested[..4], &[255, 0, 0, 255]);
    assert!(a.has_compatibility_target_storage());
    let mut blit = SceneVM::new_with_context_and_atlas(
        replacement.clone(),
        2,
        2,
        scenevm::SharedAtlas::new(64, 64),
    );
    assert!(!blit.has_compatibility_target_storage());
    let mut source = scenevm::Texture::new(2, 2);
    source.data = vec![10, 20, 30, 255].repeat(4);
    blit.blit_texture(&mut source, &mut [], 2, 2);
    assert!(blit.has_compatibility_target_storage());
    assert!(!SceneVM::compatibility_gpu_initialized());
}
