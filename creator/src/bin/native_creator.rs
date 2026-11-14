//! Compatibility alias using the canonical Creator GPU host.
#![windows_subsystem = "windows"]

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(error) = eldiron_creator::desktop::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn browser_start() {
    eldiron_creator::browser::start();
}
#[cfg(target_arch = "wasm32")]
fn main() {}
