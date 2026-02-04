use theframework::*;

pub mod scenegen;
use crate::scenegen::Circle;

fn main() {
    let circle = Circle::new();
    let app = TheApp::new();

    () = app.run(Box::new(circle));
}
