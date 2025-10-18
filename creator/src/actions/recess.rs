use crate::prelude::*;

pub struct Recess {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for Recess {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::FloatEditSlider(
            "actionRecessDepth".into(),
            "Depth".into(),
            "The depth of the recess.".into(),
            0.1,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Creates a recess in the selected profile sector.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Recess"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Creates a recess for the profile sector."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Profile
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'r'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        // Only applicable if we have selected sectors and we are in 2D Profile Editing Mode
        !map.selected_sectors.is_empty()
            && server_ctx.editor_view_mode == EditorViewMode::D2
            && server_ctx.editing_surface.is_some()
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

        let depth = self
            .nodeui
            .get_f32_value("actionRecessDepth")
            .unwrap_or(0.0);

        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set("profile_op", Value::Int(2));
                sector.properties.set("profile_depth", Value::Float(depth));
                changed = true;
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

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
