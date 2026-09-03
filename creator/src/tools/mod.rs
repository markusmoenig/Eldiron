pub use crate::prelude::*;
use rusterix::Assets;

// pub mod code;
// pub mod config;
// pub mod data;
pub mod game;
pub mod geometry;
pub mod iso_paint;
// pub mod info;
pub mod blocks;
pub mod builder;
pub mod collision_probe;
pub mod linedef;
pub mod palette;
pub mod rect;
// pub mod render;
pub mod sector;
pub mod tile_picker;
pub mod wall;
// pub mod terrain;
// pub mod tileset;
pub mod effects;
pub mod entity;
pub mod vertex;

pub enum PanelIndices {
    TilePicker,
    ShadeGridFx,
}

pub fn draw_screen_rectangle_preview(buffer: &mut TheRGBABuffer, rect: (Vec2<f32>, Vec2<f32>)) {
    let min_x = rect.0.x.min(rect.1.x).round().max(0.0) as i32;
    let min_y = rect.0.y.min(rect.1.y).round().max(0.0) as i32;
    let max_x = rect
        .0
        .x
        .max(rect.1.x)
        .round()
        .min((buffer.dim().width - 1).max(0) as f32) as i32;
    let max_y = rect
        .0
        .y
        .max(rect.1.y)
        .round()
        .min((buffer.dim().height - 1).max(0) as f32) as i32;

    if max_x < min_x || max_y < min_y {
        return;
    }

    let dim = TheDim::new(
        min_x,
        min_y,
        (max_x - min_x).max(1) + 1,
        (max_y - min_y).max(1) + 1,
    );
    buffer.draw_rect_outline(&dim, &[255, 255, 255, 255]);
}

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

    /// Optional full-viewport editor owned by this persistent tool.
    fn editor_display(&self) -> Option<crate::editor_display::EditorDisplay> {
        None
    }

    /// Receive persistent display data edited by the shared viewport host.
    fn update_editor_display(&mut self, _display: &crate::editor_display::EditorDisplay) -> bool {
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
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
    ) -> Option<ProjectUndoAtom> {
        None
    }

    /// Per-frame update for interactions that continue while the pointer is held stationary.
    fn update(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _map: &mut Map,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }

    fn region_map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        region: &mut Region,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        self.map_event(map_event, ui, ctx, &mut region.map, server_ctx)
    }

    fn prefab_map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _asset_id: Uuid,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        project
            .prefab_editor_map
            .as_mut()
            .and_then(|map| self.map_event(map_event, ui, ctx, map, server_ctx))
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
        assets: &Assets,
    ) {
    }
}
