//! Item-browser entry points for shared MZ icons and geometry.
use crate::{
    editor::UNDOMANAGER,
    prelude::*,
    undo::{
        project_atoms::ProjectUndoAtom, project_helper::resolved_item_default_icon_frames_for_state,
    },
};
use denrim_materials::{
    geometry::{Document, Operation, Row, Tile},
    image_source::ImageSource,
    integration::EditableMaterialAsset,
};

fn attribute(data: &str, key: &str) -> Option<String> {
    let root = data.parse::<toml::Table>().ok()?;
    root.get("attributes")
        .and_then(toml::Value::as_table)
        .and_then(|attrs| attrs.get(key))
        .or_else(|| root.get(key))?
        .as_str()
        .map(str::to_owned)
}
fn set_attribute(data: &mut String, key: &str, value: String) -> Result<(), String> {
    let mut root = data.parse::<toml::Table>().map_err(|e| e.to_string())?;
    root.remove(key);
    let target = root
        .entry("attributes")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or("Invalid item attributes")?;
    target.insert(key.into(), toml::Value::String(value));
    *data = toml::to_string_pretty(&root).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn handle_event(event: &TheEvent, project: &mut Project, ctx: &mut TheContext) -> bool {
    let TheEvent::StateChanged(id, state) = event else {
        return false;
    };
    if !matches!(state, TheWidgetState::Selected | TheWidgetState::Clicked) {
        return false;
    }
    if let Some(source) = id.name.strip_prefix("Item Worn Material MZ: ") {
        let result = (|| -> Result<Uuid, String> {
            let item = project
                .items
                .get(&id.references)
                .ok_or("Item was removed")?;
            if !shared::ruleset_materials::worn_material_sources(project, &item.data)
                .contains(source)
            {
                return Err("This material is no longer worn by the item".into());
            }
            let material = if let Some(name) = source.strip_prefix("ruleset:material/") {
                shared::rulesets::material_asset_id(name)
            } else {
                Uuid::parse_str(source).map_err(|_| "Invalid worn material source")?
            };
            if !project.procedural_materials.contains_key(&material) {
                return Err("Worn material is missing".into());
            }
            Ok(material)
        })();
        match result {
            Ok(material) => ctx.ui.send(TheEvent::Custom(
                TheId::named("Open Procedural Tile"),
                TheValue::Id(material),
            )),
            Err(e) => ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), e)),
        }
        return true;
    }
    if id.name == "Item Worn Geometry MZ" {
        if let Some(target) = project
            .items
            .get(&id.references)
            .and_then(|item| shared::ruleset_materials::worn_geometry_id(&item.data))
        {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Open Worn Geometry"),
                TheValue::Id(target),
            ));
        }
        return true;
    }
    let on = id.name == "Item Icon MZ On";
    let icon = on || id.name == "Item Icon MZ Off";
    if !icon
        && !matches!(
            id.name.as_str(),
            "Item Geometry MZ" | "Item Geometry MZ Off"
        )
    {
        return false;
    }
    let mut before = None;
    let result = (|| -> Result<(), String> {
        let item = project
            .items
            .get(&id.references)
            .ok_or("Item was removed")?
            .clone();
        let mut created = false;
        let target = if icon {
            if let Some(id) =
                shared::ruleset_materials::icon_id_for_state(&item.data, on).filter(|id| {
                    item.icon_frames_for_state(on).is_empty()
                        || id.as_u128() >> 64 == 0x4d5a49434f4e4544
                })
            {
                id
            } else {
                let asset_id = shared::ruleset_materials::new_icon_id();
                before = Some(project.clone());
                let frames = if !item.icon_frames_for_state(on).is_empty() {
                    item.icon_frames_for_state(on).clone()
                } else {
                    resolved_item_default_icon_frames_for_state(&item, project, on)
                };
                let frames = if frames.is_empty() {
                    vec![rusterix::Texture::alloc(32, 32)]
                } else {
                    frames
                };
                let size = (frames[0].width, frames[0].height);
                let frames = frames
                    .into_iter()
                    .map(|t| t.resized(size.0, size.1).data)
                    .take(32)
                    .collect::<Vec<_>>();
                let image =
                    ImageSource::rgba8(size.0, size.1, &frames).map_err(|e| e.to_string())?;
                let mut asset=EditableMaterialAsset::parse(&asset_id.to_string(),"project 1\nmaterial \"icon\" called \"Item icon\"\n  mode pbr\n  default roughness to 0.8\n  default emission strength to 0\n  row \"image\" called \"Image or tile source\"\n    use \"Image / Tile Source\" id \"image-source\"\n  output base color from row \"image\" xyz\n  output opacity from row \"image\" w\n","icon").map_err(|e|e.to_string())?;
                let material = &mut asset.root.project.materials[0];
                material.label = format!("{} icon", item.name);
                material.animation.frames = frames.len() as u32;
                material.rows[0].tiles[0].image = Some(image);
                crate::procedural_material::palette::ensure_color_bindings(
                    &mut asset,
                    &project.palette,
                )?;
                project
                    .procedural_materials
                    .insert(asset_id, asset.save().map_err(|e| e.to_string())?);
                let mut swatch = rusterix::Tile::from_texture(rusterix::Texture::alloc(32, 32));
                swatch.id = asset_id;
                swatch.alias = format!("{} icon", item.name);
                project.tiles.insert(asset_id, swatch);
                set_attribute(
                    &mut project.items.get_mut(&item.id).unwrap().data,
                    if on {
                        "appearance_icon"
                    } else {
                        "appearance_icon_off"
                    },
                    asset_id.to_string(),
                )?;
                let item = project.items.get_mut(&item.id).unwrap();
                if on {
                    item.icon_frames.clear();
                    item.icon_frame_paths.clear();
                } else {
                    item.icon_off_frames.clear();
                    item.icon_off_frame_paths.clear();
                }
                created = true;
                asset_id
            }
        } else {
            let key = if id.name == "Item Geometry MZ Off" {
                "appearance_geometry_off"
            } else {
                "appearance_geometry"
            };
            if let Some(source) = attribute(&item.data, key) {
                shared::appearance_assets::asset_id(&source).or_else(|_| {
                    shared::appearance_assets::asset_id(&format!("ruleset:geometry/{source}"))
                })?
            } else {
                let asset_id = Uuid::new_v4();
                before = Some(project.clone());
                let doc = Document {
                    version: 1,
                    id: asset_id.to_string(),
                    label: format!("{} model", item.name),
                    rows: vec![
                        Row {
                            id: "model".into(),
                            label: "Item body".into(),
                            tiles: vec![Tile {
                                id: "body".into(),
                                op: Operation::Box {
                                    size: [0.3, 0.3, 0.3],
                                },
                            }],
                        },
                        Row {
                            id: "hand-grip".into(),
                            label: "Hand grip".into(),
                            tiles: vec![Tile {
                                id: "grip".into(),
                                op: Operation::Grip {
                                    translation: [0.; 3],
                                    rotation: [0.; 3],
                                },
                            }],
                        },
                    ],
                    output: "model".into(),
                    grip: Some("hand-grip".into()),
                };
                let mut doc = if key == "appearance_geometry_off" {
                    attribute(&item.data, "appearance_geometry")
                        .and_then(|source| {
                            shared::appearance_assets::asset_id(&source)
                                .or_else(|_| {
                                    shared::appearance_assets::asset_id(&format!(
                                        "ruleset:geometry/{source}"
                                    ))
                                })
                                .ok()
                        })
                        .and_then(|id| project.procedural_appearances.get(&id))
                        .and_then(|text| serde_json::from_str::<Document>(text).ok())
                        .unwrap_or(doc)
                } else {
                    doc
                };
                doc.id = asset_id.to_string();
                doc.label = format!(
                    "{} {} model",
                    item.name,
                    if key.ends_with("_off") { "Off" } else { "On" }
                );
                project
                    .procedural_appearances
                    .insert(asset_id, doc.save().map_err(|e| e.to_string())?);
                set_attribute(
                    &mut project.items.get_mut(&item.id).unwrap().data,
                    key,
                    asset_id.to_string(),
                )?;
                created = true;
                asset_id
            }
        };
        if created {
            let key = if icon {
                if on {
                    "appearance_icon"
                } else {
                    "appearance_icon_off"
                }
            } else if id.name == "Item Geometry MZ Off" {
                "appearance_geometry_off"
            } else {
                "appearance_geometry"
            };
            shared::entity_graph::set_owner_attribute(
                project,
                &shared::entity_graph::item_key(item.id),
                key,
                serde_json::Value::String(target.to_string()),
            )?;
            UNDOMANAGER.write().unwrap().add_undo(
                ProjectUndoAtom::ProjectEdit(
                    "Create item appearance".into(),
                    Box::new(before.take().unwrap()),
                    Box::new(project.clone()),
                ),
                ctx,
            );
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named(if icon {
                "Open Procedural Tile"
            } else {
                "Open Worn Geometry"
            }),
            TheValue::Id(target),
        ));
        Ok(())
    })();
    if let Err(e) = result {
        if let Some(before) = before {
            *project = before;
        }
        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), e));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_geometry_actions_reopen_all_authored_models_and_torch_states() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let geometry = project.procedural_appearances.clone();
        let mut ctx = TheContext::new(800, 600, 1.);
        let items = project.items.values().cloned().collect::<Vec<_>>();
        assert_eq!(items.len(), 41);
        for item in items {
            let name = attribute(&item.data, "appearance_geometry").unwrap();
            let id = shared::rulesets::geometry_asset_id(&name);
            assert!(project.procedural_appearances.contains_key(&id));
            let event = TheEvent::StateChanged(
                TheId::named_with_reference("Item Geometry MZ", item.id),
                TheWidgetState::Clicked,
            );
            assert!(handle_event(&event, &mut project, &mut ctx));
            assert_eq!(
                attribute(&project.items[&item.id].data, "appearance_geometry").as_deref(),
                Some(name.as_str())
            );
            if attribute(&item.data, "appearance_geometry_off").is_some() {
                let event = TheEvent::StateChanged(
                    TheId::named_with_reference("Item Geometry MZ Off", item.id),
                    TheWidgetState::Clicked,
                );
                assert!(handle_event(&event, &mut project, &mut ctx));
            }
        }
        assert_eq!(project.procedural_appearances, geometry);
    }
    #[test]
    fn worn_material_actions_open_existing_mz_assets_without_creating_geometry() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let item = project
            .items
            .values()
            .find(|i| i.data.contains("ruleset_id = \"linen_shirt\""))
            .unwrap()
            .clone();
        let before = project.procedural_materials.len();
        let geometry = project.procedural_appearances.clone();
        let mut ctx = TheContext::new(800, 600, 1.);
        let sources = shared::ruleset_materials::worn_material_sources(&project, &item.data);
        assert_eq!(sources.len(), 2);
        for source in sources {
            let event = TheEvent::StateChanged(
                TheId::named_with_reference(&format!("Item Worn Material MZ: {source}"), item.id),
                TheWidgetState::Clicked,
            );
            assert!(handle_event(&event, &mut project, &mut ctx));
        }
        assert_eq!(project.procedural_materials.len(), before);
        assert_eq!(project.procedural_appearances, geometry);
        let event = TheEvent::StateChanged(
            TheId::named_with_reference("Item Worn Geometry MZ", item.id),
            TheWidgetState::Clicked,
        );
        assert!(handle_event(&event, &mut project, &mut ctx));
        assert_eq!(project.procedural_appearances, geometry);
    }
    #[test]
    fn bitmap_icons_port_to_editable_animated_mz_and_reopen_without_duplication() {
        let mut p = Project::default();
        let mut item = Item::new();
        item.name = "Test item".into();
        let id = item.id;
        for index in [37, 41] {
            item.icon_frames.push(rusterix::Texture::new(
                p.palette.colors[index]
                    .as_ref()
                    .unwrap()
                    .to_u8_array()
                    .repeat(32 * 32),
                32,
                32,
            ));
        }
        item.icon_off_frames.push(
            rusterix::Texture::from_color(p.palette.colors[31].as_ref().unwrap().to_u8_array())
                .resized(32, 32),
        );
        p.items.insert(id, item);
        let mut ctx = TheContext::new(200, 200, 1.);
        let event = TheEvent::StateChanged(
            TheId::named_with_reference("Item Icon MZ On", id),
            TheWidgetState::Clicked,
        );
        assert!(handle_event(&event, &mut p, &mut ctx));
        let icon = shared::ruleset_materials::icon_id(&p.items[&id].data).unwrap();
        assert!(shared::ruleset_materials::is_library_tile(&p.config, &icon));
        let frames = shared::ruleset_materials::frames_for_item(&p, &p.items[&id], true)
            .unwrap()
            .unwrap();
        assert_eq!(frames.len(), 2);
        assert_ne!(frames[0].data, frames[1].data);
        let count = p.procedural_materials.len();
        assert!(handle_event(&event, &mut p, &mut ctx));
        assert_eq!(p.procedural_materials.len(), count);
        let on_source = p.procedural_materials[&icon].clone();
        let off = TheEvent::StateChanged(
            TheId::named_with_reference("Item Icon MZ Off", id),
            TheWidgetState::Clicked,
        );
        assert!(handle_event(&off, &mut p, &mut ctx));
        let off_icon =
            shared::ruleset_materials::icon_id_for_state(&p.items[&id].data, false).unwrap();
        assert_ne!(icon, off_icon);
        assert_eq!(p.procedural_materials[&icon], on_source);
        let states = p.procedural_materials.len();
        assert!(handle_event(&off, &mut p, &mut ctx));
        assert_eq!(p.procedural_materials.len(), states);
        shared::entity_graph::synchronize(&mut p).unwrap();
        assert_eq!(
            shared::ruleset_materials::icon_id_for_state(&p.items[&id].data, true),
            Some(icon)
        );
        assert_eq!(
            shared::ruleset_materials::icon_id_for_state(&p.items[&id].data, false),
            Some(off_icon)
        );
        let geometry = TheEvent::StateChanged(
            TheId::named_with_reference("Item Geometry MZ", id),
            TheWidgetState::Clicked,
        );
        assert!(handle_event(&geometry, &mut p, &mut ctx));
        let model = Uuid::parse_str(&attribute(&p.items[&id].data, "appearance_geometry").unwrap())
            .unwrap();
        let mut runtime = rusterix::Item::default();
        rusterix::server::data::apply_item_data(&mut runtime, &p.items[&id].data);
        assert_eq!(
            runtime.attributes.get_str("appearance_geometry"),
            Some(model.to_string().as_str())
        );
        assert_eq!(
            runtime.attributes.get_str("appearance_icon"),
            Some(icon.to_string().as_str())
        );
        assert!(
            Document::load(&p.procedural_appearances[&model])
                .unwrap()
                .evaluate()
                .unwrap()
                .grip
                .is_some()
        );
        let reopened =
            shared::project_io::decode_project(&shared::project_io::encode_project(&p).unwrap())
                .unwrap();
        assert_eq!(
            reopened.procedural_materials[&icon],
            p.procedural_materials[&icon]
        );
        assert_eq!(
            reopened.procedural_appearances[&model],
            p.procedural_appearances[&model]
        );
    }
}
