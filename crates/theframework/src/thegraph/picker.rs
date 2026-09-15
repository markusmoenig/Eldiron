use super::*;
/// Reusable searchable popup. Item IDs are stable keys, never list indices.
#[derive(Clone, Debug)]
pub struct GraphPickerItem {
    pub id: String,
    pub label: String,
}
pub struct GraphPicker {
    pub title: String,
    pub query: String,
    pub search_label: String,
    pub empty_label: String,
    pub origin: Point,
    pub items: Vec<GraphPickerItem>,
    pub scroll: usize,
    pub hovered: Option<String>,
    pointer: Option<Point>,
    scroll_remainder: f32,
    compact: bool,
    width: f32,
    max_rows: usize,
}
impl GraphPicker {
    pub fn new(title: &str, origin: Point, items: Vec<GraphPickerItem>) -> Self {
        Self {
            title: title.into(),
            query: String::new(),
            search_label: "Search".into(),
            empty_label: "No matches".into(),
            origin,
            items,
            scroll: 0,
            hovered: None,
            pointer: None,
            scroll_remainder: 0.,
            compact: false,
            width: 330.,
            max_rows: 8,
        }
    }
    /// A node-anchored dropdown with no title or unused space. Typing still filters.
    pub fn compact(anchor: GraphRect, viewport: Point, items: Vec<GraphPickerItem>) -> Self {
        let mut p = Self::new(
            "",
            [anchor.origin[0], anchor.origin[1] + anchor.size[1]],
            items,
        );
        p.compact = true;
        p.width = anchor.size[0].clamp(180., 260.);
        p.max_rows = (((viewport[1] - 32.) / 24.) as usize).clamp(1, 6);
        if p.origin[1] + p.size()[1] > viewport[1] {
            p.origin[1] = anchor.origin[1] - p.size()[1];
        }
        p.fit(viewport);
        p
    }
    pub fn fit(&mut self, viewport: Point) {
        if !self.compact {
            return;
        }
        self.max_rows = (((viewport[1] - 32.).max(0.) / 24.) as usize).clamp(1, 6);
        self.scroll = self
            .scroll
            .min(self.filtered().len().saturating_sub(self.max_rows));
        self.width = self.width.min(viewport[0].max(1.));
        self.origin[0] = self.origin[0].clamp(0., (viewport[0] - self.width).max(0.));
        self.origin[1] = self.origin[1].clamp(0., (viewport[1] - self.size()[1]).max(0.));
        self.hover(self.pointer);
    }
    fn header(&self) -> f32 {
        if self.compact {
            if self.query.is_empty() { 4. } else { 28. }
        } else {
            64.
        }
    }
    fn step(&self) -> f32 {
        if self.compact { 24. } else { 30. }
    }
    pub fn size(&self) -> Point {
        if self.compact {
            [
                self.width,
                self.header() + self.filtered().len().clamp(1, self.max_rows) as f32 * 24. + 4.,
            ]
        } else {
            [330., 320.]
        }
    }
    pub fn filtered(&self) -> Vec<&GraphPickerItem> {
        let query = self.query.to_lowercase();
        self.items
            .iter()
            .filter(|i| i.label.to_lowercase().contains(&query))
            .collect()
    }
    pub fn type_char(&mut self, c: char) {
        if !c.is_control() {
            self.query.push(c);
            self.scroll = 0;
            self.scroll_remainder = 0.;
            self.hover(self.pointer);
        }
    }
    pub fn backspace(&mut self) {
        self.query.pop();
        self.scroll = 0;
        self.scroll_remainder = 0.;
        self.hover(self.pointer);
    }
    pub fn hover(&mut self, point: Option<Point>) -> bool {
        self.pointer = point;
        let next = point.and_then(|p| self.pick(p));
        let changed = self.hovered != next;
        self.hovered = next;
        changed
    }
    /// Pixel deltas from a touchpad accumulate into rows instead of skipping the
    /// entire list on each event. Positive deltas move down through the list.
    pub fn scroll_pixels(&mut self, delta: f32) {
        self.scroll_remainder += delta;
        let rows = (self.scroll_remainder / self.step()).trunc() as i32;
        self.scroll_remainder -= rows as f32 * self.step();
        self.scroll_by(rows);
    }
    pub fn scroll_by(&mut self, delta: i32) {
        self.scroll = (self.scroll as i32 + delta).max(0) as usize;
        self.scroll = self
            .scroll
            .min(self.filtered().len().saturating_sub(self.max_rows));
        self.hover(self.pointer);
    }
    pub fn contains(&self, p: Point) -> bool {
        GraphRect {
            origin: self.origin,
            size: self.size(),
        }
        .contains(p)
    }
    pub fn pick(&self, p: Point) -> Option<String> {
        let r = GraphRect {
            origin: self.origin,
            size: self.size(),
        };
        if !r.contains(p) || p[1] < self.origin[1] + self.header() {
            return None;
        }
        let index = ((p[1] - self.origin[1] - self.header()) / self.step()) as usize;
        if index >= self.max_rows {
            return None;
        }
        self.filtered()
            .get(self.scroll + index)
            .map(|i| i.id.clone())
    }
    fn paint_scrollbar(&self, p: &mut dyn GraphPainter) {
        let count = self.filtered().len();
        if count <= self.max_rows {
            return;
        }
        let height = self.max_rows as f32 * self.step();
        let thumb = (height * self.max_rows as f32 / count as f32)
            .max(12.)
            .min(height);
        let x = self.origin[0] + self.width - 4.;
        let y = self.origin[1] + self.header();
        p.round_rect(
            GraphRect {
                origin: [x, y],
                size: [3., height],
            },
            1.5,
            [34, 39, 40, 255],
        );
        let offset = (height - thumb) * self.scroll as f32 / (count - self.max_rows) as f32;
        p.round_rect(
            GraphRect {
                origin: [x, y + offset],
                size: [3., thumb],
            },
            1.5,
            [120, 152, 157, 255],
        );
    }
    pub fn paint(&self, p: &mut dyn GraphPainter) {
        if self.compact {
            p.round_rect(
                GraphRect {
                    origin: self.origin,
                    size: self.size(),
                },
                5.,
                [24, 29, 30, 255],
            );
            if !self.query.is_empty() {
                p.text(
                    GraphRect {
                        origin: [self.origin[0] + 8., self.origin[1] + 4.],
                        size: [self.width - 16., 20.],
                    },
                    &self.query,
                    13.,
                    [100, 205, 227, 255],
                );
            }
            let filtered = self.filtered();
            for (index, item) in filtered
                .iter()
                .skip(self.scroll)
                .take(self.max_rows)
                .enumerate()
            {
                let rect = GraphRect {
                    origin: [
                        self.origin[0] + 4.,
                        self.origin[1] + self.header() + index as f32 * 24.,
                    ],
                    size: [self.width - 8., 22.],
                };
                p.round_rect(
                    rect,
                    3.,
                    if self.hovered.as_deref() == Some(item.id.as_str()) {
                        [66, 105, 116, 255]
                    } else {
                        [50, 57, 58, 255]
                    },
                );
                p.text(
                    GraphRect {
                        origin: [rect.origin[0] + 6., rect.origin[1] + 2.],
                        size: [rect.size[0] - 12., 18.],
                    },
                    &item.label,
                    13.,
                    [233, 235, 230, 255],
                );
            }
            if filtered.is_empty() {
                p.text(
                    GraphRect {
                        origin: [self.origin[0] + 8., self.origin[1] + self.header()],
                        size: [self.width - 16., 22.],
                    },
                    &self.empty_label,
                    13.,
                    [185, 190, 185, 255],
                );
            }
            self.paint_scrollbar(p);
            return;
        }
        p.round_rect(
            GraphRect {
                origin: self.origin,
                size: self.size(),
            },
            12.,
            [19, 23, 25, 255],
        );
        p.text(
            GraphRect {
                origin: [self.origin[0] + 12., self.origin[1] + 10.],
                size: [306., 22.],
            },
            &self.title,
            16.,
            [232, 239, 235, 255],
        );
        p.text(
            GraphRect {
                origin: [self.origin[0] + 12., self.origin[1] + 36.],
                size: [306., 22.],
            },
            &format!("{}: {}", self.search_label, self.query),
            13.,
            [100, 205, 227, 255],
        );
        let items = self.filtered();
        for (i, item) in items.iter().skip(self.scroll).take(8).enumerate() {
            let r = GraphRect {
                origin: [self.origin[0] + 8., self.origin[1] + 64. + i as f32 * 30.],
                size: [314., 27.],
            };
            p.round_rect(
                r,
                6.,
                if self.hovered.as_deref() == Some(item.id.as_str()) {
                    [66, 105, 116, 255]
                } else {
                    [50, 57, 58, 255]
                },
            );
            p.text(
                GraphRect {
                    origin: [r.origin[0] + 8., r.origin[1] + 4.],
                    size: [298., 22.],
                },
                &item.label,
                13.,
                [233, 235, 230, 255],
            );
        }
        if items.is_empty() {
            p.text(
                GraphRect {
                    origin: [self.origin[0] + 12., self.origin[1] + 70.],
                    size: [300., 22.],
                },
                &self.empty_label,
                13.,
                [185, 190, 185, 255],
            );
        }
        self.paint_scrollbar(p);
    }
}
