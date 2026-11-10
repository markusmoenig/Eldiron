//! Derived terrain material outputs. Geometry settings remain the authored input.
use crate::{Assets, Map};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const OUTPUTS: &str = "terrain_material_outputs";
pub const PREFIX: &str = "@terrain-material/";
pub type Outputs = BTreeMap<Uuid, Uuid>;

pub fn outputs(map: &Map) -> Outputs {
    map.properties
        .get_str(OUTPUTS)
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}
pub fn output_id(map: Uuid, base: Uuid) -> Uuid {
    let a = crate::procedural_avatar::fingerprint(&("terrain-material", map, base, 0u8));
    let b = crate::procedural_avatar::fingerprint(&("terrain-material", map, base, 1u8));
    Uuid::from_u128((u128::from(a) << 64) | u128::from(b))
}
pub fn resolve(outputs: &Outputs, assets: &Assets, base: Uuid) -> Uuid {
    outputs
        .get(&base)
        .copied()
        .filter(|id| assets.tiles.contains_key(id) && assets.tile_index(id).is_some())
        .unwrap_or(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Chunk, PixelSource, Texture, Tile, Value};
    use vek::Vec2;
    #[test]
    fn shared_terrain_mesh_uses_derived_material_and_falls_back_if_cache_is_absent() {
        let mut map = Map::default();
        map.add_vertex_at(-2., -2.);
        map.add_vertex_at(2., 2.);
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        let base = Tile::from_texture(Texture::from_color([20, 80, 20, 255]));
        let mut result = Tile::from_texture(Texture::from_color([100, 60, 30, 255]));
        result.id = output_id(map.id, base.id);
        let outputs: Outputs = [(base.id, result.id)].into_iter().collect();
        map.properties.set(
            OUTPUTS,
            Value::Str(serde_json::to_string(&outputs).unwrap()),
        );
        map.properties.set(
            "default_terrain_tile",
            Value::Source(PixelSource::TileId(base.id)),
        );
        let mut assets = Assets::new();
        assets.set_tiles(
            [(base.id, base.clone()), (result.id, result.clone())]
                .into_iter()
                .collect(),
        );
        let chunk = Chunk::new(Vec2::new(-2, -2), 4);
        let generator = super::super::terrain_generator::TerrainGenerator::new(
            super::super::terrain_generator::TerrainConfig::for_map(&map),
        );
        let meshes = generator
            .generate(&map, &chunk, &assets, base.id, None)
            .unwrap();
        assert!(meshes.iter().all(|(id, _, _, _)| *id == result.id));
        assets.set_tiles([(base.id, base.clone())].into_iter().collect());
        let meshes = generator
            .generate(&map, &chunk, &assets, base.id, None)
            .unwrap();
        assert!(meshes.iter().all(|(id, _, _, _)| *id == base.id));
    }
}
