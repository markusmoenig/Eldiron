//! Catalogue state and logical cards use shared rules/geometry, never a bitmap.
use super::*;
use actor_panels::{actor_frame, clipped};
use screen_model::CatalogPanelKind;

pub(super) struct OpenCatalog {
    pub panel: OpenActorPanel,
    pub kind: CatalogPanelKind,
    pub page: usize,
    pub tab: String,
    pub selected: Option<String>,
    pub assignment: bool,
    pub detail_scroll: f32,
}
pub(super) struct CatalogCache {
    rules: String,
    class: Option<String>,
    groups: Arc<Vec<rules_ui::ActionCatalogGroup>>,
}
pub(in crate::game_screen::hud) fn configuration(
    elements: &ScreenElements,
    kind: CatalogPanelKind,
) -> &screen_model::CatalogPanelConfig {
    match kind {
        CatalogPanelKind::Actions => elements
            .custom_actions_panel_config
            .as_ref()
            .unwrap_or(&elements.toolbar_actions_panel_config),
        CatalogPanelKind::Spellbook => elements
            .custom_spellbook_config
            .as_ref()
            .unwrap_or(&elements.toolbar_spellbook_config),
    }
}
impl HudState {
    pub fn finish_assignment(&mut self, id: HudPanelId) {
        if let Some(open) = self.catalog_panel_mut(id) {
            open.assignment = false;
        }
    }

    pub(super) fn catalog_panel_mut(&mut self, id: HudPanelId) -> Option<&mut OpenCatalog> {
        let HudPanelId::Catalog {
            kind, incarnation, ..
        } = id
        else {
            return None;
        };
        self.catalog
            .as_mut()
            .filter(|open| open.kind == kind && open.panel.incarnation == incarnation)
    }
    pub(super) fn toggle_catalog(&mut self, command: &str) -> bool {
        let kind = match command.trim().to_ascii_lowercase().as_str() {
            "actions" | "action_catalog" | "abilities" => CatalogPanelKind::Actions,
            "spellbook" | "spells" => CatalogPanelKind::Spellbook,
            _ => return false,
        };
        let was_open = self.catalog.as_ref().is_some_and(|open| open.kind == kind);
        self.close_all();
        if !was_open {
            self.serial = self
                .serial
                .checked_add(1)
                .expect("HUD incarnation exhausted");
            self.catalog = Some(OpenCatalog {
                panel: OpenActorPanel {
                    incarnation: self.serial,
                    position: None,
                    scroll: [0.; 2],
                },
                kind,
                page: 0,
                tab: "all".into(),
                selected: None,
                assignment: false,
                detail_scroll: 0.,
            });
        }
        true
    }
    pub(super) fn activate_catalog(&mut self, action: &HudAction) -> bool {
        let Some(open) = &mut self.catalog else {
            return false;
        };
        match action {
            HudAction::CatalogTab(tab) => {
                open.tab.clone_from(tab);
                open.page = 0;
                open.selected = None;
            }
            HudAction::CatalogPage(page) => {
                open.page = *page;
                open.selected = None;
            }
            HudAction::CatalogAssign => {
                open.assignment = !open.assignment;
                return true;
            }
            _ => return false,
        }
        open.panel.scroll = [0.; 2];
        open.detail_scroll = 0.;
        true
    }
    pub fn select_catalog(&mut self, id: HudPanelId, command: &str) {
        if let Some(open) = self.catalog_panel_mut(id) {
            if open.selected.as_deref() != Some(command) {
                open.detail_scroll = 0.;
            }
            open.selected = Some(command.into());
        }
    }
    pub fn scroll_catalog_detail(&mut self, id: HudPanelId, delta: f32, max: f32) {
        if delta.is_finite() && max.is_finite() {
            if let Some(open) = self.catalog_panel_mut(id) {
                open.detail_scroll = (open.detail_scroll - delta).clamp(0., max.max(0.));
            }
        }
    }
    pub(super) fn catalog_frame(
        &self,
        world: &RuntimeView<'_>,
        elements: &ScreenElements,
    ) -> Option<PanelFrame> {
        let open = self.catalog.as_ref()?;
        let actor = world
            .map
            .and_then(|map| screen_model::resolve_party_entity(map, None));
        let class = actor.and_then(|actor| {
            actor
                .get_attr_string("class")
                .or_else(|| actor.get_attr_string("class_name"))
        });
        let groups = {
            let mut cache = self.catalog_cache.borrow_mut();
            if cache
                .as_ref()
                .is_none_or(|old| old.rules != world.assets.rules || old.class != class)
            {
                *cache = Some(CatalogCache {
                    rules: world.assets.rules.clone(),
                    class: class.clone(),
                    groups: Arc::new(rules_ui::action_catalog(world.assets, actor)),
                });
            }
            cache.as_ref()?.groups.clone()
        };
        let c = configuration(elements, open.kind);
        let size = elements.layout.viewport;
        let geometry = screen_model::catalog_panel_geometry(
            &groups,
            c,
            &open.tab,
            open.page,
            open.panel.position,
            size,
        );
        let title = if c.title.trim().is_empty() {
            open.kind.title().to_string()
        } else {
            c.title.clone()
        };
        let mut frame = actor_frame(
            HudPanelId::Catalog {
                kind: open.kind,
                entity: actor.map(|actor| actor.id),
                incarnation: open.panel.incarnation,
            },
            geometry.rect,
            c.title_height,
            c.padding,
            title,
            PanelStyle::catalog(c),
            size,
        );
        frame.catalog_skin = Some(open.kind);
        let padding = c
            .padding
            .min(frame.rect.width / 2.)
            .min(frame.rect.height / 2.);
        let title_height = c.title_height.min(frame.rect.height);
        let tabs_height = if c.show_tabs {
            c.tab_height.min((frame.rect.height - title_height).max(0.))
        } else {
            0.
        };
        let width = (frame.rect.width - padding * 2.).max(0.);
        let detail_width = if c.show_details {
            c.detail_width.min(width * 0.45)
        } else {
            0.
        };
        let detail_gap = if c.show_details {
            c.detail_gap.min((width - detail_width).max(0.))
        } else {
            0.
        };
        let y = frame.rect.y + title_height + tabs_height + padding;
        let height = (frame.rect.height - title_height - tabs_height - padding * 2.).max(0.);
        frame.clip = Rect::new(
            frame.rect.x + padding,
            y,
            (width - detail_width - detail_gap).max(0.),
            height,
        );
        let content_right = geometry
            .entries
            .iter()
            .map(|entry| entry.rect.x + entry.rect.width)
            .fold(frame.clip.x + frame.clip.width, f32::max);
        let content_bottom = geometry
            .entries
            .iter()
            .map(|entry| entry.rect.y + entry.rect.height)
            .chain(
                geometry
                    .groups
                    .iter()
                    .map(|group| group.title_rect.y + group.title_rect.height),
            )
            .fold(y + height, f32::max);
        frame.max_scroll = [
            (content_right as f64 - frame.clip.x as f64 - frame.clip.width as f64).max(0.),
            (content_bottom as f64 - y as f64 - height as f64).max(0.),
        ];
        frame.scroll = [
            open.panel.scroll[0].clamp(0., frame.max_scroll[0]),
            open.panel.scroll[1].clamp(0., frame.max_scroll[1]),
        ];
        let shift = |mut bounds: Rect| {
            bounds.x -= frame.scroll[0] as f32;
            bounds.y -= frame.scroll[1] as f32;
            bounds
        };
        let selected = open
            .selected
            .as_deref()
            .filter(|command| {
                geometry
                    .entries
                    .iter()
                    .any(|entry| entry.command == *command)
            })
            .or_else(|| geometry.entries.first().map(|entry| entry.command.as_str()));
        for group in &geometry.groups {
            frame.labels.push(PanelLabel {
                text: group.name.clone(),
                rect: shift(group.title_rect),
                size: c.small_font_size,
                color: c.muted_text_color,
                centered: false,
                content: true,
            });
        }
        for (index, entry) in geometry.entries.iter().enumerate() {
            let bounds = shift(entry.rect);
            let Some(hit) = clipped(bounds, frame.clip) else {
                continue;
            };
            frame.cards.push(CommandCard {
                index,
                rect: bounds,
                hit,
                icon_rect: shift(entry.icon_rect),
                command: entry.command.clone(),
                name: entry.name.clone(),
                state: rules_ui::command_state(world.assets, actor, &entry.command),
                selected: selected == Some(entry.command.as_str()),
                action: HudAction::CatalogCommand {
                    command: entry.command.clone(),
                    assignment: open.assignment,
                    class: class.clone(),
                },
            });
            if c.show_names {
                frame.labels.push(PanelLabel {
                    text: entry.name.clone(),
                    rect: Rect::new(bounds.x, bounds.y + bounds.height - 18., bounds.width, 18.),
                    size: c.small_font_size,
                    color: c.text_color,
                    centered: true,
                    content: true,
                });
            }
        }
        if geometry.entries.is_empty() {
            frame.labels.push(PanelLabel {
                text: format!(
                    "No {} available for this character.",
                    open.kind.title().to_ascii_lowercase()
                ),
                rect: frame.clip,
                size: c.font_size,
                color: c.muted_text_color,
                centered: true,
                content: true,
            });
        }
        if c.show_details {
            let bounds = Rect::new(
                frame.rect.x + frame.rect.width - padding - detail_width,
                y,
                detail_width,
                height,
            );
            let description = selected
                .map(|command| rules_ui::describe_command(world.assets, actor, command))
                .unwrap_or_default();
            let state = selected
                .map(|command| rules_ui::command_state(world.assets, actor, command))
                .unwrap_or_default();
            frame.detail = Some(DetailSpec {
                rect: bounds,
                rows: screen_model::catalog_detail_rows(description.clone(), &state),
                description,
                scroll: open.detail_scroll,
                font_size: c.font_size,
                small_size: c.small_font_size,
                background: c.detail_background_color,
            });
        }
        let tabs = geometry.tabs.len().max(1);
        for (index, tab) in geometry.tabs.iter().enumerate() {
            let tab_width = (width / tabs as f32).min(104.);
            frame.button(
                HudPart::CatalogTab(index),
                Rect::new(
                    frame.rect.x + padding + index as f32 * tab_width,
                    frame.rect.y + title_height,
                    tab_width,
                    tabs_height,
                ),
                HudAction::CatalogTab(tab.id.clone()),
                tab.name.clone(),
                if tab.id.eq_ignore_ascii_case(&geometry.tab) {
                    c.tab_selected_color
                } else {
                    c.tab_background_color
                },
                tab.id.eq_ignore_ascii_case(&geometry.tab),
                false,
            );
        }
        let right = frame
            .close
            .map(|rect| rect.x)
            .unwrap_or(frame.rect.x + frame.rect.width)
            - 4.;
        let assign_width = if c.show_assign {
            66f32.min(width * 0.25)
        } else {
            0.
        };
        if c.show_assign {
            frame.button(
                HudPart::Assign,
                Rect::new(
                    right - assign_width,
                    frame.rect.y + 5.,
                    assign_width,
                    20f32.min(title_height),
                ),
                HudAction::CatalogAssign,
                "Assign".into(),
                c.tab_background_color,
                open.assignment,
                false,
            );
        }
        if geometry.page_count > 1 {
            let next_x = right - assign_width - 24.;
            let previous_x = next_x - 54.;
            if geometry.page > 0 {
                frame.button(
                    HudPart::PreviousPage,
                    Rect::new(previous_x, frame.rect.y + 5., 20., 20.),
                    HudAction::CatalogPage(geometry.page - 1),
                    "‹".into(),
                    c.tab_background_color,
                    false,
                    false,
                );
            }
            if geometry.page + 1 < geometry.page_count {
                frame.button(
                    HudPart::NextPage,
                    Rect::new(next_x, frame.rect.y + 5., 20., 20.),
                    HudAction::CatalogPage(geometry.page + 1),
                    "›".into(),
                    c.tab_background_color,
                    false,
                    false,
                );
            }
            frame.labels.push(PanelLabel {
                text: format!("{}/{}", geometry.page + 1, geometry.page_count),
                rect: Rect::new(previous_x + 20., frame.rect.y + 5., 34., 20.),
                size: c.small_font_size,
                color: c.text_color,
                centered: true,
                content: false,
            });
        }
        if let Some(title) = &mut frame.title_rect {
            title.width =
                (right - assign_width - if geometry.page_count > 1 { 82. } else { 0. } - title.x)
                    .max(0.);
        }
        Some(frame)
    }
}
