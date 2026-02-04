/// Pointer/mouse/touch events in logical coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiEventKind {
    PointerDown,
    PointerUp,
    PointerMove,
    Scroll { delta: [f32; 2] }, // [dx, dy] in logical units
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiEvent {
    pub kind: UiEventKind,
    pub pos: [f32; 2],
    /// Identifier for the pointer/gesture (0 for mouse primary).
    pub pointer_id: u32,
}

impl UiEvent {
    pub fn new(kind: UiEventKind, pos: [f32; 2]) -> Self {
        Self {
            kind,
            pos,
            pointer_id: 0,
        }
    }
}

/// Actions emitted by UI elements.
#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    ButtonPressed(String),
    ButtonToggled(String, bool),
    /// Slider value changed: (id, current_value, original_value, is_final)
    /// - `current_value`: The current slider value (use for preview/immediate updates)
    /// - `original_value`: The value when drag started (use for undo)
    /// - `is_final`: true when mouse released (ready for undo command)
    SliderChanged(String, f32, f32, bool),
    ButtonGroupChanged(String, usize), // (group_name, active_index)
    DropdownChanged(String, usize),    // (dropdown_name, selected_index)
    /// Color changed: (id, current_color, original_color, is_final)
    /// - `current_color`: The current RGBA color (use for preview/immediate updates)
    /// - `original_color`: The color when drag started (use for undo)
    /// - `is_final`: true when mouse released (ready for undo command)
    ColorChanged(String, [f32; 4], [f32; 4], bool),
    Custom {
        source_id: String,
        action: String,
    }, // For custom widgets like ProjectBrowser
}

/// Result of handling an event: whether a view dirtied itself and any actions.
#[derive(Debug, Default)]
pub struct UiEventOutcome {
    pub dirty: bool,
    pub actions: Vec<UiAction>,
}

impl UiEventOutcome {
    pub fn none() -> Self {
        Self {
            dirty: false,
            actions: Vec::new(),
        }
    }

    pub fn dirty() -> Self {
        Self {
            dirty: true,
            actions: Vec::new(),
        }
    }

    pub fn with_action(action: UiAction) -> Self {
        Self {
            dirty: true,
            actions: vec![action],
        }
    }

    pub fn action(action: UiAction) -> Self {
        Self::with_action(action)
    }

    pub fn redraw() -> Self {
        Self::dirty()
    }

    pub fn merge(&mut self, other: UiEventOutcome) {
        self.dirty |= other.dirty;
        self.actions.extend(other.actions);
    }
}
