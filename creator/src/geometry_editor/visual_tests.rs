use super::*;
use denrim_materials::geometry::authoring;
fn near(a: [f32; 3], b: [f32; 3]) {
    for k in 0..3 {
        assert!((a[k] - b[k]).abs() < 0.0002, "{a:?} != {b:?}");
    }
}
#[test]
fn arbitrary_tree_profiles_mirroring_and_reparenting() {
    let mut d = scene::new_document();
    assert_eq!(d.evaluate().unwrap().mesh.vertex_count(), 0);
    let a = scene::add_joint(&mut d, "root", [0.3, 0.8, 0.]).unwrap();
    let b = scene::add_joint(&mut d, &a, [0.4, 1.1, 0.]).unwrap();
    let other = scene::mirror(&mut d, &a).unwrap();
    let e = d.evaluate().unwrap();
    let rig = e.mesh.rig.as_ref().unwrap();
    assert_eq!(rig.joints.len(), 5);
    assert_eq!(e.mesh.parts.len(), 4);
    near(
        rig.rest()[rig.index(&other).unwrap()].translation,
        [-0.3, 0.8, 0.],
    );
    let verts = &e.mesh.parts[0].vertices;
    let mirrored = &e.mesh.parts[2].vertices;
    let bounds = |v: &[denrim_materials::geometry::Vertex]| {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for v in v {
            for k in 0..3 {
                lo[k] = lo[k].min(v.position[k]);
                hi[k] = hi[k].max(v.position[k]);
            }
        }
        (lo, hi)
    };
    let (lo, hi) = bounds(verts);
    let (ml, mh) = bounds(mirrored);
    near(ml, [-hi[0], lo[1], lo[2]]);
    near(mh, [-lo[0], hi[1], hi[2]]);
    let before = rig.rest()[rig.index(&b).unwrap()];
    authoring::reparent(
        scene::armature_mut(&mut d).unwrap(),
        &b,
        Some("root".into()),
    )
    .unwrap();
    let rig = authoring::skeleton(scene::armature(&d).unwrap().1).unwrap();
    near(
        before.translation,
        rig.rest()[rig.index(&b).unwrap()].translation,
    );
    let mut bad = scene::armature(&d).unwrap().1.to_vec();
    assert!(authoring::reparent(&mut bad, "root", Some(a)).is_err());
    assert_eq!(Document::load(&d.save().unwrap()).unwrap(), d);
}
#[test]
fn horizontal_bone_stretch_keeps_profile_cross_section_and_follows_tip() {
    let mut d = scene::new_document();
    let tip = scene::add_joint(&mut d, "root", [0., 1.5, 0.]).unwrap();
    let part = scene::profiles(&d).unwrap().2[0].clone();
    authoring::set_world(scene::armature_mut(&mut d).unwrap(), &tip, [2., 0.5, 0.]).unwrap();
    let e = d.evaluate().unwrap();
    let r = e.mesh.rig.as_ref().unwrap();
    let (p, n, _) = part.map(r, [0., 1., 0.], [1., 0., 0.]).unwrap();
    near(p, [2., 0.5, 0.]);
    near(n, [0., -1., 0.]);
    let (p, _, _) = part.map(r, [0.1, 0.1, 0.], [1., 0., 0.]).unwrap();
    near(p, [0.2, 0.4, 0.]);
}
#[test]
fn quadruped_uses_the_same_resources_and_pose_keys_interpolate() {
    let mut d = scene::quadruped();
    let e = d.evaluate().unwrap();
    let rig = e.mesh.rig.as_ref().unwrap();
    assert_eq!(rig.joints.len(), 25);
    assert!(e.mesh.parts.len() > 25);
    assert_eq!(Document::load(&d.save().unwrap()).unwrap(), d);
    let Value::Motion(m) = &e.rows["trot"] else {
        panic!()
    };
    assert_eq!(m.tracks.len(), 4);
    let pose = e.mesh.posed(Some(m), m.duration * 0.25).unwrap();
    assert_ne!(pose.mesh.parts[5].vertices, e.mesh.parts[5].vertices);
    let joint = m.tracks[0].joint.clone();
    let at = rig.index(&joint).unwrap();
    let old = rig.sample(Some(m), 0.125 * m.duration).unwrap();
    scene::motion_key(&mut d, "trot", &joint, 0.125, [0.; 3]).unwrap();
    let e = d.evaluate().unwrap();
    let Value::Motion(m) = &e.rows["trot"] else {
        panic!()
    };
    let new = m.rig.sample(Some(m), 0.125 * m.duration).unwrap();
    near(old[at].point([0., 0.3, 0.]), new[at].point([0., 0.3, 0.]));
    assert!(m.tracks[0].keys.iter().any(|k| k.time == 0.125));
}
#[test]
fn viewport_drag_saves_once_and_escape_restores_source() {
    let mut d = scene::new_document();
    let joint = scene::add_joint(&mut d, "root", [0.2, 1., 0.]).unwrap();
    let e = d.evaluate().unwrap();
    let original = d.save().unwrap();
    let frame = scene::Projection::new(([-1., 0., -1.], [1., 2., 1.]), 400, 400, 0., 0.);
    let q = frame.project([0.2, 1., 0.]);
    let p = Vec2::new(q[0].round() as i32, q[1].round() as i32);
    let id = Uuid::new_v4();
    let mut project = Project::default();
    project.procedural_appearances.insert(id, original.clone());
    let mut host = GeometryEditor {
        active: true,
        selected: Some(id),
        document: Some(d),
        evaluation: Some(e),
        ..Default::default()
    };
    host.scene.tool = 1;
    host.scene.projection = Some(frame);
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(400, 400, 1.);
    *UNDOMANAGER.write().unwrap() = crate::undo::UndoManager::new();
    let event = |p| TheEvent::RenderViewClicked(TheId::named(PREVIEW), p);
    host.scene_event(&event(p), &mut project, &mut ui, &mut ctx)
        .unwrap();
    for n in [5, 15, 30] {
        host.scene_event(
            &TheEvent::RenderViewDragged(TheId::named(PREVIEW), Vec2::new(p.x + n, p.y)),
            &mut project,
            &mut ui,
            &mut ctx,
        )
        .unwrap();
    }
    host.scene_event(
        &TheEvent::RenderViewUp(TheId::named(PREVIEW), p),
        &mut project,
        &mut ui,
        &mut ctx,
    )
    .unwrap();
    let changed = project.procedural_appearances[&id].clone();
    assert_ne!(changed, original);
    let next = Document::load(&changed).unwrap();
    assert!(
        authoring::skeleton(scene::armature(&next).unwrap().1)
            .unwrap()
            .rest()
            .iter()
            .any(|p| p.translation[0] > 0.2)
    );
    let mut server = ServerContext::default();
    UNDOMANAGER
        .write()
        .unwrap()
        .undo(&mut server, &mut project, &mut ui, &mut ctx);
    assert_eq!(project.procedural_appearances[&id], original);
    UNDOMANAGER
        .write()
        .unwrap()
        .redo(&mut server, &mut project, &mut ui, &mut ctx);
    assert_eq!(project.procedural_appearances[&id], changed);
    host.document = Some(next.clone());
    host.evaluation = Some(next.evaluate().unwrap());
    let r = host.evaluation.as_ref().unwrap().mesh.rig.as_ref().unwrap();
    let q = host
        .scene
        .projection
        .as_ref()
        .unwrap()
        .project(r.rest()[r.index(&joint).unwrap()].translation);
    let p = Vec2::new(q[0].round() as i32, q[1].round() as i32);
    host.scene_event(&event(p), &mut project, &mut ui, &mut ctx)
        .unwrap();
    host.scene_event(
        &TheEvent::RenderViewDragged(TheId::named(PREVIEW), Vec2::new(p.x + 25, p.y)),
        &mut project,
        &mut ui,
        &mut ctx,
    )
    .unwrap();
    host.finish_scene_drag(&mut project, &mut ctx, true);
    assert_eq!(project.procedural_appearances[&id], changed);
}
#[test]
fn shield_surface_is_linked_to_the_ruleset_not_owned_by_the_model() {
    let doc = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap();
    let mut p = Project::default();
    let e = doc.evaluate().unwrap();
    let spec = e
        .mesh
        .parts
        .iter()
        .filter_map(|p| p.texture.as_ref())
        .find(|t| t.source == "ruleset:icon/round_shield/on")
        .unwrap();
    assert_eq!(spec.source, "ruleset:icon/round_shield/on");
    assert!(spec.bundle.is_none());
    let a = textures::asset(spec, &p.palette, Some(&p)).unwrap();
    assert_eq!(a.selected_material, "shield");
    assert!(
        a.root.project.materials[0]
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .any(|t| t.kernel == "Circle")
    );
    assert!(!a.instances[0].palette_bindings.is_empty());
    let id = shared::rulesets::material_asset_id("icon/torch/on");
    let replacement=denrim_materials::integration::EditableMaterialAsset::parse("ruleset-shield","project 1\nmaterial \"m\" called \"Replacement\"\n  mode pbr\n  row \"noise\" called \"Noise\"\n    use \"Perlin Noise\" id \"noise\"\n  output base color from row \"noise\" xyz\n","m").unwrap();
    let linked = "ruleset:icon/torch/on";
    let before = textures::input_key(&p, linked);
    p.procedural_materials
        .insert(id, replacement.save().unwrap());
    assert_ne!(before, textures::input_key(&p, linked));
    let torch = denrim_materials::geometry::Texture {
        source: linked.into(),
        bundle: None,
        size: 32,
    };
    let a = textures::asset(&torch, &p.palette, Some(&p)).unwrap();
    assert_eq!(a.selected_material, "m");
    let mut e = doc.evaluate().unwrap();
    textures::resolve_evaluation(&mut e, &p).unwrap();
    assert!(
        e.mesh
            .parts
            .iter()
            .filter_map(|p| p.texture.as_ref())
            .all(|t| t.bundle.is_some())
    );
    assert!(
        doc.evaluate()
            .unwrap()
            .mesh
            .parts
            .iter()
            .filter_map(|p| p.texture.as_ref())
            .all(|t| t.bundle.is_none())
    );
}
#[test]
fn human_tools_navigate_to_shared_resources_and_keep_pose_phase() {
    let root = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap();
    let mut p = Project::default();
    let id = Uuid::new_v4();
    p.procedural_appearances.insert(id, root.save().unwrap());
    let mut host = GeometryEditor {
        active: true,
        selected: Some(id),
        root_document: Some(root.clone()),
        document: Some(root),
        ..Default::default()
    };
    let mut ctx = TheContext::new(400, 1000, 1.);
    let mut ui = TheUI::new();
    let mut pages = TheStackLayout::new(TheId::named("Sidebar Page Stack"));
    for _ in 0..crate::procedural_material::SIDEBAR_PAGE {
        pages.add_canvas(TheCanvas::new());
    }
    pages.add_canvas(workspace(TheCanvas::new()));
    pages.set_index(crate::procedural_material::SIDEBAR_PAGE);
    ui.canvas.set_layout(pages);
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    host.scene_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    )
    .unwrap();
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert_eq!(host.group_path, ["body-op-0"]);
    host.playback.phase = 0.37;
    host.scene_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 4),
        &mut p,
        &mut ui,
        &mut ctx,
    )
    .unwrap();
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert!(host.group_path.is_empty());
    assert!(!host.playback.playing);
    assert_eq!(host.playback.phase, 0.37);
    assert!(
        host.playback
            .motion(host.evaluation.as_ref().unwrap())
            .is_some()
    );
    host.scene_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    )
    .unwrap();
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    host.inspector.joint = scene::armature(host.document.as_ref().unwrap())
        .unwrap()
        .1
        .iter()
        .position(|j| j.id == "left-hand")
        .unwrap();
    host.scene_event(
        &TheEvent::StateChanged(TheId::named("MZ Delete Part"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    )
    .unwrap();
    let root = Document::load(&p.procedural_appearances[&id]).unwrap();
    let e = root.evaluate().unwrap();
    assert!(e.mesh.rig.as_ref().unwrap().index("left-hand").is_err());
    assert_eq!(e.mesh.attachments.len(), 1);
}
#[test]
#[ignore = "requires a native GPU; exports generic creature authoring and textured shield"]
fn visual_authoring_gpu_review() {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let renderer = preview::Renderer::new(&device);
    let p = Project::default();
    let theme = TheBlackBlueTheme::new();
    let d = scene::quadruped();
    let e = d.evaluate().unwrap();
    let Value::Motion(m) = &e.rows["trot"] else {
        panic!()
    };
    std::fs::write(
        "/tmp/eldiron-visual-quadruped.mzgeometry.json",
        d.save().unwrap(),
    )
    .unwrap();
    let fit = Some(animation::bounds(&e.mesh, &[m]));
    let render =
        |name: &str, mesh: denrim_materials::geometry::Mesh, bones, framing, yaw, pitch| {
            let image = renderer
                .render(
                    &device,
                    &queue,
                    &preview::Job {
                        surface_seconds: 0.,
                        revision: 0,
                        mesh,
                        palette: p.palette.clone(),
                        width: 512,
                        height: 512,
                        yaw,
                        pitch,
                        background: *theme.color(MaterialCanvas),
                        grip: None,
                        axes: std::array::from_fn(|i| {
                            theme.palette_color(TheThemePalettes::MaterialCategories, i)
                        }),
                        framing,
                        bones,
                        attachment_frames: vec![],
                    },
                )
                .unwrap();
            std::fs::write(
                format!("/tmp/eldiron-visual-{name}.png"),
                image.to_png().unwrap(),
            )
            .unwrap();
            image
        };
    let mut image = render("creature", e.mesh.clone(), vec![], fit, -45., 12.);
    let scene = scene::Scene {
        tool: 1,
        projection: Some(scene::Projection::new(fit.unwrap(), 512, 512, -45., 12.)),
        ..Default::default()
    };
    scene.paint(&mut image, &d, &e, &rigging::Selection::default(), &theme);
    tests::workspace_snapshot_selected(
        &d,
        image,
        &rigging::Selection::default(),
        "armature",
        "/tmp/eldiron-visual-workspace.png",
    );
    for frame in 0..24 {
        let posed = e
            .mesh
            .posed(Some(m), m.duration * frame as f32 / 24.)
            .unwrap();
        render(
            &format!("trot-{frame:02}"),
            posed.mesh,
            vec![],
            fit,
            -45.,
            12.,
        );
    }
    let bones = e
        .mesh
        .rig
        .as_ref()
        .unwrap()
        .joints
        .iter()
        .enumerate()
        .filter_map(|(i, j)| {
            j.parent.map(|p| {
                (
                    e.mesh.rig.as_ref().unwrap().rest()[p].translation,
                    e.mesh.rig.as_ref().unwrap().rest()[i].translation,
                )
            })
        })
        .collect();
    render("creature-rig", e.mesh.clone(), bones, fit, -45., 12.);
    let h = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE)
        .unwrap()
        .evaluate()
        .unwrap();
    let Value::Mesh(shield) = &h.operations["shield-op-0"] else {
        panic!()
    };
    render("shield", shield.clone(), vec![], None, -15., 5.);
    eprintln!(
        "Generic creature: {} joints, {} parts, {} vertices",
        e.mesh.rig.as_ref().unwrap().joints.len(),
        e.mesh.parts.len(),
        e.mesh.vertex_count()
    );
}

#[test]
#[ignore = "requires a native GPU adapter"]
fn torch_particle_authoring_gpu_preview() {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let renderer = preview::Renderer::new(&device);
    let p = Project::default();
    let source = shared::rulesets::bundled_appearance_assets_for_ruleset(
        shared::rulesets::OFFICIAL_RULESET_ID,
        shared::rulesets::OFFICIAL_RULESET_VERSION,
    )
    .into_iter()
    .find(|a| a.id == "torch")
    .unwrap();
    let root = Document::load(source.source).unwrap();
    let doc = root.group_document(&["details-group".into()]).unwrap();
    let e = doc.evaluate().unwrap();
    let mut image = renderer
        .render(
            &device,
            &queue,
            &preview::Job {
                surface_seconds: 0.,
                revision: 1,
                mesh: e.mesh.clone(),
                palette: p.palette.clone(),
                width: 512,
                height: 512,
                yaw: 20.,
                pitch: 8.,
                background: [24, 22, 20, 255],
                grip: e.grip.clone(),
                axes: [[120, 120, 120, 255]; 3],
                framing: Some(rusterix::procedural_avatar::effects::preview_bounds(
                    &e.mesh,
                    e.mesh.bounds(),
                )),
                bones: vec![],
                attachment_frames: vec![],
            },
        )
        .unwrap();
    assert!(
        image
            .pixels()
            .chunks_exact(4)
            .any(|p| p[0] > 150 && p[0] as u16 > p[2] as u16 * 2),
        "flame must be visible"
    );
    let scene = scene::Scene {
        tool: 5,
        projection: Some(scene::Projection::new(
            rusterix::procedural_avatar::effects::preview_bounds(&e.mesh, e.mesh.bounds()),
            512,
            512,
            20.,
            8.,
        )),
        ..Default::default()
    };
    scene.paint(
        &mut image,
        doc,
        &e,
        &Default::default(),
        &TheBrownTheme::new(),
    );
    tests::workspace_snapshot_selected(
        doc,
        image,
        &Default::default(),
        "flame_tip",
        "/tmp/torch-particle-editor.png",
    );
}
