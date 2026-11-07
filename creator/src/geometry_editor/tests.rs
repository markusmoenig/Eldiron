use super::*;
fn sword() -> Document {
    Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap()
}
#[test]
fn worn_geometry_exposes_part_selection_and_offset_controls_and_saves_with_undo() {
    let asset = shared::rulesets::bundled_appearance_assets_for_ruleset(
        shared::rulesets::OFFICIAL_RULESET_ID,
        shared::rulesets::OFFICIAL_RULESET_VERSION,
    )
    .into_iter()
    .find(|a| a.id == "garment_linen_shirt")
    .unwrap();
    let mut doc = Document::load(asset.source).unwrap();
    let before = doc.save().unwrap();
    let evaluation = doc.evaluate().unwrap();
    let p = Project::default();
    let view = presentation::view(&doc, &evaluation, &p.palette);
    let selection = &view.rows[1].operations[1];
    assert!(
        selection
            .properties
            .iter()
            .any(|p| p.id == "selected:left-palm")
    );
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetNumber {
            operation: "shell-parts".into(),
            parameter: "selected:left-palm".into(),
            value: 1.,
        },
    )
    .unwrap();
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetNumber {
            operation: "shell-ease".into(),
            parameter: "distance".into(),
            value: 0.025,
        },
    )
    .unwrap();
    assert!(
        doc.evaluate()
            .unwrap()
            .mesh
            .parts
            .iter()
            .any(|p| p.id == "left-palm")
    );
    let after = doc.save().unwrap();
    let id = Uuid::parse_str(asset.asset_id).unwrap();
    let mut project = Project::default();
    project.procedural_appearances.insert(id, after.clone());
    let bytes = shared::project_io::encode_project(&project).unwrap();
    assert_eq!(
        shared::project_io::decode_project(&bytes)
            .unwrap()
            .procedural_appearances[&id],
        after
    );
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(800, 600, 1.);
    let mut server = ServerContext::default();
    let atom =
        ProjectUndoAtom::ProceduralAppearanceEdit(id, Some(before.clone()), Some(after.clone()));
    atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
    assert_eq!(project.procedural_appearances[&id], before);
    atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
    assert_eq!(project.procedural_appearances[&id], after);
}
#[test]
fn geometry_edits_save_in_project_and_undo_without_changing_world_tiles() {
    let id = Uuid::new_v4();
    let doc = sword();
    let before = doc.save().unwrap();
    let mut next = doc.clone();
    presentation::edit(
        &mut next,
        MaterialEditorCommand::SetNumber {
            operation: "blade-1".into(),
            parameter: "depth".into(),
            value: 0.04,
        },
    )
    .unwrap();
    let after = next.save().unwrap();
    let mut project = Project::default();
    project.procedural_appearances.insert(id, after.clone());
    let bytes = shared::project_io::encode_project(&project).unwrap();
    let reopened = shared::project_io::decode_project(&bytes).unwrap();
    assert_eq!(reopened.procedural_appearances[&id], after);
    let world = serde_json::to_value(&project.regions[0].map).unwrap();
    let tiles = project.tiles.clone();
    let atom =
        ProjectUndoAtom::ProceduralAppearanceEdit(id, Some(before.clone()), Some(after.clone()));
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(800, 600, 1.);
    let mut server = ServerContext::default();
    atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
    assert_eq!(project.procedural_appearances[&id], before);
    atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
    assert_eq!(project.procedural_appearances[&id], after);
    assert_eq!(project.tiles, tiles);
    assert_eq!(
        serde_json::to_value(&project.regions[0].map).unwrap(),
        world
    );
    let create = ProjectUndoAtom::ProceduralAppearanceEdit(id, None, Some(after));
    create.undo(&mut project, &mut ui, &mut ctx, &mut server);
    assert!(!project.procedural_appearances.contains_key(&id));
}
#[test]
fn geometry_view_has_typed_rows_palette_and_no_surface_settings() {
    let doc = sword();
    let evaluation = doc.evaluate().unwrap();
    let p = Project::default();
    let view = presentation::view(&doc, &evaluation, &p.palette);
    assert!(view.geometry);
    assert_eq!(view.rows.len(), 9);
    assert_eq!(
        view.outputs
            .iter()
            .map(|o| o.label.as_str())
            .collect::<Vec<_>>(),
        ["Mesh", "Grip"]
    );
    assert!(view.rows[3].label.contains("Mesh"));
    assert!(
        view.rows[3].operations[0]
            .properties
            .iter()
            .any(|p| p.label.starts_with("Point"))
    );
    assert_eq!(view.palette_bindings[0].index, 7);
    let mut changed = doc.clone();
    presentation::edit(
        &mut changed,
        MaterialEditorCommand::SetPaletteParameter {
            operation: "steel-0".into(),
            parameter: "color".into(),
            index: 41,
        },
    )
    .unwrap();
    assert_eq!(
        changed.evaluate().unwrap().mesh.parts[0]
            .surface
            .as_ref()
            .unwrap()
            .palette,
        41
    );
    let mut invalid = doc.clone();
    assert!(
        presentation::edit(
            &mut invalid,
            MaterialEditorCommand::ConnectOutput {
                output: "mesh".into(),
                row: "steel".into()
            }
        )
        .is_err()
    );
    assert_eq!(
        doc,
        sword(),
        "Candidates must never mutate the source document"
    );
}
#[test]
fn queued_geometry_commands_use_current_source_without_a_ui_refresh() {
    let id = Uuid::new_v4();
    let mut project = Project::default();
    project
        .procedural_appearances
        .insert(id, sword().save().unwrap());
    let mut host = GeometryEditor {
        selected: Some(id),
        document: Some(sword()),
        ..Default::default()
    };
    let mut ctx = TheContext::new(400, 800, 1.);
    host.apply_command(
        MaterialEditorCommand::SetNumber {
            operation: "blade-1".into(),
            parameter: "depth".into(),
            value: 0.05,
        },
        &mut project,
        &mut ctx,
    )
    .unwrap();
    host.apply_command(
        MaterialEditorCommand::SetNumber {
            operation: "blade-0".into(),
            parameter: "point3:1".into(),
            value: 1.1,
        },
        &mut project,
        &mut ctx,
    )
    .unwrap();
    let result = Document::load(&project.procedural_appearances[&id]).unwrap();
    assert!(matches!(result.rows[3].tiles[1].op,Operation::Extrude{depth} if depth==0.05));
    assert!(matches!(&result.rows[3].tiles[0].op,Operation::Contour{points} if points[3][1]==1.1));
}
#[test]
fn contour_editor_projects_drags_and_keeps_view_fixed() {
    let points = vec![[-0.1, 0.], [0.1, 0.], [0., 0.8]];
    let (image, frame) = presentation::contour_image(&points, 360, 240, &TheDarkTheme::new(), None);
    let f = frame.unwrap();
    assert_eq!(image.pixels().len(), 360 * 240 * 4);
    // Numeric and pointer paths use the same coordinate contract.
    let q = f.unproject(Vec2::new(180, 120));
    assert!((q[0]).abs() < 0.01);
    assert!((q[1] - 0.4).abs() < 0.01);
    let (_, kept) = presentation::contour_image(
        &[[0., 0.], [0.5, 0.], [0., 2.]],
        360,
        240,
        &TheDarkTheme::new(),
        Some(&f),
    );
    assert_eq!(
        f.unproject(Vec2::new(80, 80)),
        kept.unwrap().unproject(Vec2::new(80, 80))
    );
    let (_, small) = presentation::contour_image(&points, 64, 64, &TheDarkTheme::new(), None);
    let small = small.unwrap();
    let top = small.unproject(Vec2::new(32, 8));
    let bottom = small.unproject(Vec2::new(32, 56));
    assert!(
        (top[1] - bottom[1] - 0.8).abs() < 0.01,
        "Thumbnail contours need the same positive, fitted scale"
    );
}

#[test]
#[ignore = "requires a native GPU; exports geometry review images"]
fn geometry_gpu_preview_exports_sword_and_edit() {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let pipeline_started = std::time::Instant::now();
    let renderer = preview::Renderer::new(&device);
    eprintln!(
        "Geometry pipeline creation: {:?}",
        pipeline_started.elapsed()
    );
    let mut doc = sword();
    let palette = Project::default().palette;
    for (name, yaw, pitch) in [("front", 0., 0.), ("orbit", -35., 15.), ("side", 90., 0.)] {
        let started = std::time::Instant::now();
        let image = renderer
            .render(
                &device,
                &queue,
                &preview::Job {
                    surface_seconds: 0.,
                    revision: 0,
                    framing: None,
                    bones: vec![],
                    attachment_frames: vec![],
                    mesh: doc.evaluate().unwrap().mesh,
                    palette: palette.clone(),
                    width: 512,
                    height: 512,
                    yaw,
                    pitch,
                    background: [20, 23, 28, 255],
                    grip: doc.evaluate().unwrap().grip,
                    axes: [
                        [230, 100, 100, 255],
                        [100, 230, 100, 255],
                        [100, 130, 230, 255],
                    ],
                },
            )
            .unwrap();
        eprintln!(
            "{name}: {} vertices, {:?} including readback",
            doc.evaluate().unwrap().mesh.vertex_count(),
            started.elapsed()
        );
        save_png(
            &format!("/tmp/eldiron-geometry-{name}.png"),
            &image,
            512,
            512,
        );
        assert!(
            image
                .pixels()
                .chunks_exact(4)
                .filter(|p| p[..3] != [20, 23, 28])
                .count()
                > 1000
        );
    }
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetNumber {
            operation: "blade-0".into(),
            parameter: "point3:1".into(),
            value: 1.1,
        },
    )
    .unwrap();
    let image = renderer
        .render(
            &device,
            &queue,
            &preview::Job {
                surface_seconds: 0.,
                revision: 1,
                framing: None,
                bones: vec![],
                attachment_frames: vec![],
                mesh: doc.evaluate().unwrap().mesh,
                palette,
                width: 512,
                height: 512,
                yaw: -35.,
                pitch: 15.,
                background: [20, 23, 28, 255],
                grip: doc.evaluate().unwrap().grip,
                axes: [
                    [230, 100, 100, 255],
                    [100, 230, 100, 255],
                    [100, 130, 230, 255],
                ],
            },
        )
        .unwrap();
    save_png("/tmp/eldiron-geometry-edited.png", &image, 512, 512);
    workspace_snapshot(&doc, image);
}
fn workspace_snapshot(doc: &Document, image: TheRGBABuffer) {
    workspace_snapshot_selected(
        doc,
        image,
        &Default::default(),
        if doc.id == "editable-human" {
            "body-op-0"
        } else {
            "blade-0"
        },
        "/tmp/eldiron-geometry-workspace.png",
    );
}
pub(super) fn workspace_snapshot_selected(
    doc: &Document,
    image: TheRGBABuffer,
    selection: &rigging::Selection,
    operation: &str,
    path: &str,
) {
    let (mut ui, mut ctx) = viewport_tests::fixture(1400, 1000);
    ui.get_stack_layout(STACK).unwrap().set_index(1);
    ui.get_widget(MODE)
        .unwrap()
        .as_group_button()
        .unwrap()
        .set_index(1);
    ui.get_widget(ASSETS)
        .unwrap()
        .as_drop_down_menu()
        .unwrap()
        .clear_options();
    ui.get_widget(ASSETS)
        .unwrap()
        .as_drop_down_menu()
        .unwrap()
        .add_option(doc.label.clone());
    ui.get_widget("MZ Geometry Name")
        .unwrap()
        .set_value(TheValue::Text(doc.label.clone()));
    ui.get_widget("MZ Group Path")
        .unwrap()
        .as_text()
        .unwrap()
        .set_text(doc.label.clone());
    let p = Project::default();
    let evaluation = doc.evaluate().unwrap();
    let thumbnails = evaluation
        .operations
        .iter()
        .map(|(id, v)| {
            (
                id.clone(),
                presentation::thumbnail(v, &p.palette, ui.style.theme().as_ref()),
            )
        })
        .collect::<Vec<_>>();
    let w = ui
        .get_widget(EDITOR)
        .unwrap()
        .as_any()
        .downcast_mut::<MaterialEditorWidget>()
        .unwrap();
    w.set_view(presentation::view_selected(
        doc,
        &evaluation,
        &p.palette,
        selection,
    ));
    for (id, image) in thumbnails {
        w.set_thumbnail(id, image);
    }
    if scene::armature(doc).is_some_and(|(id, _)| id == operation) {
        layout::set_choice(&mut ui, "MZ Geometry Tool", 1);
        ui.get_stack_layout("Geometry Context Tools")
            .unwrap()
            .set_index(1);
        ui.get_widget("MZ Joint Name")
            .unwrap()
            .set_value(TheValue::Text(
                scene::armature(doc).unwrap().1[selection.joint]
                    .label
                    .clone(),
            ));
    }
    sync_workspace(&mut ui, &mut ctx);
    ui.canvas.set_dim(TheDim::new(0, 0, 1400, 1000), &mut ctx);
    let d = *ui.get_widget(PREVIEW).unwrap().dim();
    let mut padded = TheRGBABuffer::new(TheDim::new(0, 0, d.width, d.height));
    padded.fill(*ui.style.theme().color(MaterialCanvas));
    let scale = (d.width as f32 / image.dim().width as f32)
        .min(d.height as f32 / image.dim().height as f32);
    let image = image.scaled(
        (image.dim().width as f32 * scale) as i32,
        (image.dim().height as f32 * scale) as i32,
    );
    padded.copy_into(
        (d.width - image.dim().width) / 2,
        (d.height - image.dim().height) / 2,
        &image,
    );
    *ui.get_widget(PREVIEW)
        .unwrap()
        .as_render_view()
        .unwrap()
        .render_buffer_mut() = padded;
    ui.get_widget(EDITOR)
        .unwrap()
        .as_any()
        .downcast_mut::<MaterialEditorWidget>()
        .unwrap()
        .select_operation(operation);
    ui.get_widget("MZ Edit Group").unwrap().set_disabled(
        !doc.rows
            .iter()
            .flat_map(|r| &r.tiles)
            .any(|t| t.id == operation && matches!(t.op, Operation::Group { .. })),
    );
    ui.get_widget("MZ Parent Group").unwrap().set_disabled(true);
    let mut pixels = vec![0; 1400 * 1000 * 4];
    ui.draw(&mut pixels, &mut ctx);
    std::fs::write(
        path,
        TheRGBABuffer::from(pixels, 1400, 1000).to_png().unwrap(),
    )
    .unwrap();
    for id in [
        ASSETS,
        "MZ Geometry Name",
        "MZ New Geometry",
        "MZ Duplicate Geometry",
        "MZ Delete Geometry",
        "MZ Edit Group",
        "MZ Parent Group",
    ] {
        let dim = ui.get_widget(id).unwrap().dim();
        assert!(
            dim.width > 0 && dim.x >= 0 && dim.x + dim.width <= 1400,
            "{id}: {dim:?}"
        );
    }
}
fn save_png(path: &str, image: &TheRGBABuffer, w: u32, h: u32) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(image.pixels())
        .unwrap();
}

#[test]
fn human_controls_edit_profiles_bindings_and_motion_without_touching_tiles() {
    let mut doc = Document::load(denrim_materials::geometry::HUMAN_DETAIL_EXAMPLE).unwrap();
    let before = doc.evaluate().unwrap().mesh.vertex_count();
    for (operation, parameter, value, selection) in [
        (
            "body-torso-op-0",
            "radius0",
            0.35,
            rigging::Selection {
                section: 2,
                ..Default::default()
            },
        ),
        ("right-hand-op-0", "position1", -0.26, Default::default()),
        (
            "attack-op-0",
            "rotation0",
            -85.,
            rigging::Selection {
                track: 1,
                key: 2,
                ..Default::default()
            },
        ),
    ] {
        presentation::edit_selected(
            &mut doc,
            MaterialEditorCommand::SetNumber {
                operation: operation.into(),
                parameter: parameter.into(),
                value,
            },
            &selection,
        )
        .unwrap();
    }
    assert_eq!(doc.evaluate().unwrap().mesh.vertex_count(), before);
    let project = Project::default();
    let view = presentation::view_selected(
        &doc,
        &doc.evaluate().unwrap(),
        &project.palette,
        &rigging::Selection {
            section: 2,
            track: 1,
            key: 2,
            ..Default::default()
        },
    );
    assert!(view.sources.iter().any(|s| s.id == "skin"));
    let loft = &view
        .rows
        .iter()
        .find(|r| r.id == "body-torso")
        .unwrap()
        .operations[0];
    assert_eq!(
        loft.parameters
            .iter()
            .find(|n| n.id == "radius0")
            .unwrap()
            .value,
        0.35
    );
    let id = Uuid::new_v4();
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    let mut p = Project::default();
    p.procedural_appearances.insert(id, doc.save().unwrap());
    let mut ctx = TheContext::new(400, 1000, 1.);
    let saved = p.procedural_appearances[&id].clone();
    host.apply_command(
        MaterialEditorCommand::SetNumber {
            operation: "body-torso-op-0".into(),
            parameter: "@section".into(),
            value: 3.,
        },
        &mut p,
        &mut ctx,
    )
    .unwrap();
    assert_eq!(
        saved, p.procedural_appearances[&id],
        "Inspector selection must not create a source edit"
    );
    assert_eq!(host.inspector.section, 3);
}

#[test]
fn motion_actions_and_profile_edits_stay_valid_and_survive_saving() {
    let mut doc = Document::load(denrim_materials::geometry::HUMAN_DETAIL_EXAMPLE).unwrap();
    let edit = |doc: &mut Document, key: &str, value: f32, selection: &rigging::Selection| {
        presentation::edit_selected(
            doc,
            MaterialEditorCommand::SetNumber {
                operation: "idle-op-0".into(),
                parameter: key.into(),
                value,
            },
            selection,
        )
        .unwrap()
    };
    let selection = rigging::Selection::default();
    edit(&mut doc, "rotation0", 3., &selection);
    edit(&mut doc, "visible", 0., &selection);
    let clip = doc.rows.iter().find(|r| r.id == "idle").unwrap();
    let Operation::Motion { tracks, .. } = &clip.tiles[0].op else {
        panic!()
    };
    assert_eq!(
        tracks[0].keys[0].rotation,
        tracks[0].keys.last().unwrap().rotation
    );
    assert!(!tracks[0].keys[0].visible);
    assert!(!tracks[0].keys.last().unwrap().visible);
    edit(&mut doc, "track_action", 1., &selection);
    edit(&mut doc, "track_action", 3., &selection);
    let selection = rigging::Selection {
        key: 1,
        ..Default::default()
    };
    edit(&mut doc, "track_action", 4., &selection);
    let op = &mut doc
        .rows
        .iter_mut()
        .find(|r| r.id == "body-torso")
        .unwrap()
        .tiles[0]
        .op;
    rigging::move_point(op, 1, [-0.18, 1.12]).unwrap();
    rigging::ring_context(op, None, [-0.19, 1.20]).unwrap();
    doc.evaluate().unwrap();
    let saved = doc.save().unwrap();
    assert_eq!(Document::load(&saved).unwrap(), doc);
}

#[test]
fn grouped_controls_use_real_ranges_and_palette_pickers() {
    let mut doc = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap();
    let p = Project::default();
    let e = doc.evaluate().unwrap();
    let view = presentation::view(&doc, &e, &p.palette);
    assert_eq!(view.rows.len(), 7);
    let body = &view.rows[0].operations[0];
    assert_eq!(body.label, "Body");
    let height = body
        .parameters
        .iter()
        .find(|n| n.id == "control:height")
        .unwrap();
    assert_eq!((height.value, height.min, height.max), (1.8, 1.35, 2.2));
    let view = presentation::view_selected(
        &doc,
        &e,
        &p.palette,
        &rigging::Selection {
            category: 1,
            ..Default::default()
        },
    );
    let body = &view.rows[0].operations[0];
    assert!(
        body.properties
            .iter()
            .any(|p| p.id == "control:shirt-color" && p.kind == MaterialPropertyKind::Color)
    );
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetPaletteParameter {
            operation: "body-op-0".into(),
            parameter: "control:shirt-color".into(),
            index: 51,
        },
    )
    .unwrap();
    assert_eq!(
        doc.evaluate()
            .unwrap()
            .mesh
            .parts
            .iter()
            .find(|p| p.id == "body-torso")
            .unwrap()
            .surface
            .as_ref()
            .unwrap()
            .palette,
        51
    );
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetNumber {
            operation: "body-op-0".into(),
            parameter: "control:height".into(),
            value: 2.,
        },
    )
    .unwrap();
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetNumber {
            operation: "body-op-0".into(),
            parameter: "group_reset".into(),
            value: 2.,
        },
    )
    .unwrap();
    assert_eq!(
        doc,
        Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap()
    );
}

#[test]
fn group_details_preserve_parent_controls_and_undo_the_whole_recipe() {
    let mut p = Project::default();
    let id = Uuid::new_v4();
    let mut doc = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap();
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetNumber {
            operation: "body-op-0".into(),
            parameter: "control:height".into(),
            value: 2.,
        },
    )
    .unwrap();
    let before = doc.save().unwrap();
    p.procedural_appearances.insert(id, before.clone());
    let mut host = GeometryEditor {
        selected: Some(id),
        group_path: vec!["body-op-0".into()],
        ..Default::default()
    };
    let mut ctx = TheContext::new(400, 1000, 1.);
    host.apply_command(
        MaterialEditorCommand::SetNumber {
            operation: "body-profiles".into(),
            parameter: "profile_width".into(),
            value: 0.3,
        },
        &mut p,
        &mut ctx,
    )
    .unwrap();
    let after = p.procedural_appearances[&id].clone();
    let root = Document::load(&after).unwrap();
    assert_eq!(root.rows.len(), 7);
    let Operation::Group { controls, .. } = &root.rows[0].tiles[0].op else {
        panic!()
    };
    assert_eq!(
        controls.iter().find(|c| c.id == "height").unwrap().value,
        2.
    );
    let details = root.group_document(&["body-op-0".into()]).unwrap();
    let Operation::Profiles { parts, .. } = &details.rows[1].tiles[0].op else {
        panic!()
    };
    assert_eq!(parts[0].sections[0].radius[0], 0.15);
    let bytes = shared::project_io::encode_project(&p).unwrap();
    assert_eq!(
        shared::project_io::decode_project(&bytes)
            .unwrap()
            .procedural_appearances[&id],
        after
    );
    let atom =
        ProjectUndoAtom::ProceduralAppearanceEdit(id, Some(before.clone()), Some(after.clone()));
    let mut ui = TheUI::new();
    let mut server = ServerContext::default();
    atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
    assert_eq!(p.procedural_appearances[&id], before);
    atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
    assert_eq!(p.procedural_appearances[&id], after);
}

#[test]
fn group_navigation_opens_details_and_returns_to_compact_body_controls() {
    let mut p = Project::default();
    let mut ctx = TheContext::new(400, 1000, 1.);
    let mut ui = TheUI::new();
    let mut pages = TheStackLayout::new(TheId::named("Sidebar Page Stack"));
    for _ in 0..crate::procedural_material::SIDEBAR_PAGE {
        pages.add_canvas(TheCanvas::new());
    }
    pages.add_canvas(workspace(TheCanvas::new()));
    pages.set_index(crate::procedural_material::SIDEBAR_PAGE);
    ui.canvas.set_layout(pages);
    let mut host = GeometryEditor::default();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.handle_event(
        &TheEvent::StateChanged(TheId::named("MZ New Human"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
    assert_eq!(host.scene.tool, 1);
    assert_eq!(host.group_path, ["body-op-0"]);
    host.handle_event(
        &TheEvent::StateChanged(TheId::named("MZ Parent Group"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
    assert!(host.group_path.is_empty());
    assert!(
        host.document
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .any(|row| row.id == "bow-shot")
    );
    assert_eq!(host.selected_operation.as_deref(), Some("body-op-0"));
    assert!(!ui.get_widget("MZ Edit Group").unwrap().disabled());
    assert!(ui.get_widget("MZ Parent Group").unwrap().disabled());
    host.camera = 3;
    host.preview_selected = true;
    host.handle_event(
        &TheEvent::StateChanged(TheId::named("MZ Edit Group"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
    assert_eq!(host.group_path, ["body-op-0"]);
    assert_eq!(host.document.as_ref().unwrap().label, "Body details");
    assert_eq!(host.camera, 0);
    assert!(!host.preview_selected);
    assert!(ui.get_widget("MZ Edit Group").unwrap().disabled());
    assert!(!ui.get_widget("MZ Parent Group").unwrap().disabled());
    host.handle_event(
        &TheEvent::StateChanged(TheId::named("MZ Parent Group"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
    assert!(host.group_path.is_empty());
    assert!(
        host.document
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .any(|row| row.id == "spear-thrust")
    );
    let saved = p.procedural_appearances.values().next().unwrap().clone();
    assert_eq!(Document::load(&saved).unwrap(), host.root_document.unwrap());
}

#[test]
#[ignore = "requires a native GPU; exports human mesh and animated equipment review"]
fn human_gpu_review() {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let renderer = preview::Renderer::new(&device);
    let doc = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap();
    let e = doc.evaluate().unwrap();
    let motions = ["idle", "walk", "attack"].map(|id| {
        if let Value::Motion(m) = &e.rows[id] {
            m
        } else {
            panic!()
        }
    });
    let framing = Some(animation::bounds(&e.mesh, &motions));
    let p = Project::default();
    let theme = TheBlackBlueTheme::new();
    eprintln!(
        "Human: {} rows, {} vertices, {} parts",
        doc.rows.len(),
        e.mesh.vertex_count(),
        e.mesh.parts.len()
    );
    let mut timings = vec![];
    let mut render = |name: &str,
                      motion: Option<&denrim_materials::geometry::rig::Motion>,
                      phase: f32,
                      yaw: f32,
                      pitch: f32,
                      rig: bool,
                      fit: Option<([f32; 3], [f32; 3])>| {
        let started = std::time::Instant::now();
        let posed = e
            .mesh
            .posed(motion, motion.map_or(0., |m| m.duration * phase))
            .unwrap();
        let bones = if rig {
            e.mesh
                .rig
                .as_ref()
                .unwrap()
                .joints
                .iter()
                .enumerate()
                .filter_map(|(i, j)| {
                    j.parent.map(|parent| {
                        (
                            posed.joints[parent].translation,
                            posed.joints[i].translation,
                        )
                    })
                })
                .collect()
        } else {
            vec![]
        };
        let image = renderer
            .render(
                &device,
                &queue,
                &preview::Job {
                    surface_seconds: 0.,
                    revision: 0,
                    mesh: posed.mesh,
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
                    framing: fit,
                    bones,
                    attachment_frames: if rig {
                        posed.attachments.values().cloned().collect()
                    } else {
                        vec![]
                    },
                },
            )
            .unwrap();
        timings.push(started.elapsed().as_secs_f32() * 1000.);
        save_png(
            &format!("/tmp/eldiron-mz-human-{name}.png"),
            &image,
            512,
            512,
        );
        image
    };
    for (name, yaw, pitch) in [
        ("front", 0., 0.),
        ("orbit", -30., 8.),
        ("side", 90., 0.),
        ("back", 180., 0.),
    ] {
        render(name, None, 0., yaw, pitch, false, None);
    }
    render("rig", None, 0., -30., 8., true, framing);
    render(
        "face",
        None,
        0.,
        0.,
        0.,
        false,
        Some(([-0.145, 1.55, -0.12], [0.145, 1.84, 0.16])),
    );
    let image = render("workspace", None, 0., -30., 8., false, framing);
    for (clip, motion) in ["idle", "walk", "attack"].into_iter().zip(motions) {
        for i in 0..32 {
            render(
                &format!("{clip}-{i:02}"),
                Some(motion),
                i as f32 / 32.,
                -30.,
                8.,
                false,
                framing,
            );
        }
    }
    timings.sort_by(f32::total_cmp);
    eprintln!(
        "CPU posing + GPU mesh render + readback median {:.2} ms, p95 {:.2} ms",
        timings[timings.len() / 2],
        timings[timings.len() * 95 / 100]
    );
    workspace_snapshot(&doc, image);

    let fixed = Some(([-0.6, -0.1, -0.4], [0.6, 2.3, 0.5]));
    for (name, values) in [
        ("default", vec![]),
        (
            "short",
            vec![
                ("height", 1.5),
                ("leg-length", 85.),
                ("torso-length", 110.),
                ("shoulder-width", 125.),
                ("hip-width", 135.),
                ("head-size", 115.),
            ],
        ),
        (
            "tall",
            vec![
                ("height", 2.1),
                ("leg-length", 120.),
                ("arm-length", 110.),
                ("shoulder-width", 85.),
                ("hip-width", 85.),
                ("head-size", 90.),
            ],
        ),
    ] {
        let mut variant = doc.clone();
        for (id, value) in values {
            presentation::edit(
                &mut variant,
                MaterialEditorCommand::SetNumber {
                    operation: "body-op-0".into(),
                    parameter: format!("control:{id}"),
                    value,
                },
            )
            .unwrap();
        }
        let e = variant.evaluate().unwrap();
        let job = preview::Job {
            surface_seconds: 0.,
            revision: 0,
            mesh: e.mesh,
            palette: p.palette.clone(),
            width: 512,
            height: 512,
            yaw: -30.,
            pitch: 8.,
            background: *theme.color(MaterialCanvas),
            grip: None,
            axes: std::array::from_fn(|i| {
                theme.palette_color(TheThemePalettes::MaterialCategories, i)
            }),
            framing: fixed,
            bones: vec![],
            attachment_frames: vec![],
        };
        let image = renderer.render(&device, &queue, &job).unwrap();
        save_png(
            &format!("/tmp/eldiron-mz-group-{name}.png"),
            &image,
            512,
            512,
        );
        if name == "default" {
            workspace_snapshot_selected(
                &variant,
                image.clone(),
                &Default::default(),
                "body-op-0",
                "/tmp/eldiron-mz-group-body-ui.png",
            );
            workspace_snapshot_selected(
                &variant,
                image,
                &rigging::Selection {
                    category: 1,
                    ..Default::default()
                },
                "body-op-0",
                "/tmp/eldiron-mz-group-colors-ui.png",
            );
        }
    }
}

#[test]
fn particle_effect_source_controls_palette_save_and_visual_attachment() {
    let mut doc = sword();
    let output = doc.output.clone();
    let after = doc
        .rows
        .iter()
        .find(|r| r.id == output)
        .unwrap()
        .tiles
        .last()
        .unwrap()
        .id
        .clone();
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::AddSource {
            source: "particle_effect".into(),
            row: Some(output),
            after: Some(after),
        },
    )
    .unwrap();
    let tile = doc
        .rows
        .iter()
        .flat_map(|r| &r.tiles)
        .find(|t| matches!(t.op, Operation::ParticleEffect { .. }))
        .unwrap()
        .id
        .clone();
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetNumber {
            operation: tile.clone(),
            parameter: "position1".into(),
            value: 0.64,
        },
    )
    .unwrap();
    presentation::edit(
        &mut doc,
        MaterialEditorCommand::SetPaletteParameter {
            operation: tile.clone(),
            parameter: "effect_color1".into(),
            index: 17,
        },
    )
    .unwrap();
    let doc = Document::load(&doc.save().unwrap()).unwrap();
    let e = doc.evaluate().unwrap();
    let effect = &e.mesh.effects[0];
    assert_eq!(effect.definition.position[1], 0.64);
    assert_eq!(effect.definition.palette[1], 17);
    let view = presentation::view(&doc, &e, &ThePalette::default());
    assert!(view.sources.iter().any(|s| s.id == "particle_effect"));
    assert_eq!(
        view.palette_bindings
            .iter()
            .filter(|p| p.operation == tile)
            .count(),
        4
    );
    let scene = scene::Scene {
        tool: 5,
        ..Default::default()
    };
    assert!(scene.handles(&doc, &e, &Default::default()).iter().any(
        |(handle, p, _)| matches!(handle,scene::Handle::Grip(id) if id==&tile) && p[1] == 0.64
    ));
}
