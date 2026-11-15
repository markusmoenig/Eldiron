use super::*;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket};
use theframework::{
    paint::*,
    render_wgpu::{PaintRenderer, UiImages},
    theui::paint::WidgetPainter,
};
fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
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
#[ignore = "native GPU; recipe icon publication, fitting and retirement"]
fn native_recipe_icons_keep_reference_pixels_and_scoped_owners() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(context.clone());
    let mut editor = GeometryEditor::default();
    let mut project = Project::default();
    let history = CreatorHistory::default();
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(1000, 760, 1.);
    ui.init(&mut ctx);
    ctx.ui.command_paint = true;
    editor.recipe_browser_event(
        &TheEvent::StateChanged(TheId::named("MZ New Box"), TheWidgetState::Clicked),
        &mut project,
        &mut ui,
        &mut ctx,
        &history,
    );
    let requests = editor.recipe_browser.as_ref().unwrap().native_icons.clone();
    assert_eq!(
        requests.len(),
        super::super::recipe_browser::bundled().len()
    );
    let mut icons = Icons::default();
    let mut warm_handles = Vec::new();
    for step in 1..=requests.len() {
        let (waiting, errors) =
            icons.prepare(Some(&requests), &mut ui, &mut ctx, &mut images.registry);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(icons.entries.len(), step, "one new source per frame");
        assert_eq!(waiting, step < requests.len());
    }
    assert_eq!(context.transfers().readback_bytes, 0);
    assert_eq!(
        context.transfers().uploaded_bytes,
        0,
        "no CPU display textures"
    );
    let pool = context.target_pool_usage();
    let uploads = context.immutable_buffer_cache_usage().uploaded_bytes;
    icons.prepare(Some(&requests), &mut ui, &mut ctx, &mut images.registry);
    assert_eq!(context.target_pool_usage().allocations, pool.allocations);
    assert_eq!(
        context.immutable_buffer_cache_usage().uploaded_bytes,
        uploads
    );
    let theme = TheDarkTheme::new();
    for (id, published) in &icons.entries {
        let image = published.image.unwrap();
        warm_handles.push((*id, image));
        let request = &requests[id];
        let reference = super::super::presentation::thumbnail(
            request.value.as_ref().unwrap(),
            &request.palette,
            &theme,
        )
        .scaled(40, 40);
        assert_eq!(
            capture(&context, images.registry.get(image).unwrap()),
            reference.pixels(),
            "recipe icon {id}"
        );
    }
    // Real list-widget GPU output uses the same two-stage fitting and text
    // placement as a reference icon, without storing its CPU pixels.
    let (_, icon) = warm_handles[0];
    let request = &requests[&warm_handles[0].0];
    let reference = super::super::presentation::thumbnail(
        request.value.as_ref().unwrap(),
        &request.palette,
        &theme,
    )
    .scaled(40, 40);
    let mut paint = Vec::new();
    for native in [true, false] {
        let mut item = TheListItem::new(TheId::empty());
        item.set_dim(TheDim::sized(230, 48), &mut ctx);
        item.set_text("Mesh recipe".into());
        item.set_sub_text("Building parts".into());
        if native {
            item.set_icon_handle(Some(icon));
        } else {
            item.set_icon(reference.clone());
        }
        let mut list = PaintList::default();
        item.paint(&mut WidgetPainter {
            list: &mut list,
            text: &mut ctx.text,
            images: &mut images,
            ui: &ctx.ui,
            theme: &mut TheDarkTheme::new(),
        })
        .unwrap();
        paint.push(list);
    }
    let mut renderer = PaintRenderer::new(context.clone()).unwrap();
    for density in [1f32, 1.5] {
        let size = ((230. * density) as u32, (48. * density) as u32);
        let mut pixels = Vec::new();
        for list in &paint {
            let output =
                GpuImage::render_target(&context, ImageDescriptor::rgba(size.0, size.1)).unwrap();
            renderer
                .render(
                    list,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    size,
                    density,
                    [0.; 4],
                )
                .unwrap();
            pixels.push(capture(&context, output));
        }
        let delta = pixels[0]
            .iter()
            .zip(&pixels[1])
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(delta <= 1, "list icon density={density}, delta={delta}");
    }
    // Filtering recreates item widgets but retains unchanged immutable requests.
    editor.recipe_browser_event(
        &TheEvent::ValueChanged(
            TheId::named("MZ Recipe Search"),
            TheValue::Text("wall".into()),
        ),
        &mut project,
        &mut ui,
        &mut ctx,
        &history,
    );
    let filtered = editor.recipe_browser.as_ref().unwrap().native_icons.clone();
    assert!(!filtered.is_empty() && filtered.len() < requests.len());
    for (id, request) in &filtered {
        assert!(Arc::ptr_eq(request, &requests[id]));
    }
    let removed = warm_handles
        .iter()
        .find(|(id, _)| !filtered.contains_key(id))
        .unwrap()
        .1;
    let held = images.registry.get(removed).unwrap();
    let weak = Arc::downgrade(&held);
    let (waiting, errors) = icons.prepare(Some(&filtered), &mut ui, &mut ctx, &mut images.registry);
    assert!(!waiting && errors.is_empty());
    assert_eq!(icons.entries.len(), filtered.len());
    assert!(images.registry.get(removed).is_none());
    assert!(weak.upgrade().is_some());
    drop(held);
    assert!(weak.upgrade().is_none());
    for (id, image) in &warm_handles {
        if filtered.contains_key(id) {
            assert_eq!(icons.entries[id].image, Some(*image));
        }
    }
    // Recovery detaches old-domain handles from every current widget before
    // rebuilding just one source per frame on the replacement device.
    let next_context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut next_images = ImageRegistry::new(next_context.clone());
    for _ in 0..filtered.len() {
        let (_, errors) = icons.prepare(Some(&filtered), &mut ui, &mut ctx, &mut next_images);
        assert!(errors.is_empty(), "{errors:?}");
    }
    let handles: Vec<_> = icons
        .entries
        .values()
        .map(|entry| entry.image.unwrap())
        .collect();
    for (id, entry) in &icons.entries {
        let request = &filtered[id];
        let reference = super::super::presentation::thumbnail(
            request.value.as_ref().unwrap(),
            &request.palette,
            &theme,
        )
        .scaled(40, 40);
        assert_eq!(
            capture(
                &next_context,
                next_images.get(entry.image.unwrap()).unwrap()
            ),
            reference.pixels()
        );
    }
    ui.clear_dialog();
    editor.recipe_browser_event(
        &TheEvent::Custom(TheId::empty(), TheValue::Empty),
        &mut project,
        &mut ui,
        &mut ctx,
        &history,
    );
    assert!(editor.recipe_browser.is_none());
    let (waiting, errors) = icons.prepare(None, &mut ui, &mut ctx, &mut next_images);
    assert!(!waiting && errors.is_empty());
    assert!(icons.entries.is_empty() && icons.renderer.is_none() && icons.fitter.is_none());
    for handle in handles {
        assert!(next_images.get(handle).is_none());
    }
}
