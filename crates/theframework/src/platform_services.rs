//! OS services shared by the legacy and GPU desktop hosts. Widget/UI state
//! queues neutral requests; an Xcode host can drain them without these adapters.
#[cfg(all(
    feature = "desktop-dialogs",
    any(target_os = "macos", target_os = "windows", target_os = "linux")
))]
pub fn service_files(ui: &mut crate::theui::theuicontext::TheUIContext) {
    use crate::theui::theuicontext::FileRequestKind;
    if ui.file_requester_receiver.is_some() {
        return;
    }
    let Some(request) = ui.file_requests.pop_front() else {
        return;
    };
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter(request.extension.name, &request.extension.extensions)
        .set_title(request.title);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let paths = futures::executor::block_on(async move {
            match request.kind {
                FileRequestKind::Open => dialog
                    .pick_files()
                    .await
                    .unwrap_or_default()
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .collect::<Vec<_>>(),
                FileRequestKind::Save => dialog
                    .save_file()
                    .await
                    .into_iter()
                    .map(|f| f.path().to_path_buf())
                    .collect(),
            }
        });
        let _ = tx.send(paths);
    });
    ui.file_requester_receiver = Some((request.id, rx));
}
#[cfg(not(all(
    feature = "desktop-dialogs",
    any(target_os = "macos", target_os = "windows", target_os = "linux")
)))]
pub fn service_files(_ui: &mut crate::theui::theuicontext::TheUIContext) {}
