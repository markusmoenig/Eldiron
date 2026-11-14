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
    /// Advance at most one cooperative channel, then poll lookups freely.
    /// Call exactly once per preparation step, before resolving visible tiles.
    pub fn advance_decode(&self) -> Result<(), String> {
        self.tiles.advance()
    }
    pub fn may_have_tiles(&self) -> bool {
        self.tiles.may_have_tiles()
    }
    /// Schedule/poll bounded original-channel work without blocking for a CPU
    /// decode. Native uses bounded workers; browsers advance one channel/step.
    pub fn poll_tile(&self, x: i32, y: i32) -> Result<OrthographicBakeTilePoll, String> {
        self.tiles.poll(self.asset, (x, y))
    }
    /// Identity of this original tile publication, independent of CPU decode
    /// residency and updates to other tiles. Never serialized into the asset.
    pub fn tile_revision(&self, x: i32, y: i32) -> Option<Uuid> {
        self.tiles.revision((x, y))
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
        let active = if self.is_rendering() {
            self.requested_region
        } else {
            self.committed_region
        };
        if !self.visible || active != Some(region_id) {
            return None;
        }
        let (asset, tiles) = if self.is_rendering() {
            (self.staging_asset.as_ref()?, &self.staging_tiles)
        } else {
            (self.committed.as_ref()?, &self.committed_tiles)
        };
        if !asset_matches_camera(asset, camera) || !tiles.may_have_tiles() {
            return None;
        }
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
                color_png_base64: BASE64.encode(encode_png(1, 1, &[80, 120, 160, 255]).unwrap()).into(),
                depth_base64: Some(BASE64.encode(encode_depth(&[2.])).into()),
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

    fn wait_deferred(
        source: &OrthographicBakeSource<'_>,
        at: (i32, i32),
    ) -> OrthographicBakeTileLease {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            source.advance_decode().unwrap();
            match source.poll_tile(at.0, at.1).unwrap() {
                OrthographicBakeTilePoll::Ready(lease) => return lease,
                OrthographicBakeTilePoll::Missing => panic!("original tile unexpectedly missing"),
                OrthographicBakeTilePoll::Pending => (),
            }
            assert!(
                std::time::Instant::now() < deadline,
                "deferred tile stalled"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn deferred_decode_publishes_only_when_advanced_and_revokes_replaced_work() {
        let map = Uuid::new_v4();
        let camera = Camera3D::iso();
        let mut asset = fixture(&camera);
        let mut owner = OrthographicBakeController::default();
        owner.sync_persisted_asset(map, Some(&asset));
        {
            let source = owner.presentation_source(map, &camera).unwrap();
            assert_eq!(source.decoded_resident_bytes(), 0);
            assert!(matches!(
                source.poll_tile(0, 0).unwrap(),
                OrthographicBakeTilePoll::Pending
            ));
            for _ in 0..8 {
                assert!(
                    matches!(
                        source.poll_tile(0, 0).unwrap(),
                        OrthographicBakeTilePoll::Pending
                    ),
                    "lookup alone cannot advance or publish work"
                );
            }
        }
        // Same coordinate and encoded length, new installation. An old queued
        // result has no receiver/publication path into the replacement cache.
        asset.tiles[0].depth_base64 = Some(BASE64.encode(encode_depth(&[7.])).into());
        owner.sync_persisted_asset(map, Some(&asset));
        let green = wait_deferred(&owner.presentation_source(map, &camera).unwrap(), (0, 0));
        assert_eq!(green.depth, [7.]);
        assert_eq!(green.color, [80, 120, 160, 255]);
        // A progressive producer may supersede an encoded decode directly.
        owner.committed_tiles.reset(Some(&asset));
        let source = owner.presentation_source(map, &camera).unwrap();
        assert!(matches!(
            source.poll_tile(0, 0).unwrap(),
            OrthographicBakeTilePoll::Pending
        ));
        owner
            .committed_tiles
            .insert(
                (0, 0),
                Some(0),
                DecodedBakeTile {
                    color: vec![9, 8, 7, 255],
                    depth: vec![11.],
                    ..Default::default()
                },
            )
            .unwrap();
        let source = owner.presentation_source(map, &camera).unwrap();
        for _ in 0..8 {
            source.advance_decode().unwrap();
            let OrthographicBakeTilePoll::Ready(tile) = source.poll_tile(0, 0).unwrap() else {
                panic!("fresh producer source lost");
            };
            assert_eq!(tile.depth, [11.]);
            assert_eq!(tile.color, [9, 8, 7, 255]);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(green.depth, [7.], "external originals remain immutable");
        owner.clear();
        assert!(
            owner.decoded_resident_bytes() >= 8,
            "controller reset must retain the external lease allocation domain"
        );
        drop(green);
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while owner.decoded_resident_bytes() != 0 {
            assert!(
                std::time::Instant::now() < end,
                "cancelled decode charge leaked"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn deferred_budget_pressure_retries_while_an_external_original_is_pinned() {
        let map = Uuid::new_v4();
        let camera = Camera3D::iso();
        let asset = fixture(&camera);
        let mut owner = OrthographicBakeController::default();
        owner.sync_persisted_asset(map, Some(&asset));
        owner
            .committed_tiles
            .insert(
                (1, 0),
                Some(0), // Encoded backing permits eviction after the lease retires.
                DecodedBakeTile {
                    color: vec![1; cache::DECODE_BUDGET as usize],
                    ..Default::default()
                },
            )
            .unwrap();
        let source = owner.presentation_source(map, &camera).unwrap();
        let pinned = source.try_tile(1, 0).unwrap().unwrap();
        assert!(matches!(
            source.poll_tile(0, 0).unwrap(),
            OrthographicBakeTilePoll::Pending
        ));
        assert_eq!(source.decoded_resident_bytes(), cache::DECODE_BUDGET);
        drop(pinned);
        assert_eq!(wait_deferred(&source, (0, 0)).depth, [2.]);
        assert!(source.decoded_resident_bytes() <= cache::DECODE_BUDGET);
    }

    #[test]
    fn corrupt_deferred_originals_revoke_presentation_without_poisoning_replacement() {
        let map = Uuid::new_v4();
        let camera = Camera3D::iso();
        let mut asset = fixture(&camera);
        let valid = asset.tiles[0].color_png_base64.clone();
        asset.tiles[0].color_png_base64 = "invalid".into();
        let mut owner = OrthographicBakeController::default();
        owner.sync_persisted_asset(map, Some(&asset));
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        {
            let source = owner.presentation_source(map, &camera).unwrap();
            loop {
                source.advance_decode().unwrap();
                match source.poll_tile(0, 0).unwrap() {
                    OrthographicBakeTilePoll::Missing => break,
                    OrthographicBakeTilePoll::Ready(_) => panic!("corrupt source published"),
                    OrthographicBakeTilePoll::Pending => (),
                }
                assert!(std::time::Instant::now() < end);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        assert!(owner.presentation_source(map, &camera).is_none());
        assert_eq!(owner.decoded_resident_bytes(), 0);
        asset.tiles[0].color_png_base64 = valid.into();
        owner.sync_persisted_asset(map, Some(&asset));
        assert_eq!(
            wait_deferred(&owner.presentation_source(map, &camera).unwrap(), (0, 0)).depth,
            [2.]
        );
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
        tile.color_png_base64 = rgba.clone().into();
        tile.albedo_png_base64 = Some(rgba.clone().into());
        tile.normal_png_base64 = Some(rgba.clone().into());
        tile.material_png_base64 = Some(rgba.into());
        tile.depth_base64 = Some(BASE64.encode(encode_depth(&vec![2.; pixels])).into());
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
            0,
            "presentation readiness and metadata enumeration do not decode originals"
        );
        let first_revision = source.tile_revision(0, 0).unwrap();
        let evicted_revision = source.tile_revision(1, 0).unwrap();
        let first = source.try_tile(0, 0).unwrap().unwrap();
        for x in 1..32 {
            let lease = source.try_tile(x, 0).unwrap().unwrap();
            assert_eq!(lease.depth[0], 2.);
            assert!(source.decoded_resident_bytes() <= cache::DECODE_BUDGET);
        }
        assert_eq!(
            &source.try_tile(1, 0).unwrap().unwrap().color[..4],
            &[80, 120, 160, 255]
        );
        assert_eq!(
            source.tile_revision(1, 0),
            Some(evicted_revision),
            "re-decoding an evicted original preserves its publication identity"
        );
        assert_eq!(&first.color[..4], &[80, 120, 160, 255]);
        assert_eq!(
            source.tile_revision(0, 0),
            Some(first_revision),
            "cache eviction cannot change original identity"
        );
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
        assert_ne!(
            source.tile_revision(0, 0),
            Some(first_revision),
            "new installation revokes all prior tile identities"
        );
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
        asset.tiles[0].color_png_base64 = valid.into();
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
        asset.tiles[0].depth_base64 = Some(BASE64.encode(encode_depth(&[7.])).into());
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
