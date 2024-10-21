use crate::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
pub enum PreRenderCmd {
    SetTextures(FxHashMap<Uuid, TheRGBATile>),
    SetMaterials(IndexMap<Uuid, MaterialFXObject>),
    SetPalette(ThePalette),
    SetPaused(bool),
    Restart,
    MaterialChanged(MaterialFXObject),
    RenderRegion(Region, Option<Vec<Vec2i>>),
    Quit,
}

#[allow(clippy::large_enum_variant)]
pub enum PreRenderResult {
    RenderedRegionTile(Uuid, Vec2i, u16, PreRenderedTileData),
    MaterialPreviewRendered(Uuid, TheRGBABuffer),
    ClearRegionTile(Uuid, Vec2i),
    Clear(Uuid),
    Progress(Uuid),
    UpdateMiniMap,
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

    pub fn restart(&self) {
        self.send(PreRenderCmd::Restart);
    }

    pub fn startup(&mut self) {
        let (tx, rx) = mpsc::channel::<PreRenderCmd>();

        self.tx = Some(tx);

        let (result_tx, result_rx) = mpsc::channel::<PreRenderResult>();

        self.rx = Some(result_rx);

        let renderer_thread = thread::spawn(move || {
            // We allocate all but 2 cpus to the background pool
            let mut cpus = num_cpus::get();
            if cpus > 2 {
                cpus -= 2;
            } else {
                cpus = 1;
            }
            let background_pool = ThreadPoolBuilder::new().num_threads(cpus).build().unwrap();

            let mut renderer = Renderer::new();
            let mut palette = ThePalette::default();
            let mut curr_region = Region::default();

            let mut draw_settings = RegionDrawSettings::new();
            draw_settings.daylight = vec3f(1.0, 1.0, 1.0);

            let mut prerendered_region_data: FxHashMap<Uuid, PreRendered> = FxHashMap::default();

            let mut in_progress = false;
            let mut exit_loop = false;
            let mut paused = false;

            let mut material_preview: Option<MaterialFXObject> = None;
            let mut material_preview_buffer = TheRGBABuffer::new(TheDim::sized(160, 160));
            let mut material_preview_passes = 0;

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
                        PreRenderCmd::Restart => {
                            if let Some(data) = prerendered_region_data.get_mut(&curr_region.id) {
                                data.tile_samples.clear();
                                in_progress = true;
                            }
                        }
                        PreRenderCmd::MaterialChanged(changed_material) => {
                            println!("PreRenderCmd::MaterialChanged");
                            renderer
                                .materials
                                .insert(changed_material.id, changed_material.clone());
                            material_preview = Some(changed_material);
                            material_preview_passes = 0;
                        }
                        PreRenderCmd::RenderRegion(region, tiles) => {
                            println!(
                                "PreRenderCmd::RenderRegion {}",
                                if let Some(tiles) = &tiles {
                                    tiles.len().to_string()
                                } else {
                                    "None".to_string()
                                }
                            );

                            result_tx.send(PreRenderResult::UpdateMiniMap).unwrap();

                            renderer.set_region(&region);
                            renderer.position =
                                vec3f(region.width as f32 / 2.0, 0.0, region.height as f32 / 2.0);
                            curr_region = region;

                            prerendered_region_data
                                .entry(curr_region.id)
                                .or_insert_with(|| curr_region.prerendered.clone());

                            if let Some(pre) = prerendered_region_data.get_mut(&curr_region.id) {
                                if let Some(tiles) = tiles {
                                    pre.remove_tiles(&tiles);
                                } else {
                                    result_tx
                                        .send(PreRenderResult::Clear(curr_region.id))
                                        .unwrap();
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

                // Material Preview in Progress ?
                if let Some(material) = &mut material_preview {
                    material.render_preview_3d(
                        &palette,
                        &renderer.textures,
                        &mut material_preview_buffer,
                        material_preview_passes,
                    );
                    material_preview_passes += 1;

                    if material_preview_passes % 10 == 0 {
                        result_tx
                            .send(PreRenderResult::MaterialPreviewRendered(
                                material.id,
                                material_preview_buffer.clone(),
                            ))
                            .unwrap();
                    }

                    if material_preview_passes == 300 {
                        println!("Material finished");
                        material_preview = None;
                    }
                }

                // Rendering in progress ?
                if in_progress && !paused {
                    let w = curr_region.width * curr_region.tile_size;
                    let h = curr_region.height * curr_region.tile_size;

                    if let Some(prerendered) = prerendered_region_data.get_mut(&curr_region.id) {
                        background_pool.install(|| {
                            in_progress = renderer.prerender(
                                vec2i(w, h),
                                prerendered,
                                &curr_region,
                                &mut draw_settings,
                                &palette,
                                result_tx.clone(),
                            );
                            if !in_progress {
                                println!("finished");
                                result_tx.send(PreRenderResult::Finished).unwrap();
                            } else {
                                result_tx
                                    .send(PreRenderResult::Progress(curr_region.id))
                                    .unwrap();
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
