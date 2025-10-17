pub use crate::prelude::*;
// use rusterix::Assets;

pub mod add_shader_library;
pub mod apply_shader;
pub mod apply_tile;
pub mod clear_shader;
pub mod clear_tile;
pub mod edit_linedef;
pub mod edit_sector;
pub mod edit_vertex;
pub mod extrude;
pub mod load_shader;
pub mod new_shader;
pub mod split;
pub mod toggle_rect_geo;

pub enum ActionRole {
    Geometry,
    Property,
    UI,
}

impl ActionRole {
    pub fn to_color(&self) -> [u8; 4] {
        match self {
            ActionRole::Geometry => [195, 170, 150, 255],
            ActionRole::Property => [160, 175, 190, 255],
            ActionRole::UI => [200, 195, 150, 255],
        }
    }
}

#[allow(unused)]
pub trait Action: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> &'static str;
    fn role(&self) -> ActionRole;

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, ctx: &mut TheContext, server_ctx: &ServerContext) -> bool;

    fn load_params(&mut self, map: &Map) {}

    fn apply(
        &self,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom>;

    fn params(&self) -> TheNodeUI;
    fn handle_event(&mut self, event: &TheEvent) -> bool;
}
