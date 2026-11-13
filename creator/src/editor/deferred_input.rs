//! Key dispatch retains the event's modifier snapshot, while current host state
//! remains authoritative outside that one deferred application event.
use super::*;
use theframework::platform::Modifiers;

#[derive(Clone, Copy)]
pub(super) struct Snapshot {
    widgets: Modifiers,
    #[cfg(feature = "native-ui")]
    text: Modifiers,
}
impl Snapshot {
    pub(super) fn capture(ui: &TheUI, _ctx: &TheContext) -> Self {
        Self {
            widgets: Modifiers {
                shift: ui.shift,
                control: ui.ctrl,
                alt: ui.alt,
                super_key: ui.logo,
            },
            #[cfg(feature = "native-ui")]
            text: _ctx.ui.input_modifiers,
        }
    }
    pub(super) fn restore(self, ui: &mut TheUI, ctx: &mut TheContext) {
        apply(ui, ctx, self.widgets);
        // Direct legacy UI callers can set widget flags independently. Restore
        // their exact text state too, rather than silently synchronizing it.
        #[cfg(feature = "native-ui")]
        {
            ctx.ui.input_modifiers = self.text;
        }
    }
}
fn apply(ui: &mut TheUI, _ctx: &mut TheContext, modifiers: Modifiers) {
    ui.shift = modifiers.shift;
    ui.ctrl = modifiers.control;
    ui.alt = modifiers.alt;
    ui.logo = modifiers.super_key;
    #[cfg(feature = "native-ui")]
    {
        _ctx.ui.input_modifiers = modifiers;
    }
}

pub(super) fn decode(event: TheEvent, ui: &mut TheUI, ctx: &mut TheContext) -> TheEvent {
    match event {
        TheEvent::InputKey { event, modifiers } => {
            apply(ui, ctx, modifiers);
            *event
        }
        event => event,
    }
}
