pub use crate::prelude::*;
use rusterix::Assets;

// pub mod code;
// pub mod config;
// pub mod data;
pub mod game;
pub mod geometry;
pub mod iso_paint;
pub(crate) mod paint_overlay;
pub(crate) mod paint_picker;
// pub mod info;
pub mod blocks;
pub mod collision_probe;
pub mod linedef;
pub mod palette;
pub(crate) mod props_overlay;
pub(crate) mod props_recipe;
pub mod rect;
// pub mod render;
pub mod sector;
pub mod terrain;
pub mod terrain_overlay;
pub mod rect_overlay;
pub(crate) mod paint_preview;
pub mod tile_picker;
// pub mod tileset;
pub mod effects;
pub mod entity;
pub mod overlay_frame;
pub mod vertex;
mod view_input;
pub use view_input::{ToolPaintInput, ToolViewInput};
pub use overlay_frame::{
    ToolOverlayCommand, ToolOverlayFrame, ToolOverlayInputs, ToolOverlayTarget,
};

pub enum PanelIndices {
    TilePicker,
    ShadeGridFx,
}

fn screen_rectangle_preview_dim(
    size: (usize, usize),
    rect: (Vec2<f32>, Vec2<f32>),
) -> Option<TheDim> {
    let min_x = rect.0.x.min(rect.1.x).round().max(0.0) as i32;
    let min_y = rect.0.y.min(rect.1.y).round().max(0.0) as i32;
    let max_x = rect
        .0
        .x
        .max(rect.1.x)
        .round()
        .min((size.0 as i32 - 1).max(0) as f32) as i32;
    let max_y = rect
        .0
        .y
        .max(rect.1.y)
        .round()
        .min((size.1 as i32 - 1).max(0) as f32) as i32;

    if max_x < min_x || max_y < min_y {
        return None;
    }

    let dim = TheDim::new(
        min_x,
        min_y,
        (max_x - min_x).max(1) + 1,
        (max_y - min_y).max(1) + 1,
    );
    Some(dim)
}

pub fn draw_screen_rectangle_preview(buffer: &mut TheRGBABuffer, rect: (Vec2<f32>, Vec2<f32>)) {
    let size = (
        buffer.dim().width.max(0) as usize,
        buffer.dim().height.max(0) as usize,
    );
    if let Some(dim) = screen_rectangle_preview_dim(size, rect) {
        buffer.draw_rect_outline(&dim, &[255, 255, 255, 255]);
    }
}
pub(crate) fn prepare_screen_rectangle_preview(
    size: (usize, usize),
    rect: (Vec2<f32>, Vec2<f32>),
) -> crate::editor_overlay::EditorOverlay {
    let mut commands = crate::editor_overlay::EditorOverlay::new(size);
    if let Some(dim) = screen_rectangle_preview_dim(size, rect) {
        commands.signed_outline(&dim, &[255, 255, 255, 255]);
    }
    commands
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

    /// The viewport supplies checked CPU queries immediately before dispatch.
    fn set_view_input(&mut self, _input: Option<ToolViewInput>) {}
    /// Camera of the current explicitly supplied authoring query, if retained.
    fn view_input_camera(&self) -> Option<scenevm::Camera3D> {
        None
    }
    /// Request CPU authoring surface data for this event, independently of paint.
    fn view_surface_capture(&self, _event: &MapEvent) -> bool { false }
    fn sync_overlay_sources(&mut self, _project: &Project, _server: &ServerContext) {}
    fn overlay_paint(&mut self) -> Option<&mut paint_overlay::PaintOverlay> { None }
    fn clear_paint_overlay_hover(&mut self, _server: &mut ServerContext) {}
    fn view_rectangle_query(
        &self,
        _point: Vec2<i32>,
    ) -> Option<(Vec2<f32>, Vec2<f32>, scenevm::GeoId)> {
        None
    }
    /// Optional common HUD controller. Unconverted tools return no native frame.
    fn overlay_hud(&mut self) -> Option<&mut crate::hud::Hud> {
        None
    }
    fn uses_native_overlay_input(&mut self) -> bool {
        self.overlay_hud().is_some_and(|hud| hud.uses_native_input())
    }
    fn set_native_overlay_input(&mut self, native: bool) {
        if let Some(hud) = self.overlay_hud() {
            hud.set_native_input(native);
        }
    }
    fn hud_selected_id(&self, _map: &Map, _server: &ServerContext) -> Option<u32> {
        None
    }
    fn overlay_screen_rectangle(
        &self,
        _map: &Map,
        _server: &ServerContext,
    ) -> Option<(Vec2<f32>, Vec2<f32>)> {
        None
    }
    fn prepare_overlay(&mut self, input: ToolOverlayInputs<'_>) -> Option<ToolOverlayFrame> {
        ToolOverlayFrame::prepare_standard(self, input)
    }
    fn invalidate_overlay(&mut self) {
        if let Some(hud) = self.overlay_hud() {
            hud.invalidate_frame();
        }
    }
    fn overlay_input_current(
        &mut self,
        frame: &ToolOverlayFrame,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> bool {
        self.overlay_hud()
            .is_some_and(|hud| frame.hud_input_current(hud, target, map, server))
    }
    fn overlay_command_source_current(
        &mut self,
        frame: &ToolOverlayFrame,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> bool {
        self.overlay_hud()
            .is_some_and(|hud| frame.hud_command_source_current(hud, target, map, server))
    }
    fn apply_rect_overlay(
        &mut self, _action: rect_overlay::Action, _map: &mut Map,
        _server: &mut ServerContext,
    ) -> Option<Vec<TheEvent>> { None }
    fn apply_terrain_overlay(
        &mut self,
        _action: terrain_overlay::Action,
        _map: &Map,
        _server: &mut ServerContext,
        _ctx: &mut TheContext,
    ) -> bool {
        false
    }
    fn show_overlay_popup(
        &mut self,
        _command: &ToolOverlayCommand,
        _server: &ServerContext,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
    ) -> bool {
        false
    }
    #[cfg(feature = "native-ui")]
    fn presented_overlay(
        &mut self,
        frame: &ToolOverlayFrame,
        receipts: &[theframework::paint::FrameReceipt],
        target: ToolOverlayTarget,
        map: &Map,
        server: &mut ServerContext,
    ) -> bool {
        let accepted = frame.presented_hud(self, receipts, target, map, server);
        if !accepted {
            self.invalidate_overlay();
        }
        accepted
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

#[cfg(test)]
mod iso_geometry_tests;
