//! Prepare transactions away from the UI; recheck collaboration context at publication.
use super::*;
use serde_json::{Value as Json, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
static PREPARING: AtomicBool = AtomicBool::new(false);
#[derive(Debug)]
pub(crate) struct Permit;
impl Permit {
    pub(super) fn acquire() -> Option<Self> {
        PREPARING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        PREPARING.store(false, Ordering::Release);
    }
}
#[derive(Debug)]
pub struct PreparedEdit {
    pub(crate) session: String,
    pub(crate) revision: String,
    pub(crate) selection: Json,
    pub(crate) dry_run: bool,
    pub(crate) result: Result<(Box<Project>, Vec<Json>), String>,
    pub(crate) job: Arc<crate::scepter::jobs::Job>,
    pub(crate) permit: Permit,
}
impl Editor {
    pub(super) fn scepter_finish_staged(
        &mut self,
        edit: PreparedEdit,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        let _permit = edit.permit;
        let revision = self.scepter_revision();
        let session = self.scepter_session();
        if revision != edit.revision || session != edit.session {
            edit.job.finish(json!({"ok":false,"http_status":409,"error":"Project changed while the transaction was preparing; inspect before retrying","session":session,"revision":revision}));
            return false;
        }
        let (mut staged, results) = match edit.result {
            Ok(v) => v,
            Err(error) => {
                edit.job.finish(
                    json!({"ok":false,"error":error,"session":session,"revision":revision}),
                );
                return false;
            }
        };
        if edit.dry_run {
            edit.job.finish(json!({"ok":true,"dry_run":true,"operations":results,"session":session,"revision":revision}));
            return false;
        }
        let selection_changed = self.scepter_view(ui)["selection"] != edit.selection;
        preserve_view(&self.project, &mut staged, selection_changed);
        let mut result = self.scepter_commit(*staged, results, ui, ctx);
        result["ok"] = json!(true);
        result["session"] = json!(session);
        result["revision"] = json!(self.scepter_revision());
        self.scepter_observe(true, ui);
        edit.job.finish(result);
        true
    }
}
fn preserve_view(live: &Project, staged: &mut Project, selection_changed: bool) {
    if live.config == staged.config {
        let pixels = crate::procedural_material::pixel_style::density(&staged.config);
        for (id, bundle) in &staged.procedural_materials {
            if live.procedural_materials.get(id) != Some(bundle) {
                continue;
            }
            let key = crate::procedural_material::pixel_style::bake_key(bundle, pixels);
            if live.procedural_material_cache_keys.get(id) == Some(&key) {
                if let (Some(source), Some(dest)) = (live.tiles.get(id), staged.tiles.get_mut(id)) {
                    dest.textures = source.textures.clone();
                    dest.uv_pages = source.uv_pages.clone();
                    dest.baked_material_data = source.baked_material_data.clone();
                    staged.procedural_material_cache_keys.insert(*id, key);
                }
            }
        }
    }
    for region in &mut staged.regions {
        let Some(from) = live.regions.iter().find(|r| r.id == region.id) else {
            continue;
        };
        region.editing_position_3d = from.editing_position_3d;
        region.editing_look_at_3d = from.editing_look_at_3d;
        region.editing_position_iso_3d = from.editing_position_iso_3d;
        region.editing_look_at_iso_3d = from.editing_look_at_iso_3d;
        region.editing_position_orbit_3d = from.editing_position_orbit_3d;
        region.editing_look_at_orbit_3d = from.editing_look_at_orbit_3d;
        region.editing_position_firstp_3d = from.editing_position_firstp_3d;
        region.editing_look_at_firstp_3d = from.editing_look_at_firstp_3d;
        region.editing_iso_scale = from.editing_iso_scale;
        let a = &from.map;
        let b = &mut region.map;
        b.offset = a.offset;
        b.grid_size = a.grid_size;
        b.subdivisions = a.subdivisions;
        b.camera = a.camera;
        b.curr_grid_pos = a.curr_grid_pos;
        b.curr_grid_pos_3d = a.curr_grid_pos_3d;
        b.curr_mouse_pos = a.curr_mouse_pos;
        if selection_changed {
            b.selected_vertices = a
                .selected_vertices
                .iter()
                .copied()
                .filter(|id| b.find_vertex(*id).is_some())
                .collect();
            b.selected_linedefs = a
                .selected_linedefs
                .iter()
                .copied()
                .filter(|id| b.find_linedef(*id).is_some())
                .collect();
            b.selected_sectors = a
                .selected_sectors
                .iter()
                .copied()
                .filter(|id| b.find_sector(*id).is_some())
                .collect();
            b.selected_geometry_objects = a
                .selected_geometry_objects
                .iter()
                .copied()
                .filter(|id| b.geometry_objects.iter().any(|o| o.id == *id))
                .collect();
            b.selected_geometry_faces = a
                .selected_geometry_faces
                .iter()
                .copied()
                .filter(|(id, f)| {
                    b.geometry_objects
                        .iter()
                        .any(|o| o.id == *id && o.faces.get(*f).is_some())
                })
                .collect();
            b.selected_geometry_vertices = a
                .selected_geometry_vertices
                .iter()
                .copied()
                .filter(|(id, v)| {
                    b.geometry_objects
                        .iter()
                        .any(|o| o.id == *id && o.vertices.get(*v).is_some())
                })
                .collect();
            b.selected_entity_item = a
                .selected_entity_item
                .filter(|id| region.characters.contains_key(id) || region.items.contains_key(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eldiron_scepter::{
        EditOperation, GeometryCreate, RegionRef, ScepterCommand, ScepterRequest, Transaction,
    };
    #[test]
    fn scepter_preparation_rechecks_revision_and_releases_the_worker() {
        let mut editor = Editor::new();
        editor.project = Project::default();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(800, 600, 1.);
        let (tx, rx) = std::sync::mpsc::channel();
        editor.scepter_sender = Some(tx);
        let id = Uuid::new_v4();
        let command = |dry_run| {
            ScepterCommand::Transaction(Transaction {
                operations: vec![EditOperation::GeometryCreate(GeometryCreate {
                    region: RegionRef::id(editor.project.regions[0].id.to_string()),
                    id: id.to_string(),
                    name: "Background box".into(),
                    shape: "box".into(),
                    min: [0.; 3],
                    max: [1.; 3],
                    material: None,
                    segments: None,
                })],
                dry_run,
            })
        };
        let command_a = command(false);
        let command_b = command(true);
        let request = |editor: &Editor, command| ScepterRequest {
            request_id: Some(Uuid::new_v4().to_string()),
            session: Some(editor.scepter_session()),
            revision: Some(editor.scepter_revision()),
            command,
        };
        let job = Arc::new(crate::scepter::jobs::Job::new());
        job.begin();
        editor.scepter_execute(request(&editor, command_a), job.clone(), &mut ui, &mut ctx);
        assert_eq!(job.response(std::time::Duration::ZERO)["http_status"], 202);
        editor.project.regions[0].map.changed += 1;
        let crate::scepter::ScepterEvent::PreparedEdit(edit) =
            rx.recv_timeout(std::time::Duration::from_secs(20)).unwrap()
        else {
            panic!("Missing prepared edit")
        };
        assert!(!editor.scepter_finish_staged(edit, &mut ui, &mut ctx));
        assert_eq!(job.response(std::time::Duration::ZERO)["http_status"], 409);
        assert!(
            !editor.project.regions[0]
                .map
                .geometry_objects
                .iter()
                .any(|o| o.id == id)
        );
        let job = Arc::new(crate::scepter::jobs::Job::new());
        job.begin();
        editor.scepter_execute(request(&editor, command_b), job.clone(), &mut ui, &mut ctx);
        let crate::scepter::ScepterEvent::PreparedEdit(edit) =
            rx.recv_timeout(std::time::Duration::from_secs(20)).unwrap()
        else {
            panic!("Missing prepared edit")
        };
        assert!(!editor.scepter_finish_staged(edit, &mut ui, &mut ctx));
        assert_eq!(job.response(std::time::Duration::ZERO)["dry_run"], true);
        assert!(
            !editor.project.regions[0]
                .map
                .geometry_objects
                .iter()
                .any(|o| o.id == id)
        );
    }
    #[test]
    fn scepter_publication_keeps_live_camera_selection_and_completed_bakes() {
        let mut live = Project::default();
        let mut staged = live.clone();
        live.regions[0].map.offset = Vec2::new(123., 456.);
        live.regions[0].editing_position_3d = Vec3::new(7., 8., 9.);
        let selected = live.regions[0].map.geometry_objects[0].id;
        live.regions[0].map.selected_geometry_objects = vec![selected];
        let id = Uuid::new_v4();
        live.procedural_materials.insert(id, "same-bundle".into());
        staged.procedural_materials.insert(id, "same-bundle".into());
        live.tiles.insert(
            id,
            rusterix::Tile {
                id,
                textures: vec![rusterix::Texture::alloc(8, 8)],
                ..Default::default()
            },
        );
        staged.tiles.insert(
            id,
            rusterix::Tile {
                id,
                textures: vec![rusterix::Texture::alloc(2, 2)],
                ..Default::default()
            },
        );
        let key = crate::procedural_material::pixel_style::bake_key(
            "same-bundle",
            crate::procedural_material::pixel_style::density(&live.config),
        );
        live.procedural_material_cache_keys.insert(id, key);
        preserve_view(&live, &mut staged, true);
        assert_eq!(staged.regions[0].map.offset, live.regions[0].map.offset);
        assert_eq!(
            staged.regions[0].map.selected_geometry_objects,
            vec![selected]
        );
        assert_eq!(staged.tiles[&id].textures[0].width, 8);
    }
}
