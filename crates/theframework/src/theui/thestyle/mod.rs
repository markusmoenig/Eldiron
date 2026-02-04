use crate::prelude::*;

pub mod classic;

pub mod prelude {
    pub use crate::theui::thestyle::classic::TheClassicStyle;
}

#[allow(unused)]
pub trait TheStyle: Send {
    fn new() -> Self
    where
        Self: Sized;

    #[allow(clippy::borrowed_box)]
    /// Returns the current theme of the style
    fn theme(&mut self) -> &mut Box<dyn TheTheme>;

    /// Draw the widget border
    fn draw_widget_border(
        &mut self,
        buffer: &mut TheRGBABuffer,
        widget: &mut dyn TheWidget,
        shrinker: &mut TheDimShrinker,
        ctx: &mut TheContext,
    ) {
    }

    /// Draw the widget border
    fn draw_text_edit_border(
        &mut self,
        buffer: &mut TheRGBABuffer,
        widget: &dyn TheWidget,
        shrinker: &mut TheDimShrinker,
        ctx: &mut TheContext,
        draw_focus: bool,
        disabled: bool,
    ) {
    }

    /// Draw the widget border
    fn draw_text_area_border(
        &mut self,
        buffer: &mut TheRGBABuffer,
        widget: &dyn TheWidget,
        shrinker: &mut TheDimShrinker,
        ctx: &mut TheContext,
        draw_focus: bool,
        disabled: bool,
    ) {
    }

    /// Creates a preview image for the drop.
    fn create_drop_image(&mut self, drop: &mut TheDrop, ctx: &mut TheContext) {
        let mut width: i32 = 120;

        let size = ctx.draw.get_text_size(
            &drop.title,
            &TheFontSettings {
                size: 12.5,
                ..Default::default()
            },
        );
        width = size.0 as i32 + 20;

        if drop.offset.x > width {
            drop.offset.x = width - 10;
        }

        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, width, 24));

        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink(2);
        let mut utuple = buffer.dim().to_buffer_shrunk_utuple(&shrinker);

        let stride = buffer.stride();

        ctx.draw.rounded_rect_with_border(
            buffer.pixels_mut(),
            &utuple,
            stride,
            &self.theme().color(DropItemBackground).clone(),
            &(2.0, 2.0, 2.0, 2.0),
            self.theme().color(DropItemBorder),
            1.5,
        );

        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &utuple,
            stride,
            &drop.title,
            TheFontSettings {
                size: 12.5,
                ..Default::default()
            },
            self.theme().color(DropItemText),
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );

        drop.set_image(buffer);
    }
}
