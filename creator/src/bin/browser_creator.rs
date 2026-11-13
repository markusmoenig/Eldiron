#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn browser_start() {
    eldiron_creator::browser::start();
}
#[cfg(target_arch = "wasm32")]
fn main() {}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("Build eldiron-creator-web for wasm32-unknown-unknown.");
}
