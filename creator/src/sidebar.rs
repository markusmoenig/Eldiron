use crate::editor::{CODEEDITOR, MAPRENDER, SIDEBARMODE, TILEDRAWER, TILEMAPEDITOR, UNDOMANAGER};
use crate::minimap::draw_minimap;
use crate::prelude::*;

#[derive(PartialEq, Debug)]
pub enum SidebarMode {
    Region,
    Character,
    Item,
    Tilemap,
    Module,
    Screen,
    Asset,
    Model,
    Material,
    Node,
    Debug,
    Palette,
}

pub struct Sidebar {
    pub width: i32,

    stack_layout_id: TheId,

    curr_tilemap_uuid: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    pub fn new() -> Self {
        Self {
            width: 380,

            stack_layout_id: TheId::empty(),
            curr_tilemap_uuid: None,
        }
    }

    pub fn init_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
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
        character_sectionbar_button.set_text("Character".to_string());
        character_sectionbar_button.set_status_text(
            "Edit and manage the characers (and their behavior) available in the game.",
        );

        let mut item_sectionbar_button = TheSectionbarButton::new(TheId::named("Item Section"));
        item_sectionbar_button.set_text("Item".to_string());
        item_sectionbar_button.set_status_text("Edit and manage the items available in the game.");

        let mut tilemap_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Tilemap Section"));
        tilemap_sectionbar_button.set_text("Tilemap".to_string());
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

        let mut model_sectionbar_button = TheSectionbarButton::new(TheId::named("Model Section"));
        model_sectionbar_button.set_text("Model".to_string());
        model_sectionbar_button.set_status_text("Models");

        let mut material_sectionbar_button =
            TheSectionbarButton::new(TheId::named("Material Section"));
        material_sectionbar_button.set_text("Material".to_string());
        material_sectionbar_button.set_status_text("Currently available Materials.");

        let mut node_sectionbar_button = TheSectionbarButton::new(TheId::named("Node Section"));
        node_sectionbar_button.set_text("Node".to_string());
        node_sectionbar_button.set_status_text("The UI of the currently selected node.");

        let mut debug_sectionbar_button = TheSectionbarButton::new(TheId::named("Debug Section"));
        debug_sectionbar_button.set_text("Debug".to_string());
        debug_sectionbar_button.set_status_text(
            "See debug messages and warnings and errors produced by the game code.",
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
        vlayout.add_widget(Box::new(model_sectionbar_button));
        vlayout.add_widget(Box::new(material_sectionbar_button));
        vlayout.add_widget(Box::new(node_sectionbar_button));
        vlayout.add_widget(Box::new(debug_sectionbar_button));
        vlayout.add_widget(Box::new(palette_sectionbar_button));
        vlayout.set_margin(vec4i(5, 10, 5, 5));
        vlayout.set_padding(4);
        vlayout.set_background_color(Some(SectionbarBackground));
        vlayout.limiter_mut().set_max_width(90);
        vlayout.set_reverse_index(Some(2));
        sectionbar_canvas.set_layout(vlayout);

        //

        let mut header = TheCanvas::new();
        let mut switchbar = TheSwitchbar::new(TheId::named("Switchbar Section Header"));
        switchbar.set_text("Region".to_string());
        header.set_widget(switchbar);

        let mut stack_layout = TheStackLayout::new(TheId::named("List Stack Layout"));

        stack_layout.limiter_mut().set_max_width(self.width);

        self.stack_layout_id = stack_layout.id().clone();

        // Regions

        let mut regions_canvas = TheCanvas::default();
        let mut region_shared_layout = TheSharedVLayout::new(TheId::named("Region Shared"));

        let mut list_layout = TheListLayout::new(TheId::named("Region List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 200));
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
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
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
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 250));
        let mut content_canvas = TheCanvas::default();
        content_canvas.set_layout(list_layout);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Region Content Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(vec2i(85, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Region Content Dropdown"));
        drop_down.add_option("All".to_string());
        drop_down.add_option("Character".to_string());
        drop_down.add_option("Areas".to_string());
        drop_down.add_option("Item".to_string());
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        content_canvas.set_top(toolbar_canvas);

        region_tab.add_canvas("Content".to_string(), content_canvas);

        // Region Settings

        let mut settings_canvas = TheCanvas::default();

        let mut text_layout: TheTextLayout = TheTextLayout::new(TheId::empty());
        text_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 250));

        let mut grid_edit = TheTextLineEdit::new(TheId::named("Region Grid Edit"));
        grid_edit.set_range(TheValue::RangeI32(1..=1000));
        grid_edit.set_status_text("The size of the region grid in pixels for the 2D renderer.");
        text_layout.add_pair("Grid Size".to_string(), Box::new(grid_edit));

        let mut tile_size = TheTextLineEdit::new(TheId::named("Region Tile Size"));
        tile_size.set_range(TheValue::RangeI32(1..=100));
        tile_size.set_status_text("The tile size for the 3D renderer.");
        text_layout.add_pair("Tile Size".to_string(), Box::new(tile_size));

        let mut tracer_edit = TheTextLineEdit::new(TheId::named("Region Tracer Samples Edit"));
        tracer_edit.set_range(TheValue::RangeI32(1..=100));
        tracer_edit.set_status_text("The pathtracing samples for the region.");
        text_layout.add_pair("Tracer Samples".to_string(), Box::new(tracer_edit));

        let mut minbr = TheSlider::new(TheId::named("Region Min Brightness"));
        minbr.set_value(TheValue::Float(0.3));
        minbr.set_continuous(true);
        minbr.set_status_text("The minimum brightness of the region for the daylight cycle.");
        text_layout.add_pair("Min. Brightness".to_string(), Box::new(minbr));

        let mut maxbr = TheSlider::new(TheId::named("Region Max Brightness"));
        maxbr.set_value(TheValue::Float(1.0));
        maxbr.set_continuous(true);
        maxbr.set_status_text("The maximum brightness of the region for the daylight cycle.");
        text_layout.add_pair("Max. Brightness".to_string(), Box::new(maxbr));

        let mut region1 = TheTextLineEdit::new(TheId::named("Region Property 1"));
        region1.set_status_text("The region property #1 you can query from CodeGridFX.");
        text_layout.add_pair("Property #1".to_string(), Box::new(region1));

        let mut region2 = TheTextLineEdit::new(TheId::named("Region Property 2"));
        region2.set_status_text("The region property #2 you can query from CodeGridFX.");
        text_layout.add_pair("Property #2".to_string(), Box::new(region2));

        let mut region3 = TheTextLineEdit::new(TheId::named("Region Property 3"));
        region3.set_status_text("The region property #3 you can query from CodeGridFX.");
        text_layout.add_pair("Property #3".to_string(), Box::new(region3));

        let mut region4 = TheTextLineEdit::new(TheId::named("Region Property 4"));
        region4.set_status_text("The region property #4 you can query from CodeGridFX.");
        text_layout.add_pair("Property #4".to_string(), Box::new(region4));

        settings_canvas.set_layout(text_layout);
        region_tab.add_canvas("Settings".to_string(), settings_canvas);

        let mut center_tab = TheCanvas::default();
        center_tab.set_layout(region_tab);

        region_shared_layout.add_canvas(list_canvas);
        region_shared_layout.add_canvas(center_tab);
        region_shared_layout.set_mode(TheSharedVLayoutMode::Shared);
        region_shared_layout.set_shared_ratio(0.4);
        regions_canvas.set_layout(region_shared_layout);

        // Mini Map

        let mut minimap_canvas = TheCanvas::default();
        let mut minimap = TheRenderView::new(TheId::named("MiniMap"));

        minimap.limiter_mut().set_max_size(vec2i(self.width, 200));
        minimap_canvas.set_widget(minimap);

        regions_canvas.set_bottom(minimap_canvas);
        stack_layout.add_canvas(regions_canvas);

        // Character

        let mut character_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Character List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 400));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut regions_add_button = TheTraybarButton::new(TheId::named("Character Add"));
        regions_add_button.set_icon_name("icon_role_add".to_string());
        regions_add_button.set_status_text("Add a new character.");
        let mut regions_remove_button = TheTraybarButton::new(TheId::named("Character Remove"));
        regions_remove_button.set_icon_name("icon_role_remove".to_string());
        regions_remove_button.set_status_text("Remove the current character.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(regions_add_button));
        toolbar_hlayout.add_widget(Box::new(regions_remove_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        character_canvas.set_top(list_canvas);

        let mut empty = TheCanvas::new();
        let mut layout = TheListLayout::new(TheId::empty());
        layout.limiter_mut().set_max_width(self.width);
        //empty.set_layout(layout);
        empty.set_widget(TheColorButton::new(TheId::empty()));
        empty.limiter_mut().set_max_width(self.width);
        character_canvas.set_bottom(empty);

        stack_layout.add_canvas(character_canvas);

        // Item

        let mut item_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("Item List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 400));
        let mut list_canvas = TheCanvas::default();
        list_canvas.set_layout(list_layout);

        let mut item_add_button = TheTraybarButton::new(TheId::named("Item Add"));
        item_add_button.set_icon_name("icon_role_add".to_string());
        item_add_button.set_status_text("Add a new item.");
        let mut item_remove_button = TheTraybarButton::new(TheId::named("Item Remove"));
        item_remove_button.set_icon_name("icon_role_remove".to_string());
        item_remove_button.set_status_text("Remove the current item.");

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(item_add_button));
        toolbar_hlayout.add_widget(Box::new(item_remove_button));
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
            .set_max_size(vec2i(self.width, 200));
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
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
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

        tiles_list_header_canvas_hlayout.set_margin(vec4i(10, 1, 5, 1));
        tiles_list_header_canvas_hlayout.set_padding(3);
        tiles_list_header_canvas_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Tilemap Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(vec2i(75, 18));
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
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 400));
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
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(module_add_button));
        toolbar_hlayout.add_widget(Box::new(module_remove_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        module_canvas.set_top(list_canvas);
        stack_layout.add_canvas(module_canvas);

        // Screens

        let mut screens_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Screen List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 200));
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
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(screen_add_button));
        toolbar_hlayout.add_widget(Box::new(screen_remove_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        list_canvas.set_bottom(toolbar_canvas);

        let mut screen_canvas: TheCanvas = TheCanvas::new();
        let mut screen_tab = TheTabLayout::new(TheId::named("Screen Tab Layout"));

        // Screen Content

        let mut list_layout = TheListLayout::new(TheId::named("Screen Content List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 250));
        let mut content_canvas = TheCanvas::default();
        content_canvas.set_layout(list_layout);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Screen Content Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(vec2i(85, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Screen Content Dropdown"));
        drop_down.add_option("All".to_string());
        drop_down.add_option("Widgets".to_string());
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut widget_add_button = TheTraybarButton::new(TheId::named("Widget Add"));
        widget_add_button.set_icon_name("icon_role_add".to_string());
        widget_add_button.set_status_text("Add a new widget to the screen.");

        let mut widget_remove_button = TheTraybarButton::new(TheId::named("Widget Remove"));
        widget_remove_button.set_icon_name("icon_role_remove".to_string());
        widget_remove_button.set_status_text("Remove the current widget.");
        widget_remove_button.set_disabled(true);

        let mut move_up_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Widget Move Up"));
        move_up_button.set_icon_name("caret-up".to_string());
        move_up_button.set_status_text("Move the widget up.");

        let mut move_down_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Widget Move Down"));
        move_down_button.set_icon_name("caret-down".to_string());
        move_down_button.set_status_text("Move the widget down.");

        let mut widget_bottom_toolbar_hlayout = TheHLayout::new(TheId::empty());
        widget_bottom_toolbar_hlayout.set_background_color(None);
        widget_bottom_toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
        widget_bottom_toolbar_hlayout.add_widget(Box::new(widget_add_button));
        widget_bottom_toolbar_hlayout.add_widget(Box::new(widget_remove_button));
        widget_bottom_toolbar_hlayout.add_widget(Box::new(move_up_button));
        widget_bottom_toolbar_hlayout.add_widget(Box::new(move_down_button));
        //toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        widget_bottom_toolbar_hlayout.set_reverse_index(Some(2));

        let mut widget_bottom_toolbar_canvas = TheCanvas::default();
        widget_bottom_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        widget_bottom_toolbar_canvas.set_layout(widget_bottom_toolbar_hlayout);

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        content_canvas.set_top(toolbar_canvas);
        content_canvas.set_bottom(widget_bottom_toolbar_canvas);

        screen_tab.add_canvas("Content".to_string(), content_canvas);

        // Screen Settings

        let mut settings_canvas = TheCanvas::default();

        let mut text_layout: TheTextLayout = TheTextLayout::new(TheId::empty());
        text_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 250));
        let mut drop_down = TheDropdownMenu::new(TheId::named("Screen Aspect Ratio Dropdown"));
        for aspect in ScreenAspectRatio::iterator() {
            drop_down.add_option(aspect.to_string().to_string());
        }
        drop_down.set_status_text("The aspect ratio of the screen.");
        text_layout.add_pair("Aspect Ratio".to_string(), Box::new(drop_down));
        let mut width_edit = TheTextLineEdit::new(TheId::named("Screen Width Edit"));
        width_edit.set_range(TheValue::RangeI32(1..=100000));
        width_edit.set_status_text("The width of the region in pixel.");
        text_layout.add_pair("Width".to_string(), Box::new(width_edit));
        let mut height_edit = TheTextLineEdit::new(TheId::named("Screen Height Edit"));
        height_edit.set_range(TheValue::RangeI32(1..=100000));
        height_edit.set_status_text("The height of the region in pixels.");
        text_layout.add_pair("Height".to_string(), Box::new(height_edit));
        let mut grid_edit = TheTextLineEdit::new(TheId::named("Screen Grid Edit"));
        grid_edit.set_range(TheValue::RangeI32(1..=1000));
        grid_edit.set_status_text("The size of the screen grid in pixels.");
        text_layout.add_pair("Grid Size".to_string(), Box::new(grid_edit));

        settings_canvas.set_layout(text_layout);
        screen_tab.add_canvas("Settings".to_string(), settings_canvas);

        screen_canvas.set_layout(screen_tab);
        screens_canvas.set_top(list_canvas);
        //regions_canvas.set_layout(text_layout);
        screens_canvas.set_center(screen_canvas);

        let mut empty = TheCanvas::new();
        let mut layout = TheListLayout::new(TheId::empty());
        layout.limiter_mut().set_max_width(self.width);
        layout.limiter_mut().set_max_height(200);
        empty.set_layout(layout);

        screens_canvas.set_bottom(empty);

        stack_layout.add_canvas(screens_canvas);

        // Asset

        let mut asset_canvas = TheCanvas::default();

        let mut list_layout = TheListLayout::new(TheId::named("Asset List"));
        list_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 300));
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
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));
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

        // Model UI

        let mut model_canvas = TheCanvas::default();
        let mut model_list_canvas = TheCanvas::default();

        let mut model_list_header_canvas = TheCanvas::default();
        model_list_header_canvas.set_widget(TheStatusbar::new(TheId::empty()));
        let mut model_list_header_canvas_hlayout = TheHLayout::new(TheId::empty());
        model_list_header_canvas_hlayout.set_background_color(None);
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        model_list_header_canvas_hlayout.set_margin(vec4i(10, 1, 5, 1));
        model_list_header_canvas_hlayout.set_padding(3);
        model_list_header_canvas_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Model Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(vec2i(120, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show models containing the given text.");
        filter_edit.set_continuous(true);
        model_list_header_canvas_hlayout.add_widget(Box::new(filter_edit));

        // let mut drop_down = TheDropdownMenu::new(TheId::named("Material Filter Role"));
        // drop_down.add_option("All".to_string());
        // for dir in TileRole::iterator() {
        //     drop_down.add_option(dir.to_string().to_string());
        // }
        // material_list_header_canvas_hlayout.add_widget(Box::new(drop_down));

        model_list_header_canvas.set_layout(model_list_header_canvas_hlayout);

        let mut model_list_layout = TheListLayout::new(TheId::named("Model List"));
        model_list_layout.set_item_size(42);
        model_list_canvas.set_bottom(model_list_header_canvas);
        model_list_canvas.set_layout(model_list_layout);

        model_canvas.set_center(model_list_canvas);
        stack_layout.add_canvas(model_canvas);

        // Material UI

        let mut material_canvas = TheCanvas::default();
        let mut material_list_canvas = TheCanvas::default();

        let mut material_list_header_canvas = TheCanvas::default();
        material_list_header_canvas.set_widget(TheStatusbar::new(TheId::empty()));
        let mut material_list_header_canvas_hlayout = TheHLayout::new(TheId::empty());
        material_list_header_canvas_hlayout.set_background_color(None);
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        material_list_header_canvas_hlayout.set_margin(vec4i(10, 1, 5, 1));
        material_list_header_canvas_hlayout.set_padding(3);
        material_list_header_canvas_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Material Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(vec2i(120, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show materials containing the given text.");
        filter_edit.set_continuous(true);
        material_list_header_canvas_hlayout.add_widget(Box::new(filter_edit));

        // let mut drop_down = TheDropdownMenu::new(TheId::named("Material Filter Role"));
        // drop_down.add_option("All".to_string());
        // for dir in TileRole::iterator() {
        //     drop_down.add_option(dir.to_string().to_string());
        // }
        // material_list_header_canvas_hlayout.add_widget(Box::new(drop_down));

        material_list_header_canvas.set_layout(material_list_header_canvas_hlayout);

        let mut material_list_layout = TheListLayout::new(TheId::named("Material List"));
        material_list_layout.set_item_size(42);
        material_list_canvas.set_bottom(material_list_header_canvas);
        material_list_canvas.set_layout(material_list_layout);

        material_canvas.set_center(material_list_canvas);
        stack_layout.add_canvas(material_canvas);

        // Node UI

        let mut node_ui_canvas = TheCanvas::default();

        let mut text_layout = TheTextLayout::new(TheId::named("Node Settings"));
        text_layout.limiter_mut().set_max_width(self.width);
        //text_layout.set_fixed_text_width(110);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        node_ui_canvas.set_layout(text_layout);

        stack_layout.add_canvas(node_ui_canvas);

        // Debug

        let mut debug_canvas = TheCanvas::default();

        let mut debug_layout = TheListLayout::new(TheId::named("Debug List"));

        let mut item: TheListItem = TheListItem::new(TheId::named("Debug Item"));
        item.set_text("Eldiron Creater Startup".to_string());
        debug_layout.add_item(item, ctx);

        debug_canvas.set_layout(debug_layout);
        stack_layout.add_canvas(debug_canvas);

        // Palette

        let mut palette_canvas = TheCanvas::default();
        let palette_picker = ThePalettePicker::new(TheId::named("Palette Picker"));
        palette_canvas.set_widget(palette_picker);

        let mut picker_canvas = TheCanvas::default();
        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(5, 2, 5, 2));

        let mut hex_edit = TheTextLineEdit::new(TheId::named("Palette Hex Edit"));
        hex_edit.limiter_mut().set_max_width(100);
        hex_edit.set_status_text("Edit the color in hex format.");

        let mut import_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Palette Import"));
        import_button.set_icon_name("import".to_string());
        import_button.set_status_text("Import a palette in .txt format.");

        let mut clear_all_button: TheTraybarButton =
            TheTraybarButton::new(TheId::named("Palette Clear"));
        clear_all_button.set_icon_name("trash".to_string());
        clear_all_button.set_status_text("Clear all colors in the current palette.");

        let mut picker_layout = TheVLayout::new(TheId::empty());

        toolbar_hlayout.add_widget(Box::new(hex_edit));
        toolbar_hlayout.add_widget(Box::new(import_button));
        toolbar_hlayout.add_widget(Box::new(clear_all_button));
        toolbar_hlayout.set_reverse_index(Some(2));

        toolbar_canvas.set_layout(toolbar_hlayout);
        picker_canvas.set_top(toolbar_canvas);
        picker_layout
            .limiter_mut()
            .set_max_size(vec2i(self.width, 240));
        //toolbar_hlayout.add_widget(Box::new(screen_add_button));
        //toolbar_hlayout.add_widget(Box::new(screen_remove_button));

        let w = TheColorPicker::new(TheId::named("Palette Color Picker"));
        //w.set_value(TheValue::ColorObject(color.clone(), 0.0));
        picker_layout.set_background_color(Some(ListLayoutBackground));
        picker_layout.set_margin(vec4i(20, 10, 20, 10));
        picker_layout.add_widget(Box::new(w));
        picker_canvas.set_layout(picker_layout);

        //palette_canvas.set_top(palette_canvas);
        palette_canvas.set_bottom(picker_canvas);

        stack_layout.add_canvas(palette_canvas);

        //

        let mut canvas = TheCanvas::new();

        canvas.set_top(header);
        canvas.set_right(sectionbar_canvas);
        canvas.top_is_expanding = false;
        canvas.set_layout(stack_layout);

        ui.canvas.set_right(canvas);

        self.apply_region(ui, ctx, None, server, &project.palette);
        self.apply_character(ui, ctx, None);
        self.apply_item(ui, ctx, None);
        self.apply_tilemap(ui, ctx, None);
        self.apply_code(ui, ctx, None);
        self.apply_screen(ui, ctx, None);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::RenderViewClicked(id, coord) | TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "MiniMap" {
                    if let Some(render_view) = ui.get_render_view("MiniMap") {
                        let dim = *render_view.dim();
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let width = dim.width as f32;
                            let height = dim.height as f32;

                            if let Some(mut bbox) = region.map.bounding_box() {
                                bbox.x -= 0.5;
                                bbox.y -= 0.5;
                                bbox.z += 1.0;
                                bbox.w += 1.0;

                                let scale_x = width / bbox.z;
                                let scale_y = height / bbox.w;

                                let grid_size = min(scale_x, scale_y);

                                let bbox_center_x = bbox.x + bbox.z / 2.0;
                                let bbox_center_y = bbox.y + bbox.w / 2.0;

                                let offset_x = -bbox_center_x * region.map.grid_size;
                                let offset_y = bbox_center_y * region.map.grid_size;

                                let grid_x = (coord.x as f32 - width / 2.0 - offset_x) / grid_size;
                                let grid_y = (coord.y as f32 - height / 2.0 + offset_y) / grid_size;

                                server_ctx.center_map_at_grid_pos(
                                    vec2f(width, height),
                                    vec2f(grid_x, grid_y),
                                    &mut region.map,
                                );
                                region.editing_position_3d = vec3f(grid_x, 0.0, grid_y);
                                server.set_editing_position_3d(region.editing_position_3d);
                                server.update_region(region);

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Minimap"),
                                    TheValue::Empty,
                                ));
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
                // self.show_filtered_materials(ui, ctx, project, server_ctx);
                // self.show_filtered_models(ui, ctx, project, server_ctx);
            }
            TheEvent::Custom(id, value) => {
                if id.name == "Update Model List" {
                    self.show_filtered_models(ui, ctx, project, server_ctx);

                    self.deselect_sections_buttons(ui, "Model Section".to_string());
                    self.select_section_button(ui, "Model Section".to_string());

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Model;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Model as usize,
                    ));
                } else if id.name == "Update Minimap" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(render_view) = ui.get_render_view("MiniMap") {
                            let dim = *render_view.dim();
                            let buffer = render_view.render_buffer_mut();
                            buffer.resize(dim.width, dim.height);
                            draw_minimap(region, buffer);
                        }
                    }
                } else if id.name == "Update Tiles" {
                    self.update_tiles(ui, ctx, project, server, client);
                } else if id.name == "Show Node Settings" {
                    self.deselect_sections_buttons(ui, "Node Section".to_string());
                    self.select_section_button(ui, "Node Section".to_string());

                    if let TheValue::Text(text) = value {
                        if let Some(widget) = ui
                            .canvas
                            .get_widget(Some(&"Switchbar Section Header".into()), None)
                        {
                            widget.set_value(TheValue::Text(text.clone()));
                        }
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Node;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Node as usize,
                    ));
                }
            }
            TheEvent::PaletteIndexChanged(_, index) => {
                project.palette.current_index = *index;
                if let Some(widget) = ui.get_widget("Palette Color Picker") {
                    if let Some(color) = &project.palette[*index as usize] {
                        widget.set_value(TheValue::ColorObject(color.clone()));
                    }
                }
                if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                    if let Some(color) = &project.palette[*index as usize] {
                        widget.set_value(TheValue::Text(color.to_hex()));
                    }
                }
            }
            TheEvent::DialogValueOnClose(role, name, uuid, value) => {
                if name == "Rename Region" && *role == TheDialogButtonRole::Accept {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Module" && *role == TheDialogButtonRole::Accept {
                    if let Some(bundle) = project.codes.get_mut(uuid) {
                        bundle.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Screen" && *role == TheDialogButtonRole::Accept {
                    if let Some(screen) = project.screens.get_mut(uuid) {
                        screen.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Widget" && *role == TheDialogButtonRole::Accept {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(widget_id) = server_ctx.curr_widget {
                            if let Some(widget) = screen.get_widget_mut(&widget_id) {
                                widget.name = value.describe();
                                ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                            }
                        }
                    }
                } else if name == "Rename Asset" && *role == TheDialogButtonRole::Accept {
                    if let Some(asset) = project.assets.get_mut(uuid) {
                        asset.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Model" && *role == TheDialogButtonRole::Accept {
                    if let Some(model) = project.models.get_mut(uuid) {
                        model.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                } else if name == "Rename Material" && *role == TheDialogButtonRole::Accept {
                    if let Some(material) = project.materials.get_mut(uuid) {
                        material.name = value.describe();
                        ctx.ui.send(TheEvent::SetValue(*uuid, value.clone()));
                    }
                }
            }
            TheEvent::ContextMenuSelected(widget_id, item_id) => {
                if item_id.name == "Sidebar Delete Character Instance" {
                    if let Some(char_inst) = server_ctx.curr_character_instance {
                        if let Some((value, _)) = server.get_character_property(
                            server_ctx.curr_region,
                            char_inst,
                            "name".to_string(),
                        ) {
                            open_delete_confirmation_dialog(
                                "Delete Character Instance ?",
                                format!("Permanently delete '{}' ?", value.describe()).as_str(),
                                char_inst,
                                ui,
                                ctx,
                            );
                        }
                    }
                } else if item_id.name == "Sidebar Delete Item Instance" {
                    if let Some(item_inst) = server_ctx.curr_item_instance {
                        let mut name = str!("Unknown");

                        if let Some((value, _)) = server.get_item_property(
                            server_ctx.curr_region,
                            item_inst,
                            "name".to_string(),
                        ) {
                            name = value.describe();
                        }
                        open_delete_confirmation_dialog(
                            "Delete Item Instance ?",
                            &format!("Permanently delete '{}' ?", name),
                            item_inst,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Sidebar Delete Area" {
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
                } else if item_id.name == "Add Image" {
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
                } else if item_id.name == "Rename Module" {
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
                } else if item_id.name == "Rename Widget" {
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
                } else if item_id.name == "Rename Asset" {
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
                } else if item_id.name == "Rename Model" {
                    if let Some(model) = project.models.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Model",
                            "Model Name",
                            &model.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                } else if item_id.name == "Rename Material" {
                    if let Some(material) = project.materials.get(&widget_id.uuid) {
                        open_text_dialog(
                            "Rename Material",
                            "Material Name",
                            &material.name,
                            widget_id.uuid,
                            ui,
                            ctx,
                        );
                    }
                }
            }
            TheEvent::DragStarted(id, text, offset) => {
                if id.name == "Character Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Character: {}", text));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                } else if id.name == "Item Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Item: {}", text));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                } else if id.name == "Model Item" {
                    let mut drop = TheDrop::new(id.clone());
                    drop.set_title(format!("Model: {}", text));
                    drop.set_text(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Region Property 1" {
                    if let Some(text) = value.to_string() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.property_1 = text;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Region Property 2" {
                    if let Some(text) = value.to_string() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.property_2 = text;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Region Property 3" {
                    if let Some(text) = value.to_string() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.property_3 = text;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Region Property 4" {
                    if let Some(text) = value.to_string() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.property_4 = text;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Palette Hex Edit" {
                    if let Some(hex) = value.to_string() {
                        let color = TheColor::from_hex(&hex);

                        if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                            palette_picker.set_color(color.clone());
                            redraw = true;
                            project.palette[palette_picker.index()] = Some(color.clone());
                            server.set_palette(&project.palette);
                        }
                        if let Some(widget) = ui.get_widget("Palette Color Picker") {
                            widget.set_value(TheValue::ColorObject(color.clone()));
                        }
                    }
                } else if id.name == "Palette Color Picker" {
                    if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                        if let Some(color) = value.to_color() {
                            palette_picker.set_color(color.clone());
                            redraw = true;
                            project.palette[palette_picker.index()] = Some(color);
                            server.set_palette(&project.palette);
                        }
                    }
                    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                        if let Some(color) = value.to_color() {
                            widget.set_value(TheValue::Text(color.to_hex()));
                        }
                    }
                } else if id.name == "Screen Aspect Ratio Dropdown" {
                    if let Some(index) = value.to_i32() {
                        if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                            if let Some(aspect) =
                                ScreenAspectRatio::from_index((index as usize).try_into().unwrap())
                            {
                                screen.aspect_ratio = aspect;

                                let new_width = screen.aspect_ratio.width(screen.height);

                                screen.width = new_width;
                                ui.set_widget_value(
                                    "Screen Width Edit",
                                    ctx,
                                    TheValue::Text(new_width.to_string()),
                                );

                                redraw = true;
                            }
                        }
                    }
                } else if id.name == "Screen Width Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(v) = value.to_i32() {
                            screen.width = v;
                        }
                        let new_height = screen.aspect_ratio.height(screen.width);

                        screen.height = new_height;
                        ui.set_widget_value(
                            "Screen Height Edit",
                            ctx,
                            TheValue::Text(new_height.to_string()),
                        );

                        redraw = true;
                    }
                } else if id.name == "Screen Height Edit" {
                    if let Some(screen) = project.screens.get_mut(&server_ctx.curr_screen) {
                        if let Some(v) = value.to_i32() {
                            screen.height = v;
                        }
                        let new_width = screen.aspect_ratio.width(screen.height);

                        screen.width = new_width;
                        ui.set_widget_value(
                            "Screen Width Edit",
                            ctx,
                            TheValue::Text(new_width.to_string()),
                        );

                        redraw = true;
                    }
                } else if id.name == "Region Grid Edit" {
                    if let Some(v) = value.to_i32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.grid_size = v;
                            server.update_region(region);

                            server.update_region(region);
                            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                                if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
                                    rgba.set_grid(Some(v));
                                    let width = region.width * region.grid_size;
                                    let height = region.height * region.grid_size;
                                    let buffer =
                                        TheRGBABuffer::new(TheDim::new(0, 0, width, height));
                                    rgba.set_buffer(buffer);
                                }
                            }
                        }
                    }
                } else if id.name == "Region Tile Size" {
                    if let Some(v) = value.to_i32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.tile_size = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Region Tracer Samples Edit" {
                    if let Some(v) = value.to_i32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.pathtracer_samples = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Region Min Brightness" {
                    if let Some(v) = value.to_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.min_brightness = v;
                            server.update_region(region);
                        }
                    }
                } else if id.name == "Region Max Brightness" {
                    if let Some(v) = value.to_f32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.max_brightness = v;
                            server.update_region(region);
                        }
                    }
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
                } else if id.name == "Material Filter Edit" {
                    self.show_filtered_materials(ui, ctx, project, server_ctx);
                } else if id.name == "Tilemap Editor Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(layout) = ui.get_rgba_layout("Tilemap Editor") {
                            layout.set_zoom(v);
                            layout.relayout(ctx);
                        }
                    }
                } else if id.name == "Region Content Filter Edit"
                    || id.name == "Region Content Dropdown"
                {
                    self.apply_region(
                        ui,
                        ctx,
                        project.get_region(&server_ctx.curr_region),
                        server,
                        &project.palette,
                    );
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
                                client.set_assets(project);
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
                                        items: vec![TheContextMenuItem::new(
                                            "Rename Tilemap...".to_string(),
                                            TheId::named("Rename Tilemap"),
                                        )],
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
                            self.update_tiles(ui, ctx, project, server, client);

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
                }
            }
            TheEvent::ImageDecodeResult(id, name, _buffer) => {
                if id.name == "Add Image" {
                    if let Some(layout) =
                        ui.canvas.get_layout(Some(&"Asset List".to_string()), None)
                    {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item =
                                TheListItem::new(TheId::named_with_id("Asset Item", id.uuid));
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
                } else if id.name == "Tilemap Add" {
                    if let Some(layout) = ui
                        .canvas
                        .get_layout(Some(&"Tilemap List".to_string()), None)
                    {
                        if let Some(list_layout) = layout.as_list_layout() {
                            let mut item =
                                TheListItem::new(TheId::named_with_id("Tilemap Item", id.uuid));
                            item.set_text(name.clone());
                            item.set_state(TheWidgetState::Selected);
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Rename Tilemap...".to_string(),
                                    TheId::named("Rename Tilemap"),
                                )],
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
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Palette Clear" {
                    let prev = project.palette.clone();
                    project.palette.clear();
                    if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                        let index = palette_picker.index();

                        palette_picker.set_palette(project.palette.clone());
                        if let Some(widget) = ui.get_widget("Palette Color Picker") {
                            if let Some(color) = &project.palette[index] {
                                widget.set_value(TheValue::ColorObject(color.clone()));
                            }
                        }
                        if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                            if let Some(color) = &project.palette[index] {
                                widget.set_value(TheValue::Text(color.to_hex()));
                            }
                        }
                    }
                    server.set_palette(&project.palette);
                    redraw = true;

                    let undo = PaletteUndoAtom::Edit(prev, project.palette.clone());
                    UNDOMANAGER.lock().unwrap().add_palette_undo(undo, ctx);
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
                }
                // Regions Add
                else if id.name == "Region Add" {
                    if let Some(list_layout) = ui.get_list_layout("Region List") {
                        let region = Region::new();

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Region Item", region.id));
                        item.set_text(region.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Region...".to_string(),
                                TheId::named("Rename Region"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        server_ctx.clear();
                        server_ctx.curr_region = region.id;
                        project.regions.push(region);
                        server.set_project(project.clone());
                    }
                } else if id.name == "Region Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Region List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_region(&selected.uuid);
                            self.apply_region(ui, ctx, None, server, &project.palette);
                        }
                    }
                } else if id.name == "Region Item" {
                    for r in &project.regions {
                        if r.id == id.uuid {
                            self.apply_region(ui, ctx, Some(r), server, &project.palette);
                            redraw = true;
                        }
                    }
                } else if id.name == "Character Add" {
                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        let mut bundle = TheCodeBundle::new();

                        let mut init = TheCodeGrid {
                            name: "init".into(),
                            ..Default::default()
                        };
                        init.insert_atom(
                            (0, 0),
                            TheCodeAtom::Set("@self.name".to_string(), TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (1, 0),
                            TheCodeAtom::Assignment(TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (2, 0),
                            TheCodeAtom::Value(TheValue::Text("Unnamed".to_string())),
                        );

                        init.insert_atom(
                            (0, 2),
                            TheCodeAtom::Set("@self.tile".to_string(), TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (1, 2),
                            TheCodeAtom::Assignment(TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (2, 2),
                            TheCodeAtom::Value(TheValue::Tile("Name".to_string(), Uuid::nil())),
                        );

                        bundle.insert_grid(init);

                        let main = TheCodeGrid {
                            name: "main".into(),
                            ..Default::default()
                        };
                        bundle.insert_grid(main);

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Character Item", bundle.id));
                        item.set_text(bundle.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_character(ui, ctx, Some(&bundle));
                        server.insert_character(bundle.clone());
                        project.add_character(bundle);
                    }
                } else if id.name == "Character Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_character(&selected.uuid);
                            self.apply_character(ui, ctx, None);
                        }
                    }
                } else if id.name == "Character Item" {
                    if let Some(c) = project.characters.get(&id.uuid) {
                        server_ctx.curr_character = Some(id.uuid);
                        //server_ctx.curr_character_instance = None;
                        self.apply_character(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Item Item" {
                    if let Some(c) = project.items.get(&id.uuid) {
                        server_ctx.curr_item = Some(id.uuid);
                        self.apply_item(ui, ctx, Some(c));
                        redraw = true;
                    }
                } else if id.name == "Item Add" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        let mut bundle = TheCodeBundle::new();

                        let mut init = TheCodeGrid {
                            name: "init".into(),
                            ..Default::default()
                        };
                        init.insert_atom(
                            (0, 0),
                            TheCodeAtom::Set("@self.name".to_string(), TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (1, 0),
                            TheCodeAtom::Assignment(TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (2, 0),
                            TheCodeAtom::Value(TheValue::Text("Unnamed".to_string())),
                        );

                        init.insert_atom(
                            (0, 2),
                            TheCodeAtom::Set("@self.tile".to_string(), TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (1, 2),
                            TheCodeAtom::Assignment(TheValueAssignment::Assign),
                        );
                        init.insert_atom(
                            (2, 2),
                            TheCodeAtom::Value(TheValue::Tile("Name".to_string(), Uuid::nil())),
                        );

                        bundle.insert_grid(init);

                        let main = TheCodeGrid {
                            name: "main".into(),
                            ..Default::default()
                        };
                        bundle.insert_grid(main);

                        let mut item =
                            TheListItem::new(TheId::named_with_id("Item Item", bundle.id));
                        item.set_text(bundle.name.clone());
                        item.set_state(TheWidgetState::Selected);
                        list_layout.deselect_all();
                        let id = item.id().clone();
                        list_layout.add_item(item, ctx);
                        ctx.ui
                            .send_widget_state_changed(&id, TheWidgetState::Selected);

                        self.apply_item(ui, ctx, Some(&bundle));
                        server.insert_item(bundle.clone());
                        project.add_item(bundle);
                    }
                } else if id.name == "Item Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_item(&selected.uuid);
                            self.apply_item(ui, ctx, None);
                        }
                    }
                } else if id.name == "Module Add" {
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

                        self.apply_code(ui, ctx, Some(&bundle));
                        project.add_code(bundle);
                    }
                } else if id.name == "Module Remove" {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            list_layout.remove(selected.clone());
                            project.remove_code(&selected.uuid);
                            self.apply_code(ui, ctx, None);
                        }
                    }
                } else if id.name == "Module Item" {
                    if let Some(c) = project.codes.get(&id.uuid) {
                        self.apply_code(ui, ctx, Some(c));
                        redraw = true;
                    }
                }
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

                        TILEMAPEDITOR.lock().unwrap().set_tilemap(t, ui, ctx);
                        self.apply_tilemap(ui, ctx, Some(t));
                        ctx.ui.relayout = true;
                    }
                    redraw = true;
                } else if id.name == "Tilemap Editor Add Anim"
                    || id.name == "Tilemap Editor Add Multi"
                {
                    let mut clear_selection = false;

                    if let Some(editor) = ui
                        .canvas
                        .get_layout(Some(&"Tilemap Editor".to_string()), None)
                    {
                        if let Some(editor) = editor.as_rgba_layout() {
                            let mut tile = Tile::new();

                            if id.name == "Tilemap Editor Add Anim" {
                                let sequence = editor
                                    .rgba_view_mut()
                                    .as_rgba_view()
                                    .unwrap()
                                    .selection_as_sequence();
                                tile.sequence = sequence;
                            } else {
                                let dim = editor
                                    .rgba_view_mut()
                                    .as_rgba_view()
                                    .unwrap()
                                    .selection_as_dim();

                                let mut grid_size = 16;

                                if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                                    if let Some(t) = project.get_tilemap(curr_tilemap_uuid) {
                                        grid_size = t.grid_size;
                                    }
                                }

                                let region = TheRGBARegion::new(
                                    dim.x as usize * grid_size as usize,
                                    dim.y as usize * grid_size as usize,
                                    dim.width as usize * grid_size as usize,
                                    dim.height as usize * grid_size as usize,
                                );

                                tile.sequence = TheRGBARegionSequence::new();
                                tile.sequence.regions.push(region);
                            }

                            if let Some(text_line_edit) =
                                ui.get_text_line_edit("Tilemap Editor Name Edit")
                            {
                                tile.name = text_line_edit.text();
                            }

                            if let Some(block_widget) = ui
                                .canvas
                                .get_widget(Some(&"Tilemap Editor Block".to_string()), None)
                            {
                                tile.blocking = block_widget.state() == TheWidgetState::Selected;
                            }

                            if let Some(role_widget) = ui.get_drop_down_menu("Tilemap Editor Role")
                            {
                                let index = role_widget.selected_index();
                                tile.role = TileRole::from_index(index as u8).unwrap();
                            }

                            // Only add if non-empty
                            if !tile.name.is_empty() && !tile.sequence.regions.is_empty() {
                                if let Some(layout) = ui
                                    .canvas
                                    .get_layout(Some(&"Tilemap Tile List".to_string()), None)
                                {
                                    let list_layout_id = layout.id().clone();
                                    if let Some(list_layout) = layout.as_list_layout() {
                                        let mut item = TheListItem::new(TheId::named_with_id(
                                            "Tilemap Tile",
                                            tile.id,
                                        ));
                                        item.set_text(tile.name.clone());
                                        let mut sub_text = if tile.blocking {
                                            "Blocking".to_string()
                                        } else {
                                            "Non-Blocking".to_string()
                                        };
                                        sub_text +=
                                            ("  ".to_string() + tile.role.to_string()).as_str();
                                        item.set_sub_text(sub_text);
                                        item.set_state(TheWidgetState::Selected);
                                        item.set_size(42);
                                        item.set_associated_layout(list_layout_id);
                                        if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                                            if let Some(t) = project.get_tilemap(curr_tilemap_uuid)
                                            {
                                                item.set_icon(
                                                    tile.sequence.regions[0]
                                                        .scale(&t.buffer, 36, 36),
                                                );
                                            }
                                        }
                                        list_layout.deselect_all();
                                        let id = item.id().clone();
                                        list_layout.add_item(item, ctx);
                                        ctx.ui.send_widget_state_changed(
                                            &id,
                                            TheWidgetState::Selected,
                                        );

                                        clear_selection = true;
                                        redraw = true;
                                    }
                                }

                                if let Some(curr_tilemap_uuid) = self.curr_tilemap_uuid {
                                    if let Some(tilemap) = project.get_tilemap(curr_tilemap_uuid) {
                                        tilemap.tiles.push(tile);
                                    }
                                }

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Tilepicker"),
                                    TheValue::Empty,
                                ));

                                self.update_tiles(ui, ctx, project, server, client);
                            } else if tile.name.is_empty() {
                                open_info_dialog(
                                    "Tilemap Editor",
                                    "Tile does not have any tags.",
                                    ui,
                                    ctx,
                                );
                            }
                        }
                    }

                    // Clear the selection if successful
                    if clear_selection {
                        if let Some(editor) = ui
                            .canvas
                            .get_layout(Some(&"Tilemap Editor".to_string()), None)
                        {
                            if let Some(editor) = editor.as_rgba_layout() {
                                editor
                                    .rgba_view_mut()
                                    .as_rgba_view()
                                    .unwrap()
                                    .set_selection(FxHashSet::default());
                            }
                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named("Tilemap Editor Clear"),
                                TheWidgetState::Clicked,
                            ))
                        }
                    }
                } else if id.name == "Screen Item" {
                    if let Some(s) = project.screens.get(&id.uuid) {
                        self.apply_screen(ui, ctx, Some(s));
                        server_ctx.curr_screen = id.uuid;
                        redraw = true;
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
                        client.update_screen(&screen);
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
                } else if id.name == "Widget Add" {
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
                }
                // Section Buttons
                else if id.name == "Region Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());
                    CODEEDITOR.lock().unwrap().set_allow_modules(true);
                    set_server_externals();

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Regions".to_string()));
                    }

                    // if let Some(button) = ui.get_group_button("Editor Group") {
                    //     if button.index() == EditorMode::Pick as i32 {
                    //         ctx.ui.send(TheEvent::IndexChanged(button.id().clone(), 1));
                    //     }
                    // }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Region;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Region as usize,
                    ));
                    redraw = true;
                } else if id.name == "Character Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());
                    CODEEDITOR.lock().unwrap().set_allow_modules(true);
                    set_server_externals();

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

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Character;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Character as usize,
                    ));
                    redraw = true;
                } else if id.name == "Item Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());
                    CODEEDITOR.lock().unwrap().set_allow_modules(true);
                    set_server_externals();

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

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Item;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Item as usize,
                    ));
                    redraw = true;
                } else if id.name == "Tilemap Section" && *state == TheWidgetState::Selected {
                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Tilemaps".to_string()));
                    }

                    if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
                        if let Some(selected) = list_layout.selected() {
                            ctx.ui
                                .send(TheEvent::StateChanged(selected, TheWidgetState::Selected));
                        }
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Tilemap;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Tilemap as usize,
                    ));
                    self.deselect_sections_buttons(ui, id.name.clone());
                    redraw = true;
                } else if id.name == "Module Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());
                    CODEEDITOR.lock().unwrap().set_allow_modules(false);
                    set_server_externals();

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

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Module;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Module as usize,
                    ));
                    redraw = true;
                } else if id.name == "Screen Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());
                    CODEEDITOR.lock().unwrap().set_allow_modules(true);
                    set_client_externals();

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

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Screen;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Screen as usize,
                    ));
                    redraw = true;
                } else if id.name == "Asset Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());

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

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Asset;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Asset as usize,
                    ));
                    redraw = true;
                } else if id.name == "Model Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Models".to_string()));
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Model;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Model as usize,
                    ));
                    redraw = true;
                } else if id.name == "Material Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Materials".to_string()));
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Material;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Material as usize,
                    ));
                    redraw = true;
                } else if id.name == "Node Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Node".to_string()));
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Node;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Node as usize,
                    ));

                    redraw = true;
                } else if id.name == "Debug Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Debug Output".to_string()));
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Debug;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Debug as usize,
                    ));
                    redraw = true;
                } else if id.name == "Palette Section" && *state == TheWidgetState::Selected {
                    self.deselect_sections_buttons(ui, id.name.clone());
                    CODEEDITOR.lock().unwrap().set_allow_modules(false);

                    if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Switchbar Section Header".into()), None)
                    {
                        widget.set_value(TheValue::Text("Palette".to_string()));
                    }

                    *SIDEBARMODE.lock().unwrap() = SidebarMode::Palette;

                    ctx.ui.send(TheEvent::SetStackIndex(
                        self.stack_layout_id.clone(),
                        SidebarMode::Palette as usize,
                    ));
                    redraw = true;
                } else if id.name == "Compile" {
                    // Compile button in the editor. Compile the code and send it to the server if successful.
                    // We do not need to store it in the project because thats already done in the
                    // CodeBundleChanged event.

                    if let Some(layout) = ui.get_code_layout("Code Editor") {
                        if let Some(code_view) = layout.code_view_mut().as_code_view() {
                            let grid = code_view.codegrid_mut();

                            let rc = if *SIDEBARMODE.lock().unwrap() == SidebarMode::Screen {
                                client.compiler().compile(grid)
                            } else {
                                server.compiler().compile(grid)
                            };

                            if let Ok(mut module) = rc {
                                let bundle: TheCodeBundle = CODEEDITOR.lock().unwrap().get_bundle();

                                // We need to take the module name out of the bundle to make sure
                                // to handle renames correctly.
                                if let Some(g) = bundle.get_grid(&grid.id) {
                                    module.name.clone_from(&g.name);
                                }

                                // Successfully compiled, transfer the bundle to the server.

                                if CODEEDITOR.lock().unwrap().code_id == "Character Instance" {
                                    if let Some(character_instance) =
                                        server_ctx.curr_character_instance
                                    {
                                        // This is a character instance bundle

                                        if let Some(region) =
                                            project.get_region_mut(&server_ctx.curr_region)
                                        {
                                            if let Some(character) =
                                                region.characters.get_mut(&character_instance)
                                            {
                                                // We check if the key exists first as a safety measure
                                                #[allow(clippy::map_entry)]
                                                if character.instance.grids.contains_key(&grid.id) {
                                                    // Update the character instance
                                                    character
                                                        .instance
                                                        .grids
                                                        .insert(grid.id, grid.clone());

                                                    server.update_character_instance_bundle(
                                                        server_ctx.curr_region,
                                                        character_instance,
                                                        character.instance.clone(),
                                                    );

                                                    // Just in case the user changed the name of the character
                                                    // we update the character instance name in the list
                                                    if let Some((TheValue::Text(name), _)) = server
                                                        .get_character_property(
                                                            server_ctx.curr_region,
                                                            character.instance.id,
                                                            "name".into(),
                                                        )
                                                    {
                                                        if let Some(list) = ui
                                                            .get_list_layout("Region Content List")
                                                        {
                                                            list.set_item_text(
                                                                character.instance.id,
                                                                name,
                                                            );
                                                            //println!("new name {:?}", name);
                                                        }
                                                    }
                                                } else {
                                                    println!("Character instance does not contain grid: {:?}", grid.name);
                                                }
                                            }
                                        }
                                    }
                                } else if CODEEDITOR.lock().unwrap().code_id == "Item Instance" {
                                    if let Some(item_instance) = server_ctx.curr_item_instance {
                                        // This is an item instance bundle

                                        if let Some(region) =
                                            project.get_region_mut(&server_ctx.curr_region)
                                        {
                                            if let Some(item) = region.items.get_mut(&item_instance)
                                            {
                                                // We check if the key exists first as a safety measure
                                                #[allow(clippy::map_entry)]
                                                if item.instance.grids.contains_key(&grid.id) {
                                                    // Update the character instance
                                                    item.instance
                                                        .grids
                                                        .insert(grid.id, grid.clone());

                                                    server.update_item_instance_bundle(
                                                        server_ctx.curr_region,
                                                        item_instance,
                                                        item.instance.clone(),
                                                    );
                                                    // if let Some(value) = server.get_item_property(
                                                    //     server_ctx.curr_region,
                                                    //     item_instance,
                                                    //     "name".to_string(),
                                                    // ) {
                                                    //     println!("Item name: {:?}", value);
                                                    // }
                                                } else {
                                                    println!(
                                                        "Item instance does not contain grid: {:?}",
                                                        grid.name
                                                    );
                                                }
                                            }
                                        }
                                    }
                                } else if CODEEDITOR.lock().unwrap().code_id == "Area Instance" {
                                    if let Some(area) = server_ctx.curr_area {
                                        // This is a region bundle

                                        if let Some(region) =
                                            project.get_region_mut(&server_ctx.curr_region)
                                        {
                                            if let Some(area) = region.areas.get_mut(&area) {
                                                // We check if the key exists first as a safety measure
                                                #[allow(clippy::map_entry)]
                                                if area.bundle.grids.contains_key(&grid.id) {
                                                    area.bundle.grids.insert(grid.id, grid.clone());

                                                    server.insert_area(
                                                        server_ctx.curr_region,
                                                        area.clone(),
                                                    );
                                                } else {
                                                    println!(
                                                        "Area does not contain grid: {:?}",
                                                        grid.name
                                                    );
                                                }
                                            }
                                        }
                                    }
                                } else if CODEEDITOR.lock().unwrap().code_id == "Character" {
                                    if let Some(name) = server.insert_character(bundle.clone()) {
                                        if let Some(widget) = ui.get_widget_id(bundle.id) {
                                            if let Some(list_item) = widget.as_list_item() {
                                                list_item.set_text(name.clone());
                                            }
                                        }
                                        if let Some(bundle) = project.characters.get_mut(&bundle.id)
                                        {
                                            bundle.name = name;
                                        }
                                    }
                                } else if CODEEDITOR.lock().unwrap().code_id == "Item" {
                                    if let Some(name) = server.insert_item(bundle.clone()) {
                                        if let Some(widget) = ui.get_widget_id(bundle.id) {
                                            if let Some(list_item) = widget.as_list_item() {
                                                list_item.set_text(name.clone());
                                            }
                                        }
                                        if let Some(bundle) = project.items.get_mut(&bundle.id) {
                                            bundle.name = name;
                                        }
                                    }
                                } else if CODEEDITOR.lock().unwrap().code_id == "Module" {
                                    // Update the bundle in the server
                                    server.update_bundle(bundle.clone());

                                    // Update the bundle in the project
                                    project.codes.insert(bundle.id, bundle.clone());

                                    // Provide the bundle info to the editor
                                    CODEEDITOR.lock().unwrap().insert_module(
                                        bundle.name,
                                        bundle.id,
                                        module,
                                    );
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Update Code Menu"),
                                        TheValue::Empty,
                                    ));
                                } else if CODEEDITOR.lock().unwrap().code_id == "Screen" {
                                    if let Some(screen) =
                                        project.screens.get_mut(&server_ctx.curr_screen)
                                    {
                                        if let Some(widget) = screen.get_widget_mut(&bundle.id) {
                                            widget.bundle = bundle;
                                            client.update_screen(screen);
                                        }
                                    }
                                }

                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Compiled successfully.".to_string(),
                                ));
                            } else {
                                code_view.set_debug_module(TheDebugModule::default());
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Failed to compile.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
            TheEvent::CodeBundleChanged(bundle, _) => {
                ctx.ui.relayout = true;
                /*
                if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region {
                    if let Some(character_instance) = server_ctx.curr_character_instance {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(character) = region.characters.get_mut(&character_instance)
                            {
                                // Update the character instance
                                character.instance = bundle.clone();
                            }
                        }
                    }
                } else*/
                if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                    if let Some(list_layout) = ui.get_list_layout("Character List") {
                        if let Some(selected) = list_layout.selected() {
                            if selected.uuid == bundle.id {
                                if let Some(character) = project.characters.get_mut(&bundle.id) {
                                    if character.id == bundle.id {
                                        *character = bundle.clone();
                                        server.insert_character(character.clone());
                                    }
                                }
                                redraw = true;
                            }
                        }
                    }
                } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Item {
                    if let Some(list_layout) = ui.get_list_layout("Item List") {
                        if let Some(selected) = list_layout.selected() {
                            if selected.uuid == bundle.id {
                                if let Some(item) = project.items.get_mut(&bundle.id) {
                                    if item.id == bundle.id {
                                        *item = bundle.clone();
                                    }
                                }
                                redraw = true;
                            }
                        }
                    }
                } else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Module {
                    if let Some(list_layout) = ui.get_list_layout("Module List") {
                        if let Some(selected) = list_layout.selected() {
                            if selected.uuid == bundle.id {
                                if let Some(code) = project.codes.get_mut(&bundle.id) {
                                    if code.id == bundle.id {
                                        *code = bundle.clone();
                                    }
                                }
                                redraw = true;
                            }
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
        project: &Project,
    ) {
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
                        TheContextMenuItem::new(
                            "Copy Prerendered...".to_string(),
                            TheId::named("Copy Prerendered"),
                        ),
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
                list_layout.add_item(item, ctx);
            }
        }
        if let Some(list_layout) = ui.get_list_layout("Tilemap List") {
            list_layout.clear();
            for tilemap in &project.tilemaps {
                let mut item = TheListItem::new(TheId::named_with_id("Tilemap Item", tilemap.id));
                item.set_text(tilemap.name.clone());
                item.set_context_menu(Some(TheContextMenu {
                    items: vec![TheContextMenuItem::new(
                        "Rename Tilemap...".to_string(),
                        TheId::named("Rename Tilemap"),
                    )],
                    ..Default::default()
                }));
                list_layout.add_item(item, ctx);
            }
        }
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
        }
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

            if let Some(widget) = ui.get_widget("Palette Color Picker") {
                if let Some(color) = &project.palette[index] {
                    widget.set_value(TheValue::ColorObject(color.clone()));
                }
            }
            if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                if let Some(color) = &project.palette[index] {
                    widget.set_value(TheValue::Text(color.to_hex()));
                }
            }
        }

        TILEDRAWER
            .lock()
            .unwrap()
            .set_materials(project.materials.clone());

        ui.select_first_list_item("Region List", ctx);
        ui.select_first_list_item("Character List", ctx);
        ui.select_first_list_item("Item List", ctx);
        ui.select_first_list_item("Tilemap List", ctx);
        ui.select_first_list_item("Module List", ctx);
        ui.select_first_list_item("Screen List", ctx);
        ui.select_first_list_item("Asset List", ctx);

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));

        // Set the current material
        let selected_material = if project.materials.is_empty() {
            None
        } else if let Some((id, _)) = project.materials.get_index(0) {
            Some(*id)
        } else {
            None
        };

        server_ctx.curr_material_object = selected_material;

        self.show_filtered_models(ui, ctx, project, server_ctx);
        self.show_filtered_materials(ui, ctx, project, server_ctx);
    }

    /// Apply the given character to the UI
    pub fn apply_character(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        character: Option<&TheCodeBundle>,
    ) {
        ui.set_widget_disabled_state("Character Remove", ctx, character.is_none());

        // Set the character bundle.
        if let Some(character) = character {
            let char_list_canvas: TheCanvas =
                CODEEDITOR
                    .lock()
                    .unwrap()
                    .set_bundle(character.clone(), ctx, self.width, None);
            CODEEDITOR.lock().unwrap().code_id = str!("Character");

            if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                if let Some(canvas) = stack_layout.canvas_at_mut(1) {
                    canvas.set_bottom(char_list_canvas);
                }
            }
        } else if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            if let Some(canvas) = stack_layout.canvas_at_mut(1) {
                let mut empty = TheCanvas::new();
                empty.set_layout(TheListLayout::new(TheId::empty()));
                canvas.set_bottom(empty);
            }
        }

        ctx.ui.relayout = true;
    }

    /// Apply the given item to the UI
    pub fn apply_item(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        item: Option<&TheCodeBundle>,
    ) {
        ui.set_widget_disabled_state("Item Remove", ctx, item.is_none());

        // Set the Item bundle.
        if let Some(item) = item {
            let item_list_canvas: TheCanvas =
                CODEEDITOR
                    .lock()
                    .unwrap()
                    .set_bundle(item.clone(), ctx, self.width, None);
            CODEEDITOR.lock().unwrap().code_id = str!("Item");

            if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                if let Some(canvas) = stack_layout.canvas_at_mut(2) {
                    canvas.set_bottom(item_list_canvas);
                }
            }
        } else if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            if let Some(canvas) = stack_layout.canvas_at_mut(2) {
                let mut empty = TheCanvas::new();
                empty.set_layout(TheVLayout::new(TheId::empty()));
                canvas.set_bottom(empty);
            }
        }

        ctx.ui.relayout = true;
    }

    /// Apply the given module to the UI
    pub fn apply_code(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        code: Option<&TheCodeBundle>,
    ) {
        ui.set_widget_disabled_state("Module Remove", ctx, code.is_none());

        // Set the Code bundle.
        if let Some(code) = code {
            let code_list_canvas: TheCanvas =
                CODEEDITOR
                    .lock()
                    .unwrap()
                    .set_bundle(code.clone(), ctx, self.width, None);
            CODEEDITOR.lock().unwrap().code_id = str!("Module");

            if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
                if let Some(canvas) = stack_layout.canvas_at_mut(4) {
                    canvas.set_bottom(code_list_canvas);
                }
            }
        } else if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            if let Some(canvas) = stack_layout.canvas_at_mut(4) {
                let mut empty = TheCanvas::new();
                empty.set_layout(TheVLayout::new(TheId::empty()));
                canvas.set_bottom(empty);
            }
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
                    rgba_layout.scroll_to(vec2i(0, 0));
                }
            }
        }

        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Aspect Ratio Dropdown".to_string()), None)
        {
            if let Some(screen) = screen {
                widget.set_value(TheValue::Text(screen.aspect_ratio.to_string().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Width Edit".to_string()), None)
        {
            if let Some(screen) = screen {
                widget.set_value(TheValue::Text(screen.width.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Height Edit".to_string()), None)
        {
            if let Some(screen) = screen {
                widget.set_value(TheValue::Text(screen.height.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }
        if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Screen Grid Edit".to_string()), None)
        {
            if let Some(screen) = screen {
                widget.set_value(TheValue::Text(screen.grid_size.clone().to_string()));
                widget.set_disabled(false);
            } else {
                widget.set_value(TheValue::Empty);
                widget.set_disabled(true);
            }
        }

        if let Some(screen) = screen {
            ui.set_widget_disabled_state("Widget Add", ctx, false);
            if !screen.widget_list.is_empty() {
                ui.set_widget_disabled_state("Widget Remove", ctx, false);
            }

            // if let Some(zoom) = ui.get_widget("Screen Editor Zoom") {
            //zoom.set_value(TheValue::Float(screen.zoom));
            // }
            if let Some(rgba_layout) = ui.get_rgba_layout("Screen Editor") {
                if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    //rgba.set_zoom(screen.zoom);
                    rgba.set_grid(Some(screen.grid_size));
                }
                rgba_layout.scroll_to(screen.scroll_offset);
            }
        }

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
                    // Show Widgets
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
                    }
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

    /// Apply the given item to the UI
    pub fn apply_region(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        region: Option<&Region>,
        server: &mut Server,
        _palette: &ThePalette,
    ) {
        ui.set_widget_disabled_state("Region Remove", ctx, region.is_none());
        ui.set_widget_disabled_state("Region Settings", ctx, region.is_none());

        //

        if region.is_none() {
            if let Some(zoom) = ui.get_widget("Region Editor Zoom") {
                zoom.set_value(TheValue::Float(1.0));
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
                    rgba_layout.scroll_to(vec2i(0, 0));
                }
            }
        }

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
            if let Some(region) = region {
                if filter_role < 2 {
                    // Show Characters
                    for (id, _) in region.characters.iter() {
                        let mut name = "Character".to_string();
                        if let Some((TheValue::Text(text), _)) =
                            server.get_character_property(region.id, *id, "name".to_string())
                        {
                            name = text;
                        }
                        if filter_text.is_empty() || name.to_lowercase().contains(&filter_text) {
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
                    // Show Items
                    for (id, _) in region.items.iter() {
                        let mut name = "Item".to_string();
                        if let Some((TheValue::Text(text), _)) =
                            server.get_item_property(region.id, *id, "name".to_string())
                        {
                            name = text;
                        }
                        if filter_text.is_empty() || name.to_lowercase().contains(&filter_text) {
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

                if filter_role == 0 || filter_role == 2 {
                    // Show Areas
                    for (id, area) in region.areas.iter() {
                        let name = area.name.clone();
                        if filter_text.is_empty() || name.to_lowercase().contains(&filter_text) {
                            let mut item = TheListItem::new(TheId::named_with_id(
                                "Region Content List Item",
                                *id,
                            ));
                            item.set_text(name);
                            item.add_value_column(100, TheValue::Text("Area".to_string()));
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Delete Area...".to_string(),
                                    TheId::named("Sidebar Delete Area"),
                                )],
                                ..Default::default()
                            }));
                            list.add_item(item, ctx);
                        }
                    }
                }
            }
        }
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
        }
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
                                || tile.role
                                    == TileRole::from_index(filter_role as u8 - 1).unwrap())
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

    /// Shows the filtered models of the project.
    pub fn show_filtered_models(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        let mut filter_text = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Model Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let _filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Model Filter Role".to_string()), None)
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

        if let Some(layout) = ui.canvas.get_layout(Some(&"Model List".to_string()), None) {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();
                for model in project.models.values() {
                    if filter_text.is_empty() || model.name.to_lowercase().contains(&filter_text)
                    //&& (filter_role == 0
                    //    || tile.role == TileRole::from_index(filter_role as u8 - 1).unwrap())
                    {
                        let mut item =
                            TheListItem::new(TheId::named_with_id("Model Item", model.id));
                        item.set_text(model.name.clone());
                        let sub_text =
                            format!("{}. {} Nodes", model.nodes[0].name(), model.nodes.len());
                        item.set_sub_text(sub_text);
                        item.set_size(42);
                        if Some(model.id) == server_ctx.curr_material_object {
                            item.set_state(TheWidgetState::Selected);
                        }
                        let mut buffer = TheRGBABuffer::new(TheDim::sized(36, 36));
                        model.preview_2d(
                            &mut buffer,
                            &project.palette,
                            &TILEDRAWER.lock().unwrap().tiles,
                        );
                        item.set_icon(buffer);
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Model...".to_string(),
                                TheId::named("Rename Model"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                    }
                }
            }
        }
        //ui.select_first_list_item("Material List", ctx);
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
            .get_widget(Some(&"Material Filter Edit".to_string()), None)
        {
            widget.value().to_string().unwrap_or_default()
        } else {
            "".to_string()
        };

        let _filter_role = if let Some(widget) = ui
            .canvas
            .get_widget(Some(&"Material Filter Role".to_string()), None)
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
            .get_layout(Some(&"Material List".to_string()), None)
        {
            if let Some(list_layout) = layout.as_list_layout() {
                list_layout.clear();
                for (index, material) in project.materials.values().enumerate() {
                    if filter_text.is_empty() || material.name.to_lowercase().contains(&filter_text)
                    //&& (filter_role == 0
                    //    || tile.role == TileRole::from_index(filter_role as u8 - 1).unwrap())
                    {
                        let mut item =
                            TheListItem::new(TheId::named_with_id("Material Item", material.id));
                        item.set_text(material.name.clone());
                        let sub_text = format!("Index: {}", index);
                        item.set_sub_text(sub_text);
                        item.set_size(42);
                        if Some(material.id) == server_ctx.curr_material_object {
                            item.set_state(TheWidgetState::Selected);
                        }
                        item.set_icon(material.get_preview().scaled(36, 36));
                        item.set_context_menu(Some(TheContextMenu {
                            items: vec![TheContextMenuItem::new(
                                "Rename Material...".to_string(),
                                TheId::named("Rename Material"),
                            )],
                            ..Default::default()
                        }));
                        list_layout.add_item(item, ctx);
                    }
                }
            }
        }
        //ui.select_first_list_item("Material List", ctx);
    }

    /// Apply the given asset to the UI
    pub fn apply_asset(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext, _asset: Option<&Asset>) {}

    /// Deselects the section buttons
    pub fn deselect_sections_buttons(&mut self, ui: &mut TheUI, except: String) {
        if let Some(stack_layout) = ui.get_stack_layout("List Stack Layout") {
            // Remove code bundles UI from Character / Items / Modules
            if let Some(canvas) = stack_layout.canvas_at_mut(SidebarMode::Character as usize) {
                let mut c = TheCanvas::new();
                c.set_layout(TheListLayout::new(TheId::empty()));
                canvas.set_bottom(c);
            }
            if let Some(canvas) = stack_layout.canvas_at_mut(SidebarMode::Item as usize) {
                let mut c = TheCanvas::new();
                c.set_layout(TheListLayout::new(TheId::empty()));
                canvas.set_bottom(c);
            }
            if let Some(canvas) = stack_layout.canvas_at_mut(SidebarMode::Module as usize) {
                let mut c = TheCanvas::new();
                c.set_layout(TheListLayout::new(TheId::empty()));
                canvas.set_bottom(c);
            }
            if let Some(canvas) = stack_layout.canvas_at_mut(SidebarMode::Screen as usize) {
                let mut c = TheCanvas::new();
                let mut layout = TheListLayout::new(TheId::empty());
                layout.limiter_mut().set_max_height(200);
                c.set_layout(layout);
                canvas.set_bottom(c);
            }
        }

        if let Some(layout) = ui.canvas.get_layout(Some(&"Section Buttons".into()), None) {
            for w in layout.widgets() {
                if !w.id().name.starts_with(&except) {
                    w.set_state(TheWidgetState::None);
                }
            }
        }
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
    }

    /// Tilemaps in the project have been updated, propagate the change to all relevant parties.
    pub fn update_tiles(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        client: &mut Client,
    ) {
        let tiles = project.extract_tiles();
        TILEDRAWER.lock().unwrap().set_tiles(tiles.clone());
        MAPRENDER.lock().unwrap().set_textures(tiles.clone());
        server.update_tiles(tiles.clone());
        client.update_tiles(tiles);

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
    }
}
