//! Host-owned tile assets and a node-native visual selector.
use super::*;

#[derive(Default)]
pub(super) struct TileControls {
    images: HashMap<String, TheRGBABuffer>,
    names: HashMap<String, String>,
    items: Vec<GraphPickerItem>,
}
impl TileControls {
    pub(super) fn set_preview(&mut self, key: &str, image: Option<TheRGBABuffer>) {
        if let Some(image) = image {
            self.images.insert(key.into(), image);
        } else {
            self.images.remove(key);
        }
    }

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
        // Embedded artwork supplies previews for semantic rules icon IDs.
        if let Ok(textures) = shared::rulesets::bundled_textures_for_project(&project.config) {
            for (id, texture) in textures {
                self.images.insert(
                    id.into(),
                    TheRGBABuffer::from(texture.data, texture.width as u32, texture.height as u32),
                );
                self.names.insert(id.into(), id.replace('_', " "));
            }
        }
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
        if let Ok(rules) = project.rules.current.compile() {
            if let Some(icons) = rules.get("icons").and_then(|v| v.as_table()) {
                for (id, icon) in icons {
                    if let Some(texture) = icon.get("texture").and_then(|v| v.as_str()) {
                        if !texture.trim().is_empty() {
                            if let Some(image) = self.images.get(texture).cloned() {
                                self.images.insert(id.clone(), image);
                            } else {
                                self.images.remove(id);
                            }
                        }
                    }
                }
            }
        }
        self.items[1..].sort_by(|a, b| a.label.cmp(&b.label).then(a.id.cmp(&b.id)));
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
            if matches!(kind.as_str(), "tile" | "rules_icon") {
                let id = data.as_str().unwrap_or_default();
                return if id.is_empty() {
                    fl!("entity_select_tile")
                } else if kind == "rules_icon" && !self.images.contains_key(id) {
                    format!("Missing: {id}")
                } else {
                    self.names.get(id).cloned().unwrap_or_else(|| {
                        if kind == "rules_icon" {
                            format!("Missing: {id}")
                        } else {
                            fl!("entity_missing_tile")
                        }
                    })
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
    fn paint_scaled(
        &self,
        value: &GraphControlValue,
        rect: GraphRect,
        scale: f32,
        painter: &mut dyn GraphPainter,
    ) -> bool {
        if let GraphControlValue::Text(raw) = value {
            if let Ok(color) = shared::rulesets::particle_graph::parse_color(raw) {
                let size = (rect.size[1] - 8. * scale).min(24. * scale);
                painter.round_rect(
                    GraphRect {
                        origin: [
                            rect.origin[0] + 4. * scale,
                            rect.origin[1] + (rect.size[1] - size) * 0.5,
                        ],
                        size: [size, size],
                    },
                    2. * scale,
                    color,
                );
                painter.text(
                    GraphRect {
                        origin: [rect.origin[0] + size + 10. * scale, rect.origin[1]],
                        size: [(rect.size[0] - size - 12. * scale).max(scale), rect.size[1]],
                    },
                    raw,
                    13. * scale,
                    [233, 235, 230, 255],
                );
                return true;
            }
        }
        let GraphControlValue::Custom { kind, data } = value else {
            return false;
        };
        if !matches!(kind.as_str(), "tile" | "rules_icon") {
            return false;
        }
        let size = (rect.size[1] - 4. * scale).max(scale).min(32. * scale);
        let id = data.as_str().unwrap_or_default();
        let mut offset = 8. * scale;
        if self.images.contains_key(id) {
            painter.preview(
                GraphRect {
                    origin: [
                        rect.origin[0] + 4. * scale,
                        rect.origin[1] + (rect.size[1] - size) * 0.5,
                    ],
                    size: [size, size],
                },
                id,
            );
            offset = size + 10. * scale;
        }
        painter.text(
            GraphRect {
                origin: [rect.origin[0] + offset, rect.origin[1]],
                size: [
                    (rect.size[0] - offset - 6. * scale).max(scale),
                    rect.size[1],
                ],
            },
            &format!("{} …", self.label(value)),
            13. * scale,
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
        let visual = |value: &GraphControlValue| matches!(value, GraphControlValue::Custom { kind, .. } if matches!(kind.as_str(), "tile" | "rules_icon"));
        let target = self
            .doc
            .nodes
            .iter()
            .rev()
            .filter(|n| !n.folded && self.editor.node_visible(n.id))
            .find_map(|node| {
                node.rows.iter().enumerate().find_map(|(index, row)| {
                    let rect = node.row_rect(index, &metrics);
                    if visual(&row.value) && rect.contains(point) {
                        return Some((
                            TextTarget {
                                node: node.id,
                                row: row.id,
                                cell: None,
                            },
                            rect,
                        ));
                    }
                    if let GraphControlValue::List { columns, rows, .. } = &row.value {
                        let width =
                            rect.size[0] * LIST_DELETE_FRACTION / columns.len().max(1) as f32;
                        for (r, cells) in rows.iter().enumerate() {
                            for (c, value) in cells.iter().enumerate() {
                                let cell = GraphRect {
                                    origin: [
                                        rect.origin[0] + width * c as f32,
                                        rect.origin[1]
                                            + metrics.list_header
                                            + metrics.list_row * r as f32,
                                    ],
                                    size: [width, metrics.list_row],
                                };
                                if visual(value) && cell.contains(point) {
                                    return Some((
                                        TextTarget {
                                            node: node.id,
                                            row: row.id,
                                            cell: Some((r, c)),
                                        },
                                        cell,
                                    ));
                                }
                            }
                        }
                    }
                    None
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
                .filter(|item| self.is_configuration() || self.is_rules() || !item.id.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_icon_preview_picker_replacement_roundtrip_and_undo() {
        let mut project = Project::new();
        let mut tile =
            rusterix::Tile::from_texture(rusterix::Texture::new(vec![24, 80, 140, 255], 1, 1));
        tile.alias = "Robot action".into();
        let tile_id = tile.id;
        project.tiles.insert(tile_id, tile);
        let mut ctx = TheContext::new(1200, 650, 1.);
        let mut ui = TheUI::new();
        let mut dock = NodesDock::new();
        ui.canvas = dock.setup(&mut ctx);
        ui.get_render_view(VIEW)
            .unwrap()
            .set_dim(TheDim::new(0, 0, 1200, 650), &mut ctx);
        let mut server = ServerContext::default();
        server.pc = ProjectContext::GameRules;
        dock.activate(&mut ui, &mut ctx, &project, &mut server);
        let root = dock
            .branch_order
            .iter()
            .copied()
            .find(|id| {
                shared::rulesets::graph::branch_path(&dock.branch_graphs[id])
                    == Some("/actions/basic_attack")
            })
            .unwrap();
        dock.set_branch(Some(root));
        let before = project.rules.current.compile().unwrap();
        assert!(dock.tiles.image("basic_attack").is_some());
        let (node, row_index, index) = dock
            .doc
            .nodes
            .iter()
            .find_map(|n| {
                n.rows
                    .iter()
                    .enumerate()
                    .find_map(|(r, row)| match &row.value {
                        GraphControlValue::List { rows, .. } => rows
                            .iter()
                            .position(|cells| {
                                cells.first() == Some(&GraphControlValue::Text("icon".into()))
                            })
                            .map(|i| (n, r, i)),
                        _ => None,
                    })
            })
            .unwrap();
        let rect = node.row_rect(row_index, &dock.doc.metrics());
        let width = rect.size[0] * LIST_DELETE_FRACTION / 3.;
        let point = dock.editor.viewport.to_screen([
            rect.origin[0] + width * 2.5,
            rect.origin[1]
                + dock.doc.metrics().list_header
                + dock.doc.metrics().list_row * (index as f32 + 0.5),
        ]);
        assert!(dock.open_tile_picker(point, &mut ui, &project));
        assert_eq!(dock.choice_target.unwrap().cell, Some((index, 2)));
        assert!(
            dock.popup
                .as_ref()
                .unwrap()
                .0
                .previews
                .contains_key(&tile_id.to_string())
        );
        dock.pick(
            GraphPickerItem {
                id: tile_id.to_string(),
                label: "Robot action".into(),
            },
            None,
        );
        dock.finish(&mut project);
        let resolved = project.rules.current.resolve().unwrap();
        assert_eq!(resolved.validation().error_count(), 0);
        assert_eq!(
            project.rules.current.compile().unwrap()["actions"]["basic_attack"]["ui"]["icon"]
                .as_str(),
            Some(tile_id.to_string().as_str())
        );
        let saved: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        assert_eq!(
            saved.rules.current.compile().unwrap(),
            project.rules.current.compile().unwrap()
        );
        dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(project.rules.current.compile().unwrap(), before);
        dock.redo(&mut ui, &mut ctx, &mut project, &mut server);
        assert!(project.rules_source().is_ok());
        let root = dock
            .branch_order
            .iter()
            .copied()
            .find(|id| {
                shared::rulesets::graph::branch_path(&dock.branch_graphs[id])
                    == Some("/icons/basic_attack")
            })
            .unwrap();
        dock.set_branch(Some(root));
        let node = dock.doc.nodes.iter().find(|n| n.id == root).unwrap();
        let row = node
            .rows
            .iter()
            .position(|r| r.key.as_deref() == Some("rules_icon_preview"))
            .unwrap();
        let rect = node.row_rect(row, &dock.doc.metrics());
        let point = dock
            .editor
            .viewport
            .to_screen([rect.origin[0] + 20., rect.origin[1] + 20.]);
        assert!(dock.open_tile_picker(point, &mut ui, &project));
        dock.pick(
            GraphPickerItem {
                id: tile_id.to_string(),
                label: "Robot action".into(),
            },
            None,
        );
        dock.finish(&mut project);
        assert_eq!(
            project.rules.current.compile().unwrap()["icons"]["basic_attack"]["texture"].as_str(),
            Some(tile_id.to_string().as_str())
        );
        assert!(project.rules_source().is_ok());
        dock.tiles.refresh(&project);
        assert_eq!(
            dock.tiles.image("basic_attack").unwrap().pixels(),
            &[24, 80, 140, 255]
        );
        project.rules.restore_original();
        assert_eq!(project.rules.current.compile().unwrap(), before);
    }
}
