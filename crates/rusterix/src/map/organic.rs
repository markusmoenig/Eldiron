use crate::PixelSource;
use indexmap::IndexMap;
pub use organicgraph::{OrganicBrushGraph, OrganicBrushNode, OrganicNodeKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::Vec2;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicChannelBinding {
    pub channel: i32,
    pub name: String,
    pub source: Option<PixelSource>,
    pub roughness: f32,
    pub metallic: f32,
    pub opacity: f32,
    pub emissive: f32,
}

impl OrganicChannelBinding {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                channel: 0,
                name: "Foliage".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 1,
                name: "Soil".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 2,
                name: "Stone".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 3,
                name: "Accent".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
        ]
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicSpan {
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub offset: f32,
    pub height: f32,
    pub density: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicColumn {
    pub x: i32,
    pub y: i32,
    pub spans: Vec<OrganicSpan>,
}

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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicVolumeLayer {
    pub id: Uuid,
    pub name: String,
    pub cell_size: f32,
    pub columns: Vec<OrganicColumn>,
    pub channel_bindings: Vec<OrganicChannelBinding>,
}

impl Default for OrganicVolumeLayer {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Main Organic Layer".to_string(),
            cell_size: 0.25,
            columns: Vec::new(),
            channel_bindings: OrganicChannelBinding::defaults(),
        }
    }
}

impl OrganicVolumeLayer {
    pub fn set_channel_source(&mut self, channel: i32, source: Option<PixelSource>) {
        if let Some(binding) = self
            .channel_bindings
            .iter_mut()
            .find(|binding| binding.channel == channel)
        {
            binding.source = source;
        } else {
            self.channel_bindings.push(OrganicChannelBinding {
                channel,
                name: format!("Channel {}", channel),
                source,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            });
        }
    }

    pub fn set_channel_material(
        &mut self,
        channel: i32,
        roughness: f32,
        metallic: f32,
        opacity: f32,
        emissive: f32,
    ) {
        if let Some(binding) = self
            .channel_bindings
            .iter_mut()
            .find(|binding| binding.channel == channel)
        {
            binding.roughness = roughness.clamp(0.0, 1.0);
            binding.metallic = metallic.clamp(0.0, 1.0);
            binding.opacity = opacity.clamp(0.0, 1.0);
            binding.emissive = emissive.clamp(0.0, 1.0);
        } else {
            self.channel_bindings.push(OrganicChannelBinding {
                channel,
                name: format!("Channel {}", channel),
                source: None,
                roughness: roughness.clamp(0.0, 1.0),
                metallic: metallic.clamp(0.0, 1.0),
                opacity: opacity.clamp(0.0, 1.0),
                emissive: emissive.clamp(0.0, 1.0),
            });
        }
    }

    pub fn source_for_channel(&self, channel: i32) -> Option<&PixelSource> {
        self.channel_bindings
            .iter()
            .find(|binding| binding.channel == channel)
            .and_then(|binding| binding.source.as_ref())
    }

    pub fn binding_for_channel(&self, channel: i32) -> Option<&OrganicChannelBinding> {
        self.channel_bindings
            .iter()
            .find(|binding| binding.channel == channel)
    }

    pub fn paint_sphere(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((center.x - radius) / cell_size).floor() as i32;
        let max_x = ((center.x + radius) / cell_size).ceil() as i32;
        let min_y = ((center.y - radius) / cell_size).floor() as i32;
        let max_y = ((center.y + radius) / cell_size).ceil() as i32;

        let mut changed = false;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let delta = cell_center - center;
                let dist = delta.magnitude();
                if dist > radius {
                    continue;
                }

                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                if height <= 0.0 {
                    continue;
                }

                let offset = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };

                if self.paint_column_span(
                    x,
                    y,
                    channel,
                    source.clone(),
                    offset,
                    height,
                    density * edge,
                ) {
                    changed = true;
                }
            }
        }

        changed
    }

    pub fn paint_capsule(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((start.x.min(end.x) - radius) / cell_size).floor() as i32;
        let max_x = ((start.x.max(end.x) + radius) / cell_size).ceil() as i32;
        let min_y = ((start.y.min(end.y) - radius) / cell_size).floor() as i32;
        let max_y = ((start.y.max(end.y) + radius) / cell_size).ceil() as i32;
        let segment = end - start;
        let segment_len_sq = segment.magnitude_squared();

        let mut changed = false;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let t = if segment_len_sq <= f32::EPSILON {
                    0.0
                } else {
                    ((cell_center - start).dot(segment) / segment_len_sq).clamp(0.0, 1.0)
                };
                let closest = start + segment * t;
                let dist = (cell_center - closest).magnitude();
                if dist > radius {
                    continue;
                }

                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                if height <= 0.0 {
                    continue;
                }

                let offset = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };

                if self.paint_column_span(
                    x,
                    y,
                    channel,
                    source.clone(),
                    offset,
                    height,
                    density * edge,
                ) {
                    changed = true;
                }
            }
        }

        changed
    }

    pub fn erase_sphere(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((center.x - radius) / cell_size).floor() as i32;
        let max_x = ((center.x + radius) / cell_size).ceil() as i32;
        let min_y = ((center.y - radius) / cell_size).floor() as i32;
        let max_y = ((center.y + radius) / cell_size).ceil() as i32;

        let mut changed = false;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let delta = cell_center - center;
                let dist = delta.magnitude();
                if dist > radius {
                    continue;
                }
                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                if edge <= 0.0 {
                    continue;
                }
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                let start = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };
                let end = start + height;
                if self.erase_column_range(x, y, start, end) {
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn erase_capsule(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((start.x.min(end.x) - radius) / cell_size).floor() as i32;
        let max_x = ((start.x.max(end.x) + radius) / cell_size).ceil() as i32;
        let min_y = ((start.y.min(end.y) - radius) / cell_size).floor() as i32;
        let max_y = ((start.y.max(end.y) + radius) / cell_size).ceil() as i32;
        let segment = end - start;
        let segment_len_sq = segment.magnitude_squared();

        let mut changed = false;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let t = if segment_len_sq <= f32::EPSILON {
                    0.0
                } else {
                    ((cell_center - start).dot(segment) / segment_len_sq).clamp(0.0, 1.0)
                };
                let closest = start + segment * t;
                let dist = (cell_center - closest).magnitude();
                if dist > radius {
                    continue;
                }
                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                if edge <= 0.0 {
                    continue;
                }
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                let start = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };
                let end = start + height;
                if self.erase_column_range(x, y, start, end) {
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn paint_bush_cluster(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        total_height: f32,
        anchor_offset: f32,
        layers: i32,
        taper: f32,
        breakup: f32,
        edge_softness: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 1.5);
        let total_height = total_height.max(cell_size * 1.5);
        let layer_count = layers.max(2) as usize;
        let min_x = ((center.x - radius) / cell_size).floor() as i32;
        let max_x = ((center.x + radius) / cell_size).ceil() as i32;
        let min_y = ((center.y - radius) / cell_size).floor() as i32;
        let max_y = ((center.y + radius) / cell_size).ceil() as i32;
        let slice_height = (total_height / layer_count as f32).max(cell_size * 0.6);

        let mut changed = false;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let to_cell = cell_center - center;
                let dist = to_cell.magnitude();
                if dist > radius {
                    continue;
                }

                // Stable per-cell breakup so the bush silhouette is chunky instead of conical.
                let hash = (((x * 73_856_093) ^ (y * 19_349_663)) & 1023) as f32 / 1023.0;
                let noise = (hash - 0.5) * 2.0;

                for layer_index in 0..layer_count {
                    let t = if layer_count <= 1 {
                        0.0
                    } else {
                        layer_index as f32 / (layer_count - 1) as f32
                    };
                    let layer_radius =
                        radius * (1.0 - t * taper.clamp(0.0, 1.0) * 0.72 + noise * breakup * 0.12);
                    if dist > layer_radius.max(cell_size * 0.6) {
                        continue;
                    }
                    let radial = 1.0 - (dist / layer_radius.max(cell_size * 0.6)).clamp(0.0, 1.0);
                    let softness = edge_softness.clamp(0.0, 0.999);
                    let edge = if softness <= 0.001 {
                        if radial > 0.0 { 1.0 } else { 0.0 }
                    } else {
                        let start = 1.0 - softness;
                        if radial >= start {
                            1.0
                        } else {
                            (radial / start.max(0.001)).clamp(0.0, 1.0)
                        }
                    };
                    if edge <= 0.0 {
                        continue;
                    }

                    let vertical = total_height * t * 0.72;
                    let span_height = (slice_height * (1.05 - t * 0.18 + noise * breakup * 0.08))
                        .max(cell_size * 0.45);
                    let offset = if grow_positive {
                        anchor_offset + vertical
                    } else {
                        anchor_offset - vertical - span_height
                    };
                    if self.paint_column_span(
                        x,
                        y,
                        channel,
                        source.clone(),
                        offset,
                        span_height,
                        density * edge,
                    ) {
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    fn paint_column_span(
        &mut self,
        x: i32,
        y: i32,
        channel: i32,
        source: Option<PixelSource>,
        offset: f32,
        height: f32,
        density: f32,
    ) -> bool {
        let Some(column_index) = self
            .columns
            .iter()
            .position(|column| column.x == x && column.y == y)
        else {
            self.columns.push(OrganicColumn {
                x,
                y,
                spans: vec![OrganicSpan {
                    channel,
                    source,
                    offset,
                    height,
                    density,
                }],
            });
            return true;
        };

        let start = offset;
        let end = offset + height;
        let column = &mut self.columns[column_index];

        for span in &mut column.spans {
            if span.channel != channel || span.source != source {
                continue;
            }
            let span_start = span.offset;
            let span_end = span.offset + span.height;
            if end < span_start || start > span_end {
                continue;
            }
            let merged_start = span_start.min(start);
            let merged_end = span_end.max(end);
            let merged_density = span.density.max(density);
            if (span.offset - merged_start).abs() > f32::EPSILON
                || (span.height - (merged_end - merged_start)).abs() > f32::EPSILON
                || (span.density - merged_density).abs() > f32::EPSILON
            {
                span.offset = merged_start;
                span.height = merged_end - merged_start;
                span.density = merged_density;
                return true;
            }
            return false;
        }

        column.spans.push(OrganicSpan {
            channel,
            source,
            offset,
            height,
            density,
        });
        true
    }

    fn erase_column_range(&mut self, x: i32, y: i32, start: f32, end: f32) -> bool {
        let Some(column_index) = self
            .columns
            .iter()
            .position(|column| column.x == x && column.y == y)
        else {
            return false;
        };

        let mut changed = false;
        let column = &mut self.columns[column_index];
        let mut new_spans = Vec::with_capacity(column.spans.len());

        for span in &column.spans {
            let span_start = span.offset;
            let span_end = span.offset + span.height;
            if end <= span_start || start >= span_end {
                new_spans.push(span.clone());
                continue;
            }
            changed = true;
            if start > span_start {
                new_spans.push(OrganicSpan {
                    channel: span.channel,
                    source: span.source.clone(),
                    offset: span_start,
                    height: start - span_start,
                    density: span.density,
                });
            }
            if end < span_end {
                new_spans.push(OrganicSpan {
                    channel: span.channel,
                    source: span.source.clone(),
                    offset: end,
                    height: span_end - end,
                    density: span.density,
                });
            }
        }

        column.spans = new_spans;
        if column.spans.is_empty() {
            self.columns.remove(column_index);
        }
        changed
    }
}

pub fn default_organic_layers() -> IndexMap<Uuid, OrganicVolumeLayer> {
    IndexMap::default()
}
