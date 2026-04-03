use crate::Embedded;
use crate::prelude::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
use crate::self_update::{SelfUpdateEvent, SelfUpdater};
use codegridfx::Module;
use rusterix::render_settings::RendererBackend;
use rusterix::server::message::AudioCommand;
use rusterix::{
    PlayerCamera, Rusterix, SceneManager, SceneManagerResult, Texture, Value, ValueContainer,
};
use shared::rusterix_utils::*;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::Receiver;
#[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
use std::sync::{
    Arc, Mutex,
    mpsc::{Sender, channel},
};

#[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
use std::thread;

pub static PREVIEW_ICON: LazyLock<RwLock<(TheRGBATile, i32)>> =
    LazyLock::new(|| RwLock::new((TheRGBATile::default(), 0)));

pub static SIDEBARMODE: LazyLock<RwLock<SidebarMode>> =
    LazyLock::new(|| RwLock::new(SidebarMode::Region));
pub static UNDOMANAGER: LazyLock<RwLock<UndoManager>> =
    LazyLock::new(|| RwLock::new(UndoManager::default()));
pub static TOOLLIST: LazyLock<RwLock<ToolList>> =
    LazyLock::new(|| RwLock::new(ToolList::default()));
pub static ACTIONLIST: LazyLock<RwLock<ActionList>> =
    LazyLock::new(|| RwLock::new(ActionList::default()));
// pub static PANELS: LazyLock<RwLock<Panels>> = LazyLock::new(|| RwLock::new(Panels::new()));
pub static PALETTE: LazyLock<RwLock<ThePalette>> =
    LazyLock::new(|| RwLock::new(ThePalette::default()));
pub static RUSTERIX: LazyLock<RwLock<Rusterix>> =
    LazyLock::new(|| RwLock::new(Rusterix::default()));
pub static CONFIGEDITOR: LazyLock<RwLock<ConfigEditor>> =
    LazyLock::new(|| RwLock::new(ConfigEditor::new()));
pub static CONFIG: LazyLock<RwLock<toml::Table>> =
    LazyLock::new(|| RwLock::new(toml::Table::default()));
pub static EDITCAMERA: LazyLock<RwLock<EditCamera>> =
    LazyLock::new(|| RwLock::new(EditCamera::new()));
pub static SCENEMANAGER: LazyLock<RwLock<SceneManager>> =
    LazyLock::new(|| RwLock::new(SceneManager::default()));
pub static DOCKMANAGER: LazyLock<RwLock<DockManager>> =
    LazyLock::new(|| RwLock::new(DockManager::default()));
pub static TEXTGAME: LazyLock<RwLock<TextGameState>> =
    LazyLock::new(|| RwLock::new(TextGameState::default()));

pub static CODEGRIDFX: LazyLock<RwLock<Module>> =
    LazyLock::new(|| RwLock::new(Module::as_type(codegridfx::ModuleType::CharacterTemplate)));

#[derive(Clone)]
struct ProjectSession {
    project: Project,
    project_path: Option<PathBuf>,
    undo: UndoManager,
    dirty: bool,
}

#[derive(Deserialize, Clone)]
struct StarterProjectManifest {
    #[serde(default)]
    starter: Vec<StarterProjectManifestEntry>,
}

#[derive(Deserialize, Clone)]
struct StarterProjectManifestEntry {
    id: String,
    title: String,
    description: String,
    project_path: String,
    image: String,
}

#[derive(Clone)]
struct StarterProjectEntry {
    id: Uuid,
    manifest_id: String,
    title: String,
    description: String,
    project_path: String,
    preview: Option<TheRGBATile>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct CreatorWindowState {
    x: Option<i32>,
    y: Option<i32>,
    width: Option<usize>,
    height: Option<usize>,
}

pub struct Editor {
    project: Project,
    project_path: Option<PathBuf>,
    sessions: Vec<ProjectSession>,
    active_session: usize,
    replace_next_project_load_in_active_tab: bool,
    last_active_dirty: bool,

    sidebar: Sidebar,
    mapeditor: MapEditor,

    server_ctx: ServerContext,

    update_tracker: UpdateTracker,
    event_receiver: Option<Receiver<TheEvent>>,

    #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
    self_update_rx: Receiver<SelfUpdateEvent>,
    #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
    self_update_tx: Sender<SelfUpdateEvent>,
    #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
    self_updater: Arc<Mutex<SelfUpdater>>,

    update_counter: usize,
    last_processed_log_len: usize,
    pending_game_messages: Vec<rusterix::server::Message>,
    pending_game_says: Vec<TextGameSay>,
    pending_game_choices: Vec<rusterix::MultipleChoice>,
    pending_text_game_command: Option<(String, String)>,
    pending_text_game_runtime_flush: bool,

    build_values: ValueContainer,
    window_state: CreatorWindowState,
    starter_projects: Vec<StarterProjectEntry>,
    starter_project_cache: HashMap<String, Project>,
    starter_manifest_cache: Option<Vec<StarterProjectEntry>>,
    starter_loader_rx: Option<Receiver<Vec<StarterProjectEntry>>>,
    selected_starter_manifest_id: Option<String>,
}

impl Editor {
    const STARTER_REPO_RAW_BASE: &'static str =
        "https://raw.githubusercontent.com/markusmoenig/Eldiron/master/";
    const STARTER_DIALOG_TITLE: &'static str = "Choose Starter Project";
    const STARTER_LIST_ID: &'static str = "Starter Project List";
    const STARTER_PREVIEW_ID: &'static str = "Starter Project Preview";
    const STARTER_CREATE_ID: &'static str = "Starter Project Create";
    const STARTER_CANCEL_ID: &'static str = "Starter Project Cancel";

    fn log_segment_has_warning_or_error(segment: &str) -> bool {
        segment.contains("[error]") || segment.contains("[warning]")
    }

    fn starter_manifest_url() -> String {
        format!("{}starters/manifest.toml", Self::STARTER_REPO_RAW_BASE)
    }

    fn starter_repo_url(repo_path: &str) -> String {
        format!("{}{}", Self::STARTER_REPO_RAW_BASE, repo_path)
    }

    fn fetch_url_bytes(url: &str) -> Option<Vec<u8>> {
        if let Ok(response) = ureq::get(url).call() {
            let mut reader = response.into_reader();
            let mut bytes = Vec::new();
            if reader.read_to_end(&mut bytes).is_ok() {
                return Some(bytes);
            }
        }
        None
    }

    fn fetch_url_text(url: &str) -> Option<String> {
        let bytes = Self::fetch_url_bytes(url)?;
        String::from_utf8(bytes).ok()
    }

    fn refresh_system_text_clipboard(ctx: &mut TheContext) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(mut clipboard) = arboard::Clipboard::new()
                && let Ok(text) = clipboard.get_text()
            {
                ctx.ui.clipboard = Some(TheValue::Text(text));
                ctx.ui.clipboard_app_type = Some("text/plain".to_string());
            }
        }
    }

    fn load_project_from_json_path(path: &std::path::Path) -> Option<Project> {
        let contents = std::fs::read_to_string(path).ok()?;
        let mut loaded = serde_json::from_str::<Project>(&contents).ok()?;
        loaded.palette.current_index = 0;
        Some(loaded)
    }

    fn load_empty_project_template() -> Project {
        let mut project = Project::new();
        if let Some(bytes) = crate::Embedded::get("toml/config.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.config = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/rules.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.rules = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/locales.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.locales = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/audio_fx.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.audio_fx = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/authoring.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.authoring = source.to_string();
        }
        project
    }

    fn load_starter_manifest() -> Vec<StarterProjectEntry> {
        let contents = match Self::fetch_url_text(&Self::starter_manifest_url()) {
            Some(contents) => contents,
            None => return Vec::new(),
        };
        let manifest = match toml::from_str::<StarterProjectManifest>(&contents) {
            Ok(manifest) => manifest,
            Err(_) => return Vec::new(),
        };

        manifest
            .starter
            .into_iter()
            .map(|entry| StarterProjectEntry {
                id: Uuid::new_v4(),
                preview: Self::load_starter_preview(&entry.image),
                manifest_id: entry.id,
                title: entry.title,
                description: entry.description,
                project_path: entry.project_path,
            })
            .collect()
    }

    fn load_starter_preview(repo_path: &str) -> Option<TheRGBATile> {
        let bytes = Self::fetch_url_bytes(&Self::starter_repo_url(repo_path))?;
        Self::decode_png_tile(bytes)
    }

    fn decode_png_tile(bytes: Vec<u8>) -> Option<TheRGBATile> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().ok()?;
        let buffer_size = reader.output_buffer_size()?;
        let mut buf = vec![0; buffer_size];
        let info = reader.next_frame(&mut buf).ok()?;
        let bytes = &buf[..info.buffer_size()];
        Some(TheRGBATile::buffer(TheRGBABuffer::from(
            bytes.to_vec(),
            info.width,
            info.height,
        )))
    }

    fn load_named_starter_project(&mut self, manifest_id: &str) -> Option<Project> {
        if let Some(project) = self.starter_project_cache.get(manifest_id).cloned() {
            return Some(project);
        }

        let choice = self
            .starter_manifest_cache
            .clone()
            .unwrap_or_else(|| self.starter_projects.clone())
            .into_iter()
            .find(|choice| choice.manifest_id == manifest_id)?;
        let contents = Self::fetch_url_text(&Self::starter_repo_url(&choice.project_path))?;
        let mut loaded = serde_json::from_str::<Project>(&contents).ok()?;
        loaded.palette.current_index = 0;
        self.starter_project_cache
            .insert(manifest_id.to_string(), loaded.clone());
        Some(loaded)
    }

    fn window_state_file_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".eldiron")
                .join("creator_window_state.json"),
        )
    }

    fn load_window_state() -> CreatorWindowState {
        if let Some(path) = Self::window_state_file_path()
            && let Ok(data) = fs::read_to_string(path)
            && let Ok(state) = serde_json::from_str::<CreatorWindowState>(&data)
        {
            return state;
        }
        CreatorWindowState::default()
    }

    fn save_window_state(&self) {
        if let Some(path) = Self::window_state_file_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string(&self.window_state) {
                let _ = fs::write(path, json);
            }
        }
    }

    fn persist_active_region_view_state(&mut self) {
        if let Some(region) = self.project.get_region_mut(&self.server_ctx.curr_region) {
            match self.server_ctx.editor_view_mode {
                EditorViewMode::Iso => {
                    region.editing_position_iso_3d = Some(region.editing_position_3d);
                    region.editing_look_at_iso_3d = Some(region.editing_look_at_3d);
                    region.editing_iso_scale = Some(EDITCAMERA.read().unwrap().iso_camera.scale);
                }
                EditorViewMode::Orbit => {
                    region.editing_position_orbit_3d = Some(region.editing_position_3d);
                    region.editing_look_at_orbit_3d = Some(region.editing_look_at_3d);
                    region.editing_orbit_distance =
                        Some(EDITCAMERA.read().unwrap().orbit_camera.distance);
                }
                EditorViewMode::FirstP => {
                    region.editing_position_firstp_3d = Some(region.editing_position_3d);
                    region.editing_look_at_firstp_3d = Some(region.editing_look_at_3d);
                }
                EditorViewMode::D2 => {}
            }
        }
    }

    fn project_tab_title_for(
        project: &Project,
        project_path: &Option<PathBuf>,
        fallback_index: usize,
        dirty: bool,
    ) -> String {
        let prefix = if dirty { "* " } else { "" };

        if let Some(path) = project_path
            && let Some(stem) = path.file_stem()
            && let Some(name) = stem.to_str()
            && !name.is_empty()
        {
            return format!("{}{}", prefix, name);
        }
        if !project.name.is_empty() {
            return format!("{}{}", prefix, project.name);
        }

        if project_path.is_none() {
            return format!("{}{}", prefix, fl!("new_project"));
        }

        format!("{}Project {}", prefix, fallback_index + 1)
    }

    fn sync_active_session_from_editor(&mut self) {
        if self.active_session >= self.sessions.len() {
            return;
        }
        self.persist_active_region_view_state();
        self.sessions[self.active_session].project = self.project.clone();
        self.sessions[self.active_session].project_path = self.project_path.clone();
        self.sessions[self.active_session].undo = UNDOMANAGER.read().unwrap().clone();
        self.sessions[self.active_session].dirty = self.active_session_has_changes();
    }

    fn sync_editor_from_active_session(&mut self) {
        if self.active_session >= self.sessions.len() {
            return;
        }
        let session = self.sessions[self.active_session].clone();
        self.project = session.project;
        self.project_path = session.project_path;
        *UNDOMANAGER.write().unwrap() = session.undo;
    }

    fn rebuild_project_tabs(&self, ui: &mut TheUI) {
        if let Some(widget) = ui.get_widget("Project Tabs")
            && let Some(tabbar) = widget.as_tabbar()
        {
            tabbar.clear();
            for (index, session) in self.sessions.iter().enumerate() {
                tabbar.add_tab(Self::project_tab_title_for(
                    &session.project,
                    &session.project_path,
                    index,
                    session.dirty,
                ));
            }
            tabbar.set_selection_index(self.active_session);
        }
    }

    fn open_starter_project_dialog(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.starter_loader_rx = None;
        self.selected_starter_manifest_id = None;

        let width = 980;
        let height = 340;
        let bottom_bar_height = 32;
        let preview_size = height;

        let mut dialog = TheCanvas::new();
        dialog.limiter_mut().set_max_size(Vec2::new(width, height));

        let mut left = TheCanvas::new();
        left.limiter_mut()
            .set_max_size(Vec2::new(preview_size, preview_size));
        let mut preview = TheIconView::new(TheId::named(Self::STARTER_PREVIEW_ID));
        preview
            .limiter_mut()
            .set_max_size(Vec2::new(preview_size, preview_size));
        preview.set_border_color(Some([120, 120, 120, 255]));
        preview.set_background_color(Some([218, 211, 177, 255]));
        preview.set_alpha_mode(true);
        if let Some(tile) = ctx.ui.icon("lord").cloned().map(TheRGBATile::buffer) {
            preview.set_rgba_tile(tile);
        }
        left.set_widget(preview);
        dialog.set_left(left);

        let mut center = TheCanvas::new();
        center
            .limiter_mut()
            .set_max_size(Vec2::new(width - (preview_size + 20), preview_size));
        let mut list = TheListLayout::new(TheId::named(Self::STARTER_LIST_ID));
        list.set_item_size(52);
        let mut item = TheListItem::new(TheId::named("Starter Project Loading"));
        item.set_text("Loading starter projects...".to_string());
        item.set_sub_text("Fetching metadata from the Eldiron repo.".to_string());
        item.set_size(52);
        item.set_text_color(WHITE);
        item.set_text_size(14.0);
        item.set_sub_text_size(12.0);
        list.add_item(item, ctx);
        center.set_layout(list);
        dialog.set_center(center);

        let mut bottom = TheCanvas::new();
        bottom
            .limiter_mut()
            .set_max_size(Vec2::new(width, bottom_bar_height));
        let mut actions = TheHLayout::new(TheId::named("Starter Project Actions"));
        actions
            .limiter_mut()
            .set_max_size(Vec2::new(width, bottom_bar_height));
        actions.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        actions.set_margin(Vec4::new(10, 2, 10, 2));
        actions.set_padding(8);
        actions.set_reverse_index(Some(2));

        let mut create = TheTraybarButton::new(TheId::named(Self::STARTER_CREATE_ID));
        create.set_text("Choose".to_string());
        actions.add_widget(Box::new(create));

        let mut cancel = TheTraybarButton::new(TheId::named(Self::STARTER_CANCEL_ID));
        cancel.set_text("Cancel".to_string());
        actions.add_widget(Box::new(cancel));

        bottom.set_layout(actions);
        dialog.set_bottom(bottom);

        ui.show_dialog(Self::STARTER_DIALOG_TITLE, dialog, vec![], ctx);
        if let Some(starters) = self.starter_manifest_cache.clone() {
            self.starter_projects = starters;
            self.rebuild_starter_project_list(ui, ctx);
            if let Some(first) = self.starter_projects.first() {
                self.selected_starter_manifest_id = Some(first.manifest_id.clone());
                ctx.ui.send(TheEvent::StateChanged(
                    TheId::named_with_id("Starter Project List Item", first.id),
                    TheWidgetState::Selected,
                ));
                ui.set_enabled(Self::STARTER_CREATE_ID, ctx);
            } else {
                ui.set_disabled(Self::STARTER_CREATE_ID, ctx);
            }
        } else {
            self.starter_projects.clear();
            ui.set_disabled(Self::STARTER_CREATE_ID, ctx);

            let (tx, rx) = std::sync::mpsc::channel();
            self.starter_loader_rx = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(Self::load_starter_manifest());
            });
        }
    }

    fn rebuild_starter_project_list(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(list) = ui.get_list_layout(Self::STARTER_LIST_ID) {
            list.clear();
            list.set_item_size(52);
            for (index, entry) in self.starter_projects.iter().enumerate() {
                let mut item =
                    TheListItem::new(TheId::named_with_id("Starter Project List Item", entry.id));
                item.set_text(entry.title.clone());
                item.set_sub_text(entry.description.clone());
                item.set_size(52);
                item.set_text_color(WHITE);
                item.set_text_size(14.0);
                item.set_sub_text_size(12.0);
                if index == 0 {
                    item.set_state(TheWidgetState::Selected);
                }
                if let Some(preview) = &entry.preview
                    && let Some(first) = preview.buffer.first()
                {
                    item.set_icon(first.clone());
                }
                list.add_item(item, ctx);
            }
        }
    }

    fn open_project_as_session(
        &mut self,
        mut project: Project,
        project_path: Option<PathBuf>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        Self::sanitize_loaded_project(&mut project);

        self.sync_active_session_from_editor();
        let new_index = if self.replace_next_project_load_in_active_tab {
            self.sessions[self.active_session] = ProjectSession {
                project,
                project_path,
                undo: UndoManager::default(),
                dirty: false,
            };
            self.replace_next_project_load_in_active_tab = false;
            self.active_session
        } else {
            self.sessions.push(ProjectSession {
                project,
                project_path,
                undo: UndoManager::default(),
                dirty: false,
            });
            self.sessions.len() - 1
        };
        self.switch_to_session(new_index, ui, ctx, update_server_icons, redraw);
    }

    fn activate_loaded_project(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        self.update_counter = 0;
        self.sidebar.startup = true;

        if let Some(widget) = ui.get_widget("Server Time Slider") {
            widget.set_value(TheValue::Time(self.project.time));
        }

        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.client.set_server_time(self.project.time);
            if rusterix.server.state == rusterix::ServerState::Running
                && let Some(map) = self.project.get_map(&self.server_ctx)
            {
                rusterix.server.set_time(&map.id, self.project.time);
            }
        }

        self.server_ctx.clear();
        self.server_ctx.text_game_mode = TOOLLIST.read().unwrap().text_game_mode;
        TEXTGAME.write().unwrap().reset();
        if let Some(first) = self.project.regions.first() {
            self.server_ctx.curr_region = first.id;
        }
        let restored_view_index = self
            .project
            .get_region(&self.server_ctx.curr_region)
            .map(|region| match region.map.camera {
                MapCamera::TwoD => 0,
                MapCamera::ThreeDIso => 2,
                MapCamera::ThreeDFirstPerson => 3,
            })
            .unwrap_or(0);
        self.server_ctx.editor_view_mode = EditorViewMode::from_index(restored_view_index);
        let restored_camera_action_name = match restored_view_index {
            2 => fl!("action_iso_camera"),
            3 => fl!("action_first_p_camera"),
            _ => fl!("action_editing_camera"),
        };

        self.sidebar
            .load_from_project(ui, ctx, &mut self.server_ctx, &mut self.project);
        self.mapeditor.load_from_project(ui, ctx, &self.project);
        if let Some(widget) = ui.get_widget("Editor View Switch")
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(restored_view_index);
        }
        {
            let mut actions = ACTIONLIST.write().unwrap();
            if let Some(action) = actions
                .actions
                .iter_mut()
                .find(|action| action.id().name == restored_camera_action_name)
            {
                self.server_ctx.curr_action_id = Some(action.id().uuid);
                if let Some(map) = self.project.get_map_mut(&self.server_ctx) {
                    action.load_params(map);
                    let _ = action.apply(map, ui, ctx, &mut self.server_ctx);
                }
                action.load_params_project(&self.project, &mut self.server_ctx);
                action.apply_project(&mut self.project, ui, ctx, &mut self.server_ctx);
            }
        }
        *update_server_icons = true;
        *redraw = true;

        *PALETTE.write().unwrap() = self.project.palette.clone();
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.assets.palette = self.project.palette.clone();
            rusterix.set_tiles(self.project.tiles.clone(), true);
        }
        SCENEMANAGER
            .write()
            .unwrap()
            .set_palette(self.project.palette.clone());

        UNDOMANAGER.read().unwrap().set_undo_state_to_ui(ctx);
    }

    fn switch_to_session(
        &mut self,
        index: usize,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        if index >= self.sessions.len() {
            self.rebuild_project_tabs(ui);
            return;
        }
        if index == self.active_session {
            self.sync_editor_from_active_session();
            self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
            self.rebuild_project_tabs(ui);
            return;
        }
        self.sync_active_session_from_editor();
        self.active_session = index;
        self.sync_editor_from_active_session();
        self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
        self.rebuild_project_tabs(ui);
    }

    fn sanitize_loaded_project(project: &mut Project) {
        insert_content_into_maps(project);

        let mut char_names = FxHashMap::default();
        for c in &project.characters {
            char_names.insert(c.0, c.1.name.clone());
        }
        for r in &mut project.regions {
            for c in &mut r.characters {
                if let Some(n) = char_names.get(&c.1.character_id) {
                    c.1.name = n.clone();
                }
            }
        }

        let mut item_names = FxHashMap::default();
        for c in &project.items {
            item_names.insert(c.0, c.1.name.clone());
        }
        for r in &mut project.regions {
            for c in &mut r.items {
                if let Some(n) = item_names.get(&c.1.item_id) {
                    c.1.name = n.clone();
                }
            }
            for (_, p) in &mut r.map.profiles {
                p.sanitize();
            }
            r.map.sanitize();
        }

        for (_, screen) in &mut project.screens {
            screen.map.sanitize();
        }

        if project.tiles.is_empty() {
            let tiles = project.extract_tiles();
            for (id, t) in &tiles {
                let mut texture_array: Vec<Texture> = vec![];
                for b in &t.buffer {
                    let mut texture = Texture::new(
                        b.pixels().to_vec(),
                        b.dim().width as usize,
                        b.dim().height as usize,
                    );
                    texture.generate_normals(true);
                    texture_array.push(texture);
                }
                let tile = rusterix::Tile {
                    id: t.id,
                    role: rusterix::TileRole::from_index(t.role),
                    textures: texture_array.clone(),
                    module: None,
                    blocking: t.blocking,
                    scale: t.scale,
                    tags: t.name.clone(),
                    particle_emitter: None,
                    light_emitter: None,
                };
                project.tiles.insert(*id, tile);
            }
        }

        for (_, tile) in project.tiles.iter_mut() {
            for texture in &mut tile.textures {
                if texture.data_ext.is_none() {
                    texture.generate_normals(true);
                }
            }
        }

        for (_, character) in project.characters.iter_mut() {
            if character.source.starts_with("class") {
                character.source = character.module.build(false);
                character.source_debug = character.module.build(true);
            }
        }

        for (_, item) in project.items.iter_mut() {
            if item.source.starts_with("class") {
                item.source = item.module.build(false);
                item.source_debug = item.module.build(true);
            }
        }
    }

    fn close_active_session(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        if self.sessions.is_empty() {
            return;
        }

        self.sync_active_session_from_editor();
        self.sessions.remove(self.active_session);

        if self.sessions.is_empty() {
            let project = Self::load_empty_project_template();
            self.sessions.push(ProjectSession {
                project,
                project_path: None,
                undo: UndoManager::default(),
                dirty: false,
            });
            self.active_session = 0;
        } else if self.active_session >= self.sessions.len() {
            self.active_session = self.sessions.len() - 1;
        }

        self.sync_editor_from_active_session();
        self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
        self.rebuild_project_tabs(ui);
        if self.sessions.len() == 1 && self.project_path.is_none() {
            self.replace_next_project_load_in_active_tab = true;
            self.open_starter_project_dialog(ui, ctx);
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Choose a 2D or 3D starter project.".to_string(),
            ));
            *redraw = true;
        }
    }

    fn active_session_has_changes(&self) -> bool {
        UNDOMANAGER.read().unwrap().has_unsaved() || DOCKMANAGER.read().unwrap().has_dock_changes()
    }

    fn is_realtime_mode(&self) -> bool {
        self.server_ctx.game_mode
            || RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running
    }

    fn redraw_interval_ms(&self) -> u64 {
        let config = CONFIGEDITOR.read().unwrap();
        if self.is_realtime_mode() {
            (1000 / config.target_fps.clamp(1, 60)) as u64
        } else {
            config.game_tick_ms.max(1) as u64
        }
    }

    fn help_url_for_data_context(&self) -> String {
        match self.server_ctx.pc {
            ProjectContext::ProjectSettings => "docs/configuration/game".to_string(),
            ProjectContext::GameRules | ProjectContext::GameLocales => "docs/rules".to_string(),
            ProjectContext::GameAudioFx => "docs/audio".to_string(),
            ProjectContext::GameAuthoring => "docs/creator/tools/overview".to_string(),
            ProjectContext::RegionSettings(_) => "docs/building_maps/region_settings".to_string(),
            ProjectContext::CharacterPreviewRigging(_) => "docs/characters_items/rigging".into(),
            ProjectContext::Character(_)
            | ProjectContext::CharacterData(_)
            | ProjectContext::Item(_)
            | ProjectContext::ItemData(_) => "docs/characters_items/attributes".to_string(),
            ProjectContext::Screen(_)
            | ProjectContext::ScreenWidget(_, _)
            | ProjectContext::RegionCharacterInstance(_, _)
            | ProjectContext::RegionItemInstance(_, _) => "docs/screens/widgets".to_string(),
            _ => "docs/creator/docks/attribute_editor".to_string(),
        }
    }

    fn help_url_for_widget_name(&self, widget_name: &str) -> Option<String> {
        match widget_name {
            "Tiles" | "Tilemap" | "Tile Editor Dock RGBA Layout View" | "Tile Editor Tree" => {
                Some("docs/creator/docks/tile_picker_editor".into())
            }
            "DockDataEditor" | "DockDataEditorMax" | "Data" => {
                Some(self.help_url_for_data_context())
            }
            "DockCodeEditor" | "Code" => Some("docs/creator/docks/eldrin_script_editor".into()),
            "Visual Code" => Some("docs/creator/docks/visual_script_editor".into()),
            "PolyView" => {
                if self.server_ctx.editor_view_mode == EditorViewMode::D2 {
                    Some("docs/building_maps/creating_2d".into())
                } else {
                    Some("docs/building_maps/creating_3d_maps".into())
                }
            }
            name if name.starts_with("DockVisualScripting") => {
                Some("docs/creator/docks/visual_script_editor".into())
            }
            name if name.starts_with("Tile Editor ") => {
                Some("docs/creator/docks/tile_picker_editor".into())
            }
            _ => None,
        }
    }

    fn help_url_for_editor_event(&self, event: &TheEvent, ui: &mut TheUI) -> Option<String> {
        let mut clicked = false;
        let widget_name = match event {
            TheEvent::StateChanged(id, state) if *state == TheWidgetState::Clicked => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::RenderViewClicked(id, _) => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::TilePicked(id, _) => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::TileEditorClicked(id, _) => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::MouseDown(coord) => {
                clicked = true;
                ui.get_widget_at_coord(*coord).map(|w| w.id().name.clone())
            }
            _ => None,
        };

        if let Some(widget_name) = widget_name
            && let Some(url) = self.help_url_for_widget_name(&widget_name)
        {
            return Some(url);
        }

        if clicked {
            let dm = DOCKMANAGER.read().unwrap();
            if dm.state != DockManagerState::Minimized {
                return match dm.dock.as_str() {
                    "Tiles" => Some("docs/creator/docks/tile_picker_editor".into()),
                    "Data" => Some(self.help_url_for_data_context()),
                    "Code" => Some("docs/creator/docks/eldrin_script_editor".into()),
                    "Visual Code" => Some("docs/creator/docks/visual_script_editor".into()),
                    _ => None,
                };
            }
        }
        None
    }
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut project = Project::new();
        if let Some(bytes) = crate::Embedded::get("toml/config.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.config = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/rules.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.rules = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/locales.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.locales = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/audio_fx.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.audio_fx = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/authoring.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.authoring = source.to_string();
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
        let (self_update_tx, self_update_rx) = channel();

        #[cfg(all(
            not(target_arch = "wasm32"),
            feature = "self-update",
            not(target_os = "macos")
        ))]
        let self_updater = SelfUpdater::new("markusmoenig", "Eldiron", "eldiron-creator");
        #[cfg(all(
            not(target_arch = "wasm32"),
            feature = "self-update",
            target_os = "macos"
        ))]
        let self_updater = SelfUpdater::new("markusmoenig", "Eldiron", "Eldiron-Creator.app");

        let initial_session = ProjectSession {
            project: project.clone(),
            project_path: None,
            undo: UndoManager::default(),
            dirty: false,
        };

        Self {
            project,
            project_path: None,
            sessions: vec![initial_session],
            active_session: 0,
            replace_next_project_load_in_active_tab: false,
            last_active_dirty: false,

            sidebar: Sidebar::new(),
            mapeditor: MapEditor::new(),

            server_ctx: ServerContext::default(),

            update_tracker: UpdateTracker::new(),
            event_receiver: None,

            #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
            self_update_rx,
            #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
            self_update_tx,
            #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
            self_updater: Arc::new(Mutex::new(self_updater)),

            update_counter: 0,
            last_processed_log_len: 0,
            pending_game_messages: Vec::new(),
            pending_game_says: Vec::new(),
            pending_game_choices: Vec::new(),
            pending_text_game_command: None,
            pending_text_game_runtime_flush: false,

            build_values: ValueContainer::default(),
            window_state: Self::load_window_state(),
            starter_projects: Vec::new(),
            starter_project_cache: HashMap::new(),
            starter_manifest_cache: None,
            starter_loader_rx: None,
            selected_starter_manifest_id: None,
        }
    }

    fn init(&mut self, _ctx: &mut TheContext) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
        {
            let updater = Arc::clone(&self.self_updater);
            let tx = self.self_update_tx.clone();

            thread::spawn(move || {
                let mut updater = updater.lock().unwrap();

                if let Err(err) = updater.fetch_release_list() {
                    tx.send(SelfUpdateEvent::UpdateError(err.to_string()))
                        .unwrap();
                };
            });
        }
    }

    fn window_title(&self) -> String {
        "Eldiron Creator".to_string()
    }

    fn target_fps(&self) -> f64 {
        1000.0 / self.redraw_interval_ms() as f64
    }

    fn fonts_to_load(&self) -> Vec<TheFontScript> {
        vec![TheFontScript::Han]
    }

    fn default_window_size(&self) -> (usize, usize) {
        (
            self.window_state.width.unwrap_or(1200),
            self.window_state.height.unwrap_or(720),
        )
    }

    fn min_window_size(&self) -> (usize, usize) {
        (1200, 720)
    }

    fn default_window_position(&self) -> Option<(i32, i32)> {
        Some((self.window_state.x?, self.window_state.y?))
    }

    fn window_icon(&self) -> Option<(Vec<u8>, u32, u32)> {
        if let Some(file) = Embedded::get("window_logo.png") {
            let data = std::io::Cursor::new(file.data);

            let decoder = png::Decoder::new(data);
            if let Ok(mut reader) = decoder.read_info() {
                if let Some(buffer_size) = reader.output_buffer_size() {
                    let mut buf = vec![0; buffer_size];
                    let info = reader.next_frame(&mut buf).unwrap();
                    let bytes = &buf[..info.buffer_size()];

                    Some((bytes.to_vec(), info.width, info.height))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        RUSTERIX.write().unwrap().client.messages_font = ctx.ui.font.clone();

        // Embedded Icons
        for file in Embedded::iter() {
            let name = file.as_ref();

            if name.ends_with(".png") {
                if let Some(file) = Embedded::get(name) {
                    let data = std::io::Cursor::new(file.data);

                    let decoder = png::Decoder::new(data);
                    if let Ok(mut reader) = decoder.read_info() {
                        if let Some(buffer_size) = reader.output_buffer_size() {
                            let mut buf = vec![0; buffer_size];
                            let info = reader.next_frame(&mut buf).unwrap();
                            let bytes = &buf[..info.buffer_size()];

                            let mut cut_name = name.replace("icons/", "");
                            cut_name = cut_name.replace(".png", "");

                            ctx.ui.add_icon(
                                cut_name.to_string(),
                                TheRGBABuffer::from(bytes.to_vec(), info.width, info.height),
                            );
                        }
                    }
                }
            }
        }

        // ---

        ui.set_statusbar_name("Statusbar".to_string());

        let mut top_canvas = TheCanvas::new();
        // Internal file/edit/game menu is hidden for the Xcode staticlib wrapper
        // where native menu handling is expected.
        #[cfg(not(feature = "staticlib"))]
        {
            let mut menu_canvas = TheCanvas::new();
            let mut menu = TheMenu::new(TheId::named("Menu"));

            let mut file_menu = TheContextMenu::named(fl!("menu_file"));
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_new"),
                TheId::named("New"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'n'),
            ));
            file_menu.add_separator();
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_open"),
                TheId::named("Open"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'o'),
            ));
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_close"),
                TheId::named("Close"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'w'),
            ));
            file_menu.add_separator();
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_save"),
                TheId::named("Save"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 's'),
            ));
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_save_as"),
                TheId::named("Save As"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'a'),
            ));
            let mut edit_menu = TheContextMenu::named(fl!("menu_edit"));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_undo"),
                TheId::named("Undo"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'z'),
            ));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_redo"),
                TheId::named("Redo"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT, 'z'),
            ));
            edit_menu.add_separator();
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_cut"),
                TheId::named("Cut"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'x'),
            ));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_copy"),
                TheId::named("Copy"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'c'),
            ));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_paste"),
                TheId::named("Paste"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'v'),
            ));
            edit_menu.add_separator();
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_apply_action"),
                TheId::named("Action Apply"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'p'),
            ));

            let mut game_menu = TheContextMenu::named(fl!("game"));
            game_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_play"),
                TheId::named("Play"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'p'),
            ));
            game_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_pause"),
                TheId::named("Pause"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'o'),
            ));
            game_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_stop"),
                TheId::named("Stop"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT, 'p'),
            ));
            game_menu.add_separator();
            let mut show_menu = TheContextMenu::named("Show".to_string());
            show_menu.add(TheContextMenuItem::new(
                "Settings".to_string(),
                TheId::named("Show Settings"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Rules".to_string(),
                TheId::named("Show Rules"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Locales".to_string(),
                TheId::named("Show Locales"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Audio FX".to_string(),
                TheId::named("Show Audio FX"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Authoring".to_string(),
                TheId::named("Show Authoring"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Debug Log".to_string(),
                TheId::named("Show Debug Log"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Console".to_string(),
                TheId::named("Show Console"),
            ));
            game_menu.add(TheContextMenuItem::new_submenu(
                "Show".to_string(),
                TheId::named("Show"),
                show_menu,
            ));

            file_menu.register_accel(ctx);
            edit_menu.register_accel(ctx);
            game_menu.register_accel(ctx);

            menu.add_context_menu(file_menu);
            menu.add_context_menu(edit_menu);
            menu.add_context_menu(game_menu);
            menu_canvas.set_widget(menu);
            top_canvas.set_top(menu_canvas);
        }

        let mut menubar = TheMenubar::new(TheId::named("Menubar"));
        #[cfg(feature = "staticlib")]
        menubar.limiter_mut().set_max_height(43);
        #[cfg(not(feature = "staticlib"))]
        menubar.limiter_mut().set_max_height(43 + 22);

        let mut logo_button = TheMenubarButton::new(TheId::named("Logo"));
        logo_button.set_icon_name("logo".to_string());
        logo_button.set_status_text(&fl!("status_logo_button"));

        let mut open_button = TheMenubarButton::new(TheId::named("Open"));
        open_button.set_icon_name("icon_role_load".to_string());
        open_button.set_status_text(&fl!("status_open_button"));

        let mut save_button = TheMenubarButton::new(TheId::named("Save"));
        save_button.set_status_text(&fl!("status_save_button"));
        save_button.set_icon_name("icon_role_save".to_string());

        let mut save_as_button = TheMenubarButton::new(TheId::named("Save As"));
        save_as_button.set_icon_name("icon_role_save_as".to_string());
        save_as_button.set_status_text(&fl!("status_save_as_button"));
        save_as_button.set_icon_offset(Vec2::new(2, -5));

        let mut undo_button = TheMenubarButton::new(TheId::named("Undo"));
        undo_button.set_status_text(&fl!("status_undo_button"));
        undo_button.set_icon_name("icon_role_undo".to_string());

        let mut redo_button = TheMenubarButton::new(TheId::named("Redo"));
        redo_button.set_status_text(&fl!("status_redo_button"));
        redo_button.set_icon_name("icon_role_redo".to_string());

        let mut play_button = TheMenubarButton::new(TheId::named("Play"));
        play_button.set_status_text(&fl!("status_play_button"));
        play_button.set_icon_name("play".to_string());
        //play_button.set_fixed_size(vec2i(28, 28));

        let mut pause_button = TheMenubarButton::new(TheId::named("Pause"));
        pause_button.set_status_text(&fl!("status_pause_button"));
        pause_button.set_icon_name("play-pause".to_string());

        let mut stop_button = TheMenubarButton::new(TheId::named("Stop"));
        stop_button.set_status_text(&fl!("status_stop_button"));
        stop_button.set_icon_name("stop-fill".to_string());

        let mut input_button = TheMenubarButton::new(TheId::named("GameInput"));
        input_button.set_status_text(&fl!("status_game_input_button"));
        input_button.set_icon_name("keyboard".to_string());
        input_button.set_has_state(true);

        let mut time_slider = TheTimeSlider::new(TheId::named("Server Time Slider"));
        time_slider.set_status_text(&fl!("status_time_slider"));
        time_slider.set_continuous(true);
        time_slider.limiter_mut().set_max_width(400);
        time_slider.set_value(TheValue::Time(TheTime::default()));

        let mut patreon_button = TheMenubarButton::new(TheId::named("Patreon"));
        patreon_button.set_status_text(&fl!("status_patreon_button"));
        patreon_button.set_icon_name("patreon".to_string());
        // patreon_button.set_fixed_size(vec2i(36, 36));
        patreon_button.set_icon_offset(Vec2::new(-4, -2));

        let mut help_button = TheMenubarButton::new(TheId::named("Help"));
        help_button.set_status_text(&fl!("status_help_button"));
        help_button.set_icon_name("question-mark".to_string());
        help_button.set_has_state(true);
        // patreon_button.set_fixed_size(vec2i(36, 36));
        help_button.set_icon_offset(Vec2::new(-2, -2));

        #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
        let mut update_button = {
            let mut button = TheMenubarButton::new(TheId::named("Update"));
            button.set_status_text(&fl!("status_update_button"));
            button.set_icon_name("arrows-clockwise".to_string());
            button
        };

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(Vec4::new(10, 2, 10, 1));
        hlayout.add_widget(Box::new(logo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(open_button));
        hlayout.add_widget(Box::new(save_button));
        hlayout.add_widget(Box::new(save_as_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(undo_button));
        hlayout.add_widget(Box::new(redo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(play_button));
        hlayout.add_widget(Box::new(pause_button));
        hlayout.add_widget(Box::new(stop_button));
        hlayout.add_widget(Box::new(input_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(time_slider));
        //hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));

        #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
        {
            hlayout.add_widget(Box::new(update_button));
            hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
            hlayout.add_widget(Box::new(patreon_button));
            hlayout.set_reverse_index(Some(3));
        }

        #[cfg(not(all(not(target_arch = "wasm32"), feature = "self-update")))]
        {
            hlayout.add_widget(Box::new(patreon_button));
            hlayout.add_widget(Box::new(help_button));
            hlayout.set_reverse_index(Some(2));
        }

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        ui.canvas.set_top(top_canvas);

        // Sidebar
        self.sidebar.init_ui(ui, ctx, &mut self.server_ctx);

        // Docks
        let bottom_panels = DOCKMANAGER.write().unwrap().init(ctx);

        let mut editor_canvas: TheCanvas = TheCanvas::new();

        let mut editor_stack = TheStackLayout::new(TheId::named("Editor Stack"));
        let poly_canvas = self.mapeditor.init_ui(ui, ctx, &mut self.project);
        editor_stack.add_canvas(poly_canvas);

        // Add Dock Editors
        DOCKMANAGER
            .write()
            .unwrap()
            .add_editors_to_stack(&mut editor_stack, ctx);

        editor_canvas.set_layout(editor_stack);

        // Main V Layout
        let mut vsplitlayout = TheSharedVLayout::new(TheId::named("Shared VLayout"));
        vsplitlayout.add_canvas(editor_canvas);
        vsplitlayout.add_canvas(bottom_panels);
        vsplitlayout.set_shared_ratio(crate::DEFAULT_VLAYOUT_RATIO);
        vsplitlayout.set_mode(TheSharedVLayoutMode::Shared);

        let mut shared_canvas = TheCanvas::new();
        shared_canvas.set_layout(vsplitlayout);

        let mut tabs_canvas = TheCanvas::new();
        let mut tabs = TheTabbar::new(TheId::named("Project Tabs"));
        tabs.limiter_mut().set_max_height(22);
        tabs_canvas.set_widget(tabs);
        shared_canvas.set_top(tabs_canvas);

        // Tool List
        let mut tool_list_canvas: TheCanvas = TheCanvas::new();

        let mut tool_list_bar_canvas = TheCanvas::new();
        tool_list_bar_canvas.set_widget(TheToolListBar::new(TheId::empty()));
        tool_list_canvas.set_top(tool_list_bar_canvas);

        let mut v_tool_list_layout = TheVLayout::new(TheId::named("Tool List Layout"));
        v_tool_list_layout.limiter_mut().set_max_width(51);
        v_tool_list_layout.set_margin(Vec4::new(2, 2, 2, 2));
        v_tool_list_layout.set_padding(1);

        TOOLLIST
            .write()
            .unwrap()
            .set_active_editor(&mut v_tool_list_layout, ctx);

        tool_list_canvas.set_layout(v_tool_list_layout);

        let mut tool_list_border_canvas = TheCanvas::new();
        let mut border_widget = TheIconView::new(TheId::empty());
        border_widget.set_border_color(Some([82, 82, 82, 255]));
        border_widget.limiter_mut().set_max_width(1);
        border_widget.limiter_mut().set_max_height(i32::MAX);
        tool_list_border_canvas.set_widget(border_widget);

        tool_list_canvas.set_right(tool_list_border_canvas);
        shared_canvas.set_left(tool_list_canvas);

        ui.canvas.set_center(shared_canvas);

        let mut status_canvas = TheCanvas::new();
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text(fl!("info_welcome"));
        status_canvas.set_widget(statusbar);

        ui.canvas.set_bottom(status_canvas);

        // -

        // ctx.ui.set_disabled("Save");
        // ctx.ui.set_disabled("Save As");
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");

        // Init Rusterix

        if let Some(icon) = ctx.ui.icon("light_on") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values.set("light_on", Value::Texture(texture));
        }
        if let Some(icon) = ctx.ui.icon("light_off") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values.set("light_off", Value::Texture(texture));
        }
        if let Some(icon) = ctx.ui.icon("character_on") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values
                .set("character_on", Value::Texture(texture));
        }
        if let Some(icon) = ctx.ui.icon("character_off") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values
                .set("character_off", Value::Texture(texture));
        }
        RUSTERIX
            .write()
            .unwrap()
            .client
            .builder_d2
            .set_properties(&self.build_values);
        RUSTERIX.write().unwrap().set_d2();
        SCENEMANAGER
            .write()
            .unwrap()
            .set_apply_preview_filters(true);
        SCENEMANAGER.write().unwrap().startup();

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
        self.rebuild_project_tabs(ui);
    }

    /// Set the command line arguments
    fn set_cmd_line_args(&mut self, args: Vec<String>, ctx: &mut TheContext) {
        if args.len() > 1 {
            let mut queued_any = false;
            for arg in args.iter().skip(1) {
                #[allow(irrefutable_let_patterns)]
                if let Ok(path) = PathBuf::from_str(arg) {
                    if !queued_any {
                        self.replace_next_project_load_in_active_tab = true;
                    }
                    ctx.ui.send(TheEvent::FileRequesterResult(
                        TheId::named("Open"),
                        vec![path],
                    ));
                    queued_any = true;
                }
            }
            if queued_any {
                return;
            }
        }

        self.replace_next_project_load_in_active_tab = true;
        ctx.ui.send(TheEvent::StateChanged(
            TheId::named("New"),
            TheWidgetState::Clicked,
        ));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let mut update_server_icons = false;

        if let Some((input_id, command)) = self.pending_text_game_command.take() {
            TEXTGAME.write().unwrap().handle_input(
                &input_id,
                &command,
                &mut self.project,
                &self.server_ctx,
                ui,
                ctx,
            );
            self.pending_text_game_runtime_flush = !command.trim().is_empty();
            redraw = true;
        }

        if self.pending_text_game_runtime_flush {
            let is_running =
                RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running;
            if is_running && self.server_ctx.text_game_mode {
                warmup_runtime(&mut RUSTERIX.write().unwrap(), &mut self.project, 1);

                if let Some(region) = self.project.get_region_ctx(&self.server_ctx) {
                    let region_id = region.map.id;
                    let mut messages = RUSTERIX.write().unwrap().server.get_messages(&region_id);
                    let mut says = RUSTERIX.write().unwrap().server.get_says(&region_id);

                    TEXTGAME.write().unwrap().update(
                        &self.project,
                        &self.server_ctx,
                        &mut messages,
                        &mut says,
                        ui,
                        ctx,
                    );
                }
            }
            self.pending_text_game_runtime_flush = false;
            redraw = true;
        }

        // Make sure on first startup the active tool is properly selected
        if self.update_counter == 0 {
            let mut toollist = TOOLLIST.write().unwrap();
            let id = toollist.get_current_tool().id().uuid;

            toollist.set_tool(id, ui, ctx, &mut self.project, &mut self.server_ctx);
        }

        // Get build results from the scene manager if any
        while let Some(result) = SCENEMANAGER.write().unwrap().receive() {
            match result {
                SceneManagerResult::Startup => {
                    println!("Scene manager has started up.");
                }
                SceneManagerResult::ProcessedHeights(coord, heights) => {
                    if let Some(map) = &mut self.project.get_map_mut(&self.server_ctx) {
                        let local = map.terrain.get_chunk_coords(coord.x, coord.y);
                        if let Some(chunk) = &mut map.terrain.chunks.get_mut(&local) {
                            chunk.processed_heights = Some(heights);
                        }
                    }
                }
                SceneManagerResult::Chunk(chunk, togo, total, billboards) => {
                    if togo == 0 {
                        self.server_ctx.background_progress = None;
                    } else {
                        self.server_ctx.background_progress = Some(format!("{togo}/{total}"));
                    }

                    let mut rusterix = RUSTERIX.write().unwrap();

                    rusterix
                        .scene_handler
                        .vm
                        .execute(scenevm::Atom::RemoveChunkAt {
                            origin: chunk.origin,
                        });

                    rusterix.scene_handler.vm.execute(scenevm::Atom::AddChunk {
                        id: Uuid::new_v4(),
                        chunk: chunk,
                    });

                    // Add billboards to scene_handler (indexed by GeoId)
                    for billboard in billboards {
                        rusterix
                            .scene_handler
                            .billboards
                            .insert(billboard.geo_id, billboard);
                    }

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimap"),
                        TheValue::Empty,
                    ));
                }
                SceneManagerResult::UpdatedBatch3D(coord, batch) => {
                    let mut rusterix = RUSTERIX.write().unwrap();
                    if let Some(chunk) = rusterix.client.scene.chunks.get_mut(&coord) {
                        chunk.terrain_batch3d = Some(batch);
                    }
                }
                SceneManagerResult::Clear => {
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix
                        .scene_handler
                        .vm
                        .execute(scenevm::Atom::ClearGeometry);

                    rusterix.scene_handler.billboards.clear();
                }
                SceneManagerResult::Quit => {
                    println!("Scene manager has shutdown.");
                }
            }
        }

        // Check for redraw (30fps) and tick updates
        let redraw_ms = self.redraw_interval_ms();
        let tick_ms = CONFIGEDITOR.read().unwrap().game_tick_ms.max(1) as u64;
        let (mut redraw_update, tick_update) = self.update_tracker.update(redraw_ms, tick_ms);

        // Handle queued UI events in the same update pass so input can trigger immediate redraw work.
        let mut pending_events = Vec::new();
        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                pending_events.push(event);
            }
        }
        if !pending_events.is_empty() {
            redraw_update = true;
        }

        if let Some(receiver) = &mut self.starter_loader_rx
            && let Ok(starters) = receiver.try_recv()
        {
            self.starter_manifest_cache = Some(starters.clone());
            self.starter_projects = starters;
            self.rebuild_starter_project_list(ui, ctx);
            if let Some(first) = self.starter_projects.first() {
                self.selected_starter_manifest_id = Some(first.manifest_id.clone());
                ctx.ui.send(TheEvent::StateChanged(
                    TheId::named_with_id("Starter Project List Item", first.id),
                    TheWidgetState::Selected,
                ));
                ui.set_enabled(Self::STARTER_CREATE_ID, ctx);
            } else if let Some(list) = ui.get_list_layout(Self::STARTER_LIST_ID) {
                list.clear();
                let mut item = TheListItem::new(TheId::named("Starter Project Empty"));
                item.set_text("No starter projects found.".to_string());
                item.set_sub_text("The Eldiron repo metadata could not be loaded.".to_string());
                item.set_size(52);
                item.set_text_color(WHITE);
                item.set_text_size(14.0);
                item.set_sub_text_size(12.0);
                list.add_item(item, ctx);
            }
            self.starter_loader_rx = None;
            ctx.ui.relayout = true;
            ctx.ui.redraw_all = true;
            redraw_update = true;
        }

        if tick_update {
            RUSTERIX.write().unwrap().client.inc_animation_frame();
            RUSTERIX
                .write()
                .unwrap()
                .scene_handler
                .tick_particle_clocks();

            self.server_ctx.animation_counter = self.server_ctx.animation_counter.wrapping_add(1);
            // To update animated minimaps (only for docks that need it)
            if DOCKMANAGER
                .read()
                .unwrap()
                .current_dock_supports_minimap_animation()
            {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Soft Update Minimap"),
                    TheValue::Empty,
                ));
            }
        }

        if redraw_update && !self.project.regions.is_empty() {
            // SCENEMANAGER.write().unwrap().tick();
            SCENEMANAGER.write().unwrap().tick_batch(8);

            self.build_values.set(
                "no_rect_geo",
                Value::Bool(self.server_ctx.no_rect_geo_on_map),
            );

            extract_build_values_from_config(&mut self.build_values);

            let mut messages = Vec::new();
            let mut says = Vec::new();
            let mut choices = Vec::new();

            // Update entities when the server is running
            {
                let rusterix = &mut RUSTERIX.write().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    // Send a game tick to all servers
                    if tick_update {
                        rusterix.server.system_tick();
                    }

                    // Send a redraw tick to all servers
                    if redraw_update {
                        rusterix.server.redraw_tick();
                    }

                    if let Some(new_region_name) = rusterix.update_server() {
                        rusterix.client.current_map = new_region_name;
                    }
                    if rusterix.server.log_changed {
                        let log_text = rusterix.server.get_log();
                        ui.set_widget_value("LogEdit", ctx, TheValue::Text(log_text.clone()));

                        // Auto-open Debug Log only when new log content contains warning/error.
                        let mut start = if log_text.len() < self.last_processed_log_len {
                            0
                        } else {
                            self.last_processed_log_len
                        };
                        while start < log_text.len() && !log_text.is_char_boundary(start) {
                            start += 1;
                        }
                        let new_segment = &log_text[start..];
                        if Self::log_segment_has_warning_or_error(new_segment) {
                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named("Debug Log"),
                                TheWidgetState::Clicked,
                            ));
                        }
                        self.last_processed_log_len = log_text.len();
                    }
                    let mut refresh_visual_debug = false;
                    for r in &mut self.project.regions {
                        rusterix.server.apply_entities_items(&mut r.map);

                        if r.id == self.server_ctx.curr_region {
                            refresh_visual_debug = true;
                            if let Some(time) = rusterix.server.get_time(&r.map.id) {
                                rusterix.client.set_server_time(time);
                                if let Some(widget) = ui.get_widget("Server Time Slider") {
                                    widget.set_value(TheValue::Time(rusterix.client.server_time));
                                }
                            }
                            rusterix::tile_builder(&mut r.map, &mut rusterix.assets);
                            messages = rusterix.server.get_messages(&r.map.id);
                            says = rusterix.server.get_says(&r.map.id);
                            choices = rusterix.server.get_choices(&r.map.id);

                            if !self.server_ctx.game_mode {
                                self.pending_game_messages.append(&mut messages);
                                self.pending_game_says.append(&mut says);
                                self.pending_game_choices.append(&mut choices);
                            } else {
                                if !self.pending_game_messages.is_empty() {
                                    let mut pending =
                                        std::mem::take(&mut self.pending_game_messages);
                                    pending.append(&mut messages);
                                    messages = pending;
                                }
                                if !self.pending_game_says.is_empty() {
                                    let mut pending = std::mem::take(&mut self.pending_game_says);
                                    pending.append(&mut says);
                                    says = pending;
                                }
                                if !self.pending_game_choices.is_empty() {
                                    let mut pending =
                                        std::mem::take(&mut self.pending_game_choices);
                                    pending.append(&mut choices);
                                    choices = pending;
                                }
                            }
                            for cmd in rusterix.server.get_audio_commands(&r.map.id) {
                                match cmd {
                                    AudioCommand::Play {
                                        name,
                                        bus,
                                        gain,
                                        looping,
                                    } => {
                                        rusterix.play_audio_on_bus(&name, &bus, gain, looping);
                                    }
                                    AudioCommand::ClearBus { bus } => {
                                        rusterix.clear_audio_bus(&bus);
                                    }
                                    AudioCommand::ClearAll => {
                                        rusterix.clear_all_audio();
                                    }
                                    AudioCommand::SetBusVolume { bus, volume } => {
                                        rusterix.set_audio_bus_volume(&bus, volume);
                                    }
                                }
                            }
                        }
                    }
                    if refresh_visual_debug {
                        DOCKMANAGER.write().unwrap().apply_debug_data(
                            ui,
                            ctx,
                            &self.project,
                            &self.server_ctx,
                            &rusterix.server.debug,
                        );
                    }
                }
            }
            let is_running =
                RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running;

            DOCKMANAGER.write().unwrap().sync_text_play_dock(
                ui,
                ctx,
                &self.project,
                &mut self.server_ctx,
                is_running,
            );

            if is_running && self.server_ctx.text_game_mode {
                if !self.server_ctx.game_mode {
                    if !self.pending_game_messages.is_empty() {
                        messages = std::mem::take(&mut self.pending_game_messages);
                    }
                    if !self.pending_game_says.is_empty() {
                        says = std::mem::take(&mut self.pending_game_says);
                    }
                }
                TEXTGAME.write().unwrap().update(
                    &self.project,
                    &self.server_ctx,
                    &mut messages,
                    &mut says,
                    ui,
                    ctx,
                );
            }

            // Draw Map
            if let Some(render_view) = ui.get_render_view("PolyView") {
                let dim = *render_view.dim();

                let buffer = render_view.render_buffer_mut();
                buffer.resize(dim.width, dim.height);

                {
                    // If we are drawing billboard vertices in the geometry overlay, update them.
                    if !self.server_ctx.game_mode
                        && self.server_ctx.editor_view_mode != EditorViewMode::D2
                        && self.server_ctx.curr_map_tool_type == MapToolType::Vertex
                    {
                        TOOLLIST
                            .write()
                            .unwrap()
                            .update_geometry_overlay_3d(&mut self.project, &mut self.server_ctx);
                    }

                    let rusterix = &mut RUSTERIX.write().unwrap();

                    if is_running && self.server_ctx.game_mode {
                        let game_messages = if self.server_ctx.text_game_mode {
                            Vec::new()
                        } else {
                            messages
                        };
                        let game_says = if self.server_ctx.text_game_mode {
                            Vec::new()
                        } else {
                            says
                        };
                        let game_choices = if self.server_ctx.text_game_mode {
                            Vec::new()
                        } else {
                            choices
                        };
                        for r in &mut self.project.regions {
                            if r.map.name == rusterix.client.current_map {
                                rusterix.draw_game(&r.map, game_messages, game_says, game_choices);
                                break;
                            }
                        }

                        rusterix
                            .client
                            .insert_game_buffer(render_view.render_buffer_mut());
                    } else {
                        if self.server_ctx.editor_view_mode != EditorViewMode::D2
                            && self.server_ctx.get_map_context() == MapContext::Region
                        {
                            if let Some(region) =
                                self.project.get_region_ctx_mut(&mut self.server_ctx)
                            {
                                let follow_player_firstp = is_running
                                    && self.server_ctx.editor_view_mode == EditorViewMode::FirstP;

                                if follow_player_firstp
                                    && let Some(player) =
                                        region.map.entities.iter().find(|e| e.is_player())
                                {
                                    let orientation =
                                        if player.orientation.magnitude_squared() > f32::EPSILON {
                                            player.orientation.normalized()
                                        } else {
                                            Vec2::new(1.0, 0.0)
                                        };

                                    region.editing_position_3d = Vec3::new(
                                        player.position.x,
                                        player.position.y,
                                        player.position.z,
                                    );
                                    region.editing_look_at_3d = Vec3::new(
                                        player.position.x + orientation.x,
                                        player.position.y,
                                        player.position.z + orientation.y,
                                    );
                                } else {
                                    EDITCAMERA.write().unwrap().update_action(
                                        region,
                                        &mut self.server_ctx,
                                        ctx.get_time(),
                                    );
                                }
                                EDITCAMERA.write().unwrap().update_camera(
                                    region,
                                    &mut self.server_ctx,
                                    rusterix,
                                );
                                if self.server_ctx.editor_view_mode == EditorViewMode::FirstP
                                    && EDITCAMERA.read().unwrap().move_action.is_some()
                                {
                                    ctx.ui.redraw_all = true;
                                }

                                // Keep editor 3D running mode in sync with runtime dynamic
                                // overlays (characters/items/lights).
                                let animation_frame = rusterix.client.animation_frame;
                                rusterix.build_dynamics_3d(&region.map, animation_frame);
                                rusterix.draw_d3(
                                    &region.map,
                                    render_view.render_buffer_mut().pixels_mut(),
                                    dim.width as usize,
                                    dim.height as usize,
                                );
                            }
                        } else
                        // Draw the region map
                        if self.server_ctx.get_map_context() == MapContext::Region
                            && self.server_ctx.editing_surface.is_none()
                        {
                            if let Some(region) =
                                self.project.get_region(&self.server_ctx.curr_region)
                            {
                                rusterix.client.set_clip_rect_d2(None);
                                rusterix
                                    .client
                                    .set_map_tool_type_d2(self.server_ctx.curr_map_tool_type);
                                if let Some(hover_cursor) = self.server_ctx.hover_cursor {
                                    rusterix.client.set_map_hover_info_d2(
                                        self.server_ctx.hover,
                                        Some(vek::Vec2::new(hover_cursor.x, hover_cursor.y)),
                                    );
                                } else {
                                    rusterix
                                        .client
                                        .set_map_hover_info_d2(self.server_ctx.hover, None);
                                }

                                if let Some(camera_pos) = region.map.camera_xz {
                                    rusterix.client.set_camera_info_d2(
                                        Some(Vec3::new(camera_pos.x, 0.0, camera_pos.y)),
                                        None,
                                    );
                                }

                                // let start_time = ctx.get_time();

                                let use_dungeon_concept = self.server_ctx.editor_view_mode
                                    == EditorViewMode::D2
                                    && self.server_ctx.curr_map_tool_type == MapToolType::Dungeon;

                                if let Some(clipboard) = &self.server_ctx.paste_clipboard {
                                    // During a paste operation we use a merged map

                                    let mut map = region.map.clone();
                                    if let Some(hover) = self.server_ctx.hover_cursor {
                                        map.paste_at_position(clipboard, hover);
                                    }

                                    rusterix.set_dirty();
                                    if use_dungeon_concept {
                                        rusterix.build_custom_scene_d2(
                                            Vec2::new(dim.width as f32, dim.height as f32),
                                            &map,
                                            &self.build_values,
                                            &self.server_ctx.editing_surface,
                                            true,
                                        );
                                    } else {
                                        rusterix.apply_entities_items(
                                            Vec2::new(dim.width as f32, dim.height as f32),
                                            &map,
                                            &self.server_ctx.editing_surface,
                                            false,
                                        );
                                    }
                                } else if let Some(map) = self.project.get_map(&self.server_ctx) {
                                    if use_dungeon_concept {
                                        rusterix.build_custom_scene_d2(
                                            Vec2::new(dim.width as f32, dim.height as f32),
                                            map,
                                            &self.build_values,
                                            &self.server_ctx.editing_surface,
                                            true,
                                        );
                                    } else {
                                        rusterix.apply_entities_items(
                                            Vec2::new(dim.width as f32, dim.height as f32),
                                            map,
                                            &self.server_ctx.editing_surface,
                                            false,
                                        );
                                    }
                                }

                                // Prepare the messages for the region for drawing
                                rusterix.process_messages(&region.map, says);

                                // let stop_time = ctx.get_time();
                                //println!("{} ms", stop_time - start_time);
                            }

                            if let Some(map) = self.project.get_map_mut(&self.server_ctx) {
                                if self.server_ctx.editor_view_mode == EditorViewMode::D2 {
                                    rusterix.scene_handler.settings.backend_2d =
                                        RendererBackend::Raster;
                                    rusterix.set_d2();
                                }
                                if is_running
                                    && self.server_ctx.editor_view_mode == EditorViewMode::D2
                                {
                                    let animation_frame = rusterix.client.animation_frame;
                                    rusterix.build_dynamics_2d(map, animation_frame);
                                }
                                if self.server_ctx.editor_view_mode == EditorViewMode::D2
                                    && rusterix.scene_handler.vm.vm_layer_count() > 1
                                {
                                    let overlay_enabled = if self.server_ctx.curr_map_tool_type
                                        == MapToolType::Dungeon
                                    {
                                        true
                                    } else {
                                        self.server_ctx.show_editing_geometry
                                    };
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .set_layer_enabled(1, overlay_enabled);
                                }
                                if self.server_ctx.editor_view_mode == EditorViewMode::D2
                                    && self.server_ctx.curr_map_tool_type == MapToolType::Dungeon
                                {
                                    rusterix.draw_custom_d2(
                                        map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                } else {
                                    rusterix.draw_scene(
                                        map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                }
                            }
                        } else if self.server_ctx.get_map_context() == MapContext::Region
                            && self.server_ctx.editing_surface.is_some()
                        {
                            rusterix
                                .client
                                .set_map_tool_type_d2(self.server_ctx.curr_map_tool_type);
                            if let Some(profile) = self.project.get_map_mut(&self.server_ctx) {
                                if rusterix.scene_handler.vm.vm_layer_count() > 1 {
                                    // Profile editor relies on 2D overlay guides.
                                    rusterix.scene_handler.vm.set_layer_enabled(1, true);
                                }
                                if let Some(hover_cursor) = self.server_ctx.hover_cursor {
                                    rusterix.client.set_map_hover_info_d2(
                                        self.server_ctx.hover,
                                        Some(vek::Vec2::new(hover_cursor.x, hover_cursor.y)),
                                    );
                                } else {
                                    rusterix
                                        .client
                                        .set_map_hover_info_d2(self.server_ctx.hover, None);
                                }

                                if let Some(clipboard) = &self.server_ctx.paste_clipboard {
                                    // During a paste operation we use a merged map
                                    let mut map = profile.clone();
                                    if let Some(hover) = self.server_ctx.hover_cursor {
                                        map.paste_at_position(clipboard, hover);
                                    }
                                    rusterix.set_dirty();
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        &map,
                                        &self.build_values,
                                        &self.server_ctx.editing_surface,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        &map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                } else {
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        profile,
                                        &self.build_values,
                                        &self.server_ctx.editing_surface,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        profile,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                }
                            }
                        } else
                        // Draw the screen / character / item map
                        if self.server_ctx.get_map_context() == MapContext::Character
                            || self.server_ctx.get_map_context() == MapContext::Item
                            || self.server_ctx.get_map_context() == MapContext::Screen
                        {
                            rusterix
                                .client
                                .set_map_tool_type_d2(self.server_ctx.curr_map_tool_type);
                            if let Some(map) = self.project.get_map_mut(&self.server_ctx) {
                                if rusterix.scene_handler.vm.vm_layer_count() > 1 {
                                    // Screen/character/item overlays should respect toggle.
                                    rusterix.scene_handler.vm.set_layer_enabled(
                                        1,
                                        self.server_ctx.show_editing_geometry,
                                    );
                                }
                                if let Some(hover_cursor) = self.server_ctx.hover_cursor {
                                    rusterix.client.set_map_hover_info_d2(
                                        self.server_ctx.hover,
                                        Some(vek::Vec2::new(hover_cursor.x, hover_cursor.y)),
                                    );
                                } else {
                                    rusterix
                                        .client
                                        .set_map_hover_info_d2(self.server_ctx.hover, None);
                                }

                                if self.server_ctx.get_map_context() != MapContext::Screen {
                                    rusterix.client.builder_d2.set_clip_rect(Some(
                                        rusterix::Rect {
                                            x: -5.0,
                                            y: -5.0,
                                            width: 10.0,
                                            height: 10.0,
                                        },
                                    ));
                                } else {
                                    let viewport = CONFIGEDITOR.read().unwrap().viewport;
                                    let grid_size = CONFIGEDITOR.read().unwrap().grid_size as f32;
                                    let w = viewport.x as f32 / grid_size;
                                    let h = viewport.y as f32 / grid_size;
                                    rusterix.client.builder_d2.set_clip_rect(Some(
                                        rusterix::Rect {
                                            x: -w / 2.0,
                                            y: -h / 2.0,
                                            width: w,
                                            height: h,
                                        },
                                    ));
                                }

                                if let Some(clipboard) = &self.server_ctx.paste_clipboard {
                                    // During a paste operation we use a merged map
                                    let mut map = map.clone();
                                    if let Some(hover) = self.server_ctx.hover_cursor {
                                        map.paste_at_position(clipboard, hover);
                                    }
                                    rusterix.set_dirty();
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        &map,
                                        &self.build_values,
                                        &self.server_ctx.editing_surface,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        &map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                } else {
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        map,
                                        &self.build_values,
                                        &None,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                }
                            }
                        }
                    }
                }
                if !self.server_ctx.game_mode {
                    let map_for_hud = if self.server_ctx.get_map_context() == MapContext::Region
                        && self.server_ctx.editor_view_mode != EditorViewMode::D2
                        && self.server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    {
                        self.project
                            .get_region_mut(&self.server_ctx.curr_region)
                            .map(|region| &mut region.map)
                    } else {
                        self.project.get_map_mut(&self.server_ctx)
                    };
                    if let Some(map) = map_for_hud {
                        TOOLLIST.write().unwrap().draw_hud(
                            render_view.render_buffer_mut(),
                            map,
                            ctx,
                            &mut self.server_ctx,
                            &RUSTERIX.read().unwrap().assets,
                        );
                    }
                }
            }

            // Draw the 3D Preview if active.
            // if !self.server_ctx.game_mode
            //     && self.server_ctx.curr_map_tool_helper == MapToolHelper::Preview
            // {
            //     if let Some(region) = self.project.get_region_ctx(&self.server_ctx) {
            //         PREVIEWVIEW
            //             .write()
            //             .unwrap()
            //             .draw(region, ui, ctx, &mut self.server_ctx);
            //     }
            // }

            redraw = true;
        }

        for event in pending_events {
            if self.server_ctx.help_mode
                && let Some(url) = self.help_url_for_editor_event(&event, ui)
            {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Help"),
                    TheValue::Text(url),
                ));
                redraw = true;
                continue;
            }

            if self.server_ctx.game_input_mode && !self.server_ctx.game_mode {
                // In game input mode send events to the game tool
                if let Some(game_tool) =
                    TOOLLIST.write().unwrap().get_game_tool_of_name("Game Tool")
                {
                    redraw = game_tool.handle_event(
                        &event,
                        ui,
                        ctx,
                        &mut self.project,
                        &mut self.server_ctx,
                    );
                }
            }
            if self
                .sidebar
                .handle_event(&event, ui, ctx, &mut self.project, &mut self.server_ctx)
            {
                redraw = true;
            }
            if TOOLLIST.write().unwrap().handle_event(
                &event,
                ui,
                ctx,
                &mut self.project,
                &mut self.server_ctx,
            ) {
                redraw = true;
            }
            if DOCKMANAGER.write().unwrap().handle_event(
                &event,
                ui,
                ctx,
                &mut self.project,
                &mut self.server_ctx,
            ) {
                redraw = true;
            }
            if self
                .mapeditor
                .handle_event(&event, ui, ctx, &mut self.project, &mut self.server_ctx)
            {
                redraw = true;
            }
            match event {
                TheEvent::IndexChanged(id, index) => {
                    if id.name == "Project Tabs" {
                        self.switch_to_session(
                            index,
                            ui,
                            ctx,
                            &mut update_server_icons,
                            &mut redraw,
                        );
                    }
                }
                TheEvent::CustomUndo(id, p, n) => {
                    if id.name == "ModuleUndo" {
                        let _ = (&p, &n);
                    }
                }
                TheEvent::Custom(id, value) => {
                    if id.name == "Show Help" {
                        if let TheValue::Text(url) = value {
                            _ = open::that(format!("https://www.eldiron.com/{}", url));
                            ctx.ui
                                .set_widget_state("Help".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            self.server_ctx.help_mode = false;
                            redraw = true;
                        }
                    } else if id.name == "Set Project Undo State" {
                        UNDOMANAGER.read().unwrap().set_undo_state_to_ui(ctx);
                    } else if id.name == "Open Tile Node Group Workflow" {
                        self.server_ctx.tile_node_group_id = if let TheValue::Id(group_id) = value {
                            Some(group_id)
                        } else {
                            None
                        };
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Open Tile Node Editor Skeleton"),
                            value.clone(),
                        ));
                        let mut dm = DOCKMANAGER.write().unwrap();
                        dm.set_dock("Tiles".into(), ui, ctx, &self.project, &mut self.server_ctx);
                        dm.edit_maximize(ui, ctx, &mut self.project, &mut self.server_ctx);
                        redraw = true;
                    } else if id.name == "Open Builder Graph Workflow" {
                        if let TheValue::Id(builder_id) = value {
                            self.server_ctx.curr_builder_graph_id = Some(builder_id);
                        }
                        let mut dm = DOCKMANAGER.write().unwrap();
                        dm.set_dock(
                            "Builder".into(),
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                        dm.edit_maximize(ui, ctx, &mut self.project, &mut self.server_ctx);
                        redraw = true;
                    } else if id.name == "Close Tile Node Editor Skeleton" {
                        self.server_ctx.tile_node_group_id = None;
                        DOCKMANAGER.write().unwrap().minimize(ui, ctx);
                        redraw = true;
                    } else if id.name == "Open Dungeon Dock" {
                        let current = DOCKMANAGER.read().unwrap().dock.clone();
                        if current != "Dungeon" {
                            self.server_ctx.prev_dungeon_dock = if current.is_empty() {
                                None
                            } else {
                                Some(current)
                            };
                        }
                        DOCKMANAGER.write().unwrap().set_dock(
                            "Dungeon".into(),
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                        ctx.ui.relayout = true;
                        ctx.ui.redraw_all = true;
                        redraw = true;
                    } else if id.name == "Restore Previous Dock" {
                        if let TheValue::Text(dock) = value {
                            if self.server_ctx.game_mode {
                                self.server_ctx.prev_dungeon_dock = None;
                                continue;
                            }
                            let current = DOCKMANAGER.read().unwrap().dock.clone();
                            if current == "Dungeon" {
                                DOCKMANAGER.write().unwrap().set_dock(
                                    dock.clone(),
                                    ui,
                                    ctx,
                                    &self.project,
                                    &mut self.server_ctx,
                                );
                                ctx.ui.relayout = true;
                                ctx.ui.redraw_all = true;
                                redraw = true;
                            }
                        }
                    } else if id.name == "Minimize Dock" {
                        DOCKMANAGER.write().unwrap().minimize(ui, ctx);
                        ctx.ui.relayout = true;
                        ctx.ui.redraw_all = true;
                        redraw = true;
                    } else if id.name == "Mark Rusterix Dirty" {
                        RUSTERIX.write().unwrap().set_dirty();
                        redraw = true;
                    } else if id.name == "Render SceneManager Map" {
                        if self.server_ctx.pc.is_region() {
                            if self.server_ctx.editor_view_mode == EditorViewMode::D2
                                && self.server_ctx.profile_view.is_some()
                            {
                            } else {
                                crate::utils::scenemanager_render_map(
                                    &self.project,
                                    &self.server_ctx,
                                );
                                if self.server_ctx.editor_view_mode != EditorViewMode::D2 {
                                    TOOLLIST.write().unwrap().update_geometry_overlay_3d(
                                        &mut self.project,
                                        &mut self.server_ctx,
                                    );
                                }
                            }
                        }
                    } else if id.name == "Tool Changed" {
                        TOOLLIST
                            .write()
                            .unwrap()
                            .update_geometry_overlay_3d(&mut self.project, &mut self.server_ctx);
                    } else if id.name == "Update Client Properties" {
                        let mut rusterix = RUSTERIX.write().unwrap();
                        self.build_values.set(
                            "no_rect_geo",
                            rusterix::Value::Bool(self.server_ctx.no_rect_geo_on_map),
                        );
                        self.build_values.set(
                            "editing_slice",
                            rusterix::Value::Float(self.server_ctx.editing_slice),
                        );
                        self.build_values.set(
                            "editing_slice_height",
                            rusterix::Value::Float(self.server_ctx.editing_slice_height),
                        );
                        rusterix
                            .client
                            .builder_d2
                            .set_properties(&self.build_values);
                        rusterix.set_dirty();
                    }
                }

                TheEvent::DialogValueOnClose(role, name, uuid, _value) => {
                    if name == "Delete Character Instance ?" {
                        if role == TheDialogButtonRole::Delete {
                            if let Some(region) =
                                self.project.get_region_mut(&self.server_ctx.curr_region)
                            {
                                let character_id = uuid;
                                if region.characters.shift_remove(&character_id).is_some() {
                                    self.server_ctx.curr_region_content = ContentContext::Unknown;
                                    region.map.selected_entity_item = None;
                                    redraw = true;

                                    // Remove from the content list
                                    if let Some(list) = ui.get_list_layout("Region Content List") {
                                        list.remove(TheId::named_with_id(
                                            "Region Content List Item",
                                            character_id,
                                        ));
                                        ui.select_first_list_item("Region Content List", ctx);
                                        ctx.ui.relayout = true;
                                    }
                                    insert_content_into_maps(&mut self.project);
                                    RUSTERIX.write().unwrap().set_dirty();
                                }
                            }
                        }
                    } else if name == "Delete Item Instance ?" {
                        if role == TheDialogButtonRole::Delete {
                            if let Some(region) =
                                self.project.get_region_mut(&self.server_ctx.curr_region)
                            {
                                let item_id = uuid;
                                if region.items.shift_remove(&item_id).is_some() {
                                    self.server_ctx.curr_region_content = ContentContext::Unknown;
                                    redraw = true;

                                    // Remove from the content list
                                    if let Some(list) = ui.get_list_layout("Region Content List") {
                                        list.remove(TheId::named_with_id(
                                            "Region Content List Item",
                                            item_id,
                                        ));
                                        ui.select_first_list_item("Region Content List", ctx);
                                        ctx.ui.relayout = true;
                                    }
                                    insert_content_into_maps(&mut self.project);
                                    RUSTERIX.write().unwrap().set_dirty();
                                }
                            }
                        }
                    } else if name == "Close Project Tab" && role == TheDialogButtonRole::Accept {
                        self.close_active_session(ui, ctx, &mut update_server_icons, &mut redraw);
                    } else if name == "Update Eldiron" && role == TheDialogButtonRole::Accept {
                        #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
                        {
                            let updater = self.self_updater.lock().unwrap();

                            if updater.has_newer_release() {
                                let release = updater.latest_release().cloned().unwrap();

                                let updater = Arc::clone(&self.self_updater);
                                let tx = self.self_update_tx.clone();

                                self.self_update_tx
                                    .send(SelfUpdateEvent::UpdateStart(release.clone()))
                                    .unwrap();

                                thread::spawn(move || {
                                    match updater.lock().unwrap().update_latest() {
                                        Ok(status) => match status {
                                            self_update::Status::UpToDate(_) => {
                                                tx.send(SelfUpdateEvent::AlreadyUpToDate).unwrap();
                                            }
                                            self_update::Status::Updated(_) => {
                                                tx.send(SelfUpdateEvent::UpdateCompleted(release))
                                                    .unwrap();
                                            }
                                        },
                                        Err(err) => {
                                            tx.send(SelfUpdateEvent::UpdateError(err.to_string()))
                                                .unwrap();
                                        }
                                    }
                                });
                            } else {
                                self.self_update_tx
                                    .send(SelfUpdateEvent::AlreadyUpToDate)
                                    .unwrap();
                            }
                        }
                    }
                }
                TheEvent::RenderViewDrop(_id, location, drop) => {
                    if drop.id.name.starts_with("Shader") {
                        return true;
                    }

                    let mut grid_pos = Vec2::zero();
                    let mut spawn_y = 0.0;

                    if let Some(map) = self.project.get_map(&self.server_ctx) {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            grid_pos = self.server_ctx.local_to_map_cell(
                                Vec2::new(dim.width as f32, dim.height as f32),
                                Vec2::new(location.x as f32, location.y as f32),
                                map,
                                map.subdivisions,
                            );
                            grid_pos += 0.5;
                            let mut best_height: Option<f32> = None;
                            for sector in map
                                .sectors
                                .iter()
                                .filter(|s| s.layer.is_none() && s.is_inside(map, grid_pos))
                            {
                                let mut vertex_ids: Vec<u32> = Vec::new();
                                let mut sum_y = 0.0f32;
                                let mut count = 0usize;
                                for linedef_id in &sector.linedefs {
                                    if let Some(ld) = map.find_linedef(*linedef_id) {
                                        if !vertex_ids.contains(&ld.start_vertex) {
                                            vertex_ids.push(ld.start_vertex);
                                            if let Some(v) = map.get_vertex_3d(ld.start_vertex) {
                                                sum_y += v.y;
                                                count += 1;
                                            }
                                        }
                                        if !vertex_ids.contains(&ld.end_vertex) {
                                            vertex_ids.push(ld.end_vertex);
                                            if let Some(v) = map.get_vertex_3d(ld.end_vertex) {
                                                sum_y += v.y;
                                                count += 1;
                                            }
                                        }
                                    }
                                }
                                if count > 0 {
                                    let h = sum_y / count as f32;
                                    best_height = Some(best_height.map_or(h, |prev| prev.max(h)));
                                }
                            }
                            if let Some(h) = best_height {
                                spawn_y = h;
                            }
                        }
                    }

                    if drop.id.name.starts_with("Character") {
                        let mut instance = Character {
                            character_id: drop.id.references,
                            position: Vec3::new(grid_pos.x, spawn_y, grid_pos.y),
                            ..Default::default()
                        };

                        if let Some(bytes) = crate::Embedded::get("python/instcharacter.py") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                instance.source = source.to_string();
                            }
                        }

                        let mut name = "Character".to_string();
                        if let Some(character) = self.project.characters.get(&drop.id.references) {
                            name.clone_from(&character.name);
                        }
                        instance.name = name.clone();

                        let atom = ProjectUndoAtom::AddRegionCharacterInstance(
                            self.server_ctx.curr_region,
                            instance,
                        );
                        atom.redo(&mut self.project, ui, ctx, &mut self.server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    } else if drop.id.name.starts_with("Item") {
                        let mut instance = Item {
                            item_id: drop.id.references,
                            position: Vec3::new(grid_pos.x, spawn_y, grid_pos.y),
                            ..Default::default()
                        };

                        if let Some(bytes) = crate::Embedded::get("python/institem.py") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                instance.source = source.to_string();
                            }
                        }

                        let mut name = "Item".to_string();
                        if let Some(item) = self.project.items.get(&drop.id.references) {
                            name.clone_from(&item.name);
                        }
                        instance.name = name;

                        let atom = ProjectUndoAtom::AddRegionItemInstance(
                            self.server_ctx.curr_region,
                            instance,
                        );
                        atom.redo(&mut self.project, ui, ctx, &mut self.server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                }
                /*
                TheEvent::TileEditorDrop(_id, location, drop) => {
                    if drop.id.name.starts_with("Character") {
                        let mut instance = TheCodeBundle::new();

                        let mut init = TheCodeGrid {
                            name: "init".into(),
                            ..Default::default()
                        };
                        init.insert_atom(
                            (0, 0),
                            TheCodeAtom::Set(
                                "@self.position".to_string(),
                                TheValueAssignment::Assign,
                            ),
                        );
                        init.insert_atom(
                            (1, 0),
                            TheCodeAtom::Assignment(TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (2, 0),
                            TheCodeAtom::Value(TheValue::Position(Vec3::new(
                                location.x as f32,
                                0.0,
                                location.y as f32,
                            ))),
                        );
                        instance.insert_grid(init);

                        // Set the character instance bundle, disabled for now

                        // self.sidebar.code_editor.set_bundle(
                        //     instance.clone(),
                        //     ctx,
                        //     self.sidebar.width,
                        // );

                        let character = Character {
                            id: instance.id,
                            character_id: drop.id.uuid,
                            instance,
                        };

                        // Add the character instance to the region content list

                        let mut name = "Character".to_string();
                        if let Some(character) = self.project.characters.get(&drop.id.uuid) {
                            name.clone_from(&character.name);
                        }

                        if let Some(list) = ui.get_list_layout("Region Content List") {
                            let mut item = TheListItem::new(TheId::named_with_id(
                                "Region Content List Item",
                                character.id,
                            ));
                            item.set_text(name);
                            item.set_state(TheWidgetState::Selected);
                            item.add_value_column(100, TheValue::Text("Character".to_string()));

                            list.deselect_all();
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Delete Character...".to_string(),
                                    TheId::named("Sidebar Delete Character Instance"),
                                )],
                                ..Default::default()
                            }));
                            list.add_item(item, ctx);
                            list.select_item(character.id, ctx, true);
                        }

                        // Add the character instance to the project

                        if let Some(region) =
                            self.project.get_region_mut(&self.server_ctx.curr_region)
                        {
                            region.characters.insert(character.id, character.clone());
                        }

                        // Add the character instance to the server

                        self.server_ctx.curr_character = Some(character.character_id);
                        self.server_ctx.curr_character_instance = Some(character.id);
                        self.server_ctx.curr_area = None;
                        //self.sidebar.deselect_all("Character List", ui);

                        self.server_ctx.curr_grid_id =
                            self.server.add_character_instance_to_region(
                                self.server_ctx.curr_region,
                                character,
                                None,
                            );

                        // Set the character instance debug info, disabled for now

                        // if let Some(curr_grid_id) = self.server_ctx.curr_grid_id {
                        //     let debug_module = self.server.get_region_debug_module(
                        //         self.server_ctx.curr_region,
                        //         curr_grid_id,
                        //     );

                        //     self.sidebar.code_editor.set_debug_module(debug_module, ui);
                        // }
                    } else if drop.id.name.starts_with("Item") {
                        let mut instance = TheCodeBundle::new();

                        let mut init = TheCodeGrid {
                            name: "init".into(),
                            ..Default::default()
                        };
                        init.insert_atom(
                            (0, 0),
                            TheCodeAtom::Set(
                                "@self.position".to_string(),
                                TheValueAssignment::Assign,
                            ),
                        );
                        init.insert_atom(
                            (1, 0),
                            TheCodeAtom::Assignment(TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (2, 0),
                            TheCodeAtom::Value(TheValue::Position(Vec3::new(
                                location.x as f32,
                                0.0,
                                location.y as f32,
                            ))),
                        );
                        instance.insert_grid(init);

                        // Set the character instance bundle, disabled for now

                        // self.sidebar.code_editor.set_bundle(
                        //     instance.clone(),
                        //     ctx,
                        //     self.sidebar.width,
                        // );

                        let item = Item {
                            id: instance.id,
                            item_id: drop.id.uuid,
                            instance,
                        };

                        // Add the item instance to the region content list

                        let mut name = "Item".to_string();
                        if let Some(item) = self.project.items.get(&drop.id.uuid) {
                            name.clone_from(&item.name);
                        }

                        if let Some(list) = ui.get_list_layout("Region Content List") {
                            let mut list_item = TheListItem::new(TheId::named_with_id(
                                "Region Content List Item",
                                item.id,
                            ));
                            list_item.set_text(name);
                            list_item.set_state(TheWidgetState::Selected);
                            list_item.add_value_column(100, TheValue::Text("Item".to_string()));

                            list.deselect_all();
                            list.add_item(list_item, ctx);
                            list.select_item(item.id, ctx, true);
                        }

                        // Add the item instance to the project

                        if let Some(region) =
                            self.project.get_region_mut(&self.server_ctx.curr_region)
                        {
                            region.items.insert(item.id, item.clone());
                        }

                        // Add the character instance to the server

                        self.server_ctx.curr_character = None;
                        self.server_ctx.curr_character_instance = None;
                        self.server_ctx.curr_item = Some(item.item_id);
                        self.server_ctx.curr_item_instance = Some(item.id);
                        self.server_ctx.curr_area = None;

                        self.server_ctx.curr_grid_id = self
                            .server
                            .add_item_instance_to_region(self.server_ctx.curr_region, item);

                        // Set the character instance debug info, disabled for now

                        // if let Some(curr_grid_id) = self.server_ctx.curr_grid_id {
                        //     let debug_module = self.server.get_region_debug_module(
                        //         self.server_ctx.curr_region,
                        //         curr_grid_id,
                        //     );

                        //     self.sidebar.code_editor.set_debug_module(debug_module, ui);
                        // }
                    }
                }*/
                TheEvent::FileRequesterResult(id, paths) => {
                    // Load a palette from a file
                    if id.name == "Palette Import" {
                        for p in paths {
                            let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                            let prev = self.project.palette.clone();
                            self.project.palette.load_from_txt(contents);
                            *PALETTE.write().unwrap() = self.project.palette.clone();
                            {
                                let mut rusterix = RUSTERIX.write().unwrap();
                                rusterix.assets.palette = self.project.palette.clone();
                                rusterix.set_tiles(self.project.tiles.clone(), true);
                            }

                            if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                                let index = palette_picker.index();

                                palette_picker.set_palette(self.project.palette.clone());
                                if let Some(widget) = ui.get_widget("Palette Color Picker") {
                                    if let Some(color) = &self.project.palette[index] {
                                        widget.set_value(TheValue::ColorObject(color.clone()));
                                    }
                                }
                                if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                                    if let Some(color) = &self.project.palette[index] {
                                        widget.set_value(TheValue::Text(color.to_hex()));
                                    }
                                }
                            }
                            redraw = true;

                            let undo =
                                ProjectUndoAtom::PaletteEdit(prev, self.project.palette.clone());
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                        }
                    } else
                    // Open
                    if id.name == "Open" {
                        for p in paths {
                            if let Some(loaded) = Self::load_project_from_json_path(&p) {
                                self.open_project_as_session(
                                    loaded,
                                    Some(p.clone()),
                                    ui,
                                    ctx,
                                    &mut update_server_icons,
                                    &mut redraw,
                                );
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Project loaded successfully.".to_string(),
                                ));
                            } else {
                                self.replace_next_project_load_in_active_tab = false;
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Unable to load project!".to_string(),
                                ));
                            }
                        }
                    } else if id.name == "Save As" {
                        for p in paths {
                            self.persist_active_region_view_state();
                            let json = serde_json::to_string(&self.project);
                            if let Ok(json) = json {
                                if std::fs::write(p.clone(), json).is_ok() {
                                    self.project_path = Some(p);
                                    UNDOMANAGER.write().unwrap().mark_saved();
                                    DOCKMANAGER.write().unwrap().mark_saved();
                                    if self.active_session < self.sessions.len() {
                                        self.sessions[self.active_session].dirty = false;
                                    }
                                    self.sync_active_session_from_editor();
                                    self.rebuild_project_tabs(ui);
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Project saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save project!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                }
                TheEvent::StateChanged(id, state) => {
                    if id.name == "Help" {
                        self.server_ctx.help_mode = state == TheWidgetState::Clicked;
                    }
                    if id.name == "GameInput" {
                        self.server_ctx.game_input_mode = state == TheWidgetState::Clicked;
                    } else if id.name == "Starter Project List Item"
                        && state == TheWidgetState::Selected
                    {
                        self.selected_starter_manifest_id = self
                            .starter_projects
                            .iter()
                            .find(|entry| entry.id == id.uuid)
                            .map(|entry| entry.manifest_id.clone());
                        redraw = true;
                    } else if id.name == Self::STARTER_CREATE_ID {
                        let selected_manifest_id =
                            self.selected_starter_manifest_id.clone().or_else(|| {
                                self.starter_projects
                                    .first()
                                    .map(|entry| entry.manifest_id.clone())
                            });
                        if let Some(manifest_id) = selected_manifest_id {
                            if let Some(project) = self.load_named_starter_project(&manifest_id) {
                                ui.clear_dialog();
                                self.open_project_as_session(
                                    project,
                                    None,
                                    ui,
                                    ctx,
                                    &mut update_server_icons,
                                    &mut redraw,
                                );
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Starter project successfully initialized.".to_string(),
                                ));
                            } else {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Unable to load starter project!".to_string(),
                                ));
                            }
                        }
                        ctx.ui.set_widget_state(
                            Self::STARTER_CREATE_ID.to_string(),
                            TheWidgetState::None,
                        );
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == Self::STARTER_CANCEL_ID {
                        ui.clear_dialog();
                        ctx.ui.set_widget_state(
                            Self::STARTER_CANCEL_ID.to_string(),
                            TheWidgetState::None,
                        );
                        ctx.ui.clear_hover();
                        self.open_project_as_session(
                            Self::load_empty_project_template(),
                            None,
                            ui,
                            ctx,
                            &mut update_server_icons,
                            &mut redraw,
                        );
                        redraw = true;
                    } else if id.name == "New" {
                        self.open_starter_project_dialog(ui, ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Choose a 2D or 3D starter project.".to_string(),
                        ));
                        ctx.ui
                            .set_widget_state("New".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Logo" {
                        _ = open::that("https://eldiron.com");
                        ctx.ui
                            .set_widget_state("Logo".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Patreon" {
                        _ = open::that("https://www.patreon.com/eldiron");
                        ctx.ui
                            .set_widget_state("Patreon".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Update" {
                        #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
                        {
                            let updater = self.self_updater.lock().unwrap();

                            if updater.has_newer_release() {
                                self.self_update_tx
                                    .send(SelfUpdateEvent::UpdateConfirm(
                                        updater.latest_release().cloned().unwrap(),
                                    ))
                                    .unwrap();
                            } else {
                                if let Some(statusbar) = ui.get_widget("Statusbar") {
                                    statusbar
                                        .as_statusbar()
                                        .unwrap()
                                        .set_text(fl!("info_update_check"));
                                }

                                let updater = Arc::clone(&self.self_updater);
                                let tx = self.self_update_tx.clone();

                                thread::spawn(move || {
                                    let mut updater = updater.lock().unwrap();

                                    match updater.fetch_release_list() {
                                        Ok(_) => {
                                            if updater.has_newer_release() {
                                                tx.send(SelfUpdateEvent::UpdateConfirm(
                                                    updater.latest_release().cloned().unwrap(),
                                                ))
                                                .unwrap();
                                            } else {
                                                tx.send(SelfUpdateEvent::AlreadyUpToDate).unwrap();
                                            }
                                        }
                                        Err(err) => {
                                            tx.send(SelfUpdateEvent::UpdateError(err.to_string()))
                                                .unwrap();
                                        }
                                    }
                                });
                            }

                            ctx.ui
                                .set_widget_state("Update".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }
                    } else if id.name == "Open" {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                            "Open".into(),
                            TheFileExtension::new("Eldiron".into(), vec!["eldiron".to_string()]),
                        );
                        ctx.ui
                            .set_widget_state("Open".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Close" {
                        if self.active_session_has_changes() {
                            let uuid = Uuid::new_v4();
                            let width = 380;
                            let height = 110;

                            let mut canvas = TheCanvas::new();
                            canvas.limiter_mut().set_max_size(Vec2::new(width, height));

                            let mut hlayout: TheHLayout = TheHLayout::new(TheId::empty());
                            hlayout.limiter_mut().set_max_width(width);

                            let mut text_widget =
                                TheText::new(TheId::named_with_id("Dialog Value", uuid));
                            text_widget.set_text(
                                "This tab has unsaved changes. Close it anyway?".to_string(),
                            );
                            text_widget.limiter_mut().set_max_width(280);
                            hlayout.add_widget(Box::new(text_widget));

                            canvas.set_layout(hlayout);
                            ui.show_dialog(
                                "Close Project Tab",
                                canvas,
                                vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
                                ctx,
                            );
                        } else {
                            self.close_active_session(
                                ui,
                                ctx,
                                &mut update_server_icons,
                                &mut redraw,
                            );
                        }
                        ctx.ui
                            .set_widget_state("Close".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Save" {
                        if let Some(path) = self.project_path.clone() {
                            let mut success = false;
                            // if let Ok(output) = postcard::to_allocvec(&self.project) {
                            self.persist_active_region_view_state();
                            if let Ok(output) = serde_json::to_string(&self.project) {
                                if std::fs::write(&path, output).is_ok() {
                                    UNDOMANAGER.write().unwrap().mark_saved();
                                    DOCKMANAGER.write().unwrap().mark_saved();
                                    if self.active_session < self.sessions.len() {
                                        self.sessions[self.active_session].dirty = false;
                                    }
                                    self.sync_active_session_from_editor();
                                    self.rebuild_project_tabs(ui);
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Project saved successfully.".to_string(),
                                    ));
                                    success = true;
                                }
                            }

                            if !success {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Unable to save project!".to_string(),
                                ))
                            }
                        } else {
                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named("Save As"),
                                TheWidgetState::Clicked,
                            ));
                            ctx.ui
                                .set_widget_state("Save".to_string(), TheWidgetState::None);
                        }
                    } else if id.name == "Save As" {
                        ctx.ui.save_file_requester(
                            TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                            "Save".into(),
                            TheFileExtension::new("Eldiron".into(), vec!["eldiron".to_string()]),
                        );
                        ctx.ui
                            .set_widget_state("Save As".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    }
                    // Server
                    else if id.name == "Play" {
                        let state = RUSTERIX.read().unwrap().server.state;
                        if state == rusterix::ServerState::Paused {
                            self.pending_game_messages.clear();
                            self.pending_game_choices.clear();
                            TEXTGAME.write().unwrap().reset();
                            if self.server_ctx.text_game_mode {
                                TEXTGAME.write().unwrap().sync_output(ui, ctx);
                            }
                            RUSTERIX.write().unwrap().server.continue_instances();
                            update_server_icons = true;
                        } else {
                            if state == rusterix::ServerState::Off {
                                self.pending_game_messages.clear();
                                self.pending_game_choices.clear();
                                TEXTGAME.write().unwrap().reset();
                                if self.server_ctx.text_game_mode {
                                    TEXTGAME.write().unwrap().sync_output(ui, ctx);
                                }
                                start_server(
                                    &mut RUSTERIX.write().unwrap(),
                                    &mut self.project,
                                    true,
                                );
                                RUSTERIX.write().unwrap().clear_say_messages();
                                let commands =
                                    setup_client(&mut RUSTERIX.write().unwrap(), &mut self.project);
                                RUSTERIX
                                    .write()
                                    .unwrap()
                                    .server
                                    .process_client_commands(commands);
                                warmup_runtime(
                                    &mut RUSTERIX.write().unwrap(),
                                    &mut self.project,
                                    3,
                                );
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Server has been started.".to_string(),
                                ));
                                ui.set_widget_value("LogEdit", ctx, TheValue::Text(String::new()));
                                self.last_processed_log_len = 0;
                                RUSTERIX.write().unwrap().player_camera = PlayerCamera::D2;
                            }
                            update_server_icons = true;
                        }
                    } else if id.name == "Pause" {
                        let state = RUSTERIX.read().unwrap().server.state;
                        if state == rusterix::ServerState::Running {
                            RUSTERIX.write().unwrap().server.pause();
                            update_server_icons = true;
                        }
                    } else if id.name == "Stop" {
                        RUSTERIX.write().unwrap().server.stop();
                        RUSTERIX.write().unwrap().clear_say_messages();
                        RUSTERIX.write().unwrap().player_camera = PlayerCamera::D2;
                        {
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.client.scene.d2_dynamic.clear();
                            rusterix.client.scene.d3_dynamic.clear();
                            rusterix.client.scene.dynamic_lights.clear();
                            rusterix.scene_handler.clear_runtime_overlays();
                            rusterix.set_dirty();
                        }

                        ui.set_widget_value("InfoView", ctx, TheValue::Text("".into()));
                        insert_content_into_maps(&mut self.project);
                        update_server_icons = true;

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Render SceneManager Map"),
                            TheValue::Empty,
                        ));
                    } else if id.name == "Show Settings" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::ProjectSettings,
                        );
                        redraw = true;
                    } else if id.name == "Show Rules" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameRules,
                        );
                        redraw = true;
                    } else if id.name == "Show Locales" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameLocales,
                        );
                        redraw = true;
                    } else if id.name == "Show Audio FX" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameAudioFx,
                        );
                        redraw = true;
                    } else if id.name == "Show Authoring" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameAuthoring,
                        );
                        redraw = true;
                    } else if id.name == "Show Debug Log" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::DebugLog,
                        );
                        redraw = true;
                    } else if id.name == "Show Console" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::Console,
                        );
                        redraw = true;
                    } else if id.name == "Undo" || id.name == "Redo" {
                        let mut refresh_action_ui = false;
                        if ui.focus_widget_supports_undo_redo(ctx) {
                            if id.name == "Undo" {
                                ui.undo(ctx);
                            } else {
                                ui.redo(ctx);
                            }
                        } else if DOCKMANAGER.read().unwrap().current_dock_supports_undo() {
                            if id.name == "Undo" {
                                DOCKMANAGER.write().unwrap().undo(
                                    ui,
                                    ctx,
                                    &mut self.project,
                                    &mut self.server_ctx,
                                );
                            } else {
                                DOCKMANAGER.write().unwrap().redo(
                                    ui,
                                    ctx,
                                    &mut self.project,
                                    &mut self.server_ctx,
                                );
                            }
                            refresh_action_ui = true;
                        } else {
                            let mut manager = UNDOMANAGER.write().unwrap();

                            if id.name == "Undo" {
                                manager.undo(&mut self.server_ctx, &mut self.project, ui, ctx);
                            } else {
                                manager.redo(&mut self.server_ctx, &mut self.project, ui, ctx);
                            }
                            refresh_action_ui = true;
                        }

                        // Keep action list and TOML params in sync only when project/dock state changed.
                        if refresh_action_ui {
                            // Drop focus to avoid stale focused text-edit state surviving toolbar rebuilds.
                            ctx.ui.clear_focus();
                            // Refresh both toolbars unconditionally.
                            // Dock undo/redo may not keep CODEEDITOR.active_panel in sync.
                            {
                                let mut module = CODEGRIDFX.write().unwrap();
                                module.clear_toolbar_settings(ui, ctx);
                                module.show_settings(ui, ctx);
                            }
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Action List"),
                                TheValue::Empty,
                            ));
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Action Parameters"),
                                TheValue::Empty,
                            ));
                        }
                    } else if id.name == "Cut" {
                        if ui.focus_widget_supports_clipboard(ctx) {
                            // Widget specific
                            ui.cut(ctx);
                        } else {
                            // Global
                            ctx.ui.send(TheEvent::Cut);
                        }
                    } else if id.name == "Copy" {
                        if ui.focus_widget_supports_clipboard(ctx) {
                            // Widget specific
                            ui.copy(ctx);
                        } else {
                            // Global
                            ctx.ui.send(TheEvent::Copy);
                        }
                    } else if id.name == "Paste" {
                        Self::refresh_system_text_clipboard(ctx);
                        if ui.focus_widget_supports_clipboard(ctx) {
                            // Widget specific
                            ui.paste(ctx);
                        } else {
                            // Global
                            if let Some(value) = &ctx.ui.clipboard {
                                ctx.ui.send(TheEvent::Paste(
                                    value.clone(),
                                    ctx.ui.clipboard_app_type.clone(),
                                ));
                            } else {
                                ctx.ui.send(TheEvent::Paste(
                                    TheValue::Empty,
                                    ctx.ui.clipboard_app_type.clone(),
                                ));
                            }
                        }
                    }
                }
                TheEvent::ValueChanged(id, value) => {
                    if id.name == "Server Time Slider" {
                        if let TheValue::Time(time) = value {
                            self.project.time = time;
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.client.set_server_time(time);

                            if rusterix.server.state == rusterix::ServerState::Running {
                                if let Some(map) = self.project.get_map(&self.server_ctx) {
                                    rusterix.server.set_time(&map.id, time);
                                }
                            }
                        }
                    } else if id.name == TextGameState::GAME_INPUT_ID {
                        if let Some(command) = value.to_string() {
                            self.pending_text_game_command =
                                Some((id.name.clone(), command.clone()));
                            redraw = true;
                        }
                    } else if id.name == TextGameState::DOCK_INPUT_ID {
                        if let Some(command) = value.to_string() {
                            self.pending_text_game_command =
                                Some((id.name.clone(), command.clone()));
                            redraw = true;
                        }
                    }
                }
                _ => {}
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
        while let Ok(event) = self.self_update_rx.try_recv() {
            match event {
                SelfUpdateEvent::AlreadyUpToDate => {
                    let text = str!("Eldiron is already up-to-date.");
                    let uuid = Uuid::new_v4();

                    let width = 300;
                    let height = 100;

                    let mut canvas = TheCanvas::new();
                    canvas.limiter_mut().set_max_size(Vec2::new(width, height));

                    let mut hlayout: TheHLayout = TheHLayout::new(TheId::empty());
                    hlayout.limiter_mut().set_max_width(width);

                    let mut text_widget = TheText::new(TheId::named_with_id("Dialog Value", uuid));
                    text_widget.set_text(text.to_string());
                    text_widget.limiter_mut().set_max_width(200);
                    hlayout.add_widget(Box::new(text_widget));

                    canvas.set_layout(hlayout);

                    ui.show_dialog(
                        "Eldiron Up-to-Date",
                        canvas,
                        vec![TheDialogButtonRole::Accept],
                        ctx,
                    );
                }
                SelfUpdateEvent::UpdateCompleted(release) => {
                    if let Some(statusbar) = ui.get_widget("Statusbar") {
                        statusbar.as_statusbar().unwrap().set_text(format!(
                            "Updated to version {}. Please restart the application to enjoy the new features.",
                            release.version
                        ));
                    }
                }
                SelfUpdateEvent::UpdateConfirm(release) => {
                    let text = &format!("Update to version {}?", release.version);
                    let uuid = Uuid::new_v4();

                    let width = 300;
                    let height = 100;

                    let mut canvas = TheCanvas::new();
                    canvas.limiter_mut().set_max_size(Vec2::new(width, height));

                    let mut hlayout: TheHLayout = TheHLayout::new(TheId::empty());
                    hlayout.limiter_mut().set_max_width(width);

                    let mut text_widget = TheText::new(TheId::named_with_id("Dialog Value", uuid));
                    text_widget.set_text(text.to_string());
                    text_widget.limiter_mut().set_max_width(200);
                    hlayout.add_widget(Box::new(text_widget));

                    canvas.set_layout(hlayout);

                    ui.show_dialog(
                        "Update Eldiron",
                        canvas,
                        vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
                        ctx,
                    );
                }
                SelfUpdateEvent::UpdateError(err) => {
                    if let Some(statusbar) = ui.get_widget("Statusbar") {
                        statusbar
                            .as_statusbar()
                            .unwrap()
                            .set_text(format!("Failed to update Eldiron: {err}"));
                    }
                }
                SelfUpdateEvent::UpdateStart(release) => {
                    if let Some(statusbar) = ui.get_widget("Statusbar") {
                        statusbar
                            .as_statusbar()
                            .unwrap()
                            .set_text(format!("Updating to version {}...", release.version));
                    }
                }
            }
        }

        if update_server_icons {
            self.update_server_state_icons(ui);
            redraw = true;
        }

        let active_dirty = UNDOMANAGER.read().unwrap().has_unsaved()
            || DOCKMANAGER.read().unwrap().has_dock_changes();
        if self.active_session < self.sessions.len()
            && self.sessions[self.active_session].dirty != active_dirty
        {
            self.sessions[self.active_session].dirty = active_dirty;
            self.rebuild_project_tabs(ui);
            redraw = true;
        }
        if active_dirty != self.last_active_dirty {
            self.last_active_dirty = active_dirty;
            self.rebuild_project_tabs(ui);
            redraw = true;
        }

        self.update_counter += 1;
        if self.update_counter > 2 {
            self.sidebar.startup = false;
        }
        redraw
    }

    /// Returns true if there are changes
    fn has_changes(&self) -> bool {
        if self.active_session_has_changes() {
            return true;
        }

        for (index, session) in self.sessions.iter().enumerate() {
            if index != self.active_session && session.dirty {
                return true;
            }
        }

        false
    }

    fn window_moved(&mut self, x: i32, y: i32) {
        self.window_state.x = Some(x);
        self.window_state.y = Some(y);
        self.save_window_state();
    }

    fn window_resized(&mut self, width: usize, height: usize) {
        if width > 0 && height > 0 {
            self.window_state.width = Some(width);
            self.window_state.height = Some(height);
            self.save_window_state();
        }
    }
}

pub trait EldironEditor {
    fn update_server_state_icons(&mut self, ui: &mut TheUI);
}

impl EldironEditor for Editor {
    fn update_server_state_icons(&mut self, ui: &mut TheUI) {
        let rusterix = RUSTERIX.read().unwrap();
        if rusterix.server.state == rusterix::ServerState::Running {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-fill".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop".to_string());
                }
            }
        } else if rusterix.server.state == rusterix::ServerState::Paused {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause-fill".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop".to_string());
                }
            }
        } else if rusterix.server.state == rusterix::ServerState::Off {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop-fill".to_string());
                }
            }
        }
    }
}
