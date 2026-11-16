//! Scoped preparation/presentation and native input for source-owned displays.
use super::*;
use crate::editor_overlay::panel::{PanelFrame, PanelFrames, PanelScope};
#[cfg(test)]
use crate::editor_overlay::EditorOverlay;

/// Current authoritative binding, supplied by the owning viewport controller.
/// Scope revision is a binding/source epoch; local draft edits have their own
/// session revision. Visibility includes the final viewport/layout clip.
#[derive(Clone, Copy)]
pub(crate) struct DisplayTarget<'a> {
    pub(crate) scope: PanelScope,
    pub(crate) owner: EditorDisplayOwner,
    pub(crate) context: ProjectContext,
    pub(crate) size: (usize, usize),
    pub(crate) view_name: &'a str,
    pub(crate) visible: bool,
}
#[derive(Debug, PartialEq)]
pub(crate) enum DisplayInput {
    Unavailable,
    Outcome(EditorDisplayResult),
}
#[derive(Clone, Debug, PartialEq)]
struct Stamp {
    owner: EditorDisplayOwner,
    context: ProjectContext,
    view: String,
    revision: u64,
    zoom: f32,
    pan: Vec2<f32>,
    grid: Option<f32>,
    scale: f32,
}
#[derive(Clone)]
struct Presented {
    scope: PanelScope,
    size: (usize, usize),
    stamp: Stamp,
    transform: ProfileTransform,
}
#[derive(Clone)]
struct Capture {
    scope: PanelScope,
    size: (usize, usize),
    owner: EditorDisplayOwner,
    context: ProjectContext,
    view: String,
}
#[derive(Default)]
pub(super) struct DisplayFrames {
    frames: PanelFrames<Stamp, ()>,
    presented: Option<Presented>,
    capture: Option<Capture>,
}
impl std::fmt::Debug for DisplayFrames {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DisplayFrames")
            .field("presented", &self.presented.is_some())
            .field("captured", &self.capture.is_some())
            .finish_non_exhaustive()
    }
}
pub(crate) struct DisplayFrame {
    panel: PanelFrame<Stamp, ()>,
    transform: ProfileTransform,
}
impl DisplayFrame {
    #[cfg(test)]
    pub(crate) fn commands(&self) -> &EditorOverlay {
        &self.panel.commands
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn receipt(&self) -> theframework::paint::FrameReceipt {
        self.panel.receipt()
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn paint(
        &self,
        text: &mut theframework::text::TextSystem,
        images: &mut dyn crate::editor_overlay::OverlayImages,
        density: f32,
    ) -> Result<theframework::paint::PaintList, theframework::theui::paint::UiPaintError> {
        self.panel.paint(text, images, density)
    }
}
impl EditorDisplaySession {
    fn stamp(&self, view: &str) -> Stamp {
        Stamp {
            owner: self.owner,
            context: self.project_context,
            view: view.into(),
            revision: self.data_revision,
            zoom: self.zoom,
            pan: self.pan,
            grid: self.grid_subdivisions,
            scale: self.profile_scale,
        }
    }
    fn target_valid(&self, target: DisplayTarget<'_>) -> bool {
        target.visible
            && target.size.0 > 0
            && target.size.1 > 0
            && target.size.0 <= i32::MAX as usize
            && target.size.1 <= i32::MAX as usize
            && self.is_for(target.owner, target.context)
    }
    /// Source/geometry preparation only. No UI context, font service, GPU or
    /// destination buffer is constructed, and old input geometry is not promoted.
    pub(crate) fn prepare_frame(&mut self, target: DisplayTarget<'_>) -> Option<DisplayFrame> {
        if !self.target_valid(target) {
            self.invalidate_frame();
            return None;
        }
        let (commands, transform) = self.prepare_overlay(target.size);
        let transform = transform?;
        let bounds = TheDim::sized(target.size.0 as i32, target.size.1 as i32);
        let stamp = self.stamp(target.view_name);
        let panel =
            self.frames
                .frames
                .stage_commands(target.scope, stamp, commands, bounds, vec![]);
        Some(DisplayFrame { panel, transform })
    }
    pub(crate) fn invalidate_frame(&mut self) {
        self.frames.frames.invalidate();
        self.frames.presented = None;
        self.frames.capture = None;
        self.dragging = false;
        self.dragging_profile_scale = false;
        self.hover_coord = None;
        self.hovered_point = None;
        self.last_transform = None;
        self.last_view_width = 0;
        self.last_view_height = 0;
    }
    fn activate_frame(&mut self, frame: &DisplayFrame, target: DisplayTarget<'_>, stamp: Stamp) {
        self.last_transform = Some(frame.transform);
        self.last_view_width = target.size.0 as i32;
        self.last_view_height = target.size.1 as i32;
        self.frames.presented = Some(Presented {
            scope: target.scope,
            size: target.size,
            stamp,
            transform: frame.transform,
        });
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn presented_paint(
        &mut self,
        frame: &DisplayFrame,
        receipts: &[theframework::paint::FrameReceipt],
        target: DisplayTarget<'_>,
    ) -> bool {
        if !self.target_valid(target) {
            self.invalidate_frame();
            return false;
        }
        let stamp = self.stamp(target.view_name);
        if !self.frames.frames.presented_paint(
            &frame.panel,
            receipts,
            target.scope,
            target.size,
            &stamp,
        ) {
            return false;
        }
        self.activate_frame(frame, target, stamp);
        true
    }
    #[cfg(all(test, not(feature = "native-ui")))]
    fn presented_reference(&mut self, frame: &DisplayFrame, target: DisplayTarget<'_>) -> bool {
        if !self.target_valid(target) {
            return false;
        }
        let stamp = self.stamp(target.view_name);
        if !self.frames.frames.acknowledge_reference(
            &frame.panel,
            target.scope,
            target.size,
            &stamp,
        ) {
            return false;
        }
        self.activate_frame(frame, target, stamp);
        true
    }
    /// Native routing never falls through from Unavailable into world authoring.
    /// New picking requires visible geometry; a captured local draft drag can
    /// continue between presentations, as in the reference controller.
    pub(crate) fn handle_presented_event(
        &mut self,
        event: &TheEvent,
        target: DisplayTarget<'_>,
        focused: bool,
        command_zoom: bool,
    ) -> DisplayInput {
        let is_pointer = match event {
            TheEvent::RenderViewClicked(id, _)
            | TheEvent::RenderViewDragged(id, _)
            | TheEvent::RenderViewUp(id, _)
            | TheEvent::RenderViewHoverChanged(id, _)
            | TheEvent::RenderViewLostHover(id)
            | TheEvent::RenderViewScrollBy(id, _)
            | TheEvent::RenderViewPreciseScrollBy(id, _)
            | TheEvent::RenderViewZoomBy(id, _) => {
                if id.name != target.view_name {
                    return DisplayInput::Outcome(EditorDisplayResult::Ignored);
                }
                true
            }
            TheEvent::KeyCodeDown(_) if focused => false,
            _ => return DisplayInput::Outcome(EditorDisplayResult::Ignored),
        };
        if !self.target_valid(target) {
            self.invalidate_frame();
            return DisplayInput::Unavailable;
        }
        let Some(presented) = self.frames.presented.clone() else {
            return DisplayInput::Unavailable;
        };
        if presented.scope != target.scope
            || presented.size != target.size
            || presented.stamp.owner != target.owner
            || presented.stamp.context != target.context
            || presented.stamp.view != target.view_name
        {
            self.invalidate_frame();
            return DisplayInput::Unavailable;
        }
        if matches!(event, TheEvent::RenderViewClicked(_, _)) {
            self.frames.capture = None;
            self.dragging = false;
            self.dragging_profile_scale = false;
        }
        let continuation = matches!(
            event,
            TheEvent::RenderViewDragged(_, _) | TheEvent::RenderViewUp(_, _)
        );
        if continuation && let Some(capture) = &self.frames.capture {
            if capture.scope != target.scope
                || capture.size != target.size
                || capture.owner != target.owner
                || capture.context != target.context
                || capture.view != target.view_name
            {
                self.invalidate_frame();
                return DisplayInput::Unavailable;
            }
        } else if matches!(
            event,
            TheEvent::RenderViewClicked(_, _) | TheEvent::RenderViewHoverChanged(_, _)
        ) {
            if presented.stamp != self.stamp(target.view_name) {
                if matches!(event, TheEvent::RenderViewClicked(_, _)) {
                    self.selected_point = None;
                }
                return DisplayInput::Unavailable;
            }
            let coord = match event {
                TheEvent::RenderViewClicked(_, c) | TheEvent::RenderViewHoverChanged(_, c) => *c,
                _ => unreachable!(),
            };
            if coord.x < 0
                || coord.y < 0
                || coord.x as usize >= target.size.0
                || coord.y as usize >= target.size.1
            {
                return DisplayInput::Outcome(EditorDisplayResult::Ignored);
            }
        }
        if continuation && self.frames.capture.is_none() {
            // An orphaned release only clears transient flags. An unowned drag
            // cannot mutate a draft through stale reference-controller state.
            self.dragging = false;
            self.dragging_profile_scale = false;
        }
        self.last_transform = Some(presented.transform);
        let result = self.handle_event_with_modifiers(event, target.view_name, command_zoom);
        if is_pointer
            && matches!(event, TheEvent::RenderViewClicked(_, _))
            && (self.dragging || self.dragging_profile_scale)
        {
            self.frames.capture = Some(Capture {
                scope: target.scope,
                size: target.size,
                owner: target.owner,
                context: target.context,
                view: target.view_name.into(),
            });
        }
        if matches!(event, TheEvent::RenderViewUp(_, _)) {
            self.frames.capture = None;
        }
        if matches!(
            result,
            EditorDisplayResult::Commit | EditorDisplayResult::Cancel
        ) {
            self.invalidate_frame();
        }
        DisplayInput::Outcome(result)
    }
}
#[cfg(test)]
mod tests;
