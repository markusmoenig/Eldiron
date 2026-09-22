use super::*;

#[derive(Clone, Copy, Debug)]
pub enum GraphControlInput {
    Press {
        /// Horizontal position within the control, 0..1.
        fraction: f32,
        /// Full position within the control, 0..1 on both axes. Lists use it to
        /// address a cell; single-control rows only need `fraction`.
        point: [f32; 2],
        /// Node metrics, so a list press hits the same row bands that were
        /// painted in the current node style.
        metrics: GraphMetrics,
    },
    Drag {
        fraction: f32,
        point: [f32; 2],
    },
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

    /// Advance width of a run of text, used to place a text caret from a click.
    /// Hosts with real font metrics override this; the default approximates a
    /// proportional face so a click still lands near the intended character.
    fn text_width(&self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.6
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
            GraphControlValue::List { rows, .. } => match rows.len() {
                0 => "No entries".into(),
                1 => "1 entry".into(),
                n => format!("{n} entries"),
            },
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
                GraphControlInput::Press { fraction, .. }
                | GraphControlInput::Drag { fraction, .. },
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
                GraphControlInput::Press { fraction, .. },
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
            (
                GraphControlValue::List { columns, rows },
                GraphControlInput::Press { point, metrics, .. },
            ) => edit_list(columns, rows, point, metrics),
            _ => None,
        }
    }
}

/// One press inside a list: cycle a cell, add a row, or delete one.
fn edit_list(
    columns: &[GraphListColumn],
    rows: &[Vec<GraphControlValue>],
    point: [f32; 2],
    metrics: GraphMetrics,
) -> Option<GraphControlValue> {
    let total = metrics.list_header + metrics.list_row * (rows.len() as f32 + 1.);
    let y = point[1].clamp(0., 1.) * total;
    if y < metrics.list_header {
        return None;
    }
    let index = ((y - metrics.list_header) / metrics.list_row).floor() as usize;
    let mut rows = rows.to_vec();
    if index >= rows.len() {
        // The trailing row appends a new entry seeded from the prototypes.
        if index > rows.len() || columns.is_empty() {
            return None;
        }
        rows.push(columns.iter().map(|c| c.control.clone()).collect());
        return Some(GraphControlValue::List {
            columns: columns.to_vec(),
            rows,
        });
    }
    if point[0] >= LIST_DELETE_FRACTION {
        rows.remove(index);
        return Some(GraphControlValue::List {
            columns: columns.to_vec(),
            rows,
        });
    }
    let count = columns.len();
    let column = ((point[0].clamp(0., LIST_DELETE_FRACTION) / LIST_DELETE_FRACTION) * count as f32)
        .floor()
        .min((count - 1) as f32) as usize;
    let start = column as f32 / count as f32 * LIST_DELETE_FRACTION;
    let end = (column + 1) as f32 / count as f32 * LIST_DELETE_FRACTION;
    let local = ((point[0] - start) / (end - start)).clamp(0., 1.);
    let cell = rows.get(index)?.get(column)?.clone();
    let updated = BasicGraphControls.interact(
        &cell,
        GraphControlInput::Press {
            fraction: local,
            point: [local, 0.5],
            metrics,
        },
    )?;
    rows[index][column] = updated;
    Some(GraphControlValue::List {
        columns: columns.to_vec(),
        rows,
    })
}
