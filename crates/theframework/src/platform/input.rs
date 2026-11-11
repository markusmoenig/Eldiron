/// Addressed asynchronous clipboard completion. A failed write must not delete
/// a cut selection; unavailable reads never replace the editor contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardResult {
    Written(bool),
    Text(Option<String>),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}
/// Physical identity for controls; independent of composed text and keyboard layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalKey {
    Letter(u8),
    Digit(u8),
    Function(u8),
    Escape,
    Enter,
    Space,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
    Other(u32),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalKey {
    Character(String),
    Named(crate::TheKeyCode),
    Other,
}
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    PointerMoved {
        position: [f32; 2],
    },
    PointerButton {
        button: PointerButton,
        pressed: bool,
        position: [f32; 2],
    },
    PointerLeft,
    RelativeMotion {
        delta: [f32; 2],
    },
    /// Positive wheel movement uses line units when `precise` is false,
    /// logical pixel units when true. Applications choose their line height.
    Scroll {
        delta: [f32; 2],
        precise: bool,
    },
    Pinch(f32),
    Key {
        physical: PhysicalKey,
        logical: LogicalKey,
        pressed: bool,
        repeat: bool,
    },
    /// Committed text is a separate operation; physical key presses never insert it.
    Text(String),
    Composition {
        text: String,
        cursor: Option<(usize, usize)>,
    },
    Modifiers(Modifiers),
    Focus(bool),
    FileDropped(std::path::PathBuf),
    Clipboard {
        request: u64,
        result: ClipboardResult,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputResponse {
    pub consumed: bool,
    pub redraw: bool,
}
