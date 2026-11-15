use super::*;
use crate::{dockmanager::DockManagerState, sidebar::Sidebar};
struct Plain;
impl Dock for Plain {
    fn new() -> Self {
        Self
    }
    fn setup(&mut self, _: &mut TheContext) -> TheCanvas {
        TheCanvas::new()
    }
}
fn fixture() -> (Sidebar, Project, CreatorContext, TheUI, TheContext) {
    let project = Project::default();
    let mut server = CreatorContext::new();
    {
        let mut docks = server.docks.write();
        docks
            .docks
            .insert("Native minimap policy".into(), Box::new(Plain));
        docks.index = docks.docks.get_index_of("Native minimap policy").unwrap();
        docks.dock = "Native minimap policy".into();
        docks.state = DockManagerState::Minimized;
    }
    server.shared.curr_region = project.regions[0].id;
    let mut ctx = TheContext::new(143, 83, 1.);
    ctx.ui.command_paint = true;
    let mut ui = TheUI::new();
    let mut view = TheRenderView::new(TheId::named("MiniMap"));
    view.set_dim(TheDim::sized(143, 83), &mut ctx);
    ui.canvas.set_widget(view);
    (Sidebar::new(), project, server, ui, ctx)
}
#[test]
fn native_screen_minimap_uses_reference_map_policy_across_view_modes() {
    let (mut sidebar, mut project, mut server, mut ui, mut ctx) = fixture();
    let screen = shared::screen::Screen::new();
    let id = screen.id;
    project.screens.insert(id, screen);
    server.shared.pc = ProjectContext::Screen(id);
    let authored = serde_json::to_value(&project).unwrap();
    for mode in [
        EditorViewMode::D2,
        EditorViewMode::FirstP,
        EditorViewMode::Orbit,
        EditorViewMode::Iso,
    ] {
        server.shared.editor_view_mode = mode;
        let expected = project.get_map(&server.shared).unwrap().id;
        let actual = native_scene::overview_map(&project, &server).unwrap().0.id;
        assert_eq!(
            actual, expected,
            "screen overview follows the existing parent-map selection"
        );
        sidebar.handle_event(
            &TheEvent::Custom(TheId::named("Update Minimap"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert!(
            !ui.get_render_view("MiniMap")
                .unwrap()
                .has_cpu_display_storage()
        );
        assert!(
            !server.reference.initialized(),
            "native source publication must keep the reference renderer cold"
        );
    }
    assert_eq!(serde_json::to_value(project).unwrap(), authored);
}
#[test]
fn native_minimap_missing_sources_and_empty_main_resize_allocate_no_display_pixels() {
    let (mut sidebar, mut project, mut server, mut ui, mut ctx) = fixture();
    project.regions.clear();
    project.screens.clear();
    server.shared.pc = ProjectContext::Screen(Uuid::new_v4());
    sidebar.handle_event(
        &TheEvent::Custom(TheId::named("Update Minimap"), TheValue::Empty),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert!(
        !ui.get_render_view("MiniMap")
            .unwrap()
            .has_cpu_display_storage()
    );
    ctx.ui.add_icon(
        "eldiron".into(),
        TheRGBABuffer::from([120, 90, 50, 255].repeat(4), 2, 2),
    );
    let mut view = TheRenderView::new(TheId::named("PolyView"));
    let id = view.id().clone();
    view.set_dim(TheDim::sized(143, 83), &mut ctx);
    ui.canvas.set_widget(view);
    sidebar.handle_event(
        &TheEvent::WidgetResized(id.clone(), TheDim::sized(143, 83)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert!(
        !ui.get_render_view("PolyView")
            .unwrap()
            .has_cpu_display_storage()
    );
    assert!(!server.reference.initialized());
    // The explicit CPU reference keeps its original logo display behavior.
    ctx.ui.command_paint = false;
    sidebar.handle_event(
        &TheEvent::WidgetResized(id, TheDim::sized(143, 83)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    let view = ui.get_render_view("PolyView").unwrap();
    assert!(view.has_cpu_display_storage());
    assert!(
        view.render_buffer_mut()
            .pixels()
            .chunks_exact(4)
            .any(|p| p == [120, 90, 50, 255])
    );
}
