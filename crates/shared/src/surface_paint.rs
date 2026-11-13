//! CPU-only publication of persistent 3D surface paint. Editable UV chunks remain
//! the serialized source; atlas pixels/lookup metadata are disposable display assets.
use crate::iso_paint::{
    ISO_PAINT_BAKED_CHUNK_SIZE, IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner,
};
use indexmap::IndexMap;
use rusterix::surface_paint::SurfacePaint;
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    sync::Arc,
};

/// Publish immutable data for a map without constructing a renderer or a device.
/// Call from source preparation/update; consumers retain the Arc across GPU loss.
pub fn sync(
    assets: &mut rusterix::Assets,
    map: theframework::prelude::Uuid,
    layer: &mut IsoPaintLayer,
) -> Result<bool, String> {
    layer.refresh_sources(assets);
    layer.ensure_baked_paint_current();
    if !layer.visible || layer.baked_chunks.is_empty() {
        return Ok(assets.surface_paint.remove(&map).is_some());
    }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for (id, chunk) in &layer.baked_chunks {
        id.hash(&mut hash);
        chunk.paint_geo.hash(&mut hash);
        chunk.origin.hash(&mut hash);
        Builder::iso_paint_owner_geo_id(&chunk.owner).hash(&mut hash);
        chunk.revision.hash(&mut hash);
        chunk.color_rgba.hash(&mut hash);
        chunk.material_rgba.hash(&mut hash);
        let expected =
            ISO_PAINT_BAKED_CHUNK_SIZE as usize * ISO_PAINT_BAKED_CHUNK_SIZE as usize * 4;
        if chunk.color_rgba.len() != expected || chunk.material_rgba.len() != expected {
            return Err(format!(
                "Surface paint chunk {id} has invalid pixel dimensions"
            ));
        }
    }
    for owner in &layer.surface_instance_owners {
        Builder::iso_paint_owner_geo_id(owner).hash(&mut hash);
    }
    let key = hash.finish();
    if assets
        .surface_paint
        .get(&map)
        .is_some_and(|old| old.key == key)
    {
        return Ok(false);
    }
    let Some((width, height, color_rgba, material_rgba, entries)) =
        Builder::build_surface_paint_atlas(&layer.baked_chunks)
    else {
        return Ok(false);
    };
    let source = SurfacePaint {
        key,
        width,
        height,
        color_rgba,
        material_rgba,
        entries,
        alpha_geo_ids: Builder::surface_paint_alpha_geo_ids(layer),
    };
    assets.surface_paint.insert(map, Arc::new(source));
    Ok(true)
}

pub(crate) struct Builder;
impl Builder {
    pub(crate) fn surface_paint_entry_hash(geo: [u32; 4], origin: [i32; 2]) -> u32 {
        let mut hash = 2_166_136_261_u32;
        for value in geo.into_iter().chain(origin.map(|value| value as u32)) {
            hash = (hash ^ value).wrapping_mul(16_777_619);
        }
        hash
    }

    pub(crate) fn surface_paint_entry_table(
        entries: Vec<scenevm::Raster3DSurfacePaintEntry>,
    ) -> Vec<scenevm::Raster3DSurfacePaintEntry> {
        let table_len = (entries.len().saturating_mul(2)).next_power_of_two().max(2);
        let empty = scenevm::Raster3DSurfacePaintEntry {
            geo: [0; 4],
            uv_origin: [0; 2],
            uv_size: [0; 2],
            atlas_rect: [0; 4],
        };
        let mut table = vec![empty; table_len];
        for entry in entries {
            let mut index = Self::surface_paint_entry_hash(entry.geo, entry.uv_origin) as usize
                & (table_len - 1);
            while table[index].uv_size != [0; 2] {
                index = (index + 1) & (table_len - 1);
            }
            table[index] = entry;
        }
        table
    }

    pub(crate) fn build_surface_paint_atlas(
        baked_chunks: &IndexMap<String, IsoPaintBakedChunk>,
    ) -> Option<(
        u32,
        u32,
        Vec<u8>,
        Vec<u8>,
        Vec<scenevm::Raster3DSurfacePaintEntry>,
    )> {
        if baked_chunks.is_empty() {
            return None;
        }
        let count = baked_chunks.len();
        let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
        let rows = count.div_ceil(columns);
        let chunk_size = ISO_PAINT_BAKED_CHUNK_SIZE as usize;
        let width = (columns * chunk_size) as u32;
        let height = (rows * chunk_size) as u32;
        let mut color = vec![0_u8; width as usize * height as usize * 4];
        let mut material = vec![0_u8; width as usize * height as usize * 4];
        let mut entries = Vec::with_capacity(count);

        for (index, chunk) in baked_chunks.values().enumerate() {
            let atlas_x = (index % columns) * chunk_size;
            let atlas_y = (index / columns) * chunk_size;
            for row in 0..chunk_size {
                let source = row * chunk_size * 4;
                let destination = ((atlas_y + row) * width as usize + atlas_x) * 4;
                color[destination..destination + chunk_size * 4]
                    .copy_from_slice(&chunk.color_rgba[source..source + chunk_size * 4]);
                material[destination..destination + chunk_size * 4]
                    .copy_from_slice(&chunk.material_rgba[source..source + chunk_size * 4]);
            }
            entries.push(scenevm::Raster3DSurfacePaintEntry {
                geo: chunk.paint_geo,
                uv_origin: chunk.origin,
                uv_size: [ISO_PAINT_BAKED_CHUNK_SIZE as u32; 2],
                atlas_rect: [
                    atlas_x as u32,
                    atlas_y as u32,
                    chunk_size as u32,
                    chunk_size as u32,
                ],
            });
        }
        let entries = Self::surface_paint_entry_table(entries);
        Some((width, height, color, material, entries))
    }

    pub(crate) fn surface_paint_alpha_geo_ids(layer: &IsoPaintLayer) -> Vec<scenevm::GeoId> {
        let mut seen = HashSet::new();
        let mut geo_ids = Vec::new();
        let mut has_alpha_paint = false;
        for chunk in layer.baked_chunks.values() {
            let needs_alpha = chunk.material_rgba.chunks_exact(4).any(|pixel| {
                if (pixel[0] == 252 || (128..=187).contains(&pixel[0])) && pixel[3] > 0 {
                    return true;
                }
                if pixel[0] != 254 || pixel[3] == 0 || pixel[2] == 0 {
                    return false;
                }
                let material_id = pixel[1];
                let replace_opacity = pixel[2].saturating_sub(1);
                replace_opacity < 254 || Self::iso_paint_material_is_translucent(material_id)
            });
            if needs_alpha {
                has_alpha_paint = true;
                let geo_id = Self::iso_paint_owner_geo_id(&chunk.owner);
                if seen.insert(geo_id) {
                    geo_ids.push(geo_id);
                }
            }
        }
        if has_alpha_paint {
            for owner in &layer.surface_instance_owners {
                let geo_id = Self::iso_paint_owner_geo_id(owner);
                if seen.insert(geo_id) {
                    geo_ids.push(geo_id);
                }
            }
        }
        geo_ids
    }

    pub(crate) fn iso_paint_material_is_translucent(material_id: u8) -> bool {
        matches!(material_id / 4, 5 | 6)
    }

    pub(crate) fn iso_paint_owner_geo_id(owner: &IsoPaintOwner) -> scenevm::GeoId {
        match owner {
            IsoPaintOwner::Unknown(id) => scenevm::GeoId::Unknown(*id),
            IsoPaintOwner::Vertex(id) => scenevm::GeoId::Vertex(*id),
            IsoPaintOwner::Linedef(id) => scenevm::GeoId::Linedef(*id),
            IsoPaintOwner::Sector(id) => scenevm::GeoId::Sector(*id),
            IsoPaintOwner::Character(id) => scenevm::GeoId::Character(*id),
            IsoPaintOwner::Item(id) => scenevm::GeoId::Item(*id),
            IsoPaintOwner::Light(id) => scenevm::GeoId::Light(*id),
            IsoPaintOwner::ItemLight(id) => scenevm::GeoId::ItemLight(*id),
            IsoPaintOwner::Triangle(id) => scenevm::GeoId::Triangle(*id),
            IsoPaintOwner::Terrain { x, z } => scenevm::GeoId::Terrain(*x, *z),
            IsoPaintOwner::GeometryObject(id) => scenevm::GeoId::GeometryObject(*id),
            IsoPaintOwner::Hole { sector_id, hole_id } => {
                scenevm::GeoId::Hole(*sector_id, *hole_id)
            }
            IsoPaintOwner::Gizmo(id) => scenevm::GeoId::Gizmo(*id),
        }
    }
}
