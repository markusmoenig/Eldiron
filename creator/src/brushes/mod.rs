pub use crate::prelude::*;

pub mod disc;
pub mod rect;

#[allow(unused)]
pub trait Brush: Send {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> String;

    fn preview(&self, buffer: &mut TheRGBABuffer);
}
