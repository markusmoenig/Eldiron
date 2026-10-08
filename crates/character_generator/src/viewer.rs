use crate::*;
use serde_json::json;
use std::{collections::BTreeMap, path::Path};
/// Offline WebGL inspection of the generated mesh and sampled procedural poses.
/// Uses the same bind positions, skin weights and materials as GLB export.
pub fn export_viewer(asset: &CharacterAsset, path: impl AsRef<Path>) -> Result<()> {
    let poses: BTreeMap<_, _> = asset
        .motions
        .iter()
        .map(|motion| {
            let samples: Vec<_> = (0..=32)
                .map(|i| {
                    asset
                        .world_pose(&asset.local_pose(*motion, i as f32 / 32.0 * motion.duration()))
                })
                .collect();
            (
                motion.name(),
                json!({"duration":motion.duration(),"looping":motion.looping(),"samples":samples}),
            )
        })
        .collect();
    let payload = json!({"asset":asset,"bind":asset.bind_pose(),"clips":poses});
    let data = serde_json::to_string(&payload)?.replace('<', "\\u003c");
    std::fs::write(
        path,
        include_str!("viewer3d.html").replace("__MODEL__", &data),
    )?;
    Ok(())
}
