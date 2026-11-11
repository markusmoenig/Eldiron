//! Shared MaterialZ graph presentation and explicit CPU reference snapshots.
//! This module consumes read-only widget observations; it owns no input/session.
use super::*;

fn centered_text(
    p: &mut (impl GraphPainter + ?Sized),
    area: GraphRect,
    label: &str,
    size: f32,
    color: [u8; 4],
) {
    let mut visible = label.to_owned();
    if p.text_width(&visible, size) > area.size[0] {
        let ellipsis = "…";
        if p.text_width(ellipsis, size) > area.size[0] {
            return;
        }
        while !visible.is_empty()
            && p.text_width(&format!("{visible}{ellipsis}"), size) > area.size[0]
        {
            visible.pop();
        }
        visible.push_str(ellipsis);
    }
    let width = p.text_width(&visible, size);
    p.text(
        rect(
            area.origin[0] + (area.size[0] - width) / 2.,
            area.origin[1] + (area.size[1] - size) / 2.,
            width,
            size + 2.,
        ),
        &visible,
        size,
        color,
    );
}
fn empty_tile(p: &mut (impl GraphPainter + ?Sized), area: GraphRect, theme: &GraphTheme) {
    p.round_rect(area, 4., theme.control);
    p.round_rect(
        rect(
            area.origin[0] + 1.,
            area.origin[1] + 1.,
            area.size[0] - 2.,
            area.size[1] - 2.,
        ),
        3.,
        theme.background,
    );
}

impl MaterialEditorWidget {
    /// Explicit CPU snapshot/reference consumer. Native UI uses `paint_graph`
    /// with the ordered paint adapter and never allocates a panel framebuffer.
    pub fn paint(&mut self, buffer: &mut TheRGBABuffer, theme: &GraphTheme) {
        struct Images<'a>(&'a std::collections::HashMap<String, TheRGBABuffer>);
        impl GraphAssets for Images<'_> {
            fn image(&self, key: &str) -> Option<&TheRGBABuffer> {
                self.0.get(key)
            }
        }
        let mut resources = self.raster.take().unwrap_or_else(|| {
            let font =
                theframework::Embedded::get("fonts/Roboto-Bold.ttf").expect("framework UI font");
            GraphRasterResources::new(
                fontdue::Font::from_bytes(font.data.as_ref(), fontdue::FontSettings::default())
                    .expect("valid UI font"),
            )
        });
        let size = [buffer.dim().width as f32, buffer.dim().height as f32];
        let (width, height, density) = (
            buffer.pixel_width(),
            buffer.pixel_height(),
            buffer.render_scale(),
        );
        {
            let assets = Images(&self.thumbnails);
            let mut painter = RasterGraphPainter::new(
                buffer.pixels_mut(),
                width,
                height,
                density,
                &mut resources,
                &assets,
            );
            self.paint_graph(&mut painter, size, theme, false);
        }
        self.raster = Some(resources);
        self.dirty = false;
    }
    /// Renderer-independent row graph contribution, shared by native UI and
    /// explicit reference snapshots. Interaction geometry remains in `Layout`.
    pub(super) fn paint_graph(
        &self,
        p: &mut dyn GraphPainter,
        size: [f32; 2],
        theme: &GraphTheme,
        native_editors: bool,
    ) {
        let l = Layout::new(
            size,
            self.parameters().2.len(),
            &self.view.rows,
            self.view.geometry,
        );
        let (_, label, values) = self.parameters();
        let parameters = Some((label.to_owned(), values.to_vec()));
        let property_lines = if self.active_palette().is_some() {
            vec![]
        } else {
            self.property_lines()
        };
        let source_title = self
            .selected()
            .map_or("Select a tile to view its source".to_owned(), |op| {
                op.label.clone()
            });
        let source_document = self.selected().and_then(|op| op.source.clone());
        let source_area = self.source_area();
        let source_actions: Vec<_> = (0..4).map(|i| self.source_action_rect(i)).collect();
        let active_input = self.active_input().cloned();
        let input_labels: Vec<_> = (0..8).map(|i| self.input_control_label(i)).collect();
        let active_palette = self.active_palette();
        let palette_entries: Vec<_> = self
            .view
            .palette
            .iter()
            .skip(self.palette_scroll * self.palette_columns())
            .enumerate()
            .map(|(i, entry)| (self.palette_rect(i), entry.clone()))
            .collect();
        let mut connection_paths = self.connection_paths(&l);
        connection_paths.sort_by_key(|path| self.connection_selected(path));
        let connection_paths: Vec<_> = connection_paths
            .into_iter()
            .map(|path| {
                let style = self.connection_style(&path, theme);
                (path, style)
            })
            .collect();
        let popup = self.input_popup.and_then(|control| {
            self.popup_area()
                .map(|area| (area, self.input_options(control)))
        });
        let chrome = self.chrome.clone();
        p.round_rect(rect(0., 0., l.width, l.height), 0., theme.background);
        for (i, row) in self.view.rows.iter().enumerate() {
            let r = l.row(i, self.pan);
            if r.origin[1] + r.size[1] < HEADER || r.origin[1] > l.editor_bottom {
                continue;
            }
            if self.view.geometry {
                p.text(
                    rect(CARD_LEFT, r.origin[1] - 18., l.width - CARD_LEFT - 18., 16.),
                    &row.label,
                    10.5,
                    theme.text,
                );
            }
            let input = l.row_input(i, self.pan);
            p.round_rect(
                input,
                3.,
                if self.pane == MaterialPane::RowInput
                    && self.selected_row.as_ref() == Some(&row.id)
                {
                    theme.active
                } else {
                    theme.control
                },
            );
            for offset in [6., 10., 14.] {
                p.round_rect(
                    rect(input.origin[0] + 2., input.origin[1] + offset, 6., 1.),
                    0.,
                    theme.text,
                );
            }
            for (j, op) in row.operations.iter().enumerate() {
                let c = l.card(i, j, self.pan);
                if c.origin[1] + c.size[1] < HEADER
                    || c.origin[1] > l.editor_bottom
                    || c.origin[0] + c.size[0] > l.width - 18.
                {
                    continue;
                }
                p.round_rect(
                    c,
                    5.,
                    if self.selected.as_ref() == Some(&op.id) {
                        theme.active
                    } else {
                        theme.control
                    },
                );
                p.round_rect(
                    rect(
                        c.origin[0] + 1.,
                        c.origin[1] + 1.,
                        c.size[0] - 2.,
                        c.size[1] - 2.,
                    ),
                    4.,
                    theme.body,
                );
                let category = self
                    .view
                    .sources
                    .iter()
                    .find(|source| source.label == op.label)
                    .map_or("", |source| source.category.as_str());
                p.round_rect(
                    rect(c.origin[0] + 5., c.origin[1] + 2., c.size[0] - 10., 2.),
                    1.,
                    chrome.category(category),
                );
                if self.thumbnails.contains_key(&op.thumbnail) {
                    p.preview(
                        rect(
                            c.origin[0] + 3.,
                            c.origin[1] + 3.,
                            c.size[0] - 6.,
                            CARD - 25.,
                        ),
                        &op.thumbnail,
                    );
                } else {
                    p.text(
                        rect(c.origin[0] + 7., c.origin[1] + 8., CARD - 14., 24.),
                        "MZ",
                        11.,
                        theme.muted,
                    );
                }
                centered_text(
                    p,
                    rect(
                        c.origin[0] + 3.,
                        c.origin[1] + CARD - 22.,
                        c.size[0] - 6.,
                        20.,
                    ),
                    &op.label,
                    10.5,
                    theme.text,
                );
            }
            let count = row.operations.len();
            let slots = if count % COLUMNS == 0 {
                count + 1
            } else {
                count.div_ceil(COLUMNS) * COLUMNS
            };
            for column in row.operations.len()..slots {
                let area = l.card(i, column, self.pan);
                if area.origin[1] + CARD >= HEADER
                    && area.origin[1] < l.editor_bottom
                    && area.origin[0] + area.size[0] <= l.width - 18.
                {
                    empty_tile(p, area, theme);
                }
            }
            let terminal = l.row_port(i, self.pan);
            p.round_rect(terminal, 5., theme.active);
        }
        let new_row = l.new_row(self.view.rows.len(), self.pan);
        if new_row.origin[1] < l.editor_bottom {
            empty_tile(p, new_row, theme);
        }
        // Pinned output rail is drawn after the scrollable rows.
        p.round_rect(
            rect(0., HEADER, RAIL, l.editor_bottom - HEADER),
            0.,
            theme.body,
        );
        for (i, output) in self.view.outputs.iter().enumerate() {
            let terminal = l.output(i);
            p.text(
                rect(5., terminal.origin[1] - 2., RAIL - 23., 17.),
                &output.label,
                11.,
                theme.text,
            );
            p.round_rect(
                terminal,
                5.,
                if output.row.is_some() {
                    theme.active
                } else {
                    theme.muted
                },
            );
            if output.row.is_none() {
                p.round_rect(
                    rect(terminal.origin[0] + 2., terminal.origin[1] + 2., 6., 6.),
                    3.,
                    theme.body,
                );
                p.text(
                    rect(5., terminal.origin[1] + 16., RAIL - 12., 14.),
                    &output.fallback,
                    10.5,
                    theme.muted,
                );
            }
        }
        // Draw the selected row's incoming/outgoing connections last so they
        // remain readable at crossings with dimmed unrelated wires.
        for (path, (color, width)) in connection_paths {
            for [a, b] in path.segments(l.editor_bottom) {
                p.curve([a, a, b, b], width, color);
            }
        }
        match &self.gesture {
            Some(Gesture::RowWire { row, point }) => {
                if let Some(i) = self.view.rows.iter().position(|r| &r.id == row) {
                    let a = center(l.row_port(i, self.pan));
                    p.curve(
                        [a, [a[0] - 32., a[1]], [point[0] + 32., point[1]], *point],
                        2.,
                        theme.active,
                    );
                }
            }
            Some(Gesture::OutputWire { output, point, .. }) => {
                if let Some(i) = self.view.outputs.iter().position(|o| &o.id == output) {
                    let a = center(l.output(i));
                    p.curve(
                        [a, [a[0] + 32., a[1]], [point[0] - 32., point[1]], *point],
                        2.,
                        theme.active,
                    );
                }
            }
            _ => {}
        }
        p.round_rect(
            rect(0., l.editor_bottom, l.width, l.height - l.editor_bottom),
            0.,
            theme.body,
        );
        p.round_rect(rect(0., l.editor_bottom, l.width, 1.), 0., theme.control);
        for (index, label) in [
            "Props",
            "Sources",
            if self.view.geometry {
                "Inputs"
            } else {
                "Row Input"
            },
            "Tile",
        ]
        .into_iter()
        .enumerate()
        {
            let selected = (index == 0 && self.pane == MaterialPane::Properties)
                || (index == 1 && self.pane == MaterialPane::Sources)
                || (index == 2 && self.pane == MaterialPane::RowInput)
                || (index == 3 && self.pane == MaterialPane::Tile);
            let area = l.pane_tab(index);
            p.round_rect(area, 4., if selected { theme.control } else { theme.body });
            if selected {
                p.round_rect(
                    rect(
                        area.origin[0] + 8.,
                        area.origin[1] + area.size[1] - 2.,
                        area.size[0] - 16.,
                        2.,
                    ),
                    1.,
                    theme.active,
                );
            }
            centered_text(
                p,
                area,
                label,
                11.,
                if selected { theme.text } else { theme.muted },
            );
        }
        if self.pane == MaterialPane::Properties {
            p.text(
                rect(10., l.editor_bottom + 30., l.width - 20., 18.),
                parameters.as_ref().map_or("Material", |(label, _)| label),
                11.,
                theme.muted,
            );
            if let Some(palette) = &active_palette {
                if palette_entries.is_empty() {
                    p.text(
                        rect(16., l.editor_bottom + 58., l.width - 32., 36.),
                        "Add colors in the project Palette to use this tile.",
                        11.,
                        theme.muted,
                    );
                }
                for (area, entry) in &palette_entries {
                    let area = *area;
                    if area.origin[1] + area.size[1] > l.height {
                        break;
                    }
                    p.round_rect(
                        area,
                        3.,
                        if palette.2 == Some(entry.index) {
                            theme.active
                        } else {
                            theme.control
                        },
                    );
                    for y in 0..4 {
                        for x in 0..4 {
                            let checker = chrome.checker[(x + y) % 2];
                            let alpha = entry.color[3] as f32 / 255.;
                            let color = [
                                (entry.color[0] as f32 * alpha + checker[0] as f32 * (1. - alpha))
                                    as u8,
                                (entry.color[1] as f32 * alpha + checker[1] as f32 * (1. - alpha))
                                    as u8,
                                (entry.color[2] as f32 * alpha + checker[2] as f32 * (1. - alpha))
                                    as u8,
                                255,
                            ];
                            p.round_rect(
                                rect(
                                    area.origin[0] + 2. + x as f32 * 5.5,
                                    area.origin[1] + 2. + y as f32 * 5.5,
                                    5.5,
                                    5.5,
                                ),
                                0.,
                                color,
                            );
                        }
                    }
                }
            }
            for (i, line) in property_lines
                .iter()
                .skip(self.parameter_scroll)
                .enumerate()
            {
                let area = l.parameter(i);
                if area.origin[1] + area.size[1] > l.properties_bottom {
                    break;
                }
                match line {
                    PropertyLine::Section(label) => {
                        p.round_rect(
                            rect(area.origin[0], area.origin[1] + 5., 2., 12.),
                            1.,
                            theme.active,
                        );
                        p.text(
                            rect(
                                area.origin[0] + 4.,
                                area.origin[1] + 3.,
                                area.size[0] - 8.,
                                16.,
                            ),
                            &label.to_uppercase(),
                            8.,
                            theme.active,
                        );
                    }
                    PropertyLine::Values {
                        label,
                        values,
                        color,
                        owner,
                        ..
                    } => {
                        p.round_rect(area, 3., theme.control);
                        let swatch = if color.is_some() && values.len() >= 3 {
                            let rgba = [
                                linear_to_srgb(values[0].value),
                                linear_to_srgb(values[1].value),
                                linear_to_srgb(values[2].value),
                                values.get(3).map_or(1., |v| v.value),
                            ];
                            Some(rgba.map(|v| (v.clamp(0., 1.) * 255.) as u8))
                        } else {
                            None
                        };
                        if let Some(color) = swatch {
                            p.round_rect(
                                rect(area.origin[0] + 4., area.origin[1] + 3., 14., 14.),
                                2.,
                                color,
                            );
                        }
                        p.text(
                            rect(
                                area.origin[0] + if swatch.is_some() { 22. } else { 8. },
                                area.origin[1] + 3.,
                                area.size[0] * 0.44 - 10.,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        let width = area.size[0] * 0.55 / values.len().max(1) as f32;
                        if let Some(parameter) = color {
                            let text = self
                                .view
                                .palette_bindings
                                .iter()
                                .find(|b| &b.operation == owner && &b.parameter == parameter)
                                .map_or("Choose palette color".into(), |b| {
                                    format!("Palette {}", b.index)
                                });
                            p.text(
                                rect(
                                    area.origin[0] + area.size[0] * 0.45,
                                    area.origin[1] + 3.,
                                    area.size[0] * 0.55,
                                    16.,
                                ),
                                &text,
                                10.5,
                                theme.text,
                            );
                        }
                        for (j, value) in values.iter().enumerate().filter(|_| color.is_none()) {
                            p.text(
                                rect(
                                    area.origin[0] + area.size[0] * 0.45 + j as f32 * width + 2.,
                                    area.origin[1] + 3.,
                                    width - 5.,
                                    16.,
                                ),
                                &format!("{:.3}", value.value),
                                10.5,
                                theme.text,
                            );
                        }
                    }
                    PropertyLine::Choice {
                        label,
                        value,
                        choices,
                        ..
                    } => {
                        p.round_rect(area, 3., theme.control);
                        p.text(
                            rect(
                                area.origin[0] + 8.,
                                area.origin[1] + 3.,
                                area.size[0] * 0.44,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] * 0.45,
                                area.origin[1] + 3.,
                                area.size[0] * 0.5 - 12.,
                                16.,
                            ),
                            choices.get(*value).map_or("Choose…", |s| s.as_str()),
                            10.5,
                            theme.text,
                        );
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] - 14.,
                                area.origin[1] + 3.,
                                11.,
                                16.,
                            ),
                            "v",
                            10.5,
                            theme.muted,
                        );
                    }
                    PropertyLine::Bound { label, source, .. } => {
                        p.round_rect(area, 3., theme.control);
                        p.text(
                            rect(
                                area.origin[0] + 8.,
                                area.origin[1] + 3.,
                                area.size[0] * 0.44,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] * 0.45,
                                area.origin[1] + 3.,
                                area.size[0] * 0.55 - 8.,
                                16.,
                            ),
                            &format!("← {source}"),
                            10.5,
                            theme.active,
                        );
                    }
                    PropertyLine::Boolean { label, value, .. } => {
                        p.round_rect(area, 3., theme.control);
                        p.text(
                            rect(
                                area.origin[0] + 8.,
                                area.origin[1] + 3.,
                                area.size[0] * 0.75,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        let x = area.origin[0] + area.size[0] - 39.;
                        p.round_rect(
                            rect(x, area.origin[1] + 3., 30., 14.),
                            7.,
                            if *value { theme.active } else { theme.body },
                        );
                        p.round_rect(
                            rect(
                                x + if *value { 17. } else { 2. },
                                area.origin[1] + 5.,
                                11.,
                                11.,
                            ),
                            5.,
                            theme.text,
                        );
                    }
                }
            }
        } else if self.pane == MaterialPane::RowInput {
            for control in 0..8 {
                if self.view.geometry && control >= 3 {
                    continue;
                }
                if control == 7
                    && !matches!(
                        active_input.as_ref().map(|i| &i.target),
                        Some(MaterialInputTarget::RowLink { .. })
                    )
                {
                    continue;
                }
                let visible = control < 3
                    || control == 7
                    || matches!(
                        active_input.as_ref().map(|i| &i.binding),
                        Some(MaterialInputBinding::Row(_))
                    ) && (control < 5
                        || matches!(
                            active_input.as_ref().map(|i| &i.binding),
                            Some(MaterialInputBinding::Row(MaterialReferenceView {
                                mapping: Some(_),
                                ..
                            }))
                        ));
                if !visible {
                    continue;
                }
                let area = l.input_control(control);
                if area.origin[1] + area.size[1] > l.height {
                    continue;
                }
                p.round_rect(area, 3., theme.control);
                p.text(
                    rect(
                        area.origin[0] + 6.,
                        area.origin[1] + 5.,
                        area.size[0] - 18.,
                        16.,
                    ),
                    &input_labels[control],
                    10.5,
                    theme.text,
                );
                if control < 3 || control == 7 {
                    p.text(
                        rect(
                            area.origin[0] + area.size[0] - 14.,
                            area.origin[1] + 5.,
                            11.,
                            16.,
                        ),
                        "v",
                        10.5,
                        theme.muted,
                    );
                }
                if control == 3 {
                    if let Some(MaterialInputBinding::Row(reference)) =
                        active_input.as_ref().map(|i| &i.binding)
                    {
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] * 0.5,
                                area.origin[1] + 4.,
                                area.size[0] * 0.45,
                                16.,
                            ),
                            &reference.components,
                            11.,
                            theme.text,
                        );
                    }
                }
            }
            if active_input.is_none() {
                p.text(
                    rect(16., l.editor_bottom + 122., l.width - 32., 36.),
                    "Select a row or tile to inspect its inputs.",
                    11.,
                    theme.muted,
                );
            }
        } else if self.pane == MaterialPane::Tile {
            p.text(
                rect(10., l.editor_bottom + 30., l.width - 20., 18.),
                &source_title,
                11.,
                theme.muted,
            );
            p.round_rect(source_area, 3., theme.body);
            for (i, label) in [
                "Apply",
                "Revert",
                "Reset",
                if self.source_help { "Source" } else { "Help" },
            ]
            .into_iter()
            .enumerate()
            {
                let area = source_actions[i];
                p.round_rect(area, 3., theme.control);
                centered_text(p, area, label, 10.5, theme.text);
            }
            if self.source_edit.is_none() {
                if let Some(source) = &source_document {
                    let text = if self.source_help {
                        source.help.as_str()
                    } else {
                        source.code.as_str()
                    };
                    let area = source_area;
                    for (i, line) in text
                        .lines()
                        .take((area.size[1] / 15.).max(0.) as usize)
                        .enumerate()
                    {
                        p.text(
                            rect(
                                area.origin[0] + 5.,
                                area.origin[1] + i as f32 * 15. + 3.,
                                area.size[0] - 10.,
                                15.,
                            ),
                            line,
                            10.5,
                            theme.text,
                        );
                    }
                }
            }
        } else {
            let search = l.search();
            let category = l.category();
            let category_color =
                chrome.category(self.source_category.as_deref().unwrap_or("Coordinates"));
            p.round_rect(category, 4., category_color);
            p.round_rect(
                rect(
                    category.origin[0] + 1.,
                    category.origin[1] + 1.,
                    category.size[0] - 2.,
                    category.size[1] - 2.,
                ),
                3.,
                chrome.dropdown,
            );
            p.text(
                rect(
                    category.origin[0] + 6.,
                    category.origin[1] + 4.,
                    category.size[0] - 18.,
                    16.,
                ),
                self.source_category.as_deref().unwrap_or("All"),
                10.5,
                category_color,
            );
            p.text(
                rect(
                    category.origin[0] + category.size[0] - 13.,
                    category.origin[1] + 4.,
                    11.,
                    16.,
                ),
                "v",
                10.5,
                category_color,
            );
            p.round_rect(
                search,
                4.,
                if self.search_focused {
                    theme.active
                } else {
                    theme.control
                },
            );
            if !native_editors {
                p.text(
                    rect(16., search.origin[1] + 2., search.size[0] - 12., 18.),
                    if self.query.is_empty() {
                        "Find a source…"
                    } else {
                        &self.query
                    },
                    10.5,
                    theme.text,
                );
            }
            let query = self.query.to_lowercase();
            let sources: Vec<_> = self
                .view
                .sources
                .iter()
                .filter(|s| {
                    self.source_category
                        .as_ref()
                        .is_none_or(|category| category == &s.category)
                        && (s.label.to_lowercase().contains(&query)
                            || s.category.to_lowercase().contains(&query))
                })
                .collect();
            for (i, source) in sources.iter().skip(self.library_scroll).enumerate() {
                let y = l.library_top + i as f32 * SOURCE_STEP;
                if y + 30. > l.height {
                    break;
                }
                let area = rect(10., y, l.width - 20., 30.);
                p.round_rect(
                    area,
                    4.,
                    if self.selected_source.as_ref() == Some(&source.id) {
                        theme.active
                    } else {
                        chrome.source
                    },
                );
                if self.selected_source.as_ref() == Some(&source.id) {
                    p.round_rect(rect(11., y + 1., l.width - 22., 28.), 3., theme.body);
                }
                let accent = chrome.category(&source.category);
                p.round_rect(rect(10., y + 4., 3., 22.), 1., accent);
                p.text(
                    rect(16., y + 1., l.width - 32., 16.),
                    &source.label,
                    10.5,
                    theme.text,
                );
                p.text(
                    rect(16., y + 16., l.width - 32., 12.),
                    &source.category,
                    9.5,
                    accent,
                );
            }
            if sources.is_empty() {
                p.text(
                    rect(20., l.library_top + 12., l.width - 40., 32.),
                    "No matching sources",
                    11.,
                    theme.muted,
                );
            }
        }
        if let Some((area, options)) = popup {
            p.round_rect(
                rect(
                    area.origin[0] + 3.,
                    area.origin[1] + 4.,
                    area.size[0],
                    area.size[1],
                ),
                4.,
                chrome.shadow,
            );
            p.round_rect(area, 3., theme.active);
            p.round_rect(
                rect(
                    area.origin[0] + 1.,
                    area.origin[1] + 1.,
                    area.size[0] - 2.,
                    area.size[1] - 2.,
                ),
                2.,
                chrome.popup,
            );
            for (index, option) in options.iter().skip(self.popup_scroll).take(8).enumerate() {
                let item = rect(
                    area.origin[0] + 1.,
                    area.origin[1] + index as f32 * 22. + 1.,
                    area.size[0] - 2.,
                    20.,
                );
                let selected = self.input_popup == Some(8)
                    && option == self.source_category.as_deref().unwrap_or("All");
                let hovered = item.contains(self.pointer);
                p.round_rect(
                    item,
                    0.,
                    if selected {
                        chrome.selection
                    } else if hovered {
                        chrome.hover
                    } else {
                        chrome.popup
                    },
                );
                let accent = if self.input_popup == Some(8) {
                    chrome.category(option)
                } else {
                    theme.active
                };
                if self.input_popup == Some(8) {
                    p.round_rect(
                        rect(item.origin[0] + 5., item.origin[1] + 7., 3., 7.),
                        1.,
                        accent,
                    );
                }
                p.text(
                    rect(
                        item.origin[0] + 13.,
                        item.origin[1] + 4.,
                        item.size[0] - 19.,
                        16.,
                    ),
                    option,
                    10.5,
                    if selected { accent } else { theme.text },
                );
            }
        }
        if let Some(Gesture::Source { id, start, point }) = &self.gesture
            && (point[0] - start[0]).abs() + (point[1] - start[1]).abs() > 4.
        {
            if let Some(source) = self.view.sources.iter().find(|s| &s.id == id) {
                p.round_rect(
                    rect(point[0] + 8., point[1] + 8., 116., 27.),
                    4.,
                    theme.active,
                );
                p.text(
                    rect(point[0] + 16., point[1] + 10., 100., 23.),
                    &source.label,
                    11.,
                    theme.text,
                );
            }
        }
    }
}
