use super::*;

#[derive(Clone, Copy, Debug)]
pub enum GraphControlInput {
    Press { fraction: f32 },
    Drag { fraction: f32 },
}
/// Node-native, mouse-only controls operate in normalized local coordinates.
/// Custom implementations can resolve previews or open a host picker via an edit request.
pub trait GraphControls {
    /// Draw custom content inside the control bounds; return true to replace
    /// default content. Coordinates are already transformed by the viewport.
    fn paint(
        &self,
        _value: &GraphControlValue,
        _rect: GraphRect,
        _painter: &mut dyn GraphPainter,
    ) -> bool {
        false
    }

    fn label(&self, value: &GraphControlValue) -> String;
    fn interact(
        &self,
        value: &GraphControlValue,
        input: GraphControlInput,
    ) -> Option<GraphControlValue>;
}
pub struct BasicGraphControls;
impl GraphControls for BasicGraphControls {
    fn label(&self, value: &GraphControlValue) -> String {
        match value {
            GraphControlValue::Number {
                value, min, step, ..
            } if step.fract() == 0. && min.fract() == 0. => format!("{value:.0}"),
            GraphControlValue::Number { value, .. } => format!("{value:.2}"),
            GraphControlValue::Choice { options, selected } => options
                .get(*selected)
                .cloned()
                .unwrap_or_else(|| "—".into()),
            GraphControlValue::Toggle(v) => if *v { "Yes" } else { "No" }.into(),
            GraphControlValue::Label(v) | GraphControlValue::Text(v) => v.clone(),
            GraphControlValue::Preview { caption, .. } => caption.clone(),
            GraphControlValue::Custom { kind, .. } => kind.clone(),
        }
    }
    fn interact(
        &self,
        value: &GraphControlValue,
        input: GraphControlInput,
    ) -> Option<GraphControlValue> {
        match (value, input) {
            (
                GraphControlValue::Number { min, max, step, .. },
                GraphControlInput::Press { fraction } | GraphControlInput::Drag { fraction },
            ) if min.is_finite()
                && max.is_finite()
                && step.is_finite()
                && max >= min
                && *step > 0. =>
            {
                let v = min + ((max - min) * fraction.clamp(0., 1.) / step).round() * step;
                Some(GraphControlValue::Number {
                    value: v.clamp(*min, *max),
                    min: *min,
                    max: *max,
                    step: *step,
                })
            }
            (
                GraphControlValue::Choice { options, selected },
                GraphControlInput::Press { fraction },
            ) if !options.is_empty() => {
                let current = *selected % options.len();
                let next = if fraction < 0.5 {
                    (current + options.len() - 1) % options.len()
                } else {
                    (current + 1) % options.len()
                };
                Some(GraphControlValue::Choice {
                    options: options.clone(),
                    selected: next,
                })
            }
            (GraphControlValue::Toggle(v), GraphControlInput::Press { .. }) => {
                Some(GraphControlValue::Toggle(!v))
            }
            _ => None,
        }
    }
}
