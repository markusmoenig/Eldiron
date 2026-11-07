//! Exercise one MZ selection recipe in a map and a linked Prefab, with native previews.
//! cargo run -p eldiron-shared --example mz_selection_recipe -- /tmp/mz-selection
use eldiron_shared::{
    geometry_recipe::{self, Binding},
    project::Project,
    project_io::encode_project,
};
use rusterix::{
    BlockPropAsset, BlockPropGeometrySource, BlockPropPart, D3Camera, D3IsoCamera, GeometryObject,
    MapCamera, PixelSource, PlayerCamera, Rusterix, client::widget::game::GameWidget,
};
use theframework::prelude::*;

fn painted_box(name: &str, center: Vec3<f32>, size: Vec3<f32>, palette: u16) -> GeometryObject {
    let mut object = GeometryObject::box_(name, center, size);
    for face in &mut object.faces {
        face.tile = Some(PixelSource::PaletteIndex(palette));
    }
    object
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or("/tmp/mz-selection".into()),
    );
    std::fs::create_dir_all(&out)?;
    let mut project = Project::default();
    project.name = "MZ selection recipes".into();
    project.config="[game]\ncollision_mode = \"mesh\"\n[simulation]\nenabled = false\n[render]\nambient_strength = 0.55\nsun_enabled = true\nsun_intensity = 1.4\nsun_direction = [-0.5, -1.0, -0.3]\nsky_color = \"#9bb9c5\"\nsun_color = \"#fff0db\"\n".into();
    let choice = geometry_recipe::choices(&project).remove(0);
    project
        .procedural_appearances
        .insert(choice.0, choice.2.clone());
    let mut map = project.regions[0].map.clone();
    map.geometry_objects.clear();
    for (name, center, size) in [
        ("Post NW", [-3., 1.4, -2.], [0.18, 2.8, 0.18]),
        ("Post NE", [3., 1.4, -2.], [0.18, 2.8, 0.18]),
        ("Post SW", [-3., 1.4, 2.], [0.18, 2.8, 0.18]),
        ("Post SE", [3., 1.4, 2.], [0.18, 2.8, 0.18]),
        ("Beam north", [0., 2.7, -2.], [6.18, 0.2, 0.18]),
        ("Beam south", [0., 2.7, 2.], [6.18, 0.2, 0.18]),
        ("Beam east", [3., 2.7, 0.], [0.18, 0.2, 4.18]),
        ("Beam west", [-3., 2.7, 0.], [0.18, 0.2, 4.18]),
    ] {
        map.geometry_objects
            .push(painted_box(name, Vec3::from(center), Vec3::from(size), 18));
    }
    map.selected_geometry_objects = map.geometry_objects.iter().map(|o| o.id).collect();
    let binding = Binding::from_selection(&project, &map, choice.0)?;
    let roof = geometry_recipe::generate(&mut project, &map, &binding, None)?;
    // A Prop part uses the very same binding and generated object.
    let mut prefab = BlockPropAsset::new_authored("MZ timber frame", map.geometry_objects.clone());
    let mut part = BlockPropPart::new_authored("MZ roof", vec![]);
    part.geometry_source = BlockPropGeometrySource::MZ {
        binding: roof
            .properties
            .get_str(geometry_recipe::BINDING)
            .unwrap()
            .into(),
        generated_cache: vec![roof.clone()],
    };
    prefab.parts.push(part);
    project.block_props.insert(prefab.id, prefab);
    map.geometry_objects.push(roof.clone());
    map.geometry_objects.push(painted_box(
        "Ground",
        Vec3::new(0., -0.15, 0.),
        Vec3::new(18., 0.2, 18.),
        36,
    ));
    map.selected_geometry_objects = vec![roof.id];
    map.camera = MapCamera::ThreeDIso;
    project.regions[0].map = map.clone();
    project.regions[0].editing_look_at_3d = Vec3::new(0., 1.4, 0.);
    project.regions[0].editing_position_3d = Vec3::new(-10., 10., 10.);
    project.regions[0].editing_position_iso_3d = Some(project.regions[0].editing_position_3d);
    project.regions[0].editing_look_at_iso_3d = Some(project.regions[0].editing_look_at_3d);
    project.regions[0].editing_iso_scale = Some(6.);
    std::fs::write(
        out.join("MZSelectionRecipes.eldiron"),
        encode_project(&project)?,
    )?;
    std::fs::write(out.join("pitched-roof.mzgeometry.json"), choice.2)?;
    eprintln!(
        "Saved selection-driven map, recipe and linked Prefab to {}",
        out.display()
    );
    if std::env::var_os("ELDIRON_RECIPE_NO_RENDER").is_some() {
        return Ok(());
    }
    let mut runtime = Rusterix::default();
    runtime.assets.palette = project.palette.clone();
    runtime.set_tiles_for_maps(project.tiles.clone(), false, [&map]);
    runtime.scene_handler.settings.read(&project.config)?;
    let mut widget = GameWidget::new();
    widget.rect.width = 1000.;
    widget.rect.height = 760.;
    widget.buffer = TheRGBABuffer::new(TheDim::sized(1000, 760));
    widget.set_camera_mode(PlayerCamera::D3Iso);
    let mut camera = D3IsoCamera::new();
    camera.scale = 6.;
    camera.set_parameter_vec3("center", Vec3::new(0., 1.4, 0.));
    widget.camera_d3 = Box::new(camera);
    for name in ["fitted-roof", "wider-frame"] {
        if name == "wider-frame" {
            for object in &mut map.geometry_objects {
                if binding.objects.contains(&object.id) {
                    for point in &mut object.vertices {
                        point.x *= 1.4;
                    }
                }
            }
            let (changed, errors) = geometry_recipe::synchronize_map(&mut project, &mut map);
            assert!(changed && errors.is_empty(), "{errors:?}");
            assert_eq!(
                map.geometry_objects
                    .iter()
                    .find(|o| o.id == roof.id)
                    .unwrap()
                    .faces[0]
                    .id,
                roof.faces[0].id
            );
            map.changed = map.changed.wrapping_add(1);
        }
        for frame in 0..60 {
            widget.prepare_frame(
                &map,
                &project.time,
                frame / 12,
                &runtime.assets,
                &mut runtime.scene_handler,
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let mut pixels = vec![0; 1000 * 760 * 4];
        runtime
            .scene_handler
            .vm
            .render_frame(&mut pixels, 1000, 760);
        std::fs::write(
            out.join(format!("{name}.png")),
            TheRGBABuffer::from(pixels, 1000, 760).to_png()?,
        )?;
    }
    Ok(())
}
