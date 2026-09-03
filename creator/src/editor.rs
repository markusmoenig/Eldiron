use crate::Embedded;
use crate::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::scepter::{ScepterEvent, ScepterRegionRequest, ScepterService};
#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use crate::self_update::{SelfUpdateEvent, SelfUpdater};
#[cfg(not(target_arch = "wasm32"))]
use eldiron_scepter::{
    ActionList as ScepterActionList, ActionRun, ActionRunScript, AttributesGet, AttributesPatch,
    GridPoint, RegionPaintCells, RegionPaintRect, RegionRef, RegionRenderPreview, ScriptGet,
    ScriptPatch, ScriptTarget, ScriptTargetKind, TileSelector, ToolList as ScepterToolList,
    ToolSelect,
};
use rayon::prelude::*;
use rusterix::render_settings::RendererBackend;
use rusterix::server::message::AudioCommand;
use rusterix::{
    PlayerCamera, Rusterix, SceneManager, SceneManagerResult, Texture, Value, ValueContainer,
};
#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use self_update::update::Release;
use shared::iso_paint_brush::{self, IsoPaintBrushSample};
use shared::iso_paint_render::{
    IsoPaintRenderCache as SharedIsoPaintRenderCache, IsoPaintRenderer,
};
use shared::rusterix_utils::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::Receiver;
#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use std::sync::{
    Arc, Mutex,
    mpsc::{Sender, channel},
};

#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use std::thread;

#[allow(dead_code)]
const ISO_PAINT_PAR_COMPOSITE_PIXELS: usize = 32_768;

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

const COMPACT_NAVIGATION_ICON_PATHS: [(&str, &str); 8] = [
    (
        "project",
        "M248.23,112.31A20,20,0,0,0,232,104H220V88a20,20,0,0,0-20-20H132L105.34,48a20.12,20.12,0,0,0-12-4H40A20,20,0,0,0,20,64V208a12,12,0,0,0,12,12H211.1a12,12,0,0,0,11.33-8l28.49-81.47.06-.17A20,20,0,0,0,248.23,112.31ZM92,68l28.8,21.6A12,12,0,0,0,128,92h68v12H69.77a20,20,0,0,0-18.94,13.58L44,137.15V68ZM202.59,196H48.89l23.72-68H226.37Z",
    ),
    (
        "graph",
        "M200,152a35.77,35.77,0,0,0-16.46,4l-21.39-16.64A35.49,35.49,0,0,0,164,128.65l10.35-3.44A36,36,0,1,0,164,100c0,1.11.06,2.21.16,3.3l-7.78,2.59A36,36,0,0,0,128,92c-1,0-1.88,0-2.81.12l-4.45-10A36,36,0,1,0,96,92c1,0,1.88,0,2.81-.12l4.45,10a35.91,35.91,0,0,0-8.59,39.7L73.39,160.49a36,36,0,1,0,15.94,17.93l21.28-18.91a35.91,35.91,0,0,0,36.8-1.21L167,173.56A36,36,0,1,0,200,152Zm0-64a12,12,0,1,1-12,12A12,12,0,0,1,200,88ZM84,56A12,12,0,1,1,96,68,12,12,0,0,1,84,56ZM56,204a12,12,0,1,1,12-12A12,12,0,0,1,56,204Zm60-76a12,12,0,1,1,12,12A12,12,0,0,1,116,128Zm84,72a12,12,0,1,1,12-12A12,12,0,0,1,200,200Z",
    ),
    (
        "terminal-nav",
        "M72.5,150.63,100.79,128,72.5,105.37a12,12,0,1,1,15-18.74l40,32a12,12,0,0,1,0,18.74l-40,32a12,12,0,0,1-15-18.74ZM144,172h32a12,12,0,0,0,0-24H144a12,12,0,0,0,0,24ZM236,56V200a20,20,0,0,1-20,20H40a20,20,0,0,1-20-20V56A20,20,0,0,1,40,36H216A20,20,0,0,1,236,56Zm-24,4H44V196H212Z",
    ),
    (
        "diagnostics-nav",
        "M140,88a16,16,0,1,1,16,16A16,16,0,0,1,140,88ZM100,72a16,16,0,1,0,16,16A16,16,0,0,0,100,72Zm120,72a91.84,91.84,0,0,1-2.34,20.64L236.81,173a12,12,0,0,1-9.62,22l-18-7.85a92,92,0,0,1-162.46,0l-18,7.85a12,12,0,1,1-9.62-22l19.15-8.36A91.84,91.84,0,0,1,36,144v-4H16a12,12,0,0,1,0-24H36v-4a91.84,91.84,0,0,1,2.34-20.64L19.19,83a12,12,0,0,1,9.62-22l18,7.85a92,92,0,0,1,162.46,0l18-7.85a12,12,0,1,1,9.62,22l-19.15,8.36A91.84,91.84,0,0,1,220,112v4h20a12,12,0,0,1,0,24H220ZM60,116H196v-4a68,68,0,0,0-136,0Zm56,94.92V140H60v4A68.1,68.1,0,0,0,116,210.92ZM196,144v-4H140v70.92A68.1,68.1,0,0,0,196,144Z",
    ),
    (
        "square",
        "M208,28H48A20,20,0,0,0,28,48V208a20,20,0,0,0,20,20H208a20,20,0,0,0,20-20V48A20,20,0,0,0,208,28Zm-4,176H52V52H204Z",
    ),
    (
        "perspective",
        "M240,116H228V48a20,20,0,0,0-23.58-19.67l-160,29.09A20,20,0,0,0,28,77.09V116H16a12,12,0,0,0,0,24H28v38.91a20,20,0,0,0,16.42,19.67l160,29.09A20,20,0,0,0,228,208V140h12a12,12,0,0,0,0-24ZM52,80.43,204,52.8V116H52ZM204,203.2,52,175.57V140H204Z",
    ),
    (
        "cube",
        "M225.6,62.64l-88-48.17a19.91,19.91,0,0,0-19.2,0l-88,48.17A20,20,0,0,0,20,80.19v95.62a20,20,0,0,0,10.4,17.55l88,48.17a19.89,19.89,0,0,0,19.2,0l88-48.17A20,20,0,0,0,236,175.81V80.19A20,20,0,0,0,225.6,62.64ZM128,36.57,200,76,128,115.4,56,76ZM44,96.79l72,39.4v76.67L44,173.44Zm96,116.07V136.19l72-39.4v76.65Z",
    ),
    (
        "camera",
        "M208,52H182.42L170,33.34A12,12,0,0,0,160,28H96a12,12,0,0,0-10,5.34L73.57,52H48A28,28,0,0,0,20,80V192a28,28,0,0,0,28,28H208a28,28,0,0,0,28-28V80A28,28,0,0,0,208,52Zm4,140a4,4,0,0,1-4,4H48a4,4,0,0,1-4-4V80a4,4,0,0,1,4-4H80a12,12,0,0,0,10-5.34L102.42,52h51.15L166,70.66A12,12,0,0,0,176,76h32a4,4,0,0,1,4,4ZM128,84a48,48,0,1,0,48,48A48.05,48.05,0,0,0,128,84Zm0,72a24,24,0,1,1,24-24A24,24,0,0,1,128,156Z",
    ),
];

const ENTITY_TOOL_ICON_PATH: &str = "M71.59,61.47a8,8,0,0,0-15.18,0l-40,120A8,8,0,0,0,24,192h80a8,8,0,0,0,7.59-10.53ZM35.1,176,64,89.3,92.9,176ZM208,76a52,52,0,1,0-52,52A52.06,52.06,0,0,0,208,76Zm-88,0a36,36,0,1,1,36,36A36,36,0,0,1,120,76Zm104,68H136a8,8,0,0,0-8,8v56a8,8,0,0,0,8,8h88a8,8,0,0,0,8-8V152A8,8,0,0,0,224,144Zm-8,56H144V160h72Z";

fn register_compact_navigation_icons(ctx: &mut TheContext) {
    for (name, path) in COMPACT_NAVIGATION_ICON_PATHS {
        ctx.ui.add_icon(
            name.to_string(),
            rasterize_svg_path_icon(path, 18, 256.0, [242, 242, 242, 255]),
        );
    }
    ctx.ui.add_icon(
        "shapes".to_string(),
        rasterize_svg_path_icon(ENTITY_TOOL_ICON_PATH, 24, 256.0, [242, 242, 242, 255]),
    );
}
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

#[derive(Clone)]
struct ProjectSession {
    project: Project,
    project_path: Option<PathBuf>,
    undo: UndoManager,
    dirty: bool,
    /// Dock-local undo stacks are global UI objects and are cleared when a tab
    /// is detached. Keep their unsaved status with the owning session.
    detached_dock_dirty: bool,
}

#[allow(dead_code)]
#[derive(Clone)]
struct IsoPaintStrokeRenderCache {
    order: u64,
    origin: [i32; 2],
    screen_anchor: Option<[i32; 2]>,
    world_anchor: Option<[f32; 3]>,
    camera_scale: Option<f32>,
    clip_geo_id: Option<scenevm::GeoId>,
    color_coverage_scale: f32,
    replace_material: bool,
    replace_opacity: u8,
    writes_material: bool,
    brush: String,
    clip: String,
    material_id: u8,
    color: [u8; 4],
    pattern_kind: String,
    pattern_scale: f32,
    pattern_mortar: f32,
    pattern_detail: f32,
    pattern_variation: f32,
    path_points: Vec<[f32; 2]>,
    path_lengths: Vec<f32>,
    erase: bool,
    buffer: TheRGBABuffer,
}

#[allow(dead_code)]
#[derive(Clone)]
struct IsoPaintCachedStrokeRender {
    key: u64,
    strokes: Vec<IsoPaintStrokeRenderCache>,
}

#[allow(dead_code)]
#[derive(Default)]
struct IsoPaintChunkRenderCache {
    revision: u64,
    strokes: Vec<IsoPaintStrokeRenderCache>,
    stroke_caches: HashMap<Uuid, IsoPaintCachedStrokeRender>,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum IsoPaintRenderItem<'a> {
    Stroke(&'a IsoPaintStrokeRenderCache),
    Stamp(&'a IsoPaintStamp),
}

#[allow(dead_code)]
#[derive(Default)]
struct IsoPaintRenderCache {
    region_id: Option<Uuid>,
    chunks: HashMap<String, IsoPaintChunkRenderCache>,
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
    #[serde(default)]
    dimension: String,
    description: String,
    project_path: String,
    #[serde(default)]
    preview: Option<String>,
}

#[derive(Clone)]
struct StarterProjectEntry {
    id: Uuid,
    manifest_id: String,
    title: String,
    dimension: String,
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
    #[cfg(not(target_arch = "wasm32"))]
    scepter_receiver: Option<Receiver<ScepterEvent>>,
    last_3d_hover_redraw_at: Option<std::time::Instant>,

    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    self_update_rx: Receiver<SelfUpdateEvent>,
    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    self_update_tx: Sender<SelfUpdateEvent>,
    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    self_updater: Arc<Mutex<SelfUpdater>>,

    update_counter: usize,
    last_processed_log_len: usize,
    pending_game_messages: Vec<rusterix::server::Message>,
    pending_game_says: Vec<TextGameSay>,
    pending_game_choices: Vec<rusterix::MultipleChoice>,
    pending_text_game_command: Option<(String, String)>,
    pending_text_game_runtime_flush: bool,
    /// Authored maps are kept separate from maps replaced/generated by the
    /// runtime. Playing must never permanently install a runtime dungeon into
    /// the Creator project.
    play_map_snapshots: Option<Vec<(Uuid, Map)>>,

    build_values: ValueContainer,
    window_state: CreatorWindowState,
    starter_projects: Vec<StarterProjectEntry>,
    starter_project_cache: HashMap<String, Project>,
    starter_manifest_cache: Option<Vec<StarterProjectEntry>>,
    starter_loader_rx: Option<Receiver<Vec<StarterProjectEntry>>>,
    starter_project_loader_rx: Option<Receiver<(String, Option<Project>)>>,
    selected_starter_manifest_id: Option<String>,
    iso_paint_render_cache: SharedIsoPaintRenderCache,
}

#[allow(dead_code)]
impl Editor {
    const PROJECT_EXTENSION: &'static str = "eldiron";
    const STARTER_REPO_RAW_BASE: &'static str =
        "https://raw.githubusercontent.com/markusmoenig/Eldiron/master/";
    const STARTER_LIST_ID: &'static str = "Starter Project List";
    const STARTER_PREVIEW_ID: &'static str = "Starter Project Preview";
    const STARTER_PREVIEW_KIND_ID: &'static str = "Starter Project Preview Kind";
    const STARTER_PREVIEW_TITLE_ID: &'static str = "Starter Project Preview Title";
    const STARTER_PREVIEW_DESCRIPTION_ID: &'static str = "Starter Project Preview Description";
    const STARTER_CREATE_ID: &'static str = "Starter Project Create";
    const STARTER_CANCEL_ID: &'static str = "Starter Project Cancel";

    /// Programmatic Eldrin entry point for editor automation and plugin hosts.
    pub fn execute_eldrin_action_script(
        &mut self,
        source: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<usize, String> {
        self.sidebar
            .execute_action_script(source, ui, ctx, &mut self.project, &mut self.server_ctx)
    }

    fn coalesce_polyview_hover_events(events: &mut Vec<TheEvent>) {
        let mut coalesced = Vec::with_capacity(events.len());
        for event in events.drain(..) {
            let is_polyview_hover = matches!(
                &event,
                TheEvent::RenderViewHoverChanged(id, _) if id.name == "PolyView"
            );
            let previous_is_polyview_hover = matches!(
                coalesced.last(),
                Some(TheEvent::RenderViewHoverChanged(id, _)) if id.name == "PolyView"
            );

            if is_polyview_hover && previous_is_polyview_hover {
                if let Some(previous) = coalesced.last_mut() {
                    *previous = event;
                }
            } else {
                coalesced.push(event);
            }
        }
        *events = coalesced;
    }

    fn iso_paint_color_with_opacity(mut color: [u8; 4], opacity: f32) -> [u8; 4] {
        color[3] = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        color
    }

    fn iso_paint_material_pixel(
        material_id: u8,
        replace_opacity: Option<u8>,
        coverage: u8,
    ) -> [u8; 4] {
        let mode = replace_opacity
            .map(|opacity| opacity.saturating_add(1).max(1))
            .unwrap_or(0);
        [254, material_id, mode, coverage]
    }

    fn iso_paint_set_material_pixel_at(
        material_pixels: &mut [u8],
        index: usize,
        material_id: u8,
        replace_material: bool,
        replace_opacity: u8,
        coverage: u8,
    ) {
        if coverage == 0 || index + 3 >= material_pixels.len() {
            return;
        }
        let existing = material_pixels[index + 3] as u16;
        let src = coverage as u16;
        let out_alpha = (src + (existing * (255 - src)) / 255).min(255) as u8;
        let material = Self::iso_paint_material_pixel(
            material_id,
            replace_material.then_some(replace_opacity),
            out_alpha,
        );
        material_pixels[index..index + 4].copy_from_slice(&material);
    }

    fn iso_paint_clear_material_pixel_at(material_pixels: &mut [u8], index: usize, coverage: u8) {
        if coverage == 0 || index + 3 >= material_pixels.len() {
            return;
        }
        let keep = 255_u16.saturating_sub(coverage as u16);
        let next_alpha = ((material_pixels[index + 3] as u16 * keep) / 255) as u8;
        if next_alpha == 0 {
            material_pixels[index..index + 4]
                .copy_from_slice(&Self::iso_paint_material_pixel(0, None, 0));
        } else {
            material_pixels[index + 3] = next_alpha;
        }
    }

    fn iso_paint_blend_pixel_at(pixels: &mut [u8], index: usize, color: [u8; 4]) {
        if color[3] == 0 || index + 3 >= pixels.len() {
            return;
        }

        let src_a = color[3] as u32;
        let dst_a = pixels[index + 3] as u32;
        let inv_a = 255 - src_a;
        let out_a = (src_a + (dst_a * inv_a) / 255).min(255);
        if out_a == 0 {
            pixels[index..index + 4].copy_from_slice(&[0, 0, 0, 0]);
            return;
        }

        let denom = out_a * 255;
        pixels[index] = ((color[0] as u32 * src_a * 255 + pixels[index] as u32 * dst_a * inv_a)
            / denom)
            .min(255) as u8;
        pixels[index + 1] =
            ((color[1] as u32 * src_a * 255 + pixels[index + 1] as u32 * dst_a * inv_a) / denom)
                .min(255) as u8;
        pixels[index + 2] =
            ((color[2] as u32 * src_a * 255 + pixels[index + 2] as u32 * dst_a * inv_a) / denom)
                .min(255) as u8;
        pixels[index + 3] = out_a as u8;
    }

    fn iso_paint_coat_pixel_at(pixels: &mut [u8], index: usize, color: [u8; 4]) {
        if color[3] == 0 || index + 3 >= pixels.len() {
            return;
        }

        let src_a = color[3] as u32;
        let dst_a = pixels[index + 3] as u32;
        if dst_a == 0 || src_a >= dst_a {
            pixels[index] = color[0];
            pixels[index + 1] = color[1];
            pixels[index + 2] = color[2];
            pixels[index + 3] = color[3];
            return;
        }

        let keep_a = dst_a.saturating_sub(src_a);
        pixels[index] =
            ((color[0] as u32 * src_a + pixels[index] as u32 * keep_a) / dst_a).min(255) as u8;
        pixels[index + 1] =
            ((color[1] as u32 * src_a + pixels[index + 1] as u32 * keep_a) / dst_a).min(255) as u8;
        pixels[index + 2] =
            ((color[2] as u32 * src_a + pixels[index + 2] as u32 * keep_a) / dst_a).min(255) as u8;
        pixels[index + 3] = dst_a as u8;
    }

    fn iso_paint_write_overlay_pixel_at(pixels: &mut [u8], index: usize, color: [u8; 4]) {
        if color[3] == 0 || index + 3 >= pixels.len() || color[3] <= pixels[index + 3] {
            return;
        }

        pixels[index] = color[0];
        pixels[index + 1] = color[1];
        pixels[index + 2] = color[2];
        pixels[index + 3] = color[3];
    }

    fn iso_paint_color_coverage_scale(brush: &str, material_id: u8) -> f32 {
        let family = material_id / 4;
        if brush == "puddle" {
            1.0
        } else if matches!(family, 5 | 6) {
            0.12
        } else {
            1.0
        }
    }

    fn iso_paint_material_is_translucent(material_id: u8) -> bool {
        matches!(material_id / 4, 5 | 6)
    }

    fn iso_paint_alpha_geo_ids(
        material_pixels: &[u8],
        width: usize,
        height: usize,
        paint_surface: Option<&scenevm::PaintSurfaceBuffer>,
    ) -> Vec<scenevm::GeoId> {
        let Some(paint_surface) = paint_surface else {
            return Vec::new();
        };
        let mut seen = HashSet::new();
        let mut geo_ids = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) * 4;
                if index + 3 >= material_pixels.len()
                    || material_pixels[index] != 254
                    || material_pixels[index + 3] == 0
                {
                    continue;
                }
                let material_id = material_pixels[index + 1];
                let replace_mode = material_pixels[index + 2];
                let opaque_replace = replace_mode > 0
                    && replace_mode.saturating_sub(1) == 254
                    && !Self::iso_paint_material_is_translucent(material_id);
                if opaque_replace {
                    continue;
                }
                let Some(pixel) = paint_surface.pixel(x as i32, y as i32) else {
                    continue;
                };
                if pixel.valid && seen.insert(pixel.geo_id) {
                    geo_ids.push(pixel.geo_id);
                }
            }
        }
        geo_ids
    }

    fn iso_paint_set_material_pixel(
        material_pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        material_id: u8,
        replace_material: bool,
        replace_opacity: u8,
        coverage: u8,
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || coverage == 0 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= material_pixels.len() {
            return;
        }
        Self::iso_paint_set_material_pixel_at(
            material_pixels,
            index,
            material_id,
            replace_material,
            replace_opacity,
            coverage,
        );
    }

    fn iso_paint_set_stamp_material_pixel(
        material_pixels: &mut [u8],
        width: usize,
        height: usize,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        owner_geo_id: Option<scenevm::GeoId>,
        x: i32,
        y: i32,
        material_id: u8,
        coverage: u8,
    ) {
        if !Self::iso_paint_stamp_pixel_visible(surface_buffer, None, owner_geo_id, x, y) {
            return;
        }
        Self::iso_paint_set_material_pixel(
            material_pixels,
            width,
            height,
            x,
            y,
            material_id,
            true,
            254,
            coverage,
        );
    }

    fn iso_paint_clear_material_pixel(
        material_pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        coverage: u8,
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || coverage == 0 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= material_pixels.len() {
            return;
        }
        Self::iso_paint_clear_material_pixel_at(material_pixels, index, coverage);
    }

    fn iso_paint_blend_pixel(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || color[3] == 0 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= pixels.len() {
            return;
        }
        Self::iso_paint_blend_pixel_at(pixels, index, color);
    }

    fn iso_paint_coat_pixel(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || color[3] == 0 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= pixels.len() {
            return;
        }
        Self::iso_paint_coat_pixel_at(pixels, index, color);
    }

    fn iso_paint_write_coverage_pixel(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || color[3] == 0 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= pixels.len() || color[3] <= pixels[index + 3] {
            return;
        }
        Self::iso_paint_write_overlay_pixel_at(pixels, index, color);
    }

    fn iso_paint_write_overlay_pixel(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || color[3] == 0 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= pixels.len() || color[3] <= pixels[index + 3] {
            return;
        }

        pixels[index] = color[0];
        pixels[index + 1] = color[1];
        pixels[index + 2] = color[2];
        pixels[index + 3] = color[3];
    }

    fn iso_paint_stamp_coverage(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        local_x: i32,
        local_y: i32,
        radius: i32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        brush: &str,
        shape: &str,
        seed: u32,
    ) {
        let radius = radius.max(1);
        let sample = IsoPaintBrushSample {
            brush,
            shape,
            color,
            palette,
            opacity: 1.0,
            radius,
            seed,
        };
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let Some(shaped_color) = iso_paint_brush::sample_pixel(&sample, ox, oy) else {
                    continue;
                };
                Self::iso_paint_write_coverage_pixel(
                    pixels,
                    width,
                    height,
                    local_x + ox,
                    local_y + oy,
                    shaped_color,
                );
            }
        }
    }

    fn iso_paint_draw_segment_coverage(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        a: [i32; 2],
        b: [i32; 2],
        origin: [i32; 2],
        radius: i32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        brush: &str,
        shape: &str,
        seed: u32,
    ) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let distance = ((dx * dx + dy * dy) as f32).sqrt();
        let step_spacing = (radius as f32 * 0.35).clamp(1.0, 10.0);
        let steps = (distance / step_spacing).ceil().max(1.0) as i32;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = (a[0] as f32 + dx as f32 * t).round() as i32;
            let y = (a[1] as f32 + dy as f32 * t).round() as i32;
            Self::iso_paint_stamp_coverage(
                pixels,
                width,
                height,
                x - origin[0],
                y - origin[1],
                radius,
                color,
                palette,
                brush,
                shape,
                seed ^ (step as u32).wrapping_mul(0x27d4_eb2d),
            );
        }
    }

    fn iso_paint_sample_brick_color(
        pattern_x: f32,
        pattern_y: f32,
        base: [u8; 4],
        pattern_kind: &str,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
    ) -> [u8; 4] {
        let pattern_scale = pattern_scale.clamp(0.25, 4.0);
        let pattern_mortar = pattern_mortar.clamp(0.0, 0.4);
        let pattern_detail = pattern_detail.clamp(0.0, 1.0);
        let pattern_variation = pattern_variation.clamp(0.0, 1.0);
        let staggered = !matches!(pattern_kind, "tile" | "tiles");
        let brick_w = if staggered { 34.0 } else { 24.0 } * pattern_scale;
        let brick_h = if staggered { 17.0 } else { 24.0 } * pattern_scale;
        let mortar =
            (brick_w.min(brick_h) * pattern_mortar).clamp(0.0, brick_w.min(brick_h) * 0.45);

        let row = (pattern_y / brick_h).floor();
        let offset_x = if staggered && row as i32 & 1 != 0 {
            brick_w * 0.5
        } else {
            0.0
        };
        let local_x = (pattern_x + offset_x).rem_euclid(brick_w);
        let local_y = pattern_y.rem_euclid(brick_h);
        let col = ((pattern_x + offset_x) / brick_w).floor() as i32;
        let row_i = row as i32;

        let hash = |x: i32, y: i32, salt: i32| -> f32 {
            let mut n = x
                .wrapping_mul(374_761_393)
                .wrapping_add(y.wrapping_mul(668_265_263))
                .wrapping_add(salt.wrapping_mul(2_147_483_647));
            n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
            ((n ^ (n >> 16)) & 0xffff) as f32 / 65_535.0
        };

        if local_x < mortar || local_y < mortar {
            return [base[0], base[1], base[2], 0];
        }

        let edge_distance = local_x
            .min(local_y)
            .min(brick_w - local_x)
            .min(brick_h - local_y);
        let edge_wear = if edge_distance < mortar + 1.6 {
            1.0 - 0.12 * pattern_detail + hash(col, row_i, 31) * 0.06 * pattern_detail
        } else {
            1.0
        };
        let brick_variation = 1.0 + (hash(col, row_i, 11) - 0.5) * 0.44 * pattern_variation;
        let grain = 1.0
            + (hash(
                pattern_x.floor() as i32,
                pattern_y.floor() as i32,
                col.wrapping_mul(19) ^ row_i.wrapping_mul(23),
            ) - 0.5)
                * 0.20
                * pattern_detail;
        let hairline = if (local_y - mortar).abs() < 1.0 || (local_x - mortar).abs() < 0.8 {
            1.0 - 0.07 * pattern_detail
        } else {
            1.0
        };
        let shade = brick_variation * grain * edge_wear * hairline;
        [
            (base[0] as f32 * shade).clamp(0.0, 255.0) as u8,
            (base[1] as f32 * shade).clamp(0.0, 255.0) as u8,
            (base[2] as f32 * shade).clamp(0.0, 255.0) as u8,
            base[3],
        ]
    }

    fn iso_paint_sample_brick_surface_color(
        surface_uv: [f32; 2],
        base: [u8; 4],
        pattern_kind: &str,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
    ) -> [u8; 4] {
        let pixels_per_world = 42.0;
        Self::iso_paint_sample_brick_color(
            surface_uv[0] * pixels_per_world,
            surface_uv[1] * pixels_per_world,
            base,
            pattern_kind,
            pattern_scale,
            pattern_mortar,
            pattern_detail,
            pattern_variation,
        )
    }

    fn iso_paint_path_pattern_coord(
        screen: [i32; 2],
        path_points: &[[f32; 2]],
        path_lengths: &[f32],
        origin: [i32; 2],
        scale: f32,
    ) -> Option<[f32; 2]> {
        if path_points.len() < 2 || path_lengths.len() != path_points.len() {
            return None;
        }

        let px = screen[0] as f32;
        let py = screen[1] as f32;
        let scale = scale.clamp(0.05, 20.0);
        let mut best: Option<(f32, f32, f32)> = None;

        for index in 0..path_points.len().saturating_sub(1) {
            let a = path_points[index];
            let b = path_points[index + 1];
            let ax = origin[0] as f32 + a[0] * scale;
            let ay = origin[1] as f32 + a[1] * scale;
            let bx = origin[0] as f32 + b[0] * scale;
            let by = origin[1] as f32 + b[1] * scale;
            let vx = bx - ax;
            let vy = by - ay;
            let len2 = vx * vx + vy * vy;
            if len2 <= f32::EPSILON {
                continue;
            }
            let t = (((px - ax) * vx + (py - ay) * vy) / len2).clamp(0.0, 1.0);
            let qx = ax + vx * t;
            let qy = ay + vy * t;
            let dx = px - qx;
            let dy = py - qy;
            let dist2 = dx * dx + dy * dy;
            let segment_len = len2.sqrt();
            let along = path_lengths[index] * scale + segment_len * t;
            let signed_across = (vx * dy - vy * dx).signum() * dist2.sqrt();
            if best.map_or(true, |(best_dist2, _, _)| dist2 < best_dist2) {
                best = Some((dist2, along, signed_across));
            }
        }

        best.map(|(_, along, across)| [along, across])
    }

    fn iso_paint_arch_pattern_coord(
        screen: [i32; 2],
        path_points: &[[f32; 2]],
        path_lengths: &[f32],
        origin: [i32; 2],
        scale: f32,
    ) -> Option<[f32; 2]> {
        let coord =
            Self::iso_paint_path_pattern_coord(screen, path_points, path_lengths, origin, scale)?;
        Some([coord[0], coord[1] + 8192.0])
    }

    fn iso_paint_sample_arch_brick_color(
        screen: [i32; 2],
        path_points: &[[f32; 2]],
        path_lengths: &[f32],
        origin: [i32; 2],
        scale: f32,
        base: [u8; 4],
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
    ) -> Option<[u8; 4]> {
        let coord =
            Self::iso_paint_arch_pattern_coord(screen, path_points, path_lengths, origin, scale)?;
        Some(Self::iso_paint_sample_brick_color(
            coord[0],
            coord[1],
            base,
            "tile",
            pattern_scale,
            pattern_mortar,
            pattern_detail,
            pattern_variation,
        ))
    }

    fn iso_paint_geo_object_matches(a: scenevm::GeoId, b: scenevm::GeoId) -> bool {
        match (a, b) {
            (scenevm::GeoId::GeometryObject(a), scenevm::GeoId::GeometryObject(b)) => a == b,
            (scenevm::GeoId::Sector(a), scenevm::GeoId::Sector(b)) => a == b,
            (scenevm::GeoId::Terrain(..), scenevm::GeoId::Terrain(..)) => true,
            (scenevm::GeoId::Character(a), scenevm::GeoId::Character(b)) => a == b,
            (scenevm::GeoId::Item(a), scenevm::GeoId::Item(b)) => a == b,
            (scenevm::GeoId::Triangle(a), scenevm::GeoId::Triangle(b)) => a == b,
            _ => a == b,
        }
    }

    fn iso_paint_owner_geo_id(owner: &IsoPaintOwner) -> scenevm::GeoId {
        match owner {
            IsoPaintOwner::Unknown(id) => scenevm::GeoId::Unknown(*id),
            IsoPaintOwner::Vertex(id) => scenevm::GeoId::Vertex(*id),
            IsoPaintOwner::Linedef(id) => scenevm::GeoId::Linedef(*id),
            IsoPaintOwner::Sector(id) => scenevm::GeoId::Sector(*id),
            IsoPaintOwner::Character(id) => scenevm::GeoId::Character(*id),
            IsoPaintOwner::Item(id) => scenevm::GeoId::Item(*id),
            IsoPaintOwner::Light(id) => scenevm::GeoId::Light(*id),
            IsoPaintOwner::ItemLight(id) => scenevm::GeoId::ItemLight(*id),
            IsoPaintOwner::Triangle(id) => scenevm::GeoId::Triangle(*id),
            IsoPaintOwner::Terrain { x, z } => scenevm::GeoId::Terrain(*x, *z),
            IsoPaintOwner::GeometryObject(id) => scenevm::GeoId::GeometryObject(*id),
            IsoPaintOwner::Hole { sector_id, hole_id } => {
                scenevm::GeoId::Hole(*sector_id, *hole_id)
            }
            IsoPaintOwner::Gizmo(id) => scenevm::GeoId::Gizmo(*id),
        }
    }

    fn iso_paint_start_clip_geo_id(
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        clip_geo_id: Option<scenevm::GeoId>,
        start_screen: Option<[i32; 2]>,
    ) -> Option<scenevm::GeoId> {
        if clip == "none" {
            return None;
        }
        if clip_geo_id.is_some() {
            return clip_geo_id;
        }
        let start_screen = start_screen?;
        surface_buffer?
            .pixel(start_screen[0], start_screen[1])
            .copied()
            .filter(|pixel| pixel.valid)
            .map(|pixel| pixel.geo_id)
    }

    fn iso_paint_brush_clip_geo_id(
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        clip_geo_id: Option<scenevm::GeoId>,
        start_screen: Option<[i32; 2]>,
        paint: &TheRGBABuffer,
        draw_origin: [i32; 2],
        scale: f32,
    ) -> Option<scenevm::GeoId> {
        let center_geo_id =
            Self::iso_paint_start_clip_geo_id(surface_buffer, clip, clip_geo_id, start_screen);
        if clip == "none" {
            return None;
        }
        if let Some(stored_geo_id) = clip_geo_id {
            return Some(stored_geo_id);
        }

        let Some(surface_buffer) = surface_buffer else {
            return center_geo_id;
        };
        let paint_dim = *paint.dim();
        if paint_dim.width <= 0 || paint_dim.height <= 0 {
            return center_geo_id;
        }

        let scale = scale.clamp(0.05, 20.0);
        let paint_w = paint_dim.width as usize;
        let paint_h = paint_dim.height as usize;
        let draw_w = ((paint_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((paint_dim.height as f32) * scale).round().max(1.0) as usize;
        let paint_pixels = paint.pixels();
        let mut weights: HashMap<scenevm::GeoId, usize> = HashMap::new();
        for gy in 0..draw_h {
            let sy = ((gy as f32) / scale).floor() as usize;
            if sy >= paint_h {
                continue;
            }
            let dst_y = draw_origin[1] + gy as i32;
            for gx in 0..draw_w {
                let sx = ((gx as f32) / scale).floor() as usize;
                if sx >= paint_w {
                    continue;
                }
                let src_index = (sy * paint_w + sx) * 4;
                let Some(alpha) = paint_pixels.get(src_index + 3).copied() else {
                    continue;
                };
                if alpha == 0 {
                    continue;
                }
                let dst_x = draw_origin[0] + gx as i32;
                if let Some(pixel) = surface_buffer.pixel(dst_x, dst_y)
                    && pixel.valid
                {
                    *weights.entry(pixel.geo_id).or_insert(0) += alpha as usize;
                }
            }
        }

        let dominant = weights
            .iter()
            .max_by_key(|(_, weight)| *weight)
            .map(|(geo_id, weight)| (*geo_id, *weight));
        let Some((dominant_geo_id, dominant_weight)) = dominant else {
            return center_geo_id;
        };
        let center_weight = center_geo_id
            .and_then(|geo_id| weights.get(&geo_id).copied())
            .unwrap_or(0);

        let chosen = if center_weight == 0 || dominant_weight > center_weight.saturating_mul(2) {
            Some(dominant_geo_id)
        } else {
            center_geo_id
        };
        chosen
    }

    fn iso_paint_clip_allows(
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        start_geo_id: Option<scenevm::GeoId>,
        x: i32,
        y: i32,
    ) -> bool {
        match clip {
            "none" => true,
            _ => {
                let Some(start_geo_id) = start_geo_id else {
                    return false;
                };
                surface_buffer
                    .and_then(|surface| surface.pixel(x, y))
                    .is_some_and(|pixel| {
                        pixel.valid
                            && Self::iso_paint_geo_object_matches(start_geo_id, pixel.geo_id)
                    })
            }
        }
    }

    fn iso_paint_collect_material_geo_ids(
        paint_surface: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        start_geo_id: Option<scenevm::GeoId>,
        paint: &TheRGBABuffer,
        draw_origin: [i32; 2],
        scale: f32,
        seen: &mut HashSet<scenevm::GeoId>,
        geo_ids: &mut Vec<scenevm::GeoId>,
    ) {
        let Some(surface_buffer) = paint_surface else {
            if let Some(geo_id) = start_geo_id
                && seen.insert(geo_id)
            {
                geo_ids.push(geo_id);
            }
            return;
        };
        let paint_dim = *paint.dim();
        if paint_dim.width <= 0 || paint_dim.height <= 0 {
            return;
        }
        let scale = scale.clamp(0.05, 20.0);
        let paint_w = paint_dim.width as usize;
        let paint_h = paint_dim.height as usize;
        let draw_w = ((paint_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((paint_dim.height as f32) * scale).round().max(1.0) as usize;
        let paint_pixels = paint.pixels();

        for gy in 0..draw_h {
            let sy = ((gy as f32) / scale).floor() as usize;
            if sy >= paint_h {
                continue;
            }
            let dst_y = draw_origin[1] + gy as i32;
            for gx in 0..draw_w {
                let sx = ((gx as f32) / scale).floor() as usize;
                if sx >= paint_w {
                    continue;
                }
                let src_index = (sy * paint_w + sx) * 4;
                if paint_pixels.get(src_index + 3).copied().unwrap_or(0) == 0
                    || !Self::iso_paint_clip_allows(
                        Some(surface_buffer),
                        clip,
                        start_geo_id,
                        draw_origin[0] + gx as i32,
                        dst_y,
                    )
                {
                    continue;
                }
                let dst_x = draw_origin[0] + gx as i32;
                if let Some(pixel) = surface_buffer.pixel(dst_x, dst_y)
                    && pixel.valid
                    && seen.insert(pixel.geo_id)
                {
                    geo_ids.push(pixel.geo_id);
                }
            }
        }
    }

    fn iso_paint_composite_overlay_scaled_at(
        target: &mut TheRGBABuffer,
        material_pixels: &mut [u8],
        paint: &TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        material_id: u8,
        start_screen: Option<[i32; 2]>,
        clip_geo_id: Option<scenevm::GeoId>,
        color_coverage_scale: f32,
        replace_material: bool,
        replace_opacity: u8,
        writes_material: bool,
        x: i32,
        y: i32,
        scale: f32,
    ) {
        let target_dim = *target.dim();
        let paint_dim = *paint.dim();
        if target_dim.width <= 0
            || target_dim.height <= 0
            || paint_dim.width <= 0
            || paint_dim.height <= 0
        {
            return;
        }

        let scale = scale.clamp(0.05, 20.0);
        let target_w = target_dim.width as usize;
        let target_h = target_dim.height as usize;
        let paint_w = paint_dim.width as usize;
        let paint_h = paint_dim.height as usize;
        let draw_w = ((paint_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((paint_dim.height as f32) * scale).round().max(1.0) as usize;
        let target_pixels = target.pixels_mut();
        let paint_pixels = paint.pixels();
        let start_geo_id = Self::iso_paint_brush_clip_geo_id(
            surface_buffer,
            clip,
            clip_geo_id,
            start_screen,
            paint,
            [x, y],
            scale,
        );
        let draw_area = draw_w.saturating_mul(draw_h);

        if draw_area >= ISO_PAINT_PAR_COMPOSITE_PIXELS {
            let row_stride = target_w * 4;
            let color_coverage_scale = color_coverage_scale.clamp(0.0, 1.0);
            target_pixels
                .par_chunks_exact_mut(row_stride)
                .zip(material_pixels.par_chunks_exact_mut(row_stride))
                .enumerate()
                .for_each(|(dy, (target_row, material_row))| {
                    let dy = dy as i32;
                    let dy_local = dy - y;
                    if dy_local < 0 || dy_local >= draw_h as i32 {
                        return;
                    }
                    let sy = ((dy_local as f32) / scale).floor() as usize;
                    if sy >= paint_h {
                        return;
                    }

                    let dx_start = x.max(0);
                    let dx_end = (x + draw_w as i32).min(target_dim.width);
                    for dx in dx_start..dx_end {
                        let dx_local = dx - x;
                        let sx = ((dx_local as f32) / scale).floor() as usize;
                        if sx >= paint_w
                            || !Self::iso_paint_clip_allows(
                                surface_buffer,
                                clip,
                                start_geo_id,
                                dx,
                                dy,
                            )
                        {
                            continue;
                        }

                        let src_index = (sy * paint_w + sx) * 4;
                        if src_index + 3 >= paint_pixels.len() {
                            continue;
                        }
                        let src = [
                            paint_pixels[src_index],
                            paint_pixels[src_index + 1],
                            paint_pixels[src_index + 2],
                            paint_pixels[src_index + 3],
                        ];
                        if src[3] == 0 {
                            continue;
                        }

                        let row_index = dx as usize * 4;
                        let mut color_src = src;
                        color_src[3] = ((color_src[3] as f32 * color_coverage_scale)
                            .round()
                            .clamp(0.0, 255.0)) as u8;
                        if color_src[3] > 0 && replace_material {
                            Self::iso_paint_write_overlay_pixel_at(
                                target_row, row_index, color_src,
                            );
                        } else if color_src[3] > 0 {
                            Self::iso_paint_coat_pixel_at(target_row, row_index, color_src);
                        }
                        if writes_material {
                            Self::iso_paint_set_material_pixel_at(
                                material_row,
                                row_index,
                                material_id,
                                replace_material,
                                replace_opacity,
                                src[3],
                            );
                        }
                    }
                });
            return;
        }

        for dy_local in 0..draw_h {
            let dy = y + dy_local as i32;
            if dy < 0 || dy >= target_dim.height {
                continue;
            }
            let sy = ((dy_local as f32) / scale).floor() as usize;
            if sy >= paint_h {
                continue;
            }
            for dx_local in 0..draw_w {
                let dx = x + dx_local as i32;
                if dx < 0 || dx >= target_dim.width {
                    continue;
                }
                let sx = ((dx_local as f32) / scale).floor() as usize;
                if sx >= paint_w {
                    continue;
                }
                if !Self::iso_paint_clip_allows(surface_buffer, clip, start_geo_id, dx, dy) {
                    continue;
                }

                let src_index = (sy * paint_w + sx) * 4;
                if src_index + 3 >= paint_pixels.len() {
                    continue;
                }
                let src = [
                    paint_pixels[src_index],
                    paint_pixels[src_index + 1],
                    paint_pixels[src_index + 2],
                    paint_pixels[src_index + 3],
                ];
                if src[3] == 0 {
                    continue;
                }
                let mut color_src = src;
                color_src[3] = ((color_src[3] as f32 * color_coverage_scale.clamp(0.0, 1.0))
                    .round()
                    .clamp(0.0, 255.0)) as u8;
                if color_src[3] > 0 && replace_material {
                    Self::iso_paint_write_overlay_pixel(
                        target_pixels,
                        target_w,
                        target_h,
                        dx,
                        dy,
                        color_src,
                    );
                } else if color_src[3] > 0 {
                    Self::iso_paint_coat_pixel(
                        target_pixels,
                        target_w,
                        target_h,
                        dx,
                        dy,
                        color_src,
                    );
                }
                if writes_material {
                    Self::iso_paint_set_material_pixel(
                        material_pixels,
                        target_w,
                        target_h,
                        dx,
                        dy,
                        material_id,
                        replace_material,
                        replace_opacity,
                        src[3],
                    );
                }
            }
        }
    }

    fn iso_paint_composite_brick_overlay_scaled_at(
        target: &mut TheRGBABuffer,
        material_pixels: &mut [u8],
        mask: &TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        material_id: u8,
        start_screen: Option<[i32; 2]>,
        clip_geo_id: Option<scenevm::GeoId>,
        replace_material: bool,
        replace_opacity: u8,
        x: i32,
        y: i32,
        scale: f32,
        base: [u8; 4],
        pattern_kind: &str,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
        path_points: &[[f32; 2]],
        path_lengths: &[f32],
    ) {
        let target_dim = *target.dim();
        let mask_dim = *mask.dim();
        if target_dim.width <= 0
            || target_dim.height <= 0
            || mask_dim.width <= 0
            || mask_dim.height <= 0
        {
            return;
        }

        let Some(surface_buffer) = surface_buffer else {
            return;
        };

        let scale = scale.clamp(0.05, 20.0);
        let target_w = target_dim.width as usize;
        let target_h = target_dim.height as usize;
        let mask_w = mask_dim.width as usize;
        let mask_h = mask_dim.height as usize;
        let draw_w = ((mask_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((mask_dim.height as f32) * scale).round().max(1.0) as usize;
        let target_pixels = target.pixels_mut();
        let mask_pixels = mask.pixels();
        let start_geo_id = Self::iso_paint_brush_clip_geo_id(
            Some(surface_buffer),
            clip,
            clip_geo_id,
            start_screen,
            mask,
            [x, y],
            scale,
        );
        let draw_area = draw_w.saturating_mul(draw_h);

        if draw_area >= ISO_PAINT_PAR_COMPOSITE_PIXELS {
            let row_stride = target_w * 4;
            target_pixels
                .par_chunks_exact_mut(row_stride)
                .zip(material_pixels.par_chunks_exact_mut(row_stride))
                .enumerate()
                .for_each(|(dy, (target_row, material_row))| {
                    let dy = dy as i32;
                    let dy_local = dy - y;
                    if dy_local < 0 || dy_local >= draw_h as i32 {
                        return;
                    }
                    let sy = ((dy_local as f32) / scale).floor() as usize;
                    if sy >= mask_h {
                        return;
                    }

                    let dx_start = x.max(0);
                    let dx_end = (x + draw_w as i32).min(target_dim.width);
                    for dx in dx_start..dx_end {
                        let dx_local = dx - x;
                        let sx = ((dx_local as f32) / scale).floor() as usize;
                        if sx >= mask_w
                            || !Self::iso_paint_clip_allows(
                                Some(surface_buffer),
                                clip,
                                start_geo_id,
                                dx,
                                dy,
                            )
                        {
                            continue;
                        }

                        let src_index = (sy * mask_w + sx) * 4;
                        if src_index + 3 >= mask_pixels.len() {
                            continue;
                        }
                        let mask_alpha = mask_pixels[src_index + 3];
                        if mask_alpha == 0 {
                            continue;
                        }
                        let Some(surface_pixel) =
                            surface_buffer.pixel(dx, dy).filter(|pixel| pixel.valid)
                        else {
                            continue;
                        };
                        let mut color = if matches!(pattern_kind, "arch" | "trim") {
                            Self::iso_paint_sample_arch_brick_color(
                                [dx, dy],
                                path_points,
                                path_lengths,
                                [x, y],
                                scale,
                                base,
                                pattern_scale,
                                pattern_mortar,
                                pattern_detail,
                                pattern_variation,
                            )
                            .unwrap_or_else(|| {
                                Self::iso_paint_sample_brick_surface_color(
                                    surface_pixel.uv,
                                    base,
                                    "brick",
                                    pattern_scale,
                                    pattern_mortar,
                                    pattern_detail,
                                    pattern_variation,
                                )
                            })
                        } else {
                            Self::iso_paint_sample_brick_surface_color(
                                surface_pixel.uv,
                                base,
                                pattern_kind,
                                pattern_scale,
                                pattern_mortar,
                                pattern_detail,
                                pattern_variation,
                            )
                        };
                        let color_alpha = ((color[3] as u16 * mask_alpha as u16) / 255) as u8;
                        color[3] = if replace_material {
                            ((color_alpha as u16 * replace_opacity as u16) / 254) as u8
                        } else {
                            color_alpha
                        };

                        let row_index = dx as usize * 4;
                        if color[3] > 0 {
                            if replace_material {
                                Self::iso_paint_write_overlay_pixel_at(
                                    target_row, row_index, color,
                                );
                            } else {
                                Self::iso_paint_coat_pixel_at(target_row, row_index, color);
                            }
                        }
                        Self::iso_paint_set_material_pixel_at(
                            material_row,
                            row_index,
                            material_id,
                            replace_material,
                            replace_opacity,
                            mask_alpha,
                        );
                    }
                });
            return;
        }

        for dy_local in 0..draw_h {
            let dy = y + dy_local as i32;
            if dy < 0 || dy >= target_dim.height {
                continue;
            }
            let sy = ((dy_local as f32) / scale).floor() as usize;
            if sy >= mask_h {
                continue;
            }
            for dx_local in 0..draw_w {
                let dx = x + dx_local as i32;
                if dx < 0 || dx >= target_dim.width {
                    continue;
                }
                let sx = ((dx_local as f32) / scale).floor() as usize;
                if sx >= mask_w {
                    continue;
                }
                if !Self::iso_paint_clip_allows(Some(surface_buffer), clip, start_geo_id, dx, dy) {
                    continue;
                }

                let src_index = (sy * mask_w + sx) * 4;
                if src_index + 3 >= mask_pixels.len() {
                    continue;
                }
                let mask_alpha = mask_pixels[src_index + 3];
                if mask_alpha == 0 {
                    continue;
                }
                let Some(surface_pixel) = surface_buffer.pixel(dx, dy).filter(|pixel| pixel.valid)
                else {
                    continue;
                };
                let mut color = if matches!(pattern_kind, "arch" | "trim") {
                    Self::iso_paint_sample_arch_brick_color(
                        [dx, dy],
                        path_points,
                        path_lengths,
                        [x, y],
                        scale,
                        base,
                        pattern_scale,
                        pattern_mortar,
                        pattern_detail,
                        pattern_variation,
                    )
                    .unwrap_or_else(|| {
                        Self::iso_paint_sample_brick_surface_color(
                            surface_pixel.uv,
                            base,
                            "brick",
                            pattern_scale,
                            pattern_mortar,
                            pattern_detail,
                            pattern_variation,
                        )
                    })
                } else {
                    Self::iso_paint_sample_brick_surface_color(
                        surface_pixel.uv,
                        base,
                        pattern_kind,
                        pattern_scale,
                        pattern_mortar,
                        pattern_detail,
                        pattern_variation,
                    )
                };
                let color_alpha = ((color[3] as u16 * mask_alpha as u16) / 255) as u8;
                color[3] = if replace_material {
                    ((color_alpha as u16 * replace_opacity as u16) / 254) as u8
                } else {
                    color_alpha
                };
                if color[3] > 0 {
                    if replace_material {
                        Self::iso_paint_write_overlay_pixel(
                            target_pixels,
                            target_w,
                            target_h,
                            dx,
                            dy,
                            color,
                        );
                    } else {
                        Self::iso_paint_coat_pixel(
                            target_pixels,
                            target_w,
                            target_h,
                            dx,
                            dy,
                            color,
                        );
                    }
                }
                Self::iso_paint_set_material_pixel(
                    material_pixels,
                    target_w,
                    target_h,
                    dx,
                    dy,
                    material_id,
                    replace_material,
                    replace_opacity,
                    mask_alpha,
                );
            }
        }
    }

    fn iso_paint_clear_overlay_scaled_at(
        target: &mut TheRGBABuffer,
        material_pixels: &mut [u8],
        mask: &TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        start_screen: Option<[i32; 2]>,
        clip_geo_id: Option<scenevm::GeoId>,
        clears_material: bool,
        x: i32,
        y: i32,
        scale: f32,
    ) {
        let target_dim = *target.dim();
        let mask_dim = *mask.dim();
        if target_dim.width <= 0
            || target_dim.height <= 0
            || mask_dim.width <= 0
            || mask_dim.height <= 0
        {
            return;
        }

        let scale = scale.clamp(0.05, 20.0);
        let target_w = target_dim.width as usize;
        let target_h = target_dim.height as usize;
        let mask_w = mask_dim.width as usize;
        let mask_h = mask_dim.height as usize;
        let draw_w = ((mask_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((mask_dim.height as f32) * scale).round().max(1.0) as usize;
        let target_pixels = target.pixels_mut();
        let mask_pixels = mask.pixels();
        let start_geo_id = Self::iso_paint_brush_clip_geo_id(
            surface_buffer,
            clip,
            clip_geo_id,
            start_screen,
            mask,
            [x, y],
            scale,
        );
        let draw_area = draw_w.saturating_mul(draw_h);

        if draw_area >= ISO_PAINT_PAR_COMPOSITE_PIXELS {
            let row_stride = target_w * 4;
            target_pixels
                .par_chunks_exact_mut(row_stride)
                .zip(material_pixels.par_chunks_exact_mut(row_stride))
                .enumerate()
                .for_each(|(dy, (target_row, material_row))| {
                    let dy = dy as i32;
                    let dy_local = dy - y;
                    if dy_local < 0 || dy_local >= draw_h as i32 {
                        return;
                    }
                    let sy = ((dy_local as f32) / scale).floor() as usize;
                    if sy >= mask_h {
                        return;
                    }

                    let dx_start = x.max(0);
                    let dx_end = (x + draw_w as i32).min(target_dim.width);
                    for dx in dx_start..dx_end {
                        let dx_local = dx - x;
                        let sx = ((dx_local as f32) / scale).floor() as usize;
                        if sx >= mask_w
                            || !Self::iso_paint_clip_allows(
                                surface_buffer,
                                clip,
                                start_geo_id,
                                dx,
                                dy,
                            )
                        {
                            continue;
                        }

                        let src_index = (sy * mask_w + sx) * 4;
                        if src_index + 3 >= mask_pixels.len() {
                            continue;
                        }
                        let mask_a = mask_pixels[src_index + 3] as u16;
                        if mask_a == 0 {
                            continue;
                        }
                        let row_index = dx as usize * 4;
                        if row_index + 3 >= target_row.len() {
                            continue;
                        }
                        let keep = 255 - mask_a;
                        target_row[row_index + 3] =
                            ((target_row[row_index + 3] as u16 * keep) / 255) as u8;
                        if clears_material {
                            Self::iso_paint_clear_material_pixel_at(
                                material_row,
                                row_index,
                                mask_pixels[src_index + 3],
                            );
                        }
                    }
                });
            return;
        }

        for dy_local in 0..draw_h {
            let dy = y + dy_local as i32;
            if dy < 0 || dy >= target_dim.height {
                continue;
            }
            let sy = ((dy_local as f32) / scale).floor() as usize;
            if sy >= mask_h {
                continue;
            }
            for dx_local in 0..draw_w {
                let dx = x + dx_local as i32;
                if dx < 0 || dx >= target_dim.width {
                    continue;
                }
                let sx = ((dx_local as f32) / scale).floor() as usize;
                if sx >= mask_w {
                    continue;
                }
                if !Self::iso_paint_clip_allows(surface_buffer, clip, start_geo_id, dx, dy) {
                    continue;
                }

                let src_index = (sy * mask_w + sx) * 4;
                let dst_index = (dy as usize * target_w + dx as usize) * 4;
                if src_index + 3 >= mask_pixels.len() || dst_index + 3 >= target_pixels.len() {
                    continue;
                }
                let mask_a = mask_pixels[src_index + 3] as u16;
                if mask_a == 0 {
                    continue;
                }
                let keep = 255 - mask_a;
                target_pixels[dst_index + 3] =
                    ((target_pixels[dst_index + 3] as u16 * keep) / 255) as u8;
                if clears_material {
                    Self::iso_paint_clear_material_pixel(
                        material_pixels,
                        target_w,
                        target_h,
                        dx,
                        dy,
                        mask_pixels[src_index + 3],
                    );
                }
            }
        }
    }

    fn iso_paint_preview_color(layer: &IsoPaintLayer) -> [u8; 4] {
        match layer.active_operation.as_str() {
            "erase" => [242, 92, 84, 230],
            "pick" => [87, 186, 255, 230],
            _ => {
                let mut color = layer.active_color;
                color[3] = 230;
                color
            }
        }
    }

    fn draw_iso_paint_preview(
        buffer: &mut TheRGBABuffer,
        layer: &IsoPaintLayer,
        hover: Option<Vec2<i32>>,
    ) {
        if !layer.visible || layer.active_operation == "pick" && hover.is_none() {
            return;
        }

        let Some(hover) = hover else {
            return;
        };
        let dim = *buffer.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return;
        }

        let radius = (layer.active_size * 2.0).round().clamp(3.0, 96.0) as i32;
        let outer = radius + 2;
        let radius_sq = radius * radius;
        let inner_sq = (radius - 2).max(1).pow(2);
        let shadow_sq = outer * outer;
        let color = Self::iso_paint_preview_color(layer);
        let fill = [color[0], color[1], color[2], 38];
        let shadow = [8, 10, 12, 145];
        let pixels = buffer.pixels_mut();
        let width = dim.width as usize;
        let height = dim.height as usize;

        for oy in -outer..=outer {
            for ox in -outer..=outer {
                let d = ox * ox + oy * oy;
                let x = hover.x + ox;
                let y = hover.y + oy;
                if d <= shadow_sq && d > radius_sq {
                    Self::iso_paint_blend_pixel(pixels, width, height, x, y, shadow);
                } else if d <= radius_sq && d >= inner_sq {
                    Self::iso_paint_blend_pixel(pixels, width, height, x, y, color);
                } else if d < inner_sq && layer.active_operation != "pick" {
                    Self::iso_paint_blend_pixel(pixels, width, height, x, y, fill);
                }
            }
        }
    }

    fn iso_paint_project_world(
        point: [f32; 3],
        view: Mat4<f32>,
        proj: Mat4<f32>,
        width: i32,
        height: i32,
    ) -> Option<[i32; 2]> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let clip = (proj * view) * Vec4::new(point[0], point[1], point[2], 1.0);
        if clip.w.abs() <= f32::EPSILON {
            return None;
        }
        let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
        Some([
            ((ndc.x * 0.5 + 0.5) * width as f32).round() as i32,
            ((1.0 - (ndc.y * 0.5 + 0.5)) * height as f32).round() as i32,
        ])
    }

    fn iso_paint_blend_line(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: [u8; 4],
    ) {
        let dim = *buffer.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return;
        }
        let width = dim.width as usize;
        let height = dim.height as usize;
        let pixels = buffer.pixels_mut();
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if Self::iso_paint_stamp_pixel_visible(surface_buffer, stamp_depth, owner_geo_id, x, y)
            {
                Self::iso_paint_blend_lit_stamp_pixel(pixels, width, height, x, y, color);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn iso_paint_world_depth(point: [f32; 3], camera: scenevm::Camera3D) -> Option<f32> {
        let point = Vec3::new(point[0], point[1], point[2]);
        let depth = (point - camera.pos).dot(camera.forward);
        (depth.is_finite() && depth > camera.near && depth < camera.far).then_some(depth)
    }

    fn iso_paint_stamp_lit_color(
        pixels: &[u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) -> [u8; 4] {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || color[3] == 0 {
            return color;
        }
        let sample_luma = |sx: i32, sy: i32| -> f32 {
            if sx < 0 || sy < 0 || sx as usize >= width || sy as usize >= height {
                return 0.0;
            }
            let index = (sy as usize * width + sx as usize) * 4;
            if index + 2 >= pixels.len() {
                return 0.0;
            }
            (pixels[index] as f32 * 0.2126
                + pixels[index + 1] as f32 * 0.7152
                + pixels[index + 2] as f32 * 0.0722)
                / 255.0
        };

        let local_offsets = [
            (0, 0),
            (-24, 0),
            (24, 0),
            (0, -18),
            (0, 18),
            (-36, -24),
            (36, -24),
            (-36, 24),
            (36, 24),
        ];
        let local_luma = local_offsets
            .iter()
            .map(|(ox, oy)| sample_luma(x + ox, y + oy))
            .fold(0.0_f32, f32::max);

        let width_i = width as i32;
        let height_i = height as i32;
        let broad_points = [
            (width_i / 2, height_i / 2),
            (width_i / 4, height_i / 4),
            (width_i * 3 / 4, height_i / 4),
            (width_i / 4, height_i * 3 / 4),
            (width_i * 3 / 4, height_i * 3 / 4),
            (width_i / 2, height_i / 4),
            (width_i / 2, height_i * 3 / 4),
            (width_i / 4, height_i / 2),
            (width_i * 3 / 4, height_i / 2),
        ];
        let broad_luma = broad_points
            .iter()
            .map(|(sx, sy)| sample_luma(*sx, *sy))
            .sum::<f32>()
            / broad_points.len() as f32;

        let global_light = (0.30 + broad_luma * 1.35).clamp(0.34, 1.08);
        let local_light = (0.30 + local_luma * 1.35).clamp(0.34, 1.08);
        let mut light = if local_light < global_light {
            let ratio = (local_light / global_light.max(0.001)).clamp(0.0, 1.0);
            global_light * (0.86 + ratio * 0.14)
        } else {
            (global_light * 0.75 + local_light * 0.25).min(1.08)
        };

        if color[0] > 220 && color[1] > 120 && color[2] < 130 {
            light = light.max(0.72);
        } else if color[0] > 220 && color[1] > 210 && color[2] > 90 {
            light = light.max(0.82);
        }

        [
            (color[0] as f32 * light).round().clamp(0.0, 255.0) as u8,
            (color[1] as f32 * light).round().clamp(0.0, 255.0) as u8,
            (color[2] as f32 * light).round().clamp(0.0, 255.0) as u8,
            color[3],
        ]
    }

    fn iso_paint_blend_lit_stamp_pixel(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        let color = Self::iso_paint_stamp_lit_color(pixels, width, height, x, y, color);
        Self::iso_paint_blend_pixel(pixels, width, height, x, y, color);
    }

    fn iso_paint_stamp_pixel_visible(
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        x: i32,
        y: i32,
    ) -> bool {
        let Some(surface_pixel) = surface_buffer.and_then(|surface| surface.pixel(x, y)) else {
            return true;
        };
        if !surface_pixel.valid {
            return true;
        }
        if let Some(stamp_depth) = stamp_depth {
            return surface_pixel.depth + 0.12 >= stamp_depth;
        };
        owner_geo_id.is_none_or(|owner_geo_id| {
            Self::iso_paint_geo_object_matches(owner_geo_id, surface_pixel.geo_id)
        })
    }

    fn iso_paint_adjust_rgb(color: [u8; 4], amount: f32) -> [u8; 4] {
        [
            (color[0] as f32 * amount).round().clamp(0.0, 255.0) as u8,
            (color[1] as f32 * amount).round().clamp(0.0, 255.0) as u8,
            (color[2] as f32 * amount).round().clamp(0.0, 255.0) as u8,
            color[3],
        ]
    }

    fn iso_paint_blend_stamp_pixel(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if !Self::iso_paint_stamp_pixel_visible(surface_buffer, stamp_depth, owner_geo_id, x, y) {
            return;
        }
        let dim = *buffer.dim();
        let width = dim.width.max(0) as usize;
        let height = dim.height.max(0) as usize;
        Self::iso_paint_blend_lit_stamp_pixel(buffer.pixels_mut(), width, height, x, y, color);
    }

    fn draw_iso_paint_rotated_ellipse(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        center: [i32; 2],
        radius_major: f32,
        radius_minor: f32,
        angle: f32,
        color: [u8; 4],
        variation: u32,
    ) {
        let radius_major = radius_major.max(1.0);
        let radius_minor = radius_minor.max(1.0);
        let cos = angle.cos();
        let sin = angle.sin();
        let bound = (radius_major.max(radius_minor) + 1.0).ceil() as i32;
        for y in -bound..=bound {
            for x in -bound..=bound {
                let lx = x as f32 * cos + y as f32 * sin;
                let ly = -x as f32 * sin + y as f32 * cos;
                let edge = lx * lx / (radius_major * radius_major)
                    + ly * ly / (radius_minor * radius_minor);
                if edge > 1.0 {
                    continue;
                }
                let hash = iso_paint_brush::hash_u32(center[0] + x, center[1] + y, variation);
                let noise = (hash & 0xff) as f32 / 255.0;
                let shade = if ly < -radius_minor * 0.35 {
                    1.08 + noise * 0.14
                } else if edge > 0.78 || ly > radius_minor * 0.45 {
                    0.62 + noise * 0.18
                } else {
                    0.82 + noise * 0.20
                };
                let mut pixel = Self::iso_paint_adjust_rgb(color, shade);
                if edge > 0.9 {
                    pixel[3] = ((pixel[3] as f32) * 0.65).round() as u8;
                }
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + x,
                    center[1] + y,
                    pixel,
                );
            }
        }
    }

    fn draw_iso_paint_leaves_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let leaf_count = 5 + (variation % 6) as i32;
        let spread = (size * 10.0).round().clamp(5.0, 38.0) as i32;
        let shadow = [12, 10, 7, (opacity * 42.0).round() as u8];
        for i in 0..leaf_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x27d4_eb2d))
                .rotate_left(((i * 5) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 190;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 300;
            let center = [screen[0] + ox, screen[1] + oy];
            let angle = rotation + (((seed >> 16) & 0xff) as f32 / 255.0 - 0.5) * 0.85;
            let major = size * (2.2 + ((seed >> 24) & 0x7f) as f32 / 75.0);
            let minor = major * (0.34 + ((seed >> 11) & 0x3f) as f32 / 260.0);
            let shade = 0.68 + ((seed >> 5) & 0xff) as f32 / 255.0 * 0.78;
            let mut leaf = Self::iso_paint_adjust_rgb(color, shade);
            leaf[3] = (opacity * 215.0).round() as u8;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0] + 1, center[1] + 1],
                major,
                minor,
                angle,
                shadow,
                seed ^ 0x51ad_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                major,
                minor,
                angle,
                leaf,
                seed,
            );
            let vein_alpha = (opacity * 92.0).round() as u8;
            let vein = Self::iso_paint_adjust_rgb(leaf, 0.42);
            let vein = [vein[0], vein[1], vein[2], vein_alpha];
            let vx = (angle.cos() * major * 0.65).round() as i32;
            let vy = (angle.sin() * major * 0.65).round() as i32;
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center[0] - vx,
                center[1] - vy,
                center[0] + vx,
                center[1] + vy,
                vein,
            );
        }
    }

    fn iso_paint_stamp_palette_color(
        palette: &[[u8; 4]],
        index: usize,
        fallback: [u8; 4],
        opacity: f32,
        alpha: f32,
    ) -> [u8; 4] {
        let mut color = palette.get(index).copied().unwrap_or(fallback);
        color[3] = (opacity.clamp(0.0, 1.0) * alpha).round().clamp(0.0, 255.0) as u8;
        color
    }

    fn iso_paint_stamp_wood_color(
        palette: &[[u8; 4]],
        index: usize,
        fallback: [u8; 4],
        opacity: f32,
        alpha: f32,
    ) -> [u8; 4] {
        let palette_color = palette.get(index).copied().filter(|color| {
            color[0] >= color[1].saturating_add(10)
                && color[1] >= color[2].saturating_add(4)
                && color[0] >= 54
        });
        let mut color = palette_color.unwrap_or(fallback);
        color[3] = (opacity.clamp(0.0, 1.0) * alpha).round().clamp(0.0, 255.0) as u8;
        color
    }

    fn draw_iso_paint_flowers_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variant: &str,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let flower_count = match variant {
            "bluebells" => 3 + (variation % 3) as i32,
            "poppies" => 3 + (variation % 4) as i32,
            _ => 4 + (variation % 5) as i32,
        };
        let spread = (size * 8.0).round().clamp(5.0, 32.0) as i32;
        let stem_source = palette.first().copied().unwrap_or(color);
        let mut stem = Self::iso_paint_adjust_rgb(stem_source, 0.82);
        stem[3] = (opacity * 220.0).round() as u8;
        let mut leaf = Self::iso_paint_adjust_rgb(stem_source, 1.12);
        leaf[3] = (opacity * 165.0).round() as u8;
        let mut shadow = Self::iso_paint_adjust_rgb(stem_source, 0.18);
        shadow[3] = (opacity * 48.0).round() as u8;

        for i in 0..flower_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x7feb_352d))
                .rotate_left(((i * 6) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 190;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 420;
            let base = [screen[0] + ox, screen[1] + oy];
            let height = (size * (5.6 + ((seed >> 16) & 0x7f) as f32 / 32.0))
                .round()
                .clamp(5.0, 24.0) as i32;
            let lean = ((seed >> 24) as i32 & 0xff) - 128;
            let lean = lean * spread / 520 + (rotation.sin() * spread as f32 * 0.2).round() as i32;
            let tip = [base[0] + lean, base[1] - height];

            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0] + 1,
                base[1] + 1,
                tip[0] + 1,
                tip[1] + 1,
                shadow,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0],
                base[1],
                tip[0],
                tip[1],
                stem,
            );

            if i % 2 == 0 {
                let leaf_center = [
                    base[0] + lean / 2 + if seed & 1 == 0 { -1 } else { 1 },
                    base[1] - height / 2,
                ];
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    leaf_center,
                    (size * 1.5).clamp(1.2, 5.0),
                    (size * 0.55).clamp(0.9, 2.4),
                    rotation + if seed & 1 == 0 { -0.45 } else { 0.45 },
                    leaf,
                    seed ^ 0x3311_aa01,
                );
            }

            let petal_slot = if variant == "wildflowers" {
                1 + (((seed >> 13) as usize) % 3)
            } else {
                1
            };
            let petal_fallback = Self::iso_paint_adjust_rgb(stem_source, 1.25);
            let petal = Self::iso_paint_stamp_palette_color(
                palette,
                petal_slot,
                petal_fallback,
                opacity,
                225.0,
            );
            let radius = (size * (0.9 + ((seed >> 5) & 0x3f) as f32 / 120.0)).clamp(1.1, 4.2);
            let petal_count = if variant == "poppies" { 5 } else { 4 };
            for petal_index in 0..petal_count {
                let angle = rotation
                    + petal_index as f32 * std::f32::consts::TAU / petal_count as f32
                    + ((seed >> 9) & 0x1f) as f32 / 255.0;
                let center = if variant == "bluebells" {
                    [
                        tip[0] + ((petal_index as f32 - 1.5) * radius * 0.45).round() as i32,
                        tip[1] + (radius * (petal_index as f32 * 0.9 + 0.6)).round() as i32,
                    ]
                } else {
                    [
                        tip[0] + (angle.cos() * radius * 0.75).round() as i32,
                        tip[1] + (angle.sin() * radius * 0.55).round() as i32,
                    ]
                };
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center,
                    radius,
                    if variant == "bluebells" {
                        radius * 0.8
                    } else {
                        radius * 0.62
                    },
                    angle,
                    petal,
                    seed ^ petal_index as u32,
                );
            }
            let center_slot = if variant == "wildflowers" { 2 } else { 3 };
            let center_fallback = Self::iso_paint_adjust_rgb(stem_source, 0.55);
            let center = Self::iso_paint_stamp_palette_color(
                palette,
                center_slot,
                center_fallback,
                opacity,
                if variant == "bluebells" { 150.0 } else { 230.0 },
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                tip,
                (radius * 0.55).max(1.0),
                (radius * 0.45).max(1.0),
                rotation,
                center,
                seed ^ 0x7777_0013,
            );
        }
    }

    fn draw_iso_paint_vines_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let mut stem = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.74),
            opacity,
            225.0,
        );
        stem = Self::iso_paint_adjust_rgb(stem, 0.86);
        let leaf_a = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 1.12),
            opacity,
            205.0,
        );
        let leaf_b = Self::iso_paint_stamp_palette_color(
            palette,
            2,
            Self::iso_paint_adjust_rgb(color, 0.92),
            opacity,
            190.0,
        );
        let mut shadow = Self::iso_paint_adjust_rgb(stem, 0.22);
        shadow[3] = (opacity * 54.0).round() as u8;

        let vine_count = 2 + (variation % 3) as i32;
        let spread = (size * 7.0).round().clamp(3.0, 28.0) as i32;
        for i in 0..vine_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x632b_e5ab))
                .rotate_left(((i * 5) as u32) & 15);
            let base = [
                screen[0] + ((seed & 0xff) as i32 - 128) * spread / 220,
                screen[1] + (((seed >> 8) & 0xff) as i32 - 128) * spread / 300,
            ];
            let length = (size * (14.0 + ((seed >> 16) & 0x7f) as f32 / 4.6))
                .round()
                .clamp(10.0, 58.0);
            let angle = rotation - std::f32::consts::FRAC_PI_2
                + ((seed >> 24) & 0xff) as f32 / 255.0 * 1.35
                - 0.68;
            let dir = [angle.cos(), angle.sin()];
            let normal = [-dir[1], dir[0]];
            let sway = (size * (3.0 + ((seed >> 10) & 0x3f) as f32 / 22.0)).clamp(2.0, 13.0);
            let phase = ((seed >> 4) & 0xff) as f32 / 255.0 * std::f32::consts::TAU;
            let segments = 5 + ((seed >> 7) & 3) as i32;
            let mut prev = base;
            for step in 1..=segments {
                let t = step as f32 / segments as f32;
                let wave = (phase + t * std::f32::consts::TAU * 0.72).sin() * sway;
                let point = [
                    base[0] + (dir[0] * length * t + normal[0] * wave).round() as i32,
                    base[1] + (dir[1] * length * t + normal[1] * wave).round() as i32,
                ];
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0] + 1,
                    prev[1] + 1,
                    point[0] + 1,
                    point[1] + 1,
                    shadow,
                );
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0],
                    prev[1],
                    point[0],
                    point[1],
                    stem,
                );

                if step % 2 == 0 || step == segments {
                    let side = if (seed >> (step as u32)) & 1 == 0 {
                        -1.0
                    } else {
                        1.0
                    };
                    let leaf_center = [
                        point[0] + (normal[0] * side * size * 2.2).round() as i32,
                        point[1] + (normal[1] * side * size * 2.2).round() as i32,
                    ];
                    let leaf_seed = seed ^ (step as u32).wrapping_mul(0x45d9_f3b);
                    Self::draw_iso_paint_rotated_ellipse(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        leaf_center,
                        (size * 2.4).clamp(1.5, 8.0),
                        (size * 0.85).clamp(0.8, 3.2),
                        angle + side * 0.78,
                        if step % 3 == 0 { leaf_b } else { leaf_a },
                        leaf_seed,
                    );
                }
                prev = point;
            }
        }
    }

    fn draw_iso_paint_roots_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let mut root = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.82),
            opacity,
            230.0,
        );
        root = Self::iso_paint_adjust_rgb(root, 0.92);
        let dark = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 0.48),
            opacity,
            180.0,
        );
        let highlight = Self::iso_paint_stamp_palette_color(
            palette,
            2,
            Self::iso_paint_adjust_rgb(color, 1.18),
            opacity,
            145.0,
        );
        let branch_count = 3 + (variation % 4) as i32;
        let spread = (size * 9.0).round().clamp(5.0, 34.0) as i32;
        let base_angle = rotation + std::f32::consts::FRAC_PI_2;

        for i in 0..branch_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x94d0_49bb))
                .rotate_left(((i * 4) as u32) & 15);
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let angle = base_angle + side * (0.35 + ((seed >> 8) & 0xff) as f32 / 255.0 * 0.7);
            let dir = [angle.cos(), angle.sin()];
            let normal = [-dir[1], dir[0]];
            let start = [
                screen[0] + (((seed >> 16) & 0xff) as i32 - 128) * spread / 300,
                screen[1] + (((seed >> 24) & 0xff) as i32 - 128) * spread / 420,
            ];
            let length = (size * (11.0 + ((seed >> 4) & 0x7f) as f32 / 5.0))
                .round()
                .clamp(9.0, 42.0);
            let bend = (size * (2.0 + ((seed >> 11) & 0x3f) as f32 / 24.0)).clamp(1.5, 10.0);
            let segments = 4 + ((seed >> 19) & 3) as i32;
            let mut prev = start;
            for step in 1..=segments {
                let t = step as f32 / segments as f32;
                let wave = (t * std::f32::consts::PI).sin() * bend * side;
                let point = [
                    start[0] + (dir[0] * length * t + normal[0] * wave).round() as i32,
                    start[1] + (dir[1] * length * t + normal[1] * wave).round() as i32,
                ];
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0] + 1,
                    prev[1] + 1,
                    point[0] + 1,
                    point[1] + 1,
                    dark,
                );
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0],
                    prev[1],
                    point[0],
                    point[1],
                    root,
                );
                if size > 1.35 && step < segments {
                    Self::iso_paint_blend_line(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        prev[0],
                        prev[1] - 1,
                        point[0],
                        point[1] - 1,
                        root,
                    );
                }

                if step == 2 || (step == segments - 1 && seed & 1 == 0) {
                    let twig_angle = angle - side * 0.7;
                    let twig_len = (size * (4.5 + ((seed >> 13) & 0x3f) as f32 / 16.0))
                        .round()
                        .clamp(3.0, 16.0);
                    let twig_end = [
                        point[0] + (twig_angle.cos() * twig_len).round() as i32,
                        point[1] + (twig_angle.sin() * twig_len).round() as i32,
                    ];
                    Self::iso_paint_blend_line(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        point[0],
                        point[1],
                        twig_end[0],
                        twig_end[1],
                        dark,
                    );
                }
                prev = point;
            }

            if i % 2 == 0 {
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    start,
                    (size * 1.35).clamp(1.0, 5.0),
                    (size * 0.8).clamp(0.8, 3.0),
                    angle,
                    highlight,
                    seed ^ 0x7015_0001,
                );
            }
        }
    }

    fn draw_iso_paint_leaf_mass(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        center: [i32; 2],
        radius_x: f32,
        radius_y: f32,
        seed: u32,
        dark: [u8; 4],
        mid: [u8; 4],
        light: [u8; 4],
    ) {
        let radius_x = radius_x.max(2.0);
        let radius_y = radius_y.max(2.0);
        let bound_x = (radius_x + 2.0).ceil() as i32;
        let bound_y = (radius_y + 2.0).ceil() as i32;
        for y in -bound_y..=bound_y {
            for x in -bound_x..=bound_x {
                let nx = x as f32 / radius_x;
                let ny = y as f32 / radius_y;
                let edge = nx * nx + ny * ny;
                let hash = iso_paint_brush::hash_u32(center[0] + x, center[1] + y, seed);
                let noise = (hash & 0xff) as f32 / 255.0;
                let wobble = (((hash >> 8) & 0xff) as f32 / 255.0 - 0.5) * 0.34;
                if edge > 0.94 + wobble {
                    continue;
                }
                if edge > 0.62 && ((hash >> 17) & 7) == 0 {
                    continue;
                }
                if edge < 0.36 && ((hash >> 21) & 31) == 0 {
                    continue;
                }

                let mut pixel = if ny < -0.34 && noise > 0.34 {
                    light
                } else if ny > 0.28 || edge > 0.72 {
                    dark
                } else {
                    mid
                };
                let shade = 0.76 + noise * 0.42 + (-ny).max(0.0) * 0.16;
                pixel = Self::iso_paint_adjust_rgb(pixel, shade);
                if edge > 0.76 {
                    pixel[3] = ((pixel[3] as f32) * (0.58 + noise * 0.32)).round() as u8;
                }
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + x,
                    center[1] + y,
                    pixel,
                );

                if radius_x > 10.0 && noise > 0.91 && edge < 0.68 {
                    let mut fleck = light;
                    fleck[3] = ((fleck[3] as f32) * 0.72).round() as u8;
                    Self::iso_paint_blend_stamp_pixel(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        center[0] + x + 1,
                        center[1] + y,
                        fleck,
                    );
                }
            }
        }
    }

    fn draw_iso_paint_bushes_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let dark = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.68),
            opacity,
            220.0,
        );
        let mid = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 0.98),
            opacity,
            230.0,
        );
        let light = Self::iso_paint_stamp_palette_color(
            palette,
            2,
            Self::iso_paint_adjust_rgb(color, 1.22),
            opacity,
            175.0,
        );
        let branch =
            Self::iso_paint_stamp_wood_color(palette, 3, [74, 49, 28, 255], opacity, 210.0);
        let bark_dark = Self::iso_paint_adjust_rgb(branch, 0.48);
        let root_y = screen[1];
        let art_size = (size * 0.58).clamp(1.0, 5.0);
        let stem_count = 2 + (variation % 2) as i32;
        let spread_x = (art_size * 3.0).round().clamp(3.0, 12.0) as i32;
        let stem_height = (art_size * 6.6).round().clamp(7.0, 30.0) as i32;

        for i in 0..stem_count {
            let seed = variation ^ (i as u32 + 1).wrapping_mul(0x45d9_f3b);
            let lane = i - stem_count / 2;
            let base_x = screen[0] + lane * spread_x / stem_count.max(1);
            let lean = (((seed >> 8) & 0xff) as i32 - 128) * spread_x / 360
                + (rotation.sin() * art_size * 1.2).round() as i32;
            let top = [
                base_x + lean,
                root_y - stem_height + (((seed >> 16) & 0x1f) as i32 * stem_height / 170),
            ];
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x + 1,
                root_y,
                top[0] + 1,
                top[1],
                bark_dark,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x,
                root_y,
                top[0],
                top[1],
                branch,
            );

            for node in 0..2 {
                let t = 0.42 + node as f32 * 0.25;
                let center = [
                    (base_x as f32 + (top[0] - base_x) as f32 * t).round() as i32,
                    (root_y as f32 + (top[1] - root_y) as f32 * t).round() as i32,
                ];
                let side = if (seed >> (node as u32)) & 1 == 0 {
                    -1.0
                } else {
                    1.0
                };
                let leaf_center = [
                    center[0] + (side * art_size * (0.95 + node as f32 * 0.42)).round() as i32,
                    center[1] - (art_size * 0.35).round() as i32,
                ];
                let leaf_seed = seed ^ (node as u32).wrapping_mul(0x9e37_79b9);
                Self::draw_iso_paint_leaf_mass(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    leaf_center,
                    (art_size * (0.78 + node as f32 * 0.12)).clamp(1.6, 4.0),
                    (art_size * (0.98 + node as f32 * 0.12)).clamp(2.0, 5.4),
                    leaf_seed,
                    dark,
                    mid,
                    light,
                );
            }

            let tip_leaf = if i % 2 == 0 { light } else { mid };
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                top,
                (art_size * 0.72).clamp(1.3, 3.7),
                (art_size * 1.05).clamp(1.8, 5.2),
                rotation + lean as f32 * 0.03,
                tip_leaf,
                seed ^ 0xb055_0001,
            );

            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x + 1,
                root_y,
                top[0] + 1,
                top[1],
                bark_dark,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x,
                root_y,
                top[0],
                top[1],
                branch,
            );
        }

        let base = Self::iso_paint_adjust_rgb(branch, 0.62);
        for dx in -spread_x / 2..=spread_x / 2 {
            if dx.abs() <= spread_x / 3 {
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    screen[0] + dx,
                    root_y,
                    base,
                );
            }
        }
    }

    fn draw_iso_paint_tree_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let leaf_dark = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.7),
            opacity,
            230.0,
        );
        let leaf_mid = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 1.02),
            opacity,
            235.0,
        );
        let trunk =
            Self::iso_paint_stamp_palette_color(palette, 2, [92, 58, 36, 255], opacity, 225.0);
        let leaf_light = Self::iso_paint_stamp_palette_color(
            palette,
            3,
            Self::iso_paint_adjust_rgb(color, 1.25),
            opacity,
            185.0,
        );
        let base = [screen[0], screen[1] + (size * 2.4).round() as i32];
        let canopy_center = [screen[0], screen[1] - (size * 12.5).round() as i32];
        let shadow = [6, 7, 5, (opacity * 68.0).round() as u8];
        Self::draw_iso_paint_rotated_ellipse(
            buffer,
            surface_buffer,
            stamp_depth,
            owner_geo_id,
            base,
            (size * 4.5).clamp(3.0, 18.0),
            (size * 1.8).clamp(1.4, 8.0),
            rotation * 0.2,
            shadow,
            variation ^ 0x7aee_0001,
        );

        let trunk_height = (size * 14.0).round().clamp(10.0, 46.0) as i32;
        let trunk_width = (size * 1.5).round().clamp(2.0, 7.0) as i32;
        for dx in -trunk_width..=trunk_width {
            let shade = if dx < 0 { 1.12 } else { 0.64 };
            let trunk_pixel = Self::iso_paint_adjust_rgb(trunk, shade);
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0] + dx,
                base[1],
                base[0] + dx / 2,
                base[1] - trunk_height,
                trunk_pixel,
            );
        }

        let bark_dark = Self::iso_paint_adjust_rgb(trunk, 0.42);
        for stripe in [-1, 1] {
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0] + stripe,
                base[1] - 1,
                base[0] + stripe / 2,
                base[1] - trunk_height + 2,
                bark_dark,
            );
        }

        for side in [-1.0_f32, 1.0] {
            let start = [base[0], base[1] - trunk_height * 2 / 3];
            let end = [
                start[0] + (side * size * 7.2).round() as i32,
                start[1] - (size * 5.3).round() as i32,
            ];
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                start[0],
                start[1],
                end[0],
                end[1],
                bark_dark,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                start[0],
                start[1] - 1,
                end[0],
                end[1] - 1,
                trunk,
            );
        }

        let crown = [
            (-8.5_f32, -7.0_f32, 7.4_f32, 7.2_f32),
            (6.5, -9.5, 8.2, 7.8),
            (-15.0, -1.8, 7.0, 6.8),
            (14.5, -1.2, 7.0, 6.6),
            (-5.0, 3.5, 9.2, 7.2),
            (6.5, 4.5, 8.4, 6.5),
            (0.0, -16.0, 7.4, 7.4),
            (-1.5, -3.6, 10.8, 8.2),
        ];
        for (i, (ox, oy, rx, ry)) in crown.iter().enumerate() {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x85eb_ca6b))
                .rotate_left(((i * 6) as u32) & 15);
            let jitter_x = ((seed & 0xff) as f32 / 255.0 - 0.5) * size * 2.4;
            let jitter_y = (((seed >> 8) & 0xff) as f32 / 255.0 - 0.5) * size * 2.0;
            let center = [
                canopy_center[0] + (ox * size + jitter_x).round() as i32,
                canopy_center[1] + (oy * size + jitter_y).round() as i32,
            ];
            Self::draw_iso_paint_leaf_mass(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                (rx * size).clamp(4.0, 28.0),
                (ry * size).clamp(3.0, 24.0),
                seed,
                leaf_dark,
                leaf_mid,
                leaf_light,
            );
        }

        for i in 0..18 {
            let seed = variation ^ (i as u32).wrapping_mul(0x27d4_eb2d);
            if (seed & 3) == 0 {
                continue;
            }
            let x = canopy_center[0] + (((seed >> 8) & 0xff) as i32 - 128) * (size as i32 + 10) / 9;
            let y =
                canopy_center[1] + (((seed >> 16) & 0xff) as i32 - 128) * (size as i32 + 8) / 11;
            let fleck = if (seed & 8) == 0 {
                leaf_light
            } else {
                leaf_dark
            };
            Self::iso_paint_blend_stamp_pixel(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                x,
                y,
                fleck,
            );
        }
    }

    fn draw_iso_paint_candles_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let wax = Self::iso_paint_stamp_palette_color(palette, 0, color, opacity, 235.0);
        let side = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(wax, 0.66),
            opacity,
            210.0,
        );
        let flame =
            Self::iso_paint_stamp_palette_color(palette, 2, [255, 151, 45, 230], opacity, 230.0);
        let core =
            Self::iso_paint_stamp_palette_color(palette, 3, [255, 239, 142, 245], opacity, 245.0);
        let candle_count = 1 + (variation % 3) as i32;
        let shadow = [8, 6, 4, (opacity * 62.0).round() as u8];
        let glow = [255, 151, 45, (opacity * 32.0).round() as u8];
        for i in 0..candle_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x85eb_ca6b))
                .rotate_left(((i * 6) as u32) & 15);
            let offset = (i - (candle_count - 1) / 2) as f32;
            let jitter = ((seed & 0xff) as f32 / 255.0 - 0.5) * size * 4.0;
            let base = [
                screen[0] + (offset * size * 6.0 + jitter).round() as i32,
                screen[1] + (((seed >> 8) & 0x3f) as f32 / 63.0 * size * 3.0).round() as i32,
            ];
            let height = (size * (8.0 + ((seed >> 14) & 0x7f) as f32 / 13.0))
                .round()
                .clamp(7.0, 28.0) as i32;
            let half_width = (size * (1.45 + ((seed >> 22) & 0x3f) as f32 / 72.0))
                .round()
                .clamp(1.0, 6.0) as i32;
            let top_y = base[1] - height;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], base[1] + 1],
                half_width as f32 + 1.2,
                (size * 1.0).clamp(0.8, 3.2),
                0.0,
                shadow,
                seed ^ 0x1188_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], top_y - (size * 5.3).round() as i32],
                (size * 4.4).clamp(2.0, 14.0),
                (size * 5.6).clamp(2.6, 18.0),
                0.0,
                glow,
                seed ^ 0x7f7f_0009,
            );
            for dx in -half_width..=half_width {
                let mut body = if dx > half_width / 2 { side } else { wax };
                let shade = if dx < -half_width / 2 { 1.09 } else { 1.0 };
                body = Self::iso_paint_adjust_rgb(body, shade);
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    base[0] + dx,
                    base[1],
                    base[0] + dx,
                    top_y,
                    body,
                );
            }
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], base[1]],
                half_width as f32 + 0.6,
                (size * 0.85).clamp(0.7, 2.8),
                0.0,
                side,
                seed ^ 0x5511_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], top_y],
                half_width as f32 + 0.4,
                (size * 0.75).clamp(0.7, 2.4),
                0.0,
                wax,
                seed ^ 0x5511_0002,
            );
            let wick = [24, 17, 12, (opacity * 185.0).round() as u8];
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0],
                top_y,
                base[0],
                top_y - (size * 2.3).round().max(2.0) as i32,
                wick,
            );
            let flame_y = top_y - (size * 4.2).round().max(4.0) as i32;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], flame_y],
                (size * 1.7).clamp(1.2, 5.5),
                (size * 3.2).clamp(2.0, 8.8),
                0.0,
                flame,
                seed ^ 0xf17e_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], flame_y + (size * 0.55).round() as i32],
                (size * 0.85).clamp(0.8, 3.0),
                (size * 1.55).clamp(1.0, 4.8),
                0.0,
                core,
                seed ^ 0xf17e_0002,
            );
        }
    }

    fn draw_iso_paint_stamp_shape(
        buffer: &mut TheRGBABuffer,
        stamp: &IsoPaintStamp,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
    ) {
        match stamp.kind.as_str() {
            "grass" | "grass_stamp" => Self::draw_iso_paint_grass_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "rubble" => Self::draw_iso_paint_rubble_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "leaves" => Self::draw_iso_paint_leaves_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "flowers" => Self::draw_iso_paint_flowers_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variant.as_str(),
                stamp.variation,
                stamp.rotation,
            ),
            "vines" => Self::draw_iso_paint_vines_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "roots" => Self::draw_iso_paint_roots_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "bushes" => Self::draw_iso_paint_bushes_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "tree" => Self::draw_iso_paint_tree_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "candles" => Self::draw_iso_paint_candles_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
            ),
            "footprints" => Self::draw_iso_paint_footprints_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "mud" => Self::draw_iso_paint_mud_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            _ => {}
        }
    }

    fn iso_paint_write_stamp_material(
        material_pixels: &mut [u8],
        width: usize,
        height: usize,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp: &IsoPaintStamp,
        screen: [i32; 2],
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
    ) {
        if stamp.material_id == 0 || width == 0 || height == 0 {
            return;
        }

        let mut mask = TheRGBABuffer::new(TheDim::sized(width as i32, height as i32));
        Self::draw_iso_paint_stamp_shape(
            &mut mask,
            stamp,
            surface_buffer,
            screen,
            None,
            owner_geo_id,
            size,
        );

        for (index, pixel) in mask.pixels().chunks_exact(4).enumerate() {
            let coverage = pixel[3];
            if coverage == 0 {
                continue;
            }
            let x = (index % width) as i32;
            let y = (index / width) as i32;
            Self::iso_paint_set_stamp_material_pixel(
                material_pixels,
                width,
                height,
                surface_buffer,
                owner_geo_id,
                x,
                y,
                stamp.material_id,
                coverage,
            );
        }
    }

    fn iso_paint_stamp_screen_and_size(
        stamp: &IsoPaintStamp,
        target_width: i32,
        target_height: i32,
        current_camera_scale: Option<f32>,
        project_world_anchor: &impl Fn([f32; 3], i32, i32) -> Option<[i32; 2]>,
    ) -> ([i32; 2], f32) {
        let screen = stamp
            .world
            .and_then(|world| project_world_anchor(world, target_width, target_height))
            .unwrap_or(stamp.screen);
        let size = if let (Some(source_scale), Some(current_scale)) =
            (stamp.camera_scale, current_camera_scale)
        {
            stamp.size * (source_scale / current_scale.max(0.001)).clamp(0.05, 20.0)
        } else {
            stamp.size
        };
        (screen, size)
    }

    fn draw_iso_paint_footprints_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let angle = rotation;
        let forward = [angle.cos(), angle.sin()];
        let side = [-forward[1], forward[0]];
        let step = (size * 4.6).round().clamp(3.0, 16.0);
        let stride = (size * 4.2).round().clamp(3.0, 16.0);
        let foot_len = (size * 3.5).clamp(3.0, 13.0);
        let foot_w = (size * 1.35).clamp(1.2, 5.0);
        let shadow = [8, 6, 5, (opacity * 45.0).round() as u8];
        for i in 0..2 {
            let phase = if i == 0 { -1.0 } else { 1.0 };
            let seed = variation ^ (i as u32 + 1).wrapping_mul(0x9e37_79b9);
            let center = [
                screen[0]
                    + (side[0] * step * phase + forward[0] * stride * phase * 0.55).round() as i32,
                screen[1]
                    + (side[1] * step * phase + forward[1] * stride * phase * 0.55).round() as i32,
            ];
            let foot_angle = angle + phase * 0.16;
            let shade = 0.64 + ((seed >> 12) & 0xff) as f32 / 255.0 * 0.26;
            let mut print = Self::iso_paint_adjust_rgb(color, shade);
            print[3] = (opacity * 190.0).round() as u8;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0] + 1, center[1] + 1],
                foot_len,
                foot_w,
                foot_angle,
                shadow,
                seed ^ 0x5a5a_0011,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                foot_len,
                foot_w,
                foot_angle,
                print,
                seed,
            );
            let toe = [
                center[0] + (forward[0] * foot_len * 0.68).round() as i32,
                center[1] + (forward[1] * foot_len * 0.68).round() as i32,
            ];
            for toe_side in [-0.9_f32, 0.0, 0.9] {
                let toe_center = [
                    toe[0] + (side[0] * foot_w * toe_side).round() as i32,
                    toe[1] + (side[1] * foot_w * toe_side).round() as i32,
                ];
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    toe_center,
                    foot_w * 0.48,
                    foot_w * 0.42,
                    foot_angle,
                    print,
                    seed ^ ((toe_side.to_bits()).wrapping_mul(0x45d9_f3b)),
                );
            }
        }
    }

    fn draw_iso_paint_mud_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let spread = (size * 9.0).round().clamp(5.0, 34.0) as i32;
        let shadow = [8, 6, 5, (opacity * 48.0).round() as u8];
        let mut base = Self::iso_paint_adjust_rgb(color, 0.78);
        base[3] = (opacity * 165.0).round() as u8;
        for i in 0..3 {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x4cf5_ad43))
                .rotate_left(((i * 4) as u32) & 15);
            let ox = if i == 0 {
                0
            } else {
                ((seed & 0xff) as i32 - 128) * spread / 260
            };
            let oy = if i == 0 {
                0
            } else {
                (((seed >> 8) & 0xff) as i32 - 128) * spread / 360
            };
            let center = [screen[0] + ox, screen[1] + oy];
            let angle = rotation * 0.18 + (((seed >> 18) & 0xff) as f32 / 255.0 - 0.5) * 0.5;
            let major = size * (4.3 + ((seed >> 10) & 0x7f) as f32 / 52.0);
            let minor = size * (2.0 + ((seed >> 25) & 0x3f) as f32 / 55.0);
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0] + 1, center[1] + 1],
                major,
                minor,
                angle,
                shadow,
                seed ^ 0x011d_1111,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                major,
                minor,
                angle,
                base,
                seed,
            );
        }

        let bubble_count = 3 + (variation % 4) as i32;
        for i in 0..bubble_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x9e37_79b9))
                .rotate_left(((i * 7) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 180;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 280;
            let center = [screen[0] + ox, screen[1] + oy];
            let radius = (size * (1.05 + ((seed >> 16) & 0x7f) as f32 / 92.0)).clamp(1.2, 6.0);
            let mut dome = Self::iso_paint_adjust_rgb(color, 1.18);
            dome[3] = (opacity * 122.0).round() as u8;
            let mut rim = Self::iso_paint_adjust_rgb(color, 0.54);
            rim[3] = (opacity * 120.0).round() as u8;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                radius,
                radius * 0.72,
                rotation,
                rim,
                seed ^ 0x8b8b_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0], center[1] - 1],
                radius * 0.68,
                radius * 0.44,
                rotation,
                dome,
                seed,
            );
            let highlight = [210, 224, 208, (opacity * 112.0).round() as u8];
            Self::iso_paint_blend_stamp_pixel(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center[0] - radius.round() as i32 / 2,
                center[1] - radius.round() as i32 / 2,
                highlight,
            );
        }
    }

    fn draw_iso_paint_rubble_ellipse(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        center: [i32; 2],
        radius_x: i32,
        radius_y: i32,
        color: [u8; 4],
        variation: u32,
    ) {
        let radius_x = radius_x.max(1);
        let radius_y = radius_y.max(1);
        let rx2 = (radius_x * radius_x).max(1) as f32;
        let ry2 = (radius_y * radius_y).max(1) as f32;
        for y in -radius_y..=radius_y {
            for x in -radius_x..=radius_x {
                let edge = x as f32 * x as f32 / rx2 + y as f32 * y as f32 / ry2;
                if edge > 1.0 {
                    continue;
                }
                let hash = iso_paint_brush::hash_u32(center[0] + x, center[1] + y, variation);
                let noise = (hash & 0xff) as f32 / 255.0;
                let shade = if y <= -radius_y / 3 && x <= radius_x / 3 {
                    1.18 + noise * 0.16
                } else if y >= radius_y / 3 || edge > 0.78 {
                    0.56 + noise * 0.16
                } else {
                    0.80 + noise * 0.24
                };
                let mut pixel = Self::iso_paint_adjust_rgb(color, shade);
                if edge > 0.88 {
                    pixel[3] = ((pixel[3] as f32) * 0.72).round() as u8;
                }
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + x,
                    center[1] + y,
                    pixel,
                );
            }
        }
    }

    fn draw_iso_paint_rubble_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let stone_count = 4 + (variation % 5) as i32;
        let spread = (size * 9.0).round().clamp(5.0, 36.0) as i32;
        let shadow = [9, 8, 7, (opacity * 72.0).round() as u8];
        for i in 0..stone_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x85eb_ca6b))
                .rotate_left(((i * 3) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 210;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 360;
            let lean = (rotation.sin() * spread as f32 * 0.18).round() as i32;
            let center = [screen[0] + ox + lean, screen[1] + oy];
            let radius_x = (size * (1.7 + ((seed >> 16) & 0x7f) as f32 / 90.0))
                .round()
                .clamp(2.0, 10.0) as i32;
            let radius_y = (radius_x as f32 * (0.42 + ((seed >> 23) & 0x3f) as f32 / 180.0))
                .round()
                .max(1.0) as i32;
            for sx in -radius_x..=radius_x {
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + sx,
                    center[1] + radius_y,
                    shadow,
                );
            }
            let shade = 0.68 + ((seed >> 11) & 0xff) as f32 / 255.0 * 0.64;
            let mut stone = Self::iso_paint_adjust_rgb(color, shade);
            stone[3] = (opacity * 235.0).round() as u8;
            Self::draw_iso_paint_rubble_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                radius_x,
                radius_y,
                stone,
                seed,
            );
        }
    }

    fn draw_iso_paint_grass_stamp(
        buffer: &mut TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let blade_count = 5 + (variation % 5) as i32;
        let base_alpha = (opacity * 235.0).round() as u8;
        let base_color = [color[0], color[1], color[2], base_alpha];
        let shadow = [7, 11, 8, (opacity * 72.0).round() as u8];
        let height = (size * 12.0).round().clamp(10.0, 56.0) as i32;
        let spread = (size * 5.0).round().clamp(4.0, 28.0) as i32;
        let dim = *buffer.dim();
        let width = dim.width.max(0) as usize;
        let height_px = dim.height.max(0) as usize;
        for sx in -spread / 2..=spread / 2 {
            let x = screen[0] + sx;
            if Self::iso_paint_stamp_pixel_visible(
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                x,
                screen[1],
            ) {
                Self::iso_paint_blend_pixel(
                    buffer.pixels_mut(),
                    width,
                    height_px,
                    x,
                    screen[1],
                    shadow,
                );
            }
        }
        for i in 0..blade_count {
            let lane = i - blade_count / 2;
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x9e37_79b9))
                .rotate_left((i as u32) & 15);
            let bend = ((seed & 0xff) as i32 - 128) * spread / 190;
            let lean = (rotation.sin() * spread as f32 * 0.45).round() as i32;
            let base_x = screen[0] + lane * spread / blade_count.max(1);
            let top_x = base_x + bend + lean;
            let top_y = screen[1] - height + ((seed >> 8) & 9) as i32;
            let shade = 0.68 + ((seed >> 16) & 0xff) as f32 / 255.0 * 0.68;
            let blade = Self::iso_paint_adjust_rgb(base_color, shade);
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x,
                screen[1],
                top_x,
                top_y,
                blade,
            );
            if size > 1.7 {
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    base_x + 1,
                    screen[1],
                    top_x + 1,
                    top_y,
                    blade,
                );
            }
        }
    }

    fn iso_paint_stroke_anchor(
        stroke: &IsoPaintStroke,
    ) -> (Option<[i32; 2]>, Option<[f32; 3]>, Option<f32>) {
        if matches!(stroke.clip.as_str(), "surface" | "object")
            && let Some(point) = stroke
                .points
                .iter()
                .find(|point| point.world.is_some() && point.owner.is_some())
        {
            return (Some(point.screen), point.world, point.camera_scale);
        }
        for point in &stroke.points {
            if let Some(world) = point.world {
                return (Some(point.screen), Some(world), point.camera_scale);
            }
        }
        (stroke.points.first().map(|point| point.screen), None, None)
    }

    fn iso_paint_stroke_bounds(stroke: &IsoPaintStroke) -> ([i32; 2], [i32; 2]) {
        let pad = (stroke.size * 2.0).round().max(1.0) as i32 + 1;
        let min = [stroke.screen_bounds[0] - pad, stroke.screen_bounds[1] - pad];
        let max = [stroke.screen_bounds[2] + pad, stroke.screen_bounds[3] + pad];
        (min, max)
    }

    fn iso_paint_stroke_screen_points(stroke: &IsoPaintStroke) -> Vec<[i32; 2]> {
        stroke.points.iter().map(|point| point.screen).collect()
    }

    fn iso_paint_resampled_point(points: &[[f32; 2]], distance: f32) -> [f32; 2] {
        if points.is_empty() {
            return [0.0, 0.0];
        }
        if points.len() == 1 || distance <= 0.0 {
            return points[0];
        }

        let mut travelled = 0.0;
        for pair in points.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let segment = (dx * dx + dy * dy).sqrt();
            if segment <= f32::EPSILON {
                continue;
            }
            if travelled + segment >= distance {
                let t = ((distance - travelled) / segment).clamp(0.0, 1.0);
                return [a[0] + dx * t, a[1] + dy * t];
            }
            travelled += segment;
        }

        *points.last().unwrap_or(&points[0])
    }

    fn iso_paint_stabilized_arch_points(stroke: &IsoPaintStroke) -> Vec<[i32; 2]> {
        let mut raw = Vec::new();
        for point in &stroke.points {
            let candidate = [point.screen[0] as f32, point.screen[1] as f32];
            if raw.last().is_none_or(|last: &[f32; 2]| {
                let dx = candidate[0] - last[0];
                let dy = candidate[1] - last[1];
                dx * dx + dy * dy >= 4.0
            }) {
                raw.push(candidate);
            }
        }

        if raw.len() < 3 {
            return raw
                .into_iter()
                .map(|point| [point[0].round() as i32, point[1].round() as i32])
                .collect();
        }

        let mut total = 0.0;
        for pair in raw.windows(2) {
            let dx = pair[1][0] - pair[0][0];
            let dy = pair[1][1] - pair[0][1];
            total += (dx * dx + dy * dy).sqrt();
        }
        if total <= f32::EPSILON {
            return Self::iso_paint_stroke_screen_points(stroke);
        }

        let spacing = (stroke.size * 0.65).clamp(3.0, 8.0);
        let count = (total / spacing).ceil().max(2.0) as usize + 1;
        let mut points = Vec::with_capacity(count);
        for index in 0..count {
            let t = index as f32 / (count.saturating_sub(1).max(1)) as f32;
            points.push(Self::iso_paint_resampled_point(&raw, total * t));
        }

        for _ in 0..5 {
            if points.len() < 3 {
                break;
            }
            let mut smoothed = points.clone();
            for index in 1..points.len() - 1 {
                smoothed[index][0] = points[index - 1][0] * 0.25
                    + points[index][0] * 0.5
                    + points[index + 1][0] * 0.25;
                smoothed[index][1] = points[index - 1][1] * 0.25
                    + points[index][1] * 0.5
                    + points[index + 1][1] * 0.25;
            }
            points = smoothed;
        }

        points
            .into_iter()
            .map(|point| [point[0].round() as i32, point[1].round() as i32])
            .collect()
    }

    fn iso_paint_screen_path_local(
        screen_points: &[[i32; 2]],
        origin: [i32; 2],
    ) -> (Vec<[f32; 2]>, Vec<f32>) {
        let mut points = Vec::new();
        for point in screen_points {
            let local = [(point[0] - origin[0]) as f32, (point[1] - origin[1]) as f32];
            if points
                .last()
                .is_none_or(|last: &[f32; 2]| last[0] != local[0] || last[1] != local[1])
            {
                points.push(local);
            }
        }

        let mut lengths = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for index in 0..points.len() {
            if index > 0 {
                let previous = points[index - 1];
                let current = points[index];
                let dx = current[0] - previous[0];
                let dy = current[1] - previous[1];
                total += (dx * dx + dy * dy).sqrt();
            }
            lengths.push(total);
        }

        (points, lengths)
    }

    fn iso_paint_stroke_cache_key(stroke: &IsoPaintStroke) -> u64 {
        let mut hasher = DefaultHasher::new();
        stroke.id.hash(&mut hasher);
        stroke.order.hash(&mut hasher);
        stroke.operation.hash(&mut hasher);
        stroke.brush.hash(&mut hasher);
        stroke.brush_shape.hash(&mut hasher);
        stroke.material_id.hash(&mut hasher);
        stroke.material_mode.hash(&mut hasher);
        stroke.clip.hash(&mut hasher);
        stroke.color.hash(&mut hasher);
        stroke.palette_indices.hash(&mut hasher);
        stroke.palette_colors.hash(&mut hasher);
        stroke.pattern_kind.hash(&mut hasher);
        stroke.pattern_scale.to_bits().hash(&mut hasher);
        stroke.pattern_mortar.to_bits().hash(&mut hasher);
        stroke.pattern_detail.to_bits().hash(&mut hasher);
        stroke.pattern_variation.to_bits().hash(&mut hasher);
        stroke.size.to_bits().hash(&mut hasher);
        stroke.opacity.to_bits().hash(&mut hasher);
        stroke.screen_bounds.hash(&mut hasher);
        stroke.points.len().hash(&mut hasher);
        for point in &stroke.points {
            point.screen.hash(&mut hasher);
            if let Some(world) = point.world {
                for value in world {
                    value.to_bits().hash(&mut hasher);
                }
            }
            if let Some(uv) = point.surface_uv {
                for value in uv {
                    value.to_bits().hash(&mut hasher);
                }
            }
            if let Some(normal) = point.surface_normal {
                for value in normal {
                    value.to_bits().hash(&mut hasher);
                }
            }
            if let Some(camera_scale) = point.camera_scale {
                camera_scale.to_bits().hash(&mut hasher);
            }
            match &point.owner {
                Some(IsoPaintOwner::Unknown(id)) => (0_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Vertex(id)) => (1_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Linedef(id)) => (2_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Sector(id)) => (3_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Character(id)) => (4_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Item(id)) => (5_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Light(id)) => (6_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::ItemLight(id)) => (7_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Triangle(id)) => (8_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Terrain { x, z }) => (9_u8, *x, *z).hash(&mut hasher),
                Some(IsoPaintOwner::GeometryObject(id)) => (10_u8, *id).hash(&mut hasher),
                Some(IsoPaintOwner::Hole { sector_id, hole_id }) => {
                    (11_u8, *sector_id, *hole_id).hash(&mut hasher)
                }
                Some(IsoPaintOwner::Gizmo(id)) => (12_u8, *id).hash(&mut hasher),
                None => 255_u8.hash(&mut hasher),
            }
        }
        hasher.finish()
    }

    fn build_iso_paint_stroke_caches(stroke: &IsoPaintStroke) -> Vec<IsoPaintStrokeRenderCache> {
        if stroke.points.is_empty() || stroke.operation == "pick" {
            return Vec::new();
        }

        let erase = stroke.operation == "erase";
        let (origin, max) = Self::iso_paint_stroke_bounds(stroke);
        let width = (max[0] - origin[0] + 1).max(1);
        let height = (max[1] - origin[1] + 1).max(1);
        let mut paint = TheRGBABuffer::new(TheDim::sized(width, height));
        let paint_w = width as usize;
        let paint_h = height as usize;

        let (screen_anchor, world_anchor, camera_scale) = Self::iso_paint_stroke_anchor(stroke);
        let clip_geo_id = stroke
            .points
            .iter()
            .find_map(|point| point.owner.as_ref().map(Self::iso_paint_owner_geo_id));
        let replace_material = stroke.material_mode == "replace";
        let replace_opacity = ((stroke.opacity.clamp(0.0, 1.0) * 254.0).round() as u8).min(254);
        let writes_material = stroke.brush != "screen";
        let color_coverage_scale =
            Self::iso_paint_color_coverage_scale(&stroke.brush, stroke.material_id);
        if !erase && stroke.brush == "brick" && world_anchor.is_none() {
            return Vec::new();
        }

        let color = if erase {
            [
                0,
                0,
                0,
                (stroke.opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]
        } else if stroke.brush == "brick" {
            [
                255,
                255,
                255,
                (stroke.opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]
        } else {
            Self::iso_paint_color_with_opacity(stroke.color, stroke.opacity)
        };
        let radius = (stroke.size * 2.0).round().max(1.0) as i32;
        let mut shape_hasher = DefaultHasher::new();
        stroke.id.hash(&mut shape_hasher);
        stroke.brush_shape.hash(&mut shape_hasher);
        let shape_seed = shape_hasher.finish() as u32;
        let pixels = paint.pixels_mut();
        let arch_pattern =
            stroke.brush == "brick" && matches!(stroke.pattern_kind.as_str(), "arch" | "trim");
        let render_points = if arch_pattern {
            Self::iso_paint_stabilized_arch_points(stroke)
        } else {
            Self::iso_paint_stroke_screen_points(stroke)
        };

        if render_points.len() == 1 {
            let point = render_points[0];
            Self::iso_paint_stamp_coverage(
                pixels,
                paint_w,
                paint_h,
                point[0] - origin[0],
                point[1] - origin[1],
                radius,
                color,
                &stroke.palette_colors,
                &stroke.brush,
                &stroke.brush_shape,
                shape_seed,
            );
        } else {
            for pair in render_points.windows(2) {
                Self::iso_paint_draw_segment_coverage(
                    pixels,
                    paint_w,
                    paint_h,
                    pair[0],
                    pair[1],
                    origin,
                    radius,
                    color,
                    &stroke.palette_colors,
                    &stroke.brush,
                    &stroke.brush_shape,
                    shape_seed,
                );
            }
        }

        let (path_points, path_lengths) = Self::iso_paint_screen_path_local(&render_points, origin);

        vec![IsoPaintStrokeRenderCache {
            order: stroke.order,
            origin,
            screen_anchor,
            world_anchor,
            camera_scale,
            clip_geo_id,
            color_coverage_scale,
            replace_material,
            replace_opacity,
            writes_material,
            brush: stroke.brush.clone(),
            clip: stroke.clip.clone(),
            material_id: stroke.material_id,
            color: Self::iso_paint_color_with_opacity(stroke.color, 1.0),
            pattern_kind: stroke.pattern_kind.clone(),
            pattern_scale: stroke.pattern_scale,
            pattern_mortar: stroke.pattern_mortar,
            pattern_detail: stroke.pattern_detail,
            pattern_variation: stroke.pattern_variation,
            path_points,
            path_lengths,
            erase,
            buffer: paint,
        }]
    }

    fn build_iso_paint_chunk_cache(
        chunk: &IsoPaintChunk,
        previous: Option<IsoPaintChunkRenderCache>,
    ) -> IsoPaintChunkRenderCache {
        let mut previous_strokes = previous
            .map(|cache| cache.stroke_caches)
            .unwrap_or_default();
        let mut stroke_caches = HashMap::new();
        let mut strokes = Vec::new();

        for stroke in &chunk.strokes {
            let key = Self::iso_paint_stroke_cache_key(stroke);
            let cached = previous_strokes
                .remove(&stroke.id)
                .filter(|cached| cached.key == key)
                .unwrap_or_else(|| IsoPaintCachedStrokeRender {
                    key,
                    strokes: Self::build_iso_paint_stroke_caches(stroke),
                });
            strokes.extend(cached.strokes.iter().cloned());
            stroke_caches.insert(stroke.id, cached);
        }

        IsoPaintChunkRenderCache {
            revision: chunk.revision,
            strokes,
            stroke_caches,
        }
    }

    fn ensure_iso_paint_chunk_caches(
        render_cache: &mut IsoPaintRenderCache,
        layer: &IsoPaintLayer,
    ) {
        render_cache
            .chunks
            .retain(|key, _| layer.chunks.contains_key(key));

        for (key, chunk) in &layer.chunks {
            let rebuild = render_cache
                .chunks
                .get(key)
                .map(|cached| cached.revision != chunk.revision)
                .unwrap_or(true);
            if rebuild {
                let previous = render_cache.chunks.remove(key);
                let cached = Self::build_iso_paint_chunk_cache(chunk, previous);
                render_cache.chunks.insert(key.clone(), cached);
            }
        }
    }

    fn iso_paint_render_order_key(
        order: u64,
        chunk_index: usize,
        local_index: usize,
    ) -> (u8, u64, usize, usize) {
        ((order != 0) as u8, order, chunk_index, local_index)
    }

    fn ordered_iso_paint_strokes<'a>(
        render_cache: &'a IsoPaintRenderCache,
        layer: &IsoPaintLayer,
    ) -> Vec<&'a IsoPaintStrokeRenderCache> {
        let mut strokes = Vec::new();
        for (chunk_index, (key, _chunk)) in layer.chunks.iter().enumerate() {
            let Some(cached) = render_cache.chunks.get(key) else {
                continue;
            };
            for (stroke_index, stroke) in cached.strokes.iter().enumerate() {
                strokes.push((
                    Self::iso_paint_render_order_key(stroke.order, chunk_index, stroke_index),
                    stroke,
                ));
            }
        }
        strokes.sort_by_key(|(key, _)| *key);
        strokes.into_iter().map(|(_, stroke)| stroke).collect()
    }

    fn ordered_iso_paint_render_items<'a>(
        render_cache: &'a IsoPaintRenderCache,
        layer: &'a IsoPaintLayer,
    ) -> Vec<IsoPaintRenderItem<'a>> {
        let mut items = Vec::new();
        for (chunk_index, (key, chunk)) in layer.chunks.iter().enumerate() {
            let stroke_count = render_cache
                .chunks
                .get(key)
                .map(|cached| {
                    for (stroke_index, stroke) in cached.strokes.iter().enumerate() {
                        items.push((
                            Self::iso_paint_render_order_key(
                                stroke.order,
                                chunk_index,
                                stroke_index,
                            ),
                            IsoPaintRenderItem::Stroke(stroke),
                        ));
                    }
                    cached.strokes.len()
                })
                .unwrap_or(0);

            for (stamp_index, stamp) in chunk.stamps.iter().enumerate() {
                items.push((
                    Self::iso_paint_render_order_key(
                        stamp.order,
                        chunk_index,
                        stroke_count + stamp_index,
                    ),
                    IsoPaintRenderItem::Stamp(stamp),
                ));
            }
        }
        items.sort_by_key(|(key, _)| *key);
        items.into_iter().map(|(_, item)| item).collect()
    }

    fn iso_paint_layer_key(layer: &IsoPaintLayer) -> u64 {
        let mut hasher = DefaultHasher::new();
        layer.visible.hash(&mut hasher);
        layer.chunks.len().hash(&mut hasher);
        for (key, chunk) in &layer.chunks {
            key.hash(&mut hasher);
            chunk.origin.hash(&mut hasher);
            chunk.revision.hash(&mut hasher);
            chunk.stamp_revision.hash(&mut hasher);
            chunk.strokes.len().hash(&mut hasher);
            chunk.stamps.len().hash(&mut hasher);
        }
        hasher.finish()
    }

    fn iso_paint_camera_key(camera: scenevm::Camera3D) -> u64 {
        let mut hasher = DefaultHasher::new();
        let kind = match camera.kind {
            scenevm::CameraKind::OrthoIso => 0_u8,
            scenevm::CameraKind::OrbitPersp => 1_u8,
            scenevm::CameraKind::FirstPersonPersp => 2_u8,
        };
        kind.hash(&mut hasher);
        for value in [
            camera.pos.x,
            camera.pos.y,
            camera.pos.z,
            camera.forward.x,
            camera.forward.y,
            camera.forward.z,
            camera.right.x,
            camera.right.y,
            camera.right.z,
            camera.up.x,
            camera.up.y,
            camera.up.z,
            camera.vfov_deg,
            camera.ortho_half_h,
            camera.near,
            camera.far,
        ] {
            value.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }

    fn ensure_project_extension(mut path: PathBuf) -> PathBuf {
        if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
            let file_name = file_name.to_string();
            if !file_name
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", Self::PROJECT_EXTENSION))
            {
                path.set_file_name(format!("{file_name}.{}", Self::PROJECT_EXTENSION));
            }
        } else if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(Self::PROJECT_EXTENSION))
        {
            path.set_extension(Self::PROJECT_EXTENSION);
        }

        path
    }

    fn activate_edit_tile_meta_action(&mut self) {
        if self.server_ctx.curr_tile_id.is_none() {
            return;
        }

        if let Some(action) = ACTIONLIST
            .read()
            .unwrap()
            .get_action_by_command_id("tile.edit_metadata")
        {
            self.server_ctx.curr_action_id = Some(action.id().uuid);
        }
    }

    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    fn set_update_button_text(ui: &mut TheUI, ctx: &mut TheContext, text: Option<String>) {
        if let Some(widget) = ui.get_widget("Update") {
            if let Some(text) = text {
                widget.set_value(TheValue::Text(text));
                widget.set_disabled(false);
                widget.limiter_mut().set_max_width(180);
            } else {
                widget.set_value(TheValue::Text(String::new()));
                widget.set_disabled(true);
                widget.limiter_mut().set_max_width(0);
            }

            ctx.ui.relayout = true;
        }
    }

    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    fn set_update_button(ui: &mut TheUI, ctx: &mut TheContext, release: Option<&Release>) {
        Self::set_update_button_text(
            ui,
            ctx,
            release.map(|release| format!("Update to v{}", release.version)),
        );
    }

    fn log_segment_has_warning_or_error(segment: &str) -> bool {
        let segment = segment.to_ascii_lowercase();
        segment.contains("[error]") || segment.contains("[warning]") || segment.contains("[warn]")
    }

    fn starter_manifest_url() -> String {
        format!("{}starters/manifest.toml", Self::STARTER_REPO_RAW_BASE)
    }

    fn starter_repo_url(repo_path: &str) -> String {
        format!("{}{}", Self::STARTER_REPO_RAW_BASE, repo_path)
    }

    fn fetch_url_bytes(url: &str) -> Option<Vec<u8>> {
        if let Ok(response) = ureq::get(url)
            .set("Cache-Control", "no-cache")
            .set("Pragma", "no-cache")
            .call()
        {
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

    fn load_project_from_bytes(bytes: &[u8]) -> Result<Project, String> {
        let mut loaded = shared::project_io::decode_project(bytes)?;
        loaded.migrate_default_ruleset();
        loaded.migrate_button_commands();
        let _ = loaded.sync_ruleset_items();
        loaded.art_palette.current_index = 0;
        Ok(loaded)
    }

    fn load_project_from_path(path: &std::path::Path) -> Result<Project, String> {
        let bytes = std::fs::read(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        Self::load_project_from_bytes(&bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_project_to_path(path: &std::path::Path, project: &Project) -> Result<(), String> {
        let bytes = shared::project_io::encode_project(project)?;
        let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|err| {
            format!(
                "failed to create a temporary project beside {}: {err}",
                path.display()
            )
        })?;
        temporary
            .write_all(&bytes)
            .and_then(|_| temporary.flush())
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        temporary
            .persist(path)
            .map(|_| ())
            .map_err(|err| format!("failed to replace {}: {}", path.display(), err.error))
    }

    #[cfg(target_arch = "wasm32")]
    fn save_project_to_path(path: &std::path::Path, project: &Project) -> Result<(), String> {
        let bytes = shared::project_io::encode_project(project)?;
        std::fs::write(path, bytes)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))
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
        let _ = project.sync_ruleset_items();
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
                preview: entry
                    .preview
                    .as_deref()
                    .and_then(Self::load_starter_preview),
                manifest_id: entry.id,
                title: entry.title,
                dimension: entry.dimension,
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

    fn load_starter_project(repo_path: &str) -> Option<Project> {
        let bytes = Self::fetch_url_bytes(&Self::starter_repo_url(repo_path))?;
        Self::load_project_from_bytes(&bytes).ok()
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
        let dock_dirty = DOCKMANAGER.read().unwrap().has_dock_changes();
        self.sessions[self.active_session].project = self.project.clone();
        self.sessions[self.active_session].project_path = self.project_path.clone();
        self.sessions[self.active_session].undo = UNDOMANAGER.read().unwrap().clone();
        self.sessions[self.active_session].detached_dock_dirty |= dock_dirty;
        self.sessions[self.active_session].dirty = UNDOMANAGER.read().unwrap().has_unsaved()
            || self.sessions[self.active_session].detached_dock_dirty;
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

    fn snapshot_authored_maps_for_play(&mut self) {
        if self.play_map_snapshots.is_none() {
            self.play_map_snapshots = Some(
                self.project
                    .regions
                    .iter()
                    .map(|region| (region.id, region.map.clone()))
                    .collect(),
            );
        }
    }

    fn restore_authored_maps_after_play(&mut self) -> bool {
        let Some(snapshots) = self.play_map_snapshots.take() else {
            return false;
        };
        let snapshots: FxHashMap<Uuid, Map> = snapshots.into_iter().collect();
        let mut restored = false;
        for region in &mut self.project.regions {
            if let Some(map) = snapshots.get(&region.id) {
                region.map = map.clone();
                restored = true;
            }
        }
        restored
    }

    fn deactivate_project_for_switch(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        // Finish editor-local interactions while the outgoing project is still
        // installed, then snapshot it before clearing global UI state.
        DOCKMANAGER
            .write()
            .unwrap()
            .minimize(ui, ctx, &self.project, &mut self.server_ctx);
        {
            let mut tools = TOOLLIST.write().unwrap();
            if tools.editor_mode {
                tools.set_game_tools(ui, ctx);
            }
            tools.deactivte_tool(ui, ctx, &mut self.project, &mut self.server_ctx);
            tools.reset_for_project_switch(ctx);
        }
        *SIDEBARMODE.write().unwrap() = SidebarMode::Region;
        EDITCAMERA.write().unwrap().reset_for_project_switch();
        self.restore_authored_maps_after_play();
        self.sync_active_session_from_editor();
        DOCKMANAGER.write().unwrap().reset_for_project_switch();

        ctx.ui.clear_focus();
        ctx.ui.clear_hover();
        self.last_3d_hover_redraw_at = None;
        self.pending_game_messages.clear();
        self.pending_game_says.clear();
        self.pending_game_choices.clear();
        self.pending_text_game_command = None;
        self.pending_text_game_runtime_flush = false;
        self.last_processed_log_len = 0;
        self.iso_paint_render_cache = SharedIsoPaintRenderCache::default();
        TEXTGAME.write().unwrap().reset();

        SCENEMANAGER.write().unwrap().reset_for_project_switch();
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            if rusterix.server.state != rusterix::ServerState::Off {
                rusterix.server.stop();
            }
            rusterix.clear_say_messages();
            rusterix.player_camera = PlayerCamera::D2;
            rusterix.scene_handler.clear_runtime_scene();
            rusterix.scene_handler.clear_overlay();
            rusterix.scene_handler.build_index.clear();
            rusterix.client.scene.d2_static.clear();
            rusterix.client.scene.d2_dynamic.clear();
            rusterix.client.scene.d3_static.clear();
            rusterix.client.scene.d3_dynamic.clear();
            rusterix.client.scene.d3_overlay.clear();
            rusterix.client.scene.lights.clear();
            rusterix.client.scene.dynamic_lights.clear();
            rusterix.client.scene.chunks.clear();
            rusterix.set_dirty();
        }
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
        self.starter_project_loader_rx = None;
        self.selected_starter_manifest_id = None;

        let width = 900;
        let height = 430;
        let intro_height = 58;
        let bottom_bar_height = 46;
        let list_width = 560;
        let preview_height = 191;

        let mut dialog = TheCanvas::new();
        dialog.limiter_mut().set_max_size(Vec2::new(width, height));
        dialog.bottom_is_expanding = true;

        let mut intro_canvas = TheCanvas::new();
        intro_canvas
            .limiter_mut()
            .set_max_size(Vec2::new(width, intro_height));
        let mut intro = TheVLayout::new(TheId::named("Starter Project Intro"));
        intro
            .limiter_mut()
            .set_max_size(Vec2::new(width, intro_height));
        intro.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        intro.set_alignment(TheHorizontalAlign::Left);
        intro.set_margin(Vec4::new(16, 7, 16, 6));
        intro.set_padding(0);

        let mut intro_title = TheText::new(TheId::named("Starter Project Intro Title"));
        intro_title.set_text(fl!("starter_intro_title"));
        intro_title.set_text_size(16.0);
        intro_title.set_text_color(WHITE);
        intro.add_widget(Box::new(intro_title));

        let mut intro_sub = TheText::new(TheId::named("Starter Project Intro Sub"));
        intro_sub.set_text(fl!("starter_intro_sub"));
        intro_sub.set_text_size(12.0);
        intro_sub.set_text_color([170, 176, 184, 255]);
        intro.add_widget(Box::new(intro_sub));
        intro_canvas.set_layout(intro);
        dialog.set_top(intro_canvas);

        let mut content = TheCanvas::new();

        let mut list_canvas = TheCanvas::new();
        list_canvas
            .limiter_mut()
            .set_max_size(Vec2::new(list_width, height));
        let mut list = TheListLayout::new(TheId::named(Self::STARTER_LIST_ID));
        list.limiter_mut().set_max_width(list_width);
        list.set_item_size(70);
        list.set_margin(Vec4::new(10, 10, 10, 10));
        let mut item = TheListItem::new(TheId::named("Starter Project Loading"));
        item.set_text(fl!("starter_loading"));
        item.set_sub_text(fl!("starter_loading_sub"));
        item.set_size(70);
        item.set_text_color(WHITE);
        item.set_text_size(14.0);
        item.set_sub_text_color([170, 176, 184, 255]);
        item.set_sub_text_size(12.0);
        list.add_item(item, ctx);
        list_canvas.set_layout(list);
        content.set_left(list_canvas);

        let mut details = TheCanvas::new();
        let mut preview_canvas = TheCanvas::new();
        preview_canvas
            .limiter_mut()
            .set_max_size(Vec2::new(width - list_width, preview_height));
        let mut preview = TheIconView::new(TheId::named(Self::STARTER_PREVIEW_ID));
        preview
            .limiter_mut()
            .set_max_size(Vec2::new(width - list_width, preview_height));
        preview.set_border_color(Some([65, 71, 79, 255]));
        preview.set_background_color(Some([24, 27, 31, 255]));
        preview.set_alpha_mode(true);
        preview_canvas.set_widget(preview);
        details.set_top(preview_canvas);

        let mut preview_details_canvas = TheCanvas::new();
        preview_details_canvas
            .limiter_mut()
            .set_max_size(Vec2::new(width - list_width, height - preview_height));
        let mut preview_details = TheVLayout::new(TheId::named("Starter Project Preview Details"));
        preview_details.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        preview_details.set_alignment(TheHorizontalAlign::Left);
        preview_details.set_margin(Vec4::new(16, 12, 16, 12));
        preview_details.set_padding(2);

        let mut preview_kind = TheText::new(TheId::named(Self::STARTER_PREVIEW_KIND_ID));
        preview_kind.set_text(String::new());
        preview_kind.set_text_size(11.0);
        preview_kind.set_text_color([104, 169, 232, 255]);
        preview_details.add_widget(Box::new(preview_kind));

        let mut preview_title = TheText::new(TheId::named(Self::STARTER_PREVIEW_TITLE_ID));
        preview_title.set_text(fl!("starter_loading"));
        preview_title.set_text_size(16.0);
        preview_title.set_text_color(WHITE);
        preview_details.add_widget(Box::new(preview_title));

        let mut preview_description =
            TheTextView::new(TheId::named(Self::STARTER_PREVIEW_DESCRIPTION_ID));
        preview_description
            .limiter_mut()
            .set_max_size(Vec2::new(width - list_width - 32, 58));
        preview_description.set_text(String::new());
        preview_description.set_font_size(11.0);
        preview_description.set_selectable(false);
        preview_description.set_word_wrap(true);
        preview_description.set_padding((0, 3, 0, 0));
        preview_description.draw_background(false);
        preview_description.draw_border(false);
        preview_details.add_widget(Box::new(preview_description));
        preview_details_canvas.set_layout(preview_details);
        details.set_center(preview_details_canvas);
        content.set_center(details);
        dialog.set_center(content);

        let mut bottom = TheCanvas::new();
        bottom
            .limiter_mut()
            .set_max_size(Vec2::new(width, bottom_bar_height));
        let mut actions = TheHLayout::new(TheId::named("Starter Project Actions"));
        actions
            .limiter_mut()
            .set_max_size(Vec2::new(width, bottom_bar_height));
        actions.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        actions.set_margin(Vec4::new(16, 8, 10, 8));
        actions.set_padding(8);
        actions.set_reverse_index(Some(2));

        let mut note = TheText::new(TheId::named("Starter Project Note"));
        note.set_text(fl!("starter_unsaved_note"));
        note.set_text_size(11.0);
        note.set_text_color([145, 152, 161, 255]);
        actions.add_widget(Box::new(note));

        let mut cancel = TheTraybarButton::new(TheId::named(Self::STARTER_CANCEL_ID));
        cancel.set_text(fl!("starter_cancel"));
        actions.add_widget(Box::new(cancel));

        let mut create = TheTraybarButton::new(TheId::named(Self::STARTER_CREATE_ID));
        create.set_text(fl!("starter_create"));
        actions.add_widget(Box::new(create));

        bottom.set_layout(actions);
        dialog.set_bottom(bottom);

        ui.show_dialog(&fl!("starter_dialog_title"), dialog, vec![], ctx);
        if let Some(starters) = self.starter_manifest_cache.clone() {
            self.starter_projects = starters;
            self.rebuild_starter_project_list(ui, ctx);
            if let Some(first) = self.starter_projects.first() {
                let manifest_id = first.manifest_id.clone();
                self.selected_starter_manifest_id = Some(manifest_id.clone());
                ctx.ui.send(TheEvent::StateChanged(
                    TheId::named_with_id("Starter Project List Item", first.id),
                    TheWidgetState::Selected,
                ));
                self.update_starter_project_preview(&manifest_id, ui, ctx);
                ui.set_enabled(Self::STARTER_CREATE_ID, ctx);
            } else {
                ui.set_disabled(Self::STARTER_CREATE_ID, ctx);
            }
        } else {
            self.starter_projects.clear();
            ui.set_disabled(Self::STARTER_CREATE_ID, ctx);
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.starter_loader_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(Self::load_starter_manifest());
        });
    }

    fn rebuild_starter_project_list(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(list) = ui.get_list_layout(Self::STARTER_LIST_ID) {
            list.clear();
            list.set_item_size(70);
            for (index, entry) in self.starter_projects.iter().enumerate() {
                let mut item =
                    TheListItem::new(TheId::named_with_id("Starter Project List Item", entry.id));
                item.set_text(entry.title.clone());
                item.set_sub_text(entry.dimension.clone());
                item.set_size(70);
                item.set_text_color(WHITE);
                item.set_text_size(14.0);
                item.set_sub_text_color([104, 169, 232, 255]);
                item.set_sub_text_size(12.0);
                if index == 0 {
                    item.set_state(TheWidgetState::Selected);
                }
                list.add_item(item, ctx);
            }
        }
    }

    fn update_starter_project_preview(
        &self,
        manifest_id: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let Some(entry) = self
            .starter_projects
            .iter()
            .find(|entry| entry.manifest_id == manifest_id)
        else {
            return;
        };

        if let Some(widget) = ui.get_widget(Self::STARTER_PREVIEW_ID)
            && let Some(preview) = widget.as_icon_view()
        {
            preview.set_rgba_tile(entry.preview.clone().unwrap_or_default());
        }
        ui.set_widget_value(
            Self::STARTER_PREVIEW_KIND_ID,
            ctx,
            TheValue::Text(entry.dimension.to_uppercase()),
        );
        ui.set_widget_value(
            Self::STARTER_PREVIEW_TITLE_ID,
            ctx,
            TheValue::Text(entry.title.clone()),
        );
        ui.set_widget_value(
            Self::STARTER_PREVIEW_DESCRIPTION_ID,
            ctx,
            TheValue::Text(entry.description.clone()),
        );
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
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

        self.deactivate_project_for_switch(ui, ctx);
        let new_index = if self.replace_next_project_load_in_active_tab {
            self.sessions[self.active_session] = ProjectSession {
                project,
                project_path,
                undo: UndoManager::default(),
                dirty: false,
                detached_dock_dirty: false,
            };
            self.replace_next_project_load_in_active_tab = false;
            self.active_session
        } else {
            self.sessions.push(ProjectSession {
                project,
                project_path,
                undo: UndoManager::default(),
                dirty: false,
                detached_dock_dirty: false,
            });
            self.sessions.len() - 1
        };
        self.active_session = new_index;
        self.sync_editor_from_active_session();
        self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
        self.rebuild_project_tabs(ui);
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
            rusterix.assets.config = self.project.config.clone();
            sync_editor_visual_assets(&mut rusterix, &self.project);
            rusterix
                .scene_handler
                .sync_base_render_settings(&self.project.config);
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
        let restored_camera_command_id = match restored_view_index {
            2 => "camera.isometric",
            3 => "camera.first_person",
            _ => "camera.editing",
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
            if let Some(action) = actions.get_action_by_command_id_mut(restored_camera_command_id) {
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

        *PALETTE.write().unwrap() = self.project.art_palette.clone();
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.assets.palette = self.project.art_palette.clone();
            rusterix.assets.palette_materials = self
                .project
                .art_palette_materials
                .iter()
                .map(|m| m.rmoe_values())
                .collect();
            rusterix.assets.palette_material_ids =
                crate::undo::project_helper::palette_material_ids(&self.project);
            rusterix.set_tiles(self.project.tiles.clone(), true);
            rusterix.set_tile_groups(self.project.tile_groups.clone());
        }
        SCENEMANAGER.write().unwrap().set_palette(
            self.project.art_palette.clone(),
            self.project
                .art_palette_materials
                .iter()
                .map(|m| m.rmoe_values())
                .collect(),
            crate::undo::project_helper::palette_material_ids(&self.project),
        );

        // Project activation is a hard scene boundary. Rebuild even when two
        // tabs were cloned from the same project and therefore reuse map UUIDs.
        crate::utils::editor_scene_full_rebuild(&self.project, &self.server_ctx);
        if self.server_ctx.editor_view_mode != EditorViewMode::D2 {
            TOOLLIST
                .write()
                .unwrap()
                .update_geometry_overlay_3d(&mut self.project, &mut self.server_ctx);
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));

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
        if !Self::session_switch_required(self.active_session, index) {
            self.rebuild_project_tabs(ui);
            return;
        }

        self.deactivate_project_for_switch(ui, ctx);
        self.active_session = index;
        self.sync_editor_from_active_session();
        self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
        self.rebuild_project_tabs(ui);
    }

    fn session_switch_required(active_index: usize, requested_index: usize) -> bool {
        active_index != requested_index
    }

    fn sanitize_loaded_project(project: &mut Project) {
        project.migrate_button_commands();
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

        for (_, tile) in project.tiles.iter_mut() {
            for texture in &mut tile.textures {
                if texture.data_ext.is_none() {
                    texture.generate_normals(true);
                }
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

        self.deactivate_project_for_switch(ui, ctx);
        self.sessions.remove(self.active_session);

        if self.sessions.is_empty() {
            let project = Self::load_empty_project_template();
            self.sessions.push(ProjectSession {
                project,
                project_path: None,
                undo: UndoManager::default(),
                dirty: false,
                detached_dock_dirty: false,
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
                fl!("status_starter_choose"),
            ));
            *redraw = true;
        }
    }

    fn active_session_has_changes(&self) -> bool {
        let detached_dock_dirty = self
            .sessions
            .get(self.active_session)
            .map(|session| session.detached_dock_dirty)
            .unwrap_or(false);
        UNDOMANAGER.read().unwrap().has_unsaved()
            || DOCKMANAGER.read().unwrap().has_dock_changes()
            || detached_dock_dirty
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_project_snapshot(&self) -> serde_json::Value {
        let current_region = self
            .project
            .regions
            .iter()
            .find(|region| region.id == self.server_ctx.curr_region)
            .map(|region| {
                serde_json::json!({
                    "id": region.id.to_string(),
                    "name": region.name,
                    "map_name": region.map.name,
                    "camera": format!("{:?}", region.map.camera),
                })
            });

        let regions = self
            .project
            .regions
            .iter()
            .map(|region| {
                serde_json::json!({
                    "id": region.id.to_string(),
                    "name": region.name,
                    "map_name": region.map.name,
                    "camera": format!("{:?}", region.map.camera),
                    "sectors": region.map.sectors.len(),
                    "items": region.items.len(),
                    "characters": region.characters.len(),
                })
            })
            .collect::<Vec<_>>();

        let characters = self
            .project
            .characters
            .values()
            .map(|character| {
                serde_json::json!({
                    "id": character.id.to_string(),
                    "name": character.name,
                    "source_len": character.source.len(),
                    "data_len": character.data.len(),
                    "has_authoring": !character.authoring.trim().is_empty(),
                    "has_preview_rigging": !character.preview_rigging.trim().is_empty(),
                })
            })
            .collect::<Vec<_>>();

        let items = self
            .project
            .items
            .values()
            .map(|item| {
                serde_json::json!({
                    "id": item.id.to_string(),
                    "name": item.name,
                    "source_len": item.source.len(),
                    "data_len": item.data.len(),
                    "has_authoring": !item.authoring.trim().is_empty(),
                })
            })
            .collect::<Vec<_>>();

        serde_json::json!({
            "name": self.project.name,
            "path": self.project_path.as_ref().map(|path| path.display().to_string()),
            "dirty": self.active_session_has_changes(),
            "active_session": self.active_session,
            "session_count": self.sessions.len(),
            "current_region": current_region,
            "regions": regions,
            "characters": characters,
            "items": items,
            "counts": {
                "regions": self.project.regions.len(),
                "tiles": self.project.tiles.len(),
                "tile_groups": self.project.tile_groups.len(),
                "characters": self.project.characters.len(),
                "items": self.project.items.len(),
                "screens": self.project.screens.len(),
                "assets": self.project.assets.len(),
            }
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_action_catalog(
        &self,
        command: &ScepterActionList,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let requested_group = command
            .group
            .as_deref()
            .map(str::trim)
            .filter(|group| !group.is_empty())
            .map(str::to_ascii_lowercase);
        let group_ids: Vec<&str> = ActionGroup::ALL.iter().map(|group| group.id()).collect();
        if let Some(group) = &requested_group
            && !group_ids.contains(&group.as_str())
        {
            return serde_json::json!({
                "ok": false,
                "command": "action.list",
                "error": format!("Unknown action group '{group}'."),
                "groups": group_ids,
            });
        }

        let default_map = Map::default();
        let map = self
            .project
            .get_map(&self.server_ctx)
            .unwrap_or(&default_map);
        let actions = ACTIONLIST.read().unwrap();
        let entries: Vec<serde_json::Value> = actions
            .actions
            .iter()
            .filter_map(|action| {
                let descriptor = actions.descriptor_by_id(action.id().uuid)?;
                if requested_group
                    .as_deref()
                    .is_some_and(|group| group != descriptor.group.id())
                {
                    return None;
                }
                let applicable = action.is_applicable(map, ctx, &self.server_ctx);
                if command.applicable_only && !applicable {
                    return None;
                }
                Some(serde_json::json!({
                    "id": descriptor.command_id,
                    "name": descriptor.group.qualified_name(&action.id().name),
                    "action_name": action.id().name,
                    "description": action.info(),
                    "group": descriptor.group.id(),
                    "group_name": descriptor.group.label(),
                    "palette_slot": descriptor.group.palette_slot(),
                    "role": action.role().id(),
                    "applicable": applicable,
                    "selected": self.server_ctx.curr_action_id == Some(action.id().uuid),
                    "accelerator": action.accel().map(|accelerator| accelerator.description()),
                }))
            })
            .collect();

        serde_json::json!({
            "ok": true,
            "command": "action.list",
            "count": entries.len(),
            "groups": group_ids,
            "actions": entries,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_run_action(
        &mut self,
        command: ActionRun,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let request = EditorActionRequest {
            command_id: command.id,
            parameters_toml: command.parameters_toml,
        };
        match self.sidebar.execute_action_command(
            &request,
            ui,
            ctx,
            &mut self.project,
            &mut self.server_ctx,
        ) {
            Ok(()) => serde_json::json!({
                "ok": true,
                "command": "action.run",
                "action": request.command_id,
                "executed": 1,
                "dirty": self.active_session_has_changes(),
            }),
            Err(error) => serde_json::json!({
                "ok": false,
                "command": "action.run",
                "action": request.command_id,
                "error": error,
            }),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_run_action_script(
        &mut self,
        command: ActionRunScript,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        match self.execute_eldrin_action_script(&command.source, ui, ctx) {
            Ok(executed) => serde_json::json!({
                "ok": true,
                "command": "action.run_script",
                "executed": executed,
                "dirty": self.active_session_has_changes(),
            }),
            Err(error) => serde_json::json!({
                "ok": false,
                "command": "action.run_script",
                "error": error,
            }),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_tool_catalog(&self, command: &ScepterToolList) -> serde_json::Value {
        let tools = TOOLLIST.read().unwrap();
        let entries: Vec<serde_json::Value> = tools
            .game_tools
            .iter()
            .filter_map(|tool| {
                let descriptor = tools.game_tool_descriptor_by_id(tool.id().uuid)?;
                let available = tools.game_tool_is_available(&descriptor.command_id);
                if !command.include_hidden && !available {
                    return None;
                }
                Some(serde_json::json!({
                    "id": descriptor.command_id,
                    "name": tool.id().name,
                    "description": tool.info(),
                    "icon": tool.icon_name(),
                    "accelerator": tool.accel().map(|accelerator| accelerator.to_string()),
                    "available": available,
                    "selected": tools.current_game_tool_command_id()
                        == Some(descriptor.command_id.as_str()),
                }))
            })
            .collect();

        serde_json::json!({
            "ok": true,
            "command": "tool.list",
            "count": entries.len(),
            "tools": entries,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_select_tool(
        &mut self,
        command: ToolSelect,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        match self.sidebar.execute_tool_command(
            &command.id,
            ui,
            ctx,
            &mut self.project,
            &mut self.server_ctx,
        ) {
            Ok(changed) => serde_json::json!({
                "ok": true,
                "command": "tool.select",
                "tool": command.id,
                "changed": changed,
            }),
            Err(error) => serde_json::json!({
                "ok": false,
                "command": "tool.select",
                "tool": command.id,
                "error": error,
            }),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_tiles_snapshot(&self) -> serde_json::Value {
        let mut tiles = self
            .project
            .tiles
            .values()
            .map(|tile| {
                let first_frame = tile.textures.first().map(|texture| {
                    serde_json::json!({
                        "width": texture.width,
                        "height": texture.height,
                    })
                });

                serde_json::json!({
                    "id": tile.id.to_string(),
                    "alias": tile.alias,
                    "role": tile.role.to_string(),
                    "blocking": tile.blocking,
                    "scale": tile.scale,
                    "frame_count": tile.textures.len(),
                    "first_frame": first_frame,
                    "procedural": {
                        "style": tile.procedural.style,
                        "kind": tile.procedural.kind,
                        "weight": tile.procedural.weight,
                    },
                    "has_module": tile.module.is_some(),
                    "has_particle_emitter": tile.particle_emitter.is_some(),
                    "has_light_emitter": tile.light_emitter.is_some(),
                })
            })
            .collect::<Vec<_>>();
        tiles.sort_by(|a, b| {
            let alias_a = a["alias"].as_str().unwrap_or_default();
            let alias_b = b["alias"].as_str().unwrap_or_default();
            alias_a.cmp(alias_b).then_with(|| {
                a["id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(b["id"].as_str().unwrap_or_default())
            })
        });

        let mut tile_groups = self
            .project
            .tile_groups
            .values()
            .map(|group| {
                let members = group
                    .members
                    .iter()
                    .map(|member| {
                        serde_json::json!({
                            "tile_id": member.tile_id.to_string(),
                            "x": member.x,
                            "y": member.y,
                        })
                    })
                    .collect::<Vec<_>>();

                serde_json::json!({
                    "id": group.id.to_string(),
                    "name": group.name,
                    "width": group.width,
                    "height": group.height,
                    "tags": group.tags,
                    "members": members,
                })
            })
            .collect::<Vec<_>>();
        tile_groups.sort_by(|a, b| {
            let name_a = a["name"].as_str().unwrap_or_default();
            let name_b = b["name"].as_str().unwrap_or_default();
            name_a.cmp(name_b).then_with(|| {
                a["id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(b["id"].as_str().unwrap_or_default())
            })
        });

        let roles = rusterix::TileRole::iterator()
            .map(|role| role.to_string())
            .collect::<Vec<_>>();

        serde_json::json!({
            "roles": roles,
            "tiles": tiles,
            "tile_groups": tile_groups,
            "counts": {
                "tiles": self.project.tiles.len(),
                "tile_groups": self.project.tile_groups.len(),
            }
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_tile_summary(&self, tile_id: &Uuid) -> Option<serde_json::Value> {
        self.project.tiles.get(tile_id).map(|tile| {
            let first_frame = tile.textures.first().map(|texture| {
                serde_json::json!({
                    "width": texture.width,
                    "height": texture.height,
                })
            });

            serde_json::json!({
                "id": tile.id.to_string(),
                "alias": tile.alias,
                "role": tile.role.to_string(),
                "blocking": tile.blocking,
                "frame_count": tile.textures.len(),
                "first_frame": first_frame,
                "procedural": {
                    "style": tile.procedural.style,
                    "kind": tile.procedural.kind,
                    "weight": tile.procedural.weight,
                },
            })
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_source_summary(&self, source: &rusterix::PixelSource) -> serde_json::Value {
        use rusterix::PixelSource;

        let mut summary = match source {
            PixelSource::Off => serde_json::json!({ "kind": "off" }),
            PixelSource::TileId(tile_id) => serde_json::json!({
                "kind": "tile_id",
                "tile_id": tile_id.to_string(),
            }),
            PixelSource::TileGroup(group_id) => {
                let group = self.project.tile_groups.get(group_id);
                serde_json::json!({
                    "kind": "tile_group",
                    "group_id": group_id.to_string(),
                    "group": group.map(|group| serde_json::json!({
                        "id": group.id.to_string(),
                        "name": group.name,
                        "width": group.width,
                        "height": group.height,
                        "member_count": group.members.len(),
                        "tags": group.tags,
                    })),
                })
            }
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                let member = self
                    .project
                    .tile_groups
                    .get(group_id)
                    .and_then(|group| group.members.get(*member_index as usize));
                serde_json::json!({
                    "kind": "tile_group_member",
                    "group_id": group_id.to_string(),
                    "member_index": member_index,
                    "tile_id": member.map(|member| member.tile_id.to_string()),
                    "member": member.map(|member| serde_json::json!({
                        "x": member.x,
                        "y": member.y,
                    })),
                })
            }
            PixelSource::ProceduralTile(tile_id) => serde_json::json!({
                "kind": "procedural_tile",
                "tile_id": tile_id.to_string(),
            }),
            PixelSource::PaletteIndex(index) => serde_json::json!({
                "kind": "palette_index",
                "index": index,
            }),
            PixelSource::MaterialId(material_id) => serde_json::json!({
                "kind": "material_id",
                "material_id": material_id.to_string(),
            }),
            PixelSource::Sequence(name) => serde_json::json!({
                "kind": "sequence",
                "name": name,
            }),
            PixelSource::EntityTile(entity_id, tile_index) => serde_json::json!({
                "kind": "entity_tile",
                "entity_id": entity_id,
                "tile_index": tile_index,
            }),
            PixelSource::ItemTile(item_id, tile_index) => serde_json::json!({
                "kind": "item_tile",
                "item_id": item_id,
                "tile_index": tile_index,
            }),
            PixelSource::Color(color) => serde_json::json!({
                "kind": "color",
                "rgba": color.to_u8_array(),
            }),
            PixelSource::LegacyShapeFXGraphId(graph_id) => serde_json::json!({
                "kind": "legacy_shape_fx_graph_id",
                "graph_id": graph_id.to_string(),
            }),
            PixelSource::StaticTileIndex(index) => serde_json::json!({
                "kind": "static_tile_index",
                "index": index,
            }),
            PixelSource::DynamicTileIndex(index) => serde_json::json!({
                "kind": "dynamic_tile_index",
                "index": index,
            }),
            PixelSource::Pixel(_) => serde_json::json!({
                "kind": "pixel",
            }),
        };

        let resolved_tile_id = match source {
            PixelSource::TileId(tile_id)
            | PixelSource::ProceduralTile(tile_id)
            | PixelSource::MaterialId(tile_id) => Some(*tile_id),
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => self
                .project
                .tile_groups
                .get(group_id)
                .and_then(|group| group.members.get(*member_index as usize))
                .map(|member| member.tile_id),
            _ => None,
        };

        if let Some(tile_id) = resolved_tile_id
            && let Some(tile) = self.scepter_tile_summary(&tile_id)
            && let Some(object) = summary.as_object_mut()
        {
            object.insert("resolved_tile".to_string(), tile);
        }

        summary
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_resolved_tile_id(&self, source: &rusterix::PixelSource) -> Option<Uuid> {
        use rusterix::PixelSource;

        match source {
            PixelSource::TileId(tile_id)
            | PixelSource::ProceduralTile(tile_id)
            | PixelSource::MaterialId(tile_id) => Some(*tile_id),
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => self
                .project
                .tile_groups
                .get(group_id)
                .and_then(|group| group.members.get(*member_index as usize))
                .map(|member| member.tile_id),
            _ => None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_value_snapshot(&self, value: &Value) -> serde_json::Value {
        match value {
            Value::Source(source) => serde_json::json!({
                "type": "source",
                "source": self.scepter_source_summary(source),
            }),
            Value::TileOverrides(tiles) => {
                let mut entries = tiles
                    .iter()
                    .map(|((x, y), source)| {
                        serde_json::json!({
                            "cell": [x, y],
                            "source": self.scepter_source_summary(source),
                        })
                    })
                    .collect::<Vec<_>>();
                entries.sort_by_key(|entry| {
                    (
                        entry["cell"][0].as_i64().unwrap_or_default(),
                        entry["cell"][1].as_i64().unwrap_or_default(),
                    )
                });
                serde_json::json!({
                    "type": "tile_overrides",
                    "entries": entries,
                })
            }
            Value::BlendOverrides(blend_tiles) => {
                let mut entries = blend_tiles
                    .iter()
                    .map(|((x, y), (preset, source))| {
                        serde_json::json!({
                            "cell": [x, y],
                            "preset": serde_json::to_value(preset)
                                .unwrap_or_else(|_| serde_json::json!(format!("{preset:?}"))),
                            "source": self.scepter_source_summary(source),
                        })
                    })
                    .collect::<Vec<_>>();
                entries.sort_by_key(|entry| {
                    (
                        entry["cell"][0].as_i64().unwrap_or_default(),
                        entry["cell"][1].as_i64().unwrap_or_default(),
                    )
                });
                serde_json::json!({
                    "type": "blend_overrides",
                    "entries": entries,
                })
            }
            _ => {
                serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!(value.to_string()))
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_properties_snapshot(&self, properties: &ValueContainer) -> serde_json::Value {
        let mut values = serde_json::Map::new();
        for key in properties.keys_sorted() {
            if let Some(value) = properties.get(key) {
                values.insert(key.clone(), self.scepter_value_snapshot(value));
            }
        }
        serde_json::Value::Object(values)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_region_snapshot(&self, request: &ScepterRegionRequest) -> serde_json::Value {
        let region = if let Some(id) = &request.id {
            Uuid::from_str(id)
                .ok()
                .and_then(|id| self.project.regions.iter().find(|region| region.id == id))
        } else if let Some(name) = &request.name {
            self.project
                .regions
                .iter()
                .find(|region| region.name.eq_ignore_ascii_case(name))
        } else {
            self.project
                .regions
                .iter()
                .find(|region| region.id == self.server_ctx.curr_region)
                .or_else(|| self.project.regions.first())
        };

        let Some(region) = region else {
            return serde_json::json!({
                "error": "region not found",
                "request": {
                    "id": request.id,
                    "name": request.name,
                }
            });
        };

        let vertices = region
            .map
            .vertices
            .iter()
            .map(|vertex| {
                serde_json::json!({
                    "id": vertex.id,
                    "name": vertex.name,
                    "position": [vertex.x, vertex.y, vertex.z],
                    "properties": self.scepter_properties_snapshot(&vertex.properties),
                })
            })
            .collect::<Vec<_>>();

        let linedefs = region
            .map
            .linedefs
            .iter()
            .map(|linedef| {
                let start = region.map.find_vertex(linedef.start_vertex);
                let end = region.map.find_vertex(linedef.end_vertex);
                serde_json::json!({
                    "id": linedef.id,
                    "creator_id": linedef.creator_id.to_string(),
                    "name": linedef.name,
                    "start_vertex": linedef.start_vertex,
                    "end_vertex": linedef.end_vertex,
                    "start": start.map(|vertex| serde_json::json!([vertex.x, vertex.y, vertex.z])),
                    "end": end.map(|vertex| serde_json::json!([vertex.x, vertex.y, vertex.z])),
                    "sector_ids": linedef.sector_ids,
                    "length": linedef.length(&region.map),
                    "properties": self.scepter_properties_snapshot(&linedef.properties),
                })
            })
            .collect::<Vec<_>>();

        let sectors = region
            .map
            .sectors
            .iter()
            .map(|sector| {
                let polygon = sector
                    .linedefs
                    .iter()
                    .filter_map(|linedef_id| {
                        let linedef = region.map.find_linedef(*linedef_id)?;
                        let vertex = region.map.find_vertex(linedef.start_vertex)?;
                        Some(serde_json::json!([vertex.x, vertex.y, vertex.z]))
                    })
                    .collect::<Vec<_>>();
                let bbox = sector.bounding_box(&region.map);
                let center = sector.center(&region.map);

                serde_json::json!({
                    "id": sector.id,
                    "creator_id": sector.creator_id.to_string(),
                    "name": sector.name,
                    "layer": sector.layer,
                    "linedefs": sector.linedefs,
                    "polygon": polygon,
                    "bbox": {
                        "min": [bbox.min.x, bbox.min.y],
                        "max": [bbox.max.x, bbox.max.y],
                    },
                    "center": center.map(|center| serde_json::json!([center.x, center.y])),
                    "area": sector.area(&region.map),
                    "properties": self.scepter_properties_snapshot(&sector.properties),
                })
            })
            .collect::<Vec<_>>();

        let characters = region
            .characters
            .values()
            .map(|character| {
                serde_json::json!({
                    "id": character.id.to_string(),
                    "template_id": character.character_id.to_string(),
                    "name": character.name,
                    "position": [character.position.x, character.position.y, character.position.z],
                    "orientation": [character.orientation.x, character.orientation.y],
                    "source_len": character.source.len(),
                    "data_len": character.data.len(),
                })
            })
            .collect::<Vec<_>>();

        let items = region
            .items
            .values()
            .map(|item| {
                serde_json::json!({
                    "id": item.id.to_string(),
                    "template_id": item.item_id.to_string(),
                    "name": item.name,
                    "position": [item.position.x, item.position.y, item.position.z],
                    "source_len": item.source.len(),
                    "data_len": item.data.len(),
                })
            })
            .collect::<Vec<_>>();

        let mut body = serde_json::json!({
            "id": region.id.to_string(),
            "name": region.name,
            "map": {
                "id": region.map.id.to_string(),
                "name": region.map.name,
                "camera": format!("{:?}", region.map.camera),
                "grid_size": region.map.grid_size,
                "subdivisions": region.map.subdivisions,
                "authoring_notes": {
                    "primary_2d_surface_source_key": "source",
                    "coordinate_system": "2D origin uses x right, negative y up, positive y down",
                    "ceiling_source": "screen/button selected-state legacy usage; not current 2D map authoring",
                    "terrain": "deprecated in current form; defer Scepter terrain commands until the replacement terrain system exists",
                },
                "vertices": vertices,
                "linedefs": linedefs,
                "sectors": sectors,
                "characters": characters,
                "items": items,
                "counts": {
                    "vertices": region.map.vertices.len(),
                    "linedefs": region.map.linedefs.len(),
                    "sectors": region.map.sectors.len(),
                    "geometry_objects": region.map.geometry_objects.len(),
                    "lights": region.map.lights.len(),
                    "entities": region.map.entities.len(),
                    "items": region.map.items.len(),
                    "region_characters": region.characters.len(),
                    "region_items": region.items.len(),
                },
            },
        });

        if request.include_tiles
            && let Some(object) = body.as_object_mut()
        {
            object.insert("tile_lookup".to_string(), self.scepter_tiles_snapshot());
        }

        body
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_source_overview_char(&self, source: Option<&rusterix::PixelSource>) -> char {
        let Some(source) = source else {
            return ' ';
        };
        let Some(tile_id) = self.scepter_resolved_tile_id(source) else {
            return ' ';
        };
        let Some(tile) = self.project.tiles.get(&tile_id) else {
            return '?';
        };

        match tile.procedural.kind.as_str() {
            "entrance" => 'E',
            "exit" => 'X',
            "wall" => '#',
            "floor" => '.',
            "door" => 'D',
            _ => match tile.role {
                rusterix::TileRole::Water => '~',
                rusterix::TileRole::Mountain => '^',
                rusterix::TileRole::Road => '=',
                rusterix::TileRole::Nature => {
                    if tile.blocking {
                        'T'
                    } else {
                        ','
                    }
                }
                rusterix::TileRole::ManMade => {
                    if tile.blocking {
                        '#'
                    } else {
                        '.'
                    }
                }
                rusterix::TileRole::Dungeon => {
                    if tile.blocking {
                        '#'
                    } else {
                        '.'
                    }
                }
                _ => '?',
            },
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_base64_encode(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);

        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;

            encoded.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
            encoded.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
            if chunk.len() > 1 {
                encoded.push(TABLE[((triple >> 6) & 0x3f) as usize] as char);
            } else {
                encoded.push('=');
            }
            if chunk.len() > 2 {
                encoded.push(TABLE[(triple & 0x3f) as usize] as char);
            } else {
                encoded.push('=');
            }
        }

        encoded
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_region_render_preview(&self, command: &RegionRenderPreview) -> serde_json::Value {
        let region_index = match self.scepter_resolve_region_index(&command.region) {
            Ok(index) => index,
            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
        };
        let region = &self.project.regions[region_index];
        let map = &region.map;

        let (min_x, max_x, min_y, max_y) = if let Some([x, y, width, height]) = command.bounds {
            if width == 0 || height == 0 {
                return serde_json::json!({
                    "ok": false,
                    "error": "region.render_preview bounds require non-zero width and height",
                });
            }
            (
                x.min(x + width),
                x.max(x + width),
                y.min(y + height),
                y.max(y + height),
            )
        } else if map.vertices.is_empty() {
            return serde_json::json!({
                "ok": false,
                "error": "region has no geometry to render",
            });
        } else {
            let min_x = map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::INFINITY, f32::min)
                .floor() as i32;
            let max_x = map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil() as i32;
            let min_y = map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::INFINITY, f32::min)
                .floor() as i32;
            let max_y = map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil() as i32;
            (min_x, max_x, min_y, max_y)
        };

        let grid_width = (max_x - min_x).max(0) as usize;
        let grid_height = (max_y - min_y).max(0) as usize;
        if grid_width == 0 || grid_height == 0 {
            return serde_json::json!({
                "ok": false,
                "error": "region.render_preview resolved empty bounds",
            });
        }
        if grid_width > 128 || grid_height > 128 {
            return serde_json::json!({
                "ok": false,
                "error": "region.render_preview bounds exceed 128x128 cells",
                "bounds": {
                    "min": [min_x, min_y],
                    "max": [max_x, max_y],
                    "size": [grid_width, grid_height],
                }
            });
        }

        let zoom = command.zoom.unwrap_or(2).clamp(1, 8) as usize;
        let cell_pixels = 16usize * zoom;
        let width = grid_width * cell_pixels;
        let height = grid_height * cell_pixels;
        let mut rgb = vec![16u8; width * height * 3];

        for sector in &map.sectors {
            let Some(source) = sector
                .properties
                .get("source")
                .and_then(|value| value.to_source())
            else {
                continue;
            };
            let Some(tile_id) = self.scepter_resolved_tile_id(source) else {
                continue;
            };
            let Some(tile) = self.project.tiles.get(&tile_id) else {
                continue;
            };
            let Some(texture) = tile.textures.first() else {
                continue;
            };

            let bbox = sector.bounding_box(map);
            let sx0 = (bbox.min.x.floor() as i32).max(min_x);
            let sx1 = (bbox.max.x.ceil() as i32).min(max_x);
            let sy0 = (bbox.min.y.floor() as i32).max(min_y);
            let sy1 = (bbox.max.y.ceil() as i32).min(max_y);

            for cell_y in sy0..sy1 {
                for cell_x in sx0..sx1 {
                    let cell_origin_x = (cell_x - min_x) as usize * cell_pixels;
                    let cell_origin_y = (cell_y - min_y) as usize * cell_pixels;

                    for py in 0..cell_pixels {
                        let ty = py * texture.height / cell_pixels;
                        for px in 0..cell_pixels {
                            let tx = px * texture.width / cell_pixels;
                            let source_index = (ty * texture.width + tx) * 4;
                            if source_index + 3 >= texture.data.len() {
                                continue;
                            }

                            let alpha = texture.data[source_index + 3] as u16;
                            if alpha == 0 {
                                continue;
                            }
                            let image_x = cell_origin_x + px;
                            let image_y = cell_origin_y + py;
                            let target_index = (image_y * width + image_x) * 3;
                            for channel in 0..3 {
                                let src = texture.data[source_index + channel] as u16;
                                let dst = rgb[target_index + channel] as u16;
                                rgb[target_index + channel] =
                                    ((src * alpha + dst * (255 - alpha)) / 255) as u8;
                            }
                        }
                    }
                }
            }
        }

        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            match encoder.write_header() {
                Ok(mut writer) => {
                    if let Err(error) = writer.write_image_data(&rgb) {
                        return serde_json::json!({
                            "ok": false,
                            "error": format!("could not encode preview PNG: {error}"),
                        });
                    }
                }
                Err(error) => {
                    return serde_json::json!({
                        "ok": false,
                        "error": format!("could not write preview PNG header: {error}"),
                    });
                }
            }
        }

        serde_json::json!({
            "ok": true,
            "region": {
                "id": region.id.to_string(),
                "name": region.name,
            },
            "bounds": {
                "min": [min_x, min_y],
                "max": [max_x, max_y],
                "size": [grid_width, grid_height],
                "coordinate_system": "x right, negative y up; first image row is min_y/up",
            },
            "image": {
                "mime": "image/png",
                "encoding": "base64",
                "data": Self::scepter_base64_encode(&png_data),
                "width": width,
                "height": height,
                "grid_width": grid_width,
                "grid_height": grid_height,
                "cell_pixels": cell_pixels,
                "bytes": png_data.len(),
            }
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_region_summary(&self, request: &ScepterRegionRequest) -> serde_json::Value {
        let region = if let Some(id) = &request.id {
            Uuid::from_str(id)
                .ok()
                .and_then(|id| self.project.regions.iter().find(|region| region.id == id))
        } else if let Some(name) = &request.name {
            self.project
                .regions
                .iter()
                .find(|region| region.name.eq_ignore_ascii_case(name))
        } else {
            self.project
                .regions
                .iter()
                .find(|region| region.id == self.server_ctx.curr_region)
                .or_else(|| self.project.regions.first())
        };

        let Some(region) = region else {
            return serde_json::json!({
                "error": "region not found",
                "request": {
                    "id": request.id,
                    "name": request.name,
                }
            });
        };

        let bounds = if region.map.vertices.is_empty() {
            None
        } else {
            let min_x = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::INFINITY, f32::min);
            let max_x = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_y = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::INFINITY, f32::min);
            let max_y = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::NEG_INFINITY, f32::max);
            Some((min_x, max_x, min_y, max_y))
        };

        let mut layer_counts: HashMap<String, usize> = HashMap::new();
        let mut sector_source_counts: HashMap<Uuid, usize> = HashMap::new();
        let mut linedef_source_counts: HashMap<Uuid, usize> = HashMap::new();
        let mut role_counts: HashMap<String, (usize, usize)> = HashMap::new();
        let mut kind_counts: HashMap<String, usize> = HashMap::new();
        let mut off_sector_count = 0usize;
        let mut named_sectors = Vec::new();
        let mut procedural_sectors: HashMap<String, usize> = HashMap::new();

        for sector in &region.map.sectors {
            *layer_counts
                .entry(
                    sector
                        .layer
                        .map(|layer| layer.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                )
                .or_default() += 1;

            if let Some(Value::Source(source)) = sector.properties.get("source") {
                if let Some(tile_id) = self.scepter_resolved_tile_id(source) {
                    *sector_source_counts.entry(tile_id).or_default() += 1;
                    if let Some(tile) = self.project.tiles.get(&tile_id) {
                        let role = tile.role.to_string().to_string();
                        let entry = role_counts.entry(role).or_default();
                        entry.0 += 1;
                        if tile.blocking {
                            entry.1 += 1;
                        }
                        if !tile.procedural.kind.is_empty() {
                            *kind_counts.entry(tile.procedural.kind.clone()).or_default() += 1;
                        }
                    }
                } else if matches!(source, rusterix::PixelSource::Off) {
                    off_sector_count += 1;
                }
            }

            if sector
                .properties
                .get_bool_default("procedural_generated", false)
            {
                let kind = sector
                    .properties
                    .get_str("procedural_kind")
                    .unwrap_or("unknown")
                    .to_string();
                *procedural_sectors.entry(kind).or_default() += 1;
            }

            if !sector.name.is_empty() {
                let bbox = sector.bounding_box(&region.map);
                let center = sector.center(&region.map);
                named_sectors.push(serde_json::json!({
                    "id": sector.id,
                    "name": sector.name,
                    "layer": sector.layer,
                    "bbox": {
                        "min": [bbox.min.x, bbox.min.y],
                        "max": [bbox.max.x, bbox.max.y],
                    },
                    "center": center.map(|center| serde_json::json!([center.x, center.y])),
                    "source": sector
                        .properties
                        .get("source")
                        .and_then(|value| value.to_source())
                        .map(|source| self.scepter_source_summary(source)),
                    "data": sector.properties.get_str("data"),
                }));
            }
        }

        for linedef in &region.map.linedefs {
            for key in [
                "source",
                "row1_source",
                "row2_source",
                "row3_source",
                "row4_source",
            ] {
                if let Some(Value::Source(source)) = linedef.properties.get(key)
                    && let Some(tile_id) = self.scepter_resolved_tile_id(source)
                {
                    *linedef_source_counts.entry(tile_id).or_default() += 1;
                }
            }
        }

        let source_usage = |counts: HashMap<Uuid, usize>| {
            let mut usage = counts
                .into_iter()
                .map(|(tile_id, count)| {
                    serde_json::json!({
                        "tile_id": tile_id.to_string(),
                        "count": count,
                        "tile": self.scepter_tile_summary(&tile_id),
                    })
                })
                .collect::<Vec<_>>();
            usage.sort_by(|a, b| {
                b["count"]
                    .as_u64()
                    .unwrap_or_default()
                    .cmp(&a["count"].as_u64().unwrap_or_default())
            });
            usage.truncate(30);
            usage
        };

        let mut layers = layer_counts
            .into_iter()
            .map(|(layer, count)| serde_json::json!({ "layer": layer, "count": count }))
            .collect::<Vec<_>>();
        layers.sort_by(|a, b| {
            a["layer"]
                .as_str()
                .unwrap_or_default()
                .cmp(b["layer"].as_str().unwrap_or_default())
        });

        let mut roles = role_counts
            .into_iter()
            .map(|(role, (count, blocking_count))| {
                serde_json::json!({
                    "role": role,
                    "count": count,
                    "blocking": blocking_count,
                    "walkable": count.saturating_sub(blocking_count),
                })
            })
            .collect::<Vec<_>>();
        roles.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap_or_default()
                .cmp(&a["count"].as_u64().unwrap_or_default())
        });

        let mut kinds = kind_counts
            .into_iter()
            .map(|(kind, count)| serde_json::json!({ "kind": kind, "count": count }))
            .collect::<Vec<_>>();
        kinds.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap_or_default()
                .cmp(&a["count"].as_u64().unwrap_or_default())
        });

        let mut procedural = procedural_sectors
            .into_iter()
            .map(|(kind, count)| serde_json::json!({ "kind": kind, "count": count }))
            .collect::<Vec<_>>();
        procedural.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap_or_default()
                .cmp(&a["count"].as_u64().unwrap_or_default())
        });

        let characters = region
            .characters
            .values()
            .map(|character| {
                serde_json::json!({
                    "id": character.id.to_string(),
                    "name": character.name,
                    "position": [character.position.x, character.position.y, character.position.z],
                    "orientation": [character.orientation.x, character.orientation.y],
                })
            })
            .collect::<Vec<_>>();

        let items = region
            .items
            .values()
            .map(|item| {
                serde_json::json!({
                    "id": item.id.to_string(),
                    "name": item.name,
                    "position": [item.position.x, item.position.y, item.position.z],
                })
            })
            .collect::<Vec<_>>();

        let overview = if request.include_ascii {
            bounds.map(|(min_x, max_x, min_y, max_y)| {
                let min_x_i = min_x.floor() as i32;
                let max_x_i = max_x.ceil() as i32;
                let min_y_i = min_y.floor() as i32;
                let max_y_i = max_y.ceil() as i32;
                let width = (max_x_i - min_x_i).max(0) as usize;
                let height = (max_y_i - min_y_i).max(0) as usize;
                let mut grid = vec![vec![' '; width]; height];

                if width <= 100 && height <= 100 {
                    for sector in &region.map.sectors {
                        let bbox = sector.bounding_box(&region.map);
                        let ch = self.scepter_source_overview_char(
                            sector
                                .properties
                                .get("source")
                                .and_then(|value| value.to_source()),
                        );
                        let sx0 = (bbox.min.x.floor() as i32).max(min_x_i);
                        let sx1 = (bbox.max.x.ceil() as i32).min(max_x_i);
                        let sy0 = (bbox.min.y.floor() as i32).max(min_y_i);
                        let sy1 = (bbox.max.y.ceil() as i32).min(max_y_i);
                        for y in sy0..sy1 {
                            for x in sx0..sx1 {
                                let gx = (x - min_x_i) as usize;
                                let gy = (y - min_y_i) as usize;
                                if let Some(row) = grid.get_mut(gy)
                                    && let Some(cell) = row.get_mut(gx)
                                {
                                    *cell = ch;
                                }
                            }
                        }
                    }

                    for character in region.characters.values() {
                        let x = character.position.x.floor() as i32 - min_x_i;
                        let y = character.position.z.floor() as i32 - min_y_i;
                        if let Some(row) = grid.get_mut(y as usize)
                            && let Some(cell) = row.get_mut(x as usize)
                        {
                            *cell = if character.name == "Player" { 'P' } else { 'C' };
                        }
                    }

                    for item in region.items.values() {
                        let x = item.position.x.floor() as i32 - min_x_i;
                        let y = item.position.z.floor() as i32 - min_y_i;
                        if let Some(row) = grid.get_mut(y as usize)
                            && let Some(cell) = row.get_mut(x as usize)
                        {
                            *cell = if item.name == "Door" { 'D' } else { 'i' };
                        }
                    }
                }

                let rows = if width <= 100 && height <= 100 {
                    grid.into_iter()
                        .map(|row| row.into_iter().collect::<String>())
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                serde_json::json!({
                    "bounds": {
                        "min": [min_x_i, min_y_i],
                        "max": [max_x_i, max_y_i],
                        "width": width,
                        "height": height,
                        "orientation": "first row is most negative y/up; later rows move downward toward positive y",
                    },
                    "legend": {
                        "~": "water",
                        "^": "mountain/blocking terrain",
                        "#": "blocking wall or manmade blocker",
                        ".": "floor/manmade walkable",
                        ",": "nature walkable",
                        "T": "blocking nature",
                        "=": "road/path",
                        "E": "entrance",
                        "X": "exit",
                        "P": "player",
                        "C": "character",
                        "D": "door item or door tile",
                        "i": "item"
                    },
                    "rows": rows,
                    "omitted": width > 100 || height > 100,
                })
            })
        } else {
            None
        };

        serde_json::json!({
            "id": region.id.to_string(),
            "name": region.name,
            "map": {
                "id": region.map.id.to_string(),
                "name": region.map.name,
                "camera": format!("{:?}", region.map.camera),
                "grid_size": region.map.grid_size,
                "subdivisions": region.map.subdivisions,
                "bounds": bounds.map(|(min_x, max_x, min_y, max_y)| serde_json::json!({
                    "min": [min_x, min_y],
                    "max": [max_x, max_y],
                    "size": [max_x - min_x, max_y - min_y],
                })),
                "counts": {
                    "vertices": region.map.vertices.len(),
                    "linedefs": region.map.linedefs.len(),
                    "sectors": region.map.sectors.len(),
                    "geometry_objects": region.map.geometry_objects.len(),
                    "characters": region.characters.len(),
                    "items": region.items.len(),
                    "off_sectors": off_sector_count,
                },
                "layers": layers,
                "tile_roles": roles,
                "procedural_kinds": kinds,
                "procedural_sectors": procedural,
                "sector_source_usage": source_usage(sector_source_counts),
                "linedef_source_usage": source_usage(linedef_source_counts),
                "named_sectors": named_sectors,
                "characters": characters,
                "items": items,
                "overview": overview,
                "authoring_notes": {
                    "primary_2d_surface_source_key": "source",
                    "coordinate_system": "2D origin uses x right, negative y up, positive y down",
                    "ceiling_source": "screen/button selected-state legacy usage; not current 2D map authoring",
                    "terrain": "deprecated in current form; defer Scepter terrain commands until the replacement terrain system exists",
                },
            },
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_resolve_region_index(&self, region: &RegionRef) -> Result<usize, String> {
        match region {
            RegionRef::Id { id } => {
                let id = Uuid::from_str(id).map_err(|err| format!("invalid region id: {err}"))?;
                self.project
                    .regions
                    .iter()
                    .position(|region| region.id == id)
                    .ok_or_else(|| format!("region id not found: {id}"))
            }
            RegionRef::Name { name } => self
                .project
                .regions
                .iter()
                .position(|region| region.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("region name not found: {name}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_normalize_match_text(value: &str) -> String {
        value
            .chars()
            .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != '-')
            .flat_map(char::to_lowercase)
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_resolve_tile_selector(&self, selector: &TileSelector) -> Result<Uuid, String> {
        if let Some(id) = &selector.id {
            let id = Uuid::from_str(id).map_err(|err| format!("invalid tile id: {err}"))?;
            if self.project.tiles.contains_key(&id) {
                return Ok(id);
            }
            return Err(format!("tile id not found: {id}"));
        }

        if let Some(alias) = &selector.alias
            && let Some(tile) = self
                .project
                .tiles
                .values()
                .find(|tile| tile.alias.eq_ignore_ascii_case(alias))
        {
            return Ok(tile.id);
        }

        let role = selector
            .role
            .as_deref()
            .map(Self::scepter_normalize_match_text);
        let kind = selector
            .kind
            .as_deref()
            .map(Self::scepter_normalize_match_text);
        let style = selector
            .style
            .as_deref()
            .map(Self::scepter_normalize_match_text);
        let tags = selector
            .tags
            .iter()
            .map(|tag| Self::scepter_normalize_match_text(tag))
            .collect::<Vec<_>>();

        self.project
            .tiles
            .values()
            .find(|tile| {
                role.as_ref().is_none_or(|role| {
                    Self::scepter_normalize_match_text(tile.role.to_string()) == *role
                }) && kind.as_ref().is_none_or(|kind| {
                    Self::scepter_normalize_match_text(&tile.procedural.kind) == *kind
                }) && style.as_ref().is_none_or(|style| {
                    Self::scepter_normalize_match_text(&tile.procedural.style) == *style
                }) && tags.iter().all(|tag| {
                    let alias = Self::scepter_normalize_match_text(&tile.alias);
                    let kind = Self::scepter_normalize_match_text(&tile.procedural.kind);
                    let style = Self::scepter_normalize_match_text(&tile.procedural.style);
                    alias.contains(tag) || kind.contains(tag) || style.contains(tag)
                })
            })
            .map(|tile| tile.id)
            .ok_or_else(|| format!("no tile matched selector: {selector:?}"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_find_rect_sector(map: &rusterix::Map, expected: &[(f32, f32); 4]) -> Option<u32> {
        Self::scepter_find_rect_sectors(map, expected)
            .into_iter()
            .next()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_find_rect_sectors(map: &rusterix::Map, expected: &[(f32, f32); 4]) -> Vec<u32> {
        map.sectors
            .iter()
            .filter_map(|sector| {
                let mut points = sector
                    .linedefs
                    .iter()
                    .filter_map(|linedef_id| {
                        let linedef = map.find_linedef(*linedef_id)?;
                        let vertex = map.find_vertex(linedef.start_vertex)?;
                        Some((vertex.x, vertex.y))
                    })
                    .collect::<Vec<_>>();
                points.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
                points.dedup();

                let mut expected = expected.to_vec();
                expected.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
                expected.dedup();

                if points == expected {
                    Some(sector.id)
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_find_cell_replacement_sectors(map: &rusterix::Map, x: i32, y: i32) -> Vec<u32> {
        let x0 = x as f32;
        let y0 = y as f32;
        let x1 = x0 + 1.0;
        let y1 = y0 + 1.0;
        let epsilon = 0.0001;

        map.sectors
            .iter()
            .filter_map(|sector| {
                let source = sector.properties.get_default_source()?;
                if matches!(source, rusterix::PixelSource::Off) {
                    return None;
                }

                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;
                for linedef_id in &sector.linedefs {
                    let linedef = map.find_linedef(*linedef_id)?;
                    let vertex = map.find_vertex(linedef.start_vertex)?;
                    min_x = min_x.min(vertex.x);
                    min_y = min_y.min(vertex.y);
                    max_x = max_x.max(vertex.x);
                    max_y = max_y.max(vertex.y);
                }

                let overlaps = min_x < x1 - epsilon
                    && max_x > x0 + epsilon
                    && min_y < y1 - epsilon
                    && max_y > y0 + epsilon;

                overlaps.then_some(sector.id)
            })
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_create_cell_sector(
        map: &mut rusterix::Map,
        x: i32,
        y: i32,
        source: rusterix::PixelSource,
        layer: u8,
    ) -> Option<u32> {
        let x0 = x as f32;
        let y0 = y as f32;
        let x1 = x0 + 1.0;
        let y1 = y0 + 1.0;
        let expected = [(x0, y0), (x0, y1), (x1, y1), (x1, y0)];

        let v0 = map.add_vertex_at(x0, y0);
        let v1 = map.add_vertex_at(x0, y1);
        let v2 = map.add_vertex_at(x1, y1);
        let v3 = map.add_vertex_at(x1, y0);

        map.possible_polygon.clear();
        let _ = map.create_linedef_manual(v0, v1);
        let _ = map.create_linedef_manual(v1, v2);
        let _ = map.create_linedef_manual(v2, v3);
        let _ = map.create_linedef_manual(v3, v0);

        let sector_id = map
            .close_polygon_manual()
            .or_else(|| Self::scepter_find_rect_sector(map, &expected))?;

        if let Some(sector) = map.find_sector_mut(sector_id) {
            sector.properties.set("rect", Value::Bool(true));
            sector.properties.set("source", Value::Source(source));
            sector.layer = Some(layer);
        }

        Some(sector_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_region_paint_cells_batch(
        &mut self,
        region: &RegionRef,
        tile: &TileSelector,
        cells: &[GridPoint],
        layer: Option<&str>,
        select: bool,
        replace_existing: bool,
        command_name: &str,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let region_index = match self.scepter_resolve_region_index(region) {
            Ok(index) => index,
            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
        };
        let tile_id = match self.scepter_resolve_tile_selector(tile) {
            Ok(tile_id) => tile_id,
            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
        };

        if cells.is_empty() {
            return serde_json::json!({
                "ok": false,
                "error": format!("{command_name} requires at least one cell"),
            });
        }

        let layer = layer
            .and_then(|layer| layer.parse::<u8>().ok())
            .unwrap_or(1);
        let source = rusterix::PixelSource::TileId(tile_id);
        let region_id = self.project.regions[region_index].id;
        let old_map = self.project.regions[region_index].map.clone();

        let mut created_sector_ids = Vec::new();
        let mut replaced_sector_ids = Vec::new();
        {
            let map = &mut self.project.regions[region_index].map;

            for [x, y] in cells {
                if replace_existing {
                    let sectors = Self::scepter_find_cell_replacement_sectors(map, *x, *y);
                    if !sectors.is_empty() {
                        let linedefs = sectors
                            .iter()
                            .filter_map(|sector_id| map.find_sector(*sector_id))
                            .flat_map(|sector| sector.linedefs.clone())
                            .collect::<Vec<_>>();
                        map.delete_elements(&[], &linedefs, &sectors);
                        replaced_sector_ids.extend(sectors);
                    }
                }

                match Self::scepter_create_cell_sector(map, *x, *y, source.clone(), layer) {
                    Some(sector_id) => created_sector_ids.push(sector_id),
                    None => {
                        return serde_json::json!({
                            "ok": false,
                            "error": format!("could not create cell sector at [{x}, {y}]"),
                            "created_sector_ids": created_sector_ids,
                            "replaced_sector_ids": replaced_sector_ids,
                        });
                    }
                }
            }

            if select {
                map.selected_vertices.clear();
                map.selected_linedefs.clear();
                map.selected_sectors = created_sector_ids.clone();
            }
            map.changed = map.changed.saturating_add(created_sector_ids.len() as u32);
        }

        let new_map = self.project.regions[region_index].map.clone();
        let undo_atom = ProjectUndoAtom::MapEdit(
            ProjectContext::Region(region_id),
            Box::new(old_map.clone()),
            Box::new(new_map.clone()),
        );
        editor_scene_apply_map_edit(&self.project, &self.server_ctx, &old_map, &new_map);
        update_region(ctx);
        UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);

        serde_json::json!({
            "ok": true,
            "command": command_name,
            "region_id": region_id.to_string(),
            "sector_ids": created_sector_ids,
            "replaced_sector_ids": replaced_sector_ids,
            "tile_id": tile_id.to_string(),
            "cell_count": cells.len(),
            "layer": layer,
            "replace_existing": replace_existing,
            "tile": self.scepter_tile_summary(&tile_id),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_region_paint_rect(
        &mut self,
        command: RegionPaintRect,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let [x, y, width, height] = command.rect;
        if width == 0 || height == 0 {
            return serde_json::json!({
                "ok": false,
                "error": "region.paint_rect requires non-zero width and height",
            });
        }

        let x0 = x.min(x + width);
        let x1 = x.max(x + width);
        let y0 = y.min(y + height);
        let y1 = y.max(y + height);
        let mut cells = Vec::new();
        for y in y0..y1 {
            for x in x0..x1 {
                cells.push([x, y]);
            }
        }

        self.scepter_apply_region_paint_cells_batch(
            &command.region,
            &command.tile,
            &cells,
            command.layer.as_deref(),
            command.select.unwrap_or(false),
            command.replace_existing.unwrap_or(true),
            "region.paint_rect",
            ctx,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_region_paint_cells(
        &mut self,
        command: RegionPaintCells,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        self.scepter_apply_region_paint_cells_batch(
            &command.region,
            &command.tile,
            &command.cells,
            command.layer.as_deref(),
            command.select.unwrap_or(false),
            command.replace_existing.unwrap_or(true),
            "region.paint_cells",
            ctx,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_script_target_region_index(
        &self,
        target: &ScriptTarget,
    ) -> Result<Option<usize>, String> {
        if let Some(region) = &target.region {
            return self.scepter_resolve_region_index(region).map(Some);
        }

        if target.kind == ScriptTargetKind::Region {
            if let Some(id) = &target.id {
                return self
                    .scepter_resolve_region_index(&RegionRef::Id { id: id.clone() })
                    .map(Some);
            }
            if let Some(name) = &target.name {
                return self
                    .scepter_resolve_region_index(&RegionRef::Name { name: name.clone() })
                    .map(Some);
            }
            return self
                .project
                .regions
                .iter()
                .position(|region| region.id == self.server_ctx.curr_region)
                .or(Some(0))
                .ok_or_else(|| "project has no regions".to_string())
                .map(Some);
        }

        Ok(None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_target_match(id: &Uuid, name: &str, target: &ScriptTarget) -> Result<bool, String> {
        if let Some(target_id) = &target.id {
            let target_id =
                Uuid::from_str(target_id).map_err(|err| format!("invalid target id: {err}"))?;
            return Ok(*id == target_id);
        }

        if let Some(target_name) = &target.name {
            return Ok(name.eq_ignore_ascii_case(target_name));
        }

        Ok(false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_target_missing_error(target: &ScriptTarget) -> String {
        match target.kind {
            ScriptTargetKind::World => "world target not found".to_string(),
            ScriptTargetKind::Region => "region target not found".to_string(),
            ScriptTargetKind::Character => {
                "character target requires an id or name and must exist".to_string()
            }
            ScriptTargetKind::Item => {
                "item target requires an id or name and must exist".to_string()
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_script_payload(
        kind: &str,
        scope: &str,
        id: Option<Uuid>,
        name: &str,
        source: &str,
        source_debug: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "ok": true,
            "kind": kind,
            "scope": scope,
            "id": id.map(|id| id.to_string()),
            "name": name,
            "source": source,
            "source_debug": source_debug,
            "source_len": source.len(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_get_script(&self, command: &ScriptGet) -> serde_json::Value {
        let target = &command.target;
        match target.kind {
            ScriptTargetKind::World => Self::scepter_script_payload(
                "world",
                "project",
                None,
                "World",
                &self.project.world_source,
                &self.project.world_source_debug,
            ),
            ScriptTargetKind::Region => {
                let region_index = match self.scepter_script_target_region_index(target) {
                    Ok(Some(index)) => index,
                    Ok(None) => {
                        return serde_json::json!({ "ok": false, "error": "region not found" });
                    }
                    Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                };
                let region = &self.project.regions[region_index];
                Self::scepter_script_payload(
                    "region",
                    "project",
                    Some(region.id),
                    &region.name,
                    &region.source,
                    &region.source_debug,
                )
            }
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(character) = region.characters.values().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, target)
                            .unwrap_or(false)
                    }) {
                        return Self::scepter_script_payload(
                            "character",
                            "region_instance",
                            Some(character.id),
                            &character.name,
                            &character.source,
                            &character.source_debug,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(character) = self.project.characters.values().find(|character| {
                    Self::scepter_target_match(&character.id, &character.name, target)
                        .unwrap_or(false)
                }) {
                    return Self::scepter_script_payload(
                        "character",
                        "template",
                        Some(character.id),
                        &character.name,
                        &character.source,
                        &character.source_debug,
                    );
                }

                serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(target) })
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(item) = region.items.values().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                    }) {
                        return Self::scepter_script_payload(
                            "item",
                            "region_instance",
                            Some(item.id),
                            &item.name,
                            &item.source,
                            &item.source_debug,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(item) = self.project.items.values().find(|item| {
                    Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                }) {
                    return Self::scepter_script_payload(
                        "item",
                        "template",
                        Some(item.id),
                        &item.name,
                        &item.source,
                        &item.source_debug,
                    );
                }

                serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(target) })
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_validate_eldrin_source(
        &self,
        target: &ScriptTarget,
        source: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "ok": true,
            "valid": true,
            "target": {
                "kind": format!("{:?}", target.kind).to_lowercase(),
                "id": target.id,
                "name": target.name,
                "region": target.region,
            },
            "source_len": source.len(),
            "diagnostics": [],
            "note": "Scepter currently stores Eldrin source and reports structural command validity; parser-backed diagnostics can be wired in a later pass.",
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_script_patch(
        &mut self,
        command: ScriptPatch,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        if command.validate {
            let validation = self.scepter_validate_eldrin_source(&command.target, &command.patch);
            if !validation
                .get("valid")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                return validation;
            }
        }

        let old_project = self.project.clone();
        let target = command.target;
        let new_source = command.patch;
        let mut changed = None::<(String, String, String)>;

        match target.kind {
            ScriptTargetKind::World => {
                self.project.world_source = new_source.clone();
                self.project.world_source_debug = new_source.clone();
                changed = Some((
                    "world".to_string(),
                    "project".to_string(),
                    "World".to_string(),
                ));
            }
            ScriptTargetKind::Region => {
                let region_index = match self.scepter_script_target_region_index(&target) {
                    Ok(Some(index)) => index,
                    Ok(None) => {
                        return serde_json::json!({ "ok": false, "error": "region not found" });
                    }
                    Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                };
                let region = &mut self.project.regions[region_index];
                region.source = new_source.clone();
                region.source_debug = new_source.clone();
                changed = Some((
                    "region".to_string(),
                    "project".to_string(),
                    region.name.clone(),
                ));
            }
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(character) = region.characters.values_mut().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, &target)
                            .unwrap_or(false)
                    }) {
                        character.source = new_source.clone();
                        character.source_debug = new_source.clone();
                        changed = Some((
                            "character".to_string(),
                            "region_instance".to_string(),
                            character.name.clone(),
                        ));
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(character) =
                        self.project.characters.values_mut().find(|character| {
                            Self::scepter_target_match(&character.id, &character.name, &target)
                                .unwrap_or(false)
                        })
                {
                    character.source = new_source.clone();
                    character.source_debug = new_source.clone();
                    changed = Some((
                        "character".to_string(),
                        "template".to_string(),
                        character.name.clone(),
                    ));
                }
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(item) = region.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    }) {
                        item.source = new_source.clone();
                        item.source_debug = new_source.clone();
                        changed = Some((
                            "item".to_string(),
                            "region_instance".to_string(),
                            item.name.clone(),
                        ));
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(item) = self.project.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    })
                {
                    item.source = new_source.clone();
                    item.source_debug = new_source.clone();
                    changed = Some((
                        "item".to_string(),
                        "template".to_string(),
                        item.name.clone(),
                    ));
                }
            }
        }

        let Some((kind, scope, name)) = changed else {
            self.project = old_project;
            return serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(&target) });
        };

        let new_project = self.project.clone();
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                format!("Scepter Script Edit: {kind} {name}"),
                Box::new(old_project),
                Box::new(new_project),
            ),
            ctx,
        );
        shared::rusterix_utils::insert_content_into_maps(&mut self.project);
        update_region(ctx);

        serde_json::json!({
            "ok": true,
            "command": "script.patch",
            "mode": "replace_source",
            "kind": kind,
            "scope": scope,
            "name": name,
            "source_len": new_source.len(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_json_to_toml(value: serde_json::Value) -> Result<toml::Value, String> {
        match value {
            serde_json::Value::Null => {
                Err("null is not a TOML value; use remove for deletion".to_string())
            }
            serde_json::Value::Bool(value) => Ok(toml::Value::Boolean(value)),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Ok(toml::Value::Integer(value))
                } else if let Some(value) = value.as_f64() {
                    Ok(toml::Value::Float(value))
                } else {
                    Err("number is outside TOML's supported range".to_string())
                }
            }
            serde_json::Value::String(value) => Ok(toml::Value::String(value)),
            serde_json::Value::Array(values) => values
                .into_iter()
                .map(Self::scepter_json_to_toml)
                .collect::<Result<Vec<_>, _>>()
                .map(toml::Value::Array),
            serde_json::Value::Object(values) => {
                let mut table = toml::Table::new();
                for (key, value) in values {
                    table.insert(key, Self::scepter_json_to_toml(value)?);
                }
                Ok(toml::Value::Table(table))
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_attributes_payload(
        kind: &str,
        scope: &str,
        id: Uuid,
        name: &str,
        data: &str,
    ) -> serde_json::Value {
        let parsed = data.parse::<toml::Table>();
        let (attributes, parse_error) = match parsed {
            Ok(table) => (
                table
                    .get("attributes")
                    .and_then(toml::Value::as_table)
                    .cloned()
                    .unwrap_or_default(),
                None,
            ),
            Err(error) => (toml::Table::new(), Some(error.to_string())),
        };

        serde_json::json!({
            "ok": parse_error.is_none(),
            "kind": kind,
            "scope": scope,
            "id": id.to_string(),
            "name": name,
            "data": data,
            "attributes": attributes,
            "parse_error": parse_error,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_get_attributes(&self, command: &AttributesGet) -> serde_json::Value {
        let target = &command.target;
        match target.kind {
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(character) = region.characters.values().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, target)
                            .unwrap_or(false)
                    }) {
                        return Self::scepter_attributes_payload(
                            "character",
                            "region_instance",
                            character.id,
                            &character.name,
                            &character.data,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(character) = self.project.characters.values().find(|character| {
                    Self::scepter_target_match(&character.id, &character.name, target)
                        .unwrap_or(false)
                }) {
                    return Self::scepter_attributes_payload(
                        "character",
                        "template",
                        character.id,
                        &character.name,
                        &character.data,
                    );
                }
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(item) = region.items.values().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                    }) {
                        return Self::scepter_attributes_payload(
                            "item",
                            "region_instance",
                            item.id,
                            &item.name,
                            &item.data,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(item) = self.project.items.values().find(|item| {
                    Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                }) {
                    return Self::scepter_attributes_payload(
                        "item", "template", item.id, &item.name, &item.data,
                    );
                }
            }
            ScriptTargetKind::World | ScriptTargetKind::Region => {
                return serde_json::json!({
                    "ok": false,
                    "error": "attributes.get currently supports character and item targets",
                });
            }
        }

        serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(target) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_patch_data_source(
        data: &str,
        values: serde_json::Map<String, serde_json::Value>,
        remove: &[String],
    ) -> Result<(String, Vec<String>, Vec<String>), String> {
        let mut table = if data.trim().is_empty() {
            toml::Table::new()
        } else {
            data.parse::<toml::Table>()
                .map_err(|err| format!("existing TOML data is invalid: {err}"))?
        };

        let attributes_value = table
            .entry("attributes".to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if !attributes_value.is_table() {
            return Err("[attributes] exists but is not a TOML table".to_string());
        }
        let attributes = attributes_value
            .as_table_mut()
            .ok_or_else(|| "could not access [attributes] table".to_string())?;

        let mut changed = Vec::new();
        for (key, value) in values {
            attributes.insert(key.clone(), Self::scepter_json_to_toml(value)?);
            changed.push(key);
        }

        let mut removed = Vec::new();
        for key in remove {
            if attributes.remove(key).is_some() {
                removed.push(key.clone());
            }
        }

        let source = toml::to_string_pretty(&table)
            .map_err(|err| format!("could not serialize TOML data: {err}"))?;
        Ok((source, changed, removed))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_attributes_patch(
        &mut self,
        command: AttributesPatch,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let old_project = self.project.clone();
        let target = command.target;
        let values = command.values;
        let remove = command.remove;
        let mut changed = None::<(String, String, String, Vec<String>, Vec<String>)>;

        match target.kind {
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(character) = region.characters.values_mut().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, &target)
                            .unwrap_or(false)
                    }) {
                        match Self::scepter_patch_data_source(
                            &character.data,
                            values.clone(),
                            &remove,
                        ) {
                            Ok((data, keys, removed)) => {
                                if command.validate
                                    && let Err(err) = data.parse::<toml::Table>()
                                {
                                    return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                                }
                                character.data = data;
                                changed = Some((
                                    "character".to_string(),
                                    "region_instance".to_string(),
                                    character.name.clone(),
                                    keys,
                                    removed,
                                ));
                            }
                            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                        }
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(character) =
                        self.project.characters.values_mut().find(|character| {
                            Self::scepter_target_match(&character.id, &character.name, &target)
                                .unwrap_or(false)
                        })
                {
                    match Self::scepter_patch_data_source(&character.data, values, &remove) {
                        Ok((data, keys, removed)) => {
                            if command.validate
                                && let Err(err) = data.parse::<toml::Table>()
                            {
                                return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                            }
                            character.data = data;
                            changed = Some((
                                "character".to_string(),
                                "template".to_string(),
                                character.name.clone(),
                                keys,
                                removed,
                            ));
                        }
                        Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                    }
                }
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(item) = region.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    }) {
                        match Self::scepter_patch_data_source(&item.data, values.clone(), &remove) {
                            Ok((data, keys, removed)) => {
                                if command.validate
                                    && let Err(err) = data.parse::<toml::Table>()
                                {
                                    return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                                }
                                item.data = data;
                                changed = Some((
                                    "item".to_string(),
                                    "region_instance".to_string(),
                                    item.name.clone(),
                                    keys,
                                    removed,
                                ));
                            }
                            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                        }
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(item) = self.project.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    })
                {
                    match Self::scepter_patch_data_source(&item.data, values, &remove) {
                        Ok((data, keys, removed)) => {
                            if command.validate
                                && let Err(err) = data.parse::<toml::Table>()
                            {
                                return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                            }
                            item.data = data;
                            changed = Some((
                                "item".to_string(),
                                "template".to_string(),
                                item.name.clone(),
                                keys,
                                removed,
                            ));
                        }
                        Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                    }
                }
            }
            ScriptTargetKind::World | ScriptTargetKind::Region => {
                return serde_json::json!({
                    "ok": false,
                    "error": "attributes.patch currently supports character and item targets",
                });
            }
        }

        let Some((kind, scope, name, changed_keys, removed_keys)) = changed else {
            self.project = old_project;
            return serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(&target) });
        };

        let new_project = self.project.clone();
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                format!("Scepter Attribute Edit: {kind} {name}"),
                Box::new(old_project),
                Box::new(new_project),
            ),
            ctx,
        );
        shared::rusterix_utils::insert_content_into_maps(&mut self.project);
        update_region(ctx);

        serde_json::json!({
            "ok": true,
            "command": "attributes.patch",
            "kind": kind,
            "scope": scope,
            "name": name,
            "changed": changed_keys,
            "removed": removed_keys,
        })
    }

    fn redraw_interval_ms(&self) -> u64 {
        let config = CONFIGEDITOR.read().unwrap();
        // UI presentation must stay independent from the simulation tick.
        // Stonefall deliberately ticks every 100 ms; using that interval for
        // redraws as soon as the server stopped dropped the Creator to 10 FPS.
        // `tick_ms` below still controls simulation/animation updates.
        (1000 / config.target_fps.clamp(1, 60)) as u64
    }

    #[inline]
    fn should_advance_animation_frame(
        server_state: rusterix::ServerState,
        editor_view_mode: EditorViewMode,
    ) -> bool {
        server_state == rusterix::ServerState::Running || editor_view_mode == EditorViewMode::D2
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
        let _ = project.sync_ruleset_items();

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        let (self_update_tx, self_update_rx) = channel();

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        let self_updater = SelfUpdater::github_creator();

        let initial_session = ProjectSession {
            project: project.clone(),
            project_path: None,
            undo: UndoManager::default(),
            dirty: false,
            detached_dock_dirty: false,
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
            #[cfg(not(target_arch = "wasm32"))]
            scepter_receiver: None,
            last_3d_hover_redraw_at: None,

            #[cfg(all(
                feature = "self-update",
                any(target_os = "windows", target_os = "linux", target_os = "macos")
            ))]
            self_update_rx,
            #[cfg(all(
                feature = "self-update",
                any(target_os = "windows", target_os = "linux", target_os = "macos")
            ))]
            self_update_tx,
            #[cfg(all(
                feature = "self-update",
                any(target_os = "windows", target_os = "linux", target_os = "macos")
            ))]
            self_updater: Arc::new(Mutex::new(self_updater)),

            update_counter: 0,
            last_processed_log_len: 0,
            pending_game_messages: Vec::new(),
            pending_game_says: Vec::new(),
            pending_game_choices: Vec::new(),
            pending_text_game_command: None,
            pending_text_game_runtime_flush: false,
            play_map_snapshots: None,

            build_values: ValueContainer::default(),
            window_state: Self::load_window_state(),
            starter_projects: Vec::new(),
            starter_project_cache: HashMap::new(),
            starter_manifest_cache: None,
            starter_loader_rx: None,
            starter_project_loader_rx: None,
            selected_starter_manifest_id: None,
            iso_paint_render_cache: SharedIsoPaintRenderCache::default(),
        }
    }

    fn init(&mut self, _ctx: &mut TheContext) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (service, receiver) = ScepterService::start();
            self.scepter_receiver = Some(receiver);
            println!("{}", service.status_line());
        }

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        {
            let updater = Arc::clone(&self.self_updater);
            let tx = self.self_update_tx.clone();

            thread::spawn(move || {
                let mut updater = updater.lock().unwrap();

                if let Err(err) = updater.fetch_release_list() {
                    tx.send(SelfUpdateEvent::UpdateError(err.to_string()))
                        .unwrap();
                } else if updater.has_newer_release() {
                    tx.send(SelfUpdateEvent::UpdateAvailable(
                        updater.latest_release().cloned().unwrap(),
                    ))
                    .unwrap();
                };
            });
        }
    }

    fn window_title(&self) -> String {
        format!("Eldiron Creator v{}", env!("CARGO_PKG_VERSION"))
    }

    fn target_fps(&self) -> f64 {
        // Simulation ticks may intentionally be sparse, but they must not
        // throttle input processing and UI presentation.
        CONFIGEDITOR.read().unwrap().target_fps.clamp(1, 60) as f64
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
        ui.set_theme(Box::new(TheBlackBlueTheme::new()), ctx);
        TOOLLIST
            .write()
            .unwrap()
            .set_overlay_theme(ui.style.theme().as_ref());
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

        // Compact navigation controls use SVG path data rasterized by Zeno at
        // their exact draw size. This guarantees transparent backgrounds and
        // avoids bitmap resampling artifacts on every frame.
        register_compact_navigation_icons(ctx);

        // ---

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
            show_menu.add(TheContextMenuItem::new_with_accel(
                "Project Sidebar".to_string(),
                TheId::named("Show Project Sidebar"),
                TheAccelerator::new(
                    TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
                    crate::sidebar::SIDEBAR_NAVIGATION_SHORTCUTS[0],
                ),
            ));
            show_menu.add(TheContextMenuItem::new_with_accel(
                "Actions Sidebar".to_string(),
                TheId::named("Show Actions Sidebar"),
                TheAccelerator::new(
                    TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
                    crate::sidebar::SIDEBAR_NAVIGATION_SHORTCUTS[1],
                ),
            ));
            show_menu.add(TheContextMenuItem::new_with_accel(
                "Console".to_string(),
                TheId::named("Show Console"),
                TheAccelerator::new(
                    TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
                    crate::sidebar::SIDEBAR_NAVIGATION_SHORTCUTS[2],
                ),
            ));
            show_menu.add(TheContextMenuItem::new_with_accel(
                "Debug".to_string(),
                TheId::named("Show Debug Log"),
                TheAccelerator::new(
                    TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
                    crate::sidebar::SIDEBAR_NAVIGATION_SHORTCUTS[3],
                ),
            ));
            show_menu.add(TheContextMenuItem::new_with_accel(
                "Help".to_string(),
                TheId::named("Show Help Sidebar"),
                TheAccelerator::new(
                    TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
                    crate::sidebar::SIDEBAR_NAVIGATION_SHORTCUTS[4],
                ),
            ));
            show_menu.add_separator();
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
        time_slider.set_tall(true);
        time_slider.set_continuous(true);
        time_slider.limiter_mut().set_max_width(400);
        time_slider.set_value(TheValue::Time(TheTime::default()));

        let mut patreon_button = TheMenubarButton::new(TheId::named("Patreon"));
        patreon_button.set_status_text(&fl!("status_patreon_button"));
        patreon_button.set_icon_name("patreon".to_string());
        // patreon_button.set_fixed_size(vec2i(36, 36));
        patreon_button.set_icon_offset(Vec2::new(-4, -2));

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        let update_button = {
            let mut button = TheTraybarButton::new(TheId::named("Update"));
            button.set_status_text(&fl!("status_update_button"));
            button.set_text(String::new());
            button.set_disabled(true);
            button.limiter_mut().set_max_width(0);
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

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        {
            hlayout.add_widget(Box::new(update_button));
            hlayout.add_widget(Box::new(patreon_button));
            hlayout.set_reverse_index(Some(2));
        }

        #[cfg(not(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        )))]
        {
            hlayout.add_widget(Box::new(patreon_button));
            hlayout.set_reverse_index(Some(1));
        }

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        ui.canvas.set_top(top_canvas);

        // Sidebar
        self.sidebar.init_ui(ui, ctx, &mut self.server_ctx);

        // Docks
        let mut bottom_panels = DOCKMANAGER.write().unwrap().init_docks(ctx);
        bottom_panels.limiter_mut().set_min_height(140);

        let mut editor_canvas: TheCanvas = TheCanvas::new();
        editor_canvas.limiter_mut().set_min_height(240);

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

        let mut project_strip_layout = TheHLayout::new(TheId::named("Project Strip Layout"));
        project_strip_layout.set_background_color(None);
        project_strip_layout.set_margin(Vec4::new(4, 1, 5, 1));
        project_strip_layout.set_padding(2);
        EDITCAMERA.write().unwrap().setup_toolbar(
            &mut project_strip_layout,
            ctx,
            &mut self.project,
            &mut self.server_ctx,
        );

        let (dock_is_normal, dock_is_available) = {
            let dock_manager = DOCKMANAGER.read().unwrap();
            (
                dock_manager.state == DockManagerState::Minimized,
                !dock_manager.dock.is_empty(),
            )
        };

        let mut dock_control_separator = TheHDivider::new(TheId::named("Dock Control Separator"));
        dock_control_separator
            .limiter_mut()
            .set_max_size(Vec2::new(8, 20));
        project_strip_layout.add_widget(Box::new(dock_control_separator));

        let mut dock_edit_maximize = TheTraybarButton::new(TheId::named("Dock Edit Maximize"));
        dock_edit_maximize.set_icon_name("frame_corners".to_string());
        dock_edit_maximize.set_status_text(&format!(
            "{} ({})",
            fl!("action_edit_maximize_desc"),
            DockManager::edit_maximize_accelerator().description()
        ));
        dock_edit_maximize.set_disabled(!dock_is_normal || !dock_is_available);
        project_strip_layout.add_widget(Box::new(dock_edit_maximize));

        let mut dock_restore = TheTraybarButton::new(TheId::named("Dock Restore"));
        dock_restore.set_icon_name("caret-down".to_string());
        dock_restore.set_status_text(&format!(
            "{} ({})",
            fl!("action_minimize_desc"),
            DockManager::restore_accelerator().description()
        ));
        dock_restore.set_disabled(dock_is_normal);
        project_strip_layout.add_widget(Box::new(dock_restore));
        project_strip_layout.set_reverse_index(Some(4));

        tabs_canvas.set_layout(project_strip_layout);
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
        // The active tool can select its dock while the UI is still being
        // assembled. Synchronize once more now that these controls are part of
        // the live canvas, so their widget-local disabled state is updated too.
        DOCKMANAGER.read().unwrap().sync_size_controls(ui, ctx);

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

        #[cfg(not(target_arch = "wasm32"))]
        let mut scepter_events = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(receiver) = &mut self.scepter_receiver {
            while let Ok(event) = receiver.try_recv() {
                scepter_events.push(event);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        for event in scepter_events {
            match event {
                ScepterEvent::Ping { message, peer } => {
                    let status = if message.trim().is_empty() {
                        format!("Scepter ping received from {peer}.")
                    } else {
                        format!("Scepter ping received from {peer}: {message}")
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    redraw = true;
                }
                ScepterEvent::ServiceError(message) => {
                    let status = format!("Scepter service: {message}");
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    redraw = true;
                }
                ScepterEvent::ProjectSnapshot { reply } => {
                    let _ = reply.send(self.scepter_project_snapshot());
                }
                ScepterEvent::ProjectUndo { reply } => {
                    let had_undo = UNDOMANAGER.read().unwrap().has_undo();
                    if had_undo {
                        UNDOMANAGER.write().unwrap().undo(
                            &mut self.server_ctx,
                            &mut self.project,
                            ui,
                            ctx,
                        );
                    }
                    let result = serde_json::json!({
                        "ok": had_undo,
                        "command": "project.undo",
                        "dirty": self.active_session_has_changes(),
                        "message": if had_undo { "undo applied" } else { "nothing to undo" },
                    });
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Scepter undo.".into(),
                    ));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::ProjectRedo { reply } => {
                    let had_redo = UNDOMANAGER.read().unwrap().has_redo();
                    if had_redo {
                        UNDOMANAGER.write().unwrap().redo(
                            &mut self.server_ctx,
                            &mut self.project,
                            ui,
                            ctx,
                        );
                    }
                    let result = serde_json::json!({
                        "ok": had_redo,
                        "command": "project.redo",
                        "dirty": self.active_session_has_changes(),
                        "message": if had_redo { "redo applied" } else { "nothing to redo" },
                    });
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Scepter redo.".into(),
                    ));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::RegionSnapshot { request, reply } => {
                    let _ = reply.send(self.scepter_region_snapshot(&request));
                }
                ScepterEvent::RegionSummary { request, reply } => {
                    let _ = reply.send(self.scepter_region_summary(&request));
                }
                ScepterEvent::RegionRenderPreview { command, reply } => {
                    let _ = reply.send(self.scepter_region_render_preview(&command));
                }
                ScepterEvent::RegionPaintRect { command, reply } => {
                    let result = self.scepter_apply_region_paint_rect(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter painted {} region cells.",
                            result
                                .get("cell_count")
                                .and_then(|value| value.as_u64())
                                .unwrap_or_default()
                        )
                    } else {
                        format!(
                            "Scepter paint failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::RegionPaintCells { command, reply } => {
                    let result = self.scepter_apply_region_paint_cells(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter painted {} region cells.",
                            result
                                .get("cell_count")
                                .and_then(|value| value.as_u64())
                                .unwrap_or_default()
                        )
                    } else {
                        format!(
                            "Scepter paint failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::ActionList { command, reply } => {
                    let _ = reply.send(self.scepter_action_catalog(&command, ctx));
                }
                ScepterEvent::ActionRun { command, reply } => {
                    let result = self.scepter_run_action(command, ui, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter ran action {}.",
                            result
                                .get("action")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown")
                        )
                    } else {
                        format!(
                            "Scepter action failed: {}",
                            result
                                .get("error")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown error")
                        )
                    };
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::ActionRunScript { command, reply } => {
                    let result = self.scepter_run_action_script(command, ui, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter ran {} scripted action(s).",
                            result
                                .get("executed")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or_default()
                        )
                    } else {
                        format!(
                            "Scepter action script failed: {}",
                            result
                                .get("error")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown error")
                        )
                    };
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::ToolList { command, reply } => {
                    let _ = reply.send(self.scepter_tool_catalog(&command));
                }
                ScepterEvent::ToolSelect { command, reply } => {
                    let result = self.scepter_select_tool(command, ui, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter selected tool {}.",
                            result
                                .get("tool")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown")
                        )
                    } else {
                        format!(
                            "Scepter tool selection failed: {}",
                            result
                                .get("error")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown error")
                        )
                    };
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::ScriptGet { command, reply } => {
                    let _ = reply.send(self.scepter_get_script(&command));
                }
                ScepterEvent::ScriptValidate { command, reply } => {
                    let _ = reply.send(
                        self.scepter_validate_eldrin_source(&command.target, &command.source),
                    );
                }
                ScepterEvent::ScriptPatch { command, reply } => {
                    let result = self.scepter_apply_script_patch(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter edited {} script: {}.",
                            result
                                .get("kind")
                                .and_then(|value| value.as_str())
                                .unwrap_or("target"),
                            result
                                .get("name")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown")
                        )
                    } else {
                        format!(
                            "Scepter script edit failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::AttributesGet { command, reply } => {
                    let _ = reply.send(self.scepter_get_attributes(&command));
                }
                ScepterEvent::AttributesPatch { command, reply } => {
                    let result = self.scepter_apply_attributes_patch(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter edited {} attributes: {}.",
                            result
                                .get("kind")
                                .and_then(|value| value.as_str())
                                .unwrap_or("target"),
                            result
                                .get("name")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown")
                        )
                    } else {
                        format!(
                            "Scepter attribute edit failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::TilesSnapshot { reply } => {
                    let _ = reply.send(self.scepter_tiles_snapshot());
                }
            }
        }

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

        if TOOLLIST.write().unwrap().update_current_tool(
            ui,
            ctx,
            &mut self.project,
            &mut self.server_ctx,
        ) {
            redraw = true;
        }

        // Get build results from the scene manager if any
        while let Some(result) = SCENEMANAGER.write().unwrap().receive() {
            match result {
                SceneManagerResult::Startup => {
                    println!("Scene manager has started up.");
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
                        .build_index
                        .remove_chunk_origin((chunk.origin.x, chunk.origin.y));
                    rusterix
                        .scene_handler
                        .vm
                        .execute(scenevm::Atom::RemoveChunkAt {
                            origin: chunk.origin,
                        });

                    rusterix.scene_handler.build_index.index_chunk(&chunk);
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
                SceneManagerResult::Clear => {
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix
                        .scene_handler
                        .vm
                        .execute(scenevm::Atom::ClearGeometry);

                    rusterix.scene_handler.billboards.clear();
                    rusterix.scene_handler.build_index.clear();
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
        Self::coalesce_polyview_hover_events(&mut pending_events);
        if !pending_events.is_empty() {
            let only_3d_polyview_hover = self.server_ctx.editor_view_mode != EditorViewMode::D2
                && pending_events.iter().all(|event| {
                    matches!(
                        event,
                        TheEvent::RenderViewHoverChanged(id, _)
                            | TheEvent::RenderViewLostHover(id) if id.name == "PolyView"
                    )
                });
            let only_3d_geometry_drag = self.server_ctx.editor_view_mode != EditorViewMode::D2
                && matches!(
                    self.server_ctx.curr_map_tool_type,
                    MapToolType::Vertex | MapToolType::Linedef | MapToolType::Sector
                )
                && self.server_ctx.geometry_edit_mode != GeometryEditMode::Detail
                && pending_events.iter().all(|event| {
                    matches!(
                        event,
                        TheEvent::RenderViewDragged(id, _) if id.name == "PolyView"
                    )
                });
            if only_3d_polyview_hover || only_3d_geometry_drag {
                if let Some(last_event) = pending_events.pop() {
                    pending_events.clear();
                    pending_events.push(last_event);
                }
            }

            if only_3d_polyview_hover {
                self.last_3d_hover_redraw_at = Some(std::time::Instant::now());
                redraw_update = true;
            } else {
                redraw_update = true;
            }
        }

        if let Some(receiver) = &mut self.starter_loader_rx
            && let Ok(starters) = receiver.try_recv()
        {
            self.starter_manifest_cache = Some(starters.clone());
            self.starter_projects = starters;
            self.rebuild_starter_project_list(ui, ctx);
            if let Some(first) = self.starter_projects.first() {
                let manifest_id = first.manifest_id.clone();
                self.selected_starter_manifest_id = Some(manifest_id.clone());
                ctx.ui.send(TheEvent::StateChanged(
                    TheId::named_with_id("Starter Project List Item", first.id),
                    TheWidgetState::Selected,
                ));
                self.update_starter_project_preview(&manifest_id, ui, ctx);
                ui.set_enabled(Self::STARTER_CREATE_ID, ctx);
            } else if let Some(list) = ui.get_list_layout(Self::STARTER_LIST_ID) {
                list.clear();
                let mut item = TheListItem::new(TheId::named("Starter Project Empty"));
                item.set_text(fl!("starter_empty"));
                item.set_sub_text(fl!("starter_empty_sub"));
                item.set_size(70);
                item.set_text_color(WHITE);
                item.set_text_size(14.0);
                item.set_sub_text_color([170, 176, 184, 255]);
                item.set_sub_text_size(12.0);
                list.add_item(item, ctx);
                ui.set_widget_value(
                    Self::STARTER_PREVIEW_KIND_ID,
                    ctx,
                    TheValue::Text(String::new()),
                );
                ui.set_widget_value(
                    Self::STARTER_PREVIEW_TITLE_ID,
                    ctx,
                    TheValue::Text(fl!("starter_empty")),
                );
                ui.set_widget_value(
                    Self::STARTER_PREVIEW_DESCRIPTION_ID,
                    ctx,
                    TheValue::Text(String::new()),
                );
            }
            self.starter_loader_rx = None;
            ctx.ui.relayout = true;
            ctx.ui.redraw_all = true;
            redraw_update = true;
        }

        let starter_cancel_pending = pending_events.iter().any(|event| {
            matches!(
                event,
                TheEvent::StateChanged(id, TheWidgetState::Clicked)
                    if id.name == Self::STARTER_CANCEL_ID
            )
        });
        let loaded_starter = (!starter_cancel_pending)
            .then(|| {
                self.starter_project_loader_rx
                    .as_mut()
                    .and_then(|receiver| receiver.try_recv().ok())
            })
            .flatten();
        if let Some((manifest_id, project)) = loaded_starter {
            self.starter_project_loader_rx = None;
            ui.set_widget_value(
                Self::STARTER_CREATE_ID,
                ctx,
                TheValue::Text(fl!("starter_create")),
            );
            // The tray button sizes itself from its label. Re-layout after
            // replacing the longer loading text so a failed load restores the
            // compact action button instead of retaining the loading width.
            ctx.ui.relayout = true;
            ui.set_enabled(Self::STARTER_CREATE_ID, ctx);

            if let Some(project) = project {
                self.starter_project_cache
                    .insert(manifest_id, project.clone());
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
                    fl!("status_starter_initialized"),
                ));
            } else {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_starter_load_failed"),
                ));
            }

            ctx.ui
                .set_widget_state(Self::STARTER_CREATE_ID.to_string(), TheWidgetState::None);
            ctx.ui.clear_hover();
            redraw_update = true;
            redraw = true;
        }

        if tick_update {
            {
                let mut rusterix = RUSTERIX.write().unwrap();
                let server_state = rusterix.server.state;
                let is_running = server_state == rusterix::ServerState::Running;
                let animate_editor = self.server_ctx.editor_view_mode == EditorViewMode::D2;
                let preview_prefab_effects = self.server_ctx.pc.is_prefab()
                    && self.server_ctx.curr_map_tool_type == MapToolType::Effects
                    && self.server_ctx.editor_view_mode != EditorViewMode::D2;
                if Self::should_advance_animation_frame(
                    server_state,
                    self.server_ctx.editor_view_mode,
                ) {
                    rusterix.client.inc_animation_frame();
                }
                if is_running {
                    rusterix.scene_handler.tick_particle_clocks();
                } else if animate_editor {
                    // Keep inexpensive 2D editor particles animated. A stopped
                    // 3D editor freezes both its animation frame and particles;
                    // either value changes the dynamics hash and would rebuild
                    // a generated scene for hundreds of milliseconds per frame.
                    rusterix.scene_handler.tick_particle_clock_2d();
                } else if preview_prefab_effects {
                    // The isolated Prefab FX editor is deliberately live. It
                    // contains only one asset, so advancing its 3D emitters is
                    // bounded and gives authors immediate lifetime feedback.
                    rusterix.scene_handler.tick_particle_clock_3d();
                }
            }

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
            SCENEMANAGER.write().unwrap().tick_batch(8);

            self.build_values.set(
                "no_rect_geo",
                Value::Bool(self.server_ctx.no_rect_geo_on_map),
            );

            extract_build_values_from_config(&mut self.build_values);

            let mut messages = Vec::new();
            let mut says = Vec::new();
            let mut choices = Vec::new();
            let is_running =
                RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running;
            let running_game_mode = is_running && self.server_ctx.game_mode;

            // Update entities when the server is running
            {
                let rusterix = &mut RUSTERIX.write().unwrap();
                if is_running {
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
                        crate::docks::log::LogDock::set_output(&log_text, ui, ctx);

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
                                TheId::named("Show Debug Log"),
                                TheWidgetState::Clicked,
                            ));
                        }
                        self.last_processed_log_len = log_text.len();
                    }
                    let active_game_map = rusterix.client.current_map.clone();
                    let mut refresh_visual_debug = false;
                    for r in &mut self.project.regions {
                        rusterix.server.apply_entities_items(&mut r.map);

                        let is_active_region = if running_game_mode {
                            r.map.name == active_game_map
                        } else {
                            r.id == self.server_ctx.curr_region
                        };

                        if is_active_region {
                            refresh_visual_debug = true;
                            if let Some(time) = rusterix.server.get_time(&r.map.id) {
                                rusterix.client.set_server_time(time);
                                if let Some(widget) = ui.get_widget("Server Time Slider") {
                                    widget.set_value(TheValue::Time(rusterix.client.server_time));
                                }
                            }
                            messages = rusterix.server.get_messages(&r.map.id);
                            says = rusterix.server.get_says(&r.map.id);
                            choices = rusterix.server.get_choices(&r.map.id);

                            if !running_game_mode {
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
                        DOCKMANAGER.write().unwrap().apply_eldrin_debug_data(
                            ui,
                            ctx,
                            &self.project,
                            &self.server_ctx,
                            &rusterix.server.eldrin_debug,
                        );
                    }
                }
            }
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

            // Draw the regular region canvas or the isolated Prefab canvas.
            // They intentionally have distinct widget identities so editor
            // state and visual content cannot leak between them.
            let render_view_name = if self.server_ctx.pc.is_prefab() {
                "PrefabView"
            } else {
                "PolyView"
            };
            if let Some(render_view) = ui.get_render_view(render_view_name) {
                let dim = *render_view.dim();

                let buffer = render_view.render_buffer_mut();
                buffer.resize(dim.width, dim.height);

                {
                    for paint in self.project.block_prop_paint.values_mut() {
                        paint.ensure_baked_paint_current();
                    }
                    let prefab_paint_catalog = self.project.block_prop_paint.clone();
                    // If we are drawing billboard vertices in the geometry overlay, update them.
                    if !running_game_mode && self.server_ctx.editor_view_mode != EditorViewMode::D2
                    {
                        let mut tools = TOOLLIST.write().unwrap();
                        // `curr_map_tool_type` is also changed by docks/actions
                        // and can therefore say Vertex while another toolbar
                        // tool is visibly active. This redraw is specifically
                        // owned by VertexTool, so query the actual selected tool.
                        if tools.current_game_tool_command_id() == Some("tool.vertex") {
                            tools.update_geometry_overlay_3d(
                                &mut self.project,
                                &mut self.server_ctx,
                            );
                        }
                    }

                    let rusterix = &mut RUSTERIX.write().unwrap();
                    rusterix.client.builder_d2.draw_grid = self.server_ctx.show_editor_grid;

                    if running_game_mode {
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
                                let region_id = r.id;
                                r.iso_paint.ensure_baked_paint_current();
                                let mut iso_paint = r.iso_paint.clone();
                                crate::block_props::merge_prefab_paint_for_map(
                                    &mut iso_paint,
                                    &r.map,
                                    &rusterix.assets.block_props,
                                    &prefab_paint_catalog,
                                );
                                let has_iso_paint = iso_paint.visible
                                    && (!iso_paint.surface_commit_strokes.is_empty()
                                        || !iso_paint.chunks.is_empty()
                                        || !iso_paint.baked_chunks.is_empty());
                                if !has_iso_paint {
                                    let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                    rusterix.scene_handler.vm.set_active_vm(0);
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .execute(scenevm::Atom::ClearPaintBillboards);
                                    rusterix.scene_handler.vm.set_active_vm(active_vm);
                                    self.iso_paint_render_cache = Default::default();
                                }
                                rusterix.draw_game_with_widget_overlays(
                                    &r.map,
                                    game_messages,
                                    game_says,
                                    game_choices,
                                    |widget, scene_handler| {
                                        let render_dim = widget.render_surface_dim();
                                        if render_dim.width <= 0 || render_dim.height <= 0 {
                                            scene_handler
                                                .vm
                                                .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                            scene_handler
                                                .vm
                                                .execute(scenevm::Atom::ClearPaintBillboards);
                                            self.iso_paint_render_cache = Default::default();
                                            return true;
                                        }
                                        let scene_camera =
                                            widget.camera_d3.as_scenevm_camera_for_surface(
                                                render_dim.width as f32,
                                                render_dim.height as f32,
                                            );
                                        let active_vm = scene_handler.vm.active_vm_index();
                                        scene_handler.vm.set_active_vm(0);
                                        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
                                            camera: scene_camera,
                                        });
                                        let view = widget.camera_d3.view_matrix_for_surface(
                                            render_dim.width as f32,
                                            render_dim.height as f32,
                                        );
                                        let render_proj = widget.camera_d3.projection_matrix(
                                            render_dim.width as f32,
                                            render_dim.height as f32,
                                        );
                                        let camera_scale = Some(widget.camera_d3.scale());
                                        IsoPaintRenderer::upload_overlay_cached(
                                            &mut self.iso_paint_render_cache,
                                            region_id,
                                            1,
                                            &mut iso_paint,
                                            &mut scene_handler.vm,
                                            scene_camera,
                                            view,
                                            render_proj,
                                            render_dim.width as u32,
                                            render_dim.height as u32,
                                            camera_scale,
                                        );
                                        scene_handler.vm.set_active_vm(active_vm);
                                        false
                                    },
                                    |_, _| {},
                                );
                                break;
                            }
                        }

                        rusterix
                            .client
                            .insert_game_buffer(render_view.render_buffer_mut());
                    } else {
                        if self.server_ctx.pc.is_prefab()
                            && self.server_ctx.editor_view_mode != EditorViewMode::D2
                        {
                            rusterix.client.set_camera_d3(Box::new(
                                EDITCAMERA.read().unwrap().orbit_camera.clone(),
                            ));
                            let asset_id = match self.server_ctx.pc {
                                ProjectContext::Prefab(asset_id) => Some(asset_id),
                                _ => None,
                            };
                            let mut prefab_paint = asset_id
                                .and_then(|asset_id| {
                                    self.project.block_prop_paint.get(&asset_id).cloned()
                                })
                                .unwrap_or_default();
                            let has_prefab_paint = prefab_paint.visible
                                && (!prefab_paint.surface_commit_strokes.is_empty()
                                    || !prefab_paint.chunks.is_empty()
                                    || !prefab_paint.baked_chunks.is_empty());
                            if has_prefab_paint {
                                let view = rusterix
                                    .client
                                    .camera_d3
                                    .view_matrix_for_surface(dim.width as f32, dim.height as f32);
                                let proj = rusterix
                                    .client
                                    .camera_d3
                                    .projection_matrix(dim.width as f32, dim.height as f32);
                                let camera_scale = Some(rusterix.client.camera_d3.scale());
                                let scene_camera =
                                    rusterix.client.camera_d3.as_scenevm_camera_for_surface(
                                        dim.width as f32,
                                        dim.height as f32,
                                    );
                                let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                rusterix.scene_handler.vm.set_active_vm(0);
                                rusterix
                                    .scene_handler
                                    .vm
                                    .execute(scenevm::Atom::SetCamera3D {
                                        camera: scene_camera,
                                    });
                                IsoPaintRenderer::upload_overlay_cached(
                                    &mut self.iso_paint_render_cache,
                                    asset_id.unwrap_or_default(),
                                    2,
                                    &mut prefab_paint,
                                    &mut rusterix.scene_handler.vm,
                                    scene_camera,
                                    view,
                                    proj,
                                    dim.width as u32,
                                    dim.height as u32,
                                    camera_scale,
                                );
                                rusterix.scene_handler.vm.set_active_vm(active_vm);
                            } else {
                                let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                rusterix.scene_handler.vm.set_active_vm(0);
                                rusterix
                                    .scene_handler
                                    .vm
                                    .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                rusterix
                                    .scene_handler
                                    .vm
                                    .execute(scenevm::Atom::ClearPaintBillboards);
                                rusterix.scene_handler.vm.set_active_vm(active_vm);
                                self.iso_paint_render_cache = Default::default();
                            }
                            if let Some(map) = self.project.get_map(&self.server_ctx) {
                                // The isolated Prefab scene has no running game
                                // widget to build its dynamic layer. Build it
                                // here so authored particles and lights are
                                // actually uploaded before the viewport draw.
                                let animation_frame = rusterix.client.animation_frame;
                                rusterix.build_dynamics_3d(map, animation_frame);
                                rusterix.draw_d3_with_editor_background(
                                    map,
                                    render_view.render_buffer_mut().pixels_mut(),
                                    dim.width as usize,
                                    dim.height as usize,
                                    true,
                                );
                            }
                        } else if self.server_ctx.editor_view_mode != EditorViewMode::D2
                            && self.server_ctx.get_map_context() == MapContext::Region
                        {
                            let entity_item_selection_visible =
                                TOOLLIST.read().unwrap().current_game_tool_is("tool.entity");
                            rusterix
                                .set_entity_item_selection_visible(entity_item_selection_visible);
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
                                    && (self.server_ctx.editor_fly_nav_active
                                        || EDITCAMERA.read().unwrap().move_action.is_some())
                                {
                                    ctx.ui.redraw_all = true;
                                }

                                // Rebuild dynamic overlays only when their
                                // content/hash requires it. Invalidating the
                                // cache here made the stopped Game tool rebuild
                                // all Stonefall particle overlays every frame,
                                // leaving the whole Creator permanently jerky.
                                let animation_frame = rusterix.client.animation_frame;
                                rusterix.build_dynamics_3d(&region.map, animation_frame);
                                let editor_neutral_background = !is_running;
                                region.iso_paint.ensure_baked_paint_current();
                                let mut combined_iso_paint = region.iso_paint.clone();
                                crate::block_props::merge_prefab_paint_for_map(
                                    &mut combined_iso_paint,
                                    &region.map,
                                    &rusterix.assets.block_props,
                                    &prefab_paint_catalog,
                                );
                                let has_iso_paint = combined_iso_paint.visible
                                    && (!combined_iso_paint.surface_commit_strokes.is_empty()
                                        || !combined_iso_paint.chunks.is_empty()
                                        || !combined_iso_paint.baked_chunks.is_empty());
                                if has_iso_paint {
                                    let view = rusterix.client.camera_d3.view_matrix_for_surface(
                                        dim.width as f32,
                                        dim.height as f32,
                                    );
                                    let proj = rusterix
                                        .client
                                        .camera_d3
                                        .projection_matrix(dim.width as f32, dim.height as f32);
                                    let camera_scale = Some(rusterix.client.camera_d3.scale());
                                    let scene_camera =
                                        rusterix.client.camera_d3.as_scenevm_camera_for_surface(
                                            dim.width as f32,
                                            dim.height as f32,
                                        );
                                    let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                    rusterix.scene_handler.vm.set_active_vm(0);
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .execute(scenevm::Atom::SetCamera3D {
                                            camera: scene_camera,
                                        });
                                    IsoPaintRenderer::upload_overlay_cached(
                                        &mut self.iso_paint_render_cache,
                                        region.id,
                                        0,
                                        &mut combined_iso_paint,
                                        &mut rusterix.scene_handler.vm,
                                        scene_camera,
                                        view,
                                        proj,
                                        dim.width as u32,
                                        dim.height as u32,
                                        camera_scale,
                                    );
                                    rusterix.scene_handler.vm.set_active_vm(active_vm);
                                } else {
                                    let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                    rusterix.scene_handler.vm.set_active_vm(0);
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .execute(scenevm::Atom::ClearPaintBillboards);
                                    rusterix.scene_handler.vm.set_active_vm(active_vm);
                                    self.iso_paint_render_cache = Default::default();
                                }
                                rusterix.draw_d3_with_editor_background(
                                    &region.map,
                                    render_view.render_buffer_mut().pixels_mut(),
                                    dim.width as usize,
                                    dim.height as usize,
                                    editor_neutral_background,
                                );
                                if let Some(persisted) =
                                    rusterix.take_orthographic_bake_persisted_update()
                                {
                                    region.map.orthographic_bake = persisted;
                                    region.map.changed = region.map.changed.wrapping_add(1);
                                    if self.active_session < self.sessions.len() {
                                        self.sessions[self.active_session].dirty = true;
                                    }
                                }
                                let bake_rendering = rusterix.orthographic_bake.is_rendering();
                                let bake_progress = rusterix.orthographic_bake.progress_text();
                                if bake_progress.is_some()
                                    || self.server_ctx.background_progress.as_deref().is_some_and(
                                        |text| {
                                            text.starts_with("Bake")
                                                || text.starts_with("Preparing orthographic bake")
                                        },
                                    )
                                {
                                    self.server_ctx.background_progress = bake_progress;
                                }
                                if bake_rendering {
                                    ctx.ui.redraw_all = true;
                                }
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

                                // let start_time = ctx.get_time();

                                if let Some(clipboard) = &self.server_ctx.paste_clipboard {
                                    // During a paste operation we use a merged map

                                    let mut map = region.map.clone();
                                    if let Some(hover) = self.server_ctx.hover_cursor {
                                        map.paste_at_position(clipboard, hover);
                                    }

                                    rusterix.set_dirty();
                                    rusterix.apply_entities_items(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        &map,
                                        &self.server_ctx.editing_surface,
                                        false,
                                    );
                                } else if let Some(map) = self.project.get_map(&self.server_ctx) {
                                    rusterix.apply_entities_items(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        map,
                                        &self.server_ctx.editing_surface,
                                        false,
                                    );
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
                                    rusterix.scene_handler.vm.set_layer_enabled(1, true);
                                }
                                rusterix.draw_scene(
                                    map,
                                    render_view.render_buffer_mut().pixels_mut(),
                                    dim.width as usize,
                                    dim.height as usize,
                                );
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
                                    // A screen's grid and widget bounds are the editing canvas,
                                    // and editing guides remain available in every 2D context.
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
                                    if rusterix.scene_handler.vm.vm_layer_count() > 1 {
                                        rusterix.scene_handler.vm.set_layer_enabled(1, true);
                                    }
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
                                    if rusterix.scene_handler.vm.vm_layer_count() > 1 {
                                        rusterix.scene_handler.vm.set_layer_enabled(1, true);
                                    }
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
                if !running_game_mode && self.server_ctx.editor_view_mode != EditorViewMode::D2 {
                    let iso_paint = match self.server_ctx.pc {
                        ProjectContext::Prefab(asset_id) => {
                            self.project.block_prop_paint.get(&asset_id).cloned()
                        }
                        _ if self.server_ctx.get_map_context() == MapContext::Region => self
                            .project
                            .get_region(&self.server_ctx.curr_region)
                            .map(|region| region.iso_paint.clone()),
                        _ => None,
                    };
                    if let Some(iso_paint) = iso_paint {
                        let buffer = render_view.render_buffer_mut();
                        if self.server_ctx.curr_map_tool_type == MapToolType::IsoPaint
                            && (self.server_ctx.pc.is_prefab()
                                || self.server_ctx.editor_view_mode == EditorViewMode::Iso)
                        {
                            Self::draw_iso_paint_preview(
                                buffer,
                                &iso_paint,
                                self.server_ctx.iso_paint_hover_screen,
                            );
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
            if self.server_ctx.game_input_mode && !self.server_ctx.game_mode {
                // In game input mode send events to the game tool
                if let Some(game_tool) = TOOLLIST
                    .write()
                    .unwrap()
                    .get_game_tool_by_command_id("tool.game")
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
                    if id.name == "Set Project Undo State" {
                        UNDOMANAGER.read().unwrap().set_undo_state_to_ui(ctx);
                    } else if id.name == "Pick Tile Source" {
                        if let TheValue::List(values) = value {
                            let picked = match values.as_slice() {
                                [TheValue::Text(kind), TheValue::Id(id)] if kind == "single" => {
                                    Some(rusterix::TileSource::SingleTile(*id))
                                }
                                [TheValue::Text(kind), TheValue::Id(id)] if kind == "group" => {
                                    Some(rusterix::TileSource::TileGroup(*id))
                                }
                                [TheValue::Text(kind), TheValue::Id(id), TheValue::Int(index)]
                                    if kind == "group_member" =>
                                {
                                    Some(rusterix::TileSource::TileGroupMember {
                                        group_id: *id,
                                        member_index: (*index).max(0) as u16,
                                    })
                                }
                                [TheValue::Text(kind), TheValue::Id(id)]
                                    if kind == "procedural" =>
                                {
                                    Some(rusterix::TileSource::Procedural(*id))
                                }
                                _ => None,
                            };

                            if let Some(source) = picked {
                                self.server_ctx.curr_tile_source = Some(source.clone());
                                self.server_ctx.curr_tile_id = match source {
                                    rusterix::TileSource::SingleTile(tile_id) => Some(tile_id),
                                    rusterix::TileSource::TileGroupMember {
                                        group_id,
                                        member_index,
                                    } => self
                                        .project
                                        .tile_groups
                                        .get(&group_id)
                                        .and_then(|group| group.members.get(member_index as usize))
                                        .map(|member| member.tile_id),
                                    rusterix::TileSource::TileGroup(group_id) => self
                                        .project
                                        .tile_groups
                                        .get(&group_id)
                                        .and_then(|group| group.members.first())
                                        .map(|member| member.tile_id),
                                    rusterix::TileSource::Procedural(_) => None,
                                };

                                if let Some(tile_id) = self.server_ctx.curr_tile_id {
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Tile Picked"),
                                        TheValue::Id(tile_id),
                                    ));
                                }
                                self.activate_edit_tile_meta_action();
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Tilepicker"),
                                    TheValue::Empty,
                                ));
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Reveal Tilepicker Source"),
                                    TheValue::Empty,
                                ));
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Action List"),
                                    TheValue::Empty,
                                ));
                                redraw = true;
                            }
                        }
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
                    } else if id.name == "Minimize Dock" {
                        DOCKMANAGER.write().unwrap().minimize(
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
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
                                crate::utils::editor_scene_full_rebuild(
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
                        #[cfg(all(
                            feature = "self-update",
                            any(target_os = "windows", target_os = "linux")
                        ))]
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
                TheEvent::RenderViewDrop(id, location, drop) if id.name == "PolyView" => {
                    if drop.id.name.starts_with("Shader") {
                        return true;
                    }

                    let mut grid_pos = Vec2::zero();
                    let mut spawn_y = 0.0;
                    let mut placement_reference_y: Option<f32> = None;
                    let mut support_surface_sample: Option<
                        crate::tools::entity::SupportSurfaceSample,
                    > = None;
                    let use_3d_hit = self.server_ctx.editor_view_mode != EditorViewMode::D2;
                    let placement_clearance = if drop.id.name.starts_with("Character") {
                        2.0
                    } else {
                        1.0
                    };

                    if let Some(map) = self.project.get_map(&self.server_ctx) {
                        if use_3d_hit && let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            let screen_uv = [
                                location.x as f32 / dim.width as f32,
                                location.y as f32 / dim.height as f32,
                            ];
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.scene_handler.vm.set_active_vm(0);
                            let ray = rusterix.scene_handler.vm.ray_from_uv_with_size(
                                dim.width as u32,
                                dim.height as u32,
                                screen_uv,
                            );

                            if let Some(raw) = rusterix.scene_handler.vm.pick_geo_id_at_uv(
                                dim.width as u32,
                                dim.height as u32,
                                screen_uv,
                                false,
                                false,
                            ) {
                                if drop.id.name.starts_with("Item") {
                                    let rendered_object_id = match raw.0 {
                                        scenevm::GeoId::GeometryObject(object_id) => {
                                            Some(object_id)
                                        }
                                        _ => None,
                                    };
                                    let paint_surface_id = rusterix
                                        .scene_handler
                                        .vm
                                        .pick_paint_surface_at_uv(
                                            dim.width as u32,
                                            dim.height as u32,
                                            screen_uv,
                                        )
                                        .filter(|surface| surface.valid)
                                        .map(|surface| surface.paint_geo);
                                    let paint_surface_hit =
                                        rendered_object_id.and_then(|object_id| {
                                            paint_surface_id.and_then(|paint_surface_id| {
                                                rusterix::resolve_block_prop_support_surface_hit(
                                                    &map.block_prop_instances,
                                                    &rusterix.assets.block_props,
                                                    object_id,
                                                    paint_surface_id,
                                                )
                                            })
                                        });
                                    let object_point_surface_hit =
                                        rendered_object_id.and_then(|object_id| {
                                            rusterix::resolve_block_prop_support_surface_hit_at_point(
                                                &map.block_prop_instances,
                                                &rusterix.assets.block_props,
                                                object_id,
                                                raw.1,
                                            )
                                        });
                                    let world_point_surface_hit =
                                        rusterix::resolve_block_prop_support_surface_hit_at_world_point(
                                                    &map.block_prop_instances,
                                                    &rusterix.assets.block_props,
                                                    raw.1,
                                                );
                                    let surface_hit = paint_surface_hit
                                        .or(object_point_surface_hit)
                                        .or(world_point_surface_hit);
                                    if let Some(surface_hit) = surface_hit
                                        && let Some(instance) = map
                                            .block_prop_instances
                                            .iter()
                                            .find(|instance| instance.id == surface_hit.instance_id)
                                        && let Some(asset) =
                                            rusterix.assets.block_props.get(&surface_hit.asset_id)
                                        && let Some(surface) =
                                            asset.find_support_surface(surface_hit.surface_id)
                                        && let Some(mut local_position) =
                                            rusterix::block_prop_support_surface_local_point(
                                                asset, instance, surface.id, raw.1,
                                            )
                                    {
                                        local_position.y = 0.0;
                                        if surface.snap_spacing > 0.0 {
                                            local_position.x =
                                                (local_position.x / surface.snap_spacing).round()
                                                    * surface.snap_spacing;
                                            local_position.z =
                                                (local_position.z / surface.snap_spacing).round()
                                                    * surface.snap_spacing;
                                        }
                                        if let Some(world_position) =
                                            rusterix::block_prop_support_surface_world_point(
                                                asset,
                                                instance,
                                                surface.id,
                                                [
                                                    local_position.x,
                                                    local_position.y,
                                                    local_position.z,
                                                ],
                                            )
                                        {
                                            grid_pos =
                                                Vec2::new(world_position.x, world_position.z);
                                            spawn_y = world_position.y;
                                            placement_reference_y = Some(world_position.y);
                                            support_surface_sample =
                                                Some(crate::tools::entity::SupportSurfaceSample {
                                                    instance_id: instance.id,
                                                    surface_id: surface.id,
                                                    local_position,
                                                    world_position,
                                                });
                                        }
                                    }
                                }
                                if support_surface_sample.is_none()
                                    && let Some((ray_origin, ray_dir)) = ray
                                    && let Some((floor_hit, reference_y)) = map
                                        .geometry_floor_hit_from_ray_for_placement(
                                            ray_origin,
                                            ray_dir,
                                            raw.1,
                                            placement_clearance,
                                        )
                                {
                                    grid_pos = Vec2::new(floor_hit.x, floor_hit.z);
                                    spawn_y = floor_hit.y;
                                    placement_reference_y = Some(reference_y);
                                } else if support_surface_sample.is_none() {
                                    grid_pos = Vec2::new(raw.1.x, raw.1.z);
                                    spawn_y = raw.1.y;
                                    placement_reference_y = Some(raw.1.y);
                                }
                            } else {
                                grid_pos = self.server_ctx.local_to_map_cell(
                                    Vec2::new(dim.width as f32, dim.height as f32),
                                    Vec2::new(location.x as f32, location.y as f32),
                                    map,
                                    map.subdivisions,
                                );
                                grid_pos += 0.5;
                            }
                        } else if let Some(render_view) = ui.get_render_view("PolyView") {
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

                        if use_3d_hit && support_surface_sample.is_none() {
                            let floor_height = if let Some(reference_y) = placement_reference_y {
                                map.geometry_floor_height_nearest(grid_pos, reference_y)
                            } else {
                                map.geometry_floor_height_at(grid_pos)
                            };
                            if let Some(height) = floor_height {
                                spawn_y = height;
                            }
                        }
                    }

                    if drop.id.name.starts_with("Character") {
                        let mut instance = Character {
                            character_id: drop.id.references,
                            position: Vec3::new(grid_pos.x, spawn_y, grid_pos.y),
                            ..Default::default()
                        };

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

                        let mut name = "Item".to_string();
                        if let Some(item) = self.project.items.get(&drop.id.references) {
                            name.clone_from(&item.name);
                        }
                        instance.name = name;
                        let item_instance_id = instance.id;

                        let atom = ProjectUndoAtom::AddRegionItemInstance(
                            self.server_ctx.curr_region,
                            instance,
                        );
                        atom.redo(&mut self.project, ui, ctx, &mut self.server_ctx);
                        let placement = if support_surface_sample.is_some() {
                            self.project
                                .get_map_mut(&self.server_ctx)
                                .map(|map| {
                                    crate::tools::entity::EntityTool::commit_item_surface_placement(
                                        map,
                                        item_instance_id,
                                        support_surface_sample,
                                    )
                                })
                                .unwrap_or_else(|| Err(fl!("status_prefab_surface_item_missing")))
                        } else {
                            Ok(())
                        };
                        match placement {
                            Ok(()) => {
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                                if support_surface_sample.is_some() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        fl!("status_item_placed_on_prefab_surface"),
                                    ));
                                }
                            }
                            Err(message) => {
                                atom.undo(&mut self.project, ui, ctx, &mut self.server_ctx);
                                ctx.ui
                                    .send(TheEvent::SetStatusText(TheId::empty(), message));
                            }
                        }
                    }
                }
                TheEvent::FileRequesterResult(id, paths) => {
                    // Load a palette from a file
                    if id.name == "Palette Import" {
                        for p in paths {
                            let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                            let prev = self.project.art_palette.clone();
                            let prev_materials = self.project.art_palette_materials.clone();
                            self.project.load_art_palette_from_text(contents);
                            *PALETTE.write().unwrap() = self.project.art_palette.clone();
                            {
                                let mut rusterix = RUSTERIX.write().unwrap();
                                rusterix.assets.palette = self.project.art_palette.clone();
                                rusterix.assets.palette_materials =
                                    crate::undo::project_helper::palette_material_values(
                                        &self.project,
                                    );
                                rusterix.assets.palette_material_ids =
                                    crate::undo::project_helper::palette_material_ids(
                                        &self.project,
                                    );
                                rusterix.set_tiles(self.project.tiles.clone(), true);
                                rusterix.set_tile_groups(self.project.tile_groups.clone());
                            }

                            if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                                let index = palette_picker.index();

                                palette_picker.set_palette(self.project.art_palette.clone());
                                if let Some(widget) = ui.get_widget("Palette Color Picker") {
                                    if let Some(color) = &self.project.art_palette[index] {
                                        widget.set_value(TheValue::ColorObject(color.clone()));
                                    }
                                }
                                if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                                    if let Some(color) = &self.project.art_palette[index] {
                                        widget.set_value(TheValue::Text(color.to_hex()));
                                    }
                                }
                            }
                            redraw = true;

                            let undo = ProjectUndoAtom::PaletteEdit(
                                prev,
                                prev_materials,
                                self.project.art_palette.clone(),
                                self.project.art_palette_materials.clone(),
                            );
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                        }
                    } else
                    // Open
                    if id.name == "Open" {
                        for p in paths {
                            match Self::load_project_from_path(&p) {
                                Ok(loaded) => {
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
                                }
                                Err(error) => {
                                    self.replace_next_project_load_in_active_tab = false;
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        format!("Unable to load project: {error}"),
                                    ));
                                }
                            }
                        }
                    } else if id.name == "Save As" {
                        for p in paths {
                            let p = Self::ensure_project_extension(p);
                            self.persist_active_region_view_state();
                            match Self::save_project_to_path(&p, &self.project) {
                                Ok(()) => {
                                    self.project_path = Some(p);
                                    UNDOMANAGER.write().unwrap().mark_saved();
                                    DOCKMANAGER.write().unwrap().mark_saved();
                                    if self.active_session < self.sessions.len() {
                                        self.sessions[self.active_session].dirty = false;
                                        self.sessions[self.active_session].detached_dock_dirty =
                                            false;
                                    }
                                    self.sync_active_session_from_editor();
                                    self.rebuild_project_tabs(ui);
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Project saved successfully.".to_string(),
                                    ));
                                }
                                Err(error) => {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        format!("Unable to save project: {error}"),
                                    ));
                                }
                            }
                        }
                    }
                }
                TheEvent::StateChanged(id, state) => {
                    if id.name == "Dock Edit Maximize" && state == TheWidgetState::Clicked {
                        DOCKMANAGER.write().unwrap().edit_maximize(
                            ui,
                            ctx,
                            &mut self.project,
                            &mut self.server_ctx,
                        );
                        ctx.ui.set_widget_state(
                            "Dock Edit Maximize".to_string(),
                            TheWidgetState::None,
                        );
                        ctx.ui.relayout = true;
                        ctx.ui.redraw_all = true;
                        redraw = true;
                    } else if id.name == "Dock Restore" && state == TheWidgetState::Clicked {
                        DOCKMANAGER.write().unwrap().minimize(
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                        ctx.ui
                            .set_widget_state("Dock Restore".to_string(), TheWidgetState::None);
                        ctx.ui.relayout = true;
                        ctx.ui.redraw_all = true;
                        redraw = true;
                    } else if id.name == "GameInput" {
                        self.server_ctx.game_input_mode = state == TheWidgetState::Clicked;
                    } else if id.name == "Starter Project List Item"
                        && state == TheWidgetState::Selected
                    {
                        let selected_manifest_id = self
                            .starter_projects
                            .iter()
                            .find(|entry| entry.id == id.uuid)
                            .map(|entry| entry.manifest_id.clone());
                        self.selected_starter_manifest_id = selected_manifest_id.clone();
                        if let Some(manifest_id) = selected_manifest_id {
                            self.update_starter_project_preview(&manifest_id, ui, ctx);
                        }
                        redraw = true;
                    } else if id.name == Self::STARTER_CREATE_ID {
                        let selected_manifest_id =
                            self.selected_starter_manifest_id.clone().or_else(|| {
                                self.starter_projects
                                    .first()
                                    .map(|entry| entry.manifest_id.clone())
                            });
                        if let Some(manifest_id) = selected_manifest_id {
                            if let Some(project) =
                                self.starter_project_cache.get(&manifest_id).cloned()
                            {
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
                                    fl!("status_starter_initialized"),
                                ));
                            } else if self.starter_project_loader_rx.is_none()
                                && let Some(choice) = self
                                    .starter_projects
                                    .iter()
                                    .find(|choice| choice.manifest_id == manifest_id)
                            {
                                let project_path = choice.project_path.clone();
                                let (tx, rx) = std::sync::mpsc::channel();
                                self.starter_project_loader_rx = Some(rx);
                                ui.set_widget_value(
                                    Self::STARTER_CREATE_ID,
                                    ctx,
                                    TheValue::Text(fl!("starter_loading_project")),
                                );
                                // The button was initially measured for its action label.
                                // Re-layout so the loading label is not clipped.
                                ctx.ui.relayout = true;
                                ui.set_disabled(Self::STARTER_CREATE_ID, ctx);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("starter_loading_project"),
                                ));
                                std::thread::spawn(move || {
                                    let project = Self::load_starter_project(&project_path);
                                    let _ = tx.send((manifest_id, project));
                                });
                            } else {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_starter_load_failed"),
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
                        self.starter_project_loader_rx = None;
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
                            fl!("status_starter_choose"),
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
                        #[cfg(all(
                            feature = "self-update",
                            any(target_os = "windows", target_os = "linux")
                        ))]
                        {
                            let updater = self.self_updater.lock().unwrap();

                            if updater.has_newer_release() {
                                self.self_update_tx
                                    .send(SelfUpdateEvent::UpdateConfirm(
                                        updater.latest_release().cloned().unwrap(),
                                    ))
                                    .unwrap();
                            } else {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("info_update_check"),
                                ));

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
                        #[cfg(all(feature = "self-update", target_os = "macos"))]
                        {
                            _ = open::that("https://github.com/markusmoenig/Eldiron/releases");
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
                            let path = Self::ensure_project_extension(path);
                            self.persist_active_region_view_state();
                            match Self::save_project_to_path(&path, &self.project) {
                                Ok(()) => {
                                    self.project_path = Some(path.clone());
                                    UNDOMANAGER.write().unwrap().mark_saved();
                                    DOCKMANAGER.write().unwrap().mark_saved();
                                    if self.active_session < self.sessions.len() {
                                        self.sessions[self.active_session].dirty = false;
                                        self.sessions[self.active_session].detached_dock_dirty =
                                            false;
                                    }
                                    self.sync_active_session_from_editor();
                                    self.rebuild_project_tabs(ui);
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Project saved successfully.".to_string(),
                                    ));
                                }
                                Err(error) => {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        format!("Unable to save project: {error}"),
                                    ));
                                }
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
                            TheFileExtension::new(
                                "Eldiron".into(),
                                vec![Self::PROJECT_EXTENSION.to_string()],
                            ),
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
                                self.snapshot_authored_maps_for_play();
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
                                crate::docks::log::LogDock::set_output("", ui, ctx);
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
                        {
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.server.stop();
                            rusterix.clear_say_messages();
                            rusterix.clear_all_audio();
                            rusterix.player_camera = PlayerCamera::D2;
                            rusterix.client.scene.d2_dynamic.clear();
                            rusterix.client.scene.d3_dynamic.clear();
                            rusterix.client.scene.dynamic_lights.clear();
                            rusterix.scene_handler.clear_runtime_overlays();
                            // Stopping changes runtime entities/overlays, not
                            // authored static geometry. Keep the existing
                            // SceneVM chunks and invalidate only dynamics.
                            rusterix.set_overlay_dirty();
                        }

                        self.pending_game_messages.clear();
                        self.pending_game_says.clear();
                        self.pending_game_choices.clear();
                        self.pending_text_game_command = None;
                        self.pending_text_game_runtime_flush = false;
                        TEXTGAME.write().unwrap().reset();

                        // Game input can leave FirstP fly navigation with a
                        // stale pointer or key action. That changes the editor
                        // camera every redraw and therefore defeats the entire
                        // 3D dynamics cache after Stop.
                        self.server_ctx.editor_fly_nav_active = false;
                        self.server_ctx.editor_fly_nav_mouse_down = false;
                        self.server_ctx.editor_fly_nav_space_down = false;
                        {
                            let mut edit_camera = EDITCAMERA.write().unwrap();
                            edit_camera.move_action = None;
                            edit_camera.reset_mouse_tracking();
                        }

                        ui.set_widget_value("InfoView", ctx, TheValue::Text("".into()));
                        let restored_authored_maps = self.restore_authored_maps_after_play();
                        insert_content_into_maps(&mut self.project);
                        self.server_ctx.game_input_mode = false;
                        ctx.set_cursor_visible(true);
                        if restored_authored_maps {
                            // Runtime procedural generation can replace the
                            // whole Stonefall map. Reinstall the authored map
                            // once; otherwise the heavy generated dungeon stays
                            // in the editor forever after Stop.
                            crate::utils::editor_scene_full_rebuild(
                                &self.project,
                                &self.server_ctx,
                            );
                        }
                        if TOOLLIST.write().unwrap().leave_game_tool_after_stop(
                            ui,
                            ctx,
                            &mut self.project,
                            &mut self.server_ctx,
                        ) {
                            redraw = true;
                        }
                        update_server_icons = true;
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
                    } else if id.name == "Show Project Sidebar" {
                        redraw |= self.sidebar.show_project_page(
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                    } else if id.name == "Show Actions Sidebar" {
                        redraw |= self.sidebar.show_actions_page(
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                    } else if id.name == "Show Console" {
                        redraw |= self.sidebar.show_console_page(
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                    } else if id.name == "Show Debug Log" {
                        redraw |= self.sidebar.show_debug_page(
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                    } else if id.name == "Show Help Sidebar" {
                        redraw |= self.sidebar.show_help_page(
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
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
                    if id.name == "Shared V Splitter" {
                        if let Some(screen_y) = value.to_i32() {
                            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                                layout.set_split_position(screen_y);
                            }
                            DOCKMANAGER.write().unwrap().remember_normal_split(ui);
                            ctx.ui.relayout = true;
                            ctx.ui.redraw_all = true;
                            redraw = true;
                        }
                    } else if id.name == "Server Time Slider" {
                        if let TheValue::Time(time) = value {
                            self.project.time = time;
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.client.set_server_time(time);

                            if rusterix.server.state == rusterix::ServerState::Running {
                                if let Some(map) = self.project.get_map(&self.server_ctx) {
                                    rusterix.server.set_time(&map.id, time);
                                }
                            }
                            rusterix.set_dirty();
                            redraw = true;
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

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        while let Ok(event) = self.self_update_rx.try_recv() {
            match event {
                SelfUpdateEvent::AlreadyUpToDate => {
                    Self::set_update_button(ui, ctx, None);

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
                SelfUpdateEvent::UpdateAvailable(release) => {
                    Self::set_update_button(ui, ctx, Some(&release));
                }
                SelfUpdateEvent::UpdateCompleted(release) => {
                    Self::set_update_button(ui, ctx, None);

                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        format!(
                            "Updated to version {}. Please restart the application to enjoy the new features.",
                            release.version
                        ),
                    ));
                }
                SelfUpdateEvent::UpdateConfirm(release) => {
                    Self::set_update_button(ui, ctx, Some(&release));

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
                    Self::set_update_button(ui, ctx, None);

                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        format!("Failed to update Eldiron: {err}"),
                    ));
                }
                SelfUpdateEvent::UpdateStart(release) => {
                    Self::set_update_button(ui, ctx, None);

                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        format!("Updating to version {}...", release.version),
                    ));
                }
            }
        }

        if update_server_icons {
            self.update_server_state_icons(ui);
            redraw = true;
        }

        if DOCKMANAGER.write().unwrap().poll_background(
            ui,
            ctx,
            &mut self.project,
            &mut self.server_ctx,
        ) {
            redraw = true;
        }

        let active_dirty = self.active_session_has_changes();
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

    fn mouse_motion(&mut self, delta_x: f32, delta_y: f32, ctx: &mut TheContext) -> bool {
        if self.server_ctx.game_input_mode
            || self.server_ctx.editor_view_mode == EditorViewMode::D2
            || self.server_ctx.curr_map_tool_type == MapToolType::Game
        {
            return false;
        }

        let Some(region) = self.project.get_region_mut(&self.server_ctx.curr_region) else {
            return false;
        };

        let delta = Vec2::new(delta_x, delta_y);
        let mut handled = false;
        if self.server_ctx.editor_view_mode == EditorViewMode::FirstP
            && self.server_ctx.editor_fly_nav_active
            && self.server_ctx.editor_fly_nav_mouse_down
        {
            EDITCAMERA
                .write()
                .unwrap()
                .mouse_delta_firstp(region, delta);
            handled = true;
        } else if self.server_ctx.editor_view_mode == EditorViewMode::Orbit {
            EDITCAMERA.write().unwrap().mouse_delta_orbit(delta);
            handled = true;
        } else if self.server_ctx.editor_view_mode == EditorViewMode::Iso {
            EDITCAMERA.write().unwrap().pan_3d_by_delta_f32(
                region,
                &self.server_ctx,
                delta,
                Vec2::new(ctx.width as i32, ctx.height as i32),
            );
            handled = true;
        }

        if handled {
            RUSTERIX.write().unwrap().set_dirty();
            ctx.ui.redraw_all = true;
        }
        handled
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_navigation_icons_are_transparent_at_their_runtime_draw_size() {
        for (name, path) in COMPACT_NAVIGATION_ICON_PATHS {
            let icon = rasterize_svg_path_icon(path, 18, 256.0, WHITE);
            assert_eq!((icon.dim().width, icon.dim().height), (18, 18), "{name}");
            let alpha = icon
                .pixels()
                .chunks_exact(4)
                .map(|pixel| pixel[3])
                .collect::<Vec<_>>();
            assert!(alpha.iter().any(|value| *value > 0), "{name} is empty");
            assert!(
                alpha.iter().filter(|value| **value == 0).count() >= 18 * 18 / 4,
                "{name} has an opaque block background"
            );
        }
    }

    #[test]
    fn stopped_3d_editor_does_not_advance_the_dynamics_animation_frame() {
        assert!(!Editor::should_advance_animation_frame(
            rusterix::ServerState::Off,
            EditorViewMode::FirstP,
        ));
        assert!(!Editor::should_advance_animation_frame(
            rusterix::ServerState::Off,
            EditorViewMode::Iso,
        ));
        assert!(Editor::should_advance_animation_frame(
            rusterix::ServerState::Off,
            EditorViewMode::D2,
        ));
        assert!(Editor::should_advance_animation_frame(
            rusterix::ServerState::Running,
            EditorViewMode::FirstP,
        ));
    }

    #[test]
    fn ensure_project_extension_appends_when_missing() {
        let path = PathBuf::from("/tmp/My Project");

        assert_eq!(
            Editor::ensure_project_extension(path),
            PathBuf::from("/tmp/My Project.eldiron")
        );
    }

    #[test]
    fn ensure_project_extension_leaves_existing_extension() {
        let path = PathBuf::from("/tmp/My Project.eldiron");

        assert_eq!(Editor::ensure_project_extension(path.clone()), path);
    }

    #[test]
    fn ensure_project_extension_treats_existing_extension_case_insensitively() {
        let path = PathBuf::from("/tmp/My Project.ELDIRON");

        assert_eq!(Editor::ensure_project_extension(path.clone()), path);
    }

    #[test]
    fn ensure_project_extension_appends_after_other_suffixes() {
        let path = PathBuf::from("/tmp/My Project.backup");

        assert_eq!(
            Editor::ensure_project_extension(path),
            PathBuf::from("/tmp/My Project.backup.eldiron")
        );
    }

    #[test]
    fn coalesces_consecutive_polyview_hover_events_to_the_latest_position() {
        let mut events = vec![
            TheEvent::RenderViewHoverChanged(TheId::named("PolyView"), Vec2::new(10, 20)),
            TheEvent::RenderViewHoverChanged(TheId::named("PolyView"), Vec2::new(30, 40)),
            TheEvent::RenderViewHoverChanged(TheId::named("PolyView"), Vec2::new(50, 60)),
        ];

        Editor::coalesce_polyview_hover_events(&mut events);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            TheEvent::RenderViewHoverChanged(id, coord)
                if id.name == "PolyView" && *coord == Vec2::new(50, 60)
        ));
    }

    #[test]
    fn hover_coalescing_preserves_non_hover_event_boundaries() {
        let mut events = vec![
            TheEvent::RenderViewHoverChanged(TheId::named("PolyView"), Vec2::new(10, 20)),
            TheEvent::RenderViewLostHover(TheId::named("PolyView")),
            TheEvent::RenderViewHoverChanged(TheId::named("PolyView"), Vec2::new(30, 40)),
        ];

        Editor::coalesce_polyview_hover_events(&mut events);

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn selecting_the_active_project_tab_does_not_reload_its_snapshot() {
        assert!(!Editor::session_switch_required(1, 1));
        assert!(Editor::session_switch_required(1, 0));
    }

    #[test]
    fn debug_log_severity_detection_requires_an_explicit_marker() {
        assert!(Editor::log_segment_has_warning_or_error(
            "[warning] missing entrance"
        ));
        assert!(Editor::log_segment_has_warning_or_error(
            "[ERROR] setup failed"
        ));
        assert!(!Editor::log_segment_has_warning_or_error(
            "StartScene: Startup with 0 errors."
        ));
    }

    #[test]
    fn stopping_play_restores_the_authored_map_after_runtime_replacement() {
        let mut editor = Editor::new();
        editor.project.regions = vec![Region::new()];
        editor.project.regions[0].map.name = "Authored".to_string();
        editor.snapshot_authored_maps_for_play();

        editor.project.regions[0].map.name = "Generated Runtime Dungeon".to_string();

        assert!(editor.restore_authored_maps_after_play());
        assert_eq!(editor.project.regions[0].map.name, "Authored");
        assert!(editor.play_map_snapshots.is_none());
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
