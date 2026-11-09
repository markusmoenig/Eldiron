//! Opt-in native documentation captures. Never opens or saves a user session.
use super::*;

const WIDTH: usize = 1440;
const HEIGHT: usize = 960;

struct Capture {
    editor: Editor,
    ui: TheUI,
    ctx: TheContext,
    destination: PathBuf,
}

impl Capture {
    fn new(destination: PathBuf) -> Self {
        let mut ctx = TheContext::new(WIDTH, HEIGHT, 1.0);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        ui.canvas.root = true;
        ui.canvas
            .set_dim(TheDim::sized(WIDTH as i32, HEIGHT as i32), &mut ctx);
        let mut editor = Editor::new();
        editor.init_ui(&mut ui, &mut ctx);
        CreatorTheme::Brown.apply(&mut ui, &mut ctx);
        std::fs::create_dir_all(&destination).unwrap();
        Self {
            editor,
            ui,
            ctx,
            destination,
        }
    }

    fn open(&mut self, project: Project, path: &std::path::Path) {
        self.editor.open_project_as_session(
            project,
            Some(path.into()),
            &mut self.ui,
            &mut self.ctx,
            &mut false,
            &mut false,
        );
    }

    fn context(&mut self, context: ProjectContext) {
        set_project_context(
            &mut self.ctx,
            &mut self.ui,
            &mut self.editor.project,
            &mut self.editor.server_ctx,
            context,
        );
    }

    fn tool(&mut self, command: &str) {
        self.editor
            .sidebar
            .execute_tool_command(
                command,
                &mut self.ui,
                &mut self.ctx,
                &mut self.editor.project,
                &mut self.editor.server_ctx,
            )
            .unwrap();
    }

    fn save(&mut self, name: &str) {
        self.ui
            .canvas
            .layout(WIDTH as i32, HEIGHT as i32, &mut self.ctx);
        if DOCKMANAGER.read().unwrap().dock == "Nodes" {
            self.ctx.ui.send(TheEvent::StateChanged(
                TheId::named("Node Graph Branch Tidy"),
                TheWidgetState::Clicked,
            ));
        }
        let mut frame = vec![0; WIDTH * HEIGHT * 4];
        // Drain normal UI events and asynchronous scene builds before capturing.
        for _ in 0..24 {
            self.editor.update_tracker = crate::misc::UpdateTracker::new();
            self.ctx.ui.redraw_all = true;
            self.ui.update(&mut self.ctx);
            self.editor.update_ui(&mut self.ui, &mut self.ctx);
            self.ui.draw(&mut frame, &mut self.ctx);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        std::fs::write(
            self.destination.join(format!("{name}.png")),
            TheRGBABuffer::from(frame, WIDTH as u32, HEIGHT as u32)
                .to_png()
                .unwrap(),
        )
        .unwrap();
        eprintln!("Captured {name}");
    }

    fn center(&mut self, position: Vec2<f32>, zoom: f32) {
        self.ui
            .canvas
            .layout(WIDTH as i32, HEIGHT as i32, &mut self.ctx);
        let dim = *self.ui.get_render_view("PolyView").unwrap().dim();
        let map = self
            .editor
            .project
            .get_map_mut(&self.editor.server_ctx)
            .unwrap();
        map.grid_size = zoom;
        self.editor.server_ctx.center_map_at_grid_pos(
            Vec2::new(dim.width as f32, dim.height as f32),
            position,
            map,
        );
        crate::utils::editor_scene_full_rebuild(&self.editor.project, &self.editor.server_ctx);
    }
}

#[test]
#[ignore = "captures actual Creator UI; requires a native GPU"]
fn documentation_screenshots() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let destination = std::env::var_os("ELDIRON_DOC_CAPTURES")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("generated/docs-current"));
    let mut capture = Capture::new(destination);
    if std::env::var_os("ELDIRON_DOC_3D_ONLY").is_some() {
        capture_3d(&mut capture, &root);
        return;
    }
    let hideout_path = root.join("starters/projects/Hideout2D.eldiron");
    let mut hideout = Editor::load_project_from_path(&hideout_path).unwrap();
    hideout.regions[0].map.camera = MapCamera::TwoD;
    capture.open(hideout, &hideout_path);
    let region = capture.editor.project.regions[0].id;
    capture.context(ProjectContext::Region(region));
    capture.tool("tool.rect");
    capture.save("creator-overview");

    let tile = Uuid::parse_str("a334a1b7-7ee4-4028-9d84-84a91fe1ee1a").unwrap();
    assert!(capture.editor.project.tiles.contains_key(&tile));
    {
        capture.editor.server_ctx.curr_tile_id = Some(tile);
        capture.editor.server_ctx.curr_tile_source = Some(rusterix::TileSource::SingleTile(tile));
        capture.ctx.ui.send(TheEvent::Custom(
            TheId::named("Tile Picked"),
            TheValue::Id(tile),
        ));
    }
    capture.save("rect-tool");
    let action = ACTIONLIST
        .read()
        .unwrap()
        .get_action_by_command_id("tile.edit_metadata")
        .map(|a| a.id().clone());
    if let Some(action) = action {
        capture
            .ctx
            .ui
            .send(TheEvent::StateChanged(action, TheWidgetState::Selected));
        capture.editor.sidebar.show_actions_page(
            &mut capture.ui,
            &mut capture.ctx,
            &mut capture.editor.project,
            &mut capture.editor.server_ctx,
        );
        capture.save("tile-metadata");
    }

    if let Some(tilemap) = capture.editor.project.tilemaps.first().map(|t| t.id) {
        capture.context(ProjectContext::Tilemap(tilemap));
        capture.save("tileset-editor");
    }
    if let Some(character) = capture
        .editor
        .project
        .characters
        .values()
        .find(|c| c.name == "Player")
        .or_else(|| capture.editor.project.characters.values().next())
        .map(|c| c.id)
    {
        capture.context(ProjectContext::CharacterData(character));
        DOCKMANAGER.write().unwrap().edit_maximize(
            &mut capture.ui,
            &mut capture.ctx,
            &mut capture.editor.project,
            &mut capture.editor.server_ctx,
        );
        capture.save("character-attributes");
        capture.context(ProjectContext::CharacterCode(character));
        capture.save("behavior-nodes");
        DOCKMANAGER.write().unwrap().minimize(
            &mut capture.ui,
            &mut capture.ctx,
            &mut capture.editor.project,
            &mut capture.editor.server_ctx,
        );
    }
    capture.context(ProjectContext::ProjectSettings);
    DOCKMANAGER.write().unwrap().edit_maximize(
        &mut capture.ui,
        &mut capture.ctx,
        &mut capture.editor.project,
        &mut capture.editor.server_ctx,
    );
    capture.save("game-settings");
    DOCKMANAGER.write().unwrap().minimize(
        &mut capture.ui,
        &mut capture.ctx,
        &capture.editor.project,
        &mut capture.editor.server_ctx,
    );
    capture.context(ProjectContext::Region(region));
    if let Some(garden) = capture.editor.project.regions[0]
        .map
        .sectors
        .iter()
        .find(|s| s.name == "Garden")
        .map(|s| s.id)
    {
        capture.editor.project.regions[0].map.selected_sectors = vec![garden];
    }
    capture.ctx.ui.send(TheEvent::StateChanged(
        TheId::named("Authoring"),
        TheWidgetState::Clicked,
    ));
    capture.save("authoring");
    capture.ctx.ui.send(TheEvent::StateChanged(
        TheId::named("Authoring"),
        TheWidgetState::Clicked,
    ));
    let screen = capture
        .editor
        .project
        .screens
        .values()
        .find(|s| s.name == "Game")
        .map(|s| s.id)
        .unwrap();
    capture.context(ProjectContext::Screen(screen));
    capture.editor.sidebar.apply_screen(
        &mut capture.ui,
        &mut capture.ctx,
        capture.editor.project.screens.get(&screen),
    );
    capture.tool("tool.sector");
    if let Some(widget) = capture.editor.project.screens[&screen]
        .map
        .sectors
        .iter()
        .find(|s| !s.name.is_empty())
        .map(|s| s.creator_id)
    {
        capture.context(ProjectContext::ScreenWidget(screen, widget));
    }
    capture.save("screen-editor");

    // The geometry guide uses a small, reproducible polygon rather than a
    // screenshot of an unrelated finished level.
    let mut project = Editor::load_project_from_path(&hideout_path).unwrap();
    project.name = "Geometry Guide".into();
    project.regions.truncate(1);
    project.regions[0].characters.clear();
    project.regions[0].items.clear();
    let map = &mut project.regions[0].map;
    *map = Map::default();
    let vertices: Vec<_> = [(0., 0.), (6., 0.), (6., 4.), (0., 4.)]
        .into_iter()
        .map(|(x, y)| map.add_vertex_at(x, y))
        .collect();
    for i in 0..4 {
        map.create_linedef(vertices[i], vertices[(i + 1) % 4]);
    }
    let sector = map.sectors[0].id;
    capture.open(
        project,
        &root.join("generated/docs-current/GeometryGuide.eldiron"),
    );
    let region = capture.editor.project.regions[0].id;
    capture.context(ProjectContext::Region(region));
    capture.tool("tool.linedef");
    capture.center(Vec2::new(3., 2.), 64.);
    capture.save("linedef-tool");
    capture.tool("tool.sector");
    capture.editor.project.regions[0].map.selected_sectors = vec![sector];
    capture.save("selected-sector");
    capture.editor.project.regions[0].map.sectors[0]
        .properties
        .set("source", Value::Source(rusterix::PixelSource::TileId(tile)));
    crate::utils::editor_scene_full_rebuild(&capture.editor.project, &capture.editor.server_ctx);
    capture.save("applied-tile");

    capture_3d(&mut capture, &root);
}

fn capture_3d(capture: &mut Capture, root: &std::path::Path) {
    let stonefall_path = root.join("starters/projects/StonefallDungeon.eldiron");
    let mut stonefall = Editor::load_project_from_path(&stonefall_path).unwrap();
    stonefall.regions[0].map.camera = MapCamera::ThreeDIso;
    capture.open(stonefall, &stonefall_path);
    let region = capture.editor.project.regions[0].id;
    capture.context(ProjectContext::Region(region));
    capture.tool("tool.geometry");
    // These are ordinary editor preview controls, not changes to the project.
    capture.editor.server_ctx.show_editor_grid = false;
    {
        let mut runtime = RUSTERIX.write().unwrap();
        runtime.editor_preview_lighting_enabled = true;
        runtime.editor_preview_fill_strength = 1.0;
        runtime.editor_preview_post_enabled = false;
        runtime.set_dirty();
    }
    TOOLLIST
        .write()
        .unwrap()
        .update_geometry_overlay_3d(&mut capture.editor.project, &mut capture.editor.server_ctx);
    {
        let region = &mut capture.editor.project.regions[0];
        let mut lo = Vec3::broadcast(f32::INFINITY);
        let mut hi = Vec3::broadcast(f32::NEG_INFINITY);
        for p in region
            .map
            .geometry_objects
            .iter()
            .flat_map(|o| o.vertices.iter().map(|p| o.transform_point(*p)))
        {
            lo = lo.map2(p, f32::min);
            hi = hi.map2(p, f32::max);
        }
        let center = (lo + hi) * 0.5;
        region.editing_position_3d = center;
        region.editing_look_at_3d = center;
        EDITCAMERA.write().unwrap().iso_camera.scale = ((hi.x - lo.x + hi.z - lo.z) * 0.4).max(4.);
    }
    capture.save("3d-editor");
    capture.tool("tool.iso_paint");
    capture.save("3d-paint");
    capture.tool("tool.blocks");
    capture.save("prefab-tool");
}
