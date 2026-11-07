//! Native CPU lighting jobs. One in-flight snapshot per VM; rendering never waits.
//! No GPU objects cross the worker boundary. A small separate Rayon pool keeps
//! lighting work from occupying the server's shared update pool.
use super::{IrradianceCache, IrradianceInput, VM};
use std::{
    hash::{Hash, Hasher},
    sync::{OnceLock, mpsc},
};

#[derive(Clone, Copy, PartialEq)]
struct Scene {
    epoch: u64,
    layout: u64,
    revision: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{RasterPointLight, raster_mesh::Bounds};
    use vek::Vec3;

    fn input() -> IrradianceInput {
        IrradianceInput {
            sources: vec![RasterPointLight {
                position: Vec3::new(2., 2., 2.),
                color: Vec3::new(1., 0.4, 0.2),
                intensity: 2.,
                range: 8.,
                score: 16.,
            }],
            occluders: vec![],
            occluder_hash: 0,
            bounds: Bounds {
                min: Vec3::zero(),
                max: Vec3::broadcast(8.),
            },
            layout: 1,
        }
    }
    fn ready(worker: &mut Worker, input: &IrradianceInput, epoch: u64) -> Vec<[f32; 4]> {
        let start = std::time::Instant::now();
        loop {
            if let (_, Some(data)) = worker.update(Some(input.clone()), epoch) {
                return data;
            }
            assert!(
                start.elapsed().as_secs() < 5,
                "lighting job did not complete"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn background_lighting_matches_serial_and_relights_after_clear() {
        let input = input();
        let reference =
            VM::solve_prepared_irradiance(input.clone(), &mut IrradianceCache::default(), false)
                .unwrap();
        let mut worker = Worker::new();
        assert_eq!(ready(&mut worker, &input, 0), reference);
        assert!(worker.update(None, 0).0);
        assert_eq!(
            ready(&mut worker, &input, 0),
            reference,
            "Turning the same light back on must not reuse an unpublishable cached result"
        );
        let mut replacement = input.clone();
        replacement.layout = 2;
        assert_eq!(ready(&mut worker, &replacement, 0), reference);
    }
    #[test]
    fn obsolete_world_job_is_rejected_and_latest_pose_eventually_publishes() {
        let mut worker = Worker::new();
        let initial = input();
        worker.update(Some(initial.clone()), 1);
        assert!(worker.busy);
        // Invalidate the world without waiting for the old job.
        let mut latest = initial.clone();
        latest.sources[0].position.x = 5.;
        let expected =
            VM::solve_prepared_irradiance(latest.clone(), &mut IrradianceCache::default(), false)
                .unwrap();
        assert_eq!(ready(&mut worker, &latest, 2), expected);
        // Pose changes keep the completed grid usable, with one snapshot in flight.
        for i in 0..30 {
            latest.sources[0].position.x = 1. + i as f32 * 0.1;
            worker.update(Some(latest.clone()), 2);
        }
        let expected =
            VM::solve_prepared_irradiance(latest.clone(), &mut IrradianceCache::default(), false)
                .unwrap();
        let start = std::time::Instant::now();
        loop {
            if worker.update(Some(latest.clone()), 2).1.as_ref() == Some(&expected) {
                break;
            }
            assert!(
                start.elapsed().as_secs() < 5,
                "latest lighting snapshot starved"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
struct Job {
    scene: Scene,
    input: IrradianceInput,
}
struct Completed {
    scene: Scene,
    data: Option<Vec<[f32; 4]>>,
}

pub(super) struct Worker {
    jobs: mpsc::SyncSender<Job>,
    results: std::sync::Mutex<mpsc::Receiver<Completed>>,
    busy: bool,
    scene: Option<Scene>,
    requested: Option<(u64, u64)>,
    revision: u64,
}

impl Worker {
    pub fn new() -> Self {
        let (jobs, input) = mpsc::sync_channel::<Job>(1);
        let (output, results) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("scenevm-lighting".into())
            .spawn(move || {
                static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
                let pool = POOL.get_or_init(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(2)
                        .thread_name(|i| format!("lighting-probes-{i}"))
                        .build()
                        .expect("lighting thread pool")
                });
                let mut cache = IrradianceCache::default();
                let mut previous_scene = None;
                while let Ok(job) = input.recv() {
                    if previous_scene != Some(job.scene) {
                        cache = IrradianceCache::default();
                        previous_scene = Some(job.scene);
                    }
                    let data =
                        pool.install(|| VM::solve_prepared_irradiance(job.input, &mut cache, true));
                    if output
                        .send(Completed {
                            scene: job.scene,
                            data,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("lighting worker");
        Self {
            jobs,
            results: std::sync::Mutex::new(results),
            busy: false,
            scene: None,
            requested: None,
            revision: 0,
        }
    }

    /// Returns whether the old grid became invalid, and a completed valid grid.
    pub fn update(
        &mut self,
        input: Option<IrradianceInput>,
        epoch: u64,
    ) -> (bool, Option<Vec<[f32; 4]>>) {
        let identity = input.as_ref().map(|i| (epoch, i.layout));
        if self.scene.map(|s| s.epoch) != identity.map(|(epoch, _)| epoch) {
            self.revision = self.revision.wrapping_add(1);
        }
        let scene = identity.map(|(epoch, layout)| Scene {
            epoch,
            layout,
            revision: self.revision,
        });
        let invalidated =
            self.scene.map(|s| (s.epoch, s.revision)) != scene.map(|s| (s.epoch, s.revision));
        self.scene = scene;
        let mut data = None;
        match self.results.lock().unwrap().try_recv() {
            Ok(result) => {
                self.busy = false;
                // Moving/added lights can use the preceding completed snapshot.
                // Clearing lights or replacing the world must reject old work.
                if self.scene.is_some_and(|s| {
                    s.epoch == result.scene.epoch && s.revision == result.scene.revision
                }) {
                    data = result.data;
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.busy = false;
                self.requested = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if let (Some(input), Some(scene)) = (input, scene) {
            let mut h = rustc_hash::FxHasher::default();
            input.occluder_hash.hash(&mut h);
            scene.layout.hash(&mut h);
            scene.revision.hash(&mut h);
            for v in [
                input.bounds.min.x,
                input.bounds.min.y,
                input.bounds.min.z,
                input.bounds.max.x,
                input.bounds.max.y,
                input.bounds.max.z,
            ] {
                v.to_bits().hash(&mut h);
            }
            for s in &input.sources {
                for v in [
                    s.position.x,
                    s.position.y,
                    s.position.z,
                    s.color.x,
                    s.color.y,
                    s.color.z,
                    s.intensity,
                    s.range,
                ] {
                    v.to_bits().hash(&mut h);
                }
            }
            let token = (epoch, h.finish());
            if !self.busy && self.requested != Some(token) {
                if self.jobs.try_send(Job { scene, input }).is_ok() {
                    self.busy = true;
                    self.requested = Some(token);
                }
            }
        } else {
            self.requested = None;
        }
        (invalidated, data)
    }
}
