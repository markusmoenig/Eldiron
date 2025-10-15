pub use crate::prelude::*;
// use rusterix::Assets;

pub mod apply_shader;
pub mod apply_tile;
pub mod clear_shader;
pub mod clear_tile;
pub mod extrude;
pub mod toggle_rect_geo;

#[allow(unused)]
pub trait Action: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> &'static str;
    fn role(&self) -> &'static str;

    fn accel(&self) -> Option<char> {
        None
    }

    fn is_applicable(&self, map: &Map, ctx: &mut TheContext, server_ctx: &ServerContext) -> bool;

    fn apply(
        &self,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom>;

    fn params(&self) -> TheNodeUI;
    fn handle_event(&mut self, event: &TheEvent) -> bool;
}
