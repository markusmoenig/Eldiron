use crate::*;
use serde_json::{Value, json};
use std::path::Path;

#[derive(Default)]
struct Buffer {
    bytes: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}
impl Buffer {
    fn accessor(
        &mut self,
        data: Vec<u8>,
        component: u32,
        kind: &str,
        count: usize,
        bounds: Option<(Vec<f32>, Vec<f32>)>,
    ) -> usize {
        while !self.bytes.len().is_multiple_of(4) {
            self.bytes.push(0);
        }
        let offset = self.bytes.len();
        let len = data.len();
        self.bytes.extend(data);
        let view = self.views.len();
        self.views
            .push(json!({"buffer":0,"byteOffset":offset,"byteLength":len}));
        let mut accessor =
            json!({"bufferView":view,"componentType":component,"count":count,"type":kind});
        if let Some((min, max)) = bounds {
            accessor["min"] = json!(min);
            accessor["max"] = json!(max);
        }
        let id = self.accessors.len();
        self.accessors.push(accessor);
        id
    }
    fn floats(
        &mut self,
        data: &[f32],
        kind: &str,
        width: usize,
        bounds: Option<(Vec<f32>, Vec<f32>)>,
    ) -> usize {
        self.accessor(
            data.iter().flat_map(|v| v.to_le_bytes()).collect(),
            5126,
            kind,
            data.len() / width,
            bounds,
        )
    }
}
/// Standard glTF 2.0 binary container with skinning, sockets and procedural clips with loop metadata.
pub fn export_glb(asset: &CharacterAsset, path: impl AsRef<Path>) -> Result<()> {
    let mut b = Buffer::default();
    let mut primitives = Vec::new();
    for p in &asset.primitives {
        let positions: Vec<f32> = p.vertices.iter().flat_map(|v| v.position).collect();
        let min: Vec<f32> = (0..3)
            .map(|i| {
                p.vertices
                    .iter()
                    .map(|v| v.position[i])
                    .fold(f32::INFINITY, f32::min)
            })
            .collect();
        let max: Vec<f32> = (0..3)
            .map(|i| {
                p.vertices
                    .iter()
                    .map(|v| v.position[i])
                    .fold(f32::NEG_INFINITY, f32::max)
            })
            .collect();
        let pos = b.floats(&positions, "VEC3", 3, Some((min, max)));
        let normals = b.floats(
            &p.vertices.iter().flat_map(|v| v.normal).collect::<Vec<_>>(),
            "VEC3",
            3,
            None,
        );
        let joints = b.accessor(
            p.vertices
                .iter()
                .flat_map(|v| v.joints)
                .flat_map(|v| v.to_le_bytes())
                .collect(),
            5123,
            "VEC4",
            p.vertices.len(),
            None,
        );
        let weights = b.floats(
            &p.vertices
                .iter()
                .flat_map(|v| v.weights)
                .collect::<Vec<_>>(),
            "VEC4",
            4,
            None,
        );
        let uv = asset.materials[p.material].texture.as_ref().map(|_| {
            b.floats(
                &p.vertices.iter().flat_map(|v| v.uv).collect::<Vec<_>>(),
                "VEC2",
                2,
                None,
            )
        });
        let indices = b.accessor(
            p.indices.iter().flat_map(|v| v.to_le_bytes()).collect(),
            5125,
            "SCALAR",
            p.indices.len(),
            None,
        );
        for accessor in [pos, normals, joints, weights, indices]
            .into_iter()
            .chain(uv)
        {
            let view = b.accessors[accessor]["bufferView"].as_u64().unwrap() as usize;
            b.views[view]["target"] = json!(if accessor == indices { 34963 } else { 34962 });
        }
        let mut primitive = json!({"attributes":{"POSITION":pos,"NORMAL":normals,"JOINTS_0":joints,"WEIGHTS_0":weights},"indices":indices,"material":p.material,"extras":{"part":p.name}});
        if let Some(uv) = uv {
            primitive["attributes"]["TEXCOORD_0"] = json!(uv);
        }
        primitives.push(primitive);
    }
    let bind = asset.bind_pose();
    let mut matrices = Vec::new();
    for t in &bind {
        matrices.extend([
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            -t.translation[0],
            -t.translation[1],
            -t.translation[2],
            1.0,
        ]);
    }
    let inverse = b.floats(&matrices, "MAT4", 16, None);
    let mut nodes: Vec<Value> = asset
        .joints
        .iter()
        .enumerate()
        .map(|(i, j)| {
            let children: Vec<usize> = asset
                .joints
                .iter()
                .enumerate()
                .filter(|(_, c)| c.parent == Some(i))
                .map(|(k, _)| k)
                .collect();
            let mut node = json!({"name":j.name,"translation":j.translation});
            if !children.is_empty() {
                node["children"] = json!(children);
            }
            node
        })
        .collect();
    for s in &asset.sockets {
        let id = nodes.len();
        nodes.push(json!({"name":s.name,"translation":s.translation,"extras":{"attachment":true}}));
        if nodes[s.joint].get("children").is_none() {
            nodes[s.joint]["children"] = json!([]);
        }
        nodes[s.joint]["children"]
            .as_array_mut()
            .unwrap()
            .push(json!(id));
    }
    let mesh_node = nodes.len();
    nodes.push(json!({"name":asset.name,"mesh":0,"skin":0}));
    let mut animations = Vec::new();
    for &motion in &asset.motions {
        let times: Vec<f32> = (0..=32)
            .map(|i| i as f32 / 32.0 * motion.duration())
            .collect();
        let input = b.floats(
            &times,
            "SCALAR",
            1,
            Some((vec![0.0], vec![motion.duration()])),
        );
        let poses: Vec<_> = times.iter().map(|&t| asset.local_pose(motion, t)).collect();
        let mut samplers = Vec::new();
        let mut channels = Vec::new();
        for j in 0..asset.joints.len() {
            let rotations: Vec<f32> = poses.iter().flat_map(|p| p[j].rotation).collect();
            let output = b.floats(&rotations, "VEC4", 4, None);
            let sampler = samplers.len();
            samplers.push(json!({"input":input,"output":output,"interpolation":"LINEAR"}));
            channels.push(json!({"sampler":sampler,"target":{"node":j,"path":"rotation"}}));
            if asset.joints[j].parent.is_none() {
                let translations: Vec<f32> = poses.iter().flat_map(|p| p[j].translation).collect();
                let output = b.floats(&translations, "VEC3", 3, None);
                let sampler = samplers.len();
                samplers.push(json!({"input":input,"output":output,"interpolation":"LINEAR"}));
                channels.push(json!({"sampler":sampler,"target":{"node":j,"path":"translation"}}));
            }
        }
        animations.push(json!({"name":motion.name(),"samplers":samplers,"channels":channels,"extras":{"loop":motion.looping()}}));
    }
    let mut roots: Vec<usize> = asset
        .joints
        .iter()
        .enumerate()
        .filter(|(_, j)| j.parent.is_none())
        .map(|(i, _)| i)
        .collect();
    roots.push(mesh_node);
    let mut textures = Vec::new();
    let mut images = Vec::new();
    let mut materials = Vec::new();
    for m in &asset.materials {
        let mut material = json!({"name":m.name,"pbrMetallicRoughness":{"baseColorFactor":m.color,"metallicFactor":if m.channel==10 {0.3} else {0.0},"roughnessFactor":0.85},"extras":{"semantic_channel":m.channel}});
        if let Some(texture) = &m.texture {
            for (field, data) in [
                ("baseColorTexture", &texture.rgba),
                ("metallicRoughnessTexture", &texture.orm),
            ] {
                let mut png = std::io::Cursor::new(Vec::new());
                image::RgbaImage::from_raw(texture.size, texture.size, data.clone())
                    .ok_or("invalid material image")?
                    .write_to(&mut png, image::ImageFormat::Png)?;
                while !b.bytes.len().is_multiple_of(4) {
                    b.bytes.push(0);
                }
                let view = b.views.len();
                let offset = b.bytes.len();
                let data = png.into_inner();
                b.views
                    .push(json!({"buffer":0,"byteOffset":offset,"byteLength":data.len()}));
                b.bytes.extend(data);
                let image = images.len();
                images.push(json!({"bufferView":view,"mimeType":"image/png"}));
                let index = textures.len();
                textures.push(json!({"sampler":0,"source":image}));
                material["pbrMetallicRoughness"][field] = json!({"index":index});
            }
            material["pbrMetallicRoughness"]["roughnessFactor"] = json!(1.0);
            material["pbrMetallicRoughness"]["metallicFactor"] = json!(1.0);
        }
        materials.push(material);
    }
    let mut doc = json!({
        "asset":{"version":"2.0","generator":"eldiron-character-generator prototype"},
        "scene":0,"scenes":[{"nodes":roots}],"nodes":nodes,"extras":{"character_style":asset.style},
        "meshes":[{"name":asset.name,"primitives":primitives}],
        "skins":[{"inverseBindMatrices":inverse,"joints":(0..asset.joints.len()).collect::<Vec<_>>(),"skeleton":0}],
        "materials":materials,
        "animations":animations,"buffers":[{"byteLength":b.bytes.len()}],"bufferViews":b.views,"accessors":b.accessors
    });
    if !textures.is_empty() {
        doc["textures"] = json!(textures);
        doc["images"] = json!(images);
        doc["samplers"] = json!([{"magFilter":9728,"minFilter":9728,"wrapS":33071,"wrapT":33071}]);
    }
    let mut json = serde_json::to_vec(&doc)?;
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    while !b.bytes.len().is_multiple_of(4) {
        b.bytes.push(0);
    }
    let total = 12 + 8 + json.len() + 8 + b.bytes.len();
    let mut bytes = Vec::with_capacity(total);
    bytes.extend(b"glTF");
    bytes.extend(2u32.to_le_bytes());
    bytes.extend((total as u32).to_le_bytes());
    bytes.extend((json.len() as u32).to_le_bytes());
    bytes.extend(0x4e4f534au32.to_le_bytes());
    bytes.extend(json);
    bytes.extend((b.bytes.len() as u32).to_le_bytes());
    bytes.extend(0x004e4942u32.to_le_bytes());
    bytes.extend(b.bytes);
    std::fs::write(path, bytes)?;
    Ok(())
}
