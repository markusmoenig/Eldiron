use theframework::theui::TheRGBABuffer;

use crate::prelude::*;
//use rand::Rng;
use rayon::prelude::*;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use theframework::prelude::*;

pub enum RenderCmd {
    SetRegion(Region),
    SetPalette(ThePalette),
    Quit,
}

pub enum RenderResult {
    Bitmap(TheRGBABuffer),
    Rendered(RenderedMap),
    Quit,
}

#[derive(Debug)]
pub struct RenderThread {
    pub tx: Option<mpsc::Sender<RenderCmd>>,

    pub rx: Option<mpsc::Receiver<RenderResult>>,

    renderer_thread: Option<JoinHandle<()>>,
}

impl Default for RenderThread {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderThread {
    pub fn new() -> Self {
        Self {
            tx: None,
            rx: None,
            renderer_thread: None,
        }
    }

    /// Check for a renderer result
    pub fn receive(&self) -> Option<RenderResult> {
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                return Some(result);
            }
        }

        None
    }

    /// Send a cmd to the renderer.
    pub fn send(&self, cmd: RenderCmd) {
        if let Some(tx) = &self.tx {
            tx.send(cmd).unwrap();
        }
    }

    pub fn set_region(&self, region: Region) {
        self.send(RenderCmd::SetRegion(region));
    }

    pub fn set_palette(&self, palette: ThePalette) {
        self.send(RenderCmd::SetPalette(palette));
    }

    pub fn startup(&mut self) {
        let (tx, rx) = mpsc::channel::<RenderCmd>();

        self.tx = Some(tx);

        let (result_tx, result_rx) = mpsc::channel::<RenderResult>();

        self.rx = Some(result_rx);

        let renderer_thread = thread::spawn(move || {
            let mut region = Region::default();
            let mut map = RenderedMap::default();
            let mut palette = ThePalette::default();
            let tile_size = 24;

            loop {
                if let Ok(cmd) = rx.recv() {
                    match cmd {
                        RenderCmd::SetRegion(r) => {
                            println!("RenderCmd::Set Region");
                            region = r;
                            map = RenderedMap::new(
                                region.id,
                                region.width * tile_size,
                                region.height * tile_size,
                            );
                            render_region(&mut map, &region, tile_size, &palette);
                            result_tx.send(RenderResult::Rendered(map.clone())).unwrap();
                            //rgba.to_clipboard();
                        }
                        RenderCmd::SetPalette(p) => {
                            println!("RenderCmd::SetPalette");
                            palette = p;
                        }
                        RenderCmd::Quit => {
                            println!("RenderCmd::Quit");
                            break;
                        }
                    }
                }
            }

            println!("Renderer thread exiting")
        });
        self.renderer_thread = Some(renderer_thread);
    }
}

fn render_region(buffer: &mut RenderedMap, region: &Region, tile_size: i32, palette: &ThePalette) {
    let _start = get_time();

    let width = buffer.width as usize;
    let width_f = width as f32;
    let height_f = buffer.height as f32;

    let settings = RegionDrawSettings::new();

    let mut tilted_iso_alignment = 0;
    if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
        str!("Camera"),
        str!("Tilted Iso Alignment"),
        &settings.time,
        TheInterpolation::Switch,
    ) {
        tilted_iso_alignment = value;
    }

    let mut max_render_distance = 10;
    if let Some(v) = region.regionfx.get(
        str!("Distance / Fog"),
        str!("Maximum Render Distance"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_i32() {
            max_render_distance = value;
        }
    }

    let mut render_settings = RegionDrawSettings::new();
    let position = vec3f(0.5, 0.0, 79.5);
    let (ro, rd, fov, camera_mode, camera_type) =
        create_camera_setup(position, region, &mut render_settings);

    let tile_size_f = tile_size as f32;

    buffer
        .map
        .par_rchunks_exact_mut(width * 4)
        .enumerate()
        .for_each(|(j, line)| {
            for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                let i = j * width + i;

                let xx = (i % width) as f32;
                let yy = (i / width) as f32;

                let tx = xx / tile_size_f;
                let ty = yy / tile_size_f;

                let uv = vec2f(frac(tx), frac(ty));

                let mut ray = if camera_type == CameraType::TiltedIso {
                    let p = vec3f(tx, 0.0, -ty);
                    let camera = Camera::new(ro + p, rd + p, fov);
                    camera.create_tilted_isometric_ray(
                        uv,
                        vec2f(tile_size_f, tile_size_f),
                        vec2f(1.0, 1.0),
                        tilted_iso_alignment,
                    )
                } else if camera_mode == CameraMode::Pinhole {
                    let p = vec3f(tx, 0.0, -ty);
                    let camera = Camera::new(ro + p, rd + p, fov);
                    camera.create_ray(uv, vec2f(width_f, height_f), vec2f(1.0, 1.0))
                } else {
                    let iso_x = (tx - ty) * 1.0;
                    let iso_y = (tx + ty) * 1.0;
                    let p = vec3f(iso_x, 0.0, -iso_y);
                    let camera = Camera::new(ro + p, rd + p, fov);
                    camera.create_ortho_ray(uv, vec2f(tile_size_f, tile_size_f), vec2f(1.0, 1.0))
                };

                if yy as i32 == 0 && (xx as i32) < 24 {
                    println!("xx {} ray.o {:?}", xx, ray.o);
                }

                // In top down view, intersect ray with plane at 1.1 y
                // to speed up the ray / voxel casting
                if camera_type != CameraType::FirstPerson {
                    let plane_normal = vec3f(0.0, 1.0, 0.0);
                    let denom = dot(plane_normal, ray.d);

                    if denom.abs() > 0.0001 {
                        let t = dot(vec3f(0.0, 1.1, 0.0) - ray.o, plane_normal) / denom;
                        if t >= 0.0 {
                            ray.o += ray.d * t;
                        }
                    }
                }

                let mut rendered = Rendered::default();
                let color = render_pixel(ray, region, max_render_distance, palette);

                pixel.copy_from_slice(&[rendered]);

                /*
                pixel.copy_from_slice(&self.render_pixel(
                    ray,
                    region,
                    update,
                    settings,
                    camera_type,
                    &level,
                    &saturation,
                    max_render_distance,
                    palette,
                    ));*/
            }
        });

    let _stop = get_time();
    println!("region render time {:?}", _stop - _start);
}

fn render_pixel(ray: Ray, region: &Region, max_render_distance: i32, palette: &ThePalette) -> RGBA {
    let mut color = vec4f(0.0, 0.0, 0.0, 1.0);

    fn equal(l: f32, r: Vec3f) -> Vec3f {
        vec3f(
            if l == r.x { 1.0 } else { 0.0 },
            if l == r.y { 1.0 } else { 0.0 },
            if l == r.z { 1.0 } else { 0.0 },
        )
    }

    let ro = ray.o;
    let rd = ray.d;

    let mut i = floor(ro);
    let mut dist = 0.0;

    let mut normal = Vec3f::zero();
    let srd = signum(rd);

    let rdi = 1.0 / (2.0 * rd);

    let mut key: Vec3<i32>;
    let mut hit = false;

    for _ii in 0..max_render_distance {
        key = Vec3i::from(i);

        if key.y < -1 {
            break;
        }

        if let Some(model) = region.models.get(&(key.x, key.y, key.z)) {
            //println!("yo");
            let mut lro = ray.at(dist);
            lro -= Vec3f::from(key);
            lro -= rd * 0.01;

            let mut r = ray.clone();
            r.o = lro;

            if let Some(hit_struct) = model.render(&r, 1.01, i, palette) {
                color = hit_struct.color;
                hit = true;
                //normal = hit_struct.normal;
                dist += hit_struct.distance;
                break;
            }
        }

        let plain = (1.0 + srd - 2.0 * (ro - i)) * rdi;
        dist = min(plain.x, min(plain.y, plain.z));
        normal = equal(dist, plain) * srd;
        i += normal;
    }

    TheColor::from_vec4f(color).to_u8_array()
}
