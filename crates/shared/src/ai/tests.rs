use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
struct Script {
    replies: Mutex<VecDeque<Result<Value, String>>>,
    calls: AtomicUsize,
}
impl Provider for Script {
    fn complete(&self, instructions: &str, context: &Value) -> Result<Value, String> {
        assert!(!instructions.contains("Concept"));
        assert!(context.get("campaign").is_none());
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("Unexpected provider request")
    }
}
fn scene() -> Scene {
    Scene {
        location: "room:1".into(),
        title: "Hall".into(),
        targets: vec![Target {
            id: "entity:7".into(),
            title: "Guard".into(),
            kind: "character".into(),
            intents: vec!["look".into(), "talk".into()],
        }],
        exits: vec![Exit {
            id: "exit:2".into(),
            title: "Cellar stairs".into(),
            direction: "north".into(),
        }],
        ..Default::default()
    }
}
fn assistant(replies: Vec<Result<Value, String>>) -> (Assistant, Arc<Script>) {
    let p = Arc::new(Script {
        replies: Mutex::new(replies.into()),
        calls: AtomicUsize::new(0),
    });
    let mut a = Assistant::default();
    a.set_provider(Some(p.clone()));
    (a, p)
}
fn wait(a: &mut Assistant, s: &Scene) -> Event {
    for _ in 0..1000 {
        if let Some(e) = a.poll(s) {
            return e;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("Worker did not finish")
}
#[test]
fn ai_interpretation_requests_one_existing_action_then_narrates_the_confirmed_result() {
    let s = scene();
    let (a, p) = assistant(vec![
        Ok(
            json!({"command":{"operation":"interact","target":"entity:7","intent":"talk"},"reply":""}),
        ),
        Ok(json!({"reply":"The guard greets you."})),
    ]);
    let mut a = a;
    a.begin("Ask the guard what happened", s.clone()).unwrap();
    assert!(matches!(
        wait(&mut a, &s),
        Event::Action(Action::Interact { .. })
    ));
    assert!(a.poll(&s).is_none());
    assert!(a.is_busy());
    a.narrate_result("Guard: Hello.", s.clone()).unwrap();
    assert!(matches!(wait(&mut a, &s), Event::Reply(_)));
    assert!(!a.is_busy());
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    assert!(a.narrate_result("Duplicate", s).is_err());
}
#[test]
fn ai_cannot_invent_exits_intents_or_file_operations() {
    for command in [
        json!({"operation":"move","exit":"secret"}),
        json!({"operation":"interact","target":"entity:7","intent":"destroy"}),
        json!({"operation":"load","path":"anything"}),
        json!({"operation":"wait","steps":100}),
    ] {
        let (mut a, p) = assistant(vec![Ok(json!({"command":command,"reply":""}))]);
        let s = scene();
        a.begin("Do something", s.clone()).unwrap();
        assert!(matches!(wait(&mut a, &s), Event::Error(_)));
        assert!(!a.is_busy());
        assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    }
}
#[test]
fn ai_clarification_and_failed_planning_never_return_an_action() {
    for reply in [
        Ok(json!({"command":null,"reply":"Which guard?"})),
        Err("offline".into()),
        Ok(json!({"command":{"operation":"wait"},"reply":"You received gold!"})),
    ] {
        let (mut a, _) = assistant(vec![reply]);
        let s = scene();
        a.begin("Please help", s.clone()).unwrap();
        assert!(!matches!(wait(&mut a, &s), Event::Action(_)));
    }
}
#[test]
fn ai_scene_changes_and_cancellation_make_late_replies_inert() {
    let (mut a, _) = assistant(vec![Ok(json!({"command":{"operation":"wait"},"reply":""}))]);
    let mut s = scene();
    a.begin("Pause for a moment", s.clone()).unwrap();
    s.location = "room:2".into();
    assert!(matches!(wait(&mut a, &s), Event::Error(_)));
    assert!(!a.is_busy());
    let (mut a, _) = assistant(vec![Ok(json!({"command":{"operation":"wait"},"reply":""}))]);
    a.begin("Pause for a moment", s.clone()).unwrap();
    assert!(a.cancel());
    assert!(!a.is_busy());
    assert!(a.poll(&s).is_none());
}
#[test]
fn ai_narration_failure_cannot_repeat_the_completed_game_action() {
    let (mut a, p) = assistant(vec![
        Ok(json!({"command":{"operation":"wait"},"reply":""})),
        Err("offline after action".into()),
    ]);
    let s = scene();
    a.begin("Wait a little", s.clone()).unwrap();
    assert!(matches!(wait(&mut a, &s), Event::Action(Action::Wait {})));
    a.narrate_result("You wait.", s.clone()).unwrap();
    assert!(matches!(wait(&mut a, &s), Event::Error(_)));
    assert!(a.poll(&s).is_none());
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
}
#[test]
fn ai_deadlines_limits_and_unavailable_providers_leave_the_host_untouched() {
    let s = scene();
    let mut a = Assistant::default();
    assert!(a.begin("Please wait", s.clone()).is_err());
    let (mut a, _) = assistant(vec![Ok(json!({"command":{"operation":"wait"},"reply":""}))]);
    assert!(a.begin(&"x".repeat(4001), s.clone()).is_err());
    a.begin("Pause", s.clone()).unwrap();
    a.pending.as_mut().unwrap().started = Instant::now() - DEADLINE - Duration::from_secs(1);
    assert!(matches!(wait(&mut a, &s), Event::Error(_)));
    assert!(!a.is_busy());
}

#[test]
fn polling_does_not_rebuild_the_scene_until_a_reply_is_ready() {
    let mut a = Assistant::default();
    assert!(a.poll_with(|| panic!("No request is pending")).is_none());
    let (tx, rx) = mpsc::channel();
    a.pending = Some(Pending {
        receiver: Mutex::new(rx),
        started: Instant::now(),
        scene: scene(),
        input: "Hello".into(),
        phase: Phase::Plan,
    });
    assert!(
        a.poll_with(|| panic!("The worker has not replied"))
            .is_none()
    );
    tx.send(Ok(json!({"command":null,"reply":"Hello, traveler."})))
        .unwrap();
    assert!(matches!(a.poll_with(|| Ok(scene())), Some(Event::Reply(_))));
    assert!(!a.is_busy());
}
