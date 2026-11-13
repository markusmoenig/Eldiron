use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_TEXT_TARGET: AtomicU64 = AtomicU64::new(1);
static NEXT_CLIPBOARD_REQUEST: AtomicU64 = AtomicU64::new(1);
/// One request domain for text editors and application-owned diagram commands.
/// Host clipboard completions retain the existing opaque u64 wire contract.
pub fn new_clipboard_request() -> u64 {
    NEXT_CLIPBOARD_REQUEST.fetch_add(1, Ordering::Relaxed)
}
/// Process-unique editor identity. It survives layout/device replacement and
/// changes on focus/document replacement or controller recreation, including
/// when the authored widget ID is reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextInputTarget(u64);
impl TextInputTarget {
    pub fn new() -> Self {
        Self(NEXT_TEXT_TARGET.fetch_add(1, Ordering::Relaxed))
    }
    pub fn raw(self) -> u64 {
        self.0
    }
    /// Round-trip an identity supplied to a native/foreign host by Rust.
    pub fn from_raw(raw: u64) -> Option<Self> {
        (raw != 0).then_some(Self(raw))
    }
}
impl Default for TextInputTarget {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(feature = "paint")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputState {
    pub target: TextInputTarget,
    pub area: crate::paint::Rect,
}

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
/// Native IME/accessibility operations address an immutable editor source
/// snapshot. Byte ranges use UTF-8 and must lie on character boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeTextAction {
    Select {
        anchor: usize,
        focus: usize,
    },
    /// A typed/IME commit retains typing policy and its captured selection.
    Insert {
        selection: (usize, usize),
        text: String,
    },
    DeleteBackward {
        selection: (usize, usize),
    },
    Replace {
        range: std::ops::Range<usize>,
        text: String,
    },
    Compose {
        range: Option<std::ops::Range<usize>>,
        text: String,
        cursor: Option<(usize, usize)>,
    },
    CommitComposition,
    CancelComposition,
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
    /// Relative device motion in host units. Unlike absolute pointer positions
    /// and precise scrolling, camera deltas are independent of UI/window scale.
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
    /// Native asynchronous callbacks carry the editor that began the operation.
    TextFor {
        target: TextInputTarget,
        text: String,
    },
    CompositionFor {
        target: TextInputTarget,
        text: String,
        cursor: Option<(usize, usize)>,
    },
    Composition {
        text: String,
        cursor: Option<(usize, usize)>,
    },
    NativeText {
        target: TextInputTarget,
        revision: u64,
        action: NativeTextAction,
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

impl InputEvent {
    pub fn text_target(&self) -> Option<TextInputTarget> {
        match self {
            Self::TextFor { target, .. }
            | Self::CompositionFor { target, .. }
            | Self::NativeText { target, .. } => Some(*target),
            _ => None,
        }
    }
}

impl PhysicalKey {
    /// Stable native-host wire identity, independent of composed text/layout.
    pub fn from_native_code(code: u32) -> Self {
        match code {
            65..=90 => Self::Letter(code as u8 + 32),
            97..=122 => Self::Letter(code as u8),
            48..=57 => Self::Digit(code as u8 - b'0'),
            0x100 => Self::Enter,
            0x101 => Self::Escape,
            0x102 => Self::Backspace,
            0x103 => Self::Tab,
            0x104 => Self::Space,
            0x105 => Self::Delete,
            0x106 => Self::Left,
            0x107 => Self::Right,
            0x108 => Self::Up,
            0x109 => Self::Down,
            0x10a => Self::Home,
            0x10b => Self::End,
            0x10c => Self::PageUp,
            0x10d => Self::PageDown,
            0x110 => Self::ShiftLeft,
            0x111 => Self::ShiftRight,
            0x112 => Self::ControlLeft,
            0x113 => Self::ControlRight,
            0x114 => Self::AltLeft,
            0x115 => Self::AltRight,
            0x116 => Self::SuperLeft,
            0x117 => Self::SuperRight,
            0x201..=0x218 => Self::Function((code - 0x200) as u8),
            _ => Self::Other(code),
        }
    }
}
