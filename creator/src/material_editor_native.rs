//! Native MaterialZ panel presentation. Source thumbnails are explicit assets;
//! panel chrome, graph geometry and embedded editor text are ordered commands.
use super::*;
use std::collections::HashMap;
use theframework::{
    paint::{ImageHandle, PaintOp, Transform},
    thegraph::{GraphImageHandles, PaintGraphPainter},
    theui::paint::{UiPaintResult, WidgetPainter, bounds},
};
struct PreviewHandles(HashMap<String, ImageHandle>);
impl GraphImageHandles for PreviewHandles {
    fn image_handle(&self, key: &str) -> Option<ImageHandle> {
        self.0.get(key).copied()
    }
}
impl MaterialEditorWidget {
    pub(super) fn prepare_native(&mut self, ctx: &mut TheContext, theme: &dyn TheTheme) {
        self.chrome = MaterialEditorChrome::from_theme(theme);
        self.sync_number_edits(ctx);
        self.sync_source_edit(ctx);
        self.prepare_native_search(ctx, theme);
        self.sync_native_scope(ctx);
        if self.input_popup.is_none() {
            for (_, _, edit) in &mut self.number_edits {
                edit.prepare_paint(ctx, theme);
            }
        }
        if self.pane == MaterialPane::Tile {
            if let Some((_, edit)) = &mut self.source_edit {
                edit.prepare_paint(ctx, theme);
            }
        }
    }
    pub(super) fn paint_native(&mut self, painter: &mut WidgetPainter<'_>) -> UiPaintResult {
        let mut handles = PreviewHandles(HashMap::new());
        let layout = self.layout();
        // Resolve only previews which the common row painter will display.
        for (i, row) in self.view.rows.iter().enumerate() {
            let r = layout.row(i, self.pan);
            if r.origin[1] + r.size[1] < HEADER || r.origin[1] > layout.editor_bottom {
                continue;
            }
            for (j, op) in row.operations.iter().enumerate() {
                let card = layout.card(i, j, self.pan);
                if card.origin[1] + card.size[1] < HEADER
                    || card.origin[1] > layout.editor_bottom
                    || card.origin[0] + card.size[0] > layout.width - 18.
                {
                    continue;
                }
                if handles.0.contains_key(&op.thumbnail) {
                    continue;
                }
                if let Some(image) = self.thumbnails.get(&op.thumbnail) {
                    let handle = painter.images.image(
                        &format!("material:{}:{}", self.id.uuid, op.thumbnail),
                        image,
                        self.thumbnail_changes.contains(&op.thumbnail),
                    )?;
                    handles.0.insert(op.thumbnail.clone(), handle);
                    self.thumbnail_changes.remove(&op.thumbnail);
                }
            }
        }
        let outer = bounds(&self.dim);
        painter.list.ops.push(PaintOp::PushClip(outer));
        painter
            .list
            .ops
            .push(PaintOp::PushTransform(Transform::translate(
                outer.x, outer.y,
            )));
        {
            let mut graph = PaintGraphPainter {
                paint: painter.list,
                text: painter.text,
                images: &handles,
                family: "Roboto",
            };
            self.paint_graph(
                &mut graph,
                [outer.width, outer.height],
                &material_editor_theme(painter.theme),
                true,
            );
        }
        if self.input_popup.is_none() {
            for (_, _, edit) in &mut self.number_edits {
                painter.widget(edit)?;
            }
        }
        if self.pane == MaterialPane::Tile {
            if let Some((_, edit)) = &mut self.source_edit {
                painter.widget(edit)?;
            }
        }
        if self.pane == MaterialPane::Sources && self.input_popup.is_none() {
            painter.widget(&mut self.native_search)?;
            if self.query.is_empty() && !self.search_focused {
                let r = bounds(self.native_search.dim());
                painter.label(
                    r.inset(4.),
                    "Find a source…",
                    10.5,
                    *painter.theme.color(MaterialMuted),
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }
        }
        painter.list.ops.push(PaintOp::PopTransform);
        painter.list.ops.push(PaintOp::PopClip);
        self.dirty = false;
        Ok(())
    }
}

pub(super) fn search_editor(owner: &TheId) -> TheTextLineEdit {
    let mut search = TheTextLineEdit::new(TheId::named("Material source filter"));
    search.set_parent_id(owner.clone());
    search.set_embedded(true);
    search.set_font_size(10.5);
    search.set_continuous(false);
    search
}
impl MaterialEditorWidget {
    fn prepare_native_search(&mut self, ctx: &mut TheContext, theme: &dyn TheTheme) {
        if self.native_search.text() != self.query {
            self.native_search.set_text(self.query.clone());
        }
        let r = self.layout().search();
        self.native_search.set_dim(
            TheDim::new(
                r.origin[0] as i32,
                r.origin[1] as i32,
                r.size[0] as i32,
                r.size[1] as i32,
            ),
            ctx,
        );
        self.native_search.prepare_paint(ctx, theme);
    }
    pub(super) fn sync_native_scope(&mut self, ctx: &mut TheContext) {
        let active = self.native_active && ctx.ui.has_focus(&self.id) && self.input_popup.is_none();
        self.native_search.editor_set_active(
            active && self.search_focused && self.pane == MaterialPane::Sources,
            ctx,
        );
        for (index, (_, _, edit)) in self.number_edits.iter_mut().enumerate() {
            edit.editor_set_active(
                active
                    && self.number_focus == Some(index)
                    && matches!(self.pane, MaterialPane::Properties | MaterialPane::RowInput),
                ctx,
            );
        }
        if let Some((_, edit)) = &mut self.source_edit {
            edit.editor_set_active(
                active && self.source_focus && self.pane == MaterialPane::Tile,
                ctx,
            );
        }
    }
    pub(super) fn native_editor_mut(&mut self) -> Option<&mut dyn TheWidget> {
        if self.input_popup.is_some() {
            return None;
        }
        if self.source_focus && self.pane == MaterialPane::Tile {
            return self
                .source_edit
                .as_mut()
                .map(|(_, editor)| editor as &mut dyn TheWidget);
        }
        if self.search_focused && self.pane == MaterialPane::Sources {
            return Some(&mut self.native_search);
        }
        if matches!(self.pane, MaterialPane::Properties | MaterialPane::RowInput) {
            let index = self.number_focus?;
            return self
                .number_edits
                .get_mut(index)
                .map(|(_, _, editor)| editor as &mut dyn TheWidget);
        }
        None
    }
    pub(super) fn finish_native_response(
        &mut self,
        mut response: theframework::text_edit::EditorResponse,
        ctx: &mut TheContext,
    ) -> theframework::text_edit::EditorResponse {
        use theframework::text_edit::EditorAction;
        if self.search_focused && response.changed {
            self.query = self.native_search.text();
            self.library_scroll = 0;
        }
        if response.action == Some(EditorAction::Cancel) {
            self.search_focused = false;
            self.source_focus = false;
            self.number_focus = None;
            self.number_drag = false;
            self.cancel_gesture();
            self.sync_native_scope(ctx);
            response.action = None;
        } else if let Some(EditorAction::FocusNext { backwards }) = response.action {
            if let Some(index) = self.number_focus {
                let next = if backwards {
                    index.checked_sub(1)
                } else {
                    index
                        .checked_add(1)
                        .filter(|next| *next < self.number_edits.len())
                };
                if let Some(next) = next {
                    self.number_focus = Some(next);
                    self.sync_native_scope(ctx);
                    self.number_edits[next]
                        .2
                        .editor_command(theframework::text_edit::EditorCommand::SelectAll, ctx);
                    response.action = None;
                }
            }
        }
        self.dirty |= response.input.redraw;
        response
    }
    pub(super) fn native_search_event(
        &mut self,
        event: &TheEvent,
        ctx: &mut TheContext,
    ) -> Option<bool> {
        match event {
            TheEvent::LostFocus(_) => {
                self.native_active = false;
                self.sync_native_scope(ctx);
                return None;
            }
            TheEvent::GainedFocus(_) => {
                self.native_active = true;
                self.sync_native_scope(ctx);
                return None;
            }
            TheEvent::MouseDown(coord) => {
                let point = [coord.x as f32, coord.y as f32];
                let search = self.pane == MaterialPane::Sources
                    && self.input_popup.is_none()
                    && self.layout().search().contains(point);
                if search {
                    ctx.ui.set_focus(&self.id);
                    self.search_focused = true;
                    self.source_focus = false;
                    self.number_focus = None;
                    self.native_active = true;
                    self.sync_native_scope(ctx);
                } else {
                    if self.search_focused {
                        self.native_search.editor_set_active(false, ctx);
                    }
                    self.search_focused = false;
                    return None;
                }
            }
            TheEvent::MouseDragged(_) | TheEvent::MouseUp(_) if self.search_focused => {}
            TheEvent::ModifierChanged(..)
            | TheEvent::Copy
            | TheEvent::Cut
            | TheEvent::Paste(..)
            | TheEvent::KeyDown(_)
            | TheEvent::KeyCodeDown(_)
            | TheEvent::TextInput(_)
            | TheEvent::TextComposition(..)
                if self.search_focused => {}
            _ => return None,
        }
        let dim = *self.native_search.dim();
        let local = |p: &Vec2<i32>| Vec2::new(p.x - dim.x, p.y - dim.y);
        let event = match event {
            TheEvent::MouseDown(p) => TheEvent::MouseDown(local(p)),
            TheEvent::MouseDragged(p) => TheEvent::MouseDragged(local(p)),
            TheEvent::MouseUp(p) => TheEvent::MouseUp(local(p)),
            _ => event.clone(),
        };
        let redraw = self.native_search.on_event(&event, ctx);
        let query = self.native_search.text();
        if self.query != query {
            self.query = query;
            self.library_scroll = 0;
        }
        self.dirty |= redraw;
        Some(redraw)
    }
}
