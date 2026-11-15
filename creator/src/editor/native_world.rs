//! Main authoring-world viewport on the host device. CPU source/controller
//! bridges remain explicit while the remaining Creator owners are extracted.
use super::*;
#[path = "native_world_paste.rs"]
mod paste;
#[path = "native_world_source.rs"]
mod source;
#[path = "native_world_stamps.rs"]
mod stamps;
#[path = "native_world_paint.rs"]
mod paint;
#[cfg(not(target_arch = "wasm32"))]
#[path = "native_world_cpu_jobs.rs"]
mod cpu_jobs;
#[path = "native_world_navigation.rs"]
mod navigation;
#[path = "native_world_visibility.rs"]
mod visibility;
#[path = "native_world_pixels.rs"]
mod pixels;
#[path = "native_world_speech.rs"]
mod speech;
#[path = "native_world_scope.rs"]
mod scope;
#[path = "native_world_custom.rs"]
mod custom;
#[path = "native_world_bake.rs"]
mod bake;
#[path = "native_world_bake_job.rs"]
mod bake_job;
use crate::editor_overlay::panel::PanelScope;
use crate::editor_overlay::procedural_gpu::GpuIcons;
use crate::hud::{Hud, HudInputs, HudStyle};
use crate::tools::overlay_frame::{
    ScopedToolOverlayEvent, ToolOverlayInputState, ToolOverlayRoute,
};
use crate::tools::{ToolOverlayFrame, ToolOverlayInputs, ToolOverlayTarget};
use crate::viewcube::{CubeFrame, CubePick, CubeTarget, ViewCubeFrames};
use eldiron_presentation::{EditorPick, SceneViews, ViewId, ViewImage};
use rusterix::D3Camera;
use rusterix::scene_build_index::SceneBuildSnapshot;
use std::sync::Arc;
use theframework::paint::{FrameReceipt, PaintList, PaintOp, Rect, Sampling};
use theframework::render_wgpu::{UiFrameImages, UiImageCache};

#[derive(Clone, PartialEq)]
struct AssetKey {
    source: u64,
    avatars: u64,
    tiles: u64,
    prefabs: u64,
    config: String,
    palette: ThePalette,
    material: u64,
    appearance: u64,
    geometry: u64,
}
impl AssetKey {
    fn same_atlas(&self, other: &Self) -> bool {
        self.tiles == other.tiles
            && self.config == other.config
            && self.palette == other.palette
            && self.material == other.material
            && self.appearance == other.appearance
            && self.geometry == other.geometry
    }
}
#[derive(Clone, PartialEq)]
struct Binding {
    document: Uuid,
    runtime: Option<Uuid>,
    paste: Option<paste::Stamp>,
    detail: scope::DetailScope,
    custom: Option<custom::Description>,
    view: TheId,
    tool: Uuid,
    context: ProjectContext,
    size: (usize, usize),
    map: Uuid,
    mode: EditorViewMode,
    camera: MapCamera,
    geometry_config: String,
    guide_properties: ValueContainer,
    density: u32,
    assets: AssetKey,
    slots: Option<Vec<(String, Option<rusterix::PixelSource>)>>,
    style: HudStyle,
}
impl Binding {
    fn capture_compatible(&self, next: &Self) -> bool {
        self.document == next.document
            && self.runtime == next.runtime
            && self.paste.is_some() == next.paste.is_some()
            && self.detail == next.detail
            && self.custom == next.custom
            && self.view == next.view
            && self.tool == next.tool
            && self.context == next.context
            && self.map == next.map
            && self.mode == next.mode
            && self.camera == next.camera
            && self.size == next.size
            && self.density == next.density
            && self.guide_properties == next.guide_properties
    }
    fn target(&self, epoch: u64) -> ToolOverlayTarget {
        ToolOverlayTarget {
            document: self.document,
            tool: self.tool,
            view: self.view.uuid,
            revision: epoch,
            context: self.context,
            size: self.size,
            visible: true,
        }
    }
    fn cube_target(&self, epoch: u64) -> CubeTarget {
        CubeTarget {
            view: self.view.uuid,
            scope: PanelScope {
                document: self.document,
                tool: self.tool,
                revision: epoch,
            },
            size: self.size,
            visible: self.mode == EditorViewMode::Orbit,
        }
    }
}
#[derive(Clone, PartialEq)]
struct SourceStamp {
    map: Uuid,
    changed: u32,
    grid: u32,
    subdivisions: u32,
    offset: [u32; 2],
    camera: MapCamera,
    iso_height: u32,
}
impl SourceStamp {
    fn new(map: &Map) -> Self {
        Self {
            map: map.id,
            changed: map.changed,
            grid: map.grid_size.to_bits(),
            subdivisions: map.subdivisions.to_bits(),
            offset: [map.offset.x.to_bits(), map.offset.y.to_bits()],
            camera: map.camera,
            iso_height: map
                .properties
                .get_float_default("iso_edit_height", 0.)
                .to_bits(),
        }
    }
}
#[derive(Clone, PartialEq)]
struct CameraStamp([u32; 16]);
impl CameraStamp {
    fn new(camera: &dyn D3Camera, size: (usize, usize)) -> Self {
        Self::from_scene(camera.as_scenevm_camera_for_surface(size.0 as f32, size.1 as f32))
    }
    fn from_scene(camera: scenevm::Camera3D) -> Self {
        Self(
            [
                camera.pos.x,
                camera.pos.y,
                camera.pos.z,
                camera.forward.x,
                camera.forward.y,
                camera.forward.z,
                camera.right.x,
                camera.right.y,
                camera.right.z,
                camera.up.x,
                camera.up.y,
                camera.up.z,
                camera.vfov_deg,
                camera.ortho_half_h,
                camera.near,
                camera.far,
            ]
            .map(f32::to_bits),
        )
    }
}
/// Opaque acknowledged authoring-view input scope; asset refreshes do not
/// replace the CPU held-key owner, while camera/document/layout changes revoke it.
#[derive(Clone)]
pub(super) struct GameInputFrame {
    binding: Binding,
    camera: Option<CameraStamp>,
}
impl GameInputFrame {
    pub(super) fn owns_runtime(&self, runtime: Uuid) -> bool { self.binding.runtime == Some(runtime) }
    pub(super) fn view(&self) -> &TheId {
        &self.binding.view
    }
}
impl PartialEq for GameInputFrame {
    fn eq(&self, other: &Self) -> bool {
        self.binding.capture_compatible(&other.binding) && self.camera == other.camera
    }
}
#[derive(PartialEq)]
struct SceneInputKey {
    detail: scope::DetailScope,
    custom_content: Option<u64>,
    paste: Option<paste::Stamp>,
    terrain_paint: Option<u64>,
    camera: Option<CameraStamp>,
    stamp: SourceStamp,
    size: (usize, usize),
    tool: MapToolType,
    hover: (Option<u32>, Option<u32>, Option<u32>),
    cursor: Option<Vec2<f32>>,
    vertices: Vec<u32>,
    linedefs: Vec<u32>,
    sectors: Vec<u32>,
    entity: Option<Uuid>,
    rectangle: Option<(Vec2<f32>, Vec2<f32>)>,
    polygon: Vec<u32>,
    line: (Option<Vec2<f32>>, Option<Vec2<f32>>),
    grid: bool,
    hour: u32,
    animation: usize,
    preview_character_setup: bool,
    canvas: Option<[RGBA; 3]>,
    build: ValueContainer,
    runtime: rusterix::server::message::RuntimeRenderState,
}
impl SceneInputKey {
    fn same_private_scope(&self, other: &Self) -> bool {
        // Temporal inputs belong to the frozen request. Enumerate every other
        // field so adding a new scope input requires updating this guard.
        let Self { hour: _, animation: _, runtime: _,
            detail, custom_content, paste, terrain_paint, camera, stamp, size, tool, hover, cursor, vertices, linedefs, sectors, entity, rectangle, polygon, line, grid, canvas, build, preview_character_setup } = self;
        detail == &other.detail
            && custom_content == &other.custom_content
            && paste == &other.paste
            && terrain_paint == &other.terrain_paint
            && camera == &other.camera
            && stamp == &other.stamp
            && size == &other.size
            && tool == &other.tool
            && hover == &other.hover
            && cursor == &other.cursor
            && vertices == &other.vertices
            && linedefs == &other.linedefs
            && sectors == &other.sectors
            && entity == &other.entity
            && rectangle == &other.rectangle
            && polygon == &other.polygon
            && line == &other.line
            && grid == &other.grid
            && canvas == &other.canvas
            && build == &other.build
            && preview_character_setup == &other.preview_character_setup
    }
}

struct WorldGpu {
    registry: u64,
    scope: Option<(Uuid, ProjectContext, Uuid, Option<Uuid>)>,
    scenes: SceneViews,
    view: ViewId,
    configured: Option<AssetKey>,
    scene_key: Option<SceneInputKey>,
    projection_paste: Option<paste::Stamp>,
    scene_source: Option<Arc<SceneBuildSnapshot>>,
    guides: rusterix::scenebuilder::d2preview::D2PreviewBuilder,
    guide_scene: rusterix::Scene,
    paint_ready: bool,
    stamps_ready: bool,
    icons: GpuIcons,
    images: UiImageCache,
    last_root_images: PaintList,
    custom: custom::Gpu,
    bake: bake::Gpu,
    pixels: pixels::Gpu,
}
impl WorldGpu {
    fn new(images: &renderkit::ImageRegistry) -> Result<Self, String> {
        let mut scenes = SceneViews::new(images, scenevm::SharedAtlas::new(1024, 1024));
        let view = ViewId::new();
        scenes.create(view, (0, 0)).map_err(|e| e.to_string())?;
        Ok(Self {
            registry: images.owner_id(),
            scope: None,
            scenes,
            view,
            configured: None,
            scene_key: None,
            projection_paste: None,
            scene_source: None,
            guides: Default::default(),
            guide_scene: Default::default(),
            paint_ready: true,
            stamps_ready: true,
            icons: GpuIcons::new(images.context().clone(), 16 * 1024 * 1024, 512)
                .map_err(|e| e.to_string())?,
            images: UiImageCache::default(),
            last_root_images: PaintList::default(),
            custom: Default::default(),
            bake: Default::default(),
            pixels: Default::default(),
        })
    }
    fn query(
        &mut self,
        frame: &NativeWorldFrame,
        at: Vec2<i32>,
        server: &CreatorContext,
        rectangle: Option<(Vec2<f32>, Vec2<f32>, scenevm::GeoId)>,
        capture_surface: bool,
    ) -> Option<(EditorPick, crate::tools::ToolViewInput)> {
        let pick = self.scenes.pick_editor(
            frame.output,
            [
                at.x as f32 / frame.binding.size.0 as f32,
                at.y as f32 / frame.binding.size.1 as f32,
            ],
            matches!(
                server.shared.curr_map_tool_type,
                MapToolType::Vertex
                    | MapToolType::Linedef
                    | MapToolType::Sector
                    | MapToolType::Effects
            ),
            server.shared.curr_map_tool_type != MapToolType::Sector,
        )?;
        let rectangle = match rectangle {
            Some((top, bottom, kind)) => Some((
                top,
                bottom,
                self.scenes
                    .pick_editor_rectangle(frame.output, top, bottom, kind)?,
            )),
            None => None,
        };
        let paint = if server.shared.curr_map_tool_type == MapToolType::IsoPaint {
            let uv = [
                at.x as f32 / frame.binding.size.0 as f32,
                at.y as f32 / frame.binding.size.1 as f32,
            ];
            let surface = self.scenes.pick_editor_paint(frame.output, uv)?;
            let capture = if capture_surface {
                Some(self.scenes.capture_editor_paint(frame.output)?.buffer)
            } else {
                None
            };
            Some(crate::tools::ToolPaintInput {
                surface: surface.surface,
                brush_transform: surface.brush_transform,
                capture,
            })
        } else {
            None
        };
        let input = crate::tools::ToolViewInput {
            camera: Some(pick.camera),
            ray: Some(pick.ray?),
            isometric_item: None,
            base: pick.base.map(|(geo, at, depth, _)| (geo, at, depth)),
            dynamic: pick.dynamic,
            paint_geo: pick.paint_geo,
            paint,
            rectangle,
        };
        Some((pick, input))
    }
    fn finish(
        &mut self,
        images: &mut renderkit::ImageRegistry,
        root: Option<&PaintList>,
    ) -> Result<(), String> {
        if images.owner_id() != self.registry {
            return Ok(());
        }
        let mut retained = PaintList::default();
        if root.is_none() {
            for image in self
                .last_root_images
                .image_handles()
                .filter(|id| images.get(*id).is_some())
            {
                retained.image(image, Rect::new(0., 0., 1., 1.), Sampling::Nearest);
            }
        }
        let list = root.unwrap_or(&retained);
        self.images
            .finish_frame(images, list)
            .map_err(|e| e.to_string())?;
        self.icons
            .finish_frame(images, list)
            .map_err(|e| e.to_string())?;
        if let Some(root) = root {
            // Retain only exact resource pins for cancellation, not another copy
            // of the application's entire command/glyph stream.
            let mut pins = PaintList::default();
            let mut seen = std::collections::HashSet::new();
            for image in root.image_handles().filter(|id| seen.insert(*id)) {
                pins.image(image, Rect::new(0., 0., 1., 1.), Sampling::Nearest);
            }
            self.last_root_images = pins;
        }
        Ok(())
    }
}
pub(super) struct NativeWorldFrame {
    binding: Binding,
    target: ToolOverlayTarget,
    stamp: SourceStamp,
    custom_content: Option<u64>,
    source: Arc<SceneBuildSnapshot>,
    ready: bool,
    waiting_cpu_sources: bool,
    camera: Option<CameraStamp>,
    output: ViewImage,
    baked: bool,
    paste_ignored: Option<Arc<FxHashSet<scenevm::GeoId>>>,
    cube: Option<CubeFrame>,
    isometric: Option<Arc<eldiron_presentation::isometric::IsometricWorldTargets>>,
    overlay: Arc<ToolOverlayFrame>,
    pub(super) receipts: Vec<FrameReceipt>,
    pub(super) receipt: FrameReceipt,
}
#[cfg(test)]
impl NativeWorldFrame {
    pub(super) fn ready_for_test(&self) -> bool { self.ready }
}

pub(crate) struct NativeWorld {
    source: Option<source::Publication>,
    stamps: stamps::Owner,
    uv: paint::Owner,
    paint_capture: paint::Capture,
    bake_jobs: bake_job::Jobs,
    paste: Option<paste::Preview>,
    binding: Option<Binding>,
    epoch: u64,
    serial: u64,
    receipt_owner: u64,
    presented: Option<Arc<NativeWorldFrame>>,
    overlay_input: ToolOverlayInputState,
    world_capture: Option<WorldGesture>,
    retry_click: Option<RetryClick>,
    cube: ViewCubeFrames,
    cube_capture: Option<CubeCapture>,
    relative_navigation: Option<Navigation>,
    events: std::collections::VecDeque<ScopedToolOverlayEvent>,
    published: Option<(TheId, Arc<PaintList>)>,
    gpu: Option<WorldGpu>,
    speech: speech::WorldSpeech,
}
/// An already accepted down which changed preview geometry before authoring.
/// Keep its physical continuation bounded until the replacement is presented.
struct RetryClick {
    binding: Binding,
    stamp: SourceStamp,
    camera: Option<CameraStamp>,
    modifiers: (bool, bool, bool, bool),
    events: std::collections::VecDeque<TheEvent>,
    started: bool,
}
struct WorldGesture {
    binding: Binding,
    point: Vec2<i32>,
    query: Option<crate::tools::ToolViewInput>,
}
struct Navigation {
    binding: Binding,
    pointer: [f32; 2],
}
struct CubeCapture {
    binding: Binding,
    origin: Vec2<i32>,
    face: crate::viewcube::ViewCubeFace,
    orbiting: bool,
}
impl Default for NativeWorld {
    fn default() -> Self {
        Self {
            source: None,
            stamps: Default::default(),
            uv: Default::default(),
            paint_capture: Default::default(),
            bake_jobs: Default::default(),
            paste: None,
            binding: None,
            epoch: 0,
            serial: 0,
            receipt_owner: theframework::paint::new_receipt_owner(),
            presented: None,
            overlay_input: Default::default(),
            world_capture: None,
            retry_click: None,
            cube: Default::default(),
            cube_capture: None,
            relative_navigation: None,
            events: Default::default(),
            published: None,
            gpu: None,
            speech: Default::default(),
        }
    }
}
impl NativeWorld {
    fn frame_is_fresh(
        &self,
        frame: &NativeWorldFrame,
        map: &Map,
        camera: &Option<CameraStamp>,
    ) -> bool {
        frame.ready
            && frame.camera == *camera
            && frame.stamp == SourceStamp::new(map)
            && frame.custom_content == frame.binding.custom.as_ref().map(|_| custom::content(map))
            && if let Some(targets) = &frame.isometric {
                self.gpu
                    .as_ref()
                    .and_then(|gpu| gpu.scenes.isometric_targets(frame.output))
                    .is_some_and(|current| Arc::ptr_eq(targets, &current))
            } else if frame.binding.paste.is_some() {
                self.paste.as_ref().is_some_and(|preview| {
                    preview.matches(&frame.binding, map) && preview.ready()
                        && preview.snapshot().is_some_and(|source| Arc::ptr_eq(&source, &frame.source))
                })
            } else {
                self.source.as_ref().is_some_and(|source| {
                    source.matches(&frame.binding, map) && source.ready()
                        && Arc::ptr_eq(&frame.source, &source.snapshot())
                })
            }
    }
    fn bind(
        &mut self,
        next: Option<Binding>,
        ui: &mut TheUI,
        camera: &mut EditCamera,
        creator_tools: &CreatorTools,
    ) {
        if self.binding == next {
            return;
        }
        // Source/material/style refreshes revoke fresh picking, but do not
        // abandon a draft owned by the same document/tool/view transform.
        let capture = self.world_capture.take().and_then(|mut owner| {
            next.as_ref()
                .filter(|next| owner.binding.capture_compatible(next))
                .map(|next| {
                    owner.binding = next.clone();
                    owner
                })
        });
        let retry = self.retry_click.take().filter(|retry| {
            next.as_ref()
                .is_some_and(|next| retry.binding.capture_compatible(next))
        });
        let relative_navigation = self.relative_navigation.take().filter(|owner| {
            next.as_ref().is_some_and(|next| owner.binding.capture_compatible(next))
        });
        let had_cube_capture = self.cube_capture.is_some();
        let cube_capture = self.cube_capture.take().and_then(|mut owner| {
            next.as_ref()
                .filter(|next| owner.binding.capture_compatible(next))
                .map(|next| {
                    owner.binding = next.clone();
                    owner
                })
        });
        if had_cube_capture && cube_capture.is_none() {
            camera.reset_mouse_tracking();
        }
        self.invalidate(ui, camera, creator_tools);
        self.binding = next;
        self.world_capture = capture;
        self.retry_click = retry;
        self.cube_capture = cube_capture;
        self.relative_navigation = relative_navigation;
        if self.binding.is_some() {
            let mut tools = creator_tools.write();
            tools.native_world_input = true;
            tools.get_current_tool().set_native_overlay_input(true);
        }
    }
    pub(super) fn invalidate(
        &mut self,
        ui: &mut TheUI,
        camera: &mut EditCamera,
        creator_tools: &CreatorTools,
    ) {
        if let Some((id, list)) = self.published.take()
            && let Some(view) = ui.get_render_view(&id.name)
            && view.id().uuid == id.uuid
        {
            view.clear_paint_list(&list);
        }
        if self.binding.take().is_some() {
            let mut tools = creator_tools.write();
            tools.native_world_input = false;
            for tool in &mut tools.game_tools {
                tool.set_native_overlay_input(false);
                tool.set_view_input(None);
                tool.invalidate_overlay();
            }
        }
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("world binding epoch exhausted");
        self.presented = None;
        self.speech.invalidate_geometry();
        self.overlay_input.invalidate();
        self.world_capture = None;
        self.relative_navigation = None;
        self.retry_click = None;
        self.cube.invalidate();
        if self.cube_capture.take().is_some() {
            camera.reset_mouse_tracking();
        }
        self.events.clear();
    }
    pub(super) fn device_reset(
        &mut self,
        ui: &mut TheUI,
        camera: &mut EditCamera,
        creator_tools: &CreatorTools,
    ) {
        self.invalidate(ui, camera, creator_tools);
        // The replaced host registry owns old handles and teardown.
        self.gpu = None;
    }
}
fn slots(
    map: &Map,
    server: &CreatorContext,
) -> Option<Vec<(String, Option<rusterix::PixelSource>)>> {
    crate::actions::named_geometry_hud_material_slots_for_selection(map)
        .or_else(|| crate::actions::prefab_hud_material_slots_for_selected_instances(map, &server.reference))
        .or_else(|| {
            let actions = server.actions.read();
            actions
                .get_action_by_id(server.shared.curr_action_id?)?
                .hud_material_slots(map, server)
        })
        .map(|slots| slots.into_iter().map(|s| (s.label, s.source)).collect())
}
/// Project the acknowledged scene query into the existing authoring controller
/// state. A miss clears previous hits while retaining its valid ground ray.
fn apply_pick(pick: EditorPick, map: &Map, server: &mut CreatorContext) {
    server.shared.hover_cursor_3d = None;
    server.shared.hover_ray_origin_3d = pick.ray.map(|ray| ray.0);
    server.shared.hover_ray_dir_3d = pick.ray.map(|ray| ray.1);
    server.shared.hover_surface = None;
    server.shared.hover_surface_hit_pos = None;
    server.shared.hover_surface_normal = None;
    server.shared.geo_hit = None;
    server.shared.geo_hit_pos = Vec3::zero();
    if let Some((geo, at, _)) = pick.gizmo {
        server.shared.geo_hit = Some(geo);
        server.shared.geo_hit_pos = at;
        return;
    }
    let mut base_preferred = false;
    if let Some((geo, at, _, normal)) = pick.base {
        server.shared.hover_cursor_3d = Some(at);
        server.shared.hover_surface_normal = Some(normal);
        server.shared.hover_surface_hit_pos = Some(at);
        server.shared.hover_surface = map
            .surfaces
            .values()
            .filter_map(|surface| {
                let length = surface.plane.normal.magnitude();
                (length > 1e-6).then(|| {
                    (
                        surface,
                        ((at - surface.plane.origin).dot(surface.plane.normal / length)).abs(),
                    )
                })
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(surface, _)| surface.clone());
        base_preferred = matches!(
            server.shared.curr_map_tool_type,
            MapToolType::IsoPaint
                | MapToolType::Rect
                | MapToolType::Terrain
                | MapToolType::CollisionProbe
        ) || (server.shared.curr_map_tool_type == MapToolType::Sector
            && server.shared.geometry_edit_mode != GeometryEditMode::Detail)
            || ((server.shared.block_tool_active
                || matches!(
                    server.shared.curr_map_tool_type,
                    MapToolType::Selection
                        | MapToolType::Vertex
                        | MapToolType::Linedef
                        | MapToolType::Sector
                ))
                && matches!(geo, scenevm::GeoId::GeometryObject(_)));
    }
    let hit = if base_preferred || server.shared.curr_map_tool_type == MapToolType::Sector {
        pick.base.map(|(geo, at, depth, _)| (geo, at, depth))
    } else {
        pick.guide
    };
    if let Some((geo, at, _)) = hit {
        server.shared.geo_hit = Some(geo);
        server.shared.geo_hit_pos = at;
    } else if let Some((origin, direction)) = pick.ray {
        if direction.y.abs() > 1e-6 {
            let distance = -origin.y / direction.y;
            if distance >= 0. {
                server.shared.hover_cursor_3d = Some(origin + direction * distance);
            }
        }
    }
}
fn snap_cube(
    face: crate::viewcube::ViewCubeFace,
    ctx: &mut TheContext,
    camera: &mut EditCamera,
) {
    crate::viewcube::ViewCube::snap_camera(&mut camera.orbit_camera, face);
    camera.reset_mouse_tracking();
    ctx.ui.send(TheEvent::SetStatusText(
        TheId::empty(),
        fl!("status_viewcube_camera_aligned"),
    ));
    ctx.ui.invalidate_scene();
}
fn isometric_input(
    world: &NativeWorld,
    frame: &NativeWorldFrame,
    at: Vec2<i32>,
) -> Option<crate::tools::ToolViewInput> {
    let targets = frame.isometric.as_ref()?;
    let current = world.gpu.as_ref()?.scenes.isometric_targets(frame.output)?;
    if !Arc::ptr_eq(targets, &current) {
        return None;
    }
    Some(crate::tools::ToolViewInput {
        camera: None,
        ray: None,
        isometric_item: Some(targets.editor_item([
            at.x as f32 / frame.binding.size.0 as f32,
            at.y as f32 / frame.binding.size.1 as f32,
        ])),
        base: None,
        dynamic: None,
        paint_geo: None,
        paint: None,
        rectangle: None,
    })
}
impl Editor {
    pub(super) fn native_world_waiting(&self) -> bool {
        let world = &self.native_viewport.world;
        let waits_for_wake = world.gpu.as_ref().is_some_and(|gpu| {
            !world.paint_capture.pending() && (gpu.paint_ready || world.uv.waits_for_wake())
                && (gpu.stamps_ready || world.stamps.waits_for_wake())
        });
        self.native_world_owns_viewport() && world.binding.is_some()
            && (world.paint_capture.pending() || world.presented.as_ref().map_or(!world.stamps.failed() && !world.uv.failed(), |frame| {
                !frame.ready && (!frame.waiting_cpu_sources || !waits_for_wake)
            }) || world.gpu.as_ref().is_some_and(|gpu| gpu.bake.waiting()))
    }

    fn bind_native_world(&mut self, next: Option<Binding>, ui: &mut TheUI, ctx: &mut TheContext) {
        // Paint's surface cache and selection plane belong to the down-camera.
        // Navigation cannot reinterpret them using a newly presented camera.
        let paint_camera_changed = self.server_ctx.shared.curr_map_tool_type == MapToolType::IsoPaint
            && self
                .native_viewport
                .world
                .world_capture
                .as_ref()
                .and_then(|owner| owner.query.as_ref()?.camera)
                .is_some_and(|old| {
                    next.as_ref().and_then(|binding| {
                        self.native_world_camera()
                            .map(|camera| CameraStamp::new(camera.as_ref(), binding.size))
                    }) != Some(CameraStamp::from_scene(old))
                });
        if paint_camera_changed {
            self.cancel_native_world_gesture(ui, ctx);
        }
        if self
            .native_viewport
            .world
            .world_capture
            .as_ref()
            .is_some_and(|owner| {
                next.as_ref()
                    .is_none_or(|next| !owner.binding.capture_compatible(next))
            })
        {
            self.cancel_native_world_gesture(ui, ctx);
        }
        self.native_viewport.world.bind(next, ui, &mut self.server_ctx.camera, &self.server_ctx.tools);
    }
    pub(crate) fn cancel_native_world_gesture(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.native_viewport.world.relative_navigation = None;
        self.native_viewport.world.retry_click = None;
        let Some(gesture) = self.native_viewport.world.world_capture.take() else {
            return;
        };
        let owner = gesture.binding;
        if self.play_session.as_ref().map(|session| session.pump().session_id()) != owner.runtime
            || self.documents.active_id() != Some(owner.document)
        {
            return;
        }
        if owner.context.is_prefab() {
            self.server_ctx.docks.write().cancel_native_view_gesture(
                owner.context,
                ui,
                ctx,
                &mut self.documents.project,
                &mut self.server_ctx,
            );
        }
        if self.server_ctx.shared.pc != owner.context
            || scope::map(&self.documents.project, &self.server_ctx)
                .is_none_or(|map| map.id != owner.map)
        {
            return;
        }
        let mut tools = self.server_ctx.tools.write();
        if tools.get_current_tool().id().uuid != owner.tool {
            return;
        }
        tools.get_current_tool().set_view_input(gesture.query);
        // Cancel through the gesture's original projection. Camera switches
        // may already have updated the current view before this barrier runs.
        let mode = self.server_ctx.shared.editor_view_mode;
        let navigation = scope::map(&self.documents.project, &self.server_ctx)
            .map(|map| (map.camera, map.offset, map.grid_size, map.subdivisions));
        self.server_ctx.shared.editor_view_mode = owner.mode;
        if let Some(map) = scope::map_mut(&mut self.documents.project, &self.server_ctx) {
            map.camera = owner.camera;
        }
        tools.cancel_native_world_gesture(ui, ctx, &mut self.documents.project, &mut self.server_ctx);
        tools.get_current_tool().set_view_input(None);
        self.server_ctx.shared.editor_view_mode = mode;
        let mut projection_changed = false;
        if let (Some((camera, offset, grid, subdivisions)), Some(map)) =
            (navigation, scope::map_mut(&mut self.documents.project, &self.server_ctx))
        {
            projection_changed = map.camera != camera;
            map.camera = camera;
            map.offset = offset;
            map.grid_size = grid;
            map.subdivisions = subdivisions;
        }
        drop(tools);
        if projection_changed {
            crate::utils::scenemanager_render_map(&self.documents.project, &self.server_ctx);
        }
    }
    pub(crate) fn invalidate_native_world_viewport(&mut self, ui: &mut TheUI) {
        self.native_viewport.world.invalidate(ui, &mut self.server_ctx.camera, &self.server_ctx.tools);
    }
    pub(crate) fn native_world_owns_viewport(&self) -> bool {
        if !self.native_viewport.enabled
            || self.native_display_owns_viewport()
            || self.server_ctx.shared.game_mode
            || self.server_ctx.shared.text_game_mode
            || self.server_ctx.shared.world_mode
            || self.server_ctx.shared.profile_view.is_some()
        {
            return false;
        }
        let Some(map) = scope::map(&self.documents.project, &self.server_ctx) else {
            return false;
        };
        if !(scope::uses_region(&self.server_ctx) || self.server_ctx.shared.pc.is_prefab() || self.server_ctx.shared.pc.is_screen()
            || custom::Description::current(map, &self.server_ctx).is_some()) { return false; }
        let tools = self.server_ctx.tools.read();
        if tools.editor_mode
            || !matches!(
                tools.current_game_tool_command_id(),
                Some(
                    "tool.vertex"
                        | "tool.linedef"
                        | "tool.sector"
                        | "tool.entity"
                        | "tool.geometry"
                        | "tool.blocks"
                        | "tool.terrain"
                        | "tool.rect"
                        | "tool.iso_paint"
                        | "tool.effects"
                        | "tool.tile_picker"
                )
            )
        {
            return false;
        }
        if !self.server_ctx.shared.pc.is_prefab()
            && matches!(
                tools.current_game_tool_command_id(),
                Some("tool.effects" | "tool.tile_picker")
            )
        {
            return false;
        }
        let bridge = self.server_ctx.reference.read();
        // The shared CPU pump publishes live actors into a separate projection.
        // Native orthographic bakes own a separate frozen producer scene.
        let owned_play = self.play_session.as_ref().is_some_and(|play| {
            self.documents.active_id() == Some(play.document)
        });
        (bridge.server.state == rusterix::ServerState::Off || owned_play)
            && (owned_play || bridge.client.messages_to_draw.is_empty())

    }
    pub(super) fn native_world_gesture_active(&self) -> bool {
        self.native_viewport.world.world_capture.is_some()
    }
    pub(super) fn native_game_input_frame(
        &self,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<GameInputFrame> {
        let binding = self.native_world_binding(ui, ctx)?;
        let camera = self
            .native_world_camera()
            .map(|camera| CameraStamp::new(camera.as_ref(), binding.size));
        let frame = self.native_viewport.world.presented.as_ref()?;
        if !frame.ready || !frame.binding.capture_compatible(&binding) || frame.camera != camera {
            return None;
        }
        Some(GameInputFrame { binding, camera })
    }
    fn native_world_binding(&self, ui: &mut TheUI, ctx: &TheContext) -> Option<Binding> {
        if !self.native_world_owns_viewport() || crate::geometry_editor::is_visible(ui) {
            return None;
        }
        let map = scope::map(&self.documents.project, &self.server_ctx)?;
        let document = self.documents.active_id()?;
        let tool = self.server_ctx.tools.write().get_current_tool().id().uuid;
        let view = crate::utils::map_editor_render_view(ui, &self.server_ctx)?;
        let dim = *view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return None;
        }
        let view = view.id().clone();
        if self.native_hidden_viewport(ui, &view.name) { return None; }
        let slots = slots(map, &self.server_ctx);
        let bridge = self.server_ctx.reference.read();
        let assets = self.play_session.as_ref().map(|play| play.assets(&bridge.assets)).unwrap_or(&bridge.assets);
        Some(Binding {
            document,
            runtime: self.play_session.as_ref().map(|session| session.pump().session_id()),
            paste: paste::Stamp::current(&self.server_ctx),
            detail: scope::DetailScope::current(&self.server_ctx),
            custom: custom::Description::current(map, &self.server_ctx),
            view,
            tool,
            context: self.server_ctx.shared.pc,
            size: (dim.width as usize, dim.height as usize),
            map: map.id,
            mode: self.server_ctx.shared.editor_view_mode,
            camera: map.camera,
            geometry_config: self.native_world_geometry_config(),
            guide_properties: self.build_values.clone(),
            density: ctx.scale_factor.to_bits(),
            style: HudStyle::current(&self.server_ctx),
            slots,
            assets: self.native_world_asset_key(assets),
        })
    }
    /// Advance only pending particle production at the editor's update cadence.
    /// Completed VM geometry remains queryable until the next frame is prepared.
    pub(crate) fn tick_native_world_particles(&mut self) {
        if !self.native_world_owns_viewport() {
            return;
        }
        let d2 = self.server_ctx.shared.editor_view_mode == EditorViewMode::D2;
        let fx = self.server_ctx.shared.pc.is_prefab()
            && self.server_ctx.shared.curr_map_tool_type == MapToolType::Effects;
        let running = self.runtime_state() == rusterix::ServerState::Running;
        if (d2 || fx || running)
            && let Some(gpu) = &mut self.native_viewport.world.gpu
        {
            let _ = gpu.scenes.tick_editor_particles(gpu.view, d2);
        }
    }
    /// CPU navigation belongs to update, before frame preparation. The renderer
    /// receives an immutable camera snapshot and never advances navigation.
    pub(crate) fn advance_native_world_camera(&mut self, ctx: &mut TheContext) -> bool {
        if !self.native_world_owns_viewport()
            || self.server_ctx.shared.game_input_mode
            || self.server_ctx.shared.editor_view_mode == EditorViewMode::D2
            || self.server_ctx.shared.pc.is_prefab()
        {
            return false;
        }
        let Some(region) = self.documents.project.get_region_ctx_mut(&self.server_ctx.shared) else {
            return false;
        };
        let camera = &mut self.server_ctx.camera;
        let moving = self.server_ctx.shared.editor_fly_nav_active || camera.move_action.is_some();
        camera.update_action(region, &mut self.server_ctx.shared, ctx.get_time());
        region.iso_paint.ensure_baked_paint_current();
        moving
    }
    /// Publish editable paint sources after authoring dispatch, before immutable
    /// scene preparation. This replaces the skipped reference draw-time bridge.
    pub(crate) fn publish_native_world_paint_sources(&mut self) {
        if !self.native_world_owns_viewport() {
            return;
        }
        if let ProjectContext::Prefab(id) = self.server_ctx.shared.pc {
            if let Some(paint) = self.documents.project.block_prop_paint.get_mut(&id) {
                if self.server_ctx.shared.editor_view_mode == EditorViewMode::D2
                    && let Some(map) = self.documents.project.prefab_editor_map.as_ref()
                    && map.camera == MapCamera::TwoDIso
                {
                    shared::iso_paint_terrain::sync(
                        &mut self.server_ctx.reference.write().assets,
                        map.id,
                        paint,
                    );
                } else {
                    paint.ensure_baked_paint_current();
                }
            }
            return;
        }
        let Some(region) = self.documents.project.get_region_ctx_mut(&self.server_ctx.shared) else {
            return;
        };
        if self.server_ctx.shared.editor_view_mode == EditorViewMode::D2
            && region.map.camera == MapCamera::TwoDIso
        {
            shared::iso_paint_terrain::sync(
                &mut self.server_ctx.reference.write().assets,
                region.map.id,
                &mut region.iso_paint,
            );
        } else {
            region.iso_paint.ensure_baked_paint_current();
        }
    }
    fn native_world_camera(&self) -> Option<Box<dyn D3Camera>> {
        if self.server_ctx.shared.pc.is_prefab() {
            return (self.server_ctx.shared.editor_view_mode != EditorViewMode::D2).then(|| {
                Box::new(self.server_ctx.camera.orbit_camera.clone()) as Box<dyn D3Camera>
            });
        }
        let region = self.documents.project.get_region_ctx(&self.server_ctx.shared)?;
        self.server_ctx.camera
            .camera_for_view(region, &self.server_ctx.shared)
    }
    pub(super) fn prepare_native_world_viewport(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut UiFrameImages<'_>,
    ) -> Result<Option<Arc<NativeWorldFrame>>, String> {
        self.server_ctx.tools.write()
            .get_current_tool()
            .sync_overlay_sources(&self.documents.project, &self.server_ctx);
        let next = self.native_world_binding(ui, ctx);
        self.bind_native_world(next.clone(), ui, ctx);
        let Some(binding) = next else {
            self.native_viewport.world.stamps.clear();
            self.native_viewport.world.uv.clear();
            if let Some(gpu) = &mut self.native_viewport.world.gpu { gpu.scenes.cancel_surface_paint_uploads(); }
            return Ok(None);
        };
        if ctx
            .ui
            .focus
            .as_ref()
            .is_some_and(|id| id.name == binding.view.name && id.uuid != binding.view.uuid)
        {
            ctx.ui.set_focus(&binding.view);
        }
        let map = scope::map(&self.documents.project, &self.server_ctx)
            .ok_or("missing native world map")?;
        let authored_stamp = SourceStamp::new(map);
        let custom_content = binding.custom.as_ref().map(|_| custom::content(map));
        let paste = self
            .native_viewport
            .world
            .paste
            .as_ref()
            .filter(|preview| preview.matches(&binding, map));
        let paste_map = paste.map(|preview| preview.map.clone());
        let paste_source = paste.and_then(|preview| preview.snapshot());
        let paste_ready = paste.is_some_and(|preview| preview.ready());
        let paste_ignored = paste.map(|preview| preview.ignored.clone());
        let authored_map = map;
        let map = paste_map.as_deref().unwrap_or(map);
        let projected = self
            .play_session
            .as_ref()
            .map(|play| play.authoring_map(map));
        let map = projected.as_deref().unwrap_or(map);
        let camera = self.native_world_camera();
        let camera_stamp = camera
            .as_ref()
            .map(|camera| CameraStamp::new(camera.as_ref(), binding.size));
        let world = &mut self.native_viewport.world;
        if binding.mode == EditorViewMode::D2 || binding.custom.is_some() {
            world.stamps.clear();
            world.uv.clear();
        } else {
            world.stamps.scope((binding.document, binding.context, binding.map, binding.runtime));
            world.uv.scope((binding.document, binding.context, binding.map, binding.runtime));
        }
        if world
            .gpu
            .as_ref()
            .is_none_or(|gpu| gpu.registry != images.registry.owner_id())
        {
            if world.gpu.is_some() {
                world.presented = None;
                world.overlay_input.invalidate();
                world.world_capture = None;
                world.retry_click = None;
                world.cube.invalidate();
                world.cube_capture = None;
                world.events.clear();
                world.epoch = world
                    .epoch
                    .checked_add(1)
                    .expect("world binding epoch exhausted");
                self.server_ctx.tools.write()
                    .get_current_tool()
                    .invalidate_overlay();
            }
            world.gpu = Some(WorldGpu::new(images.registry)?);
        }
        let target = binding.target(world.epoch);
        let gpu = world.gpu.as_mut().unwrap();
        let scope = (
            binding.document,
            binding.context,
            binding.map,
            binding.runtime,
        );
        let atlas_changed = gpu.configured.as_ref().is_some_and(|old| {
            !old.same_atlas(&binding.assets)
        });
        if gpu.scope != Some(scope) || atlas_changed {
            // Reopened/copied documents may reuse map IDs and local revisions.
            // Neither atlas residency nor builder caches can cross that lifetime.
            gpu.scope = Some(scope);
            gpu.custom.clear(images.registry);
            gpu.bake.clear(images.registry);
            gpu.pixels.clear(images.registry);
            gpu.scenes
                .remove(gpu.view, images.registry)
                .map_err(|e| e.to_string())?;
            // This owner contains only the main world view. Retire its atlas
            // with the old source, including random guide IDs and deleted tiles.
            gpu.scenes = SceneViews::new(images.registry, scenevm::SharedAtlas::new(1024, 1024));
            gpu.scenes
                .create(gpu.view, (0, 0))
                .map_err(|e| e.to_string())?;
            gpu.configured = None;
            gpu.scene_key = None;
            gpu.projection_paste = None;
            gpu.scene_source = None;
            gpu.guides = Default::default();
            gpu.guide_scene = Default::default();
            gpu.paint_ready = true;
            gpu.stamps_ready = true;
        }
        if binding.mode == EditorViewMode::D2 || binding.custom.is_some() {
            gpu.scenes.set_stamp_paint(gpu.view, None).map_err(|e| e.to_string())?;
            gpu.stamps_ready = true;
        }
        // CPU-only bridge inputs: no legacy VM is borrowed, swapped or rendered.
        let bridge = self.server_ctx.reference.read();
        let assets = self
            .play_session
            .as_ref()
            .map(|play| play.assets(&bridge.assets))
            .unwrap_or(&bridge.assets);
        let bake_camera = camera.as_ref().map(|camera| camera.as_scenevm_camera_for_surface(
            binding.size.0 as f32, binding.size.1 as f32));
        let mut bake_owner = bake_camera.as_ref().map(|_| self.server_ctx.bake.write());
        if let Some(bake) = bake_owner.as_mut() {
            bake.sync_persisted_asset(map.id, map.orthographic_bake.as_ref());
        }
        let bake_source = bake_owner.as_ref().zip(bake_camera.as_ref())
            .and_then(|(bake, camera)| bake.presentation_source(map.id, camera));
        let baked = bake_source.is_some();
        let authored_source = world.source.as_ref().filter(|source| source.matches(&binding, authored_map));
        let source = paste_source.unwrap_or_else(|| {
            authored_source.map(|source| source.snapshot()).unwrap_or_else(|| {
                rusterix::scene_build_index::SceneBuildIndex::default().snapshot()
            })
        });
        let ready = if binding.paste.is_some() {
            paste_ready
        } else {
            authored_source.is_some_and(|source| source.ready())
        };
        if gpu.configured.as_ref() != Some(&binding.assets) {
            if gpu
                .configured
                .as_ref()
                .is_some_and(|old| old.same_atlas(&binding.assets))
            {
                gpu.scenes
                    .scene_mut(gpu.view)
                    .map_err(|e| e.to_string())?
                    .sync_prefab_particle_sprites(&assets.block_props);
            } else {
                gpu.scenes
                    .configure_editor(gpu.view, assets)
                    .map_err(|e| e.to_string())?;
            }
            gpu.configured = Some(binding.assets.clone());
            gpu.scene_key = None;
        }
        let key = SceneInputKey {
            detail: binding.detail.clone(),
            custom_content: binding.custom.as_ref().map(|_| custom::content(map)),
            paste: binding.paste.clone(),
            camera: camera_stamp.clone(),
            stamp: SourceStamp::new(map),
            size: binding.size,
            tool: self.server_ctx.shared.curr_map_tool_type,
            hover: self.server_ctx.shared.hover,
            cursor: self.server_ctx.shared.hover_cursor,
            vertices: map.selected_vertices.clone(),
            linedefs: map.selected_linedefs.clone(),
            sectors: map.selected_sectors.clone(),
            entity: map.selected_entity_item,
            rectangle: map.curr_rectangle,
            polygon: map.possible_polygon.clone(),
            line: (map.curr_grid_pos, map.curr_mouse_pos),
            grid: self.server_ctx.shared.show_editor_grid,
            hour: bridge.client.server_time.to_f32().to_bits(),
            animation: bridge.client.animation_frame,
            preview_character_setup: self.play_session.as_ref()
                .map(|play| play.state(&bridge.server)).unwrap_or(bridge.server.state)
                == rusterix::ServerState::Off,
            canvas: bridge.client.editor_canvas_colors.filter(|_| {
                binding.custom.is_none() || bridge.client.use_editor_canvas
            }),
            build: self.build_values.clone(),
            runtime: bridge.render_state_for_view_from(
                self.play_session
                    .as_ref()
                    .map(|play| play.server(&bridge.server))
                    .unwrap_or(&bridge.server),
                map,
                true,
            ),
            terrain_paint: assets.terrain_paint.get(&map.id).map(|p| p.key),
        };
        // The existing 3D guides include transient gizmos/ghosts beyond the 2D
        // key. Prepare them on requested frames until their owner revisions are
        // extracted; do not cache an incomplete description.
        let same_scene = camera.is_none()
            && gpu.scene_key.as_ref() == Some(&key)
            && gpu
                .scene_source
                .as_ref()
                .is_some_and(|old| Arc::ptr_eq(old, &source));
        gpu.scenes.begin_surface_paint_upload_frame();
        let needs_paint = camera.is_some() && binding.custom.is_none();
        let paint = if needs_paint {
            world.paint_capture.request(scope, &self.documents.project, &self.server_ctx, map, assets)?
        } else { world.paint_capture.clear(); None };
        let paint_pending = needs_paint && paint.is_none();
        if let Some(paint) = &paint {
            let limit = u64::from(images.registry.context().device.limits().max_storage_buffer_binding_size);
            let publication = world.uv.request(paint, limit)?;
            gpu.paint_ready = publication.is_some();
            if let Some(publication) = publication {
                gpu.scenes.set_surface_paint_source(gpu.view, publication.source()).map_err(|e| e.to_string())?;
            } else {
                gpu.scenes.defer_surface_paint_source(gpu.view).map_err(|e| e.to_string())?;
            }
        } else if paint_pending {
            gpu.paint_ready = false;
            gpu.scenes.defer_surface_paint_source(gpu.view).map_err(|e| e.to_string())?;
            gpu.scenes.defer_stamp_paint(gpu.view).map_err(|e| e.to_string())?;
        } else {
            gpu.paint_ready = true;
            gpu.scenes.set_surface_paint_source(gpu.view, None).map_err(|e| e.to_string())?;
        }
        let continue_private = (gpu.scenes.private_paint_frame_pending(gpu.view).map_err(|e| e.to_string())?
            || gpu.scenes.stamp_placement_pending(gpu.view).map_err(|e| e.to_string())?
            || gpu.stamps_ready && gpu.paint_ready
                && !gpu.scenes.scene_sources_ready(gpu.view).map_err(|e| e.to_string())?)
            && gpu.scene_key.as_ref().is_some_and(|old| key.same_private_scope(old))
            && gpu.scene_source.as_ref().is_some_and(|old| Arc::ptr_eq(old, &source));
        if !same_scene && !continue_private && !paint_pending {
            if gpu.projection_paste != binding.paste {
                rusterix::chunkbuilder::terrain_surface::TerrainSurface::invalidate_cached(map.id);
                gpu.scenes
                    .invalidate_isometric_geometry(gpu.view)
                    .map_err(|e| e.to_string())?;
                gpu.projection_paste = binding.paste.clone();
            }
            if binding.custom.is_none() && gpu.custom.image.is_some() {
                gpu.custom.clear(images.registry);
            }
            let source_changed = gpu
                .scenes
                .apply_editor_source(gpu.view, source.clone())
                .map_err(|e| e.to_string())?;
            let scene = gpu.scenes.scene_mut(gpu.view).map_err(|e| e.to_string())?;
            scene.runtime_render_state = key.runtime.clone();
            scene.preview_character_setup = key.preview_character_setup;
            if source_changed {
                scene.billboards = if binding.paste.is_some() {
                    world.paste.as_ref().map(|preview| preview.billboards()).unwrap_or_default()
                } else {
                    authored_source.map(|source| source.billboards()).unwrap_or_default()
                };
            }
            let isometric = map.camera == MapCamera::TwoDIso && binding.mode == EditorViewMode::D2;
            if isometric {
                gpu.scenes
                    .prepare_isometric_editor(
                        gpu.view,
                        (binding.size.0 as u32, binding.size.1 as u32),
                        &eldiron_presentation::GameSceneInput {
                            scope: binding.document,
                            map,
                            assets: assets,
                            time: bridge.client.server_time,
                            animation_frame: bridge.client.animation_frame,
                            render_state: &key.runtime,
                        },
                        images.registry,
                    )
                    .map_err(|e| e.to_string())?;
            } else {
                let description = if let Some(camera) = camera.as_ref() {
                    let camera_frame = camera.as_scenevm_camera_for_surface(
                        binding.size.0 as f32,
                        binding.size.1 as f32,
                    );
                    scene.set_entity_item_selection_visible(
                        self.server_ctx.tools.read().current_game_tool_is("tool.entity"),
                    );
                    scene.build_dynamics_3d(
                        map,
                        camera.as_ref(),
                        bridge.client.animation_frame,
                        assets,
                        &Default::default(),
                    );
                    self.server_ctx.tools.write().prepare_geometry_overlay_3d_for_map(
                        &self.documents.project,
                        &self.server_ctx,
                        assets,
                        camera.as_ref(),
                        scene,
                        Some(map),
                        paste_ignored.as_deref(),
                    );
                    let paint = paint.as_ref().ok_or("missing native UV snapshot")?;
                    let has_paint = (paint.visible
                        && (!paint.surface_commit_strokes.is_empty()
                            || !paint.chunks.is_empty()
                            || !paint.baked_chunks.is_empty()))
                        || (paint.selection_visible && paint.active_selection().is_some());
                    scene.vm.set_active_vm(0);
                    if has_paint {
                        scene.vm.execute(scenevm::Atom::SetCamera3D {
                            camera: camera_frame,
                        });
                        if let Some(sprites) = world.stamps.request(paint)? {
                            gpu.scenes.set_stamp_paint(gpu.view, Some((paint, &sprites)))
                                .map_err(|e| e.to_string())?;
                        } else {
                            gpu.scenes.defer_stamp_paint(gpu.view).map_err(|e| e.to_string())?;
                        }
                    } else {
                        scene.vm.execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                        gpu.paint_ready = true;
                        gpu.scenes.set_stamp_paint(gpu.view, None).map_err(|e| e.to_string())?;
                        world.stamps.clear();
                    }
                    rusterix::editor_view::EditorViewFrame::D3(
                        rusterix::editor_view::EditorView3D {
                            size: (binding.size.0 as u32, binding.size.1 as u32),
                            grid_size: map.grid_size,
                            offset: map.offset,
                            hour: bridge.client.server_time.to_f32(),
                            animation_frame: bridge.client.animation_frame,
                            camera: camera_frame,
                            overlay_camera: camera_frame,
                            canvas_colors: bridge.client.editor_canvas_colors,
                            neutral_background: !baked,
                            static_geometry: !baked,
                            transparent_background: baked,
                        },
                    )
                } else {
                    let mut description = bridge
                        .client
                        .editor_view_2d(map, (binding.size.0 as u32, binding.size.1 as u32));
                    if let rusterix::editor_view::EditorViewFrame::D2(frame) = &mut description {
                        frame.draw_grid = self.server_ctx.shared.show_editor_grid;
                        frame.canvas_colors = key.canvas;
                    }
                    scene.settings.backend_2d = if binding.custom.is_some() {
                        rusterix::render_settings::RendererBackend::Compute
                    } else { rusterix::render_settings::RendererBackend::Raster };
                    gpu.guides.set_clip_rect(binding.custom.as_ref().and_then(|custom| custom.clip()));
                    gpu.guides.set_properties(&self.build_values);
                    gpu.guides
                        .set_map_tool_type(self.server_ctx.shared.curr_map_tool_type);
                    gpu.guides
                        .set_map_hover_info(self.server_ctx.shared.hover, self.server_ctx.shared.hover_cursor);
                    gpu.guides.draw_grid = self.server_ctx.shared.show_editor_grid;
                    gpu.guides.show_entity_markers = true;
                    if binding.custom.is_some() {
                        gpu.guide_scene = gpu.guides.build(map, assets,
                            Vec2::new(binding.size.0 as f32, binding.size.1 as f32), &self.build_values);
                    }
                    gpu.guides.build_entities_items(
                        map,
                        assets,
                        &mut gpu.guide_scene,
                        Vec2::new(binding.size.0 as f32, binding.size.1 as f32),
                        if binding.custom.is_some() && scope::uses_region(&self.server_ctx) {
                            &self.server_ctx.shared.editing_surface
                        } else { &None },
                        scene,
                        binding.custom.is_some(),
                    );
                    scene.build_dynamics_2d(
                        map,
                        bridge.client.animation_frame,
                        assets,
                        &Default::default(),
                    );
                    if let Some(custom) = &binding.custom {
                        gpu.guide_scene.animation_frame = bridge.client.animation_frame;
                        gpu.custom.prepare(custom, key.custom_content.unwrap(), map, assets,
                            &binding, &gpu.guide_scene, images.registry)?;
                    }
                    description
                };
                gpu.scenes
                    .prepare_editor(gpu.view, &description, images.registry)
                    .map_err(|e| e.to_string())?;
                if binding.custom.is_some() {
                    // The custom base is already a GPU image. Only the shared
                    // guide layer is composited above it, as in the reference.
                    gpu.scenes.scene_mut(gpu.view).map_err(|e| e.to_string())?
                        .vm.set_layer_enabled(0, false);
                }
            }
            gpu.scenes.prepare_scene_source_uploads(gpu.view).map_err(|e| e.to_string())?;
            gpu.scenes
                .render(gpu.view, map.changed as u64, images.registry)
                .map_err(|e| e.to_string())?;
            gpu.scene_key = Some(key);
            gpu.scene_source = Some(source.clone());
        } else if continue_private {
            gpu.scenes.prepare_scene_source_uploads(gpu.view).map_err(|e| e.to_string())?;
            gpu.scenes.render(gpu.view, map.changed as u64, images.registry).map_err(|e| e.to_string())?;
        }
        gpu.stamps_ready = gpu.scenes.stamp_paint_ready(gpu.view).map_err(|e| e.to_string())?;
        gpu.paint_ready &= gpu.scenes.surface_paint_ready(gpu.view).map_err(|e| e.to_string())?;
        gpu.paint_ready &= gpu.scenes.scene_sources_ready(gpu.view).map_err(|e| e.to_string())?;
        let output = match gpu.scenes.output(gpu.view) {
            Some(output) => output,
            None if !gpu.paint_ready => {
                world.presented = None;
                world.overlay_input.invalidate();
                if let Some((id, list)) = world.published.take()
                    && let Some(view) = ui.get_render_view(&id.name)
                    && view.id().uuid == id.uuid {
                    view.clear_paint_list(&list);
                    ctx.ui.invalidate_paint();
                }
                return Ok(None);
            }
            None => return Err("native world has no visible output".into()),
        };
        let world_image = if let Some(source) = &bake_source {
            match gpu.bake.compose(source, bake_camera.as_ref().unwrap(), output,
                &mut gpu.scenes, &bridge.client, images.registry)? {
                Some(image) => image,
                None => {
                    world.presented = None;
                    world.overlay_input.invalidate();
                    if let Some((id, list)) = world.published.take()
                        && let Some(view) = ui.get_render_view(&id.name)
                        && view.id().uuid == id.uuid {
                        view.clear_paint_list(&list);
                    }
                    ctx.ui.invalidate_paint();
                    return Ok(None);
                }
            }
        } else {
            gpu.bake.clear(images.registry);
            output.image
        };
        drop(bake_source);
        drop(bake_owner);
        let isometric = gpu.scenes.isometric_targets(output);
        let material_slots = binding.slots.as_ref().map(|slots| {
            slots
                .iter()
                .map(|(label, source)| crate::actions::ActionMaterialSlot {
                    label: label.clone(),
                    source: source.clone(),
                })
                .collect::<Vec<_>>()
        });
        let light = ctx
            .ui
            .icon("light_small")
            .map(|icon| crate::hud::HudImageSource {
                width: icon.pixel_width(),
                height: icon.pixel_height(),
                pixels: icon.pixels(),
            });
        let mut tools = self.server_ctx.tools.write();
        let tool = tools.get_current_tool();
        let overlay_inputs = ToolOverlayInputs {
            target,
            style: &binding.style,
            hud: HudInputs {
                map: if binding.paste.is_some() {
                    authored_map
                } else {
                    map
                },
                server: &self.server_ctx,
                assets: assets,
                selected_id: None,
                material_slots: material_slots.as_deref(),
                light,
            },
        };
        let overlay = Arc::new(
            if binding.paste.is_some() {
                ToolOverlayFrame::passive(
                    tool.as_ref(),
                    overlay_inputs,
                    crate::editor_overlay::EditorOverlay::new(binding.size),
                )
            } else {
                tool.prepare_overlay(overlay_inputs)
            }
            .ok_or("native world tool has no overlay producer")?,
        );
        drop(tools);
        let mut list = PaintList::default();
        if map.camera == MapCamera::TwoDIso && binding.mode == EditorViewMode::D2 && binding.custom.is_none() {
            // The reference isometric viewport clears its transparent canvas
            // over black. Preserve that view policy instead of exposing the
            // surrounding dock color through the GPU output's alpha.
            list.rect(Rect::new(0., 0., binding.size.0 as f32, binding.size.1 as f32), 0., [0., 0., 0., 1.]);
        }
        if let Some(image) = gpu.custom.image {
            list.ops.push(PaintOp::SpriteImage { image, origin: [0., 0.], scale: [1., 1.], tint: [1.; 4] });
        }
        if binding.mode == EditorViewMode::D2 {
            // Use the same stable logical-pixel lookup as guides/visibility.
            // Interpolated normalized nearest UVs can drift to a neighboring
            // texel at exact boundaries under fractional host density.
            list.ops.push(PaintOp::SpriteImage {
                image: world_image,
                origin: [0., 0.],
                scale: [1., 1.],
                tint: [1.; 4],
            });
        } else {
            list.image(
                world_image,
                Rect::new(0., 0., binding.size.0 as f32, binding.size.1 as f32),
                Sampling::Nearest,
            );
        }
        if binding.mode == EditorViewMode::D2 && map.camera != MapCamera::TwoDIso && binding.custom.is_none() {
            let settings = &gpu.scenes.scene(gpu.view).map_err(|e| e.to_string())?.settings;
            if let Some(mask) = visibility::paint(map, assets, binding.size, settings) {
                list.ops.push(mask);
            }
        }
        if isometric.is_some() {
            gpu.guides.set_properties(&self.build_values);
            gpu.guides
                .set_map_tool_type(self.server_ctx.shared.curr_map_tool_type);
            gpu.guides
                .set_map_hover_info(self.server_ctx.shared.hover, self.server_ctx.shared.hover_cursor);
            gpu.guides.draw_grid = self.server_ctx.shared.show_editor_grid;
            let size = Vec2::new(binding.size.0 as f32, binding.size.1 as f32);
            rusterix::isometric::editor::guide_segments(
                map,
                size,
                rusterix::isometric::Projection::editor(map, size)
                    .at_height(map.properties.get_float_default("iso_edit_height", 0.)),
                gpu.guides.isometric_guides(),
                |a, b, color| {
                    list.ops.push(PaintOp::SampledLine {
                        from: [a.x, a.y],
                        to: [b.x, b.y],
                        color: color.map(|v| v as f32 / 255.),
                    })
                },
            );
        }
        if isometric.is_some() && binding.mode == EditorViewMode::D2 {
            // The reference scales the completed logical scene+guides buffer.
            // Keep that operation on the shared GPU; HUD/text remain at host DPI.
            list = gpu.pixels.compose(list, output, binding.size, f32::from_bits(binding.density), images.registry)?;
        } else {
            gpu.pixels.clear(images.registry);
        }
        if let Some(runtime) = self.play_session.as_ref().filter(|play| {
            play.document == binding.document
        }).and_then(|play| play.view_region(&bridge, &self.documents.project, &map.id)) {
            world.speech.paint(
                &runtime,
                &binding, map, assets, camera.as_deref(), &mut list,
            );
        } else {
            world.speech.clear();
        }
        let mut resources = gpu.icons.frame(&mut gpu.images, images.registry);
        list.ops.extend(
            overlay
                .paint(&mut ctx.text, &mut resources, ctx.scale_factor)
                .map_err(|e| e.to_string())?
                .ops,
        );
        let cube = if binding.mode == EditorViewMode::Orbit {
            world.cube.prepare(
                binding.cube_target(world.epoch),
                &self.server_ctx.camera.orbit_camera,
            )
        } else {
            world.cube.invalidate();
            None
        };
        if let Some(cube) = &cube {
            list.ops.extend(
                cube.paint(&mut ctx.text, &mut resources, ctx.scale_factor)
                    .map_err(|e| e.to_string())?
                    .ops,
            );
        }
        let help = crate::mapeditor::geometry_selection_status_text(map, &self.server_ctx);
        list.ops.extend(
            Hud::prepare_guidance(binding.size, help.as_deref(), binding.style.colors)
                .paint(&mut ctx.text, &mut resources, ctx.scale_factor)
                .map_err(|e| e.to_string())?
                .ops,
        );
        world.serial = world
            .serial
            .checked_add(1)
            .expect("world frame serial exhausted");
        let receipt = FrameReceipt {
            owner: world.receipt_owner,
            serial: world.serial,
        };
        list.ops.push(PaintOp::Receipt(receipt));
        let receipts = list.receipts().collect();
        let list = Arc::new(list);
        let view = ui
            .get_render_view(&binding.view.name)
            .ok_or("native world view removed")?;
        if view.id().uuid != binding.view.uuid {
            return Err("native world widget replaced during preparation".into());
        }
        view.set_paint_list(Some(list.clone()))
            .map_err(|e| e.to_string())?;
        world.published = Some((binding.view.clone(), list));
        let waiting_cpu_sources = ready && (!gpu.stamps_ready || !gpu.paint_ready);
        let ready = ready && gpu.stamps_ready && gpu.paint_ready;
        Ok(Some(Arc::new(NativeWorldFrame {
            binding,
            target,
            stamp: authored_stamp,
            custom_content,
            source,
            ready,
            waiting_cpu_sources,
            camera: camera_stamp,
            output,
            baked,
            paste_ignored,
            cube,
            isometric,
            overlay,
            receipts,
            receipt,
        })))
    }
    pub(super) fn presented_native_world_viewport(
        &mut self,
        frame: Option<Arc<NativeWorldFrame>>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.server_ctx.tools.write()
            .get_current_tool()
            .sync_overlay_sources(&self.documents.project, &self.server_ctx);
        let next = self.native_world_binding(ui, ctx);
        self.bind_native_world(next.clone(), ui, ctx);
        let current_camera = next.as_ref().and_then(|binding| {
            self.native_world_camera()
                .map(|camera| CameraStamp::new(camera.as_ref(), binding.size))
        });
        let world = &mut self.native_viewport.world;
        let Some(frame) = frame else {
            if next.is_some() {
                self.server_ctx.tools.write()
                    .get_current_tool()
                    .invalidate_overlay();
                world.presented = None;
            }
            return;
        };
        if next.as_ref() != Some(&frame.binding) || frame.target.revision != world.epoch {
            return;
        }
        let Some(map) = scope::map(&self.documents.project, &self.server_ctx) else {
            return;
        };
        if SourceStamp::new(map) != frame.stamp || current_camera != frame.camera {
            return;
        }
        if let Some(cube) = &frame.cube {
            if !world.cube.presented(
                cube,
                &frame.receipts,
                frame.binding.cube_target(world.epoch),
                &self.server_ctx.camera.orbit_camera,
            ) {
                ctx.ui.invalidate_paint();
                return;
            }
        }
        let accepted = frame.binding.paste.is_some() || self.server_ctx.tools.write()
            .get_current_tool()
            .presented_overlay(
                &frame.overlay,
                &frame.receipts,
                frame.target,
                map,
                &mut self.server_ctx,
            );
        if accepted {
            world.presented = Some(frame);
        } else {
            ctx.ui.invalidate_paint();
        }
    }
    pub(crate) fn finish_native_viewport_frame(
        &mut self,
        root: Option<&PaintList>,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<(), String> {
        if let Some(gpu) = &mut self.native_viewport.world.gpu {
            gpu.finish(images, root)?;
        }
        Ok(())
    }
    pub(crate) fn take_native_world_event(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Option<TheEvent> {
        if let Some(event) = self.take_native_retry_click(ui, ctx) {
            return Some(event);
        }
        while let Some(event) = self.native_viewport.world.events.pop_front() {
            self.server_ctx.tools.write()
                .get_current_tool()
                .sync_overlay_sources(&self.documents.project, &self.server_ctx);
            let next = self.native_world_binding(ui, ctx);
            self.bind_native_world(next.clone(), ui, ctx);
            let Some(binding) = next else {
                continue;
            };
            let target = binding.target(self.native_viewport.world.epoch);
            let Some(map) = scope::map(&self.documents.project, &self.server_ctx) else {
                continue;
            };
            let mut tools = self.server_ctx.tools.write();
            if let Some(event) = event.take_if_current(
                tools.get_current_tool().as_mut(),
                target,
                map,
                &self.server_ctx,
            ) {
                return Some(event);
            }
        }
        None
    }
    fn take_native_retry_click(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Option<TheEvent> {
        self.native_viewport.world.retry_click.as_ref()?;
        let next = self.native_world_binding(ui, ctx);
        self.bind_native_world(next.clone(), ui, ctx);
        let binding = next?;
        let camera = self
            .native_world_camera()
            .map(|camera| CameraStamp::new(camera.as_ref(), binding.size));
        let map = scope::map(&self.documents.project, &self.server_ctx)?;
        let world = &mut self.native_viewport.world;
        let mut retry = world.retry_click.take()?;
        if !retry.binding.capture_compatible(&binding) || retry.camera != camera {
            return None;
        }
        if !retry.started {
            if retry.stamp != SourceStamp::new(map)
                || retry.modifiers != (ui.shift, ui.ctrl, ui.alt, ui.logo)
            {
                return None;
            }
            if !world.presented.as_ref().is_some_and(|frame| {
                frame.binding == binding && world.frame_is_fresh(frame, map, &camera)
            }) {
                world.retry_click = Some(retry);
                ctx.ui.invalidate_paint();
                return None;
            }
            retry.started = true;
        } else if !world
            .world_capture
            .as_ref()
            .is_some_and(|gesture| gesture.binding.capture_compatible(&binding))
        {
            // A consumed/rejected replay down cannot invent a later drag/up.
            return None;
        }
        let event = retry.events.pop_front();
        if !retry.events.is_empty() {
            world.retry_click = Some(retry);
        }
        event
    }
    pub(crate) fn handle_native_world_input(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        if !self.native_viewport.enabled {
            return false;
        }
        if self.server_ctx.shared.native_game_input {
            if matches!(event, TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape))) {
                self.cancel_native_world_gesture(ui,ctx);
                ctx.ui.invalidate_paint();
                return true;
            }
            if matches!(event, TheEvent::KeyDown(_) | TheEvent::KeyUp(_) | TheEvent::KeyCodeDown(_) | TheEvent::KeyCodeUp(_)) {
                return true;
            }
        }
        for tool in &mut self.server_ctx.tools.write().game_tools {
            tool.set_view_input(None);
        }
        if !matches!(
            event,
            TheEvent::RenderViewClicked(..)
                | TheEvent::RenderViewDragged(..)
                | TheEvent::RenderViewUp(..)
                | TheEvent::RenderViewHoverChanged(..)
                | TheEvent::KeyDown(..)
                | TheEvent::KeyCodeDown(..)
        ) {
            return false;
        }
        self.server_ctx.tools.write()
            .get_current_tool()
            .sync_overlay_sources(&self.documents.project, &self.server_ctx);
        let next = self.native_world_binding(ui, ctx);
        self.bind_native_world(next.clone(), ui, ctx);
        let Some(binding) = next else {
            return false;
        };
        let focused = ui.dialog.is_none()
            && ui.popover.is_none()
            && ctx
                .ui
                .focus
                .as_ref()
                .is_some_and(|id| id.uuid == binding.view.uuid);
        let (id, input) = match event {
            TheEvent::RenderViewClicked(id, at) => (Some(id), MapEvent::MapClicked(*at)),
            TheEvent::RenderViewDragged(id, at) => (Some(id), MapEvent::MapDragged(*at)),
            TheEvent::RenderViewUp(id, at) => (Some(id), MapEvent::MapUp(*at)),
            TheEvent::RenderViewHoverChanged(id, at) => (Some(id), MapEvent::MapHover(*at)),
            TheEvent::KeyDown(TheValue::Char(key))
                if focused && !ui.logo && !ui.ctrl && !ui.alt =>
            {
                (None, MapEvent::MapKey(*key))
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) if focused => {
                (None, MapEvent::MapDelete)
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape)) if focused => {
                (None, MapEvent::MapEscape)
            }
            _ => return false,
        };
        if let Some(id) = id {
            if id.name != binding.view.name {
                return binding.context.is_prefab() && id.name == "PolyView";
            }
            if id.uuid != binding.view.uuid {
                return true;
            }
        }
        if binding.paste.is_some() {
            if focused
                && ui.context_menu.is_none()
                && ctx.ui.overlay.is_none()
                && self.handle_native_viewcube_input(&binding, input, ctx)
            {
                return true;
            }
            return self.handle_native_paste_input(&binding, input, ui, ctx, focused);
        }
        let current_camera = self
            .native_world_camera()
            .map(|camera| CameraStamp::new(camera.as_ref(), binding.size));
        if self
            .native_viewport
            .world
            .retry_click
            .as_ref()
            .is_some_and(|retry| !retry.started)
        {
            match input {
                MapEvent::MapDragged(_) | MapEvent::MapUp(_) => {
                    let world = &mut self.native_viewport.world;
                    let retry = world.retry_click.as_mut().unwrap();
                    if retry.events.len() < 128
                        && retry.modifiers == (ui.shift, ui.ctrl, ui.alt, ui.logo)
                    {
                        retry.events.push_back(event.clone());
                    } else {
                        world.retry_click = None;
                    }
                    return true;
                }
                MapEvent::MapHover(_) => return true,
                MapEvent::MapEscape => {
                    self.native_viewport.world.retry_click = None;
                    return true;
                }
                _ => self.native_viewport.world.retry_click = None,
            }
        }
        if matches!(input, MapEvent::MapEscape)
            && self.native_viewport.world.world_capture.is_some()
        {
            self.cancel_native_world_gesture(ui, ctx);
            ctx.ui.invalidate_paint();
            return true;
        }
        if binding.context.is_prefab()
            && matches!(input, MapEvent::MapClicked(_))
            && self
                .native_viewport
                .world
                .presented
                .as_ref()
                .is_some_and(|frame| {
                    frame.binding == binding
                        && frame.ready
                        && frame.camera == current_camera
                        && scope::map(&self.documents.project, &self.server_ctx)
                            .is_some_and(|map| frame.stamp == SourceStamp::new(map))
                })
            && self.server_ctx.docks.write().prepare_native_view_input(
                ui,
                ctx,
                &mut self.documents.project,
                &mut self.server_ctx,
            )
        {
            let next = self.native_world_binding(ui, ctx);
            self.bind_native_world(next.clone(), ui, ctx);
            if let (Some(binding), Some(map)) = (next, scope::map(&self.documents.project, &self.server_ctx)) {
                self.native_viewport.world.retry_click = Some(RetryClick {
                    binding,
                    stamp: SourceStamp::new(map),
                    camera: current_camera,
                    modifiers: (ui.shift, ui.ctrl, ui.alt, ui.logo),
                    events: std::collections::VecDeque::from([event.clone()]),
                    started: false,
                });
            }
            ctx.ui.invalidate_paint();
            return true;
        }
        if matches!(input, MapEvent::MapClicked(_)) {
            self.native_viewport.world.world_capture = None;
        }
        if self.handle_native_viewcube_input(&binding, input, ctx) {
            return true;
        }
        let world = &mut self.native_viewport.world;
        if matches!(input, MapEvent::MapUp(_))
            && world
                .world_capture
                .as_ref()
                .is_some_and(|owner| owner.binding == binding)
        {
            let gesture = world.world_capture.take().unwrap();
            let mut tools = self.server_ctx.tools.write();
            tools.get_current_tool().set_view_input(gesture.query);
            // Use the complete existing tool dispatcher for undo grouping,
            // surface rebuilding, moved entities and navigation release.
            let redraw = tools.handle_event(
                &TheEvent::RenderViewUp(binding.view.clone(), gesture.point),
                ui,
                ctx,
                &mut self.documents.project,
                &mut self.server_ctx,
            );
            tools.get_current_tool().set_view_input(None);
            drop(tools);
            let dock_redraw = if binding.context.is_prefab() {
                self.server_ctx.docks.write().handle_event(
                    &TheEvent::RenderViewUp(binding.view.clone(), gesture.point),
                    ui,
                    ctx,
                    &mut self.documents.project,
                    &mut self.server_ctx,
                )
            } else {
                false
            };
            let redraw = redraw || dock_redraw;
            ctx.ui.invalidate_paint_if(redraw);
            return true;
        }
        let mut accepted_query = None;
        let frame = world.presented.clone();
        let target = binding.target(world.epoch);
        let Some(map) = scope::map_mut(&mut self.documents.project, &self.server_ctx) else {
            return true;
        };
        let mut tools = self.server_ctx.tools.write();
        let tool = tools.get_current_tool().as_mut();
        let route = world.overlay_input.route(
            tool,
            frame.as_ref().map(|f| &f.overlay),
            target,
            &input,
            focused,
            map,
            &self.server_ctx,
        );
        match route {
            ToolOverlayRoute::Unavailable => {
                ctx.ui.invalidate_paint();
                return true;
            }
            ToolOverlayRoute::Consumed { redraw, command } => {
                ctx.ui.invalidate_paint_if(redraw);
                if let Some(command) = command
                    && let Some(effects) =
                        command.apply(tool, target, map, &mut self.server_ctx, ui, ctx)
                {
                    ctx.ui.invalidate_paint_if(
                        effects.redraw || effects.scene_dirty || effects.overlay_dirty,
                    );
                    world.events.extend(effects.events);
                }
                return true;
            }
            ToolOverlayRoute::Outside { redraw } => ctx.ui.invalidate_paint_if(redraw),
        }
        drop(tools);
        let pointer = match input {
            MapEvent::MapClicked(at)
            | MapEvent::MapDragged(at)
            | MapEvent::MapUp(at)
            | MapEvent::MapHover(at) => Some(at),
            _ => None,
        };
        let continuation = matches!(
            input,
            MapEvent::MapDragged(_) | MapEvent::MapUp(_) | MapEvent::MapEscape
        );
        if continuation
            && world
                .world_capture
                .as_ref()
                .is_some_and(|owner| owner.binding == binding)
            && binding.mode != EditorViewMode::D2
        {
            if let Some(at) = pointer {
                let mut tools = self.server_ctx.tools.write();
                let tool = tools.get_current_tool();
                let rectangle = tool.view_rectangle_query(at);
                let query = frame
                    .as_ref()
                    .filter(|frame| frame.camera == current_camera)
                    .and_then(|frame| {
                        world
                            .gpu
                            .as_mut()?
                            .query(frame, at, &self.server_ctx, rectangle, false)
                    });
                if let Some((pick, input)) = query {
                    apply_pick(pick, map, &mut self.server_ctx);
                    accepted_query = Some(input.clone());
                    tool.set_view_input(Some(input));
                } else {
                    ctx.ui.invalidate_paint();
                    return true;
                }
            }
        }
        if continuation
            && world
                .world_capture
                .as_ref()
                .is_some_and(|owner| owner.binding == binding)
            && binding.mode == EditorViewMode::D2
            && binding.camera == MapCamera::TwoDIso
        {
            if let Some(at) = pointer {
                let Some(query) = frame
                    .as_ref()
                    .and_then(|frame| isometric_input(world, frame, at))
                else {
                    ctx.ui.invalidate_paint();
                    return true;
                };
                accepted_query = Some(query.clone());
                self.server_ctx.tools.write()
                    .get_current_tool()
                    .set_view_input(Some(query));
            }
        }
        if continuation
            && world
                .world_capture
                .as_ref()
                .is_some_and(|owner| owner.binding == binding)
        {
            if let MapEvent::MapDragged(at) = input
                && let Some(owner) = &mut world.world_capture
            {
                owner.point = at;
                owner.query = accepted_query;
            }
            if matches!(input, MapEvent::MapEscape) {
                world.world_capture = None;
            }
            return false;
        }
        let fresh = frame
            .as_ref()
            .is_some_and(|frame| world.frame_is_fresh(frame, map, &current_camera));
        if !fresh || matches!(input, MapEvent::MapDragged(_) | MapEvent::MapUp(_)) {
            ctx.ui.invalidate_paint();
            return true;
        }
        if binding.mode != EditorViewMode::D2 {
            if let Some(at) = pointer {
                if at.x < 0
                    || at.y < 0
                    || at.x as usize >= binding.size.0
                    || at.y as usize >= binding.size.1
                {
                    return true;
                }
                let capture_surface = self.server_ctx.tools.read()
                    .game_tools
                    .iter()
                    .find(|tool| tool.id().uuid == binding.tool)
                    .is_some_and(|tool| tool.view_surface_capture(&input));
                let query = frame.as_ref().and_then(|frame| {
                    world
                        .gpu
                        .as_mut()?
                        .query(frame, at, &self.server_ctx, None, capture_surface)
                });
                let Some((pick, input)) = query else {
                    ctx.ui.invalidate_paint();
                    return true;
                };
                apply_pick(pick, map, &mut self.server_ctx);
                accepted_query = Some(input.clone());
                self.server_ctx.tools.write()
                    .get_current_tool()
                    .set_view_input(Some(input));
            }
            if matches!(input, MapEvent::MapClicked(_)) {
                world.world_capture = Some(WorldGesture {
                    binding,
                    point: pointer.unwrap(),
                    query: accepted_query,
                });
            }
            return false;
        }
        if binding.camera == MapCamera::TwoDIso {
            if let Some(at) = pointer {
                let Some(query) = frame
                    .as_ref()
                    .and_then(|frame| isometric_input(world, frame, at))
                else {
                    ctx.ui.invalidate_paint();
                    return true;
                };
                accepted_query = Some(query.clone());
                self.server_ctx.tools.write()
                    .get_current_tool()
                    .set_view_input(Some(query));
            }
        }
        if let MapEvent::MapClicked(at) = input {
            // Hover may have been discarded while source geometry was pending.
            // Resolve this fresh click against its acknowledged 2D transform,
            // rather than reusing a hit from an earlier pointer location/view.
            let size = Vec2::new(binding.size.0 as f32, binding.size.1 as f32);
            let position = Vec2::new(at.x as f32, at.y as f32);
            self.server_ctx.shared.geo_hit = None;
            self.server_ctx.shared.hover_ray_origin_3d = None;
            self.server_ctx.shared.hover_ray_dir_3d = None;
            if matches!(
                self.server_ctx.shared.curr_map_tool_type,
                MapToolType::Vertex | MapToolType::Linedef | MapToolType::Sector
            ) {
                self.server_ctx.shared.hover = self.server_ctx.shared.geometry_at_for_tool(
                    size,
                    position,
                    map,
                    self.server_ctx.shared.curr_map_tool_type,
                );
            }
            self.server_ctx.shared.hover_cursor =
                Some(
                    self.server_ctx
                        .shared.local_to_map_grid(size, position, map, map.subdivisions),
                );
            world.world_capture = Some(WorldGesture {
                binding,
                point: at,
                query: accepted_query,
            });
        }
        false
    }
}

#[cfg(test)]
impl Editor {
    pub(crate) fn reference_for_test(&self) -> &CreatorReference { &self.server_ctx.reference }
    pub(crate) fn reference_jobs_for_test(&self) -> &CreatorJobs {
        &self.server_ctx.jobs
    }
    pub(crate) fn authoring_camera_for_test(&mut self) -> &mut EditCamera {
        &mut self.server_ctx.camera
    }
    pub(crate) fn check_context_ownership_for_test(&mut self, gpu: &Arc<renderkit::GpuContext>) {
        // Resetting another CPU authoring context cannot cancel this owner.
        let initial_scale = self.server_ctx.camera.iso_camera.scale;
        self.server_ctx.camera.move_action = Some(CustomMoveAction::Forward);
        let mut independent = CreatorContext::new();
        let minimap_key = self.server_ctx.reference.minimap().key;
        independent.reference.minimap_mut().key = u64::MAX;
        assert_eq!(self.server_ctx.reference.minimap().key, minimap_key);
        assert!(independent.reference.minimap().image.pixels().is_empty());
        independent.reference.clear_minimap();
        assert_eq!(independent.reference.minimap().key, 0);
        let target_fps = self.server_ctx.config.target_fps;
        let sidebar_mode = self.server_ctx.sidebar_mode;
        independent.sidebar_mode = SidebarMode::Palette;
        assert_eq!(self.server_ctx.sidebar_mode, sidebar_mode);
        independent.config.read_config("[game]\ntarget_fps=59\ngame_tick_ms=123\n[render]\nsample_mode='linear'");
        assert_eq!(self.server_ctx.config.target_fps, target_fps);
        assert_eq!(independent.config.game_tick_ms, 123);
        assert_eq!(independent.config.get_string_default("render", "sample_mode", "nearest"), "linear");
        assert!(!self.server_ctx.jobs.scene_initialized());
        assert!(!self.server_ctx.reference.initialized());
        assert!(!independent.jobs.scene_initialized());
        assert!(!independent.reference.initialized());
        independent.reference.bind_context(gpu.clone());
        assert!(!independent.reference.initialized());
        independent.reference.write().assets.config = "independent owner".into();
        assert!(independent.reference.initialized());
        assert_eq!(independent.reference.read().scene_handler.vm.device_generation(), Some(gpu.generation()));
        assert!(!self.server_ctx.reference.initialized());
        independent.jobs.scene_mut().startup();
        assert!(independent.jobs.scene_initialized());
        assert!(!self.server_ctx.jobs.scene_initialized(), "another CPU context cannot initialize this owner's scene jobs");
        independent.camera.iso_camera.scale = initial_scale + 7.;
        independent.camera.move_action = Some(CustomMoveAction::Backward);
        independent.camera.reset_for_project_switch();
        assert_eq!(self.server_ctx.camera.iso_camera.scale, initial_scale);
        assert!(matches!(
            self.server_ctx.camera.move_action,
            Some(CustomMoveAction::Forward)
        ));
        assert!(independent.camera.move_action.is_none());
        self.server_ctx.camera.reset_for_project_switch();
    }
    pub(crate) fn stage_native_paste_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> (TheId, usize, Vec<u8>) {
        let mut clipboard = Map::new();
        let sector = crate::project_templates::rectangle(&mut clipboard, -0.5, -0.5, 1., 1.);
        if let Some(id) = self.server_ctx.reference.read().assets.tiles.keys().next().copied() {
            clipboard
                .find_sector_mut(sector)
                .unwrap()
                .properties
                .set("source", Value::Source(rusterix::PixelSource::TileId(id)));
        }
        self.server_ctx.shared.clipboard = clipboard;
        self.server_ctx.shared.hover_cursor = None;
        self.server_ctx.shared.hover_cursor_3d = None;
        self.server_ctx.shared.geo_hit = None;
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        let before = serde_json::to_vec(map).unwrap();
        let vertices = map.vertices.len();
        let view = crate::utils::map_editor_render_view(ui, &self.server_ctx)
            .unwrap()
            .id()
            .clone();
        ctx.ui.set_focus(&view);
        ctx.ui.send(TheEvent::Paste(TheValue::Empty, None));
        (view, vertices, before)
    }
    pub(crate) fn native_reference_source_ready_for_test(&self) -> bool {
        // Match update_ui's deliberate deferral: sprite-isometric authoring
        // leaves hidden SceneVM chunks queued until a camera needs them.
        let deferred = !self.server_ctx.shared.game_mode
            && self.server_ctx.shared.editor_view_mode == EditorViewMode::D2
            && self.server_ctx.shared.editing_surface.is_none()
            && self.documents.project.get_map(&self.server_ctx.shared)
                .is_some_and(|map| map.camera == MapCamera::TwoDIso);
        deferred || !self.server_ctx.jobs.scene_initialized()
            || !self.server_ctx.jobs.scene().has_pending_geometry()
    }
    pub(crate) fn native_world_source_for_test(&self) -> Option<(Arc<SceneBuildSnapshot>, bool)> {
        let source = self.native_viewport.world.source.as_ref()?;
        Some((source.snapshot(), source.ready()))
    }
    pub(crate) fn native_paste_authored_for_test(&self) -> Vec<u8> {
        serde_json::to_vec(scope::map(&self.documents.project, &self.server_ctx).unwrap()).unwrap()
    }
    pub(crate) fn native_paste_projection_for_test(&self) -> Option<Arc<Map>> {
        self.native_viewport
            .world
            .paste
            .as_ref()
            .map(|preview| preview.map.clone())
    }
    pub(crate) fn native_paste_has_ghost_guides_for_test(&self) -> bool {
        let world = &self.native_viewport.world;
        let Some(ignored) = world
            .presented
            .as_ref()
            .and_then(|frame| frame.paste_ignored.as_ref())
        else {
            return false;
        };
        world
            .gpu
            .as_ref()
            .and_then(|gpu| gpu.scenes.scene(gpu.view).ok())
            .is_some_and(|scene| {
                scene
                    .overlay_3d
                    .owner_geo_ids()
                    .any(|id| ignored.contains(&id))
            })
    }
    pub(crate) fn native_paste_mode_for_test(&self) -> EditorViewMode {
        self.server_ctx.shared.editor_view_mode
    }
    pub(crate) fn native_paste_active_for_test(&self) -> bool {
        self.server_ctx.shared.paste_clipboard.is_some()
    }
    pub(crate) fn native_paste_ready_for_test(&self) -> bool {
        let world = &self.native_viewport.world;
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        world.presented.as_ref().is_some_and(|frame| {
            frame.binding.paste.as_ref().is_some_and(|stamp| {
                stamp.has_target()
                    && Some(stamp) == paste::Stamp::current(&self.server_ctx).as_ref()
            }) && world.frame_is_fresh(
                frame,
                map,
                &self
                    .native_world_camera()
                    .map(|camera| CameraStamp::new(camera.as_ref(), frame.binding.size)),
            )
        })
    }
    pub(crate) fn native_paste_preview_for_test(
        &self,
    ) -> Option<(usize, theframework_core::ImageHandle)> {
        let world = &self.native_viewport.world;
        Some((
            world.paste.as_ref()?.map.vertices.len(),
            world.presented.as_ref()?.output.image,
        ))
    }
    pub(crate) fn native_retry_state_for_test(&self) -> (bool, bool) {
        (
            self.native_viewport.world.retry_click.is_some(),
            self.native_viewport.world.world_capture.is_some(),
        )
    }
    pub(crate) fn native_world_debug_for_test(&self) -> String {
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        let frame = self.native_viewport.world.presented.as_ref();
        let builds = self.server_ctx.jobs.scene();
        format!(
            "frame={} ready={} current_map={} pending={} projection={}",
            frame.is_some(),
            frame.is_some_and(|f| f.ready),
            frame.is_some_and(|f| f.stamp == SourceStamp::new(map)),
            builds.has_pending_geometry(),
            builds.source_matches(map.id, map.camera)
        )
    }
    pub(crate) fn native_prefab_particles_for_test(&self) -> usize {
        self.native_viewport
            .world
            .gpu
            .as_ref()
            .and_then(|gpu| gpu.scenes.scene(gpu.view).ok())
            .map_or(0, |scene| scene.particle_debug_stats().d3.active_particles)
    }
    pub(crate) fn native_world_ready_for_test(&self) -> bool {
        self.native_viewport
            .world
            .presented
            .as_ref()
            .is_some_and(|frame| {
                frame.ready
                    && scope::map(&self.documents.project, &self.server_ctx)
                        .is_some_and(|map| frame.stamp == SourceStamp::new(map))
            })
    }
    pub(crate) fn native_prefab_projection_for_test(&self) -> Option<Vec2<f32>> {
        let asset = self
            .documents.project
            .block_props
            .get(&self.server_ctx.shared.curr_block_asset_id?)?;
        let effect = asset.particle_effects.first()?;
        let attachment = asset
            .find_part(effect.part_id)?
            .attachments
            .iter()
            .find(|a| a.id == effect.attachment_id)?;
        let transform = rusterix::block_prop_part_world_transform(
            asset,
            &rusterix::BlockPropInstance::new(asset.id),
            effect.part_id,
        )?;
        let p = Vec3::from(attachment.position);
        let origin = Mat4::<f32>::from_col_arrays(transform) * Vec4::new(p.x, p.y, p.z, 1.);
        let camera = self.native_world_camera()?;
        let size = self.native_viewport.world.presented.as_ref()?.binding.size;
        let clip = camera.projection_matrix(size.0 as f32, size.1 as f32)
            * camera.view_matrix_for_surface(size.0 as f32, size.1 as f32)
            * origin;
        Some(Vec2::new(
            (clip.x / clip.w + 1.) * 0.5 * size.0 as f32,
            (1. - clip.y / clip.w) * 0.5 * size.1 as f32,
        ))
    }
    pub(crate) fn native_prefab_handle_for_test(&mut self) -> Option<Vec2<i32>> {
        let center = self.native_prefab_projection_for_test()?.map(|v| v as i32);
        let world = &mut self.native_viewport.world;
        let frame = world.presented.clone()?;
        let size = frame.binding.size;
        let gpu = world.gpu.as_mut()?;
        // Confirm the real marker near its projected position, avoiding a full
        // viewport scan. This still obtains every hit from the displayed scene.
        for y in center.y - 16..=center.y + 16 {
            for x in center.x - 16..=center.x + 16 {
                if x < 0 || y < 0 || x >= size.0 as i32 || y >= size.1 as i32 {
                    continue;
                }
                let uv = [x as f32 / size.0 as f32, y as f32 / size.1 as f32];
                if gpu
                    .scenes
                    .pick_editor(frame.output, uv, true, true)?
                    .gizmo
                    .is_some_and(|(id, _, _)| id == scenevm::GeoId::Gizmo(0xEFFE_0000))
                {
                    return Some(Vec2::new(x, y));
                }
            }
        }
        None
    }
    pub(crate) fn native_paint_3d_target_for_test(&mut self) -> Option<Vec2<i32>> {
        let world = &mut self.native_viewport.world;
        let frame = world.presented.clone()?;
        let gpu = world.gpu.as_mut()?;
        // Choose a real paintable triangle outside the actual Paint panel/cube.
        // No fixture hit or synthesized paint coordinate is injected.
        for y in (8..frame.binding.size.1.saturating_sub(8)).step_by(8) {
            for x in (16..frame.binding.size.0.saturating_sub(8)).step_by(8) {
                if x < 520 && y < 274 {
                    continue;
                }
                let uv = [
                    x as f32 / frame.binding.size.0 as f32,
                    y as f32 / frame.binding.size.1 as f32,
                ];
                if gpu
                    .scenes
                    .pick_editor_paint(frame.output, uv)?
                    .surface
                    .is_some()
                {
                    return Some(Vec2::new(x as i32, y as i32));
                }
            }
        }
        None
    }
    pub(crate) fn native_paint_3d_debug_for_test(&self) -> String {
        let world = &self.native_viewport.world;
        let scene = world.gpu.as_ref().and_then(|g| g.scenes.scene(g.view).ok());
        format!(
            "{} stats={:?} camera={:?} bounds={:?}",
            self.native_world_debug_for_test(),
            scene.map(|s| s.vm.vm.debug_stats()),
            scene.map(|s| s.vm.vm.camera3d),
            scene.and_then(|s| s.vm.vm.scene_bounds_3d())
        )
    }
    fn native_paint_layer_for_test(&self) -> &shared::iso_paint::IsoPaintLayer {
        if let ProjectContext::Prefab(id) = self.server_ctx.shared.pc {
            &self.documents.project.block_prop_paint[&id]
        } else {
            &self
                .documents.project
                .get_region(&self.server_ctx.shared.curr_region)
                .unwrap()
                .iso_paint
        }
    }
    pub(crate) fn native_paint_has_3d_data_for_test(&self) -> bool {
        let paint = self.native_paint_layer_for_test();
        paint
            .chunks
            .values()
            .flat_map(|c| &c.strokes)
            .max_by_key(|s| s.order)
            .is_some_and(|stroke| {
                stroke.points.iter().any(|p| {
                    p.paint_geo.is_some()
                        && p.surface_uv.is_some()
                        && p.surface_normal.is_some()
                        && p.owner.as_ref().is_some_and(|owner| {
                            !matches!(owner, shared::iso_paint::IsoPaintOwner::Terrain { .. })
                        })
                })
            })
    }
    pub(crate) fn native_paint_state_for_test(&self) -> (f32, usize, Option<u64>) {
        let paint = self.native_paint_layer_for_test();
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        (
            paint.active_opacity,
            paint
                .chunks
                .values()
                .map(|c| c.strokes.len() + c.stamps.len())
                .sum(),
            self.server_ctx.reference.read()
                .assets
                .terrain_paint
                .get(&map.id)
                .map(|p| p.key),
        )
    }
    pub(crate) fn native_paint_point_for_test(&self, size: (i32, i32)) -> Vec2<i32> {
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        let bounds = map.sectors.last().unwrap().bounding_box(map);
        let point = (bounds.min + bounds.max) * 0.5;
        assert!(
            rusterix::isometric::flat_terrain_contains(map, point),
            "Paint test target is authored ground"
        );
        rusterix::isometric::Projection::editor(map, Vec2::new(size.0 as f32, size.1 as f32))
            .screen(point, 0.)
            .map(|v| v.round() as i32)
    }
    pub(crate) fn native_rect_source_for_test(&mut self) {
        let mut tile = rusterix::Tile::default();
        tile.id = Uuid::new_v4();
        tile.textures.push(rusterix::Texture::new(
            vec![210, 120, 70, 255].repeat(12),
            4,
            3,
        ));
        tile.isometric = Some(rusterix::isometric::SpriteLayout {
            anchor: [2, 2],
            pixels_per_unit: 4.,
            directions: 1,
            frames_per_direction: 1,
            bitmap: true,
            ground: true,
            ..Default::default()
        });
        self.server_ctx.shared.curr_tile_source = Some(rusterix::TileSource::SingleTile(tile.id));
        self.documents.project.tiles.insert(tile.id, tile.clone());
        self.server_ctx.reference.write().assets.tiles.insert(tile.id, tile);
    }
    pub(crate) fn native_rect_state_for_test(&self) -> (bool, f32, usize) {
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        (
            self.server_ctx.shared.rect_compose_mode,
            map.properties.get_float_default("iso_brush_yaw", 0.),
            map.sectors.len(),
        )
    }
    pub(crate) fn native_terrain_radius_for_test(&self) -> f32 {
        self.server_ctx.shared.terrain_brush_radius
    }
    pub(crate) fn native_terrain_points_for_test(&self) -> Option<rusterix::Value> {
        scope::map(&self.documents.project, &self.server_ctx)
            .unwrap()
            .properties
            .get("terrain_sculpt")
            .cloned()
    }
    pub(crate) fn native_world_has_ray_for_test(&self) -> bool {
        self.server_ctx.shared.hover_ray_origin_3d.is_some() && self.server_ctx.shared.hover_ray_dir_3d.is_some()
    }




    pub(crate) fn native_world_has_cpu_storage_for_test(&self) -> bool {
        let gpu = self
            .native_viewport
            .world
            .gpu
            .as_ref()
            .expect("world GPU owner");
        gpu.scenes.has_cpu_target_storage(gpu.view).unwrap()
    }
    pub(crate) fn native_world_character_preview_for_test(&self) -> bool {
        let gpu = self.native_viewport.world.gpu.as_ref().expect("world GPU owner");
        gpu.scenes.scene(gpu.view).unwrap().preview_character_setup
    }
}
