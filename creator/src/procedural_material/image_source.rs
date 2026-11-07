//! Image-source ownership and project tile links for the shared material editor.
use super::*;
use denrim_materials::image_source::ImageSource;

/// Include authored bundles so undo/reopen also refreshes stale embedded caches.
/// Only linked bitmap assets are hashed; unrelated tiles do not add per-frame work.
pub(super) fn input_key(project: &Project) -> u64 {
    #[derive(Default)]
    struct Links {
        revision: u64,
        tiles: std::collections::BTreeSet<Uuid>,
        models: bool,
    }
    thread_local! { static LINKS: std::cell::RefCell<Links> = std::cell::RefCell::new(Links::default()); }
    LINKS.with(|links| {
        let mut links = links.borrow_mut();
        let revision = project.procedural_materials.revision();
        if links.revision != revision {
            links.revision = revision;
            links.tiles.clear();
            links.models = false;
            for bundle in project.procedural_materials.values() {
                links.models |= bundle.contains("eldiron:model:");
                for (offset, _) in bundle.match_indices("eldiron:tile:") {
                    if let Some(id) = bundle
                        .get(offset + 13..offset + 49)
                        .and_then(|s| Uuid::parse_str(s).ok())
                    {
                        links.tiles.insert(id);
                    }
                }
            }
        }
        let images = links
            .tiles
            .iter()
            .map(|id| {
                (
                    *id,
                    project
                        .tiles
                        .get(id)
                        .filter(|_| !project.procedural_materials.contains_key(id))
                        .map(composition::raster_source::key),
                )
            })
            .collect::<Vec<_>>();
        rusterix::procedural_avatar::fingerprint(&(
            revision,
            if links.models {
                project.procedural_appearances.revision()
            } else {
                0
            },
            if links.models {
                palette::key(&project.palette)
            } else {
                0
            },
            images,
        ))
    })
}

pub(super) fn from_tile(id: Uuid, tile: &rusterix::Tile) -> Result<ImageSource, String> {
    let first = tile.textures.first().ok_or("Source tile has no frames")?;
    let (w, h) = (first.width, first.height);
    if tile.textures.len() > 32
        || tile
            .textures
            .iter()
            .any(|t| t.width != w || t.height != h || t.data.len() != w * h * 4)
    {
        return Err("Image sources need 1–32 equally sized RGBA frames".into());
    }
    let mut image = ImageSource::rgba8(
        w,
        h,
        &tile
            .textures
            .iter()
            .map(|t| t.data.clone())
            .collect::<Vec<_>>(),
    )
    .map_err(|e| e.to_string())?;
    image.reference = Some(format!("eldiron:tile:{id}"));
    let materials = tile.to_material_array();
    let mut planes: [Vec<Vec<u8>>; 4] = std::array::from_fn(|_| vec![]);
    let encode = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    let linear = |v: u8| {
        let v = v as f32 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let srgb = |v: f32| {
        if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        }
    };
    for (frame, texture) in tile.textures.iter().enumerate() {
        let mut texture = texture.clone();
        if let Some(bytes) = materials.get(frame).filter(|b| b.len() == w * h * 4) {
            texture.data_ext = Some(bytes.clone());
        }
        let mut maps: [Vec<u8>; 4] = std::array::from_fn(|_| Vec::with_capacity(w * h * 4));
        for y in 0..h {
            for x in 0..w {
                let (rough, metal, opacity, emission) = texture.get_materials(x as u32, y as u32);
                for (i, value) in [rough, metal, opacity].into_iter().enumerate() {
                    maps[i].extend_from_slice(&[encode(value), encode(value), encode(value), 255]);
                }
                let p = &texture.data[(y * w + x) * 4..][..4];
                maps[3].extend_from_slice(&[
                    encode(srgb(linear(p[0]) * emission)),
                    encode(srgb(linear(p[1]) * emission)),
                    encode(srgb(linear(p[2]) * emission)),
                    255,
                ]);
            }
        }
        for (i, map) in maps.into_iter().enumerate() {
            planes[i].push(map);
        }
    }
    for ((channel, space), frames) in [
        ("roughness", "linear"),
        ("metallic", "linear"),
        ("opacity", "linear"),
        ("emission", "srgb"),
    ]
    .into_iter()
    .zip(planes)
    {
        image
            .set_plane(channel, space, &frames)
            .map_err(|e| e.to_string())?;
    }
    Ok(image)
}
pub(super) fn set(
    asset: &EditableMaterialAsset,
    operation: &str,
    image: Option<ImageSource>,
) -> Result<EditableMaterialAsset, String> {
    let mut next = asset.clone();
    let mat = next
        .root
        .project
        .materials
        .iter_mut()
        .find(|m| m.id == next.selected_material)
        .ok_or("Missing material")?;
    let tile = mat
        .rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find(|t| t.id == operation)
        .ok_or("Image source was removed")?;
    if tile.kernel != "Image / Tile Source" {
        return Err("Select an Image / Tile Source tile".into());
    }
    if let Some(image) = &image {
        image.validate().map_err(|e| e.to_string())?;
        mat.animation.frames = mat.animation.frames.max(image.frame_count() as u32);
    }
    if tile.image.is_none()
        && image.is_some()
        && !tile.parameters.iter().any(|(id, _)| id == "uvMode")
    {
        tile.parameters
            .push(("uvMode".into(), Binding::Constant(vec![0.])));
    }
    tile.image = image;
    next.validate().map_err(|e| e.to_string())?;
    Ok(next)
}
/// Refresh linked image caches after pixel or material edits, preserving embedded
/// pixels if the source is absent (e.g. a standalone MaterialZ export).
pub(crate) fn refresh(
    asset: &EditableMaterialAsset,
    project: &Project,
) -> Result<Option<EditableMaterialAsset>, String> {
    let mut next = asset.clone();
    let mut changed = false;
    for mat in &mut next.root.project.materials {
        for tile in mat.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            let Some(reference) = tile
                .image
                .as_ref()
                .and_then(|i| i.reference.as_deref())
                .and_then(|r| r.strip_prefix("eldiron:tile:"))
            else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(reference) else {
                continue;
            };
            let Some(source) = project
                .tiles
                .get(&id)
                .filter(|_| !project.procedural_materials.contains_key(&id))
            else {
                continue;
            };
            let image = from_tile(id, source)?;
            if tile.image.as_ref() != Some(&image) {
                mat.animation.frames = mat.animation.frames.max(image.frame_count() as u32);
                tile.image = Some(image);
                changed = true;
            }
        }
    }
    if changed {
        next.validate().map_err(|e| e.to_string())?;
        Ok(Some(next))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tile_sources_preserve_frames_materials_links_and_reopen() {
        let id = Uuid::new_v4();
        let mut first = rusterix::Texture::from_color([255, 0, 0, 128]);
        first.set_materials_all(0.25, 0.75, 0.5, 0.5);
        let mut source = rusterix::Tile::from_textures(vec![
            first,
            rusterix::Texture::from_color([0, 0, 255, 255]),
        ]);
        source.baked_material_data = source.textures.iter().map(|t| t.material_bytes()).collect();
        let image = from_tile(id, &source).unwrap();
        assert_eq!(image.frame_count(), 2);
        assert_eq!(image.planes.len(), 5);
        let a=EditableMaterialAsset::parse("image","project 1\nmaterial \"m\" called \"Image\"\n  mode pbr\n  row \"r\"\n    use \"Image / Tile Source\" id \"image\"\n  output opacity from row \"r\" w\n","m").unwrap();
        let a = set(&a, "image", Some(image)).unwrap();
        assert_eq!(a.root.project.materials[0].animation.frames, 2);
        let reopened = EditableMaterialAsset::reopen(&a.save().unwrap()).unwrap();
        assert_eq!(a.root.project, reopened.root.project);
        let p = reopened
            .program(&denrim_materials::integration::InstanceRecord::new("test"))
            .unwrap();
        let alpha = denrim_materials::cpu::Evaluator::new(&p)
            .sample("m", denrim_materials::ir::Channel::Opacity, [0., 0.])
            .unwrap()[0];
        assert!(alpha > 0.2 && alpha < 0.3);
        let mut project = Project::default();
        project.tiles.insert(id, source);
        assert!(refresh(&a, &project).unwrap().is_none());
        project.tiles.get_mut(&id).unwrap().textures[0].data[0] = 0;
        assert!(refresh(&a, &project).unwrap().is_some());
        let before = input_key(&project);
        project
            .procedural_materials
            .insert(Uuid::new_v4(), a.save().unwrap());
        let linked = input_key(&project);
        assert_ne!(before, linked);
        project.tiles.get_mut(&id).unwrap().textures[1].data[0] = 44;
        assert_ne!(
            linked,
            input_key(&project),
            "Later animation frames must invalidate image links"
        );
    }
}
