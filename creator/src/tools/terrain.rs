use crate::editor::{NODEEDITOR, WORLDEDITOR};
use crate::prelude::*;
use ToolEvent::*;

pub struct TerrainTool {
    id: TheId,
}

impl Tool for TerrainTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("World Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!(
            "Terrain Tool (W). Use brushes to edit terrain, paint with tiles, materials and rules."
        )
    }
    fn icon_name(&self) -> String {
        str!("terrain")
    }
    fn accel(&self) -> Option<char> {
        Some('W')
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
                //server_ctx.curr_map_tool_type = MapToolType::World;

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    let mut switch = TheGroupButton::new(TheId::named("World Helper Switch"));
                    switch.add_text_status(
                        "Brushes".to_string(),
                        "Edit the terrain with brushes.".to_string(),
                    );
                    switch.add_text_status(
                        "Tile Picker".to_string(),
                        "Pick and paint with tiles.".to_string(),
                    );
                    switch.add_text_status(
                        "Materials".to_string(),
                        "Pick and paint with procedural materials.".to_string(),
                    );
                    switch.add_text_status(
                        "Global".to_string(),
                        "Edit the global render graph.".to_string(),
                    );
                    switch.set_item_width(80);
                    switch.set_index(server_ctx.curr_world_tool_helper as i32);
                    layout.add_widget(Box::new(switch));

                    let mut camera_switch =
                        TheGroupButton::new(TheId::named("World Camera Helper Switch"));
                    camera_switch.add_text_status(
                        "Orbit".to_string(),
                        if cfg!(target_os = "macos") {
                            str!("Orbit Camera. Hold 'Cmd' and click-drag to rotate about center.")
                        } else {
                            str!("Orbit Camera. Hold 'Ctrl' and click-drag to rotate about center.")
                        },
                    );
                    camera_switch.add_text_status(
                        "FirstP".to_string(),
                        "Apply procedural materials.".to_string(),
                    );
                    camera_switch.set_index(server_ctx.curr_world_tool_camera as i32);
                    layout.add_widget(Box::new(camera_switch));

                    if server_ctx.curr_world_tool_helper == WorldToolHelper::Brushes {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TerrainBrush as usize,
                        ));
                    } else if server_ctx.curr_world_tool_helper == WorldToolHelper::TilePicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TilePicker as usize,
                        ));
                    }
                    /* else

                    if server_ctx.curr_world_tool_helper == WorldToolHelper::MaterialPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::MaterialPicker as usize,
                        ));
                    } */
                    else if server_ctx.curr_world_tool_helper == WorldToolHelper::GlobalRender {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::NodeEditor as usize,
                        ));
                    }

                    layout.set_reverse_index(Some(1));
                }

                if server_ctx.curr_world_tool_helper == WorldToolHelper::GlobalRender {
                    NODEEDITOR.write().unwrap().set_context(
                        NodeContext::GlobalRender,
                        ui,
                        ctx,
                        project,
                        server_ctx,
                    );
                }

                WORLDEDITOR.write().unwrap().update_brush_preview(ui);

                server_ctx.world_mode = true;
                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                // server_ctx.curr_map_tool_type = MapToolType::General;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.terrain.mark_dirty();
                }
                server_ctx.world_mode = false;
                return true;
            }
            _ => {}
        };
        false
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        WORLDEDITOR
            .write()
            .unwrap()
            .map_event(map_event, ui, ctx, map, server_ctx)
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        match event {
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == "PolyView" {
                    WORLDEDITOR
                        .write()
                        .unwrap()
                        .scroll_by(ui, ctx, server_ctx, *coord);
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "World Helper Switch" {
                    server_ctx.curr_world_tool_helper.set_from_index(*index);
                    if server_ctx.curr_world_tool_helper == WorldToolHelper::Brushes {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TerrainBrush as usize,
                        ));
                    } else if server_ctx.curr_world_tool_helper == WorldToolHelper::TilePicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TilePicker as usize,
                        ));
                        WORLDEDITOR
                            .write()
                            .unwrap()
                            .set_tile_rules_ui(ui, ctx, true);
                    }
                    /* else
                    if server_ctx.curr_world_tool_helper == WorldToolHelper::MaterialPicker {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::MaterialPicker as usize,
                        ));
                        WORLDEDITOR
                            .write()
                            .unwrap()
                            .set_tile_rules_ui(ui, ctx, true);
                    }*/
                    else if server_ctx.curr_world_tool_helper == WorldToolHelper::GlobalRender {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::NodeEditor as usize,
                        ));

                        NODEEDITOR.write().unwrap().set_context(
                            NodeContext::GlobalRender,
                            ui,
                            ctx,
                            project,
                            server_ctx,
                        );
                    }
                } else if id.name == "Brush Type" {
                    WORLDEDITOR
                        .write()
                        .unwrap()
                        .brush_type
                        .set_from_index(*index);
                }
            }
            TheEvent::ValueChanged(id, TheValue::IntRange(v, _)) => {
                if id.name == "tileRulesBlendRadius" {
                    WORLDEDITOR.write().unwrap().blend_radius = *v;
                }
            }
            TheEvent::ValueChanged(id, TheValue::FloatRange(v, _)) => {
                if id.name == "Brush Radius" {
                    WORLDEDITOR.write().unwrap().radius = *v;
                    WORLDEDITOR.write().unwrap().update_brush_preview(ui);
                } else if id.name == "Brush Falloff" {
                    WORLDEDITOR.write().unwrap().falloff = *v;
                    WORLDEDITOR.write().unwrap().update_brush_preview(ui);
                } else if id.name == "Brush Strength" {
                    WORLDEDITOR.write().unwrap().strength = *v;
                    WORLDEDITOR.write().unwrap().update_brush_preview(ui);
                } else if id.name == "Brush Fixed" {
                    WORLDEDITOR.write().unwrap().fixed = *v;
                    WORLDEDITOR.write().unwrap().update_brush_preview(ui);
                } else if id.name == "tileRulesDistance" {
                    WORLDEDITOR.write().unwrap().tile_rules_distance = *v;
                } else if id.name == "tileRulesHeight" {
                    WORLDEDITOR.write().unwrap().tile_rules_height = *v;
                } else if id.name == "tileRulesSteepness" {
                    WORLDEDITOR.write().unwrap().tile_rules_steepness = *v;
                }
            }
            TheEvent::ValueChanged(id, TheValue::Int(v)) => {
                if id.name == "tileRules" {
                    WORLDEDITOR.write().unwrap().tile_rules = *v == 0;
                }
            }
            _ => {}
        }
        redraw
    }
}
