pub mod d2builder;
pub mod d2material;
pub mod d2preview;
pub mod d3builder;

use crate::{CompiledLight, Map, Material, PixelSource, ShapeFXRole, Value, ValueContainer};
use vek::{Vec2, Vec3};

/// Gets a material from a geometry graph
pub fn get_material_from_geo_graph(
    properties: &ValueContainer,
    terminal: usize,
    map: &Map,
) -> Option<Material> {
    if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
        properties.get("region_graph")
    {
        if let Some(graph) = map.shapefx_graphs.get(graph_id) {
            let nodes = graph.collect_nodes_from(0, terminal);
            for node in nodes {
                if graph.nodes[node as usize].role == ShapeFXRole::Material {
                    return graph.nodes[node as usize].compile_material();
                }
            }
        }
    }

    None
}

/// Gets a light from a geometry graph
pub fn get_light_from_geo_graph(
    properties: &ValueContainer,
    terminal: usize,
    map: &Map,
) -> Option<CompiledLight> {
    if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
        properties.get("region_graph")
    {
        if let Some(graph) = map.shapefx_graphs.get(graph_id) {
            let nodes = graph.collect_nodes_from(0, terminal);
            for node in nodes {
                if graph.nodes[node as usize].role == ShapeFXRole::PointLight {
                    return graph.nodes[node as usize].compile_light(Vec3::zero());
                }
            }
        }
    }

    None
}

/// Gets a light from a geometry graph
pub fn get_linedef_light_from_geo_graph(
    properties: &ValueContainer,
    terminal: usize,
    map: &Map,
    p1: Vec2<f32>,
    p2: Vec2<f32>,
    y: f32,
) -> Option<CompiledLight> {
    if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
        properties.get("region_graph")
    {
        if let Some(graph) = map.shapefx_graphs.get(graph_id) {
            let nodes = graph.collect_nodes_from(0, terminal);
            for node in nodes {
                if graph.nodes[node as usize].role == ShapeFXRole::PointLight {
                    let position = (p1 + p2) / 2.0; // Midpoint of the line
                    let direction = (p2 - p1).normalized(); // Direction of the line
                    let normal = Vec2::new(direction.y, -direction.x); // Perpendicular normal
                    // let width = (p2 - p1).magnitude(); // Line segment length
                    let offset = 0.1;
                    let position = position + normal * offset;

                    let position = Vec3::new(position.x, y, position.y);
                    return graph.nodes[node as usize].compile_light(position);
                }
            }
        }
    }

    None
}
