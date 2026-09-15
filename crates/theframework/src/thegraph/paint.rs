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
        for c in &doc.connections {
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
        for n in &doc.nodes {
            let r = screen(n.rect());
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
            painter.round_rect(
                GraphRect {
                    origin: [r.origin[0] + 2. * z, r.origin[1] + 35. * z],
                    size: [r.size[0] - 4. * z, r.size[1] - 37. * z],
                },
                17. * z,
                theme.body,
            );
            painter.text(
                screen(GraphRect {
                    origin: [n.position[0] + 16., n.position[1] + 7.],
                    size: [n.width - 30., 25.],
                }),
                &context.node_title(n).unwrap_or_else(|| n.title.clone()),
                18. * z,
                theme.text,
            );
            for (i, row) in n.rows.iter().enumerate() {
                let rect = n.row_rect(i);
                painter.text(
                    screen(GraphRect {
                        origin: [rect.origin[0], rect.origin[1] - 19.],
                        size: [rect.size[0], 18.],
                    }),
                    &row.label,
                    12. * z,
                    theme.muted,
                );
                painter.round_rect(screen(rect), 15. * z, theme.control);
                if row.value_type.is_some() {
                    let button = screen(GraphRect {
                        origin: [rect.origin[0] + rect.size[0] - 25., rect.origin[1] - 19.],
                        size: [25., 18.],
                    });
                    painter.text(button, "[=]", 12. * z, theme.active);
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
                        &label,
                        12. * z,
                        theme.active,
                    );
                    continue;
                }
                if controls.paint(&row.value, screen(rect), painter) {
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
                            let w = painter.text_width(&ch.to_string(), 14. * z);
                            if width + w > text_rect.size[0] - 3. * z {
                                break;
                            }
                            width += w;
                            start = index;
                        }
                        let selection = focus.selection();
                        let sx = painter
                            .text_width(&text[start..selection.start.max(start)], 14. * z)
                            .min(text_rect.size[0]);
                        let ex = painter
                            .text_width(&text[start..selection.end.max(start)], 14. * z)
                            .min(text_rect.size[0]);
                        painter.round_rect(
                            GraphRect {
                                origin: [text_rect.origin[0] + sx, text_rect.origin[1]],
                                size: [ex - sx, text_rect.size[1]],
                            },
                            0.,
                            [42, 112, 134, 255],
                        );
                        painter.text(text_rect, &text[start..], 14. * z, theme.text);
                        let cx = painter.text_width(&text[start..focus.caret()], 14. * z);
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
                    &label,
                    14. * z,
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
            painter.text(
                screen(GraphRect {
                    origin: [n.position[0] + 16., n.position[1] + n.height() - 22.],
                    size: [n.width - 30., 16.],
                }),
                &status,
                11. * z,
                color,
            );
            for p in &n.ports {
                let center = self.viewport.to_screen(n.port_position(p));
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
                    10. * z,
                    theme.muted,
                );
            }
        }
        if let Some(id) = self.pending_wire() {
            if let Some((n, p)) = doc.port(id) {
                painter.curve(
                    port_curve(
                        self.viewport.to_screen(n.port_position(p)),
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
