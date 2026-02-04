use crate::{Map, ValueContainer};
use serde::{Deserialize, Serialize};
use theframework::prelude::{FxHashMap, FxHashSet};
use uuid::Uuid;
use vek::Vec2;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Keyform {
    pub vertex_positions: Vec<(u32, Vec2<f32>)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SoftRig {
    pub id: Uuid,
    pub name: String,
    pub keyforms: Vec<Keyform>,
    pub in_editor_playlist: bool,

    pub values: ValueContainer,
}

impl SoftRig {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            keyforms: vec![],
            in_editor_playlist: true,
            values: ValueContainer::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SoftRigAnimator {
    pub keyframes: Vec<Uuid>,
    pub total_duration: f32, // Duration in seconds
    pub progress: f32,       // 0.0..1.0 normalized
    pub playing: bool,
    pub loop_playback: bool,
}

impl Default for SoftRigAnimator {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftRigAnimator {
    pub fn new() -> Self {
        Self {
            keyframes: Vec::new(),
            total_duration: 1.0,
            progress: 0.0,
            playing: true,
            loop_playback: true,
        }
    }

    pub fn set_progress(&mut self, value: f32) {
        self.progress = value.clamp(0.0, 1.0);
    }

    pub fn tick(&mut self, delta_time: f32) {
        if !self.playing || self.keyframes.len() < 2 || self.total_duration <= 0.0 {
            return;
        }

        self.progress += delta_time / self.total_duration;

        if self.progress >= 1.0 {
            if self.loop_playback {
                self.progress %= 1.0;
            } else {
                self.progress = 1.0;
                self.playing = false;
            }
        }
    }

    pub fn get_blended_rig(&self, map: &Map) -> Option<SoftRig> {
        let len = self.keyframes.len();
        if len == 0 {
            return None;
        } else if len == 1 {
            return map.softrigs.get(&self.keyframes[0]).cloned();
        }

        let t = self.progress * (len as f32 - 1.0);
        let i = t.floor() as usize;
        let frac = t - i as f32;

        let id_a = self.keyframes.get(i)?;
        let id_b = self.keyframes.get(i + 1).unwrap_or(id_a);

        let rig_a = map.softrigs.get(id_a)?;
        let rig_b = map.softrigs.get(id_b)?;

        Some(Self::blend_softrigs(rig_a, rig_b, frac, map))
    }

    pub fn blend_softrigs(a: &SoftRig, b: &SoftRig, t: f32, map: &Map) -> SoftRig {
        let positions_a: FxHashMap<u32, Vec2<f32>> = a
            .keyforms
            .iter()
            .flat_map(|k| k.vertex_positions.iter().copied())
            .collect();

        let positions_b: FxHashMap<u32, Vec2<f32>> = b
            .keyforms
            .iter()
            .flat_map(|k| k.vertex_positions.iter().copied())
            .collect();

        let all_ids: FxHashSet<u32> = positions_a
            .keys()
            .chain(positions_b.keys())
            .copied()
            .collect();

        let mut blended_keyform = Keyform {
            vertex_positions: Vec::new(),
        };

        for id in all_ids {
            let pa = positions_a
                .get(&id)
                .copied()
                .or_else(|| map.find_vertex(id).map(|v| Vec2::new(v.x, v.y)));
            let pb = positions_b
                .get(&id)
                .copied()
                .or_else(|| map.find_vertex(id).map(|v| Vec2::new(v.x, v.y)));

            let blended = match (pa, pb) {
                (Some(a), Some(b)) => Vec2::lerp(a, b, t),
                (Some(a), None) => a,
                (None, Some(b)) => b,
                _ => continue,
            };

            blended_keyform.vertex_positions.push((id, blended));
        }

        SoftRig {
            id: Uuid::new_v4(),
            name: "Blended".into(),
            keyforms: vec![blended_keyform],
            in_editor_playlist: false,
            values: ValueContainer::default(),
        }
    }
}
