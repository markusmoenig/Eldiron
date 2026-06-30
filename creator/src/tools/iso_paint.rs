use crate::editor::{DOCKMANAGER, RUSTERIX};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;

pub struct IsoPaintTool {
    id: TheId,
    painting: bool,
    previous_dock: Option<String>,
    active_stroke: Option<Uuid>,
    stroke_before: Option<Region>,
    stroke_changed: bool,
}

impl IsoPaintTool {
    fn hit_status(server_ctx: &ServerContext) -> String {
        if server_ctx.geo_hit.is_some() {
            fl!("status_iso_paint_hit")
        } else if server_ctx.hover_cursor_3d.is_some() {
            fl!("status_iso_paint_ground")
        } else {
            fl!("status_iso_paint_active")
        }
    }

    fn owner_from_geo_id(geo_id: scenevm::GeoId) -> IsoPaintOwner {
        match geo_id {
            scenevm::GeoId::Unknown(id) => IsoPaintOwner::Unknown(id),
            scenevm::GeoId::Vertex(id) => IsoPaintOwner::Vertex(id),
            scenevm::GeoId::Linedef(id) => IsoPaintOwner::Linedef(id),
            scenevm::GeoId::Sector(id) => IsoPaintOwner::Sector(id),
            scenevm::GeoId::Character(id) => IsoPaintOwner::Character(id),
            scenevm::GeoId::Item(id) => IsoPaintOwner::Item(id),
            scenevm::GeoId::Light(id) => IsoPaintOwner::Light(id),
            scenevm::GeoId::ItemLight(id) => IsoPaintOwner::ItemLight(id),
            scenevm::GeoId::Triangle(id) => IsoPaintOwner::Triangle(id),
            scenevm::GeoId::Terrain(x, z) => IsoPaintOwner::Terrain { x, z },
            scenevm::GeoId::GeometryObject(id) => IsoPaintOwner::GeometryObject(id),
            scenevm::GeoId::Hole(sector_id, hole_id) => IsoPaintOwner::Hole { sector_id, hole_id },
            scenevm::GeoId::Gizmo(id) => IsoPaintOwner::Gizmo(id),
        }
    }

    fn paint_point(coord: Vec2<i32>, server_ctx: &ServerContext) -> IsoPaintPoint {
        let owner = server_ctx.geo_hit.map(Self::owner_from_geo_id);
        let world = if server_ctx.geo_hit.is_some() {
            Some(server_ctx.geo_hit_pos)
        } else {
            server_ctx.hover_cursor_3d
        };
        let surface_uv = server_ctx.hover_surface.as_ref().and_then(|surface| {
            server_ctx
                .hover_surface_hit_pos
                .map(|pos| surface.world_to_uv(pos))
        });
        let surface_normal = server_ctx.hover_surface_normal.or_else(|| {
            server_ctx
                .hover_surface
                .as_ref()
                .map(|surface| surface.plane.normal)
        });
        let camera_scale = RUSTERIX
            .read()
            .ok()
            .map(|rusterix| rusterix.client.camera_d3.scale());
        IsoPaintPoint::new([coord.x, coord.y], world, owner)
            .with_surface_uv(surface_uv)
            .with_surface_normal(surface_normal)
            .with_camera_scale(camera_scale)
    }

    fn request_paint_redraw(ctx: &mut TheContext) {
        ctx.ui.redraw_all = true;
    }

    fn reset_stroke(&mut self) {
        self.painting = false;
        self.active_stroke = None;
        self.stroke_before = None;
        self.stroke_changed = false;
    }
}

impl Tool for IsoPaintTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Iso Paint Tool"),
            painting: false,
            previous_dock: None,
            active_stroke: None,
            stroke_before: None,
            stroke_changed: false,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_iso_paint")
    }

    fn icon_name(&self) -> String {
        "paint-brush".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('I')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/iso-paint".to_string())
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                self.reset_stroke();
                server_ctx.curr_map_tool_type = MapToolType::IsoPaint;
                server_ctx.editor_view_mode = EditorViewMode::Iso;
                server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                server_ctx.hover_cursor = None;
                server_ctx.iso_paint_hover_screen = None;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.camera = MapCamera::ThreeDIso;
                    region.map.clear_selection();
                    region.map.clear_temp();
                }

                let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Iso Paint" {
                    self.previous_dock = if current_dock.is_empty() {
                        None
                    } else {
                        Some(current_dock)
                    };
                }
                DOCKMANAGER.write().unwrap().set_dock(
                    "Iso Paint".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );

                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
                true
            }
            DeActivate => {
                self.reset_stroke();
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                server_ctx.iso_paint_hover_screen = None;
                if DOCKMANAGER.read().unwrap().dock == "Iso Paint"
                    && let Some(prev) = self.previous_dock.take()
                {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .set_dock(prev, ui, ctx, project, server_ctx);
                }
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(_) => {
                self.painting = true;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    Self::hit_status(server_ctx),
                ));
            }
            MapDragged(_) => {
                if self.painting {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapHover(_) => {
                if !self.painting {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapUp(_) => {
                self.painting = false;
                server_ctx.iso_paint_hover_screen = None;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            MapEscape => {
                self.painting = false;
                server_ctx.iso_paint_hover_screen = None;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            _ => {}
        }

        None
    }

    fn region_map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        region: &mut Region,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(coord) => {
                self.painting = true;
                server_ctx.iso_paint_hover_screen = Some(coord);
                self.stroke_before = Some(region.clone());
                let stroke_id = region
                    .iso_paint
                    .begin_stroke(Self::paint_point(coord, server_ctx));
                self.active_stroke = Some(stroke_id);
                self.stroke_changed = true;
                Self::request_paint_redraw(ctx);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    Self::hit_status(server_ctx),
                ));
            }
            MapDragged(coord) => {
                server_ctx.iso_paint_hover_screen = Some(coord);
                if self.painting
                    && let Some(stroke_id) = self.active_stroke
                    && region
                        .iso_paint
                        .append_point(stroke_id, Self::paint_point(coord, server_ctx))
                {
                    self.stroke_changed = true;
                    Self::request_paint_redraw(ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapHover(coord) => {
                server_ctx.iso_paint_hover_screen = Some(coord);
                Self::request_paint_redraw(ctx);
                if !self.painting {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapUp(coord) => {
                server_ctx.iso_paint_hover_screen = Some(coord);
                if self.painting
                    && let Some(stroke_id) = self.active_stroke
                    && region
                        .iso_paint
                        .append_point(stroke_id, Self::paint_point(coord, server_ctx))
                {
                    self.stroke_changed = true;
                }

                let undo_atom = if self.stroke_changed {
                    self.stroke_before.take().map(|old_region| {
                        ProjectUndoAtom::RegionEdit(
                            ProjectContext::Region(region.id),
                            Box::new(old_region),
                            Box::new(region.clone()),
                        )
                    })
                } else {
                    None
                };

                self.reset_stroke();
                Self::request_paint_redraw(ctx);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
                return undo_atom;
            }
            MapEscape => {
                server_ctx.iso_paint_hover_screen = None;
                if let Some(old_region) = self.stroke_before.take() {
                    *region = old_region;
                }
                self.reset_stroke();
                Self::request_paint_redraw(ctx);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            _ => {}
        }

        None
    }
}
