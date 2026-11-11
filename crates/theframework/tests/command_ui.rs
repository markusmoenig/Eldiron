#![cfg(all(feature = "ui", feature = "text"))]
use std::collections::HashMap;
use theframework::{
    paint::{ImageHandle, PaintOp},
    prelude::*,
    theui::paint::{UiImageResources, UiPaintError},
};
#[derive(Default)]
struct RecordingImages {
    handles: HashMap<String, ImageHandle>,
    uploads: usize,
}
impl UiImageResources for RecordingImages {
    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        assert!(!source.pixels().is_empty());
        if changed || !self.handles.contains_key(key) {
            self.uploads += 1;
        }
        let slot = self.handles.len() as u32;
        Ok(*self.handles.entry(key.into()).or_insert(ImageHandle {
            owner: 99,
            slot,
            generation: 1,
        }))
    }
}
#[test]
fn ui_emits_text_and_image_commands_without_a_renderer_or_window() {
    let mut ctx = TheContext::new(360, 120, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    let mut icon = TheRGBABuffer::new(TheDim::sized(16, 16));
    icon.fill([255, 255, 255, 255]);
    ctx.ui.add_icon("headless".into(), icon);
    let mut layout = TheHLayout::new(TheId::named("headless-layout"));
    let mut label = TheText::new(TheId::named("label"));
    label.set_text("Pure UI command contract".into());
    let mut button = TheToolbarButton::new(TheId::named("button"));
    button.set_icon_name("headless".into());
    layout.widgets().push(Box::new(label));
    layout.widgets().push(Box::new(button));
    ui.canvas.set_layout(layout);
    let mut resources = RecordingImages::default();
    let list = ui.paint_commands(&mut ctx, &mut resources).unwrap();
    list.validate().unwrap();
    assert!(
        list.ops
            .iter()
            .any(|v| matches!(v, PaintOp::GlyphRun { .. }))
    );
    assert!(list.ops.iter().any(|v| matches!(v, PaintOp::Image { .. })));
    assert_eq!(resources.handles.len(), 1);
    assert!(ui.canvas.buffer.pixels().is_empty());
}

fn paint_widget(
    widget: &mut dyn TheWidget,
    ctx: &mut TheContext,
    resources: &mut RecordingImages,
) -> theframework::paint::PaintList {
    use theframework::theui::paint::WidgetPainter;
    let mut style = TheClassicStyle::new();
    widget.prepare_paint(ctx, style.theme().as_ref());
    let mut list = theframework::paint::PaintList::default();
    let mut painter = WidgetPainter {
        list: &mut list,
        text: &mut ctx.text,
        images: resources,
        ui: &ctx.ui,
        theme: style.theme().as_mut(),
    };
    painter.widget(widget).unwrap();
    list.validate().unwrap();
    list
}

#[test]
fn icon_collections_keep_content_uploads_separate_from_selection_and_scroll() {
    let mut ctx = TheContext::new(320, 180, 1.);
    let mut resources = RecordingImages::default();
    let mut icon = TheRGBABuffer::new(TheDim::sized(12, 8));
    icon.fill([255, 128, 0, 255]);
    let mut grid = TheIconGridView::new(TheId::named("grid"));
    grid.set_items(
        (0..1000)
            .map(|index| {
                let mut item = TheIconGridItem::new(format!("Item {index}"));
                item.icon = Some(icon.clone());
                item
            })
            .collect(),
    );
    grid.set_dim(TheDim::new(30, 20, 320, 150), &mut ctx);
    let first = paint_widget(&mut grid, &mut ctx, &mut resources);
    assert!(
        first
            .ops
            .iter()
            .filter(|op| matches!(op, PaintOp::Image { .. }))
            .count()
            <= 6
    );
    let uploads = resources.uploads;
    grid.on_event(&TheEvent::MouseDown(Vec2::new(10, 10)), &mut ctx);
    assert_eq!(grid.selected(), Some(0));
    grid.on_event(&TheEvent::Hover(Vec2::new(110, 10)), &mut ctx);
    paint_widget(&mut grid, &mut ctx, &mut resources);
    assert_eq!(resources.uploads, uploads);
    grid.set_item_icon(0, Some(icon.clone()));
    paint_widget(&mut grid, &mut ctx, &mut resources);
    assert_eq!(resources.uploads, uploads + 1);
    grid.on_event(&TheEvent::MouseWheel(Vec2::new(0, -104)), &mut ctx);
    paint_widget(&mut grid, &mut ctx, &mut resources);
    grid.on_event(&TheEvent::MouseDown(Vec2::new(10, 10)), &mut ctx);
    assert_eq!(grid.selected(), Some(3));
    assert!(grid.visible_items().start > 0);

    let mut row = TheScrollableIconRow::new(TheId::named("row"));
    row.set_items(
        (0..1000)
            .map(|index| {
                let mut item = TheScrollableIconRowItem::new(format!("Row {index}"));
                item.icon = Some(icon.clone());
                item
            })
            .collect(),
    );
    row.set_dim(TheDim::new(10, 20, 200, 60), &mut ctx);
    let first = paint_widget(&mut row, &mut ctx, &mut resources);
    assert!(
        first
            .ops
            .iter()
            .filter(|op| matches!(op, PaintOp::Image { .. }))
            .count()
            <= 4
    );
    let uploads = resources.uploads;
    row.on_event(&TheEvent::MouseDown(Vec2::new(85, 15)), &mut ctx);
    row.on_event(&TheEvent::MouseUp(Vec2::new(85, 15)), &mut ctx);
    assert_eq!(row.selected(), 1);
    paint_widget(&mut row, &mut ctx, &mut resources);
    assert_eq!(resources.uploads, uploads);
    row.on_event(&TheEvent::MouseWheel(Vec2::new(-80, 0)), &mut ctx);
    paint_widget(&mut row, &mut ctx, &mut resources);
    row.on_event(&TheEvent::MouseDown(Vec2::new(10, 15)), &mut ctx);
    row.on_event(&TheEvent::MouseUp(Vec2::new(10, 15)), &mut ctx);
    assert_eq!(row.selected(), 1);
}

#[test]
fn unassigned_widgets_emit_no_geometry_and_time_input_matches_the_track() {
    let mut ctx = TheContext::new(400, 100, 1.);
    let mut resources = RecordingImages::default();
    let mut switch = TheSwitchbar::new(TheId::named("switch"));
    switch.set_text("Section".into());
    assert!(
        paint_widget(&mut switch, &mut ctx, &mut resources)
            .ops
            .is_empty()
    );
    switch.set_dim(TheDim::new(10, 20, 300, 24), &mut ctx);
    let list = paint_widget(&mut switch, &mut ctx, &mut resources);
    assert!(
        list.ops
            .iter()
            .any(|op| matches!(op, PaintOp::Stroke { .. }))
    );
    assert!(
        list.ops
            .iter()
            .any(|op| matches!(op, PaintOp::GlyphRun { .. }))
    );
    let mut time = TheTimeSlider::new(TheId::named("time"));
    time.set_dim(TheDim::new(10, 50, 400, 24), &mut ctx);
    time.on_event(&TheEvent::MouseDown(Vec2::new(100, 12)), &mut ctx);
    let value = time.value();
    let list = paint_widget(&mut time, &mut ctx, &mut resources);
    assert!(
        list.ops
            .iter()
            .any(|op| matches!(op, PaintOp::GlyphRun { .. }))
    );
    assert_eq!(time.value(), value);
    if let TheValue::Time(t) = value {
        assert!(t.to_widget_offset(360).abs_diff(100) < 3);
    } else {
        panic!("time value lost");
    }
}

#[test]
fn replacing_a_registered_font_invalidates_idle_ui_and_keeps_invalid_edits_transactional() {
    let mut ctx = TheContext::new(360, 100, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    let mut row = TheHLayout::new(TheId::named("font-row"));
    let mut label = TheText::new(TheId::named("font-label"));
    label.set_text("Font replacement changes these glyphs".into());
    row.widgets().push(Box::new(label));
    ui.canvas.set_layout(row);
    let mut images = RecordingImages::default();
    ui.paint_commands(&mut ctx, &mut images).unwrap();
    assert!(
        !ui.needs_paint(&ctx),
        "a completed text-only frame can sleep"
    );
    let mono = include_bytes!("../embedded/fonts/SourceCodePro-Bold.ttf");
    assert!(ctx.text.register_font_named("Roboto", mono.to_vec()));
    assert!(
        ui.needs_paint(&ctx),
        "a font revision must wake idle UI even when text is unchanged"
    );
    let changed = ui.paint_commands(&mut ctx, &mut images).unwrap();
    assert!(
        changed
            .ops
            .iter()
            .any(|op| matches!(op,PaintOp::GlyphRun {run,..} if run.font.data.as_ref()==mono))
    );
    assert!(!ui.needs_paint(&ctx));
    let revision = ctx.text.font_revision();
    assert!(!ctx.text.register_font_named("Roboto", vec![0, 1, 2, 3]));
    assert_eq!(ctx.text.font_revision(), revision);
    assert!(
        !ui.needs_paint(&ctx),
        "a rejected font cannot erase or invalidate the prior face"
    );
    assert!(ui.canvas.buffer.pixels().is_empty());
    assert_eq!(images.uploads, 0);
}

#[test]
fn published_view_commands_use_logical_viewport_without_source_uploads() {
    use std::sync::Arc;
    use theframework::paint::{PaintList, Rect, Transform};
    let mut ctx = TheContext::new(240, 160, 2.);
    let mut resources = RecordingImages::default();
    let mut view = TheRenderView::new(TheId::named("commands"));
    view.set_dim(TheDim::new(31, 27, 140, 80), &mut ctx);
    let mut local = PaintList::default();
    local.rect(Rect::new(-20., -20., 200., 120.), 0., [1., 0., 0., 1.]);
    local
        .ops
        .push(PaintOp::PushClip(Rect::new(10., 10., 40., 20.)));
    local
        .ops
        .push(PaintOp::PushTransform(Transform::translate(10., 10.)));
    local.rect(Rect::new(0., 0., 40., 20.), 4., [0., 1., 0., 1.]);
    local.ops.push(PaintOp::PopTransform);
    local.ops.push(PaintOp::PopClip);
    view.set_paint_list(Some(Arc::new(local))).unwrap();
    let list = paint_widget(&mut view, &mut ctx, &mut resources);
    assert_eq!(resources.uploads, 0);
    assert!(matches!(list.ops[0], PaintOp::PushClip(r) if r == Rect::new(31., 27., 140., 80.)));
    assert!(
        matches!(list.ops[1], PaintOp::PushTransform(t) if t == Transform::translate(31., 27.))
    );
    assert!(matches!(list.ops.last(), Some(PaintOp::PopClip)));
    // Resize/DPI do not bake pixel dimensions into retained commands.
    view.set_dim(TheDim::new(47, 39, 100, 60), &mut ctx);
    ctx.scale_factor = 1.5;
    let resized = paint_widget(&mut view, &mut ctx, &mut resources);
    assert!(matches!(resized.ops[0], PaintOp::PushClip(r) if r == Rect::new(47., 39., 100., 60.)));
    assert_eq!(resources.uploads, 0);
    view.set_paint_list(None).unwrap();
    assert!(
        paint_widget(&mut view, &mut ctx, &mut resources)
            .ops
            .is_empty()
    );
    assert_eq!(resources.uploads, 0);
}
