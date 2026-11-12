#![windows_subsystem = "windows"]

use theframework::*;

pub mod client;
pub mod misc;

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

use crate::client::Client;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once(); // shows panic messages in browser console
    main();
}

fn main() {
    // unsafe {
    //     std::env::set_var("RUST_BACKTRACE", "1");
    // }

    let args: Vec<_> = std::env::args().collect();

    let mut client = Client::new();
    client.set_cmd_line_args_early(args.clone());
    let app = TheApp::new();

    let () = app.run(Box::new(client));
}
