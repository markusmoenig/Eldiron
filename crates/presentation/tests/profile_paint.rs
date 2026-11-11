use eldiron_presentation::screen::{
    ContentRevision, ContentSources, ProfileLayout, ScreenImageCache, ScreenPainter,
};
use renderkit::{
    GpuContext, GpuImage, ImageAsset, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu,
};
use rusterix::{
    Assets, Entity, Rect, Texture, Tile, Value,
    client::widget::{avatar::AvatarWidget, profile::ProfileWidget},
};
use std::{sync::Arc, time::Duration};
use theframework::{
    paint::{PaintList, PaintOp, Rect as Uv, Sampling, Transform},
    render_wgpu::PaintRenderer,
    text::TextSystem,
};
fn capture(gpu: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(gpu, image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().into_pixels()
}
fn mask() -> Texture {
    let mut data = vec![0; 4 * 4 * 4];
    for y in 1..3 {
        for x in 1..3 {
            data[(y * 4 + x) * 4..(y * 4 + x + 1) * 4].copy_from_slice(&[0, 80, 255, 255]);
        }
    }
    Texture::new(data, 4, 4)
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_outline_obeys_logical_density_transform_crop_and_clipping() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    // Right half is opaque green and must never affect the left UV crop.
    let source = mask();
    let mut pixels = vec![0; 8 * 4 * 4];
    for y in 0..4 {
        pixels[y * 32..y * 32 + 16].copy_from_slice(&source.data[y * 16..y * 16 + 16]);
        for x in 4..8 {
            pixels[(y * 8 + x) * 4..(y * 8 + x + 1) * 4].copy_from_slice(&[0, 255, 0, 255]);
        }
    }
    let image = registry
        .upload_linear(&ImageAsset::rgba(8, 4, pixels).unwrap())
        .unwrap();
    let bounds = Uv::new(16., 16., 32., 32.);
    let uv = Uv::new(0., 0., 0.5, 1.);
    let mut list = PaintList::default();
    list.ops
        .push(PaintOp::PushTransform(Transform::translate(8., 4.)));
    list.ops
        .push(PaintOp::PushClip(Uv::new(12., 12., 34., 40.)));
    list.ops.push(PaintOp::ImageOutline {
        image,
        rect: bounds,
        uv,
        sampling: Sampling::Nearest,
        width: 3.,
        color: [1., 0., 0., 1.],
    });
    list.ops.push(PaintOp::Image {
        image,
        rect: bounds,
        uv,
        sampling: Sampling::Nearest,
        tint: [1.; 4],
    });
    list.ops.push(PaintOp::PopClip);
    list.ops.push(PaintOp::PopTransform);
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(128, 128)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &registry,
            &output.view,
            output.descriptor.format,
            (128, 128),
            1.5,
            [1.; 4],
        )
        .unwrap();
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let pixels = capture(&gpu, output);
    let pixel = |x: f32, y: f32| {
        let i = ((y * 1.5) as usize * 128 + (x * 1.5) as usize) * 4;
        &pixels[i..i + 4]
    };
    assert_eq!(pixel(40., 36.), &[0, 80, 255, 255]);
    assert_eq!(pixel(30., 36.), &[255, 0, 0, 255]);
    assert_eq!(pixel(27., 36.), &[255, 255, 255, 255]);
    assert_eq!(
        pixel(53., 36.),
        &[255, 255, 255, 255],
        "UV crop must not see the adjacent opaque artwork"
    );
    assert_eq!(
        pixel(55., 36.),
        &[255, 255, 255, 255],
        "widget clip applies after the paint transform"
    );
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn profile_reuses_sources_shaping_and_swash_and_paints_selection_stats_and_identity() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut sources = ContentSources::default();
    let mut text = TextSystem::default();
    text.register_font_named(
        "Body",
        include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf").to_vec(),
    );
    let mut assets = Assets::default();
    let tile = Tile::from_texture(mask());
    let mut entity = Entity::default();
    entity.attributes.set("profile_tile_id", Value::Id(tile.id));
    entity
        .attributes
        .set("display_name", Value::Str("Éldiron العربية 東京".into()));
    entity
        .attributes
        .set("class_name", Value::Str("rune_mage".into()));
    entity.attributes.set("HP", Value::Float(25.));
    entity.attributes.set("MAX_HP", Value::Float(100.));
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let mut widget = ProfileWidget::new();
    widget.rect = Rect::new(12., 12., 360., 110.);
    widget.toml_str = "[ui]\nimage_size=70.0\nname_font_size=20.0\nclass_font_size=16.0\ntext_gap=4.0\nborder_color='#ff0000'\nborder_size=3\nselected_frame=true\nselected_frame_color='#00ff00'\nselected_frame_size=2\nselected_frame_padding=4\nstats_top_gap=5.0\n[[ui.stats]]\nstat='HP'\nmax_stat='MAX_HP'\nheight=12.0\nfill_color='#ff0000'\nbackground_color='#202020'\nborder_size=0".into();
    widget.init();
    let content = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    let mut layout = ProfileLayout::default();
    layout.prepare(&widget, &entity, &mut text, "Body", Some(&content));
    let view = layout.view().unwrap();
    assert_eq!(view.bars.len(), 1);
    assert_eq!(view.bars[0].ratio, 0.25);
    let bar = view.bars[0].bounds;
    assert!(
        bar.y >= view.text.unwrap().y + view.text.unwrap().height,
        "bars reserve the actual shaped identity height"
    );
    assert!(widget.buffer.pixels().is_empty());
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 210)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let paint = |registry: &mut ImageRegistry,
                 images: &mut ScreenImageCache,
                 text: &mut TextSystem,
                 layout: &ProfileLayout,
                 renderer: &mut PaintRenderer| {
        let mut list = PaintList::default();
        {
            let mut frame = images.begin_frame(registry).unwrap();
            ScreenPainter {
                list: &mut list,
                text,
                images: &mut frame,
            }
            .profile(layout, Some(&content))
            .unwrap();
        }
        list.validate().unwrap();
        assert!(
            list.ops
                .iter()
                .any(|op| matches!(op, PaintOp::GlyphRun { .. }))
        );
        assert_eq!(
            list.ops
                .iter()
                .filter(|op| matches!(op, PaintOp::ImageOutline { .. }))
                .count(),
            1
        );
        renderer
            .render(
                &list,
                registry,
                &target.view,
                target.descriptor.format,
                (600, 210),
                1.5,
                [0.025, 0.035, 0.05, 1.],
            )
            .unwrap();
    };
    paint(
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &mut renderer,
    );
    let (uploads, rasters, shapes, builds) = (
        gpu.transfers().uploaded_bytes,
        renderer.glyphs.rasterizations,
        layout.layouts_built(),
        sources.builds(),
    );
    entity.attributes.set("HP", Value::Float(50.));
    let repeated = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&content, &repeated));
    layout.prepare(&widget, &entity, &mut text, "Body", Some(&content));
    assert_eq!(layout.view().unwrap().bars[0].ratio, 0.5);
    assert_eq!(layout.layouts_built(), shapes);
    assert_eq!(sources.builds(), builds);
    paint(
        &mut registry,
        &mut images,
        &mut text,
        &layout,
        &mut renderer,
    );
    assert_eq!(gpu.transfers().uploaded_bytes, uploads);
    assert_eq!(renderer.glyphs.rasterizations, rasters);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert_eq!(
        images.len(),
        1,
        "the portrait source is the only raster upload"
    );
    let pixels = capture(&gpu, target);
    let pixel = |x: f32, y: f32| {
        let i = ((y * 1.5) as usize * 600 + (x * 1.5) as usize) * 4;
        &pixels[i..i + 4]
    };
    assert_eq!(pixel(12.5, 20.), &[0, 255, 0, 255]);
    assert_eq!(pixel(bar.x + 3., bar.y + 3.), &[255, 0, 0, 255]);
    assert_eq!(
        pixel(bar.x + bar.width * 0.8, bar.y + 3.),
        &[32, 32, 32, 255]
    );
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-profile.png").unwrap(),
        600,
        210,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    let previous_text_h = layout.view().unwrap().text.unwrap().height;
    text.register_font_named(
        "Body",
        include_bytes!("../../theframework/embedded/fonts/SourceCodePro-Bold.ttf").to_vec(),
    );
    layout.prepare(&widget, &entity, &mut text, "Body", Some(&content));
    assert_eq!(layout.layouts_built(), shapes + 2);
    assert_ne!(layout.view().unwrap().text.unwrap().height, previous_text_h);
}
#[test]
fn damage_flash_changes_visuals_without_rebuilding_avatar_source() {
    let assets = Assets::default();
    let mut entity = Entity::default();
    entity.attributes.set("HP", Value::Float(100.));
    let mut widget = AvatarWidget::new();
    widget.damage_stat = "HP".into();
    widget.border_color = [0, 0, 255, 255];
    widget.damage_border_color = [255, 0, 0, 255];
    assert_eq!(widget.prepare_outline(&entity, &assets).0, [0, 0, 255, 255]);
    let key = widget.content_recipe(&entity).fingerprint();
    entity.attributes.set("HP", Value::Float(70.));
    assert_eq!(widget.prepare_outline(&entity, &assets).0, [255, 0, 0, 255]);
    assert_eq!(key, widget.content_recipe(&entity).fingerprint());
    entity.id += 1;
    entity.attributes.set("HP", Value::Float(5.));
    assert_eq!(
        widget.prepare_outline(&entity, &assets).0,
        [0, 0, 255, 255],
        "switching party members starts a fresh damage baseline"
    );
    widget.damage_flash_seconds = 0.;
    assert_eq!(widget.prepare_outline(&entity, &assets).0, [0, 0, 255, 255]);
    assert!(widget.buffer.pixels().is_empty());
}
