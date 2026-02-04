use crate::ui::{
    drawable::Drawable,
    event::{UiAction, UiEvent, UiEventKind, UiEventOutcome},
    workspace::{UiView, ViewContext},
};
use uuid::Uuid;
use vek::Vec4;

#[derive(Debug, Clone)]
pub struct ProjectBrowserStyle {
    pub rect: [f32; 4],
    pub background: Vec4<f32>,
    pub border: Vec4<f32>,
    pub border_px: f32,
    pub radius_px: f32,
    pub layer: i32,

    // Grid layout
    pub columns: usize,
    pub cell_width: f32,
    pub cell_height: f32,
    pub spacing: f32,
    pub padding: f32,

    // Cell styling
    pub cell_background: Vec4<f32>,
    pub cell_border: Vec4<f32>,
    pub cell_hover_background: Vec4<f32>,
    pub cell_radius_px: f32,
    pub cell_border_px: f32,

    // Thumbnail
    pub thumbnail_padding: f32,

    // Text
    pub text_color: Vec4<f32>,
    pub text_size: f32,
}

#[derive(Debug, Clone)]
pub struct ProjectBrowserItem {
    pub id: String,
    pub name: String,
    pub thumbnail_tile: Option<uuid::Uuid>,
    pub subtitle: Option<String>, // e.g., "Modified 2 days ago"
}

pub struct ProjectBrowser {
    pub id: String,
    pub style: ProjectBrowserStyle,
    pub items: Vec<ProjectBrowserItem>,

    // State
    hovered_index: Option<usize>,
    scroll_offset: f32,
    max_scroll: f32,
    render_id: Uuid,
}

impl ProjectBrowser {
    pub fn new(style: ProjectBrowserStyle) -> Self {
        Self {
            id: String::new(),
            style,
            items: Vec::new(),
            hovered_index: None,
            scroll_offset: 0.0,
            max_scroll: 0.0,
            render_id: Uuid::new_v4(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_items(mut self, items: Vec<ProjectBrowserItem>) -> Self {
        self.items = items;
        self.update_scroll_max();
        self
    }

    pub fn add_item(&mut self, item: ProjectBrowserItem) {
        self.items.push(item);
        self.update_scroll_max();
    }

    pub fn clear_items(&mut self) {
        self.items.clear();
        self.scroll_offset = 0.0;
        self.max_scroll = 0.0;
    }

    pub fn set_items(&mut self, items: Vec<ProjectBrowserItem>) {
        self.items = items;
        self.update_scroll_max();
    }

    fn update_scroll_max(&mut self) {
        let [_, _, _, h] = self.style.rect;
        let rows = (self.items.len() + self.style.columns - 1) / self.style.columns;
        let total_height = self.style.padding * 2.0
            + rows as f32 * (self.style.cell_height + self.style.spacing)
            - self.style.spacing;
        self.max_scroll = (total_height - h).max(0.0);
    }

    fn get_cell_rect(&self, index: usize) -> [f32; 4] {
        let [x, y, _, _] = self.style.rect;
        let row = index / self.style.columns;
        let col = index % self.style.columns;

        let cell_x =
            x + self.style.padding + col as f32 * (self.style.cell_width + self.style.spacing);
        let cell_y =
            y + self.style.padding + row as f32 * (self.style.cell_height + self.style.spacing)
                - self.scroll_offset;

        [
            cell_x,
            cell_y,
            self.style.cell_width,
            self.style.cell_height,
        ]
    }

    fn hit_test(&self, pos: [f32; 2]) -> Option<usize> {
        let [x, y, w, h] = self.style.rect;
        if pos[0] < x || pos[0] > x + w || pos[1] < y || pos[1] > y + h {
            return None;
        }

        for (i, _) in self.items.iter().enumerate() {
            let [cx, cy, cw, ch] = self.get_cell_rect(i);

            // Check if cell is visible
            if cy + ch < y || cy > y + h {
                continue;
            }

            if pos[0] >= cx && pos[0] <= cx + cw && pos[1] >= cy && pos[1] <= cy + ch {
                return Some(i);
            }
        }

        None
    }
}

impl UiView for ProjectBrowser {
    fn build(&mut self, ctx: &mut ViewContext) {
        let [x, y, w, h] = self.style.rect;

        // Background
        ctx.push(Drawable::Rect {
            id: self.render_id,
            rect: [x, y, w, h],
            fill: self.style.background,
            border: self.style.border,
            radius_px: self.style.radius_px,
            border_px: self.style.border_px,
            layer: self.style.layer,
        });

        // Cells
        for (i, item) in self.items.iter().enumerate() {
            let cell_rect = self.get_cell_rect(i);
            let [cx, cy, cw, ch] = cell_rect;

            // Skip cells outside visible area
            if cy + ch < y || cy > y + h {
                continue;
            }

            // Cell background
            let cell_bg = if self.hovered_index == Some(i) {
                self.style.cell_hover_background
            } else {
                self.style.cell_background
            };

            ctx.push(Drawable::Rect {
                id: Uuid::new_v4(),
                rect: [cx, cy, cw, ch],
                fill: cell_bg,
                border: self.style.cell_border,
                radius_px: self.style.cell_radius_px,
                border_px: self.style.cell_border_px,
                layer: self.style.layer + 1,
            });

            // Thumbnail
            if let Some(tile_id) = item.thumbnail_tile {
                let thumb_padding = self.style.thumbnail_padding;
                let thumb_height = ch * 0.7; // 70% of cell for thumbnail
                let thumb_width = cw - thumb_padding * 2.0;

                ctx.push(Drawable::Quad {
                    id: Uuid::new_v4(),
                    tile_id,
                    rect: [
                        cx + thumb_padding,
                        cy + thumb_padding,
                        thumb_width,
                        thumb_height - thumb_padding,
                    ],
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    layer: self.style.layer + 2,
                    tint: Vec4::new(1.0, 1.0, 1.0, 1.0),
                });
            }

            // Project name (centered text)
            let text_y = cy + ch * 0.7 + 8.0;

            ctx.push(Drawable::Text {
                id: Uuid::new_v4(),
                text: item.name.clone(),
                origin: [cx + cw * 0.5, text_y],
                px_size: self.style.text_size,
                color: self.style.text_color,
                layer: self.style.layer + 3,
            });

            // Subtitle (if present)
            if let Some(subtitle) = &item.subtitle {
                let subtitle_y = text_y + self.style.text_size + 4.0;
                let subtitle_color = Vec4::new(
                    self.style.text_color.x * 0.7,
                    self.style.text_color.y * 0.7,
                    self.style.text_color.z * 0.7,
                    self.style.text_color.w * 0.8,
                );

                ctx.push(Drawable::Text {
                    id: Uuid::new_v4(),
                    text: subtitle.clone(),
                    origin: [cx + cw * 0.5, subtitle_y],
                    px_size: self.style.text_size * 0.8,
                    color: subtitle_color,
                    layer: self.style.layer + 3,
                });
            }
        }
    }

    fn handle_event(&mut self, evt: &UiEvent) -> UiEventOutcome {
        match evt.kind {
            UiEventKind::PointerMove => {
                let new_hover = self.hit_test(evt.pos);
                if new_hover != self.hovered_index {
                    self.hovered_index = new_hover;
                    return UiEventOutcome::dirty();
                }
            }
            UiEventKind::PointerDown => {
                if let Some(index) = self.hit_test(evt.pos) {
                    if let Some(item) = self.items.get(index) {
                        return UiEventOutcome::action(UiAction::Custom {
                            source_id: self.id.clone(),
                            action: format!("project_selected:{}", item.id),
                        });
                    }
                }
            }
            UiEventKind::Scroll { delta } => {
                let [x, y, w, h] = self.style.rect;
                if evt.pos[0] >= x && evt.pos[0] <= x + w && evt.pos[1] >= y && evt.pos[1] <= y + h
                {
                    self.scroll_offset =
                        (self.scroll_offset - delta[1] * 20.0).clamp(0.0, self.max_scroll);
                    return UiEventOutcome::redraw();
                }
            }
            _ => {}
        }

        UiEventOutcome::none()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn view_id(&self) -> &str {
        &self.id
    }
}
