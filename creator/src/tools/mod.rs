pub use crate::prelude::*;

pub mod code;
pub mod draw;
pub mod eraser;
pub mod fx;
pub mod game;
pub mod mapobjects;
pub mod material;
pub mod model;
pub mod picker;
pub mod render;
pub mod resize;
pub mod screen;
pub mod selection;
pub mod terrain;
pub mod tiledrawer;
pub mod tilemap;
pub mod zoom;

#[derive(PartialEq, Clone, Debug, Copy)]
pub enum ToolEvent {
    Activate,
    DeActivate,
    TileDown(Vec2i, Vec2f),
    TileDrag(Vec2i, Vec2f),
    TileUp,
}

#[derive(PartialEq, Clone, Debug, Copy)]
pub enum ToolContext {
    TwoD,
    ThreeD,
}

#[allow(unused)]
pub trait Tool: Send {
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
        server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_mask(
        &self,
        material_offset: usize,
        buffer: &mut TheRGBBuffer,
        p: Vec2f,
        coord: Vec2f,
        material_index: u8,
        brush: &dyn Brush,
        settings: &BrushSettings,
    ) {
    }
}
