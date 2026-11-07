//! The appearance workspace previews through the same sprite and mesh renderers as the map.
use crate::prelude::*;
use rusterix::{Assets, D3Camera, D3IsoCamera, Entity, Item, RenderSettings};
use scenevm::{Atom, DynamicObject, GeoId, RenderMode, SceneVM};
use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
};

#[derive(Clone)]
pub struct Request {
    pub revision: u64,
    pub assets: Arc<Assets>,
    pub entity: Entity,
    pub item: Option<Item>,
    pub icon: Option<rusterix::Texture>,
    pub ground: bool,
    pub two_d: bool,
    pub clip: String,
    pub seconds: f32,
    pub phase: f32,
    pub playing: bool,
    pub yaw: f32,
    pub zoom: f32,
    pub width: u32,
    pub height: u32,
    pub background: [u8; 4],
}
pub struct Worker {
    pub send: Sender<Request>,
    pub receive: Receiver<(u64, Result<(TheRGBABuffer, f32), String>)>,
}
impl Worker {
    pub fn new() -> Self {
        let (send, requests) = mpsc::channel::<Request>();
        let (results, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let mut vm = None;
            let mut uploaded = Default::default();
            let mut resources: Option<Arc<Assets>> = None;
            let mut effects = rusterix::procedural_avatar::effects::Player::default();
            while let Ok(mut job) = requests.recv() {
                while let Ok(next) = requests.try_recv() {
                    job = next;
                }
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let vm = vm.get_or_insert_with(|| SceneVM::new(job.width, job.height));
                    if resources
                        .as_ref()
                        .is_none_or(|assets| !Arc::ptr_eq(assets, &job.assets))
                    {
                        vm.execute(Atom::ClearTiles);
                        uploaded = Default::default();
                        resources = Some(job.assets.clone());
                        effects = Default::default();
                    }
                    render_with_effects(vm, &mut uploaded, &job, &mut effects)
                }))
                .unwrap_or_else(|_| Err("The GPU preview could not be initialized".into()));
                if results.send((job.revision, result)).is_err() {
                    break;
                }
            }
        });
        Self { send, receive }
    }
}
#[cfg(test)]
pub fn render(
    vm: &mut SceneVM,
    uploaded: &mut FxHashSet<Uuid>,
    job: &Request,
) -> Result<(TheRGBABuffer, f32), String> {
    render_with_effects(vm, uploaded, job, &mut Default::default())
}
pub fn render_with_effects(
    vm: &mut SceneVM,
    uploaded: &mut FxHashSet<Uuid>,
    job: &Request,
    effects: &mut rusterix::procedural_avatar::effects::Player,
) -> Result<(TheRGBABuffer, f32), String> {
    let uploaded_before = uploaded.len();
    let w = job.width.max(1);
    let h = job.height.max(1);
    let mut settings = RenderSettings::default();
    settings
        .read(&job.assets.config)
        .map_err(|e| e.to_string())?;
    vm.execute(Atom::ClearDynamics);
    vm.execute(Atom::ClearLights);
    vm.execute(Atom::SetPalette(
        job.assets
            .palette
            .colors
            .iter()
            .map(|c| {
                c.as_ref()
                    .map(|c| {
                        let c = c.to_array();
                        Vec4::new(c[0].powf(2.2), c[1].powf(2.2), c[2].powf(2.2), c[3])
                    })
                    .unwrap_or(Vec4::zero())
            })
            .collect(),
    ));
    let background = Vec4::new(
        (job.background[0] as f32 / 255.).powf(2.2),
        (job.background[1] as f32 / 255.).powf(2.2),
        (job.background[2] as f32 / 255.).powf(2.2),
        1.,
    );
    vm.execute(Atom::SetBackground(background));
    let mut entity = job.entity.clone();
    let angle = job.yaw.to_radians();
    entity.orientation = Vec2::new(angle.sin(), angle.cos());
    let appearance = if job.ground {
        None
    } else {
        Some(
            rusterix::procedural_avatar::Runtime::resolve(&entity, &job.assets)
                .ok_or("Choose a procedural character model")?,
        )
    };
    let duration = appearance
        .map(|a| a.clip_duration(&entity, &job.assets, &job.clip))
        .transpose()?
        .unwrap_or(1.0);
    let phase = if job.playing {
        (job.phase + job.seconds / duration).rem_euclid(1.0)
    } else {
        job.phase
    };
    let seconds = phase * duration;
    let effect_entity = if job.ground {
        Entity::default()
    } else {
        entity.clone()
    };

    if job.two_d {
        settings.apply_2d(vm);
        vm.execute(Atom::SetRenderMode(RenderMode::Raster2D));
        vm.execute(Atom::SetBackground(background));
        vm.execute(Atom::SetTransform2D(Mat3::identity()));
        let ground_model = job
            .item
            .as_ref()
            .filter(|item| {
                job.ground
                    && rusterix::procedural_avatar::Runtime::item_uses_effects(item, &job.assets)
            })
            .and_then(|item| rusterix::procedural_avatar::Runtime::resolve_item(item, &job.assets));
        let (size, rgba) = if let Some(model) = ground_model {
            (
                64,
                (*model.sprite(&Entity::default(), &job.assets, "idle", 0.)?).clone(),
            )
        } else if job.ground {
            let icon = job
                .icon
                .as_ref()
                .ok_or("No image source for this item state")?;
            (icon.width as u32, icon.data.clone())
        } else {
            let appearance = rusterix::procedural_avatar::Runtime::resolve(&entity, &job.assets)
                .ok_or("Choose a procedural character model")?;
            (
                64,
                (*appearance.sprite(&entity, &job.assets, &job.clip, seconds)?).clone(),
            )
        };
        let id = if job.ground {
            GeoId::Item(1)
        } else {
            GeoId::Character(1)
        };
        vm.execute(Atom::SetAvatarBillboardData { id, size, rgba });
        let size = h.min(w) as f32 * 0.95 / job.zoom;
        vm.execute(Atom::AddDynamic {
            object: DynamicObject::billboard_avatar_2d(
                id,
                Vec2::new(w as f32 * 0.5, h as f32 * 0.5),
                size,
                size,
            )
            .with_layer(10),
        });
        let model = if job.ground { ground_model } else { appearance };
        let sources = if job
            .item
            .as_ref()
            .is_some_and(|i| !rusterix::procedural_avatar::Runtime::item_effects_enabled(i))
            && job.ground
        {
            vec![]
        } else {
            model
                .map(|m| {
                    m.effects(
                        &effect_entity,
                        &job.assets,
                        Vec3::new(w as f32 * 0.5, 0., h as f32 * 0.5),
                        size,
                        &job.clip,
                        seconds,
                        false,
                        true,
                    )
                })
                .transpose()?
                .unwrap_or_default()
        };
        effects.render(
            vm,
            &sources,
            &job.assets.palette,
            true,
            Vec3::unit_x(),
            Vec3::unit_y(),
            uploaded,
        );
    } else {
        settings.apply_3d(vm);
        vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
        vm.execute(Atom::SetGP0(background));
        let mut camera = D3IsoCamera::new();
        camera.azimuth_deg = job.yaw;
        camera.elevation_deg = 18.;
        camera.scale = if job.ground { 0.8 } else { 1.6 } * job.zoom;
        camera.center = Vec3::new(0., if job.ground { 0.4 } else { 0.9 }, 0.);
        vm.execute(Atom::SetCamera3D {
            camera: camera.as_scenevm_camera_for_surface(w as f32, h as f32),
        });
        let objects = if job.ground {
            let item = job.item.as_ref().ok_or("No item selected")?;
            let appearance = rusterix::procedural_avatar::Runtime::resolve_item(item, &job.assets)
                .ok_or("This item has no 3D model; create a model or view its 2D icon")?;
            appearance.objects(
                &Entity::default(),
                &job.assets,
                GeoId::Item(1),
                Vec3::zero(),
                0.8,
                "idle",
                0.,
                vm,
                uploaded,
                false,
            )?
        } else {
            let appearance = rusterix::procedural_avatar::Runtime::resolve(&entity, &job.assets)
                .ok_or("Choose a procedural character model")?;
            appearance.objects(
                &entity,
                &job.assets,
                GeoId::Character(1),
                Vec3::zero(),
                1.8,
                &job.clip,
                seconds,
                vm,
                uploaded,
                false,
            )?
        };
        for object in objects {
            vm.execute(Atom::AddDynamic { object });
        }
        let model = if job.ground {
            job.item
                .as_ref()
                .and_then(|i| rusterix::procedural_avatar::Runtime::resolve_item(i, &job.assets))
        } else {
            appearance
        };
        let sources = if job.ground
            && job
                .item
                .as_ref()
                .is_some_and(|i| !rusterix::procedural_avatar::Runtime::item_effects_enabled(i))
        {
            vec![]
        } else {
            model
                .map(|m| {
                    m.effects(
                        &effect_entity,
                        &job.assets,
                        Vec3::zero(),
                        if job.ground { 0.8 } else { 1.8 },
                        &job.clip,
                        seconds,
                        false,
                        false,
                    )
                })
                .transpose()?
                .unwrap_or_default()
        };
        let (_, right, up) = camera.basis_vectors();
        effects.render(
            vm,
            &sources,
            &job.assets.palette,
            false,
            right,
            up,
            uploaded,
        );
    }
    if uploaded.len() != uploaded_before {
        vm.execute(Atom::BuildAtlas);
    }
    let mut rgba = vec![0; w as usize * h as usize * 4];
    vm.render_frame(&mut rgba, w, h);
    Ok((TheRGBABuffer::from(rgba, w, h), phase))
}
