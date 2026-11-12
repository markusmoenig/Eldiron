//! Shared catalogue/spellbook metadata. No fonts, images, host or renderer owner.
use super::CatalogPanelConfig;
use crate::{
    Rect,
    client::rules_ui::{self, CommandState, RulesDescription},
};
use vek::Vec2;
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogEntryLayout {
    pub command: String,
    pub name: String,
    pub rect: Rect,
    pub icon_rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogGroupLayout {
    pub name: String,
    pub title_rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogTabLayout {
    pub id: String,
    pub name: String,
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogDetailRowKind {
    Summary,
    Section,
    Fact,
    Effect,
    Warning,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogDetailRow {
    pub label: Option<String>,
    pub text: String,
    pub kind: CatalogDetailRowKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogPanelLayout {
    pub tab: String,
    pub rect: Rect,
    pub close_rect: Rect,
    pub assign_rect: Option<Rect>,
    pub title_rect: Rect,
    pub tabs: Vec<CatalogTabLayout>,
    pub detail_rect: Option<Rect>,
    pub groups: Vec<CatalogGroupLayout>,
    pub entries: Vec<CatalogEntryLayout>,
    pub empty_rect: Option<Rect>,
    pub previous_page_rect: Option<Rect>,
    pub next_page_rect: Option<Rect>,
    pub page_label_rect: Option<Rect>,
    pub scroll_track_rect: Option<Rect>,
    pub scroll_thumb_rect: Option<Rect>,
    pub page: usize,
    pub page_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CatalogPanelKind {
    #[default]
    Actions,
    Spellbook,
}

impl CatalogPanelKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Actions => "Actions",
            Self::Spellbook => "Spellbook",
        }
    }
}

pub fn catalog_panel_geometry(
    catalog_groups: &[rules_ui::ActionCatalogGroup],
    config: &CatalogPanelConfig,
    tab: &str,
    requested_page: usize,
    position: Option<Vec2<i32>>,
    viewport: Vec2<i32>,
) -> CatalogPanelLayout {
    let mut selected_tab = tab.to_string();
    let complete_catalog: Vec<_> = catalog_groups
        .iter()
        .filter(|group| !group.entries.is_empty())
        .cloned()
        .collect();
    let fixed_group_count = complete_catalog.len().max(1);
    let mut tab_definitions = vec![("all".to_string(), "All".to_string())];
    tab_definitions.extend(
        complete_catalog
            .iter()
            .map(|group| (group.id.clone(), group.name.clone())),
    );
    if !config.show_tabs
        || !tab_definitions
            .iter()
            .any(|(id, _)| id.eq_ignore_ascii_case(&selected_tab))
    {
        selected_tab = "all".to_string();
    }
    let catalog: Vec<_> = if selected_tab.eq_ignore_ascii_case("all") {
        complete_catalog
    } else {
        complete_catalog
            .into_iter()
            .filter(|group| group.id.eq_ignore_ascii_case(&selected_tab))
            .collect()
    };
    let padding = config.padding;
    let title_height = config.title_height;
    let tabs_height = if config.show_tabs {
        config.tab_height
    } else {
        0.0
    };
    let group_title_height = 18.0;
    let card_width = config.cell_size;
    let card_height = config.cell_size;
    let column_gap = config.spacing;
    let row_gap = config.spacing;
    let group_gap = config.spacing;
    let columns = config.columns.max(1);
    let total_entries = catalog
        .iter()
        .map(|group| group.entries.len())
        .sum::<usize>();
    let page_capacity = config.rows.map(|rows| rows.max(1).saturating_mul(columns));
    let page_count = page_capacity
        .map(|capacity| total_entries.div_ceil(capacity.max(1)).max(1))
        .unwrap_or(1);
    let page = requested_page.min(page_count.saturating_sub(1));
    let page_start = page_capacity.map(|capacity| page * capacity).unwrap_or(0);
    let page_end = page_capacity
        .map(|capacity| page_start.saturating_add(capacity).min(total_entries))
        .unwrap_or(total_entries);
    let mut offset = 0;
    let mut visible_catalog = Vec::new();
    for group in &catalog {
        let group_start = offset;
        let group_end = group_start + group.entries.len();
        offset = group_end;
        let visible_start = page_start.max(group_start);
        let visible_end = page_end.min(group_end);
        if visible_start >= visible_end {
            continue;
        }
        let mut visible_group = group.clone();
        visible_group.entries =
            group.entries[(visible_start - group_start)..(visible_end - group_start)].to_vec();
        visible_catalog.push(visible_group);
    }

    let content_width = card_width * columns as f32 + column_gap * columns.saturating_sub(1) as f32;
    let natural_content_height = if visible_catalog.is_empty() {
        let rows = config.rows.unwrap_or(2).max(1);
        card_height * rows as f32 + row_gap * rows.saturating_sub(1) as f32
    } else {
        visible_catalog
            .iter()
            .map(|group| {
                let mut rows = group.entries.len().div_ceil(columns);
                if visible_catalog.len() == 1 {
                    rows = rows.max(config.rows.unwrap_or(0));
                }
                group_title_height
                    + card_height * rows as f32
                    + row_gap * rows.saturating_sub(1) as f32
            })
            .sum::<f32>()
            + group_gap * visible_catalog.len().saturating_sub(1) as f32
    };
    // An authored row count defines the viewport, not the height of whichever
    // tab happens to be selected. Reserve the same group-heading space for
    // every tab so the complete window never jumps or resizes on selection.
    let content_height = config
        .rows
        .map(|rows| {
            let rows = rows.max(1);
            group_title_height * fixed_group_count as f32
                + card_height * rows as f32
                + row_gap * rows.saturating_sub(1) as f32
                + group_gap * fixed_group_count.saturating_sub(1) as f32
        })
        .unwrap_or(natural_content_height);
    let details_extra = if config.show_details {
        config.detail_gap + config.detail_width
    } else {
        0.0
    };
    let computed_width = padding * 2.0 + content_width + details_extra;
    let computed_height = title_height + tabs_height + padding * 2.0 + content_height;
    let (panel_width, panel_height) = config
        .authored_rect
        .map(|rect| (rect.width.max(1.0), rect.height.max(1.0)))
        .unwrap_or((computed_width, computed_height));
    let viewport_width = viewport.x.max(1) as f32;
    let viewport_height = viewport.y.max(1) as f32;
    let default_x = config
        .authored_rect
        .map(|rect| rect.x)
        .unwrap_or_else(|| ((viewport_width - panel_width) * 0.5).max(2.0));
    let default_y = config
        .authored_rect
        .map(|rect| rect.y)
        .unwrap_or_else(|| ((viewport_height - panel_height) * 0.5).max(2.0));
    let position =
        position.unwrap_or_else(|| Vec2::new(default_x.round() as i32, default_y.round() as i32));
    let x = (position.x as f32).clamp(2.0, (viewport_width - panel_width - 2.0).max(2.0));
    let y = (position.y as f32).clamp(2.0, (viewport_height - panel_height - 2.0).max(2.0));
    let rect = Rect::new(x, y, panel_width, panel_height);
    let close_rect = Rect::new(x + panel_width - 26.0, y + 5.0, 20.0, 20.0);
    let assign_rect =
        config
            .show_assign
            .then_some(Rect::new(x + panel_width - 100.0, y + 5.0, 66.0, 20.0));
    let page_controls_right = if config.show_assign { 106.0 } else { 32.0 };
    let (previous_page_rect, next_page_rect, page_label_rect) = if page_count > 1 {
        (
            Some(Rect::new(
                x + panel_width - page_controls_right - 70.0,
                y + 5.0,
                20.0,
                20.0,
            )),
            Some(Rect::new(
                x + panel_width - page_controls_right - 20.0,
                y + 5.0,
                20.0,
                20.0,
            )),
            Some(Rect::new(
                x + panel_width - page_controls_right - 48.0,
                y + 5.0,
                26.0,
                20.0,
            )),
        )
    } else {
        (None, None, None)
    };
    let title_rect = Rect::new(
        x + padding,
        y,
        (panel_width - padding * 2.0).max(1.0),
        title_height,
    );
    let tabs = if config.show_tabs {
        let tab_area_width = (panel_width - padding * 2.0).max(1.0);
        let tab_width = (tab_area_width / tab_definitions.len().max(1) as f32).min(104.0);
        tab_definitions
            .into_iter()
            .enumerate()
            .map(|(index, (id, name))| CatalogTabLayout {
                id,
                name,
                rect: Rect::new(
                    x + padding + index as f32 * tab_width,
                    y + title_height,
                    tab_width,
                    tabs_height,
                ),
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut group_layouts = Vec::with_capacity(visible_catalog.len());
    let mut entry_layouts = Vec::new();
    let mut cursor_y = y + title_height + tabs_height + padding;
    let detail_rect = config.show_details.then_some(Rect::new(
        x + padding + content_width + config.detail_gap,
        cursor_y,
        config.detail_width,
        content_height,
    ));
    let (scroll_track_rect, scroll_thumb_rect) = if page_count > 1 {
        let track_width = 6.0;
        let track_x = if config.show_details {
            x + padding + content_width + (config.detail_gap - track_width) * 0.5
        } else {
            x + padding + content_width - track_width
        };
        let track = Rect::new(track_x, cursor_y, track_width, content_height);
        let thumb_height = (content_height / page_count as f32).max(24.0);
        let travel = (content_height - thumb_height).max(0.0);
        let progress = page as f32 / page_count.saturating_sub(1).max(1) as f32;
        let thumb = Rect::new(
            track_x,
            cursor_y + travel * progress,
            track_width,
            thumb_height,
        );
        (Some(track), Some(thumb))
    } else {
        (None, None)
    };
    let empty_rect = visible_catalog.is_empty().then_some(Rect::new(
        x + padding,
        cursor_y,
        content_width,
        content_height,
    ));
    for group in &visible_catalog {
        group_layouts.push(CatalogGroupLayout {
            name: group.name.clone(),
            title_rect: Rect::new(x + padding, cursor_y, content_width, group_title_height),
        });
        cursor_y += group_title_height;
        for (index, entry) in group.entries.iter().enumerate() {
            let column = index % columns;
            let row = index / columns;
            let card_x = x + padding + column as f32 * (card_width + column_gap);
            let card_y = cursor_y + row as f32 * (card_height + row_gap);
            let rect = Rect::new(card_x, card_y, card_width, card_height);
            let label_height = if config.show_names { 18.0 } else { 0.0 };
            let icon_size = (card_width - config.icon_inset * 2.0)
                .min(card_height - config.icon_inset * 2.0 - label_height)
                .max(1.0);
            entry_layouts.push(CatalogEntryLayout {
                command: entry.command.clone(),
                name: entry.name.clone(),
                rect,
                icon_rect: Rect::new(
                    card_x + (card_width - icon_size) * 0.5,
                    card_y + config.icon_inset,
                    icon_size,
                    icon_size,
                ),
            });
        }
        let rows = entry_layouts
            .iter()
            .rev()
            .take_while(|entry| entry.rect.y >= cursor_y)
            .count()
            .div_ceil(columns);
        cursor_y += card_height * rows as f32 + row_gap * rows.saturating_sub(1) as f32 + group_gap;
    }

    CatalogPanelLayout {
        tab: selected_tab,
        rect,
        close_rect,
        assign_rect,
        title_rect,
        tabs,
        detail_rect,
        groups: group_layouts,
        entries: entry_layouts,
        empty_rect,
        previous_page_rect,
        next_page_rect,
        page_label_rect,
        scroll_track_rect,
        scroll_thumb_rect,
        page,
        page_count,
    }
}

pub fn catalog_detail_rows(
    description: RulesDescription,
    state: &CommandState,
) -> Vec<CatalogDetailRow> {
    let mut summary = Vec::new();
    let mut facts = Vec::new();
    let mut effects = Vec::new();
    for line in description.lines {
        let Some((label, value)) = line.split_once(':') else {
            summary.push(CatalogDetailRow {
                label: None,
                text: line,
                kind: CatalogDetailRowKind::Summary,
            });
            continue;
        };
        let label = label.trim();
        let value = value.trim();
        let is_fact = matches!(
            label,
            "Activation"
                | "Target"
                | "Range"
                | "Damage"
                | "Cooldown"
                | "Cost"
                | "Consumes"
                | "Requires"
                | "Requires Target"
                | "Invocation"
        );
        let row = CatalogDetailRow {
            label: Some(label.to_string()),
            text: value.to_string(),
            kind: if is_fact {
                CatalogDetailRowKind::Fact
            } else {
                CatalogDetailRowKind::Effect
            },
        };
        if is_fact {
            facts.push(row);
        } else {
            effects.push(row);
        }
    }

    let mut rows = summary;
    if !facts.is_empty() {
        rows.push(CatalogDetailRow {
            label: None,
            text: "DETAILS".to_string(),
            kind: CatalogDetailRowKind::Section,
        });
        rows.extend(facts);
    }
    if !effects.is_empty() {
        rows.push(CatalogDetailRow {
            label: None,
            text: "EFFECTS".to_string(),
            kind: CatalogDetailRowKind::Section,
        });
        rows.extend(effects);
    }
    if !state.enabled
        && let Some(reason) = state.disabled_reason.as_deref()
    {
        rows.push(CatalogDetailRow {
            label: None,
            text: reason.to_string(),
            kind: CatalogDetailRowKind::Warning,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use rules_ui::{ActionCatalogEntry, ActionCatalogGroup};
    fn catalog() -> Vec<ActionCatalogGroup> {
        [
            ("combat", "Combat", vec!["attack", "defend", "sprint"]),
            ("magic", "Magic", vec!["heal", "fire"]),
        ]
        .into_iter()
        .map(|(id, name, entries)| ActionCatalogGroup {
            id: id.into(),
            name: name.into(),
            entries: entries
                .into_iter()
                .map(|name| ActionCatalogEntry {
                    command: format!("rules.{name}"),
                    name: name.into(),
                })
                .collect(),
        })
        .collect()
    }
    fn panel_config() -> CatalogPanelConfig {
        CatalogPanelConfig {
            columns: 2,
            rows: Some(1),
            cell_size: 20.,
            spacing: 2.,
            padding: 4.,
            title_height: 24.,
            tab_height: 20.,
            show_tabs: true,
            show_details: false,
            show_assign: true,
            ..Default::default()
        }
    }
    #[test]
    fn catalog_pages_preserve_group_and_command_identity_with_stable_tab_geometry() {
        let groups = catalog();
        let config = panel_config();
        let viewport = Vec2::new(200, 160);
        let first = catalog_panel_geometry(&groups, &config, "all", 0, None, viewport);
        assert_eq!(first.page_count, 3);
        assert_eq!(
            first
                .entries
                .iter()
                .map(|entry| entry.command.as_str())
                .collect::<Vec<_>>(),
            vec!["rules.attack", "rules.defend"]
        );
        let second = catalog_panel_geometry(&groups, &config, "all", 1, None, viewport);
        assert_eq!(
            second
                .entries
                .iter()
                .map(|entry| entry.command.as_str())
                .collect::<Vec<_>>(),
            vec!["rules.sprint", "rules.heal"]
        );
        assert_eq!(
            second
                .groups
                .iter()
                .map(|group| group.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Combat", "Magic"]
        );
        let last = catalog_panel_geometry(&groups, &config, "all", usize::MAX, None, viewport);
        assert_eq!(last.page, 2);
        assert_eq!(last.entries[0].command, "rules.fire");
        let magic = catalog_panel_geometry(&groups, &config, "MAGIC", 0, None, viewport);
        assert_eq!(
            first.rect, magic.rect,
            "selecting a shorter tab cannot resize the catalogue"
        );
        assert_eq!(first.tabs, magic.tabs);
        assert_eq!(magic.entries.len(), 2);
        assert_eq!(magic.page_count, 1);
        assert!(magic.assign_rect.is_some());
        assert_eq!(
            catalog_panel_geometry(&groups, &config, "missing", 0, None, viewport).tab,
            "all"
        );
    }
    #[test]
    fn declared_large_catalog_pages_allocate_only_real_entries_and_empty_catalog_has_one_page() {
        let config = CatalogPanelConfig {
            columns: usize::MAX,
            rows: Some(usize::MAX),
            ..panel_config()
        };
        let geometry = catalog_panel_geometry(
            &catalog(),
            &config,
            "all",
            usize::MAX,
            None,
            Vec2::new(200, 160),
        );
        assert_eq!(geometry.entries.len(), 5);
        assert_eq!((geometry.page, geometry.page_count), (0, 1));
        let config = CatalogPanelConfig {
            show_tabs: false,
            ..panel_config()
        };
        let geometry = catalog_panel_geometry(
            &[],
            &config,
            "magic",
            usize::MAX,
            Some(Vec2::new(-100, -100)),
            Vec2::new(200, 160),
        );
        assert_eq!((geometry.page, geometry.page_count), (0, 1));
        assert_eq!(geometry.tab, "all");
        assert!(geometry.entries.is_empty() && geometry.tabs.is_empty());
        assert!(geometry.empty_rect.is_some());
        assert_eq!([geometry.rect.x, geometry.rect.y], [2., 2.]);
    }
    #[test]
    fn catalog_details_keep_summary_facts_effects_and_unavailability_in_display_order() {
        let description = RulesDescription {
            title: "Heal 東京".into(),
            subtitle: None,
            lines: vec![
                "Restore a friend.".into(),
                "Target: Ally".into(),
                "Damage: 4:8".into(),
                "Healing: 12 HP".into(),
            ],
        };
        let state = CommandState {
            enabled: false,
            disabled_reason: Some("Requires mana".into()),
            ..Default::default()
        };
        let rows = catalog_detail_rows(description, &state);
        assert_eq!(
            rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
            vec![
                CatalogDetailRowKind::Summary,
                CatalogDetailRowKind::Section,
                CatalogDetailRowKind::Fact,
                CatalogDetailRowKind::Fact,
                CatalogDetailRowKind::Section,
                CatalogDetailRowKind::Effect,
                CatalogDetailRowKind::Warning
            ]
        );
        assert_eq!(rows[3].text, "4:8");
        assert_eq!(rows.last().unwrap().text, "Requires mana");
        assert_eq!(rows[5].label.as_deref(), Some("Healing"));
    }
}
