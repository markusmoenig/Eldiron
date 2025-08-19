#![windows_subsystem = "windows"]

use theframework::*;

pub mod misc;
pub mod solo;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

pub mod prelude {
    pub use crate::misc::*;
    pub use ::serde::{Deserialize, Serialize};
    pub use theframework::prelude::*;
}

use crate::solo::Solo;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once(); // shows panic messages in browser console
    main();
}

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "1");

    let solo = Solo::new();
    let app = TheApp::new();

    let () = app.run(Box::new(solo));
}
