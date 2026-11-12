//! Shared container geometry; no destination bitmap, font, window or GPU owner.
//! The slot grid is lazy so a native viewport can prepare only visible content.
use crate::{Item, Rect, client::rules_ui::ContainerUiTemplate};
use vek::Vec2;

pub fn container_slot_count(item: &Item) -> usize {
    let declared = item.attributes.get_int_default("container_slots", 0).max(0) as usize;
    declared
        .max(item.max_capacity as usize)
        .max(item.container.as_ref().map_or(0, Vec::len))
        .max(1)
}

#[derive(Clone, Debug)]
pub struct ContainerGeometry {
    pub rect: Rect,
    pub title_bar: Option<Rect>,
    pub close: Option<Rect>,
    pub title: Option<Rect>,
    origin: [f64; 2],
    columns: usize,
    count: usize,
    size: f64,
    step: f64,
}
impl ContainerGeometry {
    pub fn new(
        position: Vec2<i32>,
        template: &ContainerUiTemplate,
        count: usize,
        viewport: Vec2<i32>,
    ) -> Self {
        let columns = template.columns.max(1);
        let rows = template
            .rows
            .unwrap_or_else(|| count.div_ceil(columns))
            .max(1);
        let size = template.slot_size.max(8) as f64;
        let gap = template.gap.max(0) as f64;
        let padding = template.padding.max(0) as f64;
        let title_height = if template.title { 26. } else { 0. };
        // Widen before multiplying: large authored capacities/templates must not
        // wrap integer dimensions or allocate one rectangle per invisible slot.
        let width = columns as f64 * size + columns.saturating_sub(1) as f64 * gap + padding * 2.;
        let height =
            rows as f64 * size + rows.saturating_sub(1) as f64 * gap + padding * 2. + title_height;
        let place = |position: i32, extent: f64, viewport: i32| {
            let position = position as f64;
            if position + extent > viewport as f64 {
                (viewport as f64 - extent - 2.).max(2.)
            } else {
                position.max(2.)
            }
        };
        let x = place(position.x, width, viewport.x);
        let y = place(position.y, height, viewport.y);
        Self {
            rect: Rect::new(x as f32, y as f32, width as f32, height as f32),
            title_bar: template
                .title
                .then(|| Rect::new(x as f32, y as f32, width as f32, title_height as f32)),
            close: template
                .title
                .then(|| Rect::new((x + width - 27.) as f32, (y + 3.) as f32, 22., 20.)),
            title: template.title.then(|| {
                Rect::new(
                    (x + padding) as f32,
                    (y + 3.) as f32,
                    (width - padding * 2. - 30.).max(1.) as f32,
                    20.,
                )
            }),
            origin: [x + padding, y + padding + title_height],
            columns,
            count,
            size,
            step: size + gap,
        }
    }
    pub fn slot_count(&self) -> usize {
        self.count
    }
    pub fn slot(&self, index: usize) -> Option<Rect> {
        (index < self.count).then(|| {
            Rect::new(
                (self.origin[0] + (index % self.columns) as f64 * self.step) as f32,
                (self.origin[1] + (index / self.columns) as f64 * self.step) as f32,
                self.size as f32,
                self.size as f32,
            )
        })
    }
    /// Reference adapters may enumerate all slots. Native presentation should
    /// use `visible_slots` with its actual viewport/content clip instead.
    pub fn slots(&self) -> impl Iterator<Item = (usize, Rect)> + '_ {
        (0..self.count).map(|index| (index, self.slot(index).unwrap()))
    }
    /// Row-major, positive-area intersections, with work proportional to visible
    /// rows/columns rather than declared capacity. Slot indices stay absolute.
    pub fn visible_slots(&self, clip: Rect) -> impl Iterator<Item = (usize, Rect)> + '_ {
        self.visible_slots_offset(clip, [0.; 2])
    }
    pub fn content_size(&self) -> [f64; 2] {
        let extent = |count: usize| {
            if count == 0 {
                0.
            } else {
                (count - 1) as f64 * self.step + self.size
            }
        };
        [
            extent(self.columns),
            extent(self.count.div_ceil(self.columns)),
        ]
    }
    /// Scroll in widened grid coordinates, translating back to viewport-local
    /// floats only after choosing absolute slots. Large inventories cannot lose
    /// cell positions through an early f32 conversion.
    pub fn visible_slots_offset(
        &self,
        clip: Rect,
        offset: [f64; 2],
    ) -> impl Iterator<Item = (usize, Rect)> + '_ {
        super::grid::visible_square_grid(
            self.origin,
            self.columns,
            self.count,
            self.size,
            self.step,
            clip,
            offset,
        )
    }
}

/// Authored nine-piece frame placement, in the reference painter's layer order.
/// Only source references and rectangles are returned; the renderer owns images.
pub fn container_frame_tiles(
    rect: Rect,
    template: &ContainerUiTemplate,
) -> impl Iterator<Item = (&str, Rect)> {
    let edge = template
        .slot_size
        .min((rect.width as i32 / 3).max(1))
        .min((rect.height as i32 / 3).max(1))
        .max(8) as f32;
    [
        (template.tiles.center.as_deref(), rect),
        (
            template.tiles.top.as_deref(),
            Rect::new(
                rect.x + edge,
                rect.y,
                (rect.width - edge * 2.).max(1.),
                edge,
            ),
        ),
        (
            template.tiles.bottom.as_deref(),
            Rect::new(
                rect.x + edge,
                rect.y + rect.height - edge,
                (rect.width - edge * 2.).max(1.),
                edge,
            ),
        ),
        (
            template.tiles.left.as_deref(),
            Rect::new(
                rect.x,
                rect.y + edge,
                edge,
                (rect.height - edge * 2.).max(1.),
            ),
        ),
        (
            template.tiles.right.as_deref(),
            Rect::new(
                rect.x + rect.width - edge,
                rect.y + edge,
                edge,
                (rect.height - edge * 2.).max(1.),
            ),
        ),
        (
            template.tiles.top_left.as_deref(),
            Rect::new(rect.x, rect.y, edge, edge),
        ),
        (
            template.tiles.top_right.as_deref(),
            Rect::new(rect.x + rect.width - edge, rect.y, edge, edge),
        ),
        (
            template.tiles.bottom_left.as_deref(),
            Rect::new(rect.x, rect.y + rect.height - edge, edge, edge),
        ),
        (
            template.tiles.bottom_right.as_deref(),
            Rect::new(
                rect.x + rect.width - edge,
                rect.y + rect.height - edge,
                edge,
                edge,
            ),
        ),
    ]
    .into_iter()
    .filter_map(|(source, rect)| source.map(|source| (source, rect)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_frame_pieces_keep_source_order_and_edge_and_corner_placement() {
        let mut template = ContainerUiTemplate::default();
        template.tiles.center = Some("center".into());
        template.tiles.top = Some("top".into());
        template.tiles.left = Some("left".into());
        template.tiles.bottom_right = Some("corner".into());
        assert_eq!(
            container_frame_tiles(Rect::new(10., 20., 156., 110.), &template).collect::<Vec<_>>(),
            vec![
                ("center", Rect::new(10., 20., 156., 110.)),
                ("top", Rect::new(42., 20., 92., 32.)),
                ("left", Rect::new(10., 52., 32., 46.)),
                ("corner", Rect::new(134., 98., 32., 32.)),
            ]
        );
        let small =
            container_frame_tiles(Rect::new(0., 0., 20., 20.), &template).collect::<Vec<_>>();
        assert_eq!(small[1], ("top", Rect::new(8., 0., 4., 8.)));
        assert_eq!(small[3], ("corner", Rect::new(12., 12., 8., 8.)));
    }

    #[test]
    fn default_geometry_preserves_reference_chrome_and_row_major_placement() {
        let geometry = ContainerGeometry::new(
            Vec2::new(290, 190),
            &Default::default(),
            6,
            Vec2::new(320, 240),
        );
        assert_eq!(geometry.rect, Rect::new(162., 128., 156., 110.));
        assert_eq!(geometry.title_bar, Some(Rect::new(162., 128., 156., 26.)));
        assert_eq!(geometry.title, Some(Rect::new(170., 131., 110., 20.)));
        assert_eq!(geometry.close, Some(Rect::new(291., 131., 22., 20.)));
        assert_eq!(geometry.slot(0), Some(Rect::new(170., 162., 32., 32.)));
        assert_eq!(geometry.slot(4), Some(Rect::new(170., 198., 32., 32.)));
        assert_eq!(geometry.slot(6), None);
        assert_eq!(geometry.slots().count(), 6);
    }
    #[test]
    fn authored_rows_titleless_templates_and_slot_capacity_keep_their_semantics() {
        let template = ContainerUiTemplate {
            columns: 2,
            rows: Some(4),
            slot_size: 16,
            gap: 3,
            padding: 5,
            title: false,
            ..Default::default()
        };
        let geometry =
            ContainerGeometry::new(Vec2::new(-20, -30), &template, 3, Vec2::new(320, 240));
        assert_eq!(geometry.rect, Rect::new(2., 2., 45., 83.));
        assert!(
            geometry.title.is_none() && geometry.close.is_none() && geometry.title_bar.is_none()
        );
        assert_eq!(geometry.slot(2), Some(Rect::new(7., 26., 16., 16.)));
        let mut bag = Item::new();
        bag.max_capacity = 2;
        bag.container = Some(vec![Item::new(), Item::new(), Item::new()]);
        assert_eq!(container_slot_count(&bag), 3);
        bag.attributes.set("container_slots", crate::Value::Int(8));
        assert_eq!(container_slot_count(&bag), 8);
    }
    #[test]
    fn visible_slots_match_brute_force_clipping_and_reject_invalid_or_empty_clips() {
        let geometry = ContainerGeometry::new(
            Vec2::new(10, 20),
            &Default::default(),
            23,
            Vec2::new(320, 240),
        );
        for clip in [
            Rect::new(0., 0., 320., 240.),
            Rect::new(49.5, 61.25, 42.5, 44.75),
            Rect::new(50., 54., 4., 32.),
            Rect::new(-40., -10., 20., 20.),
        ] {
            let expected = geometry
                .slots()
                .filter(|(_, slot)| {
                    slot.x < clip.x + clip.width
                        && slot.x + slot.width > clip.x
                        && slot.y < clip.y + clip.height
                        && slot.y + slot.height > clip.y
                })
                .collect::<Vec<_>>();
            assert_eq!(geometry.visible_slots(clip).collect::<Vec<_>>(), expected);
        }
        for clip in [
            Rect::new(0., 0., 0., 10.),
            Rect::new(0., 0., 10., -2.),
            Rect::new(f32::NAN, 0., 10., 10.),
            Rect::new(0., 0., f32::INFINITY, 10.),
        ] {
            assert_eq!(geometry.visible_slots(clip).count(), 0);
        }
    }
    #[test]
    fn large_scrolled_rows_keep_fractional_viewport_positions_and_exact_source_indices() {
        let geometry = ContainerGeometry::new(
            Vec2::new(2, 2),
            &Default::default(),
            u32::MAX as usize,
            Vec2::new(320, 240),
        );
        let row = 1usize << 25;
        let slots = geometry
            .visible_slots_offset(
                Rect::new(10., 36., 140., 32.),
                [0., row as f64 * 36. + 0.25],
            )
            .collect::<Vec<_>>();
        assert_eq!(slots.len(), 4);
        assert_eq!(slots[0], (row * 4, Rect::new(10., 35.75, 32., 32.)));
        assert_eq!(slots[3], (row * 4 + 3, Rect::new(118., 35.75, 32., 32.)));
    }

    #[test]
    fn huge_capacities_and_column_counts_keep_visibility_work_bounded_without_integer_wrap() {
        for columns in [4, usize::MAX] {
            let template = ContainerUiTemplate {
                columns,
                ..Default::default()
            };
            let geometry = ContainerGeometry::new(
                Vec2::new(i32::MAX, i32::MAX),
                &template,
                usize::MAX,
                Vec2::new(320, 240),
            );
            assert_eq!(geometry.slot_count(), usize::MAX);
            assert!(geometry.rect.width.is_finite() && geometry.rect.height.is_finite());
            let slots = geometry
                .visible_slots(Rect::new(0., 0., 320., 240.))
                .collect::<Vec<_>>();
            assert_eq!(slots.len(), if columns == 4 { 24 } else { 9 });
            assert!(slots.iter().all(|(index, _)| *index < usize::MAX));
        }
    }
}
