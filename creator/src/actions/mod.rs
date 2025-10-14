pub use crate::prelude::*;
// use rusterix::Assets;

pub mod apply_shader;
pub mod apply_tile;
pub mod extrude;

#[allow(unused)]
pub trait Action: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> String;
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
