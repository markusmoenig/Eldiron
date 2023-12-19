use crate::prelude::*;
use theframework::prelude::*;
use std::sync::mpsc;

pub mod region_instance;
pub mod world;

pub mod prelude {
    pub use super::Server;
    pub use super::region_instance::RegionInstance;
    pub use super::world::World;
}

use prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Server {
    project: Project,
    instances: FxHashMap<Uuid, RegionInstance>,
    pub world: World,

    pub anim_counter: usize,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            project: Project::default(),
            instances: FxHashMap::default(),
            world: World::default(),

            anim_counter: 0,
        }
    }

    /// Sets the current project. Resets the server.
    pub fn set_project(&mut self, project: Project) {
        self.world.reset();
        self.anim_counter = 0;
        self.project = project;
        self.setup_regions();
    }

    /// Tick. Compute the next frame.
    pub fn tick(& mut self) {

        self.world.tick();
        self.anim_counter = self.anim_counter.wrapping_add(1);

        let (sender, receiver) = mpsc::channel();
        let mut join_handles = vec![];

        for (key, mut instance) in self.instances.drain() {
            let sender = sender.clone();
            let handle = std::thread::spawn(move || {
                instance.tick();
                sender.send((key, instance)).unwrap();
            });

            join_handles.push(handle);
        }

        for handle in join_handles {
            handle.join().unwrap();
        }

        drop(sender);
        for (key, instance) in receiver {
            self.instances.insert(key, instance);
        }
    }

    /// Setup all regions in the project and create their instances.
    pub fn setup_regions(&mut self) {

        self.instances = FxHashMap::default();

        /*
        let (sender, receiver) = mpsc::channel();
        let mut join_handles = vec![];

        for region in &self.project.regions {//region_map.drain() {
            let sender = sender.clone();

            let uuid = region.id;
            let mut instance = RegionInstance::new();
            let project = self.project.clone();

            let handle = std::thread::spawn(move || {
                instance.setup(uuid, &project);
                sender.send((uuid, instance)).unwrap();
            });

            join_handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in join_handles {
            handle.join().unwrap();
        }

        drop(sender);
        for (key, instance) in receiver {
            self.instances.insert(key, instance);
        }*/

        // Syncronous version. Slower but has the advantage not to clone project for each thread.
        for region in &self.project.regions {
            let uuid = region.id;
            let mut instance = RegionInstance::new();

            instance.setup(uuid, &self.project);

            self.instances.insert(uuid, instance);
        }
    }

    /// Update the region instance for the region. Called after live updates from the editor.
    pub fn update_region(&mut self, region: &Region) {
        if let Some(instance) = self.instances.get_mut(&region.id) {
            instance.update(region);
        }
    }

    /// Draws the given region instance into the given buffer.
    pub fn draw_region(&self, uuid: &Uuid, buffer: &mut TheRGBABuffer, tiledrawer: &TileDrawer, ctx: &mut TheContext) {
        if let Some(instance) = self.instances.get(uuid) {
            instance.draw(buffer, tiledrawer, &self.anim_counter, ctx);
        }
    }
}