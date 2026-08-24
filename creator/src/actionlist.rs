use crate::prelude::*;
use std::collections::HashMap;

pub fn validate_command_id(command_id: &str) -> Result<(), String> {
    let valid = !command_id.is_empty()
        && command_id.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_lowercase() || ch.is_ascii_digit())
        });
    if valid {
        Ok(())
    } else {
        Err(format!(
            "Command id '{command_id}' must contain lowercase dotted identifiers."
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionDescriptor {
    /// Stable, non-localized identifier used by scripts, plugins, and automation.
    pub command_id: String,
    pub group: ActionGroup,
}

pub struct ActionList {
    pub actions: Vec<Box<dyn Action>>,
    descriptors: HashMap<Uuid, ActionDescriptor>,
    command_ids: HashMap<String, Uuid>,
}

impl Default for ActionList {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionList {
    pub fn new() -> Self {
        use ActionGroup::*;

        let mut list = Self {
            actions: Vec::new(),
            descriptors: HashMap::new(),
            command_ids: HashMap::new(),
        };

        list.register(
            "bake.render",
            Bake,
            crate::actions::orthographic_bake::RenderOrthographicBake::new(),
        );
        list.register(
            "bake.toggle_visibility",
            Bake,
            crate::actions::orthographic_bake::ToggleOrthographicBakeVisibility::new(),
        );
        list.register(
            "bake.clear",
            Bake,
            crate::actions::orthographic_bake::ClearOrthographicBake::new(),
        );
        list.register(
            "camera.editing",
            Camera,
            crate::actions::editing_camera::EditingCamera::new(),
        );
        list.register(
            "camera.first_person",
            Camera,
            crate::actions::firstp_camera::FirstPCamera::new(),
        );
        list.register(
            "camera.isometric",
            Camera,
            crate::actions::iso_camera::IsoCamera::new(),
        );
        list.register(
            "camera.orbit",
            Camera,
            crate::actions::orbit_camera::OrbitCamera::new(),
        );
        list.register(
            "tile.apply",
            Tile,
            crate::actions::apply_tile::ApplyTile::new(),
        );
        list.register(
            "procedural.build",
            Procedural,
            crate::actions::build_procedural::BuildProcedural::new(),
        );
        list.register(
            "tile.copy_id",
            Tile,
            crate::actions::copy_tile_id::CopyTileID::new(),
        );
        list.register(
            "map.create_center_vertex",
            Map,
            crate::actions::create_center_vertex::CreateCenterVertex::new(),
        );
        list.register(
            "geometry.create_fitted",
            Geometry,
            crate::actions::create_fitted_geometry::CreateFittedGeometry::new(),
        );
        list.register(
            "geometry.create_box",
            Geometry,
            crate::actions::create_geometry_box::CreateGeometryBox::new(),
        );
        list.register(
            "geometry.create_unit_box",
            Geometry,
            crate::actions::create_geometry_box::CreateGeometryUnitBox::new(),
        );
        list.register(
            "prefab.create_linked",
            Prefab,
            crate::actions::prefabs::CreateLinkedPrefab::new(),
        );
        list.register(
            "prefab.create_copy",
            Prefab,
            crate::actions::prefabs::CreatePrefabCopy::new(),
        );
        list.register(
            "prefab.update_source",
            Prefab,
            crate::actions::prefabs::UpdatePrefabSource::new(),
        );
        list.register(
            "prefab.make_unique",
            Prefab,
            crate::actions::prefabs::MakePrefabUnique::new(),
        );
        list.register(
            "prefab.unpack",
            Prefab,
            crate::actions::prefabs::UnpackPrefab::new(),
        );
        list.register(
            "face.create_cutout",
            Face,
            crate::actions::face_cut_opening::CreateCutout::new(),
        );
        list.register(
            "face.create_surface",
            Face,
            crate::actions::create_surface_face::CreateSurfaceFace::new(),
        );
        list.register(
            "surface.create_groove",
            Surface,
            crate::actions::create_ridge::CreateGroove::new(),
        );
        list.register(
            "map.create_linedef",
            Map,
            crate::actions::create_linedef::CreateLinedef::new(),
        );
        list.register(
            "surface.create_ridge",
            Surface,
            crate::actions::create_ridge::CreateRidge::new(),
        );
        list.register(
            "map.create_sector",
            Map,
            crate::actions::create_sector::CreateSector::new(),
        );
        list.register(
            "surface.cut_stairs",
            Surface,
            crate::actions::cut_stairs::CutStairs::new(),
        );
        list.register(
            "tile.clear",
            Tile,
            crate::actions::clear_tile::ClearTile::new(),
        );
        list.register(
            "palette.clear",
            Palette,
            crate::actions::clear_palette::ClearPalette::new(),
        );
        list.register(
            "surface.clear_detail",
            Surface,
            crate::actions::clear_surface_detail::ClearSurfaceDetail::new(),
        );
        list.register(
            // Keep the established command id for scripts and shortcuts; the
            // action is presented with the other geometry operations.
            "general.duplicate",
            Geometry,
            crate::actions::duplicate::Duplicate::new(),
        );
        list.register(
            "surface.duplicate_detail",
            Surface,
            crate::actions::duplicate_surface_detail::DuplicateSurfaceDetail::new(),
        );
        list.register(
            "surface.toggle_curve",
            Surface,
            crate::actions::toggle_surface_curve::ToggleSurfaceCurve::new(),
        );
        list.register(
            "tile.duplicate",
            Tile,
            crate::actions::duplicate_tile::DuplicateTile::new(),
        );
        list.register(
            "face.edit_texture",
            Face,
            crate::actions::edit_face_texture::EditFaceTexture::new(),
        );
        list.register(
            "geometry.edit",
            Geometry,
            crate::actions::edit_geometry::EditGeometry::new(),
        );
        list.register(
            "face.cut_opening",
            Face,
            crate::actions::face_cut_opening::FaceCutOpening::new(),
        );
        list.register(
            "face.delete",
            Face,
            crate::actions::face_delete::FaceDelete::new(),
        );
        list.register(
            "face.extrude",
            Face,
            crate::actions::face_extrude::FaceExtrude::new(),
        );
        list.register(
            "face.inset",
            Face,
            crate::actions::face_inset::FaceInset::new(),
        );
        list.register(
            "face.merge",
            Face,
            crate::actions::face_merge::FaceMerge::new(),
        );
        list.register(
            "face.subdivide",
            Face,
            crate::actions::face_subdivide::FaceSubdivide::new(),
        );
        list.register(
            "view.maximize_editor",
            View,
            crate::actions::edit_maximize::EditMaximize::new(),
        );
        list.register(
            "map.edit_linedef",
            Map,
            crate::actions::edit_linedef::EditLinedef::new(),
        );
        list.register(
            "map.edit_sector",
            Map,
            crate::actions::edit_sector::EditSector::new(),
        );
        list.register(
            "map.edit_vertex",
            Map,
            crate::actions::edit_vertex::EditVertex::new(),
        );
        list.register(
            "view.editing_slice",
            View,
            crate::actions::editing_slice::EditingSlice::new(),
        );
        list.register(
            "tile.edit_metadata",
            Tile,
            crate::actions::edit_tile_meta::EditTileMeta::new(),
        );
        list.register(
            "view.filter_editing_geometry",
            View,
            crate::actions::filter_editing_geo::FilterEditingGeo::new(),
        );
        list.register(
            "palette.import",
            Palette,
            crate::actions::import_palette::ImportPalette::new(),
        );
        list.register(
            "map.make_sector_rectangular",
            Map,
            crate::actions::make_sector_rectangular::MakeSectorRectangular::new(),
        );
        list.register("tile.new", Tile, crate::actions::new_tile::NewTile::new());
        list.register(
            "general.minimize",
            General,
            crate::actions::minimize::Minimize::new(),
        );
        list.register(
            "tile.remap",
            Tile,
            crate::actions::remap_tile::RemapTile::new(),
        );
        list.register("map.split", Map, crate::actions::split::Split::new());
        list.register(
            "view.toggle_editing_geometry",
            View,
            crate::actions::toggle_editing_geo::ToggleEditingGeo::new(),
        );
        list.register(
            "view.toggle_preview_post",
            View,
            crate::actions::toggle_editor_preview_render::ToggleEditorPreviewPost::new(),
        );
        list.register(
            "view.toggle_preview_lighting",
            View,
            crate::actions::toggle_editor_preview_render::ToggleEditorPreviewLighting::new(),
        );
        list.register(
            "view.toggle_rect_geometry",
            View,
            crate::actions::toggle_rect_geo::ToggleRectGeo::new(),
        );

        list
    }

    fn register<A: Action + 'static>(
        &mut self,
        command_id: &'static str,
        group: ActionGroup,
        action: A,
    ) {
        self.register_action(command_id, group, action)
            .unwrap_or_else(|error| panic!("invalid built-in action registration: {error}"));
    }

    /// Register an action supplied by Creator or a plugin.
    ///
    /// Command ids are owned and validated because plugin manifests are loaded at runtime. A
    /// dotted lowercase namespace such as `vendor.plugin.action` is recommended for plugins.
    pub fn register_action<A: Action + 'static>(
        &mut self,
        command_id: impl Into<String>,
        group: ActionGroup,
        action: A,
    ) -> Result<(), String> {
        self.register_boxed_action(command_id, group, Box::new(action))
    }

    /// Boxed counterpart used by runtime plugin loaders.
    pub fn register_boxed_action(
        &mut self,
        command_id: impl Into<String>,
        group: ActionGroup,
        action: Box<dyn Action>,
    ) -> Result<(), String> {
        let command_id = command_id.into();
        validate_command_id(&command_id)?;
        if self.command_ids.contains_key(&command_id) {
            return Err(format!("Duplicate action command id '{command_id}'."));
        }
        let id = action.id().uuid;
        if self.descriptors.contains_key(&id) {
            return Err(format!("Duplicate action UUID '{id}'."));
        }
        self.command_ids.insert(command_id.clone(), id);
        self.descriptors
            .insert(id, ActionDescriptor { command_id, group });
        self.actions.push(action);
        Ok(())
    }

    pub fn descriptor_by_id(&self, id: Uuid) -> Option<&ActionDescriptor> {
        self.descriptors.get(&id)
    }

    pub fn get_action_by_command_id(&self, command_id: &str) -> Option<&Box<dyn Action>> {
        self.command_ids
            .get(command_id)
            .and_then(|id| self.get_action_by_id(*id))
    }

    pub fn get_action_by_command_id_mut(
        &mut self,
        command_id: &str,
    ) -> Option<&mut Box<dyn Action>> {
        let id = *self.command_ids.get(command_id)?;
        self.get_action_by_id_mut(id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_action_has_unique_stable_command_metadata() {
        let actions = ActionList::new();
        assert_eq!(actions.actions.len(), actions.descriptors.len());
        assert_eq!(actions.actions.len(), actions.command_ids.len());

        for action in &actions.actions {
            let descriptor = actions.descriptor_by_id(action.id().uuid).unwrap();
            assert!(descriptor.command_id.contains('.'));
            assert!(
                actions
                    .get_action_by_command_id(&descriptor.command_id)
                    .is_some()
            );
        }
    }

    #[test]
    fn action_groups_map_one_to_one_to_theme_slots() {
        let slots = ActionGroup::ALL.map(ActionGroup::palette_slot);
        assert_eq!(slots, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn duplicate_is_presented_as_geometry_without_breaking_its_command_id() {
        let actions = ActionList::new();
        let action = actions
            .get_action_by_command_id("general.duplicate")
            .unwrap();
        let descriptor = actions.descriptor_by_id(action.id().uuid).unwrap();
        assert_eq!(descriptor.group, ActionGroup::Geometry);
    }
}
