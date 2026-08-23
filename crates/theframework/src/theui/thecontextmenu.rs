use crate::prelude::*;

fn draw_submenu_marker(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    bounds: ThePixelRect,
    color: RGBA,
    painter: &mut ThePainter,
) {
    let Ok(mut surface) = TheSurfaceMut::new(pixels, width, height) else {
        return;
    };
    surface.set_clip(bounds);

    let center_x = bounds.x as f32 + bounds.width as f32 - 12.0;
    let center_y = bounds.y as f32 + bounds.height as f32 * 0.5;
    let mut path = ThePath::new();
    path.move_to((center_x - 2.5, center_y - 4.0))
        .line_to((center_x + 3.0, center_y))
        .line_to((center_x - 2.5, center_y + 4.0))
        .close();
    painter.fill_path(&mut surface, &path, &ThePaint::solid(color));
}

// Item

#[derive(Clone, Debug)]
pub struct TheContextMenuItem {
    pub name: String,
    pub id: TheId,
    pub value: Option<TheValue>,
    pub sub_menu: Option<TheContextMenu>,
    pub text_color: Option<RGBA>,

    pub accel: Option<TheAccelerator>,
}

impl TheContextMenuItem {
    pub fn new(name: String, id: TheId) -> Self {
        Self {
            name,
            id,
            value: None,
            sub_menu: None,
            text_color: None,

            accel: None,
        }
    }

    pub fn new_with_accel(name: String, id: TheId, accel: TheAccelerator) -> Self {
        Self {
            name,
            id,
            value: None,
            sub_menu: None,
            text_color: None,

            accel: Some(accel),
        }
    }

    pub fn new_submenu(name: String, id: TheId, sub_menu: TheContextMenu) -> Self {
        Self {
            name,
            id,
            value: None,
            sub_menu: Some(sub_menu),
            text_color: None,

            accel: None,
        }
    }

    /// Sets the sub menu.
    pub fn set_sub_menu(&mut self, menu: TheContextMenu) {
        self.sub_menu = Some(menu);
    }

    pub fn set_text_color(&mut self, color: RGBA) {
        self.text_color = Some(color);
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
            if let Some(sub_menu) = &item.sub_menu {
                sub_menu.register_accel(ctx);
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
                *style.theme().color(ContextMenuTextDisabled)
            } else {
                item.text_color
                    .unwrap_or(*style.theme().color(ContextMenuTextNormal))
            };

            if Some(item.id.clone()) == self.hovered && !item.name.is_empty() && !is_disabled {
                ctx.draw.rect(
                    pixels,
                    &rect,
                    ctx.width,
                    style.theme().color(ContextMenuHighlight),
                );
                text_color = *style.theme().color(ContextMenuTextHighlight);
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
                    &text_color,
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

                let is_hovered = Some(item.id.clone()) == self.hovered;
                let role = if is_disabled {
                    ContextMenuTextDisabled
                } else if is_hovered {
                    ContextMenuTextHighlight
                } else {
                    ContextMenuTextNormal
                };
                let mut marker_color = *style.theme().color(role);
                let alpha = if is_disabled {
                    0.3
                } else if is_hovered {
                    0.8
                } else {
                    0.8
                };
                marker_color[3] = (marker_color[3] as f32 * alpha).round() as u8;
                draw_submenu_marker(
                    pixels,
                    ctx.width,
                    ctx.height,
                    ThePixelRect::new(rect.0 as i32, rect.1 as i32, rect.2 as i32, rect.3 as i32),
                    marker_color,
                    &mut ctx.painter,
                );
            }

            y += rect.3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submenu_accelerators_are_registered_recursively() {
        let item_id = TheId::named("Nested Command");
        let mut submenu = TheContextMenu::named("Nested".to_string());
        submenu.add(TheContextMenuItem::new_with_accel(
            "Command".to_string(),
            item_id.clone(),
            TheAccelerator::new(TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT, 'f'),
        ));
        let mut menu = TheContextMenu::named("Root".to_string());
        menu.add(TheContextMenuItem::new_submenu(
            "Nested".to_string(),
            TheId::named("Nested"),
            submenu,
        ));
        let mut ctx = TheContext::new(100, 100, 1.0);

        menu.register_accel(&mut ctx);

        assert!(ctx.ui.accelerators.contains_key(&item_id));
    }

    #[test]
    fn procedural_submenu_marker_preserves_guard_bytes() {
        const WIDTH: usize = 8;
        const HEIGHT: usize = 7;
        const GUARD: usize = 31;
        const SENTINEL: u8 = 0xc5;

        let body_len = WIDTH * HEIGHT * 4;
        let mut pixels = vec![0; body_len + GUARD];
        pixels[body_len..].fill(SENTINEL);
        draw_submenu_marker(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ThePixelRect::new(-20, -4, 32, 23),
            [244, 247, 250, 255],
            &mut ThePainter::new(),
        );

        assert!(pixels[..body_len].iter().any(|byte| *byte != 0));
        assert!(pixels[body_len..].iter().all(|byte| *byte == SENTINEL));
    }
}
