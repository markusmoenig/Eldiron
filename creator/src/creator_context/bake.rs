//! Reconcile transient bake scope before selecting a viewport producer.
use super::CreatorContext;
use crate::prelude::*;

impl CreatorContext {
    pub(crate) fn reconcile_bake_scope(&mut self, project: &Project, playing: bool) {
        if !self.bake.initialized() {
            return;
        }
        let region = (self.get_map_context() == MapContext::Region && !self.pc.is_prefab())
            .then(|| project.get_region(&self.curr_region))
            .flatten();
        let mut bake = self.bake.write();
        let was_rendering = bake.is_rendering();
        if playing || self.editor_view_mode != EditorViewMode::Iso || region.is_none() {
            bake.cancel_work();
        }
        if let Some(region) = region {
            bake.sync_persisted_asset(region.map.id, region.map.orthographic_bake.as_ref());
        } else {
            // No selected region owns a pending completion or cached source.
            bake.reset();
        }
        if was_rendering && !bake.is_rendering() {
            self.background_progress = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_retires_bakes_before_2d_other_map_play_or_missing_region_can_select_a_producer() {
        let mut project = Project::new();
        let region = project.regions[0].id;
        let map = project.regions[0].map.id;
        let mut owner = CreatorContext::new();
        owner.curr_region = region;
        owner.pc = ProjectContext::Region(region);
        owner.editor_view_mode = EditorViewMode::Iso;
        owner.reconcile_bake_scope(&project, false);
        assert!(!owner.bake.initialized());
        for phase in 0..4 {
            owner.curr_region = region;
            owner.pc = ProjectContext::Region(region);
            owner.editor_view_mode = EditorViewMode::Iso;
            owner.bake.write().request_render(map, 32);
            owner.background_progress = owner.bake.read().progress_text();
            match phase {
                0 => owner.editor_view_mode = EditorViewMode::D2,
                1 => {
                    let other = Region::new();
                    owner.curr_region = other.id;
                    project.regions.push(other);
                }
                2 => {}
                _ => owner.curr_region = Uuid::new_v4(),
            }
            owner.reconcile_bake_scope(&project, phase == 2);
            assert!(!owner.bake.read().is_rendering());
            assert!(owner.background_progress.is_none());
            assert!(owner.bake.write().take_persisted_update_for(map).is_none());
        }
        assert!(!owner.reference.initialized());
        assert!(!owner.jobs.scene_initialized());
    }
}
