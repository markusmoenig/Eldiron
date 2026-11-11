//! Pointer capture and pending keyboard selection shared by graphical hosts.
//! Values are committed through the containing screen's binding transaction.
use super::{ChoiceWidget, ChoiceWidgetKind, widget_name_is_hidden};
use crate::client::widget::choice::ChoiceInteraction;
use theframework::prelude::{FxHashMap, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceNavigation {
    Previous,
    Next,
    First,
    Last,
    Accept,
    Cancel,
}
#[derive(Default, Debug, PartialEq, Eq)]
pub struct ChoiceOutcome {
    pub consumed: bool,
    pub selection: Option<(String, String)>,
}
#[derive(Default)]
pub struct ChoiceController {
    open: Option<u32>,
    original: Option<usize>,
    pressed: bool,
}
impl ChoiceController {
    pub fn open_id(&self) -> Option<u32> {
        self.open
    }
    pub fn cancel(&mut self, widgets: &mut FxHashMap<u32, ChoiceWidget>) -> bool {
        let Some(id) = self.open.take() else {
            return false;
        };
        if let Some(widget) = widgets.get_mut(&id) {
            widget.open = false;
            if let Some(original) = self.original {
                widget.selected = original.min(widget.options.len().saturating_sub(1));
            }
        }
        self.original = None;
        true
    }
    /// Hidden/deleted controls release capture. Pending keyboard selection stays
    /// transient while a popup is open; other controls follow current bindings.
    pub fn prepare(
        &mut self,
        widgets: &mut FxHashMap<u32, ChoiceWidget>,
        hidden: &[String],
        state: &FxHashMap<String, String>,
    ) -> bool {
        let valid = self.open.is_some_and(|id| {
            widgets.get(&id).is_some_and(|w| {
                w.open
                    && w.kind == ChoiceWidgetKind::Dropdown
                    && !w.options.is_empty()
                    && !widget_name_is_hidden(&w.name, hidden)
            })
        });
        let changed = self.open.is_some() && !valid && self.cancel(widgets);
        for (id, widget) in widgets {
            widget.open = self.open == Some(*id);
            if !widget.open {
                widget.sync_value(state.get(&widget.binding).map(String::as_str));
            }
        }
        changed
    }
    pub fn pointer_is_captured(&self) -> bool {
        self.pressed
    }
    pub fn pointer_up(&mut self) -> bool {
        std::mem::take(&mut self.pressed)
    }
    pub fn pointer(
        &mut self,
        widgets: &mut FxHashMap<u32, ChoiceWidget>,
        hidden: &[String],
        state: &FxHashMap<String, String>,
        point: Vec2<f32>,
    ) -> ChoiceOutcome {
        let outcome = self.pointer_inner(widgets, hidden, state, point);
        self.pressed |= outcome.consumed;
        outcome
    }
    fn pointer_inner(
        &mut self,
        widgets: &mut FxHashMap<u32, ChoiceWidget>,
        hidden: &[String],
        state: &FxHashMap<String, String>,
        point: Vec2<f32>,
    ) -> ChoiceOutcome {
        self.prepare(widgets, hidden, state);
        if let Some(id) = self.open {
            let widget = widgets.get_mut(&id).unwrap();
            let interaction = widget.interact(point);
            let selection = if let ChoiceInteraction::Selected(value) = interaction {
                Some((widget.binding.clone(), value))
            } else {
                if let Some(original) = self.original {
                    widget.selected = original.min(widget.options.len().saturating_sub(1));
                }
                None
            };
            self.open = None;
            self.original = None;
            return ChoiceOutcome {
                consumed: true,
                selection,
            };
        }
        let mut ids: Vec<_> = widgets.keys().copied().collect();
        ids.sort_unstable_by(|a, b| b.cmp(a));
        for id in ids {
            let widget = widgets.get_mut(&id).unwrap();
            if widget_name_is_hidden(&widget.name, hidden)
                || !widget.contains_interactive_point(point)
            {
                continue;
            }
            let interaction = widget.interact(point);
            if widget.open {
                self.open = Some(id);
                self.original = Some(widget.selected);
            }
            let selection = if let ChoiceInteraction::Selected(value) = interaction {
                Some((widget.binding.clone(), value))
            } else {
                None
            };
            return ChoiceOutcome {
                consumed: true,
                selection,
            };
        }
        ChoiceOutcome::default()
    }
    pub fn navigate(
        &mut self,
        widgets: &mut FxHashMap<u32, ChoiceWidget>,
        hidden: &[String],
        state: &FxHashMap<String, String>,
        navigation: ChoiceNavigation,
    ) -> ChoiceOutcome {
        self.prepare(widgets, hidden, state);
        let Some(id) = self.open else {
            return ChoiceOutcome::default();
        };
        if navigation == ChoiceNavigation::Cancel {
            self.cancel(widgets);
            return ChoiceOutcome {
                consumed: true,
                selection: None,
            };
        }
        let widget = widgets.get_mut(&id).unwrap();
        let last = widget.options.len().saturating_sub(1);
        widget.selected = widget.selected.min(last);
        match navigation {
            ChoiceNavigation::Previous => widget.selected = widget.selected.saturating_sub(1),
            ChoiceNavigation::Next => widget.selected = (widget.selected + 1).min(last),
            ChoiceNavigation::First => widget.selected = 0,
            ChoiceNavigation::Last => widget.selected = last,
            ChoiceNavigation::Accept => {
                let selection = Some((widget.binding.clone(), widget.selected_value().to_owned()));
                widget.open = false;
                self.open = None;
                self.original = None;
                return ChoiceOutcome {
                    consumed: true,
                    selection,
                };
            }
            ChoiceNavigation::Cancel => unreachable!(),
        }
        ChoiceOutcome {
            consumed: true,
            selection: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rect;
    fn widget(id: u32, kind: ChoiceWidgetKind) -> ChoiceWidget {
        ChoiceWidget::from_ui(
            format!("Choice{id}"),
            id,
            Rect::new(20., 20., 160., 30.),
            kind,
            &"binding='class'\noptions=['Warrior','Mage','Rogue']\nequal_widths=true"
                .parse()
                .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn pending_navigation_and_outside_dismissal_never_commit_binding() {
        let mut widgets = FxHashMap::default();
        widgets.insert(1, widget(1, ChoiceWidgetKind::Dropdown));
        let mut state = FxHashMap::default();
        state.insert("class".into(), "warrior".into());
        let mut controller = ChoiceController::default();
        let open = controller.pointer(&mut widgets, &[], &state, Vec2::new(25., 25.));
        assert!(open.consumed && open.selection.is_none());
        assert_eq!(controller.open_id(), Some(1));
        assert!(
            controller
                .navigate(&mut widgets, &[], &state, ChoiceNavigation::Next)
                .selection
                .is_none()
        );
        controller.prepare(&mut widgets, &[], &state);
        assert_eq!(widgets[&1].selected, 1);
        assert_eq!(state["class"], "warrior");
        let dismiss = controller.pointer(&mut widgets, &[], &state, Vec2::new(500., 500.));
        assert!(dismiss.consumed && dismiss.selection.is_none());
        assert_eq!(widgets[&1].selected, 0);
        assert_eq!(controller.open_id(), None);
        assert!(
            !controller
                .pointer(&mut widgets, &[], &state, Vec2::new(500., 500.))
                .consumed
        );
        controller.pointer(&mut widgets, &[], &state, Vec2::new(25., 25.));
        controller.navigate(&mut widgets, &[], &state, ChoiceNavigation::Last);
        let accept = controller.navigate(&mut widgets, &[], &state, ChoiceNavigation::Accept);
        assert_eq!(accept.selection, Some(("class".into(), "rogue".into())));
        assert!(!widgets[&1].open);
    }
    #[test]
    fn topmost_capture_is_released_when_hidden_and_tabs_share_value_contract() {
        let mut widgets = FxHashMap::default();
        widgets.insert(1, widget(1, ChoiceWidgetKind::Dropdown));
        widgets.insert(2, widget(2, ChoiceWidgetKind::Dropdown));
        let state = FxHashMap::default();
        let mut controller = ChoiceController::default();
        controller.pointer(&mut widgets, &[], &state, Vec2::new(25., 25.));
        assert_eq!(controller.open_id(), Some(2));
        controller.navigate(&mut widgets, &[], &state, ChoiceNavigation::Next);
        assert!(controller.prepare(&mut widgets, &["Choice2".into()], &state));
        assert_eq!(widgets[&2].selected, 0);
        assert_eq!(controller.open_id(), None);
        assert!(!widgets[&2].open);
        widgets.insert(3, widget(3, ChoiceWidgetKind::TabBar));
        let rect = widgets[&3].tab_rect(1);
        let selected = controller.pointer(
            &mut widgets,
            &[],
            &state,
            Vec2::new(rect.x + 2., rect.y + 2.),
        );
        assert_eq!(selected.selection, Some(("class".into(), "mage".into())));
        assert_eq!(controller.open_id(), None);
    }
}
