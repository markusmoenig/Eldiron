//! Shared CPU-only preview metadata; no renderer, worker or project mutation.
use denrim_materials::integration::{EditableMaterialAsset, InstanceRecord};
use std::collections::HashSet;
pub(super) fn animation_settings(
    asset: &EditableMaterialAsset,
) -> Result<denrim_materials::format::Animation, String> {
    let document = asset
        .resolved(&InstanceRecord::new("animation-metadata"))
        .map_err(|e| e.to_string())?;
    let mut result = denrim_materials::format::Animation::default();
    let mut pending = vec![asset.selected_material.clone()];
    let mut visited = HashSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let material = document
            .materials
            .iter()
            .find(|m| m.id == id)
            .ok_or("Animation source is missing")?;
        let animation = material.animation.validate().map_err(|e| e.to_string())?;
        if animation.frames > 1 {
            if result.frames > 1 && result.frames != animation.frames {
                return Err("Animated composition sources must share frame count".into());
            }
            result = animation;
        }
        if let Some(mix) = &material.mixing {
            pending.extend(mix.inputs.iter().flatten().cloned());
            pending.extend(mix.mask_material.iter().cloned());
        }
    }
    Ok(result)
}
