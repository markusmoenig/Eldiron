use crate::prelude::*;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use theframework::prelude::*;

pub enum PreRenderCmd {
    SetTextures(FxHashMap<Uuid, TheRGBATile>),
    RenderRegion(Region, ThePalette),
    Quit,
}

pub enum PreRenderResult {
    RenderedRegion(Uuid, PreRendered),
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

    pub fn render_region(&self, region: Region, palette: ThePalette) {
        self.send(PreRenderCmd::RenderRegion(region, palette));
    }

    pub fn startup(&mut self) {
        let (tx, rx) = mpsc::channel::<PreRenderCmd>();

        self.tx = Some(tx);

        let (result_tx, result_rx) = mpsc::channel::<PreRenderResult>();

        self.rx = Some(result_rx);

        let renderer_thread = thread::spawn(move || {
            let mut renderer = Renderer::new();
            let mut draw_settings = RegionDrawSettings::new();
            draw_settings.daylight = vec3f(1.0, 1.0, 1.0);

            loop {
                if let Ok(cmd) = rx.recv() {
                    match cmd {
                        PreRenderCmd::SetTextures(textures) => {
                            println!("PreRenderCmd::SetTextures");
                            renderer.set_textures(textures);
                        }
                        PreRenderCmd::RenderRegion(region, palette) => {
                            println!("PreRenderCmd::RenderRegion");

                            let w = (region.width as f32 * region.grid_size as f32 * region.zoom)
                                as i32;
                            let h = (region.height as f32 * region.grid_size as f32 * region.zoom)
                                as i32;

                            let buffer = TheRGBABuffer::new(TheDim::sized(w, h));
                            let tree = RTree::new();

                            renderer.set_region(&region);
                            renderer.position =
                                vec3f(region.width as f32 / 2.0, 0.0, region.height as f32 / 2.0);

                            let mut prerendered = PreRendered {
                                albedo: buffer,
                                color: FxHashMap::default(),
                                tree,
                            };

                            renderer.prerender(
                                &mut prerendered,
                                &region,
                                &mut draw_settings,
                                &palette,
                            );
                            //buffer.to_clipboard();

                            result_tx
                                .send(PreRenderResult::RenderedRegion(region.id, prerendered))
                                .unwrap();
                            //}
                            /*
                            for (key, model) in region.models.iter_mut() {
                                model.create_voxels(
                                    region.grid_size as u8,
                                    &vec3f(key.0 as f32, key.1 as f32, key.2 as f32),
                                    &palette,
                                );

                                result_tx
                                    .send(PreRenderResult::VoxelizedModel(
                                        region.id,
                                        vec3i(key.0, key.1, key.2),
                                        model.clone(),
                                    ))
                                    .unwrap();
                                    }*/
                        }
                        PreRenderCmd::Quit => {
                            println!("PreRenderCmd::Quit");
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
