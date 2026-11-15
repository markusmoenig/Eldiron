//! Map overview ownership. CPU snapshots enter here; only GPU handles leave.
//! Navigation uses an immutable, actually presented map/projection, independent of the reference CPU cache.
use super::*;
use eldiron_presentation::{SceneViews, ViewId, ViewImage};
use rusterix::scene_build_index::SceneBuildSnapshot;
use std::sync::Arc;
use theframework::{
    paint::{FrameReceipt, PaintList, PaintOp, Rect, new_receipt_owner},
    theui::{
        paint::{UiPaintResult, WidgetPainter, rgba},
        thewidget::therenderview::RenderViewPaintSource,
    },
};

use crate::scene_source as sources;
#[path = "native_scene_source.rs"]
mod source_cache;
use source_cache::SourceCache;

#[derive(Clone, PartialEq)]
struct Binding {
    document: Uuid,
    region: Uuid,
    context: ProjectContext,
    editing_surface: Option<Uuid>,
    editing_map: Option<Uuid>,
    map: Uuid,
    source: u64,
    mode: EditorViewMode,
    view: TheId,
    rect: [i32; 4],
    density: u32,
}
struct Frame {
    binding: Binding,
    map: Arc<Map>,
    bbox: Vec4<f32>,
    output: ViewImage,
    image: renderkit::ImageHandle,
    markers: PaintList,
    marker_key: [u32; 4],
    receipt: FrameReceipt,
    label: &'static str,
}
impl RenderViewPaintSource for Frame {
    fn paint(&self, p: &mut WidgetPainter<'_>, viewport: [f32; 2]) -> UiPaintResult {
        if viewport != [self.binding.rect[2] as f32, self.binding.rect[3] as f32] {
            return Ok(());
        }
        p.list.ops.push(PaintOp::SpriteImage {
            image: self.image,
            origin: [0.; 2],
            scale: [1.; 2],
            tint: [1.; 4],
        });
        p.list.ops.extend(self.markers.ops.iter().cloned());
        p.list.ops.push(PaintOp::PixelRects {
            rects: vec![Rect::new(0., 0., 50., 16.)].into(),
            color: rgba([36, 36, 36, 255]),
        });
        p.label(
            Rect::new(4., 0., 46., 16.),
            self.label,
            11.,
            [215, 215, 215, 255],
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        p.list.ops.push(PaintOp::Receipt(self.receipt));
        Ok(())
    }
}
struct Pending;
impl RenderViewPaintSource for Pending {
    fn paint(&self, p: &mut WidgetPainter<'_>, viewport: [f32; 2]) -> UiPaintResult {
        p.list.ops.push(PaintOp::PixelRects {
            rects: vec![Rect::new(0., 0., viewport[0], viewport[1])].into(),
            color: rgba([42, 42, 42, 255]),
        });
        Ok(())
    }
}
#[derive(Clone)]
pub(crate) struct Token(Arc<Frame>);
impl Token {
    pub(crate) fn receipt(&self) -> FrameReceipt {
        self.0.receipt
    }
}
struct Gpu {
    registry: u64,
    scenes: SceneViews,
    view: ViewId,
    binding: Option<Binding>,
    assets: u64,
    source: Option<Arc<SceneBuildSnapshot>>,
    animation: usize,
    version: u32,
    opaque: Option<(renderkit::ImageHandle, renderkit::ImageHandle)>,
    overview: Option<eldiron_presentation::overview2d::Overview2DRenderer>,
    overview_output: Option<ViewImage>,
}
impl Gpu {
    fn new(images: &renderkit::ImageRegistry) -> Result<Self, String> {
        let mut scenes = SceneViews::new(images, scenevm::SharedAtlas::new(1024, 1024));
        let view = ViewId::new();
        scenes.create(view, (0, 0)).map_err(|e| e.to_string())?;
        Ok(Self {
            registry: images.owner_id(),
            scenes,
            view,
            binding: None,
            assets: 0,
            source: None,
            animation: 0,
            version: 0,
            opaque: None,
            overview: None,
            overview_output: None,
        })
    }
}
pub(crate) struct SceneOverview {
    owner: u64,
    serial: u64,
    enabled: bool,
    current: Option<Arc<Frame>>,
    presented: Option<Arc<Frame>>,
    gpu: Option<Gpu>,
    waiting: bool,
    source: Option<sources::Source>,
    source_cache: SourceCache,
}
impl Default for SceneOverview {
    fn default() -> Self {
        Self {
            owner: new_receipt_owner(),
            serial: 0,
            enabled: false,
            current: None,
            presented: None,
            gpu: None,
            waiting: false,
            source: None,
            source_cache: SourceCache::default(),
        }
    }
}
fn region<'a>(project: &'a Project, server: &CreatorContext) -> Option<&'a Region> {
    if server.shared.get_map_context() != MapContext::Region || palette_minimap_active(server) {
        return None;
    }
    let region = project.get_region(&server.shared.curr_region)?;
    if active_action_minimap_preview(&region.map, server).is_some()
        || (region.map.camera != MapCamera::TwoDIso
            && server.shared.editor_view_mode != EditorViewMode::D2
            && !region.map.geometry_objects.is_empty())
    {
        return None;
    }
    Some(region)
}
pub(crate) fn overview_map<'a>(
    project: &'a Project,
    server: &CreatorContext,
) -> Option<(&'a Map, Option<Uuid>)> {
    if server.shared.pc.is_screen()
        && !palette_minimap_active(server)
    {
        let map = project.get_map(&server.shared)?;
        if active_action_minimap_preview(map, server).is_none() {
            return Some((map, None));
        }
        return None;
    }
    region(project, server).map(|region| (&region.map, Some(region.id)))
}
fn separate_source(project: &Project, server: &CreatorContext, map: &Map) -> bool {
    !server.shared.pc.is_screen()
        && (!matches!(server.shared.pc, ProjectContext::Region(id) if id == server.shared.curr_region)
            || server.shared.editing_surface.is_some()
            || project
                .get_map(&server.shared)
                .is_none_or(|editing| editing.id != map.id))
}
impl SceneOverview {
    fn refresh_source(&mut self, project: &Project, server: &CreatorContext) -> Option<source_cache::Snapshot> {
        if let Some((map, _)) = overview_map(project, server) {
            Some(self.source_cache.publish(server.shared.document_instance, map))
        } else {
            self.source_cache.clear();
            None
        }
    }
    pub(crate) fn advance_source(
        &mut self,
        project: &Project,
        server: &CreatorContext,
        assets: &rusterix::Assets,
        asset_revision: u64,
    ) -> bool {
        let Some((map, _)) = overview_map(project, server).filter(|(map, _)| {
            self.enabled
                && map.camera != MapCamera::TwoDIso
                && separate_source(project, server, map)
        }) else {
            self.source = None;
            return false;
        };
        let snapshot = self.source_cache.publish(server.shared.document_instance, map);
        let key = sources::SourceKey {
            document: server.shared.document_instance,
            map: map.id,
            source: u64::from(snapshot.revision),
            assets: asset_revision,
        };
        if self.source.as_ref().is_none_or(|old| old.key != key) {
            self.source = Some(sources::Source::new(key, snapshot.map.as_ref().clone(), assets));
        }
        self.source.as_mut().unwrap().advance()
    }
    fn binding(
        project: &Project,
        server: &CreatorContext,
        ui: &mut TheUI,
        ctx: &TheContext,
        snapshot: source_cache::Snapshot,
    ) -> Option<(Binding, Arc<Map>)> {
        let (original, region) = overview_map(project, server)?;
        let view = ui.get_render_view("MiniMap")?;
        let dim = *view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return None;
        }
        // The producer has already checked this immutable publication against
        // the borrowed project. Input obtains it through a fresh content check.
        if snapshot.document != server.shared.document_instance || snapshot.map.id != original.id {
            return None;
        }
        let map = snapshot.map;
        let source = u64::from(snapshot.revision);
        Some((
            Binding {
                document: server.shared.document_instance,
                region: region.unwrap_or_default(),
                context: server.shared.pc,
                editing_surface: server.shared.editing_surface.as_ref().map(|s| s.id),
                editing_map: project.get_map(&server.shared).map(|map| map.id),
                map: map.id,
                source,
                mode: server.shared.editor_view_mode,
                view: view.id().clone(),
                rect: [dim.x, dim.y, dim.width, dim.height],
                density: ctx.scale_factor.to_bits(),
            },
            map,
        ))
    }
    pub(crate) fn publish(
        &mut self,
        project: &Project,
        server: &CreatorContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> bool {
        if !ui.get_render_view("MiniMap").is_some_and(|view| view.dim().is_valid())
            || overview_map(project, server).is_none()
        {
            return false;
        }
        let Some(snapshot) = self.refresh_source(project, server) else { return false; };
        if Self::binding(project, server, ui, ctx, snapshot).is_none() { return false; }
        if !self.enabled {
            self.invalidate(ui);
            ui.get_render_view("MiniMap")
                .unwrap()
                .set_paint_source(Some(Arc::new(Pending)));
        }
        self.enabled = true;
        true
    }
    /// CPU source bridge supplied by Editor. This owner never accesses a global
    /// VM, creates a device, ticks simulation or requests a color readback.
    pub(crate) fn prepare(
        &mut self,
        project: &Project,
        server: &CreatorContext,
        ui: &mut TheUI,
        ctx: &TheContext,
        assets: &rusterix::Assets,
        asset_revision: u64,
        source: Arc<SceneBuildSnapshot>,
        ready: bool,
        animation: usize,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<Option<Token>, String> {
        self.waiting = false;
        if !self.enabled {
            if let Some(gpu) = self.gpu.as_mut()
                && gpu.registry == images.owner_id()
            {
                if let Some((_, image)) = gpu.opaque.take() {
                    images.remove(image);
                }
                if let Some(output) = gpu.overview_output.take() {
                    images.remove(output.image);
                }
                gpu.scenes.clear(images).map_err(|e| e.to_string())?;
            }
            self.gpu = None;
            return Ok(None);
        }
        let snapshot = self.refresh_source(project, server);
        let Some((binding, map)) = snapshot.and_then(|snapshot| Self::binding(project, server, ui, ctx, snapshot)) else {
            self.invalidate(ui);
            return Ok(None);
        };
        let iso = map.camera == MapCamera::TwoDIso;
        let screen = binding.context.is_screen();
        let raster_overview = screen && !iso;
        let owned_source = !iso && separate_source(project, server, &map);
        let (source, ready) = if owned_source {
            let key = sources::SourceKey {
                document: binding.document,
                map: binding.map,
                source: binding.source,
                assets: asset_revision,
            };
            if let Some(source) = self.source.as_ref().filter(|source| source.key == key) {
                (source.snapshot(), source.ready())
            } else {
                (source, false)
            }
        } else {
            (source, ready)
        };
        if !iso && !screen && !ready {
            self.waiting = true;
            self.invalidate(ui);
            ui.get_render_view("MiniMap")
                .unwrap()
                .set_paint_source(Some(Arc::new(Pending)));
            return Ok(None);
        }
        if self
            .gpu
            .as_ref()
            .is_none_or(|gpu| gpu.registry != images.owner_id())
        {
            self.invalidate(ui);
            self.gpu = Some(Gpu::new(images)?);
        }
        let gpu = self.gpu.as_mut().unwrap();
        let scope_changed = gpu.binding.as_ref().is_none_or(|old| {
            old.document != binding.document
                || old.map != binding.map
                || old.context != binding.context
        });
        let bbox = if iso {
            isometric_minimap_bbox(
                &map,
                assets,
                Vec2::new(binding.rect[2] as f32, binding.rect[3] as f32),
            )
        } else {
            minimap_bbox_for_map(&map).ok_or("minimap bounds unavailable")?
        };
        if scope_changed
            || gpu.assets != asset_revision
            || gpu.binding.as_ref() != Some(&binding)
            || (!iso
                && !screen
                && (gpu.animation != animation
                    || gpu
                        .source
                        .as_ref()
                        .is_none_or(|old| !Arc::ptr_eq(old, &source))))
        {
            if scope_changed || gpu.assets != asset_revision {
                if let Some((_, image)) = gpu.opaque.take() {
                    images.remove(image);
                }
                gpu.scenes
                    .remove(gpu.view, images)
                    .map_err(|e| e.to_string())?;
                gpu.scenes
                    .create(gpu.view, (0, 0))
                    .map_err(|e| e.to_string())?;
                if let Some(output) = gpu.overview_output.take() {
                    images.remove(output.image);
                }
                gpu.overview = None;
                if !screen {
                    gpu.scenes
                        .configure_editor(gpu.view, assets)
                        .map_err(|e| e.to_string())?;
                }
            }
            gpu.version = gpu
                .version
                .checked_add(1)
                .ok_or("minimap source generation exhausted")?;
            // Source caches must distinguish replacement/undo even when authored
            // IDs and serialized revision numbers have been reused.
            // Only the producers which use Map.changed for their derived cache
            // need a temporary versioned projection. Ordinary overview frames
            // and navigation retain the immutable CPU source publication.
            let render_map = if raster_overview || iso {
                let mut projection = map.as_ref().clone();
                projection.changed = gpu.version;
                Some(projection)
            } else { None };
            let render_map = render_map.as_ref().unwrap_or(map.as_ref());
            let size = (binding.rect[2] as u32, binding.rect[3] as u32);
            if !raster_overview {
                if let Some(output) = gpu.overview_output.take() {
                    images.remove(output.image);
                }
                gpu.overview = None;
            }
            if raster_overview {
                let mut builder = rusterix::D2PreviewBuilder::new();
                let scene = builder.build(
                    render_map,
                    assets,
                    Vec2::new(size.0 as f32, size.1 as f32),
                    &Default::default(),
                );
                let scale = Vec2::new(size.0 as f32 / bbox.z, size.1 as f32 / bbox.w);
                let center = Vec2::new(bbox.x + bbox.z * 0.5, bbox.y + bbox.w * 0.5);
                let transform = Mat3::<f32>::translation_2d(
                    Vec2::new(size.0 as f32, size.1 as f32) * 0.5 - center * scale,
                ) * Mat3::new(scale.x, 0., 0., 0., scale.y, 0., 0., 0., 1.);
                if gpu.overview.is_none() {
                    gpu.overview = Some(
                        eldiron_presentation::overview2d::Overview2DRenderer::new(
                            images.context().clone(),
                        )
                        .map_err(|e| e.to_string())?,
                    );
                }
                let image = gpu
                    .overview
                    .as_mut()
                    .unwrap()
                    .render(
                        &scene,
                        assets,
                        size,
                        transform,
                        binding.document,
                        asset_revision,
                        &[], // The reference overview never installs a MapMini on Rasterizer.
                    )
                    .map_err(|e| e.to_string())?;
                let existing = gpu.overview_output.filter(|old| {
                    images
                        .get(old.image)
                        .is_some_and(|old| Arc::ptr_eq(&old, &image))
                });
                let handle = if let Some(existing) = existing {
                    existing.image
                } else {
                    if let Some(old) = gpu.overview_output.take() {
                        images.remove(old.image);
                    }
                    images.insert(image.clone()).map_err(|e| e.to_string())?
                };
                gpu.overview_output = Some(ViewImage {
                    view: gpu.view,
                    image: handle,
                    descriptor: image.descriptor,
                    revision: gpu.version as u64,
                    content_revision: gpu.version as u64,
                });
            } else if iso {
                gpu.scenes
                    .prepare_isometric_overview(
                        gpu.view,
                        size,
                        render_map,
                        assets,
                        isometric_minimap_projection(Vec2::new(size.0 as f32, size.1 as f32), bbox),
                        gpu.version as u64,
                        images,
                    )
                    .map_err(|e| e.to_string())?;
            } else {
                gpu.scenes
                    .apply_editor_source(gpu.view, source.clone())
                    .map_err(|e| e.to_string())?;
                gpu.scenes
                    .prepare_minimap(
                        gpu.view,
                        &rusterix::editor_view::MinimapView2D {
                            size,
                            bounds: bbox,
                            animation_frame: animation,
                        },
                        images,
                    )
                    .map_err(|e| e.to_string())?;
            }
            if !raster_overview {
                gpu.scenes
                    .render(gpu.view, gpu.version as u64, images)
                    .map_err(|e| e.to_string())?;
            }
            gpu.binding = Some(binding.clone());
            gpu.assets = asset_revision;
            gpu.source = Some(source);
            gpu.animation = animation;
        }
        let output = if raster_overview {
            gpu.overview_output
        } else {
            gpu.scenes.output(gpu.view)
        }
        .ok_or("minimap output unavailable")?;
        // The reference copied RGB into the opaque application framebuffer.
        // Alias the same texture with explicit opaque metadata: no copy, upload,
        // CPU composition or extra target is needed to preserve stored RGB.
        let image = if let Some((source, image)) = gpu.opaque
            && source == output.image
        {
            image
        } else {
            if let Some((_, image)) = gpu.opaque.take() {
                images.remove(image);
            }
            let source = images.get(output.image).ok_or("minimap image missing")?;
            let image = Arc::new(
                renderkit::GpuImage::from_texture(
                    images.context(),
                    source.texture.clone(),
                    renderkit::ImageDescriptor {
                        alpha_mode: renderkit::AlphaMode::Opaque,
                        ..source.descriptor
                    },
                )
                .map_err(|e| e.to_string())?,
            );
            let image = images.insert(image).map_err(|e| e.to_string())?;
            gpu.opaque = Some((output.image, image));
            image
        };
        let (original, region_id) =
            overview_map(project, server).ok_or("minimap context changed")?;
        let region = region_id.and_then(|id| project.get_region(&id));
        let size = Vec2::new(binding.rect[2] as f32, binding.rect[3] as f32);
        let center = if server.shared.editor_view_mode == EditorViewMode::D2 || region.is_none() {
            editor_view_center(original)
        } else {
            let region = region.unwrap();
            Vec2::new(region.editing_position_3d.x, region.editing_position_3d.z)
        };
        let look = region
            .filter(|_| server.shared.editor_view_mode == EditorViewMode::FirstP)
            .map(|region| Vec2::new(region.editing_look_at_3d.x, region.editing_look_at_3d.z));
        let marker_key = [
            center.x.to_bits(),
            center.y.to_bits(),
            look.map_or(0., |p| p.x).to_bits(),
            look.map_or(0., |p| p.y).to_bits(),
        ];
        let mut markers = PaintList::default();
        for (world, color) in [(Some(center), [255, 0, 0, 255]), (look, [255, 255, 0, 255])] {
            if let Some(world) = world {
                let p = minimap_world_to_pixel_on_map(world, size, bbox, &map);
                let (x, y) = (p.x as i32 as f32 - 4., p.y as i32 as f32 - 4.);
                markers.ops.push(PaintOp::PixelRects {
                    rects: vec![
                        Rect::new(x, y, 8., 1.),
                        Rect::new(x, y + 7., 8., 1.),
                        Rect::new(x, y, 1., 8.),
                        Rect::new(x + 7., y, 1., 8.),
                    ]
                    .into_iter()
                    .map(|rect| rect.intersection(Rect::new(0., 0., size.x, size.y)))
                    .filter(|rect| rect.width > 0. && rect.height > 0.)
                    .collect::<Vec<_>>()
                    .into(),
                    color: rgba(color),
                });
            }
        }
        // Keep immutable frames/receipts stable when both image and marker are unchanged.
        if let Some(frame) = &self.current
            && frame.binding == binding
            && frame.output == output
            && frame.bbox == bbox
            && frame.marker_key == marker_key
        {
            return Ok(Some(Token(frame.clone())));
        }
        let presented = self
            .presented
            .clone()
            .filter(|frame| frame.binding == binding && frame.bbox == bbox);
        self.invalidate(ui);
        self.presented = presented;
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or("minimap receipt exhausted")?;
        let frame = Arc::new(Frame {
            binding,
            map,
            bbox,
            output,
            image,
            markers,
            marker_key,
            receipt: FrameReceipt {
                owner: self.owner,
                serial: self.serial,
            },
            label: if screen { "Screen" } else { "Region" },
        });
        ui.get_render_view("MiniMap")
            .unwrap()
            .set_paint_source(Some(frame.clone()));
        self.current = Some(frame.clone());
        Ok(Some(Token(frame)))
    }
    pub(crate) fn presented(&mut self, token: Option<Token>) {
        self.presented = token
            .filter(|token| {
                self.current.as_ref().is_some_and(|frame| {
                    token.receipt().owner == self.owner
                        && frame.binding == token.0.binding
                        && frame.bbox == token.0.bbox
                })
            })
            .map(|token| token.0);
    }
    pub(crate) fn invalidate(&mut self, ui: &mut TheUI) {
        self.presented = None;
        if let Some(frame) = self.current.take() {
            let expected: Arc<dyn RenderViewPaintSource> = frame.clone();
            if let Some(view) = ui.get_render_view(&frame.binding.view.name)
                && view.id() == &frame.binding.view
            {
                view.clear_paint_source(&expected);
            }
        }
    }
    pub(crate) fn deactivate(&mut self, ui: &mut TheUI) {
        self.invalidate(ui);
        self.enabled = false;
        self.waiting = false;
        self.source = None;
        self.source_cache.clear();
    }
    pub(crate) fn device_reset(&mut self, ui: &mut TheUI) {
        self.invalidate(ui);
        if let Some(gpu) = &mut self.gpu {
            gpu.binding = None;
        }
    }
    pub(crate) fn waiting(&self) -> bool {
        self.waiting
    }
    /// Return the world position from the displayed source map, including its
    /// terrain elevation; a stale/pending native click is consumed without edits.
    pub(crate) fn navigate(
        &self,
        id: &TheId,
        at: Vec2<i32>,
        project: &Project,
        server: &CreatorContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Result<Option<Vec2<f32>>, ()> {
        let Some((original, _)) = overview_map(project, server) else { return Ok(None); };
        let snapshot = self.source_cache.current(server.shared.document_instance, original).ok_or(())?;
        let (binding, _) = Self::binding(project, server, ui, ctx, snapshot).ok_or(())?;
        let frame = self.presented.as_ref().ok_or(())?;
        if frame.binding != binding || frame.binding.view != *id {
            return Err(());
        }
        Ok(Some(minimap_pixel_to_world_on_map(
            Vec2::new(at.x as f32, at.y as f32),
            Vec2::new(binding.rect[2] as f32, binding.rect[3] as f32),
            frame.bbox,
            &frame.map,
        )))
    }
}

#[cfg(test)]
#[path = "native_scene_checkpoint.rs"]
mod checkpoint;
#[cfg(test)]
pub(crate) use checkpoint::check as check_for_test;
