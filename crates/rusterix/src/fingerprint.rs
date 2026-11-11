//! Stable domain-data fingerprints shared by preparation and presentation.
use crate::{PixelSource, Value};
use std::hash::Hasher;
// Stream serialized effect settings into the hash, avoiding a temporary byte
// vector for every sector/edge on each redraw.
pub(crate) fn hash_serialized(hasher: &mut rustc_hash::FxHasher, value: &impl serde::Serialize) {
    struct HashWriter<'a>(&'a mut rustc_hash::FxHasher);
    impl std::io::Write for HashWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.write(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let _ = serde_json::to_writer(HashWriter(hasher), value);
}

pub(crate) fn hash_properties(hasher: &mut rustc_hash::FxHasher, values: &crate::ValueContainer) {
    hasher.write_u64(values.cached_fingerprint(|| {
        let mut fingerprint = rustc_hash::FxHasher::default();
        hash_properties_uncached(&mut fingerprint, values);
        fingerprint.finish()
    }));
}

pub(crate) fn hash_properties_uncached(
    hasher: &mut rustc_hash::FxHasher,
    values: &crate::ValueContainer,
) {
    // Canonical order keeps fingerprints stable after reinsertion, cloning
    // and saving/loading. Sorting is paid only when this container changes.
    let mut entries = values.iter().collect::<Vec<_>>();
    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    for (key, value) in entries {
        hasher.write_u8(1);
        hasher.write_usize(key.len());
        hasher.write(key.as_bytes());
        match value {
            Value::Bool(value) => {
                hasher.write_u8(0);
                hasher.write_u8(u8::from(*value));
            }
            Value::Source(value) => {
                hasher.write_u8(1);
                hash_pixel_source(hasher, value);
            }
            Value::Str(value) => {
                hasher.write_u8(2);
                hasher.write_usize(value.len());
                hasher.write(value.as_bytes());
            }
            Value::Int(value) => {
                hasher.write_u8(3);
                hasher.write_i32(*value);
            }
            Value::UInt(value) => {
                hasher.write_u8(4);
                hasher.write_u32(*value);
            }
            Value::Int64(value) => {
                hasher.write_u8(5);
                hasher.write_i64(*value);
            }
            Value::Float(value) => {
                hasher.write_u8(6);
                hasher.write_u32(value.to_bits());
            }
            value => {
                hasher.write_u8(7);
                hash_serialized(hasher, value);
            }
        }
    }
    hasher.write_u8(0);
}

pub(crate) fn hash_pixel_source(hasher: &mut rustc_hash::FxHasher, source: &PixelSource) {
    match source {
        PixelSource::Off => hasher.write_u8(0),
        PixelSource::TileId(id) => {
            hasher.write_u8(1);
            hasher.write(id.as_bytes());
        }
        PixelSource::ProceduralTile(id) => {
            hasher.write_u8(15);
            hasher.write(id.as_bytes());
        }
        PixelSource::PaletteIndex(i) => {
            hasher.write_u8(12);
            hasher.write_u16(*i);
        }
        PixelSource::MaterialId(id) => {
            hasher.write_u8(2);
            hasher.write(id.as_bytes());
        }
        PixelSource::Sequence(seq) => {
            hasher.write_u8(3);
            hasher.write(seq.as_bytes());
        }
        PixelSource::EntityTile(a, b) => {
            hasher.write_u8(4);
            hasher.write_u32(*a);
            hasher.write_u32(*b);
        }
        PixelSource::ItemTile(a, b) => {
            hasher.write_u8(5);
            hasher.write_u32(*a);
            hasher.write_u32(*b);
        }
        PixelSource::Noise(noise) => {
            hasher.write_u8(16);
            hasher.write(noise.tile_id().as_bytes());
        }
        PixelSource::Color(c) => {
            hasher.write_u8(6);
            hasher.write_u32(c.r.to_bits());
            hasher.write_u32(c.g.to_bits());
            hasher.write_u32(c.b.to_bits());
            hasher.write_u32(c.a.to_bits());
        }
        PixelSource::LegacyShapeFXGraphId(id) => {
            hasher.write_u8(7);
            hasher.write(id.as_bytes());
        }
        PixelSource::StaticTileIndex(i) => {
            hasher.write_u8(8);
            hasher.write_u16(*i);
        }
        PixelSource::DynamicTileIndex(i) => {
            hasher.write_u8(9);
            hasher.write_u16(*i);
        }
        PixelSource::Pixel(px) => {
            hasher.write_u8(10);
            hasher.write_u8(px[0]);
            hasher.write_u8(px[1]);
            hasher.write_u8(px[2]);
            hasher.write_u8(px[3]);
        }
    }
}

pub(crate) fn hash_item_appearance(hasher: &mut rustc_hash::FxHasher, item: &crate::Item) {
    hasher.write_u32(item.id);
    hasher.write(item.creator_id.as_bytes());
    hasher.write(item.item_type.as_bytes());
    hash_properties(hasher, &item.attributes);
}

pub(crate) fn hash_character_items(hasher: &mut rustc_hash::FxHasher, entity: &crate::Entity) {
    let mut equipped = entity.equipped.iter().collect::<Vec<_>>();
    equipped.sort_by(|a, b| a.0.cmp(b.0));
    hasher.write_usize(equipped.len());
    for (slot, value) in equipped {
        hasher.write(slot.as_bytes());
        hash_item_appearance(hasher, value);
    }
    hasher.write_usize(entity.inventory.len());
    for value in &entity.inventory {
        hasher.write_u8(u8::from(value.is_some()));
        if let Some(value) = value {
            hash_item_appearance(hasher, value);
        }
    }
}
