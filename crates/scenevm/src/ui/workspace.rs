use crate::ui::{
    drawable::Drawable, event::UiAction, event::UiEvent, event::UiEventOutcome, text::TextCache,
};
use rustc_hash::FxHashMap;
use std::any::Any;
use uuid::Uuid;
use vek::Vec4;

/// Identifier for nodes in the UI workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Context passed to views during build; collects drawables.
pub struct ViewContext<'a> {
    drawables: &'a mut Vec<Drawable>,
    current_layer: i32,
    text_cache: &'a TextCache,
}

impl<'a> ViewContext<'a> {
    pub fn push(&mut self, drawable: Drawable) {
        self.drawables.push(drawable);
    }

    pub fn with_layer(&mut self, layer: i32) -> ViewContext<'_> {
        ViewContext {
            drawables: self.drawables,
            current_layer: layer,
            text_cache: self.text_cache,
        }
    }

    pub fn layer(&self) -> i32 {
        self.current_layer
    }

    pub fn text_cache(&self) -> &TextCache {
        self.text_cache
    }
}

/// Trait implemented by UI views to emit drawables.
pub trait UiView: Any {
    fn build(&mut self, ctx: &mut ViewContext);
    fn handle_event(&mut self, _evt: &UiEvent) -> UiEventOutcome {
        UiEventOutcome::none()
    }
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any(&self) -> &dyn Any;
    fn view_id(&self) -> &str {
        ""
    }
}

struct Node {
    view: Box<dyn UiView>,
    children: Vec<NodeId>,
}

/// Node-driven UI workspace: holds a tree of views and produces drawables.
pub struct Workspace {
    nodes: FxHashMap<NodeId, Node>,
    roots: Vec<NodeId>,
    dirty: bool,
    pending_actions: Vec<UiAction>,
    popup_layer: Option<usize>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            nodes: FxHashMap::default(),
            roots: Vec::new(),
            dirty: true,
            pending_actions: Vec::new(),
            popup_layer: None,
        }
    }

    /// Set the optional popup layer index. When set, popup drawables will be
    /// returned separately so they can be rendered to a different VM layer.
    pub fn with_popup_layer(mut self, layer: usize) -> Self {
        self.popup_layer = Some(layer);
        self
    }

    /// Set the popup layer after creation
    pub fn set_popup_layer(&mut self, layer: Option<usize>) {
        self.popup_layer = layer;
    }

    /// Insert a view as a new node and return its id.
    pub fn add_view<V: UiView + 'static>(&mut self, view: V) -> NodeId {
        let id = NodeId::new();
        self.nodes.insert(
            id,
            Node {
                view: Box::new(view),
                children: Vec::new(),
            },
        );
        id
    }

    /// Mark a node as a root in the workspace.
    pub fn add_root(&mut self, id: NodeId) {
        if self.nodes.contains_key(&id) && !self.roots.contains(&id) {
            self.roots.push(id);
        }
    }

    /// Attach a child node under a parent.
    pub fn attach(&mut self, parent: NodeId, child: NodeId) {
        let child_exists = self.nodes.contains_key(&child);
        if child_exists {
            if let Some(p) = self.nodes.get_mut(&parent) {
                if !p.children.contains(&child) {
                    p.children.push(child);
                }
            }
        }
    }

    /// Traverse roots and collect drawables.
    /// Returns main UI drawables. If popup_layer is set, popup drawables
    /// should be retrieved with `build_popups_separate()`.
    pub fn build(&mut self, text_cache: &TextCache) -> Vec<Drawable> {
        let mut drawables = Vec::new();
        let roots = self.roots.clone();
        for root in roots {
            self.build_node(root, &mut drawables, 0, text_cache);
        }

        // After rendering all normal views, render popups on top
        // (only if we're not using a separate popup layer)
        if self.popup_layer.is_none() {
            self.build_popups(&mut drawables, text_cache);
        }

        self.dirty = false;
        drawables
    }

    /// Build popup drawables separately (only when popup_layer is set).
    /// Call this after `build()` to get popup drawables for a separate layer.
    pub fn build_popups_separate(&mut self, text_cache: &TextCache) -> Vec<Drawable> {
        if self.popup_layer.is_none() {
            return Vec::new();
        }

        let mut drawables = Vec::new();
        self.build_popups(&mut drawables, text_cache);
        drawables
    }

    fn build_node(
        &mut self,
        id: NodeId,
        out: &mut Vec<Drawable>,
        layer: i32,
        text_cache: &TextCache,
    ) {
        // Check if this is a Canvas and if it's visible (before borrowing node mutably)
        let is_visible_canvas = {
            let Some(node) = self.nodes.get(&id) else {
                return;
            };
            if let Some(canvas) = node.view.as_any().downcast_ref::<crate::ui::Canvas>() {
                canvas.is_visible()
            } else {
                true // Not a canvas, always visible
            }
        };

        if !is_visible_canvas {
            return; // Skip this canvas and its children
        }

        // Apply layout if this node is a layout container
        // println!("build_node: applying layout for node {:?}", id);
        self.apply_layout(id);

        // Check if this is a TabbedPanel and get active tab info
        let active_tab_info = {
            let Some(node) = self.nodes.get(&id) else {
                return;
            };
            if let Some(tabbed_panel) = node.view.as_any().downcast_ref::<crate::ui::TabbedPanel>()
            {
                Some((
                    tabbed_panel.tab_button_group,
                    tabbed_panel.active_tab,
                    tabbed_panel.tab_contents.clone(),
                ))
            } else {
                None
            }
        };

        // Now borrow node mutably for building
        let Some(node) = self.nodes.get_mut(&id) else {
            return;
        };

        let children = node.children.clone();

        let mut ctx = ViewContext {
            drawables: out,
            current_layer: layer,
            text_cache,
        };
        node.view.build(&mut ctx);
        // node borrow is released here

        // If this is a TabbedPanel, only render button group and active tab
        if let Some((button_group_id, active_tab, tab_contents)) = active_tab_info {
            // Render the button group
            self.build_node(button_group_id, out, layer, text_cache);
            // Render only the active tab content
            if let Some(&active_content_id) = tab_contents.get(active_tab) {
                self.build_node(active_content_id, out, layer, text_cache);
            }
        } else {
            // Normal rendering: render all children
            for child in children {
                self.build_node(child, out, layer, text_cache);
            }
        }
    }

    /// Recursively apply layouts to a node and all its children
    fn apply_layouts_recursive(&mut self, id: NodeId) {
        // Apply layout for this node if it's a layout container
        self.apply_layout(id);

        // Recursively apply to all children
        let children = if let Some(node) = self.nodes.get(&id) {
            node.children.clone()
        } else {
            return;
        };

        for child in children {
            self.apply_layouts_recursive(child);
        }
    }

    /// Apply layout calculations if this node is a layout container (HStack/VStack/Toolbar)
    fn apply_layout(&mut self, layout_id: NodeId) {
        use crate::ui::layouts::{HStack, VStack};
        use crate::ui::{ParamList, Toolbar};

        // Layout type enum for cleaner handling
        #[derive(Debug, Clone, Copy)]
        enum LayoutType {
            HStack,
            VStack,
            Toolbar { horizontal: bool },
            ParamList,
            TabbedPanel,
        }

        // First, collect child sizes and check if this is a layout
        let layout_info = {
            let Some(layout_node) = self.nodes.get(&layout_id) else {
                return;
            };

            // Check if this is an HStack
            if let Some(hstack) = layout_node.view.as_any().downcast_ref::<HStack>() {
                let children = hstack.children.clone();
                Some((children, LayoutType::HStack))
            }
            // Check if this is a VStack
            else if let Some(vstack) = layout_node.view.as_any().downcast_ref::<VStack>() {
                let children = vstack.children.clone();
                Some((children, LayoutType::VStack))
            }
            // Check if this is a Toolbar
            else if let Some(toolbar) = layout_node.view.as_any().downcast_ref::<Toolbar>() {
                let children = toolbar.children().to_vec();
                let is_horizontal = matches!(
                    toolbar.orientation,
                    crate::ui::ToolbarOrientation::Horizontal
                );
                Some((
                    children,
                    LayoutType::Toolbar {
                        horizontal: is_horizontal,
                    },
                ))
            }
            // Check if this is a ParamList
            else if let Some(param_list) = layout_node.view.as_any().downcast_ref::<ParamList>() {
                let children = param_list.children();
                Some((children, LayoutType::ParamList))
            }
            // Check if this is a TabbedPanel
            else if let Some(tabbed_panel) = layout_node
                .view
                .as_any()
                .downcast_ref::<crate::ui::TabbedPanel>()
            {
                let children = tabbed_panel.children();
                Some((children, LayoutType::TabbedPanel))
            } else {
                None
            }
        };

        let Some((children, layout_type)) = layout_info else {
            return;
        };

        // Collect child sizes and identify flexible spacers
        let mut child_sizes = Vec::new();
        let mut flexible_indices = Vec::new();
        for (i, &child_id) in children.iter().enumerate() {
            if let Some(child_node) = self.nodes.get(&child_id) {
                // Check if this is a flexible spacer
                if let Some(spacer) = child_node.view.as_any().downcast_ref::<crate::ui::Spacer>() {
                    if spacer.flexible {
                        flexible_indices.push(i);
                    }
                }
                let size = self.extract_widget_size(child_node);
                child_sizes.push(size);
            }
        }

        // Calculate layout
        let computed_rects = match layout_type {
            LayoutType::Toolbar { horizontal } => {
                // Get layout from toolbar's internal HStack/VStack
                if let Some(layout_node) = self.nodes.get(&layout_id) {
                    if let Some(toolbar) = layout_node.view.as_any().downcast_ref::<Toolbar>() {
                        if horizontal {
                            toolbar
                                .hstack
                                .as_ref()
                                .map(|h| h.calculate_layout(&child_sizes, &flexible_indices))
                                .unwrap_or_default()
                        } else {
                            toolbar
                                .vstack
                                .as_ref()
                                .map(|v| v.calculate_layout(&child_sizes, &flexible_indices))
                                .unwrap_or_default()
                        }
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            LayoutType::HStack => {
                if let Some(layout_node) = self.nodes.get(&layout_id) {
                    if let Some(hstack) = layout_node.view.as_any().downcast_ref::<HStack>() {
                        hstack.calculate_layout(&child_sizes, &flexible_indices)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            LayoutType::VStack => {
                if let Some(layout_node) = self.nodes.get(&layout_id) {
                    if let Some(vstack) = layout_node.view.as_any().downcast_ref::<VStack>() {
                        vstack.calculate_layout(&child_sizes, &flexible_indices)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            LayoutType::ParamList => {
                if let Some(layout_node) = self.nodes.get(&layout_id) {
                    if let Some(param_list) = layout_node.view.as_any().downcast_ref::<ParamList>()
                    {
                        param_list.calculate_layout(&child_sizes)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            LayoutType::TabbedPanel => {
                if let Some(layout_node) = self.nodes.get(&layout_id) {
                    if let Some(tabbed_panel) = layout_node
                        .view
                        .as_any()
                        .downcast_ref::<crate::ui::TabbedPanel>()
                    {
                        tabbed_panel.calculate_layout()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
        };

        // Apply computed rects to children
        for (i, &child_id) in children.iter().enumerate() {
            if let Some(rect) = computed_rects.get(i) {
                if let Some(child_node) = self.nodes.get_mut(&child_id) {
                    Self::set_widget_rect(child_node, *rect);
                }
            }
        }
    }

    /// Extract widget size from common widget types (fallback for non-Layoutable widgets)
    fn extract_widget_size(&self, node: &Node) -> [f32; 2] {
        use crate::ui::{
            Button, ButtonGroup, ColorButton, ColorWheel, DropdownList, Slider, Spacer, TextButton,
        };

        // Try Button
        if let Some(button) = node.view.as_any().downcast_ref::<Button>() {
            let [_x, _y, w, h] = button.style.rect;
            return [w, h];
        }

        // Try ButtonGroup - use calculated width based on button count
        if let Some(button_group) = node.view.as_any().downcast_ref::<ButtonGroup>() {
            let width = button_group.calculate_width();
            let height = button_group.style.button_height;
            return [width, height];
        }

        // Try DropdownList
        if let Some(dropdown) = node.view.as_any().downcast_ref::<DropdownList>() {
            let [_x, _y, w, h] = dropdown.style.rect;
            return [w, h];
        }

        // Try Slider
        if let Some(slider) = node.view.as_any().downcast_ref::<Slider>() {
            let [_x, _y, w, h] = slider.style.rect;
            return [w, h];
        }

        // Try Spacer
        if let Some(spacer) = node.view.as_any().downcast_ref::<Spacer>() {
            let [_x, _y, w, h] = spacer.rect;
            return [w, h];
        }

        // Try TextButton
        if let Some(text_button) = node.view.as_any().downcast_ref::<TextButton>() {
            let [_x, _y, w, h] = text_button.style.rect;
            return [w, h];
        }

        // Try ColorButton
        if let Some(color_button) = node.view.as_any().downcast_ref::<ColorButton>() {
            let [_x, _y, w, h] = color_button.style.rect;
            return [w, h];
        }

        // Try ColorWheel - use a fixed size since rect is private
        if node.view.as_any().downcast_ref::<ColorWheel>().is_some() {
            return [100.0, 100.0]; // Default color wheel size
        }

        // Add more widget types here as needed

        // Default size
        [100.0, 40.0]
    }

    /// Set widget rect for common widget types (fallback for non-Layoutable widgets)
    fn set_widget_rect(node: &mut Node, rect: [f32; 4]) {
        use crate::ui::{
            Button, ButtonGroup, ColorButton, ColorWheel, DropdownList, ParamList, Slider, Spacer,
            TextButton,
        };

        // Try Button
        if let Some(button) = node.view.as_any_mut().downcast_mut::<Button>() {
            button.style.rect = rect;
            return;
        }

        // Try ButtonGroup
        if let Some(button_group) = node.view.as_any_mut().downcast_mut::<ButtonGroup>() {
            button_group.style.rect = rect;
            return;
        }

        // Try DropdownList
        if let Some(dropdown) = node.view.as_any_mut().downcast_mut::<DropdownList>() {
            dropdown.style.rect = rect;
            return;
        }

        // Try Slider
        if let Some(slider) = node.view.as_any_mut().downcast_mut::<Slider>() {
            slider.style.rect = rect;
            return;
        }

        // Try Spacer
        if let Some(spacer) = node.view.as_any_mut().downcast_mut::<Spacer>() {
            spacer.rect = rect;
            return;
        }

        // Try TextButton
        if let Some(text_button) = node.view.as_any_mut().downcast_mut::<TextButton>() {
            text_button.style.rect = rect;
            return;
        }

        // Try ParamList
        if let Some(param_list) = node.view.as_any_mut().downcast_mut::<ParamList>() {
            param_list.style.rect = rect;
            return;
        }

        // Try ColorButton
        if let Some(color_button) = node.view.as_any_mut().downcast_mut::<ColorButton>() {
            color_button.style.rect = rect;
            return;
        }

        // Try ColorWheel
        if let Some(color_wheel) = node.view.as_any_mut().downcast_mut::<ColorWheel>() {
            color_wheel.set_rect(rect);
            return;
        }

        // Add more widget types here as needed
    }

    /// Dispatch a UI event to all views; collects actions and marks dirty when a view changes.
    pub fn handle_event(&mut self, evt: &UiEvent) {
        // CRITICAL: Apply layouts BEFORE processing events to ensure hit tests use current positions
        let roots = self.roots.clone();
        for root in &roots {
            self.apply_layouts_recursive(*root);
        }

        let mut outcome = UiEventOutcome::none();

        // Dispatch to visible popup contents FIRST - they should consume events before main tree
        let popup_nodes = self.get_visible_popup_nodes();
        for popup_id in popup_nodes {
            let popup_outcome = self.dispatch_node(popup_id, evt);
            // If popup consumed the event, don't dispatch to main tree
            if popup_outcome.dirty || !popup_outcome.actions.is_empty() {
                outcome.merge(popup_outcome);
                if outcome.dirty {
                    self.dirty = true;
                }
                if !outcome.actions.is_empty() {
                    self.pending_actions.extend(outcome.actions);
                }
                return;
            }
            outcome.merge(popup_outcome);
        }

        // Only dispatch to roots if popup didn't consume event
        for root in &roots {
            outcome.merge(self.dispatch_node(*root, evt));
        }

        if outcome.dirty {
            self.dirty = true;
        }
        if !outcome.actions.is_empty() {
            self.pending_actions.extend(outcome.actions);
        }
    }

    fn dispatch_node(&mut self, id: NodeId, evt: &UiEvent) -> UiEventOutcome {
        let mut merged = UiEventOutcome::none();
        if let Some(node) = self.nodes.get_mut(&id) {
            // Check if this is a Canvas and if it's visible
            let is_visible_canvas =
                if let Some(canvas) = node.view.as_any().downcast_ref::<crate::ui::Canvas>() {
                    canvas.is_visible()
                } else {
                    true // Not a canvas, always visible
                };

            if !is_visible_canvas {
                return merged; // Skip event dispatch for invisible canvas and its children
            }

            // Check if this is a TabbedPanel and get active tab info
            let tabbed_panel_info = if let Some(tabbed_panel) =
                node.view.as_any().downcast_ref::<crate::ui::TabbedPanel>()
            {
                Some((
                    tabbed_panel.tab_button_group,
                    tabbed_panel.active_tab,
                    tabbed_panel.tab_contents.clone(),
                ))
            } else {
                None
            };

            merged.merge(node.view.handle_event(evt));

            // If this is a TabbedPanel, only dispatch to button group and active tab
            if let Some((button_group_id, active_tab, tab_contents)) = tabbed_panel_info {
                // Dispatch to button group
                merged.merge(self.dispatch_node(button_group_id, evt));
                // Dispatch only to the active tab content
                if let Some(&active_content_id) = tab_contents.get(active_tab) {
                    merged.merge(self.dispatch_node(active_content_id, evt));
                }
            } else {
                // Normal event dispatch: dispatch to all children
                let children = node.children.clone();
                for child in children {
                    merged.merge(self.dispatch_node(child, evt));
                }
            }
        }
        merged
    }

    /// Returns whether any view changed state since the last build.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the workspace as dirty, forcing a rebuild on next render.
    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }

    /// Drain and return pending UI actions generated by views.
    pub fn take_actions(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Replace a node's view with a new one and mark workspace as dirty.
    pub fn update_view<V: UiView + 'static>(&mut self, id: NodeId, view: V) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.view = Box::new(view);
            self.dirty = true;
        }
    }

    /// Get mutable access to a view and mark workspace as dirty.
    /// Returns None if the node doesn't exist or the type doesn't match.
    pub fn get_view_mut<V: UiView + 'static>(&mut self, id: NodeId) -> Option<&mut V> {
        if let Some(node) = self.nodes.get_mut(&id) {
            self.dirty = true;
            node.view.as_any_mut().downcast_mut::<V>()
        } else {
            None
        }
    }

    /// Find a view by its string ID and return mutable access.
    /// Returns None if no view with that ID exists or the type doesn't match.
    pub fn find_view_mut<V: UiView + 'static>(&mut self, id: &str) -> Option<&mut V> {
        for node in self.nodes.values_mut() {
            if node.view.view_id() == id {
                self.dirty = true;
                return node.view.as_any_mut().downcast_mut::<V>();
            }
        }
        None
    }

    /// Set the visibility of a canvas by its string ID.
    /// Returns true if the canvas was found and updated, false otherwise.
    pub fn set_canvas_visible(&mut self, id: &str, visible: bool) -> bool {
        if let Some(canvas) = self.find_view_mut::<crate::ui::Canvas>(id) {
            canvas.set_visible(visible);
            true
        } else {
            false
        }
    }

    /// Set the active tab of a TabbedPanel by its string ID.
    /// Returns true if the panel was found and updated, false otherwise.
    pub fn set_active_tab(&mut self, id: &str, index: usize) -> bool {
        if let Some(panel) = self.find_view_mut::<crate::ui::TabbedPanel>(id) {
            panel.set_active_tab(index);
            true
        } else {
            false
        }
    }

    /// Set the color of a ColorButton by its string ID.
    /// Returns true if the button was found and updated, false otherwise.
    pub fn set_color_button_color(&mut self, id: &str, color: Vec4<f32>) -> bool {
        if let Some(color_button) = self.find_view_mut::<crate::ui::ColorButton>(id) {
            color_button.set_color(color);
            true
        } else {
            false
        }
    }

    /// Set the tile tint of a Button by its string ID (useful for updating icon colors on theme change).
    /// Returns true if the button was found and updated, false otherwise.
    pub fn set_button_tile_tint(&mut self, id: &str, tint: Vec4<f32>) -> bool {
        if let Some(button) = self.find_view_mut::<crate::ui::Button>(id) {
            button.set_tile_tint(tint);
            self.dirty = true; // Mark workspace as dirty to trigger rebuild
            true
        } else {
            false
        }
    }

    /// Set the color of a ColorWheel by its string ID.
    /// Returns true if the color wheel was found and updated, false otherwise.
    pub fn set_color_wheel_color(&mut self, id: &str, color: Vec4<f32>) -> bool {
        if let Some(color_wheel) = self.find_view_mut::<crate::ui::ColorWheel>(id) {
            color_wheel.set_color(color);
            true
        } else {
            false
        }
    }

    /// Set the value of a slider by its string ID.
    pub fn set_slider_value(&mut self, id: &str, value: f32) {
        if let Some(slider) = self.find_view_mut::<crate::ui::Slider>(id) {
            slider.set_value(value);
        }
    }

    /// Set the title of a ParamList by its string ID.
    pub fn set_paramlist_title(&mut self, id: &str, title: impl Into<String>) {
        if let Some(param_list) = self.find_view_mut::<crate::ui::ParamList>(id) {
            param_list.title = Some(title.into());
        }
    }

    /// Set the active index of a ButtonGroup by its string ID.
    pub fn set_buttongroup_index(&mut self, id: &str, index: usize) {
        if let Some(button_group) = self.find_view_mut::<crate::ui::ButtonGroup>(id) {
            button_group.set_active(index);
        }
    }

    /// Set the selected index of a DropdownList by its string ID.
    pub fn set_dropdown_index(&mut self, id: &str, index: usize) {
        if let Some(dropdown) = self.find_view_mut::<crate::ui::DropdownList>(id) {
            dropdown.set_selected(index);
        }
    }

    /// Apply a new theme to all widgets in the workspace.
    /// This updates colors and styling of all widgets to match the new theme.
    pub fn apply_theme(&mut self, theme: &crate::ui::Theme) {
        use crate::ui::{
            Button, ButtonGroup, DropdownList, ParamList, Slider, TabbedPanel, TextButton, Toolbar,
        };

        for (_id, node) in self.nodes.iter_mut() {
            // Update Button styles
            if let Some(button) = node.view.as_any_mut().downcast_mut::<Button>() {
                let rect = button.style.rect;
                let new_style = theme.button(rect);
                button.tile_tint = new_style.icon_tint; // Update tile tint for icons
                button.style = new_style;
            }
            // Update ButtonGroup styles - preserve spacing!
            else if let Some(bg) = node.view.as_any_mut().downcast_mut::<ButtonGroup>() {
                let rect = bg.style.rect;
                let bw = bg.style.button_width;
                let bh = bg.style.button_height;
                let spacing = bg.style.spacing; // Preserve custom spacing
                let new_style = theme.button_group(rect, bw, bh);
                bg.text_color = new_style.text_color; // Update text color from new style
                bg.text_background_color = new_style.text_bg_color; // Update text background color
                bg.style = new_style;
                bg.style.spacing = spacing; // Restore custom spacing
            }
            // Update Slider styles
            else if let Some(slider) = node.view.as_any_mut().downcast_mut::<Slider>() {
                let rect = slider.style.rect;
                let show_value = slider.show_value; // Preserve settings
                let precision = slider.value_precision;
                slider.style = theme.slider(rect);
                slider.show_value = show_value;
                slider.value_precision = precision;
                // Update value color for better contrast
                slider.value_color = theme.text;
            }
            // Update ParamList styles - preserve title
            else if let Some(pl) = node.view.as_any_mut().downcast_mut::<ParamList>() {
                let rect = pl.style.rect;
                let title = pl.title.clone(); // Preserve title
                pl.style = theme.param_list(rect);
                pl.title = title; // Restore title
                // Update label/title colors for better contrast
                pl.label_color = theme.text;
                pl.title_color = if theme.name == "Light" {
                    theme.text
                } else {
                    theme.accent
                };
            }
            // Update Toolbar styles
            else if let Some(toolbar) = node.view.as_any_mut().downcast_mut::<Toolbar>() {
                let rect = toolbar.style.rect;
                toolbar.style = theme.toolbar(rect);
            }
            // Update DropdownList styles
            else if let Some(dropdown) = node.view.as_any_mut().downcast_mut::<DropdownList>() {
                let rect = dropdown.style.rect;
                dropdown.style = theme.dropdown_list(rect);
            }
            // Update TextButton styles and text color
            else if let Some(text_button) = node.view.as_any_mut().downcast_mut::<TextButton>() {
                let rect = text_button.style.rect;
                text_button.style = theme.button(rect);
                text_button.text_color = theme.text;
            }
            // Update TabbedPanel styles
            else if let Some(tabbed_panel) = node.view.as_any_mut().downcast_mut::<TabbedPanel>()
            {
                let rect = tabbed_panel.style.rect;
                tabbed_panel.style = theme.tabbed_panel(rect);
            }
        }
        self.dirty = true;
    }

    /// Set the position (x, y) of a widget by its string ID.
    /// This updates the widget's rect while preserving its width and height.
    pub fn set_widget_pos(&mut self, id: &str, x: f32, y: f32) {
        use crate::ui::{
            Button, ButtonGroup, DropdownList, Image, Label, ParamList, Slider, Spacer, TextButton,
            Toolbar,
        };

        // Find the node with matching view_id
        let node_entry = self
            .nodes
            .iter_mut()
            .find(|(_, node)| node.view.view_id() == id);

        if let Some((_, node)) = node_entry {
            // Try Button
            if let Some(button) = node.view.as_any_mut().downcast_mut::<Button>() {
                button.style.rect[0] = x;
                button.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Try ButtonGroup
            if let Some(button_group) = node.view.as_any_mut().downcast_mut::<ButtonGroup>() {
                button_group.style.rect[0] = x;
                button_group.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Try DropdownList
            if let Some(dropdown) = node.view.as_any_mut().downcast_mut::<DropdownList>() {
                dropdown.style.rect[0] = x;
                dropdown.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Try Slider
            if let Some(slider) = node.view.as_any_mut().downcast_mut::<Slider>() {
                slider.style.rect[0] = x;
                slider.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Try ParamList
            if let Some(param_list) = node.view.as_any_mut().downcast_mut::<ParamList>() {
                param_list.style.rect[0] = x;
                param_list.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Try Toolbar
            if let Some(toolbar) = node.view.as_any_mut().downcast_mut::<Toolbar>() {
                toolbar.style.rect[0] = x;
                toolbar.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Try Label
            if let Some(label) = node.view.as_any_mut().downcast_mut::<Label>() {
                label.origin[0] = x;
                label.origin[1] = y;
                self.dirty = true;
                return;
            }

            // Try Image
            if let Some(image) = node.view.as_any_mut().downcast_mut::<Image>() {
                image.style.rect[0] = x;
                image.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Canvas doesn't have position, skip it

            // Try TextButton
            if let Some(text_button) = node.view.as_any_mut().downcast_mut::<TextButton>() {
                text_button.style.rect[0] = x;
                text_button.style.rect[1] = y;
                self.dirty = true;
                return;
            }

            // Try Spacer
            if let Some(spacer) = node.view.as_any_mut().downcast_mut::<Spacer>() {
                spacer.rect[0] = x;
                spacer.rect[1] = y;
                self.dirty = true;
                return;
            }
        }
    }

    /// Check if a point is inside any button with an open popup
    /// Returns true if the point is inside a button with popup or inside the popup itself
    pub fn is_inside_popup_area(&self, pos: [f32; 2], popup_rect: [f32; 4]) -> bool {
        let [px, py, pw, ph] = popup_rect;
        pos[0] >= px && pos[0] <= px + pw && pos[1] >= py && pos[1] <= py + ph
    }

    /// Get list of visible popup node IDs
    fn get_visible_popup_nodes(&self) -> Vec<NodeId> {
        use crate::ui::{Button, ColorButton};

        let mut popup_nodes = Vec::new();
        for node in self.nodes.values() {
            if let Some(button) = node.view.as_any().downcast_ref::<Button>() {
                if button.is_popup_visible() {
                    if let Some(popup_id) = button.popup_content {
                        popup_nodes.push(popup_id);
                    }
                }
            } else if let Some(color_button) = node.view.as_any().downcast_ref::<ColorButton>() {
                if color_button.is_popup_visible() {
                    if let Some(popup_id) = color_button.color_wheel {
                        popup_nodes.push(popup_id);
                    }
                }
            }
        }
        popup_nodes
    }

    /// Build popups for all buttons that have visible popups
    fn build_popups(&mut self, out: &mut Vec<Drawable>, text_cache: &TextCache) {
        use crate::ui::{Button, ColorButton};

        // Collect popup info first to avoid borrow checker issues
        let mut popups_to_render = Vec::new();

        for (_node_id, node) in &self.nodes {
            // Check for regular Button popups
            if let Some(button) = node.view.as_any().downcast_ref::<Button>() {
                if button.is_popup_visible() {
                    if let Some(popup_content_id) = button.popup_content {
                        if self.nodes.contains_key(&popup_content_id) {
                            popups_to_render.push((
                                popup_content_id,
                                button.style.rect,
                                button.popup_alignment,
                            ));
                        }
                    }
                }
            }
            // Check for ColorButton popups
            else if let Some(color_button) = node.view.as_any().downcast_ref::<ColorButton>() {
                if color_button.is_popup_visible() {
                    if let Some(popup_content_id) = color_button.color_wheel {
                        if self.nodes.contains_key(&popup_content_id) {
                            popups_to_render.push((
                                popup_content_id,
                                color_button.style.rect,
                                color_button.popup_alignment,
                            ));
                        }
                    }
                }
            }
        }

        // Now position and render the popups
        for (popup_id, button_rect, alignment) in popups_to_render {
            let Some(popup_node) = self.nodes.get_mut(&popup_id) else {
                continue;
            };

            // Check if it's a ParamList popup
            if let Some(param_list) = popup_node
                .view
                .as_any_mut()
                .downcast_mut::<crate::ui::ParamList>()
            {
                // Handle ParamList popups
                let widget_updates = {
                    let popup_size = param_list.get_size();

                    // Calculate position (simplified bounds checking - assumes screen is large enough)
                    let [btn_x, btn_y, btn_w, btn_h] = button_rect;
                    let gap = 4.0;

                    let (x, y) = match alignment {
                        crate::ui::PopupAlignment::Right => (btn_x + btn_w + gap, btn_y),
                        crate::ui::PopupAlignment::Left => (btn_x - popup_size[0] - gap, btn_y),
                        crate::ui::PopupAlignment::Bottom => (btn_x, btn_y + btn_h + gap),
                        crate::ui::PopupAlignment::Top => (btn_x, btn_y - popup_size[1] - gap),
                        crate::ui::PopupAlignment::TopLeft => {
                            (btn_x + btn_w - popup_size[0], btn_y - popup_size[1] - gap)
                        }
                        crate::ui::PopupAlignment::TopRight => {
                            (btn_x, btn_y - popup_size[1] - gap) // Align left edge with button left edge
                        }
                        crate::ui::PopupAlignment::BottomLeft => {
                            (btn_x + btn_w - popup_size[0], btn_y + btn_h + gap)
                        }
                        crate::ui::PopupAlignment::BottomRight => {
                            (btn_x + btn_w + gap, btn_y + btn_h + gap)
                        }
                    };

                    param_list.set_position(x, y);

                    // Collect child widget rects
                    let children = popup_node.children.clone();
                    let popup_x = param_list.style.rect[0];
                    let popup_y = param_list.style.rect[1];
                    let num_items = param_list.widget_count();

                    // First N widget children match the ParamList's widget rows - get their rects from ParamList
                    let mut updates = Vec::new();
                    for (index, child_id) in children.iter().enumerate() {
                        if index < num_items {
                            // This is a ParamList item - get its rect from ParamList
                            let widget_rect = param_list.get_widget_rect(index, 180.0);
                            updates.push((*child_id, Some(widget_rect), popup_x, popup_y));
                        } else {
                            // This is an additional child (not a ParamList item)
                            updates.push((*child_id, None, popup_x, popup_y));
                        }
                    }
                    updates
                }; // Borrow of popup_node ends here

                // Now update child widgets
                for (child_id, widget_rect_opt, popup_x, popup_y) in widget_updates {
                    if let Some(child_node) = self.nodes.get_mut(&child_id) {
                        if let Some(widget_rect) = widget_rect_opt {
                            // This is a ParamList item - position it using the rect from ParamList
                            if let Some(slider) = child_node
                                .view
                                .as_any_mut()
                                .downcast_mut::<crate::ui::Slider>()
                            {
                                slider.set_rect(widget_rect);
                            } else if let Some(btn_group) = child_node
                                .view
                                .as_any_mut()
                                .downcast_mut::<crate::ui::ButtonGroup>(
                            ) {
                                // ButtonGroup as ParamList item - use the rect from ParamList
                                btn_group.style.rect = widget_rect;
                            } else if let Some(color_button) = child_node
                                .view
                                .as_any_mut()
                                .downcast_mut::<crate::ui::ColorButton>(
                            ) {
                                // ColorButton as ParamList item - use the rect from ParamList
                                color_button.style.rect = widget_rect;
                            }
                        } else {
                            // This is an additional child (not a ParamList item)
                            if let Some(btn_group) = child_node
                                .view
                                .as_any_mut()
                                .downcast_mut::<crate::ui::ButtonGroup>()
                            {
                                // Store original relative position on first use
                                if btn_group.original_rect.is_none() {
                                    btn_group.original_rect = Some(btn_group.style.rect);
                                }

                                // Position ButtonGroup relative to popup using original coordinates
                                let [rel_x, rel_y, w, h] = btn_group.original_rect.unwrap();
                                btn_group.style.rect = [popup_x + rel_x, popup_y + rel_y, w, h];
                            }
                        }
                    }
                }

                self.build_node(popup_id, out, 100, text_cache); // High layer for popups
            }
            // Check if it's a ColorWheel popup
            else if let Some(color_wheel) = popup_node
                .view
                .as_any_mut()
                .downcast_mut::<crate::ui::ColorWheel>()
            {
                // Handle ColorWheel popups - position it near the button
                let [btn_x, btn_y, btn_w, btn_h] = button_rect;
                let gap = 4.0;
                let wheel_size = 150.0; // Fixed size for color wheel popup

                let (x, y) = match alignment {
                    crate::ui::PopupAlignment::Right => (btn_x + btn_w + gap, btn_y),
                    crate::ui::PopupAlignment::Left => (btn_x - wheel_size - gap, btn_y),
                    crate::ui::PopupAlignment::Bottom => (btn_x, btn_y + btn_h + gap),
                    crate::ui::PopupAlignment::Top => (btn_x, btn_y - wheel_size - gap),
                    crate::ui::PopupAlignment::TopLeft => {
                        (btn_x + btn_w - wheel_size, btn_y - wheel_size - gap)
                    }
                    crate::ui::PopupAlignment::TopRight => {
                        (btn_x + btn_w + gap, btn_y - wheel_size - gap)
                    }
                    crate::ui::PopupAlignment::BottomLeft => {
                        (btn_x + btn_w - wheel_size, btn_y + btn_h + gap)
                    }
                    crate::ui::PopupAlignment::BottomRight => {
                        (btn_x + btn_w + gap, btn_y + btn_h + gap)
                    }
                };

                color_wheel.set_rect([x, y, wheel_size, wheel_size]);
                self.build_node(popup_id, out, 100, text_cache); // High layer for popups
            }
        }
    }

    /// Close all open popups (call this when clicking outside)
    pub fn close_all_popups(&mut self) {
        use crate::ui::{Button, ColorButton};

        for node in self.nodes.values_mut() {
            if let Some(button) = node.view.as_any_mut().downcast_mut::<Button>() {
                if button.is_popup_visible() {
                    button.hide_popup();
                    self.dirty = true;
                }
            } else if let Some(color_button) = node.view.as_any_mut().downcast_mut::<ColorButton>()
            {
                if color_button.is_popup_visible() {
                    color_button.hide_popup();
                    self.dirty = true;
                }
            }
        }
    }

    /// Check if a click is inside any button with a popup or its popup content
    /// Returns true if inside, false if outside (should close popups)
    pub fn is_click_inside_popup_system(&self, pos: [f32; 2]) -> bool {
        use crate::ui::{Button, ColorButton, ColorWheel, ParamList};

        for node in self.nodes.values() {
            // Check regular Button popups
            if let Some(button) = node.view.as_any().downcast_ref::<Button>() {
                if button.is_popup_visible() {
                    // Check if click is on the button itself
                    let [bx, by, bw, bh] = button.style.rect;
                    if pos[0] >= bx && pos[0] <= bx + bw && pos[1] >= by && pos[1] <= by + bh {
                        return true;
                    }

                    // Check if click is inside the popup content
                    if let Some(popup_id) = button.popup_content {
                        if let Some(popup_node) = self.nodes.get(&popup_id) {
                            // Check if it's a ParamList and if click is inside
                            if let Some(param_list) =
                                popup_node.view.as_any().downcast_ref::<ParamList>()
                            {
                                let [px, py, pw, ph] = param_list.style.rect;
                                if pos[0] >= px
                                    && pos[0] <= px + pw
                                    && pos[1] >= py
                                    && pos[1] <= py + ph
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            // Check ColorButton popups
            else if let Some(color_button) = node.view.as_any().downcast_ref::<ColorButton>() {
                if color_button.is_popup_visible() {
                    // Check if click is on the color button itself
                    let [bx, by, bw, bh] = color_button.style.rect;
                    if pos[0] >= bx && pos[0] <= bx + bw && pos[1] >= by && pos[1] <= by + bh {
                        return true;
                    }

                    // Check if click is inside the ColorWheel popup
                    if let Some(popup_id) = color_button.color_wheel {
                        if let Some(popup_node) = self.nodes.get(&popup_id) {
                            if let Some(color_wheel) =
                                popup_node.view.as_any().downcast_ref::<ColorWheel>()
                            {
                                // Check if click is inside the ColorWheel's rect
                                let [wx, wy, ww, wh] = color_wheel.rect;
                                if pos[0] >= wx
                                    && pos[0] <= wx + ww
                                    && pos[1] >= wy
                                    && pos[1] <= wy + wh
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }
}
