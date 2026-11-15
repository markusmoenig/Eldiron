//! Remote examples retain the same decoded project/controller flow on both
//! hosts. Browser downloads yield to Web APIs instead of spawning OS threads.
use super::*;
impl ExampleProjectManifestEntry {
    pub(super) fn with_preview(self, preview: Option<TheRGBATile>) -> ExampleProjectEntry {
        ExampleProjectEntry {
            id: Uuid::new_v4(),
            manifest_id: self.id,
            title: self.title,
            dimension: self.dimension,
            description: self.description,
            project_path: self.project_path,
            preview,
        }
    }
}
impl Editor {
    pub(super) fn example_manifest_entries(contents: &str) -> Vec<ExampleProjectManifestEntry> {
        toml::from_str::<ExampleProjectManifest>(contents)
            .map(|manifest| {
                manifest
                    .example
                    .into_iter()
                    .filter(|entry| !matches!(entry.id.as_str(), "starter-3d" | "cellar"))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn start_example_manifest(
        tx: theframework::platform::WakeSender<Vec<ExampleProjectEntry>>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let _ = tx.send(Self::load_example_manifest());
        });
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let result = async {
                let bytes =
                    crate::host_services::fetch_bytes(&Self::example_manifest_url(), 1_048_576)
                        .await
                        .ok()?;
                let text = String::from_utf8(bytes).ok()?;
                let mut result = Vec::new();
                for entry in Self::example_manifest_entries(&text) {
                    let preview = if let Some(path) = &entry.preview {
                        crate::host_services::fetch_bytes(
                            &Self::example_repo_url(path),
                            64 * 1024 * 1024,
                        )
                        .await
                        .ok()
                        .and_then(Self::decode_png_tile)
                    } else {
                        None
                    };
                    result.push(entry.with_preview(preview));
                }
                Some(result)
            }
            .await;
            let _ = tx.send(result.unwrap_or_default());
        });
    }
    pub(super) fn start_example_project(
        path: String,
        manifest: String,
        tx: theframework::platform::WakeSender<(String, Option<Project>)>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let _ = tx.send((manifest, Self::load_example_project(&path)));
        });
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let project = crate::host_services::fetch_bytes(
                &Self::example_repo_url(&path),
                1024 * 1024 * 1024,
            )
            .await
            .ok()
            .and_then(|bytes| Self::load_project_from_bytes(&bytes).ok());
            let _ = tx.send((manifest, project));
        });
    }
}
