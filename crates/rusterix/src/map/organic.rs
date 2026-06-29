use crate::PixelSource;
use serde::{Deserialize, Serialize};
use vek::Vec2;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicVineStroke {
    pub stroke_id: i32,
    pub seq: i32,
    pub start: Vec2<f32>,
    pub end: Vec2<f32>,
    pub anchor_offset: f32,
    pub width: f32,
    pub depth: f32,
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub grow_positive: bool,
    pub cap_start: bool,
    pub cap_end: bool,
}

pub fn default_organic_vine_strokes() -> Vec<OrganicVineStroke> {
    Vec::new()
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum OrganicGrowthShape {
    Bush,
    Tree,
}

fn default_growth_shape() -> OrganicGrowthShape {
    OrganicGrowthShape::Bush
}

fn default_growth_lobes() -> i32 {
    0
}

fn default_growth_spread() -> f32 {
    0.0
}

fn default_growth_trunk_height() -> f32 {
    0.0
}

fn default_growth_trunk_radius() -> f32 {
    0.0
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicBushCluster {
    pub center: Vec2<f32>,
    pub anchor_offset: f32,
    pub radius: f32,
    pub height: f32,
    pub layers: i32,
    pub taper: f32,
    pub breakup: f32,
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub grow_positive: bool,
    #[serde(default = "default_growth_shape")]
    pub shape: OrganicGrowthShape,
    #[serde(default = "default_growth_lobes")]
    pub canopy_lobes: i32,
    #[serde(default = "default_growth_spread")]
    pub canopy_spread: f32,
    #[serde(default = "default_growth_trunk_height")]
    pub trunk_height: f32,
    #[serde(default = "default_growth_trunk_radius")]
    pub trunk_radius: f32,
}

pub fn default_organic_bush_clusters() -> Vec<OrganicBushCluster> {
    Vec::new()
}
