use std::path::PathBuf;

/// Platform-specific file dialog utilities
/// On macOS/iOS with Xcode wrapper, these are handled by Swift layer
/// On Windows/Linux, these use rfd (rusty file dialog)
/// On WASM, stub implementations (need web APIs)

pub struct FileDialog;

// Desktop platforms (not WASM, not iOS) - use rfd
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "ios")))]
impl FileDialog {
    /// Show open file dialog
    /// Returns None if user cancelled
    pub fn open(title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title(title)
            .add_filter(filters[0].0, filters[0].1)
            .pick_file()
    }

    /// Show save file dialog
    /// Returns None if user cancelled
    pub fn save(title: &str, default_name: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title(title)
            .set_file_name(default_name)
            .add_filter(filters[0].0, filters[0].1)
            .save_file()
    }

    /// Show open directory dialog
    pub fn open_directory(title: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(title).pick_folder()
    }
}

// iOS - stub implementations (should be handled by Swift layer)
#[cfg(target_os = "ios")]
impl FileDialog {
    /// On iOS, file dialogs should be handled by the Swift wrapper layer
    /// These are stub implementations
    pub fn open(_title: &str, _filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        None
    }

    pub fn save(
        _title: &str,
        _default_name: &str,
        _filters: &[(&str, &[&str])],
    ) -> Option<PathBuf> {
        None
    }

    pub fn open_directory(_title: &str) -> Option<PathBuf> {
        None
    }
}

// WASM - stub implementations (need web APIs)
#[cfg(target_arch = "wasm32")]
impl FileDialog {
    /// On WASM, file dialogs need to be handled through web APIs
    /// For now, these are stub implementations
    pub fn open(_title: &str, _filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        None
    }

    pub fn save(
        _title: &str,
        _default_name: &str,
        _filters: &[(&str, &[&str])],
    ) -> Option<PathBuf> {
        None
    }

    pub fn open_directory(_title: &str) -> Option<PathBuf> {
        None
    }
}
