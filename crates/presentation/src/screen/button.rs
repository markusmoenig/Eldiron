use super::*;
use rusterix::{
    Assets, Rect,
    client::widget::{ButtonVisualState, Widget},
};
use theframework::paint::{ImageHandle, PaintOp, Rect as Uv, Sampling};

/// A controller/asset preparer resolves entity/item/portrait content before paint.
/// Painting receives only its source handle and display policy, never a server.
#[derive(Clone, Copy, Debug)]
pub enum ButtonContent {
    Empty,
    Image {
        image: ImageHandle,
        source_size: [u32; 2],
        contain: bool,
        inset: f32,
        quantity: u32,
    },
}
impl ScreenPainter<'_> {
    pub fn button(
        &mut self,
        widget: &Widget,
        assets: &Assets,
        state: ButtonVisualState,
        command: Option<&str>,
        frame: usize,
        content: ButtonContent,
    ) -> Result<(), String> {
        if widget.rect.width <= 0. || widget.rect.height <= 0. {
            return Ok(());
        }
        let style = widget.presentation_style(state, command);
        if let Some(background) = style.background {
            self.fill(widget.rect, background);
        }
        if !self.artwork(&widget.chrome_artwork, widget.rect, assets, state, frame)? {
            if let Some(texture) = widget.chrome_textures.get(Widget::texture_index_for_state(
                widget.chrome_textures.len(),
                state,
            )) {
                self.sliced_texture(
                    &format!("texture/{:p}", texture),
                    texture,
                    widget.rect,
                    widget.chrome_slice,
                )?;
            }
        }
        let item_slot = widget.inventory_index.is_some() || widget.equipped_slot.is_some();
        let icon = if widget.show_icon && !item_slot {
            Widget::command_icon_texture(
                assets,
                command.or(widget.command.as_deref()),
                state,
                frame,
            )
        } else {
            None
        };
        if let Some(texture) = icon {
            let image = self.images.rgba(
                &format!("texture/{:p}", texture),
                texture.width as u32,
                texture.height as u32,
                &texture.data,
            )?;
            let inset = widget
                .icon_inset
                .unwrap_or_else(|| {
                    (widget.rect.width.min(widget.rect.height) * 0.12)
                        .round()
                        .max(2.)
                })
                .max(0.);
            let bounds = Rect::new(
                widget.rect.x + inset,
                widget.rect.y + inset,
                (widget.rect.width - inset * 2.).max(1.),
                (widget.rect.height - inset * 2.).max(1.),
            );
            self.list.ops.push(PaintOp::FilteredImage {
                image,
                rect: rect(bounds),
                uv: Uv::new(0., 0., 1., 1.),
                sampling: Sampling::Nearest,
                tint: [1.; 4],
                filter: Widget::command_icon_filter(state),
            });
        } else if !self.artwork(&widget.artwork, widget.rect, assets, state, frame)? {
            if let Some(texture) = widget.textures.get(Widget::texture_index_for_state(
                widget.textures.len(),
                state,
            )) {
                self.texture(
                    &format!("texture/{:p}", texture),
                    texture,
                    widget.rect,
                    Uv::new(0., 0., 1., 1.),
                    Sampling::Nearest,
                    [1.; 4],
                )?;
            }
        }
        if let ButtonContent::Image {
            image,
            source_size,
            contain,
            inset,
            quantity,
        } = content
        {
            if source_size[0] > 0 && source_size[1] > 0 {
                let mut bounds = widget.rect.with_border(inset.max(0.));
                if contain {
                    let scale = (bounds.width / source_size[0] as f32)
                        .min(bounds.height / source_size[1] as f32);
                    let w = source_size[0] as f32 * scale;
                    let h = source_size[1] as f32 * scale;
                    bounds.x += (bounds.width - w) * 0.5;
                    bounds.y += (bounds.height - h) * 0.5;
                    bounds.width = w;
                    bounds.height = h;
                }
                self.list.image(image, rect(bounds), Sampling::Nearest);
                if quantity > 1 {
                    let label = quantity.to_string();
                    let layout = self.text.layout(&label, "Roboto", 10., None);
                    let badge = Rect::new(
                        bounds.x + bounds.width - layout.width() - 5.,
                        bounds.y + bounds.height - 14.,
                        layout.width() + 5.,
                        14.,
                    );
                    self.fill(badge, [0, 0, 0, 200]);
                    self.label(badge, &label, "Roboto", 10., [255; 4], Align::Center, true);
                }
            }
        }
        if !self.artwork(&widget.frame_artwork, widget.rect, assets, state, frame)? {
            if let Some(texture) = widget.frame_textures.get(Widget::texture_index_for_state(
                widget.frame_textures.len(),
                state,
            )) {
                self.sliced_texture(
                    &format!("texture/{:p}", texture),
                    texture,
                    widget.rect,
                    widget.frame_slice,
                )?;
            }
        }
        self.styled_border(
            widget.rect,
            widget.border_size.max(0) as f32,
            style.border,
            widget.border_gradient_color,
            widget.border_gradient_direction,
            widget.border_radius,
        );
        if !widget.label.trim().is_empty() {
            self.label(
                Rect::new(
                    widget.rect.x + 4.,
                    widget.rect.y,
                    (widget.rect.width - 8.).max(0.),
                    widget.rect.height,
                ),
                &widget.label,
                &widget.label_font,
                widget.label_font_size,
                style.label,
                Align::Center,
                true,
            );
        }
        Ok(())
    }
}
