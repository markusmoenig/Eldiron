use super::*;
use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair},
};
fn manager() -> (tempfile::TempDir, Manager) {
    let dir = tempfile::tempdir().unwrap();
    let m = Manager {
        directory: dir.path().to_owned(),
        refresh_endpoint: None,
    };
    (dir, m)
}
fn account() -> Account {
    Account {
        id: Uuid::new_v4(),
        subject: "subject-test".into(),
        email: Some("test@example.invalid".into()),
        client_id: "oaiapp_test".into(),
        model: Some("test-model".into()),
        tokens: Some(Tokens {
            access_token: "test-access".into(),
            refresh_token: None,
            id_token: "test-identity".into(),
            expires_at: now() + 3600,
            scopes: vec![DIRECT_SCOPE.into()],
        }),
    }
}
fn signed(claims: Json) -> (String, Json) {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let key =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();
    let public = key.public_key().as_ref();
    let jwks = json!({"keys":[{"kid":"test","kty":"EC","crv":"P-256","x":B64.encode(&public[1..33]),"y":B64.encode(&public[33..])}]});
    let unsigned = format!(
        "{}.{}",
        B64.encode(json!({"kid":"test","alg":"ES256"}).to_string()),
        B64.encode(claims.to_string())
    );
    let signature = key.sign(&rng, unsigned.as_bytes()).unwrap();
    (
        format!("{unsigned}.{}", B64.encode(signature.as_ref())),
        jwks,
    )
}
#[test]
fn chatgpt_identity_verifies_signature_issuer_audience_nonce_and_expiry() {
    let claims = json!({"iss":ISSUER,"aud":"oaiapp_test","sub":"subject-test","nonce":"nonce-test","exp":2000,"email":"test@example.invalid"});
    let (token, jwks) = signed(claims.clone());
    assert_eq!(
        auth::test_support::validate(&token, &jwks, "oaiapp_test", "nonce-test", 1000)
            .unwrap()
            .0,
        "subject-test"
    );
    for (key, value) in [
        ("iss", json!("https://wrong.invalid")),
        ("aud", json!("other-client")),
        ("nonce", json!("wrong")),
        ("exp", json!(999)),
    ] {
        let mut bad = claims.clone();
        bad[key] = value;
        let (token, jwks) = signed(bad);
        assert!(
            auth::test_support::validate(&token, &jwks, "oaiapp_test", "nonce-test", 1000).is_err()
        );
    }
    let mut parts = token.split('.').map(str::to_string).collect::<Vec<_>>();
    parts[1] = B64.encode(json!({"sub":"forged"}).to_string());
    assert!(
        auth::test_support::validate(&parts.join("."), &jwks, "oaiapp_test", "nonce-test", 1000)
            .is_err()
    );
    let (_, different_keys) = signed(claims);
    assert!(
        auth::test_support::validate(&token, &different_keys, "oaiapp_test", "nonce-test", 1000)
            .is_err()
    );
}
#[test]
fn chatgpt_registration_binds_state_pkce_callback_and_stable_host() {
    let (_dir, manager) = manager();
    auth::test_support::callback(&manager).unwrap();
    let first = manager.locked(|s| Ok(s.host_id.clone())).unwrap();
    auth::test_support::callback(&manager).unwrap();
    assert_eq!(manager.locked(|s| Ok(s.host_id.clone())).unwrap(), first);
}
#[test]
fn chatgpt_credentials_remain_separate_and_logout_keeps_registration() {
    let (dir, manager) = manager();
    let first = account();
    let mut second = account();
    second.client_id = "oaiapp_other".into();
    second.tokens.as_mut().unwrap().access_token = "other-access".into();
    let ids = (first.id, second.id);
    manager
        .locked(|s| {
            s.accounts = vec![first, second];
            s.selected = Some(ids.0);
            manager.save(s)
        })
        .unwrap();
    assert_eq!(
        manager.access_token(ids.0, Duration::from_secs(1)).unwrap(),
        "test-access"
    );
    manager.select(ids.1).unwrap();
    manager.set_model("other-model").unwrap();
    assert_eq!(
        manager.selected().unwrap().unwrap().model.as_deref(),
        Some("other-model")
    );
    assert!(manager.sign_out().unwrap().contains("Signed out"));
    assert!(!manager.selected().unwrap().unwrap().signed_in);
    assert!(manager.provider().is_err());
    manager.select(ids.0).unwrap();
    assert!(manager.provider().is_ok());
    assert_eq!(
        manager.selected().unwrap().unwrap().model.as_deref(),
        Some("test-model")
    );
    assert_eq!(manager.accounts().unwrap().len(), 2);
    let public = serde_json::to_string(&manager.accounts().unwrap()).unwrap();
    assert!(!public.contains("test-access"));
    assert!(!public.contains("test-identity"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(dir.path().join("accounts.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
#[test]
fn chatgpt_scope_and_refresh_rotation_are_explicit() {
    let mut previous = account().tokens.unwrap();
    previous.refresh_token = Some("old-refresh".into());
    let response = json!({"token_type":"Bearer","access_token":"new-access","refresh_token":"new-refresh","expires_in":3600});
    let updated = auth::tokens_from_response(&response, Some(&previous)).unwrap();
    assert_eq!(updated.refresh_token.as_deref(), Some("new-refresh"));
    assert_eq!(updated.scopes, previous.scopes);
    let mut missing = response.clone();
    missing.as_object_mut().unwrap().remove("refresh_token");
    assert!(auth::tokens_from_response(&missing, Some(&previous)).is_err());
    let (_dir, manager) = manager();
    let mut identity_only = account();
    identity_only.tokens.as_mut().unwrap().scopes = vec!["openid".into()];
    manager
        .locked(|s| {
            s.selected = Some(identity_only.id);
            s.accounts.push(identity_only);
            manager.save(s)
        })
        .unwrap();
    assert!(manager.provider().err().unwrap().contains("not authorized"));
}
#[test]
fn chatgpt_catalog_uses_visible_slugs_in_server_order() {
    let models = inference::test_support::models(&json!({"models":[
        {"slug":"hidden","display_name":"Hidden","visibility":"hide"},
        {"slug":"second","display_name":"Second","visibility":"list"},
        {"slug":"first","display_name":"First","visibility":"list"}]}))
    .unwrap();
    assert_eq!(
        models.iter().map(|m| m.slug.as_str()).collect::<Vec<_>>(),
        vec!["second", "first"]
    );
}
fn completed(reply: Json) -> String {
    format!(
        "event: response.completed\ndata: {}\n\n",
        json!({"type":"response.completed","response":{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":reply.to_string()}]}]}})
    )
}
#[test]
fn chatgpt_stream_requires_completed_and_rejects_late_failures() {
    let prefix = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"unconfirmed\"}\n\n";
    let reply = json!({"command":{"operation":"wait","steps":1},"reply":""});
    assert_eq!(
        inference::test_support::stream(format!("{prefix}{}", completed(reply.clone())).as_bytes())
            .unwrap(),
        reply
    );
    assert!(inference::test_support::stream(prefix.as_bytes()).is_err());
    for kind in ["response.failed", "response.incomplete", "error"] {
        let data = format!(
            "{prefix}data: {}\n\n",
            json!({"type":kind,"response":{"error":{"code":"subscription_sharing_usage_limit_exceeded"}}})
        );
        assert!(
            inference::test_support::stream(data.as_bytes())
                .unwrap_err()
                .contains("subscription_sharing_usage_limit_exceeded")
        );
    }
    assert!(inference::test_support::stream(b"data: not-json\n\n").is_err());
}
#[test]
fn chatgpt_incomplete_stream_never_proposes_a_game_action() {
    struct Interrupted;
    impl Provider for Interrupted {
        fn complete(&self, _: &str, _: &Json) -> Result<Json, String> {
            inference::test_support::stream(b"data: {}\n\n")
        }
    }
    let scene = crate::ai::Scene::default();
    let mut session = crate::ai::Assistant::default();
    session.set_provider(Some(Arc::new(Interrupted)));
    session.begin("I wait a moment", scene.clone()).unwrap();
    for _ in 0..1000 {
        if let Some(event) = session.poll(&scene) {
            assert!(matches!(event, crate::ai::Event::Error(_)));
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("Worker did not finish");
}

#[test]
fn chatgpt_returning_signin_reuses_client_and_denial_preserves_active_account() {
    let (_dir, manager) = manager();
    let first = account();
    let id = first.id;
    manager
        .locked(|s| {
            s.accounts.push(first);
            s.selected = Some(id);
            manager.save(s)
        })
        .unwrap();
    auth::test_support::returning(&manager).unwrap();
    auth::test_support::denial(&manager).unwrap();
    assert_eq!(manager.selected().unwrap().unwrap().id, id);
    assert_eq!(
        manager.access_token(id, Duration::from_secs(1)).unwrap(),
        "test-access"
    );
}
#[test]
fn chatgpt_http_uses_public_responses_route_and_subscription_contract() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = vec![];
        loop {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            bytes.push(byte[0]);
            if bytes.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let headers = String::from_utf8(bytes).unwrap();
        assert!(headers.starts_with("POST /v1/responses "));
        assert!(headers.contains("Bearer test-access"));
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::parse)
            })
            .unwrap()
            .unwrap();
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        let body: Json = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
        assert!(body["input"].is_array());
        assert!(body["instructions"].is_string());
        // JSON-mode validation inspects input messages, not just instructions.
        assert_eq!(body["input"][0]["role"], "developer");
        assert!(
            body["input"][0]["content"]
                .as_str()
                .unwrap()
                .contains("JSON")
        );
        assert_eq!(body["input"][1]["role"], "user");
        assert!(
            !body["input"][1]["content"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("json")
        );
        for unsupported in [
            "max_output_tokens",
            "temperature",
            "previous_response_id",
            "conversation",
        ] {
            assert!(body.get(unsupported).is_none());
        }
        let reply = completed(json!({"command":null,"reply":"Which partition?"}));
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",reply.len(),reply).unwrap();
    });
    let reply = inference::test_support::http(&format!("http://{addr}/v1/responses")).unwrap();
    assert_eq!(reply["reply"], "Which partition?");
    server.join().unwrap();
}
#[test]
fn chatgpt_storage_lock_serializes_process_handles() {
    let (_dir, manager) = manager();
    manager
        .locked(|s| {
            s.host_id = "host".into();
            manager.save(s)
        })
        .unwrap();
    let handles = (0..2)
        .map(|_| {
            let m = manager.clone();
            std::thread::spawn(move || {
                m.locked(|s| {
                    let old = s.host_id.clone();
                    std::thread::sleep(Duration::from_millis(40));
                    s.host_id = format!("{old}-updated");
                    m.save(s)
                })
                .unwrap()
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(
        manager.locked(|s| Ok(s.host_id.clone())).unwrap(),
        "host-updated-updated"
    );
}

#[test]
fn chatgpt_concurrent_refresh_rotates_once_and_persists_replacement() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let (_dir, mut manager) = manager();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    manager.refresh_endpoint = Some(format!(
        "http://{}/oauth/token",
        listener.local_addr().unwrap()
    ));
    let mut first = account();
    let id = first.id;
    let tokens = first.tokens.as_mut().unwrap();
    tokens.expires_at = 0;
    tokens.refresh_token = Some("old-refresh".into());
    manager
        .locked(|s| {
            s.accounts.push(first);
            s.selected = Some(id);
            manager.save(s)
        })
        .unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut headers = vec![];
        loop {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            headers.push(byte[0]);
            if headers.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let headers = String::from_utf8(headers).unwrap();
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::parse)
            })
            .unwrap()
            .unwrap();
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        let form: url::form_urlencoded::Parse<'_> = url::form_urlencoded::parse(&body);
        let form = form
            .into_owned()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(form["grant_type"], "refresh_token");
        assert_eq!(form["client_id"], "oaiapp_test");
        assert_eq!(form["refresh_token"], "old-refresh");
        assert_eq!(form["resource"], RESOURCE);
        assert!(!form.contains_key("scope"));
        let response=json!({"token_type":"Bearer","access_token":"replacement-access","refresh_token":"replacement-refresh","expires_in":3600}).to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).unwrap();
    });
    let threads = (0..2)
        .map(|_| {
            let m = manager.clone();
            std::thread::spawn(move || m.access_token(id, Duration::from_secs(5)).unwrap())
        })
        .collect::<Vec<_>>();
    for thread in threads {
        assert_eq!(thread.join().unwrap(), "replacement-access");
    }
    server.join().unwrap();
    manager
        .locked(|s| {
            let tokens = s.accounts[0].tokens.as_ref().unwrap();
            assert_eq!(tokens.refresh_token.as_deref(), Some("replacement-refresh"));
            assert_eq!(tokens.id_token, "test-identity");
            assert!(tokens.expires_at > now() + 3000);
            Ok(())
        })
        .unwrap();
}

#[test]
fn chatgpt_large_catalog_has_a_separate_bound_from_credentials_and_turns() {
    let catalog=json!({"models":[
        {"slug":"available-model","display_name":"Available model","visibility":"list","service_metadata":"x".repeat(MAX_JSON+1000)},
        {"slug":"hidden-model","display_name":"Hidden model","visibility":"hide"}
    ]}).to_string();
    assert!(catalog.len() > MAX_JSON);
    let response = ureq::Response::new(200, "OK", &catalog).unwrap();
    let models = inference::test_support::catalog_response(response).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].slug, "available-model");
    // Authentication/error response limits remain small.
    assert!(
        read_json(ureq::Response::new(200, "OK", &catalog).unwrap())
            .unwrap_err()
            .contains("size limit")
    );
    let oversized = " ".repeat(inference::test_support::catalog_limit() + 1);
    let error = inference::test_support::catalog_response(
        ureq::Response::new(200, "OK", &oversized).unwrap(),
    )
    .unwrap_err();
    assert!(error.contains("model catalog exceeded"));
}

#[test]
fn chatgpt_request_errors_identify_parameters_without_echoing_credentials() {
    let data = json!({"error":{"message":"Unsupported format for test-access-token","param":"text.format","code":"unsupported_parameter"}});
    let message = inference::test_support::diagnostic_message(400, &data, "test-access-token");
    assert!(message.contains("text.format"));
    assert!(message.contains("unsupported_parameter"));
    assert!(message.contains("req_test"));
    assert!(message.contains("[redacted]"));
    assert!(!message.contains("test-access-token"));
    assert!(!message.contains("Sign in again"));
    let admission = inference::test_support::diagnostic_message(
        400,
        &json!({"detail":"Unsupported field: text.format"}),
        "",
    );
    assert!(admission.contains("Unsupported field: text.format"));
}

#[test]
fn chatgpt_stream_accepts_completed_items_with_an_empty_terminal_output() {
    // Actual store:false subscription streams deliver text in output_item.done,
    // then send response.completed with an empty output array.
    let reply = json!({"command":{"operation":"wait","steps":1},"reply":""});
    let item = format!(
        "data: {}\n\n",
        json!({"type":"response.output_item.done","output_index":0,"item":{"type":"message","status":"completed","role":"assistant","phase":"final_answer","content":[{"type":"output_text","text":reply.to_string()}]}})
    );
    let complete = format!(
        "data: {}\n\n",
        json!({"type":"response.completed","response":{"status":"completed","output":[]}})
    );
    assert_eq!(
        inference::test_support::stream(format!("{item}{complete}").as_bytes()).unwrap(),
        reply
    );
    assert!(inference::test_support::stream(item.as_bytes()).is_err());
    let fail = "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"subscription_sharing_usage_limit_exceeded\"}}}\n\n";
    assert!(inference::test_support::stream(format!("{item}{fail}").as_bytes()).is_err());
    // Duplicate item events cannot duplicate a result.
    assert_eq!(
        inference::test_support::stream(format!("{item}{item}{complete}").as_bytes()).unwrap(),
        reply
    );
}
