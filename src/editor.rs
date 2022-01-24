
/// Which window do we show currently
enum EditorState {
    TileSet,
    Tiles,
}

/// The Editor struct
pub struct Editor {
    state                  : EditorState,
}

impl Editor  {
    
    pub fn new() -> Self {
        Self {
            state           : EditorState::TileSet,
        }
    }

    /// Update the editor
    pub fn update(&mut self) {
    }

    pub fn draw(&self, frame: &mut [u8]) {

    }
}