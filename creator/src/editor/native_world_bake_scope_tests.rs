//! Publication-owner contract. Drive the real CPU controller to a completed
//! asset without preparing a displayed scene; GPU tile production has its own
//! channel/loss oracle and assembled application coverage.
use super::*;

#[test]
#[ignore = "native GPU context injection; no world framebuffer or capture"]
fn frozen_bake_scope_survives_missing_layout_and_rejects_retired_completed_sources() {
    let gpu = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    for phase in 0..6 {
        let mut editor = Editor::new();
        let region = editor.documents.project.regions[0].id;
        let map = editor.documents.project.regions[0].map.id;
        editor.documents.project.regions[0].map.camera = MapCamera::ThreeDIso;
        editor.server_ctx.curr_region = region;
        editor.server_ctx.pc = ProjectContext::Region(region);
        editor.server_ctx.editor_view_mode = EditorViewMode::Iso;
        editor.native_viewport.enabled = true;
        editor.server_ctx.reference.bind_context(gpu.clone());
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(256, 256, 1.);
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::sized(128, 128), &mut ctx);
        ui.canvas.set_widget(view);
        editor.server_ctx.bake.write().request_render(map, 1);
        editor.update_native_bake(&mut ui, &mut ctx);
        let job = editor
            .native_viewport
            .world
            .bake_jobs
            .active
            .as_ref()
            .unwrap();
        let camera = job.frame.camera;
        assert_eq!(job.viewport, (128, 128));
        assert!(job.gpu.is_none());
        let revision = editor.server_ctx.history.read().revision();
        // Simulate the producer/controller handoff immediately before its next
        // update publication. No shader or requested GPU capture occurs here.
        let mut bake = editor.server_ctx.bake.write();
        let work = bake
            .prepare_work(map, 128, 128, camera, Some((-0.25, 0.25, -0.25, 0.25)))
            .unwrap()
            .unwrap();
        assert!(bake.finish_work_sample(&work).unwrap());
        let pixels = (work.tile_size * work.tile_size) as usize;
        assert!(
            bake.commit_work(
                &work,
                vec![180; pixels * 4],
                vec![0.; pixels],
                vec![100; pixels * 4],
                vec![128; pixels * 4],
                vec![0; pixels * 4]
            )
            .unwrap()
        );
        assert!(!bake.is_rendering());
        drop(bake);
        assert!(
            editor.documents.project.regions[0]
                .map
                .orthographic_bake
                .is_none()
        );
        if phase == 1 {
            // A widget may disappear entirely while its owning document lives.
            ui = TheUI::new();
        } else {
            ui.get_widget("PolyView")
                .unwrap()
                .set_dim(TheDim::sized(0, 0), &mut ctx);
        }
        match phase {
            2 => editor.documents.project.regions[0].map.changed += 1,
            3 => editor.documents.project.regions.clear(),
            4 => editor
                .documents
                .install(editor.documents.project.clone(), None, false),
            5 => editor.server_ctx.editor_view_mode = EditorViewMode::D2,
            _ => (),
        }
        editor.update_native_bake(&mut ui, &mut ctx);
        assert!(editor.native_viewport.world.bake_jobs.active.is_none());
        assert!(
            editor
                .server_ctx
                .bake
                .write()
                .take_persisted_update_for(map)
                .is_none(),
            "a completion must be either published or retired under its owner"
        );
        if phase < 2 {
            assert!(
                editor.documents.project.regions[0]
                    .map
                    .orthographic_bake
                    .is_some(),
                "layout availability cannot discard valid asset publication"
            );
            assert!(editor.server_ctx.history.read().revision() > revision);
            assert!(editor.documents.sessions[editor.documents.active_session].dirty);
        } else if let Some(region) = editor.documents.project.regions.first() {
            assert!(
                region.map.orthographic_bake.is_none(),
                "retired source cannot enter authored data"
            );
            assert_eq!(editor.server_ctx.history.read().revision(), revision);
        }
    }
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert_eq!(gpu.capture_queue_usage().requests, 0);
}
