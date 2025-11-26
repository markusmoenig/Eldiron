use crate::prelude::*;
use rusterix::PixelSource;

pub struct ApplyTile {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ApplyTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_apply_tile"));
        nodeui.add_item(item);

        Self {
            id: TheId::named("Apply Tile"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Applies the current tile to the selected sectors."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'a'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        !map.selected_sectors.is_empty()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        if let Some(tile_id) = server_ctx.curr_tile_id {
            for sector_id in &map.selected_sectors.clone() {
                if let Some(sector) = map.find_sector_mut(*sector_id) {
                    let mut source = "source";

                    if server_ctx.pc.is_screen() {
                        if server_ctx.selected_hud_icon_index == 1 {
                            source = "ceiling_source";
                        }
                    }

                    sector
                        .properties
                        .set(source, Value::Source(PixelSource::TileId(tile_id)));
                    changed = true;
                }
            }
        }

        if changed {
            Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
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
