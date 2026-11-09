//! Scepter uses the same material documents, composition builder and surface
//! assignment as the interactive editor. Only this module commits remote edits.
use super::*;
use crate::procedural_material::{composition, image_source, map_composition, palette};
use denrim_materials::integration::EditableMaterialAsset;
use eldiron_scepter::{EditOperation, ScepterCommand as Command, ScepterRequest, Transaction};
use serde_json::{Value as Json, json};
use shared::material_composition::{CompositionSurface, MapMaterialBinding, MaterialComposition};
use std::sync::Arc;

impl Editor {
    pub(super) fn scepter_session(&self) -> String {
        self.sessions[self.active_session].id.to_string()
    }
    pub(super) fn scepter_revision(&self) -> String {
        let mut h = DefaultHasher::new();
        UNDOMANAGER.read().unwrap().revision().hash(&mut h);
        self.project.procedural_materials.revision().hash(&mut h);
        self.project.procedural_appearances.revision().hash(&mut h);
        self.project.config.hash(&mut h);
        self.project.world_source.hash(&mut h);
        self.project.world_source_debug.hash(&mut h);
        // Bakes, animation time, selection and camera movement are derived/view
        // state. They must not invalidate an assistant's authoring context.
        serde_json::to_string(&self.project.palette.colors)
            .unwrap()
            .hash(&mut h);
        serde_json::to_string(&self.project.node_graphs)
            .unwrap()
            .hash(&mut h);
        serde_json::to_string(&self.project.material_compositions)
            .unwrap()
            .hash(&mut h);
        serde_json::to_string(&self.project.map_material_bindings)
            .unwrap()
            .hash(&mut h);
        serde_json::to_string(&self.project.palette_materials)
            .unwrap()
            .hash(&mut h);
        for tile in self.project.tiles.values() {
            tile.id.hash(&mut h);
            tile.alias.hash(&mut h);
            tile.blocking.hash(&mut h);
            tile.scale.to_bits().hash(&mut h);
            serde_json::to_string(&(
                &tile.role,
                &tile.procedural,
                &tile.material,
                &tile.geometry,
                &tile.attachments,
                &tile.light_effects,
                &tile.particle_effects,
                &tile.particle_emitter,
                &tile.light_emitter,
                &tile.module,
                &tile.gameplay_tags,
            ))
            .unwrap()
            .hash(&mut h);
            if !self.project.procedural_materials.contains_key(&tile.id) {
                // Bitmap docks have their own undo stacks. Include authored
                // pixels while excluding MZ's asynchronously produced caches.
                for texture in &tile.textures {
                    texture.width.hash(&mut h);
                    texture.height.hash(&mut h);
                    texture.data.hash(&mut h);
                    texture.data_ext.hash(&mut h);
                }
            }
        }
        for r in &self.project.regions {
            r.id.hash(&mut h);
            r.map.changed.hash(&mut h);
            r.source.hash(&mut h);
            r.config.hash(&mut h);
        }
        for c in self.project.characters.values() {
            c.id.hash(&mut h);
            c.data.hash(&mut h);
            c.source.hash(&mut h);
        }
        for i in self.project.items.values() {
            i.id.hash(&mut h);
            i.data.hash(&mut h);
            i.source.hash(&mut h);
        }
        format!("{:016x}", h.finish())
    }
    pub(super) fn scepter_execute(
        &mut self,
        request: ScepterRequest,
        job: Arc<crate::scepter::jobs::Job>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let session = self.scepter_session();
        let revision = self.scepter_revision();
        if (request.command.is_mutation()
            && (request.session.as_deref() != Some(&session)
                || request.revision.as_deref() != Some(&revision)))
            || request.session.as_ref().is_some_and(|v| v != &session)
            || request.revision.as_ref().is_some_and(|v| v != &revision)
        {
            job.finish(json!({"ok":false,"http_status":409,"error":"Project session or revision changed; inspect before retrying","session":session,"revision":revision}));
            return;
        }
        if let Command::RegionRenderPreview(command) = request.command {
            let command = match self.scepter_preview_request(command, ui) {
                Ok(c) => c,
                Err(e) => {
                    job.finish(failure(e));
                    return;
                }
            };
            let index = match self.scepter_resolve_region_index(&command.region) {
                Ok(i) => i,
                Err(e) => {
                    job.finish(failure(e));
                    return;
                }
            };
            // Owned snapshot and assets: no live selection/camera/server changes.
            let project = self.project.clone();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    super::scepter_preview::render(project, index, &command)
                }))
                .unwrap_or_else(|_| Err("Preview worker failed".into()))
                .map(|v| json!({"ok":true,"preview":v,"session":session,"revision":revision}))
                .unwrap_or_else(failure);
                job.finish(result);
            });
            return;
        }
        let result: Result<Json, String> = (|| {
            Ok(match request.command {
                Command::ScepterHelp | Command::ScepterListCommands => {
                    json!({"catalog":eldiron_scepter::ScepterLorebook::built_in()})
                }
                Command::ScepterDescribeCommand { name } => {
                    json!({"command":eldiron_scepter::ScepterLorebook::built_in().describe_command(&name).ok_or("Unknown command")?})
                }
                Command::ProjectDescribe => json!({"project":self.scepter_project_snapshot()}),
                Command::RegionList => {
                    json!({"regions":self.scepter_project_snapshot()["regions"]})
                }
                Command::RegionSnapshot(c) => {
                    json!({"region":self.scepter_region_snapshot(&region_request(c.region,c.include_tiles,true))})
                }
                Command::RegionSummary(c) => {
                    json!({"summary":self.scepter_region_summary(&region_request(c.region,false,c.include_ascii))})
                }
                Command::TileList(c) => {
                    let mut value = self.scepter_tiles_snapshot();
                    if let Some(tiles) = value["tiles"].as_array_mut() {
                        tiles.retain(|t| {
                            c.role.as_ref().is_none_or(|v| {
                                t["role"]
                                    .as_str()
                                    .is_some_and(|s| s.eq_ignore_ascii_case(v))
                            }) && c.kind.as_ref().is_none_or(|v| {
                                t["procedural"]["kind"]
                                    .as_str()
                                    .is_some_and(|s| s.eq_ignore_ascii_case(v))
                            }) && c.style.as_ref().is_none_or(|v| {
                                t["procedural"]["style"]
                                    .as_str()
                                    .is_some_and(|s| s.eq_ignore_ascii_case(v))
                            })
                        });
                    }
                    value["counts"]["tiles"] = json!(value["tiles"].as_array().map_or(0, Vec::len));
                    json!({"tiles":value})
                }
                Command::MaterialList => json!({
                    "palette":self.project.palette.colors.iter().enumerate().filter_map(|(i,c)|c.as_ref().map(|c|json!({"index":i,"rgba":c.to_u8_array()}))).collect::<Vec<_>>(),
                    "materials":self.project.procedural_materials.keys().map(|id|json!({"id":id,"tile":self.scepter_tile_summary(id),"composition":self.project.material_compositions.contains_key(id),"binding":self.project.map_material_bindings.get(id)})).collect::<Vec<_>>(),
                    "sources":denrim_materials::integration::source_catalog().into_iter().map(|s|json!({"id":s.id,"name":s.metadata.name,"source":s.source,"accepts_input":s.accepts_input,"requires_input":s.requires_input})).collect::<Vec<_>>(),
                }),
                Command::MaterialGet(c) => {
                    let id = parse_id(&c.id)?;
                    json!({"id":id,"bundle":self.project.procedural_materials.get(&id).ok_or("Material not found")?,"composition":self.project.material_compositions.get(&id),"binding":self.project.map_material_bindings.get(&id)})
                }
                Command::CompositionGet(c) => {
                    let id = parse_id(&c.id)?;
                    json!({"id":id,"composition":self.project.material_compositions.get(&id).ok_or("Composition not found")?,"binding":self.project.map_material_bindings.get(&id)})
                }
                Command::MaterialValidate(c) => {
                    let asset = validate_material(&self.project, &c.bundle)?;
                    json!({"valid":true,"bundle":asset.save().map_err(|e|e.to_string())?})
                }
                Command::SurfaceList(c) => {
                    let index = if let Some(region) = c.region {
                        self.scepter_resolve_region_index(&region)?
                    } else {
                        self.project
                            .regions
                            .iter()
                            .position(|r| r.id == self.server_ctx.curr_region)
                            .ok_or("Select a region")?
                    };
                    let map = &self.project.regions[index].map;
                    json!({"map":map.id,"sectors":map.sectors,"linedefs":map.linedefs,"geometry_objects":map.geometry_objects,"lights":map.lights,
                    "bindings":self.project.map_material_bindings.iter().filter(|(_,b)|b.map==map.id).map(|(id,b)|json!({"material":id,"surface":b.surface})).collect::<Vec<_>>()})
                }
                Command::Transaction(c) => self.scepter_transaction(job.clone(), c, ui, ctx)?,
                Command::EditorContext => self.scepter_context(ui),
                Command::EditorSelect(c) => self.scepter_select(c, ui, ctx)?,
                Command::EditorFocus(c) => self.scepter_focus(c, ui, ctx)?,
                Command::ProjectChanges(c) => self.scepter_changes(c, ui),
                Command::EntityTemplates => self.scepter_entity_templates(),
                Command::RegionEntities(c) => {
                    let i = if let Some(r) = c.region {
                        region_index(&self.project, &r)?
                    } else {
                        self.project
                            .regions
                            .iter()
                            .position(|r| r.id == self.server_ctx.curr_region)
                            .ok_or("Select a region")?
                    };
                    json!({"region":self.project.regions[i].id,"characters":self.project.regions[i].characters.values().collect::<Vec<_>>(),"items":self.project.regions[i].items.values().collect::<Vec<_>>()})
                }
                Command::RegionCreateSector(c) => self.scepter_transaction(
                    job.clone(),
                    Transaction {
                        operations: vec![EditOperation::SectorCreate(
                            eldiron_scepter::SectorCreate {
                                region: c.region,
                                name: c.name,
                                polygon: c
                                    .polygon
                                    .into_iter()
                                    .map(|[x, y]| [x as f32, y as f32])
                                    .collect(),
                                floor: None,
                                ceiling: None,
                                height: None,
                                layer: c
                                    .layer
                                    .map(|s| s.parse::<u8>().map_err(|_| "Invalid layer"))
                                    .transpose()?,
                            },
                        )],
                        dry_run: false,
                    },
                    ui,
                    ctx,
                )?,
                Command::RegionPlaceItem(c) => self.scepter_transaction(
                    job.clone(),
                    Transaction {
                        operations: vec![EditOperation::EntityPlace(
                            eldiron_scepter::EntityPlace {
                                region: c.region,
                                id: Uuid::new_v4().to_string(),
                                kind: "item".into(),
                                template: c.template,
                                position: [c.at[0] as f32, 0., c.at[1] as f32],
                                name: c.name,
                                data: None,
                                active: None,
                            },
                        )],
                        dry_run: false,
                    },
                    ui,
                    ctx,
                )?,
                Command::RegionPlaceCharacter(c) => self.scepter_transaction(
                    job.clone(),
                    Transaction {
                        operations: vec![EditOperation::EntityPlace(
                            eldiron_scepter::EntityPlace {
                                region: c.region,
                                id: Uuid::new_v4().to_string(),
                                kind: "character".into(),
                                template: c.template,
                                position: [c.at[0] as f32, 0., c.at[1] as f32],
                                name: c.name,
                                data: None,
                                active: None,
                            },
                        )],
                        dry_run: false,
                    },
                    ui,
                    ctx,
                )?,

                Command::ProjectUndo | Command::ProjectRedo => {
                    let undo = matches!(request.command, Command::ProjectUndo);
                    let mut manager = UNDOMANAGER.write().unwrap();
                    if undo {
                        if !manager.has_undo() {
                            return Err("Nothing to undo".into());
                        }
                        manager.undo(&mut self.server_ctx, &mut self.project, ui, ctx);
                    } else {
                        if !manager.has_redo() {
                            return Err("Nothing to redo".into());
                        }
                        manager.redo(&mut self.server_ctx, &mut self.project, ui, ctx);
                    }
                    drop(manager);
                    material_refresh(ctx);
                    json!({"applied":true})
                }
                Command::RegionPaintRect(c) => self.scepter_transaction(
                    job.clone(),
                    Transaction {
                        operations: vec![EditOperation::PaintRect(c)],
                        dry_run: false,
                    },
                    ui,
                    ctx,
                )?,
                Command::RegionPaintCells(c) => self.scepter_transaction(
                    job.clone(),
                    Transaction {
                        operations: vec![EditOperation::PaintCells(c)],
                        dry_run: false,
                    },
                    ui,
                    ctx,
                )?,
                Command::ScriptGet(c) => self.scepter_get_script(&c),
                Command::ScriptValidate(c) => {
                    self.scepter_validate_eldrin_source(&c.target, &c.source)
                }
                Command::ScriptPatch(c) => self.scepter_apply_script_patch(c, ctx),
                Command::AttributesGet(c) => self.scepter_get_attributes(&c),
                Command::AttributesPatch(c) => self.scepter_apply_attributes_patch(c, ctx),
                Command::ActionList(c) => self.scepter_action_catalog(&c, ctx),
                Command::ActionRun(c) => self.scepter_run_action(c, ui, ctx),
                Command::ActionRunScript(c) => self.scepter_run_action_script(c, ui, ctx),
                Command::ToolList(c) => self.scepter_tool_catalog(&c),
                Command::ToolSelect(c) => self.scepter_select_tool(c, ui, ctx),
                _ => return Err("Command is not executable".into()),
            })
        })();
        let mut result = result.unwrap_or_else(failure);
        if result["_deferred"] == true {
            return;
        }
        for key in ["region", "summary"] {
            if result.get(key).and_then(|v| v.get("error")).is_some() {
                result = result[key].take();
                break;
            }
        }
        // Existing handlers already return their own error; preserve it at the
        // top level instead of wrapping failure in an outer success envelope.
        if result.get("error").is_some() {
            result["ok"] = json!(false);
        } else if result.get("ok").is_none() {
            result["ok"] = json!(true);
        }
        result["session"] = json!(self.scepter_session());
        result["revision"] = json!(self.scepter_revision());
        self.scepter_observe(true, ui);
        job.finish(result);
    }
    fn scepter_transaction(
        &mut self,
        job: Arc<crate::scepter::jobs::Job>,
        transaction: Transaction,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<Json, String> {
        if let Some(sender) = self.scepter_sender.clone() {
            let Some(permit) = super::scepter_staging::Permit::acquire() else {
                return Ok(
                    json!({"ok":false,"http_status":409,"error":"Another transaction is preparing; poll its job before editing again"}),
                );
            };
            let project = self.project.clone();
            let session = self.scepter_session();
            let revision = self.scepter_revision();
            let selection = self.scepter_view(ui)["selection"].clone();
            let dry_run = transaction.dry_run;
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    stage(&project, &transaction)
                }))
                .unwrap_or_else(|_| Err("Transaction preparation worker failed".into()))
                .map(|(p, r)| (Box::new(p), r));
                let event = crate::scepter::ScepterEvent::PreparedEdit(
                    super::scepter_staging::PreparedEdit {
                        session,
                        revision,
                        selection,
                        dry_run,
                        result,
                        job: job.clone(),
                        permit,
                    },
                );
                if sender.send(event).is_err() {
                    job.finish(json!({"ok":false,"http_status":503,"error":"Creator closed before publication"}));
                }
            });
            return Ok(json!({"_deferred":true}));
        }
        let (staged, results) = stage(&self.project, &transaction)?;
        if transaction.dry_run {
            return Ok(json!({"dry_run":true,"operations":results}));
        }
        Ok(self.scepter_commit(staged, results, ui, ctx))
    }
    pub(super) fn scepter_commit(
        &mut self,
        staged: Project,
        results: Vec<Json>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Json {
        let materials_changed = self.project.procedural_materials != staged.procedural_materials
            || self.project.material_compositions != staged.material_compositions
            || self.project.tiles.len() != staged.tiles.len();
        let old = std::mem::replace(&mut self.project, staged);
        crate::undo::project_helper::refresh_region_tree_diff(ui, &old, &self.project);
        let missing_selection = match self.server_ctx.pc {
            ProjectContext::RegionCharacterInstance(region, id) => self
                .project
                .regions
                .iter()
                .find(|r| r.id == region)
                .is_none_or(|r| !r.characters.contains_key(&id)),
            ProjectContext::RegionItemInstance(region, id) => self
                .project
                .regions
                .iter()
                .find(|r| r.id == region)
                .is_none_or(|r| !r.items.contains_key(&id)),
            _ => false,
        };
        if missing_selection {
            let region = self.server_ctx.curr_region;
            crate::undo::project_helper::set_project_context(
                ctx,
                ui,
                &self.project,
                &mut self.server_ctx,
                ProjectContext::Region(region),
            );
        }

        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                "Scepter transaction".into(),
                Box::new(old),
                Box::new(self.project.clone()),
            ),
            ctx,
        );
        crate::utils::editor_scene_full_rebuild(&self.project, &self.server_ctx);
        update_region(ctx);
        if materials_changed {
            material_refresh(ctx);
        }
        json!({"operations":results,"undo_steps":1})
    }
}
fn region_request(
    region: Option<RegionRef>,
    include_tiles: bool,
    include_ascii: bool,
) -> ScepterRegionRequest {
    let mut r = ScepterRegionRequest {
        include_tiles,
        include_ascii,
        ..Default::default()
    };
    match region {
        Some(RegionRef::Id { id }) => r.id = Some(id),
        Some(RegionRef::Name { name }) => r.name = Some(name),
        None => {}
    }
    r
}
fn material_refresh(ctx: &mut TheContext) {
    for name in ["Scepter Materials Published", "Update Tilepicker"] {
        ctx.ui
            .send(TheEvent::Custom(TheId::named(name), TheValue::Empty));
    }
}
fn failure(error: impl ToString) -> Json {
    json!({"ok":false,"error":error.to_string()})
}
pub(super) fn parse_id(id: &str) -> Result<Uuid, String> {
    Uuid::parse_str(id).map_err(|_| format!("Invalid UUID: {id}"))
}
pub(super) fn region_index(project: &Project, reference: &RegionRef) -> Result<usize, String> {
    let matches = project
        .regions
        .iter()
        .enumerate()
        .filter(|(_, r)| match reference {
            RegionRef::Id { id } => r.id.to_string() == *id,
            RegionRef::Name { name } => r.name == *name,
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("Region must resolve to exactly one target".into());
    }
    Ok(matches[0])
}
fn validate_material(project: &Project, bundle: &str) -> Result<EditableMaterialAsset, String> {
    let mut asset = EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?;
    asset.validate().map_err(|e| e.to_string())?;
    if asset.root.project.materials.iter().any(|m| m.mode != "pbr") {
        return Err("Eldiron materials must use PBR mode".into());
    }
    // Match editor behavior: persist palette-index bindings for every color,
    // resolving values only for evaluation. Existing explicit indices survive.
    palette::ensure_color_bindings(&mut asset, &project.palette)?;
    if let Some(next) = image_source::refresh(&asset, project)? {
        asset = next;
    }
    asset.validate().map_err(|e| e.to_string())?;
    Ok(asset)
}
pub(super) fn stage(
    project: &Project,
    transaction: &Transaction,
) -> Result<(Project, Vec<Json>), String> {
    if transaction.operations.is_empty() || transaction.operations.len() > 128 {
        return Err("A transaction requires 1..128 operations".into());
    }
    let _aliases = rusterix::server::data::TileAliasScope::new(&project.tiles);
    let mut next = project.clone();
    let mut results = vec![];
    let mut total_cells = 0;
    for (index, operation) in transaction.operations.iter().enumerate() {
        if let EditOperation::EraseCells(c) = operation {
            total_cells += c.cells.len();
            if total_cells > 4096 {
                return Err("Maximum 4096 painted/erased cells per transaction".into());
            }
        }
        let apply = (|| -> Result<Json, String> {
            Ok(match operation {
                EditOperation::MaterialPut { id, bundle } => {
                    let id = parse_id(id)?;
                    if next.material_compositions.contains_key(&id) {
                        return Err("Use composition_put to edit a composition".into());
                    }
                    let asset = validate_material(&next, bundle)?;
                    if asset.root.id != id.to_string() {
                        return Err("Material document root ID must match its tile UUID".into());
                    }
                    next.procedural_materials
                        .insert(id, asset.save().map_err(|e| e.to_string())?);
                    next.tiles.entry(id).or_insert_with(|| rusterix::Tile {
                        id,
                        alias: asset.selected_material.clone(),
                        ..Default::default()
                    });
                    next.procedural_material_cache_keys.shift_remove(&id);
                    json!({"id":id})
                }
                EditOperation::CompositionPut { id, composition } => {
                    let id = parse_id(id)?;
                    let c: MaterialComposition =
                        serde_json::from_value(composition.clone()).map_err(|e| e.to_string())?;
                    validate_composition(&c)?;
                    if next.procedural_materials.contains_key(&id)
                        && !next.material_compositions.contains_key(&id)
                    {
                        return Err("ID belongs to a source material".into());
                    }
                    next.material_compositions.insert(id, c);
                    next.tiles.entry(id).or_insert_with(|| rusterix::Tile {
                        id,
                        alias: "Composition".into(),
                        ..Default::default()
                    });
                    json!({"id":id})
                }
                EditOperation::CompositionCopy { id, source } => {
                    let id = parse_id(id)?;
                    let source = parse_id(source)?;
                    if next.tiles.contains_key(&id) {
                        return Err("Copy destination already exists".into());
                    }
                    let mut c = next
                        .material_compositions
                        .get(&source)
                        .ok_or("Source composition not found")?
                        .clone();
                    for l in &mut c.layers {
                        l.id = Uuid::new_v4();
                    }
                    next.material_compositions.insert(id, c);
                    next.tiles.insert(
                        id,
                        rusterix::Tile {
                            id,
                            alias: "Composition copy".into(),
                            ..Default::default()
                        },
                    );
                    json!({"id":id})
                }
                EditOperation::SurfaceApply {
                    region,
                    surface,
                    material,
                } => {
                    let i = region_index(&next, region)?;
                    let material = parse_id(material)?;
                    if !next.tiles.contains_key(&material) {
                        return Err("Material/tile not found".into());
                    }
                    let surface: CompositionSurface =
                        serde_json::from_value(surface.clone()).map_err(|e| e.to_string())?;
                    validate_surface(&surface)?;
                    let binding = MapMaterialBinding {
                        map: next.regions[i].map.id,
                        surface,
                    };
                    // One placement-owned cache, shared source recipe. This preserves
                    // world UV coverage without modifying other placed compositions.
                    let id = next
                        .map_material_bindings
                        .iter()
                        .find(|(_, b)| **b == binding)
                        .map(|(id, _)| *id)
                        .unwrap_or_else(Uuid::new_v4);
                    if id != material {
                        let c = next
                            .material_compositions
                            .get(&material)
                            .cloned()
                            .unwrap_or(MaterialComposition {
                                base: material,
                                layers: vec![],
                                tiling: Default::default(),
                            });
                        next.material_compositions.insert(id, c);
                    }
                    next.tiles.entry(id).or_insert_with(|| rusterix::Tile {
                        id,
                        alias: "Map composition".into(),
                        ..Default::default()
                    });
                    if !map_composition::set_source(&mut next.regions[i].map, &binding, id) {
                        return Err("Surface target does not exist".into());
                    }
                    next.regions[i].map.changed = next.regions[i].map.changed.wrapping_add(1);
                    next.map_material_bindings.insert(id, binding);
                    json!({"material":id,"source":material})
                }
                EditOperation::SectorUpdate(_)
                | EditOperation::SectorCreate(_)
                | EditOperation::EraseCells(_)
                | EditOperation::MapTransform(_)
                | EditOperation::MapCopy(_)
                | EditOperation::MapDelete(_)
                | EditOperation::GeometryCreate(_)
                | EditOperation::EntityPlace(_)
                | EditOperation::EntityUpdate(_) => {
                    super::scepter_map::apply(&mut next, operation)?
                }
                EditOperation::PaintCells(c) => {
                    total_cells += c.cells.len();
                    if total_cells > 4096 {
                        return Err("Maximum 4096 painted cells per transaction".into());
                    }
                    paint_cells(&mut next, c)?
                }
                EditOperation::PaintRect(c) => {
                    let [x, y, w, h] = c.rect;
                    let ex = x.checked_add(w).ok_or("Rectangle overflows")?;
                    let ey = y.checked_add(h).ok_or("Rectangle overflows")?;
                    let count = i64::from(w).abs() * i64::from(h).abs();
                    if count == 0 || count > 4096 {
                        return Err("Rectangle requires 1..4096 cells".into());
                    }
                    let cells = (y.min(ey)..y.max(ey))
                        .flat_map(|y| (x.min(ex)..x.max(ex)).map(move |x| [x, y]))
                        .collect();
                    let c = RegionPaintCells {
                        region: c.region.clone(),
                        tile: c.tile.clone(),
                        cells,
                        layer: c.layer.clone(),
                        select: c.select,
                        replace_existing: c.replace_existing,
                    };
                    total_cells += c.cells.len();
                    if total_cells > 4096 {
                        return Err("Maximum 4096 painted cells per transaction".into());
                    }
                    paint_cells(&mut next, &c)?
                }
            })
        })();
        results.push(apply.map_err(|e| format!("Operation {index}: {e}"))?);
    }
    shared::entity_graph::synchronize(&mut next)?;
    shared::rusterix_utils::insert_content_into_maps(&mut next);
    // Rebuild all dependants after all operations, permitting forward references
    // and nested compositions. Builder rejects cycles and missing source assets.
    let errors = composition::prepare_project(&mut next);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    for (id, bundle) in &next.procedural_materials {
        EditableMaterialAsset::reopen(bundle)
            .map_err(|e| e.to_string())?
            .validate()
            .map_err(|e| e.to_string())?;
        if project.procedural_materials.get(id) != Some(bundle) {
            next.procedural_material_cache_keys.shift_remove(id);
        }
    }
    // SceneVM requires a valid first frame when the project is published.
    // New assets stay transparent until the normal background bake installs
    // their pixels; renderer-backed previews always await the finished bake.
    let pixels = crate::procedural_material::pixel_style::density(&next.config);
    let size = if pixels == 0 { 64 } else { pixels as usize };
    for id in next.procedural_materials.keys() {
        if let Some(tile) = next.tiles.get_mut(id) {
            if tile.textures.is_empty() {
                tile.textures.push(rusterix::Texture::alloc(size, size));
            }
        }
    }
    Ok((next, results))
}
fn validate_composition(c: &MaterialComposition) -> Result<(), String> {
    if c.layers.len() > 32 {
        return Err("Maximum 32 layers per composition".into());
    }
    if !["raw", "seamless", "repeat"].contains(&c.tiling.mode.as_str())
        || c.tiling
            .period
            .iter()
            .any(|v| !v.is_finite() || *v <= 0. || *v > 128.)
        || !(0. ..=1.).contains(&c.tiling.blend)
    {
        return Err("Invalid tiling settings".into());
    }
    let mut ids = HashSet::new();
    for l in &c.layers {
        if !ids.insert(l.id)
            || ![
                l.position,
                l.angle,
                l.feather,
                l.amount,
                l.wobble,
                l.wobble_scale,
                l.overlay_level,
            ]
            .iter()
            .chain(l.uv_levels.iter())
            .all(|v| v.is_finite())
            || !(0. ..=1.).contains(&l.amount)
            || l.feather < 0.
            || l.wobble < 0.
            || l.wobble_scale <= 0.
            || l.uv_direction > 1
        {
            return Err("Invalid composition layer parameters".into());
        }
    }
    Ok(())
}
fn validate_surface(surface: &CompositionSurface) -> Result<(), String> {
    if matches!(surface,CompositionSurface::Linedef{row,..} if !(1..=4).contains(row)) {
        return Err("Wall row must be 1..4".into());
    }
    Ok(())
}
fn paint_cells(project: &mut Project, c: &RegionPaintCells) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    let candidates = project
        .tiles
        .values()
        .filter(|t| {
            c.tile.id.as_ref().is_none_or(|id| t.id.to_string() == *id)
                && c.tile.alias.as_ref().is_none_or(|a| t.alias == *a)
                && c.tile
                    .role
                    .as_ref()
                    .is_none_or(|r| t.role.to_string().eq_ignore_ascii_case(r))
                && c.tile.kind.as_ref().is_none_or(|v| t.procedural.kind == *v)
                && c.tile
                    .style
                    .as_ref()
                    .is_none_or(|v| t.procedural.style == *v)
                && c.tile.tags.iter().all(|tag| t.gameplay_tags.contains(tag))
        })
        .map(|t| t.id)
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return Err("Tile selector must resolve to exactly one tile".into());
    }
    if c.cells.is_empty() {
        return Err("No cells supplied".into());
    }
    let layer = c
        .layer
        .as_deref()
        .unwrap_or("1")
        .parse::<u8>()
        .map_err(|_| "Invalid layer")?;
    if c.replace_existing.unwrap_or(true) {
        super::scepter_map::erase_cells(
            project,
            &eldiron_scepter::EraseCells {
                region: c.region.clone(),
                cells: c.cells.clone(),
                layer: Some(layer),
            },
        )?;
    }
    let map = &mut project.regions[i].map;
    let mut sectors = vec![];
    let mut unique = HashSet::new();
    for [x, y] in &c.cells {
        if !unique.insert((*x, *y)) {
            continue;
        }
        if x.abs_diff(0) > 1_000_000 || y.abs_diff(0) > 1_000_000 {
            return Err("Cell outside supported coordinate range".into());
        }
        let id = Editor::scepter_create_cell_sector(
            map,
            *x,
            *y,
            rusterix::PixelSource::TileId(candidates[0]),
            layer,
        )
        .ok_or("Could not create cell sector")?;
        sectors.push(id);
    }
    if c.select.unwrap_or(false) {
        map.selected_vertices.clear();
        map.selected_linedefs.clear();
        map.selected_sectors = sectors.clone();
    }
    map.changed = map.changed.wrapping_add(1);
    Ok(json!({"sectors":sectors,"tile":candidates[0]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::material_composition::{CompositionLayer, CompositionShape};
    pub(super) fn material(project: &Project, id: Uuid, color: u16) -> String {
        let mut asset = EditableMaterialAsset::parse(
            &id.to_string(),
            r#"project 1
material "surface" called "Test surface"
  mode pbr
  row "color"
    use "Color" id "pigment"
  output base color from row "color" xyz
  output opacity from row "color" w
"#,
            "surface",
        )
        .unwrap();
        palette::ensure_color_bindings(&mut asset, &project.palette).unwrap();
        asset =
            palette::bind_parameter(&asset, "pigment", "color", color, &project.palette).unwrap();
        asset.save().unwrap()
    }
    pub(super) fn road_fixture() -> Project {
        let p = Project::default();
        let grass = Uuid::new_v4();
        let road = Uuid::new_v4();
        let mix = Uuid::new_v4();
        let mut layer = CompositionLayer::split(road);
        layer.shape = CompositionShape::Road;
        layer.position = 0.4;
        layer.feather = 0.06;
        layer.wobble = 0.035;
        let c = MaterialComposition {
            base: grass,
            layers: vec![layer],
            tiling: Default::default(),
        };
        let mut ops = vec![
            EditOperation::MaterialPut {
                id: grass.to_string(),
                bundle: material(&p, grass, 37),
            },
            EditOperation::MaterialPut {
                id: road.to_string(),
                bundle: material(&p, road, 28),
            },
            EditOperation::CompositionPut {
                id: mix.to_string(),
                composition: json!(c),
            },
        ];
        for x in 0..3 {
            // Explicit existing floor surfaces, as in 2D Rect painting and 3D floors.
            ops.push(EditOperation::PaintCells(RegionPaintCells {
                region: RegionRef::id(p.regions[0].id.to_string()),
                tile: TileSelector {
                    id: Some(grass.to_string()),
                    ..Default::default()
                },
                cells: vec![[x, 0]],
                layer: None,
                select: None,
                replace_existing: Some(true),
            }));
        }
        let (mut p, _) = stage(
            &p,
            &Transaction {
                operations: ops,
                dry_run: false,
            },
        )
        .unwrap();
        let mut ops: Vec<_> = p.regions[0]
            .map
            .sectors
            .iter()
            .filter(|s| s.properties.get_bool_default("rect", false))
            .map(|s| EditOperation::SurfaceApply {
                region: RegionRef::id(p.regions[0].id.to_string()),
                surface: json!(CompositionSurface::RectTile { sector: s.id }),
                material: mix.to_string(),
            })
            .collect();
        for object in &p.regions[0].map.geometry_objects {
            for face in 0..object.faces.len() {
                ops.push(EditOperation::SurfaceApply {
                    region: RegionRef::id(p.regions[0].id.to_string()),
                    surface: json!(CompositionSurface::GeometryFace {
                        object: object.id,
                        face
                    }),
                    material: mix.to_string(),
                });
            }
        }
        p = stage(
            &p,
            &Transaction {
                operations: ops,
                dry_run: false,
            },
        )
        .unwrap()
        .0;
        p
    }
    #[test]
    fn failed_transaction_leaves_all_live_assets_and_geometry_untouched() {
        let p = Project::default();
        let before = serde_json::to_string(&p).unwrap();
        let id = Uuid::new_v4();
        let tx = Transaction {
            operations: vec![
                EditOperation::MaterialPut {
                    id: id.to_string(),
                    bundle: material(&p, id, 37),
                },
                EditOperation::SurfaceApply {
                    region: RegionRef::id(p.regions[0].id.to_string()),
                    surface: json!(CompositionSurface::Sector {
                        id: u32::MAX,
                        ceiling: false
                    }),
                    material: id.to_string(),
                },
            ],
            dry_run: false,
        };
        assert!(stage(&p, &tx).unwrap_err().contains("Surface target"));
        assert_eq!(serde_json::to_string(&p).unwrap(), before);
    }
    #[test]
    fn composition_supports_raster_sources_nested_layers_and_rejects_cycles() {
        let mut p = road_fixture();
        let first = *p.material_compositions.keys().next().unwrap();
        let id = Uuid::new_v4();
        let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(4, 4)]);
        let raster = tile.id;
        p.tiles.insert(raster, tile);
        let c = MaterialComposition {
            base: first,
            layers: vec![CompositionLayer::split(raster)],
            tiling: Default::default(),
        };
        let tx = Transaction {
            operations: vec![EditOperation::CompositionPut {
                id: id.to_string(),
                composition: json!(c),
            }],
            dry_run: false,
        };
        let (next, _) = stage(&p, &tx).unwrap();
        assert!(next.procedural_materials.contains_key(&id));
        let mut c = next.material_compositions[&id].clone();
        c.base = id;
        assert!(
            stage(
                &next,
                &Transaction {
                    operations: vec![EditOperation::CompositionPut {
                        id: id.to_string(),
                        composition: json!(c)
                    }],
                    dry_run: false
                }
            )
            .unwrap_err()
            .contains("cycle")
        );
    }
    #[test]
    fn rejects_invalid_palette_and_unbounded_paint() {
        let p = Project::default();
        let id = Uuid::new_v4();
        let mut asset = EditableMaterialAsset::reopen(&material(&p, id, 37)).unwrap();
        asset
            .instances
            .iter_mut()
            .flat_map(|r| &mut r.palette_bindings)
            .next()
            .unwrap()
            .index = 65535;
        assert!(validate_material(&p, &asset.save().unwrap()).is_err());
        let tx = Transaction {
            operations: vec![EditOperation::PaintRect(RegionPaintRect {
                region: RegionRef::id(p.regions[0].id.to_string()),
                tile: TileSelector::default(),
                rect: [i32::MAX, 0, 2, 1],
                layer: None,
                select: None,
                replace_existing: None,
            })],
            dry_run: false,
        };
        assert!(stage(&p, &tx).unwrap_err().contains("overflows"));
    }
    #[test]
    #[ignore = "requires a native GPU"]
    fn scepter_gpu_road_preview_2d_and_3d() {
        let project = road_fixture();
        let mut editor = Editor::new();
        editor.project = project.clone();
        *UNDOMANAGER.write().unwrap() = UndoManager::default();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(800, 600, 1.);
        let execute = |editor: &mut Editor,
                       command: Command,
                       session: Option<String>,
                       revision: Option<String>,
                       ui: &mut TheUI,
                       ctx: &mut TheContext| {
            let job = Arc::new(crate::scepter::jobs::Job::new());
            assert!(job.begin());
            editor.scepter_execute(
                ScepterRequest {
                    request_id: Some(Uuid::new_v4().to_string()),
                    session,
                    revision,
                    command,
                },
                job.clone(),
                ui,
                ctx,
            );
            job.response(std::time::Duration::ZERO)
        };
        let session = editor.scepter_session();
        let revision = editor.scepter_revision();
        let id = Uuid::new_v4();
        let bundle = material(&editor.project, id, 37);
        let command = Command::Transaction(Transaction {
            operations: vec![EditOperation::MaterialPut {
                id: id.to_string(),
                bundle,
            }],
            dry_run: false,
        });
        let result = execute(
            &mut editor,
            command.clone(),
            Some(Uuid::new_v4().to_string()),
            Some(revision.clone()),
            &mut ui,
            &mut ctx,
        );
        assert_eq!(result["http_status"], 409);
        assert!(!editor.project.tiles.contains_key(&id));
        let result = execute(
            &mut editor,
            command.clone(),
            Some(session.clone()),
            Some(revision.clone()),
            &mut ui,
            &mut ctx,
        );
        assert_eq!(result["undo_steps"], 1);
        assert!(editor.project.procedural_materials.contains_key(&id));
        let after = editor.scepter_revision();
        assert_ne!(after, revision);
        let result = execute(
            &mut editor,
            command,
            Some(session.clone()),
            Some(revision),
            &mut ui,
            &mut ctx,
        );
        assert_eq!(result["http_status"], 409);
        let result = execute(
            &mut editor,
            Command::ProjectUndo,
            Some(session.clone()),
            Some(after),
            &mut ui,
            &mut ctx,
        );
        assert_eq!(result["ok"], true);
        assert!(!editor.project.procedural_materials.contains_key(&id));
        assert!(!UNDOMANAGER.read().unwrap().has_undo());
        let revision = editor.scepter_revision();
        let result = execute(
            &mut editor,
            Command::ProjectRedo,
            Some(session),
            Some(revision),
            &mut ui,
            &mut ctx,
        );
        assert_eq!(result["ok"], true);
        assert!(editor.project.procedural_materials.contains_key(&id));
        let revision = editor.scepter_revision();
        editor.project.tiles.get_mut(&id).unwrap().textures =
            vec![rusterix::Texture::alloc(32, 32)];
        assert_eq!(
            editor.scepter_revision(),
            revision,
            "A completed GPU bake must not invalidate authoring context"
        );
        let out = std::path::Path::new("/tmp/eldiron-scepter");
        std::fs::create_dir_all(out).unwrap();
        for view in ["2d", "iso", "3d"] {
            let request:RegionRenderPreview=serde_json::from_value(json!({"region":{"id":project.regions[0].id},"view":view,"bounds":[0,0,3,1],"size":[640,384],"camera":{"center":[1.5,0.,0.5],"position":[1.5,3.,4.],"scale":2.5}})).unwrap();
            let result =
                super::super::scepter_preview::render(project.clone(), 0, &request).unwrap();
            assert_eq!(result["ready"], true);
            let base64 = result["png_base64"].as_str().unwrap();
            let mut decoded = Vec::new();
            let mut bits = 0_u32;
            let mut count = 0;
            for byte in base64.bytes().take_while(|b| *b != b'=') {
                let n = match byte {
                    b'A'..=b'Z' => byte - b'A',
                    b'a'..=b'z' => byte - b'a' + 26,
                    b'0'..=b'9' => byte - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    _ => panic!("Invalid base64"),
                };
                bits = (bits << 6) | u32::from(n);
                count += 6;
                if count >= 8 {
                    count -= 8;
                    decoded.push((bits >> count) as u8);
                }
            }
            let mut decoder = png::Decoder::new(std::io::Cursor::new(&decoded));
            decoder.set_transformations(png::Transformations::EXPAND);
            let mut reader = decoder.read_info().unwrap();
            let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
            reader.next_frame(&mut rgba).unwrap();
            let road = rgba
                .chunks_exact(4)
                .filter(|p| {
                    u32::from(p[0]) > u32::from(p[1]) * 3 / 2
                        && u32::from(p[0]) > u32::from(p[2]) * 2
                        && p[0] > 60
                })
                .count();
            let grass = rgba
                .chunks_exact(4)
                .filter(|p| {
                    u32::from(p[1]) > u32::from(p[0]) * 6 / 5
                        && u32::from(p[1]) > u32::from(p[2]) * 11 / 10
                        && p[1] > 30
                })
                .count();
            assert!(
                road > 200 && grass > 200,
                "{view} must render both authored materials, not a placeholder/checkerboard"
            );
            std::fs::write(out.join(format!("road-{view}.base64")), base64).unwrap();
            std::fs::write(out.join(format!("road-{view}.png")), &decoded).unwrap();
        }
    }
}
