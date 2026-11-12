//! Inventory metadata preserves sorted item addresses without a bitmap or dense page allocation.
use super::InventoryPanelConfig;
use crate::{Entity, Rect};
use vek::Vec2;
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryPanelSlotLayout {
    pub rect: Rect,
    pub inventory_index: Option<usize>,
    pub item_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InventoryGeometry {
    pub rect: Rect,
    pub title_rect: Rect,
    pub close_rect: Rect,
    pub tab_rect: Rect,
    pub sort_rect: Rect,
    pub capacity_rect: Rect,
    pub previous_page_rect: Option<Rect>,
    pub next_page_rect: Option<Rect>,
    pub page_rect: Option<Rect>,
    pub page: usize,
    pub page_count: usize,
    pub grid_rect: Rect,
    page_size: usize,
    columns: usize,
    cell: f32,
    step: f32,
    origin: [f64; 2],
    entries: Vec<(usize, u32)>,
    empty: Vec<usize>,
}

impl InventoryGeometry {
    pub fn slot_count(&self) -> usize {
        self.page_size
    }
    pub fn slot(&self, index: usize) -> Option<InventoryPanelSlotLayout> {
        if index >= self.page_size {
            return None;
        }
        let entry = self.entries.get(index).copied();
        Some(InventoryPanelSlotLayout {
            rect: Rect::new(
                (self.origin[0] + (index % self.columns) as f64 * self.step as f64) as f32,
                (self.origin[1] + (index / self.columns) as f64 * self.step as f64) as f32,
                self.cell,
                self.cell,
            ),
            inventory_index: entry.map(|e| e.0).or_else(|| {
                self.empty
                    .get(index.saturating_sub(self.entries.len()))
                    .copied()
            }),
            item_id: entry.map(|e| e.1),
        })
    }
    pub fn slots(&self) -> impl Iterator<Item = InventoryPanelSlotLayout> + '_ {
        (0..self.page_size).filter_map(|index| self.slot(index))
    }
    pub fn visible_slots(
        &self,
        clip: Rect,
        offset: [f64; 2],
    ) -> impl Iterator<Item = (usize, InventoryPanelSlotLayout)> + '_ {
        super::grid::visible_square_grid(
            self.origin,
            self.columns,
            self.page_size,
            self.cell as f64,
            self.step as f64,
            clip,
            offset,
        )
        .filter_map(|(index, rect)| {
            let mut slot = self.slot(index)?;
            slot.rect = rect;
            Some((index, slot))
        })
    }
}
pub fn inventory_panel_geometry(
    actor: &Entity,
    config: &InventoryPanelConfig,
    category: &str,
    sort: &str,
    page: usize,
    position: Option<Vec2<i32>>,
    viewport: Vec2<i32>,
) -> InventoryGeometry {
    let content_width = config.columns.max(1) as f32 * config.cell_size
        + config.columns.saturating_sub(1) as f32 * config.spacing;
    let grid_height = config.rows.max(1) as f32 * config.cell_size
        + config.rows.saturating_sub(1) as f32 * config.spacing;
    let computed_width = content_width + config.padding * 2.0;
    let computed_height = config.title_height
        + config.tab_height
        + grid_height
        + config.footer_height
        + config.padding * 3.0;
    let authored = config.authored_rect;
    let width = authored
        .map(|rect| rect.width.max(computed_width))
        .unwrap_or(computed_width);
    let height = authored
        .map(|rect| rect.height.max(computed_height))
        .unwrap_or(computed_height);
    let default_position = authored
        .map(|rect| Vec2::new(rect.x.round() as i32, rect.y.round() as i32))
        .unwrap_or_else(|| {
            Vec2::new(
                ((viewport.x as f32 - width) * 0.5).round() as i32,
                ((viewport.y as f32 - height) * 0.5).round() as i32,
            )
        });
    let mut position = position.unwrap_or(default_position);
    position.x = position
        .x
        .clamp(2, (viewport.x - width.round() as i32 - 2).max(2));
    position.y = position
        .y
        .clamp(2, (viewport.y - height.round() as i32 - 2).max(2));

    let rect = Rect::new(position.x as f32, position.y as f32, width, height);
    let close_rect = Rect::new(
        rect.x + rect.width - config.title_height + 5.0,
        rect.y + 5.0,
        config.title_height - 10.0,
        config.title_height - 10.0,
    );
    let title_rect = Rect::new(
        rect.x + config.padding,
        rect.y,
        rect.width - config.padding * 2.0,
        config.title_height,
    );
    let tab_rect = Rect::new(
        rect.x + config.padding,
        rect.y + config.title_height,
        rect.width - config.padding * 2.0,
        config.tab_height,
    );
    let grid_x = rect.x + (rect.width - content_width) * 0.5;
    let grid_y = tab_rect.y + tab_rect.height + config.padding;
    let footer_y = rect.y + rect.height - config.footer_height - config.padding;
    let sort_rect = Rect::new(
        rect.x + config.padding,
        footer_y,
        config.sort_width.min(rect.width * 0.4),
        config.footer_height,
    );
    let capacity_rect = Rect::new(
        rect.x + rect.width - config.padding - 116.0,
        footer_y,
        116.0,
        config.footer_height,
    );

    let items = super::inventory_items(actor, category, sort);
    let page_size = config.columns.max(1).saturating_mul(config.rows.max(1));
    let page_count = items.len().max(1).div_ceil(page_size);
    let page = page.min(page_count.saturating_sub(1));
    let page_items = items
        .iter()
        .skip(page * page_size)
        .take(page_size)
        .map(|(index, item)| (*index, item.id))
        .collect::<Vec<_>>();
    let empty_indices = actor
        .inventory
        .iter()
        .enumerate()
        .filter_map(|(index, item)| item.is_none().then_some(index))
        .collect::<Vec<_>>();
    let (previous_page_rect, next_page_rect, page_rect) = if page_count > 1 {
        let center = rect.x + rect.width * 0.5;
        (
            Some(Rect::new(
                center - 54.0,
                footer_y,
                28.0,
                config.footer_height,
            )),
            Some(Rect::new(
                center + 26.0,
                footer_y,
                28.0,
                config.footer_height,
            )),
            Some(Rect::new(
                center - 24.0,
                footer_y,
                48.0,
                config.footer_height,
            )),
        )
    } else {
        (None, None, None)
    };

    InventoryGeometry {
        rect,
        title_rect,
        close_rect,
        tab_rect,
        sort_rect,
        capacity_rect,
        previous_page_rect,
        next_page_rect,
        page_rect,
        page,
        page_count,
        page_size,
        columns: config.columns.max(1),
        cell: config.cell_size,
        step: config.cell_size + config.spacing,
        origin: [grid_x as f64, grid_y as f64],
        entries: page_items,
        empty: empty_indices,
        grid_rect: Rect::new(grid_x, grid_y, content_width, grid_height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Item, Value};
    fn actor() -> Entity {
        let mut actor = Entity::new();
        actor.inventory = vec![None; 6];
        for (index, name, slot) in [
            (0, "Beta", "material"),
            (2, "Alpha", "main_hand"),
            (5, "Zinc", "resource"),
        ] {
            let mut item = Item::new();
            item.id = index as u32 + 10;
            item.item_type = name.into();
            item.attributes.set("slot", Value::Str(slot.into()));
            actor.inventory[index] = Some(item);
        }
        actor
    }
    fn config() -> InventoryPanelConfig {
        InventoryPanelConfig {
            columns: 2,
            rows: 1,
            cell_size: 20.,
            spacing: 2.,
            padding: 4.,
            title_height: 24.,
            tab_height: 20.,
            footer_height: 20.,
            ..Default::default()
        }
    }
    #[test]
    fn inventory_pages_keep_original_sparse_indices_and_only_real_empty_destinations() {
        let actor = actor();
        let config = config();
        let first =
            inventory_panel_geometry(&actor, &config, "all", "name", 0, None, Vec2::new(200, 160));
        assert_eq!(first.page_count, 2);
        assert_eq!(first.rect, Rect::new(75., 32., 50., 96.));
        assert_eq!(first.slot(0).unwrap().inventory_index, Some(2));
        assert_eq!(first.slot(1).unwrap().inventory_index, Some(0));
        let last = inventory_panel_geometry(
            &actor,
            &config,
            "all",
            "name",
            999,
            None,
            Vec2::new(200, 160),
        );
        assert_eq!(last.page, 1);
        assert_eq!(last.slot(0).unwrap().inventory_index, Some(5));
        assert_eq!(last.slot(1).unwrap().inventory_index, Some(1));
        assert!(last.slot(1).unwrap().item_id.is_none());
        let filtered = inventory_panel_geometry(
            &actor,
            &config,
            "equipment",
            "name",
            0,
            None,
            Vec2::new(200, 160),
        );
        assert_eq!(filtered.slot(0).unwrap().inventory_index, Some(2));
        assert_eq!(filtered.slot(1).unwrap().inventory_index, Some(1));
    }
    #[test]
    fn fractional_grid_clipping_and_declared_huge_pages_do_not_allocate_dense_slots() {
        let actor = actor();
        let mut config = config();
        config.cell_size = 20.5;
        config.spacing = 2.25;
        let geometry = inventory_panel_geometry(
            &actor,
            &config,
            "all",
            "newest",
            0,
            Some(Vec2::new(2, 2)),
            Vec2::new(200, 160),
        );
        let clip = Rect::new(
            geometry.grid_rect.x + 22.75,
            geometry.grid_rect.y,
            20.5,
            20.5,
        );
        let slots = geometry.visible_slots(clip, [0.; 2]).collect::<Vec<_>>();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].0, 1);
        assert_eq!(slots[0].1.inventory_index, Some(2));
        config.columns = usize::MAX;
        config.rows = usize::MAX;
        let huge = inventory_panel_geometry(
            &actor,
            &config,
            "all",
            "newest",
            0,
            None,
            Vec2::new(200, 160),
        );
        assert_eq!(huge.slot_count(), usize::MAX);
        assert!(
            huge.visible_slots(Rect::new(0., 0., 200., 160.), [0.; 2])
                .count()
                < 100
        );
    }
}
