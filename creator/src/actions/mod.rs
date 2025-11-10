pub use crate::prelude::*;

pub mod add_arch;
// pub mod add_shader_library;
// pub mod apply_shader;
pub mod apply_tile;
// pub mod clear_shader;
pub mod clear_tile;
pub mod create_center_vertex;
pub mod create_linedef;
pub mod create_sector;
pub mod edit_linedef;
pub mod edit_sector;
pub mod edit_vertex;
pub mod extrude_linedef;
pub mod extrude_sector;
// pub mod gen_stone_trim;
pub mod copy_tile_id;
pub mod edit_tile_meta;
pub mod firstp_camera;
pub mod iso_camera;
// pub mod load_shader;
// pub mod new_shader;
pub mod orbit_camera;
pub mod recess;
pub mod relief;
pub mod set_editing_surface;
pub mod split;
pub mod toggle_rect_geo;

#[derive(PartialEq)]
pub enum ActionRole {
    Camera,
    Editor,
    Dock,
}

impl ActionRole {
    pub fn to_color(&self) -> [u8; 4] {
        match self {
            ActionRole::Camera => [160, 175, 190, 255],
            ActionRole::Editor => [195, 170, 150, 255],
            ActionRole::Dock => [200, 195, 150, 255],
            // ActionRole::Profile => [160, 185, 160, 255],
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
    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {}

    fn apply(
        &self,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        None
    }

    fn apply_project(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
    }

    fn params(&self) -> TheNodeUI;
    fn handle_event(&mut self, event: &TheEvent) -> bool;
}
