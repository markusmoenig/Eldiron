use crate::prelude::*;

// Item

#[derive(Clone, Debug)]
pub struct TheContextMenuItem {
    pub name: String,
    pub id: TheId,
    pub value: Option<TheValue>,
    pub sub_menu: Option<TheContextMenu>,

    pub accel: Option<TheAccelerator>,
}

impl TheContextMenuItem {
    pub fn new(name: String, id: TheId) -> Self {
        Self {
            name,
            id,
            value: None,
            sub_menu: None,

            accel: None,
        }
    }

    pub fn new_with_accel(name: String, id: TheId, accel: TheAccelerator) -> Self {
        Self {
            name,
            id,
            value: None,
            sub_menu: None,

            accel: Some(accel),
        }
    }

    pub fn new_submenu(name: String, id: TheId, sub_menu: TheContextMenu) -> Self {
        Self {
            name,
            id,
            value: None,
            sub_menu: Some(sub_menu),

            accel: None,
        }
    }

    /// Sets the sub menu.
    pub fn set_sub_menu(&mut self, menu: TheContextMenu) {
        self.sub_menu = Some(menu);
    }
}

// Menu

#[derive(Clone, Debug)]
pub struct TheContextMenu {
    pub name: String,
    pub id: TheId,
    pub items: Vec<TheContextMenuItem>,
    pub width: i32,
    pub item_height: i32,

    pub dim: TheDim,

    pub hovered: Option<TheId>,
    pub is_open: bool,
    pub cascading_y_offset: i32,
}

impl Default for TheContextMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl TheContextMenu {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            id: TheId::empty(),

            items: vec![],
            width: 200,
            item_height: 21,

            dim: TheDim::zero(),

            hovered: None,
            is_open: false,
            cascading_y_offset: 0,
        }
    }

    pub fn named(name: String) -> Self {
        Self {
            name,
            id: TheId::empty(),

            items: vec![],
            width: 200,
            item_height: 23,

            dim: TheDim::zero(),

            hovered: None,
            is_open: false,
            cascading_y_offset: 0,
        }
    }

    /// Add an item.
    pub fn add(&mut self, item: TheContextMenuItem) {
        self.items.push(item);
    }

    /// Add a separator.
    pub fn add_separator(&mut self) {
        self.items
            .push(TheContextMenuItem::new("".to_string(), TheId::empty()));
    }

    /// Sets the position of the context menu while making it sure it fits on the screen.
    pub fn set_position(&mut self, position: Vec2<i32>, ctx: &mut TheContext) {
        let mut height = 2 * 8; // Borders
        for item in self.items.iter() {
            if item.name.is_empty() {
                height += self.item_height / 2;
            } else {
                height += self.item_height;
            }
        }

        let mut x = position.x;
        let mut y = position.y;

        // Make sure the menu fits horizontally on screen
        if x + self.width > ctx.width as i32 {
            x = ctx.width as i32 - self.width;
        }

        // Make sure the menu fits vertically on screen
        if y + height > ctx.height as i32 {
            y = ctx.height as i32 - height;
        }

        self.dim = TheDim::new(x, y, self.width, height);
        self.dim.buffer_x = x;
        self.dim.buffer_y = y;
    }

    pub fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::MouseDown(_coord) => {
                if self.hovered.is_some() {
                    redraw = true;
                }
            }
            TheEvent::Hover(coord) => {
                for item in self.items.iter_mut() {
                    if let Some(sub_menu) = item.sub_menu.as_mut() {
                        if sub_menu.is_open {
                            let local = Vec2::new(
                                coord.x - sub_menu.dim.width,
                                coord.y - sub_menu.cascading_y_offset,
                            );
                            redraw = sub_menu.on_event(&TheEvent::Hover(local), ctx);
                        }
                    }
                }
                if coord.x >= 0 && coord.x < self.dim.width {
                    let mut y = 7; // initial y offset inside the menu
                    self.hovered = None;

                    for item in &self.items {
                        let item_height = if item.name.is_empty() {
                            self.item_height / 2
                        } else {
                            self.item_height
                        };

                        if coord.y >= y && coord.y < y + item_height {
                            if !ctx.ui.is_disabled(&item.id.name) && !item.name.is_empty() {
                                self.hovered = Some(item.id.clone());
                            }
                            break;
                        }

                        y += item_height;
                    }

                    redraw = true;
                }
            }
            _ => {}
        }

        redraw
    }

    /// Returns true if the context menu (or its sub_menus) contains the given coordinate.
    pub fn contains(&mut self, coord: Vec2<i32>) -> bool {
        if self.dim.contains(coord) {
            return true;
        }
        for item in self.items.iter_mut() {
            if let Some(sub_menu) = item.sub_menu.as_mut() {
                if sub_menu.is_open && sub_menu.contains(coord) {
                    return true;
                }
            }
        }
        false
    }

    /// Recursively returns the currently hovered menu id / item id.
    pub fn get_hovered_id(&mut self) -> Option<(TheId, TheId)> {
        for item in self.items.iter_mut() {
            if let Some(sub_menu) = item.sub_menu.as_mut() {
                if sub_menu.is_open {
                    if let Some(rc) = sub_menu.get_hovered_id() {
                        return Some(rc);
                    }
                }
            }
        }
        if let Some(hovered) = &self.hovered {
            return Some((self.id.clone(), hovered.clone()));
        }
        None
    }

    /// Register the accelerators to the system.
    pub fn register_accel(&self, ctx: &mut TheContext) {
        for item in &self.items {
            if let Some(accel) = item.accel {
                ctx.ui.accelerators.insert(item.id.clone(), accel);
            }
        }
    }

    /// Draw the menu
    pub fn draw(&mut self, pixels: &mut [u8], style: &mut Box<dyn TheStyle>, ctx: &mut TheContext) {
        let mut tuple = self.dim.to_buffer_utuple();
        let mut shrinker = TheDimShrinker::zero();

        ctx.draw.rect_outline(
            pixels,
            &tuple,
            ctx.width,
            style.theme().color(ContextMenuBorder),
        );

        shrinker.shrink(1);
        tuple = self.dim.to_buffer_shrunk_utuple(&shrinker);

        ctx.draw.rect(
            pixels,
            &tuple,
            ctx.width,
            style.theme().color(ContextMenuBackground),
        );

        let mut y = tuple.1 + 7;
        for item in self.items.iter_mut() {
            let is_disabled = ctx.ui.is_disabled(&item.id.name);

            let rect = (
                tuple.0,
                y,
                self.width as usize - 2,
                if item.name.is_empty() {
                    self.item_height as usize / 2
                } else {
                    self.item_height as usize
                },
            );

            let mut text_color = if is_disabled {
                style.theme().color(ContextMenuTextDisabled)
            } else {
                style.theme().color(ContextMenuTextNormal)
            };

            if Some(item.id.clone()) == self.hovered && !item.name.is_empty() && !is_disabled {
                ctx.draw.rect(
                    pixels,
                    &rect,
                    ctx.width,
                    style.theme().color(ContextMenuHighlight),
                );
                text_color = style.theme().color(ContextMenuTextHighlight);
            }

            if item.name.is_empty() {
                ctx.draw.rect(
                    pixels,
                    &(rect.0, rect.1 + rect.3 / 2, rect.2, 1),
                    ctx.width,
                    style.theme().color(ContextMenuSeparator),
                );
            } else {
                ctx.draw.text_rect_blend(
                    pixels,
                    &(rect.0 + 16, rect.1, &rect.2 - 16, rect.3),
                    ctx.width,
                    &item.name,
                    TheFontSettings {
                        size: 13.5,
                        ..Default::default()
                    },
                    text_color,
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }

            if let Some(accel) = &item.accel {
                ctx.draw.text_rect_blend(
                    pixels,
                    &(rect.0, rect.1, &rect.2 - 6, rect.3),
                    ctx.width,
                    &accel.description(),
                    TheFontSettings {
                        size: 12.0,
                        ..Default::default()
                    },
                    style.theme().color(ContextMenuTextDisabled),
                    TheHorizontalAlign::Right,
                    TheVerticalAlign::Center,
                );
            } else if let Some(sub_menu) = &mut item.sub_menu {
                if !is_disabled {
                    if Some(item.id.clone()) == self.hovered {
                        sub_menu
                            .set_position(Vec2::new((rect.0 + rect.2) as i32, rect.1 as i32), ctx);
                        sub_menu.draw(pixels, style, ctx);
                        sub_menu.is_open = true;
                        sub_menu.cascading_y_offset = y as i32 - tuple.1 as i32;
                    } else {
                        sub_menu.is_open = false;
                        sub_menu.cascading_y_offset = 0;
                    }
                }

                let mut alpha = if is_disabled { 0.3 } else { 0.8 };
                let mut icon_name = "menu_sub";
                if Some(item.id.clone()) == self.hovered {
                    icon_name = "menu_sub_highlight";
                    alpha = 0.5;
                }

                if let Some(icon) = ctx.ui.icon(icon_name) {
                    let r = (
                        rect.0 + rect.2 - 25,
                        rect.1 + 4,
                        icon.dim().width as usize,
                        icon.dim().height as usize,
                    );
                    ctx.draw
                        .blend_slice_alpha(pixels, icon.pixels(), &r, ctx.width, alpha);
                }
            }

            y += rect.3;
        }
    }
}
