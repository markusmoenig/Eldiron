pub use crate::prelude::*;
// use rusterix::Assets;

pub mod apply_tile;

#[allow(unused)]
pub trait Action: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> String;

    fn accel(&self) -> Option<char> {
        None
    }

    fn is_applicable(&self, map: &Map, ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        false
    }

    fn apply(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }

    fn params(&self) -> TheNodeUI;
}
