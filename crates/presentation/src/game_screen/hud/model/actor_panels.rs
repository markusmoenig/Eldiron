//! Family controllers project shared geometry into one retained panel contract.
use super::*;
impl HudState {
    pub fn select_item(&mut self, id: HudControlId, item: u32) {
        if matches!(id.panel, HudPanelId::Inventory { .. })
            && self.actor_panel_mut(id.panel).is_some()
        {
            self.selected = Some(item);
        }
    }

    pub(super) fn actor_panel_mut(&mut self, id: HudPanelId) -> Option<&mut OpenActorPanel> {
        let (open, incarnation) = match id {
            HudPanelId::Inventory { incarnation, .. } => (&mut self.inventory, incarnation),
            HudPanelId::Equipment { incarnation, .. } => (&mut self.equipment, incarnation),
            _ => return None,
        };
        open.as_mut().filter(|open| open.incarnation == incarnation)
    }
    pub fn close_all(&mut self) {
        self.close();
        self.inventory = None;
        self.equipment = None;
        self.sort_open = false;
        self.page = 0;
        self.selected = None;
    }
    pub fn close_panel(&mut self, id: HudPanelId) {
        match id {
            HudPanelId::Container { .. } => self.close(),
            HudPanelId::Inventory { .. } => {
                self.inventory = None;
                self.sort_open = false;
                self.page = 0;
                self.selected = None;
            }
            HudPanelId::Equipment { .. } => self.equipment = None,
        }
    }
    pub fn toggle(&mut self, command: &str) -> bool {
        let inventory = match command.trim().to_ascii_lowercase().as_str() {
            "inventory" | "items" | "bag" => true,
            "equipment" | "gear" | "character" => false,
            _ => return false,
        };
        let was_open = if inventory {
            self.inventory.is_some()
        } else {
            self.equipment.is_some()
        };
        self.close_all();
        if !was_open {
            self.serial = self
                .serial
                .checked_add(1)
                .expect("HUD incarnation exhausted");
            let open = Some(OpenActorPanel {
                incarnation: self.serial,
                position: None,
                scroll: [0.; 2],
            });
            if inventory {
                self.inventory = open;
            } else {
                self.equipment = open;
            }
        }
        true
    }
    pub fn popup_open(&self) -> bool {
        self.inventory.is_some() && self.sort_open
    }
    pub fn dismiss_popup(&mut self) {
        self.sort_open = false;
    }
    pub fn activate(&mut self, action: &HudAction, elements: &mut ScreenElements) {
        if self.inventory.is_none() {
            return;
        }
        match action {
            HudAction::Category(value) => {
                elements.set_binding("inventory.category", value);
                self.page = 0;
                self.selected = None;
            }
            HudAction::Sort(value) => {
                elements.set_binding("inventory.sort", value);
                self.sort_open = false;
                self.page = 0;
            }
            HudAction::ToggleSort => self.sort_open = !self.sort_open,
            HudAction::Page(page) => self.page = *page,
        }
        if let Some(open) = &mut self.inventory {
            open.scroll = [0.; 2];
        }
    }
    pub fn frames(&self, world: &RuntimeView<'_>, elements: &ScreenElements) -> Vec<PanelFrame> {
        let size = elements.layout.viewport;
        let mut frames: Vec<_> = self.layout(world, size).into_iter().collect();
        let Some(map) = world.map else {
            return frames;
        };
        if size.x < 8
            || size.y < 8
            || world.state == ServerState::Off
            || self.scope != Some((world.session_id, map.id))
        {
            return frames;
        }
        let Some(actor) = screen_model::resolve_party_entity(map, None) else {
            return frames;
        };
        if let Some(open) = &self.inventory {
            let config = elements
                .custom_inventory_panel_config
                .as_ref()
                .unwrap_or(&elements.toolbar_inventory_panel_config);
            frames.push(self.inventory_frame(actor, config, elements, open, size));
        }
        if let Some(open) = &self.equipment {
            let config = elements
                .custom_equipment_panel_config
                .as_ref()
                .unwrap_or(&elements.toolbar_equipment_panel_config);
            frames.push(self.equipment_frame(actor, config, world, open, size));
        }
        frames
    }
    fn inventory_frame(
        &self,
        actor: &rusterix::Entity,
        c: &screen_model::InventoryPanelConfig,
        elements: &ScreenElements,
        open: &OpenActorPanel,
        size: Vec2<i32>,
    ) -> PanelFrame {
        let category = elements
            .ui_state
            .get("inventory.category")
            .map(String::as_str)
            .unwrap_or("all");
        let sort = elements
            .ui_state
            .get("inventory.sort")
            .map(String::as_str)
            .unwrap_or("newest");
        let geometry = screen_model::inventory_panel_geometry(
            actor,
            c,
            category,
            sort,
            self.page,
            open.position,
            size,
        );
        let mut frame = actor_frame(
            HudPanelId::Inventory {
                entity: actor.id,
                incarnation: open.incarnation,
            },
            geometry.rect,
            c.title_height,
            c.padding,
            c.title.clone(),
            PanelStyle::inventory(c),
            size,
        );
        let padding = c
            .padding
            .min(frame.rect.width / 2.)
            .min(frame.rect.height / 2.);
        let title_height = c.title_height.min(frame.rect.height);
        let tab_height = c.tab_height.min((frame.rect.height - title_height).max(0.));
        let footer_height = c
            .footer_height
            .min((frame.rect.height - title_height - tab_height).max(0.));
        let footer = Rect::new(
            frame.rect.x + padding,
            frame.rect.y + frame.rect.height - padding - footer_height,
            (frame.rect.width - padding * 2.).max(0.),
            footer_height,
        );
        frame.clip = Rect::new(
            frame.rect.x + padding,
            frame.rect.y + title_height + tab_height + padding,
            footer.width,
            (footer.y - frame.rect.y - title_height - tab_height - padding * 2.).max(0.),
        );
        frame.max_scroll = [
            (geometry.grid_rect.width as f64 - frame.clip.width as f64).max(0.),
            (geometry.grid_rect.height as f64 - frame.clip.height as f64).max(0.),
        ];
        frame.scroll = [
            open.scroll[0].clamp(0., frame.max_scroll[0]),
            open.scroll[1].clamp(0., frame.max_scroll[1]),
        ];
        let offset = [
            frame.scroll[0]
                + if geometry.rect.width > frame.rect.width {
                    geometry.grid_rect.x as f64 - frame.clip.x as f64
                } else {
                    0.
                },
            frame.scroll[1] + geometry.grid_rect.y as f64 - frame.clip.y as f64,
        ];
        frame.body_destination = Some(ItemDestination::Actor {
            entity: Some(actor.id),
            inventory: None,
            equipped: None,
        });
        frame.slots = geometry
            .visible_slots(frame.clip, offset)
            .filter_map(|(index, slot)| {
                let hit = clipped(slot.rect, frame.clip)?;
                Some(PanelSlot {
                    index,
                    rect: slot.rect,
                    hit,
                    source: slot.item_id.map(|item| {
                        ItemSource::Direct(ItemRef {
                            item,
                            owner: Some(actor.id),
                        })
                    }),
                    destination: slot
                        .inventory_index
                        .map(|inventory| ItemDestination::Actor {
                            entity: Some(actor.id),
                            inventory: Some(inventory),
                            equipped: None,
                        }),
                    inventory_index: slot.inventory_index,
                    equipped_slot: None,
                    selected: slot.item_id.is_some() && slot.item_id == self.selected,
                })
            })
            .collect();
        let tabs = Rect::new(
            frame.rect.x + padding,
            frame.rect.y + title_height,
            footer.width,
            tab_height,
        );
        for (index, option) in c.categories.iter().enumerate() {
            let bounds = rusterix::client::widget::choice::tab_option_rect(
                &c.categories,
                tabs,
                c.font_size,
                1.,
                8.,
                false,
                index,
            );
            frame.button(
                HudPart::Category(index),
                bounds,
                HudAction::Category(option.value.clone()),
                option.label.clone(),
                if option.value == category {
                    c.tab_selected_color
                } else {
                    c.tab_background_color
                },
                option.value == category,
                false,
            );
        }
        // Keep all footer controls available on constrained windows. The shared
        // reference geometry remains unchanged; native grids scroll within it.
        let sort_width = c.sort_width.min(footer.width * 0.32).max(0.);
        let sort_rect = Rect::new(footer.x, footer.y, sort_width, footer.height);
        let sort_label = c
            .sort_options
            .iter()
            .find(|option| option.value == sort)
            .or_else(|| c.sort_options.first())
            .map(|option| option.label.clone())
            .unwrap_or_default();
        frame.button(
            HudPart::Sort,
            sort_rect,
            HudAction::ToggleSort,
            format!("{sort_label} ▴"),
            c.dropdown_background_color,
            false,
            false,
        );
        if geometry.page_count > 1 {
            let middle_x = footer.x + sort_width;
            let middle_width = footer.width * 0.4;
            let button_width = 24f32.min(middle_width / 3.);
            if geometry.page > 0 {
                frame.button(
                    HudPart::PreviousPage,
                    Rect::new(middle_x, footer.y, button_width, footer.height),
                    HudAction::Page(geometry.page - 1),
                    "‹".into(),
                    c.tab_background_color,
                    false,
                    false,
                );
            }
            if geometry.page + 1 < geometry.page_count {
                frame.button(
                    HudPart::NextPage,
                    Rect::new(
                        middle_x + middle_width - button_width,
                        footer.y,
                        button_width,
                        footer.height,
                    ),
                    HudAction::Page(geometry.page + 1),
                    "›".into(),
                    c.tab_background_color,
                    false,
                    false,
                );
            }
            frame.labels.push(PanelLabel {
                text: format!("{}/{}", geometry.page + 1, geometry.page_count),
                rect: Rect::new(
                    middle_x + button_width,
                    footer.y,
                    (middle_width - button_width * 2.).max(0.),
                    footer.height,
                ),
                size: c.font_size,
                color: c.muted_text_color,
                centered: true,
                content: false,
            });
        }
        let occupied = actor.inventory.iter().filter(|slot| slot.is_some()).count();
        frame.labels.push(PanelLabel {
            text: format!("{occupied}/{}", actor.inventory.len()),
            rect: Rect::new(
                footer.x + footer.width * 0.73,
                footer.y,
                footer.width * 0.27,
                footer.height,
            ),
            size: c.font_size,
            color: c.muted_text_color,
            centered: true,
            content: false,
        });
        if self.sort_open {
            let row_height = c
                .footer_height
                .max(8.)
                .min(frame.rect.height / c.sort_options.len().max(1) as f32);
            let height = row_height * c.sort_options.len() as f32;
            let popup = Rect::new(
                sort_rect.x,
                (sort_rect.y - height).max(frame.rect.y),
                sort_rect.width.max(1.),
                height,
            );
            frame.popup = Some(popup);
            for (index, option) in c.sort_options.iter().enumerate() {
                frame.button(
                    HudPart::SortOption(index),
                    Rect::new(
                        popup.x,
                        popup.y + index as f32 * row_height,
                        popup.width,
                        row_height,
                    ),
                    HudAction::Sort(option.value.clone()),
                    option.label.clone(),
                    c.dropdown_panel_color,
                    option.value == sort,
                    true,
                );
            }
        }
        frame
    }
    fn equipment_frame(
        &self,
        actor: &rusterix::Entity,
        c: &screen_model::EquipmentPanelConfig,
        world: &RuntimeView<'_>,
        open: &OpenActorPanel,
        size: Vec2<i32>,
    ) -> PanelFrame {
        let geometry =
            screen_model::equipment_panel_geometry(actor, c, world.assets, open.position, size);
        let mut frame = actor_frame(
            HudPanelId::Equipment {
                entity: actor.id,
                incarnation: open.incarnation,
            },
            geometry.rect,
            c.title_height,
            c.padding,
            c.title.clone(),
            PanelStyle::equipment(c),
            size,
        );
        let padding = c
            .padding
            .min(frame.rect.width / 2.)
            .min(frame.rect.height / 2.);
        frame.clip = Rect::new(
            frame.rect.x + padding,
            frame.rect.y + c.title_height.min(frame.rect.height) + padding,
            (frame.rect.width - padding * 2.).max(0.),
            (frame.rect.height - c.title_height - padding * 2.).max(0.),
        );
        frame.max_scroll = [
            (geometry.rect.width as f64 - frame.rect.width as f64).max(0.),
            (geometry.rect.height as f64 - frame.rect.height as f64).max(0.),
        ];
        frame.scroll = [
            open.scroll[0].clamp(0., frame.max_scroll[0]),
            open.scroll[1].clamp(0., frame.max_scroll[1]),
        ];
        let shifted = |mut rect: Rect| {
            rect.x -= frame.scroll[0] as f32;
            rect.y -= frame.scroll[1] as f32;
            rect
        };
        frame.avatar = Some((actor.id, shifted(geometry.avatar_rect)));
        frame.body_destination = Some(ItemDestination::Actor {
            entity: Some(actor.id),
            inventory: None,
            equipped: None,
        });
        for (index, slot) in geometry.slots.iter().enumerate() {
            let bounds = shifted(slot.rect);
            if let Some(hit) = clipped(bounds, frame.clip) {
                frame.slots.push(PanelSlot {
                    index,
                    rect: bounds,
                    hit,
                    source: slot.item_id.map(|item| {
                        ItemSource::Direct(ItemRef {
                            item,
                            owner: Some(actor.id),
                        })
                    }),
                    destination: Some(ItemDestination::Actor {
                        entity: Some(actor.id),
                        inventory: None,
                        equipped: Some(slot.slot.clone()),
                    }),
                    inventory_index: None,
                    equipped_slot: Some(slot.slot.clone()),
                    selected: false,
                });
            }
            frame.labels.push(PanelLabel {
                text: screen_model::equipment_slot_label(&slot.slot),
                rect: shifted(slot.label_rect),
                size: c.font_size,
                color: c.text_color,
                centered: true,
                content: true,
            });
        }
        frame
    }
}
fn clipped(bounds: Rect, clip: Rect) -> Option<Rect> {
    let hit = rect(bounds).intersection(rect(clip));
    (hit.width > 0. && hit.height > 0.).then(|| Rect::new(hit.x, hit.y, hit.width, hit.height))
}
fn actor_frame(
    id: HudPanelId,
    raw: Rect,
    title_height: f32,
    padding: f32,
    title: String,
    style: PanelStyle,
    size: Vec2<i32>,
) -> PanelFrame {
    let bounds = Rect::new(
        raw.x,
        raw.y,
        raw.width.min((size.x - 4) as f32),
        raw.height.min((size.y - 4) as f32),
    );
    let title_height = title_height.min(bounds.height);
    let close_size = (title_height - 10.).max(0.).min(28.).min(bounds.width);
    PanelFrame {
        id,
        rect: bounds,
        title_bar: Some(Rect::new(bounds.x, bounds.y, bounds.width, title_height)),
        close: Some(Rect::new(
            bounds.x + bounds.width - close_size - 5.,
            bounds.y + 5.,
            close_size,
            close_size,
        )),
        title_rect: Some(Rect::new(
            bounds.x + padding,
            bounds.y,
            (bounds.width - padding * 2. - close_size - 5.).max(0.),
            title_height,
        )),
        title,
        clip: bounds,
        style,
        tiles: Vec::new(),
        slot_tile: None,
        labels: Vec::new(),
        buttons: Vec::new(),
        avatar: None,
        body_destination: None,
        popup: None,
        slots: Vec::new(),
        scroll: [0.; 2],
        max_scroll: [0.; 2],
        position: Vec2::new(bounds.x.round() as i32, bounds.y.round() as i32),
    }
}
impl PanelFrame {
    fn button(
        &mut self,
        part: HudPart,
        bounds: Rect,
        action: HudAction,
        text: String,
        background: [u8; 4],
        selected: bool,
        popup: bool,
    ) {
        self.buttons.push(PanelButton {
            part,
            rect: bounds,
            action,
            label: PanelLabel {
                text,
                rect: bounds,
                size: self.style.font_size,
                color: self.style.text,
                centered: true,
                content: false,
            },
            background,
            selected,
            popup,
        });
    }
}
