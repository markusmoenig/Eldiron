use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TheKeyCode {
    Escape,
    Return,
    Delete,
    Home,
    End,
    Up,
    Right,
    Down,
    Left,
    Space,
    Tab,
}
