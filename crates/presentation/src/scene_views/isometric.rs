use super::*;
use crate::{
    GameSceneInput,
    isometric::{IsometricSceneInput, IsometricSceneRenderer, IsometricSourceVersion},
    screen::ContentRevision,
};
use rusterix::{client::widget::game::GameWidget, isometric::lighting::Lighting};
impl SceneViews {
    /// Revoke cached geometry when the owner changes a projection that retains
    /// its authored map ID/revision, such as a clipboard preview. Other views
    /// and immutable GPU source residency are unaffected.
    pub fn invalidate_isometric_geometry(&mut self, id: ViewId) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if let Some(renderer) = &mut view.isometric_renderer {
            renderer.invalidate_geometry();
        }
        view.isometric_frame = None;
        view.dirty = true;
        Ok(())
    }
    /// A fitted, static isometric overview. The caller supplies the projection;
    /// this path has no player dynamics, particles or simulation clock.
    pub fn prepare_isometric_overview(
        &mut self,
        id: ViewId,
        size: (u32, u32),
        map: &rusterix::Map,
        assets: &rusterix::Assets,
        projection: rusterix::isometric::Projection,
        revision: u64,
        images: &mut ImageRegistry,
    ) -> Result<()> {
        self.resize(id, size, images)?;
        if size.0 == 0 || size.1 == 0 {
            return Ok(());
        }
        self.select_game_mode(id, true, images)?;
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if view.isometric_renderer.is_none() {
            view.isometric_renderer = Some(IsometricSceneRenderer::new(self.context.clone())?);
        }
        let lighting = Lighting::new(&Default::default(), std::iter::empty(), 0);
        let frame = view.isometric_renderer.as_mut().unwrap().prepare_unlit(
            size,
            &IsometricSceneInput {
                map,
                assets,
                avatars: &Default::default(),
                seconds: 0.,
                projection,
                lighting: &lighting,
                particles: &[],
                source_version: IsometricSourceVersion {
                    namespace: id.0,
                    content: ContentRevision(revision),
                    tiles: assets.tiles_revision,
                },
            },
        )?;
        view.isometric_frame = Some(frame);
        view.dirty = true;
        Ok(())
    }
    /// The caller attaches this exact completed source receipt to the host's
    /// presentation acknowledgement; a newer prepare cannot rewrite it.
    pub fn isometric_targets(
        &self,
        output: ViewImage,
    ) -> Option<Arc<crate::isometric::IsometricWorldTargets>> {
        let view = self.views.get(&output.view)?;
        if self.context.is_lost() || view.dirty || self.output(output.view) != Some(output) {
            return None;
        }
        view.isometric_frame.as_ref().map(|frame| frame.targets())
    }

    pub(crate) fn select_game_mode(
        &mut self,
        id: ViewId,
        isometric: bool,
        images: &mut ImageRegistry,
    ) -> Result<()> {
        self.check_registry(images)?;
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if view.isometric == isometric {
            return Ok(());
        }
        let target = if !isometric && view.size.0 > 0 && view.size.1 > 0 {
            Some(SceneRenderTarget::new(
                self.context.clone(),
                view.size.0,
                view.size.1,
            )?)
        } else {
            None
        };
        if let Some(output) = view.output.take() {
            images.remove(output.image);
        }
        view.target = target;
        view.last_image = None;
        view.isometric_frame = None;
        if !isometric {
            view.isometric_renderer = None;
        }
        view.isometric = isometric;
        view.dirty = true;
        Ok(())
    }
    pub(crate) fn prepare_isometric_game(
        &mut self,
        id: ViewId,
        widget: &GameWidget,
        input: &GameSceneInput<'_>,
    ) -> Result<()> {
        let mut projection = widget
            .isometric_projection
            .ok_or_else(|| RenderError("isometric camera was not prepared".into()))?;
        // Camera placement remains in logical widget coordinates; source
        // rendering uses the same isotropic reduction as scene resolution.
        let reduction = widget.upscale.max(1.);
        projection.origin /= reduction;
        projection.unit /= reduction;
        self.prepare_isometric_scene(id, projection, input)
    }
    /// Sprite-isometric authoring uses its logical construction-plane projection
    /// and the same injected source producer as player views.
    pub fn prepare_isometric_editor(
        &mut self,
        id: ViewId,
        size: (u32, u32),
        input: &GameSceneInput<'_>,
        images: &mut ImageRegistry,
    ) -> Result<()> {
        self.resize(id, size, images)?;
        if size.0 == 0 || size.1 == 0 {
            return Ok(());
        }
        self.select_game_mode(id, true, images)?;
        self.scene_mut(id)?.runtime_render_state = input.render_state.clone();
        let projection = rusterix::isometric::Projection::editor(
            input.map,
            scenevm::prelude::Vec2::new(size.0 as f32, size.1 as f32),
        );
        self.prepare_isometric_scene(id, projection, input)
    }
    fn prepare_isometric_scene(
        &mut self,
        id: ViewId,
        projection: rusterix::isometric::Projection,
        input: &GameSceneInput<'_>,
    ) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let scene = &mut view.scene;
        scene.apply_runtime_render_state_settings();
        scene.settings.apply_hour(input.time.to_f32());
        scene.build_dynamics_2d(
            input.map,
            input.animation_frame,
            input.assets,
            &Default::default(),
        );
        let particles = scene.isometric_particles(input.map, input.assets, input.animation_frame);
        let avatars = scene.isometric_avatars(input.map, input.assets, input.animation_frame);
        let lighting = Lighting::new(
            &scene.settings,
            scene.vm.vm.lights.values(),
            input.animation_frame,
        );
        if view.isometric_renderer.is_none() {
            view.isometric_renderer = Some(IsometricSceneRenderer::new(self.context.clone())?);
        }
        let frame = view.isometric_renderer.as_mut().unwrap().prepare(
            view.size,
            &IsometricSceneInput {
                map: input.map,
                assets: input.assets,
                avatars: &avatars,
                seconds: input.animation_frame as f32 * input.assets.animation_tick_seconds(),
                projection,
                lighting: &lighting,
                particles: &particles,
                source_version: IsometricSourceVersion {
                    namespace: id.0,
                    content: ContentRevision(input.map.changed as u64),
                    tiles: input.assets.tiles_revision,
                },
            },
        )?;
        view.isometric_frame = Some(frame);
        view.dirty = true;
        Ok(())
    }
}
