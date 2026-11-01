//! Visual, tile-local composition presets. Highlighted pixels use the added material.
use super::*;
pub(super) const PICKER: &str = "Composition Shape Picker";
const COLUMNS: usize = 4;
const CELL_HEIGHT: i32 = 66;
pub(super) struct Preset {
    pub label: &'static str,
    shape: CompositionShape,
    angle: f32,
    position: f32,
}
pub(super) const PRESETS: [Preset; 17] = [
    Preset {
        label: "Right",
        shape: CompositionShape::Split,
        angle: 0.,
        position: 0.5,
    },
    Preset {
        label: "Left",
        shape: CompositionShape::Split,
        angle: 180.,
        position: 0.5,
    },
    Preset {
        label: "Top",
        shape: CompositionShape::Split,
        angle: -90.,
        position: 0.5,
    },
    Preset {
        label: "Bottom",
        shape: CompositionShape::Split,
        angle: 90.,
        position: 0.5,
    },
    Preset {
        label: "Diagonal ↘",
        shape: CompositionShape::Split,
        angle: 45.,
        position: 0.5,
    },
    Preset {
        label: "Diagonal ↗",
        shape: CompositionShape::Split,
        angle: -45.,
        position: 0.5,
    },
    Preset {
        label: "Curve TL",
        shape: CompositionShape::Corner,
        angle: 0.,
        position: 0.72,
    },
    Preset {
        label: "Curve TR",
        shape: CompositionShape::Corner,
        angle: 90.,
        position: 0.72,
    },
    Preset {
        label: "Curve BL",
        shape: CompositionShape::Corner,
        angle: -90.,
        position: 0.72,
    },
    Preset {
        label: "Curve BR",
        shape: CompositionShape::Corner,
        angle: 180.,
        position: 0.72,
    },
    Preset {
        label: "Top/bottom",
        shape: CompositionShape::Strips,
        angle: 0.,
        position: 0.18,
    },
    Preset {
        label: "Left/right",
        shape: CompositionShape::Strips,
        angle: 90.,
        position: 0.18,
    },
    Preset {
        label: "Road H",
        shape: CompositionShape::Road,
        angle: 0.,
        position: 0.4,
    },
    Preset {
        label: "Road V",
        shape: CompositionShape::Road,
        angle: 90.,
        position: 0.4,
    },
    Preset {
        label: "Cross +",
        shape: CompositionShape::Crossing,
        angle: 0.,
        position: 0.3,
    },
    Preset {
        label: "Cross X",
        shape: CompositionShape::Crossing,
        angle: 45.,
        position: 0.3,
    },
    Preset {
        label: "Blend",
        shape: CompositionShape::Blend,
        angle: 0.,
        position: 0.5,
    },
];
pub(super) fn corner(angle: f32) -> [f32; 2] {
    if angle > 135. || angle < -135. {
        [1., 1.]
    } else if angle > 45. {
        [1., 0.]
    } else if angle < -45. {
        [0., 1.]
    } else {
        [0., 0.]
    }
}
impl Preset {
    pub(super) fn apply(&self, layer: &mut CompositionLayer) {
        layer.shape = self.shape;
        layer.angle = self.angle;
        layer.position = self.position;
        if self.shape == CompositionShape::Blend {
            layer.amount = 0.5;
        }
    }
    fn matches(&self, layer: &CompositionLayer) -> bool {
        layer.shape == self.shape
            && (self.shape == CompositionShape::Blend || (layer.angle - self.angle).abs() < 0.1)
    }
    pub(super) fn coverage(&self, uv: [f32; 2]) -> f32 {
        let d = match self.shape {
            CompositionShape::Split => {
                let angle = self.angle.to_radians();
                (uv[0] - 0.5) * angle.cos() + (uv[1] - 0.5) * angle.sin() + 0.5 - self.position
            }
            CompositionShape::Corner => {
                let c = corner(self.angle);
                self.position - ((uv[0] - c[0]).powi(2) + (uv[1] - c[1]).powi(2)).sqrt()
            }
            CompositionShape::Strips => {
                let t = uv[usize::from(self.angle.abs() < 45.)];
                (self.position - t).max(t - (1. - self.position))
            }
            CompositionShape::Road | CompositionShape::Crossing => {
                let angle = self.angle.to_radians();
                let p = [uv[0] - 0.5, uv[1] - 0.5];
                let x = p[0] * angle.cos() + p[1] * angle.sin();
                let y = -p[0] * angle.sin() + p[1] * angle.cos();
                self.position / 2.
                    - if self.shape == CompositionShape::Road {
                        y.abs()
                    } else {
                        x.abs().min(y.abs())
                    }
            }
            _ => return 0.5,
        };
        if d >= 0. { 1. } else { 0. }
    }
}
pub(super) struct ShapePicker {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    selected: Option<usize>,
    hover: Option<usize>,
    dirty: bool,
}
impl ShapePicker {
    pub(super) fn for_layer(layer: &CompositionLayer) -> Self {
        let mut picker = Self::new(TheId::named(PICKER));
        picker.selected = PRESETS.iter().position(|preset| preset.matches(layer));
        picker
    }
    fn hit(&self, point: Vec2<i32>) -> Option<usize> {
        let width = self.dim.width / COLUMNS as i32;
        if point.x < 0 || point.y < 0 || width <= 0 || point.x >= width * COLUMNS as i32 {
            return None;
        }
        let index = (point.y / CELL_HEIGHT) as usize * COLUMNS + (point.x / width) as usize;
        (index < PRESETS.len()).then_some(index)
    }
}
impl TheWidget for ShapePicker {
    fn new(id: TheId) -> Self {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(
            i32::MAX,
            PRESETS.len().div_ceil(COLUMNS) as i32 * CELL_HEIGHT,
        ));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            selected: None,
            hover: None,
            dirty: true,
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
    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }
    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }
    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        self.dim = dim;
        self.dirty = true;
    }
    fn needs_redraw(&mut self) -> bool {
        self.dirty
    }
    fn set_needs_redraw(&mut self, dirty: bool) {
        self.dirty = dirty;
    }
    fn supports_hover(&mut self) -> bool {
        true
    }
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(point) => {
                if let Some(index) = self.hit(*point) {
                    self.selected = Some(index);
                    ctx.ui.send(TheEvent::IndexChanged(self.id.clone(), index));
                }
            }
            TheEvent::Hover(point) => {
                self.hover = self.hit(*point);
                ctx.ui.set_hover(&self.id);
                if let Some(index) = self.hover {
                    ctx.ui.send(TheEvent::SetStatusText(
                        self.id.clone(),
                        format!(
                            "{}: highlighted area uses the added material.",
                            PRESETS[index].label
                        ),
                    ));
                }
            }
            TheEvent::LostHover(_) => self.hover = None,
            _ => return false,
        }
        self.dirty = true;
        true
    }
    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }
        let stride = buffer.stride();
        let dim = self.dim.to_buffer_utuple();
        let theme = style.theme();
        let background = *theme.color(MaterialPanel);
        let base = *theme.color(MaterialCanvas);
        let added = *theme.color(SelectedTextEditBorder1);
        ctx.draw
            .rect(buffer.draw_target(), &dim, stride, &background);
        let width = self.dim.width as usize / COLUMNS;
        for (i, preset) in PRESETS.iter().enumerate() {
            let x = dim.0 + (i % COLUMNS) * width + 3;
            let y = dim.1 + (i / COLUMNS) * CELL_HEIGHT as usize + 2;
            let card = (x, y, width.saturating_sub(6), CELL_HEIGHT as usize - 5);
            let border = if self.selected == Some(i) {
                added
            } else if self.hover == Some(i) {
                *theme.color(MaterialMuted)
            } else {
                *theme.color(MaterialGrid)
            };
            ctx.draw
                .rect_outline_border(buffer.draw_target(), &card, stride, &border, 1);
            let mut preview = TheRGBABuffer::new(TheDim::sized(40, 40));
            for py in 0..40 {
                for px in 0..40 {
                    let amount =
                        preset.coverage([(px as f32 + 0.5) / 40., (py as f32 + 0.5) / 40.]);
                    let color: RGBA = std::array::from_fn(|c| {
                        (base[c] as f32 * (1. - amount) + added[c] as f32 * amount) as u8
                    });
                    let at = (py * 40 + px) * 4;
                    preview.pixels_mut()[at..at + 4].copy_from_slice(&color);
                }
            }
            ctx.draw.scale_chunk(
                buffer.draw_target(),
                &(x + card.2.saturating_sub(40) / 2, y + 3, 40, 40),
                stride,
                preview.pixels(),
                &(40, 40),
                1.,
            );
            ctx.draw.text_rect_blend(
                buffer.draw_target(),
                &(x + 1, y + 44, card.2.saturating_sub(2), 16),
                stride,
                preset.label,
                TheFontSettings {
                    size: 11.,
                    ..Default::default()
                },
                theme.color(MaterialText),
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        }
        self.dirty = false;
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
