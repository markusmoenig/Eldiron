use crate::prelude::*;
use rusterix::Surface;

pub struct ExtrudeSector {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ExtrudeSector {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        // Surface extrusion settings
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionSurfEnable".into(),
            "Surface Extrusion".into(),
            "When a sector (surface) is selected: turn on/off extrusion for that surface.".into(),
            true,
        ));

        let item = TheNodeUIItem::FloatEditSlider(
            "actionDepth".into(),
            "Depth".into(),
            "The extrusion depth.".into(),
            0.2,
            -20.0..=20.0,
            false,
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionBackOpen".into(),
            "Open back".into(),
            "Leave the back side uncapped; useful for facades/interiors.".into(),
            false,
        ));

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Sets **surface extrusion** on selected sectors, optionally with an open back.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Extrude Sector"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Extrudes the current linedef or sector."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'e'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 && server_ctx.editing_surface.is_some()
        {
            return false;
        }

        !map.selected_sectors.is_empty() && map.selected_linedefs.is_empty()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        let distance = self.nodeui.get_f32_value("actionDepth").unwrap_or(2.0);
        let surf_enable = self
            .nodeui
            .get_bool_value("actionSurfEnable")
            .unwrap_or(true);
        let back_open = self
            .nodeui
            .get_bool_value("actionBackOpen")
            .unwrap_or(false);

        // Apply to selected sectors: set/create surface extrusion settings
        for sector_id in map.selected_sectors.clone() {
            // Try to find an existing surface for this sector
            let mut surface_id_opt: Option<Uuid> = None;
            for (sid, s) in map.surfaces.iter() {
                if s.sector_id == sector_id {
                    surface_id_opt = Some(*sid);
                    break;
                }
            }

            // Create a new surface if needed
            if surface_id_opt.is_none() {
                if let Some(_sec) = map.find_sector(sector_id) {
                    let mut surf = Surface::new(sector_id);
                    surf.calculate_geometry(map);
                    let id = surf.id;
                    map.surfaces.insert(id, surf);
                    surface_id_opt = Some(id);
                }
            }

            if let Some(sid) = surface_id_opt {
                if let Some(surf) = map.surfaces.get_mut(&sid) {
                    // Distance directly sets depth; sign controls direction
                    surf.extrusion.enabled = surf_enable;
                    surf.extrusion.depth = distance;
                    surf.extrusion.cap_front = true; // always cap front
                    surf.extrusion.cap_back = !back_open; // optional back cap
                    surf.extrusion.flip_normal = false; // not exposed; depth sign handles direction
                    changed = true;
                }
            }
        }

        if changed {
            Some(RegionUndoAtom::MapEdit(
                Box::new(prev),
                Box::new(map.clone()),
            ))
        } else {
            None
        }
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
