use crate::editor::DOCKMANAGER;
use crate::prelude::*;

pub struct SetTileMaterial {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for SetTileMaterial {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::FloatEditSlider(
            "actionSetTileMaterialRoughness".into(),
            "".into(),
            "".into(),
            0.5,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionSetTileMaterialMetallic".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionSetTileMaterialOpacity".into(),
            "".into(),
            "".into(),
            1.0,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionSetTileMaterialEmissive".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_set_tile_material")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_set_tile_material_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'a'))
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Tiles" && server_ctx.curr_tile_id.is_some()
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let roughness = self
            .nodeui
            .get_f32_value("actionSetTileMaterialRoughness")
            .unwrap_or(0.5);

        let metallic = self
            .nodeui
            .get_f32_value("actionSetTileMaterialMetallic")
            .unwrap_or(0.0);

        let opacity = self
            .nodeui
            .get_f32_value("actionSetTileMaterialOpacity")
            .unwrap_or(1.0);

        let emissive = self
            .nodeui
            .get_f32_value("actionSetTileMaterialEmissive")
            .unwrap_or(0.0);

        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                for texture in &mut tile.textures {
                    texture.set_materials_all(roughness, metallic, opacity, emissive);
                }
            }
        }

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tiles"),
            TheValue::Empty,
        ));

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Render SceneManager Map"),
            TheValue::Empty,
        ));
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
