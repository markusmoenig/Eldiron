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
            Box::new(crate::actions::editing_camera::EditingCamera::new()),
            Box::new(crate::actions::firstp_camera::FirstPCamera::new()),
            Box::new(crate::actions::iso_camera::IsoCamera::new()),
            Box::new(crate::actions::orbit_camera::OrbitCamera::new()),
            Box::new(crate::actions::apply_tile::ApplyTile::new()),
            Box::new(crate::actions::build_procedural::BuildProcedural::new()),
            Box::new(crate::actions::copy_tile_id::CopyTileID::new()),
            Box::new(crate::actions::create_center_vertex::CreateCenterVertex::new()),
            Box::new(crate::actions::create_geometry_box::CreateGeometryBox::new()),
            Box::new(crate::actions::face_cut_opening::CreateCutout::new()),
            Box::new(crate::actions::create_pattern::CreatePattern::new()),
            Box::new(crate::actions::create_surface_face::CreateSurfaceFace::new()),
            Box::new(crate::actions::create_ridge::CreateGroove::new()),
            Box::new(crate::actions::create_linedef::CreateLinedef::new()),
            Box::new(crate::actions::create_ridge::CreateRidge::new()),
            Box::new(crate::actions::create_sector::CreateSector::new()),
            Box::new(crate::actions::cut_profile::CutProfile::new()),
            Box::new(crate::actions::cut_stairs::CutStairs::new()),
            Box::new(crate::actions::surface_noise::SurfaceNoise::new()),
            Box::new(crate::actions::clear_tile::ClearTile::new()),
            Box::new(crate::actions::clear_palette::ClearPalette::new()),
            Box::new(crate::actions::duplicate::Duplicate::new()),
            Box::new(crate::actions::duplicate_surface_detail::DuplicateSurfaceDetail::new()),
            Box::new(crate::actions::toggle_surface_curve::ToggleSurfaceCurve::new()),
            Box::new(crate::actions::duplicate_tile::DuplicateTile::new()),
            Box::new(crate::actions::edit_face_texture::EditFaceTexture::new()),
            Box::new(crate::actions::edit_geometry::EditGeometry::new()),
            Box::new(crate::actions::face_cut_opening::FaceCutOpening::new()),
            Box::new(crate::actions::face_delete::FaceDelete::new()),
            Box::new(crate::actions::face_extrude::FaceExtrude::new()),
            Box::new(crate::actions::face_inset::FaceInset::new()),
            Box::new(crate::actions::face_merge::FaceMerge::new()),
            Box::new(crate::actions::face_subdivide::FaceSubdivide::new()),
            Box::new(crate::actions::edit_maximize::EditMaximize::new()),
            Box::new(crate::actions::edit_linedef::EditLinedef::new()),
            Box::new(crate::actions::edit_sector::EditSector::new()),
            Box::new(crate::actions::edit_vertex::EditVertex::new()),
            Box::new(crate::actions::editing_slice::EditingSlice::new()),
            Box::new(crate::actions::edit_tile_meta::EditTileMeta::new()),
            Box::new(crate::actions::filter_editing_geo::FilterEditingGeo::new()),
            Box::new(crate::actions::import_palette::ImportPalette::new()),
            Box::new(crate::actions::make_sector_rectangular::MakeSectorRectangular::new()),
            Box::new(crate::actions::new_tile::NewTile::new()),
            Box::new(crate::actions::minimize::Minimize::new()),
            Box::new(crate::actions::remap_tile::RemapTile::new()),
            Box::new(crate::actions::set_tile_material::SetTileMaterial::new()),
            Box::new(crate::actions::split::Split::new()),
            Box::new(crate::actions::toggle_editing_geo::ToggleEditingGeo::new()),
            Box::new(crate::actions::toggle_editor_preview_render::ToggleEditorPreviewPost::new()),
            Box::new(
                crate::actions::toggle_editor_preview_render::ToggleEditorPreviewLighting::new(),
            ),
            Box::new(crate::actions::toggle_rect_geo::ToggleRectGeo::new()),
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
