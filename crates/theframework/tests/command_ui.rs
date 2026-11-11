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
}
impl UiImageResources for RecordingImages {
    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        _changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        assert!(!source.pixels().is_empty());
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
