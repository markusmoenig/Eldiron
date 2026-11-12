//! Drop highlighting follows the same acknowledged target/transfer path as release.
use super::*;
use rusterix::item_interaction;
impl GameScreenController {
    /// Presentation eligibility is optimistic; the server still owns transfers.
    pub fn drag_target_feedback(&self, world: &RuntimeView<'_>) -> Option<Rect> {
        let point = self.pointer_visual_position()?;
        if let Some(bounds) = self.command_drag_feedback(world, point) {
            return Some(bounds);
        }
        let (source, binding) = self.item_drag_source(world)?;
        // The input path enforces current/displayed ownership, target addresses,
        // modal/epoch/source checks. This is a pure query; no action is sent.
        self.release_item_drag(world, None, source, binding, point)?;
        let frame = self.world.displayed.as_ref()?;
        let owner = frame
            .controls
            .iter()
            .rev()
            .find(|control| control.rect.contains(point))?
            .owner?;
        let target = frame.drops.iter().find(|target| target.source == owner)?;
        item_interaction::can_highlight_transfer(world.map?, source, &target.destination)
            .then_some(target.rect)
    }
}
