//! Native file dialogs for desktop platforms
//! Handles export, save, open, and import dialogs using rfd

#[cfg(all(feature = "ui", not(target_arch = "wasm32"), not(target_os = "ios")))]
use crate::{SceneVM, SceneVMApp};

/// Handle export requests with native file dialog
#[cfg(all(feature = "ui", not(target_arch = "wasm32"), not(target_os = "ios")))]
pub fn handle_export<T: SceneVMApp>(app: &mut T, vm: &mut SceneVM, format: &str, filename: &str) {
    use rfd::FileDialog;

    let extension = format.to_lowercase();
    let filter_name = match extension.as_str() {
        "png" => "PNG Image",
        "jpg" | "jpeg" => "JPEG Image",
        "json" => "JSON File",
        _ => "File",
    };

    let default_name = if filename.is_empty() {
        format!("export.{}", extension)
    } else {
        format!("{}.{}", filename, extension)
    };

    if let Some(path) = FileDialog::new()
        .add_filter(filter_name, &[&extension])
        .set_file_name(&default_name)
        .save_file()
    {
        // Try app-specific export first
        if let Some(data) = app.export_data(vm, &extension) {
            if let Err(e) = std::fs::write(&path, data) {
                eprintln!("Failed to write export file: {}", e);
            } else {
            }
        } else {
            eprintln!("Export format '{}' not supported by this app", extension);
        }
    }
}

/// Handle save requests with native file dialog
#[cfg(all(feature = "ui", not(target_arch = "wasm32"), not(target_os = "ios")))]
pub fn handle_save<T: SceneVMApp>(app: &mut T, vm: &mut SceneVM, filename: &str, extension: &str) {
    use rfd::FileDialog;

    let default_name = if filename.is_empty() {
        format!("project.{}", extension)
    } else {
        format!("{}.{}", filename, extension)
    };

    let filter_name = "Project File";

    if let Some(path) = FileDialog::new()
        .add_filter(filter_name, &[extension])
        .set_file_name(&default_name)
        .save_file()
    {
        if let Some(json) = app.save_to_json(vm) {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("Failed to save project: {}", e);
            } else {
            }
        } else {
            eprintln!("Save not implemented for this app");
        }
    }
}

/// Handle open requests with native file dialog
#[cfg(all(feature = "ui", not(target_arch = "wasm32"), not(target_os = "ios")))]
pub fn handle_open<T: SceneVMApp>(app: &mut T, vm: &mut SceneVM, extension: &str) {
    use rfd::FileDialog;

    if let Some(path) = FileDialog::new()
        .add_filter("Project File", &[extension])
        .pick_file()
    {
        match std::fs::read_to_string(&path) {
            Ok(json) => {
                if app.load_from_json(vm, &json) {
                } else {
                    eprintln!("Failed to load project");
                }
            }
            Err(e) => {
                eprintln!("Failed to read file: {}", e);
            }
        }
    }
}

/// Handle import requests with native file dialog
#[cfg(all(feature = "ui", not(target_arch = "wasm32"), not(target_os = "ios")))]
pub fn handle_import<T: SceneVMApp>(_app: &mut T, _vm: &mut SceneVM, file_types: &[String]) {
    use rfd::FileDialog;

    let mut dialog = FileDialog::new();

    if !file_types.is_empty() {
        let extensions: Vec<&str> = file_types.iter().map(|s| s.as_str()).collect();
        dialog = dialog.add_filter("Files", &extensions);
    }

    if let Some(_path) = dialog.pick_file() {

        // Apps can implement custom import logic
    }
}
