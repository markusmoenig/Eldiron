//! Explicit image-domain periodicity. Seamless mode crossfades shifted source
//! evaluations in an edge band; quintic weights match value and slope at seams.
use crate::{
    Error, Result,
    ir::{Binding, Channel, Cost, Node, NodeId, Parameter, Program, Reference},
    metadata::Kernel,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Raw,
    Repeat,
    Seamless,
}
impl Mode {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "raw" => Ok(Self::Raw),
            "repeat" => Ok(Self::Repeat),
            "seamless" => Ok(Self::Seamless),
            _ => Err(Error(format!("Unknown tiling mode {s:?}"))),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Repeat => "repeat",
            Self::Seamless => "seamless",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tiling {
    pub mode: Mode,
    pub period: [f32; 2],
    pub blend: f32,
}
impl Default for Tiling {
    fn default() -> Self {
        Self {
            mode: Mode::Raw,
            period: [1.; 2],
            blend: 0.15,
        }
    }
}
impl Tiling {
    pub fn validate(self) -> Result<Self> {
        if self
            .period
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0001 || *v > 1000000.)
            || !self.blend.is_finite()
            || !(0.01..=0.5).contains(&self.blend)
        {
            return Err(Error(
                "Tiling needs positive periods and a seam blend between 0.01 and 0.5".into(),
            ));
        }
        Ok(self)
    }
}
pub(crate) fn append(
    p: &mut Program,
    input: Reference,
    tiling: Tiling,
    label: &str,
) -> Result<NodeId> {
    let t = tiling.validate()?;
    let mut text = String::from(
        "tile 1\nname \"Periodic Image\"\ncategory \"Transform\"\ngroup \"Tiling\" true\nparam float rot \"Rotation\" default 0\nparam vec2 offset \"Offset\" default 0 0\nparam float globalScale \"Scale\" default 1\nparam vec2 localScale \"Local Scale\" default 1 1\n",
    );
    text.push_str(&format!(
        "param vec2 period \"Period\" default {} {}\nparam float edge \"Seam Blend\" default {}\n",
        t.period[0], t.period[1], t.blend
    ));
    text.push_str("execution image cacheRadius cacheUnits\nparam float cacheRadius \"Cache Radius\" default 0\nparam float cacheUnits \"Cache Units\" default 0\n");
    text.push_str("let q = fract(uv / period + 0.5)\nlet p = (q - 0.5) * period\ndomain = p\n");
    if t.mode == Mode::Seamless {
        text.push_str("let t = clamp((q - (1 - edge)) / edge, 0, 1)\nlet w = t * t * t * (t * (t * 6 - 15) + 10)\nlet b = sample_image(p - vec2(period.x, 0))\nlet c = sample_image(p - vec2(0, period.y))\nlet d = sample_image(p - period)\nimage = mix(mix(input, b, w.x), mix(c, d, w.x), w.y)\n");
    } else {
        text.push_str("image = input\n")
    }
    let metadata = crate::tilelang::definition(&text)?;
    let algorithm = crate::tilelang::Algorithm::compile(&text, &metadata, Kernel::Transform)?;
    let parameters = metadata
        .parameters
        .into_iter()
        .map(|v| Parameter {
            id: v.id,
            kind: v.kind,
            binding: Binding::Constant(v.default),
            choice_count: 0,
        })
        .collect::<Vec<_>>();
    // Transient tile previews get the same expansion limits as material outputs.
    let mut cache_input = false;
    if !p.costs.is_empty() {
        let child = p.costs[input.node.0];
        let samples = if t.mode == Mode::Seamless { 4 } else { 1 };
        let mut cost = Cost {
            depth: child.depth + 1,
            expanded_calls: child.expanded_calls.saturating_mul(samples) + 1,
            operations: child.operations.saturating_mul(samples) + algorithm.operations(),
        };
        if cost.expanded_calls > 4096 || cost.operations > 131072 {
            cache_input = true;
            cost.expanded_calls = 1 + samples;
            cost.operations = 8 * samples + algorithm.operations();
        }
        if cost.depth > 64
            || cost.expanded_calls > 4096
            || cost.operations > 131072
            || p.nodes.len() >= 4096
        {
            return Err(Error("Periodic preview exceeds the computation budget; reduce complexity or bake the source".into()));
        }
        p.costs.push(cost);
        p.order.push(NodeId(p.nodes.len()));
    }
    let id = NodeId(p.nodes.len());
    p.nodes.push(Node {
        algorithm,
        code: text,
        kernel: Kernel::Transform,
        parameters,
        input: Some(input),
        coordinates: None,
        row_id: label.into(),
        tile_index: 0,
        combine: crate::format::Combine::Replace,
        has_distance: false,
        value_dependencies: [1; 4],
        distance_dependencies: 1,
        uniform_distance: false,
        cache_input,
        image_execution: metadata.image_execution.clone(),
    });
    Ok(id)
}
pub(crate) fn lower_material(p: &mut Program, index: usize, limit: usize) -> Result<()> {
    let tiling = p.materials[index].tiling.validate()?;
    if tiling.mode == Mode::Raw {
        return Ok(());
    }
    for (slot, (channel, reference)) in p.materials[index].outputs.clone().into_iter().enumerate() {
        if channel == Channel::Distance {
            continue;
        }
        let width = channel.width();
        let mut input = reference;
        if input.distance {
            let value = crate::material_mix::node(
                p,
                "distance image",
                vec![("value", Binding::From(input))],
                "image = vec4(vec3(value),1)\n",
                3,
            )?;
            if let Binding::From(r) = value {
                input = r
            } else {
                unreachable!()
            }
        }
        // Inputs are RGB functions; scalar channels select their lane in all three.
        if width == 1 {
            input.components = vec![input.components[0]; 3]
        }
        let id = append(
            p,
            input,
            tiling,
            &format!("periodic:{index}/{}", channel.name()),
        )?;
        p.materials[index].outputs[slot].1 = Reference {
            field: None,
            node: id,
            components: (0..width).collect(),
            mapping: None,
            distance: false,
        };
        if p.nodes.len() > limit {
            return Err(Error(
                "Node budget exceeded by periodic material outputs".into(),
            ));
        }
    }
    Ok(())
}
