//! Two-phase pixel clipboard edits. A successful platform write is necessary
//! but cannot authorize cutting a replaced source, selection or document.
use super::*;
use theframework::platform::{ImageClipboardRequest, ImageClipboardResult, new_clipboard_request};
use theframework_core::ImageAsset;
const VIEW: &str = "Tile Editor Dock RGBA Layout View";
#[derive(Clone, Copy)]
enum Intent {
    Copy,
    Cut,
    Paste,
}
pub(super) struct Pending {
    request: u64,
    intent: Intent,
    context: PixelEditingContext,
    source: rusterix::Texture,
    selection: FxHashSet<(i32, i32)>,
    focus: Uuid,
    revision: u64,
}
fn selection(ui: &mut TheUI) -> FxHashSet<(i32, i32)> {
    ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
        .and_then(|layout| layout.rgba_view_mut().as_rgba_view())
        .map(|view| view.selection())
        .unwrap_or_default()
}
#[cfg(feature = "native-ui")]
fn focused(ui: &mut TheUI, ctx: &TheContext) -> Option<Uuid> {
    if ui.dialog.is_some()
        || ui.popover.is_some()
        || ui.context_menu.is_some()
        || ctx.ui.overlay.is_some()
    {
        return None;
    }
    let focus = ctx.ui.focus.as_ref()?.uuid;
    let mut visible = false;
    ui.canvas.visit_visible_widgets(&mut |widget| {
        if widget.id().uuid == focus
            && widget.id().name == VIEW
            && !widget.disabled()
            && !ctx.ui.is_disabled(VIEW)
            && widget.dim().is_valid()
        {
            visible = true;
        }
    });
    visible.then_some(focus)
}
#[cfg(not(feature = "native-ui"))]
fn focused(_: &mut TheUI, _: &TheContext) -> Option<Uuid> {
    None
}
fn image(
    source: &rusterix::Texture,
    selection: &FxHashSet<(i32, i32)>,
) -> Result<ImageAsset, String> {
    if source
        .width
        .checked_mul(source.height)
        .and_then(|n| n.checked_mul(4))
        != Some(source.data.len())
    {
        return Err("Invalid source image byte count".into());
    }
    let points: Vec<_> = selection
        .iter()
        .copied()
        .filter(|(x, y)| {
            *x >= 0 && *y >= 0 && (*x as usize) < source.width && (*y as usize) < source.height
        })
        .collect();
    if selection.is_empty() {
        return ImageAsset::rgba(
            source
                .width
                .try_into()
                .map_err(|_| "Clipboard width overflow")?,
            source
                .height
                .try_into()
                .map_err(|_| "Clipboard height overflow")?,
            source.data.clone(),
        )
        .map_err(|e| e.to_string());
    }
    let min_x = points
        .iter()
        .map(|p| p.0)
        .min()
        .ok_or("Selection has no editable pixels")?;
    let min_y = points.iter().map(|p| p.1).min().unwrap();
    let max_x = points.iter().map(|p| p.0).max().unwrap();
    let max_y = points.iter().map(|p| p.1).max().unwrap();
    let width = (max_x - min_x + 1) as usize;
    let height = (max_y - min_y + 1) as usize;
    let length = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .ok_or("Clipboard dimensions overflow")?;
    if length > theframework::platform::image_clipboard::MAX_BYTES {
        return Err("Clipboard image exceeds the byte limit".into());
    }
    let mut pixels = vec![0; length];
    for (x, y) in points {
        let from = (y as usize * source.width + x as usize) * 4;
        let to = ((y - min_y) as usize * width + (x - min_x) as usize) * 4;
        pixels[to..to + 4].copy_from_slice(&source.data[from..from + 4]);
    }
    ImageAsset::rgba(width as u32, height as u32, pixels).map_err(|e| e.to_string())
}
impl TilesEditorDock {
    pub(super) fn image_clipboard_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> Option<bool> {
        if let TheEvent::ImageClipboardResult(request, result) = event {
            if self
                .pending_clipboard
                .as_ref()
                .is_none_or(|pending| pending.request != *request)
            {
                return Some(false);
            }
            let pending = self.pending_clipboard.take().unwrap();
            if pending.context != server.editing_ctx
                || pending.revision != self.clipboard_revision
                || focused(ui, ctx) != Some(pending.focus)
                || selection(ui) != pending.selection
                || project.get_editing_texture(&pending.context) != Some(&pending.source)
            {
                return Some(false);
            }
            return Some(match (pending.intent, result) {
                (Intent::Cut, ImageClipboardResult::Written(true)) => {
                    let changed = self.clear_current_selection(project, ui, ctx, server);
                    if changed {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_cut_selection"),
                        ));
                    }
                    changed
                }
                (Intent::Copy, ImageClipboardResult::Written(true)) => {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        if pending.selection.is_empty() {
                            fl!("status_tile_editor_copy_texture")
                        } else {
                            fl!("status_tile_editor_copy_selection")
                        },
                    ));
                    true
                }
                (Intent::Paste, ImageClipboardResult::Image(Some(image))) => {
                    if theframework::platform::image_clipboard::validate(image).is_err() {
                        return Some(false);
                    }
                    self.paste_preview_texture = Some(rusterix::Texture::new(
                        image.pixels().to_vec(),
                        image.width() as usize,
                        image.height() as usize,
                    ));
                    if self.paste_preview_pos.is_none() {
                        self.paste_preview_pos = Some(Vec2::new(
                            pending.source.width as i32 / 2,
                            pending.source.height as i32 / 2,
                        ));
                    }
                    self.sync_paste_preview(ui, ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_tile_editor_paste_preview_active"),
                    ));
                    true
                }
                _ => {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(),"Image clipboard was unavailable or the operation failed. No pixels were changed.".into()));
                    true
                }
            });
        }
        if !ctx.ui.command_paint {
            return None;
        }
        let intent = match event {
            TheEvent::Copy => Intent::Copy,
            TheEvent::Cut => Intent::Cut,
            TheEvent::Paste(_, _) => Intent::Paste,
            _ => return None,
        };
        self.pending_clipboard = None;
        let Some(focus) = focused(ui, ctx) else {
            self.pending_clipboard = None;
            return Some(false);
        };
        let Some(source) = project.get_editing_texture(&server.editing_ctx) else {
            return Some(false);
        };
        if source.width == 0
            || source.height == 0
            || source.width > theframework::platform::image_clipboard::MAX_DIMENSION as usize
            || source.height > theframework::platform::image_clipboard::MAX_DIMENSION as usize
            || source
                .width
                .checked_mul(source.height)
                .and_then(|n| n.checked_mul(4))
                != Some(source.data.len())
        {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Invalid clipboard source dimensions or byte count".into(),
            ));
            return Some(true);
        }
        let selection = selection(ui);
        if matches!(intent, Intent::Cut) && selection.is_empty() {
            return Some(false);
        }
        if source.data.len() > theframework::platform::image_clipboard::MAX_BYTES
            || ctx.ui.image_clipboard.len() >= 4
        {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Image clipboard exceeds its pending-operation or byte limit".into(),
            ));
            return Some(true);
        }
        let request = new_clipboard_request();
        let operation = match intent {
            Intent::Paste => ImageClipboardRequest::Read { request },
            _ => match image(source, &selection).and_then(|image| {
                theframework::platform::image_clipboard::validate(&image).map(|_| image)
            }) {
                Ok(image) => ImageClipboardRequest::Write { request, image },
                Err(error) => {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                    return Some(true);
                }
            },
        };
        self.pending_clipboard = Some(Pending {
            request,
            intent,
            context: server.editing_ctx,
            source: source.clone(),
            selection,
            focus,
            revision: self.clipboard_revision,
        });
        ctx.ui.image_clipboard.push_back(operation);
        Some(true)
    }
}

#[cfg(all(test, feature = "native-render"))]
impl TilesEditorDock {
    /// Reuses the assembled minimap's editable source, with no platform clipboard
    /// access. Host results deliberately arrive after failures/owner changes.
    pub(crate) fn check_native_image_clipboard_for_test(
        source: &Project,
        editing: PixelEditingContext,
    ) {
        use theframework::platform::UiSession;
        let mut project = source.clone();
        let mut server = ServerContext::new();
        server.editing_ctx = editing;
        let original = project.get_editing_texture(&editing).unwrap().clone();
        let mut dock = Self::new();
        let mut session = UiSession::new((640, 420), 1.25);
        session.ui.canvas = dock.setup(&mut session.context);
        session
            .ui
            .canvas
            .set_dim(TheDim::sized(640, 420), &mut session.context);
        dock.activate(&mut session.ui, &mut session.context, &project, &mut server);
        session.ui.prepare_paint_layout(&mut session.context);
        let view = {
            let view = session
                .ui
                .get_rgba_layout("Tile Editor Dock RGBA Layout")
                .unwrap()
                .rgba_view_mut();
            let id = view.id().clone();
            view.as_rgba_view()
                .unwrap()
                .set_selection([(1, 1), (2, 1)].into_iter().collect());
            id
        };
        session.context.ui.set_focus(&view);
        fn stage(
            dock: &mut TilesEditorDock,
            session: &mut UiSession,
            project: &mut Project,
            server: &mut ServerContext,
            event: TheEvent,
        ) -> ImageClipboardRequest {
            assert_eq!(
                dock.image_clipboard_event(
                    &event,
                    &mut session.ui,
                    &mut session.context,
                    project,
                    server
                ),
                Some(true)
            );
            session.context.ui.image_clipboard.pop_front().unwrap()
        }
        fn complete(
            dock: &mut TilesEditorDock,
            session: &mut UiSession,
            project: &mut Project,
            server: &mut ServerContext,
            request: u64,
            result: ImageClipboardResult,
        ) {
            dock.image_clipboard_event(
                &TheEvent::ImageClipboardResult(request, result),
                &mut session.ui,
                &mut session.context,
                project,
                server,
            );
        }
        let ImageClipboardRequest::Write { request, image } = stage(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            TheEvent::Cut,
        ) else {
            panic!("cut must stage a write");
        };
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &original,
            "cut cannot mutate before a platform acknowledgment"
        );
        assert_eq!((image.width(), image.height()), (2, 1));
        let png = theframework::platform::image_clipboard::encode_png(&image).unwrap();
        let decoded = theframework::platform::image_clipboard::decode_png(&png).unwrap();
        assert_eq!(
            decoded, image,
            "clipboard PNG keeps source bytes and straight alpha"
        );
        assert!(theframework::platform::image_clipboard::decode_png(&png[..8]).is_err());
        complete(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            request + 1,
            ImageClipboardResult::Written(true),
        );
        assert!(
            dock.pending_clipboard.is_some(),
            "wrong completion cannot retire the expected write"
        );
        complete(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            request,
            ImageClipboardResult::Written(false),
        );
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &original,
            "failed writes preserve source"
        );

        let ImageClipboardRequest::Write { request, .. } = stage(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            TheEvent::Cut,
        ) else {
            unreachable!()
        };
        project.get_editing_texture_mut(&editing).unwrap().data[0] ^= 1;
        let edited = project.get_editing_texture(&editing).unwrap().clone();
        complete(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            request,
            ImageClipboardResult::Written(true),
        );
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &edited,
            "a later source edit retires a delayed cut"
        );
        *project.get_editing_texture_mut(&editing).unwrap() = original.clone();

        let ImageClipboardRequest::Write { request, .. } = stage(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            TheEvent::Cut,
        ) else {
            unreachable!()
        };
        complete(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            request,
            ImageClipboardResult::Written(true),
        );
        let cut = project.get_editing_texture(&editing).unwrap();
        for (x, y) in [(1, 1), (2, 1)] {
            let at = (y * cut.width + x) * 4;
            assert_eq!(&cut.data[at..at + 4], &[0; 4]);
        }
        dock.mark_saved();
        assert!(
            !dock.has_changes(),
            "confirmed save records the current source cursor"
        );
        dock.undo(
            &mut session.ui,
            &mut session.context,
            &mut project,
            &mut server,
        );
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &original,
            "confirmed cut is one reversible edit"
        );
        assert!(
            dock.has_changes(),
            "undo after save remains available and becomes unsaved"
        );
        let ImageClipboardRequest::Write { request, .. } = stage(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            TheEvent::Cut,
        ) else {
            unreachable!()
        };
        dock.redo(
            &mut session.ui,
            &mut session.context,
            &mut project,
            &mut server,
        );
        dock.undo(
            &mut session.ui,
            &mut session.context,
            &mut project,
            &mut server,
        );
        complete(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            request,
            ImageClipboardResult::Written(true),
        );
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &original,
            "history roundtrip cannot resurrect an old cut"
        );

        let ImageClipboardRequest::Write { request, .. } = stage(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            TheEvent::Cut,
        ) else {
            unreachable!()
        };
        dock.reset_for_project_switch();
        complete(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            request,
            ImageClipboardResult::Written(true),
        );
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &original,
            "reused persisted IDs do not revive a closed document request"
        );
        dock.activate(&mut session.ui, &mut session.context, &project, &mut server);
        session
            .ui
            .get_rgba_layout("Tile Editor Dock RGBA Layout")
            .unwrap()
            .rgba_view_mut()
            .as_rgba_view()
            .unwrap()
            .set_selection(Default::default());
        session.context.ui.set_focus(&view);
        let ImageClipboardRequest::Read { request } = stage(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            TheEvent::Paste(TheValue::Empty, None),
        ) else {
            unreachable!()
        };
        complete(
            &mut dock,
            &mut session,
            &mut project,
            &mut server,
            request,
            ImageClipboardResult::Image(Some(decoded)),
        );
        assert!(dock.paste_preview_texture.is_some());
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &original,
            "paste completion only stages a preview"
        );
        assert!(dock.apply_paste_at_preview(
            &mut project,
            &mut session.ui,
            &mut session.context,
            &mut server
        ));
        dock.undo(
            &mut session.ui,
            &mut session.context,
            &mut project,
            &mut server,
        );
        assert_eq!(
            project.get_editing_texture(&editing).unwrap(),
            &original,
            "paste applies through the ordinary undo owner"
        );
        // Saved checkpoints must describe source history, including branches
        // and retention limits; cursor reuse alone cannot imply clean content.
        let mut first = original.clone();
        first.data[0] ^= 1;
        let mut second = original.clone();
        second.data[0] ^= 2;
        let mut third = original.clone();
        third.data[0] ^= 3;
        let mut history = TileEditorUndo::new();
        history.add(TileEditorUndoAtom::TextureEdit(
            editing,
            original.clone(),
            first.clone(),
        ));
        *project.get_editing_texture_mut(&editing).unwrap() = first.clone();
        history.mark_saved();
        history.undo(&mut project, &mut session.ui, &mut session.context);
        history.add(TileEditorUndoAtom::TextureEdit(
            editing,
            original.clone(),
            second.clone(),
        ));
        *project.get_editing_texture_mut(&editing).unwrap() = second.clone();
        assert!(
            history.has_changes(),
            "different edit at the saved cursor remains unsaved"
        );
        history.mark_saved();
        history.add(TileEditorUndoAtom::TextureEdit(
            editing,
            second.clone(),
            third.clone(),
        ));
        *project.get_editing_texture_mut(&editing).unwrap() = third.clone();
        history.truncate_to_limit(1);
        history.undo(&mut project, &mut session.ui, &mut session.context);
        assert_eq!(project.get_editing_texture(&editing).unwrap(), &second);
        assert!(
            !history.has_changes(),
            "retained checkpoint remains reachable after eviction"
        );
        let mut history = TileEditorUndo::new();
        history.add(TileEditorUndoAtom::TextureEdit(
            editing,
            original.clone(),
            first.clone(),
        ));
        history.add(TileEditorUndoAtom::TextureEdit(
            editing,
            first.clone(),
            third.clone(),
        ));
        *project.get_editing_texture_mut(&editing).unwrap() = third;
        history.truncate_to_limit(1);
        history.undo(&mut project, &mut session.ui, &mut session.context);
        assert_eq!(project.get_editing_texture(&editing).unwrap(), &first);
        assert!(
            history.has_changes(),
            "evicted original checkpoint cannot falsely clean a document"
        );
    }
}
