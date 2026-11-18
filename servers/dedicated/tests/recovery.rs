use eldiron_server::recovery;
#[test]
fn independently_loaded_release_identity_is_stable() {
    for name in ["3DStarter.eldiron", "Hideout2D.eldiron"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects")
            .join(name);
        let a = recovery::load_project(&path).unwrap();
        let b = recovery::load_project(&path).unwrap();
        let av = serde_json::to_value(&a).unwrap();
        let bv = serde_json::to_value(&b).unwrap();
        fn differences(
            a: &serde_json::Value,
            b: &serde_json::Value,
            path: &str,
            found: &mut Vec<String>,
        ) {
            if a == b || found.len() > 12 {
                return;
            }
            match (a, b) {
                (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                    for (k, a) in a {
                        differences(
                            a,
                            b.get(k).unwrap_or(&serde_json::Value::Null),
                            &format!("{path}/{k}"),
                            found,
                        );
                    }
                }
                (serde_json::Value::Array(a), serde_json::Value::Array(b))
                    if a.len() == b.len() =>
                {
                    for (n, (a, b)) in a.iter().zip(b).enumerate() {
                        differences(a, b, &format!("{path}/{n}"), found);
                    }
                }
                _ => found.push(format!("{path}: {a} versus {b}")),
            }
        }
        let mut changes = vec![];
        differences(&av, &bv, "", &mut changes);
        assert_eq!(
            eldiron_network::project_content_revision(&a).unwrap(),
            eldiron_network::project_content_revision(&b).unwrap(),
            "{name}: {changes:?}"
        );
        assert_eq!(
            eldiron_runtime::persistence::content_revision(&a).unwrap(),
            eldiron_runtime::persistence::content_revision(&b).unwrap()
        );
    }
}
