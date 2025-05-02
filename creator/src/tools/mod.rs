pub use crate::prelude::*;

pub mod code;
pub mod config;
pub mod data;
pub mod game;
pub mod info;
pub mod linedef;
pub mod rect;
pub mod sector;
pub mod selection;
pub mod shape;
pub mod tileset;
pub mod vertex;
pub mod world;

#[derive(PartialEq, Clone, Debug, Copy)]
pub enum ToolEvent {
    Activate,
    DeActivate,

    TileDown(Vec2<i32>, Vec2<f32>),
    TileDrag(Vec2<i32>, Vec2<f32>),
    TileUp,
}

#[derive(PartialEq, Clone, Debug, Copy)]
pub enum MapEvent {
    MapClicked(Vec2<i32>),
    MapDragged(Vec2<i32>),
    MapHover(Vec2<i32>),
    MapUp(Vec2<i32>),
    MapDelete,
    MapEscape,
    MapKey(char),
}

#[derive(PartialEq, Clone, Debug, Copy)]
pub enum ToolContext {
    TwoD,
    ThreeD,
}

#[allow(unused)]
pub trait Tool: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> String;
    fn icon_name(&self) -> String;

    fn accel(&self) -> Option<char> {
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
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
        None
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        palette: &ThePalette,
    ) {
    }

    fn activate_map_tool_helper(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(layout) = ui.get_hlayout("Game Tool Params") {
            layout.clear();

            let mut source_switch = TheGroupButton::new(TheId::named("Map Helper Switch"));
            source_switch.add_text_status("Tiles".to_string(), "Pick and place tiles.".to_string());
            source_switch.add_text_status(
                "Materials".to_string(),
                "Pick and place procedural materials.".to_string(),
            );
            source_switch.add_text_status("Nodes".to_string(), "Work with nodes.".to_string());
            source_switch.add_text_status(
                "Effects".to_string(),
                "Add visual effects to shapes.".to_string(),
            );
            source_switch.add_text_status(
                "Preview".to_string(),
                "Preview the final map output.".to_string(),
            );
            source_switch.set_item_width(80);
            source_switch.set_index(server_ctx.curr_map_tool_helper as i32);
            layout.add_widget(Box::new(source_switch));

            if server_ctx.curr_map_tool_helper == MapToolHelper::TilePicker {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::TilePicker as usize,
                ));
            } else if server_ctx.curr_map_tool_helper == MapToolHelper::MaterialPicker {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::MaterialPicker as usize,
                ));
            } else if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::NodeEditor as usize,
                ));
            } else if server_ctx.curr_map_tool_helper == MapToolHelper::EffectsPicker {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::EffectPicker as usize,
                ));
            } else if server_ctx.curr_map_tool_helper == MapToolHelper::Preview {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::PreviewView as usize,
                ));
            }

            let mut set_source_button = TheTraybarButton::new(TheId::named("Apply Map Properties"));
            set_source_button.set_status_text("Apply the source to the selected geometry.");
            set_source_button.set_text("Apply".to_string());
            layout.add_widget(Box::new(set_source_button));

            let mut rem_source_button =
                TheTraybarButton::new(TheId::named("Remove Map Properties"));
            rem_source_button.set_status_text("Remove the source from the selected geometry.");
            rem_source_button.set_text("Remove".to_string());
            layout.add_widget(Box::new(rem_source_button));

            layout.set_reverse_index(Some(2));
        }
    }
}
