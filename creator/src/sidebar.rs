use crate::editor::{
    ACTIONLIST, CODEEDITOR, CONFIG, CONFIGEDITOR, PALETTE, RUSTERIX, SCENEMANAGER, SHADEGRIDFX,
    SIDEBARMODE, TILEMAPEDITOR, TOOLLIST, UNDOMANAGER,
};
use crate::minimap::draw_minimap;
use crate::prelude::*;
use crate::undo::project_helper::*;
use codegridfx::Module;
use rusterix::TileRole;

#[derive(PartialEq, Debug)]
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

    stack_layout_id: TheId,
    curr_tilemap_uuid: Option<Uuid>,

    pub startup: bool,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    pub fn new() -> Self {
        Self {
            width: 380,

            stack_layout_id: TheId::empty(),
            curr_tilemap_uuid: None,

            startup: true,
        }
    }

    pub fn init_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let mut sectionbar_canvas = TheCanvas::new();

        let mut section_bar_canvas = TheCanvas::new();
        section_bar_canvas.set_widget(TheSectionbar::new(TheId::named("Sectionbar")));
        sectionbar_canvas.set_top(section_bar_canvas);

        let mut region_sectionbar_button = TheSectionbarButton::new(TheId::named("Region Section"));
        region_sectionbar_button.set_text("Region".to_string());
        region_sectionbar_button.set_state(TheWidgetState::Selected);
        region_sectionbar_button.set_status_text(
            "Edit and manage the regions available in the game. Regions can contain 2D and 3D content.",
        );

        let mut character_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Character Section"));
        character_sectionbar_button.set_text("Entity".to_string());
        character_sectionbar_button.set_status_text(
            "Edit and manage the characers (and their behavior) available in the game.",
        );

        let mut item_sectionbar_button = TheSectionbarButton::new(TheId::named("Item Section"));
        item_sectionbar_button.set_text("Item".to_string());
        item_sectionbar_button.set_status_text("Edit and manage the items available in the game.");

        let mut tilemap_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Tileset Section"));
        tilemap_sectionbar_button.set_text("Tileset".to_string());
        tilemap_sectionbar_button.set_status_text(
            "Edit and manage your tilemaps. A tilemap is an image containing square tile elements.",
        );

        let mut module_sectionbar_button = TheSectionbarButton::new(TheId::named("Module Section"));
        module_sectionbar_button.set_text("Module".to_string());
        module_sectionbar_button.set_status_text(
            "Edit and manage your games code modules. Code modules are reusable code functions.",
        );

        let mut screen_sectionbar_button = TheSectionbarButton::new(TheId::named("Screen Section"));
        screen_sectionbar_button.set_text("Screen".to_string());
        screen_sectionbar_button.set_status_text(
            "Edit and manage your game screens. Screens are the visible areas of your game.",
        );

        let mut asset_sectionbar_button = TheSectionbarButton::new(TheId::named("Asset Section"));
        asset_sectionbar_button.set_text("Asset".to_string());
        asset_sectionbar_button.set_status_text(
            "Manage assets in the asset library, such as images, sounds, and fonts.",
        );

        let mut material_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Shader Section"));
        material_sectionbar_button.set_text("Shader".to_string());
        material_sectionbar_button.set_status_text("Currently available shaders.");

        let mut action_sectionbar_button = TheSectionbarButton::new(TheId::named("Action Section"));
        action_sectionbar_button.set_text("Action".to_string());
        action_sectionbar_button.set_status_text("Actions");
        action_sectionbar_button.set_status_text(
            "Apply the applicable actions to the current 2D and 3D geometry selection.",
        );

        // let mut node_sectionbar_button = TheSectionbarButton::new(TheId::named("Node Section"));
        // node_sectionbar_button.set_text("Node".to_string());
        // node_sectionbar_button.set_status_text("The UI of the currently selected node.");

        let mut debug_sectionbar_button = TheSectionbarButton::new(TheId::named("Debug Section"));
        debug_sectionbar_button.set_text("Log".to_string());
        debug_sectionbar_button.set_status_text(
            "See the server log including debug and errors messages produced by the game code.",
        );

        let mut palette_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Palette Section"));
        palette_sectionbar_button.set_text("Palette".to_string());
        palette_sectionbar_button
            .set_status_text("Edit the color palette which contains the colors used in the game.");

        let mut vlayout = TheVLayout::new(TheId::named("Section Buttons"));
        vlayout.add_widget(Box::new(region_sectionbar_button));
        vlayout.add_widget(Box::new(character_sectionbar_button));
        vlayout.add_widget(Box::new(item_sectionbar_button));
        vlayout.add_widget(Box::new(tilemap_sectionbar_button));
        vlayout.add_widget(Box::new(module_sectionbar_button));
        vlayout.add_widget(Box::new(screen_sectionbar_button));
        vlayout.add_widget(Box::new(asset_sectionbar_button));
        vlayout.add_widget(Box::new(material_sectionbar_button));
        vlayout.add_widget(Box::new(action_sectionbar_button));
        // vlayout.add_widget(Box::new(node_sectionbar_button));
        vlayout.add_widget(Box::new(debug_sectionbar_button));
        vlayout.add_widget(Box::new(palette_sectionbar_button));
        vlayout.set_margin(Vec4::new(5, 10, 5, 5));
        vlayout.set_padding(4);
        vlayout.set_background_color(Some(SectionbarBackground));
        vlayout.limiter_mut().set_max_width(90);
        vlayout.set_reverse_index(Some(3));
        sectionbar_canvas.set_layout(vlayout);

        //

        // let mut header = TheCanvas::new();
        // let mut switchbar = TheSwitchbar::new(TheId::named("Switchbar Section Header"));
        // switchbar.set_text("Project".to_string());
        // header.set_widget(switchbar);

        let mut stack_layout = TheStackLayout::new(TheId::named("List Stack Layout"));

        stack_layout.limiter_mut().set_max_width(self.width);

        self.stack_layout_id = stack_layout.id().clone();

        // Regions

        let mut regions_canvas = TheCanvas::default();
        let mut region_shared_layout = TheSharedVLayout::new(TheId::named("Region Shared"));

        let mut list_layout = TheListLayout::new(TheId::named("Region List"));
        list_layout
            .limiter_mut()
            .set_max_size(Vec2::new(self.width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut region_add_button = TheTraybarButton::new(TheId::named("Region Add"));
        region_add_button.set_icon_name("icon_role_add".to_string());
        region_add_button.set_status_text("Add a new region.");

        let mut region_remove_button = TheTraybarButton::new(TheId::named("Region Remove"));
        region_remove_button.set_icon_name("icon_role_remove".to_string());
        region_remove_button.set_status_text("Remove the current region.");
        region_remove_button.set_disabled(true);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(region_add_button));
        toolbar_hlayout.add_widget(Box::new(region_remove_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut region_tab = TheTabLayout::new(TheId::named("Region Tab Layout"));

        // Region Content

        let mut list_layout = TheListLayout::new(TheId::named("Region Content List"));
        list_layout.limiter_mut().set_max_width(self.width);
        let mut content_canvas = TheCanvas::default();
        content_canvas.set_layout(list_layout);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Region Content Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(Vec2::new(85, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Region Content Dropdown"));
        drop_down.add_option("All".to_string());
        drop_down.add_option("Character".to_string());
        drop_down.add_option("Item".to_string());
        drop_down.add_option("Sector".to_string());
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        content_canvas.set_top(toolbar_canvas);

        region_tab.add_canvas("Content".to_string(), content_canvas);

        // Region Settings

        let mut settings_canvas = TheCanvas::default();

        let mut textedit = TheTextAreaEdit::new(TheId::named("RegionConfigEdit"));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("TOML");
            }
        }
        textedit.set_continuous(true);
        textedit.display_line_number(false);
        textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(13.0);
        settings_canvas.set_widget(textedit);

        region_tab.add_canvas("Config".to_string(), settings_canvas);

        let mut center_tab = TheCanvas::default();
        center_tab.set_layout(region_tab);

        region_shared_layout.add_canvas(list_canvas);
        region_shared_layout.add_canvas(center_tab);
        region_shared_layout.set_mode(TheSharedVLayoutMode::Shared);
        region_shared_layout.set_shared_ratio(0.4);
        regions_canvas.set_layout(region_shared_layout);

        stack_layout.add_canvas(regions_canvas);

        // Character

        let mut character_canvas = TheCanvas::default();
        let mut list_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Character List"));
        list_layout.limiter_mut().set_max_width(self.width);
        list_canvas.set_layout(list_layout);

        let mut regions_add_button = TheTraybarButton::new(TheId::named("Character Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        regions_add_button.set_status_text("Add a new character.");
        let mut regions_remove_button = TheTraybarButton::new(TheId::named("Character Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());
        regions_remove_button.set_status_text("Remove the current character.");

        let mut character_region_override =
            TheGroupButton::new(TheId::named("Character Region Override"));
        character_region_override.add_text_status(
            "Character".to_string(),
            "Show the character map.".to_string(),
        );
        character_region_override.add_text_status(
            "Region".to_string(),
            "Show the region map, drag and drop to instantiate a character.".to_string(),
        );
        character_region_override.set_item_width(80);
        character_region_override.set_index(1);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));
        toolbar_hlayout.add_widget(Box::new(character_region_override));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        toolbar_hlayout.set_reverse_index(Some(1));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        character_canvas.set_top(list_canvas);
        stack_layout.add_canvas(character_canvas);

        // Item

        let mut item_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Item List"));
        list_layout.limiter_mut().set_max_width(self.width);
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut item_add_button = TheTraybarButton::new(TheId::named("Item Add"));
        item_add_button.set_icon_name("icon_role_add".to_string());
        item_add_button.set_status_text("Add a new item.");
        let mut item_remove_button = TheTraybarButton::new(TheId::named("Item Remove"));
        item_remove_button.set_icon_name("icon_role_remove".to_string());
        item_remove_button.set_status_text("Remove the current item.");

        let mut item_region_override = TheGroupButton::new(TheId::named("Item Region Override"));
        item_region_override.add_text_status("Item".to_string(), "Show the item map.".to_string());
        item_region_override.add_text_status(
            "Region".to_string(),
            "Show the region map, drag and drop to instantiate an item.".to_string(),
        );
        item_region_override.set_item_width(80);
        item_region_override.set_index(1);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(item_add_button));
        toolbar_hlayout.add_widget(Box::new(item_remove_button));
        toolbar_hlayout.add_widget(Box::new(item_region_override));
        toolbar_hlayout.set_reverse_index(Some(1));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        item_canvas.set_top(list_canvas);
        stack_layout.add_canvas(item_canvas);

        // Tilemaps

        let mut tiles_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Tilemap List"));
        list_layout
            .limiter_mut()
            .set_max_size(Vec2::new(self.width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut regions_add_button = TheTraybarButton::new(TheId::named("Tilemap Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        regions_add_button.set_status_text("Add a new tilemap from an existing PNG image.");
        let mut regions_remove_button = TheTraybarButton::new(TheId::named("Tilemap Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());
        regions_remove_button.set_status_text("Remove the current tilemap.");

        let mut grid_text = TheText::new(TheId::empty());
        grid_text.set_text("Grid Size".to_string());
        let mut grid_edit = TheTextLineEdit::new(TheId::named("Tilemap Grid Edit"));
        grid_edit.set_range(TheValue::RangeI32(1..=100));
        grid_edit.limiter_mut().set_max_width(50);
        grid_edit.set_status_text("Edit the grid size of the tilemap.");

        let mut import_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Tilemap Import"));
        import_button.set_icon_name("import".to_string());
        import_button.set_status_text("Import a previously exported Eldiron Tilemap from file.");
        let mut export_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Tilemap Export"));
        export_button.set_icon_name("export".to_string());
        export_button.set_status_text("Export an Eldiron Tilemap with all tile metadata.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(grid_text));
        toolbar_hlayout.add_widget(Box::new(grid_edit));
        toolbar_hlayout.add_widget(Box::new(import_button));
        toolbar_hlayout.add_widget(Box::new(export_button));

        toolbar_hlayout.set_reverse_index(Some(2));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut tiles_list_canvas = TheCanvas::default();

        let mut tiles_list_header_canvas = TheCanvas::default();
        tiles_list_header_canvas.set_widget(TheStatusbar::new(TheId::empty()));
        let mut tiles_list_header_canvas_hlayout = TheHLayout::new(TheId::empty());
        tiles_list_header_canvas_hlayout.set_background_color(None);
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        tiles_list_header_canvas_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        tiles_list_header_canvas_hlayout.set_padding(3);
        tiles_list_header_canvas_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Tilemap Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(Vec2::new(75, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show tiles containing the given text.");
        filter_edit.set_continuous(true);
        tiles_list_header_canvas_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Tilemap Filter Role"));
        drop_down.add_option("All".to_string());
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        tiles_list_header_canvas_hlayout.add_widget(Box::new(drop_down));
        // for dir in TileRole::iterator() {
        //     let mut color_button = TheColorButton::new(TheId::named("Tilemap Filter Character"));
        //     color_button.limiter_mut().set_max_size(vec2i(17, 17));
        //     color_button.set_color(dir.to_color().to_u8_array());
        //     color_button.set_state(TheWidgetState::Selected);
        //     color_button.set_status_text(format!("Show \"{}\" tiles.", dir.to_string()).as_str());
        //     tiles_list_header_canvas_hlayout.add_widget(Box::new(color_button));
        // }

        tiles_list_header_canvas.set_layout(tiles_list_header_canvas_hlayout);

        let mut tile_list_layout = TheListLayout::new(TheId::named("Tilemap Tile List"));
        tile_list_layout.set_item_size(42);
        tiles_list_canvas.set_top(tiles_list_header_canvas);
        tiles_list_canvas.set_layout(tile_list_layout);

        tiles_canvas.set_top(list_canvas);
        tiles_canvas.set_bottom(tiles_list_canvas);
        stack_layout.add_canvas(tiles_canvas);

        // Module

        let mut module_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Module List"));
        list_layout.limiter_mut().set_max_width(self.width);
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut module_add_button = TheTraybarButton::new(TheId::named("Module Add"));
        module_add_button.set_icon_name("icon_role_add".to_string());
        module_add_button.set_status_text("Add a new module.");
        let mut module_remove_button = TheTraybarButton::new(TheId::named("Module Remove"));
        module_remove_button.set_icon_name("icon_role_remove".to_string());
        module_remove_button.set_status_text("Remove the current module.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(module_add_button));
        toolbar_hlayout.add_widget(Box::new(module_remove_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        module_canvas.set_top(list_canvas);
        stack_layout.add_canvas(module_canvas);

        // Screen UI

        let mut screens_canvas = TheCanvas::default();

        let mut shared_layout = TheSharedVLayout::new(TheId::named("Screen Shared Layout"));

        let mut list_layout = TheListLayout::new(TheId::named("Screen List"));
        list_layout
            .limiter_mut()
            .set_max_size(Vec2::new(self.width, 200));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut screen_add_button = TheTraybarButton::new(TheId::named("Screen Add"));
        screen_add_button.set_icon_name("icon_role_add".to_string());
        screen_add_button.set_status_text("Add a new screen.");

        let mut screen_remove_button = TheTraybarButton::new(TheId::named("Screen Remove"));
        screen_remove_button.set_icon_name("icon_role_remove".to_string());
        screen_remove_button.set_status_text("Remove the current screen.");
        screen_remove_button.set_disabled(true);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(screen_add_button));
        toolbar_hlayout.add_widget(Box::new(screen_remove_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut screen_canvas: TheCanvas = TheCanvas::new();

        // Screen Content

        let mut list_layout = TheListLayout::new(TheId::named("Screen Content List"));
        list_layout
            .limiter_mut()
            .set_max_size(Vec2::new(self.width, 250));
        let mut content_canvas = TheCanvas::default();
        content_canvas.set_layout(list_layout);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Screen Content Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(Vec2::new(85, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Screen Content Dropdown"));
        drop_down.add_option("All".to_string());
        drop_down.add_option("Sectors".to_string());
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        content_canvas.set_top(toolbar_canvas);

        screen_canvas.set_center(content_canvas);

        shared_layout.add_canvas(list_canvas);
        shared_layout.add_canvas(screen_canvas);
        shared_layout.set_mode(TheSharedVLayoutMode::Shared);
        shared_layout.set_shared_ratio(0.4);

        screens_canvas.set_layout(shared_layout);
        stack_layout.add_canvas(screens_canvas);

        // Asset

        let mut asset_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Asset List"));
        list_layout.limiter_mut().set_max_width(300);
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut screen_add_button = TheTraybarButton::new(TheId::named("Asset Add"));
        screen_add_button.set_icon_name("icon_role_add".to_string());
        screen_add_button.set_status_text("Add a new asset.");

        screen_add_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Add Image...".to_string(), TheId::named("Add Image")),
                TheContextMenuItem::new("Add Font...".to_string(), TheId::named("Add Font")),
            ],
            ..Default::default()
        }));

        let mut screen_remove_button = TheTraybarButton::new(TheId::named("Asset Remove"));
        screen_remove_button.set_icon_name("icon_role_remove".to_string());
        screen_remove_button.set_status_text("Remove the current asset.");
        screen_remove_button.set_disabled(true);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(screen_add_button));
        toolbar_hlayout.add_widget(Box::new(screen_remove_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        asset_canvas.set_top(list_canvas);

        // Asset Preview

        let mut list_layout = TheListLayout::new(TheId::named("Asset Preview List"));
        list_layout.limiter_mut().set_max_width(self.width);
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        asset_canvas.set_center(list_canvas);

        stack_layout.add_canvas(asset_canvas);

        // Material UI

        let mut material_canvas = TheCanvas::default();
        let mut material_list_canvas = TheCanvas::default();

        // -- Filter

        let mut material_filter_canvas = TheCanvas::default();
        material_filter_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut material_list_header_canvas_hlayout = TheHLayout::new(TheId::empty());
        material_list_header_canvas_hlayout.set_background_color(None);

        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        let mut filter_edit = TheTextLineEdit::new(TheId::named("Shader Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(Vec2::new(180, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show materials containing the given text.");
        filter_edit.set_continuous(true);

        material_list_header_canvas_hlayout.add_widget(Box::new(filter_text));
        material_list_header_canvas_hlayout.add_widget(Box::new(filter_edit));
        material_filter_canvas.set_layout(material_list_header_canvas_hlayout);

        // -- Panel

        let mut material_list_header_canvas = TheCanvas::default();
        material_list_header_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut material_list_header_canvas_hlayout = TheHLayout::new(TheId::empty());
        material_list_header_canvas_hlayout.set_background_color(None);

        material_list_header_canvas_hlayout.set_margin(Vec4::new(5, 2, 5, 2));

        let mut material_add_button = TheTraybarButton::new(TheId::named("Shader Add"));
        material_add_button.set_icon_name("icon_role_add".to_string());
        material_add_button.set_status_text("Add a new material.");

        let mut material_remove_button = TheTraybarButton::new(TheId::named("Shader Remove"));
        material_remove_button.set_icon_name("icon_role_remove".to_string());
        material_remove_button.set_status_text("Remove the current material.");

        let mut import_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Shader Import"));
        import_button.set_icon_name("import".to_string());
        import_button.set_status_text("Import an Eldiron shader from a file.");
        let mut export_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Shader Export"));
        export_button.set_icon_name("export".to_string());
        export_button.set_status_text("Export an Eldiron shader to a file.");

        material_list_header_canvas_hlayout.add_widget(Box::new(material_add_button));
        material_list_header_canvas_hlayout.add_widget(Box::new(material_remove_button));
        material_list_header_canvas_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        material_list_header_canvas_hlayout.add_widget(Box::new(import_button));
        material_list_header_canvas_hlayout.add_widget(Box::new(export_button));
        // material_list_header_canvas_hlayout.set_reverse_index(Some(1));

        material_list_header_canvas.set_layout(material_list_header_canvas_hlayout);

        let material_list_layout = TheListLayout::new(TheId::named("Shader List"));
        material_list_canvas.set_top(material_filter_canvas);
        material_list_canvas.set_bottom(material_list_header_canvas);
        material_list_canvas.set_layout(material_list_layout);

        material_canvas.set_center(material_list_canvas);
        stack_layout.add_canvas(material_canvas);

        // Action UI

        let mut action_canvas = TheCanvas::default();
        let mut shared_layout = TheSharedVLayout::new(TheId::named("Screen Shared Layout"));

        let mut action_list_canvas = TheCanvas::default();

        let mut action_list_header_canvas = TheCanvas::default();
        action_list_header_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut action_list_header_canvas_hlayout = TheHLayout::new(TheId::empty());
        action_list_header_canvas_hlayout.set_background_color(None);

        let mut action_apply_button = TheTraybarButton::new(TheId::named("Action Apply"));
        action_apply_button.set_text("Apply Action".to_string());
        action_apply_button.set_status_text("Apply the current action.");

        action_list_header_canvas_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        action_list_header_canvas_hlayout.set_padding(3);
        action_list_header_canvas_hlayout.add_widget(Box::new(action_apply_button));

        action_list_header_canvas.set_layout(action_list_header_canvas_hlayout);

        let mut action_settings_canvas = TheCanvas::default();
        let mut text_layout = TheTextLayout::new(TheId::named("Action Settings"));
        text_layout.limiter_mut().set_max_width(self.width);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        action_settings_canvas.set_layout(text_layout);

        // let mut action_settings_header = TheCanvas::new();
        // let mut switchbar = TheSwitchbar::new(TheId::named("Action Settings Section Header"));
        // switchbar.set_text("Action Settings".to_string());
        // action_settings_header.set_widget(switchbar);
        // action_settings_canvas.set_top(action_settings_header);

        let action_list_layout = TheListLayout::new(TheId::named("Action List"));
        action_list_canvas.set_layout(action_list_layout);
        action_list_canvas.set_bottom(action_list_header_canvas);

        shared_layout.add_canvas(action_list_canvas);
        shared_layout.add_canvas(action_settings_canvas);

        shared_layout.set_mode(TheSharedVLayoutMode::Shared);
        shared_layout.set_shared_ratio(0.5);

        action_canvas.set_layout(shared_layout);
        stack_layout.add_canvas(action_canvas);

        /*
        // Node UI

        let mut node_ui_canvas = TheCanvas::default();

        let mut text_layout = TheTextLayout::new(TheId::named("Node Settings"));
        text_layout.limiter_mut().set_max_width(self.width);
        //text_layout.set_fixed_text_width(110);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        node_ui_canvas.set_layout(text_layout);

        stack_layout.add_canvas(node_ui_canvas);
        */

        // Debug

        let mut debug_canvas = TheCanvas::default();

        let mut logwidget = TheTextAreaEdit::new(TheId::named("LogEdit"));
        logwidget.display_line_number(false);
        logwidget.readonly(true);
        logwidget.set_code_type("Text");
        logwidget.set_code_theme("base16-eighties.dark");
        logwidget.use_global_statusbar(true);
        logwidget.set_font_size(12.0);

        debug_canvas.set_widget(logwidget);
        stack_layout.add_canvas(debug_canvas);

        // Palette

        let mut palette_canvas = TheCanvas::default();
        let palette_picker = ThePalettePicker::new(TheId::named("Palette Picker"));
        palette_canvas.set_widget(palette_picker);

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));

        let mut index_text = TheText::new(TheId::named("Palette Index Text"));
        index_text.set_fixed_size_text("255.".into());
        index_text.set_value(TheValue::Text("000".into()));
        index_text.set_status_text("The index of the selected color.");

        let mut hex_edit = TheTextLineEdit::new(TheId::named("Palette Hex Edit"));
        hex_edit.limiter_mut().set_max_width(100);
        hex_edit.set_status_text("The value of the selected color in hex format.");

        // let mut name_edit = TheTextLineEdit::new(TheId::named("Palette Name Edit"));
        // name_edit.limiter_mut().set_max_width(100);
        // name_edit.set_status_text("The name of the selected color.");

        let mut import_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Palette Import"));
        import_button.set_icon_name("import".to_string());
        import_button.set_status_text("Import a palette in .txt format.");

        let mut clear_all_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Palette Clear"));
        clear_all_button.set_icon_name("trash".to_string());
        clear_all_button.set_status_text("Clear all colors in the current palette.");

        toolbar_hlayout.add_widget(Box::new(index_text));
        toolbar_hlayout.add_widget(Box::new(hex_edit));
        // toolbar_hlayout.add_widget(Box::new(name_edit));
        toolbar_hlayout.add_widget(Box::new(import_button));
        toolbar_hlayout.add_widget(Box::new(clear_all_button));
        toolbar_hlayout.set_reverse_index(Some(2));

        toolbar_canvas.set_layout(toolbar_hlayout);
        palette_canvas.set_bottom(toolbar_canvas);
        stack_layout.add_canvas(palette_canvas);

        // - End of Sections

        // Tree View

        let mut canvas: TheCanvas = TheCanvas::new();

        let mut project_canvas: TheCanvas = TheCanvas::new();
        let mut project_tree_layout = TheTreeLayout::new(TheId::named("Project Tree"));
        let root = project_tree_layout.get_root();

        let mut regions_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Regions", server_ctx.tree_regions_id));
        regions_node.set_open(true);

        root.add_child(regions_node);

        let mut characters_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            "Characters",
            server_ctx.tree_characters_id,
        ));
        characters_node.set_open(true);
        root.add_child(characters_node);

        let mut items_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Items", server_ctx.tree_items_id));
        items_node.set_open(true);
        root.add_child(items_node);

        project_canvas.set_layout(project_tree_layout);

        // Tree View Toolbar

        let mut add_button = TheTraybarButton::new(TheId::named("Project Add"));
        add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_status_text("Add to the project.");
        add_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Add Region".to_string(), TheId::named("Add Region")),
                TheContextMenuItem::new("Add Character".to_string(), TheId::named("Add Character")),
            ],
            ..Default::default()
        }));

        let mut remove_button = TheTraybarButton::new(TheId::named("Project Remove"));
        remove_button.set_icon_name("icon_role_remove".to_string());
        remove_button.set_status_text("Remove an item from the project.");

        let mut project_context_text = TheText::new(TheId::named("Project Context"));
        project_context_text.set_text("".to_string());

        let mut import_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Project Import"));
        import_button.set_icon_name("import".to_string());
        import_button.set_status_text("Import to the project.");
        import_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Import Region".to_string(), TheId::named("Import Region")),
                TheContextMenuItem::new(
                    "Import Character".to_string(),
                    TheId::named("Import Character"),
                ),
            ],
            ..Default::default()
        }));

        let mut export_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Project Export"));
        export_button.set_icon_name("export".to_string());
        export_button.set_status_text("Export from the project.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(add_button));
        toolbar_hlayout.add_widget(Box::new(remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(project_context_text));
        toolbar_hlayout.add_widget(Box::new(import_button));
        toolbar_hlayout.add_widget(Box::new(export_button));

        toolbar_hlayout.set_reverse_index(Some(2));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        project_canvas.set_bottom(toolbar_canvas);

        // Shared Layout

        let mut stack_layout = TheStackLayout::new(TheId::named("Tree Stack Layout"));
        stack_layout.add_canvas(project_canvas);

        // canvas.set_top(header);
        // canvas.set_right(sectionbar_canvas);
        // canvas.top_is_expanding = false;
        // canvas.set_layout(stack_layout);

        canvas.set_layout(stack_layout);

        // Multi functional footer canvas

        let mut right_canvas = TheCanvas::new();

        let mut shared_layout = TheSharedVLayout::new(TheId::named("Multi Shared"));

        let mut nodes_minimap_canvas: TheCanvas = TheCanvas::default();
        let mut nodes_minimap_shared = TheSharedVLayout::new(TheId::named("Multi Tab"));
        nodes_minimap_shared.set_shared_ratio(0.5);
        nodes_minimap_shared.set_mode(TheSharedVLayoutMode::Shared);

        let mut minimap_canvas = TheCanvas::default();
        let mut minimap = TheRenderView::new(TheId::named("MiniMap"));
        minimap.limiter_mut().set_max_width(self.width);
        minimap_canvas.set_widget(minimap);

        let mut node_settings_canvas = TheCanvas::default();
        let mut text_layout = TheTextLayout::new(TheId::named("Node Settings"));
        text_layout.limiter_mut().set_max_width(self.width);
        //text_layout.set_fixed_text_width(110);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        node_settings_canvas.set_layout(text_layout);

        let mut header = TheCanvas::new();
        let mut switchbar = TheSwitchbar::new(TheId::named("Action Header"));
        switchbar.set_text("Action Settings".to_string());
        header.set_widget(switchbar);

        nodes_minimap_canvas.set_top(header);

        nodes_minimap_shared.add_canvas(node_settings_canvas);
        nodes_minimap_shared.add_canvas(minimap_canvas);
        nodes_minimap_canvas.set_layout(nodes_minimap_shared);

        shared_layout.add_canvas(canvas);
        shared_layout.add_canvas(nodes_minimap_canvas);
        shared_layout.set_mode(TheSharedVLayoutMode::Shared);
        shared_layout.set_shared_ratio(0.5);
        shared_layout.limiter_mut().set_max_width(self.width);

        right_canvas.set_layout(shared_layout);
        right_canvas.top_is_expanding = false;

        // --

        ui.canvas.set_right(right_canvas);

        self.apply_region(ui, ctx, None, &mut Project::default());
        self.apply_character(ui, ctx, None);
        self.apply_item(ui, ctx, None);
        self.apply_tilemap(ui, ctx, None);
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
        let mut redraw = false;

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
                    if let Some(character) = project.characters.get(&id.uuid) {
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(id.uuid),
                        );
                        self.apply_character(ui, ctx, Some(character));
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Character Region Override" {
                    server_ctx.character_region_override = *index == 1;
                } else if id.name == "Item Region Override" {
                    server_ctx.item_region_override = *index == 1;
                }
            }
            TheEvent::RenderViewClicked(id, coord)
            | TheEvent::RenderViewDragged(id, coord)
            | TheEvent::RenderViewUp(id, coord) => {
                if id.name == "MiniMap" {
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();

                        // Color selected
                        if *SIDEBARMODE.read().unwrap() == SidebarMode::Palette {
                            if !matches!(event, TheEvent::RenderViewDragged(_, _)) {
                                let buffer = render_view.render_buffer_mut();
                                if let Some(col) = buffer.get_pixel(coord.x, coord.y) {
                                    let color = TheColor::from(col);

                                    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                                        widget.set_value(TheValue::Text(color.to_hex()));
                                    }

                                    if let Some(palette_picker) =
                                        ui.get_palette_picker("Palette Picker")
                                    {
                                        if project.palette[palette_picker.index()]
                                            != Some(color.clone())
                                        {
                                            let prev = project.palette.clone();
                                            palette_picker.set_color(color.clone());
                                            redraw = true;
                                            project.palette[palette_picker.index()] = Some(color);

                                            let undo = PaletteUndoAtom::Edit(
                                                prev,
                                                project.palette.clone(),
                                            );
                                            UNDOMANAGER
                                                .write()
                                                .unwrap()
                                                .add_palette_undo(undo, ctx);
                                        }

                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Soft Update Minimap"),
                                            TheValue::Empty,
                                        ));
                                    }

                                    *PALETTE.write().unwrap() = project.palette.clone();
                                    RUSTERIX.write().unwrap().assets.palette =
                                        project.palette.clone();
                                }
                            }

                            return redraw;
                        }

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let width = dim.width as f32;
                            let height = dim.height as f32;

                            if let Some(mut bbox) = region.map.bounding_box() {
                                if let Some(tbbox) = region.map.terrain.compute_bounds() {
                                    let bbox_min = Vec2::new(bbox.x, bbox.y);
                                    let bbox_max = bbox_min + Vec2::new(bbox.z, bbox.w);

                                    let new_min = bbox_min.map2(tbbox.min, f32::min);
                                    let new_max = bbox_max.map2(tbbox.max, f32::max);

                                    bbox.x = new_min.x;
                                    bbox.y = new_min.y;
                                    bbox.z = new_max.x - new_min.x;
                                    bbox.w = new_max.y - new_min.y;
                                }
                                bbox.x -= 0.5;
                                bbox.y -= 0.5;
                                bbox.z += 1.0;
                                bbox.w += 1.0;

                                let scale_x = width / bbox.z;
                                let scale_y = height / bbox.w;

                                let bbox_center_x = bbox.x + bbox.z / 2.0;
                                let bbox_center_y = bbox.y + bbox.w / 2.0;

                                let offset_x = -bbox_center_x * scale_x;
                                let offset_y = bbox_center_y * scale_y;

                                let grid_x = (coord.x as f32 - width / 2.0 - offset_x) / scale_x;
                                let grid_y = (coord.y as f32 - height / 2.0 + offset_y) / scale_y;

                                // If shift is pressed we move the look_at position
                                if ui.shift && server_ctx.editor_view_mode == EditorViewMode::FirstP
                                {
                                    region.editing_look_at_3d = Vec3::new(
                                        grid_x,
                                        region.map.terrain.sample_height_bilinear(grid_x, grid_y),
                                        grid_y,
                                    );
                                } else {
                                    // We move the camera position
                                    server_ctx.center_map_at_grid_pos(
                                        Vec2::new(width, height),
                                        Vec2::new(grid_x, grid_y),
                                        &mut region.map,
                                    );

                                    // let old_editing_pos = region.editing_position_3d;
                                    region.editing_position_3d = Vec3::new(
                                        grid_x,
                                        region.map.terrain.sample_height_bilinear(grid_x, grid_y),
                                        grid_y,
                                    );
                                    //region.editing_look_at_3d +=
                                    //    region.editing_position_3d - old_editing_pos;
                                }
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Soft Update Minimap"),
                                    TheValue::Empty,
                                ));

                                RUSTERIX.write().unwrap().set_dirty();
                            }

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
                self.show_filtered_materials(ui, ctx, project, server_ctx);
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
            TheEvent::Custom(id, _value) => {
                if id.name == "Backup Editing Position" {
                    if let Some(region) = project.get_region_ctx(server_ctx) {
                        server_ctx.editing_pos_buffer = Some(region.editing_position_3d);
                    }
                } else
                /*
                if id.name == "Update Materialpicker" {
                    self.show_filtered_materials(ui, ctx, project, server_ctx);
                    // Set the materials in the RUSTERIX assets
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix.assets.set_materials(
                        project
                            .materials
                            .iter()
                            .map(|(k, v)| (*k, v.clone()))
                            .collect(),
                    );
                    SCENEMANAGER.write().unwrap().set_tile_list(
                        rusterix.assets.tile_list.clone(),
                        rusterix.assets.tile_indices.clone(),
                    );
                    rusterix.set_dirty();
                } else
                 */
                if id.name == "Update Action List" {
                    // Update the current action params (if any)
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
                        }
                    }
                    self.show_actions(ui, ctx, project, server_ctx);

                    // self.deselect_sections_buttons(ctx, ui, "Action Section".to_string());
                    // self.select_section_button(ui, "Action Section".to_string());

                    // *SIDEBARMODE.write().unwrap() = SidebarMode::Action;

                    // ctx.ui.send(TheEvent::SetStackIndex(
                    //     self.stack_layout_id.clone(),
                    //     SidebarMode::Action as usize,
                    // ));
                } else if id.name == "Nodegraph Id Changed" {
                    if let Some(map) = project.get_map(server_ctx) {
                        if let Some(widget) = ui.get_widget("Graph Id Text") {
                            // map.shapefx_graphs.gener
                            if let Some(index) = map.shapefx_graphs.get_index_of(&id.uuid) {
                                widget.set_value(TheValue::Text(format!("({index:02})")));
                            } else {
                                widget.set_value(TheValue::Text("(--)".into()));
                            }
                        }
                    }
                } else if id.name == "Update Minimap" {
                    // Rerenders the minimap
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();
                        let buffer = render_view.render_buffer_mut();
                        buffer.resize(dim.width, dim.height);
                        if *SIDEBARMODE.read().unwrap() != SidebarMode::Shader {
                            if let Some(region) = project.get_region_ctx_mut(&server_ctx) {
                                draw_minimap(region, buffer, server_ctx, true);
                            }
                        } else {
                            crate::minimap::draw_material_minimap(buffer, project, server_ctx);
                        }
                    } else {
                    }
                } else if id.name == "Soft Update Minimap" {
                    // Uses the currently rendered minimap and only updates the
                    // camera markers
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();
                        let buffer = render_view.render_buffer_mut();
                        buffer.resize(dim.width, dim.height);
                        if *SIDEBARMODE.read().unwrap() != SidebarMode::Shader {
                            if let Some(region) = project.get_region_ctx_mut(&server_ctx) {
                                draw_minimap(region, buffer, server_ctx, false);
                            }
                        } else {
                            crate::minimap::draw_material_minimap(buffer, project, server_ctx);
                        }
                    }
                } else if id.name == "Update Tiles" {
                    self.update_tiles(ui, ctx, project);
                } else if id.name == "Show Node Settings" {
                    if let Some(tab) = ui.get_layout("Multi Tab") {
                        if let Some(tab) = tab.as_tab_layout() {
                            tab.set_index(1);
                        }
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
                    project.palette.current_index = *index;
                    if let Some(widget) = ui.get_widget("Palette Index Text") {
                        widget.set_value(TheValue::Text(format!("{index:03}")));
                    }
                    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                        if let Some(color) = &project.palette[*index as usize] {
                            widget.set_value(TheValue::Text(color.to_hex()));
                        }
                    }
                    // if let Some(widget) = ui.get_widget("Palette Name Edit") {
                    //     if let Some(color) = &project.palette[*index as usize] {
                    //         widget.set_value(TheValue::Text(color.name.clone()));
                    //     }
                    // }
                    *PALETTE.write().unwrap() = project.palette.clone();

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Soft Update Minimap"),
                        TheValue::Empty,
                    ));
                }
            }
            TheEvent::DialogValueOnClose(role, name, uuid, value) => {
                if name == "Add Shader To Library" && *role == TheDialogButtonRole::Accept {
                    let mut material = SHADEGRIDFX.read().unwrap().clone();
                    if let Some(routine) = material.get_selected_routine_mut() {
                        let mut routine_clone = routine.clone();
                        routine.id = Uuid::new_v4();
                        routine_clone.name = "shader".to_string();
                        let mut module: Module = Module::as_type(codegridfx::ModuleType::Shader);
                        if let Some(name) = value.to_string() {
                            module.name = name;
                            module.routines.insert(routine.id, routine_clone);
                            server_ctx.curr_material_id = Some(module.id);
                            project.shaders.insert(module.id, module);
                            self.show_filtered_materials(ui, ctx, project, server_ctx);
                            RUSTERIX.write().unwrap().set_dirty();

                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named_with_id(
                                    "Shader Item",
                                    server_ctx.curr_material_id.unwrap(),
                                ),
                                TheWidgetState::Selected,
                            ));
                        }
                    }
                } else if name == "Rename Region" && *role == TheDialogButtonRole::Accept {
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
                } else if name == "Rename Model" && *role == TheDialogButtonRole::Accept {
                    if let Some(model) = project.models.get_mut(uuid) {
                        model.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Shader" && *role == TheDialogButtonRole::Accept {
                    if let Some(material) = project.shaders.get_mut(uuid) {
                        material.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                }
            }
            TheEvent::ContextMenuSelected(widget_id, item_id) => {
                if item_id.name == "Sidebar Delete Character Instance" {
                    if let ContentContext::CharacterInstance(char_inst) =
                        server_ctx.curr_region_content
                    {
                        let mut name = str!("Unknown");
                        let mut template_id = None;
                        if let Some(region) = project.get_region_ctx(&server_ctx) {
                            if let Some(character) = region.characters.get(&char_inst) {
                                template_id = Some(character.character_id);
                            }
                        }
                        if let Some(template_id) = template_id {
                            if let Some(character) = project.characters.get(&template_id) {
                                name = character.name.clone();
                            }
                        }
                        open_delete_confirmation_dialog(
                            "Delete Character Instance ?",
                            format!("Permanently delete the '{name}' instance ?").as_str(),
                            char_inst,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Sidebar Delete Item Instance" {
                    if let ContentContext::ItemInstance(item_inst) = server_ctx.curr_region_content
                    {
                        let mut name = str!("Unknown");
                        let mut template_id = None;
                        if let Some(region) = project.get_region_ctx(&server_ctx) {
                            if let Some(item) = region.items.get(&item_inst) {
                                template_id = Some(item.item_id);
                            }
                        }
                        if let Some(template_id) = template_id {
                            if let Some(item) = project.items.get(&template_id) {
                                name = item.name.clone();
                            }
                        }
                        open_delete_confirmation_dialog(
                            "Delete Item Instance ?",
                            &format!("Permanently delete the '{name}' instance ?"),
                            item_inst,
                            ui,
                            ctx,
                        );
                    }
                }
                /*else if item_id.name == "Sidebar Delete Area" {
                    if let Some(region) = project.get_region(&server_ctx.curr_region) {
                        if let Some(area_id) = server_ctx.curr_area {
                            for area in region.areas.values() {
                                if area.id == area_id {
                                    open_delete_confirmation_dialog(
                                        "Delete Area ?",
                                        format!("Permanently delete area '{}' ?", area.name)
                                            .as_str(),
                                        area_id,
                                        ui,
                                        ctx,
                                    );
                                }
                            }
                        }
                    }
                } */
                else if item_id.name == "Add Image" {
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
                } else if item_id.name == "Rename Shader" {
                    if let Some(material) = project.shaders.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Shader",
                            "Shader Name",
                            &material.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Duplicate Shader" {
                    if let Some(mut material) = project.shaders.get(&widget_id.uuid).cloned() {
                        material.name = format!("Duplicate of {}", material.name);
                        material.id = Uuid::new_v4();
                        project.shaders.insert(material.id, material);
                        self.show_filtered_materials(ui, ctx, project, server_ctx);
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
            TheEvent::ValueChanged(id, value) => {
                if id.name.starts_with("Region Item Name Edit") {
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
                } else if let Some(action_id) = server_ctx.curr_action_id
                    && id.name.starts_with("action")
                {
                    if let Some(action) =
                        ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                    {
                        if action.handle_event(event) {
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
                            if project.palette[palette_picker.index()] != Some(color.clone()) {
                                let prev = project.palette.clone();

                                palette_picker.set_color(color.clone());
                                redraw = true;
                                project.palette[palette_picker.index()] = Some(color.clone());
                                let undo = PaletteUndoAtom::Edit(prev, project.palette.clone());
                                UNDOMANAGER.write().unwrap().add_palette_undo(undo, ctx);

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Soft Update Minimap"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }
                    *PALETTE.write().unwrap() = project.palette.clone();
                    RUSTERIX.write().unwrap().assets.palette = project.palette.clone();
                }
                // Change the size of the tilemap grid
                else if id.name == "Tilemap Grid Edit" {
                    if let Some(tilemap_uuid) = &self.curr_tilemap_uuid {
                        if let Some(tilemap) = project.get_tilemap(*tilemap_uuid) {
                            if let Some(size) = value.to_i32() {
                                tilemap.grid_size = size;
                                self.apply_tilemap(ui, ctx, Some(tilemap));
                            }
                        }
                    }
                } else if id.name == "Tilemap Filter Edit" || id.name == "Tilemap Filter Role" {
                    if let Some(id) = self.curr_tilemap_uuid {
                        self.show_filtered_tiles(ui, ctx, project.get_tilemap(id).as_deref())
                    }
                } else if id.name == "Shader Filter Edit" {
                    self.show_filtered_materials(ui, ctx, project, server_ctx);
                } else if id.name == "Tilemap Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(layout) = ui.get_rgba_layout("Tilemap Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                        if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                            if let Some(tilemap) = project.get_tilemap(curr_tilemap_uuid) {
                                tilemap.zoom = v;
                            }
                        }
                    }
                } else if id.name == "Region Content Filter Edit"
                    || id.name == "Region Content Dropdown"
                {
                    self.apply_region(ui, ctx, Some(server_ctx.curr_region), project);
                }
            }
            // Tiles Add
            TheEvent::FileRequesterResult(id, paths) => {
                if id.name == "Tilemap Add" || id.name == "Add Image" {
                    for p in paths {
                        ctx.ui.decode_image(id.clone(), p.clone());
                    }
                } else if id.name == "Add Font" {
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
                } else if id.name == "Tilemap Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let tilemap: Tilemap =
                            serde_json::from_str(&contents).unwrap_or(Tilemap::default());

                        if project.get_tilemap(tilemap.id).is_none() {
                            if let Some(layout) = ui
                                .canvas
                                .get_layout(Some(&"Tilemap List".to_string()), None)
                            {
                                if let Some(list_layout) = layout.as_list_layout() {
                                    let mut item = TheListItem::new(TheId::named_with_id(
                                        "Tilemap Item",
                                        tilemap.id,
                                    ));
                                    item.set_text(tilemap.name.clone());
                                    item.set_state(TheWidgetState::Selected);
                                    item.set_context_menu(Some(TheContextMenu {
                                        items: vec![
                                            TheContextMenuItem::new(
                                                "Rename Tileset...".to_string(),
                                                TheId::named("Rename Tileset"),
                                            ),
                                            TheContextMenuItem::new(
                                                "Add Colors to Palette".to_string(),
                                                TheId::named("Add Tileset Colors"),
                                            ),
                                        ],
                                        ..Default::default()
                                    }));
                                    list_layout.deselect_all();
                                    let id = item.id().clone();
                                    list_layout.add_item(item, ctx);
                                    list_layout.select_item(id.uuid, ctx, true);

                                    redraw = true;
                                }
                            }
                            project.add_tilemap(tilemap);
                            self.update_tiles(ui, ctx, project);

                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Tilemap loaded successfully.".to_string(),
                            ))
                        } else {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Tilemap already exists.".to_string(),
                            ))
                        }
                    }
                } else if id.name == "Tilemap Export" {
                    if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                        if let Some(tilemap) = project.get_tilemap(curr_tilemap_uuid) {
                            for p in paths {
                                let json = serde_json::to_string(&tilemap);
                                if let Ok(json) = json {
                                    if std::fs::write(p, json).is_ok() {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Tilemap saved successfully.".to_string(),
                                        ))
                                    } else {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Unable to save Tilemap!".to_string(),
                                        ))
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "Shader Import" {
                    for p in paths {
                        let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                        let mut module: Module =
                            serde_json::from_str(&contents).unwrap_or(Module::default());
                        module.id = Uuid::new_v4();
                        if module.name.is_empty() {
                            module.name = "Unnamed".into();
                        }

                        project.shaders.insert(module.id, module);
                        self.show_filtered_materials(ui, ctx, project, server_ctx);

                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Shader loaded successfully.".to_string(),
                        ))
                    }
                } else if id.name == "Shader Export" {
                    if let Some(curr_material_id) = server_ctx.curr_material_id {
                        if let Some(material) = project.shaders.get(&curr_material_id) {
                            for p in paths {
                                let json = serde_json::to_string(&material);
                                if let Ok(json) = json {
                                    if std::fs::write(p, json).is_ok() {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Shader saved successfully.".to_string(),
                                        ))
                                    } else {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Unable to save Material!".to_string(),
                                        ))
                                    }
                                }
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
                } else if id.name == "Tilemap Add" {
                    let mut tilemap = Tilemap::new();
                    tilemap.name = name.clone();
                    tilemap.id = Uuid::new_v4();
                    tilemap.buffer = buffer.clone();

                    if let Some(layout) = ui
                        .canvas
                        .get_layout(Some(&"Tilemap List".to_string()), None)
                    {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item =
                                TheListItem::new(TheId::named_with_id("Tilemap Item", tilemap.id));
                            item.set_text(name.clone());
                            item.set_state(TheWidgetState::Selected);
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![
                                    TheContextMenuItem::new(
                                        "Rename Tileset...".to_string(),
                                        TheId::named("Rename Tileset"),
                                    ),
                                    TheContextMenuItem::new(
                                        "Add Colors to Palette".to_string(),
                                        TheId::named("Add Tileset Colors"),
                                    ),
                                ],
                                ..Default::default()
                            }));
                            list_layout.deselect_all();
                            let id = item.id().clone();
                            list_layout.add_item(item, ctx);
                            ctx.ui
                                .send_widget_state_changed(&id, TheWidgetState::Selected);

                            redraw = true;
                        }
                    }
                    project.add_tilemap(tilemap);
                }
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                let action_list = ACTIONLIST.write().unwrap();
                let mut needs_scene_redraw: bool = false;
                for action in &action_list.actions {
                    if let Some(accel) = action.accel() {
                        if accel.matches(ui.shift, ui.ctrl, ui.alt, ui.logo, *c) {
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                if action.is_applicable(map, ctx, server_ctx) {
                                    println!("{}", action.id().name);
                                    needs_scene_redraw =
                                        self.apply_action(action, map, ui, ctx, server_ctx);
                                }
                            }
                            action.apply_project(project, ui, ctx, server_ctx);
                        }
                    }
                }
                if needs_scene_redraw {
                    crate::utils::scenemanager_render_map(project, server_ctx);
                    TOOLLIST
                        .write()
                        .unwrap()
                        .update_geometry_overlay_3d(project, server_ctx);
                }
            }
            TheEvent::StateChanged(id, state) => {
                // Iterate actions first
                if let Some(action) = ACTIONLIST.write().unwrap().get_action_by_id_mut(id.uuid) {
                    server_ctx.curr_action_id = Some(action.id().uuid);

                    if let Some(layout) = ui.get_text_layout("Node Settings") {
                        if let Some(map) = project.get_map_mut(&server_ctx) {
                            action.load_params(map);
                        }
                        action.load_params_project(project, server_ctx);

                        let nodeui = action.params();
                        nodeui.apply_to_text_layout(layout);
                        ctx.ui.relayout = true;

                        let mut text = action.id().name.clone();
                        if let Some(accel) = action.accel() {
                            text += &format!(" ({})", accel.description());
                        }

                        ui.set_widget_value("Action Header", ctx, TheValue::Text(text));
                    }
                } else if id.name == "Action Apply" {
                    if let Some(action_id) = server_ctx.curr_action_id {
                        if let Some(action) = ACTIONLIST.read().unwrap().get_action_by_id(action_id)
                        {
                            // let map = if server_ctx.editor_view_mode == EditorViewMode::D2 {
                            //     project.get_map_mut(&server_ctx)
                            // } else {
                            //     if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                            //         Some(&mut region.map)
                            //     } else {
                            //         None
                            //     }
                            // };

                            let mut needs_scene_redraw = false;
                            if let Some(map) = project.get_map_mut(&server_ctx) {
                                needs_scene_redraw =
                                    self.apply_action(action, map, ui, ctx, server_ctx);
                            }
                            action.apply_project(project, ui, ctx, server_ctx);

                            if needs_scene_redraw {
                                crate::utils::scenemanager_render_map(project, server_ctx);
                                TOOLLIST
                                    .write()
                                    .unwrap()
                                    .update_geometry_overlay_3d(project, server_ctx);
                            }
                        }
                    }
                } else if id.name == "Shader Item" {
                    let material_id = id.uuid;
                    server_ctx.curr_material_id = Some(material_id);
                    if let Some(material) = project.shaders.get(&id.uuid) {
                        let prev = SHADEGRIDFX.read().unwrap().clone();

                        CODEEDITOR
                            .write()
                            .unwrap()
                            .set_shader_material(ui, ctx, material);

                        let atom = MaterialUndoAtom::ShaderEdit(prev, material.clone());
                        UNDOMANAGER.write().unwrap().add_material_undo(atom, ctx);
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimap"),
                        TheValue::Empty,
                    ));
                } else if id.name == "Palette Clear" {
                    let prev = project.palette.clone();
                    project.palette.clear();
                    if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                        let index = palette_picker.index();

                        palette_picker.set_palette(project.palette.clone());
                        if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                            if let Some(color) = &project.palette[index] {
                                widget.set_value(TheValue::Text(color.to_hex()));
                            }
                        }
                    }
                    redraw = true;

                    let undo = PaletteUndoAtom::Edit(prev, project.palette.clone());
                    UNDOMANAGER.write().unwrap().add_palette_undo(undo, ctx);
                } else if id.name == "Palette Import" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "Palette (*.txt)".into(),
                            vec!["txt".to_string(), "TXT".to_string()],
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
                    let atom = ProjectUndoAtom::AddRegion(Region::default());
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
                } else if id.name == "Project Remove" {
                    if server_ctx.pc.is_region() {
                        // Remove Region
                        let mut region = Region::default();
                        if let Some(r) = project.get_region_ctx(server_ctx) {
                            region = r.clone();
                        }

                        if let Some(index) = project.regions.iter().position(|r| r.id == region.id)
                        {
                            let atom = ProjectUndoAtom::RemoveRegion(index, region);
                            atom.redo(project, ui, ctx, server_ctx);
                            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
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
                        }
                    }
                } else if id.name == "Region Item" {
                    // Add Region
                    server_ctx.editing_pos_buffer = None;
                    server_ctx.curr_region = id.references;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Region(id.references),
                    );

                    self.apply_region(ui, ctx, Some(id.references), project);
                    redraw = true;
                } else if id.name == "Character Add" {
                    // Add Character
                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        let mut character = Character::default();

                        if let Some(bytes) = crate::Embedded::get("python/basecharacter.py") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                character.source = source.to_string();
                            }
                        }

                        if let Some(bytes) = crate::Embedded::get("toml/character.toml") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                character.data = source.to_string();
                            }
                        }

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Character Item", character.id));
                        item.set_text(character.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Character...".to_string(),
                                TheId::named("Rename Character"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_character(ui, ctx, Some(&character));
                        project.add_character(character);
                    }
                } else if id.name == "Character Item" {
                    if let Some(c) = project.characters.get(&id.references) {
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
                        self.apply_character(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Character Item Name Edit" {
                    if let Some(c) = project.characters.get(&id.references) {
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
                        self.apply_character(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Character Item Code Edit" {
                    if let Some(c) = project.characters.get(&id.references) {
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
                        self.apply_character(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Character Item Data Edit" {
                    if let Some(c) = project.characters.get(&id.references) {
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
                        self.apply_character(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Item Item" {
                    if let Some(c) = project.items.get(&id.uuid) {
                        server_ctx.curr_item = ContentContext::ItemTemplate(id.uuid);
                        server_ctx.cc = ContentContext::ItemTemplate(id.uuid);
                        self.apply_item(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Item Add" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        let mut item_template = Item::default();

                        if let Some(bytes) = crate::Embedded::get("python/baseitem.py") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                item_template.source = source.to_string();
                            }
                        }

                        if let Some(bytes) = crate::Embedded::get("toml/item.toml") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                item_template.data = source.to_string();
                            }
                        }

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Item Item", item_template.id));
                        item.set_text(item_template.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Item...".to_string(),
                                TheId::named("Rename Item"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_item(ui, ctx, Some(&item_template));
                        project.add_item(item_template);
                    }
                } else if id.name == "Item Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_item(&selected.uuid);
                            self.apply_item(ui, ctx, None);
                        }
                    }
                } else if id.name == "Shader Add" {
                    let mut module: Module = Module::as_type(codegridfx::ModuleType::Shader);
                    module.update_routines();
                    module.name = "New Shader".into();
                    server_ctx.curr_material_id = Some(module.id);
                    project.shaders.insert(module.id, module);
                    self.show_filtered_materials(ui, ctx, project, server_ctx);
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::StateChanged(
                        TheId::named_with_id("Shader Item", server_ctx.curr_material_id.unwrap()),
                        TheWidgetState::Selected,
                    ));
                } else if id.name == "Shader Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Shader List") {
                        if let Some(curr_material) = server_ctx.curr_material_id {
                            project.shaders.shift_remove(&curr_material);
                            list_layout.select_first_item(ctx);
                        }
                    }
                    self.show_filtered_materials(ui, ctx, project, &server_ctx);
                } else if id.name == "Shader Import" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "Eldiron Material".into(),
                            vec!["eldiron_shader".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                }
                if id.name == "Shader Export" {
                    if let Some(curr_tilemap_uuid) = server_ctx.curr_material_id {
                        ctx.ui.save_file_requester(
                            TheId::named_with_id(id.name.as_str(), curr_tilemap_uuid),
                            "Save".into(),
                            TheFileExtension::new(
                                "Eldiron Material".into(),
                                vec!["eldiron_shader".to_string()],
                            ),
                        );
                        ctx.ui
                            .set_widget_state("Save As".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    }
                }
                /*else if id.name == "Module Add" {
                    if let Some(list_layout) = ui.get_list_layout("Module List") {
                        let bundle = TheCodeBundle::new();

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Module Item", bundle.id));
                        item.set_text(bundle.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Module...".to_string(),
                                TheId::named("Rename Module"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        project.add_code(bundle);
                    }
                }*/
                /*else if id.name == "Module Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_code(&selected.uuid);
                        }
                    }
                }
                else if id.name == "Module Item" {
                    if let Some(_c) = project.codes.get(&id.uuid) {
                        redraw = true;
                    }
                }*/
                // Tilemap Item Handling
                else if id.name == "Tilemap Add" {
                    ctx.ui.open_file_requester(
                        TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                        "Open".into(),
                        TheFileExtension::new(
                            "PNG Image".into(),
                            vec!["png".to_string(), "PNG".to_string()],
                        ),
                    );
                    ctx.ui
                        .set_widget_state("Tilemap Add".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                    redraw = true;
                } else if id.name == "Tilemap Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_tilemap(selected);
                            self.apply_tilemap(ui, ctx, None);
                            self.curr_tilemap_uuid = None;
                        }
                    }
                } else if id.name == "Tilemap Item" {
                    // Display the tilemap editor
                    if let Some(t) = project.get_tilemap(id.uuid) {
                        self.curr_tilemap_uuid = Some(t.id);

                        TILEMAPEDITOR.write().unwrap().set_tilemap(t, ui, ctx);
                        self.apply_tilemap(ui, ctx, Some(t));
                        ctx.ui.relayout = true;

                        if !self.startup {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Set Tool"),
                                TheValue::Text(str!("Tileset Tool")),
                            ));
                        }
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
                /*else if id.name == "Widget Add" {
                    let mut widget = Widget {
                        x: 0.0,
                        y: 0.0,
                        width: 10.0,
                        height: 10.0,
                        ..Default::default()
                    };

                    widget.bundle.id = widget.id;

                    let init = TheCodeGrid {
                        name: "init".into(),
                        ..Default::default()
                    };

                    widget.bundle.insert_grid(init);

                    let mut draw = TheCodeGrid {
                        name: "draw".into(),
                        ..Default::default()
                    };

                    draw.insert_atom(
                        (0, 0),
                        TheCodeAtom::ExternalCall(
                            "Fill".to_string(),
                            "Fills the widget with the given color.".to_string(),
                            vec![str!("Color")],
                            vec![TheValue::ColorObject(TheColor::default())],
                            None,
                        ),
                    );

                    draw.insert_atom(
                        (2, 0),
                        TheCodeAtom::Value(TheValue::ColorObject(TheColor::default())),
                    );

                    widget.bundle.insert_grid(draw);

                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(list) = ui.get_list_layout("Screen Content List") {
                            let mut list_item = TheListItem::new(TheId::named_with_id(
                                "Screen Content List Item",
                                widget.id,
                            ));
                            list_item.set_text(widget.name.clone());
                            list_item.set_state(TheWidgetState::Selected);
                            list_item.add_value_column(100, TheValue::Text("Widget".to_string()));

                            list_item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Rename Widget...".to_string(),
                                    TheId::named("Rename Widget"),
                                )],
                                ..Default::default()
                            }));

                            list.deselect_all();
                            list.add_item(list_item, ctx);
                            list.select_item(widget.id, ctx, true);
                        }
                        screen.widget_list.push(widget);
                        client.update_screen(screen);
                        self.apply_screen(ui, ctx, Some(screen));
                        redraw = true;
                    }
                } else if id.name == "Widget Remove" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.get_widget(&widget_id) {
                                open_delete_confirmation_dialog(
                                    "Delete Widget ?",
                                    format!("Permanently delete '{}' ?", widget.name).as_str(),
                                    widget.id,
                                    ui,
                                    ctx,
                                );
                            }
                        }
                    }
                }*/
                // Section Buttons
                else if id.name == "Region Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Regions".to_string()));
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Region;
                    server_ctx.set_map_context(MapContext::Region);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Region;
                    RUSTERIX.write().unwrap().set_dirty();

                    server_ctx.cc = server_ctx.curr_region_content;
                    set_code(ui, ctx, project, server_ctx);

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Region as usize,
                    ));
                    redraw = true;
                } else if id.name == "Character Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Character".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Character;
                    server_ctx.set_map_context(MapContext::Character);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Region;
                    RUSTERIX.write().unwrap().set_dirty();

                    server_ctx.cc = server_ctx.curr_character;
                    set_code(ui, ctx, project, server_ctx);

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Character as usize,
                    ));
                    redraw = true;
                } else if id.name == "Item Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Items".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Item;
                    server_ctx.set_map_context(MapContext::Item);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Region;
                    RUSTERIX.write().unwrap().set_dirty();

                    server_ctx.cc = server_ctx.curr_item;
                    set_code(ui, ctx, project, server_ctx);

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Item as usize,
                    ));
                    redraw = true;
                } else if id.name == "Tileset Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Tilesets".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Tilemap;
                    server_ctx.set_map_context(MapContext::Region);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Region;
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Tilemap as usize,
                    ));
                    redraw = true;
                } else if id.name == "Module Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Modules".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Module List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Module;
                    server_ctx.set_map_context(MapContext::Region);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Region;
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Module as usize,
                    ));
                    redraw = true;
                } else if id.name == "Screen Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Screens".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Screen List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Screen;
                    server_ctx.set_map_context(MapContext::Screen);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Screen;
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Screen as usize,
                    ));
                    redraw = true;
                } else if id.name == "Asset Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Assets".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Asset List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Asset;
                    server_ctx.set_map_context(MapContext::Region);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Region;
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Asset as usize,
                    ));
                    redraw = true;
                } else if id.name == "Action Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Actions".to_string()));
                    }

                    self.show_actions(ui, ctx, project, server_ctx);
                    *SIDEBARMODE.write().unwrap() = SidebarMode::Action;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Action as usize,
                    ));
                    redraw = true;
                } else if id.name == "Shader Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Shaders".to_string()));
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Shader;
                    server_ctx.set_map_context(MapContext::Shader);
                    UNDOMANAGER.write().unwrap().context = UndoManagerContext::Material;
                    RUSTERIX.write().unwrap().set_dirty();

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Shader as usize,
                    ));
                    redraw = true;
                }
                /*else if id.name == "Node Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Node UI".to_string()));
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Node;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Node as usize,
                    ));

                    redraw = true;
                } */
                else if id.name == "Debug Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Log Output".to_string()));
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Debug;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Debug as usize,
                    ));

                    redraw = true;
                } else if id.name == "Palette Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ctx, ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Palette".to_string()));
                    }

                    *SIDEBARMODE.write().unwrap() = SidebarMode::Palette;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Palette as usize,
                    ));

                    redraw = true;
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
        // If no colors we load the duel palette: https://lospec.com/palette-list/duel
        if project.palette.is_empty() {
            if let Some(bytes) = crate::Embedded::get("duel.txt") {
                if let Ok(txt) = std::str::from_utf8(bytes.data.as_ref()) {
                    project.palette.load_from_txt(txt.to_string());
                }
            }
        }

        self.apply_regions(ui, ctx, server_ctx, project);
        self.apply_characters(ui, ctx, server_ctx, project);

        if let Some(list_layout) = ui.get_list_layout("Region List") {
            list_layout.clear();
            for region in &project.regions {
                let mut item = TheListItem::new(TheId::named_with_id("Region Item", region.id));
                item.set_text(region.name.clone());
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![
                        TheContextMenuItem::new(
                            "Rename Region...".to_string(),
                            TheId::named("Rename Region"),
                        ),
                        // TheContextMenuItem::new(
                        //     "Copy Prerendered...".to_string(),
                        //     TheId::named("Copy Prerendered"),
                        // ),
                    ],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }
        self.apply_character(ui, ctx, None);
        if let Some(list_layout) = ui.get_list_layout("Character List") {
            list_layout.clear();
            let list = project.sorted_character_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Character Item", id));
                item.set_text(name);
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Character...".to_string(),
                        TheId::named("Rename Character"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }
        self.apply_item(ui, ctx, None);
        if let Some(list_layout) = ui.get_list_layout("Item List") {
            list_layout.clear();
            let list = project.sorted_item_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Item Item", id));
                item.set_text(name);
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Item...".to_string(),
                        TheId::named("Rename Item"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }
        self.apply_tilemap(ui, ctx, None);
        if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
            list_layout.clear();
            for tilemap in &project.tilemaps {
                let mut item = TheListItem::new(TheId::named_with_id("Tilemap Item", tilemap.id));
                item.set_text(tilemap.name.clone());
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![
                        TheContextMenuItem::new(
                            "Rename Tileset...".to_string(),
                            TheId::named("Rename Tileset"),
                        ),
                        TheContextMenuItem::new(
                            "Add Colors to Palette".to_string(),
                            TheId::named("Add Tileset Colors"),
                        ),
                    ],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }
        /*
        if let Some(list_layout) = ui.get_list_layout("Module List") {
            list_layout.clear();
            let list = project.sorted_code_list();
            for (id, name) in list {
                let mut item = TheListItem::new(TheId::named_with_id("Module Item", id));
                item.set_text(name);
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Module...".to_string(),
                        TheId::named("Rename Module"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }*/
        self.apply_screen(ui, ctx, None);
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
            palette_picker.set_palette(project.palette.clone());
            let index = palette_picker.index();

            if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                if let Some(color) = &project.palette[index] {
                    widget.set_value(TheValue::Text(color.to_hex()));
                }
            }
        }

        ui.select_first_list_item("Region List", ctx);
        ui.select_first_list_item("Character List", ctx);
        ui.select_first_list_item("Item List", ctx);
        ui.select_first_list_item("Tilemap List", ctx);
        ui.select_first_list_item("Module List", ctx);
        ui.select_first_list_item("Screen List", ctx);
        ui.select_first_list_item("Asset List", ctx);

        ui.set_widget_value("ConfigEdit", ctx, TheValue::Text(project.config.clone()));
        if let Ok(toml) = project.config.parse::<Table>() {
            *CONFIG.write().unwrap() = toml;
        }
        CONFIGEDITOR.write().unwrap().read_defaults();
        RUSTERIX.write().unwrap().assets.palette = project.palette.clone();

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Materialpicker"),
            TheValue::Empty,
        ));

        // Set the current material
        let selected_material = if project.shaders.is_empty() {
            None
        } else if let Some((id, _)) = project.shaders.get_index(0) {
            Some(*id)
        } else {
            None
        };

        server_ctx.curr_material_id = selected_material;

        self.show_actions(ui, ctx, project, server_ctx);
        self.show_filtered_materials(ui, ctx, project, server_ctx);
        self.update_tiles(ui, ctx, project);

        TOOLLIST.write().unwrap().get_current_tool().tool_event(
            ToolEvent::Activate,
            ui,
            ctx,
            project,
            server_ctx,
        );
    }

    /// Apply the given character to the UI
    pub fn apply_character(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _character: Option<&Character>,
    ) {
        /*
        ui.set_widget_disabled_state("Character Remove", ctx, character.is_none());

        if let Some(character) = character {
            ui.set_widget_value("CodeEdit", ctx, TheValue::Text(character.source.clone()));
            ui.set_widget_value("DataEdit", ctx, TheValue::Text(character.data.clone()));

            CODEEDITOR
                .write()
                .unwrap()
                .set_module_character(ui, ctx, character);
        } else {
            CODEEDITOR.write().unwrap().clear_module(ui, ctx);
        }

        ctx.ui.relayout = true;
        */
    }

    /// Apply the given item to the UI
    pub fn apply_item(&mut self, ui: &mut TheUI, ctx: &mut TheContext, item: Option<&Item>) {
        ui.set_widget_disabled_state("Item Remove", ctx, item.is_none());

        if let Some(item) = item {
            // if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            //     if let Some(canvas) = stack_layout.canvas_at_mut(2) {
            //         canvas.set_bottom(item_list_canvas);
            //     }
            // }
            //
            ui.set_widget_value("CodeEdit", ctx, TheValue::Text(item.source.clone()));
            ui.set_widget_value("DataEdit", ctx, TheValue::Text(item.data.clone()));

            CODEEDITOR.write().unwrap().set_module_item(ui, ctx, item);
        } else if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            if let Some(canvas) = stack_layout.canvas_at_mut(2) {
                let mut empty = TheCanvas::new();
                empty.set_layout(TheVLayout::new(TheId::empty()));
                canvas.set_bottom(empty);
            }
            CODEEDITOR.write().unwrap().clear_module(ui, ctx);
        }

        ctx.ui.relayout = true;
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

    /// Apply the current regions to the tree.
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

    /// Apply the given item to the UI
    pub fn apply_region(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        region_id: Option<Uuid>,
        project: &mut Project,
    ) {
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
        }

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

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));

        RUSTERIX.write().unwrap().set_dirty();

        if let Some(region_id) = region_id {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render SceneManager Map"),
                TheValue::Empty,
            ));
            if let Some(region) = project.get_region(&region_id) {
                ui.set_widget_value(
                    "RegionConfigEdit",
                    ctx,
                    TheValue::Text(region.config.clone()),
                );
            }
        }
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
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Region Tracer Samples Edit".to_string()), None)
        {
            if let Some(region) = region {
                widget.set_value(TheValue::Text(region.pathtracer_samples.to_string()));
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

    /// Apply the given tilemap item to the UI
    pub fn apply_tilemap(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        tilemap: Option<&Tilemap>,
    ) {
        ui.set_widget_disabled_state("Tilemap Remove", ctx, tilemap.is_none());
        ui.set_widget_disabled_state("Tilemap Export", ctx, tilemap.is_none());

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Tilemap Grid Edit".to_string()), None)
        {
            if let Some(tilemap) = tilemap {
                widget.set_value(TheValue::Text(tilemap.grid_size.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(layout) = ui.get_rgba_layout("Tilemap Editor") {
            if let Some(rgba) = layout.rgba_view_mut().as_rgba_view() {
                if let Some(tilemap) = tilemap {
                    //rgba.set_zoom(tilemap.zoom);
                    rgba.set_grid(Some(tilemap.grid_size));
                }
            }
        }

        self.show_filtered_tiles(ui, ctx, tilemap);
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

    /// Shows the filtered actions for the current selection.
    pub fn show_actions(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Action List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();

                let actions = ACTIONLIST.read().unwrap();
                let mut found_current = false;

                let mut camera_actions: Vec<TheListItem> = vec![];
                let mut editor_actions: Vec<TheListItem> = vec![];
                let mut dock_actions: Vec<TheListItem> = vec![];

                if let Some(map) = project.get_map(server_ctx) {
                    for action in &actions.actions {
                        if action.is_applicable(map, ctx, server_ctx) {
                            let mut item = TheListItem::new(action.id().clone());
                            item.set_text(action.id().name.clone());

                            // let mut accel_text = String::new();
                            // if let Some(accel) = action.accel() {
                            //     accel_text = accel.description();
                            // }
                            // item.add_value_column(110, TheValue::Text(accel_text));
                            item.set_status_text(action.info());
                            item.set_background_color(TheColor::from(action.role().to_color()));

                            if Some(action.id().uuid) == server_ctx.curr_action_id {
                                found_current = true;
                                item.set_state(TheWidgetState::Selected);
                            }

                            if action.role() == ActionRole::Camera {
                                camera_actions.push(item);
                            } else if action.role() == ActionRole::Editor {
                                editor_actions.push(item);
                            } else {
                                dock_actions.push(item);
                            }
                        }
                    }
                }

                for item in camera_actions {
                    list_layout.add_item(item, ctx);
                }
                for item in editor_actions {
                    list_layout.add_item(item, ctx);
                }
                for item in dock_actions {
                    list_layout.add_item(item, ctx);
                }

                if !found_current {
                    server_ctx.curr_action_id = None;
                }
            }
        }

        if let Some(layout) = ui.get_text_layout("Action Settings") {
            layout.clear();

            if let Some(action_id) = server_ctx.curr_action_id {
                if let Some(action) = ACTIONLIST.read().unwrap().get_action_by_id(action_id) {
                    let nodeui = action.params();
                    nodeui.apply_to_text_layout(layout);
                    ctx.ui.relayout = true;

                    let mut text = action.id().name.clone();
                    if let Some(accel) = action.accel() {
                        text += &format!(" ({})", accel.description());
                    }

                    ui.set_widget_value("Action Header", ctx, TheValue::Text(text));
                }
            }
        }
    }

    /// Shows the filtered materials of the project.
    pub fn show_filtered_materials(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Shader Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let _filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Shader Filter Role".to_string()), None)
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

        if let Some(layout) = ui.canvas.get_layout(Some(&"Shader List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();

                for material in project.shaders.values() {
                    if filter_text.is_empty() || material.name.to_lowercase().contains(&filter_text)
                    //&& (filter_role == 0
                    //    || tile.role == TileRole::from_index(filter_role as u8 - 1).unwrap())
                    {
                        let mut item =
                            TheListItem::new(TheId::named_with_id("Shader Item", material.id));
                        item.set_text(material.name.clone());
                        //let sub_text = format!("Index: {index}");
                        // item.set_sub_text(sub_text);
                        // item.set_size(42);
                        if Some(material.id) == server_ctx.curr_material_id {
                            item.set_state(TheWidgetState::Selected);
                        }

                        /*
                        if let Some(Value::Texture(texture)) = material.properties.get("Shader") {
                            let resized = texture.resized(36, 36);
                            let rgba = TheRGBABuffer::from(
                                resized.data.clone(),
                                resized.width as u32,
                                resized.height as u32,
                            );
                            item.set_icon(rgba);
                        }*/

                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![
                                TheContextMenuItem::new(
                                    "Rename Shader...".to_string(),
                                    TheId::named("Rename Shader"),
                                ),
                                TheContextMenuItem::new(
                                    "Duplicate Shader".to_string(),
                                    TheId::named("Duplicate Shader"),
                                ),
                            ],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                    }
                }
            }
        }
        //ui.select_first_list_item("Shader List", ctx);
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
                item.set_text("Server has been started".to_string());
                item.add_value_column(100, TheValue::Text("Status".to_string()));
                list_layout.add_item(item, ctx);
            }
        }
    }

    /*
    /// Adds the given debug messages to the debug list.
    pub fn add_debug_messages(
        &self,
        messages: Vec<TheDebugMessage>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if let Some(layout) = ui.canvas.get_layout(Some(&"Debug List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                for message in messages {
                    let mut item = TheListItem::new(TheId::named("Debug Item"));
                    item.add_value_column(100, TheValue::Text(message.entity));
                    item.set_text(message.message);
                    list_layout.add_item(item, ctx);
                }
            }
        }
    }*/

    pub fn apply_action(
        &self,
        action: &Box<dyn Action>,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let Some(undo_atom) = action.apply(map, ui, ctx, server_ctx) {
            if server_ctx.get_map_context() == MapContext::Region {
                UNDOMANAGER.write().unwrap().add_region_undo(
                    &server_ctx.curr_region,
                    undo_atom,
                    ctx,
                );
                if server_ctx.editor_view_mode == EditorViewMode::D2
                    && server_ctx.profile_view.is_some()
                {
                } else {
                    map.update_surfaces();
                    return true;
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            } else if server_ctx.get_map_context() == MapContext::Character {
                if let Some(character_undo_atom) = undo_atom.to_character_atom() {
                    UNDOMANAGER
                        .write()
                        .unwrap()
                        .add_character_undo(character_undo_atom, ctx);

                    /*
                    NODEEDITOR.write().unwrap().create_shape_preview(
                        map,
                        &RUSTERIX.read().unwrap().assets,
                    );*/
                }
            } else if server_ctx.get_map_context() == MapContext::Item {
                if let Some(item_undo_atom) = undo_atom.to_item_atom() {
                    UNDOMANAGER
                        .write()
                        .unwrap()
                        .add_item_undo(item_undo_atom, ctx);

                    /*
                    NODEEDITOR
                        .write()
                        .unwrap()
                        .create_shape_preview(
                            map,
                            &RUSTERIX.read().unwrap().assets,
                        );
                    */
                }
            } else if server_ctx.get_map_context() == MapContext::Screen {
                if let Some(screen_undo_atom) = undo_atom.to_screen_atom() {
                    UNDOMANAGER
                        .write()
                        .unwrap()
                        .add_screen_undo(screen_undo_atom, ctx);
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
            }
        }
        false
    }

    /// Tilemaps in the project have been updated, propagate the change to all relevant parties.
    pub fn update_tiles(&mut self, _ui: &mut TheUI, ctx: &mut TheContext, project: &mut Project) {
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.set_tiles(project.tiles.clone(), true);
        SCENEMANAGER.write().unwrap().set_tile_list(
            rusterix.assets.tile_list.clone(),
            rusterix.assets.tile_indices.clone(),
        );

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
    }
}
