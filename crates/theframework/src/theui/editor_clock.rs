//! Caret animation belongs to the active painted editor, not scene cadence.
use crate::prelude::Uuid;
use web_time::{Duration, Instant};

const BLINK: Duration = Duration::from_millis(500);

struct Owner {
    widget: Uuid,
    focus: Uuid,
    due: Instant,
    visible: bool,
    composing: bool,
}

#[derive(Default)]
pub(super) struct EditorClock {
    owner: Option<Owner>,
    seen: bool,
}

impl EditorClock {
    pub fn begin_layout(&mut self) {
        self.seen = false;
    }

    /// Embedded fields can belong to a focused composite. Track both identities
    /// so focus changes retire the old field even before the next layout pass.
    pub fn register(&mut self, widget: Uuid, focus: Option<Uuid>, active: bool, composing: bool) {
        let Some(focus) = focus.filter(|_| active) else {
            if self
                .owner
                .as_ref()
                .is_some_and(|owner| owner.widget == widget)
            {
                self.owner = None;
            }
            return;
        };
        self.seen = true;
        let now = Instant::now();
        if let Some(owner) = &mut self.owner
            && owner.widget == widget
            && owner.focus == focus
        {
            if owner.composing != composing {
                owner.visible = true;
                owner.due = now + BLINK;
                owner.composing = composing;
            }
        } else {
            self.owner = Some(Owner {
                widget,
                focus,
                due: now + BLINK,
                visible: true,
                composing,
            });
        }
    }

    pub fn finish_layout(&mut self) {
        if !self.seen {
            self.owner = None;
        }
    }

    pub fn retain_focus(&mut self, focus: Option<Uuid>) {
        if self
            .owner
            .as_ref()
            .is_some_and(|owner| Some(owner.focus) != focus)
        {
            self.owner = None;
        }
    }

    pub fn clear(&mut self) {
        self.owner = None;
    }

    pub fn visible(&self, widget: Uuid) -> bool {
        self.owner
            .as_ref()
            .is_some_and(|owner| owner.widget == widget && owner.visible)
    }

    pub fn restart(&mut self) -> bool {
        let Some(owner) = &mut self.owner else {
            return false;
        };
        let changed = !owner.visible;
        owner.visible = true;
        owner.due = Instant::now() + BLINK;
        changed
    }

    pub fn next_wake_after(&self) -> Option<Duration> {
        self.owner
            .as_ref()
            .filter(|owner| !owner.composing)
            .map(|owner| owner.due.saturating_duration_since(Instant::now()))
    }

    pub fn advance(&mut self) -> bool {
        let now = Instant::now();
        if let Some(owner) = &mut self.owner
            && !owner.composing
            && now >= owner.due
        {
            owner.visible = !owner.visible;
            // A delayed host advances once; it never spins to replay old blinks.
            owner.due = now + BLINK;
            return true;
        }
        false
    }
}
