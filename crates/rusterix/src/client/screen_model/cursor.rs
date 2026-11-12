//! Cursor selection is authored screen policy. Hosts own pointer eligibility,
//! assets and drawing; this module never allocates a destination bitmap.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorPair {
    pub normal: Option<Uuid>,
    pub pressed: Option<Uuid>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IntentCursors {
    pub entity: CursorPair,
    pub item: CursorPair,
}
impl IntentCursors {
    pub fn target(self, entity: bool, item: bool) -> Option<CursorPair> {
        if entity {
            Some(CursorPair {
                normal: self.entity.normal.or(self.item.normal),
                pressed: self.entity.pressed.or(self.item.pressed),
            })
        } else if item {
            Some(self.item)
        } else {
            None
        }
    }
}
pub fn active_intent_cursors(
    activated: &[u32],
    buttons: &FxHashMap<u32, Widget>,
    intent: &str,
) -> Option<IntentCursors> {
    activated.iter().rev().find_map(|id| {
        let widget = buttons.get(id)?;
        let active = widget
            .intent_payload()
            .is_some_and(|intent| !intent.trim().is_empty())
            || (widget.command_slot.is_some() && !intent.trim().is_empty());
        active.then_some(IntentCursors {
            entity: CursorPair {
                normal: widget.entity_cursor_id,
                pressed: widget.entity_clicked_cursor_id,
            },
            item: CursorPair {
                normal: widget.item_cursor_id,
                pressed: widget.item_clicked_cursor_id,
            },
        })
    })
}
pub fn selected_intent(
    activated: &[u32],
    buttons: &FxHashMap<u32, Widget>,
    fallback: &str,
) -> Option<String> {
    activated
        .iter()
        .rev()
        .filter_map(|id| buttons.get(id))
        .find_map(|widget| widget.intent_payload().filter(|intent| !intent.is_empty()))
        .or_else(|| (!fallback.is_empty()).then(|| fallback.to_owned()))
}
pub fn intent_cursor_command(intent: &str, hovered_item: Option<u32>, map: &Map) -> Option<String> {
    if intent.is_empty() {
        return None;
    }
    let command = if let Some(action) = intent.strip_prefix("action:") {
        format!("rules.{action}")
    } else {
        format!("intent.{intent}")
    };
    contextual_open_close_command(Some(command), hovered_item, map)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newer_intents_win_non_intent_controls_do_not_mask_and_entity_fallback_preserves_pressed_ids()
    {
        let mut buttons = FxHashMap::default();
        let mut old = Widget::new();
        old.command = Some("intent.use".into());
        let mut current = Widget::new();
        current.command = Some("intent.look".into());
        let item = Uuid::new_v4();
        let pressed = Uuid::new_v4();
        current.item_cursor_id = Some(item);
        current.entity_clicked_cursor_id = Some(pressed);
        buttons.insert(1, old);
        buttons.insert(2, current);
        buttons.insert(3, Widget::new());
        assert_eq!(
            selected_intent(&[1, 2, 3], &buttons, "drop").as_deref(),
            Some("look")
        );
        let cursors = active_intent_cursors(&[1, 2, 3], &buttons, "look").unwrap();
        assert_eq!(
            cursors.target(true, false),
            Some(CursorPair {
                normal: Some(item),
                pressed: Some(pressed)
            })
        );
        assert_eq!(
            cursors.target(false, true),
            Some(CursorPair {
                normal: Some(item),
                pressed: None
            })
        );
        assert_eq!(cursors.target(false, false), None);
        assert_eq!(
            intent_cursor_command("action:attack", None, &Map::default()).as_deref(),
            Some("rules.attack")
        );
    }
}

pub fn owned_item_cursor_intent(intent: Option<&str>) -> bool {
    intent.is_some_and(|intent| intent.eq_ignore_ascii_case("drop"))
}
pub const DRAG_TARGET_FILL: crate::Pixel = [238, 210, 96, 70];
pub const DRAG_TARGET_BORDER: crate::Pixel = [255, 236, 132, 255];
