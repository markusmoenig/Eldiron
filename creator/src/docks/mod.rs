pub mod code;
pub mod data;
pub mod tilemap;
pub mod tiles;

pub use crate::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum DockDefaultState {
    Minimized,
    Maximized,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DockMaximizedState {
    Maximized,
    Editor,
}

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

    fn supports_actions(&self) -> bool {
        true
    }

    fn default_state(&self) -> DockDefaultState {
        DockDefaultState::Minimized
    }

    fn maximized_state(&self) -> DockMaximizedState {
        DockMaximizedState::Maximized
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
}
