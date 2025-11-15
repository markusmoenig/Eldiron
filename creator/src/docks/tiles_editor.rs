use crate::prelude::*;

pub struct TilesEditorDock {
    pub tile_ids: FxHashMap<(i32, i32), Uuid>,

    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,

    pub curr_tile: Option<Uuid>,
}

impl Dock for TilesEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            tile_ids: FxHashMap::default(),
            filter: "".to_string(),
            filter_role: 0,
            zoom: 1.5,
            curr_tile: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let canvas = TheCanvas::new();

        canvas
    }

    fn activate(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
    }

    fn handle_event(
        &mut self,
        _event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        redraw
    }
}

impl TilesEditorDock {}
