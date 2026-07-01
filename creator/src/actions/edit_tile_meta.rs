use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use rusterix::material_library::{
    MATERIAL_FINISH_NAMES as MATERIAL_FINISH_VALUES,
    MATERIAL_PRESET_NAMES as MATERIAL_PRESET_VALUES,
};
use rusterix::{TileMaterialMeta, TileRole};

const PROCEDURAL_KIND_VALUES: [&str; 5] = ["none", "floor", "wall", "entrance", "exit"];

pub struct EditTileMeta {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditTileMeta {
    fn material_preset_labels() -> Vec<String> {
        vec![
            fl!("material_preset_default"),
            fl!("material_preset_stone"),
            fl!("material_preset_dirt"),
            fl!("material_preset_wood"),
            fl!("material_preset_metal"),
            fl!("material_preset_glass"),
            fl!("material_preset_water"),
            fl!("material_preset_mirror"),
            fl!("material_preset_emissive"),
            fl!("material_preset_fabric"),
            fl!("material_preset_plastic"),
            fl!("material_preset_foliage"),
            fl!("material_preset_skin"),
            fl!("material_preset_bone"),
            fl!("material_preset_wax"),
        ]
    }

    fn material_finish_labels() -> Vec<String> {
        vec![
            fl!("material_finish_natural"),
            fl!("material_finish_matte"),
            fl!("material_finish_polished"),
            fl!("material_finish_wet"),
        ]
    }
}

impl Action for EditTileMeta {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let mut roles = vec![];
        for dir in TileRole::iterator() {
            roles.push(dir.to_string().to_string());
        }
        let item = TheNodeUIItem::Selector("actionTileRole".into(), "".into(), "".into(), roles, 1);
        nodeui.add_item(item);

        let item = TheNodeUIItem::Selector(
            "actionTileBlocking".into(),
            "".into(),
            "".into(),
            vec!["No".to_string(), "Yes".to_string()],
            0,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Text(
            "actionTileAlias".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::OpenTree("procedural".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionTileProceduralStyle".into(),
            fl!("action_tile_procedural_style"),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionTileProceduralKind".into(),
            fl!("action_tile_procedural_kind"),
            "".into(),
            PROCEDURAL_KIND_VALUES
                .iter()
                .map(|kind| kind.to_string())
                .collect(),
            0,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionTileProceduralWeight".into(),
            fl!("action_tile_procedural_weight"),
            "".into(),
            1,
            1..=100,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "materialPreset".into(),
            fl!("material_preset"),
            fl!("status_material_preset"),
            Self::material_preset_labels(),
            0,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "materialFinish".into(),
            fl!("material_finish"),
            fl!("status_material_finish"),
            Self::material_finish_labels(),
            0,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_edit_tile")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_tile_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Tiles" && server_ctx.curr_tile_id.is_some()
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.get_tile(&tile_id) {
                self.nodeui
                    .set_i32_value("actionTileRole", tile.role as i32);
                self.nodeui
                    .set_i32_value("actionTileBlocking", if tile.blocking { 1 } else { 0 });
                self.nodeui
                    .set_text_value("actionTileAlias", tile.alias.clone());
                self.nodeui
                    .set_text_value("actionTileProceduralStyle", tile.procedural.style.clone());
                let kind_index = PROCEDURAL_KIND_VALUES
                    .iter()
                    .position(|kind| *kind == tile.procedural.kind.trim())
                    .unwrap_or(0) as i32;
                self.nodeui
                    .set_i32_value("actionTileProceduralKind", kind_index);
                self.nodeui.set_i32_value(
                    "actionTileProceduralWeight",
                    tile.procedural.weight.max(1) as i32,
                );
                let preset_index = MATERIAL_PRESET_VALUES
                    .iter()
                    .position(|preset| preset.eq_ignore_ascii_case(tile.material.preset.trim()))
                    .unwrap_or(0) as i32;
                let finish_index = MATERIAL_FINISH_VALUES
                    .iter()
                    .position(|finish| finish.eq_ignore_ascii_case(tile.material.finish.trim()))
                    .unwrap_or(0) as i32;
                self.nodeui.set_i32_value("materialPreset", preset_index);
                self.nodeui.set_i32_value("materialFinish", finish_index);
            }
        }
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let role = self.nodeui.get_i32_value("actionTileRole").unwrap_or(0);
        let blocking = self.nodeui.get_i32_value("actionTileBlocking").unwrap_or(0);
        let name = self
            .nodeui
            .get_text_value("actionTileAlias")
            .unwrap_or(String::new());
        let proc_style = self
            .nodeui
            .get_text_value("actionTileProceduralStyle")
            .unwrap_or_default();
        let proc_kind_index = self
            .nodeui
            .get_i32_value("actionTileProceduralKind")
            .unwrap_or(0)
            .max(0) as usize;
        let proc_kind = PROCEDURAL_KIND_VALUES
            .get(proc_kind_index)
            .copied()
            .unwrap_or("none");
        let proc_weight = self
            .nodeui
            .get_i32_value("actionTileProceduralWeight")
            .unwrap_or(1)
            .max(1) as u32;
        let material_preset_index = self
            .nodeui
            .get_i32_value("materialPreset")
            .unwrap_or(0)
            .max(0) as usize;
        let material_finish_index = self
            .nodeui
            .get_i32_value("materialFinish")
            .unwrap_or(0)
            .max(0) as usize;
        let material_preset = MATERIAL_PRESET_VALUES
            .get(material_preset_index)
            .copied()
            .unwrap_or("default");
        let mut material_finish = MATERIAL_FINISH_VALUES
            .get(material_finish_index)
            .copied()
            .unwrap_or("natural");
        if material_preset == "default" {
            material_finish = "natural";
        }

        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.get_tile_mut(&tile_id) {
                let role = TileRole::from_index(role as u8);
                let blocking = blocking == 1;

                tile.role = role;
                tile.blocking = blocking;
                tile.alias = name.clone();
                tile.procedural.style = proc_style.trim().to_string();
                tile.procedural.kind = if proc_kind == "none" {
                    String::new()
                } else {
                    proc_kind.to_string()
                };
                tile.procedural.weight = proc_weight;
                tile.material = TileMaterialMeta {
                    preset: material_preset.to_string(),
                    finish: material_finish.to_string(),
                };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::nodeui_to_toml;

    #[test]
    fn edit_tile_meta_toml_has_material_group() {
        let action = EditTileMeta::new();
        let toml = nodeui_to_toml(&action.params());

        assert!(toml.contains("[material]\n"));
        assert!(toml.contains("preset = \"Default\""));
        assert!(toml.contains("finish = \"Natural\""));
    }
}
