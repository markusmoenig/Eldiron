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
    pub use shared::prelude::*;
    pub use theframework::prelude::*;
}

use crate::solo::Solo;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let solo = Solo::new();
    let mut app = TheApp::new();

    let () = app.run(Box::new(solo));
}
