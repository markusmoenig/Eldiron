//! An opt-in native live-scene harness, using Eldiron's actual GameWidget/SceneVM path.
use character_generator::{
    Catalog, GenerationOptions, Motion, StylePreset, eldiron::SceneVmCharacter,
};
use rusterix::client::widget::game::GameWidget;
use rusterix::{D3Camera, D3IsoCamera, Map, PlayerCamera, Rusterix};
use scenevm::prelude::Vec3;
use scenevm::{Atom, GeoId, RenderMode, SceneVM, SceneVMApp, SceneVMRenderCtx};
use shared::{
    project_io::decode_project,
    rusterix_utils::{setup_client, start_server, warmup_runtime},
};
use std::{path::PathBuf, time::Instant};
use theframework::prelude::{TheDim, TheRGBABuffer};

struct Demo {
    runtime: Rusterix,
    map: Map,
    widget: GameWidget,
    character: SceneVmCharacter,
    origin: [f32; 3],
    started: Instant,
    motion: Motion,
    automatic: bool,
    registered: bool,
    scene_tiles: bool,
    zoom: f32,
}
impl Demo {
    fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let mut project = decode_project(&std::fs::read(path)?)?;
        project.migrate_default_ruleset();
        let mut runtime = Rusterix::default();
        runtime.assets.ruleset_palette = project.palette.clone();
        runtime.assets.palette = project.art_palette.clone();
        runtime.assets.palette_materials = project
            .art_palette_materials
            .iter()
            .map(|m| m.rmoe_values())
            .collect();
        runtime.assets.palette_material_ids = project
            .art_palette_materials
            .iter()
            .map(|m| m.material_id())
            .collect();
        start_server(&mut runtime, &mut project, false)?;
        let commands = setup_client(&mut runtime, &mut project);
        runtime.server.process_client_commands(commands);
        warmup_runtime(&mut runtime, &mut project, 3);
        runtime.client.server_time = project.time;
        runtime.scene_handler.settings.read(&project.config)?;
        let region = project
            .regions
            .iter_mut()
            .find(|r| r.map.name == runtime.client.current_map)
            .ok_or("project has no active region")?;
        runtime.server.apply_entities_items(&mut region.map);
        let mut map = region.map.clone();
        let player = map
            .entities
            .iter()
            .find(|e| e.is_player())
            .or_else(|| map.entities.first())
            .ok_or("demo needs a player spawn in the active region")?;
        let pos = player.position;
        let ground = map
            .geometry_floor_height_nearest(player.get_pos_xz(), pos.y)
            .unwrap_or(pos.y);
        let origin = [pos.x, ground, pos.z];
        // The generated mesh takes the player's visual place in this in-memory
        // scene copy. The original project and server entity are never changed.
        let id = player.id;
        map.entities.retain(|e| e.id != id);
        runtime.set_tiles_for_maps(project.tiles.clone(), false, [&map]);
        let asset = Catalog::parse(include_str!("../examples/heads.toml"))?.generate_with_options(
            "angular",
            &GenerationOptions {
                style: StylePreset::Natural,
                ..Default::default()
            },
        )?;
        let character = SceneVmCharacter::new_pixel_art(&asset)?;
        let mut widget = GameWidget::new();
        widget.set_camera_mode(PlayerCamera::D3Iso);
        eprintln!(
            "Live scene: {} — {} geometry objects; generated character at {:?}",
            map.name,
            map.geometry_objects.len(),
            origin
        );
        eprintln!("1 Idle · 2 Walk · 3 Attack · A automatic · D dithering · wheel zoom");
        Ok(Self {
            runtime,
            map,
            widget,
            character,
            origin,
            started: Instant::now(),
            motion: Motion::Idle,
            automatic: true,
            registered: false,
            scene_tiles: false,
            zoom: 4.0,
        })
    }
    fn prepare(&mut self, size: (u32, u32), time: f32) {
        if !self.scene_tiles {
            // A native runner supplies its own window-backed SceneVM. Upload the
            // actual project atlas into that VM on its first frame as well.
            self.runtime
                .scene_handler
                .build_atlas(&self.runtime.assets.tiles, false);
            self.scene_tiles = true;
        }
        if self.widget.buffer.dim().width != size.0 as i32
            || self.widget.buffer.dim().height != size.1 as i32
        {
            self.widget.buffer = TheRGBABuffer::new(TheDim::sized(size.0 as i32, size.1 as i32));
        }
        self.widget.rect.width = size.0 as f32;
        self.widget.rect.height = size.1 as f32;
        let mut camera = D3IsoCamera::new();
        camera.set_parameter_vec3(
            "center",
            Vec3::new(self.origin[0], self.origin[1] + 0.5, self.origin[2]),
        );
        camera.scale = self.zoom;
        self.widget.camera_d3 = Box::new(camera);
        self.runtime.scene_handler.mark_dynamics_dirty();
        self.widget.prepare_frame(
            &self.map,
            &self.runtime.client.server_time,
            (time * 4.0) as usize,
            &self.runtime.assets,
            &mut self.runtime.scene_handler,
        );
        if !self.registered {
            for m in &self.character.materials {
                self.runtime.scene_handler.vm.execute(Atom::AddTile {
                    id: m.id,
                    width: m.size,
                    height: m.size,
                    frames: vec![m.rgba.clone()],
                    material_frames: Some(vec![m.packed_material.clone()]),
                });
            }
            self.runtime.scene_handler.vm.execute(Atom::BuildAtlas);
            self.registered = true;
        }
        let (motion, phase) = if self.automatic {
            let cycle = time % 8.0;
            if cycle < 2.0 {
                (Motion::Idle, cycle)
            } else if cycle < 6.0 {
                (Motion::Walk, cycle - 2.0)
            } else {
                (Motion::Attack, (cycle - 6.0).min(1.0))
            }
        } else {
            (self.motion, time)
        };
        let vm = &mut self.runtime.scene_handler.vm;
        vm.set_active_vm(0);
        vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
        // Add AFTER Eldiron rebuilds normal scene dynamics, so its clear step
        // cannot delete the character. It shares geometry depth and lighting.
        for object in
            self.character
                .objects(GeoId::Unknown(0xf000), self.origin, 0.0, motion, phase)
        {
            vm.execute(Atom::AddDynamic { object });
        }
    }
}
impl SceneVMApp for Demo {
    fn window_title(&self) -> Option<String> {
        Some("Eldiron live character — 1 Idle / 2 Walk / 3 Attack / A Auto / D Dither".into())
    }
    fn initial_window_size(&self) -> Option<(u32, u32)> {
        Some((960, 640))
    }
    fn target_fps(&self) -> Option<f32> {
        Some(30.0)
    }
    fn render(&mut self, vm: &mut SceneVM, ctx: &mut dyn SceneVMRenderCtx) {
        std::mem::swap(vm, &mut self.runtime.scene_handler.vm);
        self.prepare(ctx.size(), self.started.elapsed().as_secs_f32());
        if let Err(e) = ctx.present(&mut self.runtime.scene_handler.vm) {
            eprintln!("Present: {e:?}");
        }
        std::mem::swap(vm, &mut self.runtime.scene_handler.vm);
    }
    fn key_down(&mut self, _: &mut SceneVM, key: &str) {
        match key.to_ascii_lowercase().as_str() {
            "1" => {
                self.motion = Motion::Idle;
                self.automatic = false;
            }
            "2" => {
                self.motion = Motion::Walk;
                self.automatic = false;
            }
            "3" => {
                self.motion = Motion::Attack;
                self.automatic = false;
            }
            "a" => self.automatic = true,
            "d" => self.character.pixel_art = !self.character.pixel_art,
            _ => return,
        }
        self.started = Instant::now();
    }
    fn scroll(&mut self, _: &mut SceneVM, _: f32, dy: f32) {
        self.zoom = (self.zoom * (dy * 0.01).exp()).clamp(2.0, 12.0);
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "test_projects/3DStarter.eldiron".into()),
    );
    let mut demo = Demo::load(&path)?;
    if args.next().as_deref() == Some("--snapshot") {
        let out = PathBuf::from(args.next().ok_or("--snapshot needs an output directory")?);
        std::fs::create_dir_all(&out)?;
        // Let the real asynchronous scene builders finish before validating.
        for _ in 0..60 {
            demo.prepare((640, 480), 0.0);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        demo.automatic = false;
        for (name, motion, time) in [
            ("idle", Motion::Idle, 0.0),
            ("walk", Motion::Walk, 0.25),
            ("attack", Motion::Attack, 0.45),
        ] {
            demo.motion = motion;
            demo.prepare((640, 480), time);
            let mut rgba = vec![0; 640 * 480 * 4];
            demo.runtime
                .scene_handler
                .vm
                .render_frame(&mut rgba, 640, 480);
            image::save_buffer(
                out.join(format!("{name}.png")),
                &rgba,
                640,
                480,
                image::ColorType::Rgba8,
            )?;
        }
        eprintln!("SceneVM GPU snapshots written to {}", out.display());
    } else {
        scenevm::run_scenevm_app(demo)?;
    }
    Ok(())
}
