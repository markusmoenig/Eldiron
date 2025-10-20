use crate::prelude::*;
use rusterix::PixelSource;

pub struct Relief {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for Relief {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::FloatEditSlider(
            "actionReliefHeight".into(),
            "Height".into(),
            "The height of the relief (emboss).".into(),
            0.1,       // default
            0.0..=1.0, // range
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Checkbox(
            "actionReliefTile".into(),
            "Apply Tile".into(),
            "Applies the current tile to the relief.".into(),
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Creates a relief (emboss) on the selected profile sector.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Relief"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Creates a relief (emboss) for the profile sector."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Profile
    }

    fn accel(&self) -> Option<TheAccelerator> {
        // Alt+E (Emboss/Relief). Change if you have a conflict.
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'e'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        // Only in 2D Profile Editing Mode with sector selection
        !map.selected_sectors.is_empty()
            && server_ctx.editor_view_mode == EditorViewMode::D2
            && server_ctx.editing_surface.is_some()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        let mut height = self
            .nodeui
            .get_f32_value("actionReliefHeight")
            .unwrap_or(0.0);
        if height < 0.0 {
            height = 0.0;
        }

        let apply_tile = self
            .nodeui
            .get_bool_value("actionReliefTile")
            .unwrap_or(false);

        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set("profile_op", Value::Int(1)); // Relief
                sector
                    .properties
                    .set("profile_height", Value::Float(height));

                sector.properties.set("profile_target", Value::Int(1));

                if let Some(tile_id) = server_ctx.curr_tile_id
                    && apply_tile
                {
                    sector
                        .properties
                        .set("relief_source", Value::Source(PixelSource::TileId(tile_id)));
                    sector.properties.set(
                        "relief_jamb_source",
                        Value::Source(PixelSource::TileId(tile_id)),
                    );
                }
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
