//! Invisible placement sockets. Families are recipe-authored, case-sensitive keys.
use super::*;

pub const MAX_CONNECTORS: usize = 256;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connector {
    pub id: String,
    pub family: String,
    pub position: [f32; 3],
    pub direction: [f32; 3],
}
impl Connector {
    pub fn validate(&self) -> Result<()> {
        if self.family.is_empty()
            || self.family.len() > 64
            || !self
                .family
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c))
        {
            return Err(fail(
                "Connector family needs 1–64 letters, digits, _, -, . or /",
            ));
        }
        if self.id.is_empty()
            || self.id.len() > 4096
            || self
                .position
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 100_000.)
            || self.direction.iter().any(|v| !v.is_finite())
            || (self.direction.iter().map(|v| v * v).sum::<f32>() - 1.).abs() > 0.001
        {
            return Err(fail(
                "Connector needs a stable identity, finite position and unit direction",
            ));
        }
        Ok(())
    }
}
pub fn validate(connectors: &[Connector]) -> Result<()> {
    if connectors.len() > MAX_CONNECTORS {
        return Err(fail("Geometry exceeds 256 connectors"));
    }
    let mut ids = HashSet::new();
    for connector in connectors {
        connector.validate()?;
        if !ids.insert(&connector.id) {
            return Err(fail("Duplicate connector identity"));
        }
    }
    Ok(())
}
