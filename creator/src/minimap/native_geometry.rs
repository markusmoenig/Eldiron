//! Geometry overview paint and its acknowledged navigation scope. No renderer,
//! pixel buffer or world simulation belongs to this CPU application owner.
use super::*;
use std::sync::Arc;
use theframework::{
    paint::{FrameReceipt, PaintList, PaintOp, Rect, new_receipt_owner},
    theui::{
        paint::{UiPaintResult, WidgetPainter, rgba},
        thewidget::therenderview::RenderViewPaintSource,
    },
};

#[derive(Clone, PartialEq)]
struct Binding {
    document: Uuid,
    region: Uuid,
    context: ProjectContext,
    editing_surface: Option<Uuid>,
    editing_map: Option<Uuid>,
    map: Uuid,
    revision: u32,
    geometry: u64,
    mode: EditorViewMode,
    view: TheId,
    rect: [i32; 4],
    density: u32,
    content: u64,
}
impl Binding {
    fn navigation_compatible(&self, next: &Self) -> bool {
        self.document == next.document
            && self.region == next.region
            && self.context == next.context
            && self.editing_surface == next.editing_surface
            && self.editing_map == next.editing_map
            && self.map == next.map
            && self.geometry == next.geometry
            && self.mode == next.mode
            && self.view == next.view
            && self.rect == next.rect
            && self.density == next.density
    }
}
struct Frame {
    binding: Binding,
    bbox: Vec4<f32>,
    paint: PaintList,
    receipt: FrameReceipt,
}
impl RenderViewPaintSource for Frame {
    fn paint(&self, p: &mut WidgetPainter<'_>, viewport: [f32; 2]) -> UiPaintResult {
        if viewport != [self.binding.rect[2] as f32, self.binding.rect[3] as f32] {
            // Layout changed before preparation; never acknowledge a stretched
            // geometry overview with the previous navigation transform.
            p.list.rect(
                Rect::new(0., 0., viewport[0], viewport[1]),
                0.,
                rgba([42, 42, 42, 255]),
            );
            return Ok(());
        }
        p.list.ops.extend(self.paint.ops.iter().cloned());
        let label = "Region";
        let width = (label.len() as i32 * 7 + 8).max(24) as f32;
        // The old copied display discarded the stored 180 alpha in this box.
        p.list.ops.push(PaintOp::PixelRects {
            rects: vec![Rect::new(0., 0., width, 16.)].into(),
            color: rgba([36, 36, 36, 255]),
        });
        p.label(
            Rect::new(4., 0., width - 4., 16.),
            label,
            11.,
            [215, 215, 215, 255],
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        p.list.ops.push(PaintOp::Receipt(self.receipt));
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
pub(crate) struct GeometryOverview {
    owner: u64,
    serial: u64,
    enabled: bool,
    current: Option<Arc<Frame>>,
    presented: Option<Arc<Frame>>,
}
impl Default for GeometryOverview {
    fn default() -> Self {
        Self {
            owner: new_receipt_owner(),
            serial: 0,
            enabled: false,
            current: None,
            presented: None,
        }
    }
}
struct Commands(PaintList);
impl MinimapDiagramPainter for Commands {
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: RGBA) {
        self.0.ops.push(PaintOp::LogicalPixelLine {
            from: [x0 as f32, y0 as f32],
            to: [x1 as f32, y1 as f32],
            color: rgba(color),
        });
    }
    fn outline(&mut self, r: &TheDim, color: &RGBA) {
        if r.width <= 0 || r.height <= 0 {
            return;
        }
        let (x, y, w, h) = (r.x as f32, r.y as f32, r.width as f32, r.height as f32);
        self.0.ops.push(PaintOp::PixelRects {
            rects: vec![
                Rect::new(x, y, w, 1.),
                Rect::new(x, y + h - 1., w, 1.),
                Rect::new(x, y, 1., h),
                Rect::new(x + w - 1., y, 1., h),
            ]
            .into(),
            color: rgba(*color),
        });
    }
}
fn region<'a>(project: &'a Project, server: &ServerContext) -> Option<&'a Region> {
    if server.get_map_context() != MapContext::Region
        || server.editor_view_mode == EditorViewMode::D2
        || palette_minimap_active(server)
    {
        return None;
    }
    let region = project.get_region(&server.curr_region)?;
    if region.map.camera == MapCamera::TwoDIso
        || region.map.geometry_objects.is_empty()
        || active_action_minimap_preview(&region.map, server).is_some()
    {
        return None;
    }
    Some(region)
}
impl GeometryOverview {
    fn binding(
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<(Binding, Vec4<f32>)> {
        let region = region(project, server)?;
        let bbox = minimap_bbox_for_map(&region.map)?;
        let view = ui.get_render_view("MiniMap")?;
        let dim = *view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return None;
        }
        let mut geometry = DefaultHasher::new();
        // Undo/replacement can reuse a serialized map revision. Hash the actual
        // diagram source at publication/input validation, never during paint.
        for object in &region.map.geometry_objects {
            object.id.hash(&mut geometry);
            for vertex in &object.vertices {
                for value in [vertex.x, vertex.y, vertex.z] {
                    value.to_bits().hash(&mut geometry);
                }
            }
            for value in object.transform.iter().flatten() {
                value.to_bits().hash(&mut geometry);
            }
            for face in &object.faces {
                face.indices.hash(&mut geometry);
                for point in &face.surface_points {
                    for value in [point.position.x, point.position.y, point.position.z] {
                        value.to_bits().hash(&mut geometry);
                    }
                }
                for segment in &face.surface_segments {
                    segment.start.hash(&mut geometry);
                    segment.end.hash(&mut geometry);
                }
            }
        }
        for value in [bbox.x, bbox.y, bbox.z, bbox.w] {
            value.to_bits().hash(&mut geometry);
        }
        std::mem::discriminant(&region.map.camera).hash(&mut geometry);
        let geometry = geometry.finish();
        let mut hash = DefaultHasher::new();
        geometry.hash(&mut hash);
        hash_minimap_map_state(&region.map, &mut hash);
        std::mem::discriminant(&server.editor_view_mode).hash(&mut hash);
        for v in [
            region.editing_position_3d.x,
            region.editing_position_3d.z,
            region.editing_look_at_3d.x,
            region.editing_look_at_3d.z,
            bbox.x,
            bbox.y,
            bbox.z,
            bbox.w,
        ] {
            v.to_bits().hash(&mut hash);
        }
        Some((
            Binding {
                document: server.document_instance,
                region: region.id,
                context: server.pc,
                editing_surface: server.editing_surface.as_ref().map(|s| s.id),
                editing_map: project.get_map(server).map(|map| map.id),
                map: region.map.id,
                revision: region.map.changed,
                geometry,
                mode: server.editor_view_mode,
                view: view.id().clone(),
                rect: [dim.x, dim.y, dim.width, dim.height],
                density: ctx.scale_factor.to_bits(),
                content: hash.finish(),
            },
            bbox,
        ))
    }
    pub(crate) fn publish(
        &mut self,
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> bool {
        let Some((binding, bbox)) = Self::binding(project, server, ui, ctx) else {
            return false;
        };
        self.enabled = true;
        self.prepare_with(binding, bbox, project, server, ui);
        true
    }
    fn prepare_with(
        &mut self,
        binding: Binding,
        bbox: Vec4<f32>,
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
    ) {
        if self
            .current
            .as_ref()
            .is_some_and(|frame| frame.binding == binding)
        {
            return;
        }
        // Marker movement does not change the displayed world-to-map transform.
        // Retain its acknowledged navigation scope during a continuous drag.
        let presented = self
            .presented
            .clone()
            .filter(|frame| frame.binding.navigation_compatible(&binding));
        self.invalidate(ui);
        self.presented = presented;
        let region = project.get_region(&binding.region).unwrap();
        let size = Vec2::new(binding.rect[2] as f32, binding.rect[3] as f32);
        let mut commands = Commands(PaintList::default());
        commands.0.ops.push(PaintOp::PixelRects {
            rects: vec![Rect::new(0., 0., size.x, size.y)].into(),
            color: rgba([42, 42, 42, 255]),
        });
        geometry_minimap(&region.map, size, bbox, &mut commands);
        for (world, color) in [
            (Some(region.editing_position_3d), [255, 0, 0, 255]),
            (
                (server.editor_view_mode == EditorViewMode::FirstP)
                    .then_some(region.editing_look_at_3d),
                [255, 255, 0, 255],
            ),
        ] {
            if let Some(world) = world {
                let point = minimap_world_to_pixel_on_map(
                    Vec2::new(world.x, world.z),
                    size,
                    bbox,
                    &region.map,
                );
                commands.outline(
                    &TheDim::rect(point.x as i32 - 4, point.y as i32 - 4, 8, 8),
                    &color,
                );
            }
        }
        self.serial = self
            .serial
            .checked_add(1)
            .expect("minimap receipt exhausted");
        let frame = Arc::new(Frame {
            binding,
            bbox,
            paint: commands.0,
            receipt: FrameReceipt {
                owner: self.owner,
                serial: self.serial,
            },
        });
        ui.get_render_view("MiniMap")
            .unwrap()
            .set_paint_source(Some(frame.clone()));
        self.current = Some(frame);
    }
    /// Called after current layout, independently of generic event delivery.
    pub(crate) fn prepare(
        &mut self,
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<Token> {
        if !self.enabled {
            return None;
        }
        let Some((binding, bbox)) = Self::binding(project, server, ui, ctx) else {
            self.invalidate(ui);
            return None;
        };
        self.prepare_with(binding, bbox, project, server, ui);
        self.current.clone().map(Token)
    }
    pub(crate) fn presented(&mut self, token: Option<Token>) {
        self.presented = token
            .filter(|token| {
                self.current.as_ref().is_some_and(|current| {
                    token.receipt().owner == self.owner
                        && current.binding.navigation_compatible(&token.0.binding)
                })
            })
            .map(|token| token.0);
    }
    pub(crate) fn invalidate(&mut self, ui: &mut TheUI) {
        self.presented = None;
        if let Some(frame) = self.current.take() {
            let expected: Arc<dyn RenderViewPaintSource> = frame.clone();
            if let Some(view) = ui.get_render_view(&frame.binding.view.name)
                && view.id().uuid == frame.binding.view.uuid
            {
                view.clear_paint_source(&expected);
            }
        }
    }
    pub(crate) fn deactivate(&mut self, ui: &mut TheUI) {
        self.invalidate(ui);
        self.enabled = false;
    }
    /// None selects the remaining explicit legacy adapter. Err consumes a stale
    /// native click; an acknowledged frame returns its displayed world bounds.
    pub(crate) fn navigation_bbox(
        &self,
        id: &TheId,
        project: &Project,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Result<Option<Vec4<f32>>, ()> {
        if region(project, server).is_none() {
            return Ok(None);
        }
        let (binding, _) = Self::binding(project, server, ui, ctx).ok_or(())?;
        let frame = self.presented.as_ref().ok_or(())?;
        if !frame.binding.navigation_compatible(&binding) || frame.binding.view != *id {
            return Err(());
        }
        Ok(Some(frame.bbox))
    }
}

/// Populated diagram and ownership cases in the existing assembled checkpoint.
#[cfg(test)]
pub(crate) fn reference_cases() -> Vec<(&'static str, Arc<dyn RenderViewPaintSource>, TheRGBABuffer)>
{
    use crate::{dockmanager::DockManagerState, editor::DOCKMANAGER, sidebar::Sidebar};
    struct Plain;
    impl Dock for Plain {
        fn new() -> Self {
            Self
        }
        fn setup(&mut self, _: &mut TheContext) -> TheCanvas {
            TheCanvas::new()
        }
    }
    const KEY: &str = "Geometry Minimap Checkpoint";
    struct Restore(DockManagerState, usize, String);
    impl Drop for Restore {
        fn drop(&mut self) {
            let mut manager = DOCKMANAGER.write().unwrap();
            manager.docks.shift_remove(KEY);
            manager.state = self.0;
            manager.index = self.1;
            manager.dock = self.2.clone();
        }
    }
    let _restore = {
        let mut manager = DOCKMANAGER.write().unwrap();
        assert!(!manager.docks.contains_key(KEY));
        let saved = Restore(manager.state, manager.index, manager.dock.clone());
        manager.docks.insert(KEY.into(), Box::new(Plain));
        manager.index = manager.docks.get_index_of(KEY).unwrap();
        manager.state = DockManagerState::Minimized;
        manager.dock = KEY.into();
        saved
    };
    struct Body(Arc<Frame>);
    impl RenderViewPaintSource for Body {
        fn paint(&self, p: &mut WidgetPainter<'_>, _: [f32; 2]) -> UiPaintResult {
            p.list.ops.extend(self.0.paint.ops.iter().cloned());
            Ok(())
        }
    }
    let mut project = Project::default();
    project.regions.clear();
    let mut author = Region::new();
    let mut object = rusterix::GeometryObject::box_from_bounds(
        "overview",
        Vec3::new(-2., 0., -1.),
        Vec3::new(2., 2., 3.),
    );
    // Oblique projected edges exercise logical Bresenham coverage at fractional DPI.
    let (sin, cos) = 0.37_f32.sin_cos();
    object.transform[0][0] = cos;
    object.transform[0][2] = sin;
    object.transform[2][0] = -sin;
    object.transform[2][2] = cos;
    let id = object.id;
    author.map.geometry_objects.push(object);
    author.map.selected_geometry_objects.push(id);
    author.map.selected_geometry_faces.push((id, 0));
    author.map.selected_geometry_vertices.push((id, 0));
    author.editing_position_3d = Vec3::new(-0.75, 0., 0.5);
    author.editing_look_at_3d = Vec3::new(1.25, 0., 2.25);
    let region_id = author.id;
    project.regions.push(author);
    let base = project;
    let mut cases = Vec::new();
    for context in 0..3 {
        let mut project = base.clone();
        let mut isolated = Map::new();
        isolated.offset = Vec2::new(29., 37.);
        isolated.add_vertex_at(100., 200.);
        let mut surface = rusterix::Surface::new(0);
        surface.profile = Some(isolated.id);
        project.regions[0]
            .map
            .profiles
            .insert(isolated.id, isolated.clone());
        project.regions[0]
            .map
            .surfaces
            .insert(surface.id, surface.clone());
        project.prefab_editor_map = Some(isolated.clone());
        let mut server = ServerContext::new();
        server.pc = ProjectContext::Region(region_id);
        server.curr_region = region_id;
        server.editor_view_mode = EditorViewMode::FirstP;
        if context == 1 {
            server.pc = ProjectContext::Prefab(Uuid::new_v4());
        }
        if context == 2 {
            server.geometry_edit_mode = GeometryEditMode::Detail;
            server.active_detail_surface = Some(surface);
        }
        let untouched = serde_json::to_value(&isolated).unwrap();
        for size in [Vec2::new(96, 96), Vec2::new(143, 83)] {
            let mut ctx = TheContext::new(size.x as usize, size.y as usize, 1.);
            ctx.ui.command_paint = true;
            let mut ui = TheUI::new();
            let mut view = TheRenderView::new(TheId::named("MiniMap"));
            let view_id = view.id().clone();
            view.set_dim(TheDim::new(13, 29, size.x, size.y), &mut ctx);
            view.render_buffer_mut().resize(size.x, size.y);
            ui.canvas.set_widget(view);
            let mut sidebar = Sidebar::new();
            let update = TheEvent::Custom(TheId::named("Soft Update Minimap"), TheValue::Empty);
            assert!(sidebar.handle_event(&update, &mut ui, &mut ctx, &mut project, &mut server));
            assert!(
                !ui.get_render_view("MiniMap")
                    .unwrap()
                    .has_cpu_display_storage()
            );
            ui.get_render_view("MiniMap")
                .unwrap()
                .set_needs_redraw(false);
            assert!(
                !sidebar.handle_event(&update, &mut ui, &mut ctx, &mut project, &mut server),
                "unchanged geometry overview cannot schedule another presentation"
            );
            let mut overview = GeometryOverview::default();
            assert!(overview.publish(&project, &server, &mut ui, &ctx));
            assert!(
                !ui.get_render_view("MiniMap")
                    .unwrap()
                    .has_cpu_display_storage()
            );
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_err()
            );
            let token = overview.prepare(&project, &server, &mut ui, &ctx).unwrap();
            let source = overview.current.as_ref().unwrap().clone();
            // Successful presentation is the only point at which navigation begins.
            overview.presented(Some(token.clone()));
            assert_eq!(
                overview.navigation_bbox(&view_id, &project, &server, &mut ui, &ctx),
                Ok(Some(source.bbox))
            );
            let mut other_document = ServerContext::new();
            other_document.pc = server.pc;
            other_document.curr_region = server.curr_region;
            other_document.editor_view_mode = server.editor_view_mode;
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &other_document, &mut ui, &ctx)
                    .is_err()
            );
            let mut reference = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
            reference.fill([42, 42, 42, 255]);
            let author = &project.regions[0];
            draw_geometry_minimap(&author.map, &mut reference, source.bbox);
            draw_camera_marker(&author.map, Some(author), &mut reference, &server, source.bbox);
            cases.push((
                "geometry minimap",
                Arc::new(Body(source)) as Arc<dyn RenderViewPaintSource>,
                reference,
            ));
            project.regions[0].editing_position_3d.x += 0.25;
            overview.prepare(&project, &server, &mut ui, &ctx).unwrap();
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_ok(),
                "camera marker updates cannot interrupt an acknowledged minimap drag"
            );
            let revision = project.regions[0].map.changed;
            project.regions[0].map.geometry_objects[0].vertices[0].x += 0.125;
            assert_eq!(project.regions[0].map.changed, revision);
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_err(),
                "source replacement cannot reuse a serialized map revision"
            );
            let changed = overview.prepare(&project, &server, &mut ui, &ctx).unwrap();
            overview.presented(Some(token.clone()));
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_err()
            );
            overview.presented(Some(changed));
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_ok()
            );
            ctx.scale_factor = 1.5;
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_err()
            );
            let next = overview.prepare(&project, &server, &mut ui, &ctx).unwrap();
            overview.presented(Some(token)); // Late acknowledgement cannot replace the new scope.
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_err()
            );
            overview.presented(Some(next));
            assert!(
                overview
                    .navigation_bbox(&view_id, &project, &server, &mut ui, &ctx)
                    .is_ok()
            );
            if context != 0 {
                // The real Sidebar consumes the presented parent bounds, while the
                // isolated Prefab/detail map retains its own view and authored data.
                sidebar.minimap_geometry = overview;
                let point = Vec2::new(70, 50);
                let expected = minimap_pixel_to_world_on_map(
                    Vec2::new(point.x as f32, point.y as f32),
                    Vec2::new(size.x as f32, size.y as f32),
                    sidebar.minimap_geometry.current.as_ref().unwrap().bbox,
                    &project.regions[0].map,
                );
                sidebar.handle_event(
                    &TheEvent::RenderViewClicked(view_id.clone(), point),
                    &mut ui,
                    &mut ctx,
                    &mut project,
                    &mut server,
                );
                let pos = project.regions[0].editing_position_3d;
                assert!((Vec2::new(pos.x, pos.z) - expected).magnitude() < 0.0001);
                assert_eq!(
                    serde_json::to_value(project.get_map(&server).unwrap()).unwrap(),
                    untouched
                );
                overview = std::mem::take(&mut sidebar.minimap_geometry);
            }
            // Identity-checked revocation cannot erase a different paint producer.
            let replacement: Arc<dyn RenderViewPaintSource> =
                Arc::new(Body(overview.current.as_ref().unwrap().clone()));
            ui.get_render_view("MiniMap")
                .unwrap()
                .set_paint_source(Some(replacement.clone()));
            overview.invalidate(&mut ui);
            assert!(
                ui.get_render_view("MiniMap")
                    .unwrap()
                    .clear_paint_source(&replacement)
            );
        }
    }
    cases
}
