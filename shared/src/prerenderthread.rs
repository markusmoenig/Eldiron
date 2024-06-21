use crate::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use theframework::prelude::*;
pub enum PreRenderCmd {
    SetTextures(FxHashMap<Uuid, TheRGBATile>),
    SetMaterials(IndexMap<Uuid, MaterialFXObject>),
    SetPalette(ThePalette),
    MaterialChanged(MaterialFXObject),
    RenderRegionCoordTree(Region),
    RenderRegion(Region, Option<Vec<Vec2i>>),
    Quit,
}

pub enum PreRenderResult {
    RenderedRegion(Uuid, PreRendered),
    RenderedRegionTile(
        Uuid,
        Vec2i,
        Vec2i,
        TheRGBBuffer,
        TheRGBBuffer,
        TheFlattenedMap<f32>,
        TheFlattenedMap<Vec<PreRenderedLight>>,
    ),
    RenderedRTree(Uuid, RTree<PreRenderedData>),
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

            let mut prerendered_regions: FxHashMap<Uuid, PreRendered> = FxHashMap::default();

            let mut in_progress = false;

            loop {
                if let Ok(cmd) = rx.try_recv() {
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

                            let mut prerendered = PreRendered::new(
                                TheRGBBuffer::new(TheDim::sized(w, h)),
                                TheRGBBuffer::new(TheDim::sized(w, h)),
                            );

                            background_pool.install(|| {
                                renderer.prerender_rtree(
                                    &mut prerendered,
                                    &region,
                                    &mut draw_settings,
                                );
                            });

                            prerendered_regions.insert(region.id, prerendered.clone());

                            result_tx
                                .send(PreRenderResult::RenderedRTree(region.id, prerendered.tree))
                                .unwrap();

                            curr_region = region;

                            println!("finished");
                        }
                        PreRenderCmd::RenderRegion(region, tiles) => {
                            println!("PreRenderCmd::RenderRegion");

                            renderer.set_region(&region);
                            renderer.position =
                                vec3f(region.width as f32 / 2.0, 0.0, region.height as f32 / 2.0);
                            curr_region = region;

                            if let Some(pre) = prerendered_regions.get_mut(&curr_region.id) {
                                if let Some(tiles) = tiles {
                                    pre.remove_tiles(tiles, curr_region.grid_size);
                                } else {
                                    pre.tile_samples.clear();
                                }
                            }

                            in_progress = true;
                        }
                        PreRenderCmd::Quit => {
                            println!("PreRenderCmd::Quit");
                            break;
                        }
                    }
                }

                // Rendering in progress ?

                if in_progress {
                    let mut reset = false;

                    let w = (curr_region.width as f32 * curr_region.grid_size as f32) as i32;
                    let h = (curr_region.height as f32 * curr_region.grid_size as f32) as i32;

                    if curr_region.prerendered.albedo.dim().width != w
                        || curr_region.prerendered.albedo.dim().height != h
                    {
                        reset = true;
                    }

                    let mut prerendered = if reset {
                        let mut prerendered = PreRendered::new(
                            TheRGBBuffer::new(TheDim::sized(w, h)),
                            TheRGBBuffer::new(TheDim::sized(w, h)),
                        );
                        if let Some(pre) = prerendered_regions.get(&curr_region.id) {
                            prerendered.tree = pre.tree.clone();
                        }

                        prerendered
                    } else {
                        let prerendered =
                            if let Some(pre) = prerendered_regions.get(&curr_region.id) {
                                pre.clone()
                            } else {
                                curr_region.prerendered.clone()
                            };
                        prerendered
                    };

                    background_pool.install(|| {
                        in_progress = !renderer.prerender(
                            &mut prerendered,
                            &curr_region,
                            &mut draw_settings,
                            &palette,
                            result_tx.clone(),
                        );
                    });

                    prerendered_regions.insert(curr_region.id, prerendered.clone());
                }
                std::thread::yield_now();
                //std::thread::sleep(std::time::Duration::from_millis(10));
            }

            println!("Renderer thread exiting")
        });
        self.renderer_thread = Some(renderer_thread);
    }
}
