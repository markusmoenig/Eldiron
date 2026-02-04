use crate::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

const TREE_INDENT: i32 = 20;
const TREE_VERTICAL_SPACING: i32 = 2;
const TREE_RIGHT_MARGIN: i32 = 2;
const TREE_BOTTOM_MARGIN: i32 = 2;

pub struct TheTreeNode {
    pub id: TheId,

    pub open: bool,
    pub widget: Box<dyn TheWidget>,

    pub childs: Vec<TheTreeNode>,
    pub widgets: Vec<Box<dyn TheWidget>>,

    layout_id: Option<TheId>,
    selected_widget: Option<Uuid>,
    layout_dirty_flag: Arc<AtomicBool>,
}

impl Default for TheTreeNode {
    fn default() -> Self {
        Self::new(TheId::default())
    }
}

impl TheTreeNode {
    pub fn new(id: TheId) -> Self {
        let layout_dirty_flag = Arc::new(AtomicBool::new(false));
        Self::new_with_dirty_flag(id, layout_dirty_flag)
    }

    fn new_with_dirty_flag(id: TheId, layout_dirty_flag: Arc<AtomicBool>) -> Self {
        let mut snapper = TheSnapperbar::new(id.clone());
        snapper.set_associated_layout(id.clone());
        snapper.set_text(id.name.clone());
        snapper.set_open(false);
        Self {
            id: id,
            open: false,
            widget: Box::new(snapper),
            childs: vec![],
            widgets: vec![],
            layout_id: None,
            selected_widget: None,
            layout_dirty_flag,
        }
    }

    fn set_layout_dirty_flag(&mut self, layout_dirty_flag: Arc<AtomicBool>) {
        self.layout_dirty_flag = layout_dirty_flag.clone();
        for child in &mut self.childs {
            child.set_layout_dirty_flag(layout_dirty_flag.clone());
        }
    }

    fn mark_layout_dirty(&self) {
        self.layout_dirty_flag.store(true, Ordering::Relaxed);
    }

    pub fn set_open(&mut self, open: bool) {
        if self.open != open {
            self.mark_layout_dirty();
        }
        self.open = open;
        if let Some(snapper) = self.widget.as_any().downcast_mut::<TheSnapperbar>() {
            snapper.set_open(open);
        }
    }

    pub fn set_root_mode(&mut self, root_mode: bool) {
        if let Some(snapper) = self.widget.as_any().downcast_mut::<TheSnapperbar>() {
            snapper.set_root_mode(root_mode);
        }
    }

    pub fn set_layout_id(&mut self, layout_id: TheId) {
        self.layout_id = Some(layout_id.clone());
        if let Some(snapper) = self.widget.as_any().downcast_mut::<TheSnapperbar>() {
            snapper.set_associated_layout(layout_id.clone());
        }
        for widget in &mut self.widgets {
            if let Some(tree_item) = widget.as_tree_item() {
                tree_item.set_associated_layout(layout_id.clone());
            } else if let Some(tree_icons) = widget.as_tree_icons() {
                tree_icons.set_associated_layout(layout_id.clone());
            } else if let Some(tree_text) = widget.as_tree_text() {
                tree_text.set_associated_layout(layout_id.clone());
            }
        }
        for child in &mut self.childs {
            child.set_layout_id(layout_id.clone());
            child.set_layout_dirty_flag(self.layout_dirty_flag.clone());
        }
    }

    pub fn add_child(&mut self, mut node: TheTreeNode) {
        if let Some(layout_id) = &self.layout_id {
            node.set_layout_id(layout_id.clone());
        }
        node.set_layout_dirty_flag(self.layout_dirty_flag.clone());
        self.childs.push(node);
        self.mark_layout_dirty();
    }

    pub fn add_child_at(&mut self, index: usize, mut node: TheTreeNode) {
        if let Some(layout_id) = &self.layout_id {
            node.set_layout_id(layout_id.clone());
        }
        node.set_layout_dirty_flag(self.layout_dirty_flag.clone());
        self.childs.insert(index, node);
        self.mark_layout_dirty();
    }

    pub fn add_widget(&mut self, mut widget: Box<dyn TheWidget>) {
        if let Some(layout_id) = &self.layout_id {
            if let Some(tree_item) = widget.as_tree_item() {
                tree_item.set_associated_layout(layout_id.clone());
            } else if let Some(tree_icons) = widget.as_tree_icons() {
                tree_icons.set_associated_layout(layout_id.clone());
            } else if let Some(tree_text) = widget.as_tree_text() {
                tree_text.set_associated_layout(layout_id.clone());
            }
        }
        self.widgets.push(widget);
        self.mark_layout_dirty();
    }

    pub fn clear_selection(&mut self) {
        for widget in &mut self.widgets {
            widget.set_state(TheWidgetState::None);
        }
        self.selected_widget = None;
        for child in &mut self.childs {
            child.clear_selection();
        }
    }

    pub fn new_item_selected(&mut self, item: &TheId) -> bool {
        let mut handled_here = false;

        for widget in &mut self.widgets {
            if widget.id().matches(Some(&item.name), Some(&item.uuid)) {
                widget.set_state(TheWidgetState::Selected);
                handled_here = true;
                self.selected_widget = Some(item.uuid);
            } else {
                widget.set_state(TheWidgetState::None);
            }
        }

        if handled_here {
            // Clear selection of all descendants so only one item stays selected.
            for child in &mut self.childs {
                child.clear_selection();
            }
            return true;
        }

        let mut handled_in_child = false;
        for child in &mut self.childs {
            if child.new_item_selected(item) {
                handled_in_child = true;
            } else {
                child.clear_selection();
            }
        }

        if handled_in_child {
            for widget in &mut self.widgets {
                widget.set_state(TheWidgetState::None);
            }
            self.selected_widget = None;
            return true;
        }

        false
    }

    pub fn remove_child_by_uuid(&mut self, uuid: &Uuid) -> Option<TheTreeNode> {
        if let Some(index) = self.childs.iter().position(|child| child.id.uuid == *uuid) {
            self.mark_layout_dirty();
            return Some(self.childs.remove(index));
        }

        for child in &mut self.childs {
            if let Some(removed) = child.remove_child_by_uuid(uuid) {
                self.mark_layout_dirty();
                return Some(removed);
            }
        }

        None
    }

    pub fn remove_widget_by_uuid(&mut self, uuid: &Uuid) -> Option<Box<dyn TheWidget>> {
        if let Some(index) = self
            .widgets
            .iter()
            .position(|widget| widget.id().uuid == *uuid)
        {
            let removed = self.widgets.remove(index);
            if self.selected_widget == Some(*uuid) {
                self.selected_widget = None;
            }
            self.mark_layout_dirty();
            return Some(removed);
        }

        for child in &mut self.childs {
            if let Some(removed) = child.remove_widget_by_uuid(uuid) {
                self.mark_layout_dirty();
                return Some(removed);
            }
        }

        None
    }

    pub fn find_node(&self, uuid: &Uuid) -> Option<&TheTreeNode> {
        if self.id.uuid == *uuid {
            return Some(self);
        }

        for child in &self.childs {
            if let Some(found) = child.find_node(uuid) {
                return Some(found);
            }
        }

        None
    }

    pub fn find_node_mut(&mut self, uuid: &Uuid) -> Option<&mut TheTreeNode> {
        if self.id.uuid == *uuid {
            return Some(self);
        }

        for child in &mut self.childs {
            if let Some(found) = child.find_node_mut(uuid) {
                return Some(found);
            }
        }

        None
    }

    pub fn node_state_changed(&mut self, id: TheId, open: bool) {
        self.node_state_changed_internal(&id, open);
    }

    fn node_state_changed_internal(&mut self, id: &TheId, open: bool) -> bool {
        if self.id.matches(Some(&id.name), Some(&id.uuid)) {
            self.open = open;
            if let Some(snapper) = self.widget.as_any().downcast_mut::<TheSnapperbar>() {
                snapper.set_open(open);
            }
            return true;
        }

        for child in &mut self.childs {
            if child.node_state_changed_internal(id, open) {
                return true;
            }
        }

        false
    }

    fn layout(
        &mut self,
        origin: Vec2<i32>,
        available_width: i32,
        max_height: i32,
        indent: i32,
        include_self: bool,
        y_cursor: &mut i32,
        ctx: &mut TheContext,
    ) {
        if include_self {
            self.widget.calculate_size(ctx);

            let node_width = (available_width - indent - TREE_RIGHT_MARGIN).max(0);
            let node_height = self.widget.limiter().get_height(max_height);

            // Make items 2 pixels taller for transparent hit areas (1px top, 1px bottom)
            let hit_height = node_height + 2;

            self.widget.set_dim(
                TheDim::new(
                    origin.x + indent,
                    origin.y + *y_cursor,
                    node_width,
                    hit_height,
                ),
                ctx,
            );
            // Position content 1 pixel lower within the larger buffer area
            self.widget
                .dim_mut()
                .set_buffer_offset(indent, *y_cursor + 1);

            *y_cursor += hit_height + TREE_VERTICAL_SPACING - 2;
        }

        if !self.open {
            return;
        }

        let child_indent = if include_self {
            indent + TREE_INDENT
        } else {
            indent
        };

        for widget in &mut self.widgets {
            widget.calculate_size(ctx);

            let available_child_width = (available_width - child_indent - TREE_RIGHT_MARGIN).max(0);
            let widget_width = widget.limiter().get_width(available_child_width);
            let widget_height = widget.limiter().get_height(max_height);

            // Make embedded widgets 2 pixels taller for transparent hit areas (1px top, 1px bottom)
            let hit_height = widget_height + 2;

            widget.set_dim(
                TheDim::new(
                    origin.x + child_indent,
                    origin.y + *y_cursor,
                    widget_width,
                    hit_height,
                ),
                ctx,
            );
            // Position content 1 pixel lower within the larger buffer area
            widget
                .dim_mut()
                .set_buffer_offset(child_indent, *y_cursor + 1);

            *y_cursor += hit_height + TREE_VERTICAL_SPACING - 2;
        }

        for child in &mut self.childs {
            child.layout(
                origin,
                available_width,
                max_height,
                child_indent,
                true,
                y_cursor,
                ctx,
            );
        }
    }

    fn apply_scroll_offset(&mut self, offset: i32) {
        // Apply scroll offset to embedded widgets
        for widget in &mut self.widgets {
            if let Some(tree_item) = widget.as_tree_item() {
                tree_item.set_scroll_offset(offset);
            } else if let Some(tree_icons) = widget.as_tree_icons() {
                tree_icons.set_scroll_offset(offset);
            } else if let Some(tree_text) = widget.as_tree_text() {
                tree_text.set_scroll_offset(offset);
            }
        }

        // Recursively apply to child nodes
        if self.open {
            for child in &mut self.childs {
                child.apply_scroll_offset(offset);
            }
        }
    }

    fn draw_recursive(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
        include_self: bool,
    ) {
        if include_self {
            self.widget.draw(buffer, style, ctx);
        }

        if !self.open {
            return;
        }

        // Draw embedded widgets only when node is open
        for widget in &mut self.widgets {
            widget.draw(buffer, style, ctx);
        }

        for child in &mut self.childs {
            child.draw_recursive(buffer, style, ctx, true);
        }
    }

    fn find_widget_at_coord(
        &mut self,
        coord: Vec2<i32>,
        include_self: bool,
    ) -> Option<&mut Box<dyn TheWidget>> {
        if include_self && self.widget.dim().contains(coord) {
            return Some(&mut self.widget);
        }

        if self.open {
            // Check embedded widgets only when node is open
            for widget in &mut self.widgets {
                if widget.dim().contains(coord) {
                    return Some(widget);
                }
            }

            for child in &mut self.childs {
                if let Some(found) = child.find_widget_at_coord(coord, true) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn find_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
        include_self: bool,
    ) -> Option<&mut Box<dyn TheWidget>> {
        if include_self && self.widget.id().matches(name, uuid) {
            return Some(&mut self.widget);
        }

        for widget in &mut self.widgets {
            if widget.id().matches(name, uuid) {
                return Some(widget);
            }
        }

        for child in &mut self.childs {
            if let Some(found) = child.find_widget(name, uuid, true) {
                return Some(found);
            }
        }

        None
    }

    fn needs_redraw_recursive(&mut self, include_self: bool) -> bool {
        if include_self && self.widget.needs_redraw() {
            return true;
        }

        for widget in &mut self.widgets {
            if widget.needs_redraw() {
                return true;
            }
        }

        for child in &mut self.childs {
            if child.needs_redraw_recursive(true) {
                return true;
            }
        }

        false
    }
}

pub struct TheTreeLayout {
    id: TheId,
    limiter: TheSizeLimiter,

    dim: TheDim,

    root: TheTreeNode,

    widgets: Vec<Box<dyn TheWidget>>,

    content_buffer: TheRGBABuffer,

    vertical_scrollbar: Box<dyn TheWidget>,
    vertical_scrollbar_visible: bool,

    background: Option<TheThemeColors>,
    headerless: bool,
    layout_dirty_flag: Arc<AtomicBool>,
}

impl TheLayout for TheTreeLayout {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let layout_dirty_flag = Arc::new(AtomicBool::new(false));
        let mut root = TheTreeNode::new(id.clone());
        root.set_layout_dirty_flag(layout_dirty_flag.clone());
        root.set_layout_id(id.clone());
        root.open = true;
        Self {
            id: id.clone(),
            limiter: TheSizeLimiter::new(),

            dim: TheDim::zero(),

            root,

            widgets: vec![],
            content_buffer: TheRGBABuffer::empty(),

            vertical_scrollbar: Box::new(TheVerticalScrollbar::new(TheId::named(
                "Vertical Scrollbar",
            ))),
            vertical_scrollbar_visible: false,

            background: Some(TextLayoutBackground),
            headerless: true,
            layout_dirty_flag,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_background_color(&mut self, color: Option<TheThemeColors>) {
        self.background = color;
    }

    fn widgets(&mut self) -> &mut Vec<Box<dyn TheWidget>> {
        &mut self.widgets
    }

    fn supports_mouse_wheel(&self) -> bool {
        true
    }

    fn mouse_wheel_scroll(&mut self, delta: Vec2<i32>) {
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_bar.scroll_by(-delta.y);
            // Apply the new scroll offset to all widgets
            let offset = scroll_bar.scroll_offset();
            self.root.apply_scroll_offset(offset);
        }
    }

    fn get_layout_at_coord(&mut self, coord: Vec2<i32>) -> Option<TheId> {
        if self.dim.contains(coord) {
            return Some(self.id.clone());
        }
        None
    }

    fn get_widget_at_coord(&mut self, coord: Vec2<i32>) -> Option<&mut Box<dyn TheWidget>> {
        if self.layout_dirty_flag.load(Ordering::Relaxed) {
            return None;
        }
        if !self.dim.contains(coord) {
            return None;
        }
        if self.vertical_scrollbar_visible && self.vertical_scrollbar.dim().contains(coord) {
            return Some(&mut self.vertical_scrollbar);
        }

        let mut scroll_offset = Vec2::zero();
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_offset = Vec2::new(0, scroll_bar.scroll_offset());
        }

        let adjusted_coord = coord + scroll_offset;

        if let Some(widget) = self
            .root
            .find_widget_at_coord(adjusted_coord, !self.headerless)
        {
            return Some(widget);
        }

        None
    }

    fn get_widget(
        &mut self,
        name: Option<&String>,
        uuid: Option<&Uuid>,
    ) -> Option<&mut Box<dyn TheWidget>> {
        if self.vertical_scrollbar_visible && self.vertical_scrollbar.id().matches(name, uuid) {
            return Some(&mut self.vertical_scrollbar);
        }

        if let Some(widget) = self.root.find_widget(name, uuid, !self.headerless) {
            return Some(widget);
        }

        None
    }

    fn needs_redraw(&mut self) -> bool {
        if self.vertical_scrollbar_visible && self.vertical_scrollbar.needs_redraw() {
            return true;
        }

        if self.root.needs_redraw_recursive(!self.headerless) {
            return true;
        }

        true
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    /// Relayouts the layout.
    fn relayout(&mut self, _ctx: &mut TheContext) {
        self.dim = TheDim::zero();
    }

    fn set_dim(&mut self, dim: TheDim, ctx: &mut TheContext) {
        if self.dim != dim || ctx.ui.relayout || self.layout_dirty_flag.load(Ordering::Relaxed) {
            self.dim = dim;
            self.recalculate_layout(ctx);
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        if self.layout_dirty_flag.load(Ordering::Relaxed) {
            self.recalculate_layout(ctx);
        }

        // let stride: usize = buffer.stride();
        // if let Some(background) = self.background {
        //     ctx.draw.rect(
        //         buffer.pixels_mut(),
        //         &self.dim.to_buffer_utuple(),
        //         stride,
        //         style.theme().color(background),
        //     );

        //     // ctx.draw.rect_outline(
        //     //     buffer.pixels_mut(),
        //     //     &self.dim.to_buffer_utuple(),
        //     //     stride,
        //     //     style.theme().color(TextLayoutBorder),
        //     // );
        // }

        let stride = self.content_buffer.stride();
        let utuple: (usize, usize, usize, usize) = self.content_buffer.dim().to_buffer_utuple();

        // Clear entire content buffer with transparency first
        ctx.draw.rect(
            self.content_buffer.pixels_mut(),
            &utuple,
            stride,
            &[0, 0, 0, 0], // Transparent
        );

        if let Some(background) = self.background {
            ctx.draw.rect(
                self.content_buffer.pixels_mut(),
                &utuple,
                stride,
                style.theme().color(background),
            );
        }

        if self.vertical_scrollbar_visible {
            self.vertical_scrollbar.draw(buffer, style, ctx);
        }

        self.root
            .draw_recursive(&mut self.content_buffer, style, ctx, !self.headerless);

        if self.vertical_scrollbar_visible {
            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
                let offset = scroll_bar.scroll_offset();
                let range = offset..offset + self.dim.height;
                // Safety check: ensure we don't copy beyond content buffer bounds
                let content_height = self.content_buffer.dim().height as i32;
                let safe_range = range.start.min(content_height)..range.end.min(content_height);
                buffer.copy_vertical_range_into(
                    self.dim.buffer_x,
                    self.dim.buffer_y,
                    &self.content_buffer,
                    safe_range,
                );
            }
        } else {
            let range = 0..self.dim.height;
            // Safety check: ensure we don't copy beyond content buffer bounds
            let content_height = self.content_buffer.dim().height as i32;
            let safe_range = 0..range.end.min(content_height);
            buffer.copy_vertical_range_into(
                self.dim.buffer_x,
                self.dim.buffer_y,
                &self.content_buffer,
                safe_range,
            );
        }

        if self.background.is_some() {
            let stride: usize = buffer.stride();
            ctx.draw.rect_outline(
                buffer.pixels_mut(),
                &self.dim.to_buffer_utuple(),
                stride,
                style.theme().color(TextLayoutBorder),
            );
        }
    }

    fn as_tree_layout(&mut self) -> Option<&mut dyn TheTreeLayoutTrait> {
        Some(self)
    }
}

/// TheTreeLayout specific functions.
pub trait TheTreeLayoutTrait: TheLayout {
    /// Set whether the root snapper should be hidden.
    fn set_headerless(&mut self, headerless: bool);
    /// Returns true if the layout is headerless.
    fn is_headerless(&self) -> bool;
    /// Returns a reference to the node with the given uuid (if any).
    fn get_node_by_id(&self, uuid: &Uuid) -> Option<&TheTreeNode>;
    /// Returns a mutable reference to the node with the given uuid (if any).
    fn get_node_by_id_mut(&mut self, uuid: &Uuid) -> Option<&mut TheTreeNode>;
    /// Notifies the layout that a tree item was selected.
    fn new_item_selected(&mut self, item: TheId);
    /// Get the root node
    fn get_root(&mut self) -> &mut TheTreeNode;
    /// Set the state of a node
    fn tree_node_state_changed(&mut self, id: TheId, open: bool);
    /// Scroll by the given amount.
    fn scroll_by(&mut self, delta: Vec2<i32>);
}

impl TheTreeLayoutTrait for TheTreeLayout {
    fn set_headerless(&mut self, headerless: bool) {
        self.headerless = headerless;
        if self.headerless {
            self.root.open = true;
        }
    }
    fn is_headerless(&self) -> bool {
        self.headerless
    }
    fn get_node_by_id(&self, uuid: &Uuid) -> Option<&TheTreeNode> {
        self.root.find_node(uuid)
    }
    fn get_node_by_id_mut(&mut self, uuid: &Uuid) -> Option<&mut TheTreeNode> {
        self.root.find_node_mut(uuid)
    }
    fn new_item_selected(&mut self, item: TheId) {
        if !self.root.new_item_selected(&item) {
            self.root.clear_selection();
        }
    }
    fn get_root(&mut self) -> &mut TheTreeNode {
        &mut self.root
    }
    fn tree_node_state_changed(&mut self, id: TheId, open: bool) {
        self.root.node_state_changed(id, open);
    }
    fn scroll_by(&mut self, delta: Vec2<i32>) {
        if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
            scroll_bar.scroll_by(-delta.y);
            // Apply the new scroll offset to all widgets
            let offset = scroll_bar.scroll_offset();
            self.root.apply_scroll_offset(offset);
        }
    }
}

impl TheTreeLayout {
    fn recalculate_layout(&mut self, ctx: &mut TheContext) {
        if self.dim.width <= 0 || self.dim.height <= 0 {
            self.layout_dirty_flag.store(false, Ordering::Relaxed);
            return;
        }

        let dim = self.dim;
        let scrollbar_width = 13;
        let mut available_width = dim.width;
        let origin = Vec2::new(dim.x, dim.y);

        loop {
            let mut y_cursor = 0;
            self.root.layout(
                origin,
                available_width,
                dim.height + 100,
                0,
                !self.headerless,
                &mut y_cursor,
                ctx,
            );

            let mut content_height = y_cursor;
            if content_height > 0 {
                // Widgets grow by 2px for transparent hit areas, so only remove the effective spacing
                let trailing_spacing = (TREE_VERTICAL_SPACING - 2).max(0);
                content_height = (content_height - trailing_spacing).max(0);
            }

            let mut total_height = content_height.max(dim.height);

            self.vertical_scrollbar.set_dim(
                TheDim::new(
                    dim.x + dim.width - scrollbar_width,
                    dim.y,
                    scrollbar_width,
                    dim.height,
                ),
                ctx,
            );
            self.vertical_scrollbar.dim_mut().set_buffer_offset(
                self.dim.buffer_x + dim.width - scrollbar_width,
                self.dim.buffer_y,
            );

            let mut scrollbar_visible = false;
            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
                scroll_bar.set_total_height(total_height);
                scrollbar_visible = scroll_bar.needs_scrollbar();
                if scrollbar_visible {
                    total_height += TREE_BOTTOM_MARGIN;
                    scroll_bar.set_total_height(total_height);
                }

                // Clamp scroll offset to valid range after content height change
                let current_offset = scroll_bar.scroll_offset();
                let max_offset = (total_height - dim.height).max(0);
                if current_offset > max_offset {
                    scroll_bar.set_scroll_offset(max_offset);
                }

                // Reset scroll offset to 0 when scrollbar becomes not visible
                if !scrollbar_visible && current_offset != 0 {
                    scroll_bar.set_scroll_offset(0);
                }
            }

            self.content_buffer
                .set_dim(TheDim::new(0, 0, available_width, total_height));

            // Apply scroll offset to all widgets
            if let Some(scroll_bar) = self.vertical_scrollbar.as_vertical_scrollbar() {
                self.root.apply_scroll_offset(scroll_bar.scroll_offset());
            }

            if scrollbar_visible && available_width == dim.width {
                available_width = (dim.width - scrollbar_width).max(0);
                continue;
            } else if !scrollbar_visible && available_width != dim.width {
                available_width = dim.width;
                continue;
            } else {
                self.vertical_scrollbar_visible = scrollbar_visible;
                break;
            }
        }

        self.layout_dirty_flag.store(false, Ordering::Relaxed);
    }
}
