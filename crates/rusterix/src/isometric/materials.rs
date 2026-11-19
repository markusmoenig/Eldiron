//! Source selection for isometric floor/terrain materials. These borrowed cell
//! policies are independent of the camera, rasterizer and destination pixels.
use crate::{Assets, Map, PixelSource, Sector, Tile, Value};
use std::{
    collections::{BTreeMap, HashMap},
    hash::{Hash, Hasher},
};

/// Shared material clock; spatial pages are selected independently of time.
pub fn animation_frame(seconds: f32) -> usize {
    (seconds.max(0.) / 0.25) as usize
}

/// Give source sampling a stable owner at exact cell and half-paint-texel
/// boundaries. GPU reciprocal/FMA rounding can move an analytical boundary by
/// a few ulps. Only values within one millionth of a world unit are snapped;
/// geometry, depth, lighting and authored coordinates remain untouched.
/// Keep synchronized with sampling_world in presentation/surface.wgsl.
pub fn sampling_coordinate(value: f32) -> f32 {
    let boundary = (value * 64.).round() / 64.;
    if (value - boundary).abs() <= 0.000001 {
        boundary
    } else {
        value
    }
}

#[derive(Clone, Copy)]
pub struct MaterialSelection<'a> {
    pub primary: &'a Tile,
    pub secondary: Option<([f32; 4], &'a Tile)>,
}
/// Cell overrides and soft-edge sources retain their reference precedence.
/// Native producers consume the same selection while rasterizing on the GPU.
pub struct SurfaceMaterials<'a> {
    base: Option<&'a Tile>,
    tiles: HashMap<(i32, i32), &'a Tile>,
    blends: HashMap<(i32, i32), ([f32; 4], &'a Tile)>,
}
fn tile<'a>(source: &PixelSource, assets: &'a Assets) -> Option<&'a Tile> {
    source
        .render_tile_id(assets)
        .and_then(|id| assets.tile_index(&id))
        .and_then(|i| assets.tile_list.get(i as usize))
}
impl<'a> SurfaceMaterials<'a> {
    pub fn for_floor(map: &Map, sector: &Sector, base: &'a Tile, assets: &'a Assets) -> Self {
        let mut sources = Self {
            base: Some(base),
            tiles: HashMap::new(),
            blends: HashMap::new(),
        };
        let height = sector.properties.get_float_default("floor_height", 0.);
        for props in [
            (height.abs() < 0.001).then_some(&map.properties),
            Some(&sector.properties),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(Value::TileOverrides(overrides)) = props.get("tiles") {
                sources.tiles.extend(
                    overrides.iter().filter_map(|(cell, source)| {
                        tile(source, assets).map(|tile| (*cell, tile))
                    }),
                );
            }
            if let Some(Value::BlendOverrides(overrides)) = props.get("blend_tiles") {
                sources
                    .blends
                    .extend(overrides.iter().filter_map(|(cell, (preset, source))| {
                        tile(source, assets).map(|tile| (*cell, (preset.weights(), tile)))
                    }));
            }
        }
        sources
    }
    pub fn for_terrain(map: &Map, assets: &'a Assets) -> Self {
        let outputs = crate::chunkbuilder::terrain_material::outputs(map);
        let resolve = |source: &PixelSource| {
            source
                .render_tile_id(assets)
                .map(|id| crate::chunkbuilder::terrain_material::resolve(&outputs, assets, id))
                .and_then(|id| assets.tile_index(&id))
                .and_then(|i| assets.tile_list.get(i as usize))
        };
        let mut sources = Self {
            base: map
                .properties
                .get_source("default_terrain_tile")
                .and_then(resolve),
            tiles: HashMap::new(),
            blends: HashMap::new(),
        };
        for sector in &map.sectors {
            if sector
                .properties
                .get_float_default("floor_height", 0.)
                .abs()
                > 0.001
            {
                continue;
            }
            if let Some(tile) = sector
                .properties
                .get_default_source()
                .and_then(resolve)
                .filter(|t| t.isometric.as_ref().is_some_and(|s| s.ground))
            {
                let bounds = sector.bounding_box(map);
                if (bounds.max - bounds.min - 1.).magnitude_squared() < 0.00001 {
                    sources.tiles.insert(
                        (bounds.min.x.floor() as i32, bounds.min.y.floor() as i32),
                        tile,
                    );
                }
            }
        }
        if let Some(Value::TileOverrides(overrides)) = map.properties.get("tiles") {
            sources.tiles.extend(
                overrides
                    .iter()
                    .filter_map(|(cell, source)| resolve(source).map(|tile| (*cell, tile))),
            );
        }
        if let Some(Value::BlendOverrides(overrides)) = map.properties.get("blend_tiles") {
            sources
                .blends
                .extend(overrides.iter().filter_map(|(cell, (preset, source))| {
                    resolve(source).map(|tile| (*cell, (preset.weights(), tile)))
                }));
        }
        sources
    }
    pub fn cell(&self, cell: (i32, i32)) -> Option<MaterialSelection<'a>> {
        Some(MaterialSelection {
            primary: self.tiles.get(&cell).copied().or(self.base)?,
            secondary: self.blends.get(&cell).copied(),
        })
    }

    /// Identity of the resolved cell policy and the source planes sampled by
    /// terrain. A publisher can advance its tile revision for unrelated
    /// assets without changing any of these inputs.
    pub fn source_fingerprint(&self) -> u64 {
        let mut hash = rustc_hash::FxHasher::default();
        self.base.map(|tile| tile.id).hash(&mut hash);
        let mut sources = BTreeMap::new();
        if let Some(base) = self.base {
            sources.insert(base.id, base);
        }
        let mut overrides: Vec<_> = self.tiles.iter().collect();
        overrides.sort_unstable_by_key(|(cell, _)| **cell);
        overrides.len().hash(&mut hash);
        for (cell, &tile) in overrides {
            (cell, tile.id).hash(&mut hash);
            sources.insert(tile.id, tile);
        }
        let mut blends: Vec<_> = self.blends.iter().collect();
        blends.sort_unstable_by_key(|(cell, _)| **cell);
        blends.len().hash(&mut hash);
        for (cell, &(weights, tile)) in blends {
            (cell, weights.map(f32::to_bits), tile.id).hash(&mut hash);
            sources.insert(tile.id, tile);
        }
        for (id, tile) in sources {
            id.hash(&mut hash);
            tile.uv_pages.as_ref().map(|pages| {
                (
                    pages.animation_frames,
                    pages.repeat_period.map(f32::to_bits),
                    pages.origin,
                    pages.size,
                    pages.units_per_page,
                )
            }).hash(&mut hash);
            (&tile.material.preset, &tile.material.finish).hash(&mut hash);
            tile.textures.len().hash(&mut hash);
            for texture in &tile.textures {
                (texture.width, texture.height).hash(&mut hash);
                texture.data.hash_source(&mut hash);
                texture.data_ext.is_some().hash(&mut hash);
                if let Some(data) = &texture.data_ext {
                    data.hash_source(&mut hash);
                }
            }
            tile.baked_material_data.len().hash(&mut hash);
            for data in &tile.baked_material_data {
                data.hash_source(&mut hash);
            }
        }
        hash.finish()
    }
}
