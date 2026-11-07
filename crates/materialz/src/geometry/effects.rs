//! Portable effect authoring. Hosts supply particle simulation and project palette colors.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticlePreset {
    Flame,
    Smoke,
    Sparks,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleEffect {
    pub preset: ParticlePreset,
    pub position: [f32; 3],
    pub joint: Option<String>,
    pub palette: [u16; 4],
    pub rate: f32,
    pub size: f32,
    pub lifetime: f32,
    pub speed: f32,
    pub turbulence: f32,
    pub light: f32,
    pub range: f32,
}
impl Default for ParticleEffect {
    fn default() -> Self {
        Self {
            preset: ParticlePreset::Flame,
            position: [0., 0.23, 0.],
            joint: None,
            palette: [31, 30, 29, 28],
            rate: 32.,
            size: 0.035,
            lifetime: 0.55,
            speed: 0.32,
            turbulence: 0.18,
            light: 2.,
            range: 3.8,
        }
    }
}
impl ParticleEffect {
    pub fn validate(&self) -> Result<()> {
        finite(&self.position, 1000.)?;
        for (value, lo, hi) in [
            (self.rate, 0., 256.),
            (self.size, 0.001, 1.),
            (self.lifetime, 0.05, 10.),
            (self.speed, 0., 10.),
            (self.turbulence, 0., 5.),
            (self.light, 0., 20.),
            (self.range, 0., 30.),
        ] {
            if !value.is_finite() || value < lo || value > hi {
                return Err(fail("Particle effect parameter outside its range"));
            }
        }
        if self.palette.iter().any(|i| *i > 255) {
            return Err(fail(
                "Particle colors require project palette indices (0–255)",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct EffectAttachment {
    pub id: String,
    /// Position is in mesh rest space. The joint carries it through posing.
    pub joint: Option<usize>,
    pub definition: ParticleEffect,
}
