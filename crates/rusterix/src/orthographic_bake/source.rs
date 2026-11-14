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
    request: Option<(Uuid, Uuid)>,
    asset: &'a OrthographicBakeAsset,
    tiles: &'a TileCache,
}
impl<'a> OrthographicBakeSource<'a> {
    pub fn revision(&self) -> Uuid {
        self.revision
    }
    /// Progressive samples share one immutable authoring/request scope while
    /// original channel publications continue to advance within that request.
    pub fn request(&self) -> Option<(Uuid, Uuid)> {
        self.request
    }
    pub fn asset(&self) -> &'a OrthographicBakeAsset {
        self.asset
    }
    pub fn matches_camera(&self, camera: &Camera3D) -> bool {
        asset_matches_camera(self.asset, camera)
    }
    pub fn tile(&self, x: i32, y: i32) -> Option<OrthographicBakeTileLease> {
        self.try_tile(x, y).ok().flatten()
    }
    /// Resolve one original tile. Budget pressure is an explicit producer error,
    /// never an omitted visible tile in the GPU path.
    pub fn try_tile(&self, x: i32, y: i32) -> Result<Option<OrthographicBakeTileLease>, String> {
        self.tiles.get(self.asset, (x, y))
    }
    pub fn positions(&self) -> impl Iterator<Item = (i32, i32)> {
        self.tiles.all_positions().into_iter()
    }
    pub fn decoded_resident_bytes(&self) -> u64 {
        self.tiles.usage()
    }
    pub fn tiles(&self) -> impl Iterator<Item = ((i32, i32), OrthographicBakeTileLease)> + '_ {
        self.positions()
            .filter_map(|at| self.tile(at.0, at.1).map(|tile| (at, tile)))
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
            request: self.active_request(),
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
    fn encoded_sources_decode_lazily_and_page_without_mutating_leased_channels() {
        let region = Uuid::new_v4();
        let camera = Camera3D::iso();
        let mut asset = fixture(&camera);
        asset.tile_size = 256;
        let pixels = 256 * 256;
        let rgba =
            BASE64.encode(encode_png(256, 256, &[80, 120, 160, 255].repeat(pixels)).unwrap());
        let mut tile = asset.tiles[0].clone();
        tile.color_png_base64 = rgba.clone();
        tile.albedo_png_base64 = Some(rgba.clone());
        tile.normal_png_base64 = Some(rgba.clone());
        tile.material_png_base64 = Some(rgba);
        tile.depth_base64 = Some(BASE64.encode(encode_depth(&vec![2.; pixels])));
        asset.tiles = (0..32)
            .map(|x| {
                let mut tile = tile.clone();
                tile.x = x;
                tile
            })
            .collect();
        let mut owner = OrthographicBakeController::default();
        owner.sync_persisted_asset(region, Some(&asset));
        assert_eq!(
            owner.decoded_resident_bytes(),
            0,
            "install only encoded originals/metadata"
        );
        let source = owner.presentation_source(region, &camera).unwrap();
        assert_eq!(source.positions().count(), 32);
        assert_eq!(
            source.decoded_resident_bytes(),
            pixels as u64 * 20,
            "first readiness probes one usable tile; metadata enumeration adds no decoded pixels"
        );
        let first = source.try_tile(0, 0).unwrap().unwrap();
        for x in 1..32 {
            let lease = source.try_tile(x, 0).unwrap().unwrap();
            assert_eq!(lease.depth[0], 2.);
            assert!(source.decoded_resident_bytes() <= cache::DECODE_BUDGET);
        }
        assert_eq!(&first.color[..4], &[80, 120, 160, 255]);
        asset.samples += 1;
        owner.sync_persisted_asset(region, Some(&asset));
        assert_eq!(
            owner.decoded_resident_bytes(),
            pixels as u64 * 20,
            "reset releases cached pixels but preserves and charges the external lease"
        );
        drop(first);
        assert_eq!(owner.decoded_resident_bytes(), 0);
        let source = owner.presentation_source(region, &camera).unwrap();
        assert_eq!(
            source.try_tile(31, 0).unwrap().unwrap().albedo.len(),
            pixels * 4
        );
    }

    #[test]
    fn missing_and_corrupt_originals_keep_geometry_fallback_and_do_not_poison_replacements() {
        let map = Uuid::new_v4();
        let camera = Camera3D::iso();
        let mut asset = fixture(&camera);
        let valid = asset.tiles[0].color_png_base64.clone();
        asset.tiles[0].color_png_base64 = "invalid".into();
        let mut owner = OrthographicBakeController::default();
        owner.sync_persisted_asset(map, Some(&asset));
        assert!(!owner.can_compose(map, &camera));
        assert!(owner.presentation_source(map, &camera).is_none());
        assert!(owner.compose_rgba(map, 1, 1, &camera).is_none());
        assert_eq!(owner.decoded_resident_bytes(), 0);
        asset.tiles[0].color_png_base64 = valid;
        owner.sync_persisted_asset(map, Some(&asset));
        assert!(owner.can_compose(map, &camera));
        assert_eq!(
            owner
                .presentation_source(map, &camera)
                .unwrap()
                .try_tile(0, 0)
                .unwrap()
                .unwrap()
                .depth,
            [2.]
        );
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
        // Complete the explicit clear publication before restoring authored data.
        assert_eq!(owner.take_persisted_update_for(region), Some(None));
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
