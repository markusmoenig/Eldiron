//! Request/sample identity for deferred asset capture completion.
use super::*;

impl OrthographicBakeController {
    /// Read-only active destination/request identity for application-owned jobs.
    pub fn active_request(&self) -> Option<(Uuid, Uuid)> {
        self.is_rendering()
            .then(|| {
                self.requested_region
                    .map(|map| (map, self.request_revision))
            })
            .flatten()
    }

    /// Retire transient work while preserving the last committed source.
    pub fn cancel_work(&mut self) -> bool {
        if !self.is_rendering() {
            return false;
        }
        self.request_revision = Uuid::new_v4();
        self.source_revision = Uuid::new_v4();
        self.requested_region = None;
        self.staging_camera = None;
        self.staging_asset = None;
        self.staging_tiles.clear();
        self.tile_plan.clear();
        self.pending_persisted_update = None;
        self.current_sample = 0;
        self.current_tile = 0;
        self.status = if self.committed.is_some() {
            OrthographicBakeStatus::Ready
        } else {
            OrthographicBakeStatus::Missing
        };
        self.visible = self.committed.is_some();
        true
    }
    pub fn work_is_current(&self, work: &OrthographicBakeWork) -> bool {
        self.is_rendering()
            && self.request_revision == work.request
            && self.requested_region == Some(work.map)
            && self.current_sample == work.sample
            && self.tile_plan.get(self.current_tile) == Some(&work.tile)
    }
    fn validate_work(&self, work: &OrthographicBakeWork) -> Result<(), String> {
        self.work_is_current(work)
            .then_some(())
            .ok_or_else(|| "The bake work was retired.".into())
    }
    pub fn preview_work(&mut self, work: &OrthographicBakeWork, rgba: &[u8]) -> Result<(), String> {
        self.validate_work(work)?;
        self.preview_current_tile(rgba)
    }
    pub fn finish_work_sample(&mut self, work: &OrthographicBakeWork) -> Result<bool, String> {
        self.validate_work(work)?;
        Ok(self.finish_sample())
    }
    pub fn commit_work(
        &mut self,
        work: &OrthographicBakeWork,
        rgba: Vec<u8>,
        depth: Vec<f32>,
        albedo: Vec<u8>,
        normal: Vec<u8>,
        material: Vec<u8>,
    ) -> Result<bool, String> {
        self.validate_work(work)?;
        if self.current_sample + 1 < self.requested_samples {
            return Err("The bake tile has unfinished progressive samples.".into());
        }
        self.commit_current_tile(rgba, depth, albedo, normal, material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn begin(
        bake: &mut OrthographicBakeController,
        map: Uuid,
        samples: u32,
    ) -> OrthographicBakeWork {
        bake.request_render(map, samples);
        bake.prepare_work(map, 1, 1, Camera3D::iso(), Some((0., 1., 0., 1.)))
            .unwrap()
            .unwrap()
    }
    fn commit(
        bake: &mut OrthographicBakeController,
        work: &OrthographicBakeWork,
    ) -> Result<bool, String> {
        let count = (work.tile_size * work.tile_size) as usize;
        bake.commit_work(
            work,
            [40, 50, 60, 255].repeat(count),
            vec![3.; count],
            [60, 70, 80, 255].repeat(count),
            [128, 255, 128, 255].repeat(count),
            [200, 32, 255, 0].repeat(count),
        )
    }
    #[test]
    fn deferred_samples_cannot_publish_after_sample_request_clear_or_map_switch() {
        let map = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut bake = OrthographicBakeController::default();
        let first = begin(&mut bake, map, 2);
        assert!(!bake.finish_work_sample(&first).unwrap());
        assert!(!bake.work_is_current(&first));
        assert!(commit(&mut bake, &first).is_err());
        let second = bake
            .prepare_work(map, 1, 1, Camera3D::iso(), None)
            .unwrap()
            .unwrap();
        assert!(bake.work_is_current(&second));
        let replacement = begin(&mut bake, map, 1);
        assert!(!bake.work_is_current(&second));
        assert!(commit(&mut bake, &second).is_err());
        assert!(bake.finish_work_sample(&replacement).unwrap());
        assert!(commit(&mut bake, &replacement).unwrap());
        assert!(bake.take_persisted_update_for(other).is_none());
        // The old authored asset may still be absent until the UI consumes this publication.
        bake.sync_persisted_asset(map, None);
        assert_eq!(*bake.status(), OrthographicBakeStatus::Ready);
        let asset = bake.take_persisted_update_for(map).unwrap().unwrap();
        assert_eq!(asset.tiles.len(), 1);
        bake.sync_persisted_asset(other, Some(&asset));
        bake.clear();
        assert!(bake.take_persisted_update_for(map).is_none());
        assert_eq!(bake.take_persisted_update_for(other), Some(None));

        let clear_work = begin(&mut bake, map, 1);
        bake.clear();
        assert!(commit(&mut bake, &clear_work).is_err());
        assert!(bake.take_persisted_update_for(other).is_none());
        assert_eq!(bake.take_persisted_update_for(map), Some(None));
        let retired = begin(&mut bake, map, 1);
        bake.sync_persisted_asset(other, None);
        assert!(!bake.is_rendering());
        assert!(!bake.work_is_current(&retired));
        assert!(commit(&mut bake, &retired).is_err());
        assert!(bake.take_persisted_update_for(other).is_none());
        let copied = begin(&mut bake, map, 1);
        bake = OrthographicBakeController::default();
        let fresh = begin(&mut bake, map, 1);
        assert!(!bake.work_is_current(&copied));
        assert!(bake.work_is_current(&fresh));
    }
    #[test]
    fn completed_publications_are_revoked_on_map_switch_and_new_request() {
        let map = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut bake = OrthographicBakeController::default();
        let work = begin(&mut bake, map, 1);
        assert!(commit(&mut bake, &work).unwrap());
        bake.sync_persisted_asset(other, None);
        assert!(bake.take_persisted_update_for(map).is_none());
        let work = begin(&mut bake, map, 1);
        assert!(commit(&mut bake, &work).unwrap());
        bake.request_render(map, 1);
        assert!(bake.take_persisted_update_for(map).is_none());
    }
    #[test]
    fn tile_plan_rejects_unbounded_and_invalid_extents_before_allocation() {
        let map = Uuid::new_v4();
        let mut bake = OrthographicBakeController::default();
        for bounds in [
            (0., f32::MAX, 0., f32::MAX),
            (0., f32::INFINITY, 0., 1.),
            (f32::NAN, 1., 0., 1.),
            (3., 2., 0., 1.),
        ] {
            bake.request_render(map, 1);
            assert!(
                bake.prepare_work(map, 256, 256, Camera3D::iso(), Some(bounds))
                    .is_err()
            );
            assert!(bake.tile_plan.is_empty());
        }
    }
}
