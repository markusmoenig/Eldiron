pub use crate::prelude::*;

pub mod disc;
pub mod rect;

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct BrushSettings {
    pub size: f32,
    pub falloff: f32,
}

#[allow(unused)]
pub trait Brush: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> String;

    fn distance(&self, p: Vec2f, pos: Vec2f, settings: &BrushSettings) -> f32;

    fn preview(&self, buffer: &mut TheRGBABuffer);
}
