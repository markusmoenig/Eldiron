//! Camera-independent, posed world meshes. Material animation remains live.
use super::*;
use denrim_materials::geometry::rig::Pose;

#[derive(PartialEq)]
struct Placement {
    id: GeoId,
    feet: Vec3<f32>,
    orientation: Vec2<f32>,
    height: f32,
    ground_flat: bool,
    joints: Vec<Pose>,
    visibility: Vec<bool>,
}

pub(super) struct CachedObjects {
    compiled: Arc<Compiled>,
    placement: Placement,
    // Keep the semantic part index even when other parts are empty or hidden.
    parts: Vec<(usize, DynamicObject)>,
}

pub(super) fn objects(
    appearance: &Appearance,
    entity: &Entity,
    assets: &Assets,
    id: GeoId,
    feet: Vec3<f32>,
    height: f32,
    animation: &str,
    seconds: f32,
    surface_seconds: f32,
    vm: &mut SceneVM,
    uploaded: &mut FxHashSet<Uuid>,
    ground_flat: bool,
    supplied_joints: Option<&[Pose]>,
    cache: &mut Option<CachedObjects>,
) -> Result<Vec<DynamicObject>, String> {
    prepare(
        appearance,
        entity,
        assets,
        id,
        feet,
        height,
        animation,
        seconds,
        ground_flat,
        supplied_joints,
        cache,
    )?;
    // Texture frames and atlas uploads have a different lifetime from poses.
    // This also restores uploads after an atlas rebuild on a mesh cache hit.
    let cached = cache.as_ref().unwrap();
    let c = &cached.compiled;
    let mut result = Vec::with_capacity(cached.parts.len());
    for (part, object) in &cached.parts {
        let (material_id, texture, packed) = c.materials[*part].at(surface_seconds);
        if uploaded.insert(material_id) {
            vm.execute(Atom::AddTile {
                id: material_id,
                width: texture.width as u32,
                height: texture.height as u32,
                frames: vec![texture.data.clone()],
                material_frames: Some(vec![packed.to_vec()]),
            });
        }
        let mut object = object.clone();
        object.tile_id = Some(material_id);
        result.push(object);
    }
    Ok(result)
}

fn prepare(
    appearance: &Appearance,
    entity: &Entity,
    assets: &Assets,
    id: GeoId,
    feet: Vec3<f32>,
    height: f32,
    animation: &str,
    seconds: f32,
    ground_flat: bool,
    supplied_joints: Option<&[Pose]>,
    cache: &mut Option<CachedObjects>,
) -> Result<(), String> {
    let (c, _) = appearance.compiled_for(entity, assets)?;
    let motion = Appearance::motion(&c, animation);
    let seconds = Appearance::animation_seconds(&c, entity, animation, seconds);
    let joints = if let Some(rig) = &c.evaluation.mesh.rig {
        if let Some(joints) = supplied_joints {
            joints.to_vec()
        } else {
            rig.sample(motion, seconds).map_err(|e| e.to_string())?
        }
    } else {
        vec![]
    };
    // Visibility keys can hide a part without changing any joint transforms
    // (for example the released arrow). They must invalidate the mesh too.
    let visibility = motion.map_or_else(Vec::new, |motion| {
        let phase = if motion.looping {
            seconds.rem_euclid(motion.duration)
        } else {
            seconds.clamp(0., motion.duration)
        } / motion.duration;
        motion
            .tracks
            .iter()
            .map(|track| {
                track
                    .keys
                    .iter()
                    .rev()
                    .find(|key| key.time <= phase)
                    .unwrap_or(&track.keys[0])
                    .visible
            })
            .collect()
    });
    let placement = Placement {
        id,
        feet,
        orientation: entity.orientation,
        height,
        ground_flat,
        joints,
        visibility,
    };
    let reuse = cache
        .as_ref()
        .is_some_and(|cached| Arc::ptr_eq(&cached.compiled, &c) && cached.placement == placement);
    if !reuse {
        let mesh = c
            .evaluation
            .mesh
            .posed_with_joints(
                motion,
                seconds,
                c.evaluation
                    .mesh
                    .rig
                    .as_ref()
                    .map(|_| placement.joints.as_slice()),
            )
            .map_err(|e| e.to_string())?
            .mesh;
        let yaw = entity.orientation.x.atan2(entity.orientation.y);
        let (s, co) = yaw.sin_cos();
        let scale = height / c.height;
        let local = |p: [f32; 3]| if ground_flat { [p[0], p[2], -p[1]] } else { p };
        let floor = if ground_flat {
            mesh.parts
                .iter()
                .flat_map(|p| &p.vertices)
                .map(|v| v.position[2])
                .fold(f32::INFINITY, f32::min)
        } else {
            c.floor
        };
        let center = if ground_flat {
            let (lo, hi) = mesh.bounds();
            [(lo[0] + hi[0]) * 0.5, -(lo[1] + hi[1]) * 0.5]
        } else {
            [0.; 2]
        };
        let mut parts = vec![];
        for (index, (part, material)) in mesh.parts.iter().zip(&c.materials).enumerate() {
            if part.vertices.is_empty() || !material.visible {
                continue;
            }
            let vertices = part
                .vertices
                .iter()
                .map(|v| DynamicMeshVertex {
                    position: {
                        let p = local(v.position);
                        let (x, y, z) = (p[0] - center[0], p[1] - floor, p[2] - center[1]);
                        Vec3::new(
                            feet.x + (co * x + s * z) * scale,
                            feet.y + y * scale,
                            feet.z + (-s * x + co * z) * scale,
                        )
                    },
                    normal: {
                        let p = local(v.normal);
                        Vec3::new(co * p[0] + s * p[2], p[1], -s * p[0] + co * p[2])
                    },
                    uv: Vec2::new(v.uv[0], v.uv[1]),
                })
                .collect::<Vec<_>>();
            let indices = (0..vertices.len() as u32).collect();
            let mut object = DynamicObject::mesh(id, material.id, vertices, indices);
            object.center = feet;
            parts.push((index, object));
        }
        *cache = Some(CachedObjects {
            compiled: c.clone(),
            placement,
            parts,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn retained_meshes_animate_materials_and_restore_atlas_uploads() {
        let mut assets = Assets::default();
        assets.palette = eldiron_ruleset::palette_template_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let mut doc = Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap();
        doc.rows
            .iter_mut()
            .find(|r| r.id == doc.output)
            .unwrap()
            .tiles
            .push(denrim_materials::geometry::Tile {
                id: "animated-material".into(),
                op: Operation::Texture {
                    preserve_uv: true,
                    parts: vec![],
                    source: "animated".into(),
                    bundle: None,
                    size: 16,
                    axes: [0, 1],
                    origin: [0.; 2],
                    span: [1.; 2],
                },
            });
        let frames = [8, 20].map(|index| {
            Texture::new(
                assets.palette.colors[index]
                    .as_ref()
                    .unwrap()
                    .to_u8_array()
                    .repeat(4),
                2,
                2,
            )
        });
        assets
            .ruleset_materials
            .insert("animated".into(), frames[0].clone());
        assets
            .material_animations
            .insert("animated".into(), (frames.to_vec(), 1.));
        let model = Appearance::new(&doc.save().unwrap(), &[], 1).unwrap();
        let entity = Entity::default();
        let id = GeoId::Item(1);
        let mut cache = None;
        let mut uploaded = FxHashSet::default();
        let mut vm = SceneVM::new(16, 16);
        let mut make = |time, uploaded: &mut FxHashSet<Uuid>| {
            objects(
                &model,
                &entity,
                &assets,
                id,
                Vec3::zero(),
                2.,
                "idle",
                0.,
                time,
                &mut vm,
                uploaded,
                false,
                None,
                &mut cache,
            )
            .unwrap()
        };
        let first = make(0.1, &mut uploaded);
        let next = make(0.6, &mut uploaded);
        assert!(Arc::ptr_eq(&first[0].mesh_vertices, &next[0].mesh_vertices));
        assert_ne!(first[0].tile_id, next[0].tile_id);
        uploaded.clear();
        let restored = make(1.1, &mut uploaded);
        assert_eq!(first[0].tile_id, restored[0].tile_id);
        assert!(
            restored
                .iter()
                .all(|object| uploaded.contains(&object.tile_id.unwrap()))
        );
        assert!(Arc::ptr_eq(
            &first[0].mesh_vertices,
            &restored[0].mesh_vertices
        ));
        let mut runtime = Runtime::default();
        runtime.meshes.insert(id, cache);
        runtime.retain(&FxHashSet::default());
        assert!(
            runtime.meshes.is_empty(),
            "Removed items must release retained meshes"
        );
    }

    #[test]
    fn world_meshes_reuse_buffers_and_follow_pose_equipment_and_placement_edits() {
        let (assets, model, mut entity) = super::super::tests::fixture();
        let id = GeoId::Character(7);
        let mut cache = None;
        let mut feet = Vec3::new(2., 0., 3.);
        let mut height = 2.;
        let mut flat = false;
        let mut time = 0.;
        let make =
            |entity: &Entity, feet, height, flat, time, cache: &mut Option<CachedObjects>| {
                prepare(
                    &model, entity, &assets, id, feet, height, "walk", time, flat, None, cache,
                )
                .unwrap();
            };
        make(&entity, feet, height, flat, time, &mut cache);
        let first = cache.as_ref().unwrap().parts[0].1.mesh_vertices.clone();
        make(&entity, feet, height, flat, time, &mut cache);
        assert!(Arc::ptr_eq(
            &first,
            &cache.as_ref().unwrap().parts[0].1.mesh_vertices
        ));
        for edit in 0..5 {
            let previous = cache.as_ref().unwrap().parts[0].1.mesh_vertices.clone();
            match edit {
                0 => time = 0.2,
                1 => feet.x += 2.,
                2 => entity.orientation = Vec2::new(1., 0.),
                3 => height = 3.,
                _ => flat = true,
            }
            make(&entity, feet, height, flat, time, &mut cache);
            assert!(
                !Arc::ptr_eq(&previous, &cache.as_ref().unwrap().parts[0].1.mesh_vertices),
                "edit {edit}"
            );
            let mut cold = None;
            make(&entity, feet, height, flat, time, &mut cold);
            for ((a_part, a), (b_part, b)) in cache
                .as_ref()
                .unwrap()
                .parts
                .iter()
                .zip(&cold.as_ref().unwrap().parts)
            {
                assert_eq!(a_part, b_part);
                assert_eq!(a.mesh_vertices, b.mesh_vertices);
                assert_eq!(a.mesh_indices, b.mesh_indices);
            }
        }
        let before = cache.as_ref().unwrap().compiled.clone();
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let mut bow = crate::Item::default();
        crate::server::data::apply_item_data(
            &mut bow,
            &templates
                .iter()
                .find(|t| t.id == "hunting_bow")
                .unwrap()
                .data,
        );
        entity.equipped.insert("main_hand".into(), bow);
        prepare(
            &model, &entity, &assets, id, feet, height, "attack", 0., false, None, &mut cache,
        )
        .unwrap();
        assert!(!Arc::ptr_eq(&before, &cache.as_ref().unwrap().compiled));
        let joints = cache.as_ref().unwrap().placement.joints.clone();
        let vertices = cache.as_ref().unwrap().parts[0].1.mesh_vertices.clone();
        prepare(
            &model,
            &entity,
            &assets,
            id,
            feet,
            height,
            "attack",
            0.01,
            false,
            Some(&joints),
            &mut cache,
        )
        .unwrap();
        assert!(
            Arc::ptr_eq(&vertices, &cache.as_ref().unwrap().parts[0].1.mesh_vertices),
            "A frozen pose must survive time changes"
        );
        let visibility = cache.as_ref().unwrap().placement.visibility.clone();
        prepare(
            &model,
            &entity,
            &assets,
            id,
            feet,
            height,
            "attack",
            0.9,
            false,
            Some(&joints),
            &mut cache,
        )
        .unwrap();
        assert_ne!(
            cache.as_ref().unwrap().placement.visibility,
            visibility,
            "Arrow release changes visibility independently of pose"
        );
        assert!(!Arc::ptr_eq(
            &vertices,
            &cache.as_ref().unwrap().parts[0].1.mesh_vertices
        ));
    }
}
