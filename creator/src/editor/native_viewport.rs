//! Main Creator viewport adoption. Frame ownership and actual document/widget
//! bindings live with the application; legacy tool state is an explicit bridge
//! until the remaining authoring controllers move under the same owner.
use super::*;
use crate::editor_display::DisplayFrame;
use crate::toollist::{NativeDisplayBinding, NativeDisplayOutcome};
use std::sync::Arc;
use theframework::render_wgpu::UiFrameImages;

#[derive(Default)]
pub(crate) struct NativeViewport {
    pub(crate) enabled: bool,
    pub(super) window_focused: Option<bool>,
    pub(super) world: super::world::NativeWorld,
    pub(crate) play: super::play::NativePlay,
    pub(super) game_input: super::game_input::NativeGameInput,
    published: Option<(TheId, Arc<theframework::paint::PaintList>)>,
    binding: Option<NativeDisplayBinding>,
    epoch: u64,
}
pub(super) struct NativeDisplayFrame {
    binding: NativeDisplayBinding,
    pub(super) frame: DisplayFrame,
}
impl Editor {
    pub(crate) fn synchronize_native_document(&self) {
        if self.native_viewport.enabled
            && let Some(document) = self.documents.active_id()
        {
            TOOLLIST.write().unwrap().bind_native_document(document);
        }
    }
    pub(crate) fn native_display_owns_viewport(&self) -> bool {
        self.native_viewport.enabled
            && !self.server_ctx.game_mode
            && !self.server_ctx.game_input_mode
            && !self.server_ctx.text_game_mode
            && TOOLLIST
                .read()
                .unwrap()
                .active_editor_display_owner(&self.server_ctx)
                .is_some()
    }
    fn resolve_native_display_binding(&mut self, ui: &mut TheUI) -> Option<NativeDisplayBinding> {
        let next = (|| {
            if self.native_display_owns_viewport() && !crate::geometry_editor::is_visible(ui) {
                let document = self.documents.active_id()?;
                let owner = TOOLLIST
                    .read()
                    .unwrap()
                    .active_editor_display_owner(&self.server_ctx)?;
                let view = crate::utils::map_editor_render_view(ui, &self.server_ctx)?;
                let dim = *view.dim();
                (dim.width > 0 && dim.height > 0).then(|| NativeDisplayBinding {
                    document,
                    view: view.id().clone(),
                    size: (dim.width as usize, dim.height as usize),
                    context: self.server_ctx.pc,
                    owner,
                    epoch: self.native_viewport.epoch,
                })
            } else {
                None
            }
        })();
        if next != self.native_viewport.binding {
            self.invalidate_native_display_viewport(ui);
            self.native_viewport.binding = next.map(|mut next| {
                next.epoch = self.native_viewport.epoch;
                next
            });
        }
        self.native_viewport.binding.clone()
    }
    pub(crate) fn invalidate_native_display_viewport(&mut self, ui: &mut TheUI) {
        self.native_viewport.binding = None;
        if let Some((id, list)) = self.native_viewport.published.take()
            && let Some(view) = ui.get_render_view(&id.name)
            && view.id().uuid == id.uuid
        {
            view.clear_paint_list(&list);
        }
        self.native_viewport.epoch = self
            .native_viewport
            .epoch
            .checked_add(1)
            .expect("native viewport binding epoch exhausted");
        TOOLLIST.write().unwrap().invalidate_native_display();
    }
    pub(super) fn prepare_native_display_viewport(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut UiFrameImages<'_>,
    ) -> Result<Option<NativeDisplayFrame>, String> {
        self.synchronize_native_document();
        let Some(binding) = self.resolve_native_display_binding(ui) else {
            return Ok(None);
        };
        // begin_action_edit retains its compatibility name-based focus request.
        // Resolve it to the real widget before the native input boundary.
        if ctx
            .ui
            .focus
            .as_ref()
            .is_some_and(|focus| focus.name == binding.view.name && focus.uuid != binding.view.uuid)
        {
            ctx.ui.set_focus(&binding.view);
        }
        let Some(map) = self.documents.project.get_map(&self.server_ctx) else {
            self.invalidate_native_display_viewport(ui);
            return Ok(None);
        };
        let frame =
            TOOLLIST
                .write()
                .unwrap()
                .prepare_native_display(&binding, map, &self.server_ctx);
        let Some(frame) = frame else {
            self.invalidate_native_display_viewport(ui);
            ctx.ui.redraw_all = true;
            return Ok(None);
        };
        let list = frame
            .paint(&mut ctx.text, images, ctx.scale_factor)
            .map_err(|error| error.to_string())?;
        let Some(view) = ui.get_render_view(&binding.view.name) else {
            return Ok(None);
        };
        if view.id().uuid != binding.view.uuid {
            return Ok(None);
        }
        let list = Arc::new(list);
        view.set_paint_list(Some(list.clone()))
            .map_err(|error| error.to_string())?;
        self.native_viewport.published = Some((binding.view.clone(), list));
        Ok(Some(NativeDisplayFrame { binding, frame }))
    }
    pub(super) fn presented_native_display_viewport(
        &mut self,
        painted: Option<NativeDisplayFrame>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let binding = self.resolve_native_display_binding(ui);
        let Some(painted) = painted else {
            if binding.is_some() {
                TOOLLIST.write().unwrap().invalidate_native_display();
            }
            return;
        };
        if binding.as_ref() != Some(&painted.binding) {
            return;
        }
        let accepted = TOOLLIST.write().unwrap().presented_native_display(
            &painted.binding,
            &painted.frame,
            &[painted.frame.receipt()],
            &self.server_ctx,
        );
        if !accepted {
            ctx.ui.redraw_all = true;
        }
    }
    pub(crate) fn handle_native_display_input(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        if !self.native_viewport.enabled {
            return false;
        }
        let pointer_id = match event {
            TheEvent::RenderViewClicked(id, _)
            | TheEvent::RenderViewDragged(id, _)
            | TheEvent::RenderViewUp(id, _)
            | TheEvent::RenderViewHoverChanged(id, _)
            | TheEvent::RenderViewLostHover(id)
            | TheEvent::RenderViewScrollBy(id, _)
            | TheEvent::RenderViewPreciseScrollBy(id, _)
            | TheEvent::RenderViewZoomBy(id, _)
            | TheEvent::RenderViewContext(id, _) => Some(id),
            TheEvent::KeyCodeDown(_) => None,
            _ => return false,
        };
        self.synchronize_native_document();
        let Some(binding) = self.resolve_native_display_binding(ui) else {
            return self.native_display_owns_viewport()
                && pointer_id.is_some_and(|id| {
                    id.name == crate::utils::map_editor_render_view_name(&self.server_ctx)
                });
        };
        if pointer_id.is_some_and(|id| id.name != binding.view.name) {
            return false;
        }
        let focused = ui.dialog.is_none()
            && ui.popover.is_none()
            && ctx
                .ui
                .focus
                .as_ref()
                .is_some_and(|id| id.uuid == binding.view.uuid);
        if pointer_id.is_none() && !focused {
            return false;
        }
        let outcome = TOOLLIST.write().unwrap().native_display_event(
            &binding,
            event,
            focused,
            ui.logo || ui.ctrl,
            &mut self.documents.project,
            &self.server_ctx,
            ctx,
        );
        match outcome {
            NativeDisplayOutcome::Pass => pointer_id.is_some(),
            NativeDisplayOutcome::Consumed => true,
            NativeDisplayOutcome::CommitAction(id) => {
                // Single platform-thread transaction: dispatch before processing
                // another document event. No generic queued Apply can cross tabs.
                if Some(id) == self.server_ctx.curr_action_id
                    && self.documents.active_id() == Some(binding.document)
                {
                    self.sidebar.handle_event(
                        &TheEvent::StateChanged(
                            TheId::named("Action Apply"),
                            TheWidgetState::Clicked,
                        ),
                        ui,
                        ctx,
                        &mut self.documents.project,
                        &mut self.server_ctx,
                    );
                }
                true
            }
        }
    }
}
