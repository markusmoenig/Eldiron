use eldiron_presentation::screen::{ScreenImageCache, ScreenImages, ScreenPainter};
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu};
use rustc_hash::FxHashMap;
use rusterix::{
    Assets, Currencies, Entity, Map, Rect, Value,
    client::widget::{
        TextInputWidget,
        choice::{ChoiceInteraction, ChoiceWidget, ChoiceWidgetKind},
        stat::StatWidget,
        text::TextWidget,
    },
};
use theframework::{paint::*, render_wgpu::PaintRenderer, text::TextSystem};
use theframework_core::TheTime;

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn screen_controls_paint_without_cpu_targets_and_keep_controller_geometry() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = ScreenImageCache::default();
    let mut text = TextSystem::default();
    let font = include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf");
    assert!(text.register_font_named("Project Font", font.to_vec()));
    assert_eq!(
        &*text.layout("chosen", "Project Font", 18., None).runs[0]
            .font
            .data,
        font
    );
    let assets = Assets::default();
    let map = Map::default();
    let mut state = FxHashMap::default();
    state.insert("name".into(), "Éldiron — العربية".into());
    let mut label = TextWidget::new();
    label.rect = Rect::new(16., 12., 500., 55.);
    label.text = "Welcome {UI.name}".into();
    label.font_size = 22.;
    let input = TextInputWidget {
        name: "Name".into(),
        id: 1,
        rect: Rect::new(16., 70., 260., 36.),
        binding: "start.name".into(),
        text: "Player é".into(),
        font: "Project Font".into(),
        font_size: 18.,
        color: [230, 230, 230, 255],
        background_color: [15, 20, 25, 255],
        border_color: [130, 140, 150, 255],
        border_size: 1,
    };
    let mut tabs = ChoiceWidget::from_ui(
        "Categories".into(),
        2,
        Rect::new(16., 120., 360., 32.),
        ChoiceWidgetKind::TabBar,
        &"binding='category'\ntabs=['All', 'Equipment', 'Magic']"
            .parse()
            .unwrap(),
    )
    .unwrap();
    let click = tabs.tab_rect(1);
    assert_eq!(
        tabs.interact(scenevm::prelude::Vec2::new(click.x + 3., click.y + 5.)),
        ChoiceInteraction::Selected("equipment".into())
    );
    let mut stat = StatWidget::new();
    stat.rect = Rect::new(16., 170., 200., 20.);
    stat.toml_str = "[ui]\nstat='HP'\nmax_stat='MAX_HP'\nfill_color='#ff0000'\nbackground_color='#101010'\nborder_size=1".into();
    stat.init();
    let mut entity = Entity::default();
    entity.attributes.set("HP", Value::Float(25.));
    entity.attributes.set("MAX_HP", Value::Float(100.));
    let mut list = PaintList::default();
    {
        let mut images = cache.begin_frame(&mut registry).unwrap();
        let mut painter = ScreenPainter {
            list: &mut list,
            text: &mut text,
            images: &mut images,
        };
        painter.text_widget(
            &label,
            &map,
            &assets,
            &Currencies::default(),
            &TheTime::default(),
            &state,
        );
        painter.text_input(&input, true);
        painter.choice(&tabs, [0., 0.]);
        painter.stat(&stat, &assets, Some(&entity), 0).unwrap();
    }
    list.validate().unwrap();
    assert!(label.buffer.pixels().is_empty() && stat.buffer.pixels().is_empty());
    assert!(
        cache.is_empty(),
        "chrome and text must not become source-image uploads"
    );
    assert!(
        list.ops
            .iter()
            .any(|op| matches!(op, PaintOp::GlyphRun { .. }))
    );
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(640, 240)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &registry,
            &output.view,
            output.descriptor.format,
            (640, 240),
            1.,
            [0.03, 0.04, 0.05, 1.],
        )
        .unwrap();
    assert!(renderer.frame_stats().glyph_quads > 20);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = capture.try_finish(&gpu).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &capture.pixels()[(y * 640 + x) * 4..(y * 640 + x + 1) * 4];
    assert_eq!(pixel(30, 180), [255, 0, 0, 255]);
    assert!(
        pixel(100, 180)[0] < 30,
        "stat must fill exactly one quarter"
    );
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-screen-controls.png").unwrap(),
        640,
        240,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(capture.pixels())
        .unwrap();
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn screen_source_cache_keeps_current_frame_revisions_and_evicts_unused_assets() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = ScreenImageCache::new(8);
    let (a, b);
    {
        let mut frame = cache.begin_frame(&mut registry).unwrap();
        a = frame.rgba("tile", 1, 1, &[255, 0, 0, 255]).unwrap();
        assert_eq!(a, frame.rgba("tile", 1, 1, &[255, 0, 0, 255]).unwrap());
        b = frame.rgba("tile", 1, 1, &[0, 255, 0, 255]).unwrap();
        assert!(frame.rgba("bad", 2, 1, &[0; 4]).is_err());
    }
    assert_eq!(cache.resident_bytes(), 16);
    assert!(registry.get(a).is_some() && registry.get(b).is_some());
    let uploads = gpu.transfers().uploaded_bytes;
    {
        let mut frame = cache.begin_frame(&mut registry).unwrap();
        assert_eq!(b, frame.rgba("tile", 1, 1, &[0, 255, 0, 255]).unwrap());
    }
    assert_eq!(cache.resident_bytes(), 8);
    assert!(registry.get(a).is_none());
    assert_eq!(gpu.transfers().uploaded_bytes, uploads);
    {
        let mut frame = cache.begin_frame(&mut registry).unwrap();
        frame.rgba("other", 1, 1, &[0, 0, 255, 255]).unwrap();
    }
    assert_eq!(cache.len(), 1);
    assert!(registry.get(b).is_none());
    let mut foreign = ImageRegistry::new(gpu.clone());
    assert!(cache.begin_frame(&mut foreign).is_err());
    cache.clear(&mut registry).unwrap();
    assert_eq!(cache.resident_bytes(), 0);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn artwork_keeps_animation_spatial_pages_and_nine_slice_edges_independent() {
    use rusterix::{
        PixelSource, Texture, Tile, TileUvPages,
        client::widget::{ButtonVisualState, artwork::ScreenArtwork},
    };
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = ScreenImageCache::default();
    let mut text = TextSystem::default();
    let mut assets = Assets::default();
    let colors = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];
    let mut tile = Tile::from_textures(
        colors
            .iter()
            .map(|c| Texture::new(c.to_vec(), 1, 1))
            .collect(),
    );
    tile.uv_pages = Some(TileUvPages {
        animation_frames: 2,
        repeat_period: [0.; 2],
        origin: [0; 2],
        size: [2, 1],
        axes: [[0.; 3]; 2],
        units_per_page: 1,
    });
    let id = tile.id;
    assets.tiles.insert(id, tile);
    let art = ScreenArtwork {
        sources: vec![PixelSource::TileId(id)],
        repeat: true,
        slice: 0,
        origin: [0.; 2],
        units_per_pixel: 1. / 32.,
    };
    let source = Texture::new((0..9).flat_map(|i| [i * 25, 0, 0, 255]).collect(), 3, 3);
    let mut list = PaintList::default();
    {
        let mut images = cache.begin_frame(&mut registry).unwrap();
        let mut p = ScreenPainter {
            list: &mut list,
            text: &mut text,
            images: &mut images,
        };
        p.artwork(
            &art,
            Rect::new(0., 0., 64., 24.),
            &assets,
            ButtonVisualState::Normal,
            0,
        )
        .unwrap();
        p.artwork(
            &art,
            Rect::new(64., 0., 64., 24.),
            &assets,
            ButtonVisualState::Normal,
            1,
        )
        .unwrap();
        let mut outside = art.clone();
        outside.origin[0] = -2.;
        p.artwork(
            &outside,
            Rect::new(128., 0., 128., 24.),
            &assets,
            ButtonVisualState::Normal,
            0,
        )
        .unwrap();
        p.sliced_texture("chrome", &source, Rect::new(0., 32., 16., 16.), 1)
            .unwrap();
        p.sliced_texture("chrome", &source, Rect::new(20., 32., 1., 1.), 1)
            .unwrap();
    }
    list.validate().unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(256, 64)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &registry,
            &output.view,
            output.descriptor.format,
            (256, 64),
            1.,
            [0.; 4],
        )
        .unwrap();
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = capture.try_finish(&gpu).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &capture.pixels()[(y * 256 + x) * 4..(y * 256 + x + 1) * 4];
    for (x, c) in [
        (8, colors[0]),
        (40, colors[1]),
        (72, colors[2]),
        (104, colors[3]),
        (136, colors[0]),
        (216, colors[0]),
        (232, colors[1]),
    ] {
        assert_eq!(pixel(x, 8), c, "spatial/animation source at {x}");
    }
    assert_eq!(pixel(0, 32), [0, 0, 0, 255]);
    assert_eq!(pixel(15, 32), [50, 0, 0, 255]);
    assert_eq!(pixel(8, 40), [100, 0, 0, 255]);
    assert_eq!(pixel(0, 47), [150, 0, 0, 255]);
    assert_eq!(pixel(15, 47), [200, 0, 0, 255]);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn prepared_screen_polygons_keep_concavity_transforms_and_spatial_animation() {
    use rusterix::{PixelSource, Texture, Tile, TileUvPages, scenebuilder::d2builder::D2Builder};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = ScreenImageCache::default();
    let mut text = TextSystem::default();
    let mut assets = Assets::default();
    let colors = [
        [90, 0, 90, 255],
        [90, 0, 90, 255],
        [90, 0, 90, 255],
        [90, 0, 90, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];
    let mut tile = Tile::from_textures(
        colors
            .into_iter()
            .map(|c| Texture::new(c.to_vec(), 1, 1))
            .collect(),
    );
    let id = tile.id;
    tile.uv_pages = Some(TileUvPages {
        animation_frames: 2,
        repeat_period: [0.; 2],
        origin: [0; 2],
        size: [2; 2],
        axes: [[0.; 3]; 2],
        units_per_page: 2,
    });
    assets.set_tiles([(id, tile)].into_iter().collect());
    let mut map = Map::default();
    let vertices = [(0., 0.), (4., 0.), (4., 1.), (1., 1.), (1., 4.), (0., 4.)]
        .map(|(x, y)| map.add_vertex_at(x, y));
    for i in 0..vertices.len() {
        map.create_linedef(vertices[i], vertices[(i + 1) % vertices.len()]);
    }
    assert_eq!(map.sectors.len(), 1);
    map.sectors[0]
        .properties
        .set("source", Value::Source(PixelSource::TileId(id)));
    let mut scene = D2Builder::new().build(&map, &assets, scenevm::prelude::Vec2::new(96., 96.));
    scene.animation_frame = 1;
    assert!(!scene.d2_static.is_empty());
    assert!(
        scene
            .d2_static
            .iter()
            .all(|b| b.projected_vertices.is_empty())
    );
    let mut list = PaintList::default();
    list.ops.push(PaintOp::PushTransform(
        Transform::scale(16., 16.).then(Transform::translate(8., 8.)),
    ));
    {
        let mut images = cache.begin_frame(&mut registry).unwrap();
        let mut painter = ScreenPainter {
            list: &mut list,
            text: &mut text,
            images: &mut images,
        };
        painter.unlit_scene(&scene, &assets).unwrap();
    }
    list.ops.push(PaintOp::PopTransform);
    list.validate().unwrap();
    assert!(
        list.ops
            .iter()
            .filter(|op| matches!(op, PaintOp::ImageMesh { .. }))
            .count()
            >= 3
    );
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(96, 96)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &registry,
            &output.view,
            output.descriptor.format,
            (96, 96),
            1.,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = capture.try_finish(&gpu).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &capture.pixels()[(y * 96 + x) * 4..(y * 96 + x + 1) * 4];
    assert_eq!(pixel(16, 16), [255, 0, 0, 255]);
    assert_eq!(pixel(56, 16), [0, 255, 0, 255]);
    assert_eq!(pixel(16, 56), [0, 0, 255, 255]);
    assert_eq!(
        pixel(56, 56),
        [0; 4],
        "concave empty area must remain clear"
    );
    assert_eq!(pixel(4, 4), [0; 4], "logical transform must apply once");
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn screen_sampler_repeats_only_the_authored_axis() {
    use rusterix::{Batch2D, PixelSource, RepeatMode, Scene, Texture, Tile};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = ScreenImageCache::default();
    let mut text = TextSystem::default();
    let mut assets = Assets::default();
    let tile = Tile::from_texture(Texture::new(
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ],
        2,
        2,
    ));
    let id = tile.id;
    assets.set_tiles([(id, tile)].into_iter().collect());
    let mut scene = Scene::default();
    for (x, repeat) in [(0., RepeatMode::RepeatX), (32., RepeatMode::RepeatY)] {
        let mut batch = Batch2D::from_rectangle(x, 0., 32., 32.);
        batch.repeat_mode = repeat;
        batch.source = PixelSource::StaticTileIndex(0);
        for uv in &mut batch.uvs {
            *uv = uv.map(|v| v * 3.);
        }
        scene.d2_static.push(batch);
    }
    let mut list = PaintList::default();
    {
        let mut images = cache.begin_frame(&mut registry).unwrap();
        ScreenPainter {
            list: &mut list,
            text: &mut text,
            images: &mut images,
        }
        .unlit_scene(&scene, &assets)
        .unwrap();
    }
    assert_eq!(cache.len(), 1);
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(64, 32)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &registry,
            &output.view,
            output.descriptor.format,
            (64, 32),
            1.,
            [0.; 4],
        )
        .unwrap();
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = capture.try_finish(&gpu).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &capture.pixels()[(y * 64 + x) * 4..(y * 64 + x + 1) * 4];
    assert_eq!(pixel(24, 24), [0, 0, 255, 255], "repeat X must clamp Y");
    assert_eq!(pixel(56, 24), [0, 255, 0, 255], "repeat Y must clamp X");
}
