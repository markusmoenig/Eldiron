//! The journal projects shared inspection into retained GPU HUD controls.
use super::actor_panels::actor_frame;
use super::*;
use screen_model::{CatalogPanelKind, JournalTab};
impl HudState {
    pub(in crate::game_screen::hud) fn journal_text(
        &mut self,
        text: Option<&str>,
        backspace: bool,
    ) -> bool {
        let Some(open) = self
            .catalog
            .as_mut()
            .filter(|open| open.kind == CatalogPanelKind::Journal)
        else {
            return false;
        };
        if backspace {
            open.search.pop();
        }
        if let Some(text) = text {
            for ch in text.chars().filter(|ch| !ch.is_control()) {
                if open.search.len() + ch.len_utf8() > 256 {
                    break;
                }
                open.search.push(ch);
            }
        }
        open.page = 0;
        open.selected = None;
        open.target = None;
        open.detail_scroll = 0.;
        open.assignment = false;
        open.journal_popup = None;
        true
    }
    pub(in crate::game_screen::hud) fn journal_effect(
        &self,
        world: &RuntimeView<'_>,
        command: &str,
        target: Option<screen_model::JournalTarget>,
    ) -> Option<ScreenEffect> {
        let actor = world.map.and_then(|map| world.party_entity(map, None))?;
        let action = command.strip_prefix("rules.")?;
        let mut model = self.journal_model.borrow_mut();
        model
            .bind(
                world.assets,
                actor,
                world.map?,
                world.state != ServerState::Running,
            )
            .ok()?;
        if !model.inspect(action, target).ready {
            return None;
        }
        let intent = Some(format!("action:{action}"));
        Some(ScreenEffect::Action(if let Some(target) = target {
            rusterix::EntityAction::ItemClicked(target.item, 0., intent, Some(actor.id))
        } else {
            rusterix::EntityAction::EntityClicked(actor.id, 0., intent)
        }))
    }
    pub(super) fn journal_frame(
        &self,
        world: &RuntimeView<'_>,
        elements: &ScreenElements,
    ) -> Option<PanelFrame> {
        let open = self.catalog.as_ref()?;
        let actor = world.map.and_then(|map| world.party_entity(map, None))?;
        let mut model = self.journal_model.borrow_mut();
        let bound = model.bind(
            world.assets,
            actor,
            world.map?,
            world.state != ServerState::Running,
        );
        let entries = bound
            .and_then(|_| model.entries(world.assets, open.journal_tab, &open.search, &open.craft));
        let error = entries.as_ref().err().cloned();
        let entries = entries.unwrap_or_default();
        let geometry = screen_model::journal_geometry(
            elements.layout.viewport,
            open.panel.position,
            entries.len(),
            open.page,
        );
        let c = catalog::configuration(elements, CatalogPanelKind::Journal);
        let mut frame = actor_frame(
            HudPanelId::Catalog {
                kind: CatalogPanelKind::Journal,
                entity: Some(actor.id),
                incarnation: open.panel.incarnation,
            },
            geometry.rect,
            28.,
            8.,
            CatalogPanelKind::Journal.title().into(),
            PanelStyle::catalog(c),
            elements.layout.viewport,
        );
        frame.clip = Rect::new(
            geometry.rect.x + 8.,
            geometry.rect.y + 94.,
            (geometry.detail.x - geometry.rect.x - 16.).max(0.),
            (geometry.rect.height - 132.).max(0.),
        );
        let selected = open
            .selected
            .as_deref()
            .and_then(|key| entries.iter().find(|entry| entry.key == key))
            .or_else(|| entries.get(geometry.start));
        let class = actor
            .get_attr_string("class")
            .or_else(|| actor.get_attr_string("class_name"));
        for (row, entry) in geometry
            .rows
            .iter()
            .zip(entries.iter().skip(geometry.start))
        {
            let index = frame.cards.len();
            frame.cards.push(CommandCard {
                index,
                rect: *row,
                hit: *row,
                icon_rect: Rect::new(row.x + 3., row.y + 3., 30., 30.),
                command: entry.key.clone(),
                name: entry.name.clone(),
                state: rules_ui::CommandState::default(),
                selected: selected.is_some_and(|selected| selected.key == entry.key),
                action: HudAction::CatalogCommand {
                    command: entry.key.clone(),
                    assignment: open.assignment && entry.action.is_some(),
                    class: class.clone(),
                },
            });
            frame.labels.push(PanelLabel {
                text: entry.name.clone(),
                rect: Rect::new(row.x + 40., row.y + 1., (row.width - 44.).max(0.), 20.),
                size: 12.,
                color: c.text_color,
                centered: false,
                content: true,
            });
            frame.labels.push(PanelLabel {
                text: model.craft_name(&entry.craft),
                rect: Rect::new(row.x + 40., row.y + 19., (row.width - 44.).max(0.), 15.),
                size: 10.,
                color: c.muted_text_color,
                centered: false,
                content: true,
            });
        }
        for (index, (tab, name)) in [
            (JournalTab::Recipes, "Recipes"),
            (JournalTab::Knowledge, "Knowledge"),
        ]
        .into_iter()
        .enumerate()
        {
            frame.button(
                HudPart::JournalTab(index),
                geometry.tabs[index],
                HudAction::JournalTab(tab),
                name.into(),
                if open.journal_tab == tab {
                    c.tab_selected_color
                } else {
                    c.tab_background_color
                },
                open.journal_tab == tab,
                false,
            );
        }
        frame.button(
            HudPart::JournalSearch,
            Rect::new(
                geometry.search.x,
                geometry.search.y,
                (geometry.search.width - 30.).max(0.),
                geometry.search.height,
            ),
            HudAction::JournalSearch,
            if open.search.is_empty() {
                "Search…".into()
            } else {
                format!("Search: {}", open.search)
            },
            c.detail_background_color,
            false,
            false,
        );
        frame.button(
            HudPart::JournalClear,
            Rect::new(
                geometry.search.x + (geometry.search.width - 28.).max(0.),
                geometry.search.y,
                28f32.min(geometry.search.width),
                geometry.search.height,
            ),
            HudAction::JournalClear,
            "×".into(),
            c.detail_background_color,
            false,
            false,
        );
        let crafts = if open.journal_tab == JournalTab::Knowledge {
            vec!["all".into(), "permanent".into(), "temporary".into()]
        } else {
            model.crafts()
        };

        frame.button(
            HudPart::JournalFilter,
            geometry.filter,
            HudAction::JournalPopup { targets: false },
            format!(
                "{}: {} ▸",
                if open.journal_tab == JournalTab::Recipes {
                    "Craft"
                } else {
                    "Show"
                },
                model.craft_name(&open.craft)
            ),
            c.tab_background_color,
            false,
            false,
        );
        if geometry.page > 0 {
            frame.button(
                HudPart::PreviousPage,
                geometry.previous,
                HudAction::CatalogPage(geometry.page - 1),
                "<".into(),
                c.tab_background_color,
                false,
                false,
            );
        }
        if geometry.page + 1 < geometry.page_count {
            frame.button(
                HudPart::NextPage,
                geometry.next,
                HudAction::CatalogPage(geometry.page + 1),
                ">".into(),
                c.tab_background_color,
                false,
                false,
            );
        }
        frame.labels.push(PanelLabel {
            text: format!(
                "{} {} · {}/{}",
                entries.len(),
                match (open.journal_tab, entries.len() == 1) {
                    (JournalTab::Recipes, true) => "recipe",
                    (JournalTab::Recipes, false) => "recipes",
                    (JournalTab::Knowledge, true) => "pattern",
                    (JournalTab::Knowledge, false) => "patterns",
                },
                geometry.page + 1,
                geometry.page_count
            ),
            rect: Rect::new(
                geometry.previous.x + 34.,
                geometry.previous.y,
                (geometry.next.x - geometry.previous.x - 38.).max(0.),
                22.,
            ),
            size: 10.,
            color: c.muted_text_color,
            centered: true,
            content: false,
        });
        let mut description = rules_ui::RulesDescription::default();
        if let Some(entry) = selected {
            if let Some(action) = &entry.action {
                description = rules_ui::describe_command(world.assets, Some(actor), &entry.key);
                let report = model.inspect(action, open.target);
                let mut inspection = vec![if report.ready {
                    "Ready to craft".into()
                } else {
                    format!(
                        "Unavailable: {}",
                        report
                            .reason
                            .as_deref()
                            .unwrap_or("Requirements are not met")
                    )
                }];
                for ingredient in &report.ingredients {
                    inspection.push(format!(
                        "{}: {} / {}{}",
                        ingredient.name,
                        ingredient.available,
                        ingredient.required,
                        if ingredient.liquid {
                            " units in owned vessels"
                        } else {
                            " available"
                        }
                    ));
                }
                inspection.extend(report.requirements.clone());
                inspection.extend(report.results.clone());
                inspection.extend(description.lines);
                description.lines = inspection;
                if model.targeted(action) {
                    let targets = model.targets(action);
                    let current = open
                        .target
                        .and_then(|target| targets.iter().position(|(id, _)| *id == target));
                    let label = current.map(|i| targets[i].1.clone()).unwrap_or_else(|| {
                        if open.target.is_some() {
                            "Selected item is no longer owned".into()
                        } else {
                            format!("Choose item ({} eligible)", targets.len())
                        }
                    });
                    let target_action = if targets.is_empty() {
                        HudAction::JournalBlocked
                    } else {
                        HudAction::JournalPopup { targets: true }
                    };
                    frame.button(
                        HudPart::JournalTarget,
                        geometry.target,
                        target_action,
                        label,
                        c.tab_background_color,
                        false,
                        false,
                    );
                }
                frame.button(
                    HudPart::JournalCraft,
                    geometry.craft,
                    if report.ready {
                        HudAction::JournalCraft {
                            command: entry.key.clone(),
                            target: open.target,
                            expected: report.clone(),
                            rules_revision: model.rules_revision(),
                        }
                    } else {
                        HudAction::JournalBlocked
                    },
                    model.operation_name(action).into(),
                    if report.ready {
                        c.tab_selected_color
                    } else {
                        c.detail_background_color
                    },
                    false,
                    false,
                );
                frame.button(
                    HudPart::Assign,
                    geometry.pin,
                    HudAction::CatalogAssign,
                    if open.assignment {
                        "Select recipe, then slot"
                    } else {
                        "Pin to action bar"
                    }
                    .into(),
                    c.tab_background_color,
                    open.assignment,
                    false,
                );
            } else {
                description.title = entry.name.clone();
                description.lines = entry.lines.clone();
            }
        } else {
            description.title = error.unwrap_or_else(|| "No matching entries".into());
            description.lines.push(if open.journal_tab==JournalTab::Recipes {"Recipes appear when open by default or granted by current knowledge. Try another search or learn a pattern."}else{"Learn patterns from teachers, books or starting choices. Equipment grants are shown separately."}.into());
        }
        if let Some(targets) = open.journal_popup {
            let options = if targets {
                selected
                    .and_then(|entry| entry.action.as_deref())
                    .map(|action| {
                        model
                            .targets(action)
                            .into_iter()
                            .enumerate()
                            .map(|(index, (target, label))| {
                                (
                                    HudPart::JournalTargetOption(index),
                                    HudAction::JournalTarget(target),
                                    label,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                crafts
                    .iter()
                    .enumerate()
                    .map(|(index, craft)| {
                        (
                            HudPart::JournalFilterOption(index),
                            HudAction::JournalFilter(craft.clone()),
                            model.craft_name(craft),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let capacity = 5;
            let pages = options.len().div_ceil(capacity).max(1);
            let page = open.journal_popup_page.min(pages - 1);
            let start = page * capacity;
            let shown = &options[start.min(options.len())..(start + capacity).min(options.len())];
            let anchor = if targets {
                geometry.target
            } else {
                geometry.filter
            };
            let height = (shown.len() as f32 * 28. + if pages > 1 { 26. } else { 0. })
                .min((geometry.rect.height - 60.).max(0.));
            let y = if targets {
                (anchor.y - height).max(geometry.rect.y + 28.)
            } else {
                (anchor.y + anchor.height).min(geometry.rect.y + geometry.rect.height - height)
            };
            let popup = Rect::new(
                anchor.x,
                y,
                anchor
                    .width
                    .max(180.)
                    .min((geometry.rect.x + geometry.rect.width - anchor.x).max(0.)),
                height,
            );
            frame.popup = Some(popup);
            for (row, (part, action, label)) in shown.iter().enumerate() {
                frame.button(
                    *part,
                    Rect::new(popup.x, popup.y + row as f32 * 28., popup.width, 28.),
                    action.clone(),
                    label.clone(),
                    c.tab_background_color,
                    false,
                    true,
                );
            }
            if pages > 1 {
                if page > 0 {
                    frame.button(
                        HudPart::JournalPopupPage(false),
                        Rect::new(popup.x, popup.y + height - 26., popup.width / 2., 26.),
                        HudAction::JournalPopupPage(page - 1),
                        "Previous".into(),
                        c.tab_background_color,
                        false,
                        true,
                    );
                }
                if page + 1 < pages {
                    frame.button(
                        HudPart::JournalPopupPage(true),
                        Rect::new(
                            popup.x + popup.width / 2.,
                            popup.y + height - 26.,
                            popup.width / 2.,
                            26.,
                        ),
                        HudAction::JournalPopupPage(page + 1),
                        "Next".into(),
                        c.tab_background_color,
                        false,
                        true,
                    );
                }
            }
        }
        frame.detail = Some(DetailSpec {
            rect: geometry.detail,
            rows: screen_model::journal_detail_rows(&description),
            description,
            scroll: open.detail_scroll,
            font_size: 12.,
            small_size: 10.,
            background: c.detail_background_color,
        });
        Some(frame)
    }
}
