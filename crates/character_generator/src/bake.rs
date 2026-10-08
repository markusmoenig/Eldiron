use crate::{math::*, *};
use image::{GrayImage, Luma, Rgba, RgbaImage};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Copy, Debug)]
pub struct BakeOptions {
    pub size: u32,
    pub frames: u32,
}
impl Default for BakeOptions {
    fn default() -> Self {
        Self {
            size: 96,
            frames: 8,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct AtlasFrame {
    pub motion: String,
    pub direction: String,
    pub frame: u32,
    pub time: f32,
    pub rect: [u32; 4],
    pub ground: [f32; 2],
    pub sockets: BTreeMap<String, [f32; 2]>,
}
#[derive(Debug, Serialize)]
pub struct AtlasMetadata {
    pub version: u32,
    pub character: String,
    pub style: ResolvedStyle,
    pub image: String,
    pub channels_image: String,
    pub size: [u32; 2],
    pub frame_size: u32,
    pub pixels_per_meter: f32,
    pub channel_ids: BTreeMap<String, u8>,
    pub durations: BTreeMap<String, f32>,
    pub looping: BTreeMap<String, bool>,
    pub frames: Vec<AtlasFrame>,
}
const DIRECTIONS: [&str; 8] = [
    "front",
    "front_right",
    "right",
    "back_right",
    "back",
    "back_left",
    "left",
    "front_left",
];
fn view(p: V3, direction: usize) -> V3 {
    let a = direction as f32 * std::f32::consts::FRAC_PI_4;
    let (s, c) = a.sin_cos();
    let (sp, cp) = 10.0f32.to_radians().sin_cos();
    let forward = -p[0] * s + p[2] * c;
    [
        p[0] * c + p[2] * s,
        p[1] * cp - forward * sp,
        forward * cp + p[1] * sp,
    ]
}
fn project(p: V3, d: usize, scale: f32, center: [f32; 2], size: u32) -> V3 {
    let v = view(p, d);
    [
        size as f32 * 0.5 + (v[0] - center[0]) * scale,
        size as f32 * 0.5 - (v[1] - center[1]) * scale,
        v[2],
    ]
}
fn edge(a: V3, b: V3, x: f32, y: f32) -> f32 {
    (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
}
/// CPU orthographic baker. Every frame uses the same framing and world scale.
pub fn bake_atlas(
    asset: &CharacterAsset,
    options: BakeOptions,
    out: impl AsRef<Path>,
) -> Result<AtlasMetadata> {
    if !(16..=512).contains(&options.size) || !(2..=32).contains(&options.frames) {
        return Err("bake size must be 16..512 and frames 2..32".into());
    }
    if asset.motions.is_empty() {
        return Err("asset has no motions".into());
    }
    let size = options.size;
    let width = size * options.frames;
    let height = size * 8 * asset.motions.len() as u32;
    // Bound memory before allocating large atlases (color + mask).
    if u64::from(width) * u64::from(height) > 32_000_000 {
        return Err("atlas exceeds 32 million pixels; reduce size or frames".into());
    }
    let bind = asset.bind_pose();
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for &motion in &asset.motions {
        for frame in 0..options.frames {
            let pose = asset
                .world_pose(&asset.local_pose(motion, motion.frame_time(frame, options.frames)));
            for d in 0..8 {
                for p in &asset.primitives {
                    for v in &p.vertices {
                        let xyz = view(asset.posed_vertex(v, &bind, &pose).0, d);
                        for k in 0..2 {
                            lo[k] = lo[k].min(xyz[k]);
                            hi[k] = hi[k].max(xyz[k]);
                        }
                    }
                }
            }
        }
    }
    let center = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5];
    let scale = size as f32 * 0.86 / (hi[0] - lo[0]).max(hi[1] - lo[1]);
    let mut atlas = RgbaImage::new(width, height);
    let mut masks = GrayImage::new(width, height);
    let mut frames = Vec::new();
    let light = unit([-0.4, 0.8, 0.6]);
    for (motion_index, &motion) in asset.motions.iter().enumerate() {
        for (d, direction) in DIRECTIONS.iter().enumerate() {
            for frame in 0..options.frames {
                let time = motion.frame_time(frame, options.frames);
                let pose = asset.world_pose(&asset.local_pose(motion, time));
                let ox = frame * size;
                let oy = (motion_index as u32 * 8 + d as u32) * size;
                let mut depth = vec![f32::NEG_INFINITY; (size * size) as usize];
                for p in &asset.primitives {
                    let material = &asset.materials[p.material];
                    let mut vertices: Vec<_> = p
                        .vertices
                        .iter()
                        .map(|v| {
                            let (xyz, n) = asset.posed_vertex(v, &bind, &pose);
                            (project(xyz, d, scale, center, size), n, v.uv)
                        })
                        .collect();
                    // Preserve a thin neutral lip at sprite scale. This changes
                    // only projected coverage; the exported 3D mark stays small.
                    if p.name == "mouth" {
                        let low = vertices
                            .iter()
                            .map(|v| v.0[1])
                            .fold(f32::INFINITY, f32::min);
                        let high = vertices
                            .iter()
                            .map(|v| v.0[1])
                            .fold(f32::NEG_INFINITY, f32::max);
                        let span = high - low;
                        if span > 1e-5 && span < 1.5 {
                            let center = (low + high) * 0.5;
                            for v in &mut vertices {
                                v.0[1] = center + (v.0[1] - center) * 1.5 / span;
                                // Expanded decal coverage needs matching depth bias
                                // to stay above the adjacent cheek samples.
                                v.0[2] += 0.75 / scale;
                            }
                        }
                    }
                    for indices in p.indices.as_chunks::<3>().0 {
                        let [(a, na, ua), (b, nb, ub), (c, nc, uc)] =
                            indices.map(|i| vertices[i as usize]);
                        let area = edge(a, b, c[0], c[1]);
                        if area.abs() < 1e-5 {
                            continue;
                        }
                        let xmin = a[0].min(b[0]).min(c[0]).floor().max(0.0) as u32;
                        let xmax = a[0].max(b[0]).max(c[0]).ceil().min(size as f32 - 1.0) as u32;
                        let ymin = a[1].min(b[1]).min(c[1]).floor().max(0.0) as u32;
                        let ymax = a[1].max(b[1]).max(c[1]).ceil().min(size as f32 - 1.0) as u32;
                        for y in ymin..=ymax {
                            for x in xmin..=xmax {
                                let px = x as f32 + 0.5;
                                let py = y as f32 + 0.5;
                                let wa = edge(b, c, px, py) / area;
                                let wb = edge(c, a, px, py) / area;
                                let wc = 1.0 - wa - wb;
                                if wa < -1e-5 || wb < -1e-5 || wc < -1e-5 {
                                    continue;
                                }
                                let z = wa * a[2] + wb * b[2] + wc * c[2];
                                let index = (y * size + x) as usize;
                                if z <= depth[index] {
                                    continue;
                                }
                                depth[index] = z;
                                let normal = unit(add(add(mul(na, wa), mul(nb, wb)), mul(nc, wc)));
                                let shade = 0.62 + 0.38 * dot(normal, light).max(0.0);
                                let uv =
                                    std::array::from_fn(|i| wa * ua[i] + wb * ub[i] + wc * uc[i]);
                                let color = material.sample_color(uv);
                                atlas.put_pixel(
                                    ox + x,
                                    oy + y,
                                    Rgba([
                                        crate::materials::srgb_byte(color[0] * shade),
                                        crate::materials::srgb_byte(color[1] * shade),
                                        crate::materials::srgb_byte(color[2] * shade),
                                        255,
                                    ]),
                                );
                                masks.put_pixel(ox + x, oy + y, Luma([material.channel]));
                            }
                        }
                    }
                }
                let ground = project([0.0, 0.0, 0.0], d, scale, center, size);
                let sockets = asset
                    .sockets
                    .iter()
                    .map(|s| {
                        let p = project(
                            add(
                                pose[s.joint].translation,
                                rotate(pose[s.joint].rotation, s.translation),
                            ),
                            d,
                            scale,
                            center,
                            size,
                        );
                        (s.name.clone(), [p[0], p[1]])
                    })
                    .collect();
                frames.push(AtlasFrame {
                    motion: motion.name().into(),
                    direction: (*direction).into(),
                    frame,
                    time,
                    rect: [ox, oy, size, size],
                    ground: [ground[0], ground[1]],
                    sockets,
                });
            }
        }
    }
    let metadata = AtlasMetadata {
        version: 1,
        character: asset.name.clone(),
        style: asset.style,
        image: "atlas.png".into(),
        channels_image: "channels.png".into(),
        size: [width, height],
        frame_size: size,
        pixels_per_meter: scale,
        channel_ids: [
            ("background", 0),
            ("skin", 1),
            ("torso", 3),
            ("legs", 5),
            ("hair", 6),
            ("eyes", 7),
            ("mouth", 8),
            ("feet", 9),
            ("equipment", 10),
            ("helmet", 11),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v))
        .collect(),
        durations: asset
            .motions
            .iter()
            .map(|m| (m.name().into(), m.duration()))
            .collect(),
        looping: asset
            .motions
            .iter()
            .map(|m| (m.name().into(), m.looping()))
            .collect(),
        frames,
    };
    let out = out.as_ref();
    std::fs::create_dir_all(out)?;
    atlas.save(out.join("atlas.png"))?;
    masks.save(out.join("channels.png"))?;
    std::fs::write(
        out.join("atlas.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    let preview = include_str!("preview.html").replace(
        "__ATLAS_METADATA__",
        &serde_json::to_string(&metadata)?.replace('<', "\\u003c"),
    );
    std::fs::write(out.join("preview.html"), preview)?;
    Ok(metadata)
}
