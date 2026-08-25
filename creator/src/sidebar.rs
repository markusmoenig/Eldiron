use crate::editor::{
    ACTIONLIST, CONFIG, CONFIGEDITOR, DOCKMANAGER, PALETTE, RUSTERIX, SCENEMANAGER, SIDEBARMODE,
    TOOLLIST, UNDOMANAGER,
};
use crate::minimap::{draw_minimap, draw_minimap_context_label, minimap_bbox_for_map};
use crate::prelude::*;
use crate::undo::project_helper::*;
use rusterix::{AudioEngine, Texture, TileRole};

pub(crate) const SIDEBAR_NAVIGATION_SHORTCUTS: [char; 5] = ['f', 'g', 'h', 'j', 'k'];

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum SidebarMode {
    Region,
    Character,
    Item,
    Tilemap,
    Module,
    Screen,
    Asset,
    Shader,
    Action,
    // Node,
    Debug,
    Palette,
}

pub struct Sidebar {
    pub width: i32,
    console: crate::docks::console::ConsoleDock,
    debug: crate::docks::log::LogDock,
    help: crate::docks::help::HelpDock,

    curr_tilemap_uuid: Option<Uuid>,
    curr_tile_collection_uuid: Option<Uuid>,
    curr_treasury_package_slug: Option<String>,
    pending_palette_drag_undo: Option<(ThePalette, Vec<shared::project::PaletteMaterial>)>,

    pub startup: bool,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    const NAVIGATION_PAGE_COUNT: usize = 5;
    const ACTION_PARAMS_EDITOR: &'static str = "Action Params TOML";
    const PROJECT_ACTION_PARAMS_EDITOR: &'static str = "Project Action Params TOML";

    fn action_params_editor(id: &str) -> TheTextAreaEdit {
        let mut textedit = TheTextAreaEdit::new(TheId::named(id));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_syntax_from_string(source);
            textedit.set_code_type("TOML");
        }
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_theme_from_string(source);
            textedit.set_code_theme("Gruvbox Dark");
        }
        textedit.set_continuous(true);
        textedit.display_line_number(false);
        textedit.use_global_statusbar(true);
        textedit.set_font_size(13.5);
        textedit
    }

    fn navigation_page_status(status: String, index: usize) -> String {
        let accelerator = TheAccelerator::new(
            TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
            SIDEBAR_NAVIGATION_SHORTCUTS[index],
        );
        format!("{status} ({})", accelerator.description())
    }

    fn next_navigation_page(current: usize, reverse: bool) -> usize {
        if reverse {
            (current + Self::NAVIGATION_PAGE_COUNT - 1) % Self::NAVIGATION_PAGE_COUNT
        } else {
            (current + 1) % Self::NAVIGATION_PAGE_COUNT
        }
    }

    fn set_navigation_page(index: usize, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        if index >= Self::NAVIGATION_PAGE_COUNT {
            return false;
        }

        let mut changed = false;
        if let Some(stack) = ui.get_stack_layout("Sidebar Page Stack") {
            changed = stack.index() != index;
            stack.set_index(index);
        }
        if let Some(widget) = ui.get_widget("Sidebar Tabs")
            && let Some(tabs) = widget.as_group_button()
        {
            tabs.set_index(index as i32);
        }

        if changed {
            ctx.ui.relayout = true;
            ctx.ui.redraw_all = true;
        }
        changed
    }

    fn activate_navigation_page(
        &mut self,
        index: usize,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let changed = Self::set_navigation_page(index, ui, ctx);
        if index == 2 {
            self.console.activate(ui, ctx, project, server_ctx);
        } else if index == 3 {
            self.debug.activate(ui, ctx, project, server_ctx);
        } else if index == 4 {
            self.help.activate(ui, ctx, project, server_ctx);
        } else if ctx.ui.focus.as_ref().is_some_and(|id| {
            id.name == "Console Input" || id.name == "LogEdit" || id.name == "Help Input"
        }) {
            ctx.ui.clear_focus();
            ctx.ui.keyboard_focus = None;
        }
        changed
    }

    pub fn reset_for_project_switch(&mut self) {
        self.curr_tilemap_uuid = None;
        self.curr_tile_collection_uuid = None;
        self.curr_treasury_package_slug = None;
        self.pending_palette_drag_undo = None;
        self.startup = true;
    }

    fn preview_audio_asset(asset: &Asset) {
        let AssetBuffer::Audio(bytes) = &asset.buffer else {
            return;
        };

        let mut rusterix = RUSTERIX.write().unwrap();
        if rusterix.audio.is_none() {
            rusterix.audio = AudioEngine::new().ok();
        }
        if let Some(engine) = rusterix.audio.as_ref() {
            let _ = engine.load_clip_from_bytes(&asset.name, bytes);
            engine.clear_bus("preview");
            let _ = engine.play_on_bus(&asset.name, "preview", 1.0, false);
        }
    }

    pub fn new() -> Self {
        Self {
            width: 380,
            console: crate::docks::console::ConsoleDock::new(),
            debug: crate::docks::log::LogDock::new(),
            help: crate::docks::help::HelpDock::new(),

            curr_tilemap_uuid: None,
            curr_tile_collection_uuid: None,
            curr_treasury_package_slug: None,
            pending_palette_drag_undo: None,

            startup: true,
        }
    }

    pub fn init_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // Tree View

        let mut canvas: TheCanvas = TheCanvas::new();

        let mut project_canvas: TheCanvas = TheCanvas::new();
        let mut project_tree_layout = TheTreeLayout::new(TheId::named("Project Tree"));
        let root = project_tree_layout.get_root();

        let mut regions_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("regions"),
            server_ctx.tree_regions_id,
        ));
        regions_node.set_open(true);

        root.add_child(regions_node);

        let characters_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("characters"),
            server_ctx.tree_characters_id,
        ));
        root.add_child(characters_node);

        let items_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("items"),
            server_ctx.tree_items_id,
        ));
        root.add_child(items_node);

        let tilemaps_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("tilesets"),
            server_ctx.tree_tilemaps_id,
        ));
        root.add_child(tilemaps_node);

        let recipes_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("recipes"),
            server_ctx.tree_recipes_id,
        ));
        root.add_child(recipes_node);

        let screens_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("screens"),
            server_ctx.tree_screens_id,
        ));
        root.add_child(screens_node);

        let avatars_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("avatars"),
            server_ctx.tree_avatars_id,
        ));
        root.add_child(avatars_node);

        let mut assets_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("assets"),
            server_ctx.tree_assets_id,
        ));

        let fonts_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("fonts"),
            server_ctx.tree_assets_fonts_id,
        ));
        assets_node.add_child(fonts_node);
        let audio_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            "Audio",
            server_ctx.tree_assets_audio_id,
        ));
        assets_node.add_child(audio_node);
        root.add_child(assets_node);

        let mut palette_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("palette"),
            server_ctx.tree_palette_id,
        ));

        let mut ruleset_palette_label = TheTreeItem::new(TheId::named("Ruleset Palette Header"));
        ruleset_palette_label.set_text(fl!("ruleset_palette"));
        ruleset_palette_label.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        palette_node.add_widget(Box::new(ruleset_palette_label));

        let mut ruleset_item = TheTreeIcons::new(TheId::named("Ruleset Palette Item"));
        ruleset_item.set_icon_count(1);
        ruleset_item.set_icons_per_row(17);
        ruleset_item.set_selected_index(None);
        palette_node.add_widget(Box::new(ruleset_item));

        let mut art_palette_label = TheTreeItem::new(TheId::named("Art Palette Header"));
        art_palette_label.set_text(fl!("art_palette"));
        art_palette_label.set_background_palette(ActionGroups, ActionRole::Editor.palette_slot());
        palette_node.add_widget(Box::new(art_palette_label));

        let mut item = TheTreeIcons::new(TheId::named("Palette Item"));
        item.set_icon_count(256);
        item.set_icons_per_row(17);
        item.set_selected_index(Some(0));
        palette_node.add_widget(Box::new(item));
        root.add_child(palette_node);

        let mut config_node: TheTreeNode = TheTreeNode::new(TheId::named(&fl!("game")));

        let mut config_item = TheTreeItem::new(TheId::named("Project Settings"));
        config_item.set_text(fl!("settings"));
        config_item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        config_node.add_widget(Box::new(config_item));

        let mut world_code_item = TheTreeItem::new(TheId::named("World Code"));
        world_code_item.set_text("World / Eldrin Scripting".to_string());
        world_code_item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        config_node.add_widget(Box::new(world_code_item));

        let mut rules_item = TheTreeItem::new(TheId::named("Game Rules"));
        rules_item.set_text("Rules".to_string());
        rules_item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        config_node.add_widget(Box::new(rules_item));

        let mut locales_item = TheTreeItem::new(TheId::named("Game Locales"));
        locales_item.set_text("Locales".to_string());
        locales_item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        config_node.add_widget(Box::new(locales_item));

        let mut audio_fx_item = TheTreeItem::new(TheId::named("Game Audio FX"));
        audio_fx_item.set_text("Audio FX".to_string());
        audio_fx_item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        config_node.add_widget(Box::new(audio_fx_item));

        let mut authoring_item = TheTreeItem::new(TheId::named("Game Authoring"));
        authoring_item.set_text("Authoring".to_string());
        authoring_item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        config_node.add_widget(Box::new(authoring_item));

        let mut shortcuts_item = TheTreeItem::new(TheId::named("Game Shortcuts"));
        shortcuts_item.set_text("Shortcuts".to_string());
        shortcuts_item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        config_node.add_widget(Box::new(shortcuts_item));

        root.add_child(config_node);

        project_canvas.set_layout(project_tree_layout);

        // Tree View Toolbar

        let mut add_button = TheTraybarButton::new(TheId::named("Project Add"));
        add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_status_text(&fl!("status_project_add_button"));
        add_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Add Region".to_string(), TheId::named("Add Region")),
                TheContextMenuItem::new("Add Character".to_string(), TheId::named("Add Character")),
                TheContextMenuItem::new("Add Item".to_string(), TheId::named("Add Item")),
                TheContextMenuItem::new("Add Tileset".to_string(), TheId::named("Add Tileset")),
                TheContextMenuItem::new("Add Screen".to_string(), TheId::named("Add Screen")),
                TheContextMenuItem::new("Add Avatar".to_string(), TheId::named("Add Avatar")),
                TheContextMenuItem::new(fl!("add_recipe"), TheId::named("Add Procedural Recipe")),
                TheContextMenuItem::new(
                    "Add Font Asset".to_string(),
                    TheId::named("Add Font Asset"),
                ),
                TheContextMenuItem::new(
                    "Add Audio Asset".to_string(),
                    TheId::named("Add Audio Asset"),
                ),
            ],
            ..Default::default()
        }));

        let mut remove_button = TheTraybarButton::new(TheId::named("Project Remove"));
        remove_button.set_icon_name("icon_role_remove".to_string());
        remove_button.set_status_text(&fl!("status_project_remove_button"));

        let mut duplicate_button = TheTraybarButton::new(TheId::named("Project Duplicate"));
        duplicate_button.set_icon_name("duplicate".to_string());
        duplicate_button.set_status_text(&fl!("status_project_duplicate_button"));
        duplicate_button.set_disabled(true);

        let mut project_context_text = TheText::new(TheId::named("Project Context"));
        project_context_text.set_text("".to_string());

        let mut import_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Project Import"));
        import_button.set_icon_name("import".to_string());
        import_button.set_status_text(&fl!("status_project_import_button"));
        import_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Import Region".to_string(), TheId::named("Import Region")),
                TheContextMenuItem::new(
                    "Import Character".to_string(),
                    TheId::named("Import Character"),
                ),
                TheContextMenuItem::new("Import Item".to_string(), TheId::named("Import Item")),
                TheContextMenuItem::new(
                    "Import Tileset".to_string(),
                    TheId::named("Import Tileset"),
                ),
                TheContextMenuItem::new("Import Screen".to_string(), TheId::named("Import Screen")),
                TheContextMenuItem::new(fl!("import_avatar"), TheId::named("Import Avatar")),
                TheContextMenuItem::new(
                    fl!("import_recipe"),
                    TheId::named("Import Procedural Recipe"),
                ),
                TheContextMenuItem::new(
                    fl!("import_avatar_atlas"),
                    TheId::named("Import Avatar Atlas"),
                ),
                TheContextMenuItem::new(
                    "Import Font Asset".to_string(),
                    TheId::named("Import Font Asset"),
                ),
                TheContextMenuItem::new(
                    "Import Audio Asset".to_string(),
                    TheId::named("Import Audio Asset"),
                ),
            ],
            ..Default::default()
        }));

        let mut export_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Project Export Menu"));
        export_button.set_icon_name("export".to_string());
        export_button.set_status_text(&fl!("status_project_export_button"));

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(add_button));
        toolbar_hlayout.add_widget(Box::new(remove_button));
        toolbar_hlayout.add_widget(Box::new(duplicate_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(project_context_text));
        toolbar_hlayout.add_widget(Box::new(import_button));
        toolbar_hlayout.add_widget(Box::new(export_button));

        toolbar_hlayout.set_reverse_index(Some(2));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        project_canvas.set_bottom(toolbar_canvas);

        // Project tree page
        let mut stack_layout = TheStackLayout::new(TheId::named("Tree Stack Layout"));
        stack_layout.add_canvas(project_canvas);
        canvas.set_layout(stack_layout);

        // Project-mode contextual settings. Selection changes already choose the
        // corresponding edit action; this compact panel exposes only that action's
        // parameters, without making the user switch to the full Actions page.
        let mut project_action_params_canvas = TheCanvas::default();
        project_action_params_canvas.set_widget(Self::action_params_editor(
            Self::PROJECT_ACTION_PARAMS_EDITOR,
        ));

        let mut project_action_title = TheText::new(TheId::named("Project Action Settings Title"));
        project_action_title.set_text(fl!("settings"));
        project_action_title.set_text_size(12.0);
        project_action_title.set_vertical_offset(2);

        let mut project_action_apply = TheTraybarButton::new(TheId::named("Project Action Apply"));
        project_action_apply.set_text(fl!("apply"));
        project_action_apply.set_status_text(&fl!("status_dock_action_apply"));

        let mut project_action_toolbar = TheHLayout::new(TheId::empty());
        project_action_toolbar.set_background_color(None);
        project_action_toolbar.set_margin(Vec4::new(10, 1, 5, 1));
        project_action_toolbar.add_widget(Box::new(project_action_title));
        project_action_toolbar.add_widget(Box::new(project_action_apply));
        project_action_toolbar.set_reverse_index(Some(1));

        let mut project_action_toolbar_canvas = TheCanvas::default();
        project_action_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        project_action_toolbar_canvas.set_layout(project_action_toolbar);
        project_action_params_canvas.set_top(project_action_toolbar_canvas);
        project_action_params_canvas.top_is_expanding = false;

        let mut project_page = TheCanvas::default();
        let mut project_shared =
            TheSharedVLayout::new(TheId::named("Project Context Settings Shared"));
        project_shared.add_canvas(canvas);
        project_shared.add_canvas(project_action_params_canvas);
        project_shared.set_mode(TheSharedVLayoutMode::Top);
        project_shared.set_shared_ratio(0.70);
        project_shared.limiter_mut().set_max_width(self.width);
        project_page.set_layout(project_shared);

        // Action parameter page. The action list itself stays owned and
        // populated by DockManager, but is mounted here in the sidebar.
        let mut action_params_canvas = TheCanvas::default();
        let mut action_stack = TheStackLayout::new(TheId::named("Sidebar Bottom Stack"));

        let mut action_params_editor_canvas = TheCanvas::default();
        action_params_editor_canvas
            .set_widget(Self::action_params_editor(Self::ACTION_PARAMS_EDITOR));
        action_stack.add_canvas(action_params_editor_canvas);

        let mut node_settings_canvas = TheCanvas::default();
        let mut text_layout = TheTextLayout::new(TheId::named("Node Settings"));
        text_layout.limiter_mut().set_max_width(self.width);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        node_settings_canvas.set_layout(text_layout);
        action_stack.add_canvas(node_settings_canvas);

        let mut collection_settings_canvas = TheCanvas::default();
        let mut collection_text_layout = TheTextLayout::new(TheId::named("Collection Settings"));
        collection_text_layout
            .limiter_mut()
            .set_max_width(self.width);
        collection_text_layout.set_text_margin(20);
        collection_text_layout.set_text_align(TheHorizontalAlign::Right);
        collection_settings_canvas.set_layout(collection_text_layout);
        action_stack.add_canvas(collection_settings_canvas);

        let mut treasury_settings_canvas = TheCanvas::default();
        let mut treasury_text_layout = TheTextLayout::new(TheId::named("Treasury Settings"));
        treasury_text_layout.limiter_mut().set_max_width(self.width);
        treasury_text_layout.set_text_margin(20);
        treasury_text_layout.set_text_align(TheHorizontalAlign::Right);
        treasury_settings_canvas.set_layout(treasury_text_layout);
        action_stack.add_canvas(treasury_settings_canvas);

        action_stack.set_index(0);
        action_params_canvas.set_layout(action_stack);

        let mut shortcut_label = TheText::new(TheId::named("Action Shortcut Label"));
        shortcut_label.set_text(fl!("action_shortcut"));
        shortcut_label.set_text_size(12.0);

        let mut shortcut_value = TheText::new(TheId::named("Action Shortcut Value"));
        shortcut_value.set_text("—".to_string());
        shortcut_value.set_text_size(12.0);

        let mut shortcut_layout = TheHLayout::new(TheId::named("Action Shortcut Layout"));
        shortcut_layout.set_background_color(None);
        shortcut_layout.set_margin(Vec4::new(10, 2, 10, 2));
        shortcut_layout.add_widget(Box::new(shortcut_label));
        shortcut_layout.add_widget(Box::new(shortcut_value));
        shortcut_layout.set_reverse_index(Some(1));

        let mut shortcut_canvas = TheCanvas::default();
        shortcut_canvas.set_widget(TheTraybar::new(TheId::empty()));
        shortcut_canvas.set_layout(shortcut_layout);
        action_params_canvas.set_top(shortcut_canvas);
        action_params_canvas.top_is_expanding = false;

        let mut actions_canvas = TheCanvas::default();
        let mut actions_shared = TheSharedVLayout::new(TheId::named("Sidebar Actions Shared"));
        actions_shared.add_canvas(crate::dockmanager::DockManager::action_panel("Action List"));
        actions_shared.add_canvas(action_params_canvas);
        actions_shared.set_mode(TheSharedVLayoutMode::Shared);
        actions_shared.set_shared_ratio(0.58);
        actions_canvas.set_layout(actions_shared);

        // Compact, icon-only navigation. A stack keeps this open-ended for
        // additional sidebar modes without consuming header width with labels.
        let mut sidebar_pages = TheStackLayout::new(TheId::named("Sidebar Page Stack"));
        sidebar_pages.add_canvas(project_page);
        sidebar_pages.add_canvas(actions_canvas);
        sidebar_pages.add_canvas(self.console.setup(ctx));
        sidebar_pages.add_canvas(self.debug.setup(ctx));
        sidebar_pages.add_canvas(self.help.setup(ctx));
        sidebar_pages.set_index(0);

        let mut pages_canvas = TheCanvas::default();
        pages_canvas.set_layout(sidebar_pages);

        let mut minimap_canvas = TheCanvas::default();
        let mut minimap = TheRenderView::new(TheId::named("MiniMap"));
        minimap.limiter_mut().set_max_width(self.width);
        minimap_canvas.set_widget(minimap);

        // The map remains visible independently of the selected sidebar page.
        let mut sidebar_shared = TheSharedVLayout::new(TheId::named("Sidebar Map Shared"));
        sidebar_shared.add_canvas(pages_canvas);
        sidebar_shared.add_canvas(minimap_canvas);
        sidebar_shared.set_mode(TheSharedVLayoutMode::Shared);
        sidebar_shared.set_shared_ratio(0.78);
        sidebar_shared.limiter_mut().set_max_width(self.width);

        let mut right_canvas = TheCanvas::new();
        right_canvas.set_layout(sidebar_shared);

        let mut sidebar_tabs = TheGroupButton::new(TheId::named("Sidebar Tabs"));
        sidebar_tabs.add_text_status_icon(
            String::new(),
            Self::navigation_page_status(fl!("tooltip_sidebar_project"), 0),
            "project".to_string(),
        );
        sidebar_tabs.add_text_status_icon(
            String::new(),
            Self::navigation_page_status(fl!("tooltip_sidebar_actions"), 1),
            "graph".to_string(),
        );
        sidebar_tabs.add_text_status_icon(
            String::new(),
            Self::navigation_page_status(fl!("tooltip_sidebar_console"), 2),
            "terminal-nav".to_string(),
        );
        sidebar_tabs.add_text_status_icon(
            String::new(),
            Self::navigation_page_status(fl!("tooltip_sidebar_debug"), 3),
            "diagnostics-nav".to_string(),
        );
        sidebar_tabs.add_text_status_icon(
            String::new(),
            Self::navigation_page_status(fl!("tooltip_sidebar_help"), 4),
            "question-mark".to_string(),
        );
        sidebar_tabs.set_item_width(30);

        let mut tab_layout = TheHLayout::new(TheId::named("Sidebar Tab Layout"));
        tab_layout.set_background_color(None);
        tab_layout.set_margin(Vec4::new(5, 2, 5, 2));
        tab_layout.add_widget(Box::new(sidebar_tabs));

        let mut tab_canvas = TheCanvas::default();
        tab_canvas.set_widget(TheTraybar::new(TheId::empty()));
        tab_canvas.set_layout(tab_layout);
        right_canvas.set_top(tab_canvas);
        right_canvas.top_is_expanding = false;

        ui.canvas.set_right(right_canvas);

        self.apply_region(ui, ctx, None, &mut Project::default());
        self.apply_screen(ui, ctx, None);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = self
            .console
            .handle_event(event, ui, ctx, project, server_ctx);
        redraw |= self.debug.handle_event(event, ui, ctx, project, server_ctx);
        redraw |= self.help.handle_event(event, ui, ctx, project, server_ctx);

        let console_requests = self.console.take_pending_requests();
        if !console_requests.is_empty() {
            let mut results = Vec::new();
            let mut clear = false;
            for request in console_requests {
                let result = match request {
                    crate::docks::console::ConsoleRequest::RunAction(request) => self
                        .execute_action_command(&request, ui, ctx, project, server_ctx)
                        .map(|_| {
                            crate::docks::console::ConsoleDock::action_success_document(&request)
                        }),
                    crate::docks::console::ConsoleRequest::SelectTool { command_id } => self
                        .execute_tool_command(&command_id, ui, ctx, project, server_ctx)
                        .map(|changed| {
                            if changed {
                                crate::docks::console::ConsoleDock::success_document(format!(
                                    "Selected tool `{command_id}`."
                                ))
                            } else {
                                crate::docks::console::ConsoleDock::success_document(format!(
                                    "Tool `{command_id}` is already selected."
                                ))
                            }
                        }),
                    crate::docks::console::ConsoleRequest::Clear => {
                        clear = true;
                        Ok(TheFeedbackDocument::default())
                    }
                    request => self
                        .console
                        .execute_local_request(&request, project, server_ctx, ctx),
                };

                match result {
                    Ok(result) => results.push(result),
                    Err(error) => {
                        results.push(crate::docks::console::ConsoleDock::error_document(error));
                        break;
                    }
                }
            }
            self.console.complete_requests(&results, clear, ui, ctx);
            redraw = true;
        }

        match event {
            TheEvent::SnapperStateChanged(id, _layout_id, open) => {
                if *open {
                    // Region
                    if project.contains_region(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Region(id.uuid),
                        );
                        self.apply_region(ui, ctx, Some(id.uuid), project);
                    } else
                    // Character
                    if let Some(_character) = project.characters.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(id.uuid),
                        );
                    } else
                    // Item
                    if let Some(_item) = project.items.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Item(id.uuid),
                        );
                    } else
                    // Tilemap
                    if let Some(_item) = project.get_tilemap(id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.uuid),
                        );
                    } else
                    // Screen
                    if let Some(_item) = project.screens.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(id.uuid),
                        );
                    } else
                    // Asset
                    if let Some(_item) = project.assets.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(id.uuid),
                        );
                    } else if id.uuid == server_ctx.tree_palette_id {
                        *SIDEBARMODE.write().unwrap() = SidebarMode::Palette;
                        apply_palette(ui, ctx, server_ctx, project);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Action List"),
                            TheValue::Empty,
                        ));
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Sidebar Tabs" {
                    redraw |= self.activate_navigation_page(*index, ui, ctx, project, server_ctx);
                } else if id.name == "Character Region Override" {
                    server_ctx.character_region_override = *index == 1;
                } else if id.name == "Item Region Override" {
                    server_ctx.item_region_override = *index == 1;
                } else if id.name == "Palette Item" {
                    project.art_palette.current_index = *index as u16;
                    apply_palette(ui, ctx, server_ctx, project);
                } else if id.name == "Avatar Perspective Count" {
                    let new_count = match index {
                        0 => AvatarPerspectiveCount::One,
                        1 => AvatarPerspectiveCount::Four,
                        _ => AvatarPerspectiveCount::Eight,
                    };
                    if let Some(avatar) = project.avatars.get(&id.references) {
                        let old_count = avatar.perspective_count;
                        if old_count != new_count {
                            let atom = ProjectUndoAtom::EditAvatarPerspectiveCount(
                                id.references,
                                old_count,
                                new_count,
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        }
                    }
                } else if id.name.starts_with("Avatar Perspective Icons ") {
                    // Parse the perspective index from the widget name
                    if let Some(persp_index_str) = id.name.strip_prefix("Avatar Perspective Icons ")
                    {
                        if let Ok(persp_index) = persp_index_str.parse::<usize>() {
                            let anim_id = id.references;
                            let frame_index = *index as usize;

                            // Find the avatar that owns this animation
                            if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                                let avatar_id = avatar.id;

                                server_ctx.editing_ctx = PixelEditingContext::AvatarFrame(
                                    avatar_id,
                                    anim_id,
                                    persp_index,
                                    frame_index,
                                );

                                // Open the tile editor dock in editor mode
                                let mut dm = DOCKMANAGER.write().unwrap();
                                dm.set_dock("Tiles".into(), ui, ctx, project, server_ctx);
                                dm.edit_maximize(ui, ctx, project, server_ctx);
                            }
                        }
                    }
                } else if id.name == "Item Icon Frames" {
                    let item_id = id.references;
                    let frame_index = *index as usize;
                    let defaults = project
                        .items
                        .get(&item_id)
                        .filter(|item| item.icon_frames.is_empty())
                        .map(|item| resolved_item_default_icon_frames(item, project));
                    if let Some(item) = project.items.get_mut(&item_id) {
                        if item.icon_frames.is_empty() {
                            item.icon_frames = defaults
                                .filter(|frames| !frames.is_empty())
                                .unwrap_or_else(|| vec![rusterix::Texture::alloc(32, 32)]);
                        }
                        if frame_index < item.icon_frames.len() {
                            server_ctx.editing_ctx =
                                PixelEditingContext::ItemIcon(item_id, frame_index);
                            let mut dm = DOCKMANAGER.write().unwrap();
                            dm.set_dock("Tiles".into(), ui, ctx, project, server_ctx);
                            dm.edit_maximize(ui, ctx, project, server_ctx);
                        }
                    }
                }
            }
            TheEvent::RenderViewClicked(id, coord)
            | TheEvent::RenderViewDragged(id, coord)
            | TheEvent::RenderViewUp(id, coord) => {
                if id.name == "MiniMap" {
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();

                        // Color selected
                        let palette_minimap_active = server_ctx.palette_tool_active
                            && DOCKMANAGER.read().unwrap().dock == "Palette";
                        if palette_minimap_active {
                            let buffer = render_view.render_buffer_mut();
                            if let Some(col) = buffer.get_pixel(coord.x, coord.y) {
                                let color = TheColor::from(col);
                                let index = project.art_palette.current_index as usize;
                                project.ensure_art_palette_materials_len();

                                if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                                    widget.set_value(TheValue::Text(color.to_hex()));
                                }

                                if project.art_palette[index] != Some(color.clone()) {
                                    if self.pending_palette_drag_undo.is_none() {
                                        self.pending_palette_drag_undo = Some((
                                            project.art_palette.clone(),
                                            project.art_palette_materials.clone(),
                                        ));
                                    }
                                    project.art_palette[index] = Some(color);
                                    redraw = true;
                                }

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Soft Update Minimap"),
                                    TheValue::Empty,
                                ));

                                apply_palette(ui, ctx, server_ctx, project);
                                if matches!(event, TheEvent::RenderViewUp(_, _)) {
                                    if let Some((prev, prev_materials)) =
                                        self.pending_palette_drag_undo.take()
                                    {
                                        let undo = ProjectUndoAtom::PaletteEdit(
                                            prev,
                                            prev_materials,
                                            project.art_palette.clone(),
                                            project.art_palette_materials.clone(),
                                        );
                                        UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                                    }
                                    crate::undo::project_helper::refresh_palette_runtime(project);
                                }
                            }

                            return redraw;
                        }

                        let width = dim.width as f32;
                        let height = dim.height as f32;
                        let mut update_pos_3d: Option<Vec3<f32>> = None;
                        let mut update_look_at_3d: Option<Vec3<f32>> = None;
                        let mut handled_minimap_input = false;

                        if let Some(map) = project.get_map_mut(server_ctx)
                            && let Some(bbox) = minimap_bbox_for_map(map)
                        {
                            let scale_x = width / bbox.z;
                            let scale_y = height / bbox.w;

                            let bbox_center_x = bbox.x + bbox.z / 2.0;
                            let bbox_center_y = bbox.y + bbox.w / 2.0;

                            let offset_x = -bbox_center_x * scale_x;
                            let offset_y = bbox_center_y * scale_y;

                            let grid_x = (coord.x as f32 - width / 2.0 - offset_x) / scale_x;
                            let grid_y = (coord.y as f32 - height / 2.0 + offset_y) / scale_y;
                            let grid_pos = Vec2::new(grid_x, grid_y);

                            // Keep 2D and 3D camera anchors independent.
                            if server_ctx.editor_view_mode == EditorViewMode::D2 {
                                server_ctx.center_map_at_grid_pos(
                                    Vec2::new(width, height),
                                    grid_pos,
                                    map,
                                );
                            } else if ui.shift
                                && server_ctx.editor_view_mode == EditorViewMode::FirstP
                                && server_ctx.get_map_context() == MapContext::Region
                                && server_ctx.editing_surface.is_none()
                            {
                                let current_y = project
                                    .get_region(&server_ctx.curr_region)
                                    .map(|region| region.editing_look_at_3d.y)
                                    .unwrap_or(0.0);
                                update_look_at_3d = Some(Vec3::new(grid_x, current_y, grid_y));
                            } else if server_ctx.get_map_context() == MapContext::Region
                                && server_ctx.editing_surface.is_none()
                            {
                                let current_y = project
                                    .get_region(&server_ctx.curr_region)
                                    .map(|region| region.editing_position_3d.y)
                                    .unwrap_or(0.0);
                                update_pos_3d = Some(Vec3::new(grid_x, current_y, grid_y));
                            }
                            handled_minimap_input = true;
                        }

                        if let Some(pos_3d) = update_pos_3d
                            && let Some(region) = project.get_region_mut(&server_ctx.curr_region)
                        {
                            region.editing_position_3d = pos_3d;
                        }
                        if let Some(look_at_3d) = update_look_at_3d
                            && let Some(region) = project.get_region_mut(&server_ctx.curr_region)
                        {
                            region.editing_look_at_3d = look_at_3d;
                        }

                        if handled_minimap_input {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Soft Update Minimap"),
                                TheValue::Empty,
                            ));

                            RUSTERIX.write().unwrap().set_dirty();

                            /*
                            let region_width = region.width * region.grid_size;
                            let region_height = region.height * region.grid_size;

                            let minimap_width = dim.width;
                            let minimap_height = dim.height;

                            let scale_x = region_width as f32 / minimap_width as f32;
                            let scale_y = region_height as f32 / minimap_height as f32;

                            // Calculate the real-world coordinates by applying scaling
                            let real_x = (coord.x as f32 * scale_x).round();
                            let real_y = (coord.y as f32 * scale_y).round();

                            // Converting real-world coordinates to tile indices
                            let tile_x = real_x / region.grid_size as f32;
                            let tile_y = real_y / region.grid_size as f32;

                            server_ctx.curr_character_instance = None;
                            server_ctx.curr_item_instance = None;
                            region.editing_position_3d = vec3f(tile_x, 0.0, tile_y);
                            server.set_editing_position_3d(region.editing_position_3d);
                            server.update_region(region);

                            region.scroll_offset = vec2i(
                                (tile_x * region.grid_size as f32) as i32,
                                (tile_y * region.grid_size as f32) as i32,
                            );

                            if let Some(rgba_layout) = ui.get_rgba_layout("TerrainMap") {
                                rgba_layout.scroll_to(region.scroll_offset);
                            }

                            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                                rgba_layout.scroll_to_grid(vec2i(tile_x as i32, tile_y as i32));
                            }
                            */
                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::Resize => {
                ctx.ui.redraw_all = true;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
            }
            TheEvent::WidgetResized(id, dim) => {
                if project.regions.is_empty() && id.name == "PolyView" {
                    if let Some(renderview) = ui.get_render_view("PolyView") {
                        if let Some(buffer) = ctx.ui.icon("eldiron") {
                            let scaled_buffer = buffer.scaled(dim.width, dim.height);
                            *renderview.render_buffer_mut() =
                                TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
                            renderview.render_buffer_mut().fill(BLACK);
                            renderview.render_buffer_mut().copy_into(
                                (dim.width - scaled_buffer.dim().width) / 2,
                                (dim.height - scaled_buffer.dim().height) / 2,
                                &scaled_buffer,
                            );
                            renderview.set_needs_redraw(true);
                        }
                    }
                }
            }
            TheEvent::Custom(id, value) => {
                if id.name == "Editing Texture Updated" {
                    // Update the avatar perspective icon in the Project Tree
                    if let PixelEditingContext::AvatarFrame(
                        avatar_id,
                        anim_id,
                        persp_index,
                        frame_index,
                    ) = server_ctx.editing_ctx
                    {
                        if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx)
                        {
                            let icon_name = format!("Avatar Perspective Icons {}", persp_index);
                            if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                                if let Some(avatars_node) =
                                    tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id)
                                {
                                    // Find the avatar node
                                    if let Some(avatar_node) = avatars_node
                                        .childs
                                        .iter_mut()
                                        .find(|c| c.id.uuid == avatar_id)
                                    {
                                        // Find the animation node
                                        if let Some(anim_node) = avatar_node
                                            .childs
                                            .iter_mut()
                                            .find(|c| c.id.uuid == anim_id)
                                        {
                                            // Find the perspective node containing our icons widget
                                            for persp_node in &mut anim_node.childs {
                                                for widget in &mut persp_node.widgets {
                                                    if widget.id().name == icon_name {
                                                        if let Some(icons) = widget.as_tree_icons()
                                                        {
                                                            icons.set_icon(
                                                                frame_index,
                                                                texture.to_rgba(),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if let PixelEditingContext::ItemIcon(item_id, frame_index) =
                        server_ctx.editing_ctx
                        && let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx)
                        && let Some(tree_layout) = ui.get_tree_layout("Project Tree")
                        && let Some(items_node) =
                            tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id)
                        && let Some(item_node) = items_node
                            .childs
                            .iter_mut()
                            .find(|node| node.id.uuid == item_id)
                    {
                        for widget in &mut item_node.widgets {
                            if widget.id().name == "Item Icon Frames"
                                && let Some(icons) = widget.as_tree_icons()
                            {
                                icons.set_icon(frame_index, texture.to_rgba());
                            }
                        }
                    }
                } else if let TheValue::Id(item_id) = value
                    && id.name == "Item Icon Frames Changed"
                    && let Some(item) = project.items.get(item_id)
                    && let Some(tree_layout) = ui.get_tree_layout("Project Tree")
                    && let Some(items_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id)
                    && let Some(item_node) = items_node
                        .childs
                        .iter_mut()
                        .find(|node| node.id.uuid == *item_id)
                {
                    for widget in &mut item_node.widgets {
                        if widget.id().name == "Item Icon Frames"
                            && let Some(icons) = widget.as_tree_icons()
                        {
                            icons.set_icon_count(item.icon_frames.len().max(1));
                            for (index, texture) in item.icon_frames.iter().enumerate() {
                                icons.set_icon(index, texture.to_rgba());
                            }
                        }
                    }
                } else if id.name == "Backup Editing Position" {
                    if let Some(region) = project.get_region_ctx(server_ctx) {
                        server_ctx.editing_pos_buffer = Some(region.editing_position_3d);
                    }
                } else if id.name == "Update Action Parameters" {
                    // Update the current action params (if any)
                    self.sync_current_action_toml_params(ui, ctx, Some(project), server_ctx);
                    if let Some(curr_action_id) = server_ctx.curr_action_id {
                        if let Some(action) = ACTIONLIST
                            .write()
                            .unwrap()
                            .get_action_by_id_mut(curr_action_id)
                        {
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                action.load_params(map);
                            }
                            action.load_params_project(project, server_ctx);
                            self.show_action_toml_params(ui, ctx, server_ctx, action.as_ref());
                        }
                    }
                } else if id.name == "Refresh Action Parameters" {
                    if let Some(curr_action_id) = server_ctx.curr_action_id
                        && let Some(action) = ACTIONLIST
                            .write()
                            .unwrap()
                            .get_action_by_id_mut(curr_action_id)
                    {
                        self.show_action_toml_params(ui, ctx, server_ctx, action.as_ref());
                    }
                } else if id.name == "Update Action List" {
                    // Update the current action params (if any)
                    if let Some(curr_action_id) = server_ctx.curr_action_id {
                        if let Some(_action) = ACTIONLIST
                            .write()
                            .unwrap()
                            .get_action_by_id_mut(curr_action_id)
                        {
                            // if let Some(map) = project.get_map_mut(&server_ctx) {
                            //     action.load_params(map);
                            // }
                            // action.load_params_project(project, server_ctx);
                        }
                    }
                    self.show_actions(ui, ctx, project, server_ctx);
                    if server_ctx.curr_action_id.is_none() {
                        self.show_empty_action_toml(ui, ctx);
                    }
                } else if id.name == "Nodegraph Id Changed" {
                    if let Some(widget) = ui.get_widget("Graph Id Text") {
                        widget.set_value(TheValue::Text("(--)".into()));
                    }
                } else if id.name == crate::docks::recipes::RECIPE_MINIMAP_PREVIEW {
                    if let TheValue::Image(preview) = value
                        && let Some(render_view) = ui.get_render_view("MiniMap")
                    {
                        // The event is emitted only by the selected Recipe
                        // editor. Do not gate it on ProjectContext: tree/tool
                        // events can change that context while the full-screen
                        // editor is still visible, which used to discard a
                        // successfully rendered preview.
                        let dim = *render_view.dim();
                        let minimap = render_view.render_buffer_mut();
                        if dim.is_valid() {
                            minimap.resize(dim.width, dim.height);
                            crate::docks::recipes::draw_preview_buffer(minimap, preview);
                        } else {
                            // Preserve the fresh pixels even if this event lands
                            // during a relayout; TheRenderView will scale them
                            // when its visible dimension becomes valid.
                            *minimap = preview.clone();
                        }
                        render_view.set_needs_redraw(true);
                        ctx.ui.redraw_all = true;
                        redraw = true;
                    }
                } else if id.name == "Update Minimap" {
                    // Rerenders the minimap
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();
                        let buffer = render_view.render_buffer_mut();
                        buffer.resize(dim.width, dim.height);

                        let mut dock_handled_drawing = false;
                        if let Some(dock) = DOCKMANAGER.read().unwrap().get_active_dock() {
                            // Test if dock is drawing minimap
                            if dock.draw_minimap(buffer, project, ctx, server_ctx) {
                                dock_handled_drawing = true;
                            }
                        }

                        if !dock_handled_drawing {
                            draw_minimap(project, buffer, server_ctx, true);
                            draw_minimap_context_label(buffer, ctx, server_ctx);
                        }
                        render_view.set_needs_redraw(true);
                        redraw = true;
                    }
                } else if id.name == "Soft Update Minimap" {
                    // Uses the currently rendered minimap and only updates the
                    // camera markers
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();
                        let buffer = render_view.render_buffer_mut();
                        buffer.resize(dim.width, dim.height);

                        let mut dock_handled_drawing = false;
                        if let Some(dock) = DOCKMANAGER.read().unwrap().get_active_dock() {
                            // Test if dock is drawing minimap
                            if dock.draw_minimap(buffer, project, ctx, server_ctx) {
                                dock_handled_drawing = true;
                            }
                        }

                        if !dock_handled_drawing {
                            draw_minimap(project, buffer, server_ctx, false);
                            draw_minimap_context_label(buffer, ctx, server_ctx);
                        }
                        render_view.set_needs_redraw(true);
                        redraw = true;
                    }
                } else if id.name == "Update Tiles" {
                    self.update_tiles(ui, ctx, project);
                } else if id.name == "Select Procedural Recipe" {
                    if let TheValue::Id(recipe_id) = value
                        && project.procedural_recipes.contains_key(recipe_id)
                    {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ProceduralRecipe(*recipe_id),
                        );
                        redraw = true;
                    }
                } else if id.name == "Refresh Recipe Tree" {
                    if let TheValue::Id(recipe_id) = value {
                        self.refresh_recipe_tree_item(ui, server_ctx, project, *recipe_id);
                    } else {
                        self.apply_recipes(ui, ctx, server_ctx, project);
                    }
                    redraw = true;
                } else if id.name == "Show Node Settings" {
                    if let Some(stack) = ui.get_stack_layout("Sidebar Bottom Stack") {
                        stack.set_index(1);
                    }
                    if let Some(tab) = ui.get_layout("Multi Tab") {
                        if let Some(tab) = tab.as_tab_layout() {
                            tab.set_index(1);
                        }
                    }
                } else if id.name == "Show Collection Settings" {
                    if let TheValue::Id(collection_id) = value {
                        self.curr_tile_collection_uuid = Some(*collection_id);
                        self.show_collection_settings(ui, ctx, project, *collection_id);
                    }
                } else if id.name == "Hide Collection Settings" {
                    self.curr_tile_collection_uuid = None;
                    if let Some(stack) = ui.get_stack_layout("Sidebar Bottom Stack")
                        && stack.index() == 2
                    {
                        stack.set_index(0);
                    }
                    if let Some(tab) = ui.get_layout("Multi Tab")
                        && let Some(tab) = tab.as_tab_layout()
                    {
                        tab.set_index(1);
                    }
                } else if id.name == "Show Treasury Settings" {
                    if let TheValue::List(values) = value
                        && values.len() >= 5
                    {
                        let slug = values[0].to_string().unwrap_or_default();
                        self.curr_treasury_package_slug = Some(slug);
                        self.show_treasury_settings(
                            ui,
                            ctx,
                            values[1].to_string().unwrap_or_default(),
                            values[2].to_string().unwrap_or_default(),
                            values[3].to_string().unwrap_or_default(),
                            values[4].to_string().unwrap_or_default(),
                        );
                    }
                } else if id.name == "Hide Treasury Settings" {
                    self.curr_treasury_package_slug = None;
                    if let Some(stack) = ui.get_stack_layout("Sidebar Bottom Stack")
                        && stack.index() == 3
                    {
                        stack.set_index(0);
                    }
                    if let Some(tab) = ui.get_layout("Multi Tab")
                        && let Some(tab) = tab.as_tab_layout()
                    {
                        tab.set_index(1);
                    }
                } else if id.name == "Update Content List" {
                    if server_ctx.get_map_context() == MapContext::Region {
                        self.apply_region(ui, ctx, Some(server_ctx.curr_region), project);
                    } else if server_ctx.get_map_context() == MapContext::Screen {
                        self.apply_screen(ui, ctx, project.get_screen_ctx(server_ctx));
                    }
                }
            }
            TheEvent::PaletteIndexChanged(id, index) => {
                if id.name == "Palette Picker" {
                    project.art_palette.current_index = *index;
                    apply_palette(ui, ctx, server_ctx, project);
                    *PALETTE.write().unwrap() = project.art_palette.clone();
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Soft Update Minimap"),
                        TheValue::Empty,
                    ));
                }
            }
            /*
            TheEvent::DialogValueOnClose(role, name, uuid, value) => {
                if name == "Rename Region" && *role == TheDialogButtonRole::Accept {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.name = value.describe();
                        region.map.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Character" && *role == TheDialogButtonRole::Accept {
                    if crate::utils::is_valid_python_variable(&value.describe()) {
                        if let Some(character) = project.characters.get_mut(uuid) {
                            character.name = value.describe();
                            ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                        }
                    }
                } else if name == "Rename Item" && *role == TheDialogButtonRole::Accept {
                    if crate::utils::is_valid_python_variable(&value.describe()) {
                        if let Some(item) = project.items.get_mut(uuid) {
                            item.name = value.describe();
                            ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                        }
                    }
                }
                /*else if name == "Rename Module" && *role == TheDialogButtonRole::Accept {
                    if let Some(bundle) = project.codes.get_mut(uuid) {
                        bundle.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } */
                else if name == "Rename Screen" && *role == TheDialogButtonRole::Accept {
                    if let Some(screen) = project.screens.get_mut(uuid) {
                        screen.name = value.describe();
                        screen.map.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                }
                /*else if name == "Rename Widget" && *role == TheDialogButtonRole::Accept {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
                                widget.name = value.describe();
                                ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                            }
                        }
                    }
                }*/
                else if name == "Rename Asset" && *role == TheDialogButtonRole::Accept {
                    if let Some(asset) = project.assets.get_mut(uuid) {
                        asset.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                }
            }*/
            TheEvent::ContextMenuSelected(widget_id, item_id) => {
                if item_id.name == "Add Image" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(item_id.name.as_str(), Uuid::new_v4()),
                        "Open Image".into(),
                        TheFileExtension::new(
                            "PNG Image".into(),
                            vec!["png".to_string(), "PNG".to_string()],
                        ),
                    );
                } else if item_id.name == "Add Font" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(item_id.name.as_str(), Uuid::new_v4()),
                        "Open Font".into(),
                        TheFileExtension::new(
                            "Font".into(),
                            vec!["ttf".to_string(), "TTF".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Region" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Region Export", export_id),
                        fl!("export_region"),
                        TheFileExtension::new(
                            fl!("eldiron_region"),
                            vec!["eldiron_region".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Character" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Character Export", export_id),
                        fl!("export_character"),
                        TheFileExtension::new(
                            fl!("eldiron_character"),
                            vec!["eldiron_character".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Item" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Item Export", export_id),
                        fl!("export_item"),
                        TheFileExtension::new(
                            fl!("eldiron_item"),
                            vec!["eldiron_item".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Tileset" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Tileset Export", export_id),
                        fl!("export_tileset"),
                        TheFileExtension::new(
                            fl!("eldiron_tileset"),
                            vec!["eldiron_tileset".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Screen" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Screen Export", export_id),
                        fl!("export_screen"),
                        TheFileExtension::new(
                            fl!("eldiron_screen"),
                            vec!["eldiron_screen".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Font Asset" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Font Asset Export", export_id),
                        fl!("export_font_asset"),
                        TheFileExtension::new(
                            fl!("eldiron_font_asset"),
                            vec!["eldiron_font_asset".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Audio Asset" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Audio Asset Export", export_id),
                        fl!("export_audio_asset"),
                        TheFileExtension::new(
                            fl!("eldiron_audio_asset"),
                            vec!["eldiron_audio_asset".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Procedural Recipe" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Procedural Recipe Export", export_id),
                        fl!("export_recipe"),
                        TheFileExtension::new(fl!("recipe_file"), vec!["recipe".to_string()]),
                    );
                } else if item_id.name == "Export Avatar Atlas" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Avatar Atlas Export", export_id),
                        fl!("export_avatar_atlas"),
                        TheFileExtension::new(
                            fl!("png_image"),
                            vec!["png".to_string(), "PNG".to_string()],
                        ),
                    );
                } else if item_id.name == "Export Avatar JSON" {
                    let export_id = item_id.references;
                    ctx.ui.save_file_requester(
                        TheId::named_with_id("Avatar Export", export_id),
                        fl!("export_avatar_json"),
                        TheFileExtension::new(
                            fl!("eldiron_avatar"),
                            vec!["eldiron_avatar".to_string()],
                        ),
                    );
                } else if item_id.name == "Rename Region" {
                    if let Some(tilemap) = project.get_region(&server_ctx.curr_region) {
                        open_text_dialog(
                            "Rename Region",
                            "Region Name",
                            tilemap.name.as_str(),
                            server_ctx.curr_region,
                            ui,
                            ctx,
                        );
                    }
                }
                /*else if item_id.name == "Rename Module" {
                    if let Some(module) = project.codes.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Module",
                            "Module Name",
                            module.name.as_str(),
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                }*/
                else if item_id.name == "Rename Character" {
                    if let Some(character) = project.characters.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Character",
                            "Character Class",
                            &character.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Rename Item" {
                    if let Some(item) = project.items.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Item",
                            "Item Class",
                            &item.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Rename Screen" {
                    if let Some(screen) = project.screens.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Screen",
                            "Screen Name",
                            &screen.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                }
                /*else if item_id.name == "Rename Widget" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
                                open_text_dialog(
                                    "Rename Widget",
                                    "Widget Name",
                                    &widget.name,
                                    widget_id,
                                    ui,
                                    ctx,
                                );
                            }
                        }
                    }
                }*/
                else if item_id.name == "Rename Asset" {
                    if let Some(asset) = project.assets.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Asset",
                            "Asset Name",
                            &asset.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                }
            }
            TheEvent::DragStarted(id, text, offset) => {
                if id.name == "Shader Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Shader: {text}"));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                } else if id.name == "Character Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Character: {text}"));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                } else if id.name == "Item Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Item: {text}"));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                }
            }
            TheEvent::TileDropped(id, _, _) => {
                if let Some(action_id) = server_ctx.curr_action_id
                    && id.name.starts_with("action")
                {
                    if let Some(action) =
                        ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        if action.handle_event(event, project, ui, ctx, server_ctx) {
                            return true;
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == Self::ACTION_PARAMS_EDITOR
                    || id.name == Self::PROJECT_ACTION_PARAMS_EDITOR
                {
                    if let Some(action_id) = server_ctx.curr_action_id
                        && let Some(source) = value.to_string()
                    {
                        self.mirror_action_params_editor(ui, &id.name, &source);
                        if let Some(action) =
                            ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                        {
                            let mut nodeui = action.params();
                            if apply_toml_to_nodeui(&mut nodeui, &source).is_ok() {
                                for (key, val) in nodeui_to_value_pairs(&nodeui) {
                                    let ev = TheEvent::ValueChanged(TheId::named(&key), val);
                                    let _ = action.handle_event(&ev, project, ui, ctx, server_ctx);
                                }

                                if server_ctx.auto_action {
                                    ctx.ui.send(TheEvent::StateChanged(
                                        TheId::named("Action Apply"),
                                        TheWidgetState::Clicked,
                                    ));
                                }
                            }
                        }
                    }
                } else if id.name.starts_with("Region Item Name Edit") {
                    // Rename a region
                    let mut old = String::new();
                    if let Some(region) = project.get_region_mut(&id.uuid) {
                        old = region.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameRegion(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Character Item Name Edit") {
                    // Rename a Character
                    let mut old = String::new();
                    if let Some(character) = project.characters.get(&id.uuid) {
                        old = character.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameCharacter(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Item Item Name Edit") {
                    // Rename an item
                    let mut old = String::new();
                    if let Some(item) = project.items.get(&id.uuid) {
                        old = item.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameItem(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Tilemap Item Name Edit") {
                    // Rename a Tilemap
                    let mut old = String::new();
                    if let Some(tilemap) = project.get_tilemap(id.uuid) {
                        old = tilemap.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameTilemap(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Tilemap Item Grid Edit") {
                    // Edit Tilemap Grid Size
                    let mut old = 0;
                    if let Some(tilemap) = project.get_tilemap(id.references) {
                        old = tilemap.grid_size;
                    }

                    if let Some(size) = value.to_i32()
                        && old != size
                    {
                        let atom = ProjectUndoAtom::EditTilemapGridSize(id.references, old, size);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Screen Item Name Edit") {
                    // Rename a Screen
                    let mut old = String::new();
                    if let Some(screen) = project.screens.get(&id.uuid) {
                        old = screen.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameScreen(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Asset Item Name Edit") {
                    // Rename an Asset
                    let mut old = String::new();
                    if let Some(asset) = project.assets.get(&id.uuid) {
                        old = asset.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameAsset(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Avatar Item Name Edit") {
                    // Rename an Avatar
                    let mut old = String::new();
                    if let Some(avatar) = project.avatars.get(&id.uuid) {
                        old = avatar.name.clone();
                    }

                    if let Some(name) = value.to_string()
                        && old != name
                    {
                        let atom = ProjectUndoAtom::RenameAvatar(id.uuid, old, name);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name.starts_with("Avatar Item Resolution Edit") {
                    // Change Avatar Resolution
                    if let Some(new_res) = value.to_i32() {
                        let new_res = new_res.max(1) as u16;
                        if let Some(avatar) = project.avatars.get(&id.references) {
                            let old_res = avatar.resolution;
                            if old_res != new_res {
                                let atom = ProjectUndoAtom::EditAvatarResolution(
                                    id.references,
                                    old_res,
                                    new_res,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Avatar Animation Name Edit" {
                    let anim_id = id.references;
                    if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                        let avatar_id = avatar.id;
                        let old = avatar
                            .animations
                            .iter()
                            .find(|a| a.id == anim_id)
                            .map(|a| a.name.clone())
                            .unwrap_or_default();
                        if let Some(name) = value.to_string() {
                            if old != name {
                                let atom = ProjectUndoAtom::RenameAvatarAnimation(
                                    avatar_id, anim_id, old, name,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Avatar Animation Frame Count Edit" {
                    let anim_id = id.references;
                    if let Some(new_count) = value.to_i32() {
                        let new_count = (new_count.max(1)) as usize;
                        if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                            let avatar_id = avatar.id;
                            let old_count = avatar.get_animation_frame_count(&anim_id);
                            if old_count != new_count {
                                let atom = ProjectUndoAtom::EditAvatarAnimationFrameCount(
                                    avatar_id, anim_id, old_count, new_count,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Avatar Animation Speed Edit" {
                    let anim_id = id.references;
                    if let Some(new_speed) = value.to_f32() {
                        let new_speed = new_speed.clamp(0.01, 100.0);
                        if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                            let avatar_id = avatar.id;
                            let old_speed = avatar
                                .animations
                                .iter()
                                .find(|a| a.id == anim_id)
                                .map(|a| a.speed)
                                .unwrap_or(1.0);
                            if (old_speed - new_speed).abs() > f32::EPSILON {
                                let atom = ProjectUndoAtom::EditAvatarAnimationSpeed(
                                    avatar_id, anim_id, old_speed, new_speed,
                                );
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if let Some(action_id) = server_ctx.curr_action_id
                    && id.name.starts_with("action")
                {
                    if let Some(action) =
                        ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        if action.handle_event(event, project, ui, ctx, server_ctx) {
                            if server_ctx.auto_action {
                                ctx.ui.send(TheEvent::StateChanged(
                                    TheId::named("Action Apply"),
                                    TheWidgetState::Clicked,
                                ));
                            }
                            return true;
                        }
                    }
                }

                if id.name == "RegionConfigEdit" {
                    if let Some(code) = value.to_string() {
                        if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                            apply_region_config(&mut region.map, code.clone());
                            region.config = code;
                        }
                    }
                }
                if id.name == "Palette Hex Edit" {
                    if let Some(hex) = value.to_string() {
                        let color = TheColor::from_hex(&hex);

                        if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                            if project.art_palette[palette_picker.index()] != Some(color.clone()) {
                                let prev = project.art_palette.clone();
                                let prev_materials = project.art_palette_materials.clone();

                                palette_picker.set_color(color.clone());
                                redraw = true;
                                project.art_palette[palette_picker.index()] = Some(color.clone());
                                let undo = ProjectUndoAtom::PaletteEdit(
                                    prev,
                                    prev_materials,
                                    project.art_palette.clone(),
                                    project.art_palette_materials.clone(),
                                );
                                UNDOMANAGER.write().unwrap().add_undo(undo, ctx);

                                apply_palette(ui, ctx, server_ctx, project);
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Soft Update Minimap"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }
                    crate::undo::project_helper::refresh_palette_runtime(project);
                } else if id.name == "Tilemap Filter Edit" || id.name == "Tilemap Filter Role" {
                    if let Some(id) = self.curr_tilemap_uuid {
                        self.show_filtered_tiles(ui, ctx, project.get_tilemap(id).as_deref())
                    }
                } else if id.name == "Tilemap Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(layout) = ui.get_rgba_layout("Tilemap Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                        if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                            if let Some(tilemap) = project.get_tilemap_mut(curr_tilemap_uuid) {
                                tilemap.zoom = v;
                            }
                        }
                    }
                } else if id.name == "Region Content Filter Edit"
                    || id.name == "Region Content Dropdown"
                {
                    self.apply_region(ui, ctx, Some(server_ctx.curr_region), project);
                } else if let Some(collection_id) = self.curr_tile_collection_uuid
                    && let Some(collection) = project.tile_collections.get_mut(&collection_id)
                {
                    let mut changed = false;
                    if id.name == "tileCollectionName" {
                        if let Some(text) = value.to_string() {
                            collection.name = text;
                            changed = true;
                        }
                    } else if id.name == "tileCollectionAuthor" {
                        if let Some(text) = value.to_string() {
                            collection.author = text;
                            changed = true;
                        }
                    } else if id.name == "tileCollectionVersion" {
                        if let Some(text) = value.to_string() {
                            collection.version = text;
                            changed = true;
                        }
                    } else if id.name == "tileCollectionDescription" {
                        if let Some(text) = value.to_string() {
                            collection.description = text;
                            changed = true;
                        }
                    }
                    if changed {
                        self.show_collection_settings(ui, ctx, project, collection_id);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tilepicker"),
                            TheValue::Empty,
                        ));
                        redraw = true;
                    }
                }
            }
            TheEvent::FileRequesterResult(id, paths) => {
                if let Some(action_id) = server_ctx.curr_action_id
                    && id.name.starts_with("action")
                {
                    if let Some(action) =
                        ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        if action.handle_event(event, project, ui, ctx, server_ctx) {
                            return true;
                        }
                    }
                } else if id.name == "Tilemap Add"
                    || id.name == "Add Tileset"
                    || id.name == "Add Image"
                    || id.name == "Avatar Atlas Import"
                {
                    for p in paths {
                        ctx.ui.decode_image(id.clone(), p.clone());
                    }
                } else if id.name == "Add Font Asset" || id.name == "Add Font" {
                    for p in paths {
                        if let Ok(bytes) = std::fs::read(p) {
                            if fontdue::Font::from_bytes(
                                bytes.clone(),
                                fontdue::FontSettings::default(),
                            )
                            .is_ok()
                            {
                                let asset = Asset {
                                    name: p
                                        .file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    id: Uuid::new_v4(),
                                    buffer: AssetBuffer::Font(bytes),
                                };

                                let atom = ProjectUndoAtom::AddAsset(asset);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Add Audio Asset" {
                    for p in paths {
                        if let Ok(bytes) = std::fs::read(&p) {
                            let ext = p
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|s| s.to_ascii_lowercase())
                                .unwrap_or_default();
                            if ext == "wav" || ext == "ogg" {
                                let asset = Asset {
                                    name: p
                                        .file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    id: Uuid::new_v4(),
                                    buffer: AssetBuffer::Audio(bytes),
                                };

                                let atom = ProjectUndoAtom::AddAsset(asset);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Add Font Old" {
                    for p in paths {
                        if let Ok(bytes) = std::fs::read(p) {
                            if fontdue::Font::from_bytes(
                                bytes.clone(),
                                fontdue::FontSettings::default(),
                            )
                            .is_ok()
                            {
                                let asset = Asset {
                                    name: if let Some(n) = p.file_stem() {
                                        n.to_string_lossy().to_string()
                                    } else {
                                        "Font".to_string()
                                    },
                                    buffer: AssetBuffer::Font(bytes),
                                    ..Asset::default()
                                };

                                if let Some(layout) =
                                    ui.canvas.get_layout(Some(&"Asset List".to_string()), None)
                                {
                                    if let Some(list_layout) = layout.as_list_layout() {
                                        let mut item = TheListItem::new(TheId::named_with_id(
                                            "Asset Item",
                                            asset.id,
                                        ));
                                        item.set_text(asset.name.clone());
                                        item.set_state(TheWidgetState::Selected);
                                        item.set_context_menu(Some(TheContextMenu {
                                            items: vec![TheContextMenuItem::new(
                                                "Rename Asset...".to_string(),
                                                TheId::named("Rename Asset"),
                                            )],
                                            ..Default::default()
                                        }));
                                        item.add_value_column(
                                            100,
                                            TheValue::Text("Font".to_string()),
                                        );
                                        list_layout.deselect_all();
                                        let id = item.id().clone();
                                        list_layout.add_item(item, ctx);
                                        ctx.ui.send_widget_state_changed(
                                            &id,
                                            TheWidgetState::Selected,
                                        );

                                        redraw = true;
                                    }
                                }
                                project.add_asset(asset);
                            }
                        }
                    }
                } else if id.name == "Region Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut region: Region =
                            serde_json::from_str(&contents).unwrap_or(Region::default());

                        region.id = Uuid::new_v4();
                        region.map.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddRegion(region);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Region Export" {
                    if let Some(region) = project.get_region(&id.uuid) {
                        let mut region = region.clone();
                        for p in paths {
                            region.id = Uuid::new_v4();
                            region.map.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&region) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Region saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Region!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Character Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut character: Character =
                            serde_json::from_str(&contents).unwrap_or(Character::default());

                        character.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddCharacter(character);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Avatar Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut avatar: Avatar =
                            serde_json::from_str(&contents).unwrap_or(Avatar::default());

                        avatar.id = Uuid::new_v4();
                        for animation in &mut avatar.animations {
                            animation.id = Uuid::new_v4();
                        }

                        let atom = ProjectUndoAtom::AddAvatar(avatar);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Character Export" {
                    if let Some(character) = project.characters.get(&id.uuid) {
                        let mut character = character.clone();
                        for p in paths {
                            character.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&character) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Character saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Character!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Avatar Atlas Export" {
                    if let Some(avatar) = project.avatars.get(&id.uuid) {
                        for p in paths {
                            let p = if p.extension().is_none() {
                                p.with_extension("png")
                            } else {
                                p.to_path_buf()
                            };
                            match crate::avatar_atlas::export_avatar_atlas(avatar).and_then(
                                |buffer| {
                                    buffer.to_png().map_err(|err| err.to_string()).and_then(
                                        |bytes| {
                                            std::fs::write(&p, bytes)
                                                .map_err(|err| format!("{}: {}", p.display(), err))
                                        },
                                    )
                                },
                            ) {
                                Ok(()) => ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_avatar_atlas_saved"),
                                )),
                                Err(err) => ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_avatar_atlas_save_failed", error = err),
                                )),
                            }
                        }
                    }
                } else if id.name == "Avatar Export" {
                    if let Some(avatar) = project.avatars.get(&id.uuid) {
                        let mut avatar = avatar.clone();
                        for p in paths {
                            avatar.id = Uuid::new_v4();
                            for animation in &mut avatar.animations {
                                animation.id = Uuid::new_v4();
                            }

                            if let Ok(json) = serde_json::to_string(&avatar) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Avatar saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Avatar!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Item Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut item: Item =
                            serde_json::from_str(&contents).unwrap_or(Item::default());

                        item.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddItem(item);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Item Export" {
                    if let Some(item) = project.items.get(&id.uuid) {
                        let mut item = item.clone();
                        for p in paths {
                            item.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&item) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Item saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Item!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Tileset Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut tilemap: Tilemap =
                            serde_json::from_str(&contents).unwrap_or(Tilemap::default());

                        tilemap.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddTilemap(tilemap);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Tileset Export" {
                    if let Some(tilemap) = project.get_tilemap(id.uuid) {
                        let mut tilemap = tilemap.clone();
                        for p in paths {
                            tilemap.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&tilemap) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Tileset saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Tileset!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Screen Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut screen: Screen =
                            serde_json::from_str(&contents).unwrap_or(Screen::default());

                        screen.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddScreen(screen);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Screen Export" {
                    if let Some(screen) = project.screens.get(&id.uuid) {
                        let mut screen = screen.clone();
                        for p in paths {
                            screen.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&screen) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Screen saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Screen!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Font Asset Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut asset: Asset =
                            serde_json::from_str(&contents).unwrap_or(Asset::default());

                        asset.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddAsset(asset);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Audio Asset Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut asset: Asset =
                            serde_json::from_str(&contents).unwrap_or(Asset::default());

                        asset.id = Uuid::new_v4();

                        let atom = ProjectUndoAtom::AddAsset(asset);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                } else if id.name == "Font Asset Export" {
                    if let Some(asset) = project.assets.get(&id.uuid) {
                        let mut asset = asset.clone();
                        for p in paths {
                            asset.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&asset) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Font Asset saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Font Asset!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Audio Asset Export" {
                    if let Some(asset) = project.assets.get(&id.uuid) {
                        let mut asset = asset.clone();
                        for p in paths {
                            asset.id = Uuid::new_v4();
                            if let Ok(json) = serde_json::to_string(&asset) {
                                if std::fs::write(p, json).is_ok() {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Audio Asset saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save Audio Asset!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                } else if id.name == "Procedural Recipe Import" {
                    for p in paths {
                        let Ok(source) = std::fs::read_to_string(p) else {
                            continue;
                        };
                        if let Err(error) = procedural_recipes::parse_document(&source) {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                format!("{}: {error}", fl!("invalid_recipe")),
                            ));
                            continue;
                        }
                        let requested = p
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let alias = crate::recipe_utils::unique_recipe_alias(project, &requested);
                        let asset = ProceduralRecipeAsset::new(alias, source);
                        let atom = ProjectUndoAtom::AddProceduralRecipe(asset);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        redraw = true;
                    }
                } else if id.name == "Procedural Recipe Export" {
                    if let Some(recipe) = project.procedural_recipes.get(&id.uuid) {
                        for p in paths {
                            let p = if p.extension().is_none() {
                                p.with_extension("recipe")
                            } else {
                                p.to_path_buf()
                            };
                            let status = if std::fs::write(&p, &recipe.source).is_ok() {
                                fl!("recipe_saved")
                            } else {
                                fl!("recipe_save_failed")
                            };
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                        }
                    }
                }
            }
            TheEvent::ImageDecodeResult(id, name, buffer) => {
                if id.name == "Add Image" {
                    let asset = Asset {
                        name: name.clone(),
                        id: Uuid::new_v4(),
                        buffer: AssetBuffer::Image(buffer.clone()),
                    };

                    if let Some(layout) =
                        ui.canvas.get_layout(Some(&"Asset List".to_string()), None)
                    {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item =
                                TheListItem::new(TheId::named_with_id("Asset Item", asset.id));
                            item.set_text(name.clone());
                            item.set_state(TheWidgetState::Selected);
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Rename Asset...".to_string(),
                                    TheId::named("Rename Asset"),
                                )],
                                ..Default::default()
                            }));
                            item.add_value_column(100, TheValue::Text("Image".to_string()));
                            list_layout.deselect_all();
                            let id = item.id().clone();
                            list_layout.add_item(item, ctx);
                            ctx.ui
                                .send_widget_state_changed(&id, TheWidgetState::Selected);

                            redraw = true;
                        }
                    }
                    project.add_asset(asset);
                } else if id.name == "Tilemap Add" || id.name == "Add Tileset" {
                    let mut tilemap = Tilemap::new();
                    tilemap.name = name.clone();
                    tilemap.id = Uuid::new_v4();
                    tilemap.buffer = buffer.clone();

                    // Use undo system to add tilemap
                    let atom = ProjectUndoAtom::AddTilemap(tilemap);
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);

                    redraw = true;
                } else if id.name == "Avatar Atlas Import" {
                    if let Some(before) = project.avatars.get(&id.uuid).cloned() {
                        let mut after = before.clone();
                        match crate::avatar_atlas::import_avatar_atlas(&mut after, buffer) {
                            Ok(frame_count) => {
                                let atom = ProjectUndoAtom::EditAvatar(id.uuid, before, after);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!(
                                        "status_avatar_atlas_imported",
                                        count = frame_count.to_string()
                                    ),
                                ));
                                redraw = true;
                            }
                            Err(err) => {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_avatar_atlas_import_failed", error = err),
                                ));
                            }
                        }
                    }
                }
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                if server_ctx.game_mode || server_ctx.game_input_mode || server_ctx.text_game_mode {
                    return false;
                }

                if DockManager::edit_maximize_accelerator()
                    .matches(ui.shift, ui.ctrl, ui.alt, ui.logo, *c)
                {
                    let can_maximize = {
                        let dock_manager = DOCKMANAGER.read().unwrap();
                        dock_manager.get_state() == DockManagerState::Minimized
                            && !dock_manager.dock.is_empty()
                    };
                    if can_maximize {
                        DOCKMANAGER
                            .write()
                            .unwrap()
                            .edit_maximize(ui, ctx, project, server_ctx);
                        return true;
                    }
                } else if DockManager::restore_accelerator()
                    .matches(ui.shift, ui.ctrl, ui.alt, ui.logo, *c)
                    && DOCKMANAGER.read().unwrap().get_state() != DockManagerState::Minimized
                {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .minimize(ui, ctx, project, server_ctx);
                    return true;
                }

                let action_list = ACTIONLIST.write().unwrap();
                let mut needs_scene_redraw: bool = false;
                let mut action_applied = false;
                for action in &action_list.actions {
                    if let Some(accel) = action.accel() {
                        if accel.matches(ui.shift, ui.ctrl, ui.alt, ui.logo, *c) {
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                if action.is_applicable(map, ctx, server_ctx) {
                                    println!("{}", action.id().name);
                                    needs_scene_redraw =
                                        self.apply_action(action, map, ui, ctx, server_ctx, true);
                                    action_applied = true;
                                }
                            }
                            action.apply_project(project, ui, ctx, server_ctx);
                        }
                    }
                }
                if needs_scene_redraw {
                    crate::utils::editor_scene_full_rebuild(project, server_ctx);
                    TOOLLIST
                        .write()
                        .unwrap()
                        .update_geometry_overlay_3d(project, server_ctx);
                }
                if action_applied {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Action List"),
                        TheValue::Empty,
                    ));
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                let navigation_input_focused = ctx.ui.focus.as_ref().is_some_and(|id| {
                    id.name == "Console Input" || id.name == "LogEdit" || id.name == "Help Input"
                });
                if *code == TheKeyCode::Tab
                    && !ui.ctrl
                    && !ui.alt
                    && !ui.logo
                    && (!ui.focus_widget_supports_text_input(ctx) || navigation_input_focused)
                {
                    let reverse = ui.shift;
                    let current = ui
                        .get_stack_layout("Sidebar Page Stack")
                        .map(|stack| stack.index());
                    if let Some(current) = current {
                        let next = Self::next_navigation_page(current, reverse);
                        redraw |= self.activate_navigation_page(next, ui, ctx, project, server_ctx);
                    }
                } else if *code == TheKeyCode::Delete
                    && let Some(focus_id) = &ctx.ui.focus
                    && (focus_id.name == "Palette Picker" || focus_id.name == "Palette Item")
                {
                    let index = project.art_palette.current_index as usize;
                    if index < project.art_palette.colors.len()
                        && project.art_palette[index].is_some()
                    {
                        let prev = project.art_palette.clone();
                        let prev_materials = project.art_palette_materials.clone();
                        project.art_palette[index] = None;
                        project.reset_art_palette_material(index);

                        let undo = ProjectUndoAtom::PaletteEdit(
                            prev,
                            prev_materials,
                            project.art_palette.clone(),
                            project.art_palette_materials.clone(),
                        );
                        UNDOMANAGER.write().unwrap().add_undo(undo, ctx);

                        apply_palette(ui, ctx, server_ctx, project);

                        if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                            palette_picker.set_palette(project.art_palette.clone());
                        }
                        if let Some(widget) = ui.get_widget("Palette Color Picker")
                            && let Some(color) = project.art_palette[index].clone()
                        {
                            widget.set_value(TheValue::ColorObject(color));
                        }
                        if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                            if let Some(color) = project.art_palette[index].clone() {
                                widget.set_value(TheValue::Text(color.to_hex()));
                            } else {
                                widget.set_value(TheValue::Text(String::new()));
                            }
                        }

                        crate::undo::project_helper::refresh_palette_runtime(project);

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Soft Update Minimap"),
                            TheValue::Empty,
                        ));

                        redraw = true;
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Action Auto" {
                    server_ctx.auto_action = *state == TheWidgetState::Selected;
                } else if id.name == "Procedural Recipe Item" && *state == TheWidgetState::Selected
                {
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::ProceduralRecipe(id.uuid),
                    );
                    redraw = true;
                } else
                // Iterate actions
                if let Some((accelerator, params, title, auto_apply)) = {
                    // Loading an action mutates its cached parameters, but UI
                    // updates must happen after releasing the global action
                    // list lock. Parameter editors can synchronously generate
                    // more UI work that also consults the action catalog.
                    let mut actions = ACTIONLIST.write().unwrap();
                    actions.get_action_by_id_mut(id.uuid).map(|action| {
                        server_ctx.curr_action_id = Some(action.id().uuid);

                        if let Some(map) = project.get_map_mut(&server_ctx) {
                            action.load_params(map);
                        }
                        action.load_params_project(project, server_ctx);

                        (
                            action.accel(),
                            action.params(),
                            action.id().name,
                            server_ctx.auto_action || action.role() == ActionRole::Camera,
                        )
                    })
                } {
                    self.show_action_toml_snapshot(ui, ctx, accelerator, &params, title);

                    if auto_apply {
                        ctx.ui.send(TheEvent::StateChanged(
                            TheId::named("Action Apply"),
                            TheWidgetState::None,
                        ));
                    }
                } else if id.name == "Action Apply" || id.name == "Project Action Apply" {
                    if let Some(action_id) = server_ctx.curr_action_id {
                        self.sync_current_action_toml_params(ui, ctx, Some(project), server_ctx);
                        if let Some(action) = ACTIONLIST.read().unwrap().get_action_by_id(action_id)
                        {
                            let mut needs_scene_redraw = false;
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                needs_scene_redraw = self.apply_action(
                                    action,
                                    map,
                                    ui,
                                    ctx,
                                    server_ctx,
                                    !(*state == TheWidgetState::None),
                                );
                            }
                            action.apply_project(project, ui, ctx, server_ctx);

                            if needs_scene_redraw {
                                crate::utils::editor_scene_full_rebuild(project, server_ctx);
                                TOOLLIST
                                    .write()
                                    .unwrap()
                                    .update_geometry_overlay_3d(project, server_ctx);
                            }
                            // Keep applicability in sync after every apply.
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Action List"),
                                TheValue::Empty,
                            ));
                        }
                    }
                } else if id.name == "Shader Item" {
                    let material_id = id.uuid;
                    server_ctx.curr_material_id = Some(material_id);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimap"),
                        TheValue::Empty,
                    ));
                } else if id.name == "Palette Clear" {
                    let prev = project.art_palette.clone();
                    let prev_materials = project.art_palette_materials.clone();
                    project.art_palette.clear();
                    project.reset_all_art_palette_materials();
                    if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                        let index = palette_picker.index();

                        palette_picker.set_palette(project.art_palette.clone());
                        if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                            if let Some(color) = &project.art_palette[index] {
                                widget.set_value(TheValue::Text(color.to_hex()));
                            }
                        }
                    }
                    redraw = true;

                    let undo = ProjectUndoAtom::PaletteEdit(
                        prev,
                        prev_materials,
                        project.art_palette.clone(),
                        project.art_palette_materials.clone(),
                    );
                    UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                } else if id.name == "Palette Import" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "Palette (*.txt, *.hex)".into(),
                            vec![
                                "txt".to_string(),
                                "TXT".to_string(),
                                "hex".to_string(),
                                "HEX".to_string(),
                            ],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Tilemap Import" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "Eldiron Tilemap".into(),
                            vec!["eldiron_tilemap".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Tilemap Export" {
                    if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                        if let Some(tilemap) = project.get_tilemap(curr_tilemap_uuid) {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id(id.name.as_str(), tilemap.id),
                                "Save".into(),
                                TheFileExtension::new(
                                    "Eldiron Tilemap".into(),
                                    vec!["eldiron_tilemap".to_string()],
                                ),
                            );
                            ctx.ui
                                .set_widget_state("Save As".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }
                    }
                } else if id.name == "Add Region" {
                    // Add Region
                    let mut region = Region::default();
                    if let Some(bytes) = crate::Embedded::get("toml/region.toml") {
                        if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                            region.config = source.to_string();
                        }
                    }
                    let atom = ProjectUndoAtom::AddRegion(region);
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Import Region" {
                    if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_region() {
                            ctx.ui.open_file_requester(
                                TheId::named_with_id("Region Import", id),
                                "Import Region".into(),
                                TheFileExtension::new(
                                    "Eldiron Region".into(),
                                    vec!["eldiron_region".to_string()],
                                ),
                            );
                        }
                    }
                } else if id.name == "Add Character" {
                    // Add Character
                    let atom = ProjectUndoAtom::AddCharacter(Character::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Import Character" {
                    if let Some(id) = server_ctx.pc.id() {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id("Character Import", id),
                            "Import Character".into(),
                            TheFileExtension::new(
                                "Eldiron Character".into(),
                                vec!["eldiron_character".to_string()],
                            ),
                        );
                    }
                } else if id.name == "Import Avatar" {
                    if let Some(id) = server_ctx.pc.id() {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id("Avatar Import", id),
                            fl!("import_avatar"),
                            TheFileExtension::new(
                                fl!("eldiron_avatar"),
                                vec!["eldiron_avatar".to_string()],
                            ),
                        );
                    }
                } else if id.name == "Import Avatar Atlas" {
                    if let Some(id) = server_ctx.pc.id()
                        && matches!(
                            server_ctx.pc,
                            ProjectContext::Avatar(_) | ProjectContext::AvatarAnimation(_, _, _)
                        )
                    {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id("Avatar Atlas Import", id),
                            fl!("import_avatar_atlas"),
                            TheFileExtension::new(
                                fl!("png_image"),
                                vec!["png".to_string(), "PNG".to_string()],
                            ),
                        );
                    }
                } else if id.name == "Add Item" {
                    // Add Item
                    let atom = ProjectUndoAtom::AddItem(Item::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Add Tileset" {
                    // Add Tileset - open PNG file requester
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Add Tileset", Uuid::new_v4()),
                        "Open PNG Image".into(),
                        TheFileExtension::new(
                            "PNG Image".into(),
                            vec!["png".to_string(), "PNG".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("Add Tileset".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Add Screen" {
                    // Add Screen
                    let atom = ProjectUndoAtom::AddScreen(Screen::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Add Font Asset" {
                    // Add Font Asset - open font file requester
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Add Font Asset", Uuid::new_v4()),
                        "Open Font File".into(),
                        TheFileExtension::new(
                            "Font File".into(),
                            vec!["ttf".to_string(), "otf".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("Add Font Asset".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Add Audio Asset" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Add Audio Asset", Uuid::new_v4()),
                        "Open Audio File".into(),
                        TheFileExtension::new(
                            "Audio File".into(),
                            vec!["wav".to_string(), "ogg".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("Add Audio Asset".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Add Avatar" {
                    // Add Avatar
                    let atom = ProjectUndoAtom::AddAvatar(Avatar::default());
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                } else if id.name == "Add Procedural Recipe" {
                    let mut recipe = ProceduralRecipeAsset::default();
                    recipe.alias = crate::recipe_utils::unique_recipe_alias(project, &recipe.alias);
                    let atom = ProjectUndoAtom::AddProceduralRecipe(recipe);
                    atom.redo(project, ui, ctx, server_ctx);
                    UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    redraw = true;
                } else if id.name == "Import Item" {
                    if let Some(id) = server_ctx.pc.id() {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id("Item Import", id),
                            "Import Item".into(),
                            TheFileExtension::new(
                                "Eldiron Item".into(),
                                vec!["eldiron_item".to_string()],
                            ),
                        );
                    }
                } else if id.name == "Import Tileset" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Tileset Import", Uuid::new_v4()),
                        "Import Tileset".into(),
                        TheFileExtension::new(
                            "Eldiron Tileset".into(),
                            vec!["eldiron_tileset".to_string()],
                        ),
                    );
                } else if id.name == "Import Screen" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Screen Import", Uuid::new_v4()),
                        "Import Screen".into(),
                        TheFileExtension::new(
                            "Eldiron Screen".into(),
                            vec!["eldiron_screen".to_string()],
                        ),
                    );
                } else if id.name == "Import Font Asset" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Font Asset Import", Uuid::new_v4()),
                        "Import Font Asset".into(),
                        TheFileExtension::new(
                            "Eldiron Font Asset".into(),
                            vec!["eldiron_font_asset".to_string()],
                        ),
                    );
                } else if id.name == "Import Audio Asset" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Audio Asset Import", Uuid::new_v4()),
                        "Import Audio Asset".into(),
                        TheFileExtension::new(
                            "Eldiron Audio Asset".into(),
                            vec!["eldiron_audio_asset".to_string()],
                        ),
                    );
                } else if id.name == "Import Procedural Recipe" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id("Procedural Recipe Import", Uuid::new_v4()),
                        fl!("import_recipe"),
                        TheFileExtension::new(fl!("recipe_file"), vec!["recipe".to_string()]),
                    );
                } else if id.name == "Project Remove" {
                    if server_ctx.pc.is_region() {
                        if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id()
                        {
                            // This is a character instance in the region

                            let mut character = Character::default();
                            let mut index = 0;

                            if let Some(r) = project.get_region_ctx(server_ctx) {
                                if let Some(ind) = r.characters.get_index_of(&instance_id) {
                                    index = ind;
                                }
                                if let Some(char) = r.characters.get(&instance_id) {
                                    character = char.clone();
                                }
                            }

                            let atom = ProjectUndoAtom::RemoveRegionCharacterInstance(
                                index,
                                server_ctx.curr_region,
                                character,
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        } else if let Some(instance_id) =
                            server_ctx.pc.get_region_item_instance_id()
                        {
                            // This is a item instance in the region

                            let mut item = Item::default();
                            let mut index = 0;

                            if let Some(r) = project.get_region_ctx(server_ctx) {
                                if let Some(ind) = r.items.get_index_of(&instance_id) {
                                    index = ind;
                                }
                                if let Some(it) = r.items.get(&instance_id) {
                                    item = it.clone();
                                }
                            }

                            let atom = ProjectUndoAtom::RemoveRegionItemInstance(
                                index,
                                server_ctx.curr_region,
                                item,
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        } else {
                            // Remove Region
                            let mut region = Region::default();
                            if let Some(r) = project.get_region_ctx(server_ctx) {
                                region = r.clone();
                            }

                            if let Some(index) =
                                project.regions.iter().position(|r| r.id == region.id)
                            {
                                let atom = ProjectUndoAtom::RemoveRegion(index, region);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_character() {
                        // Remove Character
                        let mut character: Character = Character::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(c) = project.characters.get(&id) {
                                character = c.clone();
                            }

                            if let Some(index) = project.characters.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveCharacter(index, character);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if let ProjectContext::AvatarAnimation(avatar_id, anim_id, _) =
                        server_ctx.pc
                    {
                        // Remove Avatar Animation
                        if let Some(avatar) = project.avatars.get(&avatar_id)
                            && let Some(index) =
                                avatar.animations.iter().position(|anim| anim.id == anim_id)
                            && let Some(animation) = avatar.animations.get(index)
                        {
                            let atom = ProjectUndoAtom::RemoveAvatarAnimation(
                                avatar_id,
                                index,
                                animation.clone(),
                            );
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        }
                    } else if let ProjectContext::Avatar(avatar_id) = server_ctx.pc {
                        // Remove Avatar
                        if let Some(avatar) = project.avatars.get(&avatar_id).cloned()
                            && let Some(index) = project.avatars.get_index_of(&avatar_id)
                        {
                            let atom = ProjectUndoAtom::RemoveAvatar(index, avatar);
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        }
                    } else if let ProjectContext::ProceduralRecipe(recipe_id) = server_ctx.pc {
                        if let Some(index) = project.procedural_recipes.get_index_of(&recipe_id)
                            && let Some(recipe) =
                                project.procedural_recipes.get(&recipe_id).cloned()
                        {
                            let atom = ProjectUndoAtom::RemoveProceduralRecipe(index, recipe);
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_item() {
                        // Remove Item
                        let mut item: Item = Item::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(c) = project.items.get(&id) {
                                item = c.clone();
                            }

                            if let Some(index) = project.items.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveItem(index, item);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_tilemap() {
                        // Remove Tilemap
                        let mut tilemap: Tilemap = Tilemap::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(t) = project.get_tilemap(id) {
                                tilemap = t.clone();
                            }

                            if let Some(index) =
                                project.tilemaps.iter().position(|r| r.id == tilemap.id)
                            {
                                let atom = ProjectUndoAtom::RemoveTilemap(index, tilemap);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_screen() {
                        // Remove Screen
                        let mut screen: Screen = Screen::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(s) = project.screens.get(&id) {
                                screen = s.clone();
                            }

                            if let Some(index) = project.screens.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveScreen(index, screen);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    } else if server_ctx.pc.is_asset() {
                        // Remove Asset
                        let mut asset: Asset = Asset::default();
                        if let Some(id) = server_ctx.pc.id() {
                            if let Some(a) = project.assets.get(&id) {
                                asset = a.clone();
                            }

                            if let Some(index) = project.assets.get_index_of(&id) {
                                let atom = ProjectUndoAtom::RemoveAsset(index, asset);
                                atom.redo(project, ui, ctx, server_ctx);
                                UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            }
                        }
                    }
                } else if id.name == "Project Duplicate" {
                    if let ProjectContext::ProceduralRecipe(recipe_id) = server_ctx.pc {
                        if let Some(mut duplicated) =
                            project.procedural_recipes.get(&recipe_id).cloned()
                        {
                            duplicated.id = Uuid::new_v4();
                            duplicated.alias = crate::recipe_utils::unique_recipe_alias(
                                project,
                                &format!("{}-copy", duplicated.alias),
                            );
                            duplicated.source =
                                crate::recipe_utils::duplicate_recipe_source(&duplicated.source);
                            duplicated.tile_id = None;
                            let atom = ProjectUndoAtom::AddProceduralRecipe(duplicated);
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_region() {
                        if let Some(region) = project.get_region_ctx(server_ctx).cloned() {
                            let mut duplicated = region;
                            duplicated.id = Uuid::new_v4();
                            duplicated.name = format!("{} Copy", duplicated.name);
                            duplicated.map.id = Uuid::new_v4();
                            duplicated.map.name = duplicated.name.clone();
                            let insert_index = project.regions.len();
                            project.regions.push(duplicated.clone());
                            if let Some(tree_layout) = ui.get_tree_layout("Project Tree")
                                && let Some(region_node) =
                                    tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                            {
                                let mut node = gen_region_tree_node(&duplicated);
                                node.set_open(true);
                                region_node.add_child(node);
                            }
                            server_ctx.curr_region = duplicated.id;
                            set_project_context(
                                ctx,
                                ui,
                                project,
                                server_ctx,
                                ProjectContext::Region(duplicated.id),
                            );
                            update_region(ctx);
                            let undo = ProjectUndoAtom::RemoveRegion(insert_index, duplicated);
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_character() {
                        if let Some(id) = server_ctx.pc.id()
                            && let Some(character) = project.characters.get(&id).cloned()
                        {
                            let mut duplicated = character;
                            duplicated.id = Uuid::new_v4();
                            duplicated.name = format!("{} Copy", duplicated.name);
                            duplicated.map.id = Uuid::new_v4();
                            duplicated.map.name = duplicated.name.clone();
                            duplicated.character_id = Uuid::new_v4();
                            let insert_index = project.characters.len();
                            project.add_character(duplicated.clone());
                            if let Some(tree_layout) = ui.get_tree_layout("Project Tree")
                                && let Some(characters_node) =
                                    tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                            {
                                let mut node = gen_character_tree_node(&duplicated);
                                node.set_open(true);
                                characters_node.add_child(node);
                            }
                            set_project_context(
                                ctx,
                                ui,
                                project,
                                server_ctx,
                                ProjectContext::Character(duplicated.id),
                            );
                            update_region(ctx);
                            let undo =
                                ProjectUndoAtom::RemoveCharacter(insert_index, duplicated.clone());
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_item() {
                        if let Some(id) = server_ctx.pc.id()
                            && let Some(item) = project.items.get(&id).cloned()
                        {
                            let mut duplicated = item;
                            duplicated.id = Uuid::new_v4();
                            duplicated.name = format!("{} Copy", duplicated.name);
                            duplicated.map.id = Uuid::new_v4();
                            duplicated.map.name = duplicated.name.clone();
                            duplicated.item_id = Uuid::new_v4();
                            let insert_index = project.items.len();
                            project.add_item(duplicated.clone());
                            if let Some(tree_layout) = ui.get_tree_layout("Project Tree")
                                && let Some(items_node) =
                                    tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id)
                            {
                                let mut node = gen_item_tree_node(&duplicated, project);
                                node.set_open(true);
                                items_node.add_child(node);
                            }
                            set_project_context(
                                ctx,
                                ui,
                                project,
                                server_ctx,
                                ProjectContext::Item(duplicated.id),
                            );
                            update_region(ctx);
                            let undo = ProjectUndoAtom::RemoveItem(insert_index, duplicated);
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_screen() {
                        if let Some(id) = server_ctx.pc.id()
                            && let Some(screen) = project.screens.get(&id).cloned()
                        {
                            let mut duplicated = screen;
                            duplicated.id = Uuid::new_v4();
                            duplicated.name = format!("{} Copy", duplicated.name);
                            duplicated.map.id = Uuid::new_v4();
                            duplicated.map.name = duplicated.name.clone();
                            let insert_index = project.screens.len();
                            project.add_screen(duplicated.clone());
                            if let Some(tree_layout) = ui.get_tree_layout("Project Tree")
                                && let Some(screens_node) =
                                    tree_layout.get_node_by_id_mut(&server_ctx.tree_screens_id)
                            {
                                let mut node = gen_screen_tree_node(&duplicated);
                                node.set_open(true);
                                screens_node.add_child(node);
                            }
                            server_ctx.curr_screen = duplicated.id;
                            set_project_context(
                                ctx,
                                ui,
                                project,
                                server_ctx,
                                ProjectContext::Screen(duplicated.id),
                            );
                            update_region(ctx);
                            let undo = ProjectUndoAtom::RemoveScreen(insert_index, duplicated);
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                            redraw = true;
                        }
                    } else {
                        let avatar_id = match server_ctx.pc {
                            ProjectContext::Avatar(id) => Some(id),
                            ProjectContext::AvatarAnimation(id, _, _) => Some(id),
                            _ => None,
                        };
                        if let Some(avatar_id) = avatar_id
                            && let Some(avatar) = project.avatars.get(&avatar_id).cloned()
                        {
                            let mut duplicated = avatar;
                            duplicated.id = Uuid::new_v4();
                            duplicated.name = format!("{} Copy", duplicated.name);
                            for animation in &mut duplicated.animations {
                                animation.id = Uuid::new_v4();
                            }
                            let insert_index = project.avatars.len();
                            project.add_avatar(duplicated.clone());
                            if let Some(tree_layout) = ui.get_tree_layout("Project Tree")
                                && let Some(avatars_node) =
                                    tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id)
                            {
                                let mut node = gen_avatar_tree_node(&duplicated);
                                node.set_open(true);
                                avatars_node.add_child(node);
                            }
                            set_project_context(
                                ctx,
                                ui,
                                project,
                                server_ctx,
                                ProjectContext::Avatar(duplicated.id),
                            );
                            update_region(ctx);
                            let undo = ProjectUndoAtom::RemoveAvatar(insert_index, duplicated);
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                            redraw = true;
                        }
                    }
                } else if id.name == "Project Export" {
                    if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_region() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Region Export", id),
                                "Export Region".into(),
                                TheFileExtension::new(
                                    "Eldiron Region".into(),
                                    vec!["eldiron_region".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_character() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Character Export", id),
                                "Export Character".into(),
                                TheFileExtension::new(
                                    "Eldiron Character".into(),
                                    vec!["eldiron_character".to_string()],
                                ),
                            );
                        } else if matches!(
                            server_ctx.pc,
                            ProjectContext::Avatar(_) | ProjectContext::AvatarAnimation(_, _, _)
                        ) {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Avatar Atlas Export", id),
                                fl!("export_avatar_atlas"),
                                TheFileExtension::new(
                                    fl!("png_image"),
                                    vec!["png".to_string(), "PNG".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_item() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Item Export", id),
                                "Export Item".into(),
                                TheFileExtension::new(
                                    "Eldiron Item".into(),
                                    vec!["eldiron_item".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_tilemap() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Tileset Export", id),
                                "Export Tileset".into(),
                                TheFileExtension::new(
                                    "Eldiron Tileset".into(),
                                    vec!["eldiron_tileset".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_screen() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Screen Export", id),
                                "Export Screen".into(),
                                TheFileExtension::new(
                                    "Eldiron Screen".into(),
                                    vec!["eldiron_screen".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_procedural_recipe() {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id("Procedural Recipe Export", id),
                                fl!("export_recipe"),
                                TheFileExtension::new(
                                    fl!("recipe_file"),
                                    vec!["recipe".to_string()],
                                ),
                            );
                        } else if server_ctx.pc.is_asset() {
                            if let Some(asset) = project.assets.get(&id) {
                                let (req_id, title, ext_label, ext) = match &asset.buffer {
                                    AssetBuffer::Audio(_) => (
                                        "Audio Asset Export",
                                        "Export Audio Asset",
                                        "Eldiron Audio Asset",
                                        "eldiron_audio_asset",
                                    ),
                                    _ => (
                                        "Font Asset Export",
                                        "Export Font Asset",
                                        "Eldiron Font Asset",
                                        "eldiron_font_asset",
                                    ),
                                };
                                ctx.ui.save_file_requester(
                                    TheId::named_with_id(req_id, id),
                                    title.into(),
                                    TheFileExtension::new(ext_label.into(), vec![ext.to_string()]),
                                );
                            }
                        }
                    }
                } else if id.name == "Region Item" {
                    server_ctx.editing_pos_buffer = None;
                    server_ctx.curr_region = id.references;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Region(id.references),
                    );
                    let _ = crate::utils::update_region_settings(project, server_ctx);
                    self.apply_region(ui, ctx, Some(id.references), project);
                    redraw = true;
                } else if id.name == "Region Settings Item" {
                    server_ctx.editing_pos_buffer = None;
                    server_ctx.curr_region = id.references;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::RegionSettings(id.references),
                    );
                    let _ = crate::utils::update_region_settings(project, server_ctx);
                    self.apply_region(ui, ctx, Some(id.references), project);
                    redraw = true;
                } else if id.name == "Region Code Item" {
                    server_ctx.editing_pos_buffer = None;
                    server_ctx.curr_region = id.references;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::RegionCode(id.references),
                    );
                    redraw = true;
                } else if id.name == "Character Item" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.uuid);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Name Edit" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Code Edit" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::CharacterCode(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Data Edit" {
                    if let Some(_) = project.characters.get(&id.references) {
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::CharacterData(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Character Item Preview Rigging Edit" {
                    if let Some(character) = project.characters.get_mut(&id.references) {
                        if character.preview_rigging.trim().is_empty() {
                            character.preview_rigging = default_preview_rigging_toml();
                        }
                        server_ctx.curr_character =
                            ContentContext::CharacterTemplate(id.references);
                        server_ctx.cc = ContentContext::CharacterTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::CharacterPreviewRigging(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Item Item" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_item = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.uuid);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Item(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Item Item Name Edit" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_item = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Item(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Tilemap Item Name Edit" {
                    if let Some(_tilemap) = project.get_tilemap(id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Tilemap Item Code Edit" || id.name == "Tilemap Item Grid Edit"
                {
                    if let Some(_tilemap) = project.get_tilemap(id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Screen Item" {
                    if let Some(_screen) = project.screens.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Screen Item Name Edit" {
                    if let Some(_screen) = project.screens.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Screen Settings Item" {
                    if project.screens.contains_key(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ScreenSettings(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Asset Item" {
                    if let Some(asset) = project.assets.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(id.references),
                        );
                        Self::preview_audio_asset(asset);
                        redraw = true;
                    }
                } else if id.name == "Asset Item Name Edit" {
                    if let Some(_screen) = project.assets.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Avatar Item"
                    || id.name == "Avatar Item Name Edit"
                    || id.name == "Avatar Item Resolution Edit"
                {
                    if let Some(_) = project.avatars.get(&id.references) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Avatar(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Procedural Recipe Item" {
                    if project.procedural_recipes.contains_key(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ProceduralRecipe(id.uuid),
                        );
                        redraw = true;
                    }
                } else if id.name == "Avatar Animation Item"
                    || id.name == "Avatar Animation Name Edit"
                    || id.name == "Avatar Animation Frame Count Edit"
                    || id.name == "Avatar Animation Speed Edit"
                {
                    let anim_id = id.references;
                    if let Some(avatar) = project.find_avatar_for_animation(&anim_id) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::AvatarAnimation(avatar.id, anim_id, 0),
                        );
                        redraw = true;
                    }
                } else if id.name == "Avatar Add Animation" {
                    if let Some(avatar) = project.avatars.get(&id.references) {
                        let resolution = avatar.resolution as usize;
                        let mut anim = AvatarAnimation::default();
                        anim.perspectives = avatar
                            .perspective_count
                            .directions()
                            .iter()
                            .map(|dir| AvatarPerspective {
                                direction: *dir,
                                frames: vec![AvatarAnimationFrame::new(Texture::new(
                                    vec![0; resolution * resolution * 4],
                                    resolution,
                                    resolution,
                                ))],
                                weapon_main_anchor: None,
                                weapon_off_anchor: None,
                            })
                            .collect();
                        let atom = ProjectUndoAtom::AddAvatarAnimation(id.references, anim);
                        atom.redo(project, ui, ctx, server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                        redraw = true;
                    }
                } else if id.name == "Item Item Code Edit" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_character = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ItemCode(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Item Item Data Edit" {
                    if let Some(_) = project.items.get(&id.references) {
                        server_ctx.curr_character = ContentContext::ItemTemplate(id.references);
                        server_ctx.cc = ContentContext::ItemTemplate(id.references);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::ItemData(id.references),
                        );
                        redraw = true;
                    }
                } else if id.name == "Project Settings" {
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::ProjectSettings,
                    );
                    redraw = true;
                } else if id.name == "World Code" {
                    set_project_context(ctx, ui, project, server_ctx, ProjectContext::WorldCode);
                    redraw = true;
                } else if id.name == "Game Rules" {
                    set_project_context(ctx, ui, project, server_ctx, ProjectContext::GameRules);
                    redraw = true;
                } else if id.name == "Game Locales" {
                    set_project_context(ctx, ui, project, server_ctx, ProjectContext::GameLocales);
                    redraw = true;
                } else if id.name == "Game Audio FX" {
                    set_project_context(ctx, ui, project, server_ctx, ProjectContext::GameAudioFx);
                    redraw = true;
                } else if id.name == "Game Authoring" {
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::GameAuthoring,
                    );
                    redraw = true;
                } else if id.name == "Game Shortcuts" {
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::GameShortcuts,
                    );
                    redraw = true;
                } else if id.name == "Tileset Item" {
                    // Display the tileset editor
                    if let Some(t) = project.get_tilemap(id.references) {
                        self.curr_tilemap_uuid = Some(t.id);
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(id.references),
                        );
                    }
                    redraw = true;
                } else if id.name == "Screen Item" {
                    if let Some(s) = project.screens.get(&id.uuid) {
                        self.apply_screen(ui, ctx, Some(s));
                        server_ctx.curr_screen = id.uuid;
                        redraw = true;
                        RUSTERIX.write().unwrap().set_dirty();
                    }
                } else if id.name == "Screen Add" {
                    if let Some(list_layout) = ui.get_list_layout("Screen List") {
                        let screen = Screen::default();

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Screen Item", screen.id));
                        item.set_text(screen.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Screen...".to_string(),
                                TheId::named("Rename Screen"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_screen(ui, ctx, Some(&screen));
                        project.add_screen(screen);
                    }
                } else if id.name == "Screen Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Screen List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_screen(&selected.uuid);
                            self.apply_screen(ui, ctx, None);
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Apply th given project to the UI
    pub fn load_from_project(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        self.reset_for_project_switch();

        _ = RUSTERIX
            .write()
            .unwrap()
            .scene_handler
            .settings
            .read(&project.config);

        // New projects carry the Endesga 64 art palette by default. Keep this
        // fallback for very old/corrupt project files.
        if project.art_palette.is_empty() {
            project.art_palette = Project::default().art_palette;
        }
        crate::recipe_utils::migrate_legacy_recipe_catalog(project);

        self.apply_regions(ui, ctx, server_ctx, project);
        self.apply_characters(ui, ctx, server_ctx, project);
        self.apply_items(ui, ctx, server_ctx, project);
        self.apply_tilemaps(ui, ctx, server_ctx, project);
        self.apply_screens(ui, ctx, server_ctx, project);
        self.apply_assets(ui, ctx, server_ctx, project);
        self.apply_recipes(ui, ctx, server_ctx, project);
        apply_palette(ui, ctx, server_ctx, project);
        self.apply_screen(ui, ctx, None);
        self.apply_avatars(ui, ctx, server_ctx, project);

        if let Some(list_layout) = ui.get_list_layout("Screen List") {
            list_layout.clear();
            let list = project.sorted_screens_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Screen Item", id));
                item.set_text(name);
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Screen...".to_string(),
                        TheId::named("Rename Screen"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }
        if let Some(list_layout) = ui.get_list_layout("Asset List") {
            list_layout.clear();
            let list = project.sorted_assets_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Asset Item", id));
                item.set_text(name);
                if let Some(asset) = project.assets.get(&id) {
                    let text = asset.buffer.clone().to_string().to_string();
                    item.add_value_column(100, TheValue::Text(text));
                }
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Asset...".to_string(),
                        TheId::named("Rename Asset"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }

        // Adjust Palette and Color Picker
        if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
            palette_picker.set_palette(project.art_palette.clone());
            let index = palette_picker.index();

            if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                if let Some(color) = &project.art_palette[index] {
                    widget.set_value(TheValue::Text(color.to_hex()));
                }
            }
        }

        // ui.select_first_list_item("Region List", ctx);
        // ui.select_first_list_item("Character List", ctx);
        // ui.select_first_list_item("Item List", ctx);
        // ui.select_first_list_item("Tilemap List", ctx);
        // ui.select_first_list_item("Module List", ctx);
        // ui.select_first_list_item("Screen List", ctx);
        // ui.select_first_list_item("Asset List", ctx);

        // ui.set_widget_value("ConfigEdit", ctx, TheValue::Text(project.config.clone()));
        if let Ok(toml) = project.config.parse::<Table>() {
            *CONFIG.write().unwrap() = toml;
        }
        CONFIGEDITOR.write().unwrap().read_defaults();
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.assets.ruleset_palette = project.palette.clone();
            rusterix.assets.palette = project.art_palette.clone();
            rusterix.assets.palette_material_ids = palette_material_ids(project);
            rusterix.set_tiles(project.tiles.clone(), true);
            rusterix.set_tile_groups(project.tile_groups.clone());
        }

        // ctx.ui.send(TheEvent::Custom(
        //     TheId::named("Update Tilepicker"),
        //     TheValue::Empty,
        // ));

        // ctx.ui.send(TheEvent::Custom(
        //     TheId::named("Update Materialpicker"),
        //     TheValue::Empty,
        // ));

        self.show_actions(ui, ctx, project, server_ctx);
        self.update_tiles(ui, ctx, project);

        TOOLLIST.write().unwrap().get_current_tool().tool_event(
            ToolEvent::Activate,
            ui,
            ctx,
            project,
            server_ctx,
        );
    }

    /// Apply the given screen to the UI
    pub fn apply_screen(&mut self, ui: &mut TheUI, ctx: &mut TheContext, screen: Option<&Screen>) {
        ui.set_widget_disabled_state("Screen Remove", ctx, screen.is_none());
        ui.set_widget_disabled_state("Screen Settings", ctx, screen.is_none());

        if screen.is_none() {
            ui.set_widget_disabled_state("Widget Add", ctx, true);
            ui.set_widget_disabled_state("Widget Remove", ctx, true);

            if let Some(zoom) = ui.get_widget("Screen Editor Zoom") {
                zoom.set_value(TheValue::Float(1.0));
            }

            if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Screen Editor".into()), None) {
                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                    if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                        rgba_view.set_mode(TheRGBAViewMode::Display);
                        rgba_view.set_zoom(1.0);
                        if let Some(buffer) = ctx.ui.icon("eldiron_map") {
                            rgba_view.set_buffer(buffer.clone());
                        }
                        rgba_view.set_grid(None);
                        ctx.ui.relayout = true;
                    }
                    rgba_layout.scroll_to(Vec2::new(0, 0));
                }
            }
        }

        // if let Some(screen) = screen {
        // ui.set_widget_disabled_state("Widget Add", ctx, false);
        // if !screen.widget_list.is_empty() {
        //     ui.set_widget_disabled_state("Widget Remove", ctx, false);
        // }

        // if let Some(zoom) = ui.get_widget("Screen Editor Zoom") {
        //zoom.set_value(TheValue::Float(screen.zoom));
        // }
        // if let Some(rgba_layout) = ui.get_rgba_layout("Screen Editor") {
        //     if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
        //         //rgba.set_zoom(screen.zoom);
        //         rgba.set_grid(Some(screen.grid_size));
        //     }
        //     rgba_layout.scroll_to(screen.scroll_offset);
        // }
        // }

        // Show the filter region content.

        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Content Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Content Dropdown".to_string()), None)
        {
            if let Some(drop_down_menu) = widget.as_drop_down_menu() {
                drop_down_menu.selected_index()
            } else {
                0
            }
        } else {
            0
        };

        filter_text = filter_text.to_lowercase();

        if let Some(list) = ui.get_list_layout("Screen Content List") {
            list.clear();
            if let Some(screen) = screen {
                if filter_role < 2 {
                    // Show Named Sectors
                    for sector in &screen.map.sectors {
                        if !sector.name.is_empty()
                            && (filter_text.is_empty()
                                || sector.name.to_lowercase().contains(&filter_text))
                        {
                            let mut item = TheListItem::new(TheId::named_with_id(
                                "Screen Content List Item",
                                sector.creator_id,
                            ));
                            item.set_text(sector.name.clone());
                            item.add_value_column(100, TheValue::Text("Widget".to_string()));
                            list.add_item(item, ctx);
                        }
                    }

                    /*
                    for widget in screen.widget_list.iter() {
                        let name: String = widget.name.clone();
                        if filter_text.is_empty() || name.to_lowercase().contains(&filter_text) {
                            let mut item = TheListItem::new(TheId::named_with_id(
                                "Screen Content List Item",
                                widget.id,
                            ));
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Rename Widget...".to_string(),
                                    TheId::named("Rename Widget"),
                                )],
                                ..Default::default()
                            }));
                            item.set_text(name);
                            item.add_value_column(100, TheValue::Text("Widget".to_string()));
                            list.add_item(item, ctx);
                        }
                    }*/
                }
            }

            // Activate the current widget
            // Disabled for now to show screen bundle by default.

            // if let Some(selected) = list.selected() {
            //     ctx.ui
            //         .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
            // } else {
            //     list.select_first_item(ctx);
            // }
        }

        ctx.ui.relayout = true;
    }

    /// Apply the avatars
    pub fn apply_avatars(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(avatar_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id) {
                avatar_node.widgets.clear();
                avatar_node.childs.clear();

                for (_index, avatar) in project.avatars.iter() {
                    let node = gen_avatar_tree_node(avatar);

                    avatar_node.add_child(node);
                }
            }
        }
    }

    /// Rebuild the large thumbnail rows in the top-level Recipes tree item.
    pub fn apply_recipes(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree")
            && let Some(recipes_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_recipes_id)
        {
            recipes_node.widgets.clear();
            recipes_node.childs.clear();
            for recipe in project.procedural_recipes.values() {
                recipes_node.add_widget(Box::new(gen_procedural_recipe_tree_item(recipe, project)));
            }
        }
    }

    fn refresh_recipe_tree_item(
        &self,
        ui: &mut TheUI,
        server_ctx: &ServerContext,
        project: &Project,
        recipe_id: Uuid,
    ) {
        let Some(recipe) = project.procedural_recipes.get(&recipe_id) else {
            return;
        };
        let (name, kind) =
            crate::recipe_utils::recipe_description(&recipe.source).unwrap_or_else(|_| {
                (
                    fl!("invalid_recipe"),
                    crate::recipe_utils::ProceduralRecipeKind::Tile,
                )
            });
        let sub_text = format!(
            "{} · {}",
            crate::recipe_utils::localized_recipe_kind(kind),
            recipe.alias
        );
        let preview = crate::recipe_utils::render_recipe_visual_preview(project, recipe_id).ok();
        if let Some(tree) = ui.get_tree_layout("Project Tree")
            && let Some(node) = tree.get_node_by_id_mut(&server_ctx.tree_recipes_id)
            && let Some(widget) = node
                .widgets
                .iter_mut()
                .find(|widget| widget.id().uuid == recipe_id)
            && let Some(item) = widget.as_tree_item()
        {
            item.set_text(name);
            item.set_sub_text(sub_text);
            if let Some(preview) = preview {
                item.set_icon(preview.scaled(52, 52));
            }
        }
    }

    /// Apply the current regions to the tree.
    pub fn apply_regions(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        let mut id: Option<Uuid> = None;

        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(region_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id) {
                region_node.widgets.clear();
                region_node.childs.clear();

                for (index, region) in project.regions.iter().enumerate() {
                    let mut node = gen_region_tree_node(region);
                    if index == 0 {
                        id = Some(region.id);
                        node.set_open(true);
                    }

                    region_node.add_child(node);
                }
            }
        }

        if let Some(id) = id {
            server_ctx.curr_region = id;
            set_project_context(ctx, ui, project, server_ctx, ProjectContext::Region(id));
            self.apply_region(ui, ctx, Some(id), project);
        }
    }

    /// Apply the current characters to the tree.
    pub fn apply_characters(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(characters_node) =
                tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
            {
                characters_node.widgets.clear();
                characters_node.childs.clear();

                for (_, character) in project.characters.iter() {
                    let node = gen_character_tree_node(character);

                    characters_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current items to the tree.
    pub fn apply_items(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(items_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id) {
                items_node.widgets.clear();
                items_node.childs.clear();

                for (_, item) in project.items.iter() {
                    let node = gen_item_tree_node(item, project);
                    items_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current tilemaps to the tree.
    pub fn apply_tilemaps(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(tilema_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_tilemaps_id)
            {
                tilema_node.widgets.clear();
                tilema_node.childs.clear();

                for tilemap in project.tilemaps.iter() {
                    let node = gen_tilemap_tree_node(tilemap);
                    tilema_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current screens to the tree.
    pub fn apply_screens(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(screen_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_screens_id) {
                screen_node.widgets.clear();
                screen_node.childs.clear();

                for (_, screen) in project.screens.iter() {
                    let node = gen_screen_tree_node(screen);
                    screen_node.add_child(node);
                }
            }
        }
    }

    /// Apply the current assets to the tree.
    pub fn apply_assets(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        project: &mut Project,
    ) {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(font_node) =
                tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_fonts_id)
            {
                font_node.widgets.clear();
                font_node.childs.clear();
            }
            if let Some(audio_node) =
                tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_audio_id)
            {
                audio_node.widgets.clear();
                audio_node.childs.clear();
            }

            for (_, asset) in project.assets.iter() {
                match &asset.buffer {
                    AssetBuffer::Font(_) => {
                        if let Some(font_node) =
                            tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_fonts_id)
                        {
                            font_node.add_child(gen_asset_tree_node(asset));
                        }
                    }
                    AssetBuffer::Audio(_) => {
                        if let Some(audio_node) =
                            tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_audio_id)
                        {
                            audio_node.add_child(gen_asset_tree_node(asset));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Apply the given item to the UI
    pub fn apply_region(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _region_id: Option<Uuid>,
        _project: &mut Project,
    ) {
        /*
        ui.set_widget_disabled_state("Region Remove", ctx, region_id.is_none());
        ui.set_widget_disabled_state("Region Settings", ctx, region_id.is_none());

        if UNDOMANAGER.read().unwrap().has_undo() {
            ctx.ui.set_enabled("Undo");
            ctx.ui.set_enabled("Redo");
        }

        if region_id.is_none() {
            if let Some(zoom) = ui.get_widget("Region Editor Zoom") {
                zoom.set_value(TheValue::Float(1.0));
            }

            if let Some(renderview) = ui.get_render_view("PolyView") {
                if let Some(buffer) = ctx.ui.icon("eldiron") {
                    let dim = *renderview.dim();
                    let scaled_buffer = buffer.scaled(dim.width, dim.height);
                    renderview.render_buffer_mut().fill(BLACK);
                    renderview.render_buffer_mut().copy_into(
                        (dim.width - scaled_buffer.dim().width) / 2,
                        (dim.height - scaled_buffer.dim().height) / 2,
                        &scaled_buffer,
                    );
                    renderview.set_needs_redraw(true);
                }
            }

            if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Region Editor".into()), None) {
                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                    if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                        rgba_view.set_mode(TheRGBAViewMode::Display);
                        rgba_view.set_zoom(1.0);
                        if let Some(buffer) = ctx.ui.icon("eldiron_map") {
                            rgba_view.set_buffer(buffer.clone());
                        }
                        rgba_view.set_grid(None);
                        ctx.ui.relayout = true;
                    }
                    rgba_layout.scroll_to(Vec2::new(0, 0));
                }
            }
        }*/

        /*
        // Show the filter region content.

        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Content Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Content Dropdown".to_string()), None)
        {
            if let Some(drop_down_menu) = widget.as_drop_down_menu() {
                drop_down_menu.selected_index()
            } else {
                0
            }
        } else {
            0
        };

        filter_text = filter_text.to_lowercase();

        if let Some(list) = ui.get_list_layout("Region Content List") {
            list.clear();
            if let Some(region_id) = region_id {
                if let Some(region) = project.get_region(&region_id) {
                    if filter_role < 2 {
                        // Show Characters
                        for (id, character) in region.characters.iter() {
                            let mut name = character.name.clone();

                            if let Some(character_template) =
                                project.characters.get(&character.character_id)
                            {
                                name = character_template.name.clone();
                            }

                            if filter_text.is_empty() || name.to_lowercase().contains(&filter_text)
                            {
                                let mut item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    *id,
                                ));
                                item.set_text(name);
                                item.add_value_column(100, TheValue::Text("Character".to_string()));
                                item.set_context_menu(Some(TheContextMenu {
                                    items: vec![TheContextMenuItem::new(
                                        "Delete Character...".to_string(),
                                        TheId::named("Sidebar Delete Character Instance"),
                                    )],
                                    ..Default::default()
                                }));
                                list.add_item(item, ctx);
                            }
                        }
                    }

                    if filter_role == 0 || filter_role == 3 {
                        // Show Named Sectors
                        for sector in &region.map.sectors {
                            if !sector.name.is_empty()
                                && (filter_text.is_empty()
                                    || sector.name.to_lowercase().contains(&filter_text))
                            {
                                let mut item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    sector.creator_id,
                                ));
                                item.set_text(sector.name.clone());
                                item.add_value_column(100, TheValue::Text("Sector".to_string()));
                                // item.set_context_menu(Some(TheContextMenu {
                                //     items: vec![TheContextMenuItem::new(
                                //         "Delete Character...".to_string(),
                                //         TheId::named("Sidebar Delete Character Instance"),
                                //     )],
                                //     ..Default::default()
                                // }));
                                list.add_item(item, ctx);
                            }
                        }
                    }

                    if filter_role == 0 || filter_role == 3 {
                        // Show Items
                        for (id, item) in region.items.iter() {
                            let mut name = item.name.clone();

                            if let Some(item_template) = project.items.get(&item.item_id) {
                                name = item_template.name.clone();
                            }

                            if filter_text.is_empty() || name.to_lowercase().contains(&filter_text)
                            {
                                let mut item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    *id,
                                ));
                                item.set_text(name);
                                item.add_value_column(100, TheValue::Text("Item".to_string()));
                                item.set_context_menu(Some(TheContextMenu {
                                    items: vec![TheContextMenuItem::new(
                                        "Delete Item...".to_string(),
                                        TheId::named("Sidebar Delete Item Instance"),
                                    )],
                                    ..Default::default()
                                }));
                                list.add_item(item, ctx);
                            }
                        }
                    }
                }
            }
        }*/

        // let mut changed = false;

        // ctx.ui.send(TheEvent::Custom(
        //     TheId::named("Update Minimap"),
        //     TheValue::Empty,
        // ));

        // RUSTERIX.write().unwrap().set_dirty();

        // if let Some(region_id) = region_id {
        //     ctx.ui.send(TheEvent::Custom(
        //         TheId::named("Render SceneManager Map"),
        //         TheValue::Empty,
        //     ));

        // if let Some(region) = project.get_region(&region_id) {
        //     ui.set_widget_value(
        //         "RegionConfigEdit",
        //         ctx,
        //         TheValue::Text(region.config.clone()),
        //     );
        // }
        // }
        /*
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Grid Edit".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.grid_size.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Tile Size".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.tile_size.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(region) = region {
            if let Some(zoom) = ui.get_widget("Region Editor Zoom") {
                zoom.set_value(TheValue::Float(region.zoom));
            }
            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    rgba.set_zoom(region.zoom);
                    rgba.set_grid(Some(region.grid_size));
                }
                rgba_layout.scroll_to(region.scroll_offset);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 1".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_1.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 2".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_2.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 3".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_3.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Property 4".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.property_4.clone()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        // Apply the region's timeline to the editor.
        if let Some(region) = region {
            if let Some(render_view) = ui.get_render_view("MiniMap") {
                let dim = *render_view.dim();
                let buffer = render_view.render_buffer_mut();
                buffer.resize(dim.width, dim.height);
                draw_minimap(region, buffer);
            }
        }*/
    }

    /// Shows the filtered tiles of the given tilemap.
    pub fn show_filtered_tiles(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        tilemap: Option<&Tilemap>,
    ) {
        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Tilemap Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Tilemap Filter Role".to_string()), None)
        {
            if let Some(drop_down_menu) = widget.as_drop_down_menu() {
                drop_down_menu.selected_index()
            } else {
                0
            }
        } else {
            0
        };

        filter_text = filter_text.to_lowercase();

        if let Some(layout) = ui
            .canvas
            .get_layout(Some(&"Tilemap Tile List".to_string()), None)
        {
            if let Some(list_layout) = layout.as_list_layout() {
                if let Some(tilemap) = tilemap {
                    list_layout.clear();
                    for tile in &tilemap.tiles {
                        if (filter_text.is_empty()
                            || tile.name.to_lowercase().contains(&filter_text))
                            && (filter_role == 0
                                || tile.role == TileRole::from_index(filter_role as u8 - 1))
                        {
                            let mut item =
                                TheListItem::new(TheId::named_with_id("Tilemap Tile", tile.id));
                            item.set_text(tile.name.clone());
                            let mut sub_text = if tile.blocking {
                                "Blocking".to_string()
                            } else {
                                "Non-Blocking".to_string()
                            };
                            sub_text += ("  ".to_string() + tile.role.to_string()).as_str();
                            item.set_sub_text(sub_text);
                            item.set_size(42);
                            item.set_icon(tile.sequence.regions[0].scale(&tilemap.buffer, 36, 36));
                            list_layout.add_item(item, ctx);
                        }
                    }
                } else {
                    list_layout.clear();
                }
            }
        }
        ui.select_first_list_item("Tilemap Tile List", ctx);
    }

    fn set_project_action_settings_visible(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        visible: bool,
    ) {
        if let Some(shared) = ui.get_sharedvlayout("Project Context Settings Shared") {
            let mode = if visible {
                TheSharedVLayoutMode::Shared
            } else {
                TheSharedVLayoutMode::Top
            };
            if shared.get_mode() != mode {
                shared.set_mode(mode);
                ctx.ui.relayout = true;
                ctx.ui.redraw_all = true;
            }
        }
    }

    fn set_action_params_editor_text(ui: &mut TheUI, editor_name: &str, text: &str) {
        if let Some(edit) = ui.get_text_area_edit(editor_name) {
            let previous = edit.get_state();
            if edit.text() != text {
                edit.set_text(text.to_string());

                let mut state = edit.get_state();
                let row_max = state.rows.len().saturating_sub(1);
                let row = previous.cursor.row.min(row_max);
                let col_max = state
                    .rows
                    .get(row)
                    .map(|line| line.chars().count())
                    .unwrap_or(0);

                state.cursor.row = row;
                state.cursor.column = previous.cursor.column.min(col_max);
                state.selection.reset();
                TheTextAreaEditTrait::set_state(edit, state);
            }
        }
    }

    fn set_action_params_text(&self, ui: &mut TheUI, text: &str) {
        for editor_name in [
            Self::ACTION_PARAMS_EDITOR,
            Self::PROJECT_ACTION_PARAMS_EDITOR,
        ] {
            Self::set_action_params_editor_text(ui, editor_name, text);
        }
    }

    fn mirror_action_params_editor(&self, ui: &mut TheUI, source_name: &str, text: &str) {
        for editor_name in [
            Self::ACTION_PARAMS_EDITOR,
            Self::PROJECT_ACTION_PARAMS_EDITOR,
        ] {
            if editor_name != source_name {
                Self::set_action_params_editor_text(ui, editor_name, text);
            }
        }
    }

    fn show_empty_action_toml(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.show_action_shortcut(ui, None);
        self.set_project_action_settings_visible(ui, ctx, false);
        if let Some(stack) = ui.get_stack_layout("Sidebar Bottom Stack") {
            stack.set_index(0);
        }
        self.set_action_params_text(ui, "");
        if let Some(title) = ui.get_text("Project Action Settings Title") {
            title.set_text(fl!("settings"));
        }
    }

    fn show_collection_settings(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        collection_id: Uuid,
    ) {
        if let Some(stack) = ui.get_stack_layout("Sidebar Bottom Stack") {
            stack.set_index(2);
        }
        if let Some(tab) = ui.get_layout("Multi Tab")
            && let Some(tab) = tab.as_tab_layout()
        {
            tab.set_index(1);
        }

        let mut nodeui = TheNodeUI::default();
        if let Some(collection) = project.tile_collections.get(&collection_id) {
            nodeui.add_item(TheNodeUIItem::Text(
                "tileCollectionName".into(),
                "Name".into(),
                "Set the collection name.".into(),
                collection.name.clone(),
                None,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::Text(
                "tileCollectionAuthor".into(),
                "Author".into(),
                "Set the collection author.".into(),
                collection.author.clone(),
                None,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::Text(
                "tileCollectionVersion".into(),
                "Version".into(),
                "Set the collection version.".into(),
                collection.version.clone(),
                None,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::Text(
                "tileCollectionDescription".into(),
                "Description".into(),
                "Set the collection description.".into(),
                collection.description.clone(),
                None,
                false,
            ));
        }

        if let Some(layout) = ui.get_text_layout("Collection Settings") {
            nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
    }

    fn show_treasury_settings(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        name: String,
        author: String,
        version: String,
        description: String,
    ) {
        if let Some(stack) = ui.get_stack_layout("Sidebar Bottom Stack") {
            stack.set_index(3);
        }
        if let Some(tab) = ui.get_layout("Multi Tab")
            && let Some(tab) = tab.as_tab_layout()
        {
            tab.set_index(1);
        }

        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Text(
            "treasuryPackageName".into(),
            "Name".into(),
            "Package name.".into(),
            name,
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "treasuryPackageAuthor".into(),
            "Author".into(),
            "Package author.".into(),
            author,
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "treasuryPackageVersion".into(),
            "Version".into(),
            "Package version.".into(),
            version,
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "treasuryPackageDescription".into(),
            "Description".into(),
            "Package description.".into(),
            description,
            None,
            false,
        ));

        if let Some(layout) = ui.get_text_layout("Treasury Settings") {
            nodeui.apply_to_text_layout(layout);
            ctx.ui.set_disabled("treasuryPackageName");
            ctx.ui.set_disabled("treasuryPackageAuthor");
            ctx.ui.set_disabled("treasuryPackageVersion");
            ctx.ui.set_disabled("treasuryPackageDescription");
            ctx.ui.relayout = true;
        }
    }

    fn show_action_toml_params(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &ServerContext,
        action: &dyn Action,
    ) {
        let params = action.params();
        self.show_action_toml_snapshot(ui, ctx, action.accel(), &params, action.id().name);
    }

    fn show_action_toml_snapshot(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        accelerator: Option<TheAccelerator>,
        params: &TheNodeUI,
        title_text: String,
    ) {
        self.show_action_shortcut(ui, accelerator);
        if let Some(stack) = ui.get_stack_layout("Sidebar Bottom Stack") {
            stack.set_index(0);
        }
        let toml_text = nodeui_to_toml(params);
        self.set_project_action_settings_visible(ui, ctx, !toml_text.trim().is_empty());
        self.set_action_params_text(ui, &toml_text);
        if let Some(title) = ui.get_text("Project Action Settings Title") {
            title.set_text(title_text);
        }
    }

    fn show_action_shortcut(&self, ui: &mut TheUI, accelerator: Option<TheAccelerator>) {
        if let Some(widget) = ui.get_widget("Action Shortcut Value") {
            let text = accelerator
                .map(|accelerator| accelerator.description())
                .unwrap_or_else(|| "—".to_string());
            widget.set_value(TheValue::Text(text));
        }
    }

    fn sync_current_action_toml_params(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: Option<&mut Project>,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some(action_id) = server_ctx.curr_action_id else {
            return false;
        };
        let preferred_editor = ctx
            .ui
            .focus
            .as_ref()
            .map(|id| id.name.as_str())
            .filter(|name| {
                *name == Self::ACTION_PARAMS_EDITOR || *name == Self::PROJECT_ACTION_PARAMS_EDITOR
            })
            .unwrap_or(Self::ACTION_PARAMS_EDITOR);
        let fallback_editor = if preferred_editor == Self::ACTION_PARAMS_EDITOR {
            Self::PROJECT_ACTION_PARAMS_EDITOR
        } else {
            Self::ACTION_PARAMS_EDITOR
        };
        let source = ui
            .get_text_area_edit(preferred_editor)
            .map(|edit| edit.text())
            .or_else(|| {
                ui.get_text_area_edit(fallback_editor)
                    .map(|edit| edit.text())
            })
            .unwrap_or_default();
        if source.trim().is_empty() {
            return false;
        }
        let mut actionlist = ACTIONLIST.write().unwrap();
        let Some(action) = actionlist.get_action_by_id_mut(action_id) else {
            return false;
        };
        let mut nodeui = action.params();
        if apply_toml_to_nodeui(&mut nodeui, &source).is_err() {
            return false;
        }

        if action.set_params_from_nodeui(nodeui.clone()) {
            return true;
        }

        let Some(project) = project else {
            return false;
        };

        let mut changed = false;
        for (key, val) in nodeui_to_value_pairs(&nodeui) {
            let ev = TheEvent::ValueChanged(TheId::named(&key), val);
            changed |= action.handle_event(&ev, project, ui, ctx, server_ctx);
        }
        changed
    }

    fn set_action_toml_params(
        &self,
        action: &mut Box<dyn Action>,
        source: &str,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Result<(), String> {
        if source.trim().is_empty() {
            return Ok(());
        }

        let mut nodeui = action.params();
        apply_toml_to_nodeui(&mut nodeui, source)?;
        if action.set_params_from_nodeui(nodeui.clone()) {
            return Ok(());
        }

        for (key, value) in nodeui_to_value_pairs(&nodeui) {
            let event = TheEvent::ValueChanged(TheId::named(&key), value);
            action.handle_event(&event, project, ui, ctx, server_ctx);
        }
        Ok(())
    }

    /// Execute one stable editor command through the normal Action implementation.
    ///
    /// This is the shared boundary for Eldrin scripts and future plugins. It intentionally uses
    /// the existing parameter, apply, project-apply, undo, and scene-rebuild machinery instead of
    /// introducing a parallel mutation path.
    pub fn execute_action_command(
        &mut self,
        request: &EditorActionRequest,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> Result<(), String> {
        let mut actions = ACTIONLIST.write().unwrap();
        let action = actions
            .get_action_by_command_id_mut(&request.command_id)
            .ok_or_else(|| format!("Unknown editor action '{}'.", request.command_id))?;

        let applicable = project
            .get_map(server_ctx)
            .map(|map| action.is_applicable(map, ctx, server_ctx))
            .unwrap_or(false);
        if !applicable {
            return Err(format!(
                "Editor action '{}' is not applicable in the current context.",
                request.command_id
            ));
        }

        server_ctx.curr_action_id = Some(action.id().uuid);
        if let Some(map) = project.get_map(server_ctx) {
            action.load_params(map);
        }
        action.load_params_project(project, server_ctx);
        self.set_action_toml_params(
            action,
            &request.parameters_toml,
            project,
            ui,
            ctx,
            server_ctx,
        )?;
        self.show_action_toml_params(ui, ctx, server_ctx, action.as_ref());

        let mut needs_scene_redraw = false;
        if let Some(map) = project.get_map_mut(server_ctx) {
            needs_scene_redraw = self.apply_action(action, map, ui, ctx, server_ctx, false);
        }
        action.apply_project(project, ui, ctx, server_ctx);

        if needs_scene_redraw {
            crate::utils::editor_scene_full_rebuild(project, server_ctx);
            TOOLLIST
                .write()
                .unwrap()
                .update_geometry_overlay_3d(project, server_ctx);
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
        Ok(())
    }

    /// Select one stable tool through its normal deactivate/activate lifecycle.
    pub fn execute_tool_command(
        &mut self,
        command_id: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> Result<bool, String> {
        let resolved = {
            let tools = TOOLLIST.read().unwrap();
            tools
                .get_game_tool_uuid_by_command_id(command_id)
                .and_then(|id| {
                    tools
                        .game_tools
                        .iter()
                        .find(|tool| tool.id().uuid == id)
                        .map(|tool| (id, tool.id().name, tools.game_tool_is_available(command_id)))
                })
        };
        let Some((tool_id, tool_name, available)) = resolved else {
            return Err(format!("Unknown tool '{command_id}'."));
        };
        if !available {
            return Err(format!(
                "Tool '{command_id}' is unavailable in the current editor context."
            ));
        }

        let changed = TOOLLIST
            .write()
            .unwrap()
            .set_tool(tool_id, ui, ctx, project, server_ctx);
        ctx.ui.set_widget_state(tool_name, TheWidgetState::Selected);
        Ok(changed)
    }

    /// Run an Eldrin script containing `editor_action(id, TOML)` and `editor_tool(id)` calls.
    /// Requests execute sequentially on the Creator UI thread and each successful action keeps its
    /// regular undo entry. Tool requests retain the normal activation lifecycle.
    pub fn execute_action_script(
        &mut self,
        source: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> Result<usize, String> {
        let requests = collect_editor_automation_requests(source)?;
        for (index, request) in requests.iter().enumerate() {
            let result = match request {
                EditorAutomationRequest::Action(request) => {
                    self.execute_action_command(request, ui, ctx, project, server_ctx)
                }
                EditorAutomationRequest::SelectTool { command_id } => self
                    .execute_tool_command(command_id, ui, ctx, project, server_ctx)
                    .map(|_| ()),
            };
            result.map_err(|error| format!("Editor operation {} failed: {}", index + 1, error))?;
        }
        Ok(requests.len())
    }

    /// Shows the filtered actions for the current selection.
    pub fn show_actions(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        for list_id in ["Action List", "Prefab Action List"] {
            if let Some(layout) = ui.canvas.get_layout(Some(&list_id.to_string()), None)
                && let Some(list_layout) = layout.as_list_layout()
            {
                list_layout.clear();

                let actions = ACTIONLIST.read().unwrap();
                let mut found_current = false;

                let mut visible_actions: Vec<(usize, usize, TheListItem)> = vec![];
                let hide_camera_actions =
                    DOCKMANAGER.read().unwrap().get_state() == DockManagerState::Editor;

                if let Some(map) = project.get_map(server_ctx).or(Some(&Map::default())) {
                    for (registration_index, action) in actions.actions.iter().enumerate() {
                        let is_current = Some(action.id().uuid) == server_ctx.curr_action_id;
                        let keep_current_action_slots =
                            is_current && action.preserves_hud_material_slots();
                        if action.is_applicable(map, ctx, server_ctx) || keep_current_action_slots {
                            if hide_camera_actions && action.role() == ActionRole::Camera {
                                continue;
                            }
                            let descriptor = actions
                                .descriptor_by_id(action.id().uuid)
                                .expect("registered action descriptor");
                            let mut item = TheListItem::new(action.id().clone());
                            item.set_text(descriptor.group.qualified_name(&action.id().name));

                            // let mut accel_text = String::new();
                            // if let Some(accel) = action.accel() {
                            //     accel_text = accel.description();
                            // }
                            // item.add_value_column(110, TheValue::Text(accel_text));
                            //

                            let mut status_text = action.info().to_string();
                            if let Some(accel) = action.accel() {
                                status_text = format!("{} ({})", status_text, accel.description());
                            }
                            item.set_status_text(&status_text);
                            item.set_background_palette(
                                ActionGroups,
                                descriptor.group.palette_slot(),
                            );

                            if is_current {
                                found_current = true;
                                item.set_state(TheWidgetState::Selected);
                            }

                            visible_actions.push((
                                descriptor.group.palette_slot(),
                                registration_index,
                                item,
                            ));
                        }
                    }
                }

                visible_actions.sort_by_key(|(group_order, registration_index, _)| {
                    (*group_order, *registration_index)
                });
                for (_, _, item) in visible_actions {
                    list_layout.add_item(item, ctx);
                }

                if !found_current {
                    server_ctx.curr_action_id = None;
                }
            }
        }

        if let Some(action_id) = server_ctx.curr_action_id {
            self.sync_current_action_toml_params(ui, ctx, None, server_ctx);
            if let Some(action) = ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id) {
                if let Some(map) = project.get_map(server_ctx) {
                    action.load_params(map);
                } else {
                    let default_map = Map::default();
                    action.load_params(&default_map);
                }
                action.load_params_project(project, server_ctx);
                self.show_action_toml_params(ui, ctx, server_ctx, action.as_ref());
            }
        } else {
            self.show_empty_action_toml(ui, ctx);
        }
    }

    /// Apply the given asset to the UI
    pub fn apply_asset(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext, _asset: Option<&Asset>) {}

    /// Deselects the section buttons
    pub fn deselect_sections_buttons(
        &mut self,
        ctx: &mut TheContext,
        ui: &mut TheUI,
        except: String,
    ) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Section Buttons".into()), None) {
            for w in layout.widgets() {
                if !w.id().name.starts_with(&except) {
                    w.set_state(TheWidgetState::None);
                }
            }
        }

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Soft Update Minimap"),
            TheValue::Empty,
        ));
    }

    pub fn select_section_button(&mut self, ui: &mut TheUI, name: String) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Section Buttons".into()), None) {
            for w in layout.widgets() {
                if w.id().name.starts_with(&name) {
                    w.set_state(TheWidgetState::Selected);
                }
            }
        }
    }

    /// Returns the selected id in the given list layout
    pub fn get_selected_in_list_layout(&self, ui: &mut TheUI, layout_name: &str) -> Option<TheId> {
        if let Some(layout) = ui.canvas.get_layout(Some(&layout_name.to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                return list_layout.selected();
            }
        }
        None
    }

    /// Deselects all items in the given list layout.
    pub fn deselect_all(&self, layout_name: &str, ui: &mut TheUI) {
        if let Some(layout) = ui.canvas.get_layout(Some(&layout_name.to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.deselect_all();
            }
        }
    }

    /// Clears the debug messages.
    pub fn clear_debug_messages(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Debug List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();

                let mut item = TheListItem::new(TheId::empty());
                item.set_text(fl!("info_server_started"));
                item.add_value_column(100, TheValue::Text("Status".to_string()));
                list_layout.add_item(item, ctx);
            }
        }
    }

    pub fn apply_action(
        &self,
        action: &Box<dyn Action>,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        param_update: bool,
    ) -> bool {
        if let Some(undo_atom) = action.apply(map, ui, ctx, server_ctx) {
            if server_ctx.editor_view_mode == EditorViewMode::D2
                && server_ctx.profile_view.is_some()
            {
            } else {
                map.update_surfaces();
                let used_incremental =
                    if let ProjectUndoAtom::MapEdit(_, old_map, new_map) = &undo_atom {
                        ToolList::try_incremental_map_edit(old_map, new_map, server_ctx)
                    } else {
                        false
                    };
                UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                if !used_incremental {
                    return true;
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                return false;
            }
            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
            crate::editor::RUSTERIX.write().unwrap().set_dirty();
        }

        if !param_update {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Action List"),
                TheValue::Empty,
            ));
        }
        false
    }

    /// Tilemaps in the project have been updated, propagate the change to all relevant parties.
    pub fn update_tiles(&mut self, _ui: &mut TheUI, ctx: &mut TheContext, project: &mut Project) {
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.set_tiles(project.tiles.clone(), true);
        rusterix.set_tile_groups(project.tile_groups.clone());
        SCENEMANAGER.write().unwrap().set_tile_list(
            rusterix.assets.tile_list.clone(),
            rusterix.assets.tile_indices.clone(),
        );

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
    }

    pub fn show_console_page(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.activate_navigation_page(2, ui, ctx, project, server_ctx)
    }

    pub fn show_project_page(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.activate_navigation_page(0, ui, ctx, project, server_ctx)
    }

    pub fn show_actions_page(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.activate_navigation_page(1, ui, ctx, project, server_ctx)
    }

    pub fn show_debug_page(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.activate_navigation_page(3, ui, ctx, project, server_ctx)
    }

    pub fn show_help_page(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.activate_navigation_page(4, ui, ctx, project, server_ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_navigation_wraps_in_both_directions() {
        assert_eq!(Sidebar::next_navigation_page(0, false), 1);
        assert_eq!(Sidebar::next_navigation_page(1, false), 2);
        assert_eq!(Sidebar::next_navigation_page(2, false), 3);
        assert_eq!(Sidebar::next_navigation_page(3, false), 4);
        assert_eq!(Sidebar::next_navigation_page(4, false), 0);
        assert_eq!(Sidebar::next_navigation_page(0, true), 4);
        assert_eq!(Sidebar::next_navigation_page(4, true), 3);
        assert_eq!(Sidebar::next_navigation_page(3, true), 2);
        assert_eq!(Sidebar::next_navigation_page(2, true), 1);
        assert_eq!(Sidebar::next_navigation_page(1, true), 0);
    }

    #[test]
    fn navigation_help_includes_its_direct_shortcut() {
        let help = Sidebar::navigation_page_status("Project".to_string(), 0);
        let expected = TheAccelerator::new(
            TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
            SIDEBAR_NAVIGATION_SHORTCUTS[0],
        )
        .description();

        assert!(help.contains(&expected));
    }
}
