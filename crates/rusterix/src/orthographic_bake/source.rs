//! Borrowed, immutable source channels for an application-owned GPU producer.
//! These are original bake tiles, never a CPU-composed viewport.
use super::*;

#[derive(Clone, Copy)]
pub struct OrthographicBakeTileSource<'a> {
    pub color: &'a [u8],
    pub depth: &'a [f32],
    pub albedo: &'a [u8],
    pub normal: &'a [u8],
    pub material: &'a [u8],
}
impl<'a> From<&'a DecodedBakeTile> for OrthographicBakeTileSource<'a> {
    fn from(tile: &'a DecodedBakeTile) -> Self {
        Self {
            color: &tile.color,
            depth: &tile.depth,
            albedo: &tile.albedo,
            normal: &tile.normal,
            material: &tile.material,
        }
    }
}

/// A revision is scoped to this publication, including transient progressive
/// samples. It cannot alias an old source after controller clear/replacement.
pub struct OrthographicBakeSource<'a> {
    revision: Uuid,
    asset: &'a OrthographicBakeAsset,
    tiles: &'a HashMap<(i32, i32), DecodedBakeTile>,
}
impl<'a> OrthographicBakeSource<'a> {
    pub fn revision(&self) -> Uuid {
        self.revision
    }
    pub fn asset(&self) -> &'a OrthographicBakeAsset {
        self.asset
    }
    pub fn matches_camera(&self, camera: &Camera3D) -> bool {
        asset_matches_camera(self.asset, camera)
    }
    pub fn tile(&self, x: i32, y: i32) -> Option<OrthographicBakeTileSource<'a>> {
        self.tiles.get(&(x, y)).map(Into::into)
    }
    pub fn tiles(&self) -> impl Iterator<Item = ((i32, i32), OrthographicBakeTileSource<'a>)> + '_ {
        self.tiles
            .iter()
            .map(|(position, tile)| (*position, tile.into()))
    }
}
impl OrthographicBakeController {
    /// The caller borrows only source assets. No viewport allocation, CPU
    /// composition, image capture or device creation occurs at this boundary.
    pub fn presentation_source(
        &self,
        region_id: Uuid,
        camera: &Camera3D,
    ) -> Option<OrthographicBakeSource<'_>> {
        if !self.can_compose(region_id, camera) {
            return None;
        }
        let (asset, tiles) = if self.is_rendering() {
            (self.staging_asset.as_ref()?, &self.staging_tiles)
        } else {
            (self.committed.as_ref()?, &self.committed_tiles)
        };
        Some(OrthographicBakeSource {
            revision: self.source_revision,
            asset,
            tiles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(camera: &Camera3D) -> OrthographicBakeAsset {
        OrthographicBakeAsset {
            version: BAKE_VERSION,
            tile_size: 1,
            pixels_per_world_unit: 1.,
            projected_bounds: [-0.5, 0.5, -0.5, 0.5],
            camera_forward: camera.forward.into_array(),
            camera_right: camera.right.into_array(),
            camera_up: camera.up.into_array(),
            camera_forward_origin: camera.pos.dot(camera.forward.normalized()),
            samples: 1,
            tiles: vec![OrthographicBakeTile {
                x: 0,
                y: 0,
                color_png_base64: BASE64.encode(encode_png(1, 1, &[80, 120, 160, 255]).unwrap()),
                depth_base64: Some(BASE64.encode(encode_depth(&[2.]))),
                albedo_png_base64: None,
                normal_png_base64: None,
                material_png_base64: None,
                color_png_path: None,
                depth_path: None,
                albedo_png_path: None,
                normal_png_path: None,
                material_png_path: None,
            }],
        }
    }

    #[test]
    fn equal_length_replacements_and_metadata_publish_fresh_source_channels() {
        let region = Uuid::new_v4();
        let mut camera = Camera3D::iso();
        camera.ortho_half_h = 0.5;
        let mut asset = fixture(&camera);
        let mut owner = OrthographicBakeController::default();
        owner.sync_persisted_asset(region, Some(&asset));
        let first = owner.presentation_source(region, &camera).unwrap().revision;
        owner.sync_persisted_asset(region, Some(&asset));
        assert_eq!(
            owner.presentation_source(region, &camera).unwrap().revision,
            first
        );

        // Same version, count and encoded length: the old signature missed this.
        let previous_len = asset.tiles[0].depth_base64.as_ref().unwrap().len();
        asset.tiles[0].depth_base64 = Some(BASE64.encode(encode_depth(&[7.])));
        assert_eq!(
            asset.tiles[0].depth_base64.as_ref().unwrap().len(),
            previous_len
        );
        owner.sync_persisted_asset(region, Some(&asset));
        let source = owner.presentation_source(region, &camera).unwrap();
        assert_ne!(source.revision, first);
        assert_eq!(source.tile(0, 0).unwrap().depth, &[7.]);
        assert_eq!(owner.compose_depth(region, 1, 1, &camera).unwrap(), [7.]);

        let second = source.revision;
        asset.camera_forward_origin += 3.;
        owner.sync_persisted_asset(region, Some(&asset));
        assert_ne!(
            owner.presentation_source(region, &camera).unwrap().revision,
            second
        );
        assert_eq!(owner.compose_depth(region, 1, 1, &camera).unwrap(), [10.]);

        asset.tiles[0].x = 5;
        owner.sync_persisted_asset(region, Some(&asset));
        let source = owner.presentation_source(region, &camera).unwrap();
        assert!(source.tile(0, 0).is_none());
        assert_eq!(source.tile(5, 0).unwrap().color, &[80, 120, 160, 255]);
        assert!(owner.presentation_source(Uuid::new_v4(), &camera).is_none());

        let last = source.revision;
        owner.clear();
        assert!(owner.presentation_source(region, &camera).is_none());
        owner.sync_persisted_asset(region, Some(&asset));
        assert_ne!(
            owner.presentation_source(region, &camera).unwrap().revision,
            last
        );
    }

    #[test]
    fn progressive_sources_are_transient_and_do_not_reuse_committed_publications() {
        let region = Uuid::new_v4();
        let mut camera = Camera3D::iso();
        camera.ortho_half_h = 0.5;
        let asset = fixture(&camera);
        let mut owner = OrthographicBakeController::default();
        owner.sync_persisted_asset(region, Some(&asset));
        let original = owner.presentation_source(region, &camera).unwrap().revision;
        owner.request_render(region, 1);
        let work = owner
            .prepare_work(region, 1, 1, camera, Some((-0.5, 0.5, -0.5, 0.5)))
            .unwrap()
            .unwrap();
        assert!(owner.presentation_source(region, &camera).is_none());
        let pixels = (work.tile_size * work.tile_size) as usize;
        let rgba = [20, 40, 60, 255].repeat(pixels);
        owner.preview_current_tile(&rgba).unwrap();
        let source = owner.presentation_source(region, &camera).unwrap();
        let preview = source.revision;
        assert_ne!(preview, original);
        assert_eq!(source.tiles().count(), 1);
        assert_eq!(source.tile(0, 0).unwrap().color, rgba);
        assert!(source.tile(0, 0).unwrap().depth.is_empty());
        assert!(owner.take_persisted_update().is_none());
        assert!(owner.finish_sample());
        assert!(
            owner
                .commit_current_tile(
                    rgba.clone(),
                    vec![2.; pixels],
                    vec![64; pixels * 4],
                    vec![127; pixels * 4],
                    vec![32; pixels * 4]
                )
                .unwrap()
        );
        let committed = owner.presentation_source(region, &camera).unwrap().revision;
        assert_ne!(committed, preview);
        assert!(owner.take_persisted_update().unwrap().is_some());
        assert_eq!(owner.toggle_visibility(), Some(false));
        assert!(owner.presentation_source(region, &camera).is_none());
        assert_eq!(owner.toggle_visibility(), Some(true));
        assert_ne!(
            owner.presentation_source(region, &camera).unwrap().revision,
            committed
        );
        owner.request_render(region, 1);
        owner.fail("controlled failure");
        assert_eq!(
            owner
                .presentation_source(region, &camera)
                .unwrap()
                .tile(0, 0)
                .unwrap()
                .color,
            rgba
        );
    }
}
