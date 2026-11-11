//! Native IME session identity, independent of a window implementation. Foreign
//! hosts use the same target tokens for callbacks that outlive a focus change.
use super::{InputEvent, TextInputTarget};
#[derive(Default)]
pub struct ImeSession {
    target: Option<TextInputTarget>,
    enabled_for: Option<TextInputTarget>,
    composing_for: Option<TextInputTarget>,
}
impl ImeSession {
    /// Return whether the native host must reset IME for a different editor.
    /// Preserve an old composition token until its commit/disabled boundary;
    /// dropping it here would redirect the old commit into the new field.
    pub fn set_target(&mut self, target: Option<TextInputTarget>) -> bool {
        let changed = self.target != target;
        self.target = target;
        changed
    }
    pub fn target(&self) -> Option<TextInputTarget> {
        self.target
    }
    pub fn is_enabled(&self) -> bool {
        self.target.is_some() && self.enabled_for == self.target
    }
    /// Native Enabled is the beginning of the new input session.
    pub fn enable(&mut self) {
        self.enabled_for = self.target;
        self.composing_for = None;
    }
    pub fn disable(&mut self) -> Option<InputEvent> {
        let target = self.composing_for.take().or(self.enabled_for.take());
        self.enabled_for = None;
        target.map(|target| InputEvent::CompositionFor {
            target,
            text: String::new(),
            cursor: None,
        })
    }
    pub fn preedit(&mut self, text: String, cursor: Option<(usize, usize)>) -> Option<InputEvent> {
        let target = self
            .composing_for
            .or_else(|| self.is_enabled().then_some(self.target).flatten())?;
        if !text.is_empty() {
            self.composing_for = Some(target);
        }
        Some(InputEvent::CompositionFor {
            target,
            text,
            cursor,
        })
    }
    pub fn commit(&mut self, text: String) -> Vec<InputEvent> {
        let target = self
            .composing_for
            .take()
            .or_else(|| self.is_enabled().then_some(self.target).flatten());
        let Some(target) = target else {
            return Vec::new();
        };
        let mut events = vec![InputEvent::CompositionFor {
            target,
            text: String::new(),
            cursor: None,
        }];
        if !text.is_empty() {
            events.push(InputEvent::TextFor { target, text });
        }
        events
    }
    /// A key that opens/focuses an editor is an activation, not text for the new
    /// editor. Hosts snapshot target before dispatching the physical key.
    pub fn key_text(&self, before: Option<TextInputTarget>, text: String) -> Option<InputEvent> {
        if self.is_enabled() || before != self.target {
            return None;
        }
        let target = before?;
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        (!text.is_empty()).then_some(InputEvent::TextFor { target, text })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focus_change_keeps_old_commit_addressed_until_new_native_session() {
        let a = TextInputTarget::new();
        let b = TextInputTarget::new();
        assert_ne!(a, b);
        assert_eq!(TextInputTarget::from_raw(a.raw()), Some(a));
        assert_eq!(TextInputTarget::from_raw(0), None);
        let mut ime = ImeSession::default();
        assert!(ime.set_target(Some(a)));
        ime.enable();
        assert!(
            matches!(ime.preedit("仮".into(),Some((0,3))),Some(InputEvent::CompositionFor{target,..}) if target==a)
        );
        assert!(ime.set_target(Some(b)));
        assert!(
            ime.commit("旧".into())
                .iter()
                .all(|event| event.text_target() == Some(a))
        );
        assert!(!ime.is_enabled());
        // A callback queued before focus transfer cannot become text for B.
        assert!(ime.preedit("old".into(), None).is_none());
        ime.disable();
        ime.enable();
        assert!(
            matches!(ime.preedit("新".into(),None),Some(InputEvent::CompositionFor{target,..}) if target==b)
        );
        assert!(
            ime.commit("新".into())
                .iter()
                .all(|event| event.text_target() == Some(b))
        );
        ime.set_target(None);
        assert!(matches!(ime.disable(),Some(InputEvent::CompositionFor{target,..}) if target==b));
        assert!(ime.commit("late".into()).is_empty());
    }
    #[test]
    fn key_activation_and_geometry_changes_do_not_insert_into_another_editor() {
        let a = TextInputTarget::new();
        let b = TextInputTarget::new();
        let mut ime = ImeSession::default();
        let before = ime.target();
        ime.set_target(Some(a));
        assert!(ime.key_text(before, "/".into()).is_none());
        assert!(
            !ime.set_target(Some(a)),
            "a caret/geometry update retains the input session"
        );
        assert_eq!(
            ime.key_text(Some(a), "é\n".into()),
            Some(InputEvent::TextFor {
                target: a,
                text: "é".into()
            })
        );
        ime.set_target(Some(b));
        assert!(ime.key_text(Some(a), "x".into()).is_none());
        ime.enable();
        assert!(ime.key_text(Some(b), "duplicate".into()).is_none());
    }
}
