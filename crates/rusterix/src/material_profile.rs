use theframework::prelude::*;

/// High-level PBR material profiles with a single adjustment knob `k`.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MaterialProfile {
    Wood,
    Stone,
    Dirt,
    Metal,
    Water,
    Plastic,
    Fabric,
    Glass,
}

impl MaterialProfile {
    /// Compute the *target* (metallic, roughness) at full effect for a given color.
    /// No clamping here; your pipeline can decide ranges.
    pub fn evaluate_target(&self, color: Vec3<f32>) -> (f32, f32) {
        let r = color.x;
        let g = color.y;
        let b = color.z;
        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let saturation = max_c - min_c; // chroma
        let brightness = 0.2126 * r + 0.7152 * g + 0.0722 * b; // luma-ish

        match *self {
            MaterialProfile::Wood => {
                let metallic = 0.05 * saturation; // tiny tint-driven metallic feel
                let base_r = 0.70 - 0.20 * brightness + 0.20 * (1.0 - saturation);
                let roughness = base_r - 0.50; // full-effect: polish by ~0.5
                (metallic, roughness)
            }
            MaterialProfile::Stone => {
                let metallic = 0.0;
                let base_r = 0.80 + 0.10 * (1.0 - brightness);
                let roughness = base_r - 0.60; // full-effect: strong polish
                (metallic, roughness)
            }
            MaterialProfile::Dirt => {
                let metallic = 0.0;
                let base_r = 0.90 + 0.30 * (1.0 - saturation);
                let roughness = base_r - 0.70; // wet/polished dirt at max
                (metallic, roughness)
            }
            MaterialProfile::Metal => {
                let metallic = 0.60 + 0.40 * saturation + 0.20; // extra push at max effect
                let base_r = 0.60 + 0.40 * (1.0 - brightness);
                let roughness = base_r - 0.60; // more mirror-like
                (metallic, roughness)
            }
            MaterialProfile::Water => {
                let metallic = 0.0;
                let base_r = 0.10;
                let roughness = base_r - 0.09; // smoother
                (metallic, roughness)
            }
            MaterialProfile::Plastic => {
                let metallic = 0.0;
                let base_r = 0.70 + 0.20 * (1.0 - saturation);
                let roughness = base_r - 0.60;
                (metallic, roughness)
            }
            MaterialProfile::Fabric => {
                let metallic = 0.0;
                let base_r = 0.70 + 0.20 * (1.0 - brightness);
                let roughness = base_r - 0.40;
                (metallic, roughness)
            }
            MaterialProfile::Glass => {
                let metallic = 0.0;
                let base_r = 0.05;
                let roughness = base_r - 0.04;
                (metallic, roughness)
            }
        }
    }
}
