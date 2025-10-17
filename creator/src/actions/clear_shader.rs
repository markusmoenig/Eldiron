use crate::prelude::*;

pub struct ClearShader {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ClearShader {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Clears the shaders from the selected sectors.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Clear Shader"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Clears the shader from the selected sectors."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Property
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        !map.selected_sectors.is_empty()
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

        let mut to_remove = vec![];

        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                if let Some(shader_id) = sector.shader {
                    to_remove.push(shader_id);
                    sector.shader = None;
                    changed = true;
                }
            }
        }

        for s in to_remove {
            map.shaders.shift_remove(&s);
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
