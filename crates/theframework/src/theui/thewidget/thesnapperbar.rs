use crate::prelude::*;

fn draw_snapper_chrome(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    bounds: ThePixelRect,
    radius: f32,
    open: bool,
    fill: &ThePaint,
    border: &ThePaint,
    marker: &ThePaint,
    painter: &mut ThePainter,
) {
    let inner = ThePixelRect::new(
        bounds.x.saturating_add(1),
        bounds.y.saturating_add(1),
        bounds.width.saturating_sub(2),
        bounds.height.saturating_sub(2),
    );
    let Ok(mut surface) = TheSurfaceMut::new(pixels, width, height) else {
        return;
    };
    surface.set_clip(bounds);
    painter.fill_round_rect(&mut surface, bounds, radius, border);
    painter.fill_round_rect(&mut surface, inner, (radius - 1.0).max(0.0), fill);

    let center_y = bounds.y as f32 + bounds.height as f32 * 0.5;
    let mut path = ThePath::new();
    if open {
        path.move_to((bounds.x as f32 + 8.0, center_y - 2.5))
            .line_to((bounds.x as f32 + 18.0, center_y - 2.5))
            .line_to((bounds.x as f32 + 13.0, center_y + 3.0))
            .close();
    } else {
        path.move_to((bounds.x as f32 + 10.0, center_y - 4.5))
            .line_to((bounds.x as f32 + 10.0, center_y + 4.5))
            .line_to((bounds.x as f32 + 16.0, center_y))
            .close();
    }
    painter.fill_path(&mut surface, &path, marker);
}

#[derive(Default)]
pub struct TheSnapperbar {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,
    open: bool,
    collapse_uuid: Option<Uuid>,

    selected: bool,

    dim: TheDim,
    text: String,
    text_color: RGBA,
    background_color: Option<RGBA>,
    background_palette: Option<(TheThemePalettes, usize)>,
    is_dirty: bool,

    layout_id: TheId,

    root_mode: bool,
}

impl TheWidget for TheSnapperbar {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(22);

        Self {
            id,
            limiter,

            state: TheWidgetState::None,
            open: false,
            collapse_uuid: None,

            selected: false,

            dim: TheDim::zero(),
            text: "".to_string(),
            text_color: WHITE,
            background_color: None,
            background_palette: None,
            is_dirty: false,

            layout_id: TheId::empty(),

            root_mode: true,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn state(&self) -> TheWidgetState {
        self.state
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
    }

    fn set_value(&mut self, value: TheValue) {
        if let Some(text) = value.to_string() {
            self.text = text;
            self.is_dirty = true;
        }
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(_coord) => {
                self.is_dirty = true;
                if self.state != TheWidgetState::Clicked {
                    self.state = TheWidgetState::Clicked;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.set_focus(self.id());
                }
                redraw = true;
            }
            TheEvent::MouseUp(_coord) => {
                self.is_dirty = true;
                if self.state == TheWidgetState::Clicked {
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    self.open = !self.open;

                    ctx.ui.send(TheEvent::SnapperStateChanged(
                        self.id().clone(),
                        self.layout_id.clone(),
                        self.open,
                    ));
                }
                redraw = true;
            }
            TheEvent::Hover(_coord) => {
                if self.state != TheWidgetState::Clicked && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
            TheEvent::MouseWheel(delta) => {
                ctx.ui
                    .send(TheEvent::ScrollLayout(self.layout_id.clone(), *delta));
            }
            _ => {}
        }
        redraw
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let stride = buffer.stride();
        let hovered = self.state != TheWidgetState::Clicked && self.id().equals(&ctx.ui.hover);
        let pressed = self.state == TheWidgetState::Clicked;
        let bounds = ThePixelRect::new(
            self.dim.buffer_x,
            self.dim.buffer_y,
            self.dim.width,
            self.dim.height.saturating_sub(1),
        );
        let fill = if self.selected {
            style.theme().paint(SnapperSelected, bounds)
        } else if let Some((palette, index)) = self.background_palette {
            ThePaint::solid(style.theme().palette_color(palette, index))
        } else if let Some(color) = self.background_color {
            ThePaint::solid(color)
        } else if pressed {
            style.theme().paint(SnapperPressed, bounds)
        } else if hovered {
            style.theme().paint(SnapperHover, bounds)
        } else {
            style.theme().paint(SnapperNormal, bounds)
        };
        let border_role = if self.selected {
            SelectedTextEditBorder1
        } else if pressed {
            ToolbarButtonClickedBorder
        } else if hovered {
            ToolbarButtonHoverBorder
        } else if self.root_mode {
            SectionbarHeaderBorder
        } else {
            ListItemIconBorder
        };
        let border = ThePaint::solid(*style.theme().color(border_role));
        let marker = style.theme().paint(SnapperMarker, bounds);
        let radius = if self.root_mode { 1.5 } else { 0.5 };
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        draw_snapper_chrome(
            buffer.pixels_mut(),
            width,
            height,
            bounds,
            radius,
            self.open,
            &fill,
            &border,
            &marker,
            &mut ctx.painter,
        );

        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink_by(30, 1, 0, 0);

        let text_rect = ThePixelRect::new(
            self.dim.buffer_x.saturating_add(shrinker.left),
            self.dim.buffer_y.saturating_add(shrinker.top),
            self.dim
                .width
                .saturating_sub(shrinker.left.saturating_add(shrinker.right)),
            self.dim
                .height
                .saturating_sub(shrinker.top.saturating_add(shrinker.bottom)),
        )
        .intersection(ThePixelRect::new(0, 0, width as i32, height as i32));
        if !text_rect.is_empty() {
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(
                    text_rect.x as usize,
                    text_rect.y as usize,
                    text_rect.width as usize,
                    text_rect.height as usize,
                ),
                stride,
                &self.text,
                TheFontSettings {
                    size: 13.5,
                    ..Default::default()
                },
                &self.text_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheSnapperbarTrait {
    fn set_associated_layout(&mut self, id: TheId);
    fn set_text(&mut self, text: String);
    fn set_canvas_collapse_uuid(&mut self, collapse: Uuid);
    fn is_open(&self) -> bool;
    fn set_open(&mut self, open: bool);
    fn set_selected(&mut self, open: bool);
    fn set_root_mode(&mut self, root_mode: bool);
    fn set_text_color(&mut self, color: RGBA);
    fn set_background_color(&mut self, color: Option<RGBA>);
    fn set_background_palette(&mut self, palette: TheThemePalettes, index: usize);
}

impl TheSnapperbarTrait for TheSnapperbar {
    fn set_associated_layout(&mut self, layout_id: TheId) {
        self.layout_id = layout_id;
    }
    fn set_text(&mut self, text: String) {
        self.text = text;
        self.is_dirty = true;
    }
    fn set_canvas_collapse_uuid(&mut self, collapse: Uuid) {
        self.collapse_uuid = Some(collapse);
    }
    fn is_open(&self) -> bool {
        self.open
    }
    fn set_open(&mut self, open: bool) {
        self.open = open;
    }
    fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
        self.is_dirty = true;
    }
    fn set_root_mode(&mut self, root_mode: bool) {
        self.root_mode = root_mode;
        self.is_dirty = true;
    }
    fn set_text_color(&mut self, color: RGBA) {
        if self.text_color != color {
            self.text_color = color;
            self.is_dirty = true;
        }
    }
    fn set_background_color(&mut self, color: Option<RGBA>) {
        if self.background_color != color {
            self.background_color = color;
            self.background_palette = None;
            self.is_dirty = true;
        }
    }
    fn set_background_palette(&mut self, palette: TheThemePalettes, index: usize) {
        let value = Some((palette, index));
        if self.background_palette != value {
            self.background_color = None;
            self.background_palette = value;
            self.is_dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_chrome_clips_to_surface_and_preserves_guard_bytes() {
        const WIDTH: usize = 8;
        const HEIGHT: usize = 7;
        const GUARD: usize = 32;
        const SENTINEL: u8 = 0xa7;

        let body_len = WIDTH * HEIGHT * 4;
        let mut pixels = vec![0; body_len + GUARD];
        pixels[body_len..].fill(SENTINEL);
        draw_snapper_chrome(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ThePixelRect::new(-11, -8, 30, 24),
            1.5,
            true,
            &ThePaint::solid([42, 48, 57, 255]),
            &ThePaint::solid([88, 96, 108, 255]),
            &ThePaint::solid([235, 238, 241, 255]),
            &mut ThePainter::new(),
        );

        assert!(pixels[..body_len].iter().any(|byte| *byte != 0));
        assert!(pixels[body_len..].iter().all(|byte| *byte == SENTINEL));
    }
}
