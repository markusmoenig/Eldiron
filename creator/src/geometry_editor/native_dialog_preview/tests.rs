use super::*;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket};
use theframework::{
    paint::*,
    render_wgpu::{PaintRenderer, UiImages},
    theui::paint::UiImageResources,
};

fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut capture = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(renderkit::wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    capture
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}

#[test]
#[ignore = "native GPU; modal preview fitting, retirement and recovery"]
fn native_dialog_cards_preserve_reference_pixels_and_owned_lifetimes() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(context.clone());
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(143, 83, 1.);
    ctx.ui.command_paint = true;
    let name = "Modal mesh preview contract";
    let mut view = TheRenderView::new(TheId::named(name));
    view.set_dim(TheDim::sized(143, 83), &mut ctx);
    view.render_buffer_mut().resize(143, 83);
    ui.canvas.set_widget(view);
    pending(&mut ui, name);
    assert!(!ui.get_render_view(name).unwrap().has_cpu_display_storage());
    let value = Value::Mesh(
        Document::load(denrim_materials::geometry::SWORD_EXAMPLE)
            .unwrap()
            .evaluate()
            .unwrap()
            .mesh,
    );
    let request = Arc::new(Request {
        value: Some(value),
        palette: Project::default().palette,
        max_side: 200,
    });
    let mut preview = Preview::default();
    preview
        .prepare(
            Some(&request),
            name,
            &mut ui,
            &mut ctx,
            &mut images.registry,
        )
        .unwrap();
    let original = preview.image.unwrap();
    let retained = images.registry.get(original).unwrap();
    let weak = Arc::downgrade(&retained);
    let allocations = context.image_allocation_usage().allocations;
    preview
        .prepare(
            Some(&request),
            name,
            &mut ui,
            &mut ctx,
            &mut images.registry,
        )
        .unwrap();
    assert_eq!(preview.image, Some(original));
    assert_eq!(context.image_allocation_usage().allocations, allocations);
    assert_eq!(context.transfers().readback_bytes, 0);
    let reference = super::super::presentation::thumbnail(
        request.value.as_ref().unwrap(),
        &request.palette,
        &TheDarkTheme::new(),
    );
    let background = *TheDarkTheme::new().color(MaterialControl);
    assert!(
        reference
            .pixels()
            .chunks_exact(4)
            .any(|pixel| pixel != background)
    );
    assert_eq!(
        capture(&context, preview.card.as_ref().unwrap().clone()),
        reference.pixels()
    );
    let original_card = preview.card.as_ref().unwrap().clone();
    let mut renderer = PaintRenderer::new(context.clone()).unwrap();
    for (width, height) in [(143, 83), (300, 240), (57, 40), (320, 256)] {
        ui.get_render_view(name)
            .unwrap()
            .set_dim(TheDim::sized(width, height), &mut ctx);
        preview
            .prepare(
                Some(&request),
                name,
                &mut ui,
                &mut ctx,
                &mut images.registry,
            )
            .unwrap();
        assert!(
            Arc::ptr_eq(preview.card.as_ref().unwrap(), &original_card),
            "resize reuses original card"
        );
        let mut composed = TheRGBABuffer::new(TheDim::sized(width, height));
        composed.fill([30, 32, 35, 255]);
        let side = width.min(height).min(request.max_side);
        composed.copy_into(
            (width - side) / 2,
            (height - side) / 2,
            &reference.scaled(side, side),
        );
        assert_eq!(
            capture(
                &context,
                images.registry.get(preview.image.unwrap()).unwrap()
            ),
            composed.pixels(),
            "fitted bytes {width}x{height}"
        );
        let expected_handle = images
            .image("modal CPU reference", &composed, false)
            .unwrap();
        let mut theme = TheDarkTheme::new();
        let mut native = PaintList::default();
        preview
            .source
            .as_ref()
            .unwrap()
            .paint(
                &mut WidgetPainter {
                    list: &mut native,
                    text: &mut ctx.text,
                    images: &mut images,
                    ui: &ctx.ui,
                    theme: &mut theme,
                },
                [width as f32, height as f32],
            )
            .unwrap();
        let mut expected = PaintList::default();
        expected.image(
            expected_handle,
            Rect::new(0., 0., width as f32, height as f32),
            Sampling::Nearest,
        );
        for density in [1f32, 1.5] {
            let size = (
                (width as f32 * density).round() as u32,
                (height as f32 * density).round() as u32,
            );
            let mut results = Vec::new();
            for list in [&native, &expected] {
                let output =
                    GpuImage::render_target(&context, ImageDescriptor::rgba(size.0, size.1))
                        .unwrap();
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
                results.push(capture(&context, output));
            }
            let delta = results[0]
                .iter()
                .zip(&results[1])
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(
                delta <= 1,
                "modal fitting {width}x{height} density={density}: delta={delta}"
            );
        }
    }
    let replacement = Arc::new(Request {
        value: None,
        palette: request.palette.clone(),
        max_side: 256,
    });
    preview
        .prepare(
            Some(&replacement),
            name,
            &mut ui,
            &mut ctx,
            &mut images.registry,
        )
        .unwrap();
    assert!(images.registry.get(original).is_none());
    assert!(
        weak.upgrade().is_some(),
        "prepared consumer pins original image"
    );
    assert!(preview.image.is_none());
    drop(retained);
    assert!(weak.upgrade().is_none());
    let next_context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut next_images = UiImages::new(next_context.clone());
    preview
        .prepare(
            Some(&request),
            name,
            &mut ui,
            &mut ctx,
            &mut next_images.registry,
        )
        .unwrap();
    let rebuilt = preview.image.unwrap();
    assert_eq!(
        capture(&next_context, preview.card.as_ref().unwrap().clone()),
        reference.pixels()
    );
    preview
        .prepare(None, name, &mut ui, &mut ctx, &mut next_images.registry)
        .unwrap();
    assert!(next_images.registry.get(rebuilt).is_none());
    assert!(preview.request.is_none() && preview.source.is_none() && preview.renderer.is_none());
    assert!(!ui.get_render_view(name).unwrap().has_cpu_display_storage());
}

#[test]
fn native_dialog_controllers_queue_values_without_mutating_assets_or_display_pixels() {
    let mut project = Project::default();
    let document = Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap();
    let id = Uuid::new_v4();
    project
        .procedural_appearances
        .insert(id, document.save().unwrap());
    let before = project.procedural_appearances.clone();
    let mut editor = GeometryEditor::default();
    let history = CreatorHistory::default();
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(1000, 760, 1.);
    ui.init(&mut ctx);
    ctx.ui.command_paint = true;
    assert!(editor.recipe_browser_event(
        &TheEvent::StateChanged(TheId::named("MZ New Box"), TheWidgetState::Clicked),
        &mut project,
        &mut ui,
        &mut ctx,
        &history
    ));
    assert!(
        editor
            .recipe_browser
            .as_ref()
            .unwrap()
            .native_preview
            .as_ref()
            .unwrap()
            .value
            .is_some()
    );
    assert!(
        !ui.get_render_view("MZ Recipe Preview")
            .unwrap()
            .has_cpu_display_storage()
    );
    assert_eq!(
        editor.recipe_browser.as_ref().unwrap().native_icons.len(),
        super::super::recipe_browser::bundled().len()
    );
    ui.clear_dialog();
    editor.recipe_browser_event(
        &TheEvent::Custom(TheId::empty(), TheValue::Empty),
        &mut project,
        &mut ui,
        &mut ctx,
        &history,
    );
    assert!(editor.recipe_browser.is_none());
    editor.selected = Some(id);
    assert!(editor.control_publisher_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
        &mut project,
        &mut ui,
        &mut ctx,
        &history
    ));
    assert!(
        editor
            .control_publisher
            .as_ref()
            .unwrap()
            .native_preview
            .as_ref()
            .unwrap()
            .value
            .is_some()
    );
    assert!(
        !ui.get_render_view("MZ Control Preview")
            .unwrap()
            .has_cpu_display_storage()
    );
    assert_eq!(project.procedural_appearances, before);
}
