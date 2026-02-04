use scenevm::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Application Context - This is where your backend/app state would live
// ============================================================================

/// Context struct that holds the actual application state.
/// This is separate from the UI state (workspace) and is what gets modified
/// by undo commands. In a real application, this would be your backend/domain model.
struct AppContext {
    workspace: Workspace,
    slider_value: f32,
    param_sliders: Vec<f32>,
}

impl AppContext {
    fn new() -> Self {
        let default = UiDemoData::default();
        Self {
            workspace: Workspace::new(),
            slider_value: default.slider_value,
            param_sliders: default.param_sliders,
        }
    }
}

// ============================================================================
// Undo Commands - Application-specific implementations
// ============================================================================

/// Command for slider value changes (supports merging)
#[derive(Debug, Clone)]
struct SliderChangeCommand {
    widget_id: String,
    old_value: f32,
    new_value: f32,
}

impl SliderChangeCommand {
    fn new(widget_id: String, old_value: f32, new_value: f32) -> Self {
        Self {
            widget_id,
            old_value,
            new_value,
        }
    }
}

impl UndoCommand<AppContext> for SliderChangeCommand {
    fn execute(&mut self, _vm: &mut SceneVM, context: &mut AppContext, is_new: bool) {
        // Only apply if this is a redo (is_new = false)
        // For new commands, the UI already updated the slider
        if !is_new {
            if self.widget_id == "main_slider" {
                context.slider_value = self.new_value;
                if let Some(slider) = context.workspace.find_view_mut::<Slider>(&self.widget_id) {
                    slider.set_value(self.new_value);
                }
                if let Some(label) = context.workspace.find_view_mut::<Label>("slider_label") {
                    label.set_text(format!("Slider Value: {:.1}", self.new_value));
                }
            } else if self.widget_id.starts_with("param_slider_") {
                if let Ok(idx_str) = self
                    .widget_id
                    .strip_prefix("param_slider_")
                    .unwrap()
                    .parse::<usize>()
                {
                    if idx_str < context.param_sliders.len() {
                        context.param_sliders[idx_str] = self.new_value;
                        if let Some(slider) =
                            context.workspace.find_view_mut::<Slider>(&self.widget_id)
                        {
                            slider.set_value(self.new_value);
                        }
                    }
                }
            }
            context.workspace.set_dirty();
        }
    }

    fn undo(&mut self, _vm: &mut SceneVM, context: &mut AppContext) {
        if self.widget_id == "main_slider" {
            context.slider_value = self.old_value;
            if let Some(slider) = context.workspace.find_view_mut::<Slider>(&self.widget_id) {
                slider.set_value(self.old_value);
            }
            if let Some(label) = context.workspace.find_view_mut::<Label>("slider_label") {
                label.set_text(format!("Slider Value: {:.1}", self.old_value));
            }
        } else if self.widget_id.starts_with("param_slider_") {
            if let Ok(idx_str) = self
                .widget_id
                .strip_prefix("param_slider_")
                .unwrap()
                .parse::<usize>()
            {
                if idx_str < context.param_sliders.len() {
                    context.param_sliders[idx_str] = self.old_value;
                    if let Some(slider) = context.workspace.find_view_mut::<Slider>(&self.widget_id)
                    {
                        slider.set_value(self.old_value);
                    }
                }
            }
        }
        context.workspace.set_dirty();
    }

    fn description(&self) -> &str {
        "Change Slider"
    }
}

/// Command for button group selection changes
#[derive(Debug, Clone)]
struct ButtonGroupChangeCommand {
    group_id: String,
    old_index: usize,
    new_index: usize,
}

impl ButtonGroupChangeCommand {
    fn new(group_id: String, old_index: usize, new_index: usize) -> Self {
        Self {
            group_id,
            old_index,
            new_index,
        }
    }
}

impl UndoCommand<AppContext> for ButtonGroupChangeCommand {
    fn execute(&mut self, _vm: &mut SceneVM, context: &mut AppContext, is_new: bool) {
        if !is_new {
            if let Some(group) = context
                .workspace
                .find_view_mut::<ButtonGroup>(&self.group_id)
            {
                group.set_active(self.new_index);
            }
            context.workspace.set_dirty();
        }
    }

    fn undo(&mut self, _vm: &mut SceneVM, context: &mut AppContext) {
        if let Some(group) = context
            .workspace
            .find_view_mut::<ButtonGroup>(&self.group_id)
        {
            group.set_active(self.old_index);
        }
        context.workspace.set_dirty();
    }

    fn description(&self) -> &str {
        "Change Button Group"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiDemoData {
    slider_value: f32,
    #[serde(default)]
    param_sliders: Vec<f32>,
}

impl Default for UiDemoData {
    fn default() -> Self {
        Self {
            slider_value: 50.0,
            param_sliders: vec![50.0, 60.0, 70.0, 80.0],
        }
    }
}

struct UiDemo {
    context: AppContext,
    renderer: UiRenderer,
    noise_layer: usize,
    popup_layer: usize,
    has_changes: bool,
    noise_tile_id: uuid::Uuid,
    noise_button_id: Option<NodeId>,
    update_noise_icon: bool,
    scale: f32,
    undo_stack: UndoStack<AppContext>,
    app_events: AppEventQueue,
}

impl UiDemo {
    fn new() -> Self {
        Self {
            context: AppContext::new(),
            renderer: UiRenderer::new(),
            noise_layer: 0,
            popup_layer: 0,
            has_changes: false,
            noise_tile_id: uuid::Uuid::new_v4(),
            noise_button_id: None,
            update_noise_icon: true,
            scale: 1.0,
            undo_stack: UndoStack::new(100), // Max 100 undo steps
            app_events: AppEventQueue::new(),
        }
    }
}

impl SceneVMApp for UiDemo {
    fn window_title(&self) -> Option<String> {
        Some("SceneVM UI Demo".into())
    }

    fn initial_window_size(&self) -> Option<(u32, u32)> {
        Some((960, 600))
    }

    fn init(&mut self, vm: &mut SceneVM, size: (u32, u32)) {
        // Create dark theme
        let theme = Theme::dark();

        // Simple background and render mode
        vm.execute(Atom::SetBackground(theme.background));
        vm.execute(Atom::SetRenderMode(RenderMode::Compute2D));
        if let Some(bytes) = Embedded::get("ui_body.wgsl") {
            if let Ok(src) = std::str::from_utf8(bytes.data.as_ref()) {
                vm.execute(Atom::SetSource2D(src.to_string()));
            }
        }

        // Create a test tile with a gradient pattern to test offset
        let test_tile_id = uuid::Uuid::new_v4();
        let tile_size = 64;
        let mut pixels = vec![0u8; tile_size * tile_size * 4];
        for y in 0..tile_size {
            for x in 0..tile_size {
                let idx = (y * tile_size + x) * 4;
                // Create a gradient pattern - red to blue gradient
                pixels[idx] = ((x as f32 / tile_size as f32) * 255.0) as u8; // R
                pixels[idx + 1] = 128; // G
                pixels[idx + 2] = ((y as f32 / tile_size as f32) * 255.0) as u8; // B
                pixels[idx + 3] = 255; // A

                // Add a white border to see the edges clearly
                if x == 0 || x == tile_size - 1 || y == 0 || y == tile_size - 1 {
                    pixels[idx] = 255;
                    pixels[idx + 1] = 255;
                    pixels[idx + 2] = 255;
                }
            }
        }
        vm.execute(Atom::AddTile {
            id: test_tile_id,
            width: tile_size as u32,
            height: tile_size as u32,
            frames: vec![pixels],
            material_frames: Some(vec![create_tile_material(
                tile_size as u32,
                tile_size as u32,
            )]),
        });

        // Create a pressed state tile with inverted gradient
        let pressed_tile_id = uuid::Uuid::new_v4();
        let mut pressed_pixels = vec![0u8; tile_size * tile_size * 4];
        for y in 0..tile_size {
            for x in 0..tile_size {
                let idx = (y * tile_size + x) * 4;
                // Inverted gradient - blue to red
                pressed_pixels[idx] = ((y as f32 / tile_size as f32) * 255.0) as u8; // R
                pressed_pixels[idx + 1] = 200; // G
                pressed_pixels[idx + 2] = ((x as f32 / tile_size as f32) * 255.0) as u8; // B
                pressed_pixels[idx + 3] = 255; // A

                // Add a yellow border for pressed state
                if x == 0 || x == tile_size - 1 || y == 0 || y == tile_size - 1 {
                    pressed_pixels[idx] = 255;
                    pressed_pixels[idx + 1] = 255;
                    pressed_pixels[idx + 2] = 0;
                }
            }
        }
        vm.execute(Atom::AddTile {
            id: pressed_tile_id,
            width: tile_size as u32,
            height: tile_size as u32,
            frames: vec![pressed_pixels],
            material_frames: Some(vec![create_tile_material(
                tile_size as u32,
                tile_size as u32,
            )]),
        });

        vm.execute(Atom::BuildAtlas);

        // Add a basic button to the workspace
        let button_rect = [40.0, 40.0, 180.0, 56.0];
        let button = Button::new(theme.button(button_rect))
            .with_id("toggle_button")
            .with_kind(ButtonKind::Toggle);
        let node = self.context.workspace.add_view(button);
        self.context.workspace.add_root(node);

        // Add a centered label inside the button
        let label = LabelRect::new("Toggle Me", button_rect, 18.0, theme.text).with_layer(16); // Layer above button (buttons are now on layer 15)
        let label_node = self.context.workspace.add_view(label);
        self.context.workspace.add_root(label_node);

        // Add an image button with the test tile and offset
        let image_button = Button::new(theme.button([250.0, 40.0, 64.0, 64.0]))
            .with_id("image_button")
            .with_kind(ButtonKind::Toggle)
            .with_tile(test_tile_id)
            .with_pressed_tile(pressed_tile_id) // Different tile when toggled
            .with_tile_offset(4.0); // 4px offset inside the button
        let image_button_node = self.context.workspace.add_view(image_button);
        self.context.workspace.add_root(image_button_node);

        // Add a plain image widget (no border, just the texture)
        let plain_image = Image::new(
            ImageStyle {
                rect: [350.0, 40.0, 64.0, 64.0],
                layer: 10,
            },
            test_tile_id,
        )
        .with_id("plain_image");
        let plain_image_node = self.context.workspace.add_view(plain_image);
        self.context.workspace.add_root(plain_image_node);

        // Add a slider below the button
        let slider = Slider::new(theme.slider([40.0, 120.0, 200.0, 32.0]), 0.0, 100.0)
            .with_id("main_slider")
            .with_value(self.context.slider_value);
        let slider_node = self.context.workspace.add_view(slider);
        self.context.workspace.add_root(slider_node);

        // Add a label for the slider (using fixed position Label, not LabelRect)
        let slider_label = Label::new(
            format!("Value: {:.1}", self.context.slider_value),
            [250.0, 126.0],
            16.0,
            theme.text,
        )
        .with_id("slider_label")
        .with_layer(10);
        let slider_label_node = self.context.workspace.add_view(slider_label);
        self.context.workspace.add_root(slider_label_node);

        // Add a toolbar with image buttons using automatic layout
        let window_width = size.0 as f32;
        let toolbar = Toolbar::new(
            theme.toolbar([0.0, 180.0, window_width, 48.0]),
            ToolbarOrientation::Horizontal,
        )
        .with_id("main_toolbar")
        .with_spacing(4.0)
        .with_padding(8.0);

        let toolbar_node = self.context.workspace.add_view(toolbar);
        self.context.workspace.add_root(toolbar_node);

        // Add Undo/Redo buttons at the start of the toolbar
        let button_size = 32.0;

        // Undo button (using TextButton for now - could use icon later)
        let undo_button = TextButton::new(theme.button([0.0, 0.0, 60.0, button_size]), "Undo")
            .with_id("undo_button")
            .with_text_size(12.0)
            .with_text_color(theme.text);
        let undo_node = self.context.workspace.add_view(undo_button);
        if let Some(toolbar_view) = self
            .context
            .workspace
            .find_view_mut::<Toolbar>("main_toolbar")
        {
            toolbar_view.add_child(undo_node);
        }
        self.context.workspace.attach(toolbar_node, undo_node);

        // Redo button
        let redo_button = TextButton::new(theme.button([0.0, 0.0, 60.0, button_size]), "Redo")
            .with_id("redo_button")
            .with_text_size(12.0)
            .with_text_color(theme.text);
        let redo_node = self.context.workspace.add_view(redo_button);
        if let Some(toolbar_view) = self
            .context
            .workspace
            .find_view_mut::<Toolbar>("main_toolbar")
        {
            toolbar_view.add_child(redo_node);
        }
        self.context.workspace.attach(toolbar_node, redo_node);

        // Add a small spacer after undo/redo buttons
        let separator_spacer = Spacer::new(8.0, button_size);
        let separator_spacer_node = self.context.workspace.add_view(separator_spacer);
        if let Some(toolbar_view) = self
            .context
            .workspace
            .find_view_mut::<Toolbar>("main_toolbar")
        {
            toolbar_view.add_child(separator_spacer_node);
        }
        self.context
            .workspace
            .attach(toolbar_node, separator_spacer_node);

        // Add 8 image buttons to the toolbar - they will be automatically positioned!
        let extra_gap = 16.0; // Extra spacing between button groups

        for i in 0..8 {
            // Add a spacer after the 4th button to create visual grouping
            if i == 4 {
                // Create a spacer for the gap (provides visual separation)
                let spacer = Spacer::new(extra_gap, button_size);

                let spacer_node = self.context.workspace.add_view(spacer);
                if let Some(toolbar_view) = self
                    .context
                    .workspace
                    .find_view_mut::<Toolbar>("main_toolbar")
                {
                    toolbar_view.add_child(spacer_node);
                }
                self.context.workspace.attach(toolbar_node, spacer_node);

                // Note: Manual separator removed because absolute positioning doesn't work
                // with automatic layout. The spacer provides sufficient visual separation.
            }

            let btn = Button::new(theme.button([0.0, 0.0, button_size, button_size]))
                .with_id(format!("toolbar_btn_{}", i))
                .with_kind(ButtonKind::Momentary)
                .with_tile(test_tile_id)
                .with_tile_offset(2.0);

            let btn_node = self.context.workspace.add_view(btn);

            // Add to toolbar's layout AND workspace hierarchy
            if let Some(toolbar_view) = self
                .context
                .workspace
                .find_view_mut::<Toolbar>("main_toolbar")
            {
                toolbar_view.add_child(btn_node);
            }
            self.context.workspace.attach(toolbar_node, btn_node);
        }

        // Add a noise button with procedurally generated texture
        let noise_btn = Button::new(theme.button([0.0, 0.0, button_size, button_size]))
            .with_id("noise_button")
            .with_kind(ButtonKind::Momentary)
            .with_tile(self.noise_tile_id)
            .with_tile_offset(2.0);

        let noise_btn_node = self.context.workspace.add_view(noise_btn);
        self.noise_button_id = Some(noise_btn_node);

        if let Some(toolbar_view) = self
            .context
            .workspace
            .find_view_mut::<Toolbar>("main_toolbar")
        {
            toolbar_view.add_child(noise_btn_node);
        }
        self.context.workspace.attach(toolbar_node, noise_btn_node);

        // Add a flexible spacer to push the ButtonGroup to the right
        let flex_spacer = Spacer::flexible();
        let flex_spacer_node = self.context.workspace.add_view(flex_spacer);
        if let Some(toolbar_view) = self
            .context
            .workspace
            .find_view_mut::<Toolbar>("main_toolbar")
        {
            toolbar_view.add_child(flex_spacer_node);
        }
        self.context
            .workspace
            .attach(toolbar_node, flex_spacer_node);

        // Add a ButtonGroup to the right side of the toolbar (with automatic layout)
        let toolbar_button_group = ButtonGroup::new(
            "toolbar_group",
            theme.button_group([0.0, 0.0, 0.0, 36.0], 60.0, 36.0),
        )
        .with_id("toolbar_group")
        .with_text_color(theme.text)
        .with_textures(vec![
            Some(test_tile_id),
            Some(pressed_tile_id),
            Some(test_tile_id),
        ]);

        let toolbar_group_node = self.context.workspace.add_view(toolbar_button_group);
        if let Some(toolbar_view) = self
            .context
            .workspace
            .find_view_mut::<Toolbar>("main_toolbar")
        {
            toolbar_view.add_child(toolbar_group_node);
        }
        self.context
            .workspace
            .attach(toolbar_node, toolbar_group_node);

        // Add a parameter list below the toolbar
        let param_slider_width = 180.0; // Slider width, value text appears 8px to the right

        let mut param_list = ParamList::new(theme.param_list([40.0, 250.0, 0.0, 0.0]))
            .with_id("param_list")
            .with_title("Audio Settings")
            .with_item_height(32.0)
            .with_label_width(80.0)
            .with_spacing(8.0)
            .with_label_size(14.0)
            .with_label_color(theme.text)
            .with_auto_width(param_slider_width); // Auto-calculate width based on slider width

        // Create sliders and labels for the parameter list
        let mut param_slider_nodes = Vec::new();

        for i in 0..4 {
            let label_text = match i {
                0 => "Speed",
                1 => "Volume",
                2 => "Opacity",
                3 => "Scale",
                _ => "Unknown",
            };

            // Create slider with dummy rect - workspace will position it via ParamList layout
            let mut slider_style = theme.slider([0.0, 0.0, param_slider_width, 32.0]);
            slider_style.layer = 11; // Override layer for param list children

            let slider = Slider::new(slider_style, 0.0, 100.0)
                .with_id(format!("param_slider_{}", i))
                .with_value(50.0 + (i as f32 * 10.0))
                .with_show_value(true)
                .with_value_precision(1)
                .with_value_color(theme.text)
                .with_value_size(12.0);

            let slider_node = self.context.workspace.add_view(slider);
            param_list.add_item(label_text, slider_node);
            param_slider_nodes.push(slider_node);
        }

        let param_list_node = self.context.workspace.add_view(param_list);

        // Attach all sliders as children of the ParamList
        for slider_node in param_slider_nodes {
            self.context.workspace.attach(param_list_node, slider_node);
        }

        self.context.workspace.add_root(param_list_node);

        // Add a dropdown list example
        let dropdown = DropdownList::new(
            "noise_type",
            theme.dropdown_list([40.0, 450.0, 200.0, 40.0]),
        )
        .with_id("noise_dropdown")
        .with_items(vec![
            "Simplex Noise".into(),
            "Perlin Noise".into(),
            "Value Noise".into(),
            "Worley Noise".into(),
            "Voronoi".into(),
        ])
        .with_selected(0);

        let dropdown_node = self.context.workspace.add_view(dropdown);
        self.context.workspace.add_root(dropdown_node);

        // Create a popup ParamList for a button
        let popup_slider_width = 90.0;

        let mut popup_style = theme.param_list([0.0, 0.0, 0.0, 0.0]);
        popup_style.layer = 100; // High layer for popup

        let mut popup_param_list = ParamList::new(popup_style)
            .with_id("popup_param_list")
            .with_title("Color Picker")
            .with_item_height(28.0)
            .with_label_width(70.0)
            .with_spacing(6.0)
            .with_label_size(13.0)
            .with_label_color(theme.text)
            .with_auto_width(popup_slider_width); // Auto-calculate width

        // Add sliders to the popup
        let mut slider_nodes = Vec::new();
        for i in 0..3 {
            let label_text = match i {
                0 => "Red",
                1 => "Green",
                2 => "Blue",
                _ => "Unknown",
            };

            // Create slider with dummy rect - workspace will position it via ParamList layout
            let mut popup_slider_style = theme.slider([0.0, 0.0, popup_slider_width, 28.0]);
            popup_slider_style.layer = 101;
            popup_slider_style.thumb_radius = 5.0;
            popup_slider_style.track_height = 3.0;

            let slider = Slider::new(popup_slider_style, 0.0, 255.0)
                .with_id(format!("popup_slider_{}", i))
                .with_value(128.0 + (i as f32 * 20.0))
                .with_show_value(true)
                .with_value_precision(0)
                .with_value_color(theme.text)
                .with_value_size(11.0);

            let slider_node = self.context.workspace.add_view(slider);
            popup_param_list.add_item(label_text, slider_node);
            slider_nodes.push(slider_node);
        }

        // Add a ButtonGroup to the popup ParamList
        let mut popup_group_style = theme.button_group([0.0, 0.0, 140.0, 28.0], 44.0, 28.0);
        popup_group_style.layer = 101;
        popup_group_style.spacing = 2.0;

        let popup_button_group = ButtonGroup::new("popup_group", popup_group_style)
            .with_id("popup_group")
            .with_labels(vec![
                "RGB".to_string(),
                "HSV".to_string(),
                "HEX".to_string(),
            ])
            .with_text_color(theme.text);

        let popup_group_node = self.context.workspace.add_view(popup_button_group);
        popup_param_list.add_item("Mode", popup_group_node);
        slider_nodes.push(popup_group_node);

        let popup_param_list_node = self.context.workspace.add_view(popup_param_list);

        // Attach all child widgets (sliders and button group) to the popup ParamList
        for child_node in slider_nodes {
            self.context
                .workspace
                .attach(popup_param_list_node, child_node);
        }

        // Create a button that opens the popup
        let popup_button = Button::new(theme.button([450.0, 250.0, 120.0, 44.0]))
            .with_id("popup_button")
            .with_kind(ButtonKind::Momentary)
            .with_popup(popup_param_list_node, PopupAlignment::Right);

        let popup_button_node = self.context.workspace.add_view(popup_button);
        self.context.workspace.add_root(popup_button_node);

        // Add label for popup button
        let popup_button_label =
            LabelRect::new("Colors", [450.0, 250.0, 120.0, 44.0], 16.0, theme.text).with_layer(16); // Layer above button (buttons are now on layer 15)
        let popup_button_label_node = self.context.workspace.add_view(popup_button_label);
        self.context.workspace.add_root(popup_button_label_node);

        // === Canvas Demo: Two modes that can be toggled ===

        // Create Main Canvas
        let main_canvas = Canvas::new().with_id("main_canvas").with_visible(true);
        let main_canvas_node = self.context.workspace.add_view(main_canvas);
        self.context.workspace.add_root(main_canvas_node);

        // Add widgets to main canvas
        let main_label = LabelRect::new(
            "Main Mode - Press button below to switch",
            [40.0, 520.0, 400.0, 30.0],
            16.0,
            theme.text,
        )
        .with_layer(10);
        let main_label_node = self.context.workspace.add_view(main_label);
        self.context
            .workspace
            .attach(main_canvas_node, main_label_node);

        // Create Settings Canvas (initially hidden)
        let settings_canvas = Canvas::new().with_id("settings_canvas").with_visible(false);
        let settings_canvas_node = self.context.workspace.add_view(settings_canvas);
        self.context.workspace.add_root(settings_canvas_node);

        // Add widgets to settings canvas
        let settings_label = LabelRect::new(
            "Settings Mode - Press button below to switch back",
            [40.0, 520.0, 400.0, 30.0],
            16.0,
            theme.text,
        )
        .with_layer(10);
        let settings_label_node = self.context.workspace.add_view(settings_label);
        self.context
            .workspace
            .attach(settings_canvas_node, settings_label_node);

        let mut settings_slider_style = theme.slider([40.0, 560.0, 300.0, 40.0]);
        settings_slider_style.thumb_radius = 12.0; // Larger thumb for settings

        let settings_slider = Slider::new(settings_slider_style, 0.0, 100.0)
            .with_id("settings_slider")
            .with_value(75.0)
            .with_show_value(true);
        let settings_slider_node = self.context.workspace.add_view(settings_slider);
        self.context
            .workspace
            .attach(settings_canvas_node, settings_slider_node);

        // Add a button to toggle between canvases (using TextButton)
        let canvas_toggle_button =
            TextButton::new(theme.button([450.0, 510.0, 150.0, 44.0]), "Switch Mode")
                .with_id("canvas_toggle")
                .with_text_size(14.0)
                .with_text_color(theme.text);
        let canvas_toggle_node = self.context.workspace.add_view(canvas_toggle_button);
        self.context.workspace.add_root(canvas_toggle_node);

        // === Color Wheel Demo ===
        let color_wheel = ColorWheel::new(
            [740.0, 340.0, 180.0, 180.0],  // Position in top-right area
            Vec4::new(1.0, 0.5, 0.2, 1.0), // Initial orange color
        )
        .with_id("demo_color_wheel");

        // Create the atlas tile for the color wheel
        color_wheel.ensure_tile(vm.active_vm_mut());

        let color_wheel_node = self.context.workspace.add_view(color_wheel);
        self.context.workspace.add_root(color_wheel_node);

        // Label for color wheel
        let color_wheel_label = LabelRect::new(
            "Color Wheel",
            [740.0, 310.0, 180.0, 25.0],
            14.0,
            theme.text_secondary,
        );
        let color_wheel_label_node = self.context.workspace.add_view(color_wheel_label);
        self.context.workspace.add_root(color_wheel_label_node);

        // Create a new VM layer for procedural noise shader
        self.noise_layer = vm.add_vm_layer();
        vm.set_active_vm(self.noise_layer);

        // Load the noise shader on this layer
        if let Some(bytes) = Embedded::get("noise_shader.wgsl") {
            if let Ok(src) = std::str::from_utf8(bytes.data.as_ref()) {
                vm.execute(Atom::SetSource2D(src.to_string()));
            }
        }

        // Set viewport rect to a region in the top-right corner (400x300 box)
        vm.execute(Atom::SetViewportRect2D(Some([560.0, 250.0, 400.0, 300.0])));

        // Optional: set brightness via gp0
        vm.execute(Atom::SetGP0(Vec4::new(0.1, 0.0, 0.0, 0.0)));

        // Create a popup layer for UI popups (layer 2)
        // This will render above the noise shader layer
        self.popup_layer = vm.add_vm_layer();
        self.context
            .workspace
            .set_popup_layer(Some(self.popup_layer));

        // Configure the popup layer with same settings as main UI layer
        vm.set_active_vm(self.popup_layer);
        vm.execute(Atom::SetRenderMode(RenderMode::Compute2D));
        // Set transparent background for popup layer (alpha = 0)
        // This allows proper alpha blending with layers below
        vm.execute(Atom::SetBackground(Vec4::new(0.0, 0.0, 0.0, 0.0)));
        if let Some(bytes) = Embedded::get("ui_body.wgsl") {
            if let Ok(src) = std::str::from_utf8(bytes.data.as_ref()) {
                vm.execute(Atom::SetSource2D(src.to_string()));
            }
        }
        let s = self.scale;
        let m = Mat3::<f32>::new(s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 1.0);
        vm.execute(Atom::SetTransform2D(m));

        // Disable the popup layer initially (no popups at start)
        vm.set_layer_enabled(self.popup_layer, false);

        // Switch back to layer 0 for normal rendering
        vm.set_active_vm(0);
    }

    fn needs_update(&mut self, _vm: &SceneVM) -> bool {
        self.context.workspace.is_dirty()
    }

    fn render(&mut self, vm: &mut SceneVM, ctx: &mut dyn SceneVMRenderCtx) {
        // Handle actions and update state
        for action in self.context.workspace.take_actions() {
            match action {
                UiAction::ButtonPressed(id) => {
                    println!("Button pressed: {id}");

                    // Handle undo/redo button presses
                    if id == "undo_button" {
                        self.app_events.emit(AppEvent::RequestUndo);
                    } else if id == "redo_button" {
                        self.app_events.emit(AppEvent::RequestRedo);
                    }
                    // Toggle between canvases
                    else if id == "canvas_toggle" {
                        let main_visible = self
                            .context
                            .workspace
                            .find_view_mut::<Canvas>("main_canvas")
                            .map(|c| c.is_visible())
                            .unwrap_or(false);

                        self.context
                            .workspace
                            .set_canvas_visible("main_canvas", !main_visible);
                        self.context
                            .workspace
                            .set_canvas_visible("settings_canvas", main_visible);
                    }
                }
                UiAction::ButtonToggled(id, on) => {
                    println!("Button toggled: {id} -> {on}");

                    // Example: sync image_button state with toggle_button
                    if id == "toggle_button" {
                        if let Some(img_btn) = self
                            .context
                            .workspace
                            .find_view_mut::<Button>("image_button")
                        {
                            img_btn.set_toggled(on);
                        }
                    }
                }
                UiAction::SliderChanged(id, value, original_value, is_final) => {
                    if id == "main_slider" {
                        // Update value immediately for preview
                        self.context.slider_value = value;

                        // Only create undo command on final change (mouse up)
                        if is_final {
                            let cmd = Box::new(SliderChangeCommand::new(
                                id.clone(),
                                original_value,
                                value,
                            ));
                            self.undo_stack.execute(cmd, vm, &mut self.context);
                        }

                        self.has_changes = true;
                        // Update just the label text using its string ID
                        if let Some(label) = self
                            .context
                            .workspace
                            .find_view_mut::<Label>("slider_label")
                        {
                            label.set_text(format!("Value: {:.1}", self.context.slider_value));
                        }
                    } else if id.starts_with("param_slider_") {
                        // Update param sliders
                        if let Some(idx_str) = id.strip_prefix("param_slider_") {
                            if let Ok(idx) = idx_str.parse::<usize>() {
                                if idx < self.context.param_sliders.len() {
                                    // Update value immediately for preview
                                    self.context.param_sliders[idx] = value;

                                    // Only create undo command on final change (mouse up)
                                    if is_final {
                                        let cmd = Box::new(SliderChangeCommand::new(
                                            id.clone(),
                                            original_value,
                                            value,
                                        ));
                                        self.undo_stack.execute(cmd, vm, &mut self.context);
                                    }

                                    self.has_changes = true;
                                }
                            }
                        }
                    }
                }
                UiAction::ButtonGroupChanged(name, index) => {
                    println!("Button group '{}' changed to index {}", name, index);

                    // Get the old index from the button group
                    if let Some(button_group) =
                        self.context.workspace.find_view_mut::<ButtonGroup>(&name)
                    {
                        let old_index = button_group.active_index;
                        if old_index != index {
                            let cmd = Box::new(ButtonGroupChangeCommand::new(
                                name.clone(),
                                old_index,
                                index,
                            ));
                            self.undo_stack.execute(cmd, vm, &mut self.context);
                            self.has_changes = true;
                        }
                    }
                }
                UiAction::DropdownChanged(name, index) => {
                    println!("Dropdown '{}' changed to index {}", name, index);
                }
                UiAction::ColorChanged(id, current_color, original_color, is_final) => {
                    println!(
                        "Color changed from '{}': RGBA({:.3}, {:.3}, {:.3}, {:.3}) [final: {}]",
                        id,
                        current_color[0],
                        current_color[1],
                        current_color[2],
                        current_color[3],
                        is_final
                    );
                    if is_final {
                        println!(
                            "  Original color: RGBA({:.3}, {:.3}, {:.3}, {:.3})",
                            original_color[0],
                            original_color[1],
                            original_color[2],
                            original_color[3]
                        );
                    }
                }
                UiAction::Custom { source_id, action } => {
                    println!("Custom action from {}: {}", source_id, action);
                }
            }
        }

        // Set GP0.z to the color wheel's HSV value for shader (ensure we're on layer 0)
        vm.set_active_vm(0);
        if let Some(color_wheel) = self
            .context
            .workspace
            .find_view_mut::<ColorWheel>("demo_color_wheel")
        {
            let hsv_value = color_wheel.hsv_value();
            vm.execute(Atom::SetGP0(Vec4::new(0.0, 0.0, hsv_value, 0.0)));
        }

        // Update noise button texture if needed
        if self.update_noise_icon {
            // Switch to noise layer to render the procedural texture
            vm.set_active_vm(self.noise_layer);

            vm.execute(Atom::SetViewportRect2D(None));
            vm.execute(Atom::SetGP0(Vec4::new(0.0, 0.0, 0.0, 0.0)));

            // Render noise to a small buffer that fits the button
            let tile_width = 32u32;
            let tile_height = 32u32;
            let mut pixels: Vec<u8> = vec![0; (tile_width * tile_height * 4) as usize];
            vm.render_frame(&mut pixels, tile_width, tile_height);

            // Create/update the tile with the rendered noise
            vm.execute(Atom::AddTile {
                id: self.noise_tile_id,
                width: tile_width,
                height: tile_height,
                frames: vec![pixels],
                material_frames: Some(vec![create_tile_material(
                    tile_width as u32,
                    tile_height as u32,
                )]),
            });
            vm.execute(Atom::BuildAtlas);

            self.update_noise_icon = false;

            vm.execute(Atom::SetViewportRect2D(Some([900.0, 700.0, 400.0, 300.0])));

            // Switch back to main layer
            vm.set_active_vm(0);
        }

        // Build drawables from workspace
        let text_cache = self.renderer.text_cache();
        let drawables = self.context.workspace.build(text_cache);
        let popup_drawables = self.context.workspace.build_popups_separate(text_cache);

        // Render main UI to layer 0
        vm.set_active_vm(0);
        self.renderer.render(vm.active_vm_mut(), &drawables);

        // Enable/disable popup layer based on whether there are popups
        let has_popups = !popup_drawables.is_empty();
        vm.set_layer_enabled(self.popup_layer, has_popups);

        // Render popups to popup layer (layer 2, above noise shader)
        if has_popups {
            vm.set_active_vm(self.popup_layer);
            self.renderer.render(vm.active_vm_mut(), &popup_drawables);
        }

        // Animate the noise layer
        vm.set_active_vm(self.noise_layer);
        let counter = vm.active_vm().animation_counter;
        vm.execute(Atom::SetAnimationCounter(counter + 1));
        vm.set_active_vm(0);

        let _ = ctx.present(vm);
    }

    fn mouse_down(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        // Check if click is outside popup system - close all popups if so
        if !self.context.workspace.is_click_inside_popup_system([x, y]) {
            self.context.workspace.close_all_popups();
        }

        self.context.workspace.handle_event(&UiEvent {
            kind: UiEventKind::PointerDown,
            pos: [x, y],
            pointer_id: 0,
        });
    }

    fn mouse_up(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        self.context.workspace.handle_event(&UiEvent {
            kind: UiEventKind::PointerUp,
            pos: [x, y],
            pointer_id: 0,
        });
    }

    fn mouse_move(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        self.context.workspace.handle_event(&UiEvent {
            kind: UiEventKind::PointerMove,
            pos: [x, y],
            pointer_id: 0,
        });
    }

    // Project Management Implementation

    fn save_to_json(&mut self, _vm: &mut SceneVM) -> Option<String> {
        let data = UiDemoData {
            slider_value: self.context.slider_value,
            param_sliders: self.context.param_sliders.clone(),
        };

        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                self.has_changes = false;
                Some(json)
            }
            Err(e) => {
                eprintln!("Failed to serialize project: {}", e);
                None
            }
        }
    }

    fn load_from_json(&mut self, vm: &mut SceneVM, json: &str) -> bool {
        match serde_json::from_str::<UiDemoData>(json) {
            Ok(data) => {
                self.context.slider_value = data.slider_value;
                self.context.param_sliders = data.param_sliders;
                self.has_changes = false;

                // Update UI to reflect loaded values
                if let Some(slider) = self
                    .context
                    .workspace
                    .find_view_mut::<Slider>("main_slider")
                {
                    slider.set_value(self.context.slider_value);
                }
                if let Some(label) = self
                    .context
                    .workspace
                    .find_view_mut::<Label>("slider_label")
                {
                    label.set_text(format!("Value: {:.1}", self.context.slider_value));
                }

                // Update parameter sliders
                for (i, value) in self.context.param_sliders.iter().enumerate() {
                    if let Some(slider) = self
                        .context
                        .workspace
                        .find_view_mut::<Slider>(&format!("param_slider_{}", i))
                    {
                        slider.set_value(*value);
                    }
                }

                // Rebuild the UI
                vm.execute(Atom::SetBackground(Vec4::new(0.08, 0.08, 0.1, 1.0)));

                true
            }
            Err(e) => {
                eprintln!("Failed to deserialize project: {}", e);
                false
            }
        }
    }

    fn new_project(&mut self, vm: &mut SceneVM) {
        let default = UiDemoData::default();
        self.context.slider_value = default.slider_value;
        self.context.param_sliders = default.param_sliders;
        self.has_changes = false;

        // Reset UI
        if let Some(slider) = self
            .context
            .workspace
            .find_view_mut::<Slider>("main_slider")
        {
            slider.set_value(self.context.slider_value);
        }
        if let Some(label) = self
            .context
            .workspace
            .find_view_mut::<Label>("slider_label")
        {
            label.set_text(format!("Value: {:.1}", self.context.slider_value));
        }

        for (i, value) in self.context.param_sliders.iter().enumerate() {
            if let Some(slider) = self
                .context
                .workspace
                .find_view_mut::<Slider>(&format!("param_slider_{}", i))
            {
                slider.set_value(*value);
            }
        }

        vm.execute(Atom::SetBackground(Vec4::new(0.08, 0.08, 0.1, 1.0)));
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_changes
    }

    fn take_app_events(&mut self) -> Vec<AppEvent> {
        self.app_events.take()
    }

    // Undo/Redo Implementation

    fn undo(&mut self, vm: &mut SceneVM) -> bool {
        self.undo_stack.undo(vm, &mut self.context)
    }

    fn redo(&mut self, vm: &mut SceneVM) -> bool {
        self.undo_stack.redo(vm, &mut self.context)
    }

    fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    fn undo_description(&self) -> Option<String> {
        self.undo_stack
            .undo_description()
            .map(|s| format!("Undo {}", s))
    }

    fn redo_description(&self) -> Option<String> {
        self.undo_stack
            .redo_description()
            .map(|s| format!("Redo {}", s))
    }

    fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    fn update(&mut self, vm: &mut SceneVM) {
        // Apply transform to popup layer to handle HiDPI scaling
        // This ensures the popup layer uses the same logical-to-physical transform as layer 0
        if self.popup_layer > 0 {
            vm.set_active_vm(self.popup_layer);
            let s = self.scale;
            let m = Mat3::<f32>::new(s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 1.0);
            vm.execute(Atom::SetTransform2D(m));
            vm.set_active_vm(0);
        }
    }

    fn resize(&mut self, _vm: &mut SceneVM, size: (u32, u32)) {
        // Update toolbar to span full width
        if let Some(toolbar) = self
            .context
            .workspace
            .find_view_mut::<Toolbar>("main_toolbar")
        {
            let width = size.0 as f32;
            toolbar.style.rect[2] = width; // Update width

            // Update internal HStack rect
            if let Some(ref mut hstack) = toolbar.hstack {
                hstack.rect[2] = width;
            }

            self.context.workspace.set_dirty(); // Trigger relayout
        }
    }
}

fn main() {
    scenevm::run_scenevm_app(UiDemo::new()).ok();
}
