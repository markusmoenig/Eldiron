use theframework::*;

pub mod analytical;
pub mod demo;
pub mod project;
pub mod renderer;
pub mod sidebar;

use crate::demo::UIDemo;

pub mod prelude {
    pub use crate::analytical::*;
    pub use crate::project::*;
    pub use crate::renderer::*;
    pub use crate::sidebar::*;
    pub use theframework::prelude::*;
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_var("RUST_BACKTRACE", "1");

    let demo = UIDemo::new();
    let app = TheApp::new();
    () = app.run(Box::new(demo));
}
