//! Host-owned tile assets and a node-native visual selector.
use super::*;

#[derive(Default)]
pub(super) struct TileControls {
    images: HashMap<String, TheRGBABuffer>,
    names: HashMap<String, String>,
    items: Vec<GraphPickerItem>,
}
impl TileControls {
    pub fn refresh(&mut self, project: &Project) {
        self.images.clear();
        self.names.clear();
        self.items = vec![GraphPickerItem {
            id: String::new(),
            label: fl!("entity_inherit"),
        }];
        let generated: std::collections::HashSet<_> = project
            .procedural_recipes
            .values()
            .filter_map(|recipe| recipe.tile_id)
            .collect();
        for (id, tile) in &project.tiles {
            if generated.contains(id) || tile.module.is_some() || tile.textures.is_empty() {
                continue;
            }
            let id = id.to_string();
            let label = if tile.alias.trim().is_empty() {
                fl!("entity_tile")
            } else {
                tile.alias.clone()
            };
            if let Some(texture) = tile.textures.first() {
                self.images.insert(
                    id.clone(),
                    TheRGBABuffer::from(
                        texture.data.clone(),
                        texture.width as u32,
                        texture.height as u32,
                    ),
                );
            }
            // Existing authored aliases remain valid; picker selections use stable IDs.
            if !tile.alias.trim().is_empty() {
                self.names.insert(tile.alias.clone(), label.clone());
                if let Some(image) = self.images.get(&id).cloned() {
                    self.images.insert(tile.alias.clone(), image);
                }
            }
            self.names.insert(id.clone(), label.clone());
            self.items.push(GraphPickerItem { id, label });
        }
    }
}
impl GraphAssets for TileControls {
    fn image(&self, key: &str) -> Option<&TheRGBABuffer> {
        self.images.get(key)
    }
}
impl GraphControls for TileControls {
    fn label(&self, value: &GraphControlValue) -> String {
        if let GraphControlValue::Custom { kind, data } = value {
            if kind == "tile" {
                let id = data.as_str().unwrap_or_default();
                return if id.is_empty() {
                    fl!("entity_select_tile")
                } else {
                    self.names
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| fl!("entity_missing_tile"))
                };
            }
        }
        BasicGraphControls.label(value)
    }
    fn interact(
        &self,
        value: &GraphControlValue,
        input: GraphControlInput,
    ) -> Option<GraphControlValue> {
        BasicGraphControls.interact(value, input)
    }
    fn paint(
        &self,
        value: &GraphControlValue,
        rect: GraphRect,
        painter: &mut dyn GraphPainter,
    ) -> bool {
        let GraphControlValue::Custom { kind, data } = value else {
            return false;
        };
        if kind != "tile" {
            return false;
        }
        let size = (rect.size[1] - 4.).max(1.).min(32.);
        let id = data.as_str().unwrap_or_default();
        let mut offset = 8.;
        if self.images.contains_key(id) {
            painter.preview(
                GraphRect {
                    origin: [
                        rect.origin[0] + 4.,
                        rect.origin[1] + (rect.size[1] - size) * 0.5,
                    ],
                    size: [size, size],
                },
                id,
            );
            offset = size + 10.;
        }
        painter.text(
            GraphRect {
                origin: [rect.origin[0] + offset, rect.origin[1]],
                size: [(rect.size[0] - offset - 6.).max(1.), rect.size[1]],
            },
            &format!("{} …", self.label(value)),
            13.,
            [233, 235, 230, 255],
        );
        true
    }
}
impl NodesDock {
    pub(super) fn open_tile_picker(
        &mut self,
        point: [f32; 2],
        ui: &mut TheUI,
        project: &Project,
    ) -> bool {
        let point = self.editor.viewport.to_graph(point);
        let metrics = self.doc.metrics();
        let target = self.doc.nodes.iter().rev().filter(|n| !n.folded && self.editor.node_visible(n.id)).find_map(|node| {
            node.rows.iter().enumerate().find_map(|(index, row)| {
                if matches!(&row.value, GraphControlValue::Custom { kind, .. } if kind == "tile") && node.row_rect(index, &metrics).contains(point) {
                    Some((TextTarget { node: node.id, row: row.id, cell: None }, node.row_rect(index, &metrics)))
                } else { None }
            })
        });
        let Some((target, row)) = target else {
            return false;
        };
        let Some(view) = ui.get_render_view(VIEW) else {
            return false;
        };
        let dim = *view.dim();
        self.tiles.refresh(project);
        let rect = GraphRect {
            origin: self.editor.viewport.to_screen(row.origin),
            size: [
                row.size[0] * self.editor.viewport.zoom(),
                row.size[1] * self.editor.viewport.zoom(),
            ],
        };
        let mut picker = GraphPicker::grid(
            rect,
            [dim.width as f32, dim.height as f32],
            self.tiles
                .items
                .iter()
                .filter(|item| self.is_configuration() || !item.id.is_empty())
                .cloned()
                .collect(),
        );
        picker.previews = self
            .tiles
            .images
            .keys()
            .map(|id| (id.clone(), id.clone()))
            .collect();
        picker.search_label = fl!("node_search");
        picker.empty_label = fl!("node_no_results");
        self.choice_target = Some(target);
        self.popup = Some((picker, None));
        true
    }
}
