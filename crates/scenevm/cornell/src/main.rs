use theframework::*;

pub mod cornell;
use crate::cornell::CornellBox;

fn main() {
    let circle = CornellBox::new();
    let app = TheApp::new();

    () = app.run(Box::new(circle));
}
