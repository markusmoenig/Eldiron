//! Explicit WebGPU acceptance fixture for the real Rust bake owner. It is a
//! verification executable, never an application rendering or memory-policy hook.
//! Build for wasm32 with Creator's `native-render` feature; package with
//! `eldiron-package-web`, then serve in a browser with WebGPU enabled.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("Build this acceptance fixture for wasm32-unknown-unknown");
}
#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        let result = fixture::run().await;
        let (state, result) = match result {
            Ok(value) => ("passed", value),
            Err(error) => ("failed", serde_json::json!({"error":error})),
        };
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .document_element()
            .unwrap();
        root.set_attribute("data-paint-pressure-state", state)
            .unwrap();
        root.set_attribute("data-paint-pressure-result", &result.to_string())
            .unwrap();
        web_sys::console::log_1(&result.to_string().into());
    });
}
#[cfg(target_arch = "wasm32")]
mod fixture {
    use eldiron_presentation::orthographic_bake::{BakeAdvance, OrthographicBakeJob};
    use renderkit::GpuContext;
    use rusterix::{
        Assets, D3Camera, D3IsoCamera, Map, MapCamera, OrthographicBakeController, Rusterix,
        SceneHandler, Texture,
        editor_view::{EditorView3D, EditorViewFrame},
    };
    use scenevm::{Atom, Camera3D, Chunk, GeoId, Poly3D, SceneVM, SharedAtlas};
    use shared::{
        iso_paint::{IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner},
        surface_paint::pages::{PaintPageIndex, PaintPageIndexBuilder},
    };
    use std::sync::Arc;
    use theframework::prelude::Uuid;
    use vek::{Vec2, Vec3};
    const BUDGET: u64 = 2 * 64 * 64 * 8;
    type Result<T> = std::result::Result<T, String>;
    fn check(value: bool, message: &str) -> Result<()> {
        if value { Ok(()) } else { Err(message.into()) }
    }
    async fn tick() {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1)
                .unwrap();
        });
        wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    }
    struct Fixture {
        assets: Assets,
        chunk: Chunk,
        camera: Camera3D,
        frame: EditorView3D,
        map: Map,
        source: Arc<PaintPageIndex>,
    }
    fn scene(context: &Arc<GpuContext>, f: &Fixture) -> SceneHandler {
        let vm =
            SceneVM::new_with_context_and_atlas(context.clone(), 1, 1, SharedAtlas::new(256, 256));
        let mut scene = SceneHandler::with_vm(vm);
        scene.sync_base_render_settings(&f.assets.config);
        scene.build_atlas(&f.assets.tiles, true);
        scene.vm.set_background_lighting(false);
        scene.vm.execute(Atom::AddChunk {
            id: Uuid::new_v4(),
            chunk: f.chunk.clone(),
        });
        scene
    }
    async fn prepare(context: &Arc<GpuContext>) -> Result<Fixture> {
        let mut assets = Assets::default();
        let mut texture = Texture::new(vec![210, 60, 40, 255], 1, 1);
        texture.data_ext = Some(vec![0x08, 0x0f, 128, 128].into());
        let tile = rusterix::Tile::from_texture(texture);
        let tile_id = tile.id;
        assets.set_tiles([(tile_id, tile)].into_iter().collect());
        let geo = [7, 0, 0, 1];
        let mut poly = Poly3D::cube(GeoId::Sector(1), tile_id, Vec3::zero(), 1.);
        poly.paint_surface_id = Some(geo);
        poly.paint_uvs = poly.uvs.iter().map(|uv| [uv[0] * 6., uv[1] * 2.]).collect();
        let mut chunk = Chunk::new(Vec2::zero(), 8);
        chunk.add_3d(poly);
        let mut map = Map::new();
        map.camera = MapCamera::ThreeDIso;
        let mut camera = D3IsoCamera::new();
        camera.scale = 1.;
        camera.distance = 4.;
        camera.height_clearance = 0.;
        camera.azimuth_deg = 0.;
        camera.elevation_deg = 0.;
        let camera_frame = camera.as_scenevm_camera_for_surface(256., 256.);
        // Obtain the same production frame description as Creator's editor.
        let vm =
            SceneVM::new_with_context_and_atlas(context.clone(), 1, 1, SharedAtlas::new(256, 256));
        let mut engine = Rusterix::with_scene_handler(SceneHandler::with_vm(vm));
        engine.assets = assets.clone();
        engine.client.camera_d3 = Box::new(camera);
        let EditorViewFrame::D3(frame) =
            engine
                .client
                .editor_view_3d(&map, (256, 256), true, None, true, false)
        else {
            return Err("missing editor frame".into());
        };
        drop(engine);
        let mut paint = IsoPaintLayer::default();
        for at in 0..2049u32 {
            let mut page = IsoPaintBakedChunk::new(
                IsoPaintOwner::Sector(if at < 3 { 1 } else { 99 }),
                if at < 3 { geo } else { [99, at, 0, 1] },
                if at < 3 { [at as i32 * 64, 0] } else { [0; 2] },
            );
            if at < 3 {
                for pixel in page.color_rgba.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&[30 + at as u8 * 45, 190, 60, 255]);
                }
                for pixel in page.material_rgba.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&[254, 1 + at as u8, 255, 255]);
                }
            } else {
                page.color_rgba[..4].copy_from_slice(&[at as u8, (at >> 8) as u8, 128, 255]);
            }
            paint.baked_chunks.insert(at.to_string(), page);
            if at % 32 == 0 {
                tick().await;
            }
        }
        let mut builder = PaintPageIndexBuilder::new(paint, 2 * 1024 * 1024)?;
        while !builder.advance(16)? {
            tick().await;
        }
        let source = Arc::new(builder.finish()?.0);
        Ok(Fixture {
            assets,
            chunk,
            camera: camera_frame,
            frame,
            map,
            source,
        })
    }
    fn start(
        context: &Arc<GpuContext>,
        f: &Fixture,
        full: bool,
    ) -> Result<(OrthographicBakeJob, OrthographicBakeController)> {
        let mut prepared = scene(context, f);
        if full {
            let mut color = vec![0; 192 * 64 * 4];
            let mut material = color.clone();
            let mut residents = Vec::new();
            for at in 0..3usize {
                let slot = f.source.slot([7, 0, 0, 1], [at as i32 * 64, 0]).unwrap();
                let page = f.source.page(slot).unwrap();
                for row in 0..64usize {
                    let start = (row * 192 + at * 64) * 4;
                    color[start..start + 256]
                        .copy_from_slice(&page.color[row * 256..row * 256 + 256]);
                    material[start..start + 256]
                        .copy_from_slice(&page.material[row * 256..row * 256 + 256]);
                }
                residents.push((slot, [at as u32 * 64, 0]));
            }
            prepared.vm.set_raster3d_surface_paint(
                192,
                64,
                color,
                material,
                f.source.resident_entries([192, 64], &residents)?,
                f.source.alpha_geo_ids().to_vec(),
            );
        }
        let mut job = OrthographicBakeJob::new(context.clone(), prepared, f.frame.clone())
            .map_err(|e| e.to_string())?;
        if !full {
            job.set_paint_pages(f.source.clone(), BUDGET)
                .map_err(|e| e.to_string())?;
        }
        let mut bake = OrthographicBakeController::default();
        bake.request_render(f.map.id, 3);
        bake.prepare_work(f.map.id, 256, 256, f.camera, Some((-0.5, 0.5, -0.5, 0.5)))?;
        Ok((job, bake))
    }
    async fn pressure(
        context: &Arc<GpuContext>,
        f: &Fixture,
        job: &mut OrthographicBakeJob,
        bake: &mut OrthographicBakeController,
    ) -> Result<()> {
        let before = context.transfers();
        for _ in 0..6000 {
            context.poll().map_err(|e| e.to_string())?;
            check(
                matches!(
                    job.advance(bake, f.map.id, f.camera, (256, 256))
                        .map_err(|e| e.to_string())?,
                    BakeAdvance::Pending
                ),
                "pressure published an early sample",
            )?;
            check(
                context.transfers().readback_bytes == before.readback_bytes,
                "pressure captured an unresolved original",
            )?;
            if job.paint_subdivisions() > 0 {
                return Ok(());
            }
            tick().await;
        }
        Err("pressure never subdivided".into())
    }
    async fn produce(
        context: &Arc<GpuContext>,
        f: &Fixture,
        full: bool,
    ) -> Result<(OrthographicBakeController, serde_json::Value)> {
        let (mut job, mut bake) = start(context, f, full)?;
        let before = context.transfers();
        let mut samples = 0;
        let mut completed = false;
        for _ in 0..20000 {
            context.poll().map_err(|e| e.to_string())?;
            match job
                .advance(&mut bake, f.map.id, f.camera, (256, 256))
                .map_err(|e| e.to_string())?
            {
                BakeAdvance::Sample => samples += 1,
                BakeAdvance::Tile { complete: true } => {
                    completed = true;
                    break;
                }
                BakeAdvance::Pending => (),
                other => return Err(format!("unexpected production {other:?}")),
            }
            tick().await;
        }
        check(
            completed && samples == 3,
            "progressive production did not complete all samples",
        )?;
        let usage = job.paint_residency_usage();
        if !full {
            check(
                job.paint_subdivisions() > 0,
                "bounded owner did not subdivide",
            )?;
            check(
                usage.unwrap().gpu_plane_bytes <= BUDGET,
                "paint planes exceeded budget",
            )?;
        }
        let after = context.transfers();
        check(
            after.asset_bake_readback_requests - before.asset_bake_readback_requests == 7,
            "unexpected original capture count",
        )?;
        check(
            after.readback_bytes - before.readback_bytes
                == after.asset_bake_readback_bytes - before.asset_bake_readback_bytes,
            "implicit color readback",
        )?;
        Ok((
            bake,
            serde_json::json!({"samples":samples,"subdivisions":job.paint_subdivisions(),"gpu_plane_bytes":usage.map(|u|u.gpu_plane_bytes),"page_uploads":usage.map(|u|u.page_uploads),"status_bytes":after.status_readback_bytes-before.status_readback_bytes,"original_capture_requests":after.asset_bake_readback_requests-before.asset_bake_readback_requests}),
        ))
    }
    pub async fn run() -> Result<serde_json::Value> {
        let context = GpuContext::headless().await.map_err(|e| e.to_string())?;
        let f = prepare(&context).await?;
        let (mut job, mut bake) = start(&context, &f, false)?;
        pressure(&context, &f, &mut job, &mut bake).await?;
        bake.request_render(f.map.id, 3);
        check(
            job.advance(&mut bake, f.map.id, f.camera, (256, 256))
                .map_err(|e| e.to_string())?
                == BakeAdvance::Retired,
            "request identity failed to retire pressure",
        )?;
        check(
            bake.take_persisted_update_for(f.map.id).is_none(),
            "retired request published an asset",
        )?;
        drop(job);
        let (mut job, mut bake) = start(&context, &f, false)?;
        pressure(&context, &f, &mut job, &mut bake).await?;
        context.device.destroy();
        for _ in 0..100 {
            if context.is_lost() {
                break;
            }
            tick().await;
        }
        check(context.is_lost(), "actual device loss was not delivered")?;
        check(
            job.advance(&mut bake, f.map.id, f.camera, (256, 256))
                .is_err(),
            "lost pressure job did not fail",
        )?;
        check(
            job.advance(&mut bake, f.map.id, f.camera, (256, 256))
                .map_err(|e| e.to_string())?
                == BakeAdvance::Retired,
            "lost pressure job remained active",
        )?;
        check(
            bake.take_persisted_update_for(f.map.id).is_none(),
            "lost pressure published an asset",
        )?;
        drop(job);
        let replacement = GpuContext::headless().await.map_err(|e| e.to_string())?;
        let (expected, reference_stats) = produce(&replacement, &f, true).await?;
        let (actual, pressure_stats) = produce(&replacement, &f, false).await?;
        let expected = expected
            .presentation_source(f.map.id, &f.camera)
            .ok_or("missing reference source")?;
        let actual = actual
            .presentation_source(f.map.id, &f.camera)
            .ok_or("missing pressure source")?;
        let mut compared = 0usize;
        let mut green = 0usize;
        for (at, lease) in actual.tiles() {
            let tile = lease.source();
            let reference = expected.tile(at.0, at.1).ok_or("missing reference tile")?;
            let reference = reference.source();
            for (a, b) in [
                (tile.color, reference.color),
                (tile.albedo, reference.albedo),
                (tile.material, reference.material),
                (tile.normal, reference.normal),
            ] {
                check(a == b, "subdivision changed an original RGBA channel")?;
                compared += a.len();
            }
            check(
                tile.depth.len() == reference.depth.len(),
                "depth dimensions changed",
            )?;
            for (a, b) in tile.depth.iter().zip(reference.depth) {
                check(
                    (a.is_infinite() && b.is_infinite()) || a.to_bits() == b.to_bits(),
                    "subdivision changed original depth",
                )?;
            }
            green += tile
                .albedo
                .chunks_exact(4)
                .filter(|p| p[1] > p[0].saturating_add(50))
                .count();
        }
        check(
            compared > 0 && green > 100,
            "fixture did not render actual paint",
        )?;
        Ok(
            serde_json::json!({"source_pages":f.source.source_count(),"logical_original_bytes":f.source.logical_source_bytes(),"table_bytes":f.source.table_bytes(),"plane_budget":BUDGET,"reference":reference_stats,"pressure":pressure_stats,"request_retirement":true,"actual_device_loss_retirement":true,"reconstructed_device_generation":replacement.generation(),"rgba_bytes_compared":compared,"green_albedo_pixels":green,"rgba_max_delta":0,"depth":"exact finite agreement","scope":"Real Rust OrthographicBakeJob on browser WebGPU, explicit verification assets, not canonical application surface/input acceptance"}),
        )
    }
}
