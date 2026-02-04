use crate::prelude::*;
use indexmap::IndexMap;
use std::ops::RangeInclusive;

/// The items that can be added to TheNodeUI
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum TheNodeUIItem {
    /// Text: Id, Name, Status, Value, DefaultValue, Continuous
    Text(String, String, String, String, Option<String>, bool),
    /// Text: Id, Value
    Markdown(String, String),
    /// Selector: Id, Name, Status, Values, Value
    Selector(String, String, String, Vec<String>, i32),
    /// Float Edit Slider: Id, Name, Status, Value, Range, Continuous
    FloatEditSlider(String, String, String, f32, RangeInclusive<f32>, bool),
    /// Float Slider: Id, Name, Status, Value, Range, DefaultValue, Continuous
    FloatSlider(String, String, String, f32, RangeInclusive<f32>, f32, bool),
    /// Int Edit Slider: Id, Name, Status, Value, Range, Continuous
    IntEditSlider(String, String, String, i32, RangeInclusive<i32>, bool),
    /// Palette Slider: Id, Name, Status, Value, ThePalette, Continuous
    PaletteSlider(String, String, String, i32, ThePalette, bool),
    /// Int Slider: Id, Name, Status, Value, Range, DefaultValue, Continuous
    IntSlider(String, String, String, i32, RangeInclusive<i32>, i32, bool),
    /// Button: Id, Name, Status, LayoutText
    Button(String, String, String, String),
    /// Text: Id, Name, Status, Value, DefaultValue, Continuous
    ColorPicker(String, String, String, TheColor, bool),
    /// Checkbox: Id, Name, Status, Value,
    Checkbox(String, String, String, bool),
    /// Separator: Name
    Separator(String),
    /// Icons: Id, Name, Status, (Buffer, Name, Id)
    Icons(String, String, String, Vec<(TheRGBABuffer, String, Uuid)>),
    /// Open Tree
    OpenTree(String),
    /// Open Tree
    CloseTree,
}

impl TheNodeUIItem {
    /// Returns the `id` for the item
    pub fn id(&self) -> &str {
        match self {
            TheNodeUIItem::Text(id, _, _, _, _, _) => id,
            TheNodeUIItem::Markdown(id, _) => id,
            TheNodeUIItem::Selector(id, _, _, _, _) => id,
            TheNodeUIItem::FloatEditSlider(id, _, _, _, _, _) => id,
            TheNodeUIItem::FloatSlider(id, _, _, _, _, _, _) => id,
            TheNodeUIItem::IntEditSlider(id, _, _, _, _, _) => id,
            TheNodeUIItem::PaletteSlider(id, _, _, _, _, _) => id,
            TheNodeUIItem::IntSlider(id, _, _, _, _, _, _) => id,
            TheNodeUIItem::Button(id, _, _, _) => id,
            TheNodeUIItem::ColorPicker(id, _, _, _, _) => id,
            TheNodeUIItem::Checkbox(id, _, _, _) => id,
            TheNodeUIItem::Separator(name) => name,
            TheNodeUIItem::Icons(id, _, _, _) => id,
            TheNodeUIItem::OpenTree(_) => "OpenTree",
            TheNodeUIItem::CloseTree => "CloseTree",
        }
    }
}

use TheNodeUIItem::*;

/// A container for UI items. Supports adding them to a text layout or handling events for updating the values.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TheNodeUI {
    items: IndexMap<String, TheNodeUIItem>,
}

impl Default for TheNodeUI {
    fn default() -> Self {
        Self::new()
    }
}

impl TheNodeUI {
    pub fn new() -> Self {
        Self {
            items: IndexMap::new(),
        }
    }

    /// Adds a new item to the UI
    pub fn add_item(&mut self, item: TheNodeUIItem) -> Option<TheNodeUIItem> {
        self.items.insert(item.id().into(), item)
    }

    /// Removes an item by its ID
    pub fn remove_item(&mut self, id: &str) -> Option<TheNodeUIItem> {
        self.items.shift_remove(id)
    }

    /// Retrieves a reference to an item by its ID
    pub fn get_item(&self, id: &str) -> Option<&TheNodeUIItem> {
        self.items.get(id)
    }

    /// Retrieves a mutable reference to an item by its ID
    pub fn get_item_mut(&mut self, id: &str) -> Option<&mut TheNodeUIItem> {
        self.items.get_mut(id)
    }

    /// Lists all items in the UI
    pub fn list_items(&self) -> impl Iterator<Item = (&String, &TheNodeUIItem)> {
        self.items.iter()
    }

    /// Returns the item count.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if there are no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get a text value.
    pub fn get_text_value(&self, id: &str) -> Option<String> {
        for (item_id, item) in &self.items {
            if id == item_id {
                match item {
                    Text(_, _, _, value, _, _) => {
                        return Some(value.clone());
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Get a bool value.
    pub fn get_bool_value(&self, id: &str) -> Option<bool> {
        for (item_id, item) in &self.items {
            if id == item_id {
                match item {
                    &Checkbox(_, _, _, value) => {
                        return Some(value);
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Get a tile id for the given index
    pub fn get_tile_id(&self, id: &str, index: usize) -> Option<Uuid> {
        for (item_id, item) in &self.items {
            if id == item_id {
                match item {
                    Icons(_, _, _, items) => {
                        if index < items.len() {
                            return Some(items[index].2);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Get an i32 value.
    pub fn get_i32_value(&self, id: &str) -> Option<i32> {
        for (item_id, item) in &self.items {
            if id == item_id {
                match item {
                    IntEditSlider(_, _, _, value, _, _) => {
                        return Some(*value);
                    }
                    IntSlider(_, _, _, value, _, _, _) => {
                        return Some(*value);
                    }
                    Selector(_, _, _, _, value) => {
                        return Some(*value);
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Get an f32 value.
    pub fn get_f32_value(&self, id: &str) -> Option<f32> {
        for (item_id, item) in &self.items {
            if id == item_id {
                match item {
                    FloatEditSlider(_, _, _, value, _, _) => {
                        return Some(*value);
                    }
                    FloatSlider(_, _, _, value, _, _, _) => {
                        return Some(*value);
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Set a bool value.
    pub fn set_bool_value(&mut self, id: &str, val: bool) {
        for (item_id, item) in &mut self.items {
            if id == item_id {
                match item {
                    Checkbox(_, _, _, value) => {
                        *value = val;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Set an f32 value.
    pub fn set_f32_value(&mut self, id: &str, val: f32) {
        for (item_id, item) in &mut self.items {
            if id == item_id {
                match item {
                    FloatEditSlider(_, _, _, value, _, _) => {
                        *value = val;
                    }
                    FloatSlider(_, _, _, value, _, _, _) => {
                        *value = val;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Set an f32 value.
    pub fn set_i32_value(&mut self, id: &str, val: i32) {
        for (item_id, item) in &mut self.items {
            if id == item_id {
                match item {
                    IntEditSlider(_, _, _, value, _, _) => {
                        *value = val;
                    }
                    IntSlider(_, _, _, value, _, _, _) => {
                        *value = val;
                    }
                    Selector(_, _, _, _, value) => {
                        *value = val;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Set a text value.
    pub fn set_text_value(&mut self, id: &str, val: String) {
        for (item_id, item) in &mut self.items {
            if id == item_id {
                match item {
                    Text(_, _, _, value, _, _) => {
                        *value = val.clone();
                    }
                    _ => {}
                }
            }
        }
    }

    /// Add the items to the given text layout.
    pub fn apply_to_tree_node(&self, node: &mut TheTreeNode) {
        node.widgets.clear();
        node.childs.clear();

        let mut group: Option<TheTreeNode> = None;

        for (_, item) in &self.items {
            match item {
                Text(id, name, status, value, default_value, continous) => {
                    let mut edit = TheTextLineEdit::new(TheId::named(id));
                    edit.set_text(value.clone());
                    edit.set_continuous(*continous);
                    edit.set_status_text(status);
                    edit.set_info_text(default_value.clone());

                    let mut item = TheTreeItem::new(TheId::named("Text"));
                    item.set_text(name.clone());
                    item.set_status_text(status);
                    item.add_widget_column(200, Box::new(edit));

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                Icons(id, _name, status, vec) => {
                    let mut item = TheTreeIcons::new(TheId::named(id));
                    item.set_status_text(status);
                    item.set_icon_size(32);
                    item.set_icon_count(vec.len());
                    item.set_selected_index(Some(0));

                    for (index, icon) in vec.iter().enumerate() {
                        item.set_text(index, icon.1.clone());
                        item.set_icon(index, icon.0.clone());
                    }

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                Markdown(_, _text) => {
                    //     let mut view = TheMarkdownView::new(TheId::named(id));
                    //     view.set_text(text.clone());
                    //     view.set_font_size(12.5);
                    //     view.limiter_mut().set_max_width(360);

                    // let mut item = TheTreeText::new(TheId::named("FloatEditSlider"));
                    // item.set_text(text.clone());
                    // // item.add_widget_column(200, Box::new(view));
                    // // item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));

                    // if let Some(ref mut g) = group {
                    //     g.add_widget(Box::new(item));
                    // } else {
                    //     node.add_widget(Box::new(item));
                    // }
                }
                Selector(id, name, status, values, value) => {
                    let mut dropdown = TheDropdownMenu::new(TheId::named(id));
                    for item in values {
                        dropdown.add_option(item.clone());
                    }
                    dropdown.set_selected_index(*value);
                    dropdown.set_status_text(status);

                    let mut item = TheTreeItem::new(TheId::named("FloatEditSlider"));
                    item.set_text(name.clone());
                    item.add_widget_column(200, Box::new(dropdown));
                    item.set_status_text(status);

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                FloatEditSlider(id, name, status, value, range, continous) => {
                    let mut slider = TheTextLineEdit::new(TheId::named(id));
                    slider.set_value(TheValue::Float(*value));
                    if *range.start() != 0.0 || *range.end() != 0.0 {
                        slider.set_range(TheValue::RangeF32(range.clone()));
                    }
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);

                    let mut item = TheTreeItem::new(TheId::named("FloatEditSlider"));
                    item.set_text(name.clone());
                    item.add_widget_column(200, Box::new(slider));
                    item.set_status_text(status);

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                FloatSlider(id, name, status, value, range, default_value, continous) => {
                    let mut slider = TheSlider::new(TheId::named(id));
                    slider.set_value(TheValue::Float(*value));
                    slider.set_default_value(TheValue::Float(*default_value));
                    slider.set_range(TheValue::RangeF32(range.clone()));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);

                    let mut item = TheTreeItem::new(TheId::named("FloatSlider"));
                    item.set_text(name.clone());
                    item.add_widget_column(200, Box::new(slider));
                    item.set_status_text(status);

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                IntEditSlider(id, name, status, value, range, continous) => {
                    let mut slider = TheTextLineEdit::new(TheId::named(id));
                    slider.set_value(TheValue::Int(*value));
                    if *range.start() != 0 || *range.end() != 0 {
                        slider.set_range(TheValue::RangeI32(range.clone()));
                    }
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);

                    let mut item = TheTreeItem::new(TheId::named("IntEditSlider"));
                    item.set_text(name.clone());
                    item.add_widget_column(200, Box::new(slider));
                    item.set_status_text(status);

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                PaletteSlider(id, name, status, value, palette, continous) => {
                    let mut slider = TheTextLineEdit::new(TheId::named(id));
                    slider.set_value(TheValue::Int(*value));
                    slider.set_range(TheValue::RangeI32(0..=255));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);
                    slider.set_palette(palette.clone());

                    let mut item = TheTreeItem::new(TheId::named("PaletteSlider"));
                    item.set_text(name.clone());
                    item.add_widget_column(200, Box::new(slider));
                    item.set_status_text(status);

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                IntSlider(id, name, status, value, range, default_value, continous) => {
                    let mut slider = TheSlider::new(TheId::named(id));
                    slider.set_value(TheValue::Int(*value));
                    slider.set_default_value(TheValue::Int(*default_value));
                    slider.set_range(TheValue::RangeI32(range.clone()));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);

                    let mut item = TheTreeItem::new(TheId::named("IntSlider"));
                    item.set_text(name.clone());
                    item.add_widget_column(200, Box::new(slider));
                    item.set_status_text(status);

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                // Button(id, name, status, layout_text) => {
                //     let mut button = TheTraybarButton::new(TheId::named(id));
                //     button.set_text(name.clone());
                //     button.set_status_text(status);
                //     layout.add_pair(layout_text.clone(), Box::new(button));
                // }
                // ColorPicker(id, name, status, value, continuous) => {
                //     let mut picker = TheColorPicker::new(TheId::named(id));
                //     picker.set_value(TheValue::ColorObject(value.clone()));
                //     picker.set_status_text(status);
                //     picker.set_continuous(*continuous);
                //     picker.limiter_mut().set_max_size(Vec2::new(200, 200));
                //     layout.add_pair(name.clone(), Box::new(picker));
                // }
                Checkbox(id, name, status, value) => {
                    let mut cb = TheCheckButton::new(TheId::named(id));
                    cb.set_value(TheValue::Bool(*value));
                    cb.set_status_text(status);

                    let mut item = TheTreeItem::new(TheId::named("Checkbox"));
                    item.set_text(name.clone());
                    item.add_widget_column(200, Box::new(cb));
                    item.set_status_text(status);

                    if let Some(ref mut g) = group {
                        g.add_widget(Box::new(item));
                    } else {
                        node.add_widget(Box::new(item));
                    }
                }
                OpenTree(name) => {
                    let mut group_node = TheTreeNode::new(TheId::empty());
                    group_node.widget.set_value(TheValue::Text(name.clone()));
                    group_node.set_root_mode(false);
                    group_node.set_open(true);
                    group = Some(group_node);
                }
                CloseTree => {
                    if let Some(group) = group.take() {
                        node.add_child(group);
                    }
                }

                // Separator(name) => {
                //     let sep = TheSeparator::new(TheId::named_with_id("Separator", Uuid::new_v4()));
                //     layout.add_pair(name.clone(), Box::new(sep));
                // }
                _ => {}
            }
        }
    }

    /// Add the items to the given text layout.
    pub fn apply_to_text_layout(&self, layout: &mut dyn TheTextLayoutTrait) {
        layout.clear();
        for (_, item) in &self.items {
            match item {
                Text(id, name, status, value, default_value, continous) => {
                    let mut edit = TheTextLineEdit::new(TheId::named(id));
                    edit.set_text(value.clone());
                    edit.set_continuous(*continous);
                    edit.set_status_text(status);
                    edit.set_info_text(default_value.clone());
                    layout.add_pair(name.clone(), Box::new(edit));
                }
                Markdown(id, text) => {
                    let mut view = TheMarkdownView::new(TheId::named(id));
                    view.set_text(text.clone());
                    view.set_font_size(12.5);
                    view.limiter_mut().set_max_width(360);
                    layout.add_pair("".into(), Box::new(view));
                }
                Selector(id, name, status, values, value) => {
                    let mut dropdown = TheDropdownMenu::new(TheId::named(id));
                    for item in values {
                        dropdown.add_option(item.clone());
                    }
                    dropdown.set_selected_index(*value);
                    dropdown.set_status_text(status);
                    layout.add_pair(name.clone(), Box::new(dropdown));
                }
                FloatEditSlider(id, name, status, value, range, continous) => {
                    let mut slider = TheTextLineEdit::new(TheId::named(id));
                    slider.set_value(TheValue::Float(*value));
                    slider.set_range(TheValue::RangeF32(range.clone()));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);
                    layout.add_pair(name.clone(), Box::new(slider));
                }
                FloatSlider(id, name, status, value, range, default_value, continous) => {
                    let mut slider = TheSlider::new(TheId::named(id));
                    slider.set_value(TheValue::Float(*value));
                    slider.set_default_value(TheValue::Float(*default_value));
                    slider.set_range(TheValue::RangeF32(range.clone()));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);
                    layout.add_pair(name.clone(), Box::new(slider));
                }
                IntEditSlider(id, name, status, value, range, continous) => {
                    let mut slider = TheTextLineEdit::new(TheId::named(id));
                    slider.set_value(TheValue::Int(*value));
                    slider.set_range(TheValue::RangeI32(range.clone()));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);
                    layout.add_pair(name.clone(), Box::new(slider));
                }
                PaletteSlider(id, name, status, value, palette, continous) => {
                    let mut slider = TheTextLineEdit::new(TheId::named(id));
                    slider.set_value(TheValue::Int(*value));
                    slider.set_range(TheValue::RangeI32(0..=255));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);
                    slider.set_palette(palette.clone());
                    layout.add_pair(name.clone(), Box::new(slider));
                }
                IntSlider(id, name, status, value, range, default_value, continous) => {
                    let mut slider = TheSlider::new(TheId::named(id));
                    slider.set_value(TheValue::Int(*value));
                    slider.set_default_value(TheValue::Int(*default_value));
                    slider.set_range(TheValue::RangeI32(range.clone()));
                    slider.set_continuous(*continous);
                    slider.set_status_text(status);
                    layout.add_pair(name.clone(), Box::new(slider));
                }
                Button(id, name, status, layout_text) => {
                    let mut button = TheTraybarButton::new(TheId::named(id));
                    button.set_text(name.clone());
                    button.set_status_text(status);
                    layout.add_pair(layout_text.clone(), Box::new(button));
                }
                ColorPicker(id, name, status, value, continuous) => {
                    let mut picker = TheColorPicker::new(TheId::named(id));
                    picker.set_value(TheValue::ColorObject(value.clone()));
                    picker.set_status_text(status);
                    picker.set_continuous(*continuous);
                    picker.limiter_mut().set_max_size(Vec2::new(200, 200));
                    layout.add_pair(name.clone(), Box::new(picker));
                }
                Checkbox(id, name, status, value) => {
                    let mut cb = TheCheckButton::new(TheId::named(id));
                    cb.set_value(TheValue::Bool(*value));
                    cb.set_status_text(status);
                    layout.add_pair(name.clone(), Box::new(cb));
                }
                Separator(name) => {
                    let sep = TheSeparator::new(TheId::named_with_id("Separator", Uuid::new_v4()));
                    layout.add_pair(name.clone(), Box::new(sep));
                }
                _ => {}
            }
        }
    }

    /// Handle an event and update the item values if necessary
    pub fn handle_event(&mut self, event: &TheEvent) -> bool {
        let mut updated = false;
        match event {
            TheEvent::ValueChanged(id, event_value) => {
                if let Some(item) = self.get_item_mut(&id.name) {
                    match item {
                        Text(_, _, _, value, _, _) => {
                            if let TheValue::Text(v) = event_value {
                                *value = v.clone();
                                updated = true;
                            }
                        }
                        Selector(_, _, _, _, value) => {
                            if let TheValue::Int(v) = event_value {
                                *value = *v;
                                updated = true;
                            }
                        }
                        FloatEditSlider(_, _, _, value, _, _) => {
                            if let Some(v) = event_value.to_f32() {
                                *value = v;
                                updated = true;
                            }
                        }
                        FloatSlider(_, _, _, value, _, _, _) => {
                            if let TheValue::Float(v) = event_value {
                                *value = *v;
                                updated = true;
                            }
                        }
                        IntEditSlider(_, _, _, value, _, _) => {
                            if let TheValue::Int(v) = event_value {
                                *value = *v;
                                updated = true;
                            } else if let TheValue::IntRange(v, _) = event_value {
                                *value = *v;
                                updated = true;
                            }
                        }
                        IntSlider(_, _, _, value, _, _, _) => {
                            if let TheValue::Int(v) = event_value {
                                *value = *v;
                                updated = true;
                            }
                        }
                        Checkbox(_, _, _, value) => {
                            if let TheValue::Bool(v) = event_value {
                                *value = *v;
                                updated = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        updated
    }
}
