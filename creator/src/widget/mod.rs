
pub mod prelude {

}

pub trait Widget {
    fn new(
        name: String
    ) -> Self
    where
        Self: Sized;

    fn update(&mut self) {}
    fn resize(&mut self, width: usize, height: usize) {}


}