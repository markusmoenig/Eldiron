//! CPU alignment policy shared by reference and native layout adapters.
#[derive(PartialEq, Clone, Eq)]
pub enum TheHorizontalAlign {
    Left,
    Center,
    Right,
}

#[derive(PartialEq)]
pub enum TheVerticalAlign {
    Top,
    Center,
    Bottom,
}
