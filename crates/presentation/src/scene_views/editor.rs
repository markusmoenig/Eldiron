//! Ordinary authoring scene views, owned by the application on its host device.
//! This does not handle sprite-isometric output, tool HUD or speech composition.
use crate::{SceneViews, ViewId};
use renderkit::{ImageRegistry, RenderError, Result};
use rusterix::scene_build_index::SceneBuildSnapshot;
use rusterix::{Assets, editor_view::EditorViewFrame};
use std::sync::Arc;

impl SceneViews {
    /// Overview consumers never borrow the main viewport's VM/camera or target.
    pub fn prepare_minimap(
        &mut self,
        id: ViewId,
        frame: &rusterix::editor_view::MinimapView2D,
        images: &mut ImageRegistry,
    ) -> Result<()> {
        self.resize(id, frame.size, images)?;
        if frame.size.0 == 0 || frame.size.1 == 0 {
            return Ok(());
        }
        self.select_game_mode(id, false, images)?;
        frame.prepare(self.scene_mut(id)?);
        Ok(())
    }
    /// Install source atlas/shaders and independent authoring-guide layers.
    /// Documents, build jobs and source revisions remain with the application.
    pub fn configure_editor(&mut self, id: ViewId, assets: &Assets) -> Result<()> {
        self.configure_scene(id, assets, true)?;
        let scene = self.scene_mut(id)?;
        scene.clear_overlay();
        scene.set_overlay();
        Ok(())
    }
    /// Tick the owning author's pending particle clock without modifying its
    /// completed VM output. Input may still query that displayed frame before
    /// prepare_editor consumes these steps. Call once per authoring update,
    /// never once per consumer or rendered frame.
    pub fn tick_editor_particles(&mut self, id: ViewId, two_d: bool) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if two_d {
            view.scene.tick_particle_clock_2d();
        } else {
            view.scene.tick_particle_clock_3d();
        }
        Ok(())
    }
    /// Publish streamed world geometry to this view without borrowing another
    /// renderer/VM. Unchanged snapshots are O(1); changed publications compare
    /// immutable chunk identities and preserve all independent overlay layers.
    /// Returns whether this publication changed the scene's static geometry.
    /// Once published, update these base chunks through new source snapshots;
    /// `scene_mut` remains available for settings, dynamics and guide layers.
    pub fn apply_editor_source(
        &mut self,
        id: ViewId,
        source: Arc<SceneBuildSnapshot>,
    ) -> Result<bool> {
        if self.context.is_lost() {
            return Err(RenderError("scene device lost".into()));
        }
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if view
            .editor_source
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &source))
        {
            return Ok(false);
        }
        let previous = view
            .editor_source
            .as_ref()
            .map(|snapshot| snapshot.chunks());
        let removed: Vec<_> = previous
            .into_iter()
            .flat_map(|chunks| chunks.keys())
            .filter(|origin| !source.chunks().contains_key(origin))
            .copied()
            .collect();
        let changed: Vec<_> = source
            .chunks()
            .iter()
            .filter(|(origin, chunk)| {
                previous
                    .and_then(|chunks| chunks.get(origin))
                    .is_none_or(|old| !Arc::ptr_eq(old, chunk))
            })
            .map(|(origin, chunk)| (*origin, chunk.clone()))
            .collect();
        let dirty = !removed.is_empty() || !changed.is_empty();
        if dirty {
            let active = view.scene.vm.active_vm_index();
            view.scene.vm.set_active_vm(0);
            for origin in removed
                .into_iter()
                .chain(changed.iter().map(|(origin, _)| *origin))
            {
                view.scene.vm.execute(scenevm::Atom::RemoveChunkAt {
                    origin: scenevm::prelude::Vec2::new(origin.0, origin.1),
                });
                view.scene.build_index.remove_chunk_origin(origin);
            }
            for (_, chunk) in changed {
                view.scene.build_index.index_chunk(&chunk);
                view.scene.vm.execute(scenevm::Atom::AddChunk {
                    id: theframework_core::Uuid::new_v4(),
                    chunk: chunk.as_ref().clone(),
                });
            }
            view.scene.vm.set_active_vm(active);
            view.scene.mark_dynamics_dirty();
            view.dirty = true;
            view.isometric_frame = None;
        }
        view.editor_source = Some(source);
        Ok(dirty)
    }

    /// Consume an immutable CPU description on this view's own SceneHandler.
    /// Target rendering and capture are explicit separate operations. Zero size
    /// suspends publication without executing scene preparation or CPU painting.
    pub fn prepare_editor(
        &mut self,
        id: ViewId,
        frame: &EditorViewFrame,
        images: &mut ImageRegistry,
    ) -> Result<()> {
        self.resize(id, frame.size(), images)?;
        if frame.size().0 == 0 || frame.size().1 == 0 {
            return Ok(());
        }
        self.select_game_mode(id, false, images)?;
        frame.prepare(self.scene_mut(id)?);
        Ok(())
    }
}
