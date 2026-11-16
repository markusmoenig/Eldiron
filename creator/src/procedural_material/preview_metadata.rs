//! Shared CPU-only preview metadata; no renderer, worker or project mutation.
use denrim_materials::integration::{EditableMaterialAsset, InstanceRecord};
use std::collections::HashSet;

/// Native Paint recipes store stable brush variants as image frames. Baking
/// keeps every frame, but displaying a brush must not start an animation loop.
/// Only suppress playback while the recipe still matches its native generator;
/// authored MZ animation/composition changes retain their normal preview policy.
#[cfg(any(feature = "native-render", test))]
pub(super) fn display_frames(asset: &EditableMaterialAsset) -> Result<u32, String> {
    let frames = animation_settings(asset)?.frames;
    if frames > 1
        && shared::paint_source::Spec::from_asset(asset)
            .is_some_and(|spec| super::paint_assets::native_layout(asset, &spec).generator.is_some())
    {
        Ok(1)
    } else {
        Ok(frames)
    }
}

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
