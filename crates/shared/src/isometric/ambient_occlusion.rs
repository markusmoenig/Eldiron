use rayon::prelude::*;
use rusterix::{
    Texture, Tile,
    isometric::{Projection, lighting::Caster, surface::SurfaceFrame},
};
use theframework_core::prelude::{Vec2, Vec3};

const SAMPLES: usize = 24;
const RADIUS: f32 = 0.4;
const STRENGTH: f32 = 0.24;

/// Explicit offline pass for a static isometric tile. Opening a project only
/// reads the packed result. Editing/replacing the sprite drops the old mask.
pub fn bake_ambient_occlusion(tile: &mut Tile) -> Result<usize, String> {
    let layout = tile
        .isometric
        .as_mut()
        .filter(|layout| !layout.bitmap && !layout.ground && layout.frames_per_direction == 1)
        .ok_or("Ambient occlusion requires a static isometric mesh tile")?;
    let images = if layout.frames.is_empty() {
        &tile.textures
    } else {
        &layout.frames
    };
    if images.is_empty()
        || images.len() != layout.surfaces.len()
        || !layout.pixels_per_unit.is_finite()
        || layout.pixels_per_unit <= 0.
        || images.iter().zip(&layout.surfaces).any(|(image, surface)| {
            let pixels = image.width.saturating_mul(image.height);
            image.data.len() != pixels.saturating_mul(4)
                || surface.data.len() != pixels.saturating_mul(8)
                || surface.shadow.is_none()
        })
    {
        return Err("Isometric tile is missing a matching surface or shadow bake".into());
    }
    let projection = Projection {
        origin: Vec2::new(layout.anchor[0] as f32, layout.anchor[1] as f32),
        unit: layout.pixels_per_unit,
    };
    // Fixed cosine-weighted directions keep the mask stable across rebakes.
    let rays = std::array::from_fn::<_, SAMPLES, _>(|i| {
        let radial = (i as f32 + 0.5) / SAMPLES as f32;
        let (sin, cos) = (i as f32 * 2.3999631).sin_cos();
        Vec3::new(
            cos * radial.sqrt(),
            sin * radial.sqrt(),
            (1. - radial).sqrt(),
        )
    });
    let affected = images
        .par_iter()
        .zip(&mut layout.surfaces)
        .map(|(image, surface)| {
            let mask = frame_mask(image, surface, projection, &rays);
            let affected = mask.iter().filter(|value| **value >= 0.5 / 63.).count();
            if affected > 0 || surface.has_baked_ao {
                assert!(surface.set_baked_ambient_occlusion(&mask));
            }
            affected
        })
        .sum();
    Ok(affected)
}

fn frame_mask(
    image: &Texture,
    surface: &SurfaceFrame,
    projection: Projection,
    rays: &[Vec3<f32>; SAMPLES],
) -> Vec<f32> {
    let caster = Caster {
        grid: surface.shadow.as_ref().unwrap(),
        exact: false,
        position: Vec3::zero(),
        scale: 1.,
    };
    // Offset from the shaded surface to avoid hitting its own shadow cell.
    let bias = (1.5 / projection.unit).clamp(0.02, 0.07);
    image
        .data
        .chunks_exact(4)
        .enumerate()
        .map(|(i, rgba)| {
            if rgba[3] < 128 {
                return 0.;
            }
            let Some(sample) = surface.sample(i) else {
                return 0.;
            };
            let x = (i % image.width) as f32 + 0.5;
            let y = (i / image.width) as f32 + 0.5;
            let point = projection.at_height(sample.height).ground(Vec2::new(x, y));
            let normal = sample.normal;
            let axis = if normal.y.abs() < 0.9 {
                Vec3::unit_y()
            } else {
                Vec3::unit_x()
            };
            let tangent = axis.cross(normal).normalized();
            let bitangent = normal.cross(tangent);
            let origin = Vec3::new(point.x, sample.height, point.y) + normal * bias;
            let blocked = rays
                .iter()
                .filter(|ray| {
                    let direction = tangent * ray.x + bitangent * ray.y + normal * ray.z;
                    caster.blocks(origin, direction, RADIUS)
                })
                .count();
            blocked as f32 * (STRENGTH / SAMPLES as f32)
        })
        .collect()
}
