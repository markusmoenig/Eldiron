use super::*;

/// Rendering backend: screen-space logical coordinates. Backends own device density,
/// clipping and asset resolution. No dependency on the existing widget hierarchy.
pub trait GraphPainter {
    fn round_rect(&mut self, rect: GraphRect, radius: f32, color: GraphColor);
    fn text_width(&mut self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.6
    }
    fn text(&mut self, rect: GraphRect, text: &str, size: f32, color: GraphColor);
    fn curve(&mut self, points: [Point; 4], width: f32, color: GraphColor);
    fn preview(&mut self, rect: GraphRect, asset: &str);
}
pub struct GraphTheme {
    pub background: GraphColor,
    pub grid: GraphColor,
    pub body: GraphColor,
    pub control: GraphColor,
    pub text: GraphColor,
    pub muted: GraphColor,
    pub wire: GraphColor,
    pub active: GraphColor,
    pub yes: GraphColor,
    pub no: GraphColor,
}
impl Default for GraphTheme {
    fn default() -> Self {
        Self {
            background: [27, 28, 29, 255],
            grid: [38, 39, 40, 255],
            body: [35, 36, 37, 255],
            control: [76, 77, 78, 255],
            text: [235, 235, 231, 255],
            muted: [163, 169, 166, 255],
            wire: [146, 156, 156, 255],
            active: [94, 210, 247, 255],
            yes: [48, 179, 130, 255],
            no: [213, 81, 81, 255],
        }
    }
}
impl GraphEditor {
    pub fn paint(
        &self,
        doc: &GraphDocument,
        context: &dyn GraphContext,
        controls: &dyn GraphControls,
        painter: &mut dyn GraphPainter,
        size: Point,
        theme: &GraphTheme,
    ) {
        painter.round_rect(
            GraphRect {
                origin: [0., 0.],
                size,
            },
            0.,
            theme.background,
        );
        let z = self.viewport.zoom();
        let spacing = 32. * z;
        let mut x = self.viewport.pan[0].rem_euclid(spacing);
        while x < size[0] {
            painter.round_rect(
                GraphRect {
                    origin: [x, 0.],
                    size: [1., size[1]],
                },
                0.,
                theme.grid,
            );
            x += spacing;
        }
        let mut y = self.viewport.pan[1].rem_euclid(spacing);
        while y < size[1] {
            painter.round_rect(
                GraphRect {
                    origin: [0., y],
                    size: [size[0], 1.],
                },
                0.,
                theme.grid,
            );
            y += spacing;
        }
        let owners: std::collections::HashMap<GraphId, GraphId> = doc
            .nodes
            .iter()
            .flat_map(|n| n.ports.iter().map(move |p| (p.id, n.id)))
            .collect();
        for c in &doc.connections {
            // A connection touching a hidden node belongs to another branch.
            if let (Some(from), Some(to)) = (owners.get(&c.from), owners.get(&c.to))
                && (!self.node_visible(*from) || !self.node_visible(*to))
            {
                continue;
            }
            if let Some(points) = connection_curve(doc, c, &self.viewport) {
                let active =
                    context.connection_active(c.id) || self.selected_connection == Some(c.id);
                painter.curve(
                    points,
                    if active { 3. } else { 1.8 } * z,
                    if active { theme.active } else { theme.wire },
                );
            }
        }
        let screen = |rect: GraphRect| GraphRect {
            origin: self.viewport.to_screen(rect.origin),
            size: [rect.size[0] * z, rect.size[1] * z],
        };
        let metrics = doc.metrics();
        for n in &doc.nodes {
            if !self.node_visible(n.id) {
                continue;
            }
            let r = screen(n.rect(&metrics));
            if r.origin[0] + r.size[0] < 0.
                || r.origin[1] + r.size[1] < 0.
                || r.origin[0] > size[0]
                || r.origin[1] > size[1]
            {
                continue;
            }
            let obs = context.observe(n);
            let diagnostic = context.diagnostic(n);
            // Context owns the border; execution/selection use a separate outer ring.
            let border = match obs.condition {
                GraphCondition::True => theme.yes,
                GraphCondition::False => theme.no,
                GraphCondition::Unknown => [76, 79, 78, 255],
            };
            if diagnostic.is_some()
                || context.node_active(n.id)
                || obs.execution != GraphExecution::Idle
                || self.selected == Some(n.id)
            {
                painter.round_rect(
                    GraphRect {
                        origin: [r.origin[0] - 3. * z, r.origin[1] - 3. * z],
                        size: [r.size[0] + 6. * z, r.size[1] + 6. * z],
                    },
                    22. * z,
                    if diagnostic.is_some() || obs.execution == GraphExecution::Failed {
                        theme.no
                    } else if self.selected == Some(n.id) && !context.node_active(n.id) {
                        [228, 218, 183, 255]
                    } else {
                        theme.active
                    },
                );
            }
            painter.round_rect(r, 19. * z, border);
            painter.round_rect(
                GraphRect {
                    origin: [r.origin[0] + 2. * z, r.origin[1] + 2. * z],
                    size: [r.size[0] - 4. * z, r.size[1] - 4. * z],
                },
                18. * z,
                n.color,
            );
            let band = metrics.title_band;
            // A folded node only needs a body strip when it has terminals to
            // spread across it; otherwise it would show a stray empty sliver.
            if !n.folded || n.terminals() > 0 {
                painter.round_rect(
                    GraphRect {
                        origin: [r.origin[0] + 2. * z, r.origin[1] + band * z],
                        size: [r.size[0] - 4. * z, (r.size[1] - (band + 2.) * z).max(0.)],
                    },
                    17. * z,
                    theme.body,
                );
            }
            let title = context.node_title(n).unwrap_or_else(|| n.title.clone());
            let title_size = metrics.title_size * z;
            let title_width = (n.width - 44.) * z;
            painter.text(
                screen(GraphRect {
                    origin: [n.position[0] + 16., n.position[1] + 7.],
                    size: [n.width - 30., band],
                }),
                &fit_text(&title, title_width, title_size, controls),
                title_size,
                theme.text,
            );
            // A chevron marks the fold handle in the title bar.
            if !n.rows.is_empty() {
                painter.text(
                    screen(GraphRect {
                        origin: [n.position[0] + n.width - 24., n.position[1] + 6.],
                        size: [16., band - 8.],
                    }),
                    if n.folded { ">" } else { "v" },
                    title_size,
                    theme.muted,
                );
            }
            // A folded node keeps its rows in the document, it just stops
            // drawing them, so a branch can be read at a glance.
            let rows: &[GraphRow] = if n.folded { &[] } else { &n.rows };
            for (i, row) in rows.iter().enumerate() {
                let rect = n.row_rect(i, &metrics);
                let row_size = metrics.label_size * z;
                painter.text(
                    screen(GraphRect {
                        origin: [rect.origin[0], rect.origin[1] + metrics.label_offset()],
                        size: [rect.size[0], metrics.label - 4.],
                    }),
                    &fit_text(&row.label, rect.size[0] * z, row_size, controls),
                    row_size,
                    theme.muted,
                );
                painter.round_rect(screen(rect), 15. * z, theme.control);
                if row.value_type.is_some() {
                    let button = screen(GraphRect {
                        origin: [
                            rect.origin[0] + rect.size[0] - 25.,
                            rect.origin[1] + metrics.label_offset(),
                        ],
                        size: [25., metrics.label - 4.],
                    });
                    painter.text(button, "[=]", row_size, theme.active);
                }
                if row.binding.is_some() {
                    let label = context
                        .row_label(n, row)
                        .unwrap_or_else(|| "Bound value".into());
                    painter.text(
                        screen(GraphRect {
                            origin: [rect.origin[0] + 9., rect.origin[1] + 4.],
                            size: [rect.size[0] - 18., 22.],
                        }),
                        &fit_text(&label, (rect.size[0] - 18.) * z, row_size, controls),
                        row_size,
                        theme.active,
                    );
                    continue;
                }
                if controls.paint(&row.value, screen(rect), painter) {
                    continue;
                }
                if let GraphControlValue::List { columns, rows } = &row.value {
                    let count = columns.len().max(1);
                    let cell_width = rect.size[0] * LIST_DELETE_FRACTION / count as f32;
                    let cell_size = metrics.cell_size * z;
                    let header_size = metrics.header_size * z;
                    // Text is centred in its list row, so compact rows place it
                    // closer to the row's top.
                    let text_offset = ((metrics.list_row - 18.) * 0.5).max(0.);
                    for (column, definition) in columns.iter().enumerate() {
                        painter.text(
                            screen(GraphRect {
                                origin: [
                                    rect.origin[0] + cell_width * column as f32 + 6.,
                                    rect.origin[1] + 4.,
                                ],
                                size: [cell_width - 10., metrics.list_header - 8.],
                            }),
                            &fit_text(
                                &definition.label,
                                (cell_width - 16.) * z,
                                header_size,
                                controls,
                            ),
                            header_size,
                            theme.muted,
                        );
                    }
                    for (index, cells) in rows.iter().enumerate() {
                        let y = rect.origin[1]
                            + metrics.list_header
                            + metrics.list_row * index as f32;
                        for (column, cell) in cells.iter().enumerate() {
                            let cell_rect = GraphRect {
                                origin: [
                                    rect.origin[0] + cell_width * column as f32 + 6.,
                                    y + text_offset,
                                ],
                                size: [cell_width - 10., 18.],
                            };
                            painter.text(
                                screen(cell_rect),
                                &fit_text(
                                    &BasicGraphControls.label(cell),
                                    (cell_width - 16.) * z,
                                    cell_size,
                                    controls,
                                ),
                                cell_size,
                                theme.text,
                            );
                            if self.text_focus().is_some_and(|focus| {
                                focus.node == n.id
                                    && focus.row == row.id
                                    && focus.cell == Some((index, column))
                            }) {
                                painter.round_rect(
                                    screen(GraphRect {
                                        origin: [
                                            cell_rect.origin[0],
                                            cell_rect.origin[1] + cell_rect.size[1] - 1.,
                                        ],
                                        size: [cell_rect.size[0], 2.],
                                    }),
                                    z,
                                    theme.active,
                                );
                            }
                        }
                        painter.text(
                            screen(GraphRect {
                                origin: [
                                    rect.origin[0] + rect.size[0] - 20.,
                                    y + text_offset,
                                ],
                                size: [16., 18.],
                            }),
                            "x",
                            cell_size,
                            theme.muted,
                        );
                    }
                    let y = rect.origin[1]
                        + metrics.list_header
                        + metrics.list_row * rows.len() as f32;
                    painter.text(
                        screen(GraphRect {
                            origin: [rect.origin[0] + 6., y + text_offset],
                            size: [rect.size[0] - 12., 18.],
                        }),
                        "+ Add",
                        cell_size,
                        theme.active,
                    );
                    continue;
                }
                if let GraphControlValue::Number {
                    value, min, max, ..
                } = &row.value
                {
                    let fraction = if max > min {
                        ((value - min) / (max - min)).clamp(0., 1.)
                    } else {
                        0.
                    };
                    painter.round_rect(
                        screen(GraphRect {
                            origin: rect.origin,
                            size: [rect.size[0] * fraction, rect.size[1]],
                        }),
                        15. * z,
                        [92, 106, 103, 255],
                    );
                }
                if let GraphControlValue::Preview { asset, .. } = &row.value {
                    painter.preview(screen(rect), asset);
                }
                let text_size = metrics.text_size * z;
                let text_rect = screen(GraphRect {
                    origin: [rect.origin[0] + 9., rect.origin[1] + 4.],
                    size: [rect.size[0] - 18., 22.],
                });
                if let GraphControlValue::Text(text) = &row.value {
                    if let Some(focus) = self
                        .text_focus()
                        .filter(|f| f.node == n.id && f.row == row.id)
                    {
                        painter.round_rect(
                            screen(GraphRect {
                                origin: [rect.origin[0], rect.origin[1] + rect.size[1] - 2.],
                                size: [rect.size[0], 2.],
                            }),
                            z,
                            theme.active,
                        );
                        let mut start = focus.caret();
                        let mut width = 0.;
                        for (index, ch) in text[..focus.caret()].char_indices().rev() {
                            let w = painter.text_width(&ch.to_string(), text_size);
                            if width + w > text_rect.size[0] - 3. * z {
                                break;
                            }
                            width += w;
                            start = index;
                        }
                        let selection = focus.selection();
                        let sx = painter
                            .text_width(&text[start..selection.start.max(start)], text_size)
                            .min(text_rect.size[0]);
                        let ex = painter
                            .text_width(&text[start..selection.end.max(start)], text_size)
                            .min(text_rect.size[0]);
                        painter.round_rect(
                            GraphRect {
                                origin: [text_rect.origin[0] + sx, text_rect.origin[1]],
                                size: [ex - sx, text_rect.size[1]],
                            },
                            0.,
                            [42, 112, 134, 255],
                        );
                        painter.text(text_rect, &text[start..], text_size, theme.text);
                        let cx = painter.text_width(&text[start..focus.caret()], text_size);
                        painter.round_rect(
                            GraphRect {
                                origin: [text_rect.origin[0] + cx, text_rect.origin[1]],
                                size: [z.max(1.), text_rect.size[1]],
                            },
                            0.,
                            theme.text,
                        );
                        continue;
                    }
                }
                let label = context
                    .row_label(n, row)
                    .unwrap_or_else(|| controls.label(&row.value));
                painter.text(
                    screen(GraphRect {
                        origin: [rect.origin[0] + 9., rect.origin[1] + 4.],
                        size: [rect.size[0] - 18., 22.],
                    }),
                    &fit_text(&label, text_rect.size[0], text_size, controls),
                    text_size,
                    theme.text,
                );
            }
            let color = match obs.condition {
                GraphCondition::True => theme.yes,
                GraphCondition::False => theme.no,
                GraphCondition::Unknown => theme.muted,
            };
            let status = if let Some(error) = diagnostic {
                format!("Invalid: {error}")
            } else if obs.execution == GraphExecution::Idle {
                obs.text
            } else {
                format!("{:?} · {}", obs.execution, obs.text)
            };
            // A folded node has no body, so its status line would sit on the
            // title; execution state still shows in the border ring.
            if !n.folded {
                painter.text(
                    screen(GraphRect {
                        origin: [
                            n.position[0] + 16.,
                            n.position[1] + n.height(&metrics) - 22.,
                        ],
                        size: [n.width - 30., 16.],
                    }),
                    &status,
                    metrics.header_size * z,
                    color,
                );
            }
            for p in &n.ports {
                let center = self.viewport.to_screen(n.port_position(p, &metrics));
                painter.round_rect(
                    GraphRect {
                        origin: [center[0] - 5. * z, center[1] - 5. * z],
                        size: [10. * z, 10. * z],
                    },
                    5. * z,
                    theme.wire,
                );
                // Edge labels outside the body keep parameter controls unobstructed.
                let (origin, width) = match p.side {
                    PortSide::Right => ([center[0] + 9. * z, center[1] - 15. * z], 105. * z),
                    PortSide::Left => ([center[0] - 76. * z, center[1] - 15. * z], 67. * z),
                    PortSide::Top => ([center[0] + 9. * z, center[1] - 20. * z], 100. * z),
                    PortSide::Bottom => ([center[0] + 9. * z, center[1] + 5. * z], 100. * z),
                };
                painter.text(
                    GraphRect {
                        origin,
                        size: [width, 16. * z],
                    },
                    &p.label,
                    metrics.port_size * z,
                    theme.muted,
                );
            }
        }
        if let Some(id) = self.pending_wire() {
            if let Some((n, p)) = doc.port(id) {
                painter.curve(
                    port_curve(
                        self.viewport.to_screen(n.port_position(p, &metrics)),
                        p.side,
                        self.cursor,
                        PortSide::Left,
                    ),
                    2. * z,
                    theme.active,
                );
            }
        }
    }
}

/// Trim `text` to fit `width` at `size`, ending with an ellipsis. Long dialogue
/// lines then show their start instead of widening the node.
fn fit_text(text: &str, width: f32, size: f32, controls: &dyn GraphControls) -> String {
    if width <= 0. || controls.text_width(text, size) <= width {
        return text.to_string();
    }
    let mut cut = text.to_string();
    while !cut.is_empty() && controls.text_width(&format!("{cut}…"), size) > width {
        cut.pop();
    }
    format!("{cut}…")
}
