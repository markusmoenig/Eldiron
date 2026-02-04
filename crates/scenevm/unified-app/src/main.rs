use scenevm::run_scenevm_app;
use unified_app::TemplateApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_scenevm_app(TemplateApp::new())?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() -> Result<(), wasm_bindgen::JsValue> {
    run_scenevm_app(TemplateApp::new())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    let _ = wasm_main();
}
