use super::*;
fn presenter(layer: &objc2_quartz_core::CAMetalLayer, size: (u32, u32)) -> GpuPresenter<'static> {
    let pointer = (layer as *const objc2_quartz_core::CAMetalLayer)
        .cast_mut()
        .cast();
    let surface = futures::executor::block_on(unsafe {
        renderkit::SurfaceState::from_metal_layer(pointer, size)
    })
    .unwrap();
    GpuPresenter::new(surface).unwrap()
}
#[test]
#[ignore = "requires native Metal; actual common player on foreign CAMetalLayer"]
fn foreign_player_presents_and_preserves_authored_data_and_session_across_lifecycle() {
    let layer = objc2_quartz_core::CAMetalLayer::new();
    let mut runner = ForeignPlayerHost::new(presenter(&layer, (960, 720)), (960, 720), 1.5);
    let project = super::super::tests::starter_with_menu();
    let source = shared::project_io::encode_project(&project).unwrap();
    runner.load(&source, false).unwrap();
    assert_eq!(runner.source(), source);
    let session = runner
        .application
        .as_ref()
        .unwrap()
        .player
        .runtime
        .view()
        .session_id;
    let registry = runner.presenter.images.owner_id();
    runner.update(Duration::ZERO);
    assert_eq!(runner.present().unwrap(), Some(PresentResult::Presented));
    assert_eq!(runner.presentations(), 1);
    assert_eq!(runner.present().unwrap(), None);
    let frame = runner.application.as_ref().unwrap().frame_count();
    assert!(frame > 0);
    assert!(runner.load(b"invalid project", false).is_err());
    assert_eq!(
        runner
            .application
            .as_ref()
            .unwrap()
            .player
            .runtime
            .view()
            .session_id,
        session
    );
    assert_eq!(runner.presenter.images.owner_id(), registry);
    assert_eq!(runner.source(), source);
    let app = runner.application.as_ref().unwrap();
    let p = app.viewport().transform().point([300., 220.]);
    for pressed in [true, false] {
        runner.input(&InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed,
            position: p,
        });
    }
    let target = runner.host.text_input.unwrap().target;
    runner.input(&InputEvent::TextFor {
        target,
        text: "東京".into(),
    });
    assert!(
        runner
            .application
            .as_ref()
            .unwrap()
            .screen
            .elements
            .ui_state["start.name"]
            .contains("東京")
    );
    runner.update(Duration::from_millis(250));
    assert_eq!(
        runner
            .application
            .as_ref()
            .unwrap()
            .player
            .runtime
            .tick_sequence(),
        1
    );
    assert_eq!(runner.present().unwrap(), Some(PresentResult::Presented));
    assert_eq!(runner.presenter.context().transfers().readback_bytes, 0);
    let typed = runner
        .application
        .as_ref()
        .unwrap()
        .screen
        .elements
        .ui_state["start.name"]
        .clone();
    runner.resize((0, 0), 1.5);
    let before = runner.application.as_ref().unwrap().frame_count();
    runner.update(Duration::from_millis(250));
    assert_eq!(runner.present().unwrap(), Some(PresentResult::Deferred));
    assert_eq!(runner.application.as_ref().unwrap().frame_count(), before);
    assert_eq!(
        runner
            .application
            .as_ref()
            .unwrap()
            .player
            .runtime
            .tick_sequence(),
        2
    );
    assert_eq!(runner.presentations(), 2);
    runner.suspend();
    runner.update(Duration::from_secs(30));
    assert_eq!(runner.present().unwrap(), None);
    assert_eq!(
        runner
            .application
            .as_ref()
            .unwrap()
            .player
            .runtime
            .tick_sequence(),
        2
    );
    runner.resume();
    runner.resize((1200, 900), 1.5);
    assert_eq!(runner.present().unwrap(), Some(PresentResult::Presented));
    let old_generation = runner.presenter.context().generation();
    let replacement_layer = objc2_quartz_core::CAMetalLayer::new();
    runner
        .replace_presenter(presenter(&replacement_layer, (640, 480)))
        .unwrap();
    assert_ne!(runner.presenter.context().generation(), old_generation);
    assert_eq!(runner.host.physical_size, (640, 480));
    assert_eq!(runner.host.logical_size, [640. / 1.5, 480. / 1.5]);
    assert_eq!(
        runner
            .application
            .as_ref()
            .unwrap()
            .player
            .runtime
            .view()
            .session_id,
        session
    );
    assert_eq!(
        runner
            .application
            .as_ref()
            .unwrap()
            .screen
            .elements
            .ui_state["start.name"],
        typed
    );
    assert_eq!(
        runner
            .application
            .as_ref()
            .unwrap()
            .player
            .runtime
            .tick_sequence(),
        2
    );
    assert_eq!(runner.present().unwrap(), Some(PresentResult::Presented));
    assert_eq!(runner.presentations(), 4);
    assert_eq!(runner.presenter.context().transfers().readback_bytes, 0);
    // Source preservation is byte exact even though the live project is migrated
    // and its maps/time/character UI have advanced since opening the archive.
    assert_eq!(runner.source(), source);
    assert!(runner.application.as_ref().unwrap().failure().is_none());
    drop(runner); // Rust surface and resources must precede both host layers.
    drop(replacement_layer);
    drop(layer);
}
