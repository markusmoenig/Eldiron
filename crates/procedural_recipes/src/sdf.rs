use crate::{RenderError, RenderSurface, SdfRecipe, SdfShapeKind};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedSdf {
    pub width: u32,
    pub height: u32,
    pub coverage: Vec<u8>,
}

pub struct SdfRenderer;

impl SdfRenderer {
    pub fn render(recipe: &SdfRecipe, surface: &RenderSurface) -> Result<RenderedSdf, RenderError> {
        if surface.width == 0 || surface.height == 0 {
            return Err(RenderError::Dimensions(
                "SDF render dimensions must be greater than zero".to_string(),
            ));
        }
        let shapes = recipe
            .shapes
            .iter()
            .map(|shape| (shape.name.to_ascii_lowercase(), &shape.kind))
            .collect::<BTreeMap<_, _>>();
        if !shapes.contains_key(&recipe.output.to_ascii_lowercase()) {
            return Err(RenderError::Evaluation(format!(
                "unknown SDF output shape '{}'",
                recipe.output
            )));
        }
        let antialias = 1.0 / surface.width.min(surface.height) as f32;
        let mut coverage = Vec::with_capacity((surface.width * surface.height) as usize);
        for y in 0..surface.height {
            for x in 0..surface.width {
                let uv = surface.mapping.map([
                    (x as f32 + 0.5) / surface.width as f32,
                    (y as f32 + 0.5) / surface.height as f32,
                ]);
                let distance = evaluate(&recipe.output, uv, &shapes, &mut Vec::new())?;
                let value = (0.5 - distance / antialias).clamp(0.0, 1.0);
                coverage.push((value * 255.0).round() as u8);
            }
        }
        Ok(RenderedSdf {
            width: surface.width,
            height: surface.height,
            coverage,
        })
    }
}

fn evaluate(
    name: &str,
    point: [f32; 2],
    shapes: &BTreeMap<String, &SdfShapeKind>,
    stack: &mut Vec<String>,
) -> Result<f32, RenderError> {
    let key = name.to_ascii_lowercase();
    if stack.contains(&key) {
        return Err(RenderError::Evaluation(format!(
            "cyclic SDF shape reference involving '{name}'"
        )));
    }
    let kind = shapes
        .get(&key)
        .ok_or_else(|| RenderError::Evaluation(format!("unknown SDF shape '{name}'")))?;
    stack.push(key);
    let distance = match kind {
        SdfShapeKind::Ellipse {
            position,
            size,
            rotation,
        } => ellipse(local_point(point, *position, *rotation), *size),
        SdfShapeKind::RoundedRectangle {
            position,
            size,
            radius,
            rotation,
        } => rounded_rectangle(local_point(point, *position, *rotation), *size, *radius),
        SdfShapeKind::Capsule { from, to, radius } => capsule(point, *from, *to, *radius),
        SdfShapeKind::Union { a, b } => {
            evaluate(a, point, shapes, stack)?.min(evaluate(b, point, shapes, stack)?)
        }
        SdfShapeKind::Subtract { a, b } => {
            evaluate(a, point, shapes, stack)?.max(-evaluate(b, point, shapes, stack)?)
        }
        SdfShapeKind::Intersect { a, b } => {
            evaluate(a, point, shapes, stack)?.max(evaluate(b, point, shapes, stack)?)
        }
        SdfShapeKind::Expand { source, amount } => evaluate(source, point, shapes, stack)? - amount,
        SdfShapeKind::Contract { source, amount } => {
            evaluate(source, point, shapes, stack)? + amount
        }
    };
    stack.pop();
    Ok(distance)
}

fn local_point(point: [f32; 2], position: [f32; 2], rotation: f32) -> [f32; 2] {
    let angle = -rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    let x = point[0] - position[0];
    let y = point[1] - position[1];
    [x * cos - y * sin, x * sin + y * cos]
}

fn ellipse(point: [f32; 2], size: [f32; 2]) -> f32 {
    let half = [
        size[0].abs().max(0.0001) * 0.5,
        size[1].abs().max(0.0001) * 0.5,
    ];
    let normalized = ((point[0] / half[0]).powi(2) + (point[1] / half[1]).powi(2)).sqrt();
    (normalized - 1.0) * half[0].min(half[1])
}

fn rounded_rectangle(point: [f32; 2], size: [f32; 2], radius: f32) -> f32 {
    let radius = radius.max(0.0).min(size[0].abs().min(size[1].abs()) * 0.5);
    let half = [size[0].abs() * 0.5 - radius, size[1].abs() * 0.5 - radius];
    let q = [point[0].abs() - half[0], point[1].abs() - half[1]];
    let outside = [q[0].max(0.0), q[1].max(0.0)];
    (outside[0] * outside[0] + outside[1] * outside[1]).sqrt() + q[0].max(q[1]).min(0.0) - radius
}

fn capsule(point: [f32; 2], from: [f32; 2], to: [f32; 2], radius: f32) -> f32 {
    let pa = [point[0] - from[0], point[1] - from[1]];
    let ba = [to[0] - from[0], to[1] - from[1]];
    let denom = ba[0] * ba[0] + ba[1] * ba[1];
    let h = if denom <= f32::EPSILON {
        0.0
    } else {
        ((pa[0] * ba[0] + pa[1] * ba[1]) / denom).clamp(0.0, 1.0)
    };
    let delta = [pa[0] - ba[0] * h, pa[1] - ba[1] * h];
    (delta[0] * delta[0] + delta[1] * delta[1]).sqrt() - radius.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RenderSurfaceFrame, RenderSurfaceMapping, parse_sdf_document};

    const HELMET: &str = r#"
Sdf helmet
    Shape Dome
        Ellipse
            position = F2(0.5, 0.5)
            size = F2(0.8, 0.8)
    Shape Opening
        RoundedRectangle
            position = F2(0.5, 0.7)
            size = F2(0.4, 0.3)
            radius = 0.05
    Shape Shell
        Subtract
            a = Dome
            b = Opening
    Output
        coverage = Shell
"#;

    #[test]
    fn parses_and_renders_composed_sdf_coverage() {
        let recipe = parse_sdf_document(HELMET).unwrap().recipes.remove(0);
        let rendered = SdfRenderer::render(
            &recipe,
            &RenderSurface {
                width: 32,
                height: 32,
                mapping: RenderSurfaceMapping::default(),
                fps: 0.0,
                looping: false,
                frames: vec![RenderSurfaceFrame { time: 0.0 }],
            },
        )
        .unwrap();
        assert_eq!(rendered.coverage.len(), 32 * 32);
        assert!(
            rendered.coverage[8 * 32 + 16] > 200,
            "helmet crown is filled"
        );
        assert!(
            rendered.coverage[22 * 32 + 16] < 20,
            "face opening is cut out"
        );
        assert_eq!(rendered.coverage[0], 0, "coverage remains local to the SDF");
    }

    #[test]
    fn rejects_unknown_shape_references_during_parsing() {
        let error = parse_sdf_document(
            r#"
Sdf broken
    Shape A
        Union
            a = Missing
            b = MissingToo
    Output
        coverage = A
"#,
        )
        .unwrap_err();
        assert_eq!(error.code, crate::ParseErrorCode::UnknownReference);
    }
}
