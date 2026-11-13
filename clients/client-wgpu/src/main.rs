#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    scenevm_unified_app::native_player::desktop::run(std::env::args_os())
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn browser_start() {
    scenevm_unified_app::native_player::browser::start();
}
#[cfg(target_arch = "wasm32")]
fn main() {}
