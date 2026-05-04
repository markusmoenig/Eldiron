use crate::prelude::*;
use rusterix::Surface;
use std::collections::HashSet;

pub struct MakeSectorRectangular {
    id: TheId,
    nodeui: TheNodeUI,
}

impl MakeSectorRectangular {
    fn ordered_unique_vertices(map: &Map, sector_id: u32) -> Option<Vec<u32>> {
        let sector = map.find_sector(sector_id)?;
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for linedef_id in &sector.linedefs {
            let linedef = map.find_linedef(*linedef_id)?;
            if seen.insert(linedef.start_vertex) {
                out.push(linedef.start_vertex);
            }
            if seen.insert(linedef.end_vertex) {
                out.push(linedef.end_vertex);
            }
        }
        (out.len() == 4).then_some(out)
    }
}

impl Action for MakeSectorRectangular {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            "Moves the selected four-corner sector vertices onto its bounding rectangle.".into(),
        ));

        Self {
            id: TheId::named("Make Sector Rectangular"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Move the selected sector vertices onto a rectangular bounding box.".into()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        map.selected_sectors.len() == 1
            && Self::ordered_unique_vertices(map, map.selected_sectors[0]).is_some()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let sector_id = *map.selected_sectors.first()?;
        let vertex_ids = Self::ordered_unique_vertices(map, sector_id)?;
        let bbox = map.find_sector(sector_id)?.bounding_box(map);
        if !bbox.min.x.is_finite()
            || !bbox.min.y.is_finite()
            || !bbox.max.x.is_finite()
            || !bbox.max.y.is_finite()
            || (bbox.max.x - bbox.min.x).abs() <= 0.001
            || (bbox.max.y - bbox.min.y).abs() <= 0.001
        {
            return None;
        }

        let prev = map.clone();
        let center = (bbox.min + bbox.max) * 0.5;
        for vertex_id in vertex_ids {
            let Some(vertex) = map.find_vertex(vertex_id).cloned() else {
                continue;
            };
            let target_x = if vertex.x < center.x {
                bbox.min.x
            } else {
                bbox.max.x
            };
            let target_y = if vertex.y < center.y {
                bbox.min.y
            } else {
                bbox.max.y
            };
            map.update_vertex(vertex_id, Vec2::new(target_x, target_y));
        }

        let surface_ids = map
            .surfaces
            .iter()
            .filter_map(|(id, surface)| (surface.sector_id == sector_id).then_some(*id))
            .collect::<Vec<_>>();
        for surface_id in &surface_ids {
            if let Some(mut surface) = map.surfaces.shift_remove(surface_id) {
                surface.calculate_geometry(map);
                map.surfaces.insert(surface.id, surface);
            }
        }
        if surface_ids.is_empty() {
            let mut surface = Surface::new(sector_id);
            surface.calculate_geometry(map);
            map.surfaces.insert(surface.id, surface);
        }

        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
