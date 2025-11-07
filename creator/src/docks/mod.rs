pub mod code;
pub mod data;
pub mod tiles;

pub use crate::prelude::*;

#[allow(unused)]
pub trait Dock: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn setup(&mut self, ctx: &mut TheContext) -> TheCanvas;

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
    }

    fn deactivate(&mut self) {}

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
}
