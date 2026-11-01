//! Import an ordinary tile as a self-contained image material without changing it.
use super::*;
use std::fmt::Write;
pub(super) fn asset(id: Uuid, tile: &rusterix::Tile) -> Result<EditableMaterialAsset, String> {
    let mut texture = tile.textures.first().ok_or("Tile has no image")?.clone();
    if let Some(materials) = tile
        .to_material_array()
        .first()
        .filter(|bytes| bytes.len() == texture.width * texture.height * 4)
    {
        texture.data_ext = Some(materials.clone());
    }
    if texture.width == 0
        || texture.height == 0
        || texture
            .width
            .checked_mul(texture.height)
            .is_none_or(|n| n > 1_048_576)
    {
        return Err("Tile image must contain at most 1048576 pixels".into());
    }
    let mut maps: [Vec<u8>; 5] = std::array::from_fn(|_| Vec::with_capacity(texture.data.len()));
    let encode = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    let linear = |value: u8| {
        let v = value as f32 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let srgb = |value: f32| {
        if value <= 0.0031308 {
            value * 12.92
        } else {
            1.055 * value.powf(1. / 2.4) - 0.055
        }
    };
    for y in 0..texture.height {
        for x in 0..texture.width {
            let at = (y * texture.width + x) * 4;
            let pixel = &texture.data[at..at + 4];
            let (roughness, metallic, opacity, emission) =
                texture.get_materials(x as u32, y as u32);
            maps[0].extend_from_slice(pixel);
            for (index, value) in [
                (1, roughness),
                (2, metallic),
                (3, opacity * pixel[3] as f32 / 255.),
            ] {
                maps[index].extend_from_slice(&[encode(value), encode(value), encode(value), 255]);
            }
            maps[4].extend_from_slice(&[
                encode(srgb(linear(pixel[0]) * emission)),
                encode(srgb(linear(pixel[1]) * emission)),
                encode(srgb(linear(pixel[2]) * emission)),
                255,
            ]);
        }
    }
    let mut recipe =
        "project 1\nmaterial \"surface\" called \"Image tile\"\n  mode pbr\n  tiling raw\n"
            .to_owned();
    let code = denrim_materials::tilelang::source(denrim_materials::metadata::Kernel::Color);
    for (index, channel) in ["base color", "roughness", "metallic", "opacity", "emission"]
        .into_iter()
        .enumerate()
    {
        let row = format!("raster-{index}");
        writeln!(
            recipe,
            "  output {channel} from row \"{row}\" {}",
            if index == 0 || index == 4 { "xyz" } else { "x" }
        )
        .unwrap();
        writeln!(
            recipe,
            "  row \"{row}\"\n    use \"Color\" id \"pixel-{index}\"\n      source"
        )
        .unwrap();
        for line in code.lines() {
            writeln!(recipe, "        {line}").unwrap();
        }
        write!(
            recipe,
            "        # raster-rgba8 {} {} {} ",
            texture.width,
            texture.height,
            if index == 0 || index == 4 {
                "srgb"
            } else {
                "linear"
            }
        )
        .unwrap();
        for byte in &maps[index] {
            write!(recipe, "{byte:02x}").unwrap();
        }
        recipe.push_str("\n      end source\n");
    }
    EditableMaterialAsset::parse(&id.to_string(), &recipe, "surface").map_err(|e| e.to_string())
}

/// Hash image inputs directly rather than serializing large pixel arrays every UI tick.
pub(in crate::procedural_material) fn key(tile: &rusterix::Tile) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    let mut bytes = |data: &[u8]| {
        for byte in data {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
    };
    bytes(
        serde_json::to_string(&tile.material)
            .unwrap_or_default()
            .as_bytes(),
    );
    for image in &tile.textures {
        bytes(&image.width.to_le_bytes());
        bytes(&image.height.to_le_bytes());
        bytes(&image.data);
        if let Some(ext) = &image.data_ext {
            bytes(ext);
        }
    }
    for materials in &tile.baked_material_data {
        bytes(materials);
    }
    hash
}
