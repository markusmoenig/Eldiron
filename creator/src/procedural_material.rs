//! MaterialZ is the authoring authority; normal tiles are the baked runtime cache.
//! GPU compilation/readback lives on a worker sharing SceneVM's device and queue.
use crate::{
    editor::{RUSTERIX, UNDOMANAGER},
    material_editor::*,
    prelude::*,
};
use denrim_materials::{
    format::{self, Binding, Reference},
    integration::{EditableMaterialAsset, source_catalog},
};

const EDITOR: &str = "Procedural Material Editor";
pub(crate) const SIDEBAR_PAGE: usize = 6;
pub(crate) const PREVIEW_STACK: &str = "Sidebar Preview Stack";
const PREVIEW: &str = "Material Preview";
pub(crate) mod composition;
pub(crate) mod image_source;
pub(crate) mod map_composition;
#[cfg(not(target_arch = "wasm32"))]
mod model_source;
pub(crate) mod palette;
pub(crate) mod pixel_style;
mod row_inputs;
pub(crate) mod starters;
#[cfg(test)]
mod woodland_demo;
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PreviewSelection {
    Operation(String),
    Source(String),
    Material(String),
    Input(MaterialInputTarget, MaterialReferenceView),
}

const CHANNELS: [&str; 5] = ["base color", "roughness", "metallic", "opacity", "emission"];
const LABELS: [&str; 5] = ["Color", "Roughness", "Metallic", "Opacity", "Emission"];
// The runtime texture layout remains Color/Opacity/Roughness/Metallic/Emission.
const PREVIEW_MAPS: [usize; 5] = [0, 2, 3, 1, 4];
#[cfg(test)]
const STARTER: &str = r#"project 1
material "surface" called "Procedural Tile"
  mode pbr
  tiling raw
  default roughness to 0.6
  row "color" called "Base Color"
    use "Perlin Noise" id "color-noise"
    use "Mix" id "pigment"
      set "color1" to 0.08 0.16 0.025 1
      set "color2" to 0.4 0.6 0.15 1
      set "factor" to 1 1 1
  row "grain" called "Roughness"
    use "Perlin Noise" id "grain-noise"
  output base color from row "color" xyz
  output roughness from row "grain" x
"#;

// Vector paths use the same rasterizer, size and tint as the sidebar navigation.
const PREVIEW_ICONS: [(&str, &str); 22] = [
    ("mz-add", "M10 2H14V10H22V14H14V22H10V14H2V10H10Z"),
    (
        "mz-profile",
        "M3 3H5V21H3Z M8 5L20 2V22L8 19Z M10 7V17L18 19V5Z",
    ),
    (
        "mz-pose",
        "M10 1H14V5H10Z M11 6H13V12L20 19L18 21L12 15L6 21L4 19L10 12V9H3V7H11Z",
    ),
    (
        "mz-mirror",
        "M11 1H13V23H11Z M1 5L9 2V22L1 19Z M23 5L15 2V22L23 19Z",
    ),
    (
        "mz-group",
        "M2 5H10L13 8H22V21H2Z M11 11H14V14H18V17H14V20H11V17H7V14H11Z",
    ),
    ("mz-back", "M12 3L3 12L12 21V15H22V9H12Z"),
    ("mz-copy", "M8 2H22V17H19V5H8Z M2 7H17V22H2Z M4 9V20H15V9Z"),
    (
        "mz-trash",
        "M4 5H20V7H4Z M7 8H17V22H7Z M9 10V20H11V10Z M13 10V20H15V10Z M9 2H15V4H9Z",
    ),
    ("mz-play", "M6 3L21 12L6 21Z"),
    ("mz-pause", "M5 3H10V21H5Z M14 3H19V21H14Z"),
    (
        "mz-rig",
        "M10 2H14V6H10Z M11 7H13V14H11Z M3 8H21V10H3Z M11 13H13L19 22H16L12 16L8 22H5Z",
    ),
    (
        "mz-geometry",
        "M12 2L22 7V17L12 22L2 17V7Z M3 7L12 12L21 7 M12 12V21",
    ),
    (
        "mz-output",
        "M3 6L12 2L21 6L12 10Z M3 10L12 14L21 10V13L12 17L3 13Z M3 16L12 20L21 16V19L12 23L3 19Z",
    ),
    ("mz-selected", "M3 3H21V21H3Z M5 5V19H19V5Z M9 9H15V15H9Z"),
    (
        "mz-color",
        "M12 2C6.5 2 2 6.5 2 12S6.5 22 12 22H14C17 22 18 19 16 17L15 16C14.6 15.5 15 14 16 14H18C21 14 22 12.5 22 10C22 5.5 17.5 2 12 2Z M12 4C16.5 4 20 6.5 20 10C20 11.5 19.5 12 18 12H16C13 12 11.7 15.5 13.5 17.4L14.5 18.4C15.3 19.2 15 20 14 20H12C7.5 20 4 16.5 4 12S7.5 4 12 4Z M6 9H9V12H6Z M10 6H13V9H10Z M15 7H18V10H15Z",
    ),
    (
        "mz-opacity",
        "M12 2A10 10 0 1 0 12 22A10 10 0 1 0 12 2Z M12 4A8 8 0 0 1 12 20Z",
    ),
    (
        "mz-roughness",
        "M3 3H6V6H3Z M10.5 3H13.5V6H10.5Z M18 3H21V6H18Z M3 10.5H6V13.5H3Z M10.5 10.5H13.5V13.5H10.5Z M18 10.5H21V13.5H18Z M3 18H6V21H3Z M10.5 18H13.5V21H10.5Z M18 18H21V21H18Z",
    ),
    (
        "mz-metallic",
        "M12 2L22 12L12 22L2 12Z M12 5L5 12H12Z M13 13V19L19 13Z",
    ),
    (
        "mz-emission",
        "M12 7A5 5 0 1 0 12 17A5 5 0 1 0 12 7Z M11 1H13V5H11Z M11 19H13V23H11Z M1 11H5V13H1Z M19 11H23V13H19Z M3 4L4 3L7 6L6 7Z M17 18L18 17L21 20L20 21Z M17 6L20 3L21 4L18 7Z M3 20L6 17L7 18L4 21Z",
    ),
    ("mz-single", "M3 3H21V21H3Z M5 5V19H19V5Z"),
    (
        "mz-seamless",
        "M3 3H10V5H5V10H3Z M14 3H21V10H19V5H14Z M19 14H21V21H14V19H19Z M3 14H5V19H10V21H3Z M8 11H16V13H8Z M11 8H13V16H11Z",
    ),
    (
        "mz-continuous",
        "M7 6C0 6 0 18 7 18C10 18 12 15 14 12L12 10C9 14 8 16 7 16C3 16 3 8 7 8C8 8 9 10 12 14L14 12C12 9 10 6 7 6Z M17 6C24 6 24 18 17 18C14 18 12 15 10 12L12 10C15 14 16 16 17 16C21 16 21 8 17 8C16 8 15 10 12 14L10 12C12 9 14 6 17 6Z",
    ),
];
pub(crate) fn register_preview_icons(ctx: &mut TheContext, theme: &dyn TheTheme) {
    for (name, path) in PREVIEW_ICONS {
        ctx.ui.add_icon(
            name.into(),
            rasterize_svg_path_icon(path, 18, 24., *theme.color(MaterialText)),
        );
    }
}

pub fn material_canvas() -> TheCanvas {
    let mut material = TheCanvas::default();
    let mut widget = MaterialEditorWidget::new(TheId::named(EDITOR));
    widget.set_view(MaterialEditorView {
        title: "Select a procedural tile".into(),
        ..Default::default()
    });
    material.set_widget(widget);
    let mut stack = TheStackLayout::new(TheId::named(composition::STACK));
    stack.add_canvas(material);
    stack.add_canvas(composition::canvas());
    let mut canvas = TheCanvas::default();
    canvas.set_layout(stack);
    let mut library = TheDropdownMenu::new(TheId::named("Ruleset Material"));
    library.add_option("Tile material".into());
    for def in shared::ruleset_materials::definitions(shared::rulesets::DEFAULT_RULESET_CONFIG) {
        library.add_option(format!("Ruleset · {}", def.id.replace('-', " ")));
    }
    library.limiter_mut().set_max_width(240);
    let mut row = TheHLayout::new(TheId::empty());
    row.limiter_mut().set_max_height(28);
    row.set_margin(Vec4::new(6, 4, 6, 4));
    row.add_widget(Box::new(library));
    let mut back = TheTraybarButton::new(TheId::named("MZ Return to Mesh"));
    back.set_text("Back to mesh".into());
    back.set_status_text("Return to the mesh recipe that uses this material");
    back.set_disabled(true);
    row.add_widget(Box::new(back));
    let mut top = TheCanvas::new();
    top.set_layout(row);
    canvas.set_top(top);
    crate::geometry_editor::workspace(canvas)
}

pub fn minimap_canvas(normal: TheCanvas) -> TheCanvas {
    let mut stack = TheStackLayout::new(TheId::named(PREVIEW_STACK));
    stack.add_canvas(normal);
    stack.add_canvas(preview_canvas());
    let mut canvas = TheCanvas::default();
    canvas.set_layout(stack);
    canvas
}

pub fn preview_canvas() -> TheCanvas {
    let mut canvas = TheCanvas::default();
    canvas.set_widget(TheRenderView::new(TheId::named(PREVIEW)));
    let mut channel = TheGroupButton::new(TheId::named("Material Preview Channel"));
    for (icon, status) in [
        ("mz-color", "Color"),
        ("mz-roughness", "Roughness"),
        ("mz-metallic", "Metallic"),
        ("mz-opacity", "Opacity"),
        ("mz-emission", "Emission"),
    ] {
        channel.add_text_status_icon(String::new(), status.into(), icon.into());
    }
    channel.set_item_width(28);
    let mut target = TheGroupButton::new(TheId::named("Material Preview Target"));
    target.add_text_status_icon(String::new(), "Material output".into(), "mz-output".into());
    target.add_text_status_icon(
        String::new(),
        "Selected operation, source or composition layer".into(),
        "mz-selected".into(),
    );
    target.set_item_width(28);
    let mut domain = TheGroupButton::new(TheId::named("Material Tiling"));
    domain.add_text_status_icon(
        String::new(),
        "Continuous: world coordinates vary the material across the map (saved material setting)"
            .into(),
        "mz-continuous".into(),
    );
    domain.add_text_status_icon(
        String::new(),
        "Repeat: repeat the material's pattern period across the map (saved material setting)"
            .into(),
        "mz-single".into(),
    );
    domain.add_text_status_icon(
        String::new(),
        "Seamless: blend pattern edges for repeating tiles (saved material setting)".into(),
        "mz-seamless".into(),
    );
    domain.set_item_width(28);
    let mut layout = TheHLayout::new(TheId::empty());
    layout.set_background_color(None);
    layout.set_margin(Vec4::new(6, 2, 6, 2));
    layout.set_padding(4);
    layout.add_widget(Box::new(target));
    layout.add_widget(Box::new(channel));
    layout.add_widget(Box::new(domain));
    let mut toolbar = TheCanvas::default();
    toolbar.set_widget(TheTraybar::new(TheId::empty()));
    toolbar.set_layout(layout);
    canvas.set_top(toolbar);
    canvas.top_is_expanding = false;
    canvas
}

#[cfg(test)]
fn starter(id: Uuid) -> Result<EditableMaterialAsset, String> {
    let asset = EditableMaterialAsset::parse(&id.to_string(), STARTER, "surface")
        .map_err(|e| e.to_string())?;
    asset.validate().map_err(|e| e.to_string())?;
    Ok(asset)
}
pub(crate) fn duplicate_bundle(bundle: &str, id: Uuid) -> Result<String, String> {
    let mut asset = EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?;
    let old = asset.root.id.clone();
    asset.root.id = id.to_string();
    for instance in &mut asset.instances {
        for value in &mut instance.overrides {
            if value.key.asset == old {
                value.key.asset = asset.root.id.clone();
            }
        }
        for value in &mut instance.palette_bindings {
            if value.key.asset == old {
                value.key.asset = asset.root.id.clone();
            }
        }
    }
    asset.validate().map_err(|e| e.to_string())?;
    asset.save().map_err(|e| e.to_string())
}
/// A temporary evaluation target for the selected operation. Keep the authored
/// document untouched, including the complete row used by the baked material.
fn source_preview_asset(source_id: &str) -> Result<EditableMaterialAsset, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let alias = (source_id == model_source::SOURCE).then(|| {
        source_catalog()
            .into_iter()
            .find(|s| s.metadata.name == "Image / Tile Source")
            .unwrap()
            .id
    });
    #[cfg(not(target_arch = "wasm32"))]
    let source_id = alias.as_deref().unwrap_or(source_id);
    let source = source_catalog()
        .into_iter()
        .find(|source| source.id == source_id)
        .ok_or("Unknown procedural source")?;
    let mut asset = EditableMaterialAsset::parse(
        "procedural-source-preview",
        r#"project 1
material "preview" called "Source Preview"
  mode pbr
  tiling raw
  row "source"
    use "Color" id "preview-source"
  output base color from row "source" xyz
  output opacity from row "source" w
"#,
        "preview",
    )
    .map_err(|e| e.to_string())?;
    let row = &mut asset.root.project.materials[0].rows[0];
    row.tiles[0].kernel = source.metadata.name;
    if source.requires_input {
        row.tiles.insert(
            0,
            format::Tile {
                id: "preview-input".into(),
                kernel: "Perlin Noise".into(),
                parameters: vec![],
                input: None,
                input_disconnected: false,
                code: None,
                image: None,
                combine: format::Combine::Replace,
            },
        );
    }
    asset.validate().map_err(|e| e.to_string())?;
    Ok(asset)
}

fn operation_preview_asset(
    asset: &EditableMaterialAsset,
    operation: &str,
) -> Result<EditableMaterialAsset, String> {
    let mut preview = asset.clone();
    preview.instances.clear();
    let mat = preview
        .root
        .project
        .materials
        .iter_mut()
        .find(|mat| mat.id == asset.selected_material)
        .ok_or("Material is missing")?;
    let row = mat
        .rows
        .iter_mut()
        .find(|row| row.tiles.iter().any(|tile| tile.id == operation))
        .ok_or("Select a tile in the material editor")?;
    let at = row
        .tiles
        .iter()
        .position(|tile| tile.id == operation)
        .unwrap();
    row.tiles.truncate(at + 1);
    let row = row.id.clone();
    mat.outputs = vec![
        format::Output {
            channel: "base color".into(),
            source: Reference {
                row: row.clone(),
                components: "xyz".into(),
                mapping: None,
            },
        },
        format::Output {
            channel: "opacity".into(),
            source: Reference {
                row,
                components: "w".into(),
                mapping: None,
            },
        },
    ];
    preview.validate().map_err(|e| e.to_string())?;
    Ok(preview)
}

/// Preview the row port itself, including its swizzle and remap. Its receiver may
/// have been deleted, so this must not depend on any receiving operation.
fn input_preview_asset(
    asset: &EditableMaterialAsset,
    _target: &MaterialInputTarget,
    reference: &MaterialReferenceView,
) -> Result<EditableMaterialAsset, String> {
    use denrim_materials::metadata::Kernel;
    let mut preview = asset.clone();
    preview.instances.clear();
    let mat = preview
        .root
        .project
        .materials
        .iter_mut()
        .find(|mat| mat.id == asset.selected_material)
        .ok_or("Material is missing")?;
    let mut source = core_reference(&MaterialInputBinding::Row(reference.clone()), 1)?
        .ok_or("Input is not connected")?;
    mat.mixing = None;
    mat.defaults.retain(|(key, _)| key != "opacity");
    let (field, channels) = source
        .components
        .rsplit_once('.')
        .map_or((None, source.components.as_str()), |(field, channels)| {
            (Some(field), channels)
        });
    let field = field.map(str::to_owned);
    let channels = channels.to_owned();
    let width = channels.len();
    let components = |channels: String| {
        field
            .as_ref()
            .map_or(channels.clone(), |field| format!("{field}.{channels}"))
    };
    let expression = match source.components.as_str() {
        "distance" => {
            source.components = "distance".into();
            // Output references expand scalar distance to RGB during lowering.
            mat.outputs = vec![format::Output {
                channel: "base color".into(),
                source,
            }];
            preview.validate().map_err(|e| e.to_string())?;
            return Ok(preview);
        }
        _ if width == 1 => {
            source.components = components(channels.repeat(4));
            "vec4(color.rgb, 1)"
        }
        _ if width == 2 => {
            source.components = components(channels.repeat(2));
            "vec4(color.xy, 0, 1)"
        }
        _ if width == 3 => {
            source.components =
                components(format!("{channels}{}", channels.chars().next().unwrap()));
            "vec4(color.rgb, 1)"
        }
        _ => "vec4(color.rgb, 1)",
    };
    let mut id = "eldiron-input-preview".to_string();
    while mat
        .rows
        .iter()
        .any(|row| row.id == id || row.tiles.iter().any(|tile| tile.id == format!("{id}-tile")))
    {
        id.push('_');
    }
    let declarations = denrim_materials::tilelang::source(Kernel::Color)
        .split("image =")
        .next()
        .unwrap();
    mat.rows.push(format::Row {
        id: id.clone(),
        label: "Input Preview".into(),
        coordinates: None,
        incoming: vec![],
        tiles: vec![format::Tile {
            id: format!("{id}-tile"),
            kernel: "Color".into(),
            parameters: vec![("color".into(), Binding::From(source))],
            input: None,
            input_disconnected: true,
            code: Some(format!("{declarations}image = {expression}\n")),
            image: None,
            combine: format::Combine::Replace,
        }],
    });
    mat.outputs = vec![format::Output {
        channel: "base color".into(),
        source: Reference {
            row: id,
            components: "xyz".into(),
            mapping: None,
        },
    }];
    mat.defaults.retain(|(key, _)| key != "opacity");
    preview.validate().map_err(|e| e.to_string())?;
    Ok(preview)
}

fn material(asset: &EditableMaterialAsset) -> Result<&format::Material, String> {
    asset
        .document()
        .materials
        .iter()
        .find(|m| m.id == asset.selected_material)
        .ok_or("Selected material is missing".into())
}
fn reference_view(reference: &Reference) -> MaterialReferenceView {
    MaterialReferenceView {
        row: reference.row.clone(),
        components: reference.components.clone(),
        mapping: reference.mapping.map(|(a, b)| [a, b]),
    }
}
fn reference_binding(reference: Option<&Reference>) -> MaterialInputBinding {
    reference
        .map(|r| MaterialInputBinding::Row(reference_view(r)))
        .unwrap_or(MaterialInputBinding::Default)
}
fn row_input_views(
    asset: &EditableMaterialAsset,
    row: &format::Row,
) -> Result<Vec<MaterialInputView>, String> {
    let mut inputs = vec![MaterialInputView {
        target: MaterialInputTarget::Coordinates {
            row: row.id.clone(),
        },
        label: "Coordinates (UV / XYZ)".into(),
        binding: reference_binding(row.coordinates.as_ref()),
        components: "xy".into(),
    }];
    let catalog = source_catalog();
    let parameters = asset.parameter_metadata().map_err(|e| e.to_string())?;
    for tile in &row.tiles {
        if catalog
            .iter()
            .any(|source| source.metadata.name == tile.kernel && source.accepts_input)
        {
            inputs.push(MaterialInputView {
                target: MaterialInputTarget::Image {
                    operation: tile.id.clone(),
                },
                label: format!("Image · {}", tile.kernel),
                binding: if tile.input_disconnected {
                    MaterialInputBinding::Disconnected
                } else {
                    reference_binding(tile.input.as_ref())
                },
                components: "xyz".into(),
            });
        }
        for param in parameters.iter().filter(|p| {
            p.key.asset == asset.root.id
                && p.key.tile == tile.id
                && p.definition.kind != denrim_materials::metadata::ValueType::Bool
        }) {
            let width = param.definition.default.len();
            inputs.push(MaterialInputView {
                target: MaterialInputTarget::Parameter {
                    operation: tile.id.clone(),
                    parameter: param.key.parameter.clone(),
                },
                label: format!("{} · {}", param.definition.label, tile.kernel),
                binding: match &param.binding {
                    Binding::From(r) => reference_binding(Some(r)),
                    _ => MaterialInputBinding::Default,
                },
                components: ["x", "xy", "xyz", "xyzw"][width.saturating_sub(1).min(3)].into(),
            });
        }
    }
    for link in row_inputs::unassigned(row) {
        inputs.push(MaterialInputView {
            target: MaterialInputTarget::RowLink {
                row: row.id.clone(),
                connection: link.id.clone(),
            },
            label: format!("Incoming · {}", link.source.row),
            binding: reference_binding(Some(&link.source)),
            components: link.source.components.clone(),
        });
    }
    Ok(inputs)
}
fn core_reference(
    binding: &MaterialInputBinding,
    width: usize,
) -> Result<Option<Reference>, String> {
    match binding {
        MaterialInputBinding::Row(reference) => {
            if reference
                .mapping
                .is_some_and(|pair| pair.iter().any(|v| !v.is_finite()))
            {
                return Err("Remap values must be finite".into());
            }
            let components = reference.components.trim();
            let (prefix, channels) = components
                .rsplit_once('.')
                .map_or((None, components), |(field, channels)| {
                    (Some(field), channels)
                });
            let components = if channels.len() == 1 && "xyzw".contains(channels) {
                let channels = channels.repeat(width);
                prefix.map_or(channels.clone(), |field| format!("{field}.{channels}"))
            } else {
                components.into()
            };
            Ok(Some(Reference {
                row: reference.row.clone(),
                components,
                mapping: reference.mapping.map(|[a, b]| (a, b)),
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn view(asset: &EditableMaterialAsset) -> Result<MaterialEditorView, String> {
    let mat = material(asset)?;
    let params = asset.parameter_metadata().map_err(|e| e.to_string())?;
    Ok(MaterialEditorView {
        geometry: false,
        row_types: vec![],
        title: mat.label.clone(),
        image_sources: vec![],
        image_bindings: mat
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .filter_map(|t| {
                t.image.as_ref().map(|i| {
                    (
                        t.id.clone(),
                        format!("{} × {} · {} frame(s)", i.width, i.height, i.frame_count()),
                    )
                })
            })
            .collect(),
        palette: vec![],
        palette_bindings: palette::bindings(asset),
        animation: MaterialAnimationView {
            frames: mat.animation.frames,
            duration: mat.animation.duration,
        },
        tiling: MaterialTilingView {
            mode: mat.tiling.mode.name().into(),
            period: mat.tiling.period,
            blend: mat.tiling.blend,
        },
        defaults: [
            ("base color", vec![1.; 3]),
            ("roughness", vec![0.5]),
            ("metallic", vec![0.]),
            ("opacity", vec![1.]),
            ("emission", vec![0.; 3]),
            ("emission strength", vec![1.]),
        ]
        .into_iter()
        .flat_map(|(key, default)| {
            let values = mat
                .defaults
                .iter()
                .find(|(id, _)| id == key)
                .map_or(default.as_slice(), |(_, v)| v.as_slice());
            values
                .iter()
                .enumerate()
                .map(|(i, v)| MaterialNumberView {
                    id: format!("{key}:{i}"),
                    label: if values.len() == 1 {
                        key.into()
                    } else {
                        format!("{key} {}", ["R", "G", "B"][i])
                    },
                    value: *v,
                    min: 0.,
                    max: if key == "emission strength" { 10. } else { 1. },
                })
                .collect::<Vec<_>>()
        })
        .collect(),
        rows: mat
            .rows
            .iter()
            .map(|row| MaterialRowView {
                id: row.id.clone(),
                label: row.label.clone(),
                inputs: row_input_views(asset, row).unwrap_or_default(),
                operations: row
                    .tiles
                    .iter()
                    .map(|tile| MaterialOperationView {
                        id: tile.id.clone(),
                        label: tile.kernel.clone(),
                        thumbnail: tile.id.clone(),
                        combine: denrim_materials::metadata::Kernel::find(&tile.kernel)
                            .filter(|kernel| kernel.is_shape())
                            .map(|_| tile.combine.name().replace(' ', "_")),
                        source: denrim_materials::metadata::Kernel::find(&tile.kernel).map(
                            |kernel| {
                                let metadata = kernel.metadata();
                                MaterialSourceCodeView {
                                    code: tile.code.clone().unwrap_or_else(|| {
                                        denrim_materials::tilelang::source(kernel).into()
                                    }),
                                    help: metadata.description,
                                    references: metadata.references,
                                }
                            },
                        ),
                        properties: params
                            .iter()
                            .filter(|p| p.key.asset == asset.root.id && p.key.tile == tile.id)
                            .filter(|p| {
                                !(tile.kernel == "Color"
                                    && tile.code.is_none()
                                    && p.key.parameter == "color")
                            })
                            .map(|p| {
                                use denrim_materials::metadata::ValueType;
                                let kind = if !p.definition.choices.is_empty() {
                                    MaterialPropertyKind::Choice
                                } else {
                                    match p.definition.kind {
                                        ValueType::Bool => MaterialPropertyKind::Boolean,
                                        ValueType::Color => MaterialPropertyKind::Color,
                                        ValueType::Vec2 | ValueType::Vec3 => {
                                            MaterialPropertyKind::Vector
                                        }
                                        ValueType::Float => MaterialPropertyKind::Number,
                                    }
                                };
                                MaterialPropertyView {
                                    id: p.key.parameter.clone(),
                                    label: p.definition.label.clone(),
                                    group: p.definition.group.clone(),
                                    kind,
                                    components: (0..p.definition.default.len())
                                        .map(|i| format!("{}:{i}", p.key.parameter))
                                        .collect(),
                                    choices: p.definition.choices.clone(),
                                    tooltip: p.definition.tooltip.clone().unwrap_or_default(),
                                }
                            })
                            .collect(),
                        palette: (tile.kernel == "Color"
                            && tile.code.is_none()
                            && !tile.parameters.iter().any(|(key, binding)| {
                                key == "color" && matches!(binding, Binding::From(_))
                            }))
                        .then(|| MaterialPaletteView {
                            index: palette::binding_index(asset, &tile.id),
                        }),
                        parameters: params
                            .iter()
                            .filter(|p| p.key.asset == asset.root.id && p.key.tile == tile.id)
                            .filter(|p| {
                                !(tile.kernel == "Color"
                                    && tile.code.is_none()
                                    && p.key.parameter == "color")
                            })
                            .flat_map(|p| {
                                let Binding::Constant(value) = &p.binding else {
                                    return vec![];
                                };
                                let (min, max) = if p.definition.choices.is_empty() {
                                    p.definition.range.unwrap_or((0., 1.))
                                } else {
                                    (0., p.definition.choices.len().saturating_sub(1) as f32)
                                };
                                value
                                    .iter()
                                    .enumerate()
                                    .map(|(component, value)| MaterialNumberView {
                                        id: format!("{}:{component}", p.key.parameter),
                                        label: if value_width(&p.definition) == 1 {
                                            p.definition.label.clone()
                                        } else {
                                            format!(
                                                "{} {}",
                                                p.definition.label,
                                                ["R/X", "G/Y", "B/Z", "A"][component.min(3)]
                                            )
                                        },
                                        value: *value,
                                        min,
                                        max,
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
        outputs: CHANNELS
            .iter()
            .zip(LABELS)
            .map(|(id, label)| MaterialOutputView {
                id: id.to_string(),
                label: label.into(),
                row: mat
                    .outputs
                    .iter()
                    .find(|o| o.channel == *id)
                    .map(|o| o.source.row.clone()),
                fallback: output_default_label(mat, id),
            })
            .collect(),
        sources: source_catalog()
            .into_iter()
            .map(|s| MaterialSourceView {
                id: s.id,
                label: s.metadata.name,
                category: s.metadata.category,
            })
            .collect(),
    })
}
/// Output labels describe the rendered channel. Emission color is palette data;
/// its strength determines whether that color emits any light.
fn output_default_label(material: &format::Material, channel: &str) -> String {
    let value = material.defaults.iter().find(|(name, _)| name == channel);
    if channel == "emission" {
        let strength = material
            .defaults
            .iter()
            .find(|(name, _)| name == "emission strength")
            .and_then(|(_, values)| values.first())
            .copied()
            .unwrap_or(1.);
        let emission: Vec<_> = value
            .map_or(&[0.; 3][..], |(_, values)| values.as_slice())
            .iter()
            .map(|v| v * strength)
            .collect();
        if emission.iter().all(|v| *v == 0.) {
            return "0".into();
        }
        return format!("{emission:?}");
    }
    value
        .map(|(_, values)| format!("{values:?}"))
        .unwrap_or_else(|| "Default".into())
}

fn value_width(p: &denrim_materials::metadata::Parameter) -> usize {
    p.default.len()
}

/// All mutations are applied to a clone and validated before the project changes.
fn edited(
    asset: &EditableMaterialAsset,
    command: MaterialEditorCommand,
) -> Result<EditableMaterialAsset, String> {
    let mut next = asset.clone();
    let selected = next.selected_material.clone();
    let mat = next
        .root
        .project
        .materials
        .iter_mut()
        .find(|m| m.id == selected)
        .ok_or("Material is missing")?;
    row_inputs::capture_material(mat);
    // An incoming edge belongs to its destination row. Using it in a new tile
    // changes only that receiver; the port keeps its stable saved identity.
    let command = if let MaterialEditorCommand::AssignRowInput {
        row,
        connection,
        target,
    } = command
    {
        let source = row_inputs::connection(mat, &row, &connection)?.clone();
        let owner = match &target {
            MaterialInputTarget::Coordinates { row } => Some(row.as_str()),
            MaterialInputTarget::Image { operation }
            | MaterialInputTarget::Parameter { operation, .. } => mat
                .rows
                .iter()
                .find(|r| r.tiles.iter().any(|t| &t.id == operation))
                .map(|r| r.id.as_str()),
            MaterialInputTarget::RowLink { .. } => None,
        };
        if owner != Some(row.as_str()) {
            return Err("Assign incoming connections within their owning row".into());
        }
        let width = match &target {
            MaterialInputTarget::Coordinates { .. } => 2,
            MaterialInputTarget::Image { .. } => 3,
            MaterialInputTarget::Parameter {
                operation,
                parameter,
            } => asset
                .parameter_metadata()
                .map_err(|e| e.to_string())?
                .iter()
                .find(|p| {
                    p.key.asset == asset.root.id
                        && &p.key.tile == operation
                        && &p.key.parameter == parameter
                })
                .ok_or("Parameter is missing")?
                .definition
                .default
                .len(),
            _ => unreachable!(),
        };
        let binding = MaterialInputBinding::Row(reference_view(&source));
        let adapted = core_reference(&binding, width)?.unwrap();
        row_inputs::set_connection(mat, &row, &connection, Some(adapted.clone()))?;
        MaterialEditorCommand::SetInput {
            target,
            binding: MaterialInputBinding::Row(reference_view(&adapted)),
        }
    } else {
        command
    };
    match command {
        MaterialEditorCommand::SetAnimation { frames, duration } => {
            mat.animation = denrim_materials::format::Animation { frames, duration }
                .validate()
                .map_err(|e| e.to_string())?;
        }
        MaterialEditorCommand::SetTiling {
            mode,
            period,
            blend,
        } => {
            mat.tiling = denrim_materials::periodic::Tiling {
                mode: denrim_materials::periodic::Mode::parse(&mode).map_err(|e| e.to_string())?,
                period,
                blend,
            }
            .validate()
            .map_err(|e| e.to_string())?;
        }
        MaterialEditorCommand::SetCombine { operation, mode } => {
            let tile = mat
                .rows
                .iter_mut()
                .flat_map(|r| &mut r.tiles)
                .find(|t| t.id == operation)
                .ok_or("Operation is missing")?;
            if !denrim_materials::metadata::Kernel::find(&tile.kernel).is_some_and(|k| k.is_shape())
            {
                return Err("Shape combinations require a shape tile".into());
            }
            tile.combine = match mode.as_str() {
                "replace" => format::Combine::Replace,
                "union" => format::Combine::Union,
                "subtract" => format::Combine::Subtract,
                "intersect" => format::Combine::Intersect,
                "smooth_union" | "smooth union" => format::Combine::SmoothUnion,
                _ => return Err("Unknown shape combination".into()),
            };
        }
        MaterialEditorCommand::SetSource { operation, code } => {
            let tile = mat
                .rows
                .iter_mut()
                .flat_map(|r| &mut r.tiles)
                .find(|t| t.id == operation)
                .ok_or("Operation is missing")?;
            tile.code = (!code.trim().is_empty()).then_some(code);
        }
        MaterialEditorCommand::SetInput { target, binding } => match target {
            MaterialInputTarget::RowLink { row, connection } => {
                let old = row_inputs::connection(mat, &row, &connection)?;
                let width = if old.components == "distance" {
                    1
                } else {
                    old.components.len()
                };
                row_inputs::set_connection(
                    mat,
                    &row,
                    &connection,
                    core_reference(&binding, width)?,
                )?;
                row_inputs::preserve_empty_rows(mat);
            }
            MaterialInputTarget::Coordinates { row } => {
                if binding == MaterialInputBinding::Disconnected {
                    return Err("Coordinates use their natural UV domain when disconnected".into());
                }
                mat.rows
                    .iter_mut()
                    .find(|r| r.id == row)
                    .ok_or("Row is missing")?
                    .coordinates = core_reference(&binding, 2)?;
            }
            MaterialInputTarget::Image { operation } => {
                let tile = mat
                    .rows
                    .iter_mut()
                    .flat_map(|r| &mut r.tiles)
                    .find(|t| t.id == operation)
                    .ok_or("Operation is missing")?;
                if !source_catalog()
                    .iter()
                    .any(|source| source.metadata.name == tile.kernel && source.accepts_input)
                {
                    return Err("This tile does not accept an image input".into());
                }
                tile.input = core_reference(&binding, 3)?;
                tile.input_disconnected = binding == MaterialInputBinding::Disconnected;
            }
            MaterialInputTarget::Parameter {
                operation,
                parameter,
            } => {
                if binding == MaterialInputBinding::Disconnected {
                    return Err("Choose Default to restore a parameter constant".into());
                }
                let default = asset
                    .parameter_metadata()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .find(|p| {
                        p.key.asset == asset.root.id
                            && p.key.tile == operation
                            && p.key.parameter == parameter
                    })
                    .ok_or("Parameter is missing")?;
                if default.definition.kind == denrim_materials::metadata::ValueType::Bool {
                    return Err("Boolean parameters do not accept row connections".into());
                }
                let tile = mat
                    .rows
                    .iter_mut()
                    .flat_map(|r| &mut r.tiles)
                    .find(|t| t.id == operation)
                    .ok_or("Operation is missing")?;
                tile.parameters.retain(|(key, _)| key != &parameter);
                let reference = core_reference(&binding, default.definition.default.len())?;
                tile.parameters.push((
                    parameter,
                    reference
                        .map(Binding::From)
                        .unwrap_or(Binding::Constant(default.definition.default)),
                ));
            }
        },
        MaterialEditorCommand::SetNumber {
            operation,
            parameter,
            value,
        } => {
            let (key, component) = parameter
                .rsplit_once(':')
                .ok_or("Invalid parameter identity")?;
            let component: usize = component.parse().map_err(|_| "Invalid component")?;
            if operation == "@surface" {
                let defaults = vec![
                    ("roughness", vec![0.5]),
                    ("metallic", vec![0.]),
                    ("opacity", vec![1.]),
                    ("emission", vec![0.; 3]),
                    ("emission strength", vec![1.]),
                ];
                let (_, default) = defaults
                    .into_iter()
                    .find(|(id, _)| *id == key)
                    .ok_or("Unknown material constant")?;
                let mut values = mat
                    .defaults
                    .iter()
                    .find(|(id, _)| id == key)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(default);
                let v = values.get_mut(component).ok_or("Invalid component")?;
                if !value.is_finite()
                    || value < 0.
                    || value > if key == "emission strength" { 10. } else { 1. }
                {
                    return Err("Material constant is out of range".into());
                }
                *v = value;
                if let Some((_, v)) = mat.defaults.iter_mut().find(|(id, _)| id == key) {
                    *v = values;
                } else {
                    mat.defaults.push((key.into(), values));
                }
                row_inputs::capture_material(mat);
                next.validate().map_err(|e| e.to_string())?;
                return Ok(next);
            }
            let meta = asset
                .parameter_metadata()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| {
                    p.key.asset == asset.root.id
                        && p.key.tile == operation
                        && p.key.parameter == key
                })
                .ok_or("Parameter is missing")?;
            let Binding::Constant(mut values) = meta.binding else {
                return Err("Disconnect the parameter before editing its constant".into());
            };
            let target = values.get_mut(component).ok_or("Invalid component")?;
            if !value.is_finite() {
                return Err("Parameter must be finite".into());
            }
            *target = if matches!(
                meta.definition.kind,
                denrim_materials::metadata::ValueType::Bool
            ) || !meta.definition.choices.is_empty()
            {
                value.round()
            } else {
                value
            };
            let (min, max) = if meta.definition.kind == denrim_materials::metadata::ValueType::Bool
            {
                (0., 1.)
            } else if !meta.definition.choices.is_empty() {
                (0., meta.definition.choices.len().saturating_sub(1) as f32)
            } else {
                meta.definition.range.unwrap_or((f32::MIN, f32::MAX))
            };
            if *target < min || *target > max {
                return Err("Parameter is out of range".into());
            }
            let tile = mat
                .rows
                .iter_mut()
                .flat_map(|r| &mut r.tiles)
                .find(|t| t.id == operation)
                .ok_or("Operation is missing")?;
            if let Some((_, binding)) = tile.parameters.iter_mut().find(|(id, _)| id == key) {
                *binding = Binding::Constant(values);
            } else {
                tile.parameters
                    .push((key.into(), Binding::Constant(values)));
            }
        }
        MaterialEditorCommand::AddSource { source, row, after } => {
            let source = source_catalog()
                .into_iter()
                .find(|s| s.id == source)
                .ok_or("Unknown MaterialZ source")?;
            let tile = format::Tile {
                id: format::new_tile_id(),
                input_disconnected: false,
                code: None,
                image: None,
                kernel: source.metadata.name,
                parameters: vec![],
                input: None,
                combine: format::Combine::Replace,
            };
            if let Some(row_id) = row {
                let row = mat
                    .rows
                    .iter_mut()
                    .find(|r| r.id == row_id)
                    .ok_or("Row is missing")?;
                let at = insertion_index(row, after.as_deref())?;
                if at == 0 && source.requires_input {
                    return Err(
                        "This source needs an input. Drop it after an existing source.".into(),
                    );
                }
                row.tiles.insert(at, tile);
            } else {
                if source.requires_input {
                    return Err("Start a row with a generator, then add this source.".into());
                }
                mat.rows.push(format::Row {
                    id: Uuid::new_v4().to_string(),
                    label: tile.kernel.clone(),
                    tiles: vec![tile],
                    coordinates: None,
                    incoming: vec![],
                });
            }
        }
        MaterialEditorCommand::MoveOperation {
            operation,
            row,
            after,
        } => {
            if after.as_deref() == Some(&operation) {
                return Err("An operation cannot follow itself".into());
            }
            let from = mat
                .rows
                .iter()
                .position(|r| r.tiles.iter().any(|t| t.id == operation))
                .ok_or("Operation is missing")?;
            let pos = mat.rows[from]
                .tiles
                .iter()
                .position(|t| t.id == operation)
                .unwrap();
            let tile = mat.rows[from].tiles.remove(pos);
            let to = mat
                .rows
                .iter_mut()
                .find(|r| r.id == row)
                .ok_or("Destination row is missing")?;
            let at = insertion_index(to, after.as_deref())?;
            to.tiles.insert(at, tile);
            row_inputs::preserve_empty_rows(mat);
        }
        MaterialEditorCommand::RemoveOperation(operation) => {
            let row = mat
                .rows
                .iter_mut()
                .find(|r| r.tiles.iter().any(|t| t.id == operation))
                .ok_or("Operation is missing")?;
            row.tiles.retain(|t| t.id != operation);
            row_inputs::preserve_empty_rows(mat);
        }
        MaterialEditorCommand::ConnectOutput { output, row } => {
            if !CHANNELS.contains(&output.as_str()) || !mat.rows.iter().any(|r| r.id == row) {
                return Err("Unknown output or row".into());
            }
            mat.outputs.retain(|o| o.channel != output);
            mat.outputs.push(format::Output {
                channel: output.clone(),
                source: Reference {
                    row,
                    components: if matches!(output.as_str(), "base color" | "emission") {
                        "xyz"
                    } else {
                        "x"
                    }
                    .into(),
                    mapping: None,
                },
            });
        }
        MaterialEditorCommand::DisconnectOutput(output) => {
            mat.outputs.retain(|o| o.channel != output)
        }
        _ => return Ok(next),
    }
    row_inputs::capture_material(mat);
    palette::prune_bindings(&mut next);
    next.validate().map_err(|e| e.to_string())?;
    Ok(next)
}
pub(crate) fn edited_for_project(
    asset: &EditableMaterialAsset,
    command: MaterialEditorCommand,
    project: &Project,
) -> Result<EditableMaterialAsset, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let MaterialEditorCommand::AddSource { source, row, after } = &command
            && source == model_source::SOURCE
        {
            let model = model_source::choices(project)
                .first()
                .and_then(|(id, _)| id.strip_prefix("@model:"))
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or("Create or import a model first")?;
            let source = source_catalog()
                .into_iter()
                .find(|s| s.metadata.name == "Image / Tile Source")
                .ok_or("Image source is missing")?;
            let mut next = edited(
                asset,
                MaterialEditorCommand::AddSource {
                    source: source.id,
                    row: row.clone(),
                    after: after.clone(),
                },
            )?;
            let old = asset
                .root
                .project
                .materials
                .iter()
                .flat_map(|m| &m.rows)
                .flat_map(|r| &r.tiles)
                .map(|t| &t.id)
                .collect::<std::collections::HashSet<_>>();
            let operation = next
                .root
                .project
                .materials
                .iter()
                .flat_map(|m| &m.rows)
                .flat_map(|r| &r.tiles)
                .find(|t| !old.contains(&t.id))
                .ok_or("New image source is missing")?
                .id
                .clone();
            next = model_source::assign(&next, &operation, model_source::Spec::new(model))?;
            return Ok(next);
        }
        if let MaterialEditorCommand::SetNumber {
            operation,
            parameter,
            value,
        } = &command
            && parameter.starts_with("@model-")
        {
            return model_source::edit(asset, operation, parameter, *value, project);
        }
        if let MaterialEditorCommand::ChooseImage { operation, source } = &command
            && let Some(id) = source.strip_prefix("@model:")
        {
            return model_source::assign(
                asset,
                operation,
                model_source::Spec::new(Uuid::parse_str(id).map_err(|_| "Invalid model source")?),
            );
        }
    }
    if let MaterialEditorCommand::SetImage { operation, image } = &command {
        return image_source::set(asset, operation, image.clone());
    }
    if let MaterialEditorCommand::ChooseImage { operation, source } = &command {
        if source == "@current" {
            return Ok(asset.clone());
        }
        if source == "@clear" {
            return image_source::set(asset, operation, None);
        }
        let id = Uuid::parse_str(source).map_err(|_| "Invalid image tile reference")?;
        if project.procedural_materials.contains_key(&id) {
            return Err("Choose a bitmap tile as the image source".into());
        }
        let tile = project
            .tiles
            .get(&id)
            .ok_or("Image source tile is missing")?;
        return image_source::set(asset, operation, Some(image_source::from_tile(id, tile)?));
    }
    if let MaterialEditorCommand::SetPaletteParameter {
        operation,
        parameter,
        index,
    } = &command
    {
        return palette::bind_parameter(asset, operation, parameter, *index, &project.palette);
    }
    if let MaterialEditorCommand::SetPalette { operation, index } = command {
        return palette::bind_color(asset, &operation, index, &project.palette);
    }
    let mut next = edited(asset, command)?;
    palette::ensure_color_bindings(&mut next, &project.palette)?;
    next.validate().map_err(|e| e.to_string())?;
    Ok(next)
}
fn insertion_index(row: &format::Row, after: Option<&str>) -> Result<usize, String> {
    after
        .map(|id| {
            row.tiles
                .iter()
                .position(|t| t.id == id)
                .map(|i| i + 1)
                .ok_or("Previous operation is missing".into())
        })
        .unwrap_or(Ok(0))
}

#[derive(Default)]
pub struct ProceduralMaterialEditor {
    mesh_material: bool,
    iso_revision: Option<u64>,
    iso_placement_revision: Option<u64>,
    iso_bounds_selection: Option<(Uuid, Vec<u32>)>,
    iso_mode: bool,
    tile: Option<Uuid>,
    active: bool,
    requested: std::collections::HashMap<Uuid, u64>,
    bundle: Option<String>,
    asset: Option<EditableMaterialAsset>,
    previous_sidebar: usize,
    channel: usize,
    preview_tile: bool,
    selected_operation: Option<String>,
    selected_source: Option<String>,
    selected_input: Option<(MaterialInputTarget, MaterialReferenceView)>,
    composition_preview: Option<String>,
    preview_request: bool,
    revision: u64,
    frame: Option<gpu::Frame>,
    preview_size: (usize, usize),
    preview_animation_frame: usize,
    preview_checker: Option<[RGBA; 2]>,
    preview_dirty: bool,
    palette_key: Option<u64>,
    pixel_density: u32,
    pixel_config_key: Option<u64>,
    image_source_key: Option<u64>,
    bake_keys_revision: Option<(u64, u32)>,
    bake_keys: std::collections::HashMap<Uuid, u64>,
    pending_image: Option<(Uuid, String)>,
    composition_key: Option<u64>,
    composition_ui_key: Option<(Uuid, u64, usize)>,
    composition_layer: usize,
    composition_advanced: bool,
    composition_choices: Vec<Uuid>,
    map_binding: Option<shared::material_composition::MapMaterialBinding>,
    #[cfg(not(target_arch = "wasm32"))]
    worker: Option<gpu::Worker>,
    #[cfg(not(target_arch = "wasm32"))]
    model_sources: model_source::State,
}
impl ProceduralMaterialEditor {
    fn status(_ui: &mut TheUI, ctx: &mut TheContext, text: impl Into<String>) {
        ctx.ui
            .send(TheEvent::SetStatusText(TheId::empty(), text.into()));
    }
    fn open(&mut self, id: Uuid, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        self.mesh_material = false;
        self.map_binding = None;
        let id = match shared::isometric::tile_recipe(project, id) {
            Some(shared::isometric::TileRecipe::Material(recipe)) => recipe,
            Some(shared::isometric::TileRecipe::Geometry(_)) => {
                // GeometryEditor owns the same workspace for this selection.
                // Clear the previous material without navigating away from it.
                self.active = false;
                self.tile = None;
                self.bundle = None;
                self.asset = None;
                self.frame = None;
                self.selected_operation = None;
                self.selected_source = None;
                self.selected_input = None;
                self.composition_preview = None;
                self.preview_request = false;
                self.revision = self.revision.wrapping_add(1);
                return;
            }
            None => id,
        };
        if !project.procedural_materials.contains_key(&id) {
            self.clear_selection(ui, ctx);
            return;
        }
        let current = ui
            .get_stack_layout("Sidebar Page Stack")
            .map_or(0, |stack| stack.index());
        if current != SIDEBAR_PAGE {
            self.previous_sidebar = current;
        }
        crate::sidebar::Sidebar::set_navigation_page(SIDEBAR_PAGE, ui, ctx);
        self.active = true;
        if self.tile == Some(id) && self.bundle.is_some() {
            return;
        }
        self.tile = Some(id);
        if let Some(w) = ui.get_widget("Ruleset Material") {
            if w.state() != TheWidgetState::Clicked {
                if let Some(menu) = w.as_drop_down_menu() {
                    let index = shared::ruleset_materials::definitions(&project.config)
                        .iter()
                        .position(|d| shared::rulesets::material_asset_id(d.id) == id)
                        .map_or(0, |i| i + 1);
                    menu.set_selected_index(index as i32);
                }
            }
        }
        self.composition_layer = 0;
        self.composition_ui_key = None;
        if project.material_compositions.contains_key(&id) {
            self.preview_tile = false;
            if let Some(target) = ui
                .get_widget("Material Preview Target")
                .and_then(|w| w.as_group_button())
            {
                target.set_index(0);
            }
            ctx.ui.set_enabled("Material Preview Channel");
        }
        self.selected_operation = None;
        self.selected_source = None;
        self.selected_input = None;
        self.composition_preview = None;
        self.bundle = None;
        self.asset = None;
        self.frame = None;
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
    }
    pub fn leave(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if ui
            .get_stack_layout("Sidebar Page Stack")
            .is_some_and(|stack| stack.index() == SIDEBAR_PAGE)
        {
            crate::sidebar::Sidebar::set_navigation_page(self.previous_sidebar, ui, ctx);
        }
        self.active = false;
    }
    fn clear_selection(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.leave(ui, ctx);
        self.tile = None;
        self.map_binding = None;
        self.selected_operation = None;
        self.selected_source = None;
        self.selected_input = None;
        self.composition_preview = None;
        self.preview_request = false;
        self.bundle = None;
        self.asset = None;
        self.frame = None;
        self.revision = self.revision.wrapping_add(1);
        self.preview_dirty = true;
        if let Some(widget) = ui
            .get_widget(EDITOR)
            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
        {
            widget.set_view(MaterialEditorView {
                title: "Select a procedural tile".into(),
                ..Default::default()
            });
        }
    }
    pub fn reset(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.clear_selection(ui, ctx);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.model_sources = Default::default();
        }
        self.requested.clear();
        self.palette_key = None;
    }
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        if crate::iso_editor::handle_event(event, project, server, ctx) {
            return true;
        }
        if let TheEvent::Custom(id, TheValue::Id(material)) = event {
            if id.name == "Open Mesh Material" {
                self.open(*material, ui, ctx, project);
                self.mesh_material = true;
                if let Some(w) = ui.get_widget("MZ Return to Mesh") {
                    w.set_disabled(false);
                }
                return true;
            }
        }
        if matches!(event,TheEvent::StateChanged(id,TheWidgetState::Clicked) if id.name=="MZ Return to Mesh")
            && self.mesh_material
        {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Return to Mesh Recipe"),
                TheValue::Empty,
            ));
            return true;
        }
        if matches!(event,TheEvent::Custom(id,_) if id.name=="Return to Mesh Recipe") {
            self.tile = None;
            self.bundle = None;
            self.asset = None;
            self.frame = None;
            self.mesh_material = false;
            return false;
        }
        if let TheEvent::IndexChanged(id, index) = event {
            if id.name == "Ruleset Material" {
                if let Some(def) = index.checked_sub(1).and_then(|i| {
                    shared::ruleset_materials::definitions(&project.config)
                        .get(i)
                        .cloned()
                }) {
                    self.open(
                        shared::rulesets::material_asset_id(def.id),
                        ui,
                        ctx,
                        project,
                    );
                } else if let Some(id) = server.curr_tile_id {
                    self.open(id, ui, ctx, project);
                } else {
                    self.clear_selection(ui, ctx);
                }
                return true;
            }
        }
        if let TheEvent::FileRequesterResult(id, paths) = event {
            if id.name == "Material Image Import" {
                if let (Some((target, operation)), Some(path)) =
                    (self.pending_image.take(), paths.first())
                {
                    let result = (|| -> Result<(), String> {
                        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                        let texture = rusterix::Texture::from_image_safe(bytes.as_slice())
                            .ok_or("Could not decode image")?;
                        let image = denrim_materials::image_source::ImageSource::rgba8(
                            texture.width,
                            texture.height,
                            &[texture.data],
                        )
                        .map_err(|e| e.to_string())?;
                        let before = project
                            .procedural_materials
                            .get(&target)
                            .ok_or("Material was removed")?
                            .clone();
                        let asset =
                            EditableMaterialAsset::reopen(&before).map_err(|e| e.to_string())?;
                        let next = image_source::set(&asset, &operation, Some(image))?;
                        let bundle = next.save().map_err(|e| e.to_string())?;
                        project.procedural_materials.insert(target, bundle.clone());
                        UNDOMANAGER.write().unwrap().add_undo(
                            ProjectUndoAtom::ProceduralMaterialEdit(target, before, bundle),
                            ctx,
                        );
                        if self.tile == Some(target) {
                            self.asset = Some(next);
                        }
                        Ok(())
                    })();
                    if let Err(e) = result {
                        Self::status(ui, ctx, e);
                    }
                }
                return true;
            }
        }
        if self.map_binding.is_some() {
            self.follow_map_composition(ui, ctx, project, server);
        }
        if self.composition_event(event, ui, ctx, project, server) {
            return true;
        }
        if let TheEvent::ValueChanged(id, value) = event {
            let command = ui
                .get_widget(EDITOR)
                .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                .and_then(|w| w.number_command(id, value));
            if let Some(command) = command {
                if let (Some(tile), Some(asset)) = (self.tile, self.asset.as_ref()) {
                    match edited_for_project(asset, command, project)
                        .and_then(|a| a.save().map(|b| (a, b)).map_err(|e| e.to_string()))
                    {
                        Ok((asset, bundle))
                            if project.procedural_materials.get(&tile) != Some(&bundle) =>
                        {
                            let before = project
                                .procedural_materials
                                .insert(tile, bundle.clone())
                                .unwrap();
                            UNDOMANAGER.write().unwrap().add_undo(
                                ProjectUndoAtom::ProceduralMaterialEdit(tile, before, bundle),
                                ctx,
                            );
                            self.asset = Some(asset);
                        }
                        Err(error) => {
                            Self::status(ui, ctx, error);
                            Self::set_project_view(ui, asset, project);
                        }
                        _ => {}
                    }
                }
                return true;
            }
        }
        match event {
            TheEvent::StateChanged(id, state)
                if self.active
                    && !ui.ctrl
                    && !ui.logo
                    && !ui.alt
                    && ctx.ui.focus.as_ref().is_some_and(|focus| {
                        focus.name == EDITOR
                            || focus.name == "Material Tiling"
                            || focus.name.starts_with("Composition ")
                    })
                    && *state == TheWidgetState::Clicked
                    && matches!(id.name.as_str(), "Undo" | "Redo") =>
            {
                let mut undo = UNDOMANAGER.write().unwrap();
                if id.name == "Undo" {
                    undo.undo(server, project, ui, ctx);
                } else {
                    undo.redo(server, project, ui, ctx);
                }
                true
            }
            TheEvent::Custom(id, TheValue::Id(tile)) if id.name == "Open Procedural Tile" => {
                self.open(*tile, ui, ctx, project);
                true
            }
            TheEvent::Custom(id, _) if id.name == "Ruleset Materials Restored" => {
                self.palette_key = None;
                self.preview_dirty = true;
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::Custom(id, _)
                if id.name == "Procedural Tiles Restored"
                    || id.name == "Scepter Materials Published" =>
            {
                self.palette_key = None;
                self.composition_key = None;
                self.composition_ui_key = None;
                self.requested.clear();
                refresh_tiles(ctx);
                // Scepter publication has already installed the complete scene.
                if id.name != "Scepter Materials Published" {
                    crate::utils::editor_scene_full_rebuild(project, server);
                }
                true
            }
            TheEvent::Custom(id, TheValue::Text(binding)) if id.name == map_composition::OPEN => {
                match serde_json::from_str(binding) {
                    Ok(binding) => self.open_map_composition(binding, ui, ctx, project, server),
                    Err(error) => {
                        Self::status(ui, ctx, format!("Invalid surface selection: {error}"))
                    }
                }
                true
            }
            TheEvent::Custom(id, _) if id.name == "Map Selection Changed" => {
                self.follow_map_composition(ui, ctx, project, server);
                false
            }
            TheEvent::Custom(id, _) if id.name == "Make Procedural Tile" => {
                if let Some(id) = server.curr_tile_id {
                    if project.procedural_materials.contains_key(&id) {
                        self.open(id, ui, ctx, project);
                    } else {
                        Self::status(
                            ui,
                            ctx,
                            "Select an empty tile slot to create a procedural material. Existing artwork conversion comes later.",
                        );
                    }
                    return true;
                }
                let Some(cell) = server.curr_tile_board_cell else {
                    Self::status(ui, ctx, "Select an empty tile slot first.");
                    return true;
                };
                let size = pixel_style::bake_size(pixel_style::density(&project.config));
                let mut tile =
                    rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(size, size)]);
                tile.textures[0].fill([128, 128, 128, 255]);
                tile.alias = "Procedural Tile".into();
                tile.set_default_materials();
                let id = tile.id;
                match palette::starter(id, project)
                    .and_then(|a| a.save().map_err(|e| e.to_string()))
                {
                    Ok(bundle) => {
                        let before = project.clone();
                        project.tiles.insert(id, tile);
                        project.procedural_materials.insert(id, bundle);
                        project.place_tile_at_picker_selection(id, Some(cell));
                        server.curr_tile_board_cell = None;
                        server.curr_tile_id = Some(id);
                        server.curr_tile_source = Some(rusterix::TileSource::SingleTile(id));
                        UNDOMANAGER.write().unwrap().add_undo(
                            ProjectUndoAtom::TilePickerEdit(
                                Box::new(before),
                                Box::new(project.clone()),
                            ),
                            ctx,
                        );
                        refresh_tiles(ctx);
                        self.open(id, ui, ctx, project);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Pick Tile Source"),
                            TheValue::List(vec![TheValue::Text("single".into()), TheValue::Id(id)]),
                        ));
                    }
                    Err(e) => Self::status(ui, ctx, e),
                }
                true
            }
            TheEvent::Custom(id, TheValue::Id(tile)) if id.name == "Tile Picked" => {
                if self.map_binding.is_none() {
                    self.open(*tile, ui, ctx, project);
                }
                false
            }
            TheEvent::Custom(id, _) if id.name == "Tilepicker Empty Selected" => {
                self.clear_selection(ui, ctx);
                false
            }
            TheEvent::Custom(id, _) if id.name == MATERIAL_EDITOR_COMMANDS => {
                let commands = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                    .map(|w| w.take_commands())
                    .unwrap_or_default();
                for command in commands {
                    if let MaterialEditorCommand::ChooseImage { operation, source } = &command {
                        if source == "@import" {
                            if let Some(tile) = self.tile {
                                self.pending_image = Some((tile, operation.clone()));
                                ctx.ui.open_file_requester(
                                    TheId::named("Material Image Import"),
                                    "Import Material Image".into(),
                                    TheFileExtension::new(
                                        "Image".into(),
                                        vec![
                                            "png".into(),
                                            "jpg".into(),
                                            "jpeg".into(),
                                            "bmp".into(),
                                            "webp".into(),
                                        ],
                                    ),
                                );
                            }
                            continue;
                        }
                    }
                    if let MaterialEditorCommand::InspectRow(_) = &command {
                        continue;
                    }
                    if command == MaterialEditorCommand::InspectSurface {
                        self.selected_operation = None;
                        self.selected_source = None;
                        self.selected_input = None;
                        self.preview_dirty = true;
                        continue;
                    }
                    if let MaterialEditorCommand::InspectSource(source) = command {
                        self.selected_input = None;
                        self.selected_source = Some(source);
                        self.preview_dirty = true;
                        self.preview_request = self.preview_tile;
                        continue;
                    }
                    if let MaterialEditorCommand::InspectOperation(operation) = command {
                        let was_source = self.selected_source.take().is_some()
                            | self.selected_input.take().is_some();
                        if was_source || self.selected_operation.as_ref() != Some(&operation) {
                            self.selected_operation = Some(operation);
                            self.preview_dirty = true;
                            self.preview_request = self.preview_tile;
                        }
                        continue;
                    }
                    if let MaterialEditorCommand::InspectInput { target, binding } = command {
                        self.selected_input = if let MaterialInputBinding::Row(reference) = binding
                        {
                            Some((target, reference))
                        } else {
                            None
                        };
                        self.selected_source = None;
                        self.selected_operation = None;
                        self.preview_dirty = true;
                        self.preview_request = self.preview_tile;
                        continue;
                    }
                    if let (Some(tile), Some(asset)) = (self.tile, self.asset.as_ref()) {
                        match edited_for_project(asset, command, project)
                            .and_then(|a| a.save().map(|b| (a, b)).map_err(|e| e.to_string()))
                        {
                            Ok((asset, bundle)) => {
                                if project.procedural_materials.get(&tile) == Some(&bundle) {
                                    continue;
                                }
                                let before = project
                                    .procedural_materials
                                    .insert(tile, bundle.clone())
                                    .unwrap();
                                UNDOMANAGER.write().unwrap().add_undo(
                                    ProjectUndoAtom::ProceduralMaterialEdit(tile, before, bundle),
                                    ctx,
                                );
                                self.asset = Some(asset); // update() refreshes view and queues GPU work.
                            }
                            Err(e) => {
                                Self::status(ui, ctx, e);
                                if let Some(asset) = &self.asset {
                                    Self::set_project_view(ui, asset, project);
                                }
                            }
                        }
                    }
                }
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == "Material Preview Channel" => {
                self.channel = (*index).min(CHANNELS.len() - 1);
                self.preview_dirty = true;
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == "Material Preview Target" => {
                self.preview_tile = *index == 1;
                self.preview_dirty = true;
                if self.preview_tile {
                    ctx.ui.set_disabled("Material Preview Channel");
                    self.preview_request = true;
                    if self.selection_target().is_none() {
                        Self::status(ui, ctx, "Select an operation or a source to preview it.");
                    }
                } else {
                    ctx.ui.set_enabled("Material Preview Channel");
                }
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == "Material Tiling" => {
                ctx.ui.set_focus(id);
                let Some(mode) = ["raw", "repeat", "seamless"].get(*index) else {
                    return true;
                };
                if let Some(tile) = self.tile
                    && let Some(before) = project.procedural_materials.get(&tile).cloned()
                {
                    let result = EditableMaterialAsset::reopen(&before)
                        .map_err(|e| e.to_string())
                        .and_then(|asset| {
                            let tiling = material(&asset)?.tiling;
                            edited_for_project(
                                &asset,
                                MaterialEditorCommand::SetTiling {
                                    mode: (*mode).into(),
                                    period: tiling.period,
                                    blend: tiling.blend,
                                },
                                project,
                            )
                        })
                        .and_then(|asset| {
                            asset
                                .save()
                                .map(|bundle| (asset, bundle))
                                .map_err(|e| e.to_string())
                        });
                    match result {
                        Ok((asset, bundle)) => {
                            if bundle != before {
                                project.procedural_materials.insert(tile, bundle.clone());
                                UNDOMANAGER.write().unwrap().add_undo(
                                    ProjectUndoAtom::ProceduralMaterialEdit(tile, before, bundle),
                                    ctx,
                                );
                            }
                            Self::set_project_view(ui, &asset, project);
                            self.asset = Some(asset);
                        }
                        Err(error) => {
                            Self::status(ui, ctx, error);
                            if let Some(asset) = &self.asset {
                                Self::sync_tiling_button(ui, asset);
                            }
                        }
                    }
                }
                true
            }
            // Widget keyboard events have already been delivered by TheUI. Do not
            // let a Delete or typing gesture also mutate the map hidden underneath.
            TheEvent::KeyDown(_) | TheEvent::KeyCodeDown(_)
                if self.active
                    && !ui.ctrl
                    && !ui.logo
                    && !ui.alt
                    && ctx.ui.focus.as_ref().is_some_and(|focus| {
                        focus.name == EDITOR
                            || focus.name == "Material Tiling"
                            || focus.name.starts_with("Composition ")
                    })
                    && !matches!(
                        event,
                        TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Tab))
                    ) =>
            {
                true
            }
            _ => false,
        }
    }
    fn set_project_view(ui: &mut TheUI, asset: &EditableMaterialAsset, project: &Project) {
        Self::sync_tiling_button(ui, asset);
        if let Ok(mut view) = view(asset) {
            view.image_sources = project
                .tiles
                .iter()
                .filter(|(id, t)| {
                    !project.procedural_materials.contains_key(*id) && !t.textures.is_empty()
                })
                .map(|(id, t)| {
                    (
                        id.to_string(),
                        if t.alias.is_empty() {
                            id.to_string()
                        } else {
                            t.alias.clone()
                        },
                    )
                })
                .collect();
            #[cfg(not(target_arch = "wasm32"))]
            model_source::decorate(&mut view, asset, project);
            view.image_sources
                .sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
            for (operation, label) in &mut view.image_bindings {
                let source = asset
                    .root
                    .project
                    .materials
                    .iter()
                    .flat_map(|m| &m.rows)
                    .flat_map(|r| &r.tiles)
                    .find(|t| &t.id == operation)
                    .and_then(|t| t.image.as_ref())
                    .and_then(|i| i.reference.as_deref())
                    .and_then(|s| s.strip_prefix("eldiron:tile:"))
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .and_then(|id| project.tiles.get(&id));
                if let Some(source) = source {
                    if !source.alias.is_empty() {
                        *label = format!("{} · {}", source.alias, label);
                    }
                }
            }
            view.palette = project
                .palette
                .colors
                .iter()
                .enumerate()
                .filter_map(|(index, color)| {
                    color.as_ref().map(|color| MaterialPaletteEntry {
                        index: index as u16,
                        label: format!("Color {index}"),
                        color: color.to_u8_array(),
                    })
                })
                .collect();
            if let Some(widget) = ui
                .get_widget(EDITOR)
                .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
            {
                widget.set_view(view);
            }
        }
    }
    fn sync_tiling_button(ui: &mut TheUI, asset: &EditableMaterialAsset) {
        if let Ok(material) = material(asset)
            && let Some(button) = ui
                .get_widget("Material Tiling")
                .and_then(|w| w.as_group_button())
        {
            button.set_index(match material.tiling.mode {
                denrim_materials::periodic::Mode::Raw => 0,
                denrim_materials::periodic::Mode::Repeat => 1,
                denrim_materials::periodic::Mode::Seamless => 2,
            });
        }
    }
    #[cfg(test)]
    fn set_view(ui: &mut TheUI, asset: &EditableMaterialAsset) {
        if let Ok(view) = view(asset) {
            if let Some(widget) = ui
                .get_widget(EDITOR)
                .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
            {
                widget.set_view(view);
            }
        }
    }
    pub fn update(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        let mut perf = crate::misc::FrameProfile::new();
        if let Some(map) = project
            .get_map_mut(server)
            .filter(|m| m.camera == rusterix::MapCamera::TwoDIso)
        {
            crate::iso_editor::sync_bounds_selection(map, &mut self.iso_bounds_selection);
        }
        crate::iso_editor::sync(ui, project, server, ctx);
        let mut changed = false;
        if let Some(w) = ui.get_widget("MZ Return to Mesh") {
            w.set_disabled(!self.mesh_material);
        }
        if project.ensure_material_tiles() {
            refresh_tiles(ctx);
            changed = true;
        }
        let iso_mode = project
            .get_map(server)
            .is_some_and(|m| m.camera == rusterix::MapCamera::TwoDIso);
        if self.iso_mode != iso_mode {
            self.iso_mode = iso_mode;
            self.preview_dirty = true;
        }
        if iso_mode || !project.isometric_bakes.is_empty() {
            let revision = rusterix::procedural_avatar::fingerprint(&(
                project.procedural_materials.revision(),
                project.procedural_appearances.revision(),
                palette::key(&project.palette),
                project
                    .procedural_material_cache_keys
                    .iter()
                    .collect::<Vec<_>>(),
                project.isometric_bakes.len(),
            ));
            if self.iso_revision != Some(revision) {
                self.iso_revision = Some(revision);
                let before = project.get_map(server).cloned();
                for error in shared::isometric::synchronize(project) {
                    Self::status(ui, ctx, error);
                }
                refresh_tiles(ctx);
                crate::utils::editor_scene_refresh_material_textures(project, server);
                if let (Some(before), Some(after)) = (before, project.get_map(server)) {
                    if before.changed != after.changed {
                        crate::utils::editor_scene_apply_map_edit(project, server, &before, after);
                    }
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Soft Update Minimap"),
                    TheValue::Empty,
                ));
                ctx.ui.redraw_all = true;
                self.preview_dirty = true;
                changed = true;
            }
        }
        if iso_mode {
            let revision = |p: &Project| {
                rusterix::procedural_avatar::fingerprint(
                    &p.regions
                        .iter()
                        .map(|r| (r.map.id, r.map.changed))
                        .collect::<Vec<_>>(),
                )
            };
            let current = revision(project);
            if self.iso_placement_revision != Some(current) {
                let before = project
                    .get_map(server)
                    .filter(|m| {
                        m.sectors.iter().any(|s| {
                            s.properties
                                .get_bool_default("iso_follow_mesh_bounds", false)
                        })
                    })
                    .cloned();
                let changed_maps = shared::isometric::placement::synchronize(project);
                self.iso_placement_revision = Some(revision(project));
                if let (Some(before), Some(after)) = (before, project.get_map(server)) {
                    if changed_maps.contains(&after.id) {
                        crate::utils::editor_scene_apply_map_edit(project, server, &before, after);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Soft Update Minimap"),
                            TheValue::Empty,
                        ));
                        ctx.ui.redraw_all = true;
                        changed = true;
                    }
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            changed |= self.model_sources.update(project, ctx);
        }
        let image_key = image_source::input_key(project);
        if self.image_source_key != Some(image_key) {
            self.image_source_key = Some(image_key);
            let mut updates = vec![];
            for (id, bundle) in &project.procedural_materials {
                if !bundle.contains("eldiron:tile:") {
                    continue;
                }
                if let Ok(asset) = EditableMaterialAsset::reopen(bundle) {
                    match image_source::refresh(&asset, project) {
                        Ok(Some(a)) => {
                            if let Ok(b) = a.save() {
                                updates.push((*id, b));
                            }
                        }
                        Err(e) => Self::status(ui, ctx, e),
                        _ => {}
                    }
                }
            }
            for (id, bundle) in updates {
                project.procedural_materials.insert(id, bundle);
                changed = true;
            }
            self.image_source_key = Some(image_source::input_key(project));
        }
        perf.lap("material_links");
        let config_key = cache_key(&project.config);
        if self.pixel_config_key != Some(config_key) {
            self.pixel_config_key = Some(config_key);
            let density = pixel_style::density(&project.config);
            if self.pixel_density != density {
                self.pixel_density = density;
                self.bundle = None;
                self.requested.clear();
                changed = true;
            }
        }
        let pixel_density = self.pixel_density;
        let palette_key = palette::key(&project.palette);
        if self.palette_key != Some(palette_key) {
            self.palette_key = Some(palette_key);
            for error in palette::refresh(project) {
                Self::status(ui, ctx, error);
            }
            if let Some(asset) = &self.asset {
                Self::set_project_view(ui, asset, project);
            }
            changed = true;
        }
        if self.map_binding.is_some() {
            self.follow_map_composition(ui, ctx, project, server);
        }
        perf.lap("material_palette");
        self.sync_compositions(project, ui, ctx);
        perf.lap("material_compositions");
        self.composition_view(ui, ctx, project);
        let preview_aspect = Self::preview_aspect(ui);
        let active = ui
            .get_stack_layout("Sidebar Page Stack")
            .is_some_and(|stack| stack.index() == SIDEBAR_PAGE);
        if self.active != active {
            self.active = active;
            self.preview_dirty = true;
            changed = true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // A fully baked map needs no material compiler/preview worker.
            // Initialize it only for editing or for an actual stale bake.
            let bake_revision = (project.procedural_materials.revision(), pixel_density);
            if self.bake_keys_revision != Some(bake_revision) {
                self.bake_keys = project
                    .procedural_materials
                    .iter()
                    .filter(|(id, _)| {
                        !shared::ruleset_materials::is_library_tile(&project.config, id)
                    })
                    .map(|(id, b)| (*id, pixel_style::bake_key(b, pixel_density)))
                    .collect();
                self.bake_keys_revision = Some(bake_revision);
            }
            let needs_worker = self.tile.is_some()
                || self.bake_keys.iter().any(|(id, key)| {
                    project.tiles.contains_key(id)
                        && !project.isometric_bakes.contains_key(id)
                        && project.procedural_material_cache_keys.get(id) != Some(key)
                });
            if self.worker.is_none() && needs_worker {
                if let Some((device, queue)) =
                    RUSTERIX.read().unwrap().scene_handler.vm.gpu_context()
                {
                    self.worker = Some(gpu::Worker::new(device, queue));
                }
            }
            if let Some(worker) = &self.worker {
                let mut installed_bakes = false;
                for result in worker.results.try_iter() {
                    if result.pixels_per_tile
                        != shared::ruleset_materials::resolution(&project.config, &result.tile)
                            .unwrap_or(pixel_density)
                        || project.procedural_materials.get(&result.tile) != Some(&result.bundle)
                        || (!project.tiles.contains_key(&result.tile)
                            && !shared::isometric::is_material_source(project, result.tile))
                    {
                        continue;
                    }
                    match result.frame {
                        Ok(frame) => {
                            let key = pixel_style::bake_key(&result.bundle, pixel_density);
                            let needs_bake =
                                project.procedural_material_cache_keys.get(&result.tile)
                                    != Some(&key);
                            if needs_bake
                                && project.tiles.contains_key(&result.tile)
                                && !project.isometric_bakes.contains_key(&result.tile)
                                && !shared::ruleset_materials::is_library_tile(
                                    &project.config,
                                    &result.tile,
                                )
                            {
                                let baked = project.tiles.get_mut(&result.tile).unwrap();
                                if let Some(pages) = &frame.surface_pages {
                                    baked.textures = pages.textures.clone();
                                    baked.baked_material_data =
                                        pages.textures.iter().map(|t| t.material_bytes()).collect();
                                    baked.uv_pages = Some(pages.layout.clone());
                                } else {
                                    baked.textures = vec![frame.tile.clone()];
                                    baked.baked_material_data = vec![frame.tile.material_bytes()];
                                    baked.uv_pages = None;
                                }
                                project
                                    .procedural_material_cache_keys
                                    .insert(result.tile, key);
                                installed_bakes = true;
                            }
                            if self.tile == Some(result.tile) && result.revision == self.revision {
                                let status = if frame.warnings.is_empty() {
                                    format!(
                                        "Procedural preview · GPU · {}",
                                        if pixel_density == 0 {
                                            "smooth".into()
                                        } else {
                                            format!("{pixel_density} pixels/tile")
                                        }
                                    )
                                } else {
                                    frame.warnings.join("; ")
                                };
                                if let Some(widget) = ui
                                    .get_widget(EDITOR)
                                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                                {
                                    for (key, pixels) in &frame.thumbnails {
                                        widget.set_thumbnail(
                                            key.clone(),
                                            TheRGBABuffer::from(pixels.clone(), 64, 64),
                                        );
                                    }
                                }
                                self.frame = Some(frame);
                                self.preview_dirty = true;
                                Self::status(ui, ctx, status);
                            }
                        }
                        Err(e) => Self::status(ui, ctx, format!("Procedural materials: {e}")),
                    }
                    changed = true;
                }
                if installed_bakes {
                    // Install completed tiles together before exposing one coherent scene.
                    refresh_tiles(ctx);
                    crate::utils::editor_scene_refresh_material_textures(project, server);
                }
                // The persisted cache key makes undo/redo and interrupted saves
                // rebuild correctly even when the material editor is closed.
                for (id, bundle) in &project.procedural_materials {
                    if shared::ruleset_materials::is_library_tile(&project.config, id)
                        || project.isometric_bakes.contains_key(id)
                    {
                        continue;
                    }
                    // The authoring request below renders and bakes this tile.
                    // Avoid racing a background-only bake ahead of its previews.
                    if self.tile == Some(*id) && self.bundle.as_ref() != Some(bundle) {
                        continue;
                    }
                    let key = self.bake_keys[id];
                    if project.tiles.contains_key(id)
                        && !self.model_sources.pending(*id)
                        && project.procedural_material_cache_keys.get(id) != Some(&key)
                        && self.requested.get(id) != Some(&key)
                    {
                        self.requested.insert(*id, key);
                        worker.send(gpu::Job {
                            pixels_per_tile: pixel_density,
                            tile: *id,
                            revision: 0,
                            bundle: bundle.clone(),
                            continuous: false,
                            selection: None,
                            preview_aspect,
                        });
                    }
                }
            }
        }
        perf.lap("material_bakes");
        let Some(tile) = self.tile else {
            return self.draw_preview(ui, server.animation_counter) || changed;
        };
        #[cfg(not(target_arch = "wasm32"))]
        if self.model_sources.pending(tile) {
            return self.draw_preview(ui, server.animation_counter) || changed;
        }
        let pixel_density =
            shared::ruleset_materials::resolution(&project.config, &tile).unwrap_or(pixel_density);
        if (!project.tiles.contains_key(&tile)
            && !shared::isometric::is_material_source(project, tile))
            || !project.procedural_materials.contains_key(&tile)
        {
            self.clear_selection(ui, ctx);
            return true;
        }
        let bundle = &project.procedural_materials[&tile];
        if self.bundle.as_ref() != Some(bundle) {
            self.bundle = Some(bundle.clone());
            self.revision = self.revision.wrapping_add(1);
            match EditableMaterialAsset::reopen(bundle)
                .map_err(|e| e.to_string())
                .and_then(|a| a.validate().map(|_| a).map_err(|e| e.to_string()))
            {
                Ok(asset) => {
                    Self::sync_tiling_button(ui, &asset);
                    if !project.material_compositions.contains_key(&tile) {
                        Self::set_project_view(ui, &asset, project);
                        if let Some(widget) = ui
                            .get_widget(EDITOR)
                            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                        {
                            self.selected_operation =
                                widget.selected_operation().map(str::to_owned);
                            self.selected_source = widget.selected_source().map(str::to_owned);
                            self.selected_input = widget.selected_input_preview();
                        }
                    }
                    self.asset = Some(asset);
                    Self::status(ui, ctx, "Compiling material…");
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(worker) = &self.worker {
                            self.requested
                                .insert(tile, pixel_style::bake_key(bundle, pixel_density));
                            self.preview_request = false;
                            worker.send(gpu::Job {
                                pixels_per_tile: pixel_density,
                                tile,
                                revision: self.revision,
                                bundle: bundle.clone(),
                                continuous: !shared::ruleset_materials::is_icon(
                                    &project.config,
                                    &tile,
                                ),
                                selection: if self.preview_tile {
                                    self.selection_target()
                                } else {
                                    None
                                },
                                preview_aspect,
                            });
                        } else {
                            self.bundle = None;
                            Self::status(ui, ctx, "Waiting for the renderer's GPU device…");
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    Self::status(
                        ui,
                        ctx,
                        "Material authoring GPU preview is currently available in desktop Creator.",
                    );
                }
                Err(e) => {
                    self.asset = None;
                    self.frame = None;
                    if let Some(widget) = ui
                        .get_widget(EDITOR)
                        .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                    {
                        widget.set_view(MaterialEditorView {
                            title: "Material unavailable".into(),
                            ..Default::default()
                        });
                    }
                    Self::status(ui, ctx, e);
                }
            }
            self.preview_dirty = true;
            changed = true;
        }
        if self.preview_request {
            self.preview_request = false;
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(worker) = &self.worker
                && (!self.preview_tile || self.selection_target().is_some())
            {
                self.revision = self.revision.wrapping_add(1);
                worker.send(gpu::Job {
                    pixels_per_tile: pixel_density,
                    tile,
                    revision: self.revision,
                    bundle: bundle.clone(),
                    continuous: !shared::ruleset_materials::is_icon(&project.config, &tile),
                    selection: if self.preview_tile {
                        self.selection_target()
                    } else {
                        None
                    },
                    preview_aspect,
                });
            }
        }
        changed |= self.draw_preview(ui, server.animation_counter);

        changed
    }
    fn selection_target(&self) -> Option<PreviewSelection> {
        if let Some(material) = &self.composition_preview {
            return Some(PreviewSelection::Material(material.clone()));
        }
        if let Some((target, reference)) = &self.selected_input {
            return Some(PreviewSelection::Input(target.clone(), reference.clone()));
        }
        self.selected_source
            .as_ref()
            .map(|id| PreviewSelection::Source(id.clone()))
            .or_else(|| {
                self.selected_operation
                    .as_ref()
                    .map(|id| PreviewSelection::Operation(id.clone()))
            })
    }
    fn preview_aspect(ui: &mut TheUI) -> f32 {
        ui.get_widget(PREVIEW)
            .map(|widget| {
                let dim = widget.dim();
                if dim.height > 0 {
                    dim.width as f32 / dim.height as f32
                } else {
                    1.
                }
            })
            .unwrap_or(1.)
    }
    fn draw_preview(&mut self, ui: &mut TheUI, tick: usize) -> bool {
        let checker = [
            *ui.style.theme().color(MaterialCheckerDark),
            *ui.style.theme().color(MaterialCheckerLight),
        ];
        if self.preview_checker != Some(checker) {
            self.preview_checker = Some(checker);
            self.preview_dirty = true;
        }
        let animation_frame = self.frame.as_ref().map_or(0, |f| {
            if f.animation_images.is_empty() {
                0
            } else {
                tick % f.animation_images.len()
            }
        });
        if animation_frame != self.preview_animation_frame {
            self.preview_animation_frame = animation_frame;
            self.preview_dirty = true;
        }
        if self.active
            && let Some(widget) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view())
        {
            let dim = *widget.dim();
            let buffer = widget.render_buffer_mut();
            buffer.resize(dim.width, dim.height);
            let size = (buffer.pixel_width(), buffer.pixel_height());
            if size != self.preview_size || self.preview_dirty {
                if size != self.preview_size {
                    self.preview_request = true;
                }
                self.preview_size = size;
                self.preview_dirty = false;
                let image = self.frame.as_ref().and_then(|frame| {
                    let pixels = if self.preview_tile {
                        frame
                            .selection
                            .as_ref()
                            .filter(|(target, _)| Some(target) == self.selection_target().as_ref())
                            .map(|(_, pixels)| {
                                frame
                                    .animation_selections
                                    .get(animation_frame)
                                    .unwrap_or(pixels)
                                    .as_slice()
                            })
                    } else {
                        frame
                            .animation_images
                            .get(animation_frame)
                            .unwrap_or(&frame.images)
                            .get(PREVIEW_MAPS[self.channel.min(PREVIEW_MAPS.len() - 1)])
                            .map(Vec::as_slice)
                    };
                    pixels.map(|pixels| (frame.size, pixels, frame.continuous))
                });
                // Materials are projection-independent sources. Iso projection
                // belongs to their map/model outputs, not this source preview.
                draw_preview_with_colors(buffer, image, checker);
                return true;
            }
        }
        false
    }
}
/// Stable across process launches and Rust versions (unlike DefaultHasher).
fn cache_key(bundle: &str) -> u64 {
    bundle.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}
fn refresh_tiles(ctx: &mut TheContext) {
    // The caller already installs textures in the scene. Update Tiles would
    // clear/reload them a second time through Sidebar::update_tiles.
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Update Tilepicker Textures"),
        TheValue::Empty,
    ));
}
#[cfg(test)]
fn draw_preview(buffer: &mut TheRGBABuffer, image: Option<(usize, &[u8], bool)>) {
    let theme = TheDarkTheme::new();
    draw_preview_with_colors(
        buffer,
        image,
        [
            *theme.color(MaterialCheckerDark),
            *theme.color(MaterialCheckerLight),
        ],
    );
}
fn draw_preview_with_colors(
    buffer: &mut TheRGBABuffer,
    image: Option<(usize, &[u8], bool)>,
    checker_colors: [RGBA; 2],
) {
    let w = buffer.pixel_width();
    let h = buffer.pixel_height();
    if w == 0 || h == 0 {
        return;
    }
    let tile_height = h as f32 / 2.;
    for (index, pixel) in buffer.pixels_mut().chunks_exact_mut(4).enumerate() {
        let x = index % w;
        let y = index / w;
        let checker = checker_colors[(x / 12 + y / 12) % 2];
        let mut color = checker;
        if let Some((size, pixels, continuous)) = image {
            let (u, v) = if continuous {
                (x as f32 / w as f32, y as f32 / h as f32)
            } else {
                (
                    (x as f32 / tile_height).fract(),
                    (y as f32 / tile_height).fract(),
                )
            };
            let sx = (u * size as f32) as usize;
            let sy = (v * size as f32) as usize;
            let at = (sy.min(size - 1) * size + sx.min(size - 1)) * 4;
            let source = &pixels[at..at + 4];
            let alpha = source[3] as f32 / 255.;
            for c in 0..3 {
                color[c] =
                    (source[c] as f32 * alpha + checker[c] as f32 * (1. - alpha)).round() as u8;
            }
        }
        pixel.copy_from_slice(&color);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod gpu;
#[cfg(target_arch = "wasm32")]
mod gpu {
    pub struct Frame {
        pub size: usize,
        pub animation_images: Vec<[Vec<u8>; 5]>,
        pub animation_selections: Vec<Vec<u8>>,
        pub images: [Vec<u8>; 5],
        pub thumbnails: Vec<(String, Vec<u8>)>,
        pub selection: Option<(super::PreviewSelection, Vec<u8>)>,
        pub continuous: bool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> EditableMaterialAsset {
        starter(Uuid::new_v4()).unwrap()
    }
    #[test]
    #[ignore = "requires native GPU for shared mesh material preview and scene publication"]
    fn mesh_only_material_is_editable_and_refreshes_linked_geometry_tiles() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut p = Project::default();
        p.config = "[ruleset]\nsource='project'\n".into();
        let material = Uuid::new_v4();
        let model = Uuid::new_v4();
        let source = EditableMaterialAsset::parse(
            &material.to_string(),
            r#"project 1
material "surface" called "Wall Bricks"
  mode pbr
  row "color"
    use "Color" id "brick-color"
      set "color" to 0.7 0.1 0.02 1
  output base color from row "color" xyz
"#,
            "surface",
        )
        .unwrap();
        let original = source.save().unwrap();
        p.procedural_materials.insert(material, original.clone());
        p.procedural_appearances.insert(model,format!(r#"{{"version":1,"id":"wall","label":"Wall","rows":[{{"id":"wall","label":"Wall","tiles":[{{"id":"box","op":{{"type":"box","size":[1,1,0.22]}}}},{{"id":"bricks","op":{{"type":"texture","source":"{material}","size":32,"axes":[0,1],"origin":[-0.5,-0.5],"span":[1,1]}}}}]}}],"output":"wall"}}"#));
        let outputs = [Uuid::new_v4(), Uuid::new_v4()];
        for (i, output) in outputs.iter().enumerate() {
            p.isometric_bakes.insert(
                *output,
                shared::isometric::Bake {
                    recipe: model,
                    ground: false,
                    yaw: i as f32 * 90.,
                    directions: 1,
                    motion: None,
                    frames: 1,
                    signature: 0,
                },
            );
        }
        let map = &mut p.regions[0].map;
        map.camera = rusterix::MapCamera::TwoDIso;
        let v = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        map.find_sector_mut(id).unwrap().properties.set(
            "source",
            rusterix::Value::Source(rusterix::PixelSource::TileId(outputs[0])),
        );
        let mut server = ServerContext::default();
        server.curr_region = p.regions[0].id;
        let mut host = ProceduralMaterialEditor::default();
        host.handle_event(
            &TheEvent::Custom(TheId::named("Open Mesh Material"), TheValue::Id(material)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        assert_eq!(host.tile, Some(material));
        assert_eq!(p.tiles[&material].alias, "Wall Bricks");
        assert!(
            !shared::isometric::is_material_source(&p, material),
            "This material has no ground-output route"
        );
        let old = outputs.map(|id| p.tiles[&id].textures.clone());
        let changed = edited_for_project(
            &source,
            MaterialEditorCommand::SetNumber {
                operation: "brick-color".into(),
                parameter: "color:0".into(),
                value: 0.02,
            },
            &p,
        )
        .unwrap()
        .save()
        .unwrap();
        p.procedural_materials.insert(material, changed.clone());
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        for (i, id) in outputs.iter().enumerate() {
            assert_ne!(p.tiles[id].textures, old[i]);
            assert_eq!(
                RUSTERIX.read().unwrap().assets.tiles[id].textures,
                p.tiles[id].textures
            );
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while host.frame.is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
            host.update(&mut ui, &mut ctx, &mut p, &mut server);
        }
        assert!(
            host.frame.is_some(),
            "Mesh-only material preview was discarded"
        );
        let atom = ProjectUndoAtom::ProceduralMaterialEdit(material, original, changed);
        atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        for (i, id) in outputs.iter().enumerate() {
            assert_eq!(p.tiles[id].textures, old[i]);
        }
        atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        for (i, id) in outputs.iter().enumerate() {
            assert_ne!(p.tiles[id].textures, old[i]);
        }
    }
    #[test]
    fn installed_bakes_request_one_picker_refresh_without_a_second_tile_reset() {
        let mut ctx = TheContext::new(64, 64, 1.);
        let (sender, receiver) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(sender);
        refresh_tiles(&mut ctx);
        let events: Vec<_> = receiver.try_iter().collect();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0],TheEvent::Custom(id,_) if id.name=="Update Tilepicker Textures")
        );
    }
    #[test]
    fn deleting_mix_keeps_row_ports_and_reassigns_them_after_save_and_undo() {
        let project = Project::default();
        let original = palette::starter(Uuid::new_v4(), &project).unwrap();
        let row = material(&original)
            .unwrap()
            .rows
            .iter()
            .find(|row| row.tiles.iter().any(|tile| tile.kernel == "Mix"))
            .unwrap();
        let row_id = row.id.clone();
        let mix_id = row
            .tiles
            .iter()
            .find(|tile| tile.kernel == "Mix")
            .unwrap()
            .id
            .clone();
        let before = original.save().unwrap();
        let deleted = edited(&original, MaterialEditorCommand::RemoveOperation(mix_id)).unwrap();
        let row = material(&deleted)
            .unwrap()
            .rows
            .iter()
            .find(|row| row.id == row_id)
            .unwrap();
        assert_eq!(row.incoming.len(), 3);
        assert_eq!(row_inputs::unassigned(row).len(), 3);
        assert_eq!(
            view(&deleted)
                .unwrap()
                .rows
                .iter()
                .find(|row| row.id == row_id)
                .unwrap()
                .inputs
                .iter()
                .filter(|input| matches!(input.target, MaterialInputTarget::RowLink { .. }))
                .count(),
            3
        );
        let link = row
            .incoming
            .iter()
            .find(|link| link.source.row == "palette-dark")
            .unwrap()
            .clone();
        let reopened = EditableMaterialAsset::reopen(&deleted.save().unwrap()).unwrap();
        let restored = EditableMaterialAsset::reopen(&before).unwrap();
        assert!(
            material(&restored)
                .unwrap()
                .rows
                .iter()
                .any(|row| row.tiles.iter().any(|tile| tile.kernel == "Mix"))
        );
        let mix = source_catalog()
            .into_iter()
            .find(|s| s.metadata.name == "Mix")
            .unwrap();
        let added = edited(
            &reopened,
            MaterialEditorCommand::AddSource {
                source: mix.id,
                row: Some(row_id.clone()),
                after: None,
            },
        )
        .unwrap();
        let operation = material(&added)
            .unwrap()
            .rows
            .iter()
            .find(|row| row.id == row_id)
            .unwrap()
            .tiles
            .last()
            .unwrap()
            .id
            .clone();
        let assigned = edited(
            &added,
            MaterialEditorCommand::AssignRowInput {
                row: row_id.clone(),
                connection: link.id.clone(),
                target: MaterialInputTarget::Parameter {
                    operation: operation.clone(),
                    parameter: "color1".into(),
                },
            },
        )
        .unwrap();
        let row = material(&assigned)
            .unwrap()
            .rows
            .iter()
            .find(|row| row.id == row_id)
            .unwrap();
        assert_eq!(row.incoming.len(), 3);
        assert_eq!(row_inputs::unassigned(row).len(), 2);
        assert!(row.tiles.last().unwrap().parameters.iter().any(
            |(key, binding)| key == "color1" && *binding == Binding::From(link.source.clone())
        ));
    }
    #[test]
    fn pbr_tiling_shape_combine_source_and_metadata_are_editable() {
        let original = fixture();
        let tiled = edited(
            &original,
            MaterialEditorCommand::SetTiling {
                mode: "seamless".into(),
                period: [2., 3.],
                blend: 0.2,
            },
        )
        .unwrap();
        let tiled = EditableMaterialAsset::reopen(&tiled.save().unwrap()).unwrap();
        assert_eq!(
            view(&tiled).unwrap().tiling,
            MaterialTilingView {
                mode: "seamless".into(),
                period: [2., 3.],
                blend: 0.2
            }
        );
        assert!(
            edited(
                &tiled,
                MaterialEditorCommand::SetTiling {
                    mode: "repeat".into(),
                    period: [0., 1.],
                    blend: 0.2
                }
            )
            .is_err()
        );
        let shape = source_catalog()
            .into_iter()
            .find(|source| source.metadata.name == "Circle")
            .unwrap();
        let added = edited(
            &original,
            MaterialEditorCommand::AddSource {
                source: shape.id,
                row: None,
                after: None,
            },
        )
        .unwrap();
        let first = &material(&added).unwrap().rows.last().unwrap();
        let shape = source_catalog()
            .into_iter()
            .find(|source| source.metadata.name == "Circle")
            .unwrap();
        let added = edited(
            &added,
            MaterialEditorCommand::AddSource {
                source: shape.id,
                row: Some(first.id.clone()),
                after: Some(first.tiles[0].id.clone()),
            },
        )
        .unwrap();
        let operation = material(&added)
            .unwrap()
            .rows
            .last()
            .unwrap()
            .tiles
            .last()
            .unwrap()
            .id
            .clone();
        let combined = edited(
            &added,
            MaterialEditorCommand::SetCombine {
                operation: operation.clone(),
                mode: "smooth_union".into(),
            },
        )
        .unwrap();
        assert_eq!(
            material(&combined)
                .unwrap()
                .rows
                .last()
                .unwrap()
                .tiles
                .last()
                .unwrap()
                .combine,
            format::Combine::SmoothUnion
        );
        let presentation = view(&combined).unwrap();
        let shape = presentation.rows.last().unwrap().operations.last().unwrap();
        assert!(
            shape
                .properties
                .iter()
                .any(|property| property.kind == MaterialPropertyKind::Vector)
        );
        let source = shape.source.as_ref().unwrap().code.clone();
        let custom = edited(
            &combined,
            MaterialEditorCommand::SetSource {
                operation: operation.clone(),
                code: format!("{source}\n# custom version\n"),
            },
        )
        .unwrap();
        assert!(
            material(&custom)
                .unwrap()
                .rows
                .last()
                .unwrap()
                .tiles
                .last()
                .unwrap()
                .code
                .is_some()
        );
        assert!(
            edited(
                &custom,
                MaterialEditorCommand::SetSource {
                    operation: operation.clone(),
                    code: "invalid code".into()
                }
            )
            .is_err()
        );
        let reset = edited(
            &custom,
            MaterialEditorCommand::SetSource {
                operation,
                code: String::new(),
            },
        )
        .unwrap();
        assert!(
            material(&reset)
                .unwrap()
                .rows
                .last()
                .unwrap()
                .tiles
                .last()
                .unwrap()
                .code
                .is_none()
        );
    }
    #[test]
    fn input_preview_applies_swizzle_remap_and_does_not_mutate_recipe() {
        use denrim_materials::{cpu::Evaluator, integration::InstanceRecord, ir::Channel};
        let original = fixture();
        let before = original.save().unwrap();
        for channels in ["x", "xy", "xyz", "xyzw"] {
            let reference = MaterialReferenceView {
                row: "color".into(),
                components: channels.into(),
                mapping: Some([0.1, 0.8]),
            };
            let preview = input_preview_asset(
                &original,
                &MaterialInputTarget::Coordinates {
                    row: "rough".into(),
                },
                &reference,
            )
            .unwrap();
            let program = preview.program(&InstanceRecord::new("preview")).unwrap();
            let evaluator = Evaluator::new(&program);
            let actual = evaluator
                .sample(&preview.selected_material, Channel::BaseColor, [0.2, 0.3])
                .unwrap();
            let source = original.program(&InstanceRecord::new("source")).unwrap();
            let expected = Evaluator::new(&source)
                .sample(&original.selected_material, Channel::BaseColor, [0.2, 0.3])
                .unwrap();
            assert!((actual[0] - (0.1 + 0.7 * expected[0])).abs() < 1e-5);
            if channels == "x" {
                assert_eq!(actual[0], actual[1]);
            }
            if channels == "xy" {
                assert_eq!(actual[2], 0.);
            }
        }
        assert_eq!(original.save().unwrap(), before);
    }
    #[test]
    fn row_input_bindings_round_trip_and_invalid_edits_are_transactional() {
        let asset = fixture();
        let reference = |row: &str, components: &str, mapping| {
            MaterialInputBinding::Row(MaterialReferenceView {
                row: row.into(),
                components: components.into(),
                mapping,
            })
        };
        let coords = MaterialInputTarget::Coordinates {
            row: "color".into(),
        };
        let next = edited(
            &asset,
            MaterialEditorCommand::SetInput {
                target: coords.clone(),
                binding: reference("grain", "xy", Some([-1., 2.])),
            },
        )
        .unwrap();
        let next = edited(
            &next,
            MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Image {
                    operation: "pigment".into(),
                },
                binding: reference("grain", "x", None),
            },
        )
        .unwrap();
        let next = edited(
            &next,
            MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Parameter {
                    operation: "pigment".into(),
                    parameter: "color1".into(),
                },
                binding: reference("grain", "xyzw", None),
            },
        )
        .unwrap();
        let reopened = EditableMaterialAsset::reopen(&next.save().unwrap()).unwrap();
        let mat = material(&reopened).unwrap();
        assert_eq!(
            mat.rows[0].coordinates.as_ref().unwrap().mapping,
            Some((-1., 2.))
        );
        assert_eq!(
            mat.rows[0].tiles[1].input.as_ref().unwrap().components,
            "xxx"
        );
        assert!(view(&reopened).unwrap().rows[0].inputs.iter().any(|input|matches!(input.target,MaterialInputTarget::Parameter{ref parameter,..} if parameter=="color1") && matches!(input.binding,MaterialInputBinding::Row(_))));
        let saved = next.save().unwrap();
        for command in [
            MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Coordinates {
                    row: "grain".into(),
                },
                binding: reference("color", "xy", None),
            },
            MaterialEditorCommand::SetInput {
                target: coords.clone(),
                binding: reference("missing", "xy", None),
            },
            MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Parameter {
                    operation: "pigment".into(),
                    parameter: "color1".into(),
                },
                binding: reference("grain", "xyz", None),
            },
            MaterialEditorCommand::SetInput {
                target: coords.clone(),
                binding: reference("grain", "xy", Some([f32::NAN, 1.])),
            },
        ] {
            assert!(edited(&next, command).is_err());
            assert_eq!(next.save().unwrap(), saved);
        }
        let cleared = edited(
            &next,
            MaterialEditorCommand::SetInput {
                target: coords,
                binding: MaterialInputBinding::Default,
            },
        )
        .unwrap();
        assert!(material(&cleared).unwrap().rows[0].coordinates.is_none());
        let cleared = edited(
            &cleared,
            MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Image {
                    operation: "pigment".into(),
                },
                binding: MaterialInputBinding::Disconnected,
            },
        )
        .unwrap();
        assert!(material(&cleared).unwrap().rows[0].tiles[1].input_disconnected);
    }
    #[test]
    fn palette_color_connections_prune_bindings_and_row_edits_use_project_undo() {
        let mut project = Project::default();
        let id = Uuid::new_v4();
        let asset = palette::starter(id, &project).unwrap();
        let connected = edited_for_project(
            &asset,
            MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Parameter {
                    operation: "palette-a".into(),
                    parameter: "color".into(),
                },
                binding: MaterialInputBinding::Row(MaterialReferenceView {
                    row: "grain".into(),
                    components: "xyzw".into(),
                    mapping: None,
                }),
            },
            &project,
        )
        .unwrap();
        assert_eq!(palette::binding_index(&connected, "palette-a"), None);
        assert!(palette::binding_index(&connected, "palette-b").is_some());
        let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(4, 4)]);
        project.tiles.insert(id, tile);
        let before = asset.save().unwrap();
        let after = connected.save().unwrap();
        project.procedural_materials.insert(id, after.clone());
        project.name = "Keep rename".into();
        let (mut ui, mut ctx) = workspace_ui(1200, 850);
        let mut server = ServerContext::default();
        let atom = ProjectUndoAtom::ProceduralMaterialEdit(id, before.clone(), after.clone());
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&id], before);
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&id], after);
        assert_eq!(project.name, "Keep rename");
    }
    #[test]
    fn metadata_supplies_all_parameter_components_and_pbr_outputs() {
        let asset = fixture();
        let v = view(&asset).unwrap();
        assert_eq!(
            v.outputs
                .iter()
                .map(|output| output.id.as_str())
                .collect::<Vec<_>>(),
            CHANNELS
        );
        let pigment = v
            .rows
            .iter()
            .flat_map(|r| &r.operations)
            .find(|t| t.id == "pigment")
            .unwrap();
        assert!(pigment.parameters.iter().any(|p| p.id == "color1:0"));
        assert!(pigment.parameters.iter().any(|p| p.id == "color2:3"));
        assert!(v.defaults.iter().any(|p| p.id == "opacity:0"));
        assert!(v.sources.iter().any(|s| s.label == "Perlin Noise"));
        let project = Project::default();
        let palette_asset = palette::starter(Uuid::new_v4(), &project).unwrap();
        let emission = |asset: &EditableMaterialAsset| {
            view(asset)
                .unwrap()
                .outputs
                .into_iter()
                .find(|output| output.id == "emission")
                .unwrap()
                .fallback
        };
        assert_eq!(emission(&palette_asset), "0");
        let enabled = edited_for_project(
            &palette_asset,
            MaterialEditorCommand::SetNumber {
                operation: "@surface".into(),
                parameter: "emission strength:0".into(),
                value: 2.,
            },
            &project,
        )
        .unwrap();
        assert_ne!(
            emission(&enabled),
            "0",
            "Enabling emission must display its actual palette color times strength"
        );
    }
    #[test]
    fn validated_edits_are_transactional_and_keep_stable_identities() {
        let asset = fixture();
        let before = asset.save().unwrap();
        assert!(
            edited(
                &asset,
                MaterialEditorCommand::SetNumber {
                    operation: "pigment".into(),
                    parameter: "color1:0".into(),
                    value: f32::NAN
                }
            )
            .is_err()
        );
        assert!(
            edited(
                &asset,
                MaterialEditorCommand::AddSource {
                    source: "missing".into(),
                    row: None,
                    after: None
                }
            )
            .is_err()
        );
        assert_eq!(asset.save().unwrap(), before);
        let changed = edited(
            &asset,
            MaterialEditorCommand::SetNumber {
                operation: "pigment".into(),
                parameter: "color1:0".into(),
                value: 0.75,
            },
        )
        .unwrap();
        let saved = changed.save().unwrap();
        let reopened = EditableMaterialAsset::reopen(&saved).unwrap();
        assert_eq!(material(&reopened).unwrap().rows[0].tiles[1].id, "pigment");
        assert_eq!(material(&reopened).unwrap().outputs[0].source.row, "color");
        let p = reopened
            .parameter_metadata()
            .unwrap()
            .into_iter()
            .find(|p| p.key.tile == "pigment" && p.key.parameter == "color1")
            .unwrap();
        assert_eq!(p.binding, Binding::Constant(vec![0.75, 0.16, 0.025, 1.]));
    }
    #[test]
    fn source_drop_connect_disconnect_and_surface_opacity_validate() {
        let asset = fixture();
        let source = source_catalog()
            .into_iter()
            .find(|s| s.metadata.name == "Color")
            .unwrap();
        let changed = edited(
            &asset,
            MaterialEditorCommand::AddSource {
                source: source.id,
                row: None,
                after: None,
            },
        )
        .unwrap();
        let row = material(&changed).unwrap().rows.last().unwrap().id.clone();
        let changed = edited(
            &changed,
            MaterialEditorCommand::ConnectOutput {
                output: "opacity".into(),
                row: row.clone(),
            },
        )
        .unwrap();
        assert!(
            material(&changed)
                .unwrap()
                .outputs
                .iter()
                .any(|o| o.channel == "opacity" && o.source.row == row)
        );
        let changed = edited(
            &changed,
            MaterialEditorCommand::DisconnectOutput("opacity".into()),
        )
        .unwrap();
        let changed = edited(
            &changed,
            MaterialEditorCommand::SetNumber {
                operation: "@surface".into(),
                parameter: "opacity:0".into(),
                value: 0.4,
            },
        )
        .unwrap();
        assert!(
            material(&changed)
                .unwrap()
                .defaults
                .contains(&("opacity".into(), vec![0.4]))
        );
    }
    #[test]
    fn recipe_and_cache_survive_project_save_and_deletion_removes_both() {
        let mut project = Project::default();
        let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(2, 2)]);
        let id = tile.id;
        let bundle = starter(id).unwrap().save().unwrap();
        project.tiles.insert(id, tile);
        project.procedural_materials.insert(id, bundle.clone());
        project
            .procedural_material_cache_keys
            .insert(id, cache_key(&bundle));
        let mut reopened: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        assert_eq!(reopened.procedural_materials.get(&id), Some(&bundle));
        assert_eq!(
            reopened.procedural_material_cache_keys.get(&id),
            Some(&cache_key(&bundle))
        );
        assert!(reopened.tiles.contains_key(&id));
        reopened.remove_tile(&id);
        assert!(!reopened.procedural_materials.contains_key(&id));
        assert!(!reopened.procedural_material_cache_keys.contains_key(&id));
        let mut old = serde_json::to_value(project).unwrap();
        old.as_object_mut().unwrap().remove("procedural_materials");
        old.as_object_mut()
            .unwrap()
            .remove("procedural_material_cache_keys");
        let old: Project = serde_json::from_value(old).unwrap();
        assert!(old.procedural_materials.is_empty());
    }
    #[test]
    fn duplicate_recipe_has_independent_asset_identity() {
        let asset = fixture();
        let original = asset.save().unwrap();
        let id = Uuid::new_v4();
        let duplicate = duplicate_bundle(&original, id).unwrap();
        let duplicate = EditableMaterialAsset::reopen(&duplicate).unwrap();
        assert_eq!(duplicate.root.id, id.to_string());
        assert_ne!(duplicate.root.id, asset.root.id);
        let changed = edited(
            &duplicate,
            MaterialEditorCommand::SetNumber {
                operation: "pigment".into(),
                parameter: "color1:0".into(),
                value: 0.9,
            },
        )
        .unwrap();
        assert_ne!(changed.save().unwrap(), original);
        assert_eq!(asset.save().unwrap(), original);
    }
    #[test]
    fn material_undo_restores_only_the_recipe_and_invalidates_the_cache() {
        let mut project = Project::default();
        let mut tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(2, 2)]);
        tile.alias = "Keep metadata".into();
        let id = tile.id;
        let asset = starter(id).unwrap();
        let before = asset.save().unwrap();
        let after = edited(
            &asset,
            MaterialEditorCommand::SetNumber {
                operation: "@surface".into(),
                parameter: "opacity:0".into(),
                value: 0.3,
            },
        )
        .unwrap()
        .save()
        .unwrap();
        project.tiles.insert(id, tile);
        project.procedural_materials.insert(id, after.clone());
        project
            .procedural_material_cache_keys
            .insert(id, cache_key(&after));
        let atom = ProjectUndoAtom::ProceduralMaterialEdit(id, before.clone(), after.clone());
        let (mut ui, mut ctx) = ui_fixture();
        let mut server = ServerContext::default();
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&id], before);
        assert!(!project.procedural_material_cache_keys.contains_key(&id));
        assert_eq!(project.tiles[&id].alias, "Keep metadata");
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&id], after);
        assert_eq!(project.tiles[&id].alias, "Keep metadata");
    }
    #[test]
    fn manual_sidebar_navigation_switches_preview_and_leaves_map_keyboard_input_available() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut host = ProceduralMaterialEditor::default();
        let mut project = Project::default();
        let mut server = ServerContext::default();
        crate::sidebar::Sidebar::set_navigation_page(SIDEBAR_PAGE, &mut ui, &mut ctx);
        host.update(&mut ui, &mut ctx, &mut project, &mut server);
        assert!(host.active);
        assert_eq!(ui.get_stack_layout(PREVIEW_STACK).unwrap().index(), 1);
        let delete = TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete));
        ctx.ui.set_focus(&TheId::named("PolyView"));
        assert!(!host.handle_event(&delete, &mut ui, &mut ctx, &mut project, &mut server));
        ctx.ui.set_focus(&TheId::named(EDITOR));
        assert!(host.handle_event(&delete, &mut ui, &mut ctx, &mut project, &mut server));
        assert!(!host.handle_event(
            &TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Tab)),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        crate::sidebar::Sidebar::set_navigation_page(3, &mut ui, &mut ctx);
        host.update(&mut ui, &mut ctx, &mut project, &mut server);
        assert!(!host.active);
        assert!(ctx.ui.focus.is_none());
        assert_eq!(ui.get_stack_layout(PREVIEW_STACK).unwrap().index(), 0);
        assert_eq!(ui.get_stack_layout("Game Output Stack").unwrap().index(), 1);
    }
    #[test]
    #[ignore = "requires native GPU for MZ preview and editor texture publication"]
    fn isometric_material_recipe_edits_refresh_tiles_and_scene_automatically() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut p = Project::default();
        p.config = "[ruleset]\nsource='project'\n".into();
        let recipe = Uuid::new_v4();
        let source = starter(recipe).unwrap();
        p.procedural_materials
            .insert(recipe, source.save().unwrap());
        let outputs = [Uuid::new_v4(), Uuid::new_v4()];
        for id in outputs {
            p.isometric_bakes.insert(
                id,
                shared::isometric::Bake {
                    recipe,
                    ground: true,
                    yaw: 0.,
                    directions: 1,
                    motion: None,
                    frames: 1,
                    signature: 0,
                },
            );
        }
        let map = &mut p.regions[0].map;
        *map = Map::default();
        map.camera = rusterix::MapCamera::TwoDIso;
        map.grid_size = 40.;
        let v = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let sector = map.close_polygon_manual().unwrap();
        map.find_sector_mut(sector).unwrap().properties.set(
            "source",
            rusterix::Value::Source(rusterix::PixelSource::TileId(outputs[0])),
        );
        let mut server = ServerContext::default();
        server.curr_region = p.regions[0].id;
        server.curr_tile_id = Some(outputs[0]);
        let mut host = ProceduralMaterialEditor::default();
        host.handle_event(
            &TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(outputs[0])),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        assert_eq!(host.tile, Some(recipe));
        assert!(p.tiles.contains_key(&recipe));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while host.frame.is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
            host.update(&mut ui, &mut ctx, &mut p, &mut server);
        }
        assert!(
            host.frame.is_some(),
            "A source without a flat tile must still receive its MZ preview"
        );
        let old = p.tiles[&outputs[0]].textures.clone();
        let draw = |p: &Project| {
            let mut pixels = vec![0; 320 * 240 * 4];
            RUSTERIX
                .write()
                .unwrap()
                .draw_d2(&p.regions[0].map, &mut pixels, 320, 240);
            pixels
        };
        let before = draw(&p);
        let next = edited(
            &source,
            MaterialEditorCommand::SetNumber {
                operation: "pigment".into(),
                parameter: "color2:0".into(),
                value: 0.99,
            },
        )
        .unwrap()
        .save()
        .unwrap();
        let atom = ProjectUndoAtom::ProceduralMaterialEdit(
            recipe,
            p.procedural_materials[&recipe].clone(),
            next.clone(),
        );
        p.procedural_materials.insert(recipe, next);
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        for id in outputs {
            assert_ne!(p.tiles[&id].textures, old);
            assert_eq!(
                RUSTERIX.read().unwrap().assets.tiles[&id].textures,
                p.tiles[&id].textures
            );
        }
        assert_ne!(draw(&p), before);
        atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        assert_eq!(draw(&p), before);
        atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        assert_ne!(draw(&p), before);
        // A source and its iso output may also share an ID. A late flat-preview
        // result must never replace the projected output's pixels.
        p.isometric_bakes.insert(
            recipe,
            shared::isometric::Bake {
                recipe,
                ground: true,
                yaw: 0.,
                directions: 1,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        let projected = p.tiles[&recipe].clone();
        let preview = host.frame.take().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        host.worker.as_mut().unwrap().results = receiver;
        sender
            .send(gpu::Completed {
                pixels_per_tile: pixel_style::density(&p.config),
                tile: recipe,
                bundle: p.procedural_materials[&recipe].clone(),
                revision: host.revision,
                frame: Ok(preview),
            })
            .unwrap();
        host.update(&mut ui, &mut ctx, &mut p, &mut server);
        assert_eq!(
            p.tiles[&recipe], projected,
            "Late flat previews must not overwrite projected tile data"
        );
    }

    #[test]
    fn picking_isometric_material_tiles_edits_the_source_and_supports_undo() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut project = Project::default();
        let mut server = ServerContext::default();
        let recipe = Uuid::new_v4();
        let output = Uuid::new_v4();
        let before = starter(recipe).unwrap().save().unwrap();
        project.procedural_materials.insert(recipe, before.clone());
        project.tiles.insert(output, rusterix::Tile::empty());
        project.isometric_bakes.insert(
            output,
            shared::isometric::Bake {
                recipe,
                ground: true,
                yaw: 0.,
                directions: 1,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
        server.curr_tile_id = Some(output);
        let mut host = ProceduralMaterialEditor::default();
        host.handle_event(
            &TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(output)),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(host.tile, Some(recipe));
        assert_eq!(server.curr_tile_id, Some(output));
        assert!(!project.tiles.contains_key(&recipe));
        assert!(host.handle_event(
            &TheEvent::IndexChanged(TheId::named("Material Tiling"), 1),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let after = project.procedural_materials[&recipe].clone();
        assert_ne!(before, after);
        assert!(!project.procedural_materials.contains_key(&output));
        let atom = ProjectUndoAtom::ProceduralMaterialEdit(recipe, before.clone(), after.clone());
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&recipe], before);
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&recipe], after);
    }

    #[test]
    fn selecting_procedural_material_reselects_its_tab_after_manual_navigation() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut host = ProceduralMaterialEditor::default();
        let id = Uuid::new_v4();
        let mut project = Project::default();
        project
            .procedural_materials
            .insert(id, starter(id).unwrap().save().unwrap());
        host.open(id, &mut ui, &mut ctx, &project);
        crate::sidebar::Sidebar::set_navigation_page(5, &mut ui, &mut ctx);
        host.open(id, &mut ui, &mut ctx, &project);
        assert_eq!(
            ui.get_stack_layout("Sidebar Page Stack").unwrap().index(),
            SIDEBAR_PAGE
        );
        assert_eq!(ui.get_stack_layout(PREVIEW_STACK).unwrap().index(), 1);
        assert_eq!(ui.get_stack_layout("Game Output Stack").unwrap().index(), 1);
        host.leave(&mut ui, &mut ctx);
        assert_eq!(
            ui.get_stack_layout("Sidebar Page Stack").unwrap().index(),
            5
        );
        assert_eq!(
            ui.get_widget("Sidebar Tabs")
                .unwrap()
                .as_group_button()
                .unwrap()
                .index(),
            5
        );
    }
    #[test]
    fn catalogue_preview_uses_real_sources_without_mutating_the_material() {
        let asset = fixture();
        let original = asset.save().unwrap();
        for source in source_catalog() {
            let preview = source_preview_asset(&source.id).unwrap();
            let mat = material(&preview).unwrap();
            assert_eq!(
                mat.rows[0].tiles.last().unwrap().kernel,
                source.metadata.name
            );
            assert_eq!(
                mat.rows[0].tiles.len(),
                if source.requires_input { 2 } else { 1 }
            );
            assert_eq!(mat.outputs[0].source.row, "source");
        }
        assert_eq!(asset.save().unwrap(), original);
        assert!(source_preview_asset("unknown-source").is_err());
        let mut host = ProceduralMaterialEditor::default();
        host.selected_operation = Some("pigment".into());
        host.selected_source = Some("source".into());
        assert_eq!(
            host.selection_target(),
            Some(PreviewSelection::Source("source".into()))
        );
        host.selected_source = None;
        assert_eq!(
            host.selection_target(),
            Some(PreviewSelection::Operation("pigment".into()))
        );
    }
    #[test]
    fn operation_preview_is_a_temporary_prefix_and_preserves_authored_outputs() {
        let asset = fixture();
        let source = asset.save().unwrap();
        let selected = operation_preview_asset(&asset, "color-noise").unwrap();
        let mat = material(&selected).unwrap();
        assert_eq!(mat.rows[0].tiles.len(), 1);
        assert_eq!(mat.rows[0].tiles[0].id, "color-noise");
        assert!(
            mat.outputs
                .iter()
                .any(|o| o.channel == "opacity" && o.source.components == "w")
        );
        assert_eq!(asset.save().unwrap(), source);
        assert_eq!(material(&asset).unwrap().rows[0].tiles.len(), 2);
        assert!(operation_preview_asset(&asset, "missing").is_err());
    }
    #[test]
    fn preview_icons_have_visible_transparent_rasterized_paths() {
        let mut ctx = TheContext::new(380, 200, 1.);
        register_preview_icons(&mut ctx, &TheDarkTheme::new());
        for (name, _) in PREVIEW_ICONS {
            let icon = ctx.ui.icon(name).unwrap();
            assert_eq!((icon.dim().width, icon.dim().height), (18, 18));
            assert!(
                icon.pixels().chunks_exact(4).any(|pixel| pixel[3] > 0),
                "{name}"
            );
            assert!(
                icon.pixels()
                    .chunks_exact(4)
                    .filter(|pixel| pixel[3] == 0)
                    .count()
                    > 18 * 18 / 4,
                "{name}"
            );
        }
    }
    #[test]
    fn animation_controls_save_and_reject_invalid_loop_settings() {
        let original = fixture();
        let animated = edited(
            &original,
            MaterialEditorCommand::SetAnimation {
                frames: 16,
                duration: 2.,
            },
        )
        .unwrap();
        let restored = EditableMaterialAsset::reopen(&animated.save().unwrap()).unwrap();
        assert_eq!(material(&restored).unwrap().animation.frames, 16);
        assert_eq!(view(&restored).unwrap().animation.duration, 2.);
        assert!(
            edited(
                &original,
                MaterialEditorCommand::SetAnimation {
                    frames: 0,
                    duration: 2.
                }
            )
            .is_err()
        );
        assert!(
            edited(
                &original,
                MaterialEditorCommand::SetAnimation {
                    frames: 16,
                    duration: f32::NAN
                }
            )
            .is_err()
        );
        assert_eq!(material(&original).unwrap().animation.frames, 1);
    }
    #[test]
    fn preview_tiling_edits_the_material_preserves_period_and_undoes() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut host = ProceduralMaterialEditor::default();
        let mut project = Project::default();
        let mut server = ServerContext::default();
        let tile = rusterix::Tile::from_texture(rusterix::Texture::alloc(16, 16));
        let id = tile.id;
        let asset = edited(
            &starter(id).unwrap(),
            MaterialEditorCommand::SetTiling {
                mode: "raw".into(),
                period: [2., 3.],
                blend: 0.22,
            },
        )
        .unwrap();
        let before = asset.save().unwrap();
        project.tiles.insert(id, tile);
        project.procedural_materials.insert(id, before.clone());
        host.open(id, &mut ui, &mut ctx, &project);
        for (index, expected) in [(1, "repeat"), (2, "seamless"), (0, "raw")] {
            assert!(host.handle_event(
                &TheEvent::IndexChanged(TheId::named("Material Tiling"), index),
                &mut ui,
                &mut ctx,
                &mut project,
                &mut server
            ));
            let asset = EditableMaterialAsset::reopen(&project.procedural_materials[&id]).unwrap();
            let tiling = material(&asset).unwrap().tiling;
            assert_eq!(tiling.mode.name(), expected);
            assert_eq!(tiling.period, [2., 3.]);
            assert_eq!(tiling.blend, 0.22);
            assert_eq!(
                ui.get_widget("Material Tiling")
                    .unwrap()
                    .as_group_button()
                    .unwrap()
                    .index(),
                index as i32
            );
        }
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named("Material Tiling"), 1),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        let after = project.procedural_materials[&id].clone();
        let atom = ProjectUndoAtom::ProceduralMaterialEdit(id, before.clone(), after.clone());
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&id], before);
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_materials[&id], after);
        let reopened: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        let asset = EditableMaterialAsset::reopen(&reopened.procedural_materials[&id]).unwrap();
        assert_eq!(material(&asset).unwrap().tiling.mode.name(), "repeat");
    }
    #[test]
    fn preview_buttons_change_only_view_settings() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut host = ProceduralMaterialEditor::default();
        let mut project = Project::default();
        let mut server = ServerContext::default();
        let before = serde_json::to_string(&project).unwrap();
        for (name, index) in [
            ("Material Preview Target", 1),
            ("Material Preview Channel", 2),
        ] {
            assert!(ui.get_widget(name).unwrap().as_group_button().is_some());
            assert!(host.handle_event(
                &TheEvent::IndexChanged(TheId::named(name), index),
                &mut ui,
                &mut ctx,
                &mut project,
                &mut server
            ));
        }
        assert!(host.preview_tile);
        assert_eq!(host.channel, 2);
        assert_eq!(serde_json::to_string(&project).unwrap(), before);
    }
    #[test]
    fn repeated_preview_fills_the_pane_with_two_tile_heights() {
        let image: Vec<u8> = (0..16)
            .flat_map(|i| [(i % 4 * 30) as u8, (i / 4 * 40) as u8, 10, 255])
            .collect();
        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, 12, 8));
        draw_preview(&mut buffer, Some((4, &image, false)));
        let pixel = |x: usize, y: usize| &buffer.pixels()[(y * 12 + x) * 4..(y * 12 + x) * 4 + 4];
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(pixel(x, y), pixel(x + 4, y));
                assert_eq!(pixel(x, y), pixel(x, y + 4));
            }
        }
        assert!(
            buffer
                .pixels()
                .chunks_exact(4)
                .all(|pixel| pixel[2] == 10 && pixel[3] == 255)
        );
        let mut continuous = TheRGBABuffer::new(TheDim::new(0, 0, 12, 8));
        draw_preview(&mut continuous, Some((4, &image, true)));
        assert_ne!(
            buffer.pixels(),
            continuous.pixels(),
            "continuous output spans the pane instead of repeating the source image"
        );
    }
    pub(super) fn workspace_ui(width: usize, height: usize) -> (TheUI, TheContext) {
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(width, height, 1.);
        register_preview_icons(&mut ctx, &TheDarkTheme::new());
        let mut pages = TheStackLayout::new(TheId::named("Sidebar Page Stack"));
        for _ in 0..SIDEBAR_PAGE {
            pages.add_canvas(TheCanvas::default());
        }
        pages.add_canvas(material_canvas());
        let mut page_canvas = TheCanvas::default();
        page_canvas.set_layout(pages);
        let mut normal_minimap = TheCanvas::default();
        normal_minimap.set_widget(TheRenderView::new(TheId::named("MiniMap")));
        let mut shared = TheSharedVLayout::new(TheId::named("Sidebar Map Shared"));
        shared.limiter_mut().set_max_width(380);
        shared.add_canvas(page_canvas);
        shared.add_canvas(minimap_canvas(normal_minimap));
        shared.set_mode(TheSharedVLayoutMode::Shared);
        shared.set_shared_ratio(0.78);
        let mut sidebar = TheCanvas::default();
        sidebar.set_layout(shared);
        let mut tabs = TheGroupButton::new(TheId::named("Sidebar Tabs"));
        for (label, icon) in [
            ("Project", "project"),
            ("Actions", "graph"),
            ("Nodes", "mapobjects"),
            ("Console", "terminal-nav"),
            ("Debug", "diagnostics-nav"),
            ("Help", "question-mark"),
            ("Procedural materials", "paint-bucket"),
        ] {
            tabs.add_text_status_icon(String::new(), label.into(), icon.into());
        }
        tabs.set_item_width(30);
        let mut toolbar = TheHLayout::new(TheId::empty());
        toolbar.set_background_color(None);
        toolbar.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar.add_widget(Box::new(tabs));
        let mut bar = TheCanvas::default();
        bar.set_widget(TheTraybar::new(TheId::empty()));
        bar.set_layout(toolbar);
        sidebar.set_top(bar);
        sidebar.top_is_expanding = false;
        ui.canvas.set_right(sidebar);
        let mut stack = TheStackLayout::new(TheId::named("Game Output Stack"));
        let mut map = TheCanvas::default();
        map.set_widget(TheRenderView::new(TheId::named("PolyView")));
        stack.add_canvas(map);
        let mut text = TheCanvas::default();
        text.set_widget(TheRenderView::new(TheId::named("TextView")));
        stack.add_canvas(text);
        stack.set_index(1);
        let mut center = TheCanvas::default();
        center.set_layout(stack);
        ui.canvas.set_center(center);
        (ui, ctx)
    }
    fn ui_fixture() -> (TheUI, TheContext) {
        workspace_ui(1024, 768)
    }
    #[test]
    fn library_selector_uses_the_pbr_editor_and_returns_to_the_map_tile() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut project = Project::default();
        shared::ruleset_materials::install(&mut project).unwrap();
        starters::install(&mut project).unwrap();
        let grass = *project
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "Grass")
            .unwrap()
            .0;
        let mut server = ServerContext::default();
        server.curr_tile_id = Some(grass);
        let mut host = ProceduralMaterialEditor::default();
        host.open(grass, &mut ui, &mut ctx, &project);
        let index = shared::ruleset_materials::definitions(&project.config)
            .iter()
            .position(|d| d.id == "tunic")
            .unwrap()
            + 1;
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named("Ruleset Material"), index),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(
            host.tile,
            Some(shared::rulesets::material_asset_id("tunic"))
        );
        assert_eq!(
            server.curr_tile_id,
            Some(grass),
            "Library editing must preserve the map brush"
        );
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named("Ruleset Material"), 0),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(host.tile, Some(grass));
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn minimap_animation_advances_only_with_the_tile_tick() {
        let (mut ui, mut ctx) = ui_fixture();
        ui.get_stack_layout("Sidebar Page Stack")
            .unwrap()
            .set_index(SIDEBAR_PAGE);
        ui.get_stack_layout(PREVIEW_STACK).unwrap().set_index(1);
        ui.draw(&mut vec![0; 1024 * 768 * 4], &mut ctx);
        let maps = |color: [u8; 4]| std::array::from_fn(|_| color.to_vec());
        let mut host = ProceduralMaterialEditor::default();
        host.active = true;
        host.frame = Some(gpu::Frame {
            size: 1,
            images: maps([255, 0, 0, 255]),
            animation_images: vec![maps([255, 0, 0, 255]), maps([0, 255, 0, 255])],
            animation_selections: vec![],
            tile: rusterix::Texture::alloc(1, 1),
            warnings: vec![],
            selection: None,
            continuous: false,
            thumbnails: vec![],
            surface_pages: None,
        });
        for (tick, color) in [
            (0, [255, 0, 0, 255]),
            (1, [0, 255, 0, 255]),
            (2, [255, 0, 0, 255]),
        ] {
            assert!(host.draw_preview(&mut ui, tick));
            assert_eq!(
                &ui.get_render_view(PREVIEW)
                    .unwrap()
                    .render_buffer_mut()
                    .pixels()[..4],
                &color
            );
            assert!(
                !host.draw_preview(&mut ui, tick),
                "Rendering another frame without a server tick must hold the same material frame"
            );
        }
    }
    #[test]
    fn make_proc_selects_material_tab_and_minimap_preview_without_changing_main_view() {
        let (mut ui, mut ctx) = ui_fixture();
        let mut project = Project::default();
        let mut server = ServerContext::default();
        let mut host = ProceduralMaterialEditor::default();
        server.curr_tile_board_cell = Some((None, Vec2::new(3, 4)));
        let before = project.clone();
        assert!(host.handle_event(
            &TheEvent::Custom(TheId::named("Make Procedural Tile"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let id = server.curr_tile_id.unwrap();
        assert!(project.procedural_materials.contains_key(&id));
        assert_eq!(project.tile_board_tiles.get(&id), Some(&Vec2::new(3, 4)));
        assert_eq!(ui.get_stack_layout("Game Output Stack").unwrap().index(), 1);
        assert_eq!(
            ui.get_stack_layout("Sidebar Page Stack").unwrap().index(),
            SIDEBAR_PAGE
        );
        assert_eq!(
            ui.get_widget("Sidebar Tabs")
                .unwrap()
                .as_group_button()
                .unwrap()
                .index(),
            SIDEBAR_PAGE as i32
        );
        assert_eq!(ui.get_stack_layout(PREVIEW_STACK).unwrap().index(), 1);
        let after = project.clone();
        let atom = ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(after));
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert!(!project.tiles.contains_key(&id));
        assert!(!project.procedural_materials.contains_key(&id));
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert!(project.tiles.contains_key(&id));
        assert!(project.procedural_materials.contains_key(&id));
        host.leave(&mut ui, &mut ctx);
        assert_eq!(ui.get_stack_layout("Game Output Stack").unwrap().index(), 1);
        assert_eq!(
            ui.get_stack_layout("Sidebar Page Stack").unwrap().index(),
            0
        );
        assert_eq!(ui.get_stack_layout(PREVIEW_STACK).unwrap().index(), 0);
        assert_eq!(server.curr_tile_id, Some(id));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod gpu_workspace_test {
    use super::*;
    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn opening_baked_hideout_does_not_schedule_gpu_rebakes() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test_projects/Hideout2D-MZ.eldiron"
        ))
        .unwrap();
        let mut project = shared::project_io::decode_project(&bytes).unwrap();
        let original = project.clone();
        let (mut ui, mut ctx) = super::tests::workspace_ui(1200, 850);
        ui.init(&mut ctx);
        let mut host = ProceduralMaterialEditor::default();
        let mut server = ServerContext::default();
        for _ in 0..3 {
            host.update(&mut ui, &mut ctx, &mut project, &mut server);
            assert!(
                host.requested.is_empty(),
                "Saved valid bakes must not be queued on load"
            );
            assert!(
                host.worker.is_none(),
                "A baked map needs no material compiler on load"
            );
        }
        assert_eq!(project.procedural_materials, original.procedural_materials);
        assert_eq!(project.tiles, original.tiles);
        assert_eq!(project.tile_board_tiles, original.tile_board_tiles);
    }
    #[test]
    #[ignore = "requires a GPU device; writes /tmp/eldiron-materials-palette.png"]
    fn palette_starter_initial_cards_and_palette_updates_render_in_the_sidebar() {
        let mut project = Project::default();
        project.palette.colors[37] = Some(TheColor::new(0.8, 0.1, 0.1, 0.5));
        project.palette.colors[34] = Some(TheColor::new(0.1, 0.8, 0.1, 1.));
        let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(64, 64)]);
        let id = tile.id;
        let asset = palette::starter(id, &project).unwrap();
        project.tiles.insert(id, tile);
        project
            .procedural_materials
            .insert(id, asset.save().unwrap());
        let (device, queue) = RUSTERIX
            .read()
            .unwrap()
            .scene_handler
            .vm
            .gpu_context()
            .unwrap();
        let (mut ui, mut ctx) = super::tests::workspace_ui(1200, 850);
        ui.init(&mut ctx);
        crate::editor::register_compact_navigation_icons(&mut ctx);
        let mut host = ProceduralMaterialEditor::default();
        host.worker = Some(gpu::Worker::new(device, queue));
        host.open(id, &mut ui, &mut ctx, &project);
        let mut server = ServerContext::default();
        let mut pixels = vec![0; 1200 * 850 * 4];
        ui.draw(&mut pixels, &mut ctx);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while host.frame.is_none() && std::time::Instant::now() < deadline {
            host.update(&mut ui, &mut ctx, &mut project, &mut server);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let frame = host
            .frame
            .as_ref()
            .expect("Initial material must render without selecting/changing any node");
        for operation in ["color-noise", "pigment", "palette-a", "palette-b"] {
            assert!(
                frame.thumbnails.iter().any(|(id, _)| id == operation),
                "Missing initial thumbnail for {operation}"
            );
        }
        let expected = project.palette.colors[37].as_ref().unwrap().to_u8_array();
        let color = &frame
            .thumbnails
            .iter()
            .find(|(id, _)| id == "palette-a")
            .unwrap()
            .1;
        assert!(color.chunks_exact(4).all(|pixel| {
            pixel
                .iter()
                .zip(expected)
                .all(|(a, b)| (*a as i32 - b as i32).abs() <= 2)
        }));
        if let Some(widget) = ui
            .get_widget(EDITOR)
            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
        {
            widget.press([128., 192.]);
            widget.release([128., 192.]);
        }
        host.preview_dirty = true;
        host.draw_preview(&mut ui, 0);
        ui.draw(&mut pixels, &mut ctx);
        let mut encoder = png::Encoder::new(
            std::fs::File::create("/tmp/eldiron-materials-palette.png").unwrap(),
            1200,
            850,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
        project.palette.colors[37] = Some(TheColor::new(0.1, 0.1, 0.8, 0.5));
        host.update(&mut ui, &mut ctx, &mut project, &mut server);
        let expected_key = pixel_style::bake_key(
            &project.procedural_materials[&id],
            pixel_style::density(&project.config),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while project.procedural_material_cache_keys.get(&id) != Some(&expected_key)
            && std::time::Instant::now() < deadline
        {
            host.update(&mut ui, &mut ctx, &mut project, &mut server);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(
            project.procedural_material_cache_keys.get(&id),
            Some(&expected_key)
        );
        let frame = host.frame.as_ref().unwrap();
        let color = &frame
            .thumbnails
            .iter()
            .find(|(id, _)| id == "palette-a")
            .unwrap()
            .1;
        let expected = project.palette.colors[37].as_ref().unwrap().to_u8_array();
        assert!(color.chunks_exact(4).all(|pixel| {
            pixel
                .iter()
                .zip(expected)
                .all(|(a, b)| (*a as i32 - b as i32).abs() <= 2)
        }));
        assert_eq!(
            palette::binding_index(host.asset.as_ref().unwrap(), "palette-a"),
            Some(37)
        );
    }
    /// A real GPU output in the actual sidebar/viewport stack layout, without
    /// launching a window or reading/modifying any user project.
    #[test]
    #[ignore = "requires a GPU device; writes /tmp/eldiron-procedural-workspace.png"]
    fn render_workspace_snapshot() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = gpu::Worker::new(device, queue);
        let id = Uuid::new_v4();
        let asset = starter(id).unwrap();
        let bundle = asset.save().unwrap();
        worker.send(gpu::Job {
            pixels_per_tile: 0,
            tile: id,
            revision: 1,
            bundle: bundle.clone(),
            continuous: true,
            selection: Some(PreviewSelection::Operation("color-noise".into())),
            preview_aspect: 2.5,
        });
        let frame = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        let selection = frame
            .selection
            .as_ref()
            .expect("selected operation must render");
        assert_eq!(
            selection.0,
            PreviewSelection::Operation("color-noise".into())
        );
        assert_ne!(selection.1, frame.images[0]);
        for pixel in selection.1.chunks_exact(4) {
            assert_eq!(pixel[0], pixel[1]);
            assert_eq!(pixel[1], pixel[2]);
        }
        let (mut ui, mut ctx) = super::tests::workspace_ui(1200, 850);
        ui.init(&mut ctx);
        ui.get_stack_layout("Game Output Stack")
            .unwrap()
            .set_index(0);
        for name in [
            "project",
            "graph",
            "mapobjects",
            "question-mark",
            "paint-bucket",
        ] {
            let data = crate::Embedded::get(&format!("icons/{name}.png")).unwrap();
            ctx.ui.add_icon(name.into(), {
                let mut reader = png::Decoder::new(std::io::Cursor::new(data.data.as_ref()))
                    .read_info()
                    .unwrap();
                let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
                let info = reader.next_frame(&mut bytes).unwrap();
                TheRGBABuffer::from(
                    bytes[..info.buffer_size()].to_vec(),
                    info.width,
                    info.height,
                )
            });
        }
        crate::editor::register_compact_navigation_icons(&mut ctx);
        let mut project = Project::default();
        project.procedural_materials.insert(id, bundle);
        let mut host = ProceduralMaterialEditor::default();
        host.open(id, &mut ui, &mut ctx, &project);
        ProceduralMaterialEditor::set_view(&mut ui, &asset);
        if let Some(widget) = ui
            .get_widget(EDITOR)
            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
        {
            widget.select_operation("pigment");
            widget.set_thumbnail(
                "color-noise".into(),
                TheRGBABuffer::from(frame.selection.as_ref().unwrap().1.clone(), 384, 384)
                    .scaled(64, 64),
            );
            widget.set_thumbnail(
                "pigment".into(),
                TheRGBABuffer::from(frame.images[0].clone(), 384, 384).scaled(64, 64),
            );
            widget.set_thumbnail(
                "grain-noise".into(),
                TheRGBABuffer::from(frame.images[2].clone(), 384, 384).scaled(64, 64),
            );
        }
        let mut pixels = vec![0; 1200 * 850 * 4];
        ui.draw(&mut pixels, &mut ctx);
        let map = ui.get_render_view("PolyView").unwrap();
        let map_dim = *map.dim();
        let map_buffer = map.render_buffer_mut();
        map_buffer.resize(map_dim.width, map_dim.height);
        for y in 0..map_dim.height {
            for x in 0..map_dim.width {
                map_buffer.set_pixel(
                    x,
                    y,
                    &if x % 32 == 0 || y % 32 == 0 {
                        [47, 50, 54, 255]
                    } else {
                        [22, 24, 27, 255]
                    },
                );
            }
        }
        let preview = ui.get_render_view(PREVIEW).unwrap();
        let dim = *preview.dim();
        let buffer = preview.render_buffer_mut();
        buffer.resize(dim.width, dim.height);
        draw_preview(
            buffer,
            Some((frame.size, &frame.images[0], frame.continuous)),
        );
        ui.draw(&mut pixels, &mut ctx);
        let output = std::fs::File::create("/tmp/eldiron-procedural-workspace.png").unwrap();
        let mut encoder = png::Encoder::new(output, 1200, 850);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
        host.frame = Some(frame);
        host.selected_operation = Some("color-noise".into());
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named("Material Preview Target"), 1),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut ServerContext::default(),
        );
        ui.get_widget("Material Preview Target")
            .unwrap()
            .as_group_button()
            .unwrap()
            .set_index(1);
        host.draw_preview(&mut ui, 0);
        ui.draw(&mut pixels, &mut ctx);
        let file = std::fs::File::create("/tmp/eldiron-procedural-selected-tile.png").unwrap();
        let mut encoder = png::Encoder::new(file, 1200, 850);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
        let source = source_catalog()
            .into_iter()
            .find(|source| source.metadata.name == "Disc")
            .unwrap();
        worker.send(gpu::Job {
            pixels_per_tile: 0,
            tile: id,
            revision: 2,
            bundle: project.procedural_materials[&id].clone(),
            continuous: false,
            selection: Some(PreviewSelection::Source(source.id.clone())),
            preview_aspect: 2.5,
        });
        let source_frame = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        assert_eq!(
            source_frame.selection.as_ref().unwrap().0,
            PreviewSelection::Source(source.id.clone())
        );
        assert_ne!(
            source_frame.selection.as_ref().unwrap().1,
            source_frame.images[0]
        );
        host.selected_source = Some(source.id);
        host.frame = Some(source_frame);
        host.preview_dirty = true;
        host.draw_preview(&mut ui, 0);
        ui.draw(&mut pixels, &mut ctx);
        let file = std::fs::File::create("/tmp/eldiron-procedural-source-preview.png").unwrap();
        let mut encoder = png::Encoder::new(file, 1200, 850);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
        assert!(dim.width <= 380 && dim.height < 250 && dim.height > 60);
        assert!(dim.x >= map_dim.width && dim.y > 400);
        assert_eq!(ui.get_stack_layout("Game Output Stack").unwrap().index(), 0);
        assert!(map_dim.width > 700);
        assert!(ui.get_widget(EDITOR).unwrap().dim().width <= 380);
    }
}

/// Cached offscreen bakes reuse the exact editor GPU worker. The snapshot is
/// owned by Scepter; finished bakes never race the live project's authoring.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct ScepterBakeCache {
    worker: gpu::Worker,
    tiles: std::collections::HashMap<Uuid, (u64, rusterix::Tile)>,
}
#[cfg(not(target_arch = "wasm32"))]
impl ScepterBakeCache {
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            worker: gpu::Worker::new(device, queue),
            tiles: Default::default(),
        }
    }
    pub(crate) fn prepare(&mut self, project: &mut Project) -> Result<(), String> {
        let mut errors = palette::refresh(project);
        errors.extend(composition::prepare_project(project));
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        self.tiles
            .retain(|id, _| project.procedural_materials.contains_key(id));
        let density = pixel_style::density(&project.config);
        let ids = project
            .procedural_materials
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for id in ids {
            if shared::ruleset_materials::is_library_tile(&project.config, &id) {
                continue;
            }
            let bundle = &project.procedural_materials[&id];
            let key = pixel_style::bake_key(bundle, density);
            if project.procedural_material_cache_keys.get(&id) == Some(&key) {
                continue;
            }
            let tile = project
                .tiles
                .get_mut(&id)
                .ok_or_else(|| format!("Material {id} has no tile"))?;
            if let Some((_, cached)) = self.tiles.get(&id).filter(|(k, _)| *k == key) {
                tile.textures = cached.textures.clone();
                tile.uv_pages = cached.uv_pages.clone();
                tile.baked_material_data = cached.baked_material_data.clone();
            } else {
                self.worker.send(gpu::Job {
                    pixels_per_tile: density,
                    tile: id,
                    revision: 0,
                    bundle: bundle.clone(),
                    continuous: false,
                    selection: None,
                    preview_aspect: 1.,
                });
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
                let result = loop {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    let result = self
                        .worker
                        .results
                        .recv_timeout(remaining)
                        .map_err(|e| format!("Material {id} bake: {e}"))?;
                    if result.tile == id
                        && result.bundle == *bundle
                        && result.pixels_per_tile == density
                    {
                        break result;
                    }
                };
                let frame = result.frame?;
                if let Some(pages) = frame.surface_pages {
                    tile.textures = pages.textures.clone();
                    tile.uv_pages = Some(pages.layout.clone());
                } else {
                    tile.textures = vec![frame.tile];
                    tile.uv_pages = None;
                }
                tile.baked_material_data =
                    tile.textures.iter().map(|t| t.material_bytes()).collect();
                let bytes = |tile: &rusterix::Tile| {
                    tile.textures
                        .iter()
                        .map(|t| t.data.len() + t.data_ext.as_ref().map_or(0, Vec::len))
                        .sum::<usize>()
                        + tile.baked_material_data.iter().map(Vec::len).sum::<usize>()
                };
                if self.tiles.values().map(|(_, t)| bytes(t)).sum::<usize>() + bytes(tile)
                    > 64 * 1024 * 1024
                {
                    self.tiles.clear();
                }
                if bytes(tile) <= 64 * 1024 * 1024 {
                    self.tiles.insert(id, (key, tile.clone()));
                }
            }
            project.procedural_material_cache_keys.insert(id, key);
        }
        Ok(())
    }
}
