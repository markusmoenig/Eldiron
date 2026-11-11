use eldiron_presentation::screen::{MessageLayouts, ScreenImageCache, ScreenPainter};
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu};
use rusterix::{
    Assets, Choice, Entity, EntityAction, Map, MultipleChoice, Rect, Texture, Tile, Value,
    client::widget::messages::MessagesWidget,
};
use std::{sync::Arc, time::Duration};
use theframework::{
    paint::{PaintList, PaintOp},
    render_wgpu::PaintRenderer,
    text::TextSystem,
};
use theframework_core::TheTime;
fn fonts() -> TextSystem {
    let mut text = TextSystem::default();
    assert!(text.register_font_named(
        "Body",
        include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf").to_vec()
    ));
    text
}
fn map() -> Map {
    let mut map = Map::default();
    for id in [10, 20] {
        let mut entity = Entity::default();
        entity.id = id;
        entity
            .attributes
            .set("name", Value::Str(format!("Person {id}")));
        if id == 20 {
            entity.attributes.set("player", Value::Bool(true));
        }
        map.entities.push(entity);
    }
    map
}
fn paint(
    gpu: &Arc<GpuContext>,
    registry: &mut ImageRegistry,
    cache: &mut ScreenImageCache,
    text: &mut TextSystem,
    layout: &MessageLayouts,
    assets: &Assets,
    renderer: &mut PaintRenderer,
    target: &GpuImage,
    density: f32,
) -> PaintList {
    let mut list = PaintList::default();
    {
        let mut images = cache.begin_frame(registry).unwrap();
        ScreenPainter {
            list: &mut list,
            text,
            images: &mut images,
        }
        .messages(layout, assets)
        .unwrap();
    }
    list.validate().unwrap();
    renderer
        .render(
            &list,
            registry,
            &target.view,
            target.descriptor.format,
            (target.descriptor.width, target.descriptor.height),
            density,
            [0.03, 0.04, 0.05, 1.],
        )
        .unwrap();
    assert_eq!(gpu.transfers().readback_bytes, 0);
    list
}
fn capture(gpu: &GpuContext, target: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(gpu, target).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().into_pixels()
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn transcript_wraps_unicode_and_reuses_layouts_and_visible_portrait_sources() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut text = fonts();
    let mut layout = MessageLayouts::default();
    let mut widget = MessagesWidget::new();
    widget.toml_str="[ui]\nfont='Body'\nfont_size=18.0\nspacing=1.0\nmessage_spacing=6.0\nbackground_color='#141a20d0'\nportrait=true\nportrait_size=32.0\nportrait_gap=8.0".into();
    let mut assets = Assets::default();
    assert_eq!(widget.configure(&assets), "Body");
    widget.rect = Rect::new(12., 12., 340., 180.);
    let mut map = map();
    let tile = Tile::from_texture(Texture::new(vec![32, 128, 240, 255], 1, 1));
    let id = tile.id;
    assets.tiles.insert(id, tile);
    map.entities[0]
        .attributes
        .set("portrait_tile_id", Value::Str(id.to_string()));
    let messages = [
        "Café et caféine — a message with accents and a long ending that wraps.",
        "العربية — Eldiron remembers the complete message.",
        "A long path: forêt / 森林 / путешествие; combining e\u{301} stays together.",
        "Iron Sword\u{1f}12 gold",
    ]
    .map(|s| (Some(10), None, 20, s.into(), "message".into()));
    layout
        .prepare(
            &mut widget,
            &mut text,
            &assets,
            &map,
            &TheTime::new(),
            &messages,
            &[],
        )
        .unwrap();
    assert!(widget.font.is_none() && widget.buffer.pixels().is_empty());
    assert!(layout.visible_count() > 0 && layout.visible_count() <= widget.messages.len());
    for line in &widget.messages {
        if line.rect.height > 0. {
            assert!(
                line.rect.y >= widget.rect.y
                    && line.rect.y + line.rect.height <= widget.rect.y + widget.rect.height + 0.01
            );
        }
    }
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 340)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let first = paint(
        &gpu,
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &assets,
        &mut renderer,
        &output,
        1.5,
    );
    assert!(
        first
            .ops
            .iter()
            .any(|p| matches!(p, PaintOp::GlyphRun { .. }))
    );
    assert_eq!(
        images.len(),
        1,
        "only the visible portrait source is uploaded"
    );
    let shaped = layout.layouts_built();
    let uploads = gpu.transfers().uploaded_bytes;
    let glyphs = renderer.glyphs.rasterizations;
    layout
        .prepare(
            &mut widget,
            &mut text,
            &assets,
            &map,
            &TheTime::new(),
            &[],
            &[],
        )
        .unwrap();
    paint(
        &gpu,
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &assets,
        &mut renderer,
        &output,
        1.5,
    );
    assert_eq!(layout.layouts_built(), shaped);
    assert_eq!(gpu.transfers().uploaded_bytes, uploads);
    assert_eq!(renderer.glyphs.rasterizations, glyphs);
    // A registered font replacement invalidates retained paragraph metrics/runs.
    let mono = include_bytes!("../../theframework/embedded/fonts/SourceCodePro-Bold.ttf");
    assert!(text.register_font_named("Body", mono.to_vec()));
    layout
        .prepare(
            &mut widget,
            &mut text,
            &assets,
            &map,
            &TheTime::new(),
            &[],
            &[],
        )
        .unwrap();
    assert!(layout.layouts_built() > shaped);
    let changed = paint(
        &gpu,
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &assets,
        &mut renderer,
        &output,
        1.5,
    );
    assert!(
        changed
            .ops
            .iter()
            .any(|op| matches!(op,PaintOp::GlyphRun {run,..} if run.font.data.as_ref()==mono))
    );
    let pixels = capture(&gpu, output);
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-messages.png").unwrap(),
        600,
        340,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn choice_return_scroll_and_selection_use_the_painted_clipped_rows_once() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut text = fonts();
    let mut layout = MessageLayouts::default();
    let mut widget = MessagesWidget::new();
    widget.toml_str = "[ui]\nfont='Body'\nfont_size=16.0\nbackground_color='#151a20d0'".into();
    let assets = Assets::default();
    widget.configure(&assets);
    widget.rect = Rect::new(24., 24., 280., 120.);
    let map = map();
    let time = TheTime::new();
    let context = vec![(
        Some(10),
        None,
        20,
        "The guard offers a choice.".into(),
        "message".into(),
    )];
    let mut choices = MultipleChoice::new(0, 10, 20, i64::MAX, 10.);
    choices.choices = (0..6)
        .map(|i| {
            Choice::ScriptChoice(
                format!("Option {} — a longer conversation choice", i + 1),
                format!("choice_{i}"),
                10,
                20,
                i,
                i64::MAX,
                10.,
            )
        })
        .collect();
    let map_keys = layout
        .prepare(
            &mut widget,
            &mut text,
            &assets,
            &map,
            &time,
            &context,
            &[choices],
        )
        .unwrap()
        .unwrap();
    assert_eq!(map_keys.len(), 7);
    assert!(widget.view_state().choices_below_view);
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(360, 180)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    paint(
        &gpu,
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &assets,
        &mut renderer,
        &output,
        1.,
    );
    assert!(
        widget
            .touch_down(scenevm::prelude::Vec2::new(0, 0))
            .is_none()
    );
    let shaped = layout.layouts_built();
    assert!(matches!(
        widget.touch_down(scenevm::prelude::Vec2::new(300, 140)),
        Some(EntityAction::Off)
    ));
    assert_eq!(widget.view_state().scroll_offset, 0);
    layout
        .prepare(&mut widget, &mut text, &assets, &map, &time, &[], &[])
        .unwrap();
    assert!(!widget.view_state().choices_below_view);
    assert_eq!(layout.layouts_built(), shaped);
    paint(
        &gpu,
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &assets,
        &mut renderer,
        &output,
        1.,
    );
    let line = widget
        .messages
        .iter()
        .find(|l| l.choice.is_some() && l.rect.width > 0. && l.rect.height > 0.)
        .unwrap();
    let expected = line.choice.clone().unwrap();
    let point = scenevm::prelude::Vec2::new(
        (line.rect.x + 2.).ceil() as i32,
        (line.rect.y + line.rect.height * 0.5).floor() as i32,
    );
    assert!(
        matches!(widget.touch_down(point),Some(EntityAction::Choice(actual)) if actual==expected)
    );
    assert!(
        widget.touch_down(point).is_none(),
        "a visible choice must be delivered once"
    );
    layout
        .prepare(&mut widget, &mut text, &assets, &map, &time, &[], &[])
        .unwrap();
    assert!(!widget.has_active_choices());
    assert!(widget.font.is_none() && widget.buffer.pixels().is_empty());
    assert!(
        images.is_empty(),
        "transcript/choice text is glyph paint, not a raster overlay"
    );
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn oversized_paragraph_keeps_its_visible_tail_and_continues_without_raster_storage() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut text = fonts();
    let mut layout = MessageLayouts::default();
    let mut widget = MessagesWidget::new();
    widget.toml_str = "[ui]\nfont='Body'\nfont_size=16.0\npress_to_continue=true".into();
    let assets = Assets::default();
    widget.configure(&assets);
    widget.rect = Rect::new(8., 8., 160., 70.);
    let map = map();
    let time = TheTime::new();
    let messages=["A long message that wraps onto many lines and remains visible even when its block is much taller than this transcript widget.","Next page."].map(|s| (Some(10),None,20,s.into(),"message".into()));
    layout
        .prepare(&mut widget, &mut text, &assets, &map, &time, &messages, &[])
        .unwrap();
    assert!(widget.view_state().paused);
    assert_eq!(widget.messages.len(), 1);
    assert_eq!(layout.visible_count(), 1);
    let bounds = widget.messages[0].rect;
    assert!(bounds.height > 0. && bounds.y >= widget.rect.y);
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(200, 100)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    paint(
        &gpu,
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &assets,
        &mut renderer,
        &output,
        1.,
    );
    assert!(renderer.frame_stats().glyph_quads > 20);
    assert!(matches!(
        widget.touch_down(scenevm::prelude::Vec2::new(20, 20)),
        Some(EntityAction::Off)
    ));
    layout
        .prepare(&mut widget, &mut text, &assets, &map, &time, &[], &[])
        .unwrap();
    assert_eq!(widget.messages.len(), 2);
    assert!(!widget.view_state().paused);
    assert!(widget.font.is_none() && widget.buffer.pixels().is_empty());
    let pixels = capture(&gpu, output);
    assert!(
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 120 && p[1] > 120 && p[2] > 120)
            .count()
            > 50,
        "visible tail and continue prompt must actually render"
    );
}
