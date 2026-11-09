//! Isolated, renderer-backed previews; never draw approximated sector rectangles.
use super::*;
use rusterix::{D3Camera, D3FirstPCamera, D3IsoCamera, client::widget::game::GameWidget};
use serde_json::{Value as Json, json};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
static ACTIVE: AtomicBool = AtomicBool::new(false);

struct PreviewContext {
    runtime: Rusterix,
    bakes: crate::procedural_material::ScepterBakeCache,
}
static CONTEXT: Mutex<Option<PreviewContext>> = Mutex::new(None);
struct PreviewGuard;
impl Drop for PreviewGuard {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}
pub(super) fn render(
    mut project: Project,
    index: usize,
    request: &RegionRenderPreview,
) -> Result<Json, String> {
    if ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("Another preview is rendering; wait for its job to complete".into());
    }
    let _guard = PreviewGuard;
    let _aliases = rusterix::server::data::TileAliasScope::new(&project.tiles);
    let [width, height] = request.size.unwrap_or([768, 512]);
    if !(64..=1024).contains(&width) || !(64..=1024).contains(&height) {
        return Err("Preview dimensions must be 64..1024".into());
    }
    if request.zoom.is_some_and(|v| !v.is_finite() || v <= 0.)
        || request.camera.as_ref().is_some_and(|c| {
            c.azimuth.is_some_and(|v| !v.is_finite())
                || c.elevation
                    .is_some_and(|v| !v.is_finite() || v.abs() > 89.9)
                || c.fov
                    .is_some_and(|v| !v.is_finite() || !(10. ..=150.).contains(&v))
                || c.pitch.is_some_and(|v| !v.is_finite() || v.abs() > 85.)
                || c.scale.is_some_and(|v| !v.is_finite() || v <= 0.)
        })
    {
        return Err("Invalid preview camera parameters".into());
    }
    let view = request.view.as_deref().unwrap_or("2d");
    if !["2d", "2d_iso", "iso", "3d"].contains(&view) {
        return Err("view must be 2d, 2d_iso, iso or 3d".into());
    }
    let mut context = CONTEXT.lock().unwrap_or_else(|e| {
        let mut guard = e.into_inner();
        *guard = None;
        CONTEXT.clear_poison();
        guard
    });
    if context.is_none() {
        let mut runtime = Rusterix::new_without_audio();
        runtime.scene_handler.vm.set_background_lighting(false);
        let (device, queue) = runtime
            .scene_handler
            .vm
            .gpu_context()
            .ok_or("GPU is unavailable")?;
        *context = Some(PreviewContext {
            runtime,
            bakes: crate::procedural_material::ScepterBakeCache::new(device, queue),
        });
    }
    let PreviewContext { runtime, bakes } = context.as_mut().unwrap();
    runtime.scene_handler.clear_runtime_scene();
    runtime.scene_handler.settings = Default::default();
    let geometry_errors = shared::geometry_recipe::synchronize(&mut project).1;
    if !geometry_errors.is_empty() {
        return Err(geometry_errors.join("; "));
    }
    bakes.prepare(&mut project)?;
    let iso_errors = shared::isometric::synchronize(&mut project);
    if !iso_errors.is_empty() {
        return Err(iso_errors.join("; "));
    }
    shared::rusterix_utils::insert_content_into_maps(&mut project);
    let _ = shared::rusterix_utils::setup_client(runtime, &mut project);
    shared::rusterix_utils::sync_display_templates(&mut runtime.assets, &project, false);
    runtime.scene_handler.preview_character_setup = true;
    runtime.scene_handler.set_timings(
        project.target_fps as f32,
        project.tick_ms.min(i32::MAX as u32) as i32,
    );
    let map = project
        .regions
        .get(index)
        .ok_or("Region no longer exists")?
        .map
        .clone();
    runtime.set_tiles_for_maps(project.tiles.clone(), false, [&map]);
    runtime
        .scene_handler
        .settings
        .read(&project.config)
        .map_err(|e| e.to_string())?;
    let mut world_bounds = map.bounding_box().map(|b| [b.x, b.y, b.z, b.w]);
    for object in &map.geometry_objects {
        if let Some(b) = object.bbox() {
            let old = world_bounds.unwrap_or([b.min.x, b.min.y, 0., 0.]);
            let x = old[0].min(b.min.x);
            let y = old[1].min(b.min.y);
            world_bounds = Some([
                x,
                y,
                (old[0] + old[2]).max(b.max.x) - x,
                (old[1] + old[3]).max(b.max.y) - y,
            ]);
        }
    }
    let bounds = request
        .bounds
        .map(|[x, y, w, h]| [x as f32, y as f32, w as f32, h as f32])
        .or(world_bounds)
        .unwrap_or([-4., -4., 8., 8.]);
    if bounds[2] <= 0. || bounds[3] <= 0. || bounds[2] > 256. || bounds[3] > 256. {
        return Err("Preview bounds require positive extents up to 256 units".into());
    }
    let center = request
        .camera
        .as_ref()
        .map(|c| Vec3::from(c.center))
        .unwrap_or(Vec3::new(
            bounds[0] + bounds[2] / 2.,
            0.,
            bounds[1] + bounds[3] / 2.,
        ));
    if ![center.x, center.y, center.z]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
    {
        return Err("Invalid camera center".into());
    }
    let mut widget = GameWidget::new();
    widget.follow_player_camera = false;
    widget.clamp_camera_to_map = false;
    widget.rect.width = width as f32;
    widget.rect.height = height as f32;
    widget.buffer = TheRGBABuffer::new(TheDim::sized(width as i32, height as i32));
    widget.grid_size = request.zoom.map(|z| z.clamp(0.1, 256.)).unwrap_or_else(|| {
        if view == "2d_iso" || (view == "2d" && map.camera == rusterix::MapCamera::TwoDIso) {
            (width as f32 / (bounds[2] + bounds[3]))
                .min(height as f32 / ((bounds[2] + bounds[3]) * 0.5 + 4.))
        } else {
            (width as f32 / bounds[2]).min(height as f32 / bounds[3])
        }
    });
    widget.player_pos = Vec2::new(center.x, center.z);
    widget.set_camera_mode(
        if view == "2d_iso" || (view == "2d" && map.camera == rusterix::MapCamera::TwoDIso) {
            PlayerCamera::D2Iso
        } else if view == "2d" {
            PlayerCamera::D2
        } else if view == "iso" {
            PlayerCamera::D3Iso
        } else {
            PlayerCamera::D3FirstP
        },
    );
    if view == "3d" {
        let mut camera = D3FirstPCamera::new();
        if let Some(c) = &request.camera {
            if let Some(fov) = c.fov {
                camera.fov = fov;
            }
            if let Some(pitch) = c.pitch {
                camera.pitch = pitch;
            }
        }
        let position = request
            .camera
            .as_ref()
            .and_then(|c| c.position)
            .map(Vec3::from)
            .unwrap_or(center + Vec3::new(0., 4., 6.));
        if ![position.x, position.y, position.z]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            || (position - center).magnitude_squared() < 0.001
        {
            return Err("Invalid perspective camera position".into());
        }
        camera.set_parameter_vec3("position", position);
        camera.set_parameter_vec3("center", center);
        widget.camera_d3 = Box::new(camera);
    } else {
        let mut camera = D3IsoCamera::new();
        if let Some(c) = &request.camera {
            if let Some(azimuth) = c.azimuth {
                camera.azimuth_deg = azimuth;
            }
            if let Some(elevation) = c.elevation {
                camera.elevation_deg = elevation;
            }
        }
        camera.scale = request
            .camera
            .as_ref()
            .and_then(|c| c.scale)
            .unwrap_or(bounds[2].max(bounds[3]) * 0.6)
            .clamp(0.25, 256.);
        camera.set_parameter_vec3("center", center);
        widget.camera_d3 = Box::new(camera);
    }
    // The real GameWidget streams chunks incrementally. Drain preparation to
    // completion instead of returning a half-loaded or time-delayed screenshot.
    widget.stream_load_radius_chunks =
        ((bounds[2].max(bounds[3]) / widget.scenemanager.chunk_size() as f32).ceil() as i32 + 1)
            .clamp(2, 16);
    widget.stream_prefetch_radius_chunks = widget.stream_load_radius_chunks;
    widget.chunk_build_budget_near = 256;
    widget.chunk_build_budget_far = 256;
    let frame = request.animation_frame.unwrap_or(0) as usize;
    let mut ready = false;
    for _ in 0..128 {
        widget.prepare_frame(
            &map,
            &project.time,
            frame,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        if !widget.scenemanager.is_busy() {
            ready = true;
            break;
        }
    }
    if !ready {
        return Err("Scene preparation did not finish within the preview chunk budget".into());
    }
    widget.apply_entities(&map, &runtime.assets, frame, &mut runtime.scene_handler);
    let mut pixels = vec![0; width as usize * height as usize * 4];
    runtime
        .scene_handler
        .vm
        .render_frame(&mut pixels, width, height);
    let png = TheRGBABuffer::from(pixels, width, height)
        .to_png()
        .map_err(|e| e.to_string())?;
    Ok(
        json!({"region":project.regions[index].id,"view":view,"width":width,"height":height,"animation_frame":frame,"ready":true,"renderer":"SceneVM","camera":request.camera,"zoom":request.zoom,"png_base64":Editor::scepter_base64_encode(&png)}),
    )
}
