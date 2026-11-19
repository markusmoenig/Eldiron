//! Existing MaterialZ float-map to serialized Eldiron tile conversion policy.
type Maps = [Vec<[f32; 4]>; 5];
fn srgb(v: f32) -> u8 {
    let v = v.max(0.);
    let encoded = if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    };
    (encoded.clamp(0., 1.) * 255.).round() as u8
}
fn unit(v: f32) -> u8 {
    (v.clamp(0., 1.) * 255.).round() as u8
}
pub(crate) fn preview_images<const N: usize>(maps: &[Vec<[f32; 4]>; N]) -> [Vec<u8>; N] {
    std::array::from_fn(|channel| {
        maps[channel]
            .iter()
            .enumerate()
            .flat_map(|(i, value)| {
                if channel == 0 {
                    [
                        srgb(value[0]),
                        srgb(value[1]),
                        srgb(value[2]),
                        unit(maps[1][i][0]),
                    ]
                } else if channel == 4 {
                    [srgb(value[0]), srgb(value[1]), srgb(value[2]), 255]
                } else if channel == 6 {
                    [unit(value[0]), unit(value[1]), unit(value[2]), 255]
                } else {
                    let v = unit(value[0]);
                    [v, v, v, 255]
                }
            })
            .collect()
    })
}
/// Eldiron stores R/M/O/E as four 4-bit scalars. Full precision MZ outputs
/// remain available in the editor; emission hue/HDR cannot fit this tile cache.
/// Opacity belongs to RGBA alpha exactly once; material opacity stays 1 because
/// SceneVM multiplies the two during shading.
pub(super) fn bake(maps: &Maps, size: usize) -> rusterix::Texture {
    bake_rect(maps, [size; 2])
}
pub(super) fn bake_rect(maps: &Maps, size: [usize; 2]) -> rusterix::Texture {
    let rgba = preview_images(maps)[0].clone();
    let mut texture = rusterix::Texture::alloc(size[0], size[1]);
    texture.data = rgba.into();
    for y in 0..size[1] {
        for x in 0..size[0] {
            let i = y * size[0] + x;
            texture.set_materials(
                x as u32,
                y as u32,
                maps[2][i][0],
                maps[3][i][0],
                1.,
                maps[4][i][0].max(maps[4][i][1]).max(maps[4][i][2]),
            );
        }
    }
    texture
}
