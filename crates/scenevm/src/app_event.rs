/// Events emitted by the app to the host/wrapper
/// The host handles these platform-specifically (file dialogs, menus, etc.)
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Request to save current project
    /// Host should: Show save dialog (if needed), call save_to_json(), write file
    /// filename: default filename without extension (e.g., "myproject")
    /// extension: file extension (e.g., "json", "denrim")
    RequestSave { filename: String, extension: String },

    /// Request to save with new name/location
    /// Host should: Show "Save As" dialog, call save_to_json(), write file
    /// filename: default filename without extension (e.g., "myproject")
    /// extension: file extension (e.g., "json", "denrim")
    RequestSaveAs { filename: String, extension: String },

    /// Request to open a project
    /// Host should: Show open dialog, read file, call load_from_json()
    /// extension: file extension to filter (e.g., "json", "denrim")
    RequestOpen { extension: String },

    /// Request to create new project
    /// Host should: Prompt to save current (if dirty), call new_project()
    RequestNew,

    /// Request to close current project
    /// Host should: Prompt to save (if dirty), close window/document
    RequestClose,

    /// Request to export project in different format
    /// Host should: Show export dialog with format options
    RequestExport { format: String, filename: String },

    /// Request to import file(s) into current project
    /// Host should: Show import dialog for specified file types, read files, pass data to app
    RequestImport { file_types: Vec<String> },

    /// Request to show project browser/gallery
    /// Host should: Show browser UI or switch to browser mode
    RequestShowBrowser,

    /// App state changed (for dirty tracking)
    /// Host should: Update window title, enable save button, etc.
    StateChanged { has_unsaved_changes: bool },

    /// Request to perform undo operation
    /// Host should: Call app's undo() method
    RequestUndo,

    /// Request to perform redo operation
    /// Host should: Call app's redo() method
    RequestRedo,
}

/// Queue for app events that will be consumed by the host
#[derive(Default)]
pub struct AppEventQueue {
    events: Vec<AppEvent>,
}

impl AppEventQueue {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Emit an event to the host
    pub fn emit(&mut self, event: AppEvent) {
        self.events.push(event);
    }

    /// Take all pending events (consumes them)
    pub fn take(&mut self) -> Vec<AppEvent> {
        std::mem::take(&mut self.events)
    }

    /// Check if there are pending events
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// Peek at events without consuming
    pub fn peek(&self) -> &[AppEvent] {
        &self.events
    }
}
