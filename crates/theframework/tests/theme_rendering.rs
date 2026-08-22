#![cfg(feature = "ui")]

use std::path::PathBuf;
use theframework::prelude::*;

const WIDTH: usize = 160;
const HEIGHT: usize = 112;

// FNV-1a of the approved offscreen gallery. A mismatch writes an inspectable PNG under target/.
// A few widgets contain platform-sensitive rasterization, so keep the Ubuntu CI baseline explicit
// instead of treating a macOS-approved pixel hash as portable.
const BLACK_BLUE_GALLERY_HASH: u64 = 0x2c4e55942b872e02;
const CONVERTED_BARS_HASH: u64 = 0x92b7c2dd9f940125;
const MIGRATED_NAVIGATION_BARS_HASH: u64 = 0xb9e662a442a6218f;
#[cfg(not(target_os = "linux"))]
const MIGRATED_SECTION_AND_TOOL_BUTTONS_HASH: u64 = 0xe35993c01dee13ee;
#[cfg(target_os = "linux")]
const MIGRATED_SECTION_AND_TOOL_BUTTONS_HASH: u64 = 0x2d6d6d263a9f04da;
#[cfg(not(target_os = "linux"))]
const MIGRATED_DROPDOWNS_HASH: u64 = 0x91e1b57fdee61a9b;
#[cfg(target_os = "linux")]
const MIGRATED_DROPDOWNS_HASH: u64 = 0xf169e2b9ddc2abe5;
const MIGRATED_FORM_INPUTS_HASH: u64 = 0xcecf2ad3f0bf11c9;
const MIGRATED_SCROLLBARS_AND_CHECKBOXES_HASH: u64 = 0x5ddaa670b2a75434;
#[cfg(not(target_os = "linux"))]
const ACTION_GROUP_PALETTE_HASH: u64 = 0xe46a0a6e5bab9f09;
#[cfg(target_os = "linux")]
const ACTION_GROUP_PALETTE_HASH: u64 = 0x4291d64b431c2ac7;
#[cfg(not(target_os = "linux"))]
const MIGRATED_SNAPPER_STATES_HASH: u64 = 0xf4ee3df45fa00697;
#[cfg(target_os = "linux")]
const MIGRATED_SNAPPER_STATES_HASH: u64 = 0x572f0a4fc15cfcf6;
#[cfg(not(target_os = "linux"))]
const MIGRATED_MENUS_AND_BUTTONS_HASH: u64 = 0x209e904869906b7d;
#[cfg(target_os = "linux")]
const MIGRATED_MENUS_AND_BUTTONS_HASH: u64 = 0x2f7650ebca1361b3;
const MIGRATED_CONTEXT_MENU_HASH: u64 = 0x178ee60a508da471;
const MIGRATED_SLIDER_STATES_HASH: u64 = 0xb601728df6cf6dba;
const MIGRATED_TIME_SLIDER_STATES_HASH: u64 = 0xd6e3da09a3b8d64b;
#[cfg(not(target_os = "linux"))]
const MIGRATED_NODE_CANVAS_HASH: u64 = 0x26165750ca03396f;
#[cfg(target_os = "linux")]
const MIGRATED_NODE_CANVAS_HASH: u64 = 0xfa7a01ae4d6781fd;

#[test]
fn black_blue_theme_gallery_matches_snapshot() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(WIDTH as i32, HEIGHT as i32));
    render_black_blue_gallery(&mut buffer);

    let actual = fnv1a64(buffer.pixels());
    if actual != BLACK_BLUE_GALLERY_HASH {
        let output = snapshot_failure_path();
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(&output, buffer.to_png().unwrap()).unwrap();
        panic!(
            "black/blue gallery changed: expected {BLACK_BLUE_GALLERY_HASH:#018x}, actual {actual:#018x}; inspect {}",
            output.display()
        );
    }
}

#[test]
fn converted_widget_bars_match_snapshot_without_png_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(128, 43));
    let mut ctx = TheContext::new(128, 43, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut toolbar = TheToolbar::new(TheId::named("Snapshot Toolbar"));
    toolbar.set_dim(TheDim::rect(0, 0, 128, 22), &mut ctx);
    toolbar.draw(&mut buffer, &mut style, &mut ctx);

    let mut statusbar = TheStatusbar::new(TheId::named("Snapshot Statusbar"));
    statusbar.set_dim(TheDim::rect(0, 22, 128, 21), &mut ctx);
    statusbar.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        CONVERTED_BARS_HASH,
        "converted toolbar/statusbar",
        "converted-bars.actual.png",
    );
}

#[test]
fn migrated_navigation_bars_match_snapshot_without_skin_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(300, 64));
    let mut ctx = TheContext::new(300, 64, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut switchbar = TheSwitchbar::new(TheId::named("Snapshot Switchbar"));
    switchbar.set_dim(TheDim::rect(0, 0, 300, 21), &mut ctx);
    switchbar.draw(&mut buffer, &mut style, &mut ctx);

    let mut sectionbar = TheSectionbar::new(TheId::named("Snapshot Sectionbar"));
    sectionbar.set_dim(TheDim::rect(0, 21, 300, 21), &mut ctx);
    sectionbar.draw(&mut buffer, &mut style, &mut ctx);

    let mut tabbar = TheTabbar::new(TheId::named("Snapshot Tabbar"));
    tabbar.add_tab("Selected".to_string());
    tabbar.add_tab("Hovered".to_string());
    tabbar.set_dim(TheDim::rect(0, 42, 300, 22), &mut ctx);
    tabbar.on_event(&TheEvent::Hover(Vec2::new(150, 10)), &mut ctx);
    tabbar.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_NAVIGATION_BARS_HASH,
        "migrated switchbar/sectionbar/tabbar",
        "migrated-navigation-bars.actual.png",
    );
}

#[test]
fn migrated_section_and_tool_buttons_match_snapshot() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(260, 122));
    let mut ctx = TheContext::new(260, 122, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut normal = TheSectionbarButton::new(TheId::named("Normal Section"));
    normal.set_text("Normal".to_string());
    normal.set_dim(TheDim::rect(0, 0, 81, 47), &mut ctx);

    let mut hovered = TheSectionbarButton::new(TheId::named("Hovered Section"));
    hovered.set_text("Hovered".to_string());
    hovered.set_dim(TheDim::rect(84, 0, 81, 47), &mut ctx);
    hovered.on_event(&TheEvent::Hover(Vec2::zero()), &mut ctx);

    let mut selected = TheSectionbarButton::new(TheId::named("Selected Section"));
    selected.set_text("Selected".to_string());
    selected.set_state(TheWidgetState::Selected);
    selected.set_dim(TheDim::rect(168, 0, 81, 47), &mut ctx);

    normal.draw(&mut buffer, &mut style, &mut ctx);
    hovered.draw(&mut buffer, &mut style, &mut ctx);
    selected.draw(&mut buffer, &mut style, &mut ctx);

    let mut tool_bar = TheToolListBar::new(TheId::named("Tool List Bar"));
    tool_bar.set_dim(TheDim::rect(0, 50, 260, 23), &mut ctx);
    tool_bar.draw(&mut buffer, &mut style, &mut ctx);

    let mut tool_normal = TheToolListButton::new(TheId::named("Normal Tool"));
    tool_normal.set_dim(TheDim::rect(0, 76, 46, 43), &mut ctx);
    let mut tool_hover = TheToolListButton::new(TheId::named("Hovered Tool"));
    tool_hover.set_dim(TheDim::rect(49, 76, 46, 43), &mut ctx);
    tool_hover.on_event(&TheEvent::Hover(Vec2::zero()), &mut ctx);
    let mut tool_selected = TheToolListButton::new(TheId::named("Selected Tool"));
    tool_selected.set_state(TheWidgetState::Selected);
    tool_selected.set_dim(TheDim::rect(98, 76, 46, 43), &mut ctx);

    tool_normal.draw(&mut buffer, &mut style, &mut ctx);
    tool_hover.draw(&mut buffer, &mut style, &mut ctx);
    tool_selected.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_SECTION_AND_TOOL_BUTTONS_HASH,
        "migrated section and tool buttons",
        "migrated-section-tool-buttons.actual.png",
    );
}

#[test]
fn migrated_dropdown_states_match_snapshot() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(300, 72));
    let mut ctx = TheContext::new(300, 72, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut normal = snapshot_dropdown("Normal", TheDim::rect(0, 0, 142, 20), &mut ctx);
    let mut hovered = snapshot_dropdown("Hovered", TheDim::rect(150, 0, 142, 20), &mut ctx);
    hovered.on_event(&TheEvent::Hover(Vec2::zero()), &mut ctx);
    let mut focused = snapshot_dropdown("Focused", TheDim::rect(0, 25, 142, 20), &mut ctx);
    ctx.ui.set_focus(focused.id());
    let mut pressed = snapshot_dropdown("Pressed", TheDim::rect(150, 25, 142, 20), &mut ctx);
    pressed.set_state(TheWidgetState::Clicked);
    let mut disabled = snapshot_dropdown("Disabled", TheDim::rect(0, 50, 142, 20), &mut ctx);
    disabled.set_disabled(true);
    let mut palette = ThePaletteIndexPicker::new(TheId::named("Palette Dropdown"));
    palette.set_dim(TheDim::rect(150, 50, 142, 20), &mut ctx);

    normal.draw(&mut buffer, &mut style, &mut ctx);
    hovered.draw(&mut buffer, &mut style, &mut ctx);
    focused.draw(&mut buffer, &mut style, &mut ctx);
    pressed.draw(&mut buffer, &mut style, &mut ctx);
    disabled.draw(&mut buffer, &mut style, &mut ctx);
    palette.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_DROPDOWNS_HASH,
        "migrated dropdown states",
        "migrated-dropdowns.actual.png",
    );
}

#[test]
fn migrated_form_input_states_match_snapshot() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(300, 108));
    let mut ctx = TheContext::new(300, 108, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut normal = TheTextLineEdit::new(TheId::named("Normal Input"));
    normal.set_text("Normal input".to_string());
    normal.set_dim(TheDim::rect(0, 0, 142, 20), &mut ctx);
    normal.draw(&mut buffer, &mut style, &mut ctx);

    let mut focused = TheTextLineEdit::new(TheId::named("Focused Input"));
    focused.set_text("Focused input".to_string());
    focused.set_dim(TheDim::rect(150, 0, 142, 20), &mut ctx);
    ctx.ui.set_focus(focused.id());
    focused.draw(&mut buffer, &mut style, &mut ctx);

    let mut disabled = TheTextLineEdit::new(TheId::named("Disabled Input"));
    disabled.set_text("Disabled input".to_string());
    disabled.set_disabled(true);
    disabled.set_dim(TheDim::rect(0, 25, 142, 20), &mut ctx);
    disabled.draw(&mut buffer, &mut style, &mut ctx);

    ctx.ui.clear_focus();
    let mut normal_area = TheTextAreaEdit::new(TheId::named("Normal Area"));
    normal_area.set_text("Normal area".to_string());
    normal_area.set_dim(TheDim::rect(0, 50, 142, 55), &mut ctx);
    normal_area.draw(&mut buffer, &mut style, &mut ctx);

    let mut focused_area = TheTextAreaEdit::new(TheId::named("Focused Area"));
    focused_area.set_text("Focused area".to_string());
    focused_area.set_dim(TheDim::rect(150, 50, 142, 55), &mut ctx);
    ctx.ui.set_focus(focused_area.id());
    focused_area.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_FORM_INPUTS_HASH,
        "migrated form input states",
        "migrated-form-inputs.actual.png",
    );
}

#[test]
fn migrated_scrollbars_and_checkboxes_match_snapshot() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(260, 96));
    let mut ctx = TheContext::new(260, 96, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut horizontal = TheHorizontalScrollbar::new(TheId::named("Horizontal Normal"));
    horizontal.set_total_width(320);
    horizontal.set_scroll_offset(54);
    horizontal.set_dim(TheDim::rect(0, 0, 140, 13), &mut ctx);
    horizontal.draw(&mut buffer, &mut style, &mut ctx);

    let mut horizontal_hover = TheHorizontalScrollbar::new(TheId::named("Horizontal Hover"));
    horizontal_hover.set_total_width(320);
    horizontal_hover.set_scroll_offset(90);
    horizontal_hover.set_dim(TheDim::rect(0, 18, 140, 13), &mut ctx);
    horizontal_hover.on_event(&TheEvent::Hover(Vec2::new(42, 6)), &mut ctx);
    horizontal_hover.draw(&mut buffer, &mut style, &mut ctx);

    let mut horizontal_pressed = TheHorizontalScrollbar::new(TheId::named("Horizontal Pressed"));
    horizontal_pressed.set_total_width(320);
    horizontal_pressed.set_scroll_offset(130);
    horizontal_pressed.set_state(TheWidgetState::Clicked);
    horizontal_pressed.set_dim(TheDim::rect(0, 36, 140, 13), &mut ctx);
    horizontal_pressed.draw(&mut buffer, &mut style, &mut ctx);

    let mut vertical = TheVerticalScrollbar::new(TheId::named("Vertical Normal"));
    vertical.set_total_height(260);
    vertical.set_scroll_offset(82);
    vertical.set_dim(TheDim::rect(150, 0, 13, 92), &mut ctx);
    vertical.draw(&mut buffer, &mut style, &mut ctx);

    let mut vertical_pressed = TheVerticalScrollbar::new(TheId::named("Vertical Pressed"));
    vertical_pressed.set_total_height(260);
    vertical_pressed.set_scroll_offset(120);
    vertical_pressed.set_state(TheWidgetState::Clicked);
    vertical_pressed.set_dim(TheDim::rect(168, 0, 13, 92), &mut ctx);
    vertical_pressed.draw(&mut buffer, &mut style, &mut ctx);

    let mut checkbox = TheCheckButton::new(TheId::named("Checkbox Normal"));
    checkbox.set_dim(TheDim::rect(200, 2, 16, 18), &mut ctx);
    checkbox.draw(&mut buffer, &mut style, &mut ctx);

    let mut checkbox_selected = TheCheckButton::new(TheId::named("Checkbox Selected"));
    checkbox_selected.set_state(TheWidgetState::Selected);
    checkbox_selected.set_dim(TheDim::rect(220, 2, 16, 18), &mut ctx);
    checkbox_selected.draw(&mut buffer, &mut style, &mut ctx);

    let mut checkbox_hover = TheCheckButton::new(TheId::named("Checkbox Hover"));
    checkbox_hover.set_dim(TheDim::rect(240, 2, 16, 18), &mut ctx);
    checkbox_hover.on_event(&TheEvent::Hover(Vec2::new(5, 5)), &mut ctx);
    checkbox_hover.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_SCROLLBARS_AND_CHECKBOXES_HASH,
        "migrated scrollbars and checkboxes",
        "migrated-scrollbars-checkboxes.actual.png",
    );
}

#[test]
fn migrated_snapper_states_match_snapshot_without_skin_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(300, 96));
    let mut ctx = TheContext::new(300, 96, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut normal = TheSnapperbar::new(TheId::named("Normal Snapper"));
    normal.set_text("Normal closed".to_string());
    normal.set_dim(TheDim::rect(0, 0, 145, 22), &mut ctx);
    normal.draw(&mut buffer, &mut style, &mut ctx);

    let mut hovered = TheSnapperbar::new(TheId::named("Hovered Snapper"));
    hovered.set_text("Hover open".to_string());
    hovered.set_open(true);
    hovered.set_dim(TheDim::rect(150, 0, 145, 22), &mut ctx);
    hovered.on_event(&TheEvent::Hover(Vec2::zero()), &mut ctx);
    hovered.draw(&mut buffer, &mut style, &mut ctx);

    let mut pressed = TheSnapperbar::new(TheId::named("Pressed Snapper"));
    pressed.set_text("Pressed".to_string());
    pressed.set_state(TheWidgetState::Clicked);
    pressed.set_dim(TheDim::rect(0, 24, 145, 22), &mut ctx);
    pressed.draw(&mut buffer, &mut style, &mut ctx);

    let mut selected = TheSnapperbar::new(TheId::named("Selected Snapper"));
    selected.set_text("Selected".to_string());
    selected.set_selected(true);
    selected.set_open(true);
    selected.set_dim(TheDim::rect(150, 24, 145, 22), &mut ctx);
    selected.draw(&mut buffer, &mut style, &mut ctx);

    let mut palette_a = TheSnapperbar::new(TheId::named("Palette A Snapper"));
    palette_a.set_text("Action group 1".to_string());
    palette_a.set_root_mode(false);
    palette_a.set_background_palette(ActionGroups, 0);
    palette_a.set_dim(TheDim::rect(0, 48, 145, 22), &mut ctx);
    palette_a.draw(&mut buffer, &mut style, &mut ctx);

    let mut palette_b = TheSnapperbar::new(TheId::named("Palette B Snapper"));
    palette_b.set_text("Action group 7".to_string());
    palette_b.set_root_mode(false);
    palette_b.set_background_palette(ActionGroups, 6);
    palette_b.set_open(true);
    palette_b.set_dim(TheDim::rect(150, 48, 145, 22), &mut ctx);
    palette_b.draw(&mut buffer, &mut style, &mut ctx);

    let mut child = TheSnapperbar::new(TheId::named("Child Snapper"));
    child.set_text("Nested normal".to_string());
    child.set_root_mode(false);
    child.set_dim(TheDim::rect(0, 72, 145, 22), &mut ctx);
    child.draw(&mut buffer, &mut style, &mut ctx);

    let mut future_palette = TheSnapperbar::new(TheId::named("Future Palette Snapper"));
    future_palette.set_text("Future group".to_string());
    future_palette.set_root_mode(false);
    future_palette.set_background_palette(ActionGroups, 10);
    future_palette.set_dim(TheDim::rect(150, 72, 145, 22), &mut ctx);
    future_palette.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_SNAPPER_STATES_HASH,
        "migrated snapper states",
        "migrated-snapper-states.actual.png",
    );
}

#[test]
fn migrated_menus_and_buttons_match_snapshot_without_skin_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(320, 120));
    let mut ctx = TheContext::new(320, 120, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut hovered_menu = snapshot_menu("Hovered Menu", TheDim::rect(0, 0, 320, 22), &mut ctx);
    hovered_menu.on_event(&TheEvent::Hover(Vec2::new(45, 10)), &mut ctx);
    hovered_menu.draw(&mut buffer, &mut style, &mut ctx);

    let mut selected_menu = snapshot_menu("Selected Menu", TheDim::rect(0, 24, 320, 22), &mut ctx);
    selected_menu.on_event(&TheEvent::MouseDown(Vec2::new(125, 10)), &mut ctx);
    selected_menu.draw(&mut buffer, &mut style, &mut ctx);

    let mut menubar = TheMenubar::new(TheId::named("Snapshot Menubar"));
    menubar.set_dim(TheDim::rect(0, 48, 320, 43), &mut ctx);
    menubar.draw(&mut buffer, &mut style, &mut ctx);

    let mut menubar_hover = TheMenubarButton::new(TheId::named("Menubar Hover"));
    menubar_hover.set_dim(TheDim::rect(10, 52, 35, 35), &mut ctx);
    menubar_hover.on_event(&TheEvent::Hover(Vec2::zero()), &mut ctx);
    menubar_hover.draw(&mut buffer, &mut style, &mut ctx);

    let mut menubar_pressed = TheMenubarButton::new(TheId::named("Menubar Pressed"));
    menubar_pressed.set_state(TheWidgetState::Clicked);
    menubar_pressed.set_dim(TheDim::rect(50, 52, 35, 35), &mut ctx);
    menubar_pressed.draw(&mut buffer, &mut style, &mut ctx);

    let mut separator = TheMenubarSeparator::new(TheId::named("Menubar Separator"));
    separator.set_dim(TheDim::rect(90, 53, 10, 33), &mut ctx);
    separator.draw(&mut buffer, &mut style, &mut ctx);

    let mut tray_normal = snapshot_tray_button("Normal", TheDim::rect(0, 96, 74, 20), &mut ctx);
    tray_normal.draw(&mut buffer, &mut style, &mut ctx);

    let mut tray_hover = snapshot_tray_button("Hover", TheDim::rect(80, 96, 74, 20), &mut ctx);
    tray_hover.on_event(&TheEvent::Hover(Vec2::zero()), &mut ctx);
    tray_hover.draw(&mut buffer, &mut style, &mut ctx);

    let mut tray_pressed = snapshot_tray_button("Pressed", TheDim::rect(160, 96, 74, 20), &mut ctx);
    tray_pressed.set_state(TheWidgetState::Clicked);
    tray_pressed.draw(&mut buffer, &mut style, &mut ctx);

    let mut tray_disabled =
        snapshot_tray_button("Disabled", TheDim::rect(240, 96, 74, 20), &mut ctx);
    tray_disabled.set_disabled(true);
    tray_disabled.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_MENUS_AND_BUTTONS_HASH,
        "migrated menus and buttons",
        "migrated-menus-buttons.actual.png",
    );
}

#[test]
fn migrated_context_menu_matches_snapshot_without_skin_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(390, 100));
    let mut ctx = TheContext::new(390, 100, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut first_child = TheContextMenu::named("First Child".to_string());
    first_child.width = 180;
    first_child.add(TheContextMenuItem::new(
        "Nested action".to_string(),
        TheId::named("Nested Action"),
    ));

    let mut hovered_child = TheContextMenu::named("Hovered Child".to_string());
    hovered_child.width = 180;
    hovered_child.add(TheContextMenuItem::new(
        "Visible child action".to_string(),
        TheId::named("Visible Child Action"),
    ));

    let mut menu = TheContextMenu::named("Snapshot Context Menu".to_string());
    menu.width = 180;
    menu.add(TheContextMenuItem::new_submenu(
        "Normal submenu".to_string(),
        TheId::named("Normal Submenu"),
        first_child,
    ));
    menu.add(TheContextMenuItem::new_submenu(
        "Hovered submenu".to_string(),
        TheId::named("Hovered Submenu"),
        hovered_child,
    ));
    menu.set_position(Vec2::new(4, 4), &mut ctx);
    menu.on_event(&TheEvent::Hover(Vec2::new(20, 40)), &mut ctx);
    menu.draw(buffer.pixels_mut(), &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_CONTEXT_MENU_HASH,
        "migrated context menu",
        "migrated-context-menu.actual.png",
    );
}

#[test]
fn migrated_slider_states_match_snapshot_without_skin_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(300, 36));
    let mut ctx = TheContext::new(300, 36, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut normal = snapshot_slider("Normal Slider", TheDim::rect(0, 0, 142, 13), &mut ctx);
    normal.set_value(TheValue::Float(0.25));
    normal.draw(&mut buffer, &mut style, &mut ctx);

    let mut hovered = snapshot_slider("Hovered Slider", TheDim::rect(150, 0, 142, 13), &mut ctx);
    hovered.set_value(TheValue::Float(0.5));
    hovered.on_event(&TheEvent::Hover(Vec2::zero()), &mut ctx);
    hovered.draw(&mut buffer, &mut style, &mut ctx);

    let mut pressed = snapshot_slider("Pressed Slider", TheDim::rect(0, 18, 142, 13), &mut ctx);
    pressed.set_value(TheValue::Float(0.75));
    pressed.set_state(TheWidgetState::Selected);
    pressed.draw(&mut buffer, &mut style, &mut ctx);

    let mut embedded = TheSlider::new(TheId::named("Embedded Integer Slider"));
    embedded.set_range(TheValue::RangeI32(0..=10));
    embedded.set_value(TheValue::Int(4));
    embedded.set_embedded(true);
    embedded.set_state(TheWidgetState::Selected);
    embedded.set_dim(TheDim::rect(150, 18, 142, 13), &mut ctx);
    embedded.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_SLIDER_STATES_HASH,
        "migrated slider states",
        "migrated-slider-states.actual.png",
    );
}

#[test]
fn migrated_time_slider_sizes_match_snapshot_without_skin_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(320, 55));
    let mut ctx = TheContext::new(320, 55, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut compact = TheTimeSlider::new(TheId::named("Compact Time Slider"));
    compact.set_value(TheValue::Time(TheTime::new_time(13, 20).unwrap()));
    compact.add_marker(TheTime::new_time(6, 0).unwrap(), vec!["Dawn".to_string()]);
    compact.add_marker(
        TheTime::new_time(17, 45).unwrap(),
        vec!["Evening".to_string()],
    );
    compact.set_dim(TheDim::rect(0, 0, 320, 20), &mut ctx);
    compact.draw(&mut buffer, &mut style, &mut ctx);

    let mut tall = TheTimeSlider::new(TheId::named("Menubar Time Slider"));
    tall.set_tall(true);
    assert_eq!(tall.limiter().get_max_height(), 27);
    tall.set_value(TheValue::Time(TheTime::new_time(13, 20).unwrap()));
    tall.add_marker(TheTime::new_time(6, 0).unwrap(), vec!["Dawn".to_string()]);
    tall.add_marker(
        TheTime::new_time(17, 45).unwrap(),
        vec!["Evening".to_string()],
    );
    tall.set_dim(TheDim::rect(0, 26, 320, 27), &mut ctx);
    tall.on_event(&TheEvent::MouseDown(Vec2::new(70, 10)), &mut ctx);
    tall.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_TIME_SLIDER_STATES_HASH,
        "migrated compact and menubar time sliders",
        "migrated-time-slider-states.actual.png",
    );
}

#[test]
fn migrated_node_canvas_matches_snapshot_without_skin_assets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(320, 224));
    let mut ctx = TheContext::new(320, 224, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    let mut canvas = TheNodeCanvas::new();
    canvas.node_width = 136;
    canvas
        .categories
        .insert("Data".to_string(), TheColor::from_u8(73, 169, 222, 255));
    canvas
        .categories
        .insert("Flow".to_string(), TheColor::from_u8(220, 151, 62, 255));
    canvas
        .nodes
        .push(snapshot_node("Source", Vec2::new(12, 18), false));

    let mut preview = TheRGBABuffer::new(TheDim::sized(76, 68));
    preview.fill([47, 57, 72, 255]);
    let mut selected = snapshot_node("Selected", Vec2::new(170, 22), true);
    selected.preview = preview;
    selected.outputs.push(TheNodeTerminal {
        name: "Flow".to_string(),
        category_name: "Flow".to_string(),
    });
    canvas.nodes.push(selected);
    canvas.connections.push((0, 0, 1, 0));
    canvas.selected_node = Some(1);

    let mut view = TheNodeCanvasView::new(TheId::named("Snapshot Node Canvas"));
    view.set_canvas(canvas);
    view.set_dim(TheDim::rect(0, 0, 320, 224), &mut ctx);
    view.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        MIGRATED_NODE_CANVAS_HASH,
        "migrated node canvas",
        "migrated-node-canvas.actual.png",
    );
}

#[test]
fn action_group_palette_resolves_in_tree_widgets() {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(260, 120));
    let mut ctx = TheContext::new(260, 120, 1.0);
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
        TheBlackBlueTheme::new(),
    )));

    for (index, label) in ["Camera actions", "Editor actions", "Dock actions"]
        .into_iter()
        .enumerate()
    {
        let mut item = TheTreeItem::new(TheId::named(label));
        item.set_text(label.to_string());
        item.set_background_palette(ActionGroups, index);
        item.set_dim(TheDim::rect(0, index as i32 * 24, 190, 22), &mut ctx);
        item.draw(&mut buffer, &mut style, &mut ctx);
    }

    let mut action_item = TheListItem::new(TheId::named("Action list item"));
    action_item.set_text("Action list item".to_string());
    action_item.set_background_palette(ActionGroups, 3);
    action_item.set_dim(TheDim::rect(0, 72, 190, 22), &mut ctx);
    action_item.draw(&mut buffer, &mut style, &mut ctx);

    let mut future_group = TheSnapperbar::new(TheId::named("Future action group"));
    future_group.set_text("Future action group".to_string());
    future_group.set_root_mode(false);
    future_group.set_background_palette(ActionGroups, 6);
    future_group.set_dim(TheDim::rect(0, 96, 260, 22), &mut ctx);
    future_group.draw(&mut buffer, &mut style, &mut ctx);

    assert_snapshot(
        &buffer,
        ACTION_GROUP_PALETTE_HASH,
        "action-group palette",
        "action-group-palette.actual.png",
    );
}

fn snapshot_dropdown(name: &str, dim: TheDim, ctx: &mut TheContext) -> TheDropdownMenu {
    let mut dropdown = TheDropdownMenu::new(TheId::named(name));
    dropdown.add_option(name.to_string());
    dropdown.set_dim(dim, ctx);
    dropdown
}

fn snapshot_menu(name: &str, dim: TheDim, ctx: &mut TheContext) -> TheMenu {
    let mut menu = TheMenu::new(TheId::named(name));
    menu.add_context_menu(TheContextMenu::named("File".to_string()));
    menu.add_context_menu(TheContextMenu::named("Edit".to_string()));
    menu.add_context_menu(TheContextMenu::named("Game".to_string()));
    menu.set_dim(dim, ctx);
    menu
}

fn snapshot_tray_button(name: &str, dim: TheDim, ctx: &mut TheContext) -> TheTraybarButton {
    let mut button = TheTraybarButton::new(TheId::named(&format!("Tray {name}")));
    button.set_text(name.to_string());
    button.set_dim(dim, ctx);
    button
}

fn snapshot_slider(name: &str, dim: TheDim, ctx: &mut TheContext) -> TheSlider {
    let mut slider = TheSlider::new(TheId::named(name));
    slider.set_range(TheValue::RangeF32(0.0..=1.0));
    slider.set_dim(dim, ctx);
    slider
}

fn snapshot_node(name: &str, position: Vec2<i32>, preview_open: bool) -> TheNode {
    TheNode {
        name: name.to_string(),
        status_text: None,
        position,
        inputs: vec![TheNodeTerminal {
            name: "Input".to_string(),
            category_name: "Data".to_string(),
        }],
        outputs: vec![TheNodeTerminal {
            name: "Output".to_string(),
            category_name: "Data".to_string(),
        }],
        preview: TheRGBABuffer::new(TheDim::zero()),
        supports_preview: preview_open,
        preview_is_open: preview_open,
        can_be_deleted: true,
    }
}

fn render_black_blue_gallery(buffer: &mut TheRGBABuffer) {
    let mut surface = TheSurfaceMut::new(buffer.pixels_mut(), WIDTH, HEIGHT).unwrap();
    let mut painter = ThePainter::new();

    surface.fill_rect(
        ThePixelRect::new(0, 0, WIDTH as i32, HEIGHT as i32),
        [12, 13, 15, 255],
    );

    painter.fill_round_rect(
        &mut surface,
        ThePixelRect::new(5, 5, 150, 22),
        3.0,
        &ThePaint::linear_gradient([0.0, 5.0], [0.0, 27.0], [74, 77, 81, 255], [7, 8, 10, 255]),
    );
    stroke_round_rect(
        &mut painter,
        &mut surface,
        ThePixelRect::new(5, 5, 150, 22),
        3.0,
        ThePathStroke::new(1.0, ThePaint::solid([104, 107, 111, 255])),
    );

    // Normal, hover, pressed/selected and inactive controls.
    let controls = [
        ([39, 41, 44, 255], [86, 89, 94, 255]),
        ([55, 58, 62, 255], [129, 133, 139, 255]),
        ([55, 72, 101, 255], [91, 120, 163, 255]),
        ([31, 32, 34, 255], [54, 55, 58, 255]),
    ];
    for (index, (bottom, top)) in controls.into_iter().enumerate() {
        let x = 7 + index as i32 * 38;
        painter.fill_round_rect(
            &mut surface,
            ThePixelRect::new(x, 36, 33, 18),
            2.5,
            &ThePaint::linear_gradient([0.0, 36.0], [0.0, 54.0], top, bottom),
        );
        stroke_round_rect(
            &mut painter,
            &mut surface,
            ThePixelRect::new(x, 36, 33, 18),
            2.5,
            ThePathStroke::new(1.0, ThePaint::solid([95, 98, 102, 255])),
        );
    }

    // Selected list row and bright focus ring are intentionally separate semantic roles.
    painter.fill_round_rect(
        &mut surface,
        ThePixelRect::new(7, 63, 107, 17),
        1.5,
        &ThePaint::solid([57, 75, 105, 255]),
    );
    painter.fill_round_rect(
        &mut surface,
        ThePixelRect::new(7, 84, 107, 18),
        2.0,
        &ThePaint::solid([38, 40, 44, 255]),
    );
    stroke_round_rect(
        &mut painter,
        &mut surface,
        ThePixelRect::new(7, 84, 107, 18),
        2.0,
        ThePathStroke::new(1.5, ThePaint::solid([83, 151, 207, 255])),
    );

    // Accent/notification role from the supplied reference remains independent of focus blue.
    surface.fill_rect(ThePixelRect::new(121, 64, 29, 4), [220, 255, 0, 255]);
    painter.fill_circle(
        &mut surface,
        [130.0, 91.0],
        7.0,
        &ThePaint::solid([58, 161, 218, 255]),
    );

    let mut check = ThePath::new();
    check
        .move_to((126.5, 91.0))
        .line_to((129.0, 94.0))
        .line_to((134.5, 87.5));
    painter.stroke_path(
        &mut surface,
        &check,
        &ThePathStroke::new(1.75, ThePaint::solid([238, 242, 245, 255]))
            .with_cap(TheLineCap::Round)
            .with_join(TheLineJoin::Round),
    );
}

fn stroke_round_rect(
    painter: &mut ThePainter,
    surface: &mut TheSurfaceMut<'_>,
    rect: ThePixelRect,
    radius: f32,
    stroke: ThePathStroke,
) {
    let mut path = ThePath::new();
    path.add_round_rect(
        (rect.x as f32, rect.y as f32),
        rect.width as f32,
        rect.height as f32,
        radius,
        radius,
    );
    painter.stroke_path(surface, &path, &stroke);
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn snapshot_failure_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/ui-snapshots/black-blue-gallery.actual.png")
}

fn assert_snapshot(buffer: &TheRGBABuffer, expected: u64, name: &str, filename: &str) {
    let actual = fnv1a64(buffer.pixels());
    if actual != expected {
        let output = snapshot_failure_path().with_file_name(filename);
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(&output, buffer.to_png().unwrap()).unwrap();
        panic!(
            "{name} changed: expected {expected:#018x}, actual {actual:#018x}; inspect {}",
            output.display()
        );
    }
}
