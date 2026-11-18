use eldiron_runtime::{PlayerSession,Project};
use shared::project_play::{LaunchPurpose,LaunchPlan,ProjectPlaySettings};
use std::time::Duration;
fn project()->Project {
    let mut project=Project::new();project.regions[0].name="Mode test".into();project.regions[0].map.name="Mode test".into();
    project.config="[game]\nmode='multiplayer'\nstart_region='Mode test'\nauto_create_player=true\n[multiplayer]\nendpoint='wss://example.com/play'".into();project
}
#[test]
fn multiplayer_client_never_starts_a_local_world_or_publishes_authored_actors() {
    let mut player=PlayerSession::new(project(),false).unwrap();
    assert!(player.is_remote());assert!(!player.runtime.is_started());assert!(player.runtime.server.instances.is_empty());
    assert!(player.view().map.is_none());assert!(!player.runtime.has_started_player());
    player.auto_start(true).unwrap();player.advance(Duration::from_secs(1));
    assert_eq!(player.runtime.tick_sequence(),0);assert!(player.runtime.server.instances.is_empty());
    assert!(player.game_command("start",&mut Default::default()).is_err());
}
#[test]
fn creator_override_runs_the_same_online_document_locally_without_an_endpoint() {
    let mut project=project();project.config=project.config.replace("endpoint='wss://example.com/play'", "endpoint=''");
    assert!(PlayerSession::new(project.clone(),false).is_err());
    let mut local=PlayerSession::new_local_test(project.clone(),true).unwrap();
    assert!(!local.is_remote());assert!(local.runtime.is_started());assert!(!local.runtime.server.instances.is_empty());
    assert_eq!(ProjectPlaySettings::parse(&project.config).unwrap().launch(LaunchPurpose::CreatorTest).unwrap(),LaunchPlan::Local);
    local.runtime.stop();assert!(project.config.contains("mode='multiplayer'"));
}
