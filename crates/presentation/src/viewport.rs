use theframework::{
    paint::{Rect, Transform},
    platform::InputEvent,
};
/// Authored screen space is independent of host logical units and physical DPI.
/// This single mapping is used for painting, hit testing, scroll and IME areas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenViewport {
    pub scale: f32,
    pub offset: [f32; 2],
}
impl ScreenViewport {
    pub fn fit(screen: [f32; 2], host: [f32; 2]) -> Self {
        if screen
            .into_iter()
            .chain(host)
            .any(|v| !v.is_finite() || v <= 0.)
        {
            return Self {
                scale: 0.,
                offset: [0.; 2],
            };
        }
        let scale = (host[0] / screen[0]).min(host[1] / screen[1]);
        if !scale.is_finite() || scale <= 0. {
            return Self {
                scale: 0.,
                offset: [0.; 2],
            };
        }
        Self {
            scale,
            offset: [
                (host[0] - screen[0] * scale) / 2.,
                (host[1] - screen[1] * scale) / 2.,
            ],
        }
    }
    pub fn transform(self) -> Transform {
        Transform::scale(self.scale, self.scale)
            .then(Transform::translate(self.offset[0], self.offset[1]))
    }
    pub fn host_rect(self, r: Rect) -> Rect {
        Rect::new(
            self.offset[0] + r.x * self.scale,
            self.offset[1] + r.y * self.scale,
            r.width * self.scale,
            r.height * self.scale,
        )
    }
    pub fn input(self, event: &InputEvent) -> Option<InputEvent> {
        let point = |p: [f32; 2]| {
            if self.scale > 0. && p.iter().all(|v| v.is_finite()) {
                let mapped = [
                    (p[0] - self.offset[0]) / self.scale,
                    (p[1] - self.offset[1]) / self.scale,
                ];
                mapped.iter().all(|v| v.is_finite()).then_some(mapped)
            } else {
                None
            }
        };
        let vector = |p: [f32; 2]| {
            if self.scale > 0. && p.iter().all(|v| v.is_finite()) {
                let mapped = [p[0] / self.scale, p[1] / self.scale];
                mapped.iter().all(|v| v.is_finite()).then_some(mapped)
            } else {
                None
            }
        };
        Some(match event {
            InputEvent::PointerMoved { position } => InputEvent::PointerMoved {
                position: point(*position)?,
            },
            InputEvent::PointerButton {
                button,
                pressed,
                position,
            } => InputEvent::PointerButton {
                button: *button,
                pressed: *pressed,
                position: point(*position)?,
            },
            InputEvent::Scroll {
                delta,
                precise: true,
            } => InputEvent::Scroll {
                delta: vector(*delta)?,
                precise: true,
            },
            // Relative camera motion is a device delta, not a screen-space
            // displacement. Resizing/letterboxing cannot change sensitivity.
            InputEvent::RelativeMotion { delta } => {
                if self.scale <= 0. || !delta.iter().all(|value| value.is_finite()) {
                    return None;
                }
                event.clone()
            }
            _ => event.clone(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_letterbox_paint_input_and_ime_use_the_same_mapping() {
        let viewport = ScreenViewport::fit([400., 240.], [801., 603.]);
        assert_eq!(viewport.scale, 2.0025);
        let r = viewport.host_rect(Rect::new(40., 20., 2., 24.));
        let Some(InputEvent::PointerMoved { position }) =
            viewport.input(&InputEvent::PointerMoved {
                position: [r.x, r.y],
            })
        else {
            panic!()
        };
        assert!((position[0] - 40.).abs() < 0.0001 && (position[1] - 20.).abs() < 0.0001);
        assert!((r.height - 48.06).abs() < 0.0001);
        // Line units are a policy, not transformed host pixels.
        assert_eq!(
            viewport.input(&InputEvent::Scroll {
                delta: [1., 2.],
                precise: false
            }),
            Some(InputEvent::Scroll {
                delta: [1., 2.],
                precise: false
            })
        );
        assert!(matches!(
            viewport.input(&InputEvent::Scroll {
                delta: [0., 4.005],
                precise: true
            }),
            Some(InputEvent::Scroll {
                delta: [0., 2.],
                precise: true
            })
        ));
        assert_eq!(
            viewport.input(&InputEvent::RelativeMotion { delta: [10., -10.] }),
            Some(InputEvent::RelativeMotion { delta: [10., -10.] }),
            "raw look sensitivity is independent of viewport scale"
        );
        assert!(
            ScreenViewport::fit([400., 240.], [0., 0.])
                .input(&InputEvent::PointerMoved { position: [1., 1.] })
                .is_none()
        );
        assert!(
            viewport
                .input(&InputEvent::PointerMoved {
                    position: [f32::NAN, 0.]
                })
                .is_none()
        );
    }
}

use theframework::platform::{LogicalKey, PhysicalKey};
pub fn player_binding_key(physical: PhysicalKey, logical: &LogicalKey) -> Option<String> {
    use PhysicalKey::*;
    Some(match physical {
        Letter(letter) => (letter.to_ascii_lowercase() as char).to_string(),
        Digit(digit) => char::from(b'0' + digit.min(9)).to_string(),
        Function(number) => format!("f{number}"),
        Escape => "escape".into(),
        Enter => "enter".into(),
        Space => "space".into(),
        Tab => "tab".into(),
        Backspace => "backspace".into(),
        Delete => "delete".into(),
        Left => "left".into(),
        Right => "right".into(),
        Up => "up".into(),
        Down => "down".into(),
        Home => "home".into(),
        End => "end".into(),
        PageUp => "pageup".into(),
        PageDown => "pagedown".into(),
        _ => {
            if let LogicalKey::Character(key) = logical {
                key.to_ascii_lowercase()
            } else {
                return None;
            }
        }
    })
}
