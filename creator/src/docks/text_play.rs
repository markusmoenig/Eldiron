use crate::editor::TEXTGAME;
use crate::prelude::*;

pub struct TextPlayDock;

impl Dock for TextPlayDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        TextGameState::setup_dock_canvas()
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        TEXTGAME.write().unwrap().activate_dock(ui, ctx);
    }

    fn supports_actions(&self) -> bool {
        false
    }
}
