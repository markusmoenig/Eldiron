use crate::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use theframework::prelude::*;
pub enum PreRenderCmd {
    SetTextures(FxHashMap<Uuid, TheRGBATile>),
    SetMaterials(IndexMap<Uuid, MaterialFXObject>),
    SetPalette(ThePalette),
    SetPaused(bool),
    MaterialChanged(MaterialFXObject),
    RenderRegionCoordTree(Region),
    RenderRegion(Region, Option<Vec<Vec2i>>),
    Quit,
}

pub enum PreRenderResult {
    RenderedRegionTile(
        Uuid,
        Vec2i,
        Vec2i,
        u16,
        TheRGBBuffer,
        TheRGBBuffer,
        TheFlattenedMap<f32>,
        TheFlattenedMap<Vec<PreRenderedLight>>,
    ),
    RenderedRTree(Uuid, RTree<PreRenderedData>),
    Clear(Uuid),
    Progress(String),
    Finished,
    Paused,
    Quit,
}

#[derive(Debug)]
pub struct PreRenderThread {
    pub tx: Option<mpsc::Sender<PreRenderCmd>>,

    pub rx: Option<mpsc::Receiver<PreRenderResult>>,

    renderer_thread: Option<JoinHandle<()>>,
}

impl Default for PreRenderThread {
    fn default() -> Self {
        Self::new()
    }
}

impl PreRenderThread {
    pub fn new() -> Self {
        Self {
            tx: None,
            rx: None,
            renderer_thread: None,
        }
    }

    /// Check for a renderer result
    pub fn receive(&self) -> Option<PreRenderResult> {
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                return Some(result);
            }
        }

        None
    }

    /// Send a cmd to the renderer.
    pub fn send(&self, cmd: PreRenderCmd) {
        if let Some(tx) = &self.tx {
            tx.send(cmd).unwrap();
        }
    }

    pub fn set_textures(&self, textures: FxHashMap<Uuid, TheRGBATile>) {
        self.send(PreRenderCmd::SetTextures(textures));
    }

    pub fn set_materials(&self, materials: IndexMap<Uuid, MaterialFXObject>) {
        self.send(PreRenderCmd::SetMaterials(materials));
    }

    pub fn set_palette(&self, palette: ThePalette) {
        self.send(PreRenderCmd::SetPalette(palette));
    }

    pub fn set_paused(&self, paused: bool) {
        self.send(PreRenderCmd::SetPaused(paused));
    }

    pub fn material_changed(&self, material: MaterialFXObject) {
        self.send(PreRenderCmd::MaterialChanged(material));
    }

    pub fn render_region(&self, region: Region, tiles: Option<Vec<Vec2i>>) {
        self.send(PreRenderCmd::RenderRegion(region, tiles));
    }

    pub fn render_region_coord_tree(&self, region: Region) {
        self.send(PreRenderCmd::RenderRegionCoordTree(region));
    }

    pub fn startup(&mut self) {
        let (tx, rx) = mpsc::channel::<PreRenderCmd>();

        self.tx = Some(tx);

        let (result_tx, result_rx) = mpsc::channel::<PreRenderResult>();

        self.rx = Some(result_rx);

        let renderer_thread = thread::spawn(move || {
            // We allocate half of the available cpus to the background pool
            let cpus = num_cpus::get();
            let background_pool = ThreadPoolBuilder::new()
                .num_threads(cpus / 2)
                .build()
                .unwrap();

            let mut renderer = Renderer::new();
            let mut palette = ThePalette::default();
            let mut curr_region = Region::default();

            let mut draw_settings = RegionDrawSettings::new();
            draw_settings.daylight = vec3f(1.0, 1.0, 1.0);

            let mut prerendered_region_data: FxHashMap<Uuid, PreRendered> = FxHashMap::default();

            let mut in_progress = false;
            let mut exit_loop = false;
            let mut paused = false;

            loop {
                if exit_loop {
                    break;
                }
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        PreRenderCmd::SetTextures(new_textures) => {
                            println!("PreRenderCmd::SetTextures");
                            renderer.set_textures(new_textures.clone());
                        }
                        PreRenderCmd::SetMaterials(new_materials) => {
                            println!("PreRenderCmd::SetMaterials");
                            renderer.materials.clone_from(&new_materials);
                        }
                        PreRenderCmd::SetPalette(new_palette) => {
                            println!("PreRenderCmd::SetPalette");
                            palette = new_palette;
                        }
                        PreRenderCmd::SetPaused(p) => {
                            println!("PreRenderCmd::SetPaused ({})", p);
                            paused = p;
                            if paused {
                                result_tx.send(PreRenderResult::Paused).unwrap();
                            }
                        }
                        PreRenderCmd::MaterialChanged(changed_material) => {
                            println!("PreRenderCmd::MaterialChanged");
                            renderer
                                .materials
                                .insert(changed_material.id, changed_material);
                        }
                        PreRenderCmd::RenderRegionCoordTree(region) => {
                            println!("PreRenderCmd::RenderRegionCoordTree");

                            let w = (region.width as f32 * region.grid_size as f32) as i32;
                            let h = (region.height as f32 * region.grid_size as f32) as i32;

                            renderer.set_region(&region);
                            renderer.position =
                                vec3f(region.width as f32 / 2.0, 0.0, region.height as f32 / 2.0);

                            let mut tree: RTree<PreRenderedData> = RTree::new();

                            background_pool.install(|| {
                                tree = renderer.prerender_rtree(
                                    vec2i(w, h),
                                    &region,
                                    &mut draw_settings,
                                );
                            });

                            if let Some(data) = prerendered_region_data.get_mut(&region.id) {
                                data.tree = tree.clone();
                                data.tile_samples.clear()
                            } else {
                                let mut prerendered = PreRendered::zero();
                                prerendered.tree = tree.clone();
                                prerendered
                                    .tile_samples
                                    .clone_from(&region.prerendered.tile_samples);
                                prerendered_region_data.insert(region.id, prerendered);
                            }

                            result_tx
                                .send(PreRenderResult::RenderedRTree(region.id, tree))
                                .unwrap();

                            curr_region = region;
                            in_progress = true;

                            println!("finished");
                        }
                        PreRenderCmd::RenderRegion(region, tiles) => {
                            println!("PreRenderCmd::RenderRegion");

                            renderer.set_region(&region);
                            renderer.position =
                                vec3f(region.width as f32 / 2.0, 0.0, region.height as f32 / 2.0);
                            curr_region = region;

                            if let Some(pre) = prerendered_region_data.get_mut(&curr_region.id) {
                                if let Some(tiles) = tiles {
                                    pre.remove_tiles(tiles, curr_region.grid_size);
                                } else {
                                    pre.tile_samples.clear();
                                    result_tx
                                        .send(PreRenderResult::Clear(curr_region.id))
                                        .unwrap();
                                }
                            }

                            in_progress = true;
                        }
                        PreRenderCmd::Quit => {
                            println!("PreRenderCmd::Quit");
                            exit_loop = true;
                        }
                    }
                }

                // Rendering in progress ?

                if in_progress && !paused {
                    //let mut reset = false;

                    let w = (curr_region.width as f32 * curr_region.grid_size as f32) as i32;
                    let h = (curr_region.height as f32 * curr_region.grid_size as f32) as i32;

                    // if curr_region.prerendered.albedo.dim().width != w
                    //     || curr_region.prerendered.albedo.dim().height != h
                    // {
                    //     reset = true;
                    // }

                    // let mut prerendered = if reset {
                    //     PreRendered::zero()
                    // } else {
                    //     let prerendered =
                    //         if let Some(pre) = prerendered_region_data.get(&curr_region.id) {
                    //             pre.clone()
                    //         } else {
                    //             PreRendered::zero()
                    //             //curr_region.prerendered.clone()
                    //         };
                    //     prerendered
                    // };

                    if let Some(prerendered) = prerendered_region_data.get_mut(&curr_region.id) {
                        background_pool.install(|| {
                            in_progress = !renderer.prerender(
                                vec2i(w, h),
                                prerendered,
                                &curr_region,
                                &mut draw_settings,
                                &palette,
                                result_tx.clone(),
                            );
                            if !in_progress {
                                result_tx.send(PreRenderResult::Finished).unwrap();
                                println!("finished");
                            } else {
                                // Calculate progress text

                                let w = curr_region.width;
                                let h = curr_region.height;
                                let mut togo = 0;
                                for x in 0..w {
                                    for y in 0..h {
                                        let tile = Vec2i::new(x, y);
                                        if let Some(samples) = prerendered.tile_samples.get(&tile) {
                                            if (*samples as i32) < curr_region.pathtracer_samples {
                                                togo += 1;
                                            }
                                        } else {
                                            togo += 1;
                                        }
                                    }
                                }

                                let progress = if togo == 0 {
                                    str!("Finished")
                                } else {
                                    format!("{}", togo)
                                };
                                result_tx.send(PreRenderResult::Progress(progress)).unwrap();
                            }
                        });
                    }
                }
                std::thread::yield_now();
                //std::thread::sleep(std::time::Duration::from_millis(10));
            }

            println!("Renderer thread exiting")
        });
        self.renderer_thread = Some(renderer_thread);
    }
}
