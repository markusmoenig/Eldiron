use crate::prelude::*;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use theframework::prelude::*;

pub enum VoxelRenderCmd {
    VoxelizeRegionModels(Region, ThePalette),
    Quit,
}

pub enum VoxelRenderResult {
    VoxelizedModel(Uuid, Vec3i, ModelFXStore),
    Quit,
}

#[derive(Debug)]
pub struct VoxelThread {
    pub tx: Option<mpsc::Sender<VoxelRenderCmd>>,

    pub rx: Option<mpsc::Receiver<VoxelRenderResult>>,

    renderer_thread: Option<JoinHandle<()>>,
}

impl Default for VoxelThread {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelThread {
    pub fn new() -> Self {
        Self {
            tx: None,
            rx: None,
            renderer_thread: None,
        }
    }

    /// Check for a renderer result
    pub fn receive(&self) -> Option<VoxelRenderResult> {
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                return Some(result);
            }
        }

        None
    }

    /// Send a cmd to the renderer.
    pub fn send(&self, cmd: VoxelRenderCmd) {
        if let Some(tx) = &self.tx {
            tx.send(cmd).unwrap();
        }
    }

    pub fn voxelize_region_models(&self, region: Region, palette: ThePalette) {
        self.send(VoxelRenderCmd::VoxelizeRegionModels(region, palette));
    }

    pub fn startup(&mut self) {
        let (tx, rx) = mpsc::channel::<VoxelRenderCmd>();

        self.tx = Some(tx);

        let (result_tx, result_rx) = mpsc::channel::<VoxelRenderResult>();

        self.rx = Some(result_rx);

        let renderer_thread = thread::spawn(move || {
            loop {
                if let Ok(cmd) = rx.recv() {
                    match cmd {
                        VoxelRenderCmd::VoxelizeRegionModels(mut region, palette) => {
                            println!("VoxelRenderCmd::VoxelizeRegionModels");

                            for (key, model) in region.models.iter_mut() {
                                model.create_voxels(
                                    region.grid_size as u8,
                                    &vec3f(key.0 as f32, key.1 as f32, key.2 as f32),
                                    &palette,
                                );

                                result_tx
                                    .send(VoxelRenderResult::VoxelizedModel(
                                        region.id,
                                        vec3i(key.0, key.1, key.2),
                                        model.clone(),
                                    ))
                                    .unwrap();
                            }
                        }
                        VoxelRenderCmd::Quit => {
                            println!("VoxelRenderCmd::Quit");
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
