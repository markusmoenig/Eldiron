use super::*;
use serde_json::{Value as Json, json};
use shared::ai::Assistant;
use shared::ai::provider::Provider;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
struct Script {
    replies: Mutex<std::collections::VecDeque<Result<Json, String>>>,
    calls: AtomicUsize,
    seen: Mutex<Vec<Json>>,
}
impl Provider for Script {
    fn complete(&self, _: &str, context: &Json) -> Result<Json, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(context.clone());
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("Unexpected call")
    }
}
fn fixture() -> TerminalApp {
    let mut project = Project::default();
    let mut region = shared::region::Region::new();
    region.name = "Hall".into();
    region.map = Map::default();
    region.map.name = "Hall".into();
    let vertices: Vec<_> = [(-4., -4.), (4., -4.), (4., 4.), (-4., 4.)]
        .into_iter()
        .map(|(x, y)| region.map.add_vertex_at(x, y))
        .collect();
    let edges = (0..4)
        .map(|i| {
            region
                .map
                .create_linedef_manual(vertices[i], vertices[(i + 1) % 4])
        })
        .collect();
    let mut sector = rusterix::Sector::new(0, edges);
    sector.name = "Hall".into();
    sector.properties.set(
        "data",
        Value::Str(
            "title='Entrance Hall'\ndescription='A cold hall.'\ngm_secret='SECRET_PASSWORD'".into(),
        ),
    );
    region.map.sectors.push(sector);
    let mut player = Entity::new();
    player.id = 1;
    player.set_attribute("player", Value::Bool(true));
    player.set_attribute("sector_id", Value::Int(0));
    player.set_attribute("class_name", Value::Str("Player".into()));
    region.map.entities.push(player);
    let mut guard = Entity::new();
    guard.id = 7;
    guard.set_attribute("name", Value::Str("Guard".into()));
    guard.set_attribute("sector_id", Value::Int(0));
    guard.set_attribute("private_notes", Value::Str("SECRET_GUARD".into()));
    region.map.entities.push(guard);
    let mut hidden = Entity::new();
    hidden.id = 8;
    hidden.position.x = 100.;
    hidden.set_attribute("name", Value::Str("SECRET_ACTOR".into()));
    region.map.entities.push(hidden);
    let mut character = shared::character::Character::new();
    character.name = "Player".into();
    character.data = "[input]\na='intent.attack'\nl='intent.look'\n".into();
    project.characters.insert(character.id, character);
    project.regions = vec![region];
    TerminalApp {
        project,
        assets: Assets::default(),
        server: Server::default(),
        current_map: "Hall".into(),
        session: TextSession::new(),
        screen_messages: vec![],
        auto_attack_target: None,
        server_log_cursor: 0,
        ai: Assistant::default(),
        ai_session: Uuid::new_v4(),
    }
}
fn enable(app: &mut TerminalApp, replies: Vec<Result<Json, String>>) -> Arc<Script> {
    let script = Arc::new(Script {
        replies: Mutex::new(replies.into()),
        calls: AtomicUsize::new(0),
        seen: Mutex::new(vec![]),
    });
    app.ai.set_provider(Some(script.clone()));
    script
}
fn finish(app: &mut TerminalApp) -> Vec<String> {
    let mut output = vec![];
    for _ in 0..1000 {
        output.extend(poll(app));
        if !app.ai.is_busy() {
            return output;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("AI did not finish")
}
#[test]
fn legacy_ai_uses_existing_look_and_inventory_without_concept_or_private_source() {
    let mut app = fixture();
    let original = serde_json::to_value(&app.project).unwrap();
    let ordinary = handle_command(&mut app, "look").1;
    assert!(ordinary.join("\n").contains("Entrance Hall"));
    let p = enable(
        &mut app,
        vec![
            Ok(json!({"command":{"operation":"look"},"reply":""})),
            Ok(json!({"reply":"You stand in a cold hall."})),
        ],
    );
    assert!(
        input(&mut app, "Describe my surroundings naturally")
            .1
            .is_empty()
    );
    let output = finish(&mut app);
    assert!(output.iter().any(|s| ordinary.contains(s)));
    assert_eq!(serde_json::to_value(&app.project).unwrap(), original);
    let seen = p.seen.lock().unwrap();
    let json = serde_json::to_string(&*seen).unwrap();
    assert!(!json.contains("SECRET_"));
    assert!(!json.contains("node_graphs"));
    assert_eq!(seen[0]["scene"]["targets"].as_array().unwrap().len(), 1);
}
#[test]
fn legacy_explicit_commands_errors_and_cancel_bypass_ai() {
    let mut app = fixture();
    let p = enable(&mut app, vec![]);
    let expected = handle_command(&mut app, "go missing exit");
    assert_eq!(input(&mut app, "go missing exit"), expected);
    input(&mut app, "attack missing guard");
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    let p = enable(
        &mut app,
        vec![Ok(json!({"command":{"operation":"wait"},"reply":""}))],
    );
    input(&mut app, "I would like to wait a little");
    assert!(input(&mut app, "cancel").1.join("\n").contains("cancelled"));
    assert!(!app.ai.is_busy());
    assert!(poll(&mut app).is_empty());
    let _ = p;
}
#[test]
fn legacy_ai_rejects_missing_targets_and_discards_results_after_a_scene_change() {
    let mut app = fixture();
    let before = serde_json::to_value(&app.project).unwrap();
    enable(
        &mut app,
        vec![Ok(
            json!({"command":{"operation":"interact","target":"entity:8","intent":"attack"},"reply":""}),
        )],
    );
    input(&mut app, "I would like to attack the hidden actor");
    assert!(finish(&mut app).join("\n").contains("unavailable"));
    assert_eq!(serde_json::to_value(&app.project).unwrap(), before);
    enable(
        &mut app,
        vec![Ok(json!({"command":{"operation":"wait"},"reply":""}))],
    );
    input(&mut app, "I would like to pause briefly");
    app.ai_session = Uuid::new_v4();
    assert!(finish(&mut app).join("\n").contains("scene changed"));
}
#[test]
fn legacy_project_roundtrip_and_plain_text_mode_keep_their_original_format() {
    let mut app = fixture();
    let before = handle_command(&mut app, "inventory");
    assert_eq!(input(&mut app, "inventory"), before);
    let encoded = serde_json::to_vec(&app.project).unwrap();
    assert!(!String::from_utf8_lossy(&encoded).contains("ai_provider"));
    let restored: Project = serde_json::from_slice(&encoded).unwrap();
    app.project = restored;
    assert_eq!(input(&mut app, "inventory"), before);
    assert!(parse_terminal_args(&["client", "--ai-provider=disabled"].map(str::to_string)).is_ok());
}
