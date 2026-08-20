use crate::editor::{RUSTERIX, SCENEMANAGER, UNDOMANAGER};
use crate::prelude::*;

#[derive(Clone, Copy)]
enum PrefabActionKind {
    CreateLinked,
    CreateCopy,
    UpdateSource,
    MakeUnique,
    Unpack,
}

pub struct PrefabAction {
    id: TheId,
    kind: PrefabActionKind,
}

impl PrefabAction {
    fn with_kind(kind: PrefabActionKind, label: String) -> Self {
        Self {
            id: TheId::named(&label),
            kind,
        }
    }

    fn selected_project_prefab(server_ctx: &ServerContext) -> Option<Uuid> {
        let id = server_ctx.curr_block_asset_id?;
        RUSTERIX
            .read()
            .unwrap()
            .assets
            .block_props
            .contains_key(&id)
            .then_some(id)
    }

    fn next_name(project: &Project) -> String {
        let mut number = project.block_props.len() + 1;
        loop {
            let candidate = fl!("prefab_default_name", number = number);
            if project
                .block_props
                .values()
                .all(|asset| asset.name != candidate)
            {
                return candidate;
            }
            number += 1;
        }
    }

    fn finish(
        label: String,
        before: Project,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_block_props(project.block_props.clone());
        }
        SCENEMANAGER
            .write()
            .unwrap()
            .set_block_props(project.block_props.clone());

        let updated_incrementally = before
            .get_map(server_ctx)
            .zip(project.get_map(server_ctx))
            .is_some_and(|(old_map, new_map)| {
                crate::toollist::ToolList::try_incremental_map_edit(old_map, new_map, server_ctx)
            });
        if !updated_incrementally {
            crate::utils::editor_scene_refresh_prefab_assets(&before, project, server_ctx);
        }

        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(label, Box::new(before), Box::new(project.clone())),
            ctx,
        );
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Geometry Overlay 3D"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
    }
}

macro_rules! prefab_action_type {
    ($name:ident, $kind:ident, $label:literal, $description:literal) => {
        pub struct $name(PrefabAction);

        impl Action for $name {
            fn new() -> Self {
                Self(PrefabAction::with_kind(
                    PrefabActionKind::$kind,
                    fl!($label),
                ))
            }

            fn id(&self) -> TheId {
                self.0.id.clone()
            }

            fn info(&self) -> String {
                fl!($description)
            }

            fn role(&self) -> ActionRole {
                ActionRole::Editor
            }

            fn is_applicable(
                &self,
                map: &Map,
                _ctx: &mut TheContext,
                server_ctx: &ServerContext,
            ) -> bool {
                if server_ctx.editor_view_mode == EditorViewMode::D2
                    || server_ctx.curr_map_tool_type != MapToolType::Selection
                    || server_ctx.pc.is_prefab()
                {
                    return false;
                }
                match self.0.kind {
                    PrefabActionKind::CreateLinked | PrefabActionKind::CreateCopy => {
                        !map.selected_geometry_objects.is_empty()
                    }
                    PrefabActionKind::UpdateSource => {
                        !map.selected_geometry_objects.is_empty()
                            && PrefabAction::selected_project_prefab(server_ctx).is_some()
                    }
                    PrefabActionKind::MakeUnique => map.selected_block_prop_instances.len() == 1,
                    PrefabActionKind::Unpack => !map.selected_block_prop_instances.is_empty(),
                }
            }

            fn apply_project(
                &self,
                project: &mut Project,
                _ui: &mut TheUI,
                ctx: &mut TheContext,
                server_ctx: &mut ServerContext,
            ) {
                let before = project.clone();
                let result = match self.0.kind {
                    PrefabActionKind::CreateLinked | PrefabActionKind::CreateCopy => {
                        let mode = if matches!(self.0.kind, PrefabActionKind::CreateLinked) {
                            crate::block_props::BlockPropCreateMode::ReplaceSelection
                        } else {
                            crate::block_props::BlockPropCreateMode::KeepSelection
                        };
                        crate::block_props::create_authored_block_prop(
                            project,
                            server_ctx,
                            PrefabAction::next_name(project),
                            mode,
                        )
                        .map(|created| {
                            server_ctx.curr_block_asset_id = Some(created.asset_id);
                            server_ctx.curr_block_asset_name = project
                                .block_props
                                .get(&created.asset_id)
                                .map(|asset| asset.name.clone());
                            fl!(
                                "status_prefab_created",
                                name = server_ctx.curr_block_asset_name.as_deref().unwrap_or(""),
                                count = created.source_object_count
                            )
                        })
                    }
                    PrefabActionKind::UpdateSource => {
                        let Some(asset_id) = server_ctx.curr_block_asset_id else {
                            return;
                        };
                        crate::block_props::update_authored_block_prop(
                            project, server_ctx, asset_id,
                        )
                        .map(|count| {
                            fl!(
                                "status_prefab_source_updated",
                                name = server_ctx.curr_block_asset_name.as_deref().unwrap_or(""),
                                count = count
                            )
                        })
                    }
                    PrefabActionKind::MakeUnique => {
                        crate::block_props::make_selected_block_prop_unique(project, server_ctx)
                            .map(|unique| {
                                server_ctx.curr_block_asset_id = Some(unique.unique_asset_id);
                                server_ctx.curr_block_asset_name = project
                                    .block_props
                                    .get(&unique.unique_asset_id)
                                    .map(|asset| asset.name.clone());
                                fl!(
                                    "status_prefab_made_unique",
                                    name =
                                        server_ctx.curr_block_asset_name.as_deref().unwrap_or("")
                                )
                            })
                    }
                    PrefabActionKind::Unpack => {
                        crate::block_props::unpack_selected_block_props(project, server_ctx)
                            .map(|count| fl!("status_prefab_unpacked", count = count))
                    }
                };

                match result {
                    Ok(status) => {
                        PrefabAction::finish(self.info(), before, project, ctx, server_ctx);
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
            }

            fn params(&self) -> TheNodeUI {
                TheNodeUI::default()
            }

            fn handle_event(
                &mut self,
                _event: &TheEvent,
                _project: &mut Project,
                _ui: &mut TheUI,
                _ctx: &mut TheContext,
                _server_ctx: &mut ServerContext,
            ) -> bool {
                false
            }
        }
    };
}

prefab_action_type!(
    CreateLinkedPrefab,
    CreateLinked,
    "action_create_linked_prefab",
    "action_create_linked_prefab_desc"
);
prefab_action_type!(
    CreatePrefabCopy,
    CreateCopy,
    "action_create_prefab_copy",
    "action_create_prefab_copy_desc"
);
prefab_action_type!(
    UpdatePrefabSource,
    UpdateSource,
    "action_update_prefab_source",
    "action_update_prefab_source_desc"
);
prefab_action_type!(
    MakePrefabUnique,
    MakeUnique,
    "action_make_prefab_unique",
    "action_make_prefab_unique_desc"
);
prefab_action_type!(
    UnpackPrefab,
    Unpack,
    "action_unpack_prefab",
    "action_unpack_prefab_desc"
);
