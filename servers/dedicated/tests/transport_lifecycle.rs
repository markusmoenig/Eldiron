use eldiron_network::{ClientMessage, PROTOCOL_VERSION, ServerMessage, native};
use eldiron_runtime::Project;
use eldiron_server::{
    host::Host,
    operations::Operations,
    transport::{self, TransportConfig},
};
use std::time::Duration;
use theframework_core::Uuid;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_exit_and_cancelled_owner_close_all_socket_tasks() {
    for cancelled in [false, true] {
        let mut project = Project::new();
        project.regions[0].name = "Lifecycle".into();
        project.regions[0].map.name = "Lifecycle".into();
        project.config = "[game]\nstart_region='Lifecycle'\nauto_create_player=false".into();
        let dir = std::env::temp_dir().join(format!("eldiron-lifecycle-{}", Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let mut host = Host::open(project, None).await.unwrap();
        let metrics = dir.join("metrics.json");
        let mut operations = Operations::default();
        operations.metrics_file = Some(metrics.clone());
        host.configure_operations(operations).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(transport::serve(
            listener,
            host,
            TransportConfig {
                tls: None,
                origins: vec![],
            },
            async {
                let _ = shutdown.await;
            },
        ));
        let mut socket = native::connect(&format!("ws://{address}/play"), true)
            .await
            .unwrap();
        native::send(
            &mut socket,
            &ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            native::receive(&mut socket).await.unwrap(),
            ServerMessage::Welcome { .. }
        ));
        let mut stalled = tokio::net::TcpStream::connect(address).await.unwrap();
        stalled.write_all(b"G").await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        if cancelled {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        } else {
            stop.send(()).unwrap();
            task.await.unwrap().unwrap();
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert_eq!(value["transport"]["active"], 0);
        }
        assert!(
            tokio::time::timeout(Duration::from_secs(1), native::receive(&mut socket))
                .await
                .unwrap()
                .is_err(),
            "old world socket outlived its owner"
        );
        let read = tokio::time::timeout(Duration::from_secs(1), stalled.read_u8())
            .await
            .unwrap();
        assert!(
            read.is_err(),
            "half-open HTTP connection outlived its owner"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
