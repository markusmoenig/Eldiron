//! Optional, explicit adapters. No changes to Eldiron runtime dispatch or bundled rules.
use crate::{
    materials::{linear_byte, srgb_byte},
    math::*,
    *,
};
use rusterix::avatar::{
    Avatar, AvatarAnimation, AvatarAnimationFrame, AvatarPerspective, AvatarPerspectiveCount,
};
use scenevm::prelude::{DynamicMeshVertex, DynamicObject, GeoId, SharedAtlas, Vec2, Vec3};
use std::{collections::BTreeMap, path::Path};
use uuid::Uuid;
fn stable_id(scope: &str, bytes: &[u8]) -> Uuid {
    let namespace = Uuid::new_v5(&Uuid::NAMESPACE_URL, scope.as_bytes());
    Uuid::new_v5(&namespace, bytes)
}
/// Export precolored, textured body frames, excluding weapons so Eldiron's
/// equipment renderer can attach them without drawing a second baked weapon.
/// `ticks_per_second` must match the consuming client's animation counter.
pub fn export_avatar(
    asset: &CharacterAsset,
    options: BakeOptions,
    out: impl AsRef<Path>,
    ticks_per_second: f32,
) -> Result<Avatar> {
    if !ticks_per_second.is_finite() || !(0.1..=120.0).contains(&ticks_per_second) {
        return Err("avatar tick rate must be 0.1..120".into());
    }
    let mut body = asset.clone();
    body.primitives
        .retain(|p| body.materials[p.material].channel != 10);
    let out = out.as_ref();
    let metadata = bake_atlas(&body, options, out)?;
    let image = image::open(out.join("atlas.png"))?.to_rgba8();
    let mut animations = Vec::new();
    for &motion in &body.motions {
        let mut perspectives = Vec::new();
        for &direction in AvatarPerspectiveCount::Eight.directions() {
            let mut frames = Vec::new();
            for frame in metadata
                .frames
                .iter()
                .filter(|f| f.motion == motion.name() && f.direction == direction.key())
            {
                let [x, y, w, h] = frame.rect;
                let mut rgba = image::imageops::crop_imm(&image, x, y, w, h)
                    .to_image()
                    .into_raw();
                // These are final colors, not Eldiron marker templates. Avoid accidental
                // recoloring when an authored color happens to equal a reserved marker.
                for p in rgba.as_chunks_mut::<4>().0 {
                    if p[3] > 0
                        && [
                            [255, 0, 255],
                            [200, 0, 200],
                            [0, 0, 255],
                            [0, 120, 255],
                            [0, 255, 0],
                            [255, 255, 0],
                            [0, 255, 255],
                            [255, 128, 0],
                            [255, 80, 0],
                        ]
                        .contains(&[p[0], p[1], p[2]])
                    {
                        p[2] = if p[2] == 255 { 254 } else { 1 };
                    }
                }
                let anchor = |name: &str| {
                    frame
                        .sockets
                        .get(name)
                        .map(|p| (p[0].round() as i16, p[1].round() as i16))
                };
                frames.push(AvatarAnimationFrame {
                    texture: rusterix::Texture::new(rgba, w as usize, h as usize),
                    weapon_main_anchor: anchor("main_hand"),
                    weapon_off_anchor: anchor("off_hand"),
                });
            }
            perspectives.push(AvatarPerspective {
                direction,
                frames,
                weapon_main_anchor: None,
                weapon_off_anchor: None,
            });
        }
        animations.push(AvatarAnimation {
            id: stable_id(
                "eldiron/character-generator/animation",
                format!("{}/{}", asset.name, motion.name()).as_bytes(),
            ),
            name: match motion {
                Motion::Idle => "Idle",
                Motion::Walk => "Walk",
                Motion::Cast => "Cast",
                Motion::Death => "Death",
                Motion::Sit => "Sit",
                Motion::Use => "Use",
                Motion::Attack => "Attack",
                Motion::Parry => "Parry",
            }
            .into(),
            speed: motion.duration() * ticks_per_second
                / if motion.looping() {
                    options.frames
                } else {
                    options.frames - 1
                } as f32,
            perspectives,
        });
    }
    let avatar = Avatar {
        id: stable_id("eldiron/character-generator/avatar", asset.name.as_bytes()),
        name: asset.name.clone(),
        resolution: options.size as u16,
        perspective_count: AvatarPerspectiveCount::Eight,
        animations,
    };
    std::fs::write(
        out.join("character.eldiron_avatar"),
        serde_json::to_vec_pretty(&avatar)?,
    )?;
    // Body-only atlas has no matching body-only GLB; omit that link from its preview.
    let preview = std::fs::read_to_string(out.join("preview.html"))?.replace(
        "<a href=\"character.glb\" download>Download rigged GLB</a> · ",
        "",
    );
    std::fs::write(out.join("preview.html"), preview)?;
    Ok(avatar)
}
#[derive(Clone, Debug)]
pub struct SceneVmMaterial {
    pub id: Uuid,
    pub size: u32,
    pub rgba: Vec<u8>,
    pub packed_material: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct SceneVmCharacter {
    pub asset: CharacterAsset,
    pub materials: Vec<SceneVmMaterial>,
}
impl SceneVmCharacter {
    pub fn new(asset: &CharacterAsset) -> Result<Self> {
        let mut materials = Vec::new();
        for m in &asset.materials {
            let size = m.texture.as_ref().map_or(1, |t| t.size);
            let mut rgba = Vec::new();
            let mut packed = Vec::new();
            for i in 0..size * size {
                let mut c = m.color;
                let mut rough = 0.85;
                let mut metal = if m.channel == 10 { 0.3 } else { 0.0 };
                if let Some(t) = &m.texture {
                    let off = i as usize * 4;
                    for (j, channel) in c.iter_mut().enumerate().take(3) {
                        *channel *= linear_byte(t.rgba[off + j]);
                    }
                    rough = t.orm[off + 1] as f32 / 255.0;
                    metal = t.orm[off + 2] as f32 / 255.0;
                }
                rgba.extend([srgb_byte(c[0]), srgb_byte(c[1]), srgb_byte(c[2]), 255]);
                let rm = (rough * 15.0).round() as u8 | ((metal * 15.0).round() as u8) << 4;
                // 0xFE is SceneVM's semantic-material sentinel, not numeric R/M.
                packed.extend([if rm == 0xfe { 0xff } else { rm }, 0x0f, 128, 128]);
            }
            let id = stable_id(
                "eldiron/character-generator/material",
                &serde_json::to_vec(m)?,
            );
            materials.push(SceneVmMaterial {
                id,
                size,
                rgba,
                packed_material: packed,
            });
        }
        Ok(Self {
            asset: asset.clone(),
            materials,
        })
    }
    pub fn register_materials(&self, atlas: &SharedAtlas) {
        for m in &self.materials {
            atlas.add_tile(
                m.id,
                m.size,
                m.size,
                vec![m.rgba.clone()],
                vec![m.packed_material.clone()],
            );
        }
    }
    /// CPU skinning into SceneVM's existing per-frame dynamic mesh interface.
    /// Mesh positions are world-space; positive yaw rotates +Z toward +X.
    /// Submit each object with `Atom::AddDynamic` after clearing the prior frame's dynamics.
    pub fn objects(
        &self,
        id: GeoId,
        origin: [f32; 3],
        yaw: f32,
        motion: Motion,
        time: f32,
    ) -> Vec<DynamicObject> {
        let bind = self.asset.bind_pose();
        let pose = self.asset.world_pose(&self.asset.local_pose(motion, time));
        let (s, c) = yaw.sin_cos();
        let orient = |p: V3| [c * p[0] + s * p[2], p[1], -s * p[0] + c * p[2]];
        let mut groups: BTreeMap<usize, (Vec<DynamicMeshVertex>, Vec<u32>)> = BTreeMap::new();
        for primitive in &self.asset.primitives {
            let (vertices, indices) = groups.entry(primitive.material).or_default();
            let base = vertices.len() as u32;
            for vertex in &primitive.vertices {
                let (p, n) = self.asset.posed_vertex(vertex, &bind, &pose);
                let p = add(orient(p), origin);
                let n = orient(n);
                vertices.push(DynamicMeshVertex {
                    position: Vec3::new(p[0], p[1], p[2]),
                    normal: Vec3::new(n[0], n[1], n[2]),
                    uv: Vec2::new(vertex.uv[0], vertex.uv[1]),
                });
            }
            indices.extend(primitive.indices.iter().map(|i| i + base));
        }
        groups
            .into_iter()
            .map(|(material, (vertices, indices))| {
                let mut object =
                    DynamicObject::mesh(id, self.materials[material].id, vertices, indices);
                object.center = Vec3::new(origin[0], origin[1], origin[2]);
                object
            })
            .collect()
    }
}
