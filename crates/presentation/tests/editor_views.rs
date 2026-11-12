//! CPU-reference fixtures and native authoring-view output contracts.
use renderkit::GpuContext;
use rusterix::{Assets, Client, D3Camera, D3OrbitCamera, Map, SceneHandler};
use scenevm::{
    Atom, Chunk, GeoId, Poly2D, Poly3D, SceneVM,
    prelude::{Vec2, Vec3},
};
use std::sync::Arc;
use theframework_core::Uuid;

const SIZE: (u32, u32) = (96, 80);
const CASES: [&str; 8] = [
    "raster-grid",
    "raster-no-grid",
    "compute-grid",
    "compute-no-overlay",
    "orbit-neutral",
    "orbit-sky",
    "orbit-transparent",
    "orbit-camera-override",
];

fn fixture(gpu: Arc<GpuContext>) -> (SceneHandler, Client, Map, Assets) {
    let mut scene = SceneHandler::with_vm(SceneVM::new_with_context(gpu, 1, 1));
    let assets = Assets::default();
    scene.sync_base_render_settings(&assets.config);
    let shader = rusterix::Embedded::get("shader/2d_shader.wgsl").unwrap();
    scene.vm.execute(Atom::SetSource2D(
        std::str::from_utf8(&shader.data).unwrap().into(),
    ));
    scene.build_atlas(&assets.tiles, true);
    scene.clear_overlay();
    let (client, map) = content(&mut scene);
    (scene, client, map, assets)
}
fn content(scene: &mut SceneHandler) -> (Client, Map) {
    let mut base = Chunk::new(Vec2::zero(), 8);
    base.add(Poly2D::quad(
        GeoId::Unknown(1),
        scene.white,
        [0., 0.],
        1.25,
        0,
        true,
    ));
    base.add_3d(Poly3D::cube(
        GeoId::Unknown(2),
        scene.white,
        Vec3::zero(),
        1.2,
    ));
    scene.vm.execute(Atom::AddChunk {
        id: Uuid::new_v4(),
        chunk: base,
    });
    scene.add_overlay_2d_line(
        GeoId::Unknown(3),
        Vec2::new(-1.5, -0.7),
        Vec2::new(1.5, 0.8),
        scene.red,
        2,
    );
    scene.overlay_3d.add_3d(Poly3D::cube(
        GeoId::Unknown(4),
        scene.red,
        Vec3::new(0.8, 0.6, 0.3),
        0.3,
    ));
    scene.tool_overlay_3d.add_3d(Poly3D::cube(
        GeoId::Unknown(5),
        scene.blue,
        Vec3::new(-0.7, 0.6, 0.2),
        0.3,
    ));
    scene.set_overlay();
    let mut client = Client::new();
    let mut orbit = D3OrbitCamera::new();
    orbit.distance = 4.;
    orbit.azimuth = 1.1;
    orbit.elevation = 0.45;
    client.camera_d3 = Box::new(orbit);
    client.use_editor_canvas = true;
    client.editor_canvas_colors = Some([[32, 28, 24, 255], [80, 70, 60, 255], [48, 43, 38, 255]]);
    client.animation_frame = 7;
    let mut map = Map::default();
    map.offset = Vec2::new(5., -3.);
    map.grid_size = 17.;
    map.subdivisions = 4.;
    (client, map)
}
fn configure_case(index: usize, scene: &mut SceneHandler, client: &mut Client) {
    use rusterix::render_settings::RendererBackend;
    scene.base_settings.backend_2d = if index == 2 || index == 3 {
        RendererBackend::Compute
    } else {
        RendererBackend::Raster
    };
    scene.base_settings.backend_3d = RendererBackend::Raster;
    scene.settings = scene.base_settings.clone();
    client.builder_d2.draw_grid = index != 1;
    if index == 3 {
        scene.vm.set_layer_enabled(1, false);
    }
}
fn override_camera() -> scenevm::Camera3D {
    scenevm::Camera3D::builder()
        .with_pos(Vec3::new(0., 1., 4.))
        .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
}
fn reference(
    index: usize,
    scene: &mut SceneHandler,
    client: &mut Client,
    map: &Map,
    assets: &Assets,
) -> Vec<u8> {
    let mut pixels = vec![0; (SIZE.0 * SIZE.1 * 4) as usize];
    if index < 4 {
        client.draw_d2(
            map,
            &mut pixels,
            SIZE.0 as usize,
            SIZE.1 as usize,
            assets,
            scene,
        );
    } else {
        client.draw_d3_with_camera_override(
            map,
            &mut pixels,
            SIZE.0 as usize,
            SIZE.1 as usize,
            assets,
            scene,
            index == 4,
            (index == 7).then(override_camera),
            index != 6,
            index == 6,
        );
    }
    pixels
}
fn expected(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/editor_views")
        .join(format!("{name}.png"));
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width, info.height), SIZE);
    assert_eq!(info.color_type, png::ColorType::Rgba);
    pixels.truncate(info.buffer_size());
    pixels
}
fn capture(gpu: &GpuContext, mut ticket: renderkit::ReadbackTicket) -> Vec<u8> {
    gpu.device
        .poll(renderkit::wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
fn assert_pixels(name: &str, actual: &[u8], expected: &[u8]) {
    let differing = actual.iter().zip(expected).filter(|(a, b)| a != b).count();
    assert_eq!(actual.len(), expected.len());
    assert_eq!(
        differing, 0,
        "{name}: bytes differ from pre-extraction reference"
    );
}
#[test]
#[ignore = "requires actual native GPU"]
fn reference_editor_views_preserve_pre_extraction_pixels() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    for (index, name) in CASES.iter().enumerate() {
        let (mut scene, mut client, map, assets) = fixture(gpu.clone());
        configure_case(index, &mut scene, &mut client);
        let pixels = reference(index, &mut scene, &mut client, &map, &assets);
        assert_pixels(name, &pixels, &expected(name));
    }
}
#[test]
#[ignore = "requires actual native GPU"]
fn native_editor_views_consume_frozen_frames_and_preserve_independent_targets() {
    use eldiron_presentation::{SceneViews, ViewId};
    use renderkit::{GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket};
    use theframework::{
        paint::{PaintList, Rect, Sampling},
        render_wgpu::PaintRenderer,
    };
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut views = SceneViews::new(&images, scenevm::SharedAtlas::new(1024, 1024));
    let assets = Assets::default();
    let mut outputs = Vec::new();
    let mut retained = Vec::new();
    for (index, name) in CASES.iter().enumerate() {
        let id = ViewId::new();
        views.create(id, SIZE).unwrap();
        views.configure_editor(id, &assets).unwrap();
        let (mut client, mut map) = content(views.scene_mut(id).unwrap());
        configure_case(index, views.scene_mut(id).unwrap(), &mut client);
        let authored = serde_json::to_vec(&map).unwrap();
        let frame = if index < 4 {
            client.editor_view_2d(&map, SIZE)
        } else {
            client.editor_view_3d(
                &map,
                SIZE,
                index == 4,
                (index == 7).then(override_camera),
                index != 6,
                index == 6,
            )
        };
        assert_eq!(serde_json::to_vec(&map).unwrap(), authored);
        // Captured descriptions do not retain live Client/Map controller state.
        map.offset = Vec2::new(900., 600.);
        client.animation_frame = 99;
        client.editor_canvas_colors = None;
        let before = gpu.transfers();
        views.prepare_editor(id, &frame, &mut images).unwrap();
        let output = views.render(id, 17, &mut images).unwrap().unwrap();
        assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
        assert_eq!(gpu.transfers().capture_requests, before.capture_requests);
        assert!(!views.has_cpu_target_storage(id).unwrap());
        let warm = gpu.transfers();
        assert_eq!(views.render(id, 17, &mut images).unwrap(), Some(output));
        assert_eq!(
            gpu.transfers(),
            warm,
            "{name}: unchanged views must remain idle"
        );
        let pixels = capture(&gpu, views.capture(id).unwrap().ticket);
        assert_pixels(name, &pixels, &expected(name));
        retained.push(images.get(output.image).unwrap());
        outputs.push((output, frame, pixels));
    }
    for (output, _, pixels) in &outputs {
        assert_eq!(views.output(output.view), Some(*output));
        assert_eq!(
            capture(&gpu, views.capture(output.view).unwrap().ticket),
            *pixels,
            "other views cannot replace a completed target or camera"
        );
    }
    // The common compositor consumes these ordinary authoring outputs directly.
    let mut list = PaintList::default();
    for (index, (output, _, _)) in outputs.iter().enumerate() {
        list.image(
            output.image,
            Rect::new(
                (index as u32 * SIZE.0) as f32,
                0.,
                SIZE.0 as f32,
                SIZE.1 as f32,
            ),
            Sampling::Nearest,
        );
    }
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(SIZE.0 * 8, SIZE.1)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (SIZE.0 * 8, SIZE.1),
            1.,
            [0., 0., 0., 1.],
        )
        .unwrap();
    let pixels = capture(&gpu, ReadbackTicket::capture(&gpu, output).unwrap());
    let mut encoder = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-editor-view-strip.png").unwrap(),
        SIZE.0 * 8,
        SIZE.1,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    // Zero size and foreign registry calls cannot replace the retained source/target.
    let (first, frame, _) = &outputs[0];
    let mut hidden = frame.clone();
    let rusterix::editor_view::EditorViewFrame::D2(state) = &mut hidden else {
        unreachable!()
    };
    state.size = (0, 0);
    let before = gpu.transfers();
    views
        .prepare_editor(first.view, &hidden, &mut images)
        .unwrap();
    assert_eq!(views.render(first.view, 18, &mut images).unwrap(), None);
    assert_eq!(gpu.transfers(), before);
    views
        .prepare_editor(first.view, frame, &mut images)
        .unwrap();
    let resumed = views.render(first.view, 18, &mut images).unwrap().unwrap();
    assert_eq!(resumed.image, first.image);
    assert!(Arc::ptr_eq(
        &retained[0],
        &images.get(resumed.image).unwrap()
    ));
    let mut foreign = ImageRegistry::new(gpu.clone());
    assert!(
        views
            .prepare_editor(first.view, frame, &mut foreign)
            .is_err()
    );
    assert_eq!(views.output(first.view), Some(resumed));
}

#[test]
#[ignore = "requires actual native GPU and device reconstruction"]
fn native_editor_source_publications_keep_views_overlays_and_recovery_independent() {
    use eldiron_presentation::{SceneViews, ViewId};
    use renderkit::ImageRegistry;
    use rusterix::{Texture, Tile, scene_build_index::SceneBuildIndex};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut views = SceneViews::new(&images, scenevm::SharedAtlas::new(1024, 1024));
    let tile = Tile::from_texture(Texture::from_color([200, 80, 30, 255]));
    let tile_id = tile.id;
    let mut assets = Assets::default();
    assets.tiles.insert(tile.id, tile);
    let mut index = SceneBuildIndex::default();
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add(Poly2D::quad(
        GeoId::Sector(12),
        tile_id,
        [0., 0.],
        1.25,
        0,
        true,
    ));
    index.index_chunk(&chunk);
    let original = index.snapshot();
    let client = Client::new();
    let mut map = Map::default();
    map.grid_size = 22.;
    let frame = client.editor_view_2d(&map, SIZE);
    let a = ViewId::new();
    let b = ViewId::new();
    for id in [a, b] {
        views.create(id, SIZE).unwrap();
        views.configure_editor(id, &assets).unwrap();
        let scene = views.scene_mut(id).unwrap();
        scene.add_overlay_2d_line(
            GeoId::Unknown(90),
            Vec2::new(-2., -1.),
            Vec2::new(2., 1.),
            scene.red,
            2,
        );
        scene.set_overlay();
        // Publication restores the caller's active layer, and does not replace
        // independently prepared editor guides.
        scene.vm.set_active_vm(2);
        assert!(views.apply_editor_source(id, original.clone()).unwrap());
        assert_eq!(views.scene(id).unwrap().vm.active_vm_index(), 2);
        views.prepare_editor(id, &frame, &mut images).unwrap();
        views.render(id, 1, &mut images).unwrap();
    }
    let first = views.output(a).unwrap();
    let second = views.output(b).unwrap();
    let before = capture(&gpu, views.capture(a).unwrap().ticket);
    assert_eq!(before, capture(&gpu, views.capture(b).unwrap().ticket));
    let warm = gpu.transfers();
    assert!(!views.apply_editor_source(a, original.clone()).unwrap());
    assert_eq!(views.render(a, 1, &mut images).unwrap(), Some(first));
    assert_eq!(
        gpu.transfers(),
        warm,
        "same source publication performs no GPU transfer or render"
    );
    chunk
        .polys_map
        .get_mut(&GeoId::Sector(12))
        .unwrap()
        .vertices
        .iter_mut()
        .for_each(|v| v[0] += 1.);
    index.index_chunk(&chunk);
    let changed = index.snapshot();
    assert!(views.apply_editor_source(a, changed).unwrap());
    views.render(a, 2, &mut images).unwrap();
    assert_ne!(before, capture(&gpu, views.capture(a).unwrap().ticket));
    assert_eq!(views.output(b), Some(second));
    assert_eq!(before, capture(&gpu, views.capture(b).unwrap().ticket));
    index.clear();
    assert!(views.apply_editor_source(a, index.snapshot()).unwrap());
    views.render(a, 3, &mut images).unwrap();
    let cleared = capture(&gpu, views.capture(a).unwrap().ticket);
    assert!(
        cleared.chunks_exact(4).any(|p| p[0] > p[1] + 20),
        "clearing world chunks preserves red editor guides"
    );
    assert!(
        views
            .scene(a)
            .unwrap()
            .build_index
            .snapshot()
            .chunks()
            .is_empty()
    );
    assert_eq!(
        original.chunks().len(),
        1,
        "retained source outlives current source mutation"
    );
    // CPU geometry and authored atlas assets reconstruct without retaining or
    // importing the old VM/device/targets. Native guide IDs regenerate locally.
    let replacement = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut new_images = ImageRegistry::new(replacement.clone());
    let mut rebuilt = SceneViews::new(&new_images, scenevm::SharedAtlas::new(1024, 1024));
    rebuilt.create(a, SIZE).unwrap();
    rebuilt.configure_editor(a, &assets).unwrap();
    let scene = rebuilt.scene_mut(a).unwrap();
    scene.add_overlay_2d_line(
        GeoId::Unknown(90),
        Vec2::new(-2., -1.),
        Vec2::new(2., 1.),
        scene.red,
        2,
    );
    scene.set_overlay();
    assert!(rebuilt.apply_editor_source(a, original).unwrap());
    rebuilt.prepare_editor(a, &frame, &mut new_images).unwrap();
    let result = rebuilt.render(a, 1, &mut new_images).unwrap().unwrap();
    assert_ne!(result.image, first.image);
    assert!(!rebuilt.has_cpu_target_storage(a).unwrap());
    assert_eq!(replacement.transfers().readback_bytes, 0);
    assert_eq!(
        before,
        capture(&replacement, rebuilt.capture(a).unwrap().ticket)
    );
}
