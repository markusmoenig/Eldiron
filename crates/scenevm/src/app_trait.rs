use crate::{RenderResult, SceneVM, SceneVMResult};

/// Minimal app abstraction to write one SceneVM app for native + wasm.
pub trait SceneVMApp {
    /// Optional preferred initial window size on native (physical pixels). If `None`, a platform default is used.
    fn initial_window_size(&self) -> Option<(u32, u32)> {
        None
    }
    /// Optional window title on native. If `None`, `"SceneVM"` is used.
    fn window_title(&self) -> Option<String> {
        None
    }
    /// Optional target frame rate on native (FPS). If `None`, the runner will poll/redraw as fast as possible.
    fn target_fps(&self) -> Option<f32> {
        None
    }

    /// Called by the runner before init() to inform the app of its runtime mode
    /// `is_native`: true = native wgpu runner (needs file buttons)
    ///              false = platform wrapper (Xcode, uses document system)
    fn set_native_mode(&mut self, _is_native: bool) {
        // Override this to store the mode and conditionally show UI elements
    }
    /// Get the current system theme preference (true = dark, false = light)
    /// Called before init() to determine initial theme
    #[cfg(feature = "ui")]
    fn is_dark_mode(&self) -> bool {
        true // Default to dark mode
    }
    /// Called once after the renderer is created and sized.
    fn init(&mut self, _vm: &mut SceneVM, _size: (u32, u32)) {}
    /// Per-frame update hook (e.g. animation).
    fn update(&mut self, _vm: &mut SceneVM) {}
    /// Render hook: call `ctx.present(vm)` to display.
    fn render(&mut self, vm: &mut SceneVM, ctx: &mut dyn SceneVMRenderCtx);
    /// Collect app events to be handled by the host (file operations, etc.)
    /// The host calls this after render to get events like RequestSave, RequestOpen, etc.
    #[cfg(feature = "ui")]
    fn take_app_events(&mut self) -> Vec<crate::app_event::AppEvent> {
        Vec::new()
    }
    /// Return `true` if the app wants an update/render this tick. Default is always true.
    fn needs_update(&mut self, _vm: &SceneVM) -> bool {
        true
    }
    /// Resize callback with new logical size.
    fn resize(&mut self, _vm: &mut SceneVM, _size: (u32, u32)) {}
    /// Set the scale factor (device pixel ratio) for HiDPI displays.
    fn set_scale(&mut self, _scale: f32) {}
    /// Mouse/touch down callback in logical pixels.
    fn mouse_down(&mut self, _vm: &mut SceneVM, _x: f32, _y: f32) {}
    /// Mouse/touch up callback in logical pixels.
    fn mouse_up(&mut self, _vm: &mut SceneVM, _x: f32, _y: f32) {}
    /// Mouse/touch move callback in logical pixels.
    fn mouse_move(&mut self, _vm: &mut SceneVM, _x: f32, _y: f32) {}
    /// Scroll/pan delta (e.g. trackpad or wheel) in logical units.
    fn scroll(&mut self, _vm: &mut SceneVM, _dx: f32, _dy: f32) {}
    /// Pinch gesture: scale factor and center in logical coordinates.
    fn pinch(&mut self, _vm: &mut SceneVM, _scale: f32, _center: (f32, f32)) {}

    // Project Management (optional, only available with 'project' feature)
    // File paths are handled by the wrapper layer (Swift/native runners)
    // These methods only deal with JSON serialization/deserialization

    /// Serialize current app state to JSON string
    /// Called during File->Save or auto-save operations
    /// Returns JSON string representing the current state
    #[cfg(feature = "ui")]
    fn save_to_json(&mut self, _vm: &mut SceneVM) -> Option<String> {
        None
    }

    /// Deserialize and apply JSON state to app
    /// Called during File->Open or app launch with document
    /// Return true if load was successful
    #[cfg(feature = "ui")]
    fn load_from_json(&mut self, _vm: &mut SceneVM, _json: &str) -> bool {
        false
    }

    /// Create a new/empty project state
    /// Called when user selects File->New
    /// Should reset app to default state
    #[cfg(feature = "ui")]
    fn new_project(&mut self, _vm: &mut SceneVM) {
        // Default: do nothing
    }

    /// Check if project has unsaved changes
    /// Used to prompt user before closing or creating new project
    #[cfg(feature = "ui")]
    fn has_unsaved_changes(&self) -> bool {
        false
    }

    /// Generate a thumbnail for the current project
    /// Returns RGBA pixel data (width, height, pixels)
    /// Thumbnail should be small (e.g., 256x256) for efficient storage
    #[cfg(feature = "ui")]
    fn generate_thumbnail(&mut self, _vm: &mut SceneVM) -> Option<(u32, u32, Vec<u8>)> {
        None
    }

    /// Export data in the specified format
    /// Called when user requests export (e.g., PNG, JPEG)
    /// Returns exported data as bytes, or None if format not supported
    #[cfg(feature = "ui")]
    fn export_data(&mut self, _vm: &mut SceneVM, _format: &str) -> Option<Vec<u8>> {
        None
    }

    /// Switch between light and dark themes
    /// Called when the system appearance changes (e.g., iOS dark mode toggle)
    /// or when user manually switches theme
    /// `is_dark` is true for dark mode, false for light mode
    #[cfg(feature = "ui")]
    fn set_theme(&mut self, _vm: &mut SceneVM, _is_dark: bool, _size: (u32, u32)) {
        // Default: do nothing
        // Apps should override this to rebuild their UI with the new theme
    }

    // Undo/Redo Support (optional, only available with 'ui' feature)

    /// Perform undo operation
    /// Called when user triggers undo (Cmd+Z on macOS, Ctrl+Z on Windows/Linux)
    /// Returns true if undo was performed, false if nothing to undo
    #[cfg(feature = "ui")]
    fn undo(&mut self, _vm: &mut SceneVM) -> bool {
        false
    }

    /// Perform redo operation
    /// Called when user triggers redo (Cmd+Shift+Z on macOS, Ctrl+Y on Windows/Linux)
    /// Returns true if redo was performed, false if nothing to redo
    #[cfg(feature = "ui")]
    fn redo(&mut self, _vm: &mut SceneVM) -> bool {
        false
    }

    /// Check if undo is available
    /// Used by wrappers to enable/disable undo menu items
    #[cfg(feature = "ui")]
    fn can_undo(&self) -> bool {
        false
    }

    /// Check if redo is available
    /// Used by wrappers to enable/disable redo menu items
    #[cfg(feature = "ui")]
    fn can_redo(&self) -> bool {
        false
    }

    /// Get description of next undo action (e.g., "Undo Change Slider")
    /// Used for menu item display
    #[cfg(feature = "ui")]
    fn undo_description(&self) -> Option<String> {
        None
    }

    /// Get description of next redo action (e.g., "Redo Change Slider")
    /// Used for menu item display
    #[cfg(feature = "ui")]
    fn redo_description(&self) -> Option<String> {
        None
    }
}

/// Rendering context supplied to `SceneVMApp::render`.
pub trait SceneVMRenderCtx {
    fn size(&self) -> (u32, u32);
    fn present(&mut self, vm: &mut SceneVM) -> SceneVMResult<RenderResult>;
}
