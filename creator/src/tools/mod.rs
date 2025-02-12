pub use crate::prelude::*;

pub mod code;
pub mod data;
pub mod fx;
pub mod game;
pub mod linedef;
pub mod rect;
pub mod sector;
pub mod selection;
pub mod settings;
pub mod tileset;
pub mod vertex;

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
    ) {
    }
}
