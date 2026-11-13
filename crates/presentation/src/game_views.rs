use crate::{SceneViews, ViewId};
use renderkit::{ImageRegistry, RenderError, Result};
use rusterix::server::message::RuntimeRenderState;
use rusterix::{Assets, Map, client::widget::game::GameWidget};
use theframework_core::TheTime;

/// Borrowed CPU view state. No presenter advances or drains the authoritative session.
pub struct GameSceneInput<'a> {
    pub map: &'a Map,
    pub assets: &'a Assets,
    pub time: TheTime,
    pub animation_frame: usize,
    pub render_state: &'a RuntimeRenderState,
}
impl SceneViews {
    /// Call after creating a game view or changing its source assets/configuration.
    /// The engine's 2D program and atlas are installed on this view's injected VM.
    pub fn configure_game(&mut self, id: ViewId, assets: &Assets) -> Result<()> {
        self.configure_scene(id, assets, false)
    }
    pub(crate) fn configure_scene(
        &mut self,
        id: ViewId,
        assets: &Assets,
        editor: bool,
    ) -> Result<()> {
        let scene = self.scene_mut(id)?;
        scene.sync_base_render_settings(&assets.config);
        let shader = rusterix::Embedded::get("shader/2d_shader.wgsl")
            .ok_or_else(|| RenderError("missing engine 2D program".into()))?;
        let source = std::str::from_utf8(&shader.data).map_err(|e| RenderError(e.to_string()))?;
        scene.vm.set_active_vm(0);
        scene.vm.execute(scenevm::Atom::SetSource2D(source.into()));
        scene.build_atlas(&assets.tiles, editor);
        scene.sync_prefab_particle_sprites(&assets.block_props);
        scene.vm.execute(scenevm::Atom::SetPalette(
            assets
                .palette
                .colors
                .iter()
                .map(|entry| {
                    entry
                        .as_ref()
                        .map_or(scenevm::prelude::Vec4::zero(), |color| {
                            let [r, g, b, a] = color.to_array();
                            scenevm::prelude::Vec4::new(
                                r.max(0.).powf(2.2),
                                g.max(0.).powf(2.2),
                                b.max(0.).powf(2.2),
                                a,
                            )
                        })
                })
                .collect(),
        ));
        scene.mark_dynamics_dirty();
        Ok(())
    }
    /// Prepare each game widget against its own camera/history/target. Logical
    /// dimensions and optional scene upscaling never allocate CPU output storage.
    pub fn prepare_game(
        &mut self,
        id: ViewId,
        widget: &mut GameWidget,
        logical: (u32, u32),
        input: &GameSceneInput<'_>,
        images: &mut ImageRegistry,
    ) -> Result<()> {
        if logical.0 == 0 || logical.1 == 0 {
            return self.resize(id, (0, 0), images);
        }
        let size = widget.gpu_scene_resolution(logical).map_err(RenderError)?;
        self.resize(id, size, images)?;
        let scene = self.scene_mut(id)?;
        scene.runtime_render_state = input.render_state.clone();
        widget
            .prepare_gpu_scene(
                logical,
                input.map,
                &input.time,
                input.animation_frame,
                input.assets,
                scene,
            )
            .map_err(RenderError)?;
        self.apply_game_surface_paint(id, input.assets.surface_paint.get(&input.map.id))?;
        // Player/map synchronization can change the camera during preparation.
        self.select_game_mode(id, widget.camera.is_isometric_2d(), images)?;
        if widget.camera.is_isometric_2d() {
            self.prepare_isometric_game(id, widget, input)?;
        }
        Ok(())
    }
}
