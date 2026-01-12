use crate::prelude::*;

pub struct ActionList {
    pub actions: Vec<Box<dyn Action>>,
}

impl Default for ActionList {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionList {
    pub fn new() -> Self {
        let actions: Vec<Box<dyn Action>> = vec![
            Box::new(crate::tools::editing_camera::EditingCamera::new()),
            Box::new(crate::tools::firstp_camera::FirstPCamera::new()),
            Box::new(crate::tools::iso_camera::IsoCamera::new()),
            Box::new(crate::tools::orbit_camera::OrbitCamera::new()),
            Box::new(crate::tools::add_arch::AddArch::new()),
            Box::new(crate::tools::apply_tile::ApplyTile::new()),
            Box::new(crate::tools::clear_profile::ClearProfile::new()),
            Box::new(crate::tools::clear_tile::ClearTile::new()),
            Box::new(crate::tools::copy_tile_id::CopyTileID::new()),
            Box::new(crate::tools::create_center_vertex::CreateCenterVertex::new()),
            Box::new(crate::tools::create_linedef::CreateLinedef::new()),
            Box::new(crate::tools::create_sector::CreateSector::new()),
            Box::new(crate::tools::clear_palette::ClearPalette::new()),
            Box::new(crate::tools::duplicate_tile::DuplicateTile::new()),
            Box::new(crate::tools::edit_maximize::EditMaximize::new()),
            Box::new(crate::tools::edit_linedef::EditLinedef::new()),
            Box::new(crate::tools::edit_sector::EditSector::new()),
            Box::new(crate::tools::edit_vertex::EditVertex::new()),
            Box::new(crate::tools::editing_slice::EditingSlice::new()),
            Box::new(crate::tools::edit_tile_meta::EditTileMeta::new()),
            Box::new(crate::tools::export_vcode::ExportVCode::new()),
            Box::new(crate::tools::extrude_linedef::ExtrudeLinedef::new()),
            Box::new(crate::tools::extrude_sector::ExtrudeSector::new()),
            // Box::new(crate::tools::gen_stone_trim::GenerateStoneTrim::new()),
            Box::new(crate::tools::gate_door::GateDoor::new()),
            Box::new(crate::tools::import_vcode::ImportVCode::new()),
            Box::new(crate::tools::import_palette::ImportPalette::new()),
            Box::new(crate::tools::new_tile::NewTile::new()),
            Box::new(crate::tools::minimize::Minimize::new()),
            Box::new(crate::tools::recess::Recess::new()),
            Box::new(crate::tools::relief::Relief::new()),
            Box::new(crate::tools::remap_tile::RemapTile::new()),
            Box::new(crate::tools::set_editing_surface::SetEditingSurface::new()),
            Box::new(crate::tools::set_tile_material::SetTileMaterial::new()),
            Box::new(crate::tools::split::Split::new()),
            Box::new(crate::tools::toggle_editing_geo::ToggleEditingGeo::new()),
            Box::new(crate::tools::toggle_rect_geo::ToggleRectGeo::new()),
        ];
        Self { actions }
    }

    /// Returns an action by the given id.
    pub fn get_action_by_id(&self, id: Uuid) -> Option<&Box<dyn Action>> {
        for action in &self.actions {
            if action.id().uuid == id {
                return Some(action);
            }
        }
        None
    }

    /// Returns an mutable action by the given id.
    pub fn get_action_by_id_mut(&mut self, id: Uuid) -> Option<&mut Box<dyn Action>> {
        for action in &mut self.actions {
            if action.id().uuid == id {
                return Some(action);
            }
        }
        None
    }
}
