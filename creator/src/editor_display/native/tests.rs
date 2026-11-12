use super::*;
fn target(session: &EditorDisplaySession) -> DisplayTarget<'static> {
    DisplayTarget {
        scope: PanelScope {
            document: Uuid::from_u128(601),
            tool: Uuid::from_u128(602),
            revision: 3,
        },
        owner: session.owner,
        context: session.project_context,
        size: (800, 360),
        view_name: "PrefabView",
        visible: true,
    }
}
fn session() -> EditorDisplaySession {
    super::super::native_tests::fixture(0)
}
fn acknowledge(
    session: &mut EditorDisplaySession,
    frame: &DisplayFrame,
    target: DisplayTarget<'_>,
) -> bool {
    #[cfg(feature = "native-ui")]
    {
        session.presented_paint(frame, &[frame.receipt()], target)
    }
    #[cfg(not(feature = "native-ui"))]
    {
        session.presented_reference(frame, target)
    }
}
#[test]
fn display_preparation_keeps_transform_inactive_until_presented_and_rejects_foreign_bindings() {
    let mut session = session();
    let binding = target(&session);
    let frame = session.prepare_frame(binding).unwrap();
    assert!(session.last_transform.is_none());
    assert_eq!(session.last_view_width, 0);
    let click = TheEvent::RenderViewClicked(TheId::named("PrefabView"), Vec2::new(90, 14));
    assert_eq!(
        session.handle_presented_event(&click, binding, true, false),
        DisplayInput::Unavailable
    );
    assert!(acknowledge(&mut session, &frame, binding));
    assert!(session.last_transform.is_some());
    assert!(matches!(
        session.handle_presented_event(&click, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::GridSubdivisionChanged(1.))
    ));
    let next = session.prepare_frame(binding).unwrap();
    assert!(!acknowledge(&mut session, &frame, binding));
    assert!(acknowledge(&mut session, &next, binding));
    let foreign = DisplayTarget {
        owner: EditorDisplayOwner::Tool(Uuid::new_v4()),
        ..binding
    };
    assert_eq!(
        session.handle_presented_event(&click, foreign, true, false),
        DisplayInput::Unavailable
    );
    assert!(!acknowledge(&mut session, &next, foreign));
    assert!(session.prepare_frame(foreign).is_none());
}

#[test]
fn display_captured_draft_drag_continues_between_frames_while_new_picking_waits_for_geometry() {
    let mut session = session();
    let binding = target(&session);
    let original = session.original_display().clone();
    let frame = session.prepare_frame(binding).unwrap();
    let start = frame
        .transform
        .world_to_screen(session.profile().points[1])
        .map(|v| v.round() as i32);
    assert!(acknowledge(&mut session, &frame, binding));
    let event = |coord| TheEvent::RenderViewClicked(TheId::named("PrefabView"), coord);
    assert_eq!(
        session.handle_presented_event(&event(start), binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Handled)
    );
    let drag = |coord| TheEvent::RenderViewDragged(TheId::named("PrefabView"), coord);
    assert!(matches!(
        session.handle_presented_event(&drag(start + Vec2::new(34, -17)), binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::DisplayChanged(_))
    ));
    let pending = session.prepare_frame(binding).unwrap();
    assert!(matches!(
        session.handle_presented_event(&drag(start + Vec2::new(68, -34)), binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::DisplayChanged(_))
    ));
    assert!(!acknowledge(&mut session, &pending, binding));
    let newest = session.prepare_frame(binding).unwrap();
    assert_eq!(
        session.handle_presented_event(
            &TheEvent::RenderViewUp(TheId::named("PrefabView"), start),
            binding,
            true,
            false
        ),
        DisplayInput::Outcome(EditorDisplayResult::Handled)
    );
    let working = session.working_display().clone();
    assert_eq!(
        session.handle_presented_event(&event(start), binding, true, false),
        DisplayInput::Unavailable
    );
    assert_eq!(session.working_display(), &working);
    assert!(session.selected_point.is_none());
    assert!(acknowledge(&mut session, &newest, binding));
    let current = newest
        .transform
        .world_to_screen(session.profile().points[1])
        .map(|v| v.round() as i32);
    assert_eq!(
        session.handle_presented_event(&event(current), binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Handled)
    );
    assert_eq!(session.selected_point, Some(1));
    assert_eq!(session.original_display(), &original);
}
#[test]
fn display_bindings_focus_visibility_clone_and_lifecycle_never_activate_stale_input() {
    let mut session = session();
    let binding = target(&session);
    let frame = session.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut session, &frame, binding));
    let commit = TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Return));
    assert_eq!(
        session.handle_presented_event(&commit, binding, false, false),
        DisplayInput::Outcome(EditorDisplayResult::Ignored)
    );
    let other_view = TheEvent::RenderViewClicked(TheId::named("PolyView"), Vec2::new(90, 14));
    assert_eq!(
        session.handle_presented_event(&other_view, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Ignored)
    );
    let mut clone = session.clone();
    assert!(clone.last_transform.is_none());
    assert!(!acknowledge(&mut clone, &frame, binding));
    assert_eq!(
        clone.handle_presented_event(&commit, binding, true, false),
        DisplayInput::Unavailable
    );
    let own = clone.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut clone, &own, binding));
    assert!(!acknowledge(&mut session, &own, binding));
    for changed in [
        DisplayTarget {
            scope: PanelScope {
                document: Uuid::new_v4(),
                ..binding.scope
            },
            ..binding
        },
        DisplayTarget {
            scope: PanelScope {
                revision: binding.scope.revision + 1,
                ..binding.scope
            },
            ..binding
        },
        DisplayTarget {
            context: ProjectContext::Region(Uuid::new_v4()),
            ..binding
        },
        DisplayTarget {
            size: (801, 360),
            ..binding
        },
        DisplayTarget {
            visible: false,
            ..binding
        },
        DisplayTarget {
            view_name: "PolyView",
            ..binding
        },
    ] {
        let frame = session.prepare_frame(binding).unwrap();
        assert!(acknowledge(&mut session, &frame, binding));
        assert_eq!(
            session.handle_presented_event(&commit, changed, true, false),
            DisplayInput::Unavailable
        );
        assert!(!acknowledge(&mut session, &frame, binding));
    }
    for size in [(0, 360), (800, 0)] {
        assert!(
            session
                .prepare_frame(DisplayTarget { size, ..binding })
                .is_none()
        );
    }
    let pending = session.prepare_frame(binding).unwrap();
    session.invalidate_frame();
    assert!(!acknowledge(&mut session, &pending, binding));
    let resumed = session.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut session, &resumed, binding));
    assert_eq!(
        session.handle_presented_event(&commit, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Commit)
    );
    assert_eq!(
        session.handle_presented_event(&commit, binding, true, false),
        DisplayInput::Unavailable
    );
}
#[test]
fn display_scoped_controls_keep_grid_presets_scale_and_cancel_without_exposing_clipped_buttons() {
    let mut session = session();
    let binding = target(&session);
    let original = session.original_display().clone();
    let frame = session.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut session, &frame, binding));
    let click = |p| TheEvent::RenderViewClicked(TheId::named("PrefabView"), p);
    let scale = EditorDisplaySession::scale_track_rect();
    assert!(
        matches!(
            session.handle_presented_event(
                &click(Vec2::new(scale.x + scale.width - 1, scale.y + 4)),
                binding,
                true,
                false
            ),
            DisplayInput::Outcome(EditorDisplayResult::DisplayChanged(_))
        ) || session.dragging_profile_scale
    );
    // A new press starts a new gesture rather than retaining a scale capture.
    let frame = session.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut session, &frame, binding));
    let preset = EditorDisplaySession::preset_button_rect(1);
    assert!(matches!(
        session.handle_presented_event(
            &click(Vec2::new(preset.x + 2, preset.y + 2)),
            binding,
            true,
            false
        ),
        DisplayInput::Outcome(EditorDisplayResult::DisplayChanged(_))
    ));
    assert!(!session.dragging_profile_scale);
    assert!(!session.dragging);
    assert_eq!(
        session.profile().points,
        session.profile().presets[1].points
    );
    let frame = session.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut session, &frame, binding));
    let cancel = TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape));
    assert_eq!(
        session.handle_presented_event(&cancel, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Cancel)
    );
    assert_eq!(session.original_display(), &original);
    let mut tiny = super::super::native_tests::fixture(0);
    let binding = DisplayTarget {
        size: (320, 96),
        ..binding
    };
    let frame = tiny.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut tiny, &frame, binding));
    assert!(
        tiny.preset_at(Vec2::new(20, 80)).is_none(),
        "not painted because its bottom meets the viewport edge"
    );
    assert_eq!(tiny.last_view_width, 320);
    assert!(
        tiny.last_view_width < 580,
        "scale control is not active on a narrow view"
    );
}
#[test]
#[cfg(feature = "native-ui")]
fn display_receipts_must_appear_in_the_successful_root_list() {
    let mut session = session();
    let binding = target(&session);
    let frame = session.prepare_frame(binding).unwrap();
    let mut other = session.clone();
    let foreign = other.prepare_frame(binding).unwrap();
    assert!(!session.presented_paint(&frame, &[], binding));
    assert!(!session.presented_paint(&frame, &[foreign.receipt()], binding));
    assert!(!session.presented_paint(&foreign, &[foreign.receipt()], binding));
    let mut list = theframework::paint::PaintList::default();
    list.ops
        .push(theframework::paint::PaintOp::Receipt(frame.receipt()));
    assert!(session.presented_paint(&frame, &list.receipts().collect::<Vec<_>>(), binding));
}

#[test]
fn display_presented_navigation_delete_and_commit_preserve_the_original_and_focus() {
    let mut session = session();
    let binding = target(&session);
    let original = session.original_display().clone();
    let frame = session.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut session, &frame, binding));
    let pan = TheEvent::RenderViewScrollBy(TheId::named("PrefabView"), Vec2::new(10, 20));
    assert_eq!(
        session.handle_presented_event(&pan, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Handled)
    );
    assert_eq!(session.pan, Vec2::new(10., 20.));
    let scroll = TheEvent::RenderViewPreciseScrollBy(TheId::named("PrefabView"), Vec2::new(0, 4));
    assert_eq!(
        session.handle_presented_event(&scroll, binding, true, true),
        DisplayInput::Outcome(EditorDisplayResult::Handled)
    );
    assert!(session.zoom > 1.);
    assert_eq!(session.pan, Vec2::new(10., 20.));
    let before = session.zoom;
    let pinch = TheEvent::RenderViewZoomBy(TheId::named("PrefabView"), 0.1);
    assert_eq!(
        session.handle_presented_event(&pinch, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Handled)
    );
    assert!(session.zoom > before);
    let frame = session.prepare_frame(binding).unwrap();
    let point = frame
        .transform
        .world_to_screen(session.profile().points[1])
        .map(|v| v.round() as i32);
    let click = TheEvent::RenderViewClicked(TheId::named("PrefabView"), point);
    assert_eq!(
        session.handle_presented_event(&click, binding, true, false),
        DisplayInput::Unavailable
    );
    assert!(acknowledge(&mut session, &frame, binding));
    assert_eq!(
        session.handle_presented_event(&click, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Handled)
    );
    assert_eq!(session.selected_point, Some(1));
    let delete = TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete));
    assert_eq!(
        session.handle_presented_event(&delete, binding, false, false),
        DisplayInput::Outcome(EditorDisplayResult::Ignored)
    );
    assert_eq!(session.profile().points.len(), 4);
    assert!(matches!(
        session.handle_presented_event(&delete, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::DisplayChanged(_))
    ));
    assert_eq!(session.profile().points.len(), 3);
    assert_eq!(session.original_display(), &original);
    let frame = session.prepare_frame(binding).unwrap();
    assert!(acknowledge(&mut session, &frame, binding));
    let commit = TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Return));
    assert_eq!(
        session.handle_presented_event(&commit, binding, true, false),
        DisplayInput::Outcome(EditorDisplayResult::Commit)
    );
    assert_eq!(session.original_display(), &original);
    assert!(session.last_transform.is_none());
}

#[test]
fn display_preparation_handles_large_offscreen_pan_without_destination_allocation() {
    let mut session = session();
    let binding = target(&session);
    for pan in [
        Vec2::new(2_000_000_000., 2_000_000_000.),
        Vec2::new(-2_000_000_000., -2_000_000_000.),
        Vec2::new(i32::MAX as f32, i32::MIN as f32),
    ] {
        session.pan = pan;
        let frame = session.prepare_frame(binding).unwrap();
        assert!(!frame.commands().is_empty());
        assert!(session.last_transform.is_none());
    }
}
