use super::*;
use crate::{
    GameSceneInput,
    isometric::{IsometricSceneInput, IsometricSceneRenderer, IsometricSourceVersion},
    screen::ContentRevision,
};
use rusterix::{client::widget::game::GameWidget, isometric::lighting::Lighting};
impl SceneViews {
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
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let mut projection = widget
            .isometric_projection
            .ok_or_else(|| RenderError("isometric camera was not prepared".into()))?;
        // Camera placement remains in logical widget coordinates; source
        // rendering uses the same isotropic reduction as scene resolution.
        let reduction = widget.upscale.max(1.);
        projection.origin /= reduction;
        projection.unit /= reduction;
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
