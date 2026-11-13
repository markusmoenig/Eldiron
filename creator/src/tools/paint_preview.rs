//! Brush feedback is geometry, independent of a destination framebuffer.
use super::*;
use crate::editor_overlay::EditorOverlay;
#[derive(Clone)]
pub(crate) struct Source {
    visible: bool,
    operation: String,
    color: [u8; 4],
    size: f32,
}
impl Source {
    pub(crate) fn layer(layer: &IsoPaintLayer) -> Self {
        Self {
            visible: layer.visible,
            operation: layer.active_operation.clone(),
            color: layer.active_color,
            size: layer.active_size,
        }
    }
    pub(crate) fn controls(controls: &paint_overlay::Controls) -> Self {
        Self {
            visible: controls.visible,
            operation: controls.operation.clone(),
            color: controls.color,
            size: controls.size,
        }
    }
}
pub(crate) fn prepare(
    commands: &mut EditorOverlay,
    controls: &paint_overlay::Controls,
    hover: Option<Vec2<i32>>,
    selection: Option<[[i32; 2]; 4]>,
) {
    // Ring spans do not overlap across colors; selection feedback has only one
    // color. Keep each color's primitive order, including repeated alpha hits.
    let mut batches: Vec<([u8; 4], Vec<TheDim>)> = Vec::new();
    spans(
        &Source::controls(controls),
        hover,
        selection,
        &mut |rect, color| {
            if let Some((_, rects)) = batches.iter_mut().find(|(c, _)| *c == color) {
                rects.push(rect);
            } else {
                batches.push((color, vec![rect]));
            }
        },
    );
    for (color, rects) in batches {
        commands.pixel_rects(rects, color);
    }
}
pub(crate) fn spans(
    controls: &Source,
    hover: Option<Vec2<i32>>,
    selection: Option<[[i32; 2]; 4]>,
    emit: &mut impl FnMut(TheDim, [u8; 4]),
) {
    if !controls.visible && controls.operation != "select"
        || controls.operation == "pick" && hover.is_none()
    {
        return;
    }
    if controls.operation == "select" {
        if let Some(outline) = selection {
            for edge in 0..4 {
                let a = outline[edge];
                let b = outline[(edge + 1) % 4];
                let delta = [b[0] - a[0], b[1] - a[1]];
                let steps = delta[0].abs().max(delta[1].abs()).max(1);
                for step in 0..=steps {
                    let t = step as f32 / steps as f32;
                    let x = (a[0] as f32 + delta[0] as f32 * t).round() as i32;
                    let y = (a[1] as f32 + delta[1] as f32 * t).round() as i32;
                    emit(TheDim::rect(x, y, 2, 1), [42, 174, 255, 230]);
                }
            }
        }
        return;
    }
    let Some(at) = hover else {
        return;
    };
    let mut color = match controls.operation.as_str() {
        "erase" => [242, 92, 84, 230],
        "pick" => [87, 186, 255, 230],
        _ => controls.color,
    };
    color[3] = 230;
    let fill = [color[0], color[1], color[2], 38];
    let shadow = [8, 10, 12, 145];
    let radius = (controls.size * 2.).round().clamp(3., 96.) as i32;
    let outer = radius + 2;
    let inner = (radius - 2).max(1).pow(2);
    // Coalesce each ring row into constant-color spans. These are editable UI
    // geometry commands, never a rasterized/uploaded destination bitmap.
    for y in -outer..=outer {
        let mut run: Option<(i32, [u8; 4])> = None;
        for x in -outer..=outer + 1 {
            let d = x * x + y * y;
            let next = if x > outer {
                None
            } else if d <= outer * outer && d > radius * radius {
                Some(shadow)
            } else if d <= radius * radius && d >= inner {
                Some(color)
            } else if d < inner && controls.operation != "pick" {
                Some(fill)
            } else {
                None
            };
            if run.map(|(_, c)| c) != next {
                if let Some((start, c)) = run.take() {
                    emit(TheDim::rect(at.x + start, at.y + y, x - start, 1), c);
                }
                run = next.map(|c| (x, c));
            }
        }
    }
}
