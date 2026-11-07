#![cfg(feature = "integration")]
use denrim_materials::{cpu::Evaluator, format, integration::*, ir::Channel};
const SOURCE: &str = "project 1\nmaterial \"grass\" called \"Grass\"\n  mode pbr\n  tiling raw\n  row \"color\"\n    use \"Color\" id \"grass-color\"\n      set \"color\" to 0.1 0.2 0.3 1\n    use \"Color\" id \"another-color\"\n  row \"unused\"\n    use \"Color\" id \"unused-color\"\n  output base color from row \"color\" xyz\n";
#[test]
fn persistent_keys_survive_structural_edits_and_reopen() {
    let mut asset = EditableMaterialAsset::parse("asset-grass", SOURCE, "grass").unwrap();
    let key = ParameterKey::new("asset-grass", "grass-color", "color");
    let mut instance = InstanceRecord::new("placement-1");
    instance.set(key.clone(), vec![0.7, 0.6, 0.5, 1.]);
    instance.palette_bindings.push(PaletteBinding {
        key: key.clone(),
        index: 7,
    });
    asset.instances.push(instance.clone());
    let rows = &mut asset.root.project.materials[0].rows;
    rows[0].tiles.reverse();
    let mut inserted = rows[0].tiles[0].clone();
    inserted.id = format::new_tile_id();
    rows[0].tiles.insert(0, inserted);
    rows.reverse();
    rows[0].tiles.clear();
    rows.remove(0);
    asset.validate().unwrap();
    let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
    assert_eq!(reopened.instances, asset.instances);
    assert_eq!(reopened.root.project, asset.root.project);
    assert_eq!(
        reopened.locate(&key).unwrap().tile.as_deref(),
        Some("grass-color")
    );
    let p = reopened.program(&instance).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample("grass", Channel::BaseColor, [0., 0.])
            .unwrap(),
        vec![0.7, 0.6, 0.5]
    );
    let mut deleted = reopened;
    deleted.root.project.materials[0].rows[0]
        .tiles
        .retain(|t| t.id != "grass-color");
    let error = deleted.program(&instance).unwrap_err();
    assert_eq!(error.location.tile.as_deref(), Some("grass-color"));
    assert!(error.message.contains("Unresolved"));
}
#[test]
fn legacy_ids_migrate_once_and_duplicates_are_rejected() {
    let asset = EditableMaterialAsset::parse(
        "legacy",
        &SOURCE
            .replace(" id \"grass-color\"", "")
            .replace(" id \"another-color\"", "")
            .replace(" id \"unused-color\"", ""),
        "grass",
    )
    .unwrap();
    let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
    assert_eq!(asset.root.project, reopened.root.project);
    assert!(format::parse(&SOURCE.replace("another-color", "grass-color")).is_err());
    let mut invalid = asset;
    invalid.root.project.materials[0].rows[0].tiles[1].id =
        invalid.root.project.materials[0].rows[0].tiles[0]
            .id
            .clone();
    assert!(invalid.validate().is_err());
}
#[test]
fn editable_bundle_preserves_dependencies_custom_code_and_instance_values() {
    let mut asset=EditableMaterialAsset::parse("root","project 1\nmaterial \"result\" called \"Result\"\n  mix \"Mask\"\n  mixing input 1 from material \"grass\"\n  mixing input 2 from material \"road\"\n  mixing mask to 0.5 0.5 0.5\n","result").unwrap();
    let mut dependency = EditableDocument::parse("asset-grass", SOURCE).unwrap();
    let color = denrim_materials::metadata::Kernel::Color;
    dependency.project.materials[0].rows[0].tiles[0].code =
        Some(denrim_materials::tilelang::source(color).into());
    asset.dependencies.push(dependency);
    asset.dependencies.push(
        EditableDocument::parse(
            "asset-road",
            "project 1\nmaterial \"road\" called \"Road\"\n  default base color to 0.2 0.2 0.2\n",
        )
        .unwrap(),
    );
    let mut instance = InstanceRecord::new("placement");
    instance.set(
        ParameterKey::new("asset-grass", "grass-color", "color"),
        vec![0.2, 0.3, 0.4, 1.],
    );
    asset.instances.push(instance);
    asset.validate().unwrap();
    let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
    assert_eq!(reopened.dependencies, asset.dependencies);
    assert_eq!(reopened.instances, asset.instances);
    assert_eq!(reopened.selected_material, "result");
    reopened.validate().unwrap();
    assert_eq!(reopened.source_catalog().len(), 87);
    assert!(
        reopened
            .parameter_metadata()
            .unwrap()
            .iter()
            .any(|p| p.key.tile == "grass-color"
                && p.key.parameter == "color"
                && p.definition.kind == denrim_materials::metadata::ValueType::Color)
    );
    let mut missing = reopened;
    missing.dependencies.pop();
    assert!(missing.validate().is_err());
}
#[test]
fn editor_metadata_is_available_for_an_invalid_algorithm() {
    let mut asset = EditableMaterialAsset::parse("asset-grass", SOURCE, "grass").unwrap();
    let t = &mut asset.root.project.materials[0].rows[0].tiles[0];
    t.code = Some(format!(
        "{}\nimage = missing_variable\n",
        denrim_materials::tilelang::source(denrim_materials::metadata::Kernel::Color)
    ));
    assert!(!asset.parameter_metadata().unwrap().is_empty());
    let error = asset.validate().unwrap_err();
    assert_eq!(error.location.tile.as_deref(), Some("grass-color"));
    assert_eq!(error.location.row.as_deref(), Some("color"));
    let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
    assert_eq!(reopened.root.project, asset.root.project);
}

#[test]
fn local_tile_ids_in_different_dependencies_do_not_alias() {
    let mut asset=EditableMaterialAsset::parse("root","project 1\nmaterial \"result\" called \"Mix\"\n  mix \"Mask\"\n  mixing input 1 from material \"a\"\n  mixing input 2 from material \"b\"\n  mixing mask to 0.5 0.5 0.5\n","result").unwrap();
    for (id, color) in [("a", 0.2), ("b", 0.4)] {
        let source = format!(
            "project 1\nmaterial \"{id}\" called \"{id}\"\n  row \"color\"\n    use \"Color\" id \"pigment\"\n      set \"color\" to {color} {color} {color} 1\n  output base color from row \"color\" xyz\n"
        );
        asset
            .dependencies
            .push(EditableDocument::parse(id, &source).unwrap());
    }
    let mut record = InstanceRecord::new("placement");
    record.set(
        ParameterKey::new("a", "pigment", "color"),
        vec![0.8, 0.8, 0.8, 1.],
    );
    let program = asset.program(&record).unwrap();
    let value = Evaluator::new(&program)
        .sample("result", Channel::BaseColor, [0., 0.])
        .unwrap();
    assert!(value.iter().all(|v| (v - 0.6).abs() < 1e-6));
    asset.instances.push(record);
    let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
    assert_eq!(reopened.dependencies, asset.dependencies);
    reopened.validate().unwrap();
}

#[test]
fn material_default_colors_have_persistent_palette_keys_and_accept_typed_overrides() {
    let mut asset = EditableMaterialAsset::parse(
        "asset",
        r#"project 1
material "surface" called "Surface"
  mode pbr
"#,
        "surface",
    )
    .unwrap();
    let mut instance = InstanceRecord::new("palette");
    let key = ParameterKey::new("asset", "@material:surface", "base color");
    instance.palette_bindings.push(PaletteBinding {
        key: key.clone(),
        index: 41,
    });
    instance.set(key.clone(), vec![0.2, 0.3, 0.4, 1.]);
    asset.instances.push(instance.clone());
    let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
    assert_eq!(
        reopened.locate(&key).unwrap().material.as_deref(),
        Some("surface")
    );
    assert_eq!(
        Evaluator::new(&reopened.program(&instance).unwrap())
            .sample("surface", Channel::BaseColor, [0., 0.])
            .unwrap(),
        vec![0.2, 0.3, 0.4]
    );
}

#[test]
fn release_numbers_do_not_invalidate_unchanged_material_definitions() {
    let asset = EditableMaterialAsset::parse("version-test", SOURCE, "grass").unwrap();
    let mut bundle: serde_json::Value = serde_json::from_str(&asset.save().unwrap()).unwrap();
    let library = source_library_version();
    let hash = library.rsplit('-').next().unwrap();
    bundle["source_library"] = format!("materialz-999.0.0-{hash}").into();
    let reopened = EditableMaterialAsset::reopen(&bundle.to_string()).unwrap();
    assert_eq!(reopened.root.project, asset.root.project);
    bundle["source_library"] = "materialz-999.0.0-different-definitions".into();
    assert!(EditableMaterialAsset::reopen(&bundle.to_string()).is_err());
    bundle["source_library"] = library.into();
    bundle["version"] = 999.into();
    assert!(EditableMaterialAsset::reopen(&bundle.to_string()).is_err());
}
