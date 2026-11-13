//! Native viewport binding and source-checked profile transactions. The parent
//! supplies the document-session/widget lifetime and dispatches commits inline.
use super::*;
use crate::editor_display::{DisplayFrame, DisplayInput, DisplayTarget};
use crate::editor_overlay::panel::PanelScope;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeDisplayBinding {
    pub(crate) document: Uuid,
    pub(crate) view: TheId,
    pub(crate) size: (usize, usize),
    pub(crate) owner: EditorDisplayOwner,
    pub(crate) context: ProjectContext,
    pub(crate) epoch: u64,
}
impl NativeDisplayBinding {
    pub(crate) fn target(&self) -> DisplayTarget<'_> {
        DisplayTarget {
            scope: PanelScope {
                document: self.document,
                tool: match self.owner {
                    EditorDisplayOwner::Action(id) | EditorDisplayOwner::Tool(id) => id,
                },
                revision: self.epoch,
            },
            owner: self.owner,
            context: self.context,
            size: self.size,
            view_name: &self.view.name,
            visible: true,
        }
    }
}
pub(crate) enum NativeDisplayOutcome {
    Pass,
    Consumed,
    CommitAction(Uuid),
}
impl ToolList {
    pub(crate) fn enable_native_display(&mut self) {
        self.native_display_mode = true;
        self.invalidate_native_display();
    }

    pub(crate) fn bind_native_document(&mut self, document: Uuid) {
        if self.native_document != Some(document) {
            // The outgoing document is cancelled before its normal snapshot.
            // Never restore that old draft after a new document is installed.
            self.editor_display_session = None;
            self.native_document = Some(document);
        }
    }
    pub(crate) fn active_editor_display_owner(
        &self,
        server: &ServerContext,
    ) -> Option<EditorDisplayOwner> {
        let session = self.editor_display_session.as_ref()?;
        if self.editor_mode || session.project_context != server.pc {
            return None;
        }
        let active = match session.owner {
            EditorDisplayOwner::Action(id) => Some(id) == server.curr_action_id,
            EditorDisplayOwner::Tool(id) => self
                .game_tools
                .get(self.curr_game_tool)
                .is_some_and(|tool| tool.id().uuid == id),
        };
        active.then_some(session.owner)
    }
    fn native_display_source_current(&self, creator_actions: &CreatorActions) -> bool {
        let Some(session) = &self.editor_display_session else {
            return false;
        };
        match session.owner {
            EditorDisplayOwner::Action(id) => creator_actions.read()
                .get_action_by_id(id)
                .is_some_and(|action| {
                    action.editor_display().as_ref() == Some(session.working_display())
                }),
            EditorDisplayOwner::Tool(id) => self
                .game_tools
                .iter()
                .find(|tool| tool.id().uuid == id)
                .is_some_and(|tool| {
                    tool.editor_display().as_ref() == Some(session.working_display())
                }),
        }
    }
    pub(crate) fn invalidate_native_display(&mut self) {
        if let Some(session) = &mut self.editor_display_session {
            session.invalidate_frame();
        }
    }
    pub(crate) fn prepare_native_display(
        &mut self,
        binding: &NativeDisplayBinding,
        map: &Map,
        server: &ServerContext,
    ) -> Option<DisplayFrame> {
        if self.native_document != Some(binding.document)
            || self.active_editor_display_owner(server) != Some(binding.owner)
            || binding.context != server.pc
            || !self.native_display_source_current( &server.actions)
        {
            // A concurrent owner edit wins. Never restore an old draft over it.
            self.editor_display_session = None;
            return None;
        }
        let session = self.editor_display_session.as_mut()?;
        session.set_grid_subdivisions(map.subdivisions);
        session.prepare_frame(binding.target())
    }
    pub(crate) fn presented_native_display(
        &mut self,
        binding: &NativeDisplayBinding,
        frame: &DisplayFrame,
        receipts: &[theframework::paint::FrameReceipt],
        server: &ServerContext,
    ) -> bool {
        if self.native_document != Some(binding.document)
            || self.active_editor_display_owner(server) != Some(binding.owner)
            || !self.native_display_source_current( &server.actions)
        {
            return false;
        }
        self.editor_display_session
            .as_mut()
            .is_some_and(|session| session.presented_paint(frame, receipts, binding.target()))
    }
    pub(crate) fn native_display_event(
        &mut self,
        binding: &NativeDisplayBinding,
        event: &TheEvent,
        focused: bool,
        command_zoom: bool,
        project: &mut Project,
        server: &ServerContext,
        ctx: &mut TheContext,
    ) -> NativeDisplayOutcome {
        use NativeDisplayOutcome::*;
        if self.native_document != Some(binding.document)
            || self.active_editor_display_owner(server) != Some(binding.owner)
            || binding.context != server.pc
            || !self.native_display_source_current( &server.actions)
        {
            self.editor_display_session = None;
            ctx.ui.redraw_all = true;
            return Consumed;
        }
        // Names route the established event family; UUID guards the actual
        // widget instance. A recreated view cannot reuse old painted geometry.
        let pointer_id = match event {
            TheEvent::RenderViewClicked(id, _)
            | TheEvent::RenderViewDragged(id, _)
            | TheEvent::RenderViewUp(id, _)
            | TheEvent::RenderViewHoverChanged(id, _)
            | TheEvent::RenderViewLostHover(id)
            | TheEvent::RenderViewScrollBy(id, _)
            | TheEvent::RenderViewPreciseScrollBy(id, _)
            | TheEvent::RenderViewZoomBy(id, _) => Some(id),
            _ => None,
        };
        if let Some(id) = pointer_id {
            if id.name != binding.view.name {
                return Pass;
            }
            if id.uuid != binding.view.uuid {
                return Consumed;
            }
        }
        let session = self.editor_display_session.as_mut().unwrap();
        let expected = session.working_display().clone();
        let outcome =
            session.handle_presented_event(event, binding.target(), focused, command_zoom);
        let result = match outcome {
            DisplayInput::Unavailable => {
                ctx.ui.redraw_all = true;
                return Consumed;
            }
            DisplayInput::Outcome(result) => result,
        };
        match result {
            EditorDisplayResult::Ignored => return Pass,
            EditorDisplayResult::Handled => (),
            EditorDisplayResult::DisplayChanged(display) => {
                if !self.update_editor_display_owner(binding.owner, &expected, &display, &server.actions) {
                    self.editor_display_session = None;
                }
            }
            EditorDisplayResult::GridSubdivisionChanged(value) => {
                if let Some(map) = project.get_map_mut(server) {
                    map.subdivisions = value;
                }
            }
            EditorDisplayResult::Cancel => {
                self.cancel_editor_display_session( &server.actions);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Editor display changes cancelled.".into(),
                ));
            }
            EditorDisplayResult::Commit => {
                self.editor_display_session = None;
                ctx.ui.redraw_all = true;
                if let EditorDisplayOwner::Action(id) = binding.owner {
                    return CommitAction(id);
                }
            }
        }
        ctx.ui.redraw_all = true;
        Consumed
    }
}
