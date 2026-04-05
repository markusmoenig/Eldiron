pub mod d2builder;
#[cfg(feature = "graphics")]
pub mod d2concept;
pub mod d2material;
#[cfg(feature = "graphics")]
pub mod d2preview;
#[cfg(feature = "graphics")]
pub mod d3builder;

use crate::{CompiledLight, Map, ValueContainer};
use vek::Vec2;

/// Gets a material from a geometry graph
pub fn get_material_from_geo_graph(
    _properties: &ValueContainer,
    _terminal: usize,
    _map: &Map,
) -> Option<()> {
    None
}

/// Gets a light from a geometry graph
pub fn get_light_from_geo_graph(
    _properties: &ValueContainer,
    _terminal: usize,
    _map: &Map,
) -> Option<CompiledLight> {
    None
}

/// Gets a light from a geometry graph
pub fn get_linedef_light_from_geo_graph(
    _properties: &ValueContainer,
    _terminal: usize,
    _map: &Map,
    _p1: Vec2<f32>,
    _p2: Vec2<f32>,
    _y: f32,
) -> Option<CompiledLight> {
    None
}
