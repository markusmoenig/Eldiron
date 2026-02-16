use crate::prelude::*;
use rusterix::{PixelSource, Value};

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
            "".into(),
            "".into(),
            0.1,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Selector(
            "actionRecessTarget".into(),
            "".into(),
            "".into(),
            vec!["Front".to_string(), "Back".to_string()],
            1,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Icons(
            "actionRecessTiles".into(),
            "".into(),
            "".into(),
            vec![
                (
                    TheRGBABuffer::new(TheDim::sized(36, 36)),
                    "CAP".to_string(),
                    Uuid::nil(),
                ),
                (
                    TheRGBABuffer::new(TheDim::sized(36, 36)),
                    "SIDE".to_string(),
                    Uuid::nil(),
                ),
            ],
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_recess")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_recess_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
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

    fn load_params(&mut self, map: &Map) {
        if let Some(sector_id) = map.selected_sectors.first() {
            if let Some(sector) = map.find_sector(*sector_id) {
                self.nodeui.set_f32_value(
                    "actionRecessDepth",
                    sector.properties.get_float_default("profile_amount", 0.1),
                );
                self.nodeui.set_i32_value(
                    "actionRecessTarget",
                    sector.properties.get_int_default("profile_target", 1),
                );
            }
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut cap_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut jamb_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut cap_id = Uuid::nil();
        let mut jamb_id = Uuid::nil();

        if let Some(map) = project.get_map(server_ctx) {
            if let Some(sector_id) = map.selected_sectors.first() {
                if let Some(sector) = map.find_sector(*sector_id) {
                    if let Some(Value::Source(PixelSource::TileId(id))) =
                        sector.properties.get("cap_source")
                    {
                        if let Some(tile) = project.tiles.get(id)
                            && !tile.is_empty()
                        {
                            cap_icon = tile.textures[0].to_rgba();
                            cap_id = *id;
                        }
                    }
                    if let Some(Value::Source(PixelSource::TileId(id))) =
                        sector.properties.get("jamb_source")
                    {
                        if let Some(tile) = project.tiles.get(id)
                            && !tile.is_empty()
                        {
                            jamb_icon = tile.textures[0].to_rgba();
                            jamb_id = *id;
                        }
                    }
                }
            }
        }

        if let Some(item) = self.nodeui.get_item_mut("actionRecessTiles") {
            match item {
                TheNodeUIItem::Icons(_, _, _, items) => {
                    if items.len() == 2 {
                        items[0].0 = cap_icon;
                        items[0].2 = cap_id;
                        items[1].0 = jamb_icon;
                        items[1].2 = jamb_id;
                    }
                }
                _ => {}
            }
        }
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

        let depth = self
            .nodeui
            .get_f32_value("actionRecessDepth")
            .unwrap_or(0.0);

        let target = self.nodeui.get_i32_value("actionRecessTarget").unwrap_or(1);

        let cap = self.nodeui.get_tile_id("actionRecessTiles", 0);
        let jamb = self.nodeui.get_tile_id("actionRecessTiles", 1);

        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set("profile_op", Value::Int(2));
                sector.properties.set("profile_amount", Value::Float(depth));
                sector.properties.set("profile_target", Value::Int(target));

                if let Some(cap) = cap
                    && cap != Uuid::nil()
                {
                    sector.properties.set(
                        "cap_source",
                        Value::Source(rusterix::PixelSource::TileId(cap)),
                    );
                }
                if let Some(jamb) = jamb
                    && jamb != Uuid::nil()
                {
                    sector.properties.set(
                        "jamb_source",
                        Value::Source(rusterix::PixelSource::TileId(jamb)),
                    );
                }
                changed = true;
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
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::TileDropped(id, tile_id, index) = event {
            if let Some(item) = self.nodeui.get_item_mut(&id.name) {
                match item {
                    TheNodeUIItem::Icons(_, _, _, items) => {
                        if *index < items.len() {
                            if let Some(tile) = project.tiles.get(tile_id)
                                && !tile.is_empty()
                            {
                                items[*index].0 = tile.textures[0].to_rgba();
                                items[*index].2 = *tile_id;
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Action List"),
                                    TheValue::Empty,
                                ));
                                return true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        self.nodeui.handle_event(event)
    }
}
